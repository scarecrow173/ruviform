//! [`PluginHost`] — the composite host: one public surface that
//! routes each registered `plugin_id` to the L0 in-process backend
//! ([`InProcessHost`]) or the opt-in L1 out-of-process backend (one
//! [`L1Instance`] child process per instance), behind a single
//! unified [`PluginHandle`] space.
//!
//! Per the L1 design (`docs/designs/plugin-l1-out-of-process.md`,
//! Issue #76) the layer is chosen at **registration time** —
//! [`PluginHost::register_factory`] claims a `plugin_id` for L0 and
//! [`PluginHost::register_out_of_process`] claims one for L1 — so the
//! `Instantiate { plugin_id }` call shape never changes and callers
//! stay layer-agnostic. A `plugin_id` claimed twice —
//! in either order, across either route — is the defined rejection
//! `Err(HostError::DuplicatePluginId)` on `register_out_of_process`;
//! `register_factory` keeps [`InProcessHost`]'s infallible
//! signature, so an id already claimed (L0 *or* L1) leaves the
//! existing route in place: the first registration wins and the
//! shadowed factory stays registered with the L0 backend exactly as
//! `InProcessHost` would keep it (reachable by `Scan`, shadowed at
//! `Instantiate` — L0's first-match semantics carried over).
//!
//! The unified slot table mints every public [`PluginHandle`] itself
//! under the same slot/generation rules [`InProcessHost`] uses
//! (generation bumps on slot reuse so a stale handle is rejected,
//! never aliased): the table maps each handle to the L0 *inner*
//! handle or the owning L1 instance, so dispatch can never
//! cross-route — a stale L0 handle fails the generation check before
//! any routing happens.
//!
//! ## L1 `Destroy` legality
//!
//! [`L1Instance::destroy`] is unconditional by design (control-plane
//! teardown primitive), so the L0 legality rule — `Destroy` is legal
//! only from `Created`/`Initialized`/`Failed` — is enforced **here**:
//! the composite gates it on the instance's lifecycle mirror, the
//! exact check [`InProcessHost`] performs on its own `Lifecycle`.

use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;

use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, LifecycleEvent, LifecycleState, PluginDescriptor, ProcessResult, TransportInfo,
};

use crate::l1::{ChildExit as L1ChildExit, L1Instance, SpawnSpec};
use crate::{
    ErasedPlugin, HostCommand, HostError, HostInput, HostOutput, HostReply, InProcessHost,
    PluginFactory, PluginHandle,
};

/// Spawn parameters for one out-of-process (L1) plugin route —
/// [`PluginHost::register_out_of_process`]'s declaration of how each
/// `Instantiate` of `descriptor.id` reaches its child process.
///
/// `descriptor` is the registration-supplied `'static` value that
/// carries the public `Scan`/`Instantiate` surface for this route
/// and is field-matched against the child's own `Scanned` report at
/// handshake (a mismatch fails the spawn with [`HostError::Ipc`]);
/// `program`/`args` name the runner executable.
///
/// **RT contract:** non-RT only — plain registration data.
#[derive(Debug)]
pub struct OutOfProcessSpec {
    /// The `'static` descriptor serving `Scan`/`Instantiate` and the
    /// handshake's field-equality oracle.
    pub descriptor: PluginDescriptor,
    /// Runner executable (e.g. a binary built on
    /// `ruviform_plugin_export::run`).
    pub program: OsString,
    /// Extra arguments passed before the `--ruviform-*` bootstrap
    /// flags.
    pub args: Vec<OsString>,
    /// Lookahead depth in blocks (default **2**) — fixes the L1
    /// latency contract `lookahead_blocks × block_frames /
    /// sample_rate`. `0` is rejected at spawn (`Err(HostError::Ipc)`)
    /// because the result horizon could never be met.
    pub lookahead_blocks: u32,
    /// Defined timeout for the handshake and every later `Reply`
    /// wait (default **10 s**); a timed-out command converges the
    /// instance to `Failed`.
    pub command_timeout: Duration,
}

