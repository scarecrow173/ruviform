//! [`PluginEndpoint`] — the RT processing endpoint detached from a
//! live instance: the ownership-split cell that lets an engine-side
//! graph node drive `process` without touching the host's command
//! surface (Issue #113 →
//! `docs/designs/engine-plugin-host-integration.md`).
//!
//! One instance carries at most one endpoint, minted while the
//! instance is `Activated`/`Processing` (the composite's
//! `prepare_endpoint` — Task 4's public surface — owns the
//! attach-side prelude; [`InProcessHost::attach_endpoint`] is the
//! L0 inner). The endpoint is `Send` and holds only endpoint legs:
//! shared-cell `Arc` handles for L0, the moved RT leg for L1 (Task
//! 3). It never carries a `PluginHost`, an `L1Instance`, a raw
//! non-`Send` plugin object, a shared-memory segment or a child
//! handle.
//!
//! The per-block output table (the design's defined-rejection
//! surface):
//!
//! | observation | result |
//! | --- | --- |
//! | mirror `Failed` / poisoned lock / plugin panic | silence written, `Ok(ProcessResult::CONTINUE)` |
//! | mirror not `Processing` | `Err(HostError::Lifecycle(_))`, `outputs` unchanged |
//! | block outside the activated config / bound | `Err(HostError::Prepare(_))`, `outputs` unchanged |
//! | shared cell busy (`try_lock` `WouldBlock`) | `Err(HostError::EndpointUnavailable)`, `outputs` unchanged |
//!
//! [`InProcessHost::attach_endpoint`]: crate::InProcessHost

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate, SpeakerMask};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, HostError, HostInput, HostOutput, LifecycleError,
    LifecycleEvent, LifecycleState, PrepareError, ProcessResult, TransportInfo,
};
use ruviform_rt::{Consumer, Producer, Retirer};

use crate::host::SendCore;
use crate::l1::data::{EndpointLegParts, RtLeg, Staging, mirror_decode, mirror_encode};

/// The capacity contract an endpoint was attached under — the
/// `bounds` given at `prepare_endpoint`, recorded for the
/// endpoint's lifetime.
///
/// `bound` is the scratch ceiling only; the per-block gate a
/// `process` call must pass is the activated `ProcessConfig`
/// contract (`min_frames ≤ frames ≤ max_frames`), checked against
/// the block's own in-bus-first dimensions.
///
/// [`ProcessConfig`]: ruviform_plugin_core::ProcessConfig
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EndpointBounds {
    /// Block-size ceiling: non-zero, `≤ WIRE_MAX_FRAMES` and `≥` the
    /// activated `max_frames` — the amount `in_scratch` was
    /// allocated for. A re-`Activate` carrying `max_frames` above
    /// this bound is rejected while the endpoint lives.
    ///
    /// [`WIRE_MAX_FRAMES`]: ruviform_plugin_core::wire::WIRE_MAX_FRAMES
    pub max_frames: u32,
    /// The graph-side bus width — non-zero and `≤ WIRE_MAX_CHANNELS`.
    /// v1 does not require equality with the activated main bus
    /// (plugins expose no channel query — Issue #114 contract work);
    /// the per-block rule is `out.channels() == bound.channels`,
    /// enforced by `process_audio`'s bound clip.
    ///
    /// [`WIRE_MAX_CHANNELS`]: ruviform_plugin_core::wire::WIRE_MAX_CHANNELS
    pub channels: u32,
}

/// The per-instance RT endpoint: `Send`, moved exactly once into
/// the RT-side owner (e.g. a graph node closure) after
/// `prepare_endpoint` mints it.
///
/// **RT contract:** [`PluginEndpoint::process`],
/// [`PluginEndpoint::bounds`] and [`PluginEndpoint::latency_frames`]
/// are RT-suitable — `process` never blocks and never allocates on
/// this path (L0: one `try_lock` plus the plugin call). `Drop` runs
/// a single CAS — cheap and wait-free, but the endpoint carries
/// `Arc`s, so its deallocation path is a non-RT-side responsibility
/// (the engine retires it through the plan-swap collector).
pub struct PluginEndpoint {
    /// The layer leg — internal; callers never name the layer.
    inner: EndpointInner,
    /// The attach-time capacity contract (scratch ceiling).
    bound: EndpointBounds,
    /// `process_audio`'s bound-shaped input copy target —
    /// `bound.channels × bound.max_frames`, allocated at attach so
    /// the RT path never allocates. Read by `process_audio` once the
    /// public shell lands (endpoint public shell is Task 6).
    #[expect(dead_code)]
    in_scratch: AudioBuffer<f32>,
    /// Reported pipeline latency in frames — `0` for L0,
    /// `lookahead_blocks × activated max_frames` for L1.
    latency_frames: u32,
}

