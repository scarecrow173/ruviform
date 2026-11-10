//! [`InProcessHost`] — the L0 backend: factory registry, instance
//! slots, command dispatch and the direct-call RT `process` path.
//!
//! Issue #113 splits each instance's storage into [`InstanceSlot`]:
//! the original `Plain` form (plugin owned outright) and the
//! `Shared` form for factories that opt in via
//! [`PluginFactory::create_sendable`]. A `Shared` slot keeps the
//! plugin and its processing context in a shared cell an RT
//! endpoint may `try_lock`, plus a lock-free lifecycle mirror and
//! epoch-guarded attach bookkeeping — every non-RT verb still
//! reaches the plugin through the pinned three-tier lock protocol
//! (`docs/designs/engine-plugin-host-integration.md`).

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

use ruviform_plugin_core::wire::{WIRE_MAX_CHANNELS, WIRE_MAX_FRAMES};
use ruviform_plugin_core::{
    BusDescriptor, Event, EventList, Lifecycle, LifecycleError, LifecycleEvent, LifecycleState,
    PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

use crate::endpoint::{EndpointBounds, EndpointInner, L0Endpoint, PluginEndpoint};
use crate::l1::data::{mirror_decode, mirror_encode};
use crate::{
    ErasedPlugin, GuiHandle, HostCommand, HostError, HostInput, HostOutput, HostReply,
    PluginFactory, PluginHandle,
};

/// How one live instance's plugin/context storage is held — chosen
/// at `Instantiate` by the factory's `create_sendable` capability.
enum InstanceSlot {
    /// Direct form: the plugin is owned outright — `plugin_mut` can
    /// lend it, `process` calls it with no synchronization. Not
    /// endpoint-capable.
    Plain {
        /// The boxed plugin instance.
        plugin: Box<dyn ErasedPlugin>,
        /// Opaque per-instance context forwarded to
        /// `PluginProcessor`.
        context: ProcessContext,
    },
    /// Shared form — the plugin lives in [`SharedSlot`]'s
    /// mutex-guarded cell so a detached RT endpoint can reach it
    /// through `try_lock`. Every host verb runs the three-tier lock
    /// protocol; `plugin_mut` returns `None` (a `&mut` cannot be
    /// lent out of a `Mutex`).
    Shared(SharedSlot),
}

/// The `Shared` slot's endpoint-facing shared cell plus host-side
/// attach bookkeeping. (Named `SharedSlot`, not `Shared`, to avoid
/// colliding with `l1::Shared` — the L1 proxy state.)
struct SharedSlot {
    /// The mutex-guarded core the host and the endpoint share —
    /// host verbs take `core.lock()` (non-RT), the endpoint only
    /// ever `try_lock`s it. Any plugin panic unwinding through a
    /// held guard poisons the cell — a verb-CS hook *or* the
    /// host-side `process` Shared arm, which deliberately has no
    /// `catch_unwind` (the endpoint leg does). A poisoned lock is a
    /// `Failed` observation.
    core: Arc<Mutex<SendCore>>,
    /// The lock-free lifecycle mirror (same shape as the L1
    /// `rt_state` snapshot): stored `Release` inside the `core`
    /// critical section after every transition commit, read
    /// `Acquire` lock-free — `try_lock` observers only ever see
    /// fully committed state. The endpoint is the mirror's second
    /// writer via monotonic `fetch_max(Failed)`.
    mirror: Arc<AtomicU8>,
    /// Endpoint attach epoch — `0` free, `n` epoch `n` attached.
    /// CAS-guarded both ways so a stale endpoint `Drop` can never
    /// clear a newer attachment.
    attached: Arc<AtomicU64>,
    /// Host-side bookkeeping — the latest attach's bounds, consulted
    /// by the re-`Activate` bound gate. `attached != 0` is the
    /// authoritative liveness signal; this field is comparison data
    /// only.
    bound: Option<EndpointBounds>,
    /// Host-side epoch issuer — monotonic, starts at 1, bumped per
    /// claimed attach.
    next_epoch: u64,
}

/// The `Shared` slot's lock-protected contents — everything the RT
/// side reads under `try_lock` lives here. `Send` by construction.
pub(crate) struct SendCore {
    /// The `Send`-capable erased plugin (`create_sendable`).
    pub(crate) plugin: Box<dyn ErasedPlugin + Send>,
    /// Per-instance context — inside the shared cell so the
    /// endpoint can forward it to `process_host`.
    pub(crate) context: ProcessContext,
    /// The activated `ProcessConfig` — `Some` while the mirror is
    /// `Activated`/`Processing` (written on `Activate` commit,
    /// cleared on `Deactivate`/`Destroy`, always inside the `core`
    /// critical section — the `mirror ∈ {Activated, Processing}
    /// ⟹ activated.is_some()` invariant's home).
    pub(crate) activated: Option<ProcessConfig>,
}

/// One live instance slot: the slot storage plus the host-side
/// bookkeeping the command surface drives.
struct Instance {
    /// How the plugin/context are held — `Plain` or `Shared`.
    slot: InstanceSlot,
    /// The bookkeeping lifecycle — the legality/verb authority. For
    /// `Shared` slots it is also the `&mut`-entry-point half of the
    /// `Failed` observation union (the endpoint's
    /// `fetch_max(Failed)` can reach the mirror ahead of it — the
    /// convergence happens at the next `&mut` entry point).
    lifecycle: Lifecycle,
    /// Stamped into [`PluginHandle`]s; bumped on reuse so a stale
    /// handle is rejected instead of aliasing a new instance.
    generation: u16,
}

impl Instance {
    fn new(slot: InstanceSlot, generation: u16) -> Self {
        Self {
            slot,
            lifecycle: Lifecycle::new(),
            generation,
        }
    }
}

/// The `Shared` slot's only host-side mirror-write form: publish
/// the just-committed bookkeeping state. Called inside the `core`
/// critical section — `try_lock` observers can never see a
/// half-committed mirror (the endpoint's `fetch_max(Failed)` is the
/// sole out-of-CS writer, and it is monotonic).
fn set_state(slot: &SharedSlot, state: LifecycleState) {
    slot.mirror.store(mirror_encode(state), Ordering::Release);
}

/// The `Failed` convergence every `Shared` observation funnels
/// into: commit [`LifecycleEvent::Failed`] into the bookkeeping
/// lifecycle (idempotent — `Failed` re-reports skip the illegal
/// re-transition) and return the defined rejection. The mirror is
/// not written here: host-side mirror stores happen only inside
/// the `core` critical section (unreachable on a poisoned lock —
/// the exact case this converges), and the endpoint's
/// `fetch_max(Failed)` is the design's single out-of-CS writer.
fn converge_failed(lifecycle: &mut Lifecycle) -> HostError {
    if !lifecycle.state().is_failed() {
        // Legal from every live pre-terminal state.
        let _ = lifecycle.transition(LifecycleEvent::Failed);
    }
    HostError::Failed
}

/// The bus-enumeration result [`InProcessHost::buses`] and
/// [`crate::PluginHost::buses`] return — an honest "reported" vs
/// "unavailable" distinction.
///
/// `Reported` carries the plugin's declared [`BusDescriptor`]s in
/// the adapter's declared order. `Unavailable` means the layer
/// cannot enumerate buses (an out-of-process L1 instance has no
/// wire verb for it yet) — callers must not infer a bus layout
/// from it.
///
/// **RT contract:** non-RT only — `Reported` owns a vec.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum BusReport {
    /// The plugin's declared bus table (possibly empty when the
    /// plugin does not declare one — an empty report is "declared
    /// none", distinct from `Unavailable`).
    Reported(Vec<BusDescriptor>),
    /// Bus enumeration is unavailable at this layer.
    Unavailable,
}

/// The plugin→host report drain result
/// [`InProcessHost::drain_output_reports`] and
/// [`crate::PluginHost::drain_output_reports`] return — an honest
/// "drained" vs "unavailable" distinction, the same shape
/// [`BusReport`] gives `buses`.
///
/// `Drained` means the instance's pending reports were appended to
/// the caller's buffer — possibly zero reports; callers measure the
/// drain by the `out` length delta. `Unavailable` means the layer
/// cannot surface plugin→host reports at all (an out-of-process L1
/// instance has no wire verb for them yet, and video slots have no
/// report surface) — callers must not infer report silence from it.
///
/// **RT contract:** non-RT only — returned by value.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ReportDrain {
    /// The instance's pending reports were drained into `out`.
    Drained,
    /// Report draining is unavailable at this layer.
    Unavailable,
}

/// The L0 host: executes [`HostCommand`]s in-process against
/// registered factories and exposes the direct-call RT processing
/// path.
///
/// Per the plugin-isolation design (`docs/designs/plugin-isolation.md`)
/// L0 is the default path: callers use the same command surface an L1
/// out-of-process backend will serve later, and `process` stays a
/// direct `&mut dyn PluginProcessor` call — no IPC, no blocking.
///
/// **RT contract:** command dispatch (`execute`, `register_factory`)
/// is **non-RT only**. [`InProcessHost::process`] and
/// [`InProcessHost::plugin_mut`] are the RT surface — allocation-free
/// and non-blocking by construction.
pub struct InProcessHost {
    factories: Vec<Box<dyn PluginFactory>>,
    /// `None` marks a destroyed slot available for reuse.
    instances: Vec<Option<Instance>>,
    /// Per-slot handle generation — survives `destroy`, so a reused
    /// slot's next handle bumps past every stale one instead of
    /// aliasing it.
    generations: Vec<u16>,
    /// Indices of reusable `None` slots.
    free_slots: Vec<u32>,
}

