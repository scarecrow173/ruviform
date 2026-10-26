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

use ruviform_audio_core::AudioBuffer;
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, HostError, HostInput, HostOutput, LifecycleError,
    LifecycleEvent, LifecycleState, PrepareError, ProcessResult, TransportInfo,
};

use crate::host::SendCore;
use crate::l1::data::{mirror_decode, mirror_encode};

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
    /// attach paths (`InProcessHost::attach_endpoint` for L0; the L1
    /// inner will build `EndpointInner::L1` from
    /// `l1::data::EndpointLegParts` in Task 3).
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
/// `L1` lands in Task 3 (moved `RtLeg` + arm/return channels,
/// assembled from `l1::data::EndpointLegParts` — the `src/l1/` tree
/// stays crate-free, so `endpoint.rs` is where host-side types are
/// named).
// The `L0` variant is constructed only by `attach_endpoint` — whose
// in-crate caller is Task 4's `PluginHost::prepare_endpoint`
// (`not(test)` because the tests already exercise it). `#[expect]`,
// not `#[allow]`: once T3/T4 wire the callers the non-test build's
// unfulfilled expectation fails the lint gate — remove it then.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) enum EndpointInner {
    /// L0 in-process: shares the instance's `SendCore` cell with the
    /// host.
    L0(L0Endpoint),
}

impl EndpointInner {
    /// Variant dispatch — `bound` travels with the call (the
    /// endpoint owns it); the activated contract itself is checked
    /// per variant (L0 = `SendCore.activated` under the lock).
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

    use super::*;

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
}