impl PluginEndpoint {
    /// Assembles one endpoint from its layer leg — called by the
    /// attach paths (`InProcessHost::attach_endpoint` for L0; the
    /// composite's `attach_endpoint` builds `EndpointInner::L1`
    /// from `l1::data::EndpointLegParts` for L1).
    // `attach_endpoint` is its only constructor caller — the public
    // mint surface (`PluginHost::prepare_endpoint`) lands in Task 4
    // (`not(test)` because the tests already exercise it). `#[expect]`,
    // not `#[allow]`: once T3/T4 wire the callers the non-test build's
    // unfulfilled expectation fails the lint gate — remove it then.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn new(
        inner: EndpointInner,
        bound: EndpointBounds,
        in_scratch: AudioBuffer<f32>,
        latency_frames: u32,
    ) -> Self {
        Self {
            inner,
            bound,
            in_scratch,
            latency_frames,
        }
    }

    /// The capacity contract this endpoint was attached under.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn bounds(&self) -> EndpointBounds {
        self.bound
    }

    /// The pipeline latency in frames — the value fixed at attach
    /// time (a re-`Activate` may change the true latency; the
    /// reported integer does not track it). Divide by the activated
    /// sample rate for seconds.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn latency_frames(&self) -> u32 {
        self.latency_frames
    }

    /// One processing block — [`PluginHost::process`]'s exact
    /// vocabulary: erased bus views, the per-block event list and an
    /// optional transport snapshot.
    ///
    /// The output-table rows from the module doc apply; every `Err`
    /// leaves `outputs` unchanged.
    ///
    /// **RT contract:** RT-suitable — the L0 leg takes one
    /// `try_lock`; nothing here allocates or blocks.
    ///
    /// [`PluginHost::process`]: crate::PluginHost::process
    ///
    /// # Errors
    ///
    /// [`HostError::Lifecycle`] when the instance is not
    /// `Processing`; [`HostError::Prepare`] when the block violates
    /// the activated config or the bound;
    /// [`HostError::EndpointUnavailable`] while the shared cell is
    /// busy (a non-RT verb mid-flight — retry next block).
    pub fn process(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        self.inner
            .process(self.bound, inputs, outputs, events, transport)
    }
}

/// The layer leg — `PluginEndpoint::process` dispatches here.
/// `L1` is the detached `RtLeg` + arm/return channels assembled
/// from `l1::data::EndpointLegParts` (the `src/l1/` tree stays
/// crate-free, so `endpoint.rs` is where the parts become the
/// endpoint).
// Both variants are constructed only by the `attach_endpoint`
// inners — whose in-crate caller is Task 4's
// `PluginHost::prepare_endpoint` (`not(test)` because the tests
// already exercise it). `#[expect]`, not `#[allow]`: once T4 wires
// the caller the non-test build's unfulfilled expectation fails
// the lint gate — remove it then.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) enum EndpointInner {
    /// L0 in-process: shares the instance's `SendCore` cell with the
    /// host.
    L0(L0Endpoint),
    /// L1 out-of-process: owns the checked-out RT leg outright.
    /// Boxed — the leg's channel ends and staging state are ~10×
    /// the L0 arm's; an attach-time `Box` keeps `EndpointInner`
    /// compact (allocation at attach is non-RT anyway).
    L1(Box<L1Endpoint>),
}

impl EndpointInner {
    /// Variant dispatch — `bound` travels with the call (the
    /// endpoint owns it); the activated contract itself is checked
    /// per variant (L0 = `SendCore.activated` under the lock, L1 =
    /// the armed ring's `frame_capacity`).
    fn process(
        &mut self,
        bound: EndpointBounds,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        match self {
            Self::L0(endpoint) => endpoint.process(bound, inputs, outputs, events, transport),
            Self::L1(endpoint) => endpoint.process(bound, inputs, outputs, events, transport),
        }
    }
}

/// The L0 leg — three `Arc` handles into the `SharedSlot` cell plus
/// this endpoint's own attach epoch. All fields are `Send`/`Sync`
/// handles by construction: `Mutex<SendCore>` is `Sync` because
/// `SendCore` is `Send`, so the endpoint needs no `unsafe impl
/// Send`.
pub(crate) struct L0Endpoint {
    /// The shared plugin/context/activated cell — `try_lock` only.
    pub(crate) core: Arc<Mutex<SendCore>>,
    /// The lock-free lifecycle mirror — same shape as the L1
    /// `rt_state` snapshot (`l1::data::mirror_encode`).
    pub(crate) mirror: Arc<AtomicU8>,
    /// The attach epoch flag — `0` free, `n` epoch `n` attached.
    /// `Drop` returns only its own epoch.
    pub(crate) attached: Arc<AtomicU64>,
    /// The epoch this endpoint claimed at attach.
    pub(crate) epoch: u64,
}

