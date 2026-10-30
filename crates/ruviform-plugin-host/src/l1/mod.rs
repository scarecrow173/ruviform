//! The `pub(crate)` L1 out-of-process instance machinery — the
//! shared-child bundle model of Issue #117: one child process hosts
//! N plugin instances (the dedicated route is the capacity-1 case,
//! built over exactly the same machinery), a framed stdio control
//! plane, three per-child I/O threads and the scope-split `Failed`
//! convergence the `docs/designs/issue-117-bundle-child-process.md`
//! demux table fixes.
//!
//! ## Scope
//!
//! Both planes live here: the **control plane** (spawn, the
//! `Hello`/`Scan` handshake, `Command`/`Reply` parity for the L0
//! command set, crash detection and teardown) and the shared-memory
//! **data plane** — `l1::shm` owns the named segment and the
//! I/O-thread `View`s; `l1::data` carries the `BlockIn`/`BlockDone`
//! doorbells, the cross-thread slot staging and the RT try-legs
//! behind [`L1Instance::process`]. The `crate::composite::PluginHost`
//! facade is this module's only in-crate consumer; nothing here is
//! public — the public surface (`PluginHost`/`OutOfProcessSpec`/
//! `HostNotification`/`ChildExit`) lives in `composite.rs`.
//!
//! ## Import discipline
//!
//! `tests/l1_proxy_e2e.rs` pulls this module in by `#[path]` because
//! the proxy is `pub(crate)` and cannot be reached through the crate's
//! public API yet. Everything in `src/l1/` must therefore resolve
//! without `crate::` — use `ruviform_plugin_core::` / `std::` paths
//! and `self::`/`super::` only. Do not `use crate::...` in this
//! module tree.
//!
//! ## Thread model (per child = 1 process + 3 threads)
//!
//! | thread | owns | duties |
//! | --- | --- | --- |
//! | `driver` | `Shared.child` (`Mutex<Child>` — operated only here, but shared so a converging thread can `kill` a wedged child even while this thread is blocked in `write_all`) · child stdin | serializes outgoing frames · installs/uninstalls per-instance data-plane ends on `DriverCmd::AddInstance`/`RemoveInstance` (the `submits`/`views` maps are keyed by instance) · drains every instance's submit channel (a seq dropped without a doorbell — stale generation, `Destroying` gate, dead or shutting-down child — still gets exactly one synthesized `mark_failed` completion so its leg's alternation gate never wedges): each doorbell'd seq becomes a shm in-slot write + `BlockIn{instance, seq}` · `try_wait` crash watch every [`DRIVER_POLL_INTERVAL`] · shutdown watchdog (grace → `child.kill()`) |
//! | `reader` | child stdout | demuxes child→parent frames by the Issue #117 decision table — unflagged `Reply` → `(instance, seq)` waiter correlation · flagged `Reply`/`BlockDone` → `(instance, seq)` block completion + lazy per-instance `View` attach · first frame must be `Hello` · EOF → `seen_eof` convergence path |
//! | `stderr drain` | child stderr · `Shared.diag` | pushes stderr bytes into the capture's bounded ring — started before the `Hello` wait so pre-handshake output is captured · ends on pipe EOF, or [`diag::STDERR_DRAIN_GRACE`] after the child's exit is observed (`Inner.terminal` set — never extended) · closes the read end on exit, so an inheriting grandchild's later writes fail instead of being read silently · detached, never joined — self-bounded |
//!
//! The caller thread never performs blocking pipe I/O except the
//! bounded waits documented on [`L1Instance::execute`] /
//! [`L1Instance::spawn`] (each capped by `command_timeout`).
//!
//! **Lock order:** `Shared.inner` and `Shared.child` critical
//! sections are always disjoint — no thread holds one while
//! acquiring the other, so no cross-lock deadlock exists. `child`
//! sections stay tiny (one `try_wait`/`kill`/`wait` call; `reap`
//! re-acquires per poll) so a converging `kill` never queues behind
//! a long hold. Each instance's `InstShared.result_tx` is plain
//! data — the `!Sync` producer's bounded `try_push`es run inside
//! the `inner` section that already guards the `instances` map.
//!
//! ## `Failed` convergence — child scope vs instance scope
//!
//! Crash detection has two paths that converge idempotently:
//! (i) the reader observes stdout EOF (`seen_eof`), (ii) the driver's
//! `Child::try_wait` returns `Some(ExitStatus)` (reaping as it goes).
//! Whichever fires first performs the one-time `converge_child` —
//! `terminal = Failed`, every registered instance's lifecycle mirror
//! moves to `LifecycleState::Failed`, every pending command waiter is
//! woken with `Err(HostError::Ipc)`, every in-flight doorbell gets a
//! synthesized `mark_failed` completion, and each *minted* instance
//! (a `Creating` entry minted no public handle — mid-create deaths
//! report `SpawnFailed` on the `Instantiate` caller instead) gets
//! exactly one `ChildExit` notification, all carrying the same exit
//! status. Any exit while the child is live — including a voluntary
//! `exit(0)` — is `Failed` (a protocol violation); only an exit
//! after `Shutdown` was initiated is a clean teardown
//! (`terminal = Clean`, mirrors → `Destroyed`, no notification).
//! There is no automatic restart.
//!
//! The per-instance half — `converge_instance(i)` — is what keeps
//! `Failed` granular on a shared child: a violation the demux table
//! attributes to live instance `i` (a `BlockDone`/flagged `Reply`
//! for an `(i, seq)` never rung) fails `i` alone — mirror `Failed`,
//! its waiters `Err(Ipc)`, its doorbells `mark_failed`, one
//! `ChildExit{code:None, signal:None}` notification (the child is
//! alive — there is no exit status to report) — while `terminal`,
//! `seen_eof`, `shutdown_initiated` and every sibling stay
//! untouched. The entry remains registered — the runner-side slot
//! still exists — until `DestroyInstance` reclaims it.
//!
//! Child-scope violations — an unattributable frame (`Hello` twice,
//! a parent→child kind, a waiter-less `Reply` on a *live* instance,
//! a `Reply` whose `instance` echo mismatches the waiter's stamp, a
//! `WIRE_INSTANCE_NONE` or out-of-range stamp on a block frame), a
//! malformed frame, a stdin write error, a `command_timeout` —
//! converge the same way: the converging thread itself kills the
//! child through `Shared.child` — that is what unwedges a driver
//! blocked in a pipe `write_all` on a live-but-not-reading child —
//! and pending waiters wake `Err(HostError::Ipc)`. Frames targeting
//! registered-but-not-live entries (`Creating`/`Destroying`/
//! `Failed`) or removed instances are inert drops — never
//! violations; a `BlockDone{i, seq}` whose doorbell is still in
//! flight completes normally even while `i` is `Destroying` (the
//! doorbell window), and any frame arriving after `Shutdown` was
//! initiated is a non-event.

pub(crate) mod data;
pub(crate) mod diag;
pub(crate) mod shm;
pub(crate) mod video;
pub(crate) mod vshm;

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::Write;
use std::process::{
    Child, ChildStderr, ChildStdin, ChildStdout, Command as ProcCommand, ExitStatus, Stdio,
};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_observability::{Counter, CounterReader};
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, FailureCause, HostError, HostInput, HostOutput, LifecycleState, PluginDescriptor,
    PrepareError, ProcessConfig, ProcessResult, TransportInfo, VideoProcessConfig,
};

// `composite.rs` is the only in-crate consumer of the video facade
// — inside the `#[path]` test include these re-exports are dead.
#[allow(unused_imports)]
pub(crate) use video::{VideoL1Instance, VideoSpawnSpec};

/// The driver's crash-watch and command-drain poll interval
/// (~200 µs per the design; the effective granularity is the OS
/// scheduler's, which is coarser — the interval paces a `try_wait`
/// poll, it is not a deadline).
const DRIVER_POLL_INTERVAL: Duration = Duration::from_micros(200);

/// Poll cadence for the bounded driver join in
/// [`L1Child::join_driver`].
const DRIVER_JOIN_POLL: Duration = Duration::from_millis(2);

/// Extra time past `command_timeout` (the shutdown grace) that
/// `join_driver` waits for the driver before force-killing a wedged
/// child — covers the driver's own watchdog kill + reap turnaround.
const DRIVER_JOIN_MARGIN: Duration = Duration::from_secs(1);

/// Process-wide instance counter feeding [`shm::unique_key`]'s `slot`
/// component (the composite's public slot numbers live in
/// `crate::composite` and are not shared here — this only needs to
/// discriminate instances for key uniquing).
static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How the child process ended — reported on the notification queue
/// inside `HostNotification::InstanceFailed`.
/// `signal` is `Some` only on unix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChildExit {
    /// Exit code, if the child exited with one (`None` when killed by
    /// a signal on unix, or when convergence ran before the status
    /// was observed — e.g. the EOF path fired first).
    pub(crate) code: Option<i32>,
    /// Terminating signal, unix only (`None` on Windows).
    pub(crate) signal: Option<i32>,
}

impl ChildExit {
    fn from_status(status: ExitStatus) -> Self {
        #[cfg(unix)]
        let signal = std::os::unix::process::ExitStatusExt::signal(&status);
        #[cfg(not(unix))]
        let signal = None;
        Self {
            code: status.code(),
            signal,
        }
    }
}

/// The per-instance `Failed` notification payload — the converged
/// [`ChildExit`] (status-unknown fields stay `None` when the status
/// never arrived) plus the *first-observed* [`FailureCause`] fixed
/// under the convergence lock: a later observation — including the
/// driver's reap completing a `Notify::Pending` — supplies status
/// only and never overwrites the cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FailedNotice {
    pub(crate) exit: ChildExit,
    pub(crate) cause: FailureCause,
}

/// Spawn parameters for one L1 instance — the `pub(crate)` mirror of
/// the public `crate::composite::OutOfProcessSpec`.
#[derive(Debug)]
pub(crate) struct SpawnSpec {
    /// Registration-supplied `'static` descriptor: supplies the
    /// `CreateInstance` `plugin_id` and the field-equality oracle for
    /// the child's `Scanned` reply.
    pub(crate) descriptor: PluginDescriptor,
    /// Runner executable.
    pub(crate) program: OsString,
    /// Extra arguments prepended to the runner's argv — the runner's
    /// own flags only (e.g. the test worker's fault knobs); wire v2
    /// has no `--ruviform-*` bootstrap flags to precede.
    pub(crate) args: Vec<OsString>,
    /// Lookahead depth in blocks (default 2) — fixes the shm slot
    /// depth `D = lookahead_blocks + 1`, both SPSC channel
    /// capacities and the RT leg's result horizon
    /// (`expected = block − lookahead`). `0` is rejected by
    /// [`SpawnSpec::instance_depth`] (`Err(Ipc)`): the
    /// just-submitted seq could never already have a result, so the
    /// leg would pass through forever. `D` also travels as
    /// `CreateInstance`'s `in_depth`/`out_depth`, so the codec's
    /// [`wire::WIRE_MAX_ELEMENTS`] depth cap bounds
    /// `lookahead_blocks` — values `>= WIRE_MAX_ELEMENTS`
    /// (`D > WIRE_MAX_ELEMENTS`) are rejected the same way.
    pub(crate) lookahead_blocks: u32,
    /// Defined timeout for the `Hello`, `Scan` and every later
    /// `Reply` wait (default 10 s).
    pub(crate) command_timeout: Duration,
}

impl SpawnSpec {
    /// A spec with the design defaults (`lookahead_blocks` 2,
    /// `command_timeout` 10 s); adjust the fields as needed.
    pub(crate) fn new(descriptor: PluginDescriptor, program: OsString) -> Self {
        Self {
            descriptor,
            program,
            args: Vec::new(),
            lookahead_blocks: 2,
            command_timeout: Duration::from_secs(10),
        }
    }

    /// `lookahead_blocks` legality plus the shm slot depth
    /// `D = lookahead_blocks + 1` — the spec-level gate the
    /// dedicated [`L1Instance::spawn`] and the pooled
    /// [`L1Child::create_instance`] callers both run before any
    /// segment or child exists. `0` is `Err(Ipc)`: the result
    /// horizon `expected = block` is the seq the same `process`
    /// call just submitted, so no result could ever arrive first
    /// and the leg would pass through forever. `>=
    /// wire::WIRE_MAX_ELEMENTS` (`D > WIRE_MAX_ELEMENTS`) is the
    /// same defined rejection — `D` is declared on
    /// `CreateInstance`'s `in_depth`/`out_depth`, which the codec
    /// caps at `WIRE_MAX_ELEMENTS`. The spec is rejected like any
    /// other unusable spawn configuration rather than silently
    /// degraded.
    pub(crate) fn instance_depth(&self) -> Result<u32, HostError> {
        if self.lookahead_blocks == 0 || self.lookahead_blocks >= wire::WIRE_MAX_ELEMENTS {
            return Err(HostError::Ipc);
        }
        // `lookahead_blocks < WIRE_MAX_ELEMENTS` here, so `+ 1`
        // cannot overflow and `depth` stays inside the codec cap.
        Ok(self.lookahead_blocks + 1)
    }

    /// The shared spawn-parameter view [`L1Child::spawn_scanned`]/
    /// [`L1Child::scan_match`] consume — the audio spec lends this;
    /// `lookahead_blocks` stays audio-only behind `create_instance`.
    pub(crate) fn view(&self) -> SpawnSpecView<'_> {
        SpawnSpecView {
            descriptor: &self.descriptor,
            program: &self.program,
            args: &self.args,
            command_timeout: self.command_timeout,
        }
    }
}

/// The borrowed spawn-parameter view [`L1Child::spawn_scanned`] and
/// [`L1Child::scan_match`] consume — every field both spec kinds
/// share, and nothing kind-specific (an audio spec's
/// `lookahead_blocks` never reaches the handshake path, so it is
/// not part of this view). `SpawnSpec::view` and
/// `VideoSpawnSpec::view` lend it, so exactly one
/// spawn/scan/handshake path serves both instance kinds.
pub(crate) struct SpawnSpecView<'a> {
    /// Registration-supplied descriptor — the `Scan` field-match
    /// oracle (`CreateInstance`/`CreateVideoInstance`'s `plugin_id`
    /// is read from the owning spec, not this view).
    pub(crate) descriptor: &'a PluginDescriptor,
    /// Runner executable.
    pub(crate) program: &'a OsString,
    /// The runner's own argv prefix.
    pub(crate) args: &'a [OsString],
    /// Defined timeout for `Hello`, `Scan` and every later
    /// non-render `Reply` wait.
    pub(crate) command_timeout: Duration,
}

/// The terminal condition the shared state converged to — the bit
/// that makes `Failed` idempotent across the EOF and `try_wait` paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Terminal {
    /// Live-process exit or protocol/transport failure — `Failed`
    /// transition, notification queued.
    Failed,
    /// Exit after `Shutdown` was initiated — clean teardown, no
    /// notification.
    Clean,
}

/// One pending command: the envelope `instance` the request was
/// stamped with (the reply's `instance` must echo it — a mismatch is
/// an attribution failure, child-scope `Failed` per the Issue #117
/// demux table) and the unbounded channel the reader sends the
/// child's reply (or the converger sends `Err(Ipc)`) onto. The
/// `instance` stamp is also what `converge_instance` drains by —
/// per-instance failure granularity on pending commands.
struct Waiter {
    /// The `instance` the matching `Reply` must echo — the
    /// request's stamped envelope value (`WIRE_INSTANCE_NONE` for
    /// `Scan`, the instance handle for everything else).
    instance: u32,
    tx: mpsc::Sender<Result<wire::Reply, HostError>>,
}

/// Whether the single `Failed` notification has been sent — the
/// notification carries the reaped `ExitStatus` when available, so a
/// convergence that ran before the status was known (the EOF path)
/// stays `Pending` until the driver's `try_wait`/`reap` supplies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Notify {
    /// No `Failed` notification owed (live or clean teardown).
    Idle,
    /// `Failed` is set but the child's exit status is not yet known —
    /// the notification goes out when the driver learns it (the
    /// converging thread force-kills the child itself, so in-protocol
    /// the driver always gets to reap a real status). The backstop
    /// for a status that never arrives — a driver thread death
    /// before the reap is theoretical only, but an entry leaving the
    /// registry mid-wait is not — is `remove_instance` flushing the
    /// status-unknown `ChildExit` before `notify_tx` drops.
    Pending,
    /// The one-shot notification was already queued.
    Sent,
}

/// Instance registration phase — the demux liveness axis and the
/// `DestroyInstance` doorbell-window driver (Issue #117 demux
/// table). `Creating`/`Destroying`/`Failed` are all
/// **registered-but-not-live**: their late frames are inert drops,
/// never violations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstPhase {
    /// Registered before the `CreateInstance` reply arrives —
    /// inert demux target; `converge_child` converges it but never
    /// notifies (no handle was minted — the `Instantiate` caller
    /// gets `Err(Ipc)` instead).
    Creating,
    /// `CreateInstance` replied `Ok` — a live instance:
    /// attributable violations converge it alone
    /// (`converge_instance`), child-scope events converge the whole
    /// child.
    Live,
    /// `DestroyInstance` was serialized to stdin — the doorbell
    /// window: in-flight `(i, seq)` doorbells still complete
    /// normally, new submits synthesize `mark_failed`, and the
    /// entry is removed when the reply lands.
    Destroying,
    /// Converged `Failed` (mirror, waiters, doorbells and the
    /// one-shot notification are done) — stays registered until
    /// `DestroyInstance` reclaims the runner registry slot.
    Failed,
}

/// The per-instance segment descriptor the I/O threads attach by
/// `key` — depths ride along for slot indexing. `PartialEq` is the
/// reader's stale-`View` oracle: a wire handle re-minted under a
/// fresh segment fails the compare and re-attaches.
#[derive(Clone, PartialEq, Eq)]
struct SegmentInfo {
    /// Named segment `os_id` — `shm::View::attach`'s input.
    key: String,
    /// In-slot ring depth (`D`) the instance's segment was created
    /// with — the `BlockIn` doorbell's slot-index modulus.
    in_depth: u32,
    /// Out-slot ring depth — the `BlockDone` slot-index modulus.
    out_depth: u32,
}

/// The reader halves of an audio instance's observability counters
/// (Issue #121) — cloned out to the non-RT query surface. `Clone`
/// shares the cells; `read` is a `Relaxed` load.
#[derive(Clone, Debug)]
pub(crate) struct AudioCounterReaders {
    /// `RtLeg::process` submit drops (busy slot / full channel).
    pub(crate) dropped_submits: CounterReader,
    /// `RtLeg::pass_through` substituted-output blocks.
    pub(crate) passthrough_submits: CounterReader,
    /// `drain_to` first-pop `RESULT_FAILED` observations.
    pub(crate) results_failed_observed: CounterReader,
    /// Reader-side `push_result` `try_push` failures.
    pub(crate) results_dropped: CounterReader,
}

/// The four observability counter pairs an `InstKind::Audio` entry
/// owns (Issue #121 — the `ruviform-observability` crate's first
/// real consumer). The field lives inside the `Audio` arm so a
/// video entry can never carry audio counters (type-level parity
/// pin, spec R-B6). Writer halves clone into `RtLeg`
/// ([`data::RtLegCounters`]) and are read by `push_result`; reader
/// halves clone out to the composite query surface — leg swaps,
/// endpoint checkout and `arm` leave the cells untouched, while a
/// re-minted instance builds fresh pairs starting at zero.
#[derive(Debug)]
pub(crate) struct AudioCounters {
    /// The three leg-side writers — cloned into `RtLeg`.
    pub(crate) leg: data::RtLegCounters,
    /// Reader-side `push_result` `try_push` failures — written from
    /// [`InstShared::push_result`] under the `inner` lock.
    pub(crate) results_dropped: Counter,
    /// Reader halves for the query surface.
    readers: AudioCounterReaders,
}

impl AudioCounters {
    /// Fresh pairs — **non-RT** (allocates the shared cells).
    pub(crate) fn new() -> Self {
        let (dropped_submits, dropped_r) = ruviform_observability::counter();
        let (passthrough_submits, passthrough_r) = ruviform_observability::counter();
        let (results_failed_observed, failed_r) = ruviform_observability::counter();
        let (results_dropped, dropped_results_r) = ruviform_observability::counter();
        Self {
            leg: data::RtLegCounters {
                dropped_submits,
                passthrough_submits,
                results_failed_observed,
            },
            results_dropped,
            readers: AudioCounterReaders {
                dropped_submits: dropped_r,
                passthrough_submits: passthrough_r,
                results_failed_observed: failed_r,
                results_dropped: dropped_results_r,
            },
        }
    }

    /// The reader-half bundle — an `Arc`-clone-per-field.
    pub(crate) fn readers(&self) -> AudioCounterReaders {
        self.readers.clone()
    }
}

/// The kind-specific half of an [`InstShared`] entry — the fields
/// that exist only for one instance kind. This is the type-level
/// pin that a video entry carries **no audio data-plane fields**:
/// the audio submit/result/staging/shm machinery is behind the
/// `Audio` variant, so a demux or driver path that needs it can
/// only ever name an audio entry.
enum InstKind {
    /// An audio (`CreateInstance`) entry — the L1 data plane's
    /// per-instance ends.
    Audio {
        /// The armed data-plane staging ring — `Some` after a
        /// successful `Activate`, replaced on re-activation. `Arc`
        /// so the I/O threads borrow it under the lock without
        /// holding the lock across shm work.
        staging: Option<Arc<data::Staging>>,
        /// Segment descriptor — the reader's lazy `View` attach and
        /// both threads' slot indexing read it.
        shm: SegmentInfo,
        /// The instance's result-channel producer — `!Sync`, behind
        /// a **leaf lock** so the bounded `try_push`es run inside
        /// the `inner` critical section that guards the map through
        /// an `&InstShared` (both I/O threads complete seqs here:
        /// the reader pushes real `BlockDone`/rejection
        /// completions, the driver pushes the synthesized
        /// `mark_failed` every doorbell-less drop owes the leg).
        result_tx: Mutex<ruviform_rt::Producer<u64>>,
        /// The four observability counter pairs — leg writers feed
        /// `RtLeg` via [`data::RtLegCounters`] clones, `results_dropped`
        /// feeds `push_result`, readers feed the query surface.
        counters: AudioCounters,
    },
    /// A video (`CreateVideoInstance`) entry — no audio data plane:
    /// no submit/result channels, no staging ring, no slot-ring
    /// segment. Its frame transport is the facade-owned
    /// [`vshm::Segment`], which never enters the registry (it lives
    /// on `VideoL1Instance`, reachable to the I/O threads never).
    Video {
        /// The `CreateVideoInstance` `config` — the instance's
        /// format/dims/frame-rate contract, pinned at mint so a
        /// later `VideoRender`'s input-desc check and the
        /// `Rendered` output gate read the same authority the
        /// runner stashed.
        config: VideoProcessConfig,
    },
}

/// Per-instance state — the value half of `Inner.instances` (wire
/// handle → instance). Every field mutates only under the
/// `Shared.inner` lock; `rt_state` is additionally readable
/// lock-free by the RT leg and the `L1Instance` facade (they clone
/// the `Arc`).
struct InstShared {
    /// The const-cheap lifecycle snapshot the RT leg inspects —
    /// written `Release` by `set_mirror`, read `Acquire` by
    /// `L1Instance::process` and a detached endpoint's mirror.
    /// Fails closed: the RT side never takes the mutex.
    rt_state: Arc<AtomicU8>,
    /// Local lifecycle mirror — the child's `Lifecycle` stays the
    /// authority; this only reflects reported outcomes.
    mirror: LifecycleState,
    /// The instance's own failure notification endpoint — a child
    /// crash fans one [`FailedNotice`] out to every minted
    /// instance's queue.
    notify_tx: mpsc::Sender<FailedNotice>,
    /// One-shot `Failed` notification state.
    notify: Notify,
    /// The first-observed [`FailureCause`] for *this instance's*
    /// failure — set under the `inner` lock and never overwritten by
    /// a later observation (the convergence lock serializes
    /// reader/driver/reap/timeout competition). Instance-scope
    /// (`converge_instance`) and child-scope (`converge_child`)
    /// causes share this field; the notification carries it
    /// verbatim. `None` while the entry has not failed.
    cause: Option<FailureCause>,
    /// The kind-specific payload — audio data-plane ends or the
    /// video config pin.
    kind: InstKind,
    /// Demux liveness + the `DestroyInstance` doorbell window.
    phase: InstPhase,
}

impl InstShared {
    /// The demux-table liveness test: registered, unconverged and
    /// not `Destroying` — `Creating` counts as registered-but-not-
    /// live (no handle minted; its only legal frame is the
    /// waiter-correlated `CreateInstance` reply itself).
    fn live(&self) -> bool {
        matches!(self.phase, InstPhase::Live)
    }

    /// Whether `CreateInstance` completed — only minted instances
    /// receive `InstanceFailed` notifications on a child crash (a
    /// mid-create death reports `SpawnFailed` to the `Instantiate`
    /// caller instead; emitting `Failed` for a handle that never
    /// existed would be a lie).
    fn minted(&self) -> bool {
        !matches!(self.phase, InstPhase::Creating)
    }

    /// The single mirror-mutation point: updates `mirror` and
    /// publishes the same state to the lock-free `rt_state`
    /// snapshot the RT leg reads — `Release` pairs the `Acquire`
    /// load in `L1Instance::process`.
    fn set_mirror(&mut self, state: LifecycleState) {
        self.mirror = state;
        self.rt_state
            .store(data::mirror_encode(state), Ordering::Release);
    }

    /// Pushes one completion element onto this entry's result
    /// channel — a no-op on a video entry, which carries no result
    /// channel (every caller names an audio entry in-protocol: doorbell
    /// seqs only ever exist for `InstKind::Audio`, so the video arm
    /// is unreachable and deliberately silent).
    fn push_result(&self, el: u64) {
        if let InstKind::Audio {
            result_tx,
            counters,
            ..
        } = &self.kind
            && lock_mutex(result_tx).try_push(el).is_err()
        {
            // The enqueue failed — the completion element is lost.
            // Only reader-side `try_push` failures count: a seq
            // delivered and later never consumed is a different
            // window (`results_dropped - results_failed_observed`
            // is not a meaningful loss figure).
            counters.results_dropped.increment();
        }
    }

    /// The armed staging ring clone — `None` for a video entry
    /// (which carries no audio data-plane fields) or an audio entry
    /// before `Activate` arms it.
    fn armed_staging(&self) -> Option<Arc<data::Staging>> {
        match &self.kind {
            InstKind::Audio { staging, .. } => staging.clone(),
            InstKind::Video { .. } => None,
        }
    }

    /// The audio entry's segment descriptor — `None` for a video
    /// entry. Only the doorbell paths read it, and they only ever
    /// name audio entries in-protocol (`block_in_flight` gains seqs
    /// exclusively under `InstKind::Audio`).
    fn audio_shm(&self) -> Option<&SegmentInfo> {
        match &self.kind {
            InstKind::Audio { shm, .. } => Some(shm),
            InstKind::Video { .. } => None,
        }
    }

    /// The audio entry's leg-side counter writers — `None` on a
    /// video entry (which carries no counters by construction).
    fn audio_leg_counters(&self) -> Option<data::RtLegCounters> {
        match &self.kind {
            InstKind::Audio { counters, .. } => Some(counters.leg.clone()),
            InstKind::Video { .. } => None,
        }
    }

    /// The audio entry's counter readers — `None` on a video entry.
    fn audio_counter_readers(&self) -> Option<AudioCounterReaders> {
        match &self.kind {
            InstKind::Audio { counters, .. } => Some(counters.readers()),
            InstKind::Video { .. } => None,
        }
    }
}

/// Mutex-protected child-shared state — everything the caller,
/// driver and reader threads touch under one lock. Locking is
/// legal everywhere here — none of these threads is an RT thread
/// (each instance's RT leg in `l1::data` never takes this mutex;
/// it reads its `InstShared::rt_state` snapshot instead).
struct Inner {
    /// In-flight commands by correlation seq — the seq space is
    /// **child-wide** (the mint lives on `Shared.next_cmd_seq`);
    /// only the `Waiter.instance` stamp is per-instance.
    waiters: HashMap<u64, Waiter>,
    /// Registered instances by wire handle — `Failed`-until-
    /// `Destroy` and `Destroying` entries stay registered
    /// (registered-but-not-live for the demux table); removal is
    /// `DestroyInstance`-reply-granular. The map's keyset is the
    /// lowest-free mint oracle and its `len()` is the `live_count`
    /// shutdown-on-empty counts.
    instances: HashMap<u32, InstShared>,
    /// Doorbells in flight — `(instance, seq)` because block seqs
    /// are per-instance while the doorbell set is shared. Seqs are
    /// partitioned by [`data::BLOCK_SEQ_FLAG`], so they can never
    /// alias `waiters` keys.
    block_in_flight: HashSet<(u32, u64)>,
    /// `Some` once `converge_child` ran — exactly once.
    terminal: Option<Terminal>,
    /// `Shutdown` was written on the wire — a later child exit is
    /// clean teardown rather than `Failed`. Set only on the
    /// driver, immediately before the `Shutdown` write and only
    /// while no terminal condition has converged, so a child that
    /// crashed before the wire `Shutdown` still converges `Failed`
    /// and keeps its notification.
    shutdown_initiated: bool,
    /// The reader observed stdout EOF (one `Failed` path).
    seen_eof: bool,
}

/// Shared child state: the mutex-guarded [`Inner`], the shared
/// child handle and the child-wide command-seq mint.
struct Shared {
    inner: Mutex<Inner>,
    /// The spawned child, installed by the driver immediately after
    /// `spawn` and before `driver_main` runs. Shared so any
    /// converging thread can `kill()` a wedged child even while the
    /// driver is blocked in a stdin `write_all` (a live-but-
    /// not-reading child plus a frame over the pipe capacity) —
    /// `kill`/`try_wait`/`wait` are all `&mut`, hence the `Mutex`.
    ///
    /// **Lock order:** `inner` and `child` sections are always
    /// disjoint (see module docs) — no deadlock between the two.
    child: OnceLock<Mutex<Child>>,
    /// Child-wide command-seq mint — `Relaxed` fetch-add; the
    /// [`data::BLOCK_SEQ_FLAG`] space is never drawn from it, so
    /// command seqs can never alias doorbell seqs.
    next_cmd_seq: AtomicU64,
    /// The child's live diagnostics capture — created in
    /// [`L1Child::spawn`] before the driver starts so the stderr
    /// drain thread sees it from birth; the spawn/converge paths
    /// advance its phase and record the first-observed cause.
    diag: Arc<diag::ActiveDiagnostics>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the installed child handle (poison-tolerant like
    /// [`Shared::lock`]). `None` only before the driver installs
    /// the child — unreachable once `driver_main` runs.
    fn child_lock(&self) -> Option<MutexGuard<'_, Child>> {
        self.child.get().map(|child| {
            child
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        })
    }

    /// Pushes one element onto instance `i`'s result channel under
    /// the `inner` lock — channel capacity is `D` and at most `D`
    /// seqs are ever in flight per instance, each yielding exactly
    /// one completion element, so `try_push` cannot overflow
    /// in-protocol; a `Full` is tolerated like the reader's own
    /// pushes, and a missing entry means the leg died with its
    /// `Destroy` (the seq it owed is moot).
    fn push_result(&self, instance: u32, el: u64) {
        let inner = self.lock();
        if let Some(inst) = inner.instances.get(&instance) {
            inst.push_result(el);
        }
    }
}