impl OutOfProcessSpec {
    /// A spec with the design defaults (`lookahead_blocks` 2,
    /// `command_timeout` 10 s); adjust the fields as needed.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new(descriptor: PluginDescriptor, program: OsString) -> Self {
        Self {
            descriptor,
            program,
            args: Vec::new(),
            lookahead_blocks: 2,
            command_timeout: Duration::from_secs(10),
        }
    }
}

/// How the child process behind an L1 instance ended — the `exit`
/// payload of [`HostNotification::InstanceFailed`]. `signal` is
/// `Some` only on unix.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChildExit {
    /// Exit code, if the child exited with one (`None` when killed
    /// by a signal on unix, or when the status was not yet observed
    /// — the exit the driver later reaps completes the
    /// notification).
    pub code: Option<i32>,
    /// Terminating signal, unix only (`None` on Windows).
    pub signal: Option<i32>,
}

impl From<L1ChildExit> for ChildExit {
    fn from(exit: L1ChildExit) -> Self {
        Self {
            code: exit.code,
            signal: exit.signal,
        }
    }
}

/// Asynchronous instance event surfaced by
/// [`PluginHost::try_recv_notification`]. L0 instances never emit
/// notifications; the queue only carries L1 `Failed` transitions
/// (exactly once while the instance is live, after the child's exit
/// status is known — a notification still queued when `Destroy`
/// frees the slot drops with it).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new notification kinds may be added without breaking
/// downstream `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type; the recv itself
/// is non-RT.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostNotification {
    /// An L1 instance's child process died (or broke the protocol)
    /// while live — the instance is now `LifecycleState::Failed`.
    InstanceFailed {
        /// The failed instance's handle — the same unified
        /// [`PluginHandle`] `Instantiate` returned.
        handle: PluginHandle,
        /// How the child exited.
        exit: ChildExit,
    },
}

/// How a registered `plugin_id` is served — decided at registration
/// and fixed for the id's lifetime.
enum Route {
    /// `register_factory` — instances run in this process through
    /// [`InProcessHost`].
    InProcess,
    /// `register_out_of_process` — every `Instantiate` spawns one
    /// child process per the spec.
    OutOfProcess(SpawnSpec),
}

/// One live entry of the unified slot table — the `PluginHandle`
/// dispatch target.
enum Slot {
    /// L0 instance — the *inner* handle [`InProcessHost`] minted;
    /// commands are retargeted onto it.
    InProcess(PluginHandle),
    /// L1 instance — the child-process proxy owns it end to end.
    /// Boxed: `L1Instance` is ~500 B; the slot table stays compact.
    OutOfProcess(Box<L1Instance>),
}