impl Default for InProcessHost {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessHost {
    /// An empty host with no registered factories.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
            instances: Vec::new(),
            generations: Vec::new(),
            free_slots: Vec::new(),
        }
    }

    /// Registers one plugin type for `Scan`/`Instantiate`.
    ///
    /// **RT contract:** non-RT only.
    pub fn register_factory(&mut self, factory: Box<dyn PluginFactory>) {
        self.factories.push(factory);
    }

    /// Executes one non-RT command against the registry and the
    /// instance it names.
    ///
    /// **RT contract:** non-RT only — may allocate and call non-RT
    /// plugin hooks.
    ///
    /// # Errors
    ///
    /// [`HostError`] — see each variant for the failure condition.
    pub fn execute(&mut self, command: HostCommand) -> Result<HostReply, HostError> {
        match command {
            HostCommand::Scan => Ok(HostReply::Scanned(
                self.factories
                    .iter()
                    .map(|factory| factory.descriptor())
                    .collect(),
            )),
            HostCommand::Instantiate { plugin_id } => self.instantiate(&plugin_id),
            HostCommand::Initialize { handle } => self.initialize(handle),
            HostCommand::Activate { handle, config } => self.activate(handle, &config),
            HostCommand::StartProcessing { handle } => self.start_processing(handle),
            HostCommand::StopProcessing { handle } => self.stop_processing(handle),
            HostCommand::Deactivate { handle } => self.deactivate(handle),
            HostCommand::Destroy { handle } => self.destroy(handle),
            HostCommand::SaveState { handle } => self.save_state(handle),
            HostCommand::RestoreState { handle, bytes } => self.restore_state(handle, &bytes),
            HostCommand::OpenGui { handle, parent } => self.open_gui(handle, parent),
            HostCommand::CloseGui { handle } => self.close_gui(handle),
        }
    }

    /// Direct access to an instance for the L0 RT path (e.g. bespoke
    /// [`ErasedPlugin::process_host`] dispatch).
    ///
    /// Returns `None` for an invalid handle, and always `None` for a
    /// `Shared` slot — a `&mut` cannot be lent out of the shared
    /// mutex (the same defined shape `PluginHost` gives L1 slots);
    /// callers must only invoke processing while [`lifecycle_state`]
    /// reports `Processing` — or use [`InProcessHost::process`],
    /// which checks it for them.
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking.
    ///
    /// [`lifecycle_state`]: InProcessHost::lifecycle_state
    pub fn plugin_mut(&mut self, handle: PluginHandle) -> Option<&mut dyn ErasedPlugin> {
        let instance = self
            .instances
            .get_mut(handle.slot() as usize)
            .and_then(Option::as_mut)
            .filter(|instance| instance.generation == handle.generation())?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => Some(&mut **plugin as &mut dyn ErasedPlugin),
            // `Mutex`-guarded — no `&mut` lend-out (L1-parity rule).
            InstanceSlot::Shared(_) => None,
        }
    }

    /// The instance's current lifecycle state, `None` for an invalid
    /// handle.
    ///
    /// For a `Shared` slot this reads the lock-free mirror only —
    /// never `core.lock()` — so it reports the `Failed` observation
    /// union (an endpoint's `fetch_max(Failed)` is visible before
    /// the bookkeeping lifecycle catches it up).
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking.
    #[must_use]
    pub fn lifecycle_state(&self, handle: PluginHandle) -> Option<LifecycleState> {
        let instance = self
            .instances
            .get(handle.slot() as usize)
            .and_then(Option::as_ref)
            .filter(|instance| instance.generation == handle.generation())?;
        Some(match &instance.slot {
            InstanceSlot::Plain { .. } => instance.lifecycle.state(),
            InstanceSlot::Shared(slot) => mirror_decode(slot.mirror.load(Ordering::Acquire)),
        })
    }

    /// The RT processing path: validates `Processing` state (feeding
    /// the lifecycle `Process` event) then calls the plugin's erased
    /// [`ErasedPlugin::process_host`] directly.
    ///
    /// This is the L0 branch of the isolation design — a plain
    /// in-process call, no IPC and no blocking. `inputs`/`outputs`
    /// carry one erased view per enabled bus; the caller builds the
    /// slices (stack array or reusable scratch — nothing here
    /// allocates). On a `Shared` slot the plugin call goes through
    /// `core.try_lock()` — never a blocking `lock` on the RT path —
    /// so a non-RT verb holding the cell is the defined busy bypass.
    ///
    /// Plugin panics are *not* caught here — unlike the endpoint's
    /// `process`, this direct-call path has no `catch_unwind`, so a
    /// panicking plugin propagates to the caller. RT callers that
    /// need panic isolation must use the endpoint path instead. On a
    /// `Shared` slot a propagated panic additionally unwinds through
    /// the held `core` guard and poisons the cell — the next
    /// observation converges to `Failed`. Deliberate asymmetry vs
    /// `Plain`: a panicking plugin must not be re-entered.
    ///
    /// **RT contract:** RT-suitable — the lifecycle transition is a
    /// `const` table lookup; nothing here allocates or blocks.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::Lifecycle`] when the instance is not
    /// `Processing`; [`HostError::Failed`] on the `Failed`
    /// observation union; [`HostError::EndpointUnavailable`] while a
    /// `Shared` slot's cell is busy.
    pub fn process(
        &mut self,
        handle: PluginHandle,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, context } => {
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Process)
                    .map_err(HostError::Lifecycle)?;
                Ok(plugin.process_host(inputs, outputs, events, transport, context))
            }
            InstanceSlot::Shared(slot) => {
                // `Failed` union — mirror first, then the in-lock
                // re-read below.
                if mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed() {
                    return Err(converge_failed(&mut instance.lifecycle));
                }
                // `try_lock` only — a held lock is a non-RT verb
                // mid-flight: temporary bypass, `outputs` unchanged.
                // Poisoned is a `Failed` observation.
                let mut core = match slot.core.try_lock() {
                    Ok(core) => core,
                    Err(TryLockError::WouldBlock) => {
                        return Err(HostError::EndpointUnavailable);
                    }
                    Err(TryLockError::Poisoned(_)) => {
                        return Err(converge_failed(&mut instance.lifecycle));
                    }
                };
                // In-lock mirror re-read — authoritative committed
                // state now that the CS is held.
                if mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed() {
                    return Err(converge_failed(&mut instance.lifecycle));
                }
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Process)
                    .map_err(HostError::Lifecycle)?;
                set_state(slot, instance.lifecycle.state());
                let core = &mut *core;
                Ok(core
                    .plugin
                    .process_host(inputs, outputs, events, transport, &mut core.context))
            }
        }
    }

    /// The attach-side lifecycle gate, shared by the public prelude
    /// ([`PluginHost::prepare_endpoint`](crate::PluginHost::prepare_endpoint)
    /// runs it for its own state step) and the inner
    /// [`InProcessHost::attach_endpoint`] — the design's rejection
    /// order puts state before every other check.
    ///
    /// `Failed` — the union observation (`Shared` reads the mirror,
    /// so an endpoint's `fetch_max(Failed)` is seen ahead of the
    /// bookkeeping lifecycle, **plus** the `core` poison flag,
    /// which covers a panicked plugin call that unwound before the
    /// mirror could move) — converges the bookkeeping lifecycle and
    /// reports `Err(HostError::Failed)`; anything below `Activated`
    /// (`Created`/`Initialized`/an anomalous terminal observation)
    /// is `Err(HostError::Lifecycle)`, the same shape a rejected
    /// `Process` event takes.
    ///
    /// **RT contract:** non-RT only — a bookkeeping transition may
    /// run; no locks are taken.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::Failed`] on the `Failed` union; and
    /// [`HostError::Lifecycle`] below `Activated`.
    pub(crate) fn check_attachable_state(&mut self, handle: PluginHandle) -> Result<(), HostError> {
        let instance = self.instance_mut(handle)?;
        // For `Shared` the mirror is the observation union (the
        // endpoint may have converged it to `Failed` ahead of the
        // bookkeeping lifecycle); `Plain` reads the lifecycle
        // itself.
        let observed = match &instance.slot {
            // The union's poison half: a panicked plugin call that
            // unwound through a held `core` guard (a verb-CS hook
            // or the host-side `process` arm) leaves the mirror at
            // its last committed store, so `is_poisoned` is the
            // same `Failed` observation the inner `lock()` `Err`
            // arm and `destroy` make — a non-locking flag read, so
            // "no locks in the prelude" stays true.
            InstanceSlot::Shared(slot) if slot.core.is_poisoned() => LifecycleState::Failed,
            InstanceSlot::Shared(slot) => mirror_decode(slot.mirror.load(Ordering::Acquire)),
            InstanceSlot::Plain { .. } => instance.lifecycle.state(),
        };
        match observed {
            LifecycleState::Activated | LifecycleState::Processing => Ok(()),
            LifecycleState::Failed => Err(converge_failed(&mut instance.lifecycle)),
            state => Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: state,
                event: LifecycleEvent::Process,
            })),
        }
    }

    /// The attach-side occupancy gate — the public prelude's
    /// presence check for the L0 route
    /// ([`PluginHost::prepare_endpoint`](crate::PluginHost::prepare_endpoint)
    /// runs it before the uniform bounds check so an occupied or
    /// incapable slot reports `EndpointUnavailable` ahead of any
    /// bounds violation).
    ///
    /// Both `Err(HostError::EndpointUnavailable)` reasons live here:
    /// a `Plain` slot (a non-`Send` factory — never endpoint-
    /// capable) and a `Shared` slot whose epoch flag is already
    /// claimed (`attached != 0` — the single-endpoint rule). The
    /// read is one atomic load; the inner attach's epoch CAS stays
    /// the authoritative claim (this is a pre-check, not the attach
    /// itself).
    ///
    /// **RT contract:** non-RT only by call site — the body is a
    /// lock-free atomic read.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::EndpointUnavailable`] for a `Plain` slot or a
    /// `Shared` slot with a live endpoint.
    pub(crate) fn check_endpoint_free(&self, handle: PluginHandle) -> Result<(), HostError> {
        let instance = self
            .instances
            .get(handle.slot() as usize)
            .and_then(Option::as_ref)
            .filter(|instance| instance.generation == handle.generation())
            .ok_or(HostError::InvalidHandle)?;
        match &instance.slot {
            InstanceSlot::Plain { .. } => Err(HostError::EndpointUnavailable),
            InstanceSlot::Shared(slot) => {
                if slot.attached.load(Ordering::Acquire) == 0 {
                    Ok(())
                } else {
                    Err(HostError::EndpointUnavailable)
                }
            }
        }
    }

    /// The L0 inner of `PluginHost::prepare_endpoint` — attaches one
    /// RT endpoint to a live `Shared` instance.
    ///
    /// Rejection order is pinned by the design: **(1)** state — the
    /// `Failed` observation union converges to `Err(Failed)`,
    /// anything below `Activated` is `Err(Lifecycle)`; **(2)** slot
    /// shape — a `Plain` slot is `Err(EndpointUnavailable)`;
    /// **(3)** the activated-layer bounds check (under `core.lock()`
    /// — `SendCore.activated` is its source): non-zero,
    /// `≤ WIRE_MAX_*` and `≥ activated.max_frames`, violations are
    /// `Err(Prepare(UnsupportedConfig))`; **(4)** the epoch CAS
    /// attach — non-zero `attached` is `Err(EndpointUnavailable)`;
    /// **(5)** `in_scratch` preallocation — `AudioBuffer::try_new`
    /// failure maps to `Err(Prepare(UnsupportedConfig))` and
    /// releases the epoch claim. A rejected attach consumes nothing.
    ///
    /// Order note: the inner sequence runs the bounds check *before*
    /// the occupancy CAS — deliberately the inverse of the public
    /// prelude order (state → presence/slot → bounds) that Task 4's
    /// `PluginHost::prepare_endpoint` must enforce, where an attach
    /// on an occupied slot reports `Err(EndpointUnavailable)` even
    /// when the bounds are also invalid. Inner-side, bounds-first
    /// means a bounds-rejected attach never touched `attached` —
    /// "a rejected attach consumes nothing" needs no release step.
    ///
    /// **RT contract:** non-RT only — allocates and takes
    /// `core.lock()` (the caller-side prepare step).
    ///
    /// # Errors
    ///
    /// [`HostError`] — per the pinned order above.
    pub(crate) fn attach_endpoint(
        &mut self,
        handle: PluginHandle,
        bounds: EndpointBounds,
    ) -> Result<PluginEndpoint, HostError> {
        // (1) state — the same gate the public prelude runs
        // (`PluginHost::prepare_endpoint` calls it for its own (1)
        // step; this repeat is the inner layer's defense-in-depth —
        // a state rejection always precedes every other check).
        self.check_attachable_state(handle)?;
        let instance = self.instance_mut(handle)?;
        // (2) slot shape — `Plain` instances are not endpoint-capable.
        let InstanceSlot::Shared(slot) = &mut instance.slot else {
            return Err(HostError::EndpointUnavailable);
        };
        // (3) activated-layer bounds — `SendCore.activated` is only
        // readable under `core.lock()`; like every verb critical
        // section, the lock re-checks the `Failed` union first.
        let activated = {
            let Ok(core) = slot.core.lock() else {
                return Err(converge_failed(&mut instance.lifecycle));
            };
            if mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed() {
                return Err(converge_failed(&mut instance.lifecycle));
            }
            let Some(activated) = core.activated else {
                // `Activated`/`Processing` implies
                // `activated.is_some()` — unreachable in-protocol; a
                // defined rejection, not a panic.
                debug_assert!(false, "activated lifecycle implies a stored ProcessConfig");
                return Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
                    from: instance.lifecycle.state(),
                    event: LifecycleEvent::Process,
                }));
            };
            activated
        };
        if bounds.max_frames == 0
            || bounds.channels == 0
            || bounds.max_frames > WIRE_MAX_FRAMES
            || bounds.channels > WIRE_MAX_CHANNELS
            || bounds.max_frames < activated.max_frames()
        {
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        // (4) epoch CAS — the single-endpoint rule.
        let epoch = slot.next_epoch;
        if slot
            .attached
            .compare_exchange(0, epoch, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(HostError::EndpointUnavailable);
        }
        // The counter may skip values — a post-claim rejection (the
        // `in_scratch` failure below) releases the claim but keeps
        // the bump. Epochs are opaque; the gap is harmless.
        slot.next_epoch += 1;
        // (5) bound-shaped `in_scratch` preallocation — any failure
        // releases the just-claimed epoch (a rejected attach consumes
        // nothing).
        let in_scratch = match crate::endpoint::in_scratch(bounds, activated.sample_rate()) {
            Ok(scratch) => scratch,
            Err(error) => {
                slot.attached.store(0, Ordering::Release);
                return Err(error);
            }
        };
        slot.bound = Some(bounds);
        Ok(PluginEndpoint::new(
            EndpointInner::L0(L0Endpoint {
                core: Arc::clone(&slot.core),
                mirror: Arc::clone(&slot.mirror),
                attached: Arc::clone(&slot.attached),
                epoch,
            }),
            bounds,
            in_scratch,
            // L0 is a direct call — no pipeline latency.
            0,
        ))
    }

    /// Runs one `Shared`-slot verb under the pinned three-tier lock
    /// protocol:
    ///
    /// 1. entry pre-check — `mirror == Failed` converges without
    ///    touching `core`;
    /// 2. `core.lock()` — `Err(PoisonError)` (any plugin panic
    ///    unwinding through a held guard poisons — a verb-CS hook or
    ///    the host-side `process` Shared arm) converges the same
    ///    way;
    /// 3. in-CS mirror re-read — `Failed` releases the guard and
    ///    converges without running `body` (no hook, no transition,
    ///    no `set_state`).
    ///
    /// `body` runs inside the critical section — legality check,
    /// transition commit, `set_state` mirror store and the plugin
    /// hook all under the one guard (state verbs run an empty
    /// transition part) — so `try_lock` observers only ever see
    /// fully committed state.
    ///
    /// Starvation note: the blocking `lock()` means every verb —
    /// `destroy`'s `into_inner` teardown excepted — queues behind an
    /// in-flight endpoint `process`; a wedged plugin starves verbs.
    /// Acceptable: a wedged plugin has already broken the stream.
    fn shared_verb<R>(
        slot: &SharedSlot,
        lifecycle: &mut Lifecycle,
        body: impl FnOnce(&mut Lifecycle, &SharedSlot, &mut SendCore) -> Result<R, HostError>,
    ) -> Result<R, HostError> {
        // (1) entry pre-check — converge without locking.
        if mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed() {
            return Err(converge_failed(lifecycle));
        }
        // (2) a poisoned lock is a `Failed` observation.
        let Ok(mut core) = slot.core.lock() else {
            return Err(converge_failed(lifecycle));
        };
        // (3) authoritative committed state, now the CS is held.
        if mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed() {
            return Err(converge_failed(lifecycle));
        }
        body(lifecycle, slot, &mut core)
    }

    fn instantiate(&mut self, plugin_id: &str) -> Result<HostReply, HostError> {
        let position = self
            .factories
            .iter()
            .position(|factory| factory.descriptor().id == plugin_id)
            .ok_or(HostError::UnknownPluginId)?;
        // Opt-in endpoint capability: a `Send`-capable instance goes
        // into a `Shared` slot; anything else keeps the direct
        // `Plain` form (`create()` unchanged — non-`Send` L0 use is
        // not regressed).
        let slot = match self.factories[position].create_sendable() {
            Some(plugin) => InstanceSlot::Shared(SharedSlot {
                core: Arc::new(Mutex::new(SendCore {
                    plugin,
                    context: ProcessContext::new(),
                    activated: None,
                })),
                mirror: Arc::new(AtomicU8::new(mirror_encode(LifecycleState::Created))),
                attached: Arc::new(AtomicU64::new(0)),
                bound: None,
                next_epoch: 1,
            }),
            None => InstanceSlot::Plain {
                plugin: self.factories[position].create(),
                context: ProcessContext::new(),
            },
        };
        if let Some(slot_index) = self.free_slots.pop() {
            let index = slot_index as usize;
            let generation = self.generations[index].wrapping_add(1);
            self.generations[index] = generation;
            self.instances[index] = Some(Instance::new(slot, generation));
            Ok(HostReply::Instantiated {
                handle: PluginHandle::new(slot_index, generation),
            })
        } else {
            let slot_index = u32::try_from(self.instances.len()).expect("instance slots fit u32");
            self.instances.push(Some(Instance::new(slot, 0)));
            self.generations.push(0);
            Ok(HostReply::Instantiated {
                handle: PluginHandle::new(slot_index, 0),
            })
        }
    }

    fn initialize(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                // Validate before the hook: `InitFailed` is only
                // reachable from `Created`, so the success transition
                // commits last.
                instance
                    .lifecycle
                    .state()
                    .transition(LifecycleEvent::Initialize)
                    .map_err(HostError::Lifecycle)?;
                if let Err(error) = plugin.init() {
                    // Reported failure destination: the machine stays
                    // Created.
                    let _ = instance.lifecycle.transition(LifecycleEvent::InitFailed);
                    return Err(error);
                }
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Initialize)
                    .map_err(HostError::Lifecycle)?;
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |lifecycle, slot, core| {
                    // Same semantics as `Plain` — legality → hook →
                    // commit → store — all inside the one critical
                    // section.
                    lifecycle
                        .state()
                        .transition(LifecycleEvent::Initialize)
                        .map_err(HostError::Lifecycle)?;
                    if let Err(error) = core.plugin.init() {
                        let _ = lifecycle.transition(LifecycleEvent::InitFailed);
                        set_state(slot, lifecycle.state());
                        return Err(error);
                    }
                    lifecycle
                        .transition(LifecycleEvent::Initialize)
                        .map_err(HostError::Lifecycle)?;
                    set_state(slot, lifecycle.state());
                    Ok(HostReply::Ok)
                })
            }
        }
    }

    fn activate(
        &mut self,
        handle: PluginHandle,
        config: &ProcessConfig,
    ) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, context } => {
                // Same shape: `ActivateFailed` is only reachable from
                // `Initialized`, so `Activate` commits after
                // `prepare`.
                instance
                    .lifecycle
                    .state()
                    .transition(LifecycleEvent::Activate)
                    .map_err(HostError::Lifecycle)?;
                if let Err(error) = plugin.prepare(config, context) {
                    // Reported failure destination: back to
                    // Initialized, retryable.
                    let _ = instance
                        .lifecycle
                        .transition(LifecycleEvent::ActivateFailed);
                    return Err(HostError::Prepare(error));
                }
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Activate)
                    .map_err(HostError::Lifecycle)?;
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |lifecycle, slot, core| {
                    lifecycle
                        .state()
                        .transition(LifecycleEvent::Activate)
                        .map_err(HostError::Lifecycle)?;
                    // Re-`Activate` bound gate: while an endpoint is
                    // attached (`attached != 0` is authoritative; the
                    // `bound` field is the recorded bound) a new
                    // `max_frames` above the attach-time bound would
                    // break the scratch contract — reject before any
                    // commit or `activated` update.
                    if slot.attached.load(Ordering::Acquire) != 0
                        && slot
                            .bound
                            .is_some_and(|bound| config.max_frames() > bound.max_frames)
                    {
                        return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
                    }
                    if let Err(error) = core.plugin.prepare(config, &mut core.context) {
                        let _ = lifecycle.transition(LifecycleEvent::ActivateFailed);
                        set_state(slot, lifecycle.state());
                        return Err(HostError::Prepare(error));
                    }
                    lifecycle
                        .transition(LifecycleEvent::Activate)
                        .map_err(HostError::Lifecycle)?;
                    // Publish the new activated config before the
                    // mirror — `mirror ∈ {Activated, Processing}
                    // ⟹ activated.is_some()`.
                    core.activated = Some(*config);
                    set_state(slot, lifecycle.state());
                    Ok(HostReply::Ok)
                })
            }
        }
    }

    fn start_processing(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                instance
                    .lifecycle
                    .transition(LifecycleEvent::StartProcessing)
                    .map_err(HostError::Lifecycle)?;
                if let Err(error) = plugin.start_processing() {
                    // Roll back: the plugin never started processing.
                    let _ = instance
                        .lifecycle
                        .transition(LifecycleEvent::StopProcessing);
                    return Err(error);
                }
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |lifecycle, slot, core| {
                    lifecycle
                        .transition(LifecycleEvent::StartProcessing)
                        .map_err(HostError::Lifecycle)?;
                    set_state(slot, lifecycle.state());
                    if let Err(error) = core.plugin.start_processing() {
                        let _ = lifecycle.transition(LifecycleEvent::StopProcessing);
                        set_state(slot, lifecycle.state());
                        return Err(error);
                    }
                    Ok(HostReply::Ok)
                })
            }
        }
    }

    fn stop_processing(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                instance
                    .lifecycle
                    .transition(LifecycleEvent::StopProcessing)
                    .map_err(HostError::Lifecycle)?;
                plugin.stop_processing()?;
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |lifecycle, slot, core| {
                    lifecycle
                        .transition(LifecycleEvent::StopProcessing)
                        .map_err(HostError::Lifecycle)?;
                    set_state(slot, lifecycle.state());
                    core.plugin.stop_processing()?;
                    Ok(HostReply::Ok)
                })
            }
        }
    }

    fn deactivate(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { .. } => {
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Deactivate)
                    .map_err(HostError::Lifecycle)?;
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |lifecycle, slot, core| {
                    lifecycle
                        .transition(LifecycleEvent::Deactivate)
                        .map_err(HostError::Lifecycle)?;
                    // The activation is gone — keep
                    // `mirror ∈ {Activated, Processing}
                    // ⟹ activated.is_some()`.
                    core.activated = None;
                    set_state(slot, lifecycle.state());
                    Ok(HostReply::Ok)
                })
            }
        }
    }

    fn destroy(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let index = handle.slot() as usize;
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { .. } => {
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Destroy)
                    .map_err(HostError::Lifecycle)?;
            }
            InstanceSlot::Shared(slot) => {
                // Converge the bookkeeping lifecycle first — a
                // `Failed` observation (the mirror, or a poisoned
                // `core` left by any panicked plugin call that
                // unwound through a held guard — a verb hook or the
                // host-side `process` arm) makes `Destroy` reachable
                // through `Failed → Destroyed`.
                // `is_poisoned` is a non-locking flag read.
                if (mirror_decode(slot.mirror.load(Ordering::Acquire)).is_failed()
                    || slot.core.is_poisoned())
                    && !instance.lifecycle.state().is_failed()
                {
                    let _ = instance.lifecycle.transition(LifecycleEvent::Failed);
                }
                instance
                    .lifecycle
                    .transition(LifecycleEvent::Destroy)
                    .map_err(HostError::Lifecycle)?;
                // The single `into_inner` exception: teardown runs
                // even on a poisoned lock — no plugin hook is called
                // (the possibly-panicked plugin's `destroy` hook does
                // not exist on this surface; the core contents drop
                // with the slot).
                let mut core = slot
                    .core
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                core.activated = None;
                set_state(slot, instance.lifecycle.state());
            }
        }
        self.instances[index] = None;
        self.free_slots.push(handle.slot());
        Ok(HostReply::Ok)
    }

    fn save_state(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                plugin.save_state().map(|bytes| HostReply::State { bytes })
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    core.plugin
                        .save_state()
                        .map(|bytes| HostReply::State { bytes })
                })
            }
        }
    }

    /// The instance's declared bus table — `Plain` slots read it
    /// directly; `Shared` slots take the verb lock so the report is a
    /// committed-state observation.
    ///
    /// **RT contract:** non-RT only — allocates the report vec; the
    /// `Shared` arm takes the `core` mutex.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle and
    /// [`HostError::Failed`] when the instance has already converged
    /// (same gate every `Shared` verb runs).
    pub fn buses(&mut self, handle: PluginHandle) -> Result<BusReport, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => Ok(BusReport::Reported(plugin.buses().to_vec())),
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    Ok(core.plugin.buses().to_vec())
                })
                .map(BusReport::Reported)
            }
        }
    }

    /// Drains the instance's plugin→host reports — the canonical
    /// [`Event`]s the plugin queued during `process` calls since the
    /// last drain — appending them to `out`.
    ///
    /// This is the host-side consumer surface of the neutral
    /// [`PluginProcessor::drain_output_reports`](ruviform_plugin_core::PluginProcessor::drain_output_reports)
    /// contract
    /// (`docs/designs/issue-253-hostgraph-reports.md`): a pull-based,
    /// opt-in read. Nothing drains implicitly — a consumer that
    /// never calls this simply lets reports accumulate in the
    /// plugin's bounded pending buffer, where overflow is the
    /// producer's defined drop. `Plain` slots call the plugin
    /// directly (the same ungated shape `buses` takes); `Shared`
    /// slots run the pinned three-tier lock protocol, so the drain
    /// serializes against an attached endpoint's `try_lock` RT leg
    /// on the same `core` mutex and converges `Failed`/poisoned
    /// observations exactly like every other `Shared` verb.
    ///
    /// Reports keep their producer order and within-block frame
    /// offsets; when several blocks were processed between drains the
    /// stream carries no block-boundary marks — per-block attribution
    /// is the consumer's drain cadence, not the host's reinterpretation.
    ///
    /// **RT contract:** non-RT only — the `out` append may allocate
    /// (the caller owns and may reuse the buffer) and the `Shared`
    /// arm takes the `core` mutex. Never call from inside `process`.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle and
    /// [`HostError::Failed`] when a `Shared` instance has already
    /// converged (same gate every `Shared` verb runs).
    pub fn drain_output_reports(
        &mut self,
        handle: PluginHandle,
        out: &mut Vec<Event>,
    ) -> Result<ReportDrain, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                plugin.drain_output_reports(out);
                Ok(ReportDrain::Drained)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    core.plugin.drain_output_reports(out);
                    Ok(ReportDrain::Drained)
                })
            }
        }
    }

    fn restore_state(
        &mut self,
        handle: PluginHandle,
        bytes: &[u8],
    ) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                plugin.restore_state(bytes).map(|()| HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    core.plugin.restore_state(bytes).map(|()| HostReply::Ok)
                })
            }
        }
    }

    fn open_gui(
        &mut self,
        handle: PluginHandle,
        parent: crate::GuiParentToken,
    ) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                let gui: GuiHandle = plugin.open_gui(parent)?;
                Ok(HostReply::Opened { gui })
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    core.plugin
                        .open_gui(parent)
                        .map(|gui| HostReply::Opened { gui })
                })
            }
        }
    }

    fn close_gui(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        match &mut instance.slot {
            InstanceSlot::Plain { plugin, .. } => {
                plugin.close_gui()?;
                Ok(HostReply::Ok)
            }
            InstanceSlot::Shared(slot) => {
                Self::shared_verb(slot, &mut instance.lifecycle, |_lifecycle, _slot, core| {
                    core.plugin.close_gui().map(|()| HostReply::Ok)
                })
            }
        }
    }

    fn instance_mut(&mut self, handle: PluginHandle) -> Result<&mut Instance, HostError> {
        self.instances
            .get_mut(handle.slot() as usize)
            .and_then(Option::as_mut)
            .filter(|instance| instance.generation == handle.generation())
            .ok_or(HostError::InvalidHandle)
    }
}