/// Poison-tolerant `Mutex::lock` — same convention as
/// [`Shared::lock`].
fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// One child process and its three I/O threads — the shareable
/// unit N `L1Instance`s live on (the dedicated route is the
/// capacity-1 case, built through exactly the same machinery).
/// `L1Child` carries what the last referrer's bounded teardown
/// needs — the driver channel endpoint and both join handles —
/// plus the `Arc<Shared>` every facade clones; the process handle
/// itself stays inside `Shared` so a converging I/O thread can
/// `kill` it.
///
/// The I/O threads hold `Arc<Shared>` only — never `Arc<L1Child>`
/// — so `Arc::strong_count` on a facade's `child` counts exactly
/// the live referrers (facades, and a T3 pool slot later): `1`
/// inside a `Drop` means the last referrer is tearing down.
pub(crate) struct L1Child {
    shared: Arc<Shared>,
    /// The driver's command channel — `Option` so teardown can
    /// disconnect it (the driver's `Disconnected` arm is itself a
    /// bounded teardown path: kill + reap + converge).
    driver_tx: Mutex<Option<mpsc::Sender<DriverCmd>>>,
    driver_join: Mutex<Option<thread::JoinHandle<()>>>,
    /// Detached, never joined — a stdio-inheriting grandchild can
    /// hold the child's stdout pipe open past its death
    /// (`--leave-heir` pins ~15 s), so joining could stall the
    /// teardown bound.
    reader_join: Mutex<Option<thread::JoinHandle<()>>>,
    /// `command_timeout` — doubles as the shutdown-grace and
    /// join-bounds window.
    command_timeout: Duration,
}

/// `child.kill()` through the shared handle — uncatchable on both
/// OSes. A `Failed` convergence calls this itself rather than queueing
/// a kill order on the driver's channel: the driver may be blocked in
/// `write_all` on a pipe the wedged child never drains, and the kill
/// is what turns that write into EPIPE so the driver unwinds and reaps
/// the real exit status for a [`Notify::Pending`] completion.
fn kill_child(shared: &Shared) {
    if let Some(mut child) = shared.child_lock() {
        let _ = child.kill();
    }
}

/// Orders for the driver thread — it serializes outgoing frames,
/// installs/uninstalls per-instance data-plane ends and runs the
/// process lifecycle operations through this channel. `pub(crate)`
/// because [`L1Child::send_driver`] exposes it to the T3 pool.
pub(crate) enum DriverCmd {
    /// Serialize and write a `Command` frame — `instance` is the
    /// stamped wire-v2 envelope (`WIRE_INSTANCE_NONE` for `Scan`,
    /// the minted handle for everything else).
    Command {
        seq: u64,
        instance: u32,
        cmd: wire::Command,
    },
    /// Write `Shutdown`, then give the child `command_timeout` of
    /// grace before `kill()`.
    Shutdown,
    /// Install an instance's data-plane ends on the driver: the
    /// submit-channel consumer and the `shm_key` for this thread's
    /// own `shm::View` attach (`Shmem` is `!Send` — the mapping
    /// must happen on the driver's stack frame). A failed attach
    /// is a host-side segment loss — child-scope convergence.
    AddInstance {
        instance: u32,
        submit: ruviform_rt::Consumer<data::Submit>,
        shm_key: String,
    },
    /// Uninstall an instance's ends — the consumer and view drop
    /// with the entry. Sent at `DestroyInstance`-reply-granular
    /// removal (and rollback), so a wire handle reused by a later
    /// `AddInstance` never sees the previous generation's queued
    /// submits.
    RemoveInstance { instance: u32 },
}

/// Lowest-free wire-handle mint — `[0, WIRE_MAX_INSTANCES)` minus
/// the `instances` keyset; `None` at capacity (the caller folds it
/// to `HostError::InstanceLimit`). Runs inside the `inner` lock so
/// mint-and-insert is atomic, and reuse is reply-granular: a
/// reclaimed handle only frees once the old entry left the map, so
/// a stale `(i, ·)` frame from the previous generation demuxes
/// `Creating`/inert — never attributed to the new instance.
fn mint_instance(inner: &Inner) -> Option<u32> {
    (0..wire::WIRE_MAX_INSTANCES).find(|i| !inner.instances.contains_key(i))
}

/// `DestroyInstance`-reply-granular entry removal: residual
/// `(i, seq)` doorbells first get a synthesized `mark_failed`
/// completion (the leg's alternation gate must be freed — this is
/// the reuse-safety half of the doorbell window), a `Pending`
/// notification is flushed with the status-unknown [`ChildExit`]
/// (exactly-once must hold even when the reaped status never
/// arrives — the entry leaving the registry would otherwise drop
/// `notify_tx` unsent and the minted facade's
/// `try_recv_notification` would wait forever), then the entry
/// leaves `instances`; every later `(i, ·)` frame demuxes to the
/// inert post-removal row.
fn remove_instance(inner: &mut Inner, instance: u32) {
    let residuals: Vec<u64> = inner
        .block_in_flight
        .iter()
        .filter(|(i, _)| *i == instance)
        .map(|(_, seq)| *seq)
        .collect();
    for seq in residuals {
        inner.block_in_flight.remove(&(instance, seq));
        if let Some(inst) = inner.instances.get(&instance) {
            inst.push_result(data::mark_failed(seq));
        }
    }
    if let Some(inst) = inner.instances.get_mut(&instance)
        && inst.notify == Notify::Pending
    {
        // The status-unknown value — the same `ChildExit{None,
        // None}` `converge_instance` sends for a live child; the
        // cause is the instance's first-observed one (a `Pending`
        // entry always carries it; `Internal` is the honest
        // backstop for a shape that should be unreachable).
        inst.notify = Notify::Sent;
        let _ = inst.notify_tx.send(FailedNotice {
            exit: ChildExit {
                code: None,
                signal: None,
            },
            cause: inst.cause.unwrap_or(FailureCause::Internal),
        });
    }
    inner.instances.remove(&instance);
}

/// The child-scope convergence every child-terminal path funnels
/// through. `exit` is `Some` when the caller observed the final
/// `ExitStatus` (the driver's `try_wait`/`reap`) and `None` when it
/// did not (stdout EOF, protocol violation, write error, timeout).
///
/// First caller wins the transition; on `Failed` it fans one
/// `ChildExit` notification out to every *minted* instance —
/// immediately when the status is already known, or once the driver
/// reaps it ([`Notify::Pending`] completes per-instance), so each
/// notification always carries the real exit status regardless of
/// which crash path fired first. Either way pending waiters are
/// woken with `Err(HostError::Ipc)` — a command can never hang on a
/// dead context — and every in-flight doorbell is completed with a
/// synthesized `mark_failed` so no leg's alternation gate wedges.
///
/// A `Failed` transition also force-kills the child *here* —
/// through `Shared.child`, not via an order on the driver's
/// channel: a wedged child that stops draining stdin leaves the
/// driver blocked in `write_all` (frames can exceed the ~64 KiB
/// pipe capacity), and this direct kill is what turns that write
/// into EPIPE and lets the driver unwind. `kill` is uncatchable on
/// both OSes, so the driver's `try_wait`/`reap` then observes the
/// real exit status and `Pending` notifications complete with it.
/// `cause` is the caller's classification of its own observation —
/// recorded on the child's diagnostics capture and on each failed
/// instance's entry under the `inner` lock, where *first-observed
/// wins* and a later observation (including the status-supply call
/// below) never overwrites.
fn converge_child(shared: &Shared, exit: Option<ChildExit>, cause: FailureCause) {
    // The child-scope first-observed cause — the diagnostics
    // capture's own lock makes this idempotent across competing
    // observers.
    shared.diag.record_cause(cause);
    let failed = {
        let mut inner = shared.lock();
        if inner.terminal.is_some() {
            // Already converged — but a late exit status still
            // completes pending notifications, exactly once each.
            // The cause fixed at convergence rides along; the
            // status supply changes nothing else.
            if matches!(inner.terminal, Some(Terminal::Failed))
                && let Some(exit) = exit
            {
                for inst in inner.instances.values_mut() {
                    if inst.notify == Notify::Pending {
                        inst.notify = Notify::Sent;
                        let _ = inst.notify_tx.send(FailedNotice {
                            exit,
                            cause: inst.cause.unwrap_or(FailureCause::Internal),
                        });
                    }
                }
            }
            return;
        }
        let failed = !inner.shutdown_initiated;
        inner.terminal = if failed {
            Some(Terminal::Failed)
        } else {
            Some(Terminal::Clean)
        };
        for inst in inner.instances.values_mut() {
            // `minted()` BEFORE the phase flip — a `Creating`
            // entry never minted a public handle, so it converges
            // silently: its `Instantiate` caller already gets
            // `Err(Ipc)` via the drained waiter below, and
            // emitting `Failed` for a handle that never existed
            // would be a lie.
            let minted = inst.minted();
            inst.set_mirror(if failed {
                LifecycleState::Failed
            } else {
                LifecycleState::Destroyed
            });
            if failed {
                inst.phase = InstPhase::Failed;
                // First-observed cause for this instance's
                // failure — an earlier `converge_instance` cause
                // stands.
                if inst.cause.is_none() {
                    inst.cause = Some(cause);
                }
                // Already-`Sent` entries keep their exactly-once
                // guarantee.
                if minted && inst.notify == Notify::Idle {
                    inst.notify = match exit {
                        Some(exit) => {
                            let _ = inst.notify_tx.send(FailedNotice {
                                exit,
                                cause: inst.cause.unwrap_or(FailureCause::Internal),
                            });
                            Notify::Sent
                        }
                        None => Notify::Pending,
                    };
                }
            }
        }
        // Unbounded-channel sends never block — safe under the
        // lock, and keeping them inside the critical section
        // makes "exactly once" trivially atomic.
        for (_, waiter) in inner.waiters.drain() {
            let _ = waiter.tx.send(Err(HostError::Ipc));
        }
        // Free every in-flight doorbell with a synthesized
        // `mark_failed` so live legs drain their alternation
        // gates — one seq, exactly one completion.
        for (instance, seq) in std::mem::take(&mut inner.block_in_flight) {
            if let Some(inst) = inner.instances.get(&instance) {
                inst.push_result(data::mark_failed(seq));
            }
        }
        failed
    };
    if failed {
        // Disjoint critical sections only — `inner` is already
        // released before `child` is taken (module lock order).
        kill_child(shared);
    }
}

/// The instance-scope convergence — the attributable-violation
/// half of the demux table. Fails `instance` alone: mirror
/// `Failed`, its pending waiters `Err(Ipc)`, its in-flight
/// doorbells `mark_failed`, and exactly one
/// `ChildExit{code:None, signal:None}` notification (the child is
/// alive — there is no exit status to report). The entry stays
/// registered until `DestroyInstance` reclaims the runner registry
/// slot.
///
/// **Invariant:** child state (`Inner.terminal`, `seen_eof`,
/// `shutdown_initiated`) and every sibling's entry are untouched —
/// that is what keeps `Failed` granular on a shared child.
fn converge_instance(shared: &Shared, instance: u32, cause: FailureCause) {
    let mut inner = shared.lock();
    {
        let Some(inst) = inner.instances.get_mut(&instance) else {
            return;
        };
        if !inst.live() {
            // Already converged/`Destroying`/`Creating` —
            // idempotent; the demux only routes `live` violations
            // here anyway.
            return;
        }
        inst.phase = InstPhase::Failed;
        inst.set_mirror(LifecycleState::Failed);
        // This instance's first-observed cause — the child stays
        // alive, so nothing is recorded on the child-scope
        // capture: a pooled sibling's later real crash must keep
        // its own cause.
        if inst.cause.is_none() {
            inst.cause = Some(cause);
        }
        if inst.notify == Notify::Idle {
            inst.notify = Notify::Sent;
            let _ = inst.notify_tx.send(FailedNotice {
                exit: ChildExit {
                    code: None,
                    signal: None,
                },
                cause: inst.cause.unwrap_or(FailureCause::Internal),
            });
        }
    }
    // This instance's pending commands — the per-instance drain
    // that makes `Failed` granular (siblings' waiters stay live).
    let seqs: Vec<u64> = inner
        .waiters
        .iter()
        .filter(|(_, waiter)| waiter.instance == instance)
        .map(|(&seq, _)| seq)
        .collect();
    for seq in seqs {
        if let Some(waiter) = inner.waiters.remove(&seq) {
            let _ = waiter.tx.send(Err(HostError::Ipc));
        }
    }
    // This instance's doorbells — the same `mark_failed` release
    // a child-scope convergence performs, scoped to `instance`.
    let seqs: Vec<u64> = inner
        .block_in_flight
        .iter()
        .filter(|(i, _)| *i == instance)
        .map(|(_, seq)| *seq)
        .collect();
    for seq in seqs {
        inner.block_in_flight.remove(&(instance, seq));
        if let Some(inst) = inner.instances.get(&instance) {
            inst.push_result(data::mark_failed(seq));
        }
    }
}

/// What the reader does with one inbound frame — the Issue #117
/// demux decision table (design L304–320) as data. [`demux`]
/// decides under the `inner` lock; [`demux_execute`] performs the
/// verdict.
enum Demux {
    /// Waiter-correlated `Reply` — deliver `result` to `tx` (an
    /// `Err(HostError)` payload is the defined per-command
    /// rejection, never a convergence).
    Deliver {
        tx: mpsc::Sender<Result<wire::Reply, HostError>>,
        result: Result<wire::Reply, HostError>,
    },
    /// Doorbell `(instance, seq)` completion — `Some` is a real
    /// `BlockDone` (out-slot read + `seq` push), `None` a flagged
    /// `Reply` rejection (`mark_failed` push).
    Complete {
        instance: u32,
        seq: u64,
        result: Option<ProcessResult>,
    },
    /// Registered-but-not-live (`Creating`/`Destroying`/`Failed`)
    /// or unregistered-instance frame — silently dropped.
    Inert,
    /// Post-`Shutdown` (or post-terminal) frame — clean teardown
    /// in flight or done; no-op.
    NonEvent,
    /// Instance-attributable violation — `converge_instance(i)`.
    Instance(u32),
    /// Unattributable / child-scope violation —
    /// `converge_child`.
    Child,
}

/// The classification core of the demux table — mutates only the
/// bookkeeping the frame answers (removes the correlated waiter /
/// doorbell entry) and returns what the reader should do. All
/// side-effect execution lives in [`demux_execute`], which runs
/// the verdict after releasing `inner`.
fn demux(inner: &mut Inner, msg: &wire::Message) -> Demux {
    // Post-`Shutdown` and post-terminal frames are non-events:
    // teardown is already decided, nothing left to attribute.
    if inner.terminal.is_some() || inner.shutdown_initiated {
        return Demux::NonEvent;
    }
    match msg {
        wire::Message::Reply {
            seq,
            instance,
            result,
        } => {
            if data::is_block_seq(*seq) {
                // A block rejection — completes a doorbell like a
                // `BlockDone`, just with a `mark_failed` element.
                demux_block(inner, *instance, *seq, None)
            } else {
                // Correlation first: a waiter at `seq` resolves
                // the reply regardless of the registered state of
                // `instance` — the `Scan` reply legitimately echoes
                // `WIRE_INSTANCE_NONE`, and a `Creating` entry's
                // `CreateInstance` answer is exactly this row.
                match inner.waiters.remove(seq) {
                    Some(waiter) => {
                        if waiter.instance == *instance {
                            Demux::Deliver {
                                tx: waiter.tx,
                                result: result.clone(),
                            }
                        } else {
                            // Echo mismatch — the correlation space
                            // itself is broken: unattributable,
                            // child-scope. The doomed caller still
                            // wakes now rather than at timeout.
                            let _ = waiter.tx.send(Err(HostError::Ipc));
                            Demux::Child
                        }
                    }
                    None => {
                        if *instance == wire::WIRE_INSTANCE_NONE {
                            // Child-scope replies only exist as a
                            // `Scan` answer — an uncorrelated `NONE`
                            // can attribute to nothing.
                            Demux::Child
                        } else if *instance >= wire::WIRE_MAX_INSTANCES {
                            // Out-of-range stamp — unattributable.
                            Demux::Child
                        } else {
                            match inner.instances.get(instance) {
                                // A waiter-less `Reply` on a *live*
                                // instance is a child-scope
                                // violation — nothing pending can
                                // explain it.
                                Some(inst) if inst.live() => Demux::Child,
                                _ => Demux::Inert,
                            }
                        }
                    }
                }
            }
        }
        wire::Message::BlockDone {
            seq,
            instance,
            result,
        } => demux_block(inner, *instance, *seq, Some(*result)),
        // `Hello` twice, or a parent→child kind (`Command`,
        // `BlockIn`, `Shutdown`) arriving child→parent:
        // child-scope violation.
        _ => Demux::Child,
    }
}

/// The `BlockDone`/flagged-`Reply` half of the table — doorbell
/// correlation on `(instance, seq)` first, then the
/// instance-resolution rows.
fn demux_block(inner: &mut Inner, instance: u32, seq: u64, result: Option<ProcessResult>) -> Demux {
    // The stamp must be a live-range handle — `NONE` or
    // out-of-range can never attribute a doorbell.
    if instance == wire::WIRE_INSTANCE_NONE || instance >= wire::WIRE_MAX_INSTANCES {
        return Demux::Child;
    }
    if inner.block_in_flight.remove(&(instance, seq)) {
        // A real doorbell completing — normally even while the
        // instance is `Destroying` (the doorbell window).
        return Demux::Complete {
            instance,
            seq,
            result,
        };
    }
    // `(i, seq)` was never rung — flag-bit status included.
    match inner.instances.get(&instance) {
        Some(inst) if inst.live() => Demux::Instance(instance),
        _ => Demux::Inert,
    }
}

/// What [`demux_execute`] resolved — the reader's flow-control
/// signal and the unit-test pin row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    /// A `Reply` was delivered to its waiter.
    Delivered,
    /// A doorbell was completed (real read or `mark_failed`).
    Completed,
    /// Registered-but-not-live / unregistered frame — dropped.
    Inert,
    /// Post-`Shutdown`/terminal frame — ignored.
    NonEvent,
    /// `converge_instance(i)` ran — the reader keeps going.
    Instance(u32),
    /// `converge_child` ran — the reader exits.
    Child,
}

/// Runs a `Demux::Complete`: lazily attaches the reader's `View`
/// for the instance's segment (first doorbell — and every later
/// doorbell whose registry `SegmentInfo` no longer matches, i.e.
/// a wire handle re-minted under a fresh segment — an attach
/// failure is a host-side segment loss, a child-scope
/// convergence), performs the out-slot read into the armed
/// staging, and pushes the result element.
fn complete_doorbell(
    shared: &Shared,
    views: &mut HashMap<u32, ReaderView>,
    instance: u32,
    seq: u64,
    result: Option<ProcessResult>,
) -> Verdict {
    let Some(result) = result else {
        // Flagged-`Reply` rejection — `mark_failed`, no shm work.
        shared.push_result(instance, data::mark_failed(seq));
        return Verdict::Completed;
    };
    // One `inner` acquisition serves both lookups — the entry's
    // current `SegmentInfo` (the stale-`View` oracle) and the armed
    // staging. `views` is this thread's lazy-attach map and nothing
    // ever prunes it (`RemoveInstance` reaches only the driver's
    // copies — the reader learns of removal through the inert
    // demux row), so a wire handle re-minted under a fresh segment
    // would otherwise keep the previous generation's `View`: old
    // `os_id`, old depths — a `BlockDone{i, seq}` would read the
    // stale mapping and the `slot_seq` tag check can pass on stale
    // data (silent wrong output, not pass-through). Comparing the
    // `SegmentInfo` per completion keeps the compare cheap and the
    // `View::attach` off the hot path — only a first doorbell or a
    // segment-identity mismatch pays it.
    let (attach, staging) = {
        let inner = shared.lock();
        let Some(inst) = inner.instances.get(&instance) else {
            // The entry raced removal — its `result_tx` died with
            // it; nothing left to complete.
            return Verdict::Completed;
        };
        let Some(shm) = inst.audio_shm() else {
            // Unreachable in-protocol — `block_in_flight` only ever
            // gains seqs for `InstKind::Audio` entries (the submits
            // map is `AddInstance`-installed, audio-only), so a
            // video entry never has a doorbell to complete and has
            // no result channel to push one onto.
            return Verdict::Completed;
        };
        let stale = views.get(&instance).is_none_or(|rview| rview.shm != *shm);
        (stale.then(|| shm.clone()), inst.armed_staging())
    };
    if let Some(shm) = attach {
        // `View::attach` (the mapping syscall) stays outside the
        // `inner` lock; a failure is a host-side segment loss —
        // child-scope convergence.
        let Ok(view) = shm::View::attach(&shm.key) else {
            converge_child(shared, None, FailureCause::Internal);
            return Verdict::Child;
        };
        views.insert(instance, ReaderView { view, shm });
    }
    let rview = views.get(&instance).expect("view installed above");
    // Acquire edge — pairs the child's `fence(Release)` before
    // `BlockDone`; nothing below may be reordered above it. A real
    // atomic fence (not a compiler fence): the cross-process
    // ordering rests on this hardware fence plus the pipe-read
    // syscall boundary.
    fence(Ordering::Acquire);
    if let Some(staging) = staging {
        let slot = usize::try_from(seq % u64::from(rview.shm.out_depth))
            .expect("slot index always fits usize");
        let _published = data::read_out_slot(
            rview
                .view
                .out_slot(rview.shm.in_depth, rview.shm.out_depth, seq),
            seq,
            &staging.outs[slot],
            staging.frame_capacity,
            result,
        );
        // `_published == false` means the child wrote a mismatched
        // slot seq — the data is withheld and the RT leg's tag
        // check fails closed; the done still completes the
        // alternation either way.
    }
    shared.push_result(instance, seq);
    Verdict::Completed
}

/// The reader's per-instance shm window — `View` plus the
/// [`SegmentInfo`] it was attached from. The `shm` identity is
/// re-checked against the registry entry on every doorbell: this
/// map is never pruned, so a reused wire handle must re-attach
/// rather than read the previous generation's segment.
struct ReaderView {
    view: shm::View,
    shm: SegmentInfo,
}

/// Executes one inbound frame end-to-end: [`demux`] classifies it
/// under the lock, then the verdict's side effects run — waiter
/// delivery, doorbell completion (shm work), or the converges.
fn demux_execute(
    shared: &Shared,
    msg: &wire::Message,
    views: &mut HashMap<u32, ReaderView>,
) -> Verdict {
    let action = {
        let mut inner = shared.lock();
        demux(&mut inner, msg)
    };
    match action {
        Demux::Deliver { tx, result } => {
            let _ = tx.send(result);
            Verdict::Delivered
        }
        Demux::Complete {
            instance,
            seq,
            result,
        } => complete_doorbell(shared, views, instance, seq, result),
        Demux::Inert => Verdict::Inert,
        Demux::NonEvent => Verdict::NonEvent,
        Demux::Instance(instance) => {
            // Instance-attributable violation — `Protocol`.
            converge_instance(shared, instance, FailureCause::Protocol);
            Verdict::Instance(instance)
        }
        Demux::Child => {
            // Unattributable / child-scope violation — `Protocol`.
            converge_child(shared, None, FailureCause::Protocol);
            Verdict::Child
        }
    }
}

/// Spawns the configured child and takes its piped stdio — run on the
/// driver thread so `Child` ownership never crosses a thread boundary.
/// A failure after a successful `spawn` (theoretically unreachable
/// with `Stdio::piped`) kills + reaps before reporting, so no child
/// ever leaks.
fn spawn_child(
    command: &mut ProcCommand,
) -> Result<(Child, ChildStdin, ChildStdout, ChildStderr), ()> {
    let mut child = command.spawn().map_err(|_| ())?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(());
    };
    Ok((child, stdin, stdout, stderr))
}

/// The reader thread: first frame must be `Hello`, afterwards every
/// frame goes through [`demux_execute`] — the Issue #117 decision
/// table — which delivers waiter-correlated `Reply`s, completes
/// doorbells (`BlockDone` out-slot reads and flagged-`Reply`
/// `mark_failed` rejections alike), drops inert frames, fails single
/// instances on attributable violations (`converge_instance`) and
/// ends the loop on child-scope ones (`converge_child`). An
/// unreadable stream or EOF is the `seen_eof` `Failed` path.
///
/// `views` maps each live instance to this thread's own shm mapping
/// (the `Shmem` handle is `!Send`, so the `View` is attached lazily
/// here — on the first `BlockDone` for that instance — from the
/// entry's `SegmentInfo`, and re-attached whenever the entry's
/// `SegmentInfo` changes underneath it: the map is insert-only, so
/// a wire handle re-minted under a fresh segment must not keep
/// reading the previous generation's mapping). Result pushes go
/// through each instance's `InstShared.result_tx` under the `inner`
/// lock — the driver completes dropped seqs on the same
/// per-instance channel.
fn reader_main(mut stdout: ChildStdout, shared: &Shared, hello_tx: &mpsc::Sender<Result<u16, ()>>) {
    // One scratch allocation reused across frames — the wire layer
    // owns the framing; EOF and protocol violations share the
    // `Err`/`Failed` path here.
    let mut frame_buf = Vec::new();
    let mut views: HashMap<u32, ReaderView> = HashMap::new();
    match wire::read_message(&mut stdout, &mut frame_buf) {
        Ok(wire::Message::Hello { version, .. }) => {
            let _ = hello_tx.send(Ok(version));
        }
        Ok(_) => {
            // A well-formed non-`Hello` first frame — a handshake
            // failure, not a transport one.
            let _ = hello_tx.send(Err(()));
            converge_child(shared, None, FailureCause::Handshake);
            return;
        }
        Err(ref error) => {
            // Missing `Hello` or a dead pipe before it: `InvalidData`
            // is an undecodable/over-limit frame (`Protocol`);
            // `UnexpectedEof` and the reader's own errors are
            // pipe-level (`Transport`) — phase does not matter.
            let _ = hello_tx.send(Err(()));
            let cause = if error.kind() == std::io::ErrorKind::InvalidData {
                FailureCause::Protocol
            } else {
                FailureCause::Transport
            };
            converge_child(shared, None, cause);
            return;
        }
    }
    loop {
        let msg = match wire::read_message(&mut stdout, &mut frame_buf) {
            Ok(msg) => msg,
            Err(error) => {
                {
                    let mut inner = shared.lock();
                    inner.seen_eof = true;
                }
                // Same boundary as the handshake arm: undecodable
                // frames are `Protocol`, EOF/reader errors are
                // `Transport`.
                let cause = if error.kind() == std::io::ErrorKind::InvalidData {
                    FailureCause::Protocol
                } else {
                    FailureCause::Transport
                };
                converge_child(shared, None, cause);
                return;
            }
        };
        if demux_execute(shared, &msg, &mut views) == Verdict::Child {
            return;
        }
    }
}

/// One `try_wait` on the shared child handle — the whole critical
/// section. `Err(())` folds a missing handle (unreachable inside
/// `driver_main`) into the "state unknown" arm.
fn try_wait_child(shared: &Shared) -> Result<Option<ExitStatus>, ()> {
    let Some(mut child) = shared.child_lock() else {
        return Err(());
    };
    child.try_wait().map_err(|_| ())
}

/// Reaps the child: polls `try_wait` until it yields a status; once
/// `deadline` passes the child is killed and `wait()` collects it
/// (bounded — `kill` is forceful on every supported OS). `None`
/// deadline waits indefinitely for an already-dead-or-dying child;
/// callers only reach that after `kill()` or `Shutdown`.
///
/// The `child` mutex is re-acquired per poll — it is never held
/// across the sleep — so a converging `kill` is never queued behind
/// this loop.
fn reap(shared: &Shared, deadline: Option<Instant>) -> ChildExit {
    loop {
        match try_wait_child(shared) {
            Ok(Some(status)) => return ChildExit::from_status(status),
            Ok(None) => {
                if let Some(deadline) = deadline
                    && Instant::now() >= deadline
                {
                    // The watchdog's forced kill — `Timeout` is
                    // recorded BEFORE the kill issues, so the EOF /
                    // exit observation that follows can never
                    // overwrite it (record is first-wins).
                    shared.diag.record_cause(FailureCause::Timeout);
                    kill_child(shared);
                    return match shared.child_lock().map(|mut child| child.wait()) {
                        Some(Ok(status)) => ChildExit::from_status(status),
                        Some(Err(error)) => {
                            // The `wait` syscall itself failed —
                            // `Internal`, even as first observation.
                            shared.diag.record_cause(FailureCause::Internal);
                            shared.diag.record_cleanup_error(error);
                            ChildExit::default()
                        }
                        None => ChildExit::default(),
                    };
                }
            }
            Err(()) => {
                // `try_wait` itself failed — `Internal`, even as
                // first observation.
                shared.diag.record_cause(FailureCause::Internal);
                return ChildExit::default();
            }
        }
        thread::sleep(DRIVER_POLL_INTERVAL);
    }
}

/// A driver exit on broken stdin or a vanished host segment: converge
/// `Failed` — which force-kills the wedged child itself — then reap
/// so every `Notify::Pending` completes with the real exit status.
/// The deadline is already reached when `reap` runs, so this is one
/// `kill` (idempotent) plus a `wait` on an already-doomed child —
/// bounded by the kill, never by the peer.
fn converge_and_reap(shared: &Shared, cause: FailureCause) {
    converge_child(shared, None, cause);
    let exit = reap(shared, Some(Instant::now()));
    converge_child(shared, Some(exit), FailureCause::ExitObserved);
}

/// Freezes the live capture, registers the attempt record and
/// returns the `SpawnFailed` carrying its `diagnostics_id` — the
/// shared tail of every pre-mint failure path; the record's `Arc`
/// moves to the registry, so it outlives the `Shared` that drops
/// with the failed attempt. The recorded first-observed cause is
/// the payload's cause; nothing recorded yet folds to `Internal`.
fn spawn_failed(shared: &Shared, diag: &diag::DiagRegistry) -> HostError {
    let pending = shared.diag.freeze(None);
    let cause = pending.cause.unwrap_or(FailureCause::Internal);
    let record = diag.register(pending);
    HostError::SpawnFailed {
        cause,
        diagnostics_id: record.diagnostics_id,
    }
}

/// Maps a failed `CreateInstance`/`CreateVideoInstance` outcome to
/// the rollback error — `InstanceLimit` propagates unflattened and
/// registers no record (a defined capacity rejection, not a
/// failure); every other outcome registers an *attempt* record — a
/// frozen copy of the capture at this instant, separate from a live
/// pooled child's continuing record — and returns `SpawnFailed`.
///
/// Cause classification: a well-formed reply of the wrong kind is
/// `Protocol` (also recorded on the live capture, first-wins);
/// `Ok(Ok)` reaching rollback is the mid-mint fold — the entry
/// converged while `Creating`, so the capture's recorded cause is
/// the truth. An `Err` carrying `Ipc`/`Failed` is ambiguous once
/// the waiter flattens it: a *request-level* failure (timeout,
/// terminal/shutdown gate, dead driver channel) has its cause
/// recorded at observation, while a live child's `Reply::Err`
/// defined rejection carries no record — so the recorded cause
/// wins and `Handshake` is the fallback. Every other `Err` variant
/// is the child's own `Reply::Err` defined rejection — `Handshake`
/// per the cause table.
fn create_rollback_error(
    shared: &Shared,
    diag: &diag::DiagRegistry,
    create: &Result<wire::Reply, HostError>,
) -> HostError {
    if matches!(create, Err(HostError::InstanceLimit)) {
        return HostError::InstanceLimit;
    }
    let mut pending = shared.diag.freeze(None);
    let cause = match create {
        Ok(wire::Reply::Ok) => pending.cause.unwrap_or(FailureCause::Internal),
        Err(HostError::Ipc | HostError::Failed) => pending.cause.unwrap_or(FailureCause::Handshake),
        Err(_) => FailureCause::Handshake,
        Ok(_) => {
            shared.diag.record_cause(FailureCause::Protocol);
            FailureCause::Protocol
        }
    };
    pending.cause = Some(cause);
    let record = diag.register(pending);
    HostError::SpawnFailed {
        cause,
        diagnostics_id: record.diagnostics_id,
    }
}