/// The composite host: a single [`HostCommand`]/[`HostReply`]/
/// [`HostError`] surface plus the RT `process`/`plugin_mut`/
/// `lifecycle_state` accessors, routing every `plugin_id` to its
/// registered L0 (in-process) or L1 (out-of-process) backend.
///
/// The command series is identical to [`InProcessHost`]'s — scan →
/// instantiate → lifecycle → state → destroy — so a caller switching
/// a `plugin_id` between routes changes registration only. Handles
/// come from one unified slot table under L0's slot/generation
/// rules; [`PluginHost::plugin_mut`] returns `None` for L1 slots
/// (an out-of-process instance has no [`ErasedPlugin`] to lend).
///
/// **Latency contract (L1):** a block submitted to an L1 instance is
/// answered `lookahead_blocks` blocks later — latency
/// = `lookahead_blocks × block_frames / sample_rate`
/// (default 2 × 512 frames @ 48 kHz ≈ 21.3 ms). An answer that never
/// arrives is not an error: the defined pass-through (last received
/// block, else silence) plays instead.
///
/// **RT contract:** [`PluginHost::process`] and
/// [`PluginHost::plugin_mut`] are RT-suitable — L0 slots take the
/// direct-call path and L1 slots take the `try_push`/`try_pop` legs
/// plus preallocated staging; neither allocates, blocks or touches
/// pipes/shm on the caller thread. [`PluginHost::lifecycle_state`]
/// is const-cheap for L0 slots but takes the L1 instance's shared
/// mutex — treat it as non-RT for L1 handles. `execute`,
/// `register_*` and `try_recv_notification` are **non-RT only**.
///
/// **Send/Sync:** `PluginHost` is neither `Send` nor `Sync` — the L0
/// backend already holds `Box<dyn ErasedPlugin>` (no auto-traits on
/// the erased object) and each L1 slot owns a `shared_memory::Shmem`
/// (raw pointer, `!Send`) — the composite stays single-threaded like
/// `InProcessHost`.
pub struct PluginHost {
    /// The L0 backend — factories and L0 instances unchanged.
    l0: InProcessHost,
    /// Every accepted registration's descriptor, in registration
    /// order — the `Scan` reply source. L1 entries are the
    /// registration-supplied `spec.descriptor`; a shadowed id still
    /// lists (matching `InProcessHost` reporting every registered
    /// factory) while `Instantiate` honours only the first claimant.
    descriptors: Vec<PluginDescriptor>,
    /// `plugin_id` → serving route; the first registration of an id
    /// claims it (`register_out_of_process` reports the collision as
    /// `DuplicatePluginId`, `register_factory` — infallible — keeps
    /// the existing route and shadows its factory, L0-style).
    routes: HashMap<String, Route>,
    /// The unified slot table; `None` marks a destroyed slot
    /// available for reuse. Slot/generation rules are identical to
    /// [`InProcessHost`]'s.
    instances: Vec<Option<Slot>>,
    /// Per-slot handle generation — survives `destroy`, so a reused
    /// slot's next handle bumps past every stale one.
    generations: Vec<u16>,
    /// Indices of reusable `None` slots.
    free_slots: Vec<u32>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost {
    /// An empty composite host with no registrations.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new() -> Self {
        Self {
            l0: InProcessHost::new(),
            descriptors: Vec::new(),
            routes: HashMap::new(),
            instances: Vec::new(),
            generations: Vec::new(),
            free_slots: Vec::new(),
        }
    }

    /// Registers one plugin type for the in-process (L0) route —
    /// [`InProcessHost::register_factory`]'s exact contract carried
    /// over: infallible, unconditional.
    ///
    /// If `factory.descriptor().id` was already claimed (by either
    /// route) the first registration keeps the route and the factory
    /// is shadowed at `Instantiate` — the same first-match semantics
    /// `InProcessHost` applies to a repeated id (the only fallible
    /// registration surface is [`PluginHost::register_out_of_process`]).
    ///
    /// **RT contract:** non-RT only.
    pub fn register_factory(&mut self, factory: Box<dyn PluginFactory>) {
        // `descriptor()` is queried eagerly here — it keys the route
        // claim and feeds the composite's `Scan` cache — while
        // `InProcessHost` queries it live on its own
        // `Scan`/`Instantiate` path. A non-pure `descriptor()` (one
        // that changes answers between calls) could desync the
        // composite's `Scan` report from the route `Instantiate`
        // serves; that is a plugin-side defect class — a stable
        // descriptor is the factory contract's standing assumption.
        let descriptor = factory.descriptor();
        self.routes
            .entry(descriptor.id.to_owned())
            .or_insert(Route::InProcess);
        self.descriptors.push(descriptor);
        self.l0.register_factory(factory);
    }

