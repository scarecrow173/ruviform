//! [`PluginHost`] — the composite host: one public surface that
//! routes each registered `plugin_id` to the L0 in-process backend
//! ([`InProcessHost`]) or the opt-in L1 out-of-process backend,
//! behind a single unified [`PluginHandle`] space. Each L1
//! `Instantiate` lands on a shared child process
//! ([`OutOfProcessIsolation::Bundled`] — the route's pool fills one
//! child up to `wire::WIRE_MAX_INSTANCES` before spawning the next)
//! or on its own child ([`OutOfProcessIsolation::Dedicated`] — the
//! capacity-1 route) per the spec's `isolation`.
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
use std::sync::Arc;
use std::time::Duration;

use ruviform_audio_core::SampleRate;
use ruviform_observability::source::ObservationRegistrar;
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    Event, EventList, FailureCause, LifecycleEvent, LifecycleState, ParamId, PlainParamValue,
    PluginDescriptor, PrepareError, ProcessResult, TransportInfo, VideoProcessConfig,
};

use crate::endpoint::{EndpointBounds, EndpointInner, L1Endpoint, PluginEndpoint};
use crate::l1::diag::{DiagRegistry, DiagnosticQuery, DiagnosticRecord};
use crate::l1::{
    AudioCounterReaders, ChildExit as L1ChildExit, L1Child, L1Instance, SpawnSpec, VideoL1Instance,
};
use crate::video::{VideoHostCommand, VideoHostReply, VideoOutOfProcessSpec, VideoPluginHost};
use crate::{
    BusReport, ErasedPlugin, HostCommand, HostError, HostInput, HostOutput, HostReply,
    InProcessHost, PluginFactory, PluginHandle, ReportDrain,
};

/// How L1 instances of one out-of-process route map onto child
/// processes — [`OutOfProcessSpec::isolation`]'s declaration, fixed
/// at registration.
///
/// This is a crash- *and* liveness-isolation trade-off, not merely a
/// resource one. A bundled child runs a single-threaded serve loop
/// for every co-resident instance, so one hung plugin wedges the
/// loop and stalls all of them — the failure surfaces as a
/// child-scope `command_timeout`, not a per-instance answer. Pick
/// [`Dedicated`](OutOfProcessIsolation::Dedicated) whenever an
/// instance must be protected from its neighbours' hangs as well as
/// their crashes.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new isolation modes may be added without breaking
/// downstream `match` arms.
///
/// **RT contract:** non-RT only — registration data.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutOfProcessIsolation {
    /// Every `Instantiate` spawns its own child process — the
    /// capacity-1 route: full per-instance crash *and* liveness
    /// isolation (one hung or crashing plugin touches no other
    /// instance), at the per-instance process, shm-segment, thread
    /// and handshake cost.
    Dedicated,
    /// Instances of the route share child processes — the route's
    /// pool fills each child up to
    /// `wire::WIRE_MAX_INSTANCES` co-resident instances before
    /// spawning the next. One process, thread pair and handshake
    /// serves many instances, but failure granularity is
    /// child-scoped: a crash fans `Failed` out to every co-resident
    /// instance and one hung plugin wedges the shared serve loop
    /// for all of them. The shared stdio pipe also serializes
    /// large frames (e.g. a multi-MB `RestoreState` payload), so
    /// one instance's bulk transfer can delay its siblings' replies.
    /// Isolation is state/attribution-granular, not address-space:
    /// a corrupting plugin could rewrite a co-resident instance's
    /// shm slots — the host's fail-closed out-slot checks confine
    /// that to a defined per-instance failure. **The default** for
    /// this unpublished 0.1.0 API.
    #[default]
    Bundled,
}

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
    /// Extra arguments prepended to the runner's argv — the runner's
    /// own flags only. Wire v2 retired the `--ruviform-*` bootstrap
    /// flags: plugin selection and the shm key ride the
    /// `CreateInstance` payload at handshake instead, and a
    /// `ruviform_plugin_export::run`-based runner accepts no argv at
    /// all (leaving this empty is the production default).
    pub args: Vec<OsString>,
    /// Lookahead depth in blocks (default **2**) — fixes the L1
    /// latency contract `lookahead_blocks × activated max_frames`
    /// frames. `0` is rejected at spawn (`Err(HostError::Ipc)`)
    /// because the result horizon could never be met. Values
    /// `>= ruviform_plugin_core::wire::WIRE_MAX_ELEMENTS` are
    /// rejected the same way — `lookahead_blocks + 1` is the shm
    /// slot depth `CreateInstance` declares on the wire, and the
    /// codec caps that depth at `WIRE_MAX_ELEMENTS`.
    pub lookahead_blocks: u32,
    /// Defined timeout for the handshake and every later `Reply`
    /// wait (default **10 s**); a timed-out command converges the
    /// instance to `Failed`.
    pub command_timeout: Duration,
    /// Child-process isolation for this route (default
    /// [`OutOfProcessIsolation::Bundled`]) — see the enum for the
    /// crash/liveness trade-off. `Bundled` pools instances onto
    /// shared children; `Dedicated` spawns one child per
    /// `Instantiate` and never pools it.
    pub isolation: OutOfProcessIsolation,
}

impl OutOfProcessSpec {
    /// A spec with the design defaults (`lookahead_blocks` 2,
    /// `command_timeout` 10 s, `isolation`
    /// [`OutOfProcessIsolation::Bundled`]); adjust the fields as
    /// needed.
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
            isolation: OutOfProcessIsolation::Bundled,
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
    #[non_exhaustive]
    InstanceFailed {
        /// The failed instance's handle — the same unified
        /// [`PluginHandle`] `Instantiate` returned.
        handle: PluginHandle,
        /// How the child exited.
        exit: ChildExit,
        /// Why the instance failed — the first-observed
        /// [`FailureCause`] classified by the L1 convergence (Issue
        /// #121). `exit` keeps carrying the raw status.
        cause: FailureCause,
    },
}

/// A point-in-time read of an L1 audio instance's observability
/// counters (Issue #121) — the `PluginHost::audio_counters` result.
///
/// **The four fields are independent `Relaxed` reads — they do not
/// form an atomic multi-counter snapshot** (each counter is
/// individually monotone non-decreasing).
///
/// Semantics:
/// - `dropped_submits`: blocks whose submit was dropped — a busy
///   slot or a full submit channel, at most +1 per block.
/// - `passthrough_submits`: blocks that produced substituted
///   output — warmup included.
/// - `results_failed_observed`: `RESULT_FAILED` elements observed
///   on the result channel.
/// - `results_dropped`: reader-side result `try_push` failures
///   only — **it is invalid to infer total result loss as
///   `results_dropped - results_failed_observed`** (enqueue
///   failure and observed failure are different windows).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioCounterSnapshot {
    /// Submits dropped (busy slot or full submit channel).
    pub dropped_submits: u64,
    /// Blocks that produced substituted output — warmup included.
    pub passthrough_submits: u64,
    /// `RESULT_FAILED` elements observed on the result channel.
    pub results_failed_observed: u64,
    /// Reader-side result enqueue failures.
    pub results_dropped: u64,
}

impl From<AudioCounterReaders> for AudioCounterSnapshot {
    fn from(readers: AudioCounterReaders) -> Self {
        Self {
            dropped_submits: readers.dropped_submits.read(),
            passthrough_submits: readers.passthrough_submits.read(),
            results_failed_observed: readers.results_failed_observed.read(),
            results_dropped: readers.results_dropped.read(),
        }
    }
}

/// How a registered `plugin_id` is served — decided at registration
/// and fixed for the id's lifetime.
enum Route {
    /// `register_factory` — instances run in this process through
    /// [`InProcessHost`].
    InProcess,
    /// `register_out_of_process` — every `Instantiate` either draws
    /// a child from the route's `Bundled` pool or spawns a
    /// `Dedicated` one, per `isolation`.
    OutOfProcess {
        /// The route's spawn parameters — program, argv, lookahead
        /// depth and command timeout.
        spec: SpawnSpec,
        /// The route's child-process isolation policy.
        isolation: OutOfProcessIsolation,
        /// The route's live bundled children — eviction uses
        /// `swap_remove`/`retain`, so pool order is not spawn order;
        /// selection only promises *first eligible*, not oldest.
        /// `Dedicated` routes never insert (the field exists either
        /// way so the variant is uniform), and `Bundled` eviction
        /// runs inside the instantiate scan and the
        /// shutdown-on-empty destroy path.
        pool: Vec<Arc<L1Child>>,
    },
    /// `VideoPluginHost::register_out_of_process` — a
    /// dedicated-only video route: every `Video` `Instantiate`
    /// spawns a fresh child, no `isolation`, no pool (the variant
    /// carries neither field — a bundled video route cannot be
    /// expressed).
    OutOfProcessVideo {
        /// The route's spawn parameters — program, argv and the
        /// command/render timeouts.
        spec: crate::l1::VideoSpawnSpec,
    },
}

/// One live entry of the unified slot table — the `PluginHandle`
/// dispatch target.
enum Slot {
    /// L0 instance — the *inner* handle [`InProcessHost`] minted;
    /// commands are retargeted onto it.
    InProcess(PluginHandle),
    /// L1 audio instance — the child-process proxy owns it end to
    /// end. Boxed: `L1Instance` is ~500 B; the slot table stays
    /// compact.
    OutOfProcess(Box<L1Instance>),
    /// L1 video instance — the NONRT counterpart: no RT leg, no
    /// endpoint surface — the whole facade is `execute`/`render`
    /// plus the shared lifecycle mirror. Boxed: the `vshm`
    /// segment keeps the facade non-`Send`/non-`Copy` like the
    /// audio sibling.
    OutOfProcessVideo(Box<VideoL1Instance>),
}