/// The driver thread: operates the shared `Child` handle and owns
/// the child's stdin. Drains the command channel on a
/// `DRIVER_POLL_INTERVAL` cadence, watches `try_wait` for the crash
/// path, runs the shutdown watchdog and drains every instance's
/// data-plane `submit` channel — each popped seq becomes an in-slot
/// shm write plus a `BlockIn{instance, seq}` doorbell (this thread
/// does all the blocking pipe work; the RT caller only `try_push`es
/// seqs).
///
/// `submits`/`views` are keyed by instance — `AddInstance`/
/// `RemoveInstance` install and uninstall them as the child mints
/// and reclaims registry entries.
fn driver_main(
    shared: &Shared,
    mut stdin: ChildStdin,
    rx: &mpsc::Receiver<DriverCmd>,
    shutdown_grace: Duration,
) {
    let mut shutdown_deadline: Option<Instant> = None;
    // One encode scratch reused for every outbound frame — commands,
    // the `Shutdown`, and each `BlockIn` doorbell.
    let mut encode_buf = Vec::new();
    // Per-instance data-plane ends — installed/uninstalled over the
    // child's life by `AddInstance`/`RemoveInstance` orders.
    let mut submits: HashMap<u32, ruviform_rt::Consumer<data::Submit>> = HashMap::new();
    let mut views: HashMap<u32, shm::View> = HashMap::new();
    loop {
        match rx.recv_timeout(DRIVER_POLL_INTERVAL) {
            Ok(DriverCmd::AddInstance {
                instance,
                submit,
                shm_key,
            }) => {
                // `Shmem` is `!Send` — the view must attach on this
                // thread's stack frame. A failure means the host's
                // own segment vanished — a child-scope
                // environmental failure.
                let Ok(view) = shm::View::attach(&shm_key) else {
                    // Host-side segment loss — `Internal`.
                    converge_and_reap(shared, FailureCause::Internal);
                    return;
                };
                submits.insert(instance, submit);
                views.insert(instance, view);
            }
            Ok(DriverCmd::RemoveInstance { instance }) => {
                submits.remove(&instance);
                views.remove(&instance);
            }
            Ok(DriverCmd::Command { seq, instance, cmd }) => {
                if !driver_command(
                    shared,
                    &mut stdin,
                    seq,
                    instance,
                    cmd,
                    &mut encode_buf,
                    shutdown_deadline.is_some(),
                ) {
                    return;
                }
            }
            Ok(DriverCmd::Shutdown) => {
                {
                    // The authoritative `shutdown_initiated` set —
                    // immediately before the wire `Shutdown` and only
                    // while the child still looks live (no converged
                    // terminal, no observed EOF, `try_wait` clean). An
                    // exit before the wire `Shutdown` is a `Failed`
                    // crash and keeps its notification; only an exit
                    // after it is clean teardown.
                    let dead = if let Ok(status) = try_wait_child(shared) {
                        status.is_some()
                    } else {
                        // The syscall failed — `Internal`, even
                        // as first observation; the child is
                        // treated as not-proven-dead.
                        shared.diag.record_cause(FailureCause::Internal);
                        false
                    };
                    let mut inner = shared.lock();
                    if !dead && inner.terminal.is_none() && !inner.seen_eof {
                        inner.shutdown_initiated = true;
                    }
                }
                // The peer may already be half-dead — a write failure
                // only means the watchdog will finish the job.
                let _ = wire::write_message(&mut stdin, &wire::Message::Shutdown, &mut encode_buf);
                shutdown_deadline = Some(Instant::now() + shutdown_grace);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // The owner dropped without `Destroy`: if a `Shutdown`
                // is in flight its grace still applies, otherwise kill
                // is the only bounded teardown.
                if shutdown_deadline.is_none() {
                    kill_child(shared);
                }
                let exit = reap(shared, shutdown_deadline);
                converge_child(shared, Some(exit), FailureCause::ExitObserved);
                return;
            }
        }
        // Data plane: every seq any instance's RT caller submitted
        // becomes an in-slot write plus a `BlockIn{instance, seq}`
        // doorbell — bounded by the channel capacity, never
        // blocking on the RT side.
        if !drain_submits(
            shared,
            &mut stdin,
            &mut submits,
            &mut views,
            &mut encode_buf,
        ) {
            return;
        }
        match try_wait_child(shared) {
            Ok(Some(status)) => {
                converge_child(
                    shared,
                    Some(ChildExit::from_status(status)),
                    FailureCause::ExitObserved,
                );
                return;
            }
            Ok(None) => {}
            Err(()) => {
                // `try_wait` itself failed — `Internal`, even as
                // first observation: the child's state is unknown.
                // Force the death so POSIX never leaves a zombie,
                // then collect the status best-effort (a second
                // failure converges with the default `ChildExit`,
                // still completing `Notify::Pending`).
                kill_child(shared);
                let exit = reap(shared, Some(Instant::now()));
                converge_child(shared, Some(exit), FailureCause::Internal);
                return;
            }
        }
        if shutdown_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            let exit = reap(shared, Some(Instant::now()));
            converge_child(shared, Some(exit), FailureCause::ExitObserved);
            return;
        }
    }
}

/// One `DriverCmd::Command` — the gate, the wire write and the
/// `Destroying` doorbell-window transition. Returns `false` when
/// convergence ended the driver (a stdin write failure).
///
/// A converged child (or teardown in flight) takes no new wire
/// messages — the spec forbids writes after `Failed`; the same holds
/// per-instance: a converged or `Destroying` entry's queued commands
/// never reach the wire either. `DestroyInstance` is exempt —
/// reclaiming a `Failed`/`Destroying` entry's runner slot is its
/// job. Rejected commands wake their waiter `Err(Ipc)` instead of
/// writing a doomed frame.
fn driver_command(
    shared: &Shared,
    stdin: &mut impl Write,
    seq: u64,
    instance: u32,
    cmd: wire::Command,
    encode_buf: &mut Vec<u8>,
    shutting_down: bool,
) -> bool {
    let destroy = matches!(cmd, wire::Command::DestroyInstance);
    let reject = shutting_down || {
        let inner = shared.lock();
        inner.terminal.is_some()
            || (!destroy
                && matches!(
                    inner.instances.get(&instance),
                    Some(inst) if matches!(inst.phase, InstPhase::Failed | InstPhase::Destroying)
                ))
    };
    if reject {
        let waiter = shared.lock().waiters.remove(&seq);
        if let Some(waiter) = waiter {
            let _ = waiter.tx.send(Err(HostError::Ipc));
        }
        return true;
    }
    if wire::write_message(
        stdin,
        &wire::Message::Command { seq, instance, cmd },
        encode_buf,
    )
    .is_err()
    {
        // Pipe-level write observation — `Transport`.
        converge_and_reap(shared, FailureCause::Transport);
        return false;
    }
    if destroy {
        // The doorbell window opens the instant `DestroyInstance`
        // hits the wire — NOT at the reply: from here the demux
        // table treats `i` as registered-but-not-live (inert late
        // frames) and `drain_submits` synthesizes `mark_failed` for
        // new submits instead of ringing new doorbells. A `Failed`
        // entry stays `Failed` — the destroy reclaims a dead slot.
        let mut inner = shared.lock();
        if let Some(inst) = inner.instances.get_mut(&instance)
            && inst.phase == InstPhase::Live
        {
            inst.phase = InstPhase::Destroying;
        }
    }
    true
}

/// One driver poll's worth of `submit` drains across every
/// registered instance: each popped seq becomes an in-slot shm
/// write plus a `BlockIn{instance, seq}` doorbell. A seq dropped
/// instead — stale generation, `Destroying`/`Failed`/`Creating`
/// entry, dead or shutting-down child — still owes its leg exactly
/// one completion: the synthesized `mark_failed` below. Returns
/// `false` when a doorbell write failed and convergence ran — the
/// driver exits with it.
fn drain_submits(
    shared: &Shared,
    stdin: &mut impl Write,
    submits: &mut HashMap<u32, ruviform_rt::Consumer<data::Submit>>,
    views: &mut HashMap<u32, shm::View>,
    encode_buf: &mut Vec<u8>,
) -> bool {
    for (&instance, submit) in submits.iter_mut() {
        while let Ok(submit) = submit.try_pop() {
            let seq = submit.seq;
            let staged = {
                let mut inner = shared.lock();
                let live_child = inner.terminal.is_none() && !inner.shutdown_initiated;
                match inner.instances.get_mut(&instance) {
                    Some(inst) if live_child && inst.phase == InstPhase::Live => {
                        // `armed_staging` is `None` on a video
                        // entry — unreachable in-protocol (the
                        // `submits` map is `AddInstance`-installed,
                        // audio-only), and harmless: the drop arm
                        // below then pushes a `mark_failed` that is
                        // a no-op on `InstKind::Video` (no result
                        // channel exists).
                        let staging = data::armed_for(inst.armed_staging().as_ref(), submit);
                        if let Some(staging) = staging {
                            let in_depth = inst
                                .audio_shm()
                                .expect("an armed staging implies InstKind::Audio")
                                .in_depth;
                            // The doorbell goes in flight BEFORE
                            // the write — the reader's `BlockDone` /
                            // flagged-`Reply` must always find it
                            // here.
                            inner.block_in_flight.insert((instance, seq));
                            Some((staging, in_depth))
                        } else {
                            // `None` is either "no ring armed yet"
                            // or a stale-generation submit — queued
                            // before a re-activation swapped
                            // `inst.staging`, so serializing it
                            // would doorbell the new ring's zeroed
                            // stage under a real seq. Dropped
                            // without a doorbell: `block_in_flight`
                            // never sees it — the owed
                            // `mark_failed` completion goes out
                            // here instead.
                            inst.push_result(data::mark_failed(seq));
                            None
                        }
                    }
                    // `Destroying`/`Failed`/`Creating`, or a dead /
                    // shutting-down child — the `Destroying` gate:
                    // no doorbell, but the seq still owes the leg
                    // its completion.
                    Some(inst) => {
                        inst.push_result(data::mark_failed(seq));
                        None
                    }
                    // The entry raced removal — the `result_tx`
                    // producer dropped with the `InstShared`, so
                    // no completion can be delivered for the
                    // popped seq (and it never entered
                    // `block_in_flight`, so `remove_instance` did
                    // not owe it one either). None is needed: the
                    // leg's `process` is already gated out of the
                    // block path by the terminal mirror before the
                    // missing completion could wedge a slot.
                    None => None,
                }
            };
            let Some((staging, in_depth)) = staged else {
                // A seq dropped above was never doorbell'd — no shm
                // write happened, so no out-slot race exists — and
                // (except the missing-entry arm, whose completion
                // channel died with the entry) it was already
                // answered inside the lock: a leg still waits on a
                // completion to clear the slot's alternation gate
                // (`cleared[i] >= last_pushed[i]`), which left
                // unanswered would wedge the slot — then the whole
                // ring — forever: a fully-synced leg passing
                // through 100%. The `mark_failed` push synthesizes
                // exactly one completion so the normal `drain_to`
                // path frees the slot and resolves the block as
                // the defined pass-through.
                continue;
            };
            let Some(view) = views.get_mut(&instance) else {
                // `AddInstance` installs consumer and view
                // atomically — a miss means the maps diverged. The
                // seq already went in-flight inside the lock above:
                // retract it and push the synthesized `mark_failed`
                // like every other doorbell-less drop, rather than
                // leaving a phantom doorbell that wedges the leg's
                // alternation gate until a convergence frees it
                // (and that a later real `BlockDone` could
                // double-complete).
                let mut inner = shared.lock();
                inner.block_in_flight.remove(&(instance, seq));
                if let Some(inst) = inner.instances.get(&instance) {
                    inst.push_result(data::mark_failed(seq));
                }
                continue;
            };
            let slot = usize::try_from(seq % u64::from(in_depth)).expect("slot index fits usize");
            data::write_in_slot(
                view.in_slot_mut(in_depth, seq),
                seq,
                &staging.ins[slot],
                staging.frame_capacity,
            );
            // Release edge — the slot writes above must be visible
            // to the child before the `BlockIn` it acquires on. A
            // real atomic fence (not a compiler fence): ordering
            // across the process boundary rests on this hardware
            // fence plus the pipe-write syscall.
            fence(Ordering::Release);
            if wire::write_message(stdin, &wire::Message::BlockIn { seq, instance }, encode_buf)
                .is_err()
            {
                // Pipe-level write observation — `Transport`.
                converge_and_reap(shared, FailureCause::Transport);
                return false;
            }
        }
    }
    true
}

/// The driver's spawn hand-off payload: the child's piped stdout
/// (for the reader thread) and stderr (for the drain thread) cross
/// to the caller together — the drain must start before the `Hello`
/// wait so pre-handshake output is captured.
struct SpawnedIo {
    stdout: ChildStdout,
    stderr: ChildStderr,
}

/// The driver's spawn hand-off channel: `Ok` carries [`SpawnedIo`];
/// `Err(())` reports a spawn-time failure.
type SpawnRx = mpsc::Receiver<Result<SpawnedIo, ()>>;

/// The reader's handshake channel: `Ok(version)` carries the child's
/// `Hello` version; `Err(())` reports a broken handshake/data plane.
type HelloRx = mpsc::Receiver<Result<u16, ()>>;

/// Spawns the `l1-driver` thread — the child process itself is spawned
/// *inside* the thread and handed to [`L1Child::spawn`] through
/// `spawn_rx`: keeping the `Child` created and installed on the driver
/// from birth means no failure path between thread creation and
/// handshake can leak a running worker — every post-driver error
/// funnels through channel disconnect plus the shared-child kill, and
/// a driver-spawn failure happens before the child exists.
fn spawn_driver(
    shared: &Arc<Shared>,
    mut command: ProcCommand,
    driver_rx: mpsc::Receiver<DriverCmd>,
    grace: Duration,
) -> Result<(thread::JoinHandle<()>, SpawnRx), HostError> {
    let (spawn_tx, spawn_rx) = mpsc::channel();
    let driver_shared = Arc::clone(shared);
    let join = thread::Builder::new()
        .name("l1-driver".to_owned())
        .spawn(move || {
            let Ok((child, stdin, stdout, stderr)) = spawn_child(&mut command) else {
                // The OS spawn (or pipe take) failed — `Spawn`.
                driver_shared.diag.record_cause(FailureCause::Spawn);
                let _ = spawn_tx.send(Err(()));
                return;
            };
            // Install the shared handle BEFORE any converge can
            // run — from here on every `Failed` transition can
            // force-kill the child even while this driver is
            // blocked in a stdin write.
            if let Err(still_owned) = driver_shared.child.set(Mutex::new(child)) {
                // Unreachable (single install site) — but never
                // leak a running child even so.
                driver_shared.diag.record_cause(FailureCause::Internal);
                let mut child = still_owned
                    .into_inner()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let _ = child.kill();
                let _ = child.wait();
                let _ = spawn_tx.send(Err(()));
                return;
            }
            // Ownership of `stdout`/`stderr` crosses to the reader
            // and drain threads through the caller; the child
            // itself never leaves.
            if spawn_tx.send(Ok(SpawnedIo { stdout, stderr })).is_err() {
                kill_child(&driver_shared);
                if let Some(mut child) = driver_shared.child_lock() {
                    let _ = child.wait();
                }
                return;
            }
            driver_main(&driver_shared, stdin, &driver_rx, grace);
        })
        .map_err(|_| HostError::Ipc)?;
    Ok((join, spawn_rx))
}

/// Spawns the `l1-reader` thread — runs [`reader_main`]. No shm
/// `View` is attached up front: instances arrive over the child's
/// life via `CreateInstance`, and the reader attaches each entry's
/// segment lazily on its first `BlockDone` (a host-side attach
/// failure there is a child-scope convergence).
fn spawn_reader(
    shared: &Arc<Shared>,
    stdout: ChildStdout,
) -> Result<(thread::JoinHandle<()>, HelloRx), HostError> {
    let (hello_tx, hello_rx) = mpsc::channel();
    let reader_shared = Arc::clone(shared);
    let join = thread::Builder::new()
        .name("l1-reader".to_owned())
        .spawn(move || reader_main(stdout, &reader_shared, &hello_tx))
        .map_err(|_| HostError::Ipc)?;
    Ok((join, hello_rx))
}

/// What [`L1Child::create_instance`] hands the facade — the
/// instance-local bundle a successful `CreateInstance` minted.
/// `pub(crate)` so a T3 pool slot can carry it to
/// [`L1Instance::from_parts`]; the fields stay module-private.
pub(crate) struct InstanceParts {
    /// The minted wire handle.
    instance: u32,
    /// The facade's clone of `InstShared.rt_state` — the lock-free
    /// mirror snapshot `process`/`lifecycle_state` read. Survives
    /// entry removal: post-`Destroy` `lifecycle_state` still answers
    /// `Destroyed` because `L1Instance::destroy` writes the final
    /// state onto this `Arc` when the entry goes away.
    rt_state: Arc<AtomicU8>,
    /// The instance's own `FailedNotice` queue —
    /// `InstShared.notify_tx`'s pair.
    notify_rx: mpsc::Receiver<FailedNotice>,
    /// The RT leg: caller-thread end of the data plane (submit
    /// producer, result consumer, staging, pass-through history).
    rt: data::RtLeg,
    /// The owner-side segment anchor — RAII, unlinks on drop.
    segment: shm::Segment,
    /// The shm slot count `D` (`lookahead_blocks + 1`) this
    /// instance's segment/channels/staging were sized with.
    depth: u32,
}

/// What [`L1Child::create_video_instance`] hands the video facade —
/// the `CreateVideoInstance` counterpart of [`InstanceParts`]: no
/// `RtLeg`, no audio segment — the owned [`vshm::Segment`] is the
/// whole per-instance transport.
pub(crate) struct VideoInstanceParts {
    /// The minted wire handle.
    instance: u32,
    /// The facade's clone of `InstShared.rt_state` — the lock-free
    /// mirror snapshot `lifecycle_state` reads; survives entry
    /// removal (`destroy` writes `Destroyed` onto it).
    rt_state: Arc<AtomicU8>,
    /// The instance's own `FailedNotice` queue —
    /// `InstShared.notify_tx`'s pair.
    notify_rx: mpsc::Receiver<FailedNotice>,
    /// The owner-side `vshm` segment — kept alive on the facade so
    /// the child's attach stays valid; unlinks on drop (RAII).
    segment: vshm::Segment,
}