    /// Registers one plugin type for the out-of-process (L1) route:
    /// every `Instantiate { plugin_id: spec.descriptor.id }` spawns
    /// one child process running `spec.program`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::DuplicatePluginId`] when `spec.descriptor.id` was
    /// already registered by either route.
    pub fn register_out_of_process(&mut self, spec: OutOfProcessSpec) -> Result<(), HostError> {
        let OutOfProcessSpec {
            descriptor,
            program,
            args,
            lookahead_blocks,
            command_timeout,
        } = spec;
        if self.routes.contains_key(descriptor.id) {
            return Err(HostError::DuplicatePluginId);
        }
        let mut spawn = SpawnSpec::new(descriptor, program);
        spawn.args = args;
        spawn.lookahead_blocks = lookahead_blocks;
        spawn.command_timeout = command_timeout;
        self.routes
            .insert(descriptor.id.to_owned(), Route::OutOfProcess(spawn));
        self.descriptors.push(descriptor);
        Ok(())
    }

    /// Executes one non-RT command against the registry and the
    /// instance it names — the L0 semantics exactly: `Scan` reports
    /// every registration's descriptor in registration order,
    /// `Instantiate` routes by `plugin_id` (the caller never names a
    /// layer), and every handle-carrying verb is dispatched through
    /// the unified slot table — L0 handles are retargeted onto their
    /// inner [`InProcessHost`] handle, L1 handles translate the verb
    /// onto the wire command set.
    ///
    /// **RT contract:** non-RT only — may allocate, spawn child
    /// processes and call non-RT hooks; L1 commands wait at most the
    /// spec's `command_timeout` per reply.
    ///
    /// # Errors
    ///
    /// [`HostError`] — identical meanings to [`InProcessHost`]:
    /// `Lifecycle` for an illegal transition (the child's own
    /// `Lifecycle` decides on L1), `Failed` for any command but
    /// `Destroy` on a failed L1 instance, `Unsupported` for L1 GUI
    /// verbs, `Ipc` for spawn/handshake/timeout failures and
    /// `DuplicatePluginId` is the registration-time rejection.
    pub fn execute(&mut self, command: HostCommand) -> Result<HostReply, HostError> {
        match command {
            HostCommand::Scan => Ok(HostReply::Scanned(self.descriptors.clone())),
            HostCommand::Instantiate { plugin_id } => self.instantiate(&plugin_id),
            HostCommand::Destroy { handle } => self.destroy(handle),
            command => self.dispatch(command),
        }
    }

    /// The instance's current lifecycle state, `None` for an invalid
    /// handle. L1 slots report the proxy's local mirror of the
    /// child-side `Lifecycle` (which stays the authority).
    ///
    /// **RT contract:** const-cheap for L0 slots; **non-RT for L1
    /// slots** — the mirror sits behind the instance's shared mutex.
    #[must_use]
    pub fn lifecycle_state(&self, handle: PluginHandle) -> Option<LifecycleState> {
        let index = handle.slot() as usize;
        if self.generations.get(index).copied() != Some(handle.generation()) {
            return None;
        }
        match self.instances.get(index)?.as_ref()? {
            Slot::InProcess(inner) => self.l0.lifecycle_state(*inner),
            Slot::OutOfProcess(instance) => Some(instance.lifecycle_state()),
        }
    }