impl L0Endpoint {
    /// The L0 `process` path: mirror fast-skip → `try_lock` →
    /// in-lock mirror re-read → activated gate → `process_host`
    /// under `catch_unwind`.
    ///
    /// The pre-lock mirror read is an optimization only — it can
    /// never observe a half-committed state (every host-side mirror
    /// store happens inside the `core` critical section), but it can
    /// be stale, so the in-lock re-read is authoritative for the
    /// gate decision.
    fn process(
        &mut self,
        bound: EndpointBounds,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        // (1) fast-skip — the terminal rows never touch the mutex.
        let mirror = mirror_decode(self.mirror.load(Ordering::Acquire));
        if mirror.is_failed() {
            return Ok(self.converge_failed(outputs));
        }
        if !mirror.accepts_processing() {
            return Err(process_rejected(mirror));
        }
        // (2) `try_lock` only: `WouldBlock` means a non-RT verb
        // holds the cell — a temporary bypass, `outputs` unchanged.
        // `Poisoned` (any plugin panic unwound through a held
        // guard — a verb-CS hook or the host-side `process` arm,
        // never this endpoint's `catch_unwind`'d call) is a
        // `Failed` observation.
        let mut core = match self.core.try_lock() {
            Ok(core) => core,
            Err(TryLockError::WouldBlock) => return Err(HostError::EndpointUnavailable),
            Err(TryLockError::Poisoned(_)) => return Ok(self.converge_failed(outputs)),
        };
        // (3) in-lock re-read — the authoritative committed state.
        let mirror = mirror_decode(self.mirror.load(Ordering::Acquire));
        if mirror.is_failed() {
            return Ok(self.converge_failed(outputs));
        }
        if !mirror.accepts_processing() {
            return Err(process_rejected(mirror));
        }
        // (4) activated gate — `SendCore.activated` is the
        // lock-protected source of truth; `mirror == Processing`
        // implies `activated.is_some()` by the commit-order
        // invariant (the store and the write share one CS).
        let Some(activated) = core.activated else {
            debug_assert!(false, "Processing mirror implies a stored ProcessConfig");
            return Err(process_rejected(mirror));
        };
        let (frames, channels) = block_dims(inputs, outputs);
        if frames < activated.min_frames() as usize
            || frames > activated.max_frames() as usize
            || channels != bound.channels as usize
        {
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        // (5) `process_host` under `catch_unwind` inside the CS — a
        // plugin panic must never unwind through an RT callback, so
        // it converges here: `Failed` store (the endpoint's CS
        // writer class) + silence + `Ok(CONTINUE)`.
        if let Ok(result) = catch_unwind(AssertUnwindSafe(|| {
            let core = &mut *core;
            core.plugin
                .process_host(inputs, outputs, events, transport, &mut core.context)
        })) {
            Ok(result)
        } else {
            self.mirror
                .store(mirror_encode(LifecycleState::Failed), Ordering::Release);
            silence(outputs);
            Ok(ProcessResult::CONTINUE)
        }
    }

    /// The endpoint-side `Failed` convergence for observations made
    /// *without* the lock: `fetch_max` publishes `Failed`
    /// monotonically — it can never regress a later `Destroyed`
    /// store — then silence + `Ok(CONTINUE)` (the terminal row of
    /// the output table; the endpoint is the mirror's second
    /// writer).
    fn converge_failed(&self, outputs: &mut [HostOutput<'_>]) -> ProcessResult {
        self.mirror
            .fetch_max(mirror_encode(LifecycleState::Failed), Ordering::Release);
        silence(outputs);
        ProcessResult::CONTINUE
    }
}

impl Drop for L0Endpoint {
    /// Returns this endpoint's attach epoch — CAS `epoch → 0`, so a
    /// stale drop (e.g. a delayed collector-thread retirement) can
    /// never clear a newer attachment's epoch.
    fn drop(&mut self) {
        let _ = self
            .attached
            .compare_exchange(self.epoch, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}

/// The L1 leg — the `RtLeg` checked out of `L1Instance` plus the
/// endpoint-side channel ends and the shared mirror `Arc`. It owns
/// nothing else: no `L1Instance`, no child handle, no shm segment,
/// no mutex-guarded proxy state, no I/O thread, no notification
/// receiver — every field is either the leg itself or a `Send`
/// channel/atomic end minted for this attach, so `L1Endpoint` is
/// `Send` by construction (witness pinned in tests).
///
/// - `leg` is `Option` only so `Drop` can move it onto the return
///   channel — it is `Some` for the endpoint's entire live span.
/// - `mirror` is the same `Arc<AtomicU8>` the host's `rt_state`
///   publishes — the endpoint observes `Failed`/lifecycle changes
///   lock-free, exactly as the in-place leg did.
/// - `arm_rx`/`arm_retirer` carry activation generations: arms are
///   drained at the head of `process` and every drained arm —
///   applied or stale — is pushed to the retirer so no
///   `Arc<Staging>` destructor runs on the RT thread.
/// - `return_tx` is the capacity-1 leg-return channel `Drop`
///   pushes into; the next mutable host entry point restores it.
pub(crate) struct L1Endpoint {
    leg: Option<RtLeg>,
    mirror: Arc<AtomicU8>,
    arm_rx: Consumer<Arc<Staging>>,
    arm_retirer: Retirer<Arc<Staging>>,
    return_tx: Producer<RtLeg>,
}

/// At most this many arms are popped per `process` drain pass —
/// the arm channel's capacity (2) plus one slot for a resend that
/// can race the drain.
const ARM_DRAIN_MAX: usize = 3;

impl L1Endpoint {
    /// The L1 `process` path: arm drain → mirror gate → dims
    /// pre-gate → the shared [`RtLeg::process`] data plane. The
    /// data-plane algorithm is not reimplemented — stage input,
    /// push doorbell, check the `next_block`/result horizon, read
    /// out.
    ///
    /// Ordering and the per-row output table:
    /// 1. **arm drain** — up to [`ARM_DRAIN_MAX`] pops; a
    ///    `generation > armed` arm applies via [`RtLeg::arm_apply`]
    ///    (same-generation is the normalized no-op retired
    ///    unapplied; stale is likewise retired). An arm above the
    ///    attach bound is the internal backstop — retired and the
    ///    block converged onto the defined pass-through row
    ///    (`Ok(CONTINUE)`), unreachable while the control-side
    ///    `Activate` gate holds; `Err(Ipc)` never surfaces on the
    ///    endpoint's public face.
    /// 2. **mirror gate** — `Failed` publishes monotonically and
    ///    converges (silence + `Ok(CONTINUE)`); non-`Processing`
    ///    is `Err(Lifecycle)` with `outputs` unchanged.
    /// 3. **dims pre-gate** — the L0 gate's mirror: `frames` above
    ///    the armed ring's capacity (the activated `max_frames`)
    ///    or `channels != bound.channels` violates the contract →
    ///    `Err(Prepare(UnsupportedConfig))` before any staging,
    ///    block-data or IPC can see the bad block. `min_frames`
    ///    is deliberately delegated to the child-side plugin
    ///    contract (the armed `Staging` carries no `min_frames`)
    ///    and the `bound`-clip row is `process_audio`'s (T6).
    /// 4. the [`RtLeg`] data plane — its `Err(Ipc)` (an
    ///    unrepresentable block the pre-gate should have caught)
    ///    is remapped to `Err(Prepare(UnsupportedConfig))` so a
    ///    wire failure never surfaces on the endpoint's face.
    fn process(
        &mut self,
        bound: EndpointBounds,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        let Some(leg) = self.leg.as_mut() else {
            // `Some` for the endpoint's entire live span — `Drop`
            // is the only taker. Unreachable by construction; the
            // dedicated variant keeps an anomaly off `Ipc`.
            return Err(HostError::EndpointUnavailable);
        };
        // (1) the arm drain — bounded, every drained arm retired.
        for _ in 0..ARM_DRAIN_MAX {
            let Ok(arm) = self.arm_rx.try_pop() else {
                break;
            };
            if arm.generation > leg.armed_generation() {
                if arm.frame_capacity > bound.max_frames as usize {
                    // Internal backstop — the control-side
                    // `Activate` gate rejects an over-bound ring
                    // before minting, so this is unreachable
                    // in-protocol. `Err(Ipc)` never surfaces on
                    // the endpoint's public face: retire the arm
                    // and converge the block onto the defined
                    // pass-through row instead.
                    self.arm_retirer.retire(arm);
                    leg.pass_through(outputs);
                    return Ok(ProcessResult::CONTINUE);
                }
                leg.arm_apply(arm, &mut self.arm_retirer);
            } else {
                self.arm_retirer.retire(arm);
            }
        }
        // (2) the mirror gate — the shared `rt_state` atomic.
        let mirror = mirror_decode(self.mirror.load(Ordering::Acquire));
        if mirror.is_failed() {
            // The endpoint is the mirror's second writer class —
            // `fetch_max` publishes `Failed` monotonically (it can
            // never regress a later terminal store), then silence
            // + `Ok(CONTINUE)`.
            self.mirror
                .fetch_max(mirror_encode(LifecycleState::Failed), Ordering::Release);
            silence(outputs);
            return Ok(ProcessResult::CONTINUE);
        }
        if !mirror.accepts_processing() {
            return Err(process_rejected(mirror));
        }
        // (3) the dims pre-gate — the L0 gate's mirror: `frames`
        // must not exceed the armed ring's capacity (the activated
        // `max_frames`) and `channels` must equal `bound.channels`.
        // `min_frames` is deliberately absent — the defined-
        // rejection spec delegates it to the child-side plugin
        // contract (the armed `Staging` carries no `min_frames`),
        // and `process_audio` owns the `out.channels()`/
        // `frames ≤ bound.max_frames` bound-clip row (T6), not
        // this layer.
        let (frames, channels) = block_dims(inputs, outputs);
        if frames > leg.armed_frame_capacity() || channels != bound.channels as usize {
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        // (4) the shared data plane — `Err(Ipc)` for dims is
        // unreachable behind the pre-gate; remap whatever shape it
        // takes so it never reads as a wire failure here.
        match leg.process(mirror, inputs, outputs, events, transport) {
            Err(HostError::Ipc) => Err(HostError::Prepare(PrepareError::UnsupportedConfig)),
            result => result,
        }
    }
}

impl From<EndpointLegParts> for L1Endpoint {
    /// `crate::l1` stays crate-free — the parts bundle names only
    /// `l1`/external types; this is where it becomes the endpoint.
    fn from(parts: EndpointLegParts) -> Self {
        Self {
            leg: Some(parts.leg),
            mirror: parts.mirror,
            arm_rx: parts.arm_rx,
            arm_retirer: parts.arm_retirer,
            return_tx: parts.return_tx,
        }
    }
}

impl Drop for L1Endpoint {
    /// Returns the checked-out leg to the instance — one bounded
    /// `try_push` on the capacity-1 return channel. `Full` means
    /// the channel already holds a leg (reachable only through a
    /// double-drop anomaly): `into_inner` recovers it and it drops
    /// on this thread — bounded channel teardown, never a block —
    /// so `Drop` is safe from any thread, including a collector.
    fn drop(&mut self) {
        let Some(leg) = self.leg.take() else {
            return;
        };
        if let Err(ruviform_rt::TryPushError::Full(leg)) = self.return_tx.try_push(leg) {
            drop(leg);
        }
    }
}

/// The `ChannelLayout` a bound channel count maps to for
/// `in_scratch` — a named layout where one exists, else a custom
/// mask of the low `channels` bits (`channels ≤ WIRE_MAX_CHANNELS`
/// holds already — the bounds check runs before this).
// Used only by `in_scratch` — see its `dead_code` note; the
// `#[expect]` drops with it once T4 wires the public caller.
#[cfg_attr(not(test), expect(dead_code))]
fn bound_layout(channels: u32) -> Result<ChannelLayout, HostError> {
    match channels {
        1 => Ok(ChannelLayout::Mono),
        2 => Ok(ChannelLayout::Stereo),
        4 => Ok(ChannelLayout::Quad),
        6 => Ok(ChannelLayout::FivePointOne),
        8 => Ok(ChannelLayout::SevenPointOne),
        n => ChannelLayout::custom(SpeakerMask::from_bits((1 << n) - 1))
            .map_err(|_| HostError::Prepare(PrepareError::UnsupportedConfig)),
    }
}

/// The bound-shaped `in_scratch` `AudioBuffer` every endpoint
/// carries — `bound.channels × bound.max_frames` at the activated
/// sample rate, allocated at attach so the RT path never
/// allocates. Shared by the L0 and L1 attach paths
/// (`InProcessHost::attach_endpoint` / the composite's
/// `attach_endpoint` L1 arm); any `try_new` failure maps to
/// `Err(Prepare(UnsupportedConfig))`.
// Its callers are the `attach_endpoint` inners — see their
// `dead_code` notes; the `#[expect]` drops once T4 wires the
// public caller.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn in_scratch(
    bounds: EndpointBounds,
    sample_rate: SampleRate,
) -> Result<AudioBuffer<f32>, HostError> {
    bound_layout(bounds.channels).and_then(|layout| {
        AudioBuffer::try_new(BufferSpec {
            layout,
            frames: bounds.max_frames as usize,
            sample_rate,
        })
        .map_err(|_| HostError::Prepare(PrepareError::UnsupportedConfig))
    })
}

/// The non-`Processing` rejection — the same `LifecycleError` shape
/// `InProcessHost::process` returns for an illegal `Process` event.
fn process_rejected(from: LifecycleState) -> HostError {
    HostError::Lifecycle(LifecycleError::IllegalTransition {
        from,
        event: LifecycleEvent::Process,
    })
}

/// The block dimensions under the in-bus-first rule (the same rule
/// the L1 data plane's `block_data` applies): `frames` comes from
/// the first input bus, else the first output bus; `channels` is
/// `min(in, out)` when both buses exist, else whichever exists.
fn block_dims(inputs: &[HostInput<'_>], outputs: &[HostOutput<'_>]) -> (usize, usize) {
    let in_bus = inputs.first();
    let out_bus = outputs.first();
    let frames = in_bus.map_or_else(|| out_bus.map_or(0, BufferView::frames), BufferView::frames);
    let channels = match (in_bus, out_bus) {
        (Some(input), Some(output)) => input.channels().min(output.channels()),
        (Some(input), None) => input.channels(),
        (None, Some(output)) => output.channels(),
        (None, None) => 0,
    };
    (frames, channels)
}

/// Writes silence into every channel of every output bus.
fn silence(outputs: &mut [HostOutput<'_>]) {
    for output in outputs.iter_mut() {
        for channel in 0..output.channels() {
            if let Some(data) = output.channel_mut(channel) {
                data.fill(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_not_impl_any;

    use ruviform_rt::DropCollector;

    use super::*;
    use crate::l1::data::Submit;

    /// Compile-time `Send` witnesses — the endpoint must cross into
    /// RT-side ownership without an `unsafe impl`, so every cell
    /// type it can capture is pinned `Send` by construction.
    /// (`SharedSlot` itself is `host`-internal; its `Send` witness
    /// lives in `host.rs`'s test module.)
    fn expect_send<T: Send>() {}

    #[test]
    fn endpoint_types_are_send() {
        expect_send::<SendCore>();
        expect_send::<L0Endpoint>();
        expect_send::<L1Endpoint>();
        expect_send::<EndpointLegParts>();
        expect_send::<EndpointInner>();
        expect_send::<PluginEndpoint>();
    }

    // The `!Send` negative side of the Send evidence (test-spec §Send
    // 証拠): `expect_send` can't express `!Send` at compile time, so
    // `assert_not_impl_any!` pins the types an endpoint must never
    // capture — an un-synchronized-mutable cell (`RefCell` pins
    // `!Sync`; it is `Send` when `T: Send`), a non-`Send` erased
    // plugin object, and the non-RT-side owners (`L1Instance`,
    // `PluginHost`).
    assert_not_impl_any!(std::rc::Rc<()>: Send, Sync);
    assert_not_impl_any!(std::cell::RefCell<()>: Sync);
    assert_not_impl_any!(Box<dyn crate::ErasedPlugin>: Send);
    assert_not_impl_any!(crate::l1::L1Instance: Send);
    assert_not_impl_any!(crate::PluginHost: Send);

    // ---------------------------------------------------------------
    // L1Endpoint — the detached `RtLeg` arm (Issue #113 task 3).
    // ---------------------------------------------------------------

    /// The shared 8-frame mono input for `run_l1` — stack-fixed so
    /// `assert_no_alloc` windows stay clean.
    const INPUT: [f32; 8] = [0.5_f32; 8];

    /// The host-side channel ends `l1_leg` mints — what
    /// `L1Instance::prepare_endpoint` would keep across the attach.
    struct L1HostEnds {
        /// The arm producer — the `arm_staging` send side.
        arm_tx: Producer<Arc<Staging>>,
        /// The capacity-1 leg-return consumer.
        return_rx: Consumer<RtLeg>,
        /// The arm retirement sink — every drained arm lands here.
        collector: DropCollector<Arc<Staging>>,
        /// The shared lifecycle mirror the endpoint reads.
        mirror: Arc<AtomicU8>,
    }

    /// A manually assembled `L1Endpoint` + the host-side channel
    /// ends an `L1Instance::prepare_endpoint` would keep.
    /// `armed_frames` arms the gen-1 ring; `bound` is the attach
    /// bound the drain checks arms against.
    fn l1_leg(mirror: LifecycleState, armed_frames: u32) -> (L1Endpoint, L1HostEnds) {
        const DEPTH: u32 = 3;
        let (submit_tx, _submit_rx) = ruviform_rt::channel::<Submit>(DEPTH as usize);
        let (_result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let mut leg = RtLeg::new(DEPTH, DEPTH - 1, submit_tx, result_rx);
        leg.arm(Arc::new(Staging::new(DEPTH, armed_frames, 1)));
        // `prepare_endpoint` reserves `last_output` to the bound —
        // mirror that so `arm_apply`'s resize stays in capacity.
        leg.reserve_last_output(armed_frames as usize * 16);
        let (arm_tx, arm_rx) = ruviform_rt::channel::<Arc<Staging>>(2);
        let (arm_retirer, arm_collector) = ruviform_rt::retire_channel::<Arc<Staging>>(4);
        let (return_tx, return_rx) = ruviform_rt::channel::<RtLeg>(1);
        let mirror = Arc::new(AtomicU8::new(mirror_encode(mirror)));
        (
            L1Endpoint {
                leg: Some(leg),
                mirror: Arc::clone(&mirror),
                arm_rx,
                arm_retirer,
                return_tx,
            },
            L1HostEnds {
                arm_tx,
                return_rx,
                collector: arm_collector,
                mirror,
            },
        )
    }

    /// Bit-exact sample gate — `fill` writes exact patterns, so
    /// `to_bits` asserts silence/untouched without float
    /// comparisons.
    fn all_eq(output: &[f32], value: f32) -> bool {
        output
            .iter()
            .all(|sample| sample.to_bits() == value.to_bits())
    }

    /// One `L1Endpoint::process` block over mono planar views —
    /// the caller supplies both buffers so the view plumbing is
    /// allocation-free inside `assert_no_alloc` windows.
    fn run_l1(
        endpoint: &mut L1Endpoint,
        bound: EndpointBounds,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<ProcessResult, HostError> {
        use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
        let in_list = [input];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut *output];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        endpoint.process(
            bound,
            &inputs,
            &mut outputs,
            EventList::empty(u32::try_from(input.len()).unwrap()),
            None,
        )
    }

    /// The endpoint `process` path: the `Processing` mirror passes
    /// the gate, the block stages + doorbells, the absent result
    /// resolves to the defined pass-through (silence before the
    /// first matched block) and `Ok(CONTINUE)`.
    #[test]
    fn l1_endpoint_process_passes_through() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, _ends) = l1_leg(LifecycleState::Processing, 512);
        let mut output = [1.0_f32; 8];
        assert_eq!(
            run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap(),
            ProcessResult::CONTINUE
        );
        assert!(all_eq(&output, 0.0));
    }

    /// `Failed` mirror observation converges: `fetch_max` publishes
    /// `Failed` monotonically, silence is written and `Ok(CONTINUE)`
    /// returns — the terminal row, never an `Err`.
    #[test]
    fn l1_endpoint_failed_converges() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, ends) = l1_leg(LifecycleState::Failed, 512);
        let mut output = [9.0_f32; 8];
        assert_eq!(
            run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap(),
            ProcessResult::CONTINUE
        );
        assert!(all_eq(&output, 0.0));
        assert_eq!(
            ends.mirror.load(Ordering::Acquire),
            mirror_encode(LifecycleState::Failed)
        );
    }

    /// A non-`Processing` mirror is the defined `Err(Lifecycle)`
    /// with `outputs` untouched — the same rejection L0's endpoint
    /// returns.
    #[test]
    fn l1_endpoint_non_processing_rejects() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, _ends) = l1_leg(LifecycleState::Activated, 512);
        let mut output = [9.0_f32; 8];
        assert!(matches!(
            run_l1(&mut endpoint, bound, &INPUT, &mut output),
            Err(HostError::Lifecycle(_))
        ));
        assert!(all_eq(&output, 9.0));
    }

    /// The dims pre-gate: a block above the armed ring's capacity
    /// is `Err(Prepare(UnsupportedConfig))` before staging or IPC —
    /// and `channels != bound.channels` rejects the same way (the
    /// L0 gate's mirror; `min_frames` stays delegated to the
    /// child-side plugin contract per the defined-rejection spec).
    /// The leg's own `Err(Ipc)` shapes (e.g. a channel count the
    /// wire cannot express) are remapped to the same public
    /// `Prepare` surface, never `Ipc`.
    #[test]
    fn l1_endpoint_dims_gate_never_surfaces_ipc() {
        use ruviform_audio_core::PlanarBufferViewMut;
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, _ends) = l1_leg(LifecycleState::Processing, 512);
        // `frames > armed frame_capacity` — the activated contract.
        let input_over = [0.5_f32; 1024];
        let mut output = [9.0_f32; 8];
        assert!(matches!(
            run_l1(&mut endpoint, bound, &input_over, &mut output),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig))
        ));
        assert!(all_eq(&output, 9.0));
        // `channels != bound.channels` — a stereo block against a
        // mono bound is the same contract violation L0 rejects.
        let stereo = [[0.5_f32; 8], [0.5_f32; 8]];
        let in_list: Vec<&[f32]> = stereo.iter().map(|c| &c[..]).collect();
        let input_view = ruviform_audio_core::PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut left = [9.0_f32; 8];
        let mut right = [9.0_f32; 8];
        let mut out_list = [&mut left[..], &mut right[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        assert!(matches!(
            endpoint.process(bound, &inputs, &mut outputs, EventList::empty(8), None),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig))
        ));
        assert!(all_eq(&left, 9.0) && all_eq(&right, 9.0));
    }

    /// `Err(Ipc)` non-surface: an inputs-only block of 17 channels
    /// is unrepresentable on the wire (`channels` comes from the
    /// only present bus) — the leg's `block_data` errs `Ipc`, the
    /// endpoint remaps it to `Err(Prepare(UnsupportedConfig))`.
    #[test]
    fn l1_endpoint_wire_unrepresentable_maps_to_prepare() {
        use ruviform_audio_core::PlanarBufferView;
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 17,
        };
        let (mut endpoint, _ends) = l1_leg(LifecycleState::Processing, 512);
        let input = [0.5_f32; 8];
        let in_list: Vec<&[f32]> = vec![&input[..]; 17];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut outputs: [HostOutput<'_>; 0] = [];
        assert!(matches!(
            endpoint.process(bound, &inputs, &mut outputs, EventList::empty(8), None),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig))
        ));
    }

    /// The arm drain's generation gate: the same-generation seed
    /// arm is a normalized no-op retired unapplied; a newer arm
    /// applies via `arm_apply`; a stale arm is likewise retired —
    /// every drained arm reaches the collector exactly once.
    #[test]
    fn l1_endpoint_arm_drain_generation_gate() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, mut ends) = l1_leg(LifecycleState::Processing, 512);
        let mut output = [0.0_f32; 8];
        // Same-generation seed — retired unapplied.
        let seed = Arc::new(Staging::new(3, 512, 1));
        assert!(ends.arm_tx.try_push(seed).is_ok());
        run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_generation(), 1);
        assert_eq!(ends.collector.drain(), 1);
        // Newer generation — applies; the displaced ring retires.
        let newer = Arc::new(Staging::new(3, 256, 2));
        assert!(ends.arm_tx.try_push(newer).is_ok());
        run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_generation(), 2);
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_frame_capacity(), 256);
        assert_eq!(ends.collector.drain(), 1);
        // Stale (not-newer) generation — retired unapplied.
        let stale = Arc::new(Staging::new(3, 64, 2));
        assert!(ends.arm_tx.try_push(stale).is_ok());
        run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_generation(), 2);
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_frame_capacity(), 256);
        assert_eq!(ends.collector.drain(), 1);
    }

    /// The over-bound arm backstop: an arm whose `frame_capacity`
    /// exceeds the attach bound is retired and the block converges
    /// onto the defined pass-through (`Ok(CONTINUE)` — `Err(Ipc)`
    /// never surfaces on the endpoint's public face). The
    /// control-side `Activate` gate makes it unreachable anyway.
    #[test]
    fn l1_endpoint_over_bound_arm_passes_through() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, mut ends) = l1_leg(LifecycleState::Processing, 512);
        let oversized = Arc::new(Staging::new(3, 1024, 2));
        assert!(ends.arm_tx.try_push(oversized).is_ok());
        // No received block yet — pass-through is silence.
        let mut output = [7.0_f32; 8];
        assert_eq!(
            run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap(),
            ProcessResult::CONTINUE
        );
        assert!(all_eq(&output, 0.0));
        assert_eq!(ends.collector.drain(), 1);
        // The leg's armed ring is untouched.
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_generation(), 1);
    }

    /// `Drop` returns the checked-out leg through the capacity-1
    /// channel — the next host-side drain would restore it.
    #[test]
    fn l1_endpoint_drop_returns_leg() {
        let (endpoint, mut ends) = l1_leg(LifecycleState::Processing, 512);
        drop(endpoint);
        let leg = ends
            .return_rx
            .try_pop()
            .expect("leg returned through the channel");
        assert_eq!(leg.armed_generation(), 1);
    }

    /// Return-channel `Full` recovery: if the channel already holds
    /// a leg (the double-drop anomaly), `Drop` recovers via
    /// `into_inner` and drops on the thread — the queued leg is the
    /// only one the channel ever held.
    #[test]
    fn l1_endpoint_drop_full_recovers_on_thread() {
        const DEPTH: u32 = 3;
        let (mut endpoint, mut ends) = l1_leg(LifecycleState::Processing, 512);
        // Pre-fill the capacity-1 channel through the endpoint's own
        // producer with a *different* leg — generation-tagged so the
        // test can tell the two apart.
        let (submit_tx, _submit_rx) = ruviform_rt::channel::<Submit>(DEPTH as usize);
        let (_result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let mut spare = RtLeg::new(DEPTH, DEPTH - 1, submit_tx, result_rx);
        spare.arm(Arc::new(Staging::new(DEPTH, 64, 5)));
        assert!(endpoint.return_tx.try_push(spare).is_ok());
        drop(endpoint);
        // The spare leg is what the channel held — the endpoint's
        // own leg hit `Full` and dropped on this thread.
        let leg = ends.return_rx.try_pop().expect("the pre-filled leg");
        assert_eq!(leg.armed_generation(), 5);
        assert!(ends.return_rx.try_pop().is_err());
    }

    /// Issue #22 lane: the detached `RtLeg` process path through
    /// `L1Endpoint` — arm drain, mirror gate, dims pre-gate, stage/
    /// doorbell/pass-through — must not allocate. Fixture, channels
    /// and warm-up all sit outside the window.
    #[test]
    fn rt_l1_endpoint_process_no_alloc() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, _ends) = l1_leg(LifecycleState::Processing, 512);
        let mut output = [0.0_f32; 8];
        // Warm-up outside the window.
        run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..64 {
                run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
            }
        });
    }

    /// Issue #22 lane: the arm drain inside `process` — `try_pop`,
    /// the generation gate and `arm_apply`'s in-capacity resize +
    /// retire `try_push` — must not allocate.
    #[test]
    fn rt_l1_endpoint_arm_drain_no_alloc() {
        let bound = EndpointBounds {
            max_frames: 512,
            channels: 1,
        };
        let (mut endpoint, mut ends) = l1_leg(LifecycleState::Processing, 512);
        let mut output = [0.0_f32; 8];
        // Warm-up outside the window.
        run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
        let newer = Arc::new(Staging::new(3, 512, 2));
        assert!(ends.arm_tx.try_push(newer).is_ok());
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                run_l1(&mut endpoint, bound, &INPUT, &mut output).unwrap();
            }
        });
        assert_eq!(endpoint.leg.as_ref().unwrap().armed_generation(), 2);
        assert_eq!(ends.collector.drain(), 1);
    }
}