impl L1Child {
    /// Spawns a fresh child — driver + reader threads, the process
    /// itself, the `Hello` version gate — the shareable unit a
    /// dedicated [`L1Instance::spawn`] (and later a T3 pool slot)
    /// builds on. `Scan`/descriptor match and `CreateInstance` are
    /// deliberately *not* here: `scan_match` is a child-scope verb
    /// the caller decides to run (the dedicated route always does),
    /// and `create_instance` mints per-instance.
    ///
    /// Every failure freezes the capture, registers the attempt
    /// record into `diag` and returns `SpawnFailed{cause,
    /// diagnostics_id}` — the record outlives the `Shared` that
    /// drops with the failed attempt.
    ///
    /// **RT contract:** non-RT only — blocks up to
    /// `2 × command_timeout` (spawn hand-off + `Hello` wait).
    pub(crate) fn spawn(
        command: ProcCommand,
        command_timeout: Duration,
        diag: &diag::DiagRegistry,
    ) -> Result<Arc<Self>, HostError> {
        let (driver_tx, driver_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                waiters: HashMap::new(),
                instances: HashMap::new(),
                block_in_flight: HashSet::new(),
                terminal: None,
                shutdown_initiated: false,
                seen_eof: false,
            }),
            child: OnceLock::new(),
            next_cmd_seq: AtomicU64::new(0),
            diag: Arc::new(diag::ActiveDiagnostics::new(diag::SpawnPhase::PreSpawn)),
        });
        shared.diag.set_phase(diag::SpawnPhase::Spawn);
        let (driver_join, spawn_rx) =
            match spawn_driver(&shared, command, driver_rx, command_timeout) {
                Ok(pair) => pair,
                Err(_error) => {
                    // Host-side thread start failure — `Internal`.
                    shared.diag.record_cause(FailureCause::Internal);
                    return Err(spawn_failed(&shared, diag));
                }
            };
        let io = match spawn_rx.recv_timeout(command_timeout) {
            Ok(Ok(io)) => io,
            Ok(Err(())) => {
                // Child spawn failed — the driver recorded `Spawn`
                // at the observation site. Disconnecting the command
                // channel makes the driver kill/reap whatever it
                // started (bounded); it may not have spawned at all.
                drop(driver_tx);
                let _ = driver_join.join();
                return Err(spawn_failed(&shared, diag));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                shared.diag.record_cause(FailureCause::Timeout);
                drop(driver_tx);
                let _ = driver_join.join();
                return Err(spawn_failed(&shared, diag));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // The driver thread died mid-handoff — `Internal`.
                shared.diag.record_cause(FailureCause::Internal);
                drop(driver_tx);
                let _ = driver_join.join();
                return Err(spawn_failed(&shared, diag));
            }
        };
        // The stderr drain starts before the `Hello` wait — exactly
        // so pre-handshake output lands in the capture. Its start
        // failure rolls back through the same kill/reap path as a
        // reader-start failure (detached: its `JoinHandle` drops).
        shared.diag.set_phase(diag::SpawnPhase::DrainInit);
        if diag::spawn_drain(io.stderr, Arc::clone(&shared.diag), Arc::clone(&shared)).is_err() {
            // Host-side thread start failure — `Internal`.
            shared.diag.record_cause(FailureCause::Internal);
            kill_child(&shared);
            drop(driver_tx);
            let _ = driver_join.join();
            return Err(spawn_failed(&shared, diag));
        }
        shared.diag.set_phase(diag::SpawnPhase::Handshake);
        let (reader_join, hello_rx) = match spawn_reader(&shared, io.stdout) {
            Ok(pair) => pair,
            Err(_error) => {
                // Host-side thread start failure — `Internal`. The
                // driver is already running — kill the child through
                // the shared handle and disconnect; the driver reaps
                // and exits (bounded — `kill` is forceful on both
                // OSes).
                shared.diag.record_cause(FailureCause::Internal);
                kill_child(&shared);
                drop(driver_tx);
                let _ = driver_join.join();
                return Err(spawn_failed(&shared, diag));
            }
        };
        match hello_rx.recv_timeout(command_timeout) {
            Ok(Ok(version)) if version == wire::WIRE_VERSION => {}
            other => {
                // Missing/mismatched `Hello` — the spawn is
                // unusable; kill + reap, never a live-but-empty
                // child. A version mismatch is `Handshake`; a wait
                // timeout is `Timeout`; the reader's own `Err(())`
                // (and a dead channel) already carried its cause
                // through convergence — recording is first-wins.
                shared.diag.record_cause(match other {
                    Ok(Ok(_)) => FailureCause::Handshake,
                    Err(mpsc::RecvTimeoutError::Timeout) => FailureCause::Timeout,
                    _ => FailureCause::Internal,
                });
                kill_child(&shared);
                drop(driver_tx);
                let _ = driver_join.join();
                drop(reader_join);
                return Err(spawn_failed(&shared, diag));
            }
        }
        shared.diag.set_phase(diag::SpawnPhase::Live);
        Ok(Arc::new(Self {
            shared,
            driver_tx: Mutex::new(Some(driver_tx)),
            driver_join: Mutex::new(Some(driver_join)),
            reader_join: Mutex::new(Some(reader_join)),
            command_timeout,
        }))
    }

    /// `Scan` → descriptor field-match — the child-scope handshake
    /// half the dedicated route always runs after [`L1Child::spawn`]
    /// (a T3 pooled child is scanned once at mint instead). A
    /// `Failed`-child `Err` surfaces as `Ipc`; the caller freezes
    /// and registers it through [`spawn_failed`].
    pub(crate) fn scan_match(&self, spec: &SpawnSpecView<'_>) -> Result<(), HostError> {
        let matched = match self
            .request(wire::WIRE_INSTANCE_NONE, wire::Command::Scan)
            .map_err(|_| HostError::Ipc)?
        {
            wire::Reply::Scanned(descriptors) => {
                descriptors.iter().any(|d| d.matches(spec.descriptor))
            }
            // A decodable but wrong-variant handshake reply —
            // `Handshake` like the no-match arm below.
            _ => false,
        };
        if matched {
            return Ok(());
        }
        // The spawn's handshake failed (this only ever runs on a
        // fresh child, so the record is the spawn attempt's).
        self.shared.diag.record_cause(FailureCause::Handshake);
        Err(HostError::Ipc)
    }

    /// A fresh child for `spec` — the v2 argv wiring (`program` +
    /// `spec.args` only; `plugin_id`/`shm_key` ride the
    /// `CreateInstance` payload — plus the piped-stdio layout), then
    /// [`L1Child::spawn`] (the `Hello` version gate inside) and the
    /// child-scope [`L1Child::scan_match`]. This is the whole
    /// per-child half of the handshake: the dedicated route
    /// ([`L1Instance::spawn`]) and a pooled route's fresh spawn run
    /// exactly the same sequence before per-instance minting.
    ///
    /// Every failure — spawn error, timeout, `Hello`/descriptor
    /// mismatch — registers the attempt record into `diag`, aborts
    /// the fresh child (kill + reap + join, no `Shutdown` grace:
    /// the spawn never answered as a live child) and returns
    /// `SpawnFailed`, the design's fresh-spawn rollback. `spec`
    /// legality ([`SpawnSpec::instance_depth`]) is the caller's
    /// precondition — check it before paying the spawn.
    ///
    /// **RT contract:** non-RT only — [`L1Child::spawn`]'s bounds
    /// plus the `Scan` wait.
    pub(crate) fn spawn_scanned(
        spec: &SpawnSpecView<'_>,
        diag: &diag::DiagRegistry,
    ) -> Result<Arc<Self>, HostError> {
        let mut command = ProcCommand::new(spec.program);
        command
            .args(spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Piped so the drain thread captures diagnostics —
            // stderr never implicitly echoes to the host's stderr.
            .stderr(Stdio::piped());
        let child = Self::spawn(command, spec.command_timeout, diag)?;
        if child.scan_match(spec).is_err() {
            // Fresh-child handshake failure — freeze + register
            // BEFORE teardown so the record captures stderr at the
            // failure point; then kill + reap, no notification:
            // nothing ever went live on it.
            let error = spawn_failed(&child.shared, diag);
            child.abort();
            return Err(error);
        }
        Ok(child)
    }

    /// Mints a wire handle, registers the `InstShared` (`Creating`),
    /// installs the data-plane ends on the driver and sends
    /// `CreateInstance` — the per-instance half of the v2 handshake.
    /// `Reply::Ok` flips the entry `Live` — atomically, and only
    /// while the entry is still mid-mint: an `Ok` delivered ahead
    /// of a `converge_child` folds into the same rollback as a
    /// defined rejection (the mid-create-death path —
    /// `SpawnFailed`, no notification, no facade) — and returns
    /// the facade parts.
    /// Every other outcome rolls the entry back (the runner
    /// inserts only on success, so no runner residue): the defined
    /// `Err(InstanceLimit)` capacity rejection propagates
    /// unflattened so a pooled caller can tell "try another
    /// child" apart from a transport failure; everything else
    /// registers an *attempt* record into `diag` (a frozen copy of
    /// the child's capture at that instant — a live pooled child's
    /// own record keeps mutating separately) and returns
    /// `Err(SpawnFailed{cause, diagnostics_id})`. **The caller
    /// decides** whether the child was fresh (`abort` — the
    /// dedicated route) or shared (siblings stay live — the T3
    /// pool path).
    ///
    /// `depth` is `D = lookahead_blocks + 1` — segment, channels
    /// and `RtLeg` sizing in one.
    pub(crate) fn create_instance(
        &self,
        spec: &SpawnSpec,
        depth: u32,
        diag: &diag::DiagRegistry,
    ) -> Result<InstanceParts, HostError> {
        // Segment + SPSC legs are per-instance from birth — minted
        // here so the entry's `SegmentInfo` names the key before
        // `CreateInstance` ships it. The `notify`/`result`
        // producers live in `InstShared` (child-shared); the
        // `submit` consumer rides `AddInstance` to the driver.
        let slot =
            u32::try_from(INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap_or(u32::MAX);
        // Host-side segment creation failure — a `SpawnFailed`
        // (`Internal`) attempt record, not a defined rejection.
        let segment = shm::Segment::create(slot, depth, depth)
            .map_err(|()| spawn_failed(&self.shared, diag))?;
        let (submit_tx, submit_rx) =
            ruviform_rt::channel::<data::Submit>(usize::try_from(depth).unwrap_or(usize::MAX));
        let (result_tx, result_rx) =
            ruviform_rt::channel::<u64>(usize::try_from(depth).unwrap_or(usize::MAX));
        let (notify_tx, notify_rx) = mpsc::channel();
        let rt_state = Arc::new(AtomicU8::new(data::mirror_encode(LifecycleState::Created)));
        let instance = {
            let mut inner = self.shared.lock();
            if inner.terminal.is_some() || inner.shutdown_initiated {
                return Err(spawn_failed(&self.shared, diag));
            }
            // The lowest-free mint under the lock — handle +
            // insert are atomic; a full registry is the defined
            // `InstanceLimit`.
            let Some(instance) = mint_instance(&inner) else {
                return Err(HostError::InstanceLimit);
            };
            inner.instances.insert(
                instance,
                InstShared {
                    rt_state: Arc::clone(&rt_state),
                    mirror: LifecycleState::Created,
                    notify_tx,
                    notify: Notify::Idle,
                    kind: InstKind::Audio {
                        staging: None,
                        shm: SegmentInfo {
                            key: segment.key().to_owned(),
                            in_depth: depth,
                            out_depth: depth,
                        },
                        result_tx: Mutex::new(result_tx),
                        counters: AudioCounters::new(),
                    },
                    phase: InstPhase::Creating,
                    cause: None,
                },
            );
            instance
        };
        // The driver installs the data-plane ends before
        // `CreateInstance` ships — submits cannot precede
        // `Activate`, so the ordering is slack; what matters is
        // the register/send pair is atomic from the wire's view.
        self.send_driver(DriverCmd::AddInstance {
            instance,
            submit: submit_rx,
            shm_key: segment.key().to_owned(),
        });
        let create = self.request(
            instance,
            wire::Command::CreateInstance {
                plugin_id: spec.descriptor.id.to_owned(),
                shm_key: segment.key().to_owned(),
                in_depth: depth,
                out_depth: depth,
            },
        );
        if let Ok(wire::Reply::Ok) = create {
            // The `Ok` reply correlated its waiter before it was
            // removed — a `converge_child` that ran between the
            // delivery and this lock already set `terminal` and
            // flipped the still-`Creating` entry `Failed` *without*
            // queuing a notification (`minted()` was false, so the
            // `Creating` entry converged silently). An
            // unconditional `Live` flip here would regress the
            // phase and mint a facade whose `ChildExit` never
            // arrives — the exactly-once-per-minted-instance
            // guarantee lost. The flip therefore happens under
            // the same lock that proves the entry is still
            // mid-mint; anything else folds into the
            // mid-create-death rollback below (`SpawnFailed`, no
            // notification, no facade — the same path a defined
            // `Err` reply takes).
            let (went_live, leg_counters) = {
                let mut inner = self.shared.lock();
                let went_live = inner.terminal.is_none()
                    && inner
                        .instances
                        .get(&instance)
                        .is_some_and(|inst| inst.phase == InstPhase::Creating);
                let mut leg_counters = None;
                if went_live {
                    let inst = inner
                        .instances
                        .get_mut(&instance)
                        .expect("presence checked above");
                    inst.phase = InstPhase::Live;
                    // The entry just minted is `InstKind::Audio` —
                    // the leg writers clone the same cells the
                    // query surface reads.
                    leg_counters = inst.audio_leg_counters();
                }
                (went_live, leg_counters)
            };
            if went_live {
                return Ok(InstanceParts {
                    instance,
                    rt_state,
                    notify_rx,
                    rt: data::RtLeg::new(
                        depth,
                        spec.lookahead_blocks,
                        submit_tx,
                        result_rx,
                        leg_counters.expect("a live audio entry carries counters"),
                    ),
                    segment,
                    depth,
                });
            }
        }
        // Roll the entry back — `RemoveInstance` frees this
        // handle's driver ends for reuse; the runner keeps no
        // residue for a failed `CreateInstance`. The defined
        // capacity rejection propagates unflattened so a caller can
        // distinguish it from transport failure (`InstanceLimit` is
        // unreachable once `pool_eligible` excluded a full child,
        // but the seam keeps it defined); every other outcome —
        // defined rejections, transport failures, unexpected reply
        // variants and the converged-mid-mint fold above — is
        // `SpawnFailed` carrying the classified cause.
        let error = create_rollback_error(&self.shared, diag, &create);
        let mut inner = self.shared.lock();
        remove_instance(&mut inner, instance);
        drop(inner);
        self.send_driver(DriverCmd::RemoveInstance { instance });
        Err(error)
    }

    /// The `CreateVideoInstance` counterpart of
    /// [`L1Child::create_instance`] — same create/reply/rollback
    /// shape (mint → `Creating` entry → create verb → atomic `Live`
    /// flip → facade parts), with the video-specific differences
    /// the design pins:
    ///
    /// - the segment is a [`vshm::Segment`] sized from `config`,
    ///   not a slot-ring `shm::Segment`;
    /// - the entry registers as `InstKind::Video` — **no**
    ///   `DriverCmd::AddInstance` (a video entry owns no driver-side
    ///   data-plane ends: no submit/result channels, no `View`) and
    ///   rollback likewise sends **no** `RemoveInstance`;
    /// - `Err(InstanceLimit)` still propagates unflattened — the
    ///   same capacity fact a pooled caller must distinguish —
    ///   every other failure registers the attempt record into
    ///   `diag` and returns `Err(SpawnFailed{cause,
    ///   diagnostics_id})`, mirroring [`L1Child::create_instance`].
    ///
    /// The dedicated video route always calls this on a fresh
    /// child (`VideoL1Instance::spawn`); the caller decides the
    /// child's fate on failure exactly like the audio path.
    ///
    /// **RT contract:** non-RT only — segment create plus one
    /// `command_timeout` reply wait.
    pub(crate) fn create_video_instance(
        &self,
        spec: &SpawnSpecView<'_>,
        config: VideoProcessConfig,
        diag: &diag::DiagRegistry,
    ) -> Result<VideoInstanceParts, HostError> {
        // Mint the per-instance vshm segment before the entry names
        // its key — `CreateVideoInstance` ships the `os_id`, so the
        // segment must exist first.
        let slot =
            u32::try_from(INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap_or(u32::MAX);
        let segment =
            vshm::Segment::create(slot, &config).map_err(|()| spawn_failed(&self.shared, diag))?;
        let (notify_tx, notify_rx) = mpsc::channel();
        let rt_state = Arc::new(AtomicU8::new(data::mirror_encode(LifecycleState::Created)));
        let instance = {
            let mut inner = self.shared.lock();
            if inner.terminal.is_some() || inner.shutdown_initiated {
                return Err(spawn_failed(&self.shared, diag));
            }
            // The lowest-free mint under the lock — handle +
            // insert are atomic; a full registry is the defined
            // `InstanceLimit`.
            let Some(instance) = mint_instance(&inner) else {
                return Err(HostError::InstanceLimit);
            };
            inner.instances.insert(
                instance,
                InstShared {
                    rt_state: Arc::clone(&rt_state),
                    mirror: LifecycleState::Created,
                    notify_tx,
                    notify: Notify::Idle,
                    kind: InstKind::Video { config },
                    phase: InstPhase::Creating,
                    cause: None,
                },
            );
            instance
        };
        // No `DriverCmd::AddInstance` — a video entry owns no
        // driver-side per-instance state (the `submits`/`views`
        // maps are audio data-plane only); the facade's segment is
        // host-side, the child attaches it by `shm_key`.
        let create = self.request(
            instance,
            wire::Command::CreateVideoInstance {
                plugin_id: spec.descriptor.id.to_owned(),
                shm_key: segment.key().to_owned(),
                config,
            },
        );
        if let Ok(wire::Reply::Ok) = create {
            // Same mid-mint race as `create_instance`: the `Ok`
            // reply may land after a `converge_child` already
            // flipped the `Creating` entry `Failed` — the `Live`
            // flip happens only while the entry is provably still
            // mid-mint, anything else folds into rollback.
            let went_live = {
                let mut inner = self.shared.lock();
                let went_live = inner.terminal.is_none()
                    && inner
                        .instances
                        .get(&instance)
                        .is_some_and(|inst| inst.phase == InstPhase::Creating);
                if went_live {
                    inner
                        .instances
                        .get_mut(&instance)
                        .expect("presence checked above")
                        .phase = InstPhase::Live;
                }
                went_live
            };
            if went_live {
                return Ok(VideoInstanceParts {
                    instance,
                    rt_state,
                    notify_rx,
                    segment,
                });
            }
        }
        // Roll the entry back — the runner keeps no residue for a
        // failed `CreateVideoInstance`, and there are no driver
        // ends to free (no `RemoveInstance` — nothing was ever
        // installed). `InstanceLimit` propagates; everything else
        // is `SpawnFailed` carrying the classified cause.
        let error = create_rollback_error(&self.shared, diag, &create);
        let mut inner = self.shared.lock();
        remove_instance(&mut inner, instance);
        drop(inner);
        Err(error)
    }

    /// Registers the waiter, queues the frame and waits — the
    /// command channel the handshake (`Scan`/`CreateInstance`) and
    /// [`L1Instance::request`] share. `instance` is the stamped
    /// envelope value: `WIRE_INSTANCE_NONE` for the child-scope
    /// `Scan`, the minted handle otherwise.
    ///
    /// The instance-scope gate runs **inside** the waiter insert —
    /// atomically: `Scan` needs no entry; ordinary commands require
    /// `Live`; `CreateInstance`/`CreateVideoInstance` are legal
    /// while `Creating` (they ARE the create); `DestroyInstance` is
    /// legal on `Failed`/`Destroying` entries (reclaiming their
    /// runner slot is its job). Everything else on a
    /// dead/destroying/removed entry is `Err(Ipc)` — `Failed` →
    /// `Err(HostError::Failed)`.
    ///
    /// **RT contract:** non-RT only — waits at most `command_timeout`;
    /// a reply timeout converges the child `Failed` (`Ipc`).
    pub(crate) fn request(
        &self,
        instance: u32,
        cmd: wire::Command,
    ) -> Result<wire::Reply, HostError> {
        self.request_timed(instance, cmd, self.command_timeout)
    }

    /// [`L1Child::request`] with an explicit reply wait — the
    /// timeout-generalized request the video `Render` window needs
    /// (`render_timeout` is the only verb that outlasts
    /// `command_timeout`; every other caller takes the wrapper's
    /// `command_timeout` default). Timeout behavior is identical:
    /// waiter removal, child-scope `converge_child`,
    /// `Err(HostError::Ipc)`.
    ///
    /// **RT contract:** non-RT only — waits at most `timeout`.
    pub(crate) fn request_timed(
        &self,
        instance: u32,
        cmd: wire::Command,
        timeout: Duration,
    ) -> Result<wire::Reply, HostError> {
        let seq = self.shared.next_cmd_seq.fetch_add(1, Ordering::Relaxed);
        let (tx, reply_rx) = mpsc::channel();
        {
            let mut inner = self.shared.lock();
            match inner.terminal {
                Some(Terminal::Failed) => return Err(HostError::Failed),
                Some(Terminal::Clean) => return Err(HostError::Ipc),
                None => {}
            }
            if inner.shutdown_initiated {
                return Err(HostError::Ipc);
            }
            if instance != wire::WIRE_INSTANCE_NONE {
                match inner.instances.get(&instance) {
                    Some(inst) => match inst.phase {
                        InstPhase::Live => {}
                        InstPhase::Creating
                            if matches!(
                                cmd,
                                wire::Command::CreateInstance { .. }
                                    | wire::Command::CreateVideoInstance { .. }
                            ) => {}
                        InstPhase::Failed | InstPhase::Destroying
                            if matches!(cmd, wire::Command::DestroyInstance) => {}
                        InstPhase::Failed => return Err(HostError::Failed),
                        _ => return Err(HostError::Ipc),
                    },
                    None => return Err(HostError::Ipc),
                }
            }
            inner.waiters.insert(seq, Waiter { instance, tx });
        }
        let sent = {
            let driver_tx = lock_mutex(&self.driver_tx);
            driver_tx.as_ref().is_some_and(|driver_tx| {
                driver_tx
                    .send(DriverCmd::Command { seq, instance, cmd })
                    .is_ok()
            })
        };
        if !sent {
            self.shared.lock().waiters.remove(&seq);
            return Err(HostError::Ipc);
        }
        match reply_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.shared.lock().waiters.remove(&seq);
                // A reply timeout is `Ipc` plus `Failed` — the child
                // is presumed lost; `converge_child` kills it (which
                // is also what unblocks a driver stuck mid-write).
                converge_child(&self.shared, None, FailureCause::Timeout);
                Err(HostError::Ipc)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(HostError::Ipc),
        }
    }

    /// Best-effort driver order — a dead channel means the driver is
    /// already down (converged); the order's absence cannot hurt.
    pub(crate) fn send_driver(&self, cmd: DriverCmd) {
        let driver_tx = lock_mutex(&self.driver_tx);
        if let Some(driver_tx) = driver_tx.as_ref() {
            let _ = driver_tx.send(cmd);
        }
    }

    /// Disconnects the command channel — the driver's
    /// `Disconnected` arm is itself a bounded teardown (kill +
    /// reap + converge).
    fn disconnect_driver(&self) {
        drop(lock_mutex(&self.driver_tx).take());
    }

    /// Bounded driver join — the grace window plus a reap margin,
    /// then a force-kill through the shared handle (the kill turns
    /// a wedged `write_all` into EPIPE; the post-kill join then
    /// completes once the driver unwinds and reaps). The reader is
    /// *detached* — a stdio-inheriting grandchild can hold the
    /// child's stdout pipe open past its death, so joining could
    /// stall forever; it touches only `Arc`-shared state.
    fn join_driver(&self) {
        if let Some(join) = lock_mutex(&self.driver_join).take() {
            let deadline = Instant::now() + self.command_timeout + DRIVER_JOIN_MARGIN;
            while !join.is_finished() {
                if Instant::now() >= deadline {
                    kill_child(&self.shared);
                    break;
                }
                thread::sleep(DRIVER_JOIN_POLL);
            }
            let _ = join.join();
        }
    }

    /// Registered-instance count — the `live_count` the
    /// shutdown-on-empty rule counts (`Creating`/`Failed`/
    /// `Destroying` entries included until `DestroyInstance`
    /// reclaims them). The pooled route reads this at
    /// `Destroy`-time eviction and inside [`L1Child::pool_eligible`];
    /// the dedicated route never does.
    pub(crate) fn live_count(&self) -> usize {
        self.shared.lock().instances.len()
    }

    /// The pooled route's child-selection predicate — `live_count <
    /// WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` —
    /// evaluated atomically under one `inner` lock so the answer is
    /// never a torn read. A terminal child can never serve again
    /// (the pool evicts it); a `shutdown_initiated` one is already
    /// draining toward clean teardown.
    pub(crate) fn pool_eligible(&self) -> bool {
        let inner = self.shared.lock();
        inner.instances.len() < wire::WIRE_MAX_INSTANCES as usize
            && inner.terminal.is_none()
            && !inner.shutdown_initiated
    }

    /// `Inner.terminal.is_some()` — `converge_child` ran, the child
    /// is dead/converging and can never serve again. The pool's
    /// dead-entry eviction oracle: removing the pool's `Arc` here
    /// leaves live facades holding their own references until their
    /// `destroy`/`Drop` runs the last-referrer teardown.
    pub(crate) fn is_terminal(&self) -> bool {
        self.shared.lock().terminal.is_some()
    }

    /// Fresh-spawn abort: kill + disconnect + join — no
    /// notification owed, no `Shutdown` grace (the spawn never
    /// answered as a live child).
    pub(crate) fn abort(&self) {
        kill_child(&self.shared);
        self.disconnect_driver();
        self.join_driver();
        drop(lock_mutex(&self.reader_join).take());
    }

    /// Bounded last-referrer teardown: a `Shutdown` while the child
    /// still looks live (the driver's `Disconnected` arm then honors
    /// the grace deadline — clean teardown, not a kill), then
    /// disconnect + the join bound + reader detach. Idempotent —
    /// every piece is `take()`/best-effort.
    pub(crate) fn teardown(&self) {
        {
            let inner = self.shared.lock();
            if inner.terminal.is_none() && !inner.seen_eof {
                drop(inner);
                self.send_driver(DriverCmd::Shutdown);
            }
        }
        self.disconnect_driver();
        self.join_driver();
        drop(lock_mutex(&self.reader_join).take());
        // The driver's `Disconnected` arm converges before it exits
        // — `get_or_insert` covers the defensive remainder only.
        self.shared.lock().terminal.get_or_insert(Terminal::Clean);
    }
}

/// One L1 instance: its registry entry on a shared [`L1Child`]
/// (dedicated = the capacity-1 route through the same machinery),
/// the per-instance notification queue, the `RtLeg` and the local
/// lifecycle mirror snapshot.
///
/// **RT contract:** every call here is **non-RT only** — spawns,
/// thread joins and pipe writes may block and allocate. `execute`
/// waits at most `command_timeout` per command; `destroy`/`drop` are
/// bounded by the shutdown watchdog's kill guarantee.
///
/// **Send/Sync:** `!Send`/`!Sync` — `segment` holds a
/// `shared_memory::Shmem` (raw pointer), so an instance cannot leave
/// its owning thread; `PluginHost` inherits that (documented on the
/// composite).
pub(crate) struct L1Instance {
    /// `child.shared`'s clone — the hot-path lock (design keeps both:
    /// `shared` for `inner` access, `child` for driver/ownership).
    shared: Arc<Shared>,
    /// The child this instance lives on — `Arc::strong_count == 1`
    /// inside `destroy`/`Drop` means the last referrer, which owns
    /// the bounded teardown (live siblings must never be killed).
    child: Arc<L1Child>,
    /// The minted wire handle — stamps every non-`Scan` command and
    /// this instance's `BlockIn` doorbells (`Scan` bypasses the
    /// handle with `WIRE_INSTANCE_NONE`).
    instance: u32,
    /// This instance's own `FailedNotice` notification queue —
    /// `InstShared.notify_tx`'s pair: a child crash fans one
    /// notification out per minted instance.
    notify_rx: mpsc::Receiver<FailedNotice>,
    /// The facade's clone of `InstShared.rt_state` — the lock-free
    /// mirror snapshot `process`/`lifecycle_state` read; survives
    /// entry removal (`destroy` writes `Destroyed` onto it).
    rt_state: Arc<AtomicU8>,
    /// The shm slot count `D` (`lookahead_blocks + 1`) — sizes the
    /// segment, both SPSC channels and the staging ring.
    depth: u32,
    /// The RT leg: caller-thread end of the data plane (submit
    /// producer, result consumer, staging, pass-through history).
    /// `None` while an endpoint has it checked out — the
    /// authoritative "endpoint attached" fact. The bookkeeping
    /// fields below are per-attach and cleared by the non-RT
    /// [`L1Instance::drain_endpoint_channels`] once a restored leg
    /// is home (a wait-free `process`-path restore leaves them in
    /// place), but `rt` presence is what `process`/
    /// `prepare_endpoint`/`execute`'s bound gate key on.
    rt: Option<data::RtLeg>,
    /// Counts successful `Activate`s — stamped onto each new
    /// [`data::Staging`] ring so the driver can drop `submit`s
    /// queued under a superseded ring (see [`data::Submit`]).
    staging_generation: u64,
    /// Producer half of the per-attach arm channel (capacity 2) —
    /// `Some` exactly while an endpoint holds the leg. Successful
    /// `Activate`s and the pending-arm resend push
    /// `Arc<data::Staging>` through it; the endpoint's `process`
    /// drains the consumer half.
    arm_tx: Option<ruviform_rt::Producer<Arc<data::Staging>>>,
    /// Latest-wins slot for an `Arc<data::Staging>` that did not
    /// fit a full arm channel — re-sent (`try_push`) at the top of
    /// every mutable entry point; `None` when nothing is owed.
    /// `Full` is a no-op: the endpoint holds an at-least-as-new
    /// ring already delivered or in flight.
    pending_arm: Option<Arc<data::Staging>>,
    /// Consumer half of the leg-return channel (capacity 1) —
    /// `L1Endpoint::drop` pushes the checked-out leg here and the
    /// next mutable entry point's drain restores it into `rt`.
    return_rx: Option<ruviform_rt::Consumer<data::RtLeg>>,
    /// Drains the `Retirer` the endpoint pushes drained arms
    /// (applied or stale) and displaced rings into — the
    /// destructor work stays on the non-RT side. Drained on every
    /// mutable entry point that is allowed to run destructors —
    /// never inside `process`.
    arm_collector: Option<ruviform_rt::DropCollector<Arc<data::Staging>>>,
    /// The `(max_frames, channels)` bound this attach promised the
    /// endpoint — a re-`Activate` above `bound.0` while the leg is
    /// detached is `Err(Prepare(UnsupportedConfig))`, so the ring
    /// the endpoint's `last_output` was sized for is never
    /// outgrown.
    endpoint_bound: Option<(u32, u32)>,
    /// Sample rate of the last successful `Activate` — the
    /// composite reads it to size the endpoint input scratch.
    /// `Some` exactly when a staging ring is armed.
    activated_sample_rate: Option<u32>,
    /// The named shm segment — owner-side lifecycle anchor (kept
    /// alive so the child and the I/O threads' `View`s can attach;
    /// it unlinks when the instance drops). RAII-only — never read.
    #[allow(dead_code)]
    segment: shm::Segment,
}

impl L1Instance {
    /// Spawns a dedicated child and runs the full v2 handshake:
    /// `Hello` (version must equal [`wire::WIRE_VERSION`], inside
    /// [`L1Child::spawn`]) → `Scan` → the reported
    /// `wire::Descriptor`s must field-match `spec.descriptor` →
    /// `CreateInstance` mints the registry entry. Any failure —
    /// spawn error, timeout, version/descriptor mismatch,
    /// `CreateInstance` rejection — registers the attempt record
    /// into `diag` and returns `Err(HostError::SpawnFailed{cause,
    /// diagnostics_id})` with the fresh child killed and reaped
    /// (a fresh-spawn failure tears the whole child down — the
    /// shared-child rollback lives inside
    /// [`L1Child::spawn_scanned`]). Defined rejections stay
    /// unwrapped and register no record: the child's
    /// `Err(InstanceLimit)` capacity rejection propagates
    /// unflattened as `Err(HostError::InstanceLimit)` and a
    /// spec-legality violation stays `Err(HostError::Ipc)`.
    ///
    /// **RT contract:** non-RT only — blocks up to
    /// `4 × command_timeout` in the worst case (spawn hand-off,
    /// `Hello` wait, `Scan` reply, `CreateInstance` reply).
    ///
    /// `lookahead_blocks == 0` is `Err(Ipc)` — with `D = 1` the
    /// result horizon `expected = block` is the seq the same
    /// `process` call just submitted, so no result can ever arrive
    /// first: the leg would pass through forever (permanent
    /// silence). `lookahead_blocks >= wire::WIRE_MAX_ELEMENTS`
    /// (`D > WIRE_MAX_ELEMENTS`) is the same defined rejection —
    /// `D` is declared on `CreateInstance`'s `in_depth`/`out_depth`,
    /// which the wire codec caps at `WIRE_MAX_ELEMENTS`. The spec is
    /// rejected like any other unusable spawn configuration rather
    /// than silently degraded — before any segment or child exists.
    pub(crate) fn spawn(spec: &SpawnSpec, diag: &diag::DiagRegistry) -> Result<Self, HostError> {
        // The spec-legality gate runs before any segment or child
        // exists — [`SpawnSpec::instance_depth`] holds the rule.
        let depth = spec.instance_depth()?;
        let child = L1Child::spawn_scanned(&spec.view(), diag)?;
        let parts = match child.create_instance(spec, depth, diag) {
            Ok(parts) => parts,
            Err(error) => {
                // Fresh-child failure — kill + reap, no
                // notification: the instance never went live.
                child.abort();
                return Err(error);
            }
        };
        Ok(Self::from_parts(child, parts))
    }

    /// Assembles a facade around a minted registry entry — the
    /// shared constructor the dedicated route
    /// ([`L1Instance::spawn`]) and a T3 pool slot both use: the
    /// `child` clone keeps the shared child alive (its
    /// `Arc::strong_count` drives last-referrer teardown) and
    /// `parts` supplies the per-instance half
    /// [`L1Child::create_instance`] minted.
    ///
    /// **RT contract:** non-RT only — plain field moves.
    pub(crate) fn from_parts(child: Arc<L1Child>, parts: InstanceParts) -> Self {
        Self {
            shared: Arc::clone(&child.shared),
            child,
            instance: parts.instance,
            notify_rx: parts.notify_rx,
            rt_state: parts.rt_state,
            depth: parts.depth,
            rt: Some(parts.rt),
            staging_generation: 0,
            arm_tx: None,
            pending_arm: None,
            return_rx: None,
            arm_collector: None,
            endpoint_bound: None,
            activated_sample_rate: None,
            segment: parts.segment,
        }
    }

    /// The shared child this instance lives on — the pooled route's
    /// shutdown-on-empty eviction and `kill_child`'s child-scope
    /// lookup key on it. `Arc::strong_count` on the clone is what
    /// `destroy`/`Drop`'s last-referrer teardown reads, so callers
    /// must drop the clone before that check means anything.
    ///
    /// **RT contract:** non-RT only — an `Arc` clone.
    pub(crate) fn child(&self) -> &Arc<L1Child> {
        &self.child
    }

    /// Sends one wire command and waits for its reply — the
    /// command/reply parity surface for the L0 command set.
    ///
    /// Defined rejections resolved without touching the wire:
    /// `Failed` instance → `Err(HostError::Failed)`; `OpenGui`/
    /// `CloseGui` → `Err(HostError::Unsupported)` (L1 GUI is
    /// deferred); an `Activate` config over [`wire::WIRE_MAX_FRAMES`]
    /// → `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`.
    /// A reply that never arrives inside `command_timeout` is
    /// `Err(HostError::Ipc)` — the timeout is unattributable, so
    /// `converge_child` fails the whole child: this instance *and*
    /// every live sibling minted on it, not this instance alone.
    ///
    /// **RT contract:** non-RT only — waits at most `command_timeout`.
    pub(crate) fn execute(&mut self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        // The non-RT entry-point drain — collects a returned leg,
        // re-sends `pending_arm` and drains the arm
        // `DropCollector` before any command work.
        self.drain_endpoint_channels();
        {
            let inner = self.shared.lock();
            match inner.terminal {
                Some(Terminal::Failed) => return Err(HostError::Failed),
                Some(Terminal::Clean) => return Err(HostError::Ipc),
                None => {}
            }
            if inner.shutdown_initiated {
                return Err(HostError::Ipc);
            }
            // Instance-scope rejections — a `Failed` entry is
            // `Err(Failed)` (granular: siblings still answer);
            // `Destroying`/removed entries are `Err(Ipc)`.
            // `Creating` is unreachable here — the entry flips
            // `Live` inside `create_instance` before the facade
            // exists.
            match inner.instances.get(&self.instance) {
                Some(inst) if inst.mirror.is_failed() => return Err(HostError::Failed),
                Some(inst) if inst.live() => {}
                _ => return Err(HostError::Ipc),
            }
        }
        match &cmd {
            wire::Command::OpenGui { .. } | wire::Command::CloseGui => {
                return Err(HostError::Unsupported);
            }
            wire::Command::Activate { config } if config.max_frames() > wire::WIRE_MAX_FRAMES => {
                return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
            }
            _ => {}
        }
        // The re-`Activate` bound gate: while the leg is detached
        // (`self.rt.is_none()` — the authoritative attached fact)
        // the endpoint's scratch contract is fixed, so a new
        // activation above the attach bound is rejected before the
        // command reaches the wire.
        if let wire::Command::Activate { config } = &cmd
            && self.rt.is_none()
            && let Some((bound_max_frames, _)) = self.endpoint_bound
            && config.max_frames() > bound_max_frames
        {
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        let post = post_command_state(&cmd);
        // The mirror state an `Err` reply still leaves behind —
        // `Some` only for `StopProcessing` (see
        // `err_committed_state`).
        let on_err = err_committed_state(&cmd);
        let activate_config = match &cmd {
            wire::Command::Activate { config } => Some(*config),
            _ => None,
        };
        let result = self.request(cmd);
        let committed = match &result {
            Ok(_) => post,
            // `Err(Lifecycle)` is always the pre-commit legality
            // rejection — the child never transitioned, so the
            // mirror must not move either.
            Err(HostError::Lifecycle(_)) => None,
            // Any other `Err` is post-commit for `StopProcessing`
            // alone: the child applies `Processing → Activated`
            // *before* the `stop_processing()` hook can decline
            // (the commit-then-advisory ordering
            // `InProcessHost::stop_processing` has in-process — the
            // hook error is advisory there too), so the mirror must
            // follow the commit or `lifecycle_state()` lies and the
            // RT leg keeps doorbelling a state the child already
            // left — the lag L0 does not have. Every other verb's
            // `Err` is pre-commit or rolled back child-side
            // (`err_committed_state` audits them), so it rightly
            // leaves the mirror untouched.
            Err(_) => on_err,
        };
        if let Some(state) = committed {
            let mut inner = self.shared.lock();
            // Never resurrect a converged instance — the reply may
            // have raced the crash paths (a timed-out
            // `StopProcessing` converges `Failed`; the guard keeps
            // the mirror `Failed`, never `Activated`). The instance
            // entry keys the mirror — a live entry only.
            if inner.terminal.is_none()
                && let Some(inst) = inner.instances.get_mut(&self.instance)
                && inst.live()
            {
                inst.set_mirror(state);
            }
        }
        let reply = result?;
        if let Some(config) = activate_config {
            // A successful `Activate` arms the data plane: build the
            // staging ring (non-RT allocation) and publish it to the
            // I/O threads plus the RT leg in one step. A concurrent
            // convergence leaves the armed ring unobserved — harmless.
            // The bumped `generation` lets the driver drop submits
            // queued under the superseded ring — see `data::Submit`.
            self.staging_generation += 1;
            let staging = Arc::new(data::Staging::new(
                self.depth,
                config.max_frames(),
                self.staging_generation,
            ));
            self.arm_staging(staging, config);
        }
        Ok(reply)
    }

    /// Publishes a freshly activated staging ring:
    /// `InstShared::staging` for the I/O threads, then the RT
    /// consumer — `RtLeg::arm` when the leg is local, the arm
    /// channel while an endpoint holds it (`pending_arm`
    /// latest-wins on `Full`). Also records the activation's
    /// sample rate for endpoint scratch sizing.
    fn arm_staging(&mut self, staging: Arc<data::Staging>, config: ProcessConfig) {
        if let Some(inst) = self.shared.lock().instances.get_mut(&self.instance)
            && let InstKind::Audio { staging: armed, .. } = &mut inst.kind
        {
            *armed = Some(Arc::clone(&staging));
        }
        self.activated_sample_rate = Some(config.sample_rate().get());
        match (&mut self.rt, &mut self.arm_tx) {
            (Some(rt), _) => rt.arm(staging),
            (None, Some(arm_tx)) => {
                if let Err(ruviform_rt::TryPushError::Full(arm)) = arm_tx.try_push(staging) {
                    // Latest-wins: the ring this displaced is
                    // superseded before the endpoint could see it.
                    self.pending_arm = Some(arm);
                }
            }
            (None, None) => {}
        }
    }

    /// The instance-scope gate + child-side request — the wire-v2
    /// envelope `instance` is derived from the verb scope: `Scan` is
    /// child-scoped (`WIRE_INSTANCE_NONE`); everything else is
    /// stamped with this instance's minted handle — the same value
    /// the reader's waiter attribution checks the echo against. The
    /// phase gate lives inside [`L1Child::request`] (atomic with
    /// the waiter insert).
    fn request(&mut self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        let instance = match cmd {
            wire::Command::Scan => wire::WIRE_INSTANCE_NONE,
            _ => self.instance,
        };
        self.child.request(instance, cmd)
    }

    /// `Destroy`'s control-plane meaning: wire v2's teardown is
    /// `DestroyInstance` (reclaims the child's registry slot) then —
    /// when this facade is the child's last referrer — `Shutdown`,
    /// reap and join (the shutdown-on-empty + last-reference rules).
    /// A `Failed` *child* is already killed/converged; a `Failed`
    /// *instance* still sends `DestroyInstance` (the driver's gate
    /// exempts it — reclaiming the dead entry's runner slot is its
    /// job), then the entry leaves `Inner.instances` so its
    /// `live_count` frees. Always succeeds and is idempotent — the
    /// defined rejection is `Destroy → Ok` for live, `Failed` and
    /// already-destroyed instances alike, so there is no `Err` to
    /// return. This primitive is unconditional: rejecting a
    /// `HostCommand::Destroy` from `Activated`/`Processing` (the L0
    /// legality rule) is `crate::composite::PluginHost`'s gate — it
    /// mirrors `lifecycle_state()` through `LifecycleEvent::Destroy`
    /// before calling this — not this layer's.
    ///
    /// **Last-reference teardown:** `Arc::strong_count(&self.child)
    /// == 1` means no live siblings — only then does `Shutdown`+
    /// grace+reap+join run (a live sibling's child must never be
    /// killed). Under the dedicated route the count is always 1.
    ///
    /// **RT contract:** non-RT only — bounded by `command_timeout`
    /// of shutdown grace plus the kill guarantee.
    pub(crate) fn destroy(&mut self) {
        // Teardown drains like every non-RT `&mut` entry point —
        // bounded `try_*` ops only, never a wait on endpoint
        // liveness. A live endpoint's leg simply lands in the
        // return channel after this point and is dropped when the
        // channel's last consumer goes away.
        self.drain_endpoint_channels();
        let (child_terminal, registered) = {
            let inner = self.shared.lock();
            (
                inner.terminal.is_some(),
                inner.instances.contains_key(&self.instance),
            )
        };
        if registered {
            // The registry slot is reclaimed while the child can
            // still answer — teardown is unconditional, so a dead
            // or wedged reply path (`Err(Ipc)`/timeout → child
            // convergence) still proceeds to removal below. A
            // terminal child's request no-ops to `Err`, so the
            // request is skipped there. `request` also no-ops to
            // `Err(Ipc)` on a `test_fixture` (no driver channel)
            // — harmless.
            if !child_terminal {
                let _ = self.request(wire::Command::DestroyInstance);
            }
            // `DestroyInstance`-reply-granular removal — run here
            // whether or not the reply landed: residual `(i,·)`
            // doorbells get their synthesized `mark_failed`, later
            // frames demux to the inert post-removal row.
            let mut inner = self.shared.lock();
            remove_instance(&mut inner, self.instance);
            drop(inner);
            self.child.send_driver(DriverCmd::RemoveInstance {
                instance: self.instance,
            });
        }
        // The mirror outlives the entry — `lifecycle_state` keeps
        // answering `Destroyed` post-removal through the facade's
        // own `rt_state` copy (the `Release`/`Acquire` pair the
        // `InstShared` writes used).
        self.rt_state.store(
            data::mirror_encode(LifecycleState::Destroyed),
            Ordering::Release,
        );
        if Arc::strong_count(&self.child) == 1 {
            // The last referrer tears the child down — `Shutdown`+
            // grace while the child still looks live, kill path
            // when terminal/wedged. Sibling referrers would keep
            // `strong_count > 1` and skip this entirely.
            self.child.teardown();
        }
    }

    /// Kills the child process — the `#[doc(hidden)] kill_child`
    /// contract's backing operation, used to drive the crash paths in
    /// tests. The driver's `try_wait` (or stdout EOF) performs the
    /// `Failed` convergence asynchronously.
    ///
    /// **RT contract:** non-RT only — the kill goes through the
    /// shared handle; the driver's poll observes the death.
    pub(crate) fn kill(&self) {
        kill_child(&self.shared);
    }

    /// The local lifecycle mirror — `Created` through `Processing`
    /// track successful commands, `Failed`/`Destroyed` are terminal
    /// conditions. Reads the facade's lock-free `rt_state` snapshot —
    /// the same value `InstShared::set_mirror` publishes, still
    /// answering after the entry itself is removed.
    ///
    /// **RT contract:** non-RT only by call site — a single atomic
    /// load, technically non-blocking.
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        data::mirror_decode(self.rt_state.load(Ordering::Acquire))
    }

    /// The `process` equivalent — the RT leg of the data plane.
    /// Stages the input into `staging_in[seq % D]`, submits the
    /// doorbell seq on the `submit` channel (`try_push` only), drains
    /// `result` for the `seq - lookahead_blocks` completion, copies
    /// the matching output block out, and falls back to the defined
    /// pass-through (`last_output` or silence) on every non-match.
    /// A `Failed` instance still returns `Ok(CONTINUE)` with silence
    /// — the crash is a control-plane fact, never an RT error.
    ///
    /// **RT contract:** RT-suitable — atomic staging writes and
    /// `try_push`/`try_pop` only; no locks, no pipes, no shm, no
    /// allocation (the `rt_l1_*` contract tests assert this). The
    /// endpoint-channel drain that runs here is the wait-free half
    /// only — resend + leg restore — and it runs no destructors at
    /// all: a restored leg keeps its stale per-attach channel ends
    /// until the next non-RT drain (`clear_endpoint_fields` drops
    /// the rtrb rings and runs `Arc<Staging>` destructors), and
    /// there is no `DropCollector::drain` either.
    pub(crate) fn process(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        // The wait-free drain half runs ahead of the detached
        // early-return so a returned leg is always collected — it
        // runs no destructors at all (the restore leaves the stale
        // channel ends for the non-RT drain, and the
        // `arm_collector` destructor drain stays out too).
        self.drain_endpoint_channels_wait_free();
        let Some(rt) = self.rt.as_mut() else {
            // The leg is checked out to an endpoint — the defined
            // `EndpointUnavailable` (not `Ipc`): the processing
            // surface moved; this is not a wire failure.
            return Err(HostError::EndpointUnavailable);
        };
        let mirror = data::mirror_decode(self.rt_state.load(Ordering::Acquire));
        rt.process(mirror, inputs, outputs, events, transport)
    }

    /// The attach-side lifecycle gate, shared by the public prelude
    /// ([`PluginHost::prepare_endpoint`](crate::PluginHost::prepare_endpoint)'s
    /// L1 arm runs it for its own state step — the design's
    /// rejection order puts state before every other check) and
    /// [`L1Instance::prepare_endpoint`] itself.
    ///
    /// `rt_state`'s decode — the control plane's committed state
    /// — decides: `Activated`/`Processing` pass; `Failed` is
    /// `Err(HostError::Failed)`; anything below `Activated` is
    /// `Err(HostError::Lifecycle)`, the same shape a rejected
    /// `Process` event takes. No convergence runs here — the mirror
    /// is already the committed authority on this side.
    ///
    /// **RT contract:** non-RT only by call site — a single atomic
    /// load.
    ///
    /// # Errors
    ///
    /// [`HostError::Failed`] for a `Failed` instance and
    /// [`HostError::Lifecycle`] below `Activated`.
    pub(crate) fn check_attachable_state(&self) -> Result<(), HostError> {
        match data::mirror_decode(self.rt_state.load(Ordering::Acquire)) {
            LifecycleState::Activated | LifecycleState::Processing => Ok(()),
            LifecycleState::Failed => Err(HostError::Failed),
            state => Err(HostError::Lifecycle(
                ruviform_plugin_core::LifecycleError::IllegalTransition {
                    from: state,
                    event: ruviform_plugin_core::LifecycleEvent::Process,
                },
            )),
        }
    }

    /// Whether the RT leg is checked out to a detached endpoint —
    /// `rt.is_none()`, the single-endpoint rule's authoritative
    /// occupancy fact (the leg returns only through a `return_rx`
    /// drain). The composite's attach prelude reads it so an
    /// occupied instance reports `EndpointUnavailable` before any
    /// bounds check.
    ///
    /// **RT contract:** non-RT only by call site — a field read.
    pub(crate) fn leg_detached(&self) -> bool {
        self.rt.is_none()
    }

    /// Checks the RT leg out for a detached endpoint — the L1 half
    /// of `attach_endpoint` the composite wires (Issue #113 task 3 —
    /// [`PluginHost::prepare_endpoint`](crate::PluginHost::prepare_endpoint)
    /// reaches it through the composite's `attach_endpoint` after
    /// the public prelude).
    ///
    /// Defined rejections, in the pinned order:
    /// 1. **lifecycle/state** — `Failed` → `Err(HostError::Failed)`;
    ///    anything before `Activated` → `Err(Lifecycle)` (an
    ///    endpoint exists only for a processing surface). A second
    ///    attach while the leg is already checked out →
    ///    `Err(EndpointUnavailable)` (the single-endpoint rule —
    ///    `rt.is_none()` is authoritative).
    /// 2. **bounds, before the take** — `bound_max_frames`/
    ///    `bound_channels` must be non-zero and within
    ///    [`wire::WIRE_MAX_FRAMES`]/[`wire::WIRE_MAX_CHANNELS`], and
    ///    `bound_max_frames` must cover the currently armed ring —
    ///    `Err(Prepare(UnsupportedConfig))`. Nothing moves.
    /// 3. **leg extraction** — `self.rt.take()`. From here every
    ///    fallible step would have to restore `self.rt = Some(leg)`
    ///    before returning so a rejected attach leaves the instance
    ///    attachable/processable — the helper carries that contract.
    /// 4. channel mint + seed + reserve + bookkeeping — today all
    ///    infallible; see [`L1Instance::endpoint_channel_setup`].
    ///
    /// The returned [`data::EndpointLegParts`] own only the RT leg
    /// and endpoint-side channel ends — the endpoint never sees the
    /// child, shm segment, `Shared` mutex or I/O threads.
    ///
    /// **RT contract:** non-RT only — channel minting and the
    /// `last_output` reserve allocate.
    pub(crate) fn prepare_endpoint(
        &mut self,
        bound_max_frames: u32,
        bound_channels: u32,
    ) -> Result<data::EndpointLegParts, HostError> {
        self.drain_endpoint_channels();
        // (1) lifecycle/state — before anything moves (the same
        // gate the public prelude runs; repeating it here is the
        // inner layer's defense-in-depth).
        self.check_attachable_state()?;
        if self.leg_detached() {
            return Err(HostError::EndpointUnavailable);
        }
        // (2) bounds — checked *before* the leg moves so a rejected
        // attach leaves everything exactly as found.
        let armed_frames = self
            .shared
            .lock()
            .instances
            .get(&self.instance)
            .and_then(InstShared::armed_staging)
            .map_or(0, |staging| staging.frame_capacity);
        if bound_max_frames == 0
            || bound_channels == 0
            || bound_max_frames > wire::WIRE_MAX_FRAMES
            || bound_channels > wire::WIRE_MAX_CHANNELS
            || (bound_max_frames as usize) < armed_frames
        {
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        // (3) leg extraction — post-take `Err` paths restore it.
        // Today the helper is infallible, but the contract stands:
        // any future fallible step must restore `self.rt =
        // Some(leg)` and clear the per-attach fields before
        // returning so a rejected attach leaves the instance
        // attachable/processable.
        let leg = self.rt.take().expect("presence checked above");
        Ok(self.endpoint_channel_setup(leg, bound_max_frames, bound_channels))
    }

    /// The post-take half of [`L1Instance::prepare_endpoint`] —
    /// steps (4)–(8) of the pinned order:
    /// 4. mint the per-attach channels: arm capacity 2, arm retirer
    ///    capacity 4, leg-return capacity 1;
    /// 5. seed arm — the current `Inner::staging` clone. The
    ///    endpoint applies it only when its generation is newer
    ///    than the leg's armed ring (the detached-`Activate`
    ///    self-repair); the same-generation normal path is a
    ///    normalized no-op retired unapplied;
    /// 6. `last_output` pre-reserve to `bound.max_frames ×
    ///    WIRE_MAX_CHANNELS` so `RtLeg::arm_apply`'s `resize` always
    ///    fits inside capacity and never reallocs on the RT path;
    /// 7. bookkeeping — the per-attach fields a returned leg's
    ///    restore (`drain_endpoint_channels`) resets;
    /// 8. assemble [`data::EndpointLegParts`] — the leg plus the
    ///    endpoint-side channel ends and mirror — for
    ///    `crate::endpoint::L1Endpoint`.
    ///
    /// Infallible today: channel minting, the seed push, the
    /// reserve and bookkeeping cannot fail — see the caller for
    /// the post-take restore contract any future `Err` must honor.
    fn endpoint_channel_setup(
        &mut self,
        mut leg: data::RtLeg,
        bound_max_frames: u32,
        bound_channels: u32,
    ) -> data::EndpointLegParts {
        let (mut arm_tx, arm_rx) = ruviform_rt::channel::<Arc<data::Staging>>(2);
        let (arm_retirer, arm_collector) = ruviform_rt::retire_channel::<Arc<data::Staging>>(4);
        let (return_tx, return_rx) = ruviform_rt::channel::<data::RtLeg>(1);
        // (5) seed arm — the entry's `staging` clone, same
        // generation as the leg's armed ring in the normal path (a
        // normalized no-op for the endpoint drain), legitimately
        // newer after a detached `Activate`.
        let seed = self
            .shared
            .lock()
            .instances
            .get(&self.instance)
            .and_then(InstShared::armed_staging);
        if let Some(staging) = seed {
            let _ = arm_tx.try_push(staging);
        }
        // (6) the `last_output` reserve — see the method docs.
        leg.reserve_last_output(bound_max_frames as usize * wire::WIRE_MAX_CHANNELS as usize);
        // (7) bookkeeping.
        self.arm_tx = Some(arm_tx);
        self.pending_arm = None;
        self.return_rx = Some(return_rx);
        self.arm_collector = Some(arm_collector);
        self.endpoint_bound = Some((bound_max_frames, bound_channels));
        // (8) the parts `crate::endpoint` assembles `L1Endpoint` from.
        data::EndpointLegParts {
            leg,
            mirror: Arc::clone(&self.rt_state),
            arm_rx,
            arm_retirer,
            return_tx,
        }
    }

    /// The per-attach channel drain every mutable/non-RT entry
    /// point runs first: retry `pending_arm` delivery, collect a
    /// returned leg (restoring `rt`), drain the arm `DropCollector`,
    /// then finish a restore — re-arm the leg onto a newer
    /// `Inner::staging` the detached endpoint never consumed and
    /// clear the stale per-attach bookkeeping. All bounded `try_*`
    /// ops: it never waits on endpoint liveness.
    ///
    /// The finishing step lives **only** here, never on the RT
    /// path: `clear_endpoint_fields` drops the last ends of the
    /// rtrb rings (freeing their allocations) and retiring the
    /// collector drops `Arc<Staging>`s — destructor work the
    /// wait-free half in [`L1Instance::process`] must not run.
    /// The wait-free restore therefore leaves the stale ends in
    /// place (bounded and inert); the leg keeps its old ring
    /// until this pass runs, so blocks processed in that window
    /// submit under the stale generation — the driver drops them
    /// (`armed_for`) and the leg passes through — then this pass
    /// re-arms and the divergence self-heals. The clear detector
    /// is `rt.is_some() && return_rx.is_some()`: while an attach
    /// is live `rt` is `None`; once the leg is home the stale
    /// ends are garbage either way.
    pub(crate) fn drain_endpoint_channels(&mut self) {
        self.drain_endpoint_channels_wait_free();
        if let Some(collector) = &mut self.arm_collector {
            let _ = collector.drain();
        }
        if self.rt.is_some() && self.return_rx.is_some() {
            // A leg restored above — or by an earlier wait-free
            // pass inside `process` — may carry a ring the
            // detached `Activate` already superseded: re-arm it
            // onto the entry's current `staging` or every later
            // submit fails `armed_for` and the instance passes
            // through forever (with `latency_frames` diverging
            // too). `RtLeg::arm`'s `last_output` resize fits the
            // attach-time `reserve_last_output` (the detached
            // `Activate` bound gate caps the ring at
            // `bound.max_frames`), so it never reallocs here.
            let staging = self
                .shared
                .lock()
                .instances
                .get(&self.instance)
                .and_then(InstShared::armed_staging);
            if let (Some(rt), Some(staging)) = (&mut self.rt, staging)
                && staging.generation > rt.armed_generation()
            {
                rt.arm(staging);
            }
            self.clear_endpoint_fields();
        }
    }

    /// The wait-free subset of the drain — `try_push` resend +
    /// `try_pop` leg restore only. `process` runs exactly this;
    /// it must run **no destructors at all** — the restore puts
    /// `self.rt` back but leaves the stale per-attach ends in
    /// place (freeing the rtrb rings and dropping stale
    /// `Arc<Staging>`s on the RT thread would violate the
    /// contract). [`L1Instance::drain_endpoint_channels`] adds
    /// the collector drain, the re-arm and the field clear for
    /// every other mutable entry point.
    ///
    /// The restore is gated on a non-terminal lifecycle mirror
    /// (read from the same lock-free `rt_state` `process`
    /// inspects): a leg returned after `Destroy` stays queued and
    /// is dropped with the channel ends at teardown — bounded —
    /// rather than resurrecting a `Destroyed` instance.
    fn drain_endpoint_channels_wait_free(&mut self) {
        if let Some(arm_tx) = &mut self.arm_tx
            && let Some(arm) = self.pending_arm.take()
            && let Err(ruviform_rt::TryPushError::Full(arm)) = arm_tx.try_push(arm)
        {
            // Still full — the newest arm stays owed; the next
            // mutable entry point retries the send.
            self.pending_arm = Some(arm);
        }
        let terminal = data::mirror_decode(self.rt_state.load(Ordering::Acquire)).is_destroyed();
        if !terminal
            && let Some(return_rx) = &mut self.return_rx
            && let Ok(leg) = return_rx.try_pop()
        {
            // Restore only — `clear_endpoint_fields` stays on the
            // non-RT drain (the stale ends' teardown is destructor
            // work).
            self.rt = Some(leg);
        }
    }

    /// Clears every per-attach field — **non-RT only**, called by
    /// [`L1Instance::drain_endpoint_channels`] once a returned leg
    /// is home (the attach is gone — the drops below free the rtrb
    /// rings and the collector's queued `Arc<Staging>`s, all
    /// destructor work kept off the RT path) and by the post-take
    /// `Err` restore in [`L1Instance::prepare_endpoint`]. Stale
    /// producers/consumers/collector/bound bookkeeping must not
    /// survive into the next attach, which mints fresh channels.
    /// Teardown drops the fields with the struct the same way —
    /// a queued unreclaimed leg is dropped with the channel ends.
    fn clear_endpoint_fields(&mut self) {
        self.arm_tx = None;
        self.pending_arm = None;
        self.return_rx = None;
        self.arm_collector = None;
        self.endpoint_bound = None;
    }

    /// Per-block processing latency the instance's armed ring adds
    /// — `(depth − 1) × armed frame_capacity`, the result horizon
    /// the composite reports through `PluginEndpoint` — `None`
    /// before `Activate` **and once the mirror leaves
    /// `Activated`/`Processing`**: the armed ring is never cleared,
    /// so a bare `staging` read would keep reporting a stale
    /// horizon for a `Deactivate`d/`Failed` pipeline that no longer
    /// runs (parity with the composite's L0 lifecycle gate). A
    /// `&self` observation point: deliberately **non-draining**
    /// (the signature cannot take `&mut self`).
    ///
    /// **RT contract:** non-RT only — takes the shared mutex.
    pub(crate) fn latency_frames(&self) -> Option<u32> {
        match data::mirror_decode(self.rt_state.load(Ordering::Acquire)) {
            LifecycleState::Activated | LifecycleState::Processing => {}
            _ => return None,
        }
        self.armed_latency_frames()
    }

    /// Freezes the child's live capture into a registerable record —
    /// the composite's notification path mints the public record for
    /// a minted failure (the caller stamps `handle` and `cause`; the
    /// child's capture keeps mutating afterwards).
    ///
    /// **RT contract:** non-RT only — takes the capture mutex and
    /// clones the retained stderr tail.
    pub(crate) fn diagnostics_pending(&self) -> diag::PendingRecord {
        self.shared.diag.freeze(None)
    }

    /// The armed ring's latency figure with no lifecycle gate —
    /// `Some` while a ring is armed, `None` before `Activate` or
    /// once the entry is removed. The composite's attach path reads
    /// it for endpoint bookkeeping: the endpoint is minted carrying
    /// the armed horizon its leg still holds — a mid-attach mirror
    /// flip (convergence) does not disarm the ring, so this
    /// internal read must not gate on the mirror the way the public
    /// query does.
    ///
    /// **RT contract:** non-RT only — takes the shared mutex.
    pub(crate) fn armed_latency_frames(&self) -> Option<u32> {
        let inner = self.shared.lock();
        inner
            .instances
            .get(&self.instance)
            .and_then(InstShared::armed_staging)
            .map(|staging| {
                u32::try_from((self.depth - 1) as usize * staging.frame_capacity)
                    .unwrap_or(u32::MAX)
            })
    }

    /// The sample rate of the last successful `Activate` — the
    /// composite uses it to size the endpoint input scratch.
    /// `Some` exactly when a staging ring is armed.
    pub(crate) fn activated_sample_rate(&self) -> Option<u32> {
        self.activated_sample_rate
    }

    /// The arm `DropCollector`'s cumulative overflow count — the
    /// leak-on-full saturation signal of the arm retire channel
    /// (a `Retirer` forgets the value on a full channel). `0` is
    /// the healthy steady state; any non-zero count means the
    /// endpoint retires arms faster than the non-RT drains run —
    /// the same observation point `PlanSwap`'s `retired_overflows`
    /// exposes for plan retirement.
    ///
    /// A `&self` observation point like [`L1Instance::latency_frames`]:
    /// deliberately **non-draining** — an atomic load only.
    ///
    /// **RT contract:** non-RT only by convention — an atomic load,
    /// technically non-blocking.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn arm_retire_overflows(&self) -> usize {
        self.arm_collector
            .as_ref()
            .map_or(0, ruviform_rt::DropCollector::overflows)
    }

    /// Pops the queued `Failed` notification — `Some` exactly once
    /// per convergence, `None` on clean teardown or before failure.
    /// The payload pairs the observed [`ChildExit`] (status-unknown
    /// fields stay `None` until the reap supplies them) with the
    /// first-observed [`FailureCause`].
    ///
    /// **RT contract:** non-RT only — `try_recv`, never blocks.
    pub(crate) fn try_recv_notification(&self) -> Option<FailedNotice> {
        self.notify_rx.try_recv().ok()
    }

    /// The audio instance's counter readers — `None` once the
    /// registry entry is gone (`Destroy`d or converged). The values
    /// survive leg swaps, endpoint checkout and `arm`; a re-minted
    /// instance reads fresh pairs starting at zero.
    ///
    /// **RT contract:** non-RT only — takes the shared mutex.
    pub(crate) fn audio_counters(&self) -> Option<AudioCounterReaders> {
        let inner = self.shared.lock();
        inner
            .instances
            .get(&self.instance)
            .and_then(InstShared::audio_counter_readers)
    }
}

impl Drop for L1Instance {
    /// Dropping without `Destroy` runs the same unconditional path —
    /// `DestroyInstance` slot reclaim (bounded request), entry
    /// removal, then the last-reference teardown when no sibling
    /// keeps the child (`Arc::strong_count == 1`): `Shutdown`+grace
    /// or the driver's `Disconnected` kill/reap. Bounded by the
    /// driver's kill guarantee — the endpoint-channel drain is the
    /// same bounded `try_*` subset, never a wait on endpoint
    /// liveness.
    fn drop(&mut self) {
        if !self.lifecycle_state().is_destroyed() {
            self.destroy();
        }
    }
}

/// The lifecycle state a successful command leaves the instance in —
/// `None` for commands that do not transition (`Scan`, state verbs,
/// GUI). `Deactivate` and `Initialize` share the `Initialized`
/// destination; `Activate` and `StopProcessing` share `Activated`.
fn post_command_state(cmd: &wire::Command) -> Option<LifecycleState> {
    match cmd {
        wire::Command::Initialize
        | wire::Command::Deactivate
        | wire::Command::VideoInitialize
        | wire::Command::VideoDeactivate => Some(LifecycleState::Initialized),
        wire::Command::Activate { .. }
        | wire::Command::StopProcessing
        | wire::Command::VideoActivate
        | wire::Command::VideoStopProcessing => Some(LifecycleState::Activated),
        wire::Command::StartProcessing | wire::Command::VideoStartProcessing => {
            Some(LifecycleState::Processing)
        }
        _ => None,
    }
}

/// The lifecycle state an `Err` reply still leaves the instance in —
/// `Some` only for the commit-then-advisory verbs in the runner's
/// dispatch: `stop_processing()`/`deactivate()` apply their
/// transition *before* running the hook (`ruviform_plugin_export`'s
/// exact ordering, matching L0's `InProcessHost`), so a hook/plugin
/// `Err` arrives after the transition and the child stays in the
/// post-state. `Err(Lifecycle)` is always the pre-commit legality
/// rejection — the caller filters it, not this table.
///
/// The full-verb audit behind `Some`/`None`: `Initialize` and
/// `Activate` either reject pre-commit (`Err(Lifecycle)`) or take
/// the `InitFailed`/`ActivateFailed` loop back to the pre-command
/// state; `StartProcessing` commits, then rolls a hook failure back
/// to `Activated` — likewise its pre-command state; `SaveState`,
/// `RestoreState` and the GUI verbs carry no lifecycle transition
/// at all. Every other `Err` therefore leaves the child — and the
/// mirror — in the pre-command state.
///
/// The video mapping follows the same rule the design pins for the
/// runner (`lifecycle.transition()` runs before the hook —
/// `stop_processing`'s commit-then-advisory ordering, and
/// `deactivate()`'s fallible-but-committed transition):
/// `VideoStopProcessing` commits `Activated` and
/// `VideoDeactivate` commits `Initialized` even on a hook `Err`,
/// while `VideoInitialize`/`VideoActivate`/`VideoStartProcessing`
/// `Err`s loop or roll back to the pre-command state and
/// `VideoRender`/param verbs carry no transition at all.
fn err_committed_state(cmd: &wire::Command) -> Option<LifecycleState> {
    match cmd {
        wire::Command::StopProcessing | wire::Command::VideoStopProcessing => {
            Some(LifecycleState::Activated)
        }
        wire::Command::VideoDeactivate => Some(LifecycleState::Initialized),
        _ => None,
    }
}

#[cfg(test)]
impl L1Child {
    /// Process-free child fixture for the pool tests — a real
    /// `Shared` state with `live_instances` registered `Live`
    /// entries (bare `InstShared`s: channel ends, no segments — no
    /// reader ever attaches), but no spawned child, I/O threads or
    /// driver channel. `request` fails `Err(Ipc)` past the gates,
    /// `pool_eligible`/`is_terminal`/`live_count` answer from the
    /// real `Inner`, and `abort`/`teardown` run their bounded
    /// pieces (the `Terminal::Clean` stamp still lands — the pool
    /// eviction tests use it as the teardown oracle).
    pub(crate) fn test_fixture(live_instances: u32) -> Arc<Self> {
        let mut instances = HashMap::new();
        for instance in 0..live_instances {
            let (notify_tx, _notify_rx) = mpsc::channel();
            let (result_tx, _result_rx) = ruviform_rt::channel::<u64>(3);
            instances.insert(
                instance,
                InstShared {
                    rt_state: Arc::new(AtomicU8::new(data::mirror_encode(LifecycleState::Created))),
                    mirror: LifecycleState::Created,
                    notify_tx,
                    notify: Notify::Idle,
                    kind: InstKind::Audio {
                        staging: None,
                        shm: SegmentInfo {
                            key: "l1-pool-fixture".to_owned(),
                            in_depth: 1,
                            out_depth: 1,
                        },
                        result_tx: Mutex::new(result_tx),
                        counters: AudioCounters::new(),
                    },
                    phase: InstPhase::Live,
                    cause: None,
                },
            );
        }
        Arc::new(Self {
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner {
                    waiters: HashMap::new(),
                    instances,
                    block_in_flight: HashSet::new(),
                    terminal: None,
                    shutdown_initiated: false,
                    seen_eof: false,
                }),
                child: OnceLock::new(),
                next_cmd_seq: AtomicU64::new(0),
                diag: Arc::new(diag::ActiveDiagnostics::new(diag::SpawnPhase::PreSpawn)),
            }),
            driver_tx: Mutex::new(None),
            driver_join: Mutex::new(None),
            reader_join: Mutex::new(None),
            command_timeout: Duration::from_millis(50),
        })
    }

    /// Stamps a `Terminal::Failed` convergence on the fixture —
    /// the "child died" input the pooled-route tests need without
    /// a real process (the kill-half of `converge_child` no-ops on
    /// the missing `Shared.child`). A fixed exit status rides
    /// along so the per-instance `ChildExit` fan-out completes
    /// `Sent` immediately instead of staying `Pending` for a reap
    /// that never comes.
    pub(crate) fn test_converge(&self) {
        converge_child(
            &self.shared,
            Some(ChildExit {
                code: Some(1),
                signal: None,
            }),
            FailureCause::ExitObserved,
        );
    }

    /// Marks the fixture mid-`Shutdown` — the pool's
    /// skip-but-not-evict predicate input.
    pub(crate) fn test_set_shutdown_initiated(&self) {
        self.shared.lock().shutdown_initiated = true;
    }

    /// Stamps `converge_instance(instance)` — the demux table's
    /// attributable per-instance failure — on a *live* child.
    /// The worker fixture has no instance-scope fault vocabulary
    /// (the shared serve loop cannot break one instance's
    /// correlation without breaking them all), so the bundle tests
    /// that need a `Failed` entry beside a live sibling drive the
    /// same function the reader's demux calls.
    // The callers live in `tests/l1_proxy_e2e.rs` behind `#[path]` —
    // the lib-test target never names it.
    #[allow(dead_code)]
    pub(crate) fn test_converge_instance(&self, instance: u32) {
        // The stamp stands in for the demux table's attributable
        // violation — `Protocol`.
        converge_instance(&self.shared, instance, FailureCause::Protocol);
    }
}