    /// Direct access to an L0 instance for the L0 RT path (e.g.
    /// bespoke [`ErasedPlugin::process_host`] dispatch) — `None` for
    /// an invalid handle **and for every L1 slot**: an out-of-process
    /// instance has no in-process plugin object to lend.
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking.
    #[must_use]
    pub fn plugin_mut(&mut self, handle: PluginHandle) -> Option<&mut dyn ErasedPlugin> {
        let index = handle.slot() as usize;
        if self.generations.get(index).copied() != Some(handle.generation()) {
            return None;
        }
        match self.instances.get_mut(index)?.as_mut()? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.plugin_mut(inner)
            }
            Slot::OutOfProcess(_) => None,
        }
    }

    /// The RT processing path: L0 slots take [`InProcessHost`]'s
    /// direct call; L1 slots take the `try`-leg data plane — stage
    /// the input into a preallocated slot, submit the doorbell seq,
    /// drain results up to `seq - lookahead_blocks` and copy out the
    /// matching block, falling back to the defined pass-through (last
    /// received block — silence before the first — and silence for a
    /// `Failed` instance).
    ///
    /// **RT contract:** RT-suitable — the dispatch is a slot-table
    /// lookup; the L1 leg is `try_push`/`try_pop` plus bounded
    /// staging copies only.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::Lifecycle`] when the instance is not
    /// `Processing`; [`HostError::Ipc`] when the block exceeds the
    /// activated frame capacity the wire can express.
    pub fn process(
        &mut self,
        handle: PluginHandle,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.process(inner, inputs, outputs, events, transport)
            }
            Slot::OutOfProcess(instance) => instance.process(inputs, outputs, events, transport),
        }
    }

    /// Pops one queued `Failed` notification — `Some` exactly once
    /// per failed L1 instance while it is live (after the exit
    /// status is known; a notification still queued at `Destroy`
    /// drops with the slot), `None` when nothing has failed. L0
    /// instances never notify.
    ///
    /// **RT contract:** non-RT only — `try_recv`, never blocks.
    ///
    /// # Panics
    ///
    /// Only on an internal invariant violation — every slot index is
    /// minted through `u32`, so the index→`u32` narrowing cannot fail.
    #[must_use]
    pub fn try_recv_notification(&mut self) -> Option<HostNotification> {
        for (index, slot) in self.instances.iter_mut().enumerate() {
            let Some(Slot::OutOfProcess(instance)) = slot else {
                continue;
            };
            if let Some(exit) = instance.try_recv_notification() {
                // The slot is still live (freed slots never carry an
                // instance), so the current generation is the minted
                // handle's.
                let handle = PluginHandle::new(
                    u32::try_from(index).expect("slot index fits u32"),
                    self.generations[index],
                );
                return Some(HostNotification::InstanceFailed {
                    handle,
                    exit: ChildExit::from(exit),
                });
            }
        }
        None
    }

    /// Force-kills an L1 instance's child process — **verification
    /// support**, not a stable contract: drives the crash paths
    /// (`Failed` convergence + `HostNotification::InstanceFailed`)
    /// in tests. L0 slots have no child — `Err(HostError::Unsupported)`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::Unsupported`] for an L0 slot.
    #[doc(hidden)]
    pub fn kill_child(&mut self, handle: PluginHandle) -> Result<(), HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(_) => Err(HostError::Unsupported),
            Slot::OutOfProcess(instance) => {
                instance.kill();
                Ok(())
            }
        }
    }

    /// `Instantiate` — routes by `plugin_id`, then mints the public
    /// handle from the unified slot table regardless of layer.
    fn instantiate(&mut self, plugin_id: &str) -> Result<HostReply, HostError> {
        let slot = match self.routes.get(plugin_id) {
            None => return Err(HostError::UnknownPluginId),
            Some(Route::InProcess) => {
                let HostReply::Instantiated { handle: inner } =
                    self.l0.execute(HostCommand::Instantiate {
                        plugin_id: plugin_id.to_owned(),
                    })?
                else {
                    unreachable!("Instantiate replies only Instantiated")
                };
                Slot::InProcess(inner)
            }
            Some(Route::OutOfProcess(spec)) => {
                Slot::OutOfProcess(Box::new(L1Instance::spawn(spec)?))
            }
        };
        Ok(HostReply::Instantiated {
            handle: self.mint(slot),
        })
    }

    /// `Destroy` — frees the unified slot after the route's own
    /// teardown. L0 delegates to `InProcessHost` (which applies the
    /// lifecycle legality rule itself); L1 applies that same rule
    /// here — `L1Instance::destroy` is unconditional, so the
    /// composite is where `Activated`/`Processing` rejections live —
    /// then runs `Shutdown` + reap.
    fn destroy(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let index = handle.slot() as usize;
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.execute(HostCommand::Destroy { handle: inner })?;
            }
            Slot::OutOfProcess(instance) => {
                // The L0 legality check, mirrored: `Destroy` is legal
                // only from `Created`/`Initialized`/`Failed` —
                // `Activated`/`Processing` reject `Err(Lifecycle)`
                // without touching the child.
                instance
                    .lifecycle_state()
                    .transition(LifecycleEvent::Destroy)
                    .map_err(HostError::Lifecycle)?;
                instance.destroy();
            }
        }
        self.instances[index] = None;
        self.free_slots.push(handle.slot());
        Ok(HostReply::Ok)
    }

    /// Dispatches one handle-carrying command (every verb but
    /// `Scan`/`Instantiate`/`Destroy`) through the unified slot
    /// table.
    fn dispatch(&mut self, command: HostCommand) -> Result<HostReply, HostError> {
        let handle = command_handle(&command);
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.execute(retarget(command, inner))
            }
            Slot::OutOfProcess(instance) => {
                let command = to_wire(command);
                let expected = expected_reply_kind(&command);
                instance
                    .execute(command)
                    .and_then(|reply| host_reply(expected, reply))
            }
        }
    }

    /// The live slot a handle names — generation-checked, so a stale
    /// handle is `Err(HostError::InvalidHandle)` before any routing:
    /// it can never cross-route onto a different layer's instance.
    fn slot_mut(&mut self, handle: PluginHandle) -> Result<&mut Slot, HostError> {
        let index = handle.slot() as usize;
        let live = self.generations.get(index).copied() == Some(handle.generation());
        self.instances
            .get_mut(index)
            .and_then(Option::as_mut)
            .filter(|_| live)
            .ok_or(HostError::InvalidHandle)
    }

    /// Mints a public handle under [`InProcessHost`]'s exact
    /// slot/generation rules: reuse a freed slot with a bumped
    /// generation, else grow the table.
    fn mint(&mut self, slot: Slot) -> PluginHandle {
        if let Some(slot_index) = self.free_slots.pop() {
            let index = slot_index as usize;
            let generation = self.generations[index].wrapping_add(1);
            self.generations[index] = generation;
            self.instances[index] = Some(slot);
            PluginHandle::new(slot_index, generation)
        } else {
            let slot_index = u32::try_from(self.instances.len()).expect("instance slots fit u32");
            self.instances.push(Some(slot));
            self.generations.push(0);
            PluginHandle::new(slot_index, 0)
        }
    }
}