#[cfg(test)]
mod tests {
    //! `Shared`-slot / endpoint contract tests (`docs/test-specs/
    //! plugin-endpoint-contract-tests.md`): `SendGain` rides the
    //! `create_sendable` dispatch into `Shared`; `PlainGain` is an
    //! `Rc`-marked `!Send` plugin that keeps the `Plain` slot honest.

    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use ruviform_audio_core::{AudioBufferViewMut, SampleRate};
    use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
    use ruviform_plugin_core::{
        BufferView, BufferViewMut, EventKind, ParamEvent, ParamId, ParamInfo, ParamValue,
        PluginDescriptor, PluginProcessor, ProcessStatus,
    };
    use ruviform_rt::RtContext;

    use super::*;
    use crate::{GuiParentToken, Plugin, erase, erase_sendable};

    const SEND_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.send-gain",
        name: "Test Send Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    const PLAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.plain-gain",
        name: "Test Plain Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    const REPORT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.report-gain",
        name: "Test Report Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    /// Same erased-view fixtures as `lib.rs`'s host tests — kept
    /// per-module (the duplication is intentional).
    struct SliceIn<'a>(&'a [&'a [f32]]);

    impl BufferView for SliceIn<'_> {
        fn channels(&self) -> usize {
            self.0.len()
        }
        fn frames(&self) -> usize {
            self.0.first().map_or(0, |c| c.len())
        }
        fn channel(&self, index: usize) -> Option<&[f32]> {
            self.0.get(index).copied()
        }
    }