/// Which kind of plugin a registration serves — the `Scan` kind
/// filter's axis. A route is one kind end to end: `Scan` reports
/// the matching entries only, and cross-kind `Instantiate` —
/// audio `Instantiate` naming a video route, or `Video`
/// `Instantiate` naming an audio route — is the defined
/// [`HostError::UnknownPluginId`]. Crate-internal: callers never
/// name a kind — the two `Scan` surfaces do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DescriptorKind {
    /// `register_factory` / `register_out_of_process` — audio
    /// `Scan` and audio `Instantiate` see it.
    Audio,
    /// `VideoPluginHost::register_out_of_process` — video `Scan`
    /// and `Video` `Instantiate` see it.
    Video,
}

/// One `Scan`-listed registration — the descriptor plus its kind
/// tag. Kinds never interleave into each other's scans: audio and
/// video share the registration-order listing but each `Scan`
/// surface filters to its own.
struct DescriptorEntry {
    /// Which `Scan`/`Instantiate` surface owns the entry.
    kind: DescriptorKind,
    /// The registration-supplied `'static` descriptor — `Scan`'s
    /// payload and the child's handshake `matches` oracle.
    descriptor: PluginDescriptor,
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
    /// The `DescriptorEntry` kind tag is the `Scan` kind filter's
    /// axis: audio `Scan` reports `Audio` entries only, video `Scan`
    /// `Video` entries only — an audio registration can never leak
    /// into a video scan (and vice versa).
    descriptors: Vec<DescriptorEntry>,
    /// `plugin_id` → serving route; the first registration of an id
    /// claims it (`register_out_of_process` reports the collision as
    /// `DuplicatePluginId`, `register_factory` — infallible — keeps
    /// the existing route and shadows its factory, L0-style).
    ///
    /// **Field-order is load-bearing on drop:** fields drop in
    /// declaration order, so `routes` (bundled `Arc<L1Child>` pools)
    /// must release before `instances` (facades). Each facade's
    /// last-referrer teardown keys on `Arc::strong_count == 1` —
    /// dropping pools first leaves the facade the sole owner and it
    /// runs graceful `Shutdown`+grace; reversed order would degrade
    /// every host-drop teardown to the driver's `Disconnected`
    /// kill-path.
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
    /// The host's bounded diagnostics retention — spawn/create
    /// paths take `&DiagRegistry` and register their failed attempt
    /// records into it; minted failures register at
    /// `try_recv_notification` consumption, keyed by the public
    /// handle's slot (`record_for_handle` correlation).
    diag: Arc<DiagRegistry>,
    /// The attached observation registrar (Issue #279, OBS-04) —
    /// `None` leaves every L1 audio mint's four diagnostic counters
    /// unregistered (the default); `Some` makes each successful mint
    /// register `Instance`-scoped sources and each `Destroy` retire
    /// their leases. Passed to the mint paths as `&dyn` — it never
    /// reaches an RT leg.
    observation_registrar: Option<Arc<dyn ObservationRegistrar>>,
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
            diag: Arc::new(DiagRegistry::new()),
            observation_registrar: None,
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
        self.descriptors.push(DescriptorEntry {
            kind: DescriptorKind::Audio,
            descriptor,
        });
        self.l0.register_factory(factory);
    }

    /// Registers one plugin type for the out-of-process (L1) route:
    /// every `Instantiate { plugin_id: spec.descriptor.id }` lands
    /// on a child running `spec.program` — shared with the route's
    /// other instances (`Bundled`, the default) or dedicated to it
    /// (`Dedicated`), per `spec.isolation`.
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
            isolation,
        } = spec;
        if self.routes.contains_key(descriptor.id) {
            return Err(HostError::DuplicatePluginId);
        }
        let mut spawn = SpawnSpec::new(descriptor, program);
        spawn.args = args;
        spawn.lookahead_blocks = lookahead_blocks;
        spawn.command_timeout = command_timeout;
        self.routes.insert(
            descriptor.id.to_owned(),
            Route::OutOfProcess {
                spec: spawn,
                isolation,
                pool: Vec::new(),
            },
        );
        self.descriptors.push(DescriptorEntry {
            kind: DescriptorKind::Audio,
            descriptor,
        });
        Ok(())
    }

    /// The video hosting surface — a borrowed view on `self`
    /// sharing the registration set, the plugin-id namespace and
    /// the unified slot/generation table. [`PluginHost::video`]
    /// hands out a [`VideoPluginHost`] for a `&mut` window; every
    /// video registration, scan and instance command runs through
    /// it — the audio surface stays untouched and cannot see or
    /// reach video routes/slots (cross-kind `Instantiate` is
    /// [`HostError::UnknownPluginId`], cross-kind handle verbs are
    /// [`HostError::Unsupported`]).
    ///
    /// ```
    /// use ruviform_plugin_host::{PluginHost, VideoOutOfProcessSpec};
    /// use std::ffi::OsString;
    ///
    /// let mut host = PluginHost::new();
    /// // … register video routes through `host.video()` …
    /// host.video()
    ///     .register_out_of_process(VideoOutOfProcessSpec::new(
    ///         ruviform_plugin_core::PluginDescriptor {
    ///             id: "org.example.vfx",
    ///             name: "Vfx",
    ///             vendor: "Example",
    ///             version: "1.0.0",
    ///             features: &[],
    ///         },
    ///         OsString::from("vfx-runner"),
    ///     ))
    ///     .unwrap();
    /// ```
    ///
    /// **RT contract:** non-RT only — the returned facade's methods
    /// spawn child processes and wait on replies.
    pub fn video(&mut self) -> VideoPluginHost<'_> {
        VideoPluginHost::new(self)
    }

    /// Registers one video plugin type for the dedicated L1
    /// route — the `VideoPluginHost::register_out_of_process`
    /// implementation body. A video route is structurally
    /// dedicated-only (no pool, no `isolation` field exists), and
    /// the shared `routes` map enforces the unified plugin-id
    /// namespace: an id claimed by either kind is claimed for
    /// both.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::DuplicatePluginId`] when `spec.descriptor.id`
    /// was already registered by an audio or video route.
    pub(crate) fn video_register_out_of_process(
        &mut self,
        spec: VideoOutOfProcessSpec,
    ) -> Result<(), HostError> {
        let VideoOutOfProcessSpec {
            descriptor,
            program,
            args,
            command_timeout,
            render_timeout,
        } = spec;
        if self.routes.contains_key(descriptor.id) {
            return Err(HostError::DuplicatePluginId);
        }
        let mut spawn = crate::l1::VideoSpawnSpec::new(descriptor, program);
        spawn.args = args;
        spawn.command_timeout = command_timeout;
        spawn.render_timeout = render_timeout;
        self.routes.insert(
            descriptor.id.to_owned(),
            Route::OutOfProcessVideo { spec: spawn },
        );
        self.descriptors.push(DescriptorEntry {
            kind: DescriptorKind::Video,
            descriptor,
        });
        Ok(())
    }

    /// Executes one video command — the
    /// [`VideoPluginHost::execute`] implementation body. `Scan`
    /// reports `Video`-kind registrations only, `Instantiate`
    /// routes by `plugin_id` (audio routes are
    /// `UnknownPluginId` — a cross-kind instantiation can never
    /// mint a handle), and every handle-carrying verb is
    /// dispatched through the unified slot table exactly like the
    /// audio leg.
    ///
    /// **RT contract:** non-RT only.
    pub(crate) fn video_execute(
        &mut self,
        command: VideoHostCommand,
    ) -> Result<VideoHostReply, HostError> {
        match command {
            VideoHostCommand::Scan => Ok(VideoHostReply::Scanned(
                self.descriptors
                    .iter()
                    .filter(|entry| entry.kind == DescriptorKind::Video)
                    .map(|entry| entry.descriptor)
                    .collect(),
            )),
            VideoHostCommand::Instantiate { plugin_id, config } => {
                self.video_instantiate(&plugin_id, config)
            }
            VideoHostCommand::Destroy { handle } => {
                self.destroy(handle).map(|_| VideoHostReply::Ok)
            }
            // `Render` carries a `VideoFrame`, not a wire payload —
            // it routes through `video_render` (the shm pack /
            // reply-desc validate leg), never `video_dispatch`'s
            // `video_to_wire` map.
            VideoHostCommand::Render { handle, input, at } => self
                .video_render(handle, &input, at)
                .map(|frame| VideoHostReply::Rendered { frame }),
            command => self.video_dispatch(&command),
        }
    }

    /// The `Render` leg — resolves the slot, then hands the input
    /// frame to the video facade's shm-copy + `VideoRender`
    /// round-trip. Non-video slots are the cross-kind
    /// `Unsupported`; the frame's host-side desc validation and
    /// the render timeout live inside `VideoL1Instance::render`.
    fn video_render(
        &mut self,
        handle: PluginHandle,
        input: &ruviform_video_core::VideoFrame,
        at: ruviform_media_core::MediaTime,
    ) -> Result<ruviform_video_core::VideoFrame, HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(_) | Slot::OutOfProcess(_) => Err(HostError::Unsupported),
            Slot::OutOfProcessVideo(instance) => instance.render(input, at),
        }
    }

    /// `Video` `Instantiate` — resolves the route by `plugin_id`,
    /// bounds-checks the config (before any allocation or spawn),
    /// then mints the handle from the same unified slot table the
    /// audio path uses.
    fn video_instantiate(
        &mut self,
        plugin_id: &str,
        config: VideoProcessConfig,
    ) -> Result<VideoHostReply, HostError> {
        // No such registration at all — the shared namespace — or
        // an *audio* route under that id: either is the defined
        // `UnknownPluginId` (a video `Instantiate` never names an
        // audio route's id).
        let Some(Route::OutOfProcessVideo { spec }) = self.routes.get(plugin_id) else {
            return Err(HostError::UnknownPluginId);
        };
        check_video_bounds(&config)?;
        let slot =
            Slot::OutOfProcessVideo(Box::new(VideoL1Instance::spawn(spec, config, &self.diag)?));
        Ok(VideoHostReply::Instantiated {
            handle: self.mint(slot),
        })
    }

    /// Dispatches one handle-carrying video command through the
    /// unified slot table — video slots run the wire verb; L0 or
    /// audio-L1 slots are the defined cross-kind `Unsupported`.
    fn video_dispatch(&mut self, command: &VideoHostCommand) -> Result<VideoHostReply, HostError> {
        let handle = video_command_handle(command);
        match self.slot_mut(handle)? {
            Slot::InProcess(_) | Slot::OutOfProcess(_) => Err(HostError::Unsupported),
            Slot::OutOfProcessVideo(instance) => {
                // Encode obligation (a): a non-finite
                // `PlainParamValue::Double` is the defined
                // `Err(InvalidValue)` *before* the wire — the codec
                // would reject it too, but the value domain is the
                // host's to guard (same class as the `frame_bytes`
                // early checks).
                if let VideoHostCommand::SetParamValue {
                    value: PlainParamValue::Double(value),
                    ..
                } = command
                    && !value.is_finite()
                {
                    return Err(HostError::InvalidValue);
                }
                let command = video_to_wire(command);
                let expected = video_expected_reply(&command);
                instance
                    .execute(command)
                    .and_then(|reply| video_host_reply(expected, reply))
            }
        }
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
            HostCommand::Scan => Ok(HostReply::Scanned(
                self.descriptors
                    .iter()
                    .filter(|entry| entry.kind == DescriptorKind::Audio)
                    .map(|entry| entry.descriptor)
                    .collect(),
            )),
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
            Slot::OutOfProcessVideo(instance) => Some(instance.lifecycle_state()),
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
            // A video slot has no pipeline latency — the concept
            // does not exist on the NONRT surface.
            Slot::OutOfProcessVideo(_) => None,
        }
    }

    /// The audio instance's observability counters (Issue #121) —
    /// `None` for an invalid handle, an L0 instance, or a video
    /// instance (which carries no counters by construction). The
    /// values survive leg swaps, endpoint checkout and `arm`; a
    /// destroyed handle is `None`, a re-minted one reads fresh
    /// pairs starting at zero.
    ///
    /// **RT contract:** non-RT only — takes the instance's shared
    /// control mutex, exactly like [`PluginHost::lifecycle_state`].
    #[must_use]
    pub fn audio_counters(&self, handle: PluginHandle) -> Option<AudioCounterSnapshot> {
        let index = handle.slot() as usize;
        if self.generations.get(index).copied() != Some(handle.generation()) {
            return None;
        }
        match self.instances.get(index)?.as_ref()? {
            Slot::OutOfProcess(instance) => {
                instance.audio_counters().map(AudioCounterSnapshot::from)
            }
            Slot::InProcess(_) | Slot::OutOfProcessVideo(_) => None,
        }
    }

    /// Attaches (or clears, `None`) the [`ObservationRegistrar`]
    /// every subsequent L1 audio `Instantiate` registers its four
    /// diagnostic counter sources against — Issue #279's OBS-04
    /// wiring to the OBS-02 source contract (the Issue #289
    /// collector plugs in here once it lands). `None` is the
    /// default and reproduces the pre-attach behavior exactly: no
    /// source is ever registered and `audio_counters`/
    /// [`AudioCounterSnapshot`] stay identical.
    ///
    /// The registrar applies to the *next* mint — instances already
    /// minted keep the leases they minted with, and an attached
    /// registrar covers every route (`Dedicated` and `Bundled`,
    /// including supervisor restarts through [`HostCommand::Instantiate`]).
    /// Registration is best-effort: a typed `RegistrationError`
    /// degrades only that counter's slot and never fails
    /// `Instantiate` or the plugin. Source lifetimes follow the
    /// instance — `Destroy` (or facade drop) retires each lease
    /// once the owner-side counter writers are gone, and a re-mint
    /// registers a fresh `SourceIdentity`.
    ///
    /// **RT contract:** non-RT only — stores the handle; all
    /// registration and retirement work runs on the non-RT mint/
    /// destroy paths, never inside `process`.
    pub fn set_observation_registrar(&mut self, registrar: Option<Arc<dyn ObservationRegistrar>>) {
        self.observation_registrar = registrar;
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
            Slot::OutOfProcess(_) | Slot::OutOfProcessVideo(_) => None,
        }
    }

    /// The instance's declared bus table — the honest
    /// `Reported`/`Unavailable` split the capability matrix needs.
    /// L0 slots reach the plugin through [`InProcessHost::buses`];
    /// L1 slots (audio and video) report [`BusReport::Unavailable`]
    /// — the wire protocol has no bus verb yet, and callers must not
    /// infer a layout from that answer.
    ///
    /// **RT contract:** non-RT only — allocates the report vec; the
    /// L0 `Shared` arm takes the instance's `core` mutex.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle and
    /// [`HostError::Failed`] when an L0 instance has already
    /// converged.
    pub fn buses(&mut self, handle: PluginHandle) -> Result<BusReport, HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.buses(inner)
            }
            Slot::OutOfProcess(_) | Slot::OutOfProcessVideo(_) => Ok(BusReport::Unavailable),
        }
    }

    /// Drains the instance's plugin→host reports — the canonical
    /// [`Event`]s the plugin queued during `process` calls since the
    /// last drain — appending them to `out`. The consumer-facing
    /// pull surface of the neutral
    /// [`PluginProcessor::drain_output_reports`](ruviform_plugin_core::PluginProcessor::drain_output_reports)
    /// contract (`docs/designs/issue-253-hostgraph-reports.md`):
    /// opt-in — nothing drains unless the consumer calls this, and
    /// reports a consumer never reads stay in the plugin's bounded
    /// pending buffer (producer-side defined drop past capacity).
    ///
    /// L0 slots delegate to [`InProcessHost::drain_output_reports`]
    /// — `Plain` calls the plugin directly, `Shared` runs the
    /// pinned three-tier lock protocol, so the drain serializes
    /// against an attached endpoint's `try_lock` RT leg on the same
    /// `core` mutex and converges `Failed`/poisoned observations
    /// exactly like every other `Shared` verb. L1 audio slots run
    /// the wire-v4 `DrainOutputReports` verb — the child chunks its
    /// backlog at `WIRE_MAX_EVENTS` per reply and the instance loops
    /// until a short reply, so one call still retrieves everything
    /// pending, oldest-first. L1 video slots keep answering
    /// [`ReportDrain::Unavailable`]: the video protocol has no
    /// report verb, and callers must not infer report silence from
    /// that answer.
    ///
    /// **RT contract:** non-RT only — the `out` append may allocate
    /// (the caller owns and may reuse the buffer), the L0 `Shared`
    /// arm takes the instance's `core` mutex, and the L1 arm waits
    /// at most `command_timeout` per wire verb. Never call from
    /// inside `process`.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle and
    /// [`HostError::Failed`] when an L0 `Shared` instance or an L1
    /// audio instance has already converged.
    pub fn drain_output_reports(
        &mut self,
        handle: PluginHandle,
        out: &mut Vec<Event>,
    ) -> Result<ReportDrain, HostError> {
        match self.slot_mut(handle)? {
            Slot::InProcess(inner) => {
                let inner = *inner;
                self.l0.drain_output_reports(inner, out)
            }
            Slot::OutOfProcess(instance) => {
                instance.drain_output_reports(out)?;
                Ok(ReportDrain::Drained)
            }
            Slot::OutOfProcessVideo(_) => Ok(ReportDrain::Unavailable),
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
            // The RT processing surface does not exist for a video
            // slot — `Render` rides the command façade, not
            // `process`.
            Slot::OutOfProcessVideo(_) => Err(HostError::Unsupported),
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
            // A video slot has no RT leg — there is nothing to
            // attach (the design's video-surface answer is
            // `EndpointUnavailable`, never `Lifecycle`).
            Slot::OutOfProcessVideo(_) => return Err(HostError::EndpointUnavailable),
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
            // A video slot cannot reach an inner attach — the
            // `prepare_endpoint` prelude rejects it
            // (`EndpointUnavailable`) before this point; the arm
            // repeats the defined answer as defense-in-depth.
            Slot::OutOfProcessVideo(_) => Err(HostError::EndpointUnavailable),
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
    /// **RT contract:** non-RT only — `try_recv`, never blocks on
    /// an empty queue. A confirmed-failure pop freezes the child's
    /// diagnostics record, which waits for the stderr drain's own
    /// bounded end (pipe EOF/error, or its post-exit deadline —
    /// see `diag::STDERR_DRAIN_GRACE`) so the record carries the
    /// full capture rather than a pre-drain snapshot.
    ///
    /// # Panics
    ///
    /// Only on an internal invariant violation — every slot index is
    /// minted through `u32`, so the index→`u32` narrowing cannot fail.
    #[must_use]
    pub fn try_recv_notification(&mut self) -> Option<HostNotification> {
        for (index, slot) in self.instances.iter_mut().enumerate() {
            // Audio slots run the `&mut` entry-point drain —
            // restores a returned leg and re-sends/drains the arm
            // channels before the notification check (`&self`
            // queries such as `lifecycle_state` stay deliberately
            // non-draining). Video slots have no endpoint channels
            // — they go straight to the notification try-recv.
            let notice = match slot {
                Some(Slot::OutOfProcess(instance)) => {
                    instance.drain_endpoint_channels();
                    instance.try_recv_notification()
                }
                Some(Slot::OutOfProcessVideo(instance)) => instance.try_recv_notification(),
                Some(Slot::InProcess(_)) | None => continue,
            };
            if let Some(notice) = notice {
                // The slot is still live (freed slots never carry an
                // instance), so the current generation is the minted
                // handle's.
                let handle = PluginHandle::new(
                    u32::try_from(index).expect("slot index fits u32"),
                    self.generations[index],
                );
                // Minted-failure record — the notification carries
                // no diagnostics id, so the public handle's slot is
                // the `record_for_handle` correlation key; the
                // instance's first-observed cause is stamped (the
                // frozen capture's own cause is child-scope). The
                // freeze runs only on a confirmed failure — the
                // snapshot copies the retained tail, so it is not a
                // per-poll cost.
                let mut pending = match slot {
                    Some(Slot::OutOfProcess(instance)) => instance.diagnostics_pending(),
                    Some(Slot::OutOfProcessVideo(instance)) => instance.diagnostics_pending(),
                    Some(Slot::InProcess(_)) | None => {
                        unreachable!("a notification only comes from an L1 slot")
                    }
                };
                pending.handle = Some(handle.slot());
                pending.cause = Some(notice.cause);
                self.diag.register(pending);
                return Some(HostNotification::InstanceFailed {
                    handle,
                    exit: ChildExit::from(notice.exit),
                    cause: notice.cause,
                });
            }
        }
        None
    }

    /// Resolves a `diagnostics_id` — the id an
    /// `Err(HostError::SpawnFailed{cause, diagnostics_id})` carried
    /// — against the host's bounded diagnostics retention:
    /// [`DiagnosticQuery::Found`] while the record is retained,
    /// [`DiagnosticQuery::Evicted`] once FIFO eviction passed it
    /// (ids are minted contiguously and eviction is id-ordered, so
    /// the watermark distinguishes evicted from never-minted),
    /// [`DiagnosticQuery::NotFound`] for an id never minted.
    ///
    /// **RT contract:** non-RT only — takes the registry mutex.
    #[must_use]
    pub fn diagnostics_query(&self, diagnostics_id: u64) -> DiagnosticQuery {
        self.diag.query(diagnostics_id)
    }

    /// The retained diagnostics record for a *minted* failure,
    /// correlated by the public handle [`HostNotification::
    /// InstanceFailed`] carried — `None` while no record exists for
    /// the handle's slot. The index keys on the slot alone: a
    /// reused slot's newest record wins, so a stale-generation
    /// handle may resolve a later mint's record (the record's own
    /// `handle` field is the same slot key).
    ///
    /// **RT contract:** non-RT only — takes the registry mutex.
    #[must_use]
    pub fn diagnostics_for_handle(&self, handle: PluginHandle) -> Option<Arc<DiagnosticRecord>> {
        self.diag.record_for_handle(handle.slot())
    }

    /// Force-kills an L1 instance's child process — **verification
    /// support**, not a stable contract: drives the crash paths
    /// (`Failed` convergence + `HostNotification::InstanceFailed`)
    /// in tests. L0 slots have no child — `Err(HostError::Unsupported)`.
    ///
    /// The kill is **child-scoped**: on a `Bundled` route the child
    /// may host co-resident instances, so one kill fans
    /// `InstanceFailed` out to *every* live instance on it — not
    /// only the named handle's. `Dedicated` children host exactly
    /// one instance, so the same call is per-instance there.
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
            Slot::OutOfProcessVideo(instance) => {
                instance.kill();
                Ok(())
            }
        }
    }

    /// `Instantiate` — routes by `plugin_id`, then mints the public
    /// handle from the unified slot table regardless of layer.
    fn instantiate(&mut self, plugin_id: &str) -> Result<HostReply, HostError> {
        let slot = match self.routes.get_mut(plugin_id) {
            None => return Err(HostError::UnknownPluginId),
            // An audio `Instantiate` naming a video route's id is
            // the cross-kind rejection — `UnknownPluginId`, not
            // `Unsupported` — so an audio caller can never mint a
            // video handle it has no verbs for.
            Some(Route::OutOfProcessVideo { .. }) => {
                return Err(HostError::UnknownPluginId);
            }
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
            Some(Route::OutOfProcess {
                spec,
                isolation,
                pool,
            }) => match *isolation {
                // The capacity-1 route — a fresh child per instance,
                // never pooled.
                OutOfProcessIsolation::Dedicated => Slot::OutOfProcess(Box::new(
                    L1Instance::spawn(spec, &self.diag, self.observation_registrar.as_deref())?,
                )),
                OutOfProcessIsolation::Bundled => {
                    Slot::OutOfProcess(Box::new(instantiate_bundled(
                        spec,
                        pool,
                        &self.diag,
                        self.observation_registrar.as_deref(),
                    )?))
                }
            },
        };
        Ok(HostReply::Instantiated {
            handle: self.mint(slot),
        })
    }

    /// `Destroy` — frees the unified slot after the route's own
    /// teardown. L0 delegates to `InProcessHost` (which applies the
    /// lifecycle legality rule itself); L1 applies that same rule
    /// here — `L1Instance::destroy` is unconditional, so the
    /// composite is where `Activated`/`Processing` rejections live.
    /// The teardown is child-scoped: a dedicated child runs its
    /// `Shutdown`+reap as the facade's last referrer inside
    /// `destroy`, while a pooled child whose registry just emptied
    /// is evicted and torn down by the composite — a sibling still
    /// registered on it keeps the child alive.
    fn destroy(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let index = handle.slot() as usize;
        let mut emptied = None;
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
                // Cloned *after* `destroy` — the facade's
                // `Arc::strong_count == 1` last-referrer check must
                // see no extra references: under `Dedicated` that
                // count is exactly 1 and the child's own teardown
                // already ran above.
                emptied = Some(Arc::clone(instance.child()));
            }
            Slot::OutOfProcessVideo(instance) => {
                // The same mirrored legality rule. The dedicated
                // video child tears itself down inside `destroy`
                // (its only `Arc` is the facade's) — there is no
                // pool, so `emptied`/`evict_emptied_child` has no
                // work here.
                instance
                    .lifecycle_state()
                    .transition(LifecycleEvent::Destroy)
                    .map_err(HostError::Lifecycle)?;
                instance.destroy();
            }
        }
        self.instances[index] = None;
        self.free_slots.push(handle.slot());
        if let Some(child) = emptied {
            self.evict_emptied_child(&child);
        }
        Ok(HostReply::Ok)
    }

    /// The shutdown-on-empty half of L1 `Destroy`: the child whose
    /// registry just emptied is evicted from whichever route pool
    /// holds it — a `Dedicated` child was never pooled, so the scan
    /// is a no-op for one — then torn down (`Shutdown` + bounded
    /// grace/reap/join). Runs after the slot's facade dropped, so
    /// the pool entry plus this clone are the child's last
    /// referrers; a child with live siblings (`live_count != 0`)
    /// is left serving them, never torn down under them.
    fn evict_emptied_child(&mut self, child: &Arc<L1Child>) {
        if child.live_count() != 0 {
            return;
        }
        for route in self.routes.values_mut() {
            if let Route::OutOfProcess { pool, .. } = route
                && let Some(position) = pool.iter().position(|pooled| Arc::ptr_eq(pooled, child))
            {
                pool.swap_remove(position);
                child.teardown();
                return;
            }
        }
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
            // An audio verb against a video slot — the defined
            // cross-kind `Unsupported` (the handle is valid; the
            // vocabulary is not).
            Slot::OutOfProcessVideo(_) => Err(HostError::Unsupported),
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

/// The `Bundled` `Instantiate` path: reuse the first pool child
/// that can still serve — `live_count < WIRE_MAX_INSTANCES &&
/// !terminal && !shutdown_initiated`, evaluated atomically by
/// [`L1Child::pool_eligible`] — evicting dead entries as the scan
/// passes them; spawn a fresh child when none qualifies.
///
/// Failure granularity follows the design: a `CreateInstance`
/// failure on a reused child rolls the just-minted entry back
/// alone (the child's own rollback — live siblings stay
/// untouched); on the fresh child it additionally evicts and
/// tears the child down, so a spawn-time poison can never linger
/// in `pool`. [`HostError::InstanceLimit`] — the child's defined
/// capacity rejection — propagates unflattened (unreachable in
/// the single-threaded scan: the predicate already excluded a
/// full child, and no sibling mint can interleave).
///
/// **RT contract:** non-RT only — child spawns plus `Reply` waits.
fn instantiate_bundled(
    spec: &SpawnSpec,
    pool: &mut Vec<Arc<L1Child>>,
    diag: &DiagRegistry,
    registrar: Option<&dyn ObservationRegistrar>,
) -> Result<L1Instance, HostError> {
    // The spec-legality gate runs before any pool mutation — a
    // defined rejection must consume nothing.
    let depth = spec.instance_depth()?;
    let mut index = 0;
    let child = loop {
        // `.cloned()` — the scan must not hold a `pool` borrow
        // across `create_on_fresh`'s push or `remove`'s shift.
        let Some(child) = pool.get(index).cloned() else {
            // No eligible child — pay the spawn.
            return create_on_fresh(spec, pool, depth, diag, registrar);
        };
        if child.pool_eligible() {
            break child;
        }
        // Ineligible and dead — it can never serve again, so it
        // leaves the pool now rather than being re-scanned
        // forever. A `shutdown_initiated`-but-live child is not
        // evicted (its draining instances still refer to it).
        if child.is_terminal() {
            pool.remove(index);
        } else {
            index += 1;
        }
    };
    match child.create_instance(spec, depth, diag, registrar) {
        Ok(parts) => Ok(L1Instance::from_parts(child, parts)),
        Err(error) => {
            // Reused-child create failure — the child's rollback
            // already removed the minted entry; live siblings stay
            // untouched. A child the failure converged additionally
            // leaves the pool here.
            if child.is_terminal() {
                pool.retain(|pooled| !Arc::ptr_eq(pooled, &child));
            }
            Err(error)
        }
    }
}

/// A fresh child for the pool: spawn + `Scan` — the per-child half
/// of the handshake ([`L1Child::spawn_scanned`] aborts inside on
/// either failure) — then push and mint. The `create_instance`
/// failure arm pops and tears the child down so a spawn-time
/// poison can never linger in `pool`; `Ok` leaves the pushed
/// `Arc` in place as the pool's reference.
///
/// **RT contract:** non-RT only.
fn create_on_fresh(
    spec: &SpawnSpec,
    pool: &mut Vec<Arc<L1Child>>,
    depth: u32,
    diag: &DiagRegistry,
    registrar: Option<&dyn ObservationRegistrar>,
) -> Result<L1Instance, HostError> {
    let child = L1Child::spawn_scanned(&spec.view(), diag)?;
    pool.push(Arc::clone(&child));
    match child.create_instance(spec, depth, diag, registrar) {
        Ok(parts) => Ok(L1Instance::from_parts(child, parts)),
        Err(error) => {
            pool.pop();
            child.teardown();
            Err(error)
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
/// (`Instantiate`/`Destroy` ride `CreateInstance`/`DestroyInstance`
/// plus the spawn/`Shutdown` teardown inside `l1`, not this table —
/// it only sees the per-instance verbs).
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
/// `to_wire`'s mapping. `CreateInstance`/`DestroyInstance` answer
/// `Reply::Ok` too — they never reach `to_wire` (the `l1` layer owns
/// the handshake and teardown), but the pin keeps this table
/// honest across the whole verb set. The `#[non_exhaustive]`
/// wildcard defaults a verb added by a later `WIRE_VERSION` to `Ok`,
/// fail-closed: its new reply kind pairs with nothing until this
/// table is taught.
// The registry verbs share the wildcard's `Ok` — the explicit arm is
// the required pin, so the identical-bodies lint is expected.
#[allow(clippy::match_same_arms)]
fn expected_reply_kind(cmd: &wire::Command) -> ReplyKind {
    match cmd {
        wire::Command::SaveState => ReplyKind::State,
        wire::Command::OpenGui { .. } => ReplyKind::Opened,
        wire::Command::CreateInstance { .. } | wire::Command::DestroyInstance => ReplyKind::Ok,
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

/// The `Instantiate` config bounds — the host-local gate that runs
/// before any allocation, shm segment or child spawn (design's
/// `Instantiate` bound 拒否 row): dims non-zero and inside
/// [`wire::WIRE_VIDEO_MAX_DIM`], `frame_bytes(config)` inside
/// [`wire::WIRE_VIDEO_MAX_FRAME_BYTES`], the derived
/// `segment_len` inside [`wire::WIRE_VIDEO_MAX_SEGMENT_BYTES`],
/// and a non-zero `frame_rate`. Every violation is the defined
/// `Err(HostError::Prepare(PrepareError::UnsupportedConfig))` —
/// `check_uniform_bounds`'s exact vocabulary: a wild config can
/// never reach the `vshm` create or the wire. (`format` known-ness
/// is the type's own guarantee — `PixelFormat` is a closed value
/// domain; `VideoProcessConfig::new`/`FrameRate::new` already
/// reject the zero legs, so the checks below are the wire-side
/// caps, not ctor invariants.)
fn check_video_bounds(config: &VideoProcessConfig) -> Result<(), HostError> {
    let frame_bytes = wire::vshm::frame_bytes(config);
    if config.width() == 0
        || config.height() == 0
        || config.width() > wire::WIRE_VIDEO_MAX_DIM
        || config.height() > wire::WIRE_VIDEO_MAX_DIM
        || config.frame_rate().numerator() == 0
        || config.frame_rate().denominator() == 0
        || frame_bytes > wire::WIRE_VIDEO_MAX_FRAME_BYTES
        || wire::vshm::segment_len(frame_bytes) > wire::WIRE_VIDEO_MAX_SEGMENT_BYTES
    {
        return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
    }
    Ok(())
}

/// The `handle` every dispatchable video command carries —
/// `video_dispatch` is only reached by the handle verbs.
fn video_command_handle(command: &VideoHostCommand) -> PluginHandle {
    match *command {
        VideoHostCommand::Initialize { handle }
        | VideoHostCommand::Activate { handle }
        | VideoHostCommand::StartProcessing { handle }
        | VideoHostCommand::StopProcessing { handle }
        | VideoHostCommand::Deactivate { handle }
        | VideoHostCommand::Destroy { handle }
        | VideoHostCommand::GetParamInfo { handle, .. }
        | VideoHostCommand::GetParams { handle, .. }
        | VideoHostCommand::GetParamValue { handle, .. }
        | VideoHostCommand::SetParamValue { handle, .. }
        | VideoHostCommand::Render { handle, .. } => handle,
        VideoHostCommand::Scan | VideoHostCommand::Instantiate { .. } => {
            unreachable!("Scan/Instantiate never reach video_dispatch")
        }
    }
}

/// The `wire::Command` mirror of a handle-carrying
/// `VideoHostCommand` — the public-to-wire verb map the video
/// dispatch arm runs. `Render` is not here: its payload is a
/// `VideoFrame` (frame bytes, not the wire's descriptor), so the
/// render arm in [`VideoPluginHost::render`] builds
/// `wire::Command::VideoRender` itself from the `vshm` descriptor.
fn video_to_wire(command: &VideoHostCommand) -> wire::Command {
    match *command {
        VideoHostCommand::Initialize { .. } => wire::Command::VideoInitialize,
        VideoHostCommand::Activate { .. } => wire::Command::VideoActivate,
        VideoHostCommand::StartProcessing { .. } => wire::Command::VideoStartProcessing,
        VideoHostCommand::StopProcessing { .. } => wire::Command::VideoStopProcessing,
        VideoHostCommand::Deactivate { .. } => wire::Command::VideoDeactivate,
        VideoHostCommand::GetParamInfo { id, .. } => wire::Command::VideoGetParamInfo { id: id.0 },
        VideoHostCommand::GetParams { .. } => wire::Command::VideoGetParams,
        VideoHostCommand::GetParamValue { id, .. } => {
            wire::Command::VideoGetParamValue { id: id.0 }
        }
        VideoHostCommand::SetParamValue { id, value, .. } => {
            wire::Command::VideoSetParamValue { id: id.0, value }
        }
        VideoHostCommand::Scan
        | VideoHostCommand::Instantiate { .. }
        | VideoHostCommand::Destroy { .. }
        | VideoHostCommand::Render { .. } => {
            unreachable!("Scan/Instantiate/Destroy/Render are not wire verbs here")
        }
    }
}

/// The wire reply payload kind a video verb must answer — the
/// video leg's pairing oracle (the audio `ReplyKind` stays
/// audio-shaped: `State`/`Opened` can never pair here).
#[derive(Clone, Copy)]
enum VideoReplyKind {
    /// Lifecycle verbs and `SetParamValue` answer `Ok`.
    Ok,
    /// `GetParams` answers `Params`.
    Params,
    /// `GetParamInfo` answers `ParamInfo`.
    ParamInfo,
    /// `GetParamValue` answers `ParamValue`.
    ParamValue,
}

/// The one legal reply kind per video wire verb — the pairing
/// `video_host_reply` enforces. Video verbs `Render` covers are
/// absent: `video_to_wire` never emits `VideoRender` (the render
/// path builds it directly), so it lands on the fail-closed
/// wildcard with the registry verbs — unreachable today, pinned
/// `Ok` so a future caller pairing gets a defined answer.
// `match_same_arms`: the pinned `Ok` arms share the wildcard's body
// by design — the explicit arms are the required pin, not dead
// repeats.
#[allow(clippy::match_same_arms)]
fn video_expected_reply(cmd: &wire::Command) -> VideoReplyKind {
    match cmd {
        wire::Command::VideoGetParams => VideoReplyKind::Params,
        wire::Command::VideoGetParamInfo { .. } => VideoReplyKind::ParamInfo,
        wire::Command::VideoGetParamValue { .. } => VideoReplyKind::ParamValue,
        wire::Command::CreateVideoInstance { .. }
        | wire::Command::VideoInitialize
        | wire::Command::VideoActivate
        | wire::Command::VideoStartProcessing
        | wire::Command::VideoRender { .. }
        | wire::Command::VideoStopProcessing
        | wire::Command::VideoDeactivate
        | wire::Command::VideoSetParamValue { .. }
        | wire::Command::DestroyInstance => VideoReplyKind::Ok,
        _ => VideoReplyKind::Ok,
    }
}

/// Maps a wire reply onto the public [`VideoHostReply`], paired
/// against the verb's one legal answer exactly as `host_reply`
/// does on the audio leg. A mismatched kind — `Ok` where `Params`
/// was asked — or a kind no video verb produces (`State`,
/// `Opened`, `Scanned`, `Rendered` as a `GetParams` answer) is the
/// protocol anomaly `Err(HostError::Ipc)`.
fn video_host_reply(
    expected: VideoReplyKind,
    reply: wire::Reply,
) -> Result<VideoHostReply, HostError> {
    match (expected, reply) {
        (VideoReplyKind::Ok, wire::Reply::Ok) => Ok(VideoHostReply::Ok),
        (VideoReplyKind::Params, wire::Reply::Params { ids }) => Ok(VideoHostReply::ParamIds(
            ids.iter().map(|&id| ParamId(id)).collect(),
        )),
        (VideoReplyKind::ParamInfo, wire::Reply::ParamInfo { info }) => {
            Ok(VideoHostReply::ParamInfo(info))
        }
        (VideoReplyKind::ParamValue, wire::Reply::ParamValue { value }) => {
            Ok(VideoHostReply::ParamValue(value))
        }
        // Every other pairing — a kind mismatch or a reply kind no
        // video verb produces — is a protocol anomaly.
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
    use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
    use ruviform_plugin_core::{
        BufferView, BufferViewMut, EventKind, ParamEvent, ParamId, ParamInfo, ParamValue,
        PluginDescriptor, PluginProcessor, PrepareError, ProcessConfig, ProcessContext,
        ProcessStatus,
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

    /// The per-block report pair the composite fixture queues — same
    /// producer shape as `host.rs`'s `report_pair` (the per-module
    /// duplication is intentional).
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

    /// The report-producing fixture for the composite drain route —
    /// queues the bounded pending pair per `process` block, the same
    /// producer shape `host.rs`'s `ReportGain` implements.
    struct ReportGain {
        blocks: u32,
        pending: Vec<Event>,
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
            if self.pending.len() + 2 <= 8 {
                self.pending.extend_from_slice(&report_pair(self.blocks));
            }
            self.blocks += 1;
            ProcessResult::CONTINUE
        }

        fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
            out.append(&mut self.pending);
        }
    }

    impl Plugin for ReportGain {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    struct ReportGainFactory;

    impl PluginFactory for ReportGainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESCRIPTOR
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(ReportGain {
                blocks: 0,
                pending: Vec::with_capacity(8),
            })
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            Some(erase_sendable(ReportGain {
                blocks: 0,
                pending: Vec::with_capacity(8),
            }))
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

    /// Issue #253: the composite drain surface routes by slot kind —
    /// L0 delegates to `InProcessHost::drain_output_reports` (reports
    /// reach the caller in producer order), the L1 audio arm issues
    /// the wire-v4 `DrainOutputReports` verb (the childless fixture
    /// answers the defined transport failure `Ipc` — routing is
    /// proven by *not* getting `Unavailable`), the L1 video slot
    /// keeps the honest `Unavailable` (the video protocol has no
    /// report verb), and stale handles take `InvalidHandle`.
    #[test]
    fn drain_output_reports_routes_by_slot_kind() {
        // L0 — a reporting plugin's events reach the caller through
        // the `Shared` slot's `shared_verb`.
        let (mut host, handle) = l0_instance(Box::new(ReportGainFactory), true);
        run_block(&mut host, handle).unwrap();
        run_block(&mut host, handle).unwrap();
        let mut out = Vec::new();
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        let expected: Vec<Event> = (0..2u32).flat_map(report_pair).collect();
        assert_eq!(out, expected);
        // Repeat drain — `Drained` with nothing appended.
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Drained)
        );
        assert_eq!(out, expected);

        // L1 audio — the wire-v4 verb is issued; the childless
        // fixture's request fails the defined transport `Ipc`
        // (routing proven: no `Unavailable`, no fabricated drain).
        let (mut host, handle) = l1_instance(LifecycleState::Processing, Some(512));
        let mut out = vec![Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(9),
                value: ParamValue::ONE,
            }),
        )];
        let sentinel = out.clone();
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Err(HostError::Ipc)
        );
        assert_eq!(out, sentinel);

        // L1 video — `Unavailable`, `out` untouched (video has no
        // report surface at all).
        let mut host = PluginHost::new();
        let handle = mint_video(&mut host, LifecycleState::Processing);
        assert_eq!(
            host.drain_output_reports(handle, &mut out),
            Ok(ReportDrain::Unavailable)
        );
        assert_eq!(out, sentinel);

        // Stale handle — never minted.
        assert_eq!(
            host.drain_output_reports(PluginHandle::new(77, 0), &mut out),
            Err(HostError::InvalidHandle)
        );
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

    // ---------------------------------------------------------------
    // The isolation surface + the `Bundled` pool (Issue #117 task 3):
    // `OutOfProcessIsolation`, the struct-variant `Route`, the pool
    // scan/eviction rules and shutdown-on-empty — exercised on the
    // process-free `L1Child`/`L1Instance` fixtures; real-child
    // bundling is the T4 e2e task.
    // ---------------------------------------------------------------

    /// A `SpawnSpec` for the pool tests — legal shape, dead runner,
    /// short timeouts.
    fn pool_spec() -> SpawnSpec {
        let mut spec = SpawnSpec::new(DESCRIPTOR, OsString::from("no-such-runner-binary"));
        spec.command_timeout = Duration::from_millis(50);
        spec
    }

    /// The named route's `Bundled` pool, borrowed for assertions.
    fn route_pool(host: &PluginHost) -> &[Arc<L1Child>] {
        let Some(Route::OutOfProcess { pool, .. }) = host.routes.get(DESCRIPTOR.id) else {
            panic!("route is not the struct-variant OutOfProcess")
        };
        pool
    }

    /// A `Bundled` route on `host` whose pool holds `child`, plus
    /// two co-resident instance handles minted the way
    /// `instantiate_bundled`'s `create_instance`+`from_parts` pair
    /// would mint them.
    fn pooled_host(child: &Arc<L1Child>) -> (PluginHost, PluginHandle, PluginHandle) {
        let mut host = PluginHost::new();
        host.register_out_of_process(OutOfProcessSpec::new(
            DESCRIPTOR,
            OsString::from("no-such-runner-binary"),
        ))
        .unwrap();
        let Some(Route::OutOfProcess { pool, .. }) = host.routes.get_mut(DESCRIPTOR.id) else {
            unreachable!("just registered")
        };
        pool.push(Arc::clone(child));
        let h1 = host.mint(Slot::OutOfProcess(Box::new(L1Instance::test_fixture_on(
            child,
            0,
            LifecycleState::Created,
            None,
        ))));
        let h2 = host.mint(Slot::OutOfProcess(Box::new(L1Instance::test_fixture_on(
            child,
            1,
            LifecycleState::Created,
            None,
        ))));
        (host, h1, h2)
    }

    /// The unpublished-0.1.0 default is `Bundled` — on the enum's
    /// own `Default` and on `OutOfProcessSpec::new`'s field — while
    /// the struct literal forces an explicit choice.
    #[test]
    fn isolation_defaults_to_bundled() {
        assert_eq!(
            OutOfProcessIsolation::default(),
            OutOfProcessIsolation::Bundled
        );
        let spec = OutOfProcessSpec::new(DESCRIPTOR, OsString::from("runner"));
        assert_eq!(spec.isolation, OutOfProcessIsolation::Bundled);
        // The `Dedicated` opt-in through a struct literal.
        let dedicated = OutOfProcessSpec {
            isolation: OutOfProcessIsolation::Dedicated,
            ..OutOfProcessSpec::new(DESCRIPTOR, OsString::from("runner"))
        };
        assert_eq!(dedicated.isolation, OutOfProcessIsolation::Dedicated);
    }

    /// Registration stores the struct-variant route — `spec`,
    /// `isolation`, an empty `pool` — keyed by `descriptor.id`,
    /// for both isolation values.
    #[test]
    fn register_out_of_process_builds_the_struct_variant() {
        for isolation in [
            OutOfProcessIsolation::Dedicated,
            OutOfProcessIsolation::Bundled,
        ] {
            let mut host = PluginHost::new();
            let mut spec = OutOfProcessSpec::new(DESCRIPTOR, OsString::from("runner"));
            spec.isolation = isolation;
            spec.lookahead_blocks = 5;
            host.register_out_of_process(spec).unwrap();
            let Some(Route::OutOfProcess {
                spec,
                isolation: stored,
                pool,
            }) = host.routes.get(DESCRIPTOR.id)
            else {
                panic!("route must be the struct-variant OutOfProcess")
            };
            assert_eq!(*stored, isolation);
            assert!(pool.is_empty());
            assert_eq!(spec.lookahead_blocks, 5);
            assert_eq!(spec.descriptor, DESCRIPTOR);
        }
    }

    /// The `Bundled` scan: the first `pool_eligible` child serves;
    /// terminal entries are evicted as the scan passes; a full or
    /// draining child is skipped in place; an empty pool spawns
    /// fresh — here failing fast on the dead runner — without
    /// leaving a poisoned entry.
    #[test]
    fn bundled_scan_selects_evicts_and_spawns() {
        let diag = DiagRegistry::new();
        // A dead entry at index 0 is evicted on the way to the
        // eligible child. `create_instance` then answers
        // `SpawnFailed` (the fixture has no driver channel — the
        // request never leaves, so no cause was recorded and the
        // `Err(Ipc)` arm classifies `Handshake`) — the rollback
        // frees only the minted entry: the registered sibling
        // stays, the child stays live and stays pooled.
        let terminal = L1Child::test_fixture(1);
        terminal.test_converge();
        let live = L1Child::test_fixture(1);
        let mut pool = vec![Arc::clone(&terminal), Arc::clone(&live)];
        assert!(matches!(
            instantiate_bundled(&pool_spec(), &mut pool, &diag, None),
            Err(HostError::SpawnFailed {
                cause: FailureCause::Handshake,
                ..
            })
        ));
        assert_eq!(pool.len(), 1);
        assert!(Arc::ptr_eq(&pool[0], &live));
        assert_eq!(live.live_count(), 1);
        assert!(live.pool_eligible());
        assert!(!live.is_terminal());

        // A full child is skipped without eviction — the fresh
        // spawn's `SpawnFailed` (dead runner) is the proof the full
        // child did not serve: had it been selected, `create_instance`
        // would have answered `Err(InstanceLimit)` instead.
        let full = L1Child::test_fixture(wire::WIRE_MAX_INSTANCES);
        let mut pool = vec![Arc::clone(&full)];
        assert!(matches!(
            instantiate_bundled(&pool_spec(), &mut pool, &diag, None),
            Err(HostError::SpawnFailed { .. })
        ));
        assert_eq!(pool.len(), 1);
        assert!(Arc::ptr_eq(&pool[0], &full));

        // A `shutdown_initiated` child is likewise skipped but kept.
        let draining = L1Child::test_fixture(1);
        draining.test_set_shutdown_initiated();
        let mut pool = vec![Arc::clone(&draining)];
        assert!(matches!(
            instantiate_bundled(&pool_spec(), &mut pool, &diag, None),
            Err(HostError::SpawnFailed { .. })
        ));
        assert_eq!(pool.len(), 1);
        assert!(Arc::ptr_eq(&pool[0], &draining));

        // An empty pool pays a fresh spawn — and a spawn failure
        // leaves no entry behind to poison later scans.
        let mut pool: Vec<Arc<L1Child>> = Vec::new();
        assert!(matches!(
            instantiate_bundled(&pool_spec(), &mut pool, &diag, None),
            Err(HostError::SpawnFailed { .. })
        ));
        assert!(pool.is_empty());
    }

    /// The shared spec-legality gate rejects before any spawn or
    /// pool mutation — `Err(Ipc)`, nothing poisoned — on both
    /// isolations.
    #[test]
    fn instantiate_rejects_bad_spec_before_pooling() {
        for isolation in [
            OutOfProcessIsolation::Dedicated,
            OutOfProcessIsolation::Bundled,
        ] {
            let mut host = PluginHost::new();
            let spec = OutOfProcessSpec {
                lookahead_blocks: 0,
                isolation,
                ..OutOfProcessSpec::new(DESCRIPTOR, OsString::from("runner"))
            };
            host.register_out_of_process(spec).unwrap();
            assert!(matches!(
                host.execute(HostCommand::Instantiate {
                    plugin_id: DESCRIPTOR.id.to_owned(),
                }),
                Err(HostError::Ipc)
            ));
            assert!(route_pool(&host).is_empty());
        }
    }

    /// OBS-04 (Issue #279): `set_observation_registrar` flips the
    /// attach state (`None` default keeps pre-OBS behaviour), and
    /// `instantiate_bundled` threads it to `create_instance` —
    /// where a mint that never lands `Ok` (the fixture child owns
    /// no driver channel, so `request` fails fast and folds to
    /// `SpawnFailed`) registers nothing: the rollback runs before
    /// the Live flip the registration waits on.
    #[test]
    fn registrar_attach_state_and_failed_mint_registers_nothing() {
        let mut host = PluginHost::new();
        assert!(host.observation_registrar.is_none());
        let registrar =
            Arc::new(crate::l1::sources::test_support::RecordingRegistrar::accepting(4));
        let as_registrar = || -> Arc<dyn ObservationRegistrar> { registrar.clone() };
        host.set_observation_registrar(Some(as_registrar()));
        assert!(host.observation_registrar.is_some());
        host.set_observation_registrar(None);
        assert!(host.observation_registrar.is_none());
        host.set_observation_registrar(Some(as_registrar()));

        let spec = SpawnSpec::new(
            PluginDescriptor {
                id: "org.ruviform.test.bundled-registrar",
                name: "bundled-registrar",
                vendor: "Ruviform",
                version: "0",
                features: &[],
            },
            OsString::from("unused-runner"),
        );
        let diag = DiagRegistry::new();
        let mut pool = vec![L1Child::test_fixture(0)];
        let result = instantiate_bundled(
            &spec,
            &mut pool,
            &diag,
            host.observation_registrar.as_deref(),
        );
        assert!(matches!(
            result,
            Err(HostError::SpawnFailed {
                cause: FailureCause::Handshake,
                ..
            })
        ));
        assert_eq!(registrar.accepted(), 0);
    }

    /// `Destroy` evicts and tears the child down only when the
    /// registry just emptied — a live sibling keeps it pooled and
    /// serving.
    #[test]
    fn bundled_destroy_tears_down_only_when_empty() {
        let child = L1Child::test_fixture(0);
        let (mut host, h1, h2) = pooled_host(&child);

        host.execute(HostCommand::Destroy { handle: h1 }).unwrap();
        // The sibling still registers — the child stays pooled and
        // un-torn-down.
        assert_eq!(child.live_count(), 1);
        assert_eq!(route_pool(&host).len(), 1);
        assert!(!child.is_terminal());

        host.execute(HostCommand::Destroy { handle: h2 }).unwrap();
        // Empty now — the route evicted it and bounded teardown ran
        // (the fixture child stamps `Terminal::Clean` — no process
        // to `Shutdown`/reap).
        assert_eq!(child.live_count(), 0);
        assert!(route_pool(&host).is_empty());
        assert!(child.is_terminal());
    }

    /// `kill_child` is child-scoped: on a `Bundled` route the
    /// driver-observed death converges *every* co-resident instance
    /// — one `InstanceFailed` notification each — and the pool
    /// sheds the dead child at the next touch. (The fixture child
    /// has no process, so `kill` itself is a no-op; the death is
    /// stamped by `test_converge` — the driver's `try_wait` half is
    /// the e2e lane's.)
    #[test]
    fn bundled_child_death_fans_out_to_siblings() {
        let child = L1Child::test_fixture(0);
        let (mut host, h1, h2) = pooled_host(&child);

        // L0 slots have no child; unknown handles miss the table.
        let (mut l0, l0_handle) = l0_instance(Box::new(SendGainFactory), false);
        assert!(matches!(
            l0.kill_child(l0_handle),
            Err(HostError::Unsupported)
        ));
        assert!(matches!(
            l0.kill_child(PluginHandle::new(77, 0)),
            Err(HostError::InvalidHandle)
        ));

        // The kill routes to the child — then the driver-side death
        // observation converges every co-resident instance.
        host.kill_child(h1).unwrap();
        child.test_converge();
        assert_eq!(host.lifecycle_state(h1), Some(LifecycleState::Failed));
        assert_eq!(host.lifecycle_state(h2), Some(LifecycleState::Failed));
        // One `InstanceFailed` per co-resident instance — the
        // child-scope fan-out.
        let mut failed = Vec::new();
        while let Some(HostNotification::InstanceFailed { handle, exit, .. }) =
            host.try_recv_notification()
        {
            assert_eq!(exit.code, Some(1));
            failed.push(handle);
        }
        assert_eq!(failed.len(), 2);
        assert!(failed.contains(&h1) && failed.contains(&h2));

        // `Failed` instances stay destroyable; the emptied child is
        // evicted and converged at the last `Destroy`.
        host.execute(HostCommand::Destroy { handle: h1 }).unwrap();
        assert_eq!(route_pool(&host).len(), 1);
        host.execute(HostCommand::Destroy { handle: h2 }).unwrap();
        assert!(route_pool(&host).is_empty());
        assert!(child.is_terminal());
    }

    /// `Dedicated` is the capacity-1 route: the facade is the
    /// child's only referrer, so `destroy` runs the child's own
    /// last-referrer teardown — the `Arc` fully releases — and the
    /// structural `pool` stays empty throughout.
    #[test]
    fn dedicated_destroy_is_capacity_1() {
        let mut host = PluginHost::new();
        let mut spec = OutOfProcessSpec::new(DESCRIPTOR, OsString::from("runner"));
        spec.isolation = OutOfProcessIsolation::Dedicated;
        host.register_out_of_process(spec).unwrap();
        let child = L1Child::test_fixture(0);
        let weak = Arc::downgrade(&child);
        let handle = host.mint(Slot::OutOfProcess(Box::new(L1Instance::test_fixture_on(
            &child,
            0,
            LifecycleState::Created,
            None,
        ))));
        // The facade is now the only strong referrer — the
        // dedicated shape (no pool slot holding the child).
        drop(child);
        host.execute(HostCommand::Destroy { handle }).unwrap();
        // Teardown ran inside `destroy` at `strong_count == 1`; the
        // slot drop released the last `Arc<L1Child>`.
        assert!(weak.upgrade().is_none());
        assert!(route_pool(&host).is_empty());
    }

    // ---------------------------------------------------------------
    // Issue #118 T3 — the video surface on the composite: the
    // kind-filtered `Scan`s, the shared plugin-id namespace, the
    // cross-kind `UnknownPluginId`/`Unsupported` answers, the
    // video slot's no-RT-surface replies, the `Instantiate` bounds
    // gate and the host-local `SetParamValue` finiteness check.
    // `VideoL1Instance` fixtures carry no spawned child — the wire
    // legs answer `Err(Ipc)` past the defined local rejections.
    // ---------------------------------------------------------------

    use ruviform_media_core::{FrameRate, MediaTime, TimeBase};
    use ruviform_video_core::{PixelFormat, VideoFrame};

    const VIDEO_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.composite-vfx",
        name: "Test Composite Vfx",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    fn video_config(width: u32, height: u32) -> VideoProcessConfig {
        VideoProcessConfig::new(
            PixelFormat::Yuv420p,
            width,
            height,
            FrameRate::new(30, 1).unwrap(),
        )
        .unwrap()
    }

    fn video_spec(descriptor: PluginDescriptor) -> VideoOutOfProcessSpec {
        VideoOutOfProcessSpec::new(descriptor, OsString::from("unused-video-runner"))
    }

    /// Mints a video slot on a fixture instance — the unified
    /// table's `Slot::OutOfProcessVideo` seat every cross-kind
    /// probe needs.
    fn mint_video(host: &mut PluginHost, mirror: LifecycleState) -> PluginHandle {
        host.mint(Slot::OutOfProcessVideo(Box::new(
            VideoL1Instance::test_fixture(mirror, video_config(64, 48)),
        )))
    }

    /// `Scan` reports only its own kind's registrations — audio
    /// (factory + L1) on the audio surface, video on the video
    /// surface, each in registration order.
    #[test]
    fn scans_filter_by_registration_kind() {
        const AUDIO_L1: PluginDescriptor = PluginDescriptor {
            id: "org.ruviform.test.composite-audio-l1",
            name: "Test Composite Audio L1",
            vendor: "Ruviform",
            version: "0.1.0",
            features: &[],
        };
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        host.register_out_of_process(OutOfProcessSpec::new(
            AUDIO_L1,
            OsString::from("unused-runner"),
        ))
        .unwrap();
        host.video()
            .register_out_of_process(video_spec(VIDEO_DESCRIPTOR))
            .unwrap();
        let HostReply::Scanned(audio) = host.execute(HostCommand::Scan).unwrap() else {
            panic!("Scan replies only Scanned")
        };
        assert_eq!(
            audio.iter().map(|d| d.id).collect::<Vec<_>>(),
            [DESCRIPTOR.id, AUDIO_L1.id]
        );
        let VideoHostReply::Scanned(video) = host.video().execute(VideoHostCommand::Scan).unwrap()
        else {
            panic!("video Scan replies only Scanned")
        };
        assert_eq!(
            video.iter().map(|d| d.id).collect::<Vec<_>>(),
            [VIDEO_DESCRIPTOR.id]
        );
    }

    /// The plugin-id namespace is one across both surfaces —
    /// `DuplicatePluginId` fires whether the first claim was audio
    /// or video, factory or L1.
    #[test]
    fn duplicate_plugin_id_rejected_across_kinds() {
        // Audio L1 first → video claim rejected.
        let mut host = PluginHost::new();
        host.register_out_of_process(OutOfProcessSpec::new(
            VIDEO_DESCRIPTOR,
            OsString::from("unused-runner"),
        ))
        .unwrap();
        assert_eq!(
            host.video()
                .register_out_of_process(video_spec(VIDEO_DESCRIPTOR)),
            Err(HostError::DuplicatePluginId)
        );
        // Video first → audio L1 claim rejected.
        let mut host = PluginHost::new();
        host.video()
            .register_out_of_process(video_spec(VIDEO_DESCRIPTOR))
            .unwrap();
        assert_eq!(
            host.register_out_of_process(OutOfProcessSpec::new(
                VIDEO_DESCRIPTOR,
                OsString::from("unused-runner"),
            )),
            Err(HostError::DuplicatePluginId)
        );
        // L0 factory first → video claim rejected (the routes map
        // holds every kind's claim).
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        assert_eq!(
            host.video().register_out_of_process(video_spec(DESCRIPTOR)),
            Err(HostError::DuplicatePluginId)
        );
    }

    /// `Instantiate` can never cross kinds — an audio
    /// `Instantiate` naming a video route, or a video
    /// `Instantiate` naming an audio route, is
    /// `Err(UnknownPluginId)`: a handle the caller has no verbs
    /// for must never be minted.
    #[test]
    fn cross_kind_instantiate_is_unknown_plugin_id() {
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        host.video()
            .register_out_of_process(video_spec(VIDEO_DESCRIPTOR))
            .unwrap();
        assert_eq!(
            host.execute(HostCommand::Instantiate {
                plugin_id: VIDEO_DESCRIPTOR.id.to_owned(),
            }),
            Err(HostError::UnknownPluginId)
        );
        assert!(matches!(
            host.video().execute(VideoHostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
                config: video_config(64, 48),
            }),
            Err(HostError::UnknownPluginId)
        ));
    }

    /// Every handle-carrying verb rejects cross-kind with
    /// `Err(Unsupported)`: audio commands on a video slot, video
    /// commands (dispatch and `Render` legs alike) on an L0 slot.
    #[test]
    fn cross_kind_commands_are_unsupported() {
        let mut host = PluginHost::new();
        host.register_factory(Box::new(SendGainFactory));
        let HostReply::Instantiated {
            handle: audio_handle,
        } = host
            .execute(HostCommand::Instantiate {
                plugin_id: DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        else {
            panic!("unexpected reply")
        };
        let video_handle = mint_video(&mut host, LifecycleState::Processing);
        // Audio verbs against the video slot.
        assert_eq!(
            host.execute(HostCommand::Initialize {
                handle: video_handle,
            }),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            host.execute(HostCommand::SaveState {
                handle: video_handle,
            }),
            Err(HostError::Unsupported)
        );
        // Video verbs against the L0 slot — both dispatch legs.
        assert!(matches!(
            host.video().execute(VideoHostCommand::GetParams {
                handle: audio_handle,
            }),
            Err(HostError::Unsupported)
        ));
        assert!(matches!(
            host.video().execute(VideoHostCommand::Initialize {
                handle: audio_handle,
            }),
            Err(HostError::Unsupported)
        ));
        let frame = VideoFrame::new_owned(
            PixelFormat::Yuv420p,
            64,
            48,
            MediaTime::new(0, TimeBase::COMMON_90K),
        )
        .unwrap();
        assert!(matches!(
            host.video().execute(VideoHostCommand::Render {
                handle: audio_handle,
                input: frame,
                at: MediaTime::new(0, TimeBase::COMMON_90K),
            }),
            Err(HostError::Unsupported)
        ));
    }

    /// A video slot's answers on the audio-only surfaces: `process`
    /// is `Err(Unsupported)`, `latency_frames`/`plugin_mut` are
    /// `None`, both endpoint entry points are
    /// `Err(EndpointUnavailable)`, while the shared
    /// `lifecycle_state`/`kill_child`/`Destroy` semantics still
    /// apply.
    #[test]
    fn video_slot_answers_the_audio_only_surfaces() {
        let mut host = PluginHost::new();
        let handle = mint_video(&mut host, LifecycleState::Processing);
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );
        assert_eq!(host.latency_frames(handle), None);
        assert!(host.plugin_mut(handle).is_none());
        assert!(matches!(
            run_block(&mut host, handle),
            Err(HostError::Unsupported)
        ));
        assert!(matches!(
            host.prepare_endpoint(handle, bounds(512, 2)),
            Err(HostError::EndpointUnavailable)
        ));
        assert!(matches!(
            host.attach_endpoint(handle, bounds(512, 2)),
            Err(HostError::EndpointUnavailable)
        ));
        // `kill_child` routes the video arm — `Ok`, and the kill
        // no-ops on the fixture's process-free shared child.
        assert_eq!(host.kill_child(handle), Ok(()));
        // `Processing`-mirrored `Destroy` keeps the L0 legality
        // rule on the video arm too.
        assert!(matches!(
            host.execute(HostCommand::Destroy { handle }),
            Err(HostError::Lifecycle(_))
        ));
    }

    /// `Destroy` reaches the video slot through both surfaces —
    /// the audio `HostCommand::Destroy` and the video
    /// `VideoHostCommand::Destroy` are the same legality rule and
    /// the same slot free.
    #[test]
    fn video_destroy_works_through_both_surfaces() {
        let mut host = PluginHost::new();
        let audio_surface = mint_video(&mut host, LifecycleState::Created);
        assert_eq!(
            host.execute(HostCommand::Destroy {
                handle: audio_surface,
            }),
            Ok(HostReply::Ok)
        );
        assert_eq!(host.lifecycle_state(audio_surface), None);
        let video_surface = mint_video(&mut host, LifecycleState::Created);
        assert!(matches!(
            host.video().execute(VideoHostCommand::Destroy {
                handle: video_surface,
            }),
            Ok(VideoHostReply::Ok)
        ));
        assert_eq!(host.lifecycle_state(video_surface), None);
    }

    /// A converged video instance's `ChildExit` lands on the shared
    /// notification queue as `InstanceFailed{handle}` — same payload
    /// as the audio leg, handle from the unified table.
    #[test]
    fn video_failure_notification_uses_the_unified_handle() {
        let mut host = PluginHost::new();
        let child = L1Child::test_fixture(0);
        let handle = host.mint(Slot::OutOfProcessVideo(Box::new(
            VideoL1Instance::test_fixture_on(
                &child,
                0,
                LifecycleState::Processing,
                video_config(64, 48),
            ),
        )));
        child.test_converge();
        assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));
        match host.try_recv_notification() {
            Some(HostNotification::InstanceFailed {
                handle: reported, ..
            }) => assert_eq!(reported, handle),
            other => panic!("expected InstanceFailed, got {other:?}"),
        }
        // The one-shot queue — nothing further lands.
        assert!(host.try_recv_notification().is_none());
        // `Failed` stays destroyable — the slot frees normally.
        assert_eq!(
            host.execute(HostCommand::Destroy { handle }),
            Ok(HostReply::Ok)
        );
    }

    /// The `Instantiate` bounds gate runs before any allocation,
    /// segment or spawn: dims over `WIRE_VIDEO_MAX_DIM` and a
    /// `frame_bytes`/`segment_len` over the wire caps are all the
    /// defined `Err(Prepare(UnsupportedConfig))`.
    #[test]
    fn video_instantiate_rejects_out_of_bounds_configs() {
        let mut host = PluginHost::new();
        host.video()
            .register_out_of_process(video_spec(VIDEO_DESCRIPTOR))
            .unwrap();
        let rate = FrameRate::new(30, 1).unwrap();
        for config in [
            // Dims over the wire cap.
            VideoProcessConfig::new(PixelFormat::Yuv420p, wire::WIRE_VIDEO_MAX_DIM + 1, 4, rate)
                .unwrap(),
            VideoProcessConfig::new(PixelFormat::Yuv420p, 4, wire::WIRE_VIDEO_MAX_DIM + 1, rate)
                .unwrap(),
            // In-dims but `frame_bytes`/`segment_len` over the caps
            // (8K `RgbaF32` honestly exceeds both).
            VideoProcessConfig::new(PixelFormat::RgbaF32, 7_680, 4_320, rate).unwrap(),
            // Saturating math — `u32::MAX` dims can never wrap below
            // the bound.
            VideoProcessConfig::new(PixelFormat::RgbaF32, u32::MAX, u32::MAX, rate).unwrap(),
        ] {
            assert!(
                matches!(
                    host.video().execute(VideoHostCommand::Instantiate {
                        plugin_id: VIDEO_DESCRIPTOR.id.to_owned(),
                        config,
                    }),
                    Err(HostError::Prepare(PrepareError::UnsupportedConfig))
                ),
                "{config:?} must reject before any spawn"
            );
        }
    }

    /// Encode obligation (a): a non-finite
    /// `PlainParamValue::Double` is `Err(InvalidValue)` *before* the
    /// wire — the fixture's dead driver is never consulted — while
    /// a finite `Double` and the other value kinds pass the local
    /// gate and reach the wire (the fixture's `Err(Ipc)`).
    #[test]
    fn video_set_param_rejects_non_finite_values() {
        let mut host = PluginHost::new();
        let handle = mint_video(&mut host, LifecycleState::Processing);
        for value in [
            PlainParamValue::Double(f64::NAN),
            PlainParamValue::Double(f64::INFINITY),
            PlainParamValue::Double(f64::NEG_INFINITY),
        ] {
            assert!(
                matches!(
                    host.video().execute(VideoHostCommand::SetParamValue {
                        handle,
                        id: ParamId(0),
                        value,
                    }),
                    Err(HostError::InvalidValue)
                ),
                "{value:?} must reject host-locally"
            );
        }
        // Finite `Double` and the non-`Double` kinds pass the gate
        // — the fixture's dead wire answers `Err(Ipc)`, proving the
        // value left the host.
        for value in [
            PlainParamValue::Double(0.5),
            PlainParamValue::Integer(3),
            PlainParamValue::Boolean(true),
            PlainParamValue::Choice(1),
        ] {
            assert!(
                matches!(
                    host.video().execute(VideoHostCommand::SetParamValue {
                        handle,
                        id: ParamId(0),
                        value,
                    }),
                    Err(HostError::Ipc)
                ),
                "{value:?} must reach the wire"
            );
        }
    }
}