/// The `handle` every dispatchable command carries — `dispatch` is
/// only reached by the handle verbs.
fn command_handle(command: &HostCommand) -> PluginHandle {
    match *command {
        HostCommand::Initialize { handle }
        | HostCommand::Activate { handle, .. }
        | HostCommand::StartProcessing { handle }
        | HostCommand::StopProcessing { handle }
        | HostCommand::Deactivate { handle }
        | HostCommand::Destroy { handle }
        | HostCommand::SaveState { handle }
        | HostCommand::RestoreState { handle, .. }
        | HostCommand::OpenGui { handle, .. }
        | HostCommand::CloseGui { handle } => handle,
        HostCommand::Scan | HostCommand::Instantiate { .. } => {
            unreachable!("Scan/Instantiate never reach dispatch")
        }
    }
}

/// Rewrites a command's `handle` field onto the L0 backend's inner
/// handle — the unified-slot indirection that keeps the public
/// handle space whole.
fn retarget(command: HostCommand, handle: PluginHandle) -> HostCommand {
    match command {
        HostCommand::Initialize { .. } => HostCommand::Initialize { handle },
        HostCommand::Activate { config, .. } => HostCommand::Activate { handle, config },
        HostCommand::StartProcessing { .. } => HostCommand::StartProcessing { handle },
        HostCommand::StopProcessing { .. } => HostCommand::StopProcessing { handle },
        HostCommand::Deactivate { .. } => HostCommand::Deactivate { handle },
        HostCommand::Destroy { .. } => HostCommand::Destroy { handle },
        HostCommand::SaveState { .. } => HostCommand::SaveState { handle },
        HostCommand::RestoreState { bytes, .. } => HostCommand::RestoreState { handle, bytes },
        HostCommand::OpenGui { parent, .. } => HostCommand::OpenGui { handle, parent },
        HostCommand::CloseGui { .. } => HostCommand::CloseGui { handle },
        HostCommand::Scan | HostCommand::Instantiate { .. } => {
            unreachable!("Scan/Instantiate carry no handle")
        }
    }
}

