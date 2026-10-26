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

use ruviform_audio_core::SampleRate;
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, LifecycleEvent, LifecycleState, PluginDescriptor, PrepareError, ProcessResult,
    TransportInfo,
};

use crate::endpoint::{EndpointBounds, EndpointInner, L1Endpoint, PluginEndpoint};
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
    /// latency contract `lookahead_blocks × activated max_frames`
    /// frames. `0` is rejected at spawn (`Err(HostError::Ipc)`)
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
/// = `lookahead_blocks × activated max_frames` frames
/// (default 2 × 512 = 1024 frames ≈ 21.3 ms at 48 kHz; divide
/// `frames` by the activated sample rate for seconds). An answer
/// that never arrives is not an error: the defined pass-through
/// (last received block, else silence) plays instead.
///
/// **Endpoint surface:** [`PluginHost::prepare_endpoint`] detaches
/// one instance's processing leg into a [`PluginEndpoint`] — `Send`,
/// moved once into the RT-side owner (e.g. a graph node closure) —
/// carrying the same `process`/`latency_frames` vocabulary the host
/// reports. One instance carries at most one endpoint at a time.
/// Video plugin instances never reach this path: every
/// [`VideoPluginInstance`](ruviform_plugin_core::VideoPluginInstance)
/// operation is non-RT, so the endpoint/RT surface simply does not
/// exist for them.
///
/// **RT contract:** [`PluginHost::process`] and
/// [`PluginHost::plugin_mut`] are RT-suitable — L0 slots take the
/// direct-call path and L1 slots take the `try_push`/`try_pop` legs
/// plus preallocated staging; neither allocates, blocks or touches
/// pipes/shm on the caller thread. [`PluginHost::lifecycle_state`]
/// and [`PluginHost::latency_frames`] are `&self` queries —
/// lock-free for L0 slots but they take the L1 instance's shared
/// mutex, so treat them as non-RT for L1 handles. `execute`,
/// `register_*`, `prepare_endpoint` and `try_recv_notification` are
/// **non-RT only**.
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

    /// The instance's pipeline latency in whole frames — `None`
    /// before `Activate` and for an invalid handle. L0 slots answer
    /// `Some(0)` once activated (a direct call adds no pipeline
    /// latency); L1 slots answer `Some(lookahead_blocks × activated
    /// max_frames)` — default 2 × 512 = 1024 frames ≈ 21.3 ms at
    /// 48 kHz (divide `frames` by the activated sample rate for
    /// seconds). This is the same quantity a detached
    /// [`PluginEndpoint::latency_frames`] reported at attach; a
    /// re-`Activate` can change it.
    ///
    /// **RT contract:** `&self`, non-draining. For L0 slots it is a
    /// lock-free mirror/lifecycle read — never the `core` mutex
    /// (the query may be carried on an RT-adjacent lane). For L1
    /// slots it takes the instance's shared control mutex — treat
    /// it as non-RT there, exactly like
    /// [`PluginHost::lifecycle_state`].
    #[must_use]
    pub fn latency_frames(&self, handle: PluginHandle) -> Option<u32> {
        let index = handle.slot() as usize;
        if self.generations.get(index).copied() != Some(handle.generation()) {
            return None;
        }
        match self.instances.get(index)?.as_ref()? {
            Slot::InProcess(inner) => match self.l0.lifecycle_state(*inner) {
                Some(LifecycleState::Activated | LifecycleState::Processing) => Some(0),
                _ => None,
            },
            Slot::OutOfProcess(instance) => instance.latency_frames(),
        }
    }

    /// Direct access to an L0 instance for the L0 RT path (e.g.
    /// bespoke [`ErasedPlugin::process_host`] dispatch) — `None` for
    /// an invalid handle, for every L1 slot **and for an L0
    /// `Shared` slot** (a `Send`-capable plugin lives behind the
    /// shared mutex — a `&mut` cannot be lent out of it, the same
    /// defined answer L1 gives): an out-of-process instance has no
    /// in-process plugin object to lend.
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
    /// activated frame capacity the wire can express;
    /// [`HostError::EndpointUnavailable`] while an L1 slot's RT leg
    /// is checked out to a detached endpoint (the processing
    /// surface moved — retry on the endpoint, not here) **or while
    /// an L0 `Shared` slot's cell is busy** — `try_lock`
    /// `WouldBlock`, a non-RT verb mid-flight on the shared mutex;
    /// the defined busy bypass, `outputs` untouched.
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

    /// Mints the instance's RT processing endpoint — the public
    /// attach surface of the ownership split (Issue #113 →
    /// `docs/designs/engine-plugin-host-integration.md`). The
    /// returned [`PluginEndpoint`] is `Send` and owns the RT leg:
    /// move it exactly once into the RT-side owner (e.g. a graph
    /// node closure) and drive `process` there; this host keeps the
    /// command surface.
    ///
    /// One instance carries at most one endpoint at a time. The
    /// seat frees when the endpoint drops — L0 returns its epoch on
    /// `Drop`, L1 restores the checked-out leg at the next `&mut`
    /// host entry point's drain — after which a fresh
    /// `prepare_endpoint` may attach again.
    ///
    /// Defined rejections, in the pinned public order — handle,
    /// then state, then presence, then bounds:
    ///
    /// | order | condition | result |
    /// | --- | --- | --- |
    /// | handle | stale/unknown handle | `Err(HostError::InvalidHandle)` |
    /// | (1) state | `Created`/`Initialized` | `Err(HostError::Lifecycle(_))` |
    /// | (1) state | `Failed` | `Err(HostError::Failed)` |
    /// | (2) presence | L0 `Plain` slot (a non-`Send` factory — [`PluginFactory::create_sendable`] was `None`) · L0 second attach · L1 leg checked out | `Err(HostError::EndpointUnavailable)` |
    /// | (3) bounds | `max_frames == 0` · `channels == 0` · `max_frames > WIRE_MAX_FRAMES` (8192) · `channels > WIRE_MAX_CHANNELS` (16) · `max_frames <` activated `max_frames` | `Err(HostError::Prepare(PrepareError::UnsupportedConfig))` |
    /// | internal | L1 post-take assembly anomaly — unreachable in-protocol (`debug_assert`'d; the checked-out leg is restored before the `Err` returns) | `Err(HostError::Ipc)` |
    ///
    /// Presence is checked **before** bounds on purpose — an
    /// occupied or incapable slot answers `EndpointUnavailable`
    /// even when the bounds are also invalid (the inner attach
    /// paths check bounds before their own occupancy claim; the
    /// public order is the composite's, per the design's rejection
    /// table). The bounds check is split the same way the contract
    /// splits it: the *uniform* half runs here once — non-zero and
    /// inside the wire caps, so a `u32::MAX` shape is the same
    /// defined rejection and can never reach an allocation — and
    /// the activated-layer half (`max_frames ≥ activated
    /// max_frames`) is re-validated inside the route's attach,
    /// which keeps its own pinned order. `bounds.channels` carries
    /// no activated-bus equality rule in v1 (plugins expose no
    /// channel query); the per-block rule `out.channels() ==
    /// bound.channels` is the endpoint's bound clip.
    ///
    /// A rejected `prepare_endpoint` consumes nothing: the instance
    /// stays attachable and processable exactly as found.
    ///
    /// **RT contract:** non-RT only — per-attach channel minting,
    /// the bound-shaped `in_scratch` allocation and the shared
    /// locks — the L1 instance's shared control mutex, the L0
    /// `Shared` slot's `core` cell lock — are all prepare-side
    /// costs. The minted endpoint's
    /// `process`/`latency_frames`/`bounds` are the RT legs.
    ///
    /// # Errors
    ///
    /// Per the rejection table above — every rejection is a defined
    /// `Err`, never a panic or an allocation abort.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::SampleRate;
    /// use ruviform_plugin_core::{
    ///     BufferView, BufferViewMut, EventList, ParamInfo, PluginDescriptor,
    ///     PluginProcessor, PrepareError, ProcessConfig, ProcessContext,
    ///     ProcessResult, ProcessStatus, TransportInfo,
    /// };
    /// use ruviform_plugin_host::{
    ///     EndpointBounds, ErasedPlugin, HostCommand, HostReply, Plugin,
    ///     PluginFactory, PluginHost, erase, erase_sendable,
    /// };
    ///
    /// // A `Send`-capable plugin opts into endpoint sharing — its
    /// // factory implements `create_sendable`.
    /// struct Gain;
    /// impl PluginProcessor for Gain {
    ///     fn prepare(
    ///         &mut self,
    ///         _config: &ProcessConfig,
    ///         _context: &mut ProcessContext,
    ///     ) -> Result<(), PrepareError> {
    ///         Ok(())
    ///     }
    ///     fn process<I, O>(
    ///         &mut self,
    ///         inputs: &[I],
    ///         outputs: &mut [O],
    ///         _events: EventList<'_>,
    ///         _transport: Option<TransportInfo>,
    ///         _context: &mut ProcessContext,
    ///     ) -> ProcessResult
    ///     where
    ///         I: BufferView,
    ///         O: BufferViewMut,
    ///     {
    ///         for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
    ///             for channel in 0..output.channels() {
    ///                 if let (Some(src), Some(dst)) =
    ///                     (input.channel(channel), output.channel_mut(channel))
    ///                 {
    ///                     dst.copy_from_slice(src);
    ///                 }
    ///             }
    ///         }
    ///         ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
    ///     }
    /// }
    /// impl Plugin for Gain {
    ///     fn descriptor(&self) -> PluginDescriptor {
    ///         PluginDescriptor {
    ///             id: "org.example.gain",
    ///             name: "Gain",
    ///             vendor: "Example",
    ///             version: "1.0.0",
    ///             features: &[],
    ///         }
    ///     }
    ///     fn params(&self) -> &[ParamInfo] {
    ///         &[]
    ///     }
    /// }
    /// struct GainFactory;
    /// impl PluginFactory for GainFactory {
    ///     fn descriptor(&self) -> PluginDescriptor {
    ///         Gain.descriptor()
    ///     }
    ///     fn create(&self) -> Box<dyn ErasedPlugin> {
    ///         erase(Gain)
    ///     }
    ///     fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
    ///         Some(erase_sendable(Gain))
    ///     }
    /// }
    ///
    /// let mut host = PluginHost::new();
    /// host.register_factory(Box::new(GainFactory));
    /// let HostReply::Instantiated { handle } = host
    ///     .execute(HostCommand::Instantiate {
    ///         plugin_id: "org.example.gain".to_owned(),
    ///     })
    ///     .unwrap()
    /// else {
    ///     panic!("unexpected reply")
    /// };
    /// host.execute(HostCommand::Initialize { handle }).unwrap();
    /// host.execute(HostCommand::Activate {
    ///     handle,
    ///     config: ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap(),
    /// })
    /// .unwrap();
    ///
    /// // The endpoint detaches the processing leg — `Send`, ready to
    /// // move into the RT-side owner. L0 reports zero added latency.
    /// let endpoint = host
    ///     .prepare_endpoint(
    ///         handle,
    ///         EndpointBounds {
    ///             max_frames: 512,
    ///             channels: 2,
    ///         },
    ///     )
    ///     .unwrap();
    /// assert_eq!(endpoint.latency_frames(), 0);
    /// assert_eq!(host.latency_frames(handle), Some(0));
    /// ```
    pub fn prepare_endpoint(
        &mut self,
        handle: PluginHandle,
        bounds: EndpointBounds,
    ) -> Result<PluginEndpoint, HostError> {
        // The prelude — the pinned public order. `slot_mut` resolves
        // the handle first (stale/unknown → `InvalidHandle`), then
        // each route runs (1) state and (2) presence/occupancy
        // before the uniform bounds check below: a state rejection
        // always wins, and an occupied or incapable slot answers
        // `EndpointUnavailable` even for invalid bounds.
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.check_attachable_state(inner)?;
                self.l0.check_endpoint_free(inner)?;
            }
            Slot::OutOfProcess(instance) => {
                // The `&mut` entry-point drain — collects a leg
                // queued on the return channel so the occupancy
                // read below sees post-return truth (the inner
                // `prepare_endpoint` re-runs the same drain).
                instance.drain_endpoint_channels();
                instance.check_attachable_state()?;
                if instance.leg_detached() {
                    return Err(HostError::EndpointUnavailable);
                }
            }
        }
        // The uniform bounds half — the layer-independent rules
        // only; `max_frames ≥ activated max_frames` stays with the
        // inners' activated-layer re-validation.
        check_uniform_bounds(bounds)?;
        self.attach_endpoint(handle, bounds)
    }

    /// The inner entry point for endpoint attachment (Issue #113
    /// task 3) — routes the handle to the L0 or L1 attach path.
    /// [`PluginHost::prepare_endpoint`] calls it after the public
    /// prelude (state → presence → uniform bounds); the pinned
    /// per-route order inside is the activated layer's own
    /// defense-in-depth re-validation, not the public contract's
    /// order.
    ///
    /// L0 delegates to [`InProcessHost::attach_endpoint`] — its
    /// `Shared`-slot gate already distinguishes attachable
    /// instances. L1 checks the leg out of the
    /// [`L1Instance`](crate::l1::L1Instance) — `prepare_endpoint`
    /// owns the pinned rejection order (state → bounds-before-take
    /// → leg extraction with post-take restore) — then assembles
    /// [`EndpointInner::L1`], the bound-shaped `in_scratch` at the
    /// activated sample rate and the instance's pipeline latency.
    ///
    /// **RT contract:** non-RT only — channel minting, the
    /// `last_output` reserve and `in_scratch` all allocate.
    ///
    /// # Errors
    ///
    /// [`HostError`] — per the route's own pinned order
    /// (`InvalidHandle` first from [`PluginHost::slot_mut`]).
    pub(crate) fn attach_endpoint(
        &mut self,
        handle: PluginHandle,
        bounds: EndpointBounds,
    ) -> Result<PluginEndpoint, HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.attach_endpoint(inner, bounds)
            }
            Slot::OutOfProcess(instance) => {
                // `prepare_endpoint` runs the drain, the state/bounds
                // rejections and the take/restore contract itself.
                let parts = instance.prepare_endpoint(bounds.max_frames, bounds.channels)?;
                // The leg is checked out now — assemble the
                // endpoint first so a post-take failure can restore:
                // `drop` pushes the leg onto the return channel and
                // `drain_endpoint_channels` moves it home, leaving
                // the instance attachable/processable (the
                // `prepare_endpoint` post-take contract).
                let endpoint = L1Endpoint::from(parts);
                // `Activated`/`Processing` implies a staging ring,
                // which implies a recorded sample rate — and
                // `in_scratch` for bound-validated shapes cannot
                // fail. Both `Err` legs are therefore unreachable
                // today, but they still restore the leg rather
                // than leak the checkout.
                let assembled = instance
                    .activated_sample_rate()
                    .and_then(|rate| SampleRate::new(rate).ok())
                    .ok_or(HostError::Ipc)
                    .and_then(|rate| crate::endpoint::in_scratch(bounds, rate));
                let in_scratch = match assembled {
                    Ok(scratch) => scratch,
                    Err(error) => {
                        debug_assert!(false, "post-take assembly is infallible");
                        drop(endpoint);
                        instance.drain_endpoint_channels();
                        return Err(error);
                    }
                };
                Ok(PluginEndpoint::new(
                    EndpointInner::L1(Box::new(endpoint)),
                    bounds,
                    in_scratch,
                    // `Activated`/`Processing` implies an armed ring
                    // — `armed_latency_frames` is `Some` there. The
                    // ungated read is deliberate: the endpoint
                    // carries the armed horizon its leg holds even
                    // if convergence flips the mirror mid-attach
                    // (the public `latency_frames` gate must not
                    // leak into the bookkeeping value).
                    instance.armed_latency_frames().unwrap_or(0),
                ))
            }
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
            // The `&mut` entry-point drain — restores a returned leg
            // and re-sends/drains the arm channels before the
            // notification check (`&self` queries such as
            // `lifecycle_state` stay deliberately non-draining).
            instance.drain_endpoint_channels();
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