    struct SliceOut<'a>(&'a mut [&'a mut [f32]]);

    impl BufferView for SliceOut<'_> {
        fn channels(&self) -> usize {
            self.0.len()
        }
        fn frames(&self) -> usize {
            self.0.first().map_or(0, |c| c.len())
        }
        fn channel(&self, index: usize) -> Option<&[f32]> {
            self.0.get(index).map(|c| &c[..])
        }
    }

    impl BufferViewMut for SliceOut<'_> {
        fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
            self.0.get_mut(index).map(|c| &mut c[..])
        }
    }

    /// The `Shared` fixture — a `Send` gain with injectable failure /
    /// panic points for the convergence paths. The orthogonal
    /// injection points stay as plain bools — a state machine would
    /// only obscure the fixture.
    #[expect(clippy::struct_excessive_bools)]
    struct SendGain {
        gain: f32,
        fail_prepare: bool,
        fail_start: bool,
        panic_in_process: bool,
        panic_in_save: bool,
    }

    impl PluginProcessor for SendGain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            if self.fail_prepare {
                return Err(PrepareError::ActivationFailed);
            }
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            inputs: &[I],
            outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            assert!(!self.panic_in_process, "injected process panic");
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                for channel in 0..output.channels() {
                    if let (Some(src), Some(dst)) =
                        (input.channel(channel), output.channel_mut(channel))
                    {
                        for (s, d) in src.iter().zip(dst.iter_mut()) {
                            *d = *s * self.gain;
                        }
                    }
                }
            }
            ProcessResult::CONTINUE
        }
    }

    impl Plugin for SendGain {
        fn descriptor(&self) -> PluginDescriptor {
            SEND_DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
        fn start_processing(&mut self) -> Result<(), HostError> {
            if self.fail_start {
                return Err(HostError::PluginFailed);
            }
            Ok(())
        }
        fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
            assert!(!self.panic_in_save, "injected save panic");
            Ok(self.gain.to_le_bytes().to_vec())
        }
        fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
            let bytes: [u8; 4] = state.try_into().map_err(|_| HostError::PluginFailed)?;
            self.gain = f32::from_le_bytes(bytes);
            Ok(())
        }
        fn open_gui(&mut self, _parent: GuiParentToken) -> Result<GuiHandle, HostError> {
            Ok(GuiHandle::new(7))
        }
        fn close_gui(&mut self) -> Result<(), HostError> {
            Ok(())
        }
    }

    /// The `create_sendable`-capable factory — instances land in
    /// `Shared` slots. Carries the fixture's four injection points
    /// verbatim — see `SendGain`'s `expect` note.
    #[expect(clippy::struct_excessive_bools)]
    struct SendGainFactory {
        fail_prepare: bool,
        fail_start: bool,
        panic_in_process: bool,
        panic_in_save: bool,
    }

    impl SendGainFactory {
        fn new() -> Self {
            Self {
                fail_prepare: false,
                fail_start: false,
                panic_in_process: false,
                panic_in_save: false,
            }
        }
        fn make(&self) -> SendGain {
            SendGain {
                gain: 2.0,
                fail_prepare: self.fail_prepare,
                fail_start: self.fail_start,
                panic_in_process: self.panic_in_process,
                panic_in_save: self.panic_in_save,
            }
        }
    }

    impl PluginFactory for SendGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            SEND_DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(self.make())
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            Some(erase_sendable(self.make()))
        }
    }

    /// The `!Send` fixture — the `Rc` marker keeps the instance
    /// honestly non-`Send`; the factory leaves `create_sendable` at
    /// its default `None`, so instances land in `Plain` slots.
    struct PlainGain {
        gain: f32,
        _pin: Rc<()>,
    }

    impl PluginProcessor for PlainGain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            Ok(())
        }
        fn process<I, O>(
            &mut self,
            inputs: &[I],
            outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                for channel in 0..output.channels() {
                    if let (Some(src), Some(dst)) =
                        (input.channel(channel), output.channel_mut(channel))
                    {
                        for (s, d) in src.iter().zip(dst.iter_mut()) {
                            *d = *s * self.gain;
                        }
                    }
                }
            }
            ProcessResult::CONTINUE
        }
    }

    impl Plugin for PlainGain {
        fn descriptor(&self) -> PluginDescriptor {
            PLAIN_DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    struct PlainGainFactory;

    impl PluginFactory for PlainGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            PLAIN_DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(PlainGain {
                gain: 2.0,
                _pin: Rc::new(()),
            })
        }
    }

    /// The pending-report bound the fixture declares — a real
    /// adapter's producer-side capacity. `process` drops (does not
    /// count) past it; the fixture stays silent on overflow
    /// accounting, which is producer-defined.
    const REPORT_CAPACITY: usize = 8;

    /// The report pair one `process` block queues — one
    /// `ParamValue` plus one `Midi` `NoteOn`, offsets and payloads
    /// derived from the block counter so tests can rebuild the
    /// expected stream with the same function.
    fn report_pair(block: u32) -> [Event; 2] {
        [
            Event::new(
                block % 8,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(0),
                    value: ParamValue::new(f64::from(block % 4 + 1) / 4.0).unwrap(),
                }),
            ),
            Event::new(
                4 + block % 4,
                EventKind::Midi(MidiEvent {
                    group: MidiGroup::ZERO,
                    channel: MidiChannel::new(0).unwrap(),
                    kind: MidiKind::NoteOn {
                        key: NoteKey::new(60).unwrap(),
                        velocity: block,
                    },
                }),
            ),
        ]
    }

    /// The report-producing fixture — a `Send` gain that queues
    /// canonical [`Event`] reports into a bounded pending buffer
    /// during `process`, the producer shape a real adapter's
    /// `outputParameterChanges`/`outputEvents` mirror implements.
    /// `ReportGainFactory`'s `sendable` flag picks the `Shared` or
    /// `Plain` slot so both drain arms run identical producer code.
    struct ReportGain {
        gain: f32,
        blocks: u32,
        pending: Vec<Event>,
        panic_in_process: bool,
    }

    impl PluginProcessor for ReportGain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            inputs: &[I],
            outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            assert!(!self.panic_in_process, "injected process panic");
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                for channel in 0..output.channels() {
                    if let (Some(src), Some(dst)) =
                        (input.channel(channel), output.channel_mut(channel))
                    {
                        for (s, d) in src.iter().zip(dst.iter_mut()) {
                            *d = *s * self.gain;
                        }
                    }
                }
            }
            if self.pending.len() + 2 <= REPORT_CAPACITY {
                self.pending.extend_from_slice(&report_pair(self.blocks));
            }
            self.blocks += 1;
            ProcessResult::CONTINUE
        }

        /// The fixture's drain — move the whole pending buffer into
        /// the caller's `out`, the same append contract real adapters
        /// implement over their bounded queues.
        fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
            out.append(&mut self.pending);
        }
    }

    impl Plugin for ReportGain {
        fn descriptor(&self) -> PluginDescriptor {
            REPORT_DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    /// The report fixture's factory — `sendable` picks the slot
    /// kind: `true` offers `create_sendable` (→ `Shared`), `false`
    /// leaves it at the default `None` (→ `Plain`).
    struct ReportGainFactory {
        sendable: bool,
        panic_in_process: bool,
    }

    impl PluginFactory for ReportGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            REPORT_DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(ReportGain {
                gain: 2.0,
                blocks: 0,
                pending: Vec::with_capacity(REPORT_CAPACITY),
                panic_in_process: self.panic_in_process,
            })
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            self.sendable.then(|| {
                erase_sendable(ReportGain {
                    gain: 2.0,
                    blocks: 0,
                    pending: Vec::with_capacity(REPORT_CAPACITY),
                    panic_in_process: self.panic_in_process,
                })
            })
        }
    }

    fn report_host(sendable: bool) -> InProcessHost {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(ReportGainFactory {
            sendable,
            panic_in_process: false,
        }));
        host
    }

    fn send_host() -> InProcessHost {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory::new()));
        host
    }

    fn instantiate(host: &mut InProcessHost, plugin_id: &str) -> PluginHandle {
        match host
            .execute(HostCommand::Instantiate {
                plugin_id: plugin_id.to_owned(),
            })
            .unwrap()
        {
            HostReply::Instantiated { handle } => handle,
            _ => panic!("unexpected reply"),
        }
    }

    fn config(min_frames: u32, max_frames: u32) -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), min_frames, max_frames).unwrap()
    }

    fn bounds(max_frames: u32, channels: u32) -> EndpointBounds {
        EndpointBounds {
            max_frames,
            channels,
        }
    }

    fn drive_to_activated(host: &mut InProcessHost, handle: PluginHandle) {
        host.execute(HostCommand::Initialize { handle }).unwrap();
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 512),
        })
        .unwrap();
    }

    fn drive_to_processing(host: &mut InProcessHost, handle: PluginHandle) {
        drive_to_activated(host, handle);
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
    }

    fn shared_slot(host: &InProcessHost, handle: PluginHandle) -> &SharedSlot {
        let instance = host.instances[handle.slot() as usize]
            .as_ref()
            .expect("live instance");
        match &instance.slot {
            InstanceSlot::Shared(slot) => slot,
            InstanceSlot::Plain { .. } => panic!("expected a Shared slot"),
        }
    }

    /// Runs one `PluginEndpoint::process` block over plain slices —
    /// the view plumbing (`SliceIn`/`SliceOut` → `HostInput`/
    /// `HostOutput`) is built inside the call scope so callers keep
    /// their data free between calls. `frames` follows the same
    /// in-bus-first rule `block_dims` applies.
    fn run_endpoint<'a>(
        endpoint: &mut PluginEndpoint,
        inputs_data: &[&[f32]],
        outputs_data: &'a mut [&'a mut [f32]],
    ) -> Result<ProcessResult, HostError> {
        let frames = inputs_data
            .first()
            .map_or_else(|| outputs_data.first().map_or(0, |o| o.len()), |i| i.len());
        let input_view = SliceIn(inputs_data);
        let inputs = [HostInput(&input_view)];
        let mut out_view = SliceOut(outputs_data);
        let mut outputs = [HostOutput(&mut out_view)];
        endpoint.process(
            &inputs,
            &mut outputs,
            EventList::empty(u32::try_from(frames).unwrap()),
            None,
        )
    }

    /// The same plumbing for `InProcessHost::process`.
    fn run_host<'a>(
        host: &mut InProcessHost,
        handle: PluginHandle,
        inputs_data: &[&[f32]],
        outputs_data: &'a mut [&'a mut [f32]],
    ) -> Result<ProcessResult, HostError> {
        let frames = inputs_data
            .first()
            .map_or_else(|| outputs_data.first().map_or(0, |o| o.len()), |i| i.len());
        let input_view = SliceIn(inputs_data);
        let inputs = [HostInput(&input_view)];
        let mut out_view = SliceOut(outputs_data);
        let mut outputs = [HostOutput(&mut out_view)];
        host.process(
            handle,
            &inputs,
            &mut outputs,
            EventList::empty(u32::try_from(frames).unwrap()),
            None,
        )
    }

    fn expect_send<T: Send>() {}

    /// Compile-time `Send` witnesses for the shared cell — the
    /// endpoint can only capture `Send` handles into it (endpoint-
    /// side types are witnessed in `endpoint.rs`'s test module).
    #[test]
    fn shared_cell_types_are_send() {
        expect_send::<SendCore>();
        expect_send::<SharedSlot>();
    }

    #[test]
    fn instantiate_selects_slot_by_create_sendable() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory::new()));
        host.register_factory(Box::new(PlainGainFactory));
        let shared = instantiate(&mut host, SEND_DESCRIPTOR.id);
        let plain = instantiate(&mut host, PLAIN_DESCRIPTOR.id);
        assert!(matches!(
            host.instances[plain.slot() as usize].as_ref().unwrap().slot,
            InstanceSlot::Plain { .. }
        ));
        let slot = shared_slot(&host, shared);
        assert_eq!(
            mirror_decode(slot.mirror.load(Ordering::Acquire)),
            LifecycleState::Created
        );
        assert_eq!(slot.attached.load(Ordering::Acquire), 0);
        assert!(slot.bound.is_none());
        assert_eq!(host.lifecycle_state(shared), Some(LifecycleState::Created));
    }

    /// `create_sendable() == None` keeps the direct L0 path:
    /// `plugin_mut` lends `&mut`, `process` calls straight through —
    /// and the slot is not endpoint-capable.
    #[test]
    fn plain_slot_keeps_direct_path() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(PlainGainFactory));
        let handle = instantiate(&mut host, PLAIN_DESCRIPTOR.id);
        assert!(host.plugin_mut(handle).is_some());
        drive_to_processing(&mut host, handle);

        let input = [0.5f32; 4];
        let mut out = [0.0f32; 4];
        run_host(&mut host, handle, &[&input[..]], &mut [&mut out[..]]).unwrap();
        assert_eq!(out, [1.0f32; 4]);
        // `Plain` is not endpoint-capable — `EndpointUnavailable` even
        // at `Processing`.
        assert_eq!(
            host.attach_endpoint(handle, bounds(512, 2)).err(),
            Some(HostError::EndpointUnavailable)
        );
    }

    /// The shared-cell rule from the spec: a `&mut dyn ErasedPlugin`
    /// cannot be lent out of the mutex — `plugin_mut` is `None` (the
    /// same defined shape `PluginHost` gives L1 slots).
    #[test]
    fn shared_plugin_mut_is_none() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        assert!(host.plugin_mut(handle).is_none());
    }

    /// Spec「verb 到達（graph-attached）」: every non-RT verb reaches
    /// the plugin while an endpoint holds the epoch.
    #[test]
    fn shared_verbs_reach_plugin_while_attached() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_activated(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        assert_eq!(endpoint.bounds(), bounds(512, 2));
        assert_eq!(endpoint.latency_frames(), 0);

        match host.execute(HostCommand::SaveState { handle }).unwrap() {
            HostReply::State { bytes } => assert_eq!(bytes, 2.0f32.to_le_bytes()),
            _ => panic!("unexpected reply"),
        }
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: 4.0f32.to_le_bytes().to_vec(),
        })
        .unwrap();
        match host.execute(HostCommand::SaveState { handle }).unwrap() {
            HostReply::State { bytes } => assert_eq!(bytes, 4.0f32.to_le_bytes()),
            _ => panic!("unexpected reply"),
        }
        match host
            .execute(HostCommand::OpenGui {
                handle,
                parent: GuiParentToken::new(0),
            })
            .unwrap()
        {
            HostReply::Opened { gui } => assert_eq!(gui, GuiHandle::new(7)),
            _ => panic!("unexpected reply"),
        }
        assert_eq!(
            host.execute(HostCommand::CloseGui { handle }),
            Ok(HostReply::Ok)
        );
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
        // The endpoint still processes while verb traffic flows —
        // the restored gain (4.0) is applied.
        let in_l = [0.5f32; 4];
        let in_r = [0.25f32; 4];
        let mut out_l = [0.0f32; 4];
        let mut out_r = [0.0f32; 4];
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        assert_eq!(out_l, [2.0f32; 4]);
        assert_eq!(out_r, [1.0f32; 4]);
        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        drop(endpoint);
        assert_eq!(
            host.execute(HostCommand::Destroy { handle }),
            Ok(HostReply::Ok)
        );
    }

    /// `prepare` failure: `ActivateFailed` reports back to
    /// `Initialized` — bookkeeping and mirror agree, retryable.
    #[test]
    fn shared_prepare_failure_keeps_initialized() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory {
            fail_prepare: true,
            ..SendGainFactory::new()
        }));
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        host.execute(HostCommand::Initialize { handle }).unwrap();
        assert_eq!(
            host.execute(HostCommand::Activate {
                handle,
                config: config(1, 512),
            }),
            Err(HostError::Prepare(PrepareError::ActivationFailed))
        );
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Initialized)
        );
        assert_eq!(
            mirror_decode(shared_slot(&host, handle).mirror.load(Ordering::Acquire)),
            LifecycleState::Initialized
        );
    }

    /// `start_processing` hook failure: `StartProcessing` commits
    /// first, the hook error rolls back to `Activated` — mirror
    /// follows the same path.
    #[test]
    fn shared_start_failure_rolls_back_to_activated() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory {
            fail_start: true,
            ..SendGainFactory::new()
        }));
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_activated(&mut host, handle);
        assert_eq!(
            host.execute(HostCommand::StartProcessing { handle }),
            Err(HostError::PluginFailed)
        );
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Activated)
        );
        assert_eq!(
            mirror_decode(shared_slot(&host, handle).mirror.load(Ordering::Acquire)),
            LifecycleState::Activated
        );
    }

    /// Spec 拒否順序: **(1)** state errors → **(2)** slot shape →
    /// **(3)** activated-layer bounds → **(4)** epoch CAS →
    /// **(5)** scratch alloc. A rejected attach consumes nothing.
    #[test]
    fn attach_rejects_in_pinned_order() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory::new()));
        host.register_factory(Box::new(PlainGainFactory));
        let shared = instantiate(&mut host, SEND_DESCRIPTOR.id);
        let plain = instantiate(&mut host, PLAIN_DESCRIPTOR.id);

        // (1) state errors first — even on a `Plain` slot (state is
        // checked before shape).
        assert_eq!(
            host.attach_endpoint(PluginHandle::new(9, 0), bounds(512, 2))
                .err(),
            Some(HostError::InvalidHandle)
        );
        assert!(matches!(
            host.attach_endpoint(plain, bounds(512, 2)),
            Err(HostError::Lifecycle(_))
        ));
        assert!(matches!(
            host.attach_endpoint(shared, bounds(512, 2)),
            Err(HostError::Lifecycle(_))
        ));
        // State wins over shape AND bounds — a `Created` slot with
        // invalid bounds still reports `Err(Lifecycle)`.
        assert!(matches!(
            host.attach_endpoint(shared, bounds(0, 0)),
            Err(HostError::Lifecycle(_))
        ));
        host.execute(HostCommand::Initialize { handle: shared })
            .unwrap();
        assert!(matches!(
            host.attach_endpoint(shared, bounds(512, 2)),
            Err(HostError::Lifecycle(_))
        ));

        // (2) slot shape — `Plain` at `Activated` gets past the state
        // check and fails here.
        host.execute(HostCommand::Initialize { handle: plain })
            .unwrap();
        host.execute(HostCommand::Activate {
            handle: plain,
            config: config(1, 512),
        })
        .unwrap();
        assert_eq!(
            host.attach_endpoint(plain, bounds(512, 2)).err(),
            Some(HostError::EndpointUnavailable)
        );

        // (3) activated-layer bounds — every violation is
        // `Err(Prepare(UnsupportedConfig))` and claims nothing.
        host.execute(HostCommand::Activate {
            handle: shared,
            config: config(1, 512),
        })
        .unwrap();
        for bad in [
            bounds(0, 2),
            bounds(512, 0),
            bounds(WIRE_MAX_FRAMES + 1, 2),
            bounds(512, WIRE_MAX_CHANNELS + 1),
            // below the activated `max_frames`
            bounds(256, 2),
        ] {
            assert_eq!(
                host.attach_endpoint(shared, bad).err(),
                Some(HostError::Prepare(PrepareError::UnsupportedConfig)),
                "bounds {bad:?}"
            );
            assert_eq!(
                shared_slot(&host, shared).attached.load(Ordering::Acquire),
                0,
                "rejected attach claimed the epoch: {bad:?}"
            );
        }

        // (4) success, then the single-endpoint CAS.
        let endpoint = host.attach_endpoint(shared, bounds(512, 2)).unwrap();
        assert_eq!(
            host.attach_endpoint(shared, bounds(512, 2)).err(),
            Some(HostError::EndpointUnavailable)
        );
        // The instance stays attachable after the epoch is returned.
        drop(endpoint);
        assert_eq!(
            shared_slot(&host, shared).attached.load(Ordering::Acquire),
            0
        );
        host.attach_endpoint(shared, bounds(512, 2)).unwrap();
    }

    /// Epoch bookkeeping: epochs start at 1 and bump per claimed
    /// attach; `Drop` returns only its own epoch — a stale endpoint
    /// cannot clear a newer attachment.
    #[test]
    fn attach_epoch_drop_and_stale_endpoint() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_activated(&mut host, handle);

        let first = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        let attached = Arc::clone(&shared_slot(&host, handle).attached);
        assert_eq!(attached.load(Ordering::Acquire), 1);
        drop(first);
        assert_eq!(attached.load(Ordering::Acquire), 0);

        let second = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        assert_eq!(attached.load(Ordering::Acquire), 2);
        // A stale endpoint carrying epoch 1 — its `Drop` must not
        // clear the live epoch-2 attachment.
        let stale = {
            let slot = shared_slot(&host, handle);
            L0Endpoint {
                core: Arc::clone(&slot.core),
                mirror: Arc::clone(&slot.mirror),
                attached: Arc::clone(&slot.attached),
                epoch: 1,
            }
        };
        drop(stale);
        assert_eq!(attached.load(Ordering::Acquire), 2);
        assert_eq!(
            host.attach_endpoint(handle, bounds(512, 2)).err(),
            Some(HostError::EndpointUnavailable)
        );
        drop(second);
        assert_eq!(attached.load(Ordering::Acquire), 0);
        let _third = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        assert_eq!(attached.load(Ordering::Acquire), 3);
    }

    /// The L0 endpoint `process` path: gain applied on a valid
    /// block; out-of-contract blocks are `Err(Prepare)` with
    /// `outputs` unchanged; non-`Processing` is `Err(Lifecycle)`.
    #[test]
    fn endpoint_process_applies_gain_and_gates_blocks() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        // Attach at `Processing` — the second attachable state.
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        // Happy path: 4-frame stereo block, gain 2.0.
        let in_l = [0.5f32; 4];
        let in_r = [0.25f32; 4];
        let mut out_l = [0.0f32; 4];
        let mut out_r = [0.0f32; 4];
        let result = run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out_l, [1.0f32; 4]);
        assert_eq!(out_r, [0.5f32; 4]);

        // frames > activated `max_frames` → `Err(Prepare)`, out
        // unchanged.
        let big_in_l = vec![0.5f32; 600];
        let big_in_r = vec![0.5f32; 600];
        let mut big_out_l = vec![9.0f32; 600];
        let mut big_out_r = vec![9.0f32; 600];
        assert_eq!(
            run_endpoint(
                &mut endpoint,
                &[big_in_l.as_slice(), big_in_r.as_slice()],
                &mut [big_out_l.as_mut_slice(), big_out_r.as_mut_slice()],
            )
            .err(),
            Some(HostError::Prepare(PrepareError::UnsupportedConfig))
        );
        assert_eq!(big_out_l, vec![9.0f32; 600]);

        // Channel mismatch vs the bound — mono out on a stereo bound.
        let mono_in = [0.5f32; 4];
        let mut mono_out = [9.0f32; 4];
        assert_eq!(
            run_endpoint(&mut endpoint, &[&mono_in[..]], &mut [&mut mono_out[..]]).err(),
            Some(HostError::Prepare(PrepareError::UnsupportedConfig))
        );
        assert_eq!(mono_out, [9.0f32; 4]);

        // Below `min_frames` — a zero-length block.
        let empty_in: &[f32] = &[];
        let mut empty_out = [9.0f32; 4];
        assert_eq!(
            run_endpoint(&mut endpoint, &[empty_in], &mut [&mut empty_out[..]]).err(),
            Some(HostError::Prepare(PrepareError::UnsupportedConfig))
        );

        // Non-`Processing` → `Err(Lifecycle)`, `outputs` unchanged.
        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        let mut late_out = [9.0f32; 4];
        assert!(matches!(
            run_endpoint(
                &mut endpoint,
                &[&in_l[..], &in_r[..]],
                &mut [&mut late_out[..]]
            ),
            Err(HostError::Lifecycle(_))
        ));
        assert_eq!(late_out, [9.0f32; 4]);
    }

    /// Spec「endpoint panic 収束」: a plugin panic is caught, the
    /// mirror converges `Failed` via the endpoint's second-writer
    /// `fetch_max`, silence is written, `Ok(CONTINUE)` — every later
    /// observation short-circuits.
    #[test]
    fn endpoint_panic_converges_failed_silences_output() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory {
            panic_in_process: true,
            ..SendGainFactory::new()
        }));
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        let in_l = [0.5f32; 4];
        let in_r = [0.5f32; 4];
        let mut out_l = [9.0f32; 4];
        let mut out_r = [9.0f32; 4];

        let result = run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        );
        assert_eq!(
            result.unwrap().status,
            ProcessStatus::Continue,
            "panic converges to Ok(CONTINUE)"
        );
        assert_eq!(out_l, [0.0f32; 4]);
        assert_eq!(out_r, [0.0f32; 4]);
        // The endpoint's `fetch_max` converged the mirror — the
        // lock-free observation union reports `Failed` immediately.
        assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));

        // Every later call short-circuits on the mirror.
        let result = run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        );
        assert_eq!(result.unwrap().status, ProcessStatus::Continue);
        assert_eq!(out_l, [0.0f32; 4]);
        // Verbs and host process converge through the entry pre-check.
        assert_eq!(
            host.execute(HostCommand::SaveState { handle }),
            Err(HostError::Failed)
        );
        assert_eq!(
            run_host(
                &mut host,
                handle,
                &[&in_l[..], &in_r[..]],
                &mut [&mut out_l[..], &mut out_r[..]],
            ),
            Err(HostError::Failed)
        );
        // A further attach sees the converged `Failed` observation.
        assert_eq!(
            host.attach_endpoint(handle, bounds(512, 2)).err(),
            Some(HostError::Failed)
        );

        // `Destroy` reaches `Failed` instances — the endpoint's Arc
        // handles outlive the slot teardown.
        assert_eq!(
            host.execute(HostCommand::Destroy { handle }),
            Ok(HostReply::Ok)
        );
        assert!(matches!(
            run_endpoint(
                &mut endpoint,
                &[&in_l[..], &in_r[..]],
                &mut [&mut out_l[..], &mut out_r[..]],
            ),
            Err(HostError::Lifecycle(_))
        ));
        drop(endpoint);
    }

    /// Spec「poisoned lock」: a verb-CS plugin-hook panic unwinds
    /// through the guard → poisoned. Later verbs converge via the
    /// `match lock()` `Err` arm; `destroy` is the only `into_inner`
    /// teardown — no destroy hook runs on a panicked plugin.
    #[test]
    fn poisoned_verb_converges_and_destroy_tears_down() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(SendGainFactory {
            panic_in_save: true,
            ..SendGainFactory::new()
        }));
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);

        // The hook panic unwinds through the CS → the mutex poisons.
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            host.execute(HostCommand::SaveState { handle })
        }));
        assert!(panicked.is_err());

        // Tier-2 convergence — `Err(Failed)` from the poisoned lock.
        assert_eq!(
            host.execute(HostCommand::SaveState { handle }),
            Err(HostError::Failed)
        );
        assert_eq!(
            host.execute(HostCommand::Initialize { handle }),
            Err(HostError::Failed)
        );
        // The mirror keeps the last committed store (`Processing`) —
        // only in-CS commits and the endpoint's `fetch_max` write it,
        // so a poisoned lock with no endpoint leaves the stale
        // observation until `Destroy` (design-defined).
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );
        // Host-side `process` converges the same way.
        let input = [0.5f32; 4];
        let mut out = [0.0f32; 4];
        assert_eq!(
            run_host(&mut host, handle, &[&input[..]], &mut [&mut out[..]]),
            Err(HostError::Failed)
        );

        // `destroy` converges bookkeeping `Processing → Failed` then
        // `Failed → Destroyed`, tearing down via `into_inner`.
        assert_eq!(
            host.execute(HostCommand::Destroy { handle }),
            Ok(HostReply::Ok)
        );
        assert_eq!(host.lifecycle_state(handle), None);
    }

    /// Spec「verb 実行中の endpoint `process` は busy →
    /// `Err(EndpointUnavailable)` bypass」: a held `core` lock makes
    /// both `try_lock` paths return the defined bypass — `outputs`
    /// unchanged, no blocking.
    #[test]
    fn busy_cell_bypasses_endpoint_and_host_process() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        // Hold the shared cell — the Arc clone keeps the borrow off
        // `host`, so `&mut` calls below still work.
        let core = Arc::clone(&shared_slot(&host, handle).core);
        let guard = core.lock().unwrap();

        let in_l = [0.5f32; 4];
        let in_r = [0.5f32; 4];
        let mut out_l = [9.0f32; 4];
        let mut out_r = [9.0f32; 4];

        assert_eq!(
            run_endpoint(
                &mut endpoint,
                &[&in_l[..], &in_r[..]],
                &mut [&mut out_l[..], &mut out_r[..]],
            )
            .err(),
            Some(HostError::EndpointUnavailable)
        );
        assert_eq!(out_l, [9.0f32; 4]);
        assert_eq!(out_r, [9.0f32; 4]);
        assert_eq!(
            run_host(
                &mut host,
                handle,
                &[&in_l[..], &in_r[..]],
                &mut [&mut out_l[..], &mut out_r[..]],
            ),
            Err(HostError::EndpointUnavailable)
        );

        // The lock-free observation point still answers.
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );

        drop(guard);
        // After the cell frees the endpoint processes again.
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        assert_eq!(out_l, [1.0f32; 4]);
    }

    /// Spec「`&self` 照会は観測 union のみ」: `lifecycle_state`
    /// returns while another guard holds `core` — proof the query
    /// path never locks.
    #[test]
    fn lifecycle_state_reads_mirror_while_core_held() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let core = Arc::clone(&shared_slot(&host, handle).core);
        let _guard = core.lock().unwrap();
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );
    }

    /// Spec「再 Activate bound gate」: while the epoch lives, a new
    /// `max_frames` above the attach-time bound is rejected before
    /// any commit; a bound-fitting re-activation commits and updates
    /// `SendCore.activated`.
    #[test]
    fn reactivate_bound_gate() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_activated(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(1024, 2)).unwrap();

        host.execute(HostCommand::Deactivate { handle }).unwrap();
        assert_eq!(
            host.execute(HostCommand::Activate {
                handle,
                config: config(1, 2048),
            }),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig)),
            "re-activate above the attach bound is rejected"
        );
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Initialized)
        );
        // Within the bound — commits and republishes `activated`.
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 1024),
        })
        .unwrap();
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
        // 768 > the old 512 activated max — accepted under the new.
        let in_l = vec![0.5f32; 768];
        let in_r = vec![0.5f32; 768];
        let mut out_l = vec![0.0f32; 768];
        let mut out_r = vec![0.0f32; 768];
        run_endpoint(
            &mut endpoint,
            &[in_l.as_slice(), in_r.as_slice()],
            &mut [out_l.as_mut_slice(), out_r.as_mut_slice()],
        )
        .unwrap();
        assert_eq!(out_l[0], 1.0f32);

        // Detach frees the gate — a big re-activate is legal again.
        drop(endpoint);
        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 4096),
        })
        .unwrap();
    }

    /// The endpoint outlives its instance: `Destroy` while attached
    /// tears the slot down, the endpoint's `Arc` handles keep the
    /// cell alive, and the stale mirror reports `Destroyed` —
    /// `Drop`'s CAS runs on the surviving `attached` flag.
    #[test]
    fn endpoint_outlives_instance_teardown() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        assert_eq!(
            host.execute(HostCommand::Destroy { handle }),
            Ok(HostReply::Ok)
        );

        let input = [0.5f32; 4];
        let mut out = [9.0f32; 4];
        assert!(matches!(
            run_endpoint(&mut endpoint, &[&input[..]], &mut [&mut out[..]]),
            Err(HostError::Lifecycle(_))
        ));
        drop(endpoint);
    }

    /// Issue #22/#113 lane: the L0 endpoint `process` path — mirror
    /// read, `try_lock`, `catch_unwind`, plugin call — must not
    /// allocate inside the measurement window.
    #[test]
    fn rt_endpoint_process_no_alloc() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        let in_l = [0.5f32; 8];
        let in_r = [0.5f32; 8];
        let mut out_l = [0.0f32; 8];
        let mut out_r = [0.0f32; 8];

        // Warm-up outside the window — the first `catch_unwind`
        // registers per-thread panic state.
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..64 {
                run_endpoint(
                    &mut endpoint,
                    &[&in_l[..], &in_r[..]],
                    &mut [&mut out_l[..], &mut out_r[..]],
                )
                .unwrap();
            }
        });
    }

    /// Spec「`rt_endpoint_busy_bypass`」: the held-lock bypass returns
    /// its defined `Err` without allocating.
    #[test]
    fn rt_endpoint_busy_bypass_no_alloc() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        let in_l = [0.5f32; 8];
        let in_r = [0.5f32; 8];
        let mut out_l = [0.0f32; 8];
        let mut out_r = [0.0f32; 8];

        let core = Arc::clone(&shared_slot(&host, handle).core);
        let _guard = core.lock().unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                assert_eq!(
                    run_endpoint(
                        &mut endpoint,
                        &[&in_l[..], &in_r[..]],
                        &mut [&mut out_l[..], &mut out_r[..]],
                    )
                    .err(),
                    Some(HostError::EndpointUnavailable)
                );
            }
        });
    }

    /// One `PluginEndpoint::process_audio` block over a stereo
    /// slab — the graph-node `pull` shape: `out` carries the
    /// upstream block in and the result back.
    fn run_audio(
        endpoint: &mut PluginEndpoint,
        slab: &mut [f32],
        channels: usize,
        ctx: &RtContext,
    ) -> Result<ProcessResult, HostError> {
        let frames = slab.len() / channels;
        let mut out = AudioBufferViewMut::try_new(slab, channels, frames).unwrap();
        endpoint.process_audio(
            &mut out,
            EventList::empty(u32::try_from(frames).unwrap()),
            None,
            ctx,
        )
    }

    /// Issue #22 lane (`rt_endpoint_l0_process_no_alloc`): the
    /// `process_audio` shell on the L0 leg — bound clip, the
    /// `in_scratch` channel-major copy, mirror read, `try_lock`,
    /// `catch_unwind`, plugin call — must not allocate inside the
    /// measurement window.
    #[test]
    fn rt_endpoint_l0_process_no_alloc() {
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        let ctx = RtContext::mint();
        let mut slab = [0.5f32; 16]; // 2ch × 8 frames
        // Warm-up outside the window — the first `catch_unwind`
        // registers per-thread panic state, the staged-copy path
        // runs once.
        run_audio(&mut endpoint, &mut slab, 2, &ctx).unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..64 {
                // Transform semantics — the previous output would
                // feed back as input; refill a fixed block instead.
                slab.fill(0.5);
                run_audio(&mut endpoint, &mut slab, 2, &ctx).unwrap();
            }
        });
        // Gain ×2 through the staged copy — 0.5 in, 1.0 out.
        assert_eq!(slab, [1.0f32; 16]);
    }

    /// Spec「`rt_endpoint_busy_bypass`」(`process_audio` level): the
    /// held-lock bypass, the poisoned-cell `Failed` convergence and
    /// the non-`Processing` rejection all answer their defined
    /// output without allocating or hanging.
    #[test]
    fn rt_endpoint_busy_bypass() {
        let ctx = RtContext::mint();
        // -- busy: a held `core` lock → `Err(EndpointUnavailable)`,
        // `out` unchanged.
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        let mut slab = [9.0f32; 16];
        run_audio(&mut endpoint, &mut slab, 2, &ctx).unwrap();
        let core = Arc::clone(&shared_slot(&host, handle).core);
        let guard = core.lock().unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                assert_eq!(
                    run_audio(&mut endpoint, &mut slab, 2, &ctx).err(),
                    Some(HostError::EndpointUnavailable)
                );
            }
        });
        drop(guard);
        // -- poisoned: a panic through a held guard poisons the
        // cell — `try_lock`'s `Poisoned` arm converges `Failed`
        // (defined `Ok(CONTINUE)` + silence), no `Err`.
        let poisoned = catch_unwind(AssertUnwindSafe(|| {
            let _held = core.lock().unwrap();
            panic!("poison the shared cell");
        }));
        assert!(poisoned.is_err());
        slab.fill(9.0);
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                assert_eq!(
                    run_audio(&mut endpoint, &mut slab, 2, &ctx),
                    Ok(ProcessResult::CONTINUE)
                );
            }
        });
        assert_eq!(slab, [0.0f32; 16]);
        // -- non-`Processing`: an `Activated` mirror is the defined
        // `Err(Lifecycle)` — `out` unchanged.
        let mut host = send_host();
        let handle = instantiate(&mut host, SEND_DESCRIPTOR.id);
        drive_to_activated(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        slab.fill(9.0);
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                assert!(matches!(
                    run_audio(&mut endpoint, &mut slab, 2, &ctx),
                    Err(HostError::Lifecycle(_))
                ));
            }
        });
        assert_eq!(slab, [9.0f32; 16]);
    }

    /// Issue #253 slice (b), `Plain` arm: reports queued by `process`
    /// reach the caller in producer order with offsets and payloads
    /// intact; a repeat drain appends nothing yet still reports
    /// `Drained`; pre-existing `out` contents survive (the caller
    /// measures a drain by the length delta).
    #[test]
    fn drain_output_reports_plain_slot() {
        let mut host = report_host(false);
        let handle = instantiate(&mut host, REPORT_DESCRIPTOR.id);

        // A pre-`process` drain is an honest empty `Drained` — the
        // verb is not lifecycle-gated (the same ungated shape
        // `buses` takes).
        let sentinel = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(9),
                value: ParamValue::ONE,
            }),
        );
        let mut out = vec![sentinel];
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        assert_eq!(out, [sentinel]);

        drive_to_processing(&mut host, handle);
        let input = [0.5f32; 4];
        for _ in 0..3 {
            let mut block = [0.0f32; 4];
            run_host(&mut host, handle, &[&input[..]], &mut [&mut block[..]]).unwrap();
        }
        // DR-03 — a non-drain verb between processing and the pull
        // consumes nothing: the reports still arrive complete.
        assert!(matches!(host.buses(handle), Ok(BusReport::Reported(_))));
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        // Sentinel preserved; three blocks × two reports in producer
        // order — offsets and payloads carried verbatim.
        let expected: Vec<Event> = (0..3u32).flat_map(report_pair).collect();
        assert_eq!(out.len(), 1 + expected.len());
        assert_eq!(out[0], sentinel);
        assert_eq!(&out[1..], expected.as_slice());

        // No new processing → `Drained` with nothing appended.
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        assert_eq!(out.len(), 1 + expected.len());

        // Processing after a drain surfaces only the new reports.
        let mut block = [0.0f32; 4];
        run_host(&mut host, handle, &[&input[..]], &mut [&mut block[..]]).unwrap();
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        let mut expected = expected;
        expected.extend_from_slice(&report_pair(3));
        assert_eq!(&out[1..], expected.as_slice());
    }

    /// Issue #253 slice (b), `Shared` arm — the graph-attached
    /// consumer shape: reports produced through the endpoint's
    /// `try_lock` RT leg drain through the host's blocking
    /// `shared_verb`, and reports produced host-side
    /// (`InProcessHost::process`'s `try_lock` arm) land in the same
    /// pending buffer in call order.
    #[test]
    fn drain_output_reports_shared_slot_while_endpoint_attached() {
        let mut host = report_host(true);
        let handle = instantiate(&mut host, REPORT_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();

        let in_l = [0.5f32; 4];
        let in_r = [0.25f32; 4];
        let mut out_l = [0.0f32; 4];
        let mut out_r = [0.0f32; 4];
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        // One host-side block on the same `Shared` cell — the two
        // legs feed one pending buffer.
        let input = [0.5f32; 4];
        let mut block = [0.0f32; 4];
        run_host(&mut host, handle, &[&input[..]], &mut [&mut block[..]]).unwrap();
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();

        let mut out = Vec::new();
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        let expected: Vec<Event> = (0..3u32).flat_map(report_pair).collect();
        assert_eq!(out, expected);
    }

    /// Issue #253 slice (b), error surface: stale/unknown handles
    /// take `InvalidHandle` (the `instance_mut` gate every verb
    /// shares); a `Shared` instance that converged `Failed` answers
    /// `Err(Failed)` through `shared_verb`'s entry pre-check, before
    /// any plugin reach.
    #[test]
    fn drain_output_reports_error_surface() {
        // Stale handle — never instantiated.
        let mut host = report_host(true);
        let mut out = Vec::new();
        assert_eq!(
            host.drain_output_reports(PluginHandle::new(9, 0), &mut out),
            Err(HostError::InvalidHandle)
        );

        // Destroyed handle — the slot generation moved on.
        let handle = instantiate(&mut host, REPORT_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        host.execute(HostCommand::Destroy { handle }).unwrap();
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Err(HostError::InvalidHandle)
        );

        // Failed `Shared` instance — an endpoint-side plugin panic
        // converges the mirror; the drain short-circuits on the
        // entry pre-check like every other `Shared` verb.
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(ReportGainFactory {
            sendable: true,
            panic_in_process: true,
        }));
        let handle = instantiate(&mut host, REPORT_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        let mut endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        let in_l = [0.5f32; 4];
        let in_r = [0.5f32; 4];
        let mut out_l = [0.0f32; 4];
        let mut out_r = [0.0f32; 4];
        run_endpoint(
            &mut endpoint,
            &[&in_l[..], &in_r[..]],
            &mut [&mut out_l[..], &mut out_r[..]],
        )
        .unwrap();
        assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Err(HostError::Failed)
        );
        assert_eq!(out, []);
    }
}