/// The `wire::Command` mirror of a handle-carrying `HostCommand` —
/// the proxy-side translation the design's vocabulary mapping fixes
/// (`Destroy` is `Shutdown` + reap, handled by [`PluginHost`]'s
/// destroy path, not this table).
fn to_wire(command: HostCommand) -> wire::Command {
    match command {
        HostCommand::Initialize { .. } => wire::Command::Initialize,
        HostCommand::Activate { config, .. } => wire::Command::Activate { config },
        HostCommand::StartProcessing { .. } => wire::Command::StartProcessing,
        HostCommand::StopProcessing { .. } => wire::Command::StopProcessing,
        HostCommand::Deactivate { .. } => wire::Command::Deactivate,
        HostCommand::SaveState { .. } => wire::Command::SaveState,
        HostCommand::RestoreState { bytes, .. } => wire::Command::RestoreState { bytes },
        HostCommand::OpenGui { parent, .. } => wire::Command::OpenGui { parent },
        HostCommand::CloseGui { .. } => wire::Command::CloseGui,
        HostCommand::Scan | HostCommand::Instantiate { .. } | HostCommand::Destroy { .. } => {
            unreachable!("Scan/Instantiate/Destroy are not wire verbs here")
        }
    }
}

/// The wire reply payload kind — the pairing oracle [`host_reply`]
/// enforces. `Scanned` needs no variant: no dispatchable verb may
/// answer it (the handshake consumed `Scan`), so it is always an
/// anomaly.
#[derive(Clone, Copy)]
enum ReplyKind {
    /// Every dispatchable verb but `SaveState`/`OpenGui`.
    Ok,
    /// `SaveState`'s state bytes.
    State,
    /// `OpenGui`'s surface token.
    Opened,
}

/// The one legal reply kind for each wire verb — the reply side of
/// `to_wire`'s mapping. The `#[non_exhaustive]` wildcard defaults a
/// verb added by a later `WIRE_VERSION` to `Ok`, fail-closed: its
/// new reply kind pairs with nothing until this table is taught.
fn expected_reply_kind(cmd: &wire::Command) -> ReplyKind {
    match cmd {
        wire::Command::SaveState => ReplyKind::State,
        wire::Command::OpenGui { .. } => ReplyKind::Opened,
        _ => ReplyKind::Ok,
    }
}

/// Maps a wire reply onto the public [`HostReply`] — kind for kind,
/// paired against the one legal answer for the verb that produced
/// it. Any other pairing is a protocol anomaly surfaced as
/// [`HostError::Ipc`]: `Ok` to `SaveState` would silently drop the
/// plugin's state, `State` to `Initialize` would fabricate one, and
/// `Scanned` — or a future kind — can never form a public reply at
/// all (owned wire strings never become `&'static` descriptors, and
/// no public verb asks for it).
fn host_reply(expected: ReplyKind, reply: wire::Reply) -> Result<HostReply, HostError> {
    match (expected, reply) {
        (ReplyKind::Ok, wire::Reply::Ok) => Ok(HostReply::Ok),
        (ReplyKind::State, wire::Reply::State { bytes }) => Ok(HostReply::State { bytes }),
        (ReplyKind::Opened, wire::Reply::Opened { gui }) => Ok(HostReply::Opened { gui }),
        // Every other pairing — a kind mismatch, `Scanned`, or a
        // future reply kind — is a protocol anomaly.
        _ => Err(HostError::Ipc),
    }
}