/// The uniform half of the attach bounds contract — the
/// layer-independent rules [`PluginHost::prepare_endpoint`] checks
/// at step (3) of the pinned public order (after state and
/// presence, before the route's inner attach). Every violation is
/// the defined `Err(Prepare(UnsupportedConfig))`: non-zero
/// dimensions, inside the wire caps — so a `u32::MAX` shape is the
/// same rejection and never reaches an allocation. The
/// layer-dependent half (`max_frames ≥ activated max_frames`) is
/// deliberately not here: it stays with the inner attach paths,
/// which own the activated-layer contract.
fn check_uniform_bounds(bounds: EndpointBounds) -> Result<(), HostError> {
    if bounds.max_frames == 0
        || bounds.channels == 0
        || bounds.max_frames > wire::WIRE_MAX_FRAMES
        || bounds.channels > wire::WIRE_MAX_CHANNELS
    {
        return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    //! `attach_endpoint` route tests (Issue #113 task 3): the L1
    //! arm must reach `L1Instance::prepare_endpoint` — the leg
    //! checkout — never the L0 `InstanceSlot::Shared` path, and the
    //! `&mut` entry points must drain the endpoint channels.

    use std::panic::{AssertUnwindSafe, catch_unwind};

    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_plugin_core::{
        BufferView, BufferViewMut, ParamInfo, PluginDescriptor, PluginProcessor, PrepareError,
        ProcessConfig, ProcessContext, ProcessStatus,
    };

    use super::*;
    use crate::{Plugin, erase, erase_sendable};

    const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.composite-gain",
        name: "Test Composite Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    /// Minimal `Send` gain — the `Shared`-slot fixture for the L0
    /// route check (same shape as `host.rs`'s `SendGain`; the
    /// per-module duplication is intentional).
    struct SendGain {
        gain: f32,
    }

    impl PluginProcessor for SendGain {
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
            ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
        }
    }

    impl Plugin for SendGain {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    struct SendGainFactory;

    impl PluginFactory for SendGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(SendGain { gain: 2.0 })
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            Some(erase_sendable(SendGain { gain: 2.0 }))
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

    /// One `PluginHost::process` block over mono planar views.
    fn run_block(host: &mut PluginHost, handle: PluginHandle) -> Result<ProcessResult, HostError> {
        let input = [0.5_f32; 8];
        let mut output = [0.0_f32; 8];
        let in_list = [&input[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut output[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        host.process(handle, &inputs, &mut outputs, EventList::empty(8), None)
    }

    /// The L0 arm: an `InProcess` slot delegates to
    /// `InProcessHost::attach_endpoint` — a `Shared`-slot L0
    /// instance attaches exactly as it would without the composite.
    #[test]
    fn attach_endpoint_routes_l0_slots_to_inprocess() {
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        let HostReply::Instantiated { handle } = host
            .execute(HostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        else {
            panic!("unexpected reply")
        };
        host.execute(HostCommand::Initialize { handle }).unwrap();
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 512),
        })
        .unwrap();
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
        let endpoint = host.attach_endpoint(handle, bounds(512, 2)).unwrap();
        assert_eq!(endpoint.bounds(), bounds(512, 2));
        // L0 is a direct call — no pipeline latency.
        assert_eq!(endpoint.latency_frames(), 0);
    }

    /// The L1 arm: an `OutOfProcess` slot checks the leg out of the
    /// `L1Instance` — it must not be mistaken for an
    /// `InstanceSlot::Shared`. The minted endpoint carries the
    /// bound, the armed ring's pipeline latency and the single-
    /// endpoint occupancy; `process` while detached is
    /// `EndpointUnavailable`; the `&mut` `try_recv_notification`
    /// drain restores the returned leg for a fresh attach.
    #[test]
    fn attach_endpoint_routes_l1_slots_to_the_leg() {
        let mut host = PluginHost::new();
        let handle = host.mint(Slot::OutOfProcess(Box::new(L1Instance::test_fixture(
            LifecycleState::Processing,
            Some(512),
        ))));
        let bounds = bounds(512, 2);
        let endpoint = host.attach_endpoint(handle, bounds).unwrap();
        assert_eq!(endpoint.bounds(), bounds);
        // `(depth - 1) × armed frame_capacity` — 2 × 512.
        assert_eq!(endpoint.latency_frames(), 1024);
        // Single-endpoint rule — the leg is checked out.
        assert!(matches!(
            host.attach_endpoint(handle, bounds),
            Err(HostError::EndpointUnavailable)
        ));
        // Detached host-side `process` — the defined
        // `EndpointUnavailable`, not `Ipc`.
        assert!(matches!(
            run_block(&mut host, handle),
            Err(HostError::EndpointUnavailable)
        ));
        // Endpoint drop returns the leg; the `&mut`
        // `try_recv_notification` drain restores it.
        drop(endpoint);
        assert!(host.try_recv_notification().is_none());
        assert_eq!(
            run_block(&mut host, handle).unwrap(),
            ProcessResult::CONTINUE
        );
        // Freshly attachable — a second attach mints cleanly.
        let _again = host.attach_endpoint(handle, bounds).unwrap();
    }

    // ---------------------------------------------------------------
    // `prepare_endpoint` / `latency_frames` — the public endpoint
    // surface (Issue #113 task 4): the pinned rejection order and
    // the unified defined-rejection table on both routes.
    // ---------------------------------------------------------------

    /// A factory that never opts into `create_sendable` — its
    /// instances stay `Plain` (not endpoint-capable) and answer
    /// `EndpointUnavailable`.
    struct PlainGainFactory;

    impl PluginFactory for PlainGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(SendGain { gain: 1.0 })
        }
        // `create_sendable` keeps the default `None`.
    }

    /// A `Send` plugin whose `process` panics — the endpoint's
    /// `catch_unwind` publishes `Failed` onto the shared mirror
    /// (the union's second writer), the observation path the L0
    /// `Failed`-rejection test needs.
    struct PanicGain;

    impl PluginProcessor for PanicGain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            _inputs: &[I],
            _outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            panic!("panic gain must never process again")
        }
    }

    impl Plugin for PanicGain {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    struct PanicGainFactory;

    impl PluginFactory for PanicGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(PanicGain)
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            Some(erase_sendable(PanicGain))
        }
    }

    /// Register `factory`, instantiate its plugin and drive the
    /// instance to `Processing` — `Activated` when `start` is false.
    fn l0_instance(factory: Box<dyn PluginFactory>, start: bool) -> (PluginHost, PluginHandle) {
        let mut host = PluginHost::new();
        host.register_factory(factory);
        let HostReply::Instantiated { handle } = host
            .execute(HostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        else {
            panic!("unexpected reply")
        };
        host.execute(HostCommand::Initialize { handle }).unwrap();
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 512),
        })
        .unwrap();
        if start {
            host.execute(HostCommand::StartProcessing { handle })
                .unwrap();
        }
        (host, handle)
    }

    /// An L1 slot from `L1Instance::test_fixture` — childless, but
    /// the endpoint machinery (leg checkout, mirror, arm channels)
    /// is fully exercisable.
    fn l1_instance(state: LifecycleState, frames: Option<u32>) -> (PluginHost, PluginHandle) {
        let mut host = PluginHost::new();
        let handle = host.mint(Slot::OutOfProcess(Box::new(L1Instance::test_fixture(
            state, frames,
        ))));
        (host, handle)
    }

    /// One `PluginEndpoint::process` block over mono planar views.
    fn run_endpoint(endpoint: &mut PluginEndpoint) -> Result<ProcessResult, HostError> {
        let input = [0.5_f32; 8];
        let mut output = [9.0_f32; 8];
        let in_list = [&input[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut output[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        endpoint.process(&inputs, &mut outputs, EventList::empty(8), None)
    }

    /// The public mint attaches on both routes — L0 through the
    /// `Shared`-slot inner, L1 through the leg checkout — and the
    /// minted endpoint reports the route's own latency figure.
    #[test]
    fn prepare_endpoint_mints_both_routes() {
        let (mut host, handle) = l0_instance(Box::new(SendGainFactory), true);
        let endpoint = host
            .prepare_endpoint(handle, bounds(512, 2))
            .expect("L0 Shared attach");
        assert_eq!(endpoint.bounds(), bounds(512, 2));
        assert_eq!(endpoint.latency_frames(), 0);
        assert_eq!(host.latency_frames(handle), Some(0));

        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        let endpoint = host
            .prepare_endpoint(handle, bounds(512, 2))
            .expect("L1 leg checkout");
        assert_eq!(endpoint.bounds(), bounds(512, 2));
        // `(depth - 1) × armed frame_capacity` — 2 × 512.
        assert_eq!(endpoint.latency_frames(), 1024);
        assert_eq!(host.latency_frames(handle), Some(1024));
    }

    /// The pinned public order — handle → state → presence → bounds.
    /// A state rejection wins over invalid bounds; an occupied or
    /// incapable slot answers `EndpointUnavailable` even when the
    /// bounds are also invalid (the composite's order, deliberately
    /// the inverse of the L0 inner's bounds-before-CAS).
    #[test]
    fn prepare_endpoint_pins_the_rejection_order() {
        // (1) state beats bounds — `Created` with zero frames is
        // `Lifecycle`, never `Prepare`.
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        let HostReply::Instantiated { handle } = host
            .execute(HostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        else {
            panic!("unexpected reply")
        };
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 2)),
            Err(HostError::Lifecycle(_))
        ));
        // `Initialized` is still below `Activated` — same answer.
        host.execute(HostCommand::Initialize { handle }).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 2)),
            Err(HostError::Lifecycle(_))
        ));

        // (2) presence beats bounds — an occupied L0 slot answers
        // `EndpointUnavailable` even though the bound is invalid.
        let (mut host, handle) = l0_instance(Box::new(SendGainFactory), true);
        let _endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 0)),
            Err(HostError::EndpointUnavailable)
        ));
        // The L1 route — a checked-out leg is the same occupancy.
        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        let _endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 0)),
            Err(HostError::EndpointUnavailable)
        ));
        // A `Plain` slot is never endpoint-capable — invalid bounds
        // do not change the answer.
        let (mut host, handle) = l0_instance(Box::new(PlainGainFactory), true);
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 0)),
            Err(HostError::EndpointUnavailable)
        ));

        // Handle beats everything — stale (post-`Destroy`) and
        // never-minted handles answer `InvalidHandle` before any
        // state, presence or bounds check runs.
        let (mut host, handle) = l0_instance(Box::new(SendGainFactory), false);
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        host.execute(HostCommand::Destroy { handle }).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 0)),
            Err(HostError::InvalidHandle)
        ));
        assert!(matches!(
            host.prepare_endpoint(PluginHandle::new(77, 0), bounds(0, 0)),
            Err(HostError::InvalidHandle)
        ));
    }

    /// `Failed` — the union observation — is `Err(Failed)` on both
    /// routes: L1's mirror reports the terminal state directly;
    /// L0's mirror is published by the endpoint itself when a
    /// panicking plugin is caught (`fetch_max` second writer), and
    /// the public state gate converges the bookkeeping lifecycle.
    #[test]
    fn prepare_endpoint_rejects_failed() {
        let (mut host, handle) = l1_instance(LifecycleState::Failed, Some(512));
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::Failed)
        ));

        // L0 — drive a block through a panicking `Send` plugin on
        // the endpoint; the catch publishes `Failed` to the mirror.
        let (mut host, handle) = l0_instance(Box::new(PanicGainFactory), true);
        let mut endpoint = host.prepare_endpoint(handle, bounds(512, 1)).unwrap();
        assert_eq!(
            run_endpoint(&mut endpoint).unwrap(),
            ProcessResult::CONTINUE
        );
        // The union now reports `Failed` — the state gate converges
        // the bookkeeping lifecycle and rejects the next attach.
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 1)),
            Err(HostError::Failed)
        ));
        assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));
    }

    /// A poisoned `Shared` cell is the union's other `Failed` half
    /// — host-side `process` has no `catch_unwind`, so a panicking
    /// plugin unwinds through the held `core` guard and poisons it
    /// while the mirror keeps its last committed `Processing`
    /// store. The prelude's state gate reads the poison flag the
    /// same way the inner `lock()` arm and `destroy` do, so the
    /// pinned state-before-bounds order reports `Err(Failed)` even
    /// for invalid bounds — before the fix the stale-`Processing`
    /// mirror let the same call fall through to `Err(Prepare)`.
    #[test]
    fn prepare_endpoint_observes_poisoned_cell_as_failed() {
        let (mut host, handle) = l0_instance(Box::new(PanicGainFactory), true);
        // The panic propagates out of `PluginHost::process` — and
        // unwinds through the `core` guard on the way, poisoning
        // the shared cell while the mirror stays `Processing`.
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _ = run_block(&mut host, handle);
        }));
        assert!(panicked.is_err());
        // Invalid bounds cannot mask the `Failed` observation —
        // state precedes bounds in the pinned order.
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(0, 0)),
            Err(HostError::Failed)
        ));
        // Valid bounds — the same `Failed` answer the inner
        // `lock()` arm would have converged to.
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::Failed)
        ));
    }

    /// The bounds matrix — every violation is the defined
    /// `Err(Prepare(UnsupportedConfig))`, `u32::MAX` included (a
    /// defined rejection, never an allocation abort), and consumes
    /// nothing: the instance stays attachable afterwards.
    #[test]
    fn prepare_endpoint_bounds_matrix() {
        for (max_frames, channels) in [
            (0, 2),
            (512, 0),
            (wire::WIRE_MAX_FRAMES + 1, 2),
            (u32::MAX, 2),
            (512, wire::WIRE_MAX_CHANNELS + 1),
            (512, u32::MAX),
            // Below the activated 512 — the inner layer's own check.
            (256, 2),
        ] {
            let (mut host, handle) = l0_instance(Box::new(SendGainFactory), true);
            assert!(
                matches!(
                    host.prepare_endpoint(handle, bounds(max_frames, channels)),
                    Err(HostError::Prepare(PrepareError::UnsupportedConfig))
                ),
                "L0 bounds ({max_frames}, {channels}) must reject UnsupportedConfig"
            );
            // A rejected attach consumed nothing — still attachable.
            host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        }
        // The same matrix on L1 — a rejected attach never moves the
        // leg, so the instance stays attachable.
        for (max_frames, channels) in [
            (0, 2),
            (512, 0),
            (wire::WIRE_MAX_FRAMES + 1, 2),
            (u32::MAX, 2),
            (512, wire::WIRE_MAX_CHANNELS + 1),
            (512, u32::MAX),
            (256, 2),
        ] {
            let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
            assert!(
                matches!(
                    host.prepare_endpoint(handle, bounds(max_frames, channels)),
                    Err(HostError::Prepare(PrepareError::UnsupportedConfig))
                ),
                "L1 bounds ({max_frames}, {channels}) must reject UnsupportedConfig"
            );
            host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        }
    }

    /// `EndpointUnavailable` stays a host-local topology fact —
    /// never `Ipc`/`Unsupported` — on every occupancy rejection,
    /// including the host-side `process` of a detached L1 leg; and
    /// a returned leg makes the instance attachable again.
    #[test]
    fn endpoint_unavailable_stays_local() {
        // L0 — the epoch claim makes the second attach `EU`.
        let (mut host, handle) = l0_instance(Box::new(SendGainFactory), true);
        let _endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::EndpointUnavailable)
        ));
        // L1 — the checked-out leg reports `EU` on both surfaces:
        // the second attach and the host-side `process` alike.
        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::EndpointUnavailable)
        ));
        assert!(matches!(
            run_block(&mut host, handle),
            Err(HostError::EndpointUnavailable)
        ));
        // Dropping the endpoint returns the leg through the
        // channel; the next `&mut` entry point's drain restores it
        // and a fresh attach mints cleanly.
        drop(endpoint);
        assert!(host.try_recv_notification().is_none());
        host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    }

    /// `latency_frames` — L0 `Some(0)` once activated, `None`
    /// before; L1 `Some(lookahead × armed capacity)` while a ring
    /// is armed, `None` without one; `None` for a stale handle.
    /// The `&self` query is non-draining: a live endpoint's leg
    /// stays checked out.
    #[test]
    fn latency_frames_reports_per_route() {
        // L0 — `Created`/`Initialized` report `None`; activated
        // reports `Some(0)`.
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        let HostReply::Instantiated { handle } = host
            .execute(HostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        else {
            panic!("unexpected reply")
        };
        assert_eq!(host.latency_frames(handle), None);
        host.execute(HostCommand::Initialize { handle }).unwrap();
        assert_eq!(host.latency_frames(handle), None);
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 512),
        })
        .unwrap();
        assert_eq!(host.latency_frames(handle), Some(0));
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
        assert_eq!(host.latency_frames(handle), Some(0));

        // L1 — `Some(lookahead × armed capacity)`, `None` unarmed.
        let (host, handle) = l1_instance(LifecycleState::Activated, Some(512));
        assert_eq!(host.latency_frames(handle), Some(1024));
        let (host, handle) = l1_instance(LifecycleState::Processing, Some(256));
        assert_eq!(host.latency_frames(handle), Some(512));
        let (host, handle) = l1_instance(LifecycleState::Created, None);
        assert_eq!(host.latency_frames(handle), None);

        // Stale handle — `None`.
        assert_eq!(host.latency_frames(PluginHandle::new(77, 0)), None);

        // Non-draining `&self`: the query cannot restore a leg a
        // live endpoint still holds.
        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        let _endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
        assert_eq!(host.latency_frames(handle), Some(1024));
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::EndpointUnavailable)
        ));
    }

    /// The L1 `latency_frames` read gates on the lifecycle mirror —
    /// parity with the L0 `lifecycle ∈ {Activated, Processing}`
    /// gate. The armed ring is never cleared, so a bare `staging`
    /// read would keep reporting the stale horizon past
    /// `Deactivate`/`Failed`; the fixture pins exactly the
    /// observable state those transitions leave behind
    /// (`Initialized`/`Failed` mirror + `staging` still armed — a
    /// real `Deactivate` verified end-to-end in
    /// `tests/l1_proxy_e2e.rs`'s lifecycle parity test).
    #[test]
    fn latency_frames_gates_l1_on_the_mirror() {
        // Post-`Deactivate` shape — the ring stays armed but the
        // pipeline no longer runs, so the answer is `None` again.
        let (host, handle) = l1_instance(LifecycleState::Initialized, Some(512));
        assert_eq!(host.latency_frames(handle), None);
        // `Failed` with the ring still armed — convergence keeps
        // `staging`.
        let (host, handle) = l1_instance(LifecycleState::Failed, Some(512));
        assert_eq!(host.latency_frames(handle), None);
        // `Destroyed` — the same `None` while the slot still
        // resolves.
        let (host, handle) = l1_instance(LifecycleState::Destroyed, Some(512));
        assert_eq!(host.latency_frames(handle), None);
        // Sanity — the gated answer inside the mirror's live set.
        let (host, handle) = l1_instance(LifecycleState::Activated, Some(512));
        assert_eq!(host.latency_frames(handle), Some(1024));
    }

    /// `plugin_mut` parity — `None` for an L0 `Shared` slot (the
    /// shared mutex owns the plugin) exactly as for L1, while a
    /// `Plain` slot still lends its plugin.
    #[test]
    fn plugin_mut_none_for_shared_and_l1() {
        let (mut host, handle) = l0_instance(Box::new(SendGainFactory), true);
        assert!(host.plugin_mut(handle).is_none());
        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        assert!(host.plugin_mut(handle).is_none());
        let (mut host, handle) = l0_instance(Box::new(PlainGainFactory), true);
        assert!(host.plugin_mut(handle).is_some());
    }
}