#[cfg(test)]
impl L1Instance {
    /// Child-free fixture for the endpoint-machinery tests — this
    /// module's and `crate::composite`'s. Real `Shared`, a registered
    /// `InstShared` entry, channel legs and shm segment, but no
    /// spawned child: the child's `driver_tx`/`driver_join`/
    /// `reader_join` are `None`, so `destroy`/`Drop` run the bounded
    /// teardown without touching a process and `request` fails
    /// `Err(Ipc)` past the defined rejections (the detached
    /// `Activate` gate is still exercised — it runs before the
    /// wire).
    ///
    /// `mirror` is the state the entry's `mirror`/`rt_state` start
    /// at (`Failed`/`Destroyed` mirrors register the entry as
    /// `Failed`-phase — registered-but-not-live); `activated_frames`
    /// arms a gen-1 staging ring of that capacity and records a
    /// 48 kHz activation.
    pub(crate) fn test_fixture(mirror: LifecycleState, activated_frames: Option<u32>) -> Self {
        Self::test_fixture_on(&L1Child::test_fixture(0), 0, mirror, activated_frames)
    }

    /// [`L1Instance::test_fixture`] on a caller-supplied child —
    /// `crate::composite`'s pool tests mint facades sharing one
    /// `L1Child` (co-resident siblings) the same way the pooled
    /// route's `create_instance` + `from_parts` pair does. The
    /// entry registers on `child.shared` under wire handle
    /// `instance`; `mirror`/`activated_frames` mean exactly what
    /// they do on `test_fixture`.
    pub(crate) fn test_fixture_on(
        child: &Arc<L1Child>,
        instance: u32,
        mirror: LifecycleState,
        activated_frames: Option<u32>,
    ) -> Self {
        const DEPTH: u32 = 3;
        let (submit_tx, _submit_rx) = ruviform_rt::channel::<data::Submit>(DEPTH as usize);
        let (result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let (notify_tx, notify_rx) = mpsc::channel();
        let staging = activated_frames.map(|frames| Arc::new(data::Staging::new(DEPTH, frames, 1)));
        let mut leg = data::RtLeg::new(
            DEPTH,
            DEPTH - 1,
            submit_tx,
            result_rx,
            data::RtLegCounters::fresh(),
        );
        if let Some(staging) = &staging {
            leg.arm(Arc::clone(staging));
        }
        let rt_state = Arc::new(AtomicU8::new(data::mirror_encode(mirror)));
        let segment = shm::Segment::create(
            u32::try_from(INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap_or(u32::MAX),
            DEPTH,
            DEPTH,
        )
        .expect("fixture segment");
        child.shared.lock().instances.insert(
            instance,
            InstShared {
                rt_state: Arc::clone(&rt_state),
                mirror,
                notify_tx,
                notify: Notify::Idle,
                kind: InstKind::Audio {
                    staging,
                    shm: SegmentInfo {
                        key: segment.key().to_owned(),
                        in_depth: DEPTH,
                        out_depth: DEPTH,
                    },
                    result_tx: Mutex::new(result_tx),
                    counters: AudioCounters::new(),
                },
                phase: if mirror.is_failed() || mirror.is_destroyed() {
                    InstPhase::Failed
                } else {
                    InstPhase::Live
                },
                cause: None,
            },
        );
        Self {
            shared: Arc::clone(&child.shared),
            child: Arc::clone(child),
            instance,
            notify_rx,
            rt_state,
            depth: DEPTH,
            rt: Some(leg),
            staging_generation: u64::from(activated_frames.is_some()),
            arm_tx: None,
            pending_arm: None,
            return_rx: None,
            arm_collector: None,
            endpoint_bound: None,
            activated_sample_rate: activated_frames.map(|_| 48_000),
            segment,
        }
    }

    /// The minted wire handle — the bundle tests' lowest-free-mint
    /// and per-instance-attribution oracle.
    // The callers live in `tests/l1_proxy_e2e.rs` behind `#[path]` —
    // the lib-test target never names it.
    #[allow(dead_code)]
    pub(crate) fn test_wire_instance(&self) -> u32 {
        self.instance
    }
}

#[cfg(test)]
mod tests {
    //! `L1Instance` endpoint-machinery tests — leg extraction and
    //! the pinned rejection order, the seed arm, `pending_arm`
    //! latest-wins, leg return + per-attach cleanup, the detached
    //! `process`/`Activate` surfaces and the `&self` non-drain
    //! observation points.

    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};

    fn config(min_frames: u32, max_frames: u32) -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), min_frames, max_frames).unwrap()
    }

    /// One `L1Instance::process` block over mono planar views — the
    /// erased `HostInput`/`HostOutput` surface.
    fn run_block(instance: &mut L1Instance) -> Result<ProcessResult, HostError> {
        let input = [0.5_f32; 8];
        let mut output = [0.0_f32; 8];
        let in_list = [&input[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut output[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        instance.process(&inputs, &mut outputs, EventList::empty(8), None)
    }

    /// Leg extraction: `prepare_endpoint` takes `rt` (the
    /// authoritative attached fact), mints the per-attach channels,
    /// seeds the arm channel with the armed `Inner::staging` clone
    /// and clones the shared mirror — a second attach hits the
    /// single-endpoint `EndpointUnavailable`.
    #[test]
    fn prepare_endpoint_takes_leg_and_blocks_reattach() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        assert!(instance.rt.is_none());
        assert!(instance.arm_tx.is_some());
        assert!(instance.return_rx.is_some());
        assert!(instance.arm_collector.is_some());
        assert_eq!(instance.endpoint_bound, Some((512, 2)));
        // The seed arm is the armed ring's clone — same generation
        // in the normal path (the endpoint retires it unapplied).
        let seed = parts.arm_rx.try_pop().expect("seeded arm");
        assert_eq!(seed.generation, 1);
        assert!(parts.arm_rx.try_pop().is_err());
        // The mirror is the *same* atomic `rt_state` publishes.
        assert!(Arc::ptr_eq(&parts.mirror, &instance.rt_state));
        // Single-endpoint rule — `rt.is_none()` is authoritative.
        assert!(matches!(
            instance.prepare_endpoint(512, 2),
            Err(HostError::EndpointUnavailable)
        ));
    }

    /// The pinned rejection order: lifecycle/state first, then
    /// bounds **before** the take — a rejected attach leaves the
    /// leg home, so the instance stays attachable and processable.
    #[test]
    fn prepare_endpoint_rejections_leave_leg_home() {
        // State gate — `Failed` is `Err(Failed)`, below `Activated`
        // is `Err(Lifecycle)`.
        let mut failed = L1Instance::test_fixture(LifecycleState::Failed, Some(512));
        assert!(matches!(
            failed.prepare_endpoint(512, 2),
            Err(HostError::Failed)
        ));
        assert!(failed.rt.is_some());
        for state in [
            LifecycleState::Created,
            LifecycleState::Initialized,
            LifecycleState::Destroyed,
        ] {
            let mut instance = L1Instance::test_fixture(state, None);
            assert!(
                matches!(
                    instance.prepare_endpoint(512, 2),
                    Err(HostError::Lifecycle(_))
                ),
                "{state:?} must reject with Lifecycle"
            );
            assert!(instance.rt.is_some());
        }
        // Bounds — every violation is `Prepare(UnsupportedConfig)`
        // and nothing moves.
        let mut instance = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        for (frames, channels) in [
            (0, 2),
            (512, 0),
            (wire::WIRE_MAX_FRAMES + 1, 2),
            (512, wire::WIRE_MAX_CHANNELS + 1),
            // Below the armed ring's 512-frame capacity.
            (256, 2),
        ] {
            assert!(
                matches!(
                    instance.prepare_endpoint(frames, channels),
                    Err(HostError::Prepare(PrepareError::UnsupportedConfig))
                ),
                "bounds ({frames}, {channels}) must reject with Prepare"
            );
            assert!(instance.rt.is_some());
        }
        // A legal bound still attaches — nothing was consumed.
        assert!(instance.prepare_endpoint(512, 2).is_ok());
        assert!(instance.rt.is_none());
    }

    /// `process` while the leg is detached is the defined
    /// `Err(EndpointUnavailable)` — not `Ipc` — and a returned leg
    /// restores processing plus fresh attachability. The wait-free
    /// drain subset runs inside `process` itself, before the
    /// detached check — and it restores the leg *without* clearing
    /// the per-attach ends: dropping them (rtrb ring frees +
    /// `Arc<Staging>` destructors) is non-RT work the next
    /// `drain_endpoint_channels` performs.
    #[test]
    fn detached_process_returns_endpoint_unavailable_then_restores() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Processing, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        assert!(matches!(
            run_block(&mut instance),
            Err(HostError::EndpointUnavailable)
        ));
        // The `&self` observation points stay non-draining — the
        // leg sits in the return channel unread.
        assert!(parts.return_tx.try_push(parts.leg).is_ok());
        assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);
        assert!(instance.try_recv_notification().is_none());
        assert_eq!(instance.latency_frames(), Some(1024));
        assert!(instance.rt.is_none());
        // The wait-free drain inside `process` collects it — the
        // same call proceeds to the data plane, and the stale
        // per-attach ends are deliberately left in place (the
        // wait-free half runs no destructors).
        assert_eq!(run_block(&mut instance).unwrap(), ProcessResult::CONTINUE);
        assert!(instance.rt.is_some());
        assert!(instance.arm_tx.is_some());
        assert!(instance.return_rx.is_some());
        assert!(instance.arm_collector.is_some());
        assert!(instance.endpoint_bound.is_some());
        // The non-RT drain finishes the restore — every per-attach
        // field cleared, freshly attachable.
        instance.drain_endpoint_channels();
        assert!(instance.arm_tx.is_none());
        assert!(instance.return_rx.is_none());
        assert!(instance.arm_collector.is_none());
        assert!(instance.endpoint_bound.is_none());
        assert!(instance.pending_arm.is_none());
        assert_eq!(instance.arm_retire_overflows(), 0);
        assert!(instance.prepare_endpoint(512, 2).is_ok());
    }

    /// The public `latency_frames` gates on the mirror while the
    /// internal `armed_latency_frames` does not: the armed ring is
    /// never cleared, so a `Failed`/`Initialized` (post-
    /// `Deactivate`) mirror must answer `None` — L0 parity — while
    /// the attach bookkeeping read still reports the armed horizon
    /// the leg carries.
    #[test]
    fn latency_frames_gate_vs_armed_read() {
        for state in [
            LifecycleState::Initialized,
            LifecycleState::Failed,
            LifecycleState::Destroyed,
        ] {
            let instance = L1Instance::test_fixture(state, Some(512));
            assert_eq!(
                instance.latency_frames(),
                None,
                "{state:?} with an armed ring must report None"
            );
            assert_eq!(instance.armed_latency_frames(), Some(1024));
        }
        let instance = L1Instance::test_fixture(LifecycleState::Processing, Some(512));
        assert_eq!(instance.latency_frames(), Some(1024));
        // Unarmed is `None` on both reads.
        let instance = L1Instance::test_fixture(LifecycleState::Activated, None);
        assert_eq!(instance.latency_frames(), None);
        assert_eq!(instance.armed_latency_frames(), None);
    }

    /// `pending_arm` latest-wins: once the arm channel is full, a
    /// newer activation displaces the queued newest (never a merge
    /// — the endpoint only ever wants the freshest ring); the
    /// resend keeps retrying at every mutable entry point until the
    /// channel accepts it.
    #[test]
    fn pending_arm_is_latest_wins_and_retries() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        let config = config(1, 512);
        let ring = |generation| Arc::new(data::Staging::new(3, 512, generation));
        // Channel is `[seed]` — g2 fills it; g3/g4 go pending,
        // latest-wins.
        instance.arm_staging(ring(2), config);
        instance.arm_staging(ring(3), config);
        instance.arm_staging(ring(4), config);
        assert_eq!(instance.pending_arm.as_ref().unwrap().generation, 4);
        // A resend on a still-full channel keeps the pending arm.
        instance.drain_endpoint_channels();
        assert_eq!(instance.pending_arm.as_ref().unwrap().generation, 4);
        // Pop the seed → resend lands g4 behind g2.
        assert_eq!(parts.arm_rx.try_pop().unwrap().generation, 1);
        instance.drain_endpoint_channels();
        assert!(instance.pending_arm.is_none());
        assert_eq!(parts.arm_rx.try_pop().unwrap().generation, 2);
        assert_eq!(parts.arm_rx.try_pop().unwrap().generation, 4);
        // g3 never reached the channel — latest-wins superseded it.
        assert!(parts.arm_rx.try_pop().is_err());
    }

    /// The detached `Activate` bound gate: while the leg is out, a
    /// re-activation above the attach bound is `Err(Prepare)`
    /// before the wire; at/under the bound it proceeds (and fails
    /// `Ipc` here only because the fixture has no driver). With the
    /// leg home there is no bound gate at all.
    #[test]
    fn activate_above_attach_bound_is_rejected_while_detached() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        let _parts = instance.prepare_endpoint(512, 2).unwrap();
        assert!(matches!(
            instance.execute(wire::Command::Activate {
                config: config(1, 1024),
            }),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig))
        ));
        // Within the bound the gate passes — `request` fails `Ipc`
        // (the fixture has no driver), proving the gate ran first.
        assert!(matches!(
            instance.execute(wire::Command::Activate {
                config: config(1, 512),
            }),
            Err(HostError::Ipc)
        ));
        // Leg home → no gate: the same over-sized config reaches
        // `request` untouched by `endpoint_bound`.
        let mut home = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        assert!(matches!(
            home.execute(wire::Command::Activate {
                config: config(1, 1024),
            }),
            Err(HostError::Ipc)
        ));
    }

    /// `arm_staging` while detached pushes through the arm channel
    /// (not `RtLeg::arm`); when the endpoint is dropped before
    /// consuming it, the restored leg still carries the superseded
    /// ring — the non-RT drain re-arms it onto the newer
    /// `Inner::staging` (F2: left stale, every later submit would
    /// fail the driver's `armed_for` gate and the instance would
    /// pass through forever). A re-attach then seeds the *same*
    /// generation — the normalized no-op.
    #[test]
    fn restored_leg_rearms_to_the_newer_staging() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Activated, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        instance.arm_staging(Arc::new(data::Staging::new(3, 512, 2)), config(1, 512));
        // The g2 arm queued behind the seed; the leg is still gen 1.
        assert!(parts.return_tx.try_push(parts.leg).is_ok());
        instance.drain_endpoint_channels();
        // The restored leg is re-armed to `Inner::staging` (gen 2)
        // — the endpoint never consumed the queued arm.
        assert_eq!(instance.rt.as_ref().unwrap().armed_generation(), 2);
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        assert_eq!(parts.leg.armed_generation(), 2);
        // The seed is now the same-generation normalized no-op.
        assert_eq!(parts.arm_rx.try_pop().unwrap().generation, 2);
        assert!(parts.arm_rx.try_pop().is_err());
    }

    /// A leg returned after `Destroy` is never restored onto the
    /// terminal instance — the drain's lifecycle gate leaves it
    /// queued and the channel ends die with the instance (bounded
    /// teardown). An endpoint dropped earlier — while the instance
    /// was still live — still restores.
    #[test]
    fn leg_returned_after_destroy_is_not_restored() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Processing, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        instance.destroy();
        assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);
        assert!(parts.return_tx.try_push(parts.leg).is_ok());
        // The `Destroyed` gate refuses the restore — the leg stays
        // queued in `return_rx` and drops with the channel ends.
        instance.drain_endpoint_channels();
        assert!(instance.rt.is_none());
        drop(instance);
    }

    /// A submit the driver drops without a doorbell (stale
    /// generation — or a terminal/`Destroying`/dead entry) still
    /// owes the leg exactly one completion: the synthesized
    /// `mark_failed` result `push_result` delivers. Without it the
    /// slot's `cleared[i] >= last_pushed[i]` gate wedges
    /// permanently — every later seq for that slot fails the gate.
    /// This pins the leg-side half: a pushed synthesized
    /// completion frees the slot through the normal `drain_to`.
    #[test]
    fn dropped_submit_result_frees_alternation_slot() {
        const DEPTH: u32 = 3;
        const INSTANCE: u32 = 0;
        // Live channel ends so `try_push` lands and `last_pushed`
        // records each seq — the wedge is real only while the
        // submit actually pushed.
        let (submit_tx, mut submit_rx) = ruviform_rt::channel::<data::Submit>(DEPTH as usize);
        let (result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let (inst, _notify_rx, _) = inst_shared(LifecycleState::Processing);
        let mut inst = inst;
        let InstKind::Audio {
            result_tx: inst_result_tx,
            staging: inst_staging,
            ..
        } = &mut inst.kind
        else {
            panic!("inst_shared mints an Audio entry");
        };
        *inst_result_tx = Mutex::new(result_tx);
        *inst_staging = Some(Arc::new(data::Staging::new(DEPTH, 512, 1)));
        let mut instances = HashMap::new();
        instances.insert(INSTANCE, inst);
        let shared = Shared {
            inner: Mutex::new(Inner {
                waiters: HashMap::new(),
                instances,
                block_in_flight: HashSet::new(),
                terminal: None,
                shutdown_initiated: false,
                seen_eof: false,
            }),
            child: OnceLock::new(),
            next_cmd_seq: AtomicU64::new(0),
            diag: Arc::new(diag::ActiveDiagnostics::new(diag::SpawnPhase::PreSpawn)),
        };
        let mut leg = data::RtLeg::new(
            DEPTH,
            DEPTH - 1,
            submit_tx,
            result_rx,
            data::RtLegCounters::fresh(),
        );
        leg.arm(Arc::new(data::Staging::new(DEPTH, 512, 1)));
        // Fill every slot's gate: three submits, no completions —
        // the wedge the F3 fix unwedges.
        for _ in 0..DEPTH {
            run_leg(&mut leg).unwrap();
        }
        for block in 0..u64::from(DEPTH) {
            assert_eq!(
                submit_rx.try_pop().unwrap().seq,
                data::BLOCK_SEQ_FLAG | block
            );
        }
        // Every slot busy → this call submits nothing (and passes
        // through — an unanswered seq never errored, it fell back).
        run_leg(&mut leg).unwrap();
        assert!(submit_rx.try_pop().is_err());
        // The driver-side drop synthesizes one completion per
        // dropped seq — one per slot here.
        for block in 0..u64::from(DEPTH) {
            shared.push_result(INSTANCE, data::mark_failed(data::BLOCK_SEQ_FLAG | block));
        }
        // Block 4's own submit still gates (its slot check runs
        // before the drain frees it) but its `drain_to` consumes
        // all three completions — `cleared` catches up. Block 5
        // then resubmits on its slot (`cleared[i] ==
        // last_pushed[i]`): `FLAG | 5`. Without the synthesized
        // results every slot stays wedged and this `try_pop`
        // never yields again.
        run_leg(&mut leg).unwrap();
        assert!(submit_rx.try_pop().is_err());
        run_leg(&mut leg).unwrap();
        assert_eq!(submit_rx.try_pop().unwrap().seq, data::BLOCK_SEQ_FLAG | 5);
    }

    /// One `data::RtLeg::process` block over mono planar views —
    /// the leg-level sibling of [`run_block`] for the
    /// channel-level pin above.
    fn run_leg(leg: &mut data::RtLeg) -> Result<ProcessResult, HostError> {
        let input = [0.5_f32; 8];
        let mut output = [0.0_f32; 8];
        let in_list = [&input[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut output[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        leg.process(
            LifecycleState::Processing,
            &inputs,
            &mut outputs,
            EventList::empty(8),
            None,
        )
    }

    /// `destroy` with a live endpoint never waits on it: the
    /// bounded drain runs, teardown proceeds, and the endpoint's
    /// later leg return lands on the dead channel without blocking
    /// (bounded teardown by construction).
    #[test]
    fn destroy_with_a_live_endpoint_does_not_block() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Processing, Some(512));
        let mut parts = instance.prepare_endpoint(512, 2).unwrap();
        instance.destroy();
        assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);
        drop(instance);
        // The return channel's consumer is gone — the push is a
        // bounded `try_*` either way, never a block.
        let _ = parts.return_tx.try_push(parts.leg);
    }

    /// Issue #22 lane: the detached `process` early-return — the
    /// wait-free drain subset (`try_push` resend + `try_pop`
    /// restore, no `DropCollector` destructor work) plus the
    /// `EndpointUnavailable` check — must not allocate.
    #[test]
    fn rt_l1_detached_process_no_alloc() {
        let mut instance = L1Instance::test_fixture(LifecycleState::Processing, Some(512));
        let _parts = instance.prepare_endpoint(512, 2).unwrap();
        // Warm-up outside the window.
        assert!(run_block(&mut instance).is_err());
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..8 {
                assert!(matches!(
                    run_block(&mut instance),
                    Err(HostError::EndpointUnavailable)
                ));
            }
        });
    }

    // ---- Issue #117 T2: demux + convergence machinery ---------------

    /// One registered `InstShared` entry for the demux/converge
    /// tests — `Live` phase, `mirror`/`rt_state` at `state`, real
    /// notify + result channel ends handed back for assertions.
    fn inst_shared(
        state: LifecycleState,
    ) -> (
        InstShared,
        mpsc::Receiver<FailedNotice>,
        ruviform_rt::Consumer<u64>,
    ) {
        let (notify_tx, notify_rx) = mpsc::channel();
        let (result_tx, result_rx) = ruviform_rt::channel::<u64>(3);
        let rt_state = Arc::new(AtomicU8::new(data::mirror_encode(state)));
        (
            InstShared {
                rt_state,
                mirror: state,
                notify_tx,
                notify: Notify::Idle,
                kind: InstKind::Audio {
                    staging: None,
                    shm: SegmentInfo {
                        key: "demux-test".to_owned(),
                        in_depth: 3,
                        out_depth: 3,
                    },
                    result_tx: Mutex::new(result_tx),
                    counters: AudioCounters::new(),
                },
                phase: InstPhase::Live,
                cause: None,
            },
            notify_rx,
            result_rx,
        )
    }

    /// A child-level `Shared` with `instances` registered — the
    /// demux/converge fixture; each returned receiver is the
    /// matching entry's notify queue.
    struct DemuxFixture {
        shared: Shared,
        notify_rxs: HashMap<u32, mpsc::Receiver<FailedNotice>>,
        result_rxs: HashMap<u32, ruviform_rt::Consumer<u64>>,
    }

    impl DemuxFixture {
        /// `count` live entries registered at `state` — handles
        /// `0..count`.
        fn live(count: u32, state: LifecycleState) -> Self {
            let mut instances = HashMap::new();
            let mut notify_rxs = HashMap::new();
            let mut result_rxs = HashMap::new();
            for instance in 0..count {
                let (inst, notify_rx, result_rx) = inst_shared(state);
                instances.insert(instance, inst);
                notify_rxs.insert(instance, notify_rx);
                result_rxs.insert(instance, result_rx);
            }
            Self {
                shared: Shared {
                    inner: Mutex::new(Inner {
                        waiters: HashMap::new(),
                        instances,
                        block_in_flight: HashSet::new(),
                        terminal: None,
                        shutdown_initiated: false,
                        seen_eof: false,
                    }),
                    child: OnceLock::new(),
                    next_cmd_seq: AtomicU64::new(0),
                    diag: Arc::new(diag::ActiveDiagnostics::new(diag::SpawnPhase::PreSpawn)),
                },
                notify_rxs,
                result_rxs,
            }
        }
    }

    /// Waiter-correlated `Reply` — delivered to its channel and
    /// attributed by the `instance` echo.
    #[test]
    fn demux_delivers_waiter_correlated_reply() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(7, Waiter { instance: 0, tx });
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 7,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Delivered);
        assert!(matches!(rx.try_recv(), Ok(Ok(wire::Reply::Ok))));
    }

    /// A `Reply` echoing a different `instance` than its waiter's
    /// stamp is an attribution failure — child-scope convergence,
    /// and the doomed waiter still wakes `Err(Ipc)` now rather
    /// than at timeout.
    #[test]
    fn demux_reply_instance_mismatch_is_child_scope() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(7, Waiter { instance: 0, tx });
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 7,
                instance: 1,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Child);
        assert!(matches!(rx.try_recv(), Ok(Err(HostError::Ipc))));
        assert!(matches!(
            fixture.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
    }

    /// The strict `waiter-less Reply ⇒ child-scope` rule applies
    /// only to a *live* instance — `Failed`/`Destroying`/removed
    /// targets are inert drops instead.
    #[test]
    fn demux_waiterless_reply_scope_by_instance_phase() {
        let fixture = DemuxFixture::live(3, LifecycleState::Processing);
        // Live entry, no waiter — child-scope violation.
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 9,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Child);

        // On a fresh child: mark instance 1 `Failed` — its late
        // waiter-less `Reply` is inert, not a violation.
        let fixture = DemuxFixture::live(3, LifecycleState::Processing);
        fixture.shared.lock().instances.get_mut(&1).unwrap().phase = InstPhase::Failed;
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 9,
                instance: 1,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Inert);
        // Unregistered value — also inert.
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 9,
                instance: 40,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Inert);
        assert!(fixture.shared.lock().terminal.is_none());
    }

    /// A flagged `Reply` is the child's `BlockIn` rejection —
    /// completes the `(i, seq)` doorbell with `mark_failed`, no
    /// shm work.
    #[test]
    fn demux_flagged_reply_completes_doorbell_failed() {
        let mut fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let seq = data::BLOCK_SEQ_FLAG | 5;
        fixture.shared.lock().block_in_flight.insert((0, seq));
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq,
                instance: 0,
                result: Err(HostError::PluginFailed),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Completed);
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::mark_failed(seq)
        );
        assert!(fixture.shared.lock().block_in_flight.is_empty());
    }

    /// `BlockDone` for `(i, seq)` not in `block_in_flight` — the
    /// attributable per-instance violation: `converge_instance(i)`
    /// fails that instance alone (`ChildExit{None, None}`), leaves
    /// the child live and siblings untouched, keeps the entry
    /// registered.
    #[test]
    fn demux_unknown_blockdone_converges_one_instance() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let seq = data::BLOCK_SEQ_FLAG | 5;
        // An in-flight doorbell + waiter on the OTHER instance —
        // the proof nothing leaks across.
        fixture.shared.lock().block_in_flight.insert((1, seq));
        let (tx, _rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(3, Waiter { instance: 1, tx });
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::BlockDone {
                seq,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Instance(0));
        {
            let inner = fixture.shared.lock();
            let failed = &inner.instances[&0];
            assert_eq!(failed.phase, InstPhase::Failed);
            assert_eq!(failed.mirror, LifecycleState::Failed);
            assert_eq!(failed.notify, Notify::Sent);
            assert_eq!(inner.instances[&1].phase, InstPhase::Live);
            assert_eq!(inner.instances[&1].mirror, LifecycleState::Processing);
            assert!(inner.terminal.is_none());
            // The sibling's waiter + doorbell bookkeeping survived.
            assert!(inner.waiters.contains_key(&3));
            assert!(inner.block_in_flight.contains(&(1, seq)));
        }
        let notice = fixture.notify_rxs.get(&0).unwrap().try_recv().unwrap();
        assert_eq!(
            notice.exit,
            ChildExit {
                code: None,
                signal: None
            }
        );
        assert_eq!(notice.cause, FailureCause::Protocol);
        assert!(fixture.notify_rxs.get(&1).unwrap().try_recv().is_err());
    }

    /// A `BlockDone` for an in-flight `(i, seq)` on a
    /// `Destroying` entry still completes — the doorbell window —
    /// while the same frame on a `Failed` entry demuxes inert.
    #[test]
    fn demux_destroying_window_vs_failed_inert() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let seq = data::BLOCK_SEQ_FLAG | 5;
        {
            let mut inner = fixture.shared.lock();
            inner.instances.get_mut(&0).unwrap().phase = InstPhase::Destroying;
            inner.block_in_flight.insert((0, seq));
        }
        // `Complete` — the doorbell window: a `BlockDone` on a
        // `Destroying` entry is still answered normally.
        let mut inner = fixture.shared.lock();
        let action = demux(
            &mut inner,
            &wire::Message::BlockDone {
                seq,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
        );
        assert!(matches!(
            action,
            Demux::Complete {
                instance: 0,
                seq: s,
                result: Some(ProcessResult::CONTINUE)
            } if s == seq
        ));
        // Same frame for a `Failed` entry with no in-flight
        // doorbell — inert.
        inner.instances.get_mut(&1).unwrap().phase = InstPhase::Failed;
        let action = demux(
            &mut inner,
            &wire::Message::BlockDone {
                seq,
                instance: 1,
                result: ProcessResult::CONTINUE,
            },
        );
        assert!(matches!(action, Demux::Inert));
        // The window also answers the *flagged-`Reply`* form of a
        // `BlockIn` rejection while `Destroying` — `result: None`
        // completes the doorbell as `mark_failed`.
        let rejected = data::BLOCK_SEQ_FLAG | 6;
        inner.block_in_flight.insert((0, rejected));
        let action = demux(
            &mut inner,
            &wire::Message::Reply {
                seq: rejected,
                instance: 0,
                result: Err(HostError::PluginFailed),
            },
        );
        assert!(matches!(
            action,
            Demux::Complete {
                instance: 0,
                seq: s,
                result: None
            } if s == rejected
        ));
    }

    /// Post-`Shutdown` frames are non-events — `shutdown_initiated`
    /// makes every demux verdict `NonEvent` before any instance
    /// resolution.
    #[test]
    fn demux_post_shutdown_frames_are_nonevents() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        fixture.shared.lock().shutdown_initiated = true;
        let mut inner = fixture.shared.lock();
        for msg in [
            wire::Message::Reply {
                seq: 9,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            wire::Message::BlockDone {
                seq: data::BLOCK_SEQ_FLAG | 5,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            wire::Message::Hello {
                version: wire::WIRE_VERSION,
                capabilities: 0,
            },
        ] {
            assert!(matches!(demux(&mut inner, &msg), Demux::NonEvent));
        }
    }

    /// `converge_child` fans one `ChildExit` out per *minted*
    /// instance — the `Creating` entry converges silently (its
    /// `Instantiate` caller gets `Err(Ipc)` through the drained
    /// waiter instead), waiters wake `Err(Ipc)`, in-flight
    /// doorbells get their `mark_failed` completions.
    #[test]
    fn converge_child_fans_out_per_minted_instance() {
        let mut fixture = DemuxFixture::live(2, LifecycleState::Processing);
        // A mid-create entry — registered but never minted.
        let (tx0, rx0) = mpsc::channel();
        let (tx1, rx1) = mpsc::channel();
        {
            let (mut inst, notify_rx, result_rx) = inst_shared(LifecycleState::Created);
            inst.phase = InstPhase::Creating;
            let mut inner = fixture.shared.lock();
            inner.instances.insert(7, inst);
            fixture.notify_rxs.insert(7, notify_rx);
            fixture.result_rxs.insert(7, result_rx);
            // Two waiters — one live instance, one mid-create —
            // and doorbells.
            inner.waiters.insert(
                3,
                Waiter {
                    instance: 0,
                    tx: tx0,
                },
            );
            inner.waiters.insert(
                4,
                Waiter {
                    instance: 7,
                    tx: tx1,
                },
            );
            inner.block_in_flight.insert((0, data::BLOCK_SEQ_FLAG | 5));
            inner.block_in_flight.insert((1, data::BLOCK_SEQ_FLAG | 6));
        }
        converge_child(&fixture.shared, None, FailureCause::ExitObserved);
        {
            let inner = fixture.shared.lock();
            assert!(matches!(inner.terminal, Some(Terminal::Failed)));
            assert!(inner.waiters.is_empty());
            assert!(inner.block_in_flight.is_empty());
            for instance in [0, 1, 7] {
                assert_eq!(inner.instances[&instance].phase, InstPhase::Failed);
                assert_eq!(inner.instances[&instance].mirror, LifecycleState::Failed);
            }
            // Minted → Pending (no exit status yet); the creating
            // entry stays silent.
            assert_eq!(inner.instances[&0].notify, Notify::Pending);
            assert_eq!(inner.instances[&1].notify, Notify::Pending);
            assert_eq!(inner.instances[&7].notify, Notify::Idle);
        }
        // Waiters wake `Err(Ipc)` — including the creating
        // entry's own in-flight command.
        assert!(matches!(rx0.try_recv(), Ok(Err(HostError::Ipc))));
        assert!(matches!(rx1.try_recv(), Ok(Err(HostError::Ipc))));
        // Doorbells complete `mark_failed` — one per instance.
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::mark_failed(data::BLOCK_SEQ_FLAG | 5)
        );
        assert_eq!(
            fixture.result_rxs.get_mut(&1).unwrap().try_pop().unwrap(),
            data::mark_failed(data::BLOCK_SEQ_FLAG | 6)
        );
        // A late exit status completes the pending notifications —
        // exactly once, per minted instance; the cause fixed at the
        // first convergence rides along unchanged.
        converge_child(
            &fixture.shared,
            Some(ChildExit {
                code: Some(3),
                signal: None,
            }),
            FailureCause::ExitObserved,
        );
        for instance in [0, 1] {
            let notice = fixture.notify_rxs[&instance].try_recv().unwrap();
            assert_eq!(
                notice.exit,
                ChildExit {
                    code: Some(3),
                    signal: None
                }
            );
            assert_eq!(notice.cause, FailureCause::ExitObserved);
        }
        // The mid-create entry never notifies — exactly once means
        // zero here.
        assert!(fixture.notify_rxs[&7].try_recv().is_err());
        // Idempotent — a second convergence changes nothing.
        converge_child(
            &fixture.shared,
            Some(ChildExit {
                code: Some(9),
                signal: None,
            }),
            FailureCause::ExitObserved,
        );
        assert!(fixture.notify_rxs[&0].try_recv().is_err());
    }

    /// `remove_instance` frees residual `(i,·)` doorbells with
    /// `mark_failed` — the leg's alternation gate is released
    /// before the entry leaves the map (the reuse-safety half of
    /// the doorbell window).
    #[test]
    fn remove_instance_completes_residual_doorbells() {
        let mut fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let seq = data::BLOCK_SEQ_FLAG | 5;
        {
            let mut inner = fixture.shared.lock();
            inner.block_in_flight.insert((0, seq));
            inner.block_in_flight.insert((1, data::BLOCK_SEQ_FLAG | 6));
            remove_instance(&mut inner, 0);
            assert!(!inner.instances.contains_key(&0));
            assert!(!inner.block_in_flight.contains(&(0, seq)));
            // The sibling's doorbell bookkeeping is untouched.
            assert!(
                inner
                    .block_in_flight
                    .contains(&(1, data::BLOCK_SEQ_FLAG | 6))
            );
        }
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::mark_failed(seq)
        );
        assert!(fixture.result_rxs.get_mut(&1).unwrap().try_pop().is_err());
    }

    /// `mint_instance` is lowest-free and `None` at
    /// `WIRE_MAX_INSTANCES` — the `InstanceLimit` boundary.
    #[test]
    fn mint_instance_lowest_free_and_capacity() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        {
            let inner = fixture.shared.lock();
            assert_eq!(mint_instance(&inner), Some(1));
        }
        // Fill to capacity — mint must yield `None`.
        {
            let mut inner = fixture.shared.lock();
            for instance in 1..wire::WIRE_MAX_INSTANCES {
                let (inst, _, _) = inst_shared(LifecycleState::Created);
                inner.instances.insert(instance, inst);
            }
            assert_eq!(mint_instance(&inner), None);
        }
    }

    /// A waiter-less `Reply` stamped `WIRE_INSTANCE_NONE` can
    /// attribute to nothing — the only child-scope reply kind is
    /// `Scan`'s answer, and that is always waiter-correlated.
    /// Child-scope violation.
    #[test]
    fn demux_waiterless_reply_none_stamp_is_child_scope() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 9,
                instance: wire::WIRE_INSTANCE_NONE,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Child);
        assert!(matches!(
            fixture.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
    }

    /// Doorbell stamps outside the handle space — `NONE` or
    /// `>= WIRE_MAX_INSTANCES` — can never attribute: child-scope
    /// violations, checked before doorbell correlation (an
    /// in-flight `(i, seq)` for a *valid* handle cannot legitimize
    /// an out-of-range stamp).
    #[test]
    fn demux_blockdone_out_of_range_instance_is_child_scope() {
        let seq = data::BLOCK_SEQ_FLAG | 5;
        for instance in [wire::WIRE_INSTANCE_NONE, wire::WIRE_MAX_INSTANCES] {
            let fixture = DemuxFixture::live(1, LifecycleState::Processing);
            fixture.shared.lock().block_in_flight.insert((0, seq));
            let verdict = demux_execute(
                &fixture.shared,
                &wire::Message::BlockDone {
                    seq,
                    instance,
                    result: ProcessResult::CONTINUE,
                },
                &mut HashMap::new(),
            );
            assert_eq!(
                verdict,
                Verdict::Child,
                "instance {instance:#x} must converge child-scope"
            );
            assert!(matches!(
                fixture.shared.lock().terminal,
                Some(Terminal::Failed)
            ));
        }
    }

    /// A second `Hello` and parent→child kinds (`Command`,
    /// `BlockIn`, `Shutdown`) arriving child→parent are
    /// unattributable — child-scope violations.
    #[test]
    fn demux_second_hello_and_parent_kinds_are_child_scope() {
        for msg in [
            wire::Message::Hello {
                version: wire::WIRE_VERSION,
                capabilities: 0,
            },
            wire::Message::Command {
                seq: 3,
                instance: 0,
                cmd: wire::Command::Scan,
            },
            wire::Message::BlockIn {
                seq: data::BLOCK_SEQ_FLAG | 5,
                instance: 0,
            },
            wire::Message::Shutdown,
        ] {
            let fixture = DemuxFixture::live(1, LifecycleState::Processing);
            let verdict = demux_execute(&fixture.shared, &msg, &mut HashMap::new());
            assert_eq!(verdict, Verdict::Child, "{msg:?} must converge child-scope");
        }
    }

    /// The flagged-`Reply` twin of the unknown-`BlockDone` row: a
    /// block seq never rung on a *live* instance is the
    /// attributable per-instance violation — `converge_instance`
    /// fails that instance alone.
    #[test]
    fn demux_unknown_flagged_reply_converges_one_instance() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let seq = data::BLOCK_SEQ_FLAG | 5;
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq,
                instance: 0,
                result: Err(HostError::PluginFailed),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Instance(0));
        {
            let inner = fixture.shared.lock();
            assert_eq!(inner.instances[&0].phase, InstPhase::Failed);
            assert_eq!(inner.instances[&1].phase, InstPhase::Live);
            assert!(inner.terminal.is_none());
        }
        let notice = fixture.notify_rxs[&0].try_recv().unwrap();
        assert_eq!(
            notice.exit,
            ChildExit {
                code: None,
                signal: None
            }
        );
        assert_eq!(notice.cause, FailureCause::Protocol);
        assert!(fixture.notify_rxs[&1].try_recv().is_err());
    }

    /// A per-command `Err(HostError)` reply is the defined
    /// rejection — delivered to its waiter like an `Ok`, never a
    /// convergence (the `InstanceLimit` variant a pooled
    /// `CreateInstance` caller must still see).
    #[test]
    fn demux_delivers_per_command_error_rejection() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(7, Waiter { instance: 0, tx });
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 7,
                instance: 0,
                result: Err(HostError::InstanceLimit),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Delivered);
        assert!(matches!(rx.try_recv(), Ok(Err(HostError::InstanceLimit))));
        assert!(fixture.shared.lock().terminal.is_none());
    }

    /// A waiter-less `Reply` stamped outside the live handle space —
    /// `>= WIRE_MAX_INSTANCES` — is unattributable exactly like the
    /// `NONE` stamp: a child-scope violation, checked before
    /// instance resolution (a live entry under a different handle
    /// cannot legitimize it).
    #[test]
    fn demux_waiterless_reply_out_of_range_is_child_scope() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 9,
                instance: wire::WIRE_MAX_INSTANCES,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Child);
        assert!(matches!(
            fixture.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
    }

    /// Correlation beats registration state: the `CreateInstance`
    /// reply itself arrives stamped for a `Creating`
    /// (registered-but-not-live) entry and must still deliver — the
    /// strict waiter-less rule only applies once no waiter claims
    /// the seq.
    #[test]
    fn demux_delivers_reply_on_creating_entry() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        fixture.shared.lock().instances.get_mut(&0).unwrap().phase = InstPhase::Creating;
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(7, Waiter { instance: 0, tx });
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::Reply {
                seq: 7,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            &mut HashMap::new(),
        );
        assert_eq!(verdict, Verdict::Delivered);
        assert!(matches!(rx.try_recv(), Ok(Ok(wire::Reply::Ok))));
        assert!(fixture.shared.lock().terminal.is_none());
    }

    /// The inert rows consolidated: any `Reply`/`BlockDone` stamped
    /// for a registered-but-not-live entry (`Creating`/`Destroying`/
    /// `Failed`), an unregistered handle inside the live range, or
    /// a handle the registry already removed is a silent drop —
    /// none of them may fail the entry, the child, or mint a
    /// notification.
    #[test]
    fn demux_frames_inert_by_phase_and_removal() {
        let fixture = DemuxFixture::live(3, LifecycleState::Processing);
        {
            let mut inner = fixture.shared.lock();
            inner.instances.get_mut(&0).unwrap().phase = InstPhase::Creating;
            inner.instances.get_mut(&1).unwrap().phase = InstPhase::Destroying;
            inner.instances.get_mut(&2).unwrap().phase = InstPhase::Failed;
        }
        let seq = data::BLOCK_SEQ_FLAG | 5;
        for msg in [
            // Waiter-less `Reply` on each registered-but-not-live
            // phase (the live-arm and unregistered-in-range arm are
            // covered by `demux_waiterless_reply_scope_by_instance_phase`).
            wire::Message::Reply {
                seq: 9,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            wire::Message::Reply {
                seq: 10,
                instance: 1,
                result: Ok(wire::Reply::Ok),
            },
            wire::Message::Reply {
                seq: 11,
                instance: 2,
                result: Ok(wire::Reply::Ok),
            },
            // `BlockDone` on each non-live phase with nothing in
            // flight, and on an unregistered in-range handle.
            wire::Message::BlockDone {
                seq,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            wire::Message::BlockDone {
                seq,
                instance: 1,
                result: ProcessResult::CONTINUE,
            },
            wire::Message::BlockDone {
                seq,
                instance: 2,
                result: ProcessResult::CONTINUE,
            },
            wire::Message::BlockDone {
                seq,
                instance: 40,
                result: ProcessResult::CONTINUE,
            },
            // A flagged `Reply` (the `BlockIn` rejection form) for
            // an unregistered handle — also inert.
            wire::Message::Reply {
                seq,
                instance: 40,
                result: Err(HostError::PluginFailed),
            },
        ] {
            let mut inner = fixture.shared.lock();
            assert!(
                matches!(demux(&mut inner, &msg), Demux::Inert),
                "{msg:?} must demux Inert"
            );
        }
        // Post-removal: a `BlockDone` arriving after
        // `remove_instance` reclaimed the handle is the
        // stale-generation frame — inert (this is what makes a
        // re-minted wire handle safe).
        {
            let mut inner = fixture.shared.lock();
            remove_instance(&mut inner, 2);
            let action = demux(
                &mut inner,
                &wire::Message::BlockDone {
                    seq,
                    instance: 2,
                    result: ProcessResult::CONTINUE,
                },
            );
            assert!(matches!(action, Demux::Inert));
        }
        // No false failures anywhere: the child is untouched and
        // no notification ever went out.
        {
            let inner = fixture.shared.lock();
            assert!(inner.terminal.is_none());
            assert_eq!(inner.instances[&0].notify, Notify::Idle);
            assert_eq!(inner.instances[&1].notify, Notify::Idle);
        }
        for rx in fixture.notify_rxs.values() {
            assert!(rx.try_recv().is_err());
        }
    }

    /// A converged child takes no more frames — `terminal.is_some()`
    /// demuxes every kind `NonEvent` before instance resolution,
    /// the post-convergence twin of the post-`Shutdown` row.
    #[test]
    fn demux_post_terminal_frames_are_nonevents() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        converge_child(&fixture.shared, None, FailureCause::ExitObserved);
        let mut inner = fixture.shared.lock();
        for msg in [
            wire::Message::Reply {
                seq: 9,
                instance: 0,
                result: Ok(wire::Reply::Ok),
            },
            wire::Message::BlockDone {
                seq: data::BLOCK_SEQ_FLAG | 5,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            wire::Message::Hello {
                version: wire::WIRE_VERSION,
                capabilities: 0,
            },
        ] {
            assert!(
                matches!(demux(&mut inner, &msg), Demux::NonEvent),
                "{msg:?} must demux NonEvent"
            );
        }
    }

    /// `converge_instance` is granular: it drains only *this*
    /// instance's waiters (`Err(Ipc)`) and doorbells
    /// (`mark_failed`), sends exactly one `ChildExit{None, None}`
    /// (the live child has no exit status to report), and leaves
    /// `terminal`/`seen_eof`/`shutdown_initiated`, every sibling's
    /// waiters/doorbells/phase and the entry itself registered.
    /// Idempotent, and a no-op on non-`Live` entries — the demux
    /// only routes `live` violations here.
    #[test]
    fn converge_instance_resolves_only_that_instance() {
        let mut fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let (tx0, rx0) = mpsc::channel();
        let (tx1, rx1) = mpsc::channel();
        {
            let mut inner = fixture.shared.lock();
            inner.waiters.insert(
                3,
                Waiter {
                    instance: 0,
                    tx: tx0,
                },
            );
            inner.waiters.insert(
                4,
                Waiter {
                    instance: 1,
                    tx: tx1,
                },
            );
            inner.block_in_flight.insert((0, data::BLOCK_SEQ_FLAG | 5));
            inner.block_in_flight.insert((1, data::BLOCK_SEQ_FLAG | 6));
        }
        converge_instance(&fixture.shared, 0, FailureCause::Protocol);
        {
            let inner = fixture.shared.lock();
            // This instance — `Failed`, entry still registered.
            let inst = &inner.instances[&0];
            assert_eq!(inst.phase, InstPhase::Failed);
            assert_eq!(inst.mirror, LifecycleState::Failed);
            assert_eq!(inst.notify, Notify::Sent);
            assert_eq!(
                inst.rt_state.load(Ordering::Acquire),
                data::mirror_encode(LifecycleState::Failed)
            );
            // Its waiter is gone, its doorbell released.
            assert!(!inner.waiters.contains_key(&3));
            assert!(
                !inner
                    .block_in_flight
                    .contains(&(0, data::BLOCK_SEQ_FLAG | 5))
            );
            // The sibling's bookkeeping + phase — untouched.
            assert_eq!(inner.instances[&1].phase, InstPhase::Live);
            assert_eq!(inner.instances[&1].mirror, LifecycleState::Processing);
            assert!(inner.waiters.contains_key(&4));
            assert!(
                inner
                    .block_in_flight
                    .contains(&(1, data::BLOCK_SEQ_FLAG | 6))
            );
            // The granularity invariant — child state untouched.
            assert!(inner.terminal.is_none());
            assert!(!inner.seen_eof);
            assert!(!inner.shutdown_initiated);
        }
        assert!(matches!(rx0.try_recv(), Ok(Err(HostError::Ipc))));
        assert!(rx1.try_recv().is_err());
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::mark_failed(data::BLOCK_SEQ_FLAG | 5)
        );
        assert!(fixture.result_rxs.get_mut(&1).unwrap().try_pop().is_err());
        let notice = fixture.notify_rxs[&0].try_recv().unwrap();
        assert_eq!(
            notice.exit,
            ChildExit {
                code: None,
                signal: None
            }
        );
        assert_eq!(notice.cause, FailureCause::Protocol);
        assert!(fixture.notify_rxs[&1].try_recv().is_err());
        // Idempotent — no second notification, no re-drain.
        converge_instance(&fixture.shared, 0, FailureCause::Protocol);
        assert!(fixture.notify_rxs[&0].try_recv().is_err());
        // Non-live entries are a no-op — a `Creating` sibling in
        // particular must not be converged by a violation that
        // never routed here anyway.
        fixture.shared.lock().instances.get_mut(&1).unwrap().phase = InstPhase::Creating;
        converge_instance(&fixture.shared, 1, FailureCause::Protocol);
        let inner = fixture.shared.lock();
        assert_eq!(inner.instances[&1].phase, InstPhase::Creating);
        assert_eq!(inner.instances[&1].mirror, LifecycleState::Processing);
        assert!(inner.waiters.contains_key(&4));
        assert!(inner.terminal.is_none());
    }

    /// First-observed cause wins — a `Protocol` cause fixed by
    /// `converge_instance` stands when a later child-scope crash
    /// converges everything, while a sibling that never had its own
    /// observation takes the crash's `ExitObserved`. The cause
    /// travels on each instance's own notification payload.
    #[test]
    fn converge_cause_is_first_observed_per_instance() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        converge_instance(&fixture.shared, 0, FailureCause::Protocol);
        // The child's real crash: instance 0's already-fixed
        // `Protocol` must not be overwritten; instance 1 learns
        // `ExitObserved`.
        converge_child(&fixture.shared, None, FailureCause::ExitObserved);
        {
            let inner = fixture.shared.lock();
            assert_eq!(inner.instances[&0].cause, Some(FailureCause::Protocol));
            assert_eq!(inner.instances[&1].cause, Some(FailureCause::ExitObserved));
            // Instance 0 was already `Sent` — stays `Sent` (no
            // second notification); instance 1 waits for the reap.
            assert_eq!(inner.instances[&0].notify, Notify::Sent);
            assert_eq!(inner.instances[&1].notify, Notify::Pending);
        }
        // Instance 0's own `Protocol` notice is still the only one
        // it ever sends — drain it, then nothing further arrives.
        let first = fixture.notify_rxs[&0].try_recv().unwrap();
        assert_eq!(first.cause, FailureCause::Protocol);
        assert_eq!(first.exit, ChildExit::default());
        converge_child(
            &fixture.shared,
            Some(ChildExit {
                code: Some(7),
                signal: None,
            }),
            FailureCause::ExitObserved,
        );
        let notice = fixture.notify_rxs[&1].try_recv().unwrap();
        assert_eq!(notice.exit.code, Some(7));
        assert_eq!(notice.cause, FailureCause::ExitObserved);
        // Exactly once on both — nothing further.
        assert!(fixture.notify_rxs[&0].try_recv().is_err());
        assert!(fixture.notify_rxs[&1].try_recv().is_err());
    }

    /// A synchronous `Write` double for `driver_command`/`drain_submits`
    /// (both take `&mut impl Write` — the real driver passes the
    /// child's `ChildStdin`): bytes accumulate, and each `flush` —
    /// `wire::write_message` ends in one — decodes every complete
    /// frame into `frames`. A fabricated `ChildStdin` would need an
    /// OS pipe, and `std::io::pipe()` hands Windows overlapped
    /// handles that a synchronous `WriteFile` blocks on; the generic
    /// writer is the seam the production code already offers.
    #[derive(Default)]
    struct FrameSink {
        buf: Vec<u8>,
        frames: std::collections::VecDeque<wire::Message>,
    }

    impl FrameSink {
        /// Oldest decoded frame — `None` means the writers pushed
        /// nothing (the "no doomed frame" half of a gate test).
        fn take(&mut self) -> Option<wire::Message> {
            self.frames.pop_front()
        }
    }

    impl Write for FrameSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            loop {
                let mut cursor = std::io::Cursor::new(&self.buf);
                let mut frame = Vec::new();
                match wire::read_frame(&mut cursor, &mut frame) {
                    Ok(true) => {
                        let consumed = usize::try_from(cursor.position()).unwrap_or(0);
                        self.frames
                            .push_back(wire::decode(&frame).expect("test frame decodes"));
                        self.buf.drain(..consumed);
                    }
                    // `Ok(false)` (clean end) or `Err` (a partial
                    // frame — impossible from `write_message`) —
                    // either way there is nothing more to decode.
                    _ => break,
                }
            }
            Ok(())
        }
    }

    /// `take` proof that nothing was written — the gate's
    /// "no doomed frame" half. The writers are synchronous, so
    /// anything coming was already decoded by the `flush`.
    fn assert_no_frame(sink: &mut FrameSink) {
        assert!(
            sink.take().is_none(),
            "a rejected operation must not write a wire frame"
        );
    }

    /// The `driver_command` gate: `DestroyInstance` writes the
    /// `Command` frame and flips the entry to `Destroying` the
    /// moment it hits the wire (the doorbell-window edge); from
    /// then on queued commands for that instance are rejected —
    /// the waiter wakes `Err(Ipc)` and nothing reaches the wire.
    /// `DestroyInstance` itself stays exempt on a `Failed` entry
    /// (reclaiming the runner slot is its job); a converged or
    /// shutting-down child rejects everything.
    #[test]
    fn driver_command_destroy_flips_destroying_then_gates() {
        let fixture = DemuxFixture::live(2, LifecycleState::Processing);
        let mut sink = FrameSink::default();
        let mut buf = Vec::new();

        // `DestroyInstance` ships the frame and opens the window.
        assert!(driver_command(
            &fixture.shared,
            &mut sink,
            1,
            0,
            wire::Command::DestroyInstance,
            &mut buf,
            false,
        ));
        match sink.take() {
            Some(wire::Message::Command {
                seq: 1,
                instance: 0,
                cmd: wire::Command::DestroyInstance,
            }) => {}
            other => panic!("expected DestroyInstance frame, got {other:?}"),
        }
        assert_eq!(
            fixture.shared.lock().instances[&0].phase,
            InstPhase::Destroying
        );

        // A queued command for the now-`Destroying` entry is
        // rejected: the waiter wakes `Err(Ipc)`, no doomed frame.
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(2, Waiter { instance: 0, tx });
        assert!(driver_command(
            &fixture.shared,
            &mut sink,
            2,
            0,
            wire::Command::Initialize,
            &mut buf,
            false,
        ));
        assert!(matches!(rx.try_recv(), Ok(Err(HostError::Ipc))));
        assert_no_frame(&mut sink);

        // `DestroyInstance` on a `Failed` entry is the exemption —
        // it must still write (the runner-slot reclaim), and a
        // `Failed` entry's phase does not become `Destroying`.
        fixture.shared.lock().instances.get_mut(&1).unwrap().phase = InstPhase::Failed;
        assert!(driver_command(
            &fixture.shared,
            &mut sink,
            3,
            1,
            wire::Command::DestroyInstance,
            &mut buf,
            false,
        ));
        match sink.take() {
            Some(wire::Message::Command {
                seq: 3,
                instance: 1,
                cmd: wire::Command::DestroyInstance,
            }) => {}
            other => panic!("expected DestroyInstance frame, got {other:?}"),
        }
        assert_eq!(fixture.shared.lock().instances[&1].phase, InstPhase::Failed);

        // `shutting_down` rejects every command — the wire takes
        // nothing after `Shutdown` is initiated.
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(4, Waiter { instance: 1, tx });
        assert!(driver_command(
            &fixture.shared,
            &mut sink,
            4,
            1,
            wire::Command::Initialize,
            &mut buf,
            true,
        ));
        assert!(matches!(rx.try_recv(), Ok(Err(HostError::Ipc))));
        assert_no_frame(&mut sink);

        // A converged child rejects even `DestroyInstance`.
        fixture.shared.lock().terminal = Some(Terminal::Failed);
        let (tx, rx) = mpsc::channel();
        fixture
            .shared
            .lock()
            .waiters
            .insert(5, Waiter { instance: 1, tx });
        assert!(driver_command(
            &fixture.shared,
            &mut sink,
            5,
            1,
            wire::Command::DestroyInstance,
            &mut buf,
            false,
        ));
        assert!(matches!(rx.try_recv(), Ok(Err(HostError::Ipc))));
        assert_no_frame(&mut sink);
    }

    /// One producer-side `Submit` doorbell push — the
    /// `drain_submits_rings_doorbells_and_gates_dead` test's
    /// per-instance `try_push` spelled once.
    fn push_submit(
        producers: &mut HashMap<u32, ruviform_rt::Producer<data::Submit>>,
        instance: u32,
        n: u64,
        generation: u64,
    ) {
        producers
            .get_mut(&instance)
            .unwrap()
            .try_push(data::Submit {
                seq: data::BLOCK_SEQ_FLAG | n,
                generation,
            })
            .unwrap();
    }

    /// The `drain_submits` doorbell gates: a live+armed instance's
    /// submit rings a real doorbell — `(i, seq)` is in
    /// `block_in_flight` with its `BlockIn` on the wire (the code
    /// orders the insert ahead of the write so a reply can never
    /// outrun the bookkeeping; this row observes the post-call
    /// pair, not the intra-call order) — while every gated drop
    /// (`Destroying`/`Failed`/`Creating` entry, stale
    /// generation, draining child) still owes the leg its
    /// completion: `mark_failed(seq)` on the result channel, never
    /// a frame. An unregistered entry's seq needs no completion —
    /// its result channel died with the entry.
    #[test]
    fn drain_submits_rings_doorbells_and_gates_dead() {
        let mut fixture = DemuxFixture::live(4, LifecycleState::Processing);
        let mut sink = FrameSink::default();
        let mut buf = Vec::new();
        let seq = |n: u64| data::BLOCK_SEQ_FLAG | n;

        // Instance 0: `Live` + armed staging (generation 7) + a
        // real segment — the doorbell path (shm in-slot write +
        // `BlockIn`). 1–3: one entry per non-live phase.
        let segment = shm::Segment::create(900, 3, 3).expect("test segment");
        {
            let mut inner = fixture.shared.lock();
            let inst = inner.instances.get_mut(&0).unwrap();
            let InstKind::Audio { shm, staging, .. } = &mut inst.kind else {
                panic!("DemuxFixture mints Audio entries");
            };
            *shm = SegmentInfo {
                key: segment.key().to_owned(),
                in_depth: 3,
                out_depth: 3,
            };
            *staging = Some(Arc::new(data::Staging::new(3, 64, 7)));
            inner.instances.get_mut(&1).unwrap().phase = InstPhase::Destroying;
            inner.instances.get_mut(&2).unwrap().phase = InstPhase::Failed;
            inner.instances.get_mut(&3).unwrap().phase = InstPhase::Creating;
        }
        let mut views = HashMap::new();
        views.insert(0_u32, shm::View::attach(segment.key()).expect("attach"));
        let mut submits: HashMap<u32, ruviform_rt::Consumer<data::Submit>> = HashMap::new();
        let mut producers: HashMap<u32, ruviform_rt::Producer<data::Submit>> = HashMap::new();
        for instance in [0_u32, 1, 2, 3, 9] {
            let (tx, rx) = ruviform_rt::channel::<data::Submit>(3);
            submits.insert(instance, rx);
            producers.insert(instance, tx);
        }
        // Armed submit on the live instance → real doorbell.
        push_submit(&mut producers, 0, 5, 7);
        // Stale-generation submit on the live instance → drop.
        push_submit(&mut producers, 0, 6, 8);
        // Non-live entries → drop (the `Destroying` gate).
        push_submit(&mut producers, 1, 7, 7);
        push_submit(&mut producers, 2, 8, 7);
        push_submit(&mut producers, 3, 9, 7);
        // Unregistered entry → silent drop, no completion owed.
        push_submit(&mut producers, 9, 10, 7);

        assert!(drain_submits(
            &fixture.shared,
            &mut sink,
            &mut submits,
            &mut views,
            &mut buf
        ));

        // The real doorbell went in flight BEFORE the `BlockIn`
        // frame — and it is the only frame written.
        {
            let inner = fixture.shared.lock();
            assert!(inner.block_in_flight.contains(&(0, seq(5))));
            assert!(!inner.block_in_flight.contains(&(0, seq(6))));
        }
        match sink.take() {
            Some(wire::Message::BlockIn {
                seq: s,
                instance: 0,
            }) if s == seq(5) => {}
            other => panic!("expected the BlockIn doorbell, got {other:?}"),
        }
        assert_no_frame(&mut sink);
        // Every gated drop completed `mark_failed` on its own
        // instance's result channel — exactly one completion per seq.
        for (instance, n) in [(0_u32, 6_u64), (1, 7), (2, 8), (3, 9)] {
            let rx = fixture.result_rxs.get_mut(&instance).unwrap();
            assert_eq!(rx.try_pop().unwrap(), data::mark_failed(seq(n)));
        }

        // A draining child (`shutdown_initiated`) takes no new
        // doorbells — even an armed `Live` entry's submit is a
        // gated drop.
        fixture.shared.lock().shutdown_initiated = true;
        push_submit(&mut producers, 0, 11, 7);
        assert!(drain_submits(
            &fixture.shared,
            &mut sink,
            &mut submits,
            &mut views,
            &mut buf
        ));
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::mark_failed(seq(11))
        );
        assert!(
            !fixture
                .shared
                .lock()
                .block_in_flight
                .contains(&(0, seq(11)))
        );
        assert_no_frame(&mut sink);
    }

    /// The reader's `views` map is insert-only — `RemoveInstance`
    /// reaches only the driver's copies. On wire-handle reuse the
    /// `SegmentInfo` compare must re-attach: a stale `View` would
    /// read the previous generation's segment (old `os_id`, old
    /// depths) for the new instance's `BlockDone`s.
    #[test]
    fn complete_doorbell_reattaches_view_on_handle_reuse() {
        let mut fixture = DemuxFixture::live(1, LifecycleState::Processing);
        let segment_n = shm::Segment::create(0, 3, 3).expect("gen-N segment");
        let segment_n1 = shm::Segment::create(1, 3, 3).expect("gen-N+1 segment");
        // Arm a staging ring so the out-slot read path actually
        // runs through the attached `View`.
        {
            let mut inner = fixture.shared.lock();
            let inst = inner.instances.get_mut(&0).unwrap();
            let InstKind::Audio { shm, staging, .. } = &mut inst.kind else {
                panic!("DemuxFixture mints Audio entries");
            };
            *shm = SegmentInfo {
                key: segment_n.key().to_owned(),
                in_depth: 3,
                out_depth: 3,
            };
            *staging = Some(Arc::new(data::Staging::new(3, 8, 1)));
            inner.block_in_flight.insert((0, data::BLOCK_SEQ_FLAG | 5));
        }
        let mut views: HashMap<u32, ReaderView> = HashMap::new();
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::BlockDone {
                seq: data::BLOCK_SEQ_FLAG | 5,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            &mut views,
        );
        assert_eq!(verdict, Verdict::Completed);
        assert_eq!(views[&0].shm.key, segment_n.key());
        // Re-mint under the same wire handle: the entry now names
        // the gen-N+1 segment — the next `BlockDone` must attach
        // it instead of reading gen N's mapping.
        {
            let mut inner = fixture.shared.lock();
            let inst = inner.instances.get_mut(&0).unwrap();
            let InstKind::Audio { shm, .. } = &mut inst.kind else {
                panic!("DemuxFixture mints Audio entries");
            };
            *shm = SegmentInfo {
                key: segment_n1.key().to_owned(),
                in_depth: 3,
                out_depth: 3,
            };
            inner.block_in_flight.insert((0, data::BLOCK_SEQ_FLAG | 6));
        }
        let verdict = demux_execute(
            &fixture.shared,
            &wire::Message::BlockDone {
                seq: data::BLOCK_SEQ_FLAG | 6,
                instance: 0,
                result: ProcessResult::CONTINUE,
            },
            &mut views,
        );
        assert_eq!(verdict, Verdict::Completed);
        assert_eq!(views[&0].shm.key, segment_n1.key());
        // The read ran through the new mapping (no panic) and the
        // completion still pushed the seq — not `mark_failed`.
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::BLOCK_SEQ_FLAG | 5
        );
        assert_eq!(
            fixture.result_rxs.get_mut(&0).unwrap().try_pop().unwrap(),
            data::BLOCK_SEQ_FLAG | 6
        );
    }

    /// `remove_instance` on a `Pending` entry flushes the
    /// status-unknown `ChildExit` before `notify_tx` drops — the
    /// backstop that keeps exactly-once when the entry leaves the
    /// registry ahead of the driver's reap.
    #[test]
    fn remove_instance_flushes_pending_notification() {
        let fixture = DemuxFixture::live(1, LifecycleState::Processing);
        // The EOF-path convergence arms `Pending` on the minted
        // entry — the driver's `try_wait`/`reap` would normally
        // complete it. Removing the entry first must not strand
        // the notification.
        converge_child(&fixture.shared, None, FailureCause::ExitObserved);
        {
            let mut inner = fixture.shared.lock();
            assert_eq!(inner.instances[&0].notify, Notify::Pending);
            remove_instance(&mut inner, 0);
            assert!(!inner.instances.contains_key(&0));
        }
        let notice = fixture.notify_rxs[&0].try_recv().unwrap();
        assert_eq!(
            notice.exit,
            ChildExit {
                code: None,
                signal: None
            }
        );
        assert_eq!(notice.cause, FailureCause::ExitObserved);
        // Exactly once — nothing else may arrive.
        assert!(fixture.notify_rxs[&0].try_recv().is_err());
    }

    /// A `Shared` with no instances behind an `L1Child` whose
    /// `DriverCmd` channel the test services by hand — the
    /// `create_instance` reply-path harness (no real driver).
    fn create_harness() -> (Arc<L1Child>, mpsc::Receiver<DriverCmd>) {
        let (driver_tx, driver_rx) = mpsc::channel();
        let fixture = DemuxFixture::live(0, LifecycleState::Created);
        let child = Arc::new(L1Child {
            shared: Arc::new(fixture.shared),
            driver_tx: Mutex::new(Some(driver_tx)),
            driver_join: Mutex::new(None),
            reader_join: Mutex::new(None),
            command_timeout: Duration::from_secs(30),
        });
        (child, driver_rx)
    }

    /// Runs `create_instance` on a thread (the `InstanceParts`
    /// success payload holds the `!Send` segment, so it stays and
    /// drops there — these tests assert the `Err` half and the
    /// registry state anyway) and drains the driver channel far
    /// enough to learn the `CreateInstance` waiter's seq.
    fn spawn_create(
        child: &Arc<L1Child>,
        driver_rx: &mpsc::Receiver<DriverCmd>,
        diag: &Arc<diag::DiagRegistry>,
    ) -> (u64, thread::JoinHandle<Result<(), HostError>>) {
        let spec = SpawnSpec::new(
            PluginDescriptor {
                id: "org.ruviform.test.create-seam",
                name: "create-seam",
                vendor: "Ruviform",
                version: "0",
                features: &[],
            },
            OsString::from("unused-runner"),
        );
        let join = thread::spawn({
            let child = Arc::clone(child);
            let diag = Arc::clone(diag);
            move || child.create_instance(&spec, 3, &diag).map(|_| ())
        });
        // `AddInstance` (data-plane ends) precedes the
        // `CreateInstance` `Command` carrying the waiter seq.
        let mut seq = None;
        while seq.is_none() {
            match driver_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(DriverCmd::Command { seq: s, .. }) => seq = Some(s),
                Ok(_) => {}
                Err(error) => panic!("driver channel died before CreateInstance: {error}"),
            }
        }
        (seq.expect("checked above"), join)
    }

    /// The mid-mint race, sequenced deterministically: the
    /// `CreateInstance` `Ok` reply correlates its waiter, then
    /// `converge_child` flips the still-`Creating` entry `Failed`
    /// *without* queuing a notification (`minted()` is false — the
    /// `Creating` entry converges silently), then the reply wakes
    /// `request`. `create_instance` must not mint a facade off a
    /// `Failed` entry — it folds into the same rollback as a
    /// defined rejection: `SpawnFailed` carrying the convergence's
    /// recorded cause, entry removed, no notification,
    /// `RemoveInstance` ordered.
    #[test]
    fn create_instance_ok_then_converge_rolls_back() {
        let (child, driver_rx) = create_harness();
        let diag = Arc::new(diag::DiagRegistry::new());
        let (seq, join) = spawn_create(&child, &driver_rx, &diag);
        let waiter = child
            .shared
            .lock()
            .waiters
            .remove(&seq)
            .expect("waiter registered");
        converge_child(&child.shared, None, FailureCause::ExitObserved);
        let _ = waiter.tx.send(Ok(wire::Reply::Ok));
        let Err(HostError::SpawnFailed {
            cause,
            diagnostics_id,
        }) = join.join().expect("create_instance joined")
        else {
            panic!("mid-mint convergence must surface SpawnFailed")
        };
        // The convergence's recorded `ExitObserved` is the attempt's
        // cause — the fold does not reclassify it.
        assert_eq!(cause, FailureCause::ExitObserved);
        // The attempt record resolves through the registry query.
        let diag::DiagnosticQuery::Found(record) = diag.query(diagnostics_id) else {
            panic!("the registered attempt record must resolve")
        };
        assert_eq!(record.cause, Some(FailureCause::ExitObserved));
        assert_eq!(record.handle, None);
        {
            let inner = child.shared.lock();
            // No `Failed → Live` regression — the entry is gone.
            assert!(!inner.instances.contains_key(&0));
            assert!(matches!(inner.terminal, Some(Terminal::Failed)));
        }
        // The rollback orders `RemoveInstance` so the driver frees
        // this handle's data-plane ends for reuse.
        assert!(matches!(
            driver_rx.try_recv(),
            Ok(DriverCmd::RemoveInstance { instance: 0 })
        ));
    }

    /// The defined `Err(HostError)` `CreateInstance` rejections:
    /// `InstanceLimit` is a *capacity* fact a pooled caller must
    /// distinguish (try another child) — it propagates
    /// unflattened, registering no record. `UnknownPluginId` and
    /// every other rejection is the child's defined `Err` reply —
    /// `SpawnFailed{cause: Handshake}` plus a registered attempt
    /// record. Either way the entry rolls back.
    #[test]
    fn create_instance_reply_error_surfaces() {
        for reply in [
            HostError::InstanceLimit,
            HostError::UnknownPluginId,
            HostError::PluginFailed,
        ] {
            let (child, driver_rx) = create_harness();
            let diag = Arc::new(diag::DiagRegistry::new());
            let (seq, join) = spawn_create(&child, &driver_rx, &diag);
            let waiter = child
                .shared
                .lock()
                .waiters
                .remove(&seq)
                .expect("waiter registered");
            let _ = waiter.tx.send(Err(reply));
            let error = join.join().expect("create_instance joined").unwrap_err();
            if reply == HostError::InstanceLimit {
                assert_eq!(
                    error,
                    HostError::InstanceLimit,
                    "{reply:?} must propagate unflattened"
                );
                // A defined capacity rejection registers no record.
                assert_eq!(diag.issued_max(), 0);
            } else {
                let HostError::SpawnFailed {
                    cause,
                    diagnostics_id,
                } = error
                else {
                    panic!("{reply:?} must surface as SpawnFailed, got {error:?}")
                };
                // A child-defined rejection reply is `Handshake`.
                assert_eq!(cause, FailureCause::Handshake, "{reply:?}");
                assert!(matches!(
                    diag.query(diagnostics_id),
                    diag::DiagnosticQuery::Found(_)
                ));
            }
            assert!(child.shared.lock().instances.is_empty());
            assert!(matches!(
                driver_rx.try_recv(),
                Ok(DriverCmd::RemoveInstance { instance: 0 })
            ));
        }
    }

    // ---------------------------------------------------------------
    // T3 pool seams — `pool_eligible`/`is_terminal`/`live_count`, the
    // shared spec-legality gate and the full-child `InstanceLimit`
    // the `Bundled` selection must distinguish. The composite's own
    // pool tests live in `crate::composite`.
    // ---------------------------------------------------------------

    /// The child-selection predicate the `Bundled` route scans on:
    /// `live_count < WIRE_MAX_INSTANCES && !terminal &&
    /// !shutdown_initiated`, atomically — every ineligible reason
    /// answered alone.
    #[test]
    fn pool_eligible_covers_every_rejection_reason() {
        // Eligible: registered live entries below capacity.
        let child = L1Child::test_fixture(2);
        assert_eq!(child.live_count(), 2);
        assert!(child.pool_eligible());
        assert!(!child.is_terminal());

        // Full: at capacity, still alive — ineligible but not
        // terminal (the pool keeps it, it must just not serve).
        let child = L1Child::test_fixture(wire::WIRE_MAX_INSTANCES);
        assert_eq!(child.live_count(), wire::WIRE_MAX_INSTANCES as usize);
        assert!(!child.pool_eligible());
        assert!(!child.is_terminal());

        // Terminal: converged — ineligible and the pool's eviction
        // oracle fires.
        let child = L1Child::test_fixture(1);
        child.test_converge();
        assert!(!child.pool_eligible());
        assert!(child.is_terminal());

        // `shutdown_initiated` (not yet terminal) — draining toward
        // clean teardown: ineligible, but not an eviction candidate.
        let child = L1Child::test_fixture(1);
        child.shared.lock().shutdown_initiated = true;
        assert!(!child.pool_eligible());
        assert!(!child.is_terminal());
    }

    /// `live_count` counts *registered* entries — `Creating`,
    /// `Failed` and `Destroying` included — so a converged-but-not-
    /// reclaimed child still reads full until `DestroyInstance`
    /// reclaims it; `remove_instance` frees the count.
    #[test]
    fn live_count_counts_registered_until_reclaimed() {
        let child = L1Child::test_fixture(2);
        child.test_converge();
        // Both entries converged `Failed` but stay registered —
        // `live_count` still counts them.
        assert_eq!(child.live_count(), 2);
        {
            let mut inner = child.shared.lock();
            remove_instance(&mut inner, 0);
        }
        assert_eq!(child.live_count(), 1);
    }

    /// `remove_instance` frees a wire handle for the next
    /// lowest-free mint — the reuse edge of `live_count` that the
    /// shutdown-on-empty rule and `pool_eligible` both read.
    #[test]
    fn remove_instance_frees_the_lowest_handle() {
        let child = L1Child::test_fixture(3);
        {
            let mut inner = child.shared.lock();
            remove_instance(&mut inner, 1);
            assert_eq!(inner.instances.len(), 2);
            // Lowest-free — handle 1 is minted again, not 3.
            assert_eq!(mint_instance(&inner), Some(1));
        }
    }

    /// A full child's `create_instance` answers `Err(InstanceLimit)`
    /// — the *capacity* fact the pooled caller must distinguish
    /// from `Ipc` — before any waiter or driver order ships.
    #[test]
    fn create_instance_on_full_child_is_instance_limit() {
        let child = L1Child::test_fixture(wire::WIRE_MAX_INSTANCES);
        let spec = SpawnSpec::new(
            PluginDescriptor {
                id: "org.ruviform.test.pool-full",
                name: "pool-full",
                vendor: "Ruviform",
                version: "0",
                features: &[],
            },
            OsString::from("unused-runner"),
        );
        assert!(matches!(
            child.create_instance(&spec, 3, &diag::DiagRegistry::new()),
            Err(HostError::InstanceLimit)
        ));
        // Nothing was registered — the count stays at capacity.
        assert_eq!(child.live_count(), wire::WIRE_MAX_INSTANCES as usize);
    }

    /// The spec-legality gate both `Instantiate` paths share:
    /// `lookahead_blocks` 0 and `>= WIRE_MAX_ELEMENTS` are the
    /// defined `Err(Ipc)`; a legal spec yields `D =
    /// lookahead_blocks + 1`.
    #[test]
    fn instance_depth_gates_the_spec() {
        let mut spec = SpawnSpec::new(
            PluginDescriptor {
                id: "org.ruviform.test.depth",
                name: "depth",
                vendor: "Ruviform",
                version: "0",
                features: &[],
            },
            OsString::from("unused-runner"),
        );
        spec.lookahead_blocks = 0;
        assert!(matches!(spec.instance_depth(), Err(HostError::Ipc)));
        spec.lookahead_blocks = wire::WIRE_MAX_ELEMENTS;
        assert!(matches!(spec.instance_depth(), Err(HostError::Ipc)));
        spec.lookahead_blocks = 2;
        assert_eq!(spec.instance_depth().unwrap(), 3);
    }

    // ---------------------------------------------------------------
    // Issue #118 T3 — the video side of the shared machinery: the
    // `Creating` gate's `CreateVideoInstance` arm, the
    // timeout-generalized `request_timed`, and
    // `create_video_instance`'s no-data-plane create/rollback pair.
    // `VideoL1Instance`'s own facade tests live in `l1::video`.
    // ---------------------------------------------------------------

    use ruviform_media_core::FrameRate;
    use ruviform_video_core::PixelFormat;

    /// A small legal video config for the machinery tests — dims
    /// stay tiny so the `vshm::Segment` allocation is trivial.
    fn video_config(width: u32, height: u32) -> VideoProcessConfig {
        VideoProcessConfig::new(
            PixelFormat::Yuv420p,
            width,
            height,
            FrameRate::new(30, 1).unwrap(),
        )
        .unwrap()
    }

    /// The video spec the create-path tests mint on — descriptor id
    /// is the `CreateVideoInstance` `plugin_id` echo oracle.
    fn video_spec() -> SpawnSpec {
        SpawnSpec::new(
            PluginDescriptor {
                id: "org.ruviform.test.video-create",
                name: "video-create",
                vendor: "Ruviform",
                version: "0",
                features: &[],
            },
            OsString::from("unused-video-runner"),
        )
    }

    /// Registers a `Creating`-phase `InstKind::Video` entry on
    /// `child` — the registry shape `create_video_instance` leaves
    /// between the mint and the `Live` flip.
    fn insert_creating_video(child: &L1Child, instance: u32, config: VideoProcessConfig) {
        let (notify_tx, _notify_rx) = mpsc::channel();
        child.shared.lock().instances.insert(
            instance,
            InstShared {
                rt_state: Arc::new(AtomicU8::new(data::mirror_encode(LifecycleState::Created))),
                mirror: LifecycleState::Created,
                notify_tx,
                notify: Notify::Idle,
                kind: InstKind::Video { config },
                phase: InstPhase::Creating,
                cause: None,
            },
        );
    }

    /// The `Creating` gate accepts `CreateVideoInstance` — it IS
    /// the create — while rejecting every other verb on a
    /// mid-mint video entry, the same instance-scope rule
    /// `CreateInstance` obeys on audio.
    #[test]
    fn creating_gate_accepts_create_video_instance() {
        let (child, driver_rx) = create_harness();
        let config = video_config(64, 48);
        insert_creating_video(&child, 0, config);
        let create = wire::Command::CreateVideoInstance {
            plugin_id: "org.ruviform.test.video-create".to_owned(),
            shm_key: "unused-key".to_owned(),
            config,
        };
        // The non-create verb rejects before the wire — no waiter,
        // no `DriverCmd`.
        assert!(matches!(
            child.request(0, wire::Command::VideoGetParams),
            Err(HostError::Ipc)
        ));
        assert!(matches!(
            driver_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        // The create verb passes the gate — the `Command` carries
        // the `CreateVideoInstance` verbatim.
        let join = thread::spawn({
            let child = Arc::clone(&child);
            move || child.request(0, create)
        });
        match driver_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(DriverCmd::Command { seq, instance, cmd }) => {
                assert_eq!(instance, 0);
                assert!(matches!(cmd, wire::Command::CreateVideoInstance { .. }));
                let waiter = child
                    .shared
                    .lock()
                    .waiters
                    .remove(&seq)
                    .expect("waiter registered");
                waiter.tx.send(Ok(wire::Reply::Ok)).expect("waiter open");
            }
            Ok(_) => panic!("expected the CreateVideoInstance Command"),
            Err(error) => panic!("driver channel died before CreateVideoInstance: {error}"),
        }
        assert!(matches!(
            join.join().expect("request joined"),
            Ok(wire::Reply::Ok)
        ));
    }

    /// `request_timed` is the timeout-generalized request: the
    /// explicit bound, not `command_timeout`, bounds the reply
    /// wait — a 30 ms render window against the fixture's 30 s
    /// `command_timeout` answers in milliseconds. Timeout behavior
    /// is unchanged: waiter removal, child-scope `converge_child`,
    /// `Err(Ipc)`.
    #[test]
    fn request_timed_honors_the_explicit_timeout() {
        let (child, driver_rx) = create_harness();
        let config = video_config(64, 48);
        insert_creating_video(&child, 0, config);
        child.shared.lock().instances.get_mut(&0).unwrap().phase = InstPhase::Live;
        let started = Instant::now();
        let join = thread::spawn({
            let child = Arc::clone(&child);
            move || child.request_timed(0, wire::Command::VideoGetParams, Duration::from_millis(30))
        });
        assert!(matches!(
            driver_rx.recv_timeout(Duration::from_secs(30)),
            Ok(DriverCmd::Command { .. })
        ));
        assert!(matches!(
            join.join().expect("request joined"),
            Err(HostError::Ipc)
        ));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the explicit 30 ms bound, not the 30 s command_timeout, must pace the wait"
        );
        // The reply timeout is unattributable — the whole child
        // converges `Failed`.
        assert!(matches!(
            child.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
    }

    /// Runs `create_video_instance` on a thread — the
    /// `VideoInstanceParts` success payload holds the `!Send`
    /// `vshm` segment, so it stays and drops there (these tests
    /// assert the `Err` half and the registry state). Returns the
    /// `CreateVideoInstance` waiter's seq plus the join handle;
    /// asserts the *first* driver order is the `Command` itself —
    /// the no-`AddInstance` pin the video create differs on.
    fn spawn_video_create(
        child: &Arc<L1Child>,
        driver_rx: &mpsc::Receiver<DriverCmd>,
        config: VideoProcessConfig,
        diag: &Arc<diag::DiagRegistry>,
    ) -> (u64, thread::JoinHandle<Result<(), HostError>>) {
        let spec = video_spec();
        let join = thread::spawn({
            let child = Arc::clone(child);
            let diag = Arc::clone(diag);
            move || {
                child
                    .create_video_instance(&spec.view(), config, &diag)
                    .map(|_| ())
            }
        });
        match driver_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(DriverCmd::Command {
                seq,
                cmd: wire::Command::CreateVideoInstance { .. },
                ..
            }) => (seq, join),
            Ok(_) => panic!("a video create must not order AddInstance — Command comes first"),
            Err(error) => panic!("driver channel died before CreateVideoInstance: {error}"),
        }
    }

    /// The `Ok` half: the `Creating` video entry flips `Live`
    /// atomically, `InstKind::Video` keeps the stashed config, and
    /// no driver order beyond the `Command` ever ships (no
    /// `AddInstance`, no `RemoveInstance`).
    #[test]
    fn create_video_instance_ok_flips_live() {
        let (child, driver_rx) = create_harness();
        let diag = Arc::new(diag::DiagRegistry::new());
        let config = video_config(64, 48);
        let (seq, join) = spawn_video_create(&child, &driver_rx, config, &diag);
        let waiter = child
            .shared
            .lock()
            .waiters
            .remove(&seq)
            .expect("waiter registered");
        waiter.tx.send(Ok(wire::Reply::Ok)).expect("waiter open");
        join.join().expect("create_video_instance joined").unwrap();
        {
            let inner = child.shared.lock();
            let inst = inner.instances.get(&0).expect("entry registered");
            assert!(matches!(inst.phase, InstPhase::Live));
            assert!(matches!(
                &inst.kind,
                InstKind::Video { config: pinned } if *pinned == config
            ));
            assert_eq!(inst.mirror, LifecycleState::Created);
        }
        // Nothing else was ordered — a video entry installs and
        // frees no driver ends.
        assert!(matches!(
            driver_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    /// The defined `Err(HostError)` `CreateVideoInstance`
    /// rejections mirror `create_instance`'s: `InstanceLimit`
    /// propagates unflattened and registers no record; every other
    /// rejection is `SpawnFailed{cause: Handshake}` plus a
    /// registered attempt record. The entry rolls back **without**
    /// a `RemoveInstance` order — a video entry never installed
    /// driver ends.
    #[test]
    fn create_video_instance_reply_error_surfaces() {
        for reply in [
            HostError::InstanceLimit,
            HostError::UnknownPluginId,
            HostError::PluginFailed,
        ] {
            let (child, driver_rx) = create_harness();
            let diag = Arc::new(diag::DiagRegistry::new());
            let (seq, join) = spawn_video_create(&child, &driver_rx, video_config(64, 48), &diag);
            let waiter = child
                .shared
                .lock()
                .waiters
                .remove(&seq)
                .expect("waiter registered");
            let _ = waiter.tx.send(Err(reply));
            let error = join
                .join()
                .expect("create_video_instance joined")
                .unwrap_err();
            if reply == HostError::InstanceLimit {
                assert_eq!(
                    error,
                    HostError::InstanceLimit,
                    "{reply:?} must propagate unflattened"
                );
                assert_eq!(diag.issued_max(), 0);
            } else {
                assert!(
                    matches!(
                        error,
                        HostError::SpawnFailed {
                            cause: FailureCause::Handshake,
                            ..
                        }
                    ),
                    "{reply:?} must surface as SpawnFailed{{Handshake}}, got {error:?}"
                );
            }
            assert!(child.shared.lock().instances.is_empty());
            // No `RemoveInstance` — nothing was ever installed on
            // the driver for this video entry.
            assert!(matches!(
                driver_rx.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
        }
    }

    /// The mid-mint race on the video create: `converge_child`
    /// flips the still-`Creating` entry `Failed` (silently — no
    /// notification on a `Creating` entry), then the `Ok` reply
    /// lands. The `Live` flip must not regress the converged
    /// entry — `create_video_instance` folds into rollback:
    /// `SpawnFailed` carrying the convergence's recorded cause,
    /// entry removed, no `RemoveInstance`.
    #[test]
    fn create_video_instance_ok_then_converge_rolls_back() {
        let (child, driver_rx) = create_harness();
        let diag = Arc::new(diag::DiagRegistry::new());
        let (seq, join) = spawn_video_create(&child, &driver_rx, video_config(64, 48), &diag);
        let waiter = child
            .shared
            .lock()
            .waiters
            .remove(&seq)
            .expect("waiter registered");
        converge_child(&child.shared, None, FailureCause::ExitObserved);
        let _ = waiter.tx.send(Ok(wire::Reply::Ok));
        assert!(matches!(
            join.join().expect("create_video_instance joined"),
            Err(HostError::SpawnFailed {
                cause: FailureCause::ExitObserved,
                ..
            })
        ));
        {
            let inner = child.shared.lock();
            assert!(!inner.instances.contains_key(&0));
            assert!(matches!(inner.terminal, Some(Terminal::Failed)));
        }
        assert!(matches!(
            driver_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    /// A full child's `create_video_instance` answers
    /// `Err(InstanceLimit)` — the same capacity fact
    /// `create_instance` propagates — before the create verb ever
    /// reaches the wire.
    #[test]
    fn create_video_instance_on_full_child_is_instance_limit() {
        let child = L1Child::test_fixture(wire::WIRE_MAX_INSTANCES);
        let spec = video_spec();
        assert!(matches!(
            child.create_video_instance(
                &spec.view(),
                video_config(64, 48),
                &diag::DiagRegistry::new()
            ),
            Err(HostError::InstanceLimit)
        ));
        assert_eq!(child.live_count(), wire::WIRE_MAX_INSTANCES as usize);
    }

    // ---------------------------------------------------------------
    // Issue #121 pins — the audio-counter nine-case rows that live
    // above `RtLeg` (reader-side `push_result`, entry lifecycle),
    // and the failure-cause semantic table (reader/demux/reap/
    // timeout classification + first-observed convergence).
    // ---------------------------------------------------------------

    /// `results_dropped` counts *only* enqueue failures: a
    /// `push_result` that lands is +0 even when no RT leg ever
    /// observes it (the design's "not a total-loss metric" pin);
    /// a `try_push` failure is +1 each.
    #[test]
    fn push_result_counts_only_enqueue_failures() {
        let (inst, _notify_rx, mut result_rx) = inst_shared(LifecycleState::Processing);
        let readers = inst.audio_counter_readers().expect("an audio entry");
        // A successful enqueue never observed by the RT leg —
        // exactly the window this counter must NOT count.
        inst.push_result(7);
        assert_eq!(readers.results_dropped.read(), 0);
        // Fill the bounded channel so the next enqueue fails.
        let InstKind::Audio { result_tx, .. } = &inst.kind else {
            panic!("inst_shared builds an Audio entry")
        };
        while lock_mutex(result_tx).try_push(0).is_ok() {}
        inst.push_result(8);
        assert_eq!(readers.results_dropped.read(), 1);
        inst.push_result(9);
        assert_eq!(readers.results_dropped.read(), 2);
        // Freeing a slot makes the next enqueue succeed — +0 again.
        assert!(result_rx.try_pop().is_ok());
        inst.push_result(10);
        assert_eq!(readers.results_dropped.read(), 2);
    }

    /// The counter pairs live on the registry entry, not the leg:
    /// leg-side clones checked out for a swap share the cells the
    /// query surface reads, a re-minted entry counts fresh from
    /// zero, and a removed entry answers `None`.
    #[test]
    fn audio_counters_outlive_legs_remint_at_zero() {
        let (inst, _notify_rx, _result_rx) = inst_shared(LifecycleState::Processing);
        let readers = inst.audio_counter_readers().expect("an audio entry");
        // Two leg-side clones — the checkout/restore shape — write
        // into the same cells (values accumulate, never reset).
        inst.audio_leg_counters()
            .expect("leg counters")
            .passthrough_submits
            .increment();
        inst.audio_leg_counters()
            .expect("leg counters")
            .passthrough_submits
            .increment();
        inst.audio_leg_counters()
            .expect("leg counters")
            .dropped_submits
            .increment();
        assert_eq!(readers.passthrough_submits.read(), 2);
        assert_eq!(readers.dropped_submits.read(), 1);
        assert_eq!(readers.results_failed_observed.read(), 0);
        // A re-minted handle's fresh entry counts from zero — the
        // previous generation's totals never bleed through.
        let (reminted, _, _) = inst_shared(LifecycleState::Processing);
        let reminted_readers = reminted.audio_counter_readers().expect("an audio entry");
        assert_eq!(reminted_readers.passthrough_submits.read(), 0);
        assert_eq!(reminted_readers.dropped_submits.read(), 0);
        assert_eq!(reminted_readers.results_dropped.read(), 0);
        // The query goes `None` once the entry is gone — post-death
        // diagnostics are stderr retention's job, not the counters'.
        let instance = L1Instance::test_fixture(LifecycleState::Processing, Some(8));
        assert!(instance.audio_counters().is_some());
        remove_instance(&mut instance.shared.lock(), 0);
        assert!(instance.audio_counters().is_none());
    }

    /// The `stdout` half of an emitter child — `reader_main`'s
    /// wire codec driven by a real process pipe. Only a spawned
    /// child's stdout works portably: on Windows `ChildPipe::read`
    /// issues overlapped `ReadFileEx`, whose completion never
    /// arrives on a `std::io::pipe` (synchronous) handle — reads
    /// on such a pipe hang forever. The returned `Child` should be
    /// `wait`ed on after `reader_main` returns to reap it.
    fn emitting_stdout(bytes: &[u8]) -> (Child, ChildStdout) {
        let mut child = emitting_child(bytes);
        let stdout = child.stdout.take().expect("the emitter's stdout is piped");
        (child, stdout)
    }

    /// A child that writes exactly `bytes` to stdout and exits —
    /// POSIX `printf` octal escapes are the binary-safe emitter
    /// (`\0` for NUL, `\0NNN` for the rest; `{:o}`'s minimal digits
    /// are unambiguous since every escape starts with a backslash).
    #[cfg(unix)]
    fn emitting_child(bytes: &[u8]) -> Child {
        use std::fmt::Write as _;
        let mut script = String::from("printf '");
        for &byte in bytes {
            write!(script, "\\0{byte:o}").unwrap();
        }
        script.push('\'');
        ProcCommand::new("sh")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the byte emitter")
    }

    /// Windows twin — `cmd` has no binary-safe emitter, so
    /// powershell writes the array to its (piped) standard stream.
    #[cfg(windows)]
    fn emitting_child(bytes: &[u8]) -> Child {
        let script = if bytes.is_empty() {
            String::from("exit")
        } else {
            let list = bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "$s=[Console]::OpenStandardOutput();\
                 $s.Write([byte[]]({list}),0,{});$s.Flush()",
                bytes.len()
            )
        };
        ProcCommand::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the byte emitter")
    }

    /// A `ChildStdin` write end whose read half is closed — the
    /// `driver_main` stdin double; the `Disconnected` arm under
    /// test never writes anyway.
    fn dead_stdin() -> ChildStdin {
        let (reader, writer) = std::io::pipe().expect("pipe");
        drop(reader);
        child_stdin(writer)
    }

    /// The `ChildStdin` a spawned child would carry — it only ever
    /// gets dropped in these tests, so the platform's
    /// owned-descriptor conversion is enough (no `WriteFileEx`
    /// ever runs on it).
    #[cfg(unix)]
    fn child_stdin(writer: std::io::PipeWriter) -> ChildStdin {
        let fd: std::os::fd::OwnedFd = writer.into();
        ChildStdin::from(fd)
    }

    /// Windows twin of [`child_stdin`].
    #[cfg(windows)]
    fn child_stdin(writer: std::io::PipeWriter) -> ChildStdin {
        let handle: std::os::windows::io::OwnedHandle = writer.into();
        ChildStdin::from(handle)
    }

    /// A child that exits on its own with status 3 — `sh`/`cmd`
    /// are the portable instant-exit shells.
    #[cfg(unix)]
    fn exited_child() -> Child {
        ProcCommand::new("sh")
            .args(["-c", "exit 3"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn an immediately-exiting child")
    }

    /// Windows twin of [`exited_child`].
    #[cfg(windows)]
    fn exited_child() -> Child {
        ProcCommand::new("cmd")
            .args(["/c", "exit 3"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn an immediately-exiting child")
    }

    /// A child that blocks until killed — `cat`/`findstr` read
    /// their piped stdin forever (never written, never EOF'd).
    #[cfg(unix)]
    fn blocking_child() -> Child {
        ProcCommand::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn a stdin-blocking child")
    }

    /// Windows twin of [`blocking_child`].
    #[cfg(windows)]
    fn blocking_child() -> Child {
        ProcCommand::new("findstr")
            .arg("^")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn a stdin-blocking child")
    }

    /// Classification table row 1 — a stdout EOF on the first
    /// frame is `Transport` (kind ≠ `InvalidData`), converged
    /// child-scope like every reader-observed death.
    #[test]
    fn reader_first_frame_eof_classifies_transport() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        let (hello_tx, hello_rx) = mpsc::channel();
        let (mut child, stdout) = emitting_stdout(&[]);
        reader_main(stdout, &fixture.shared, &hello_tx);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Transport));
        assert!(matches!(hello_rx.try_recv(), Ok(Err(()))));
        assert!(matches!(
            fixture.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
        let _ = child.wait();
    }

    /// Classification table row 2 — an undecodable frame is
    /// `Protocol` (`InvalidData`), whether it arrives as the first
    /// frame or mid-stream after a valid `Hello` (phase-independent).
    #[test]
    fn reader_undecodable_frame_classifies_protocol() {
        // Over-limit declared length — `read_frame` → `InvalidData`.
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        let (hello_tx, hello_rx) = mpsc::channel();
        let (mut child, stdout) = emitting_stdout(&u32::MAX.to_le_bytes());
        reader_main(stdout, &fixture.shared, &hello_tx);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Protocol));
        assert!(matches!(hello_rx.try_recv(), Ok(Err(()))));
        let _ = child.wait();

        // Post-`Hello`: a valid handshake then an undecodable
        // frame — the same boundary, not `Handshake`.
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        let (hello_tx, hello_rx) = mpsc::channel();
        let mut stream = Vec::new();
        wire::encode(
            &wire::Message::Hello {
                version: wire::WIRE_VERSION,
                capabilities: 0,
            },
            &mut stream,
        )
        .unwrap();
        stream.extend_from_slice(&u32::MAX.to_le_bytes());
        let (mut child, stdout) = emitting_stdout(&stream);
        reader_main(stdout, &fixture.shared, &hello_tx);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Protocol));
        assert_eq!(hello_rx.try_recv().unwrap(), Ok(wire::WIRE_VERSION));
        let _ = child.wait();
    }

    /// Classification table row 3 — a well-formed non-`Hello`
    /// first frame is `Handshake`, not `Protocol`/`Transport` (the
    /// semantic boundary, not the read path it arrived on).
    #[test]
    fn reader_non_hello_first_frame_classifies_handshake() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        let (hello_tx, hello_rx) = mpsc::channel();
        let mut stream = Vec::new();
        wire::encode(&wire::Message::Shutdown, &mut stream).unwrap();
        let (mut child, stdout) = emitting_stdout(&stream);
        reader_main(stdout, &fixture.shared, &hello_tx);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Handshake));
        assert!(matches!(hello_rx.try_recv(), Ok(Err(()))));
        let _ = child.wait();
    }

    /// Classification table row 4 — a command-reply timeout is a
    /// host deadline decision: `Timeout` on the capture (the
    /// `Ipc` the caller sees is the post-mint surface).
    #[test]
    fn request_reply_timeout_classifies_timeout() {
        let (child, _driver_rx) = create_harness();
        // The driver channel is held but never answered — the
        // waiter times out under the explicit bound.
        let result = child.request_timed(
            wire::WIRE_INSTANCE_NONE,
            wire::Command::Scan,
            Duration::from_millis(20),
        );
        assert!(matches!(result, Err(HostError::Ipc)));
        assert_eq!(child.shared.diag.cause(), Some(FailureCause::Timeout));
    }

    /// Classification table row 5 — the driver's `try_wait` watch
    /// observing the child's exit converges `ExitObserved` (the
    /// status itself travels separately on `ChildExit`).
    #[test]
    fn driver_reap_observation_classifies_exit_observed() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        fixture
            .shared
            .child
            .set(Mutex::new(exited_child()))
            .expect("the child installs once");
        // `Disconnected` — the owner hung up mid-life: the driver's
        // bounded teardown kills (a no-op here), reaps the real
        // status and converges `ExitObserved`.
        let (tx, rx) = mpsc::channel::<DriverCmd>();
        drop(tx);
        driver_main(&fixture.shared, dead_stdin(), &rx, Duration::ZERO);
        assert_eq!(
            fixture.shared.diag.cause(),
            Some(FailureCause::ExitObserved)
        );
        assert!(matches!(
            fixture.shared.lock().terminal,
            Some(Terminal::Failed)
        ));
    }

    /// Classification table row 6 — a `try_wait`/`wait` failure as
    /// the *first* observation still lands `Internal`: the cause
    /// is always defined.
    #[test]
    fn reap_without_child_classifies_internal() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        let exit = reap(&fixture.shared, Some(Instant::now()));
        assert_eq!(exit, ChildExit::default());
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Internal));
    }

    /// The ordering pin — `Timeout` is recorded BEFORE the kill
    /// issues: the first-observed `Timeout` survives the kill's
    /// own reap and any later exit/EOF observation (first-wins).
    #[test]
    fn reap_deadline_records_timeout_before_kill() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        fixture
            .shared
            .child
            .set(Mutex::new(blocking_child()))
            .expect("the child installs once");
        // The deadline already passed — the first `try_wait` finds
        // a live child, so `Timeout` is recorded, THEN the kill
        // issues and `wait` collects the real status.
        let exit = reap(&fixture.shared, Some(Instant::now()));
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Timeout));
        assert_ne!(
            exit,
            ChildExit::default(),
            "the watchdog kill actually reaped the child"
        );
        // A late exit observation can never overwrite it.
        converge_child(&fixture.shared, Some(exit), FailureCause::ExitObserved);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Timeout));
    }

    /// First-observed survives the whole reap path — a `Transport`
    /// fixed by the earlier observation stands through the reap's
    /// own `Internal` record and the closing `ExitObserved`.
    #[test]
    fn first_observed_cause_survives_converge_and_reap() {
        let fixture = DemuxFixture::live(0, LifecycleState::Processing);
        converge_and_reap(&fixture.shared, FailureCause::Transport);
        assert_eq!(fixture.shared.diag.cause(), Some(FailureCause::Transport));
    }
}
