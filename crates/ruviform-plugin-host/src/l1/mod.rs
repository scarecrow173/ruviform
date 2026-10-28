//! The `pub(crate)` L1 out-of-process instance machinery — one child
//! process per instance, a framed stdio control plane, two I/O threads
//! and the idempotent `Failed` convergence required by the L1 design
//! (`docs/designs/plugin-l1-out-of-process.md`).
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
//! ## Thread model (per instance = 1 child + 2 threads)
//!
//! | thread | owns | duties |
//! | --- | --- | --- |
//! | `driver` | `Shared.child` (`Mutex<Child>` — operated only here, but shared so a converging thread can `kill` a wedged child even while this thread is blocked in `write_all`) · child stdin | serializes outgoing frames · drains the data-plane `submit` channel (stale-generation or post-terminal seqs are dropped *with a synthesized `mark_failed` result* through the shared `result_tx` so the leg's alternation gate never wedges): each doorbell'd seq becomes a shm in-slot write + `BlockIn` · `try_wait` crash watch every [`DRIVER_POLL_INTERVAL`] · shutdown watchdog (grace → `child.kill()`) |
//! | `reader` | child stdout | demuxes child→parent frames (unflagged `Reply` → keyed waiter wake · flagged `Reply` → `mark_failed` block completion · `BlockDone` → shm out-slot read + result-channel push) · first frame must be `Hello` · EOF → `seen_eof` convergence path |
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
//! a long hold. `Shared.result_tx` — the result-channel producer
//! shared by both I/O threads — is a leaf mutex held for a single
//! bounded `try_push` and never nested inside `inner`/`child`.
//!
//! ## `Failed` convergence
//!
//! Crash detection has two paths that converge idempotently:
//! (i) the reader observes stdout EOF (`seen_eof`), (ii) the driver's
//! `Child::try_wait` returns `Some(ExitStatus)` (reaping as it goes).
//! Whichever fires first performs the one-time transition —
//! `terminal = Failed`, the local lifecycle mirror moves to
//! `LifecycleState::Failed`, every pending command waiter is woken
//! with `Err(HostError::Ipc)`, and exactly one `ChildExit`
//! notification is queued. Any exit while the instance is live —
//! including a voluntary `exit(0)` — is `Failed` (a protocol
//! violation); only an exit after `Shutdown` was initiated is a clean
//! teardown (`terminal = Clean`, mirror → `Destroyed`, no
//! notification). There is no automatic restart.
//!
//! Protocol-level failures (a malformed or oversized frame, a second
//! `Hello`, an unexpected `BlockDone`, a stdin write error, a
//! `command_timeout`) converge the same way: the converging thread
//! itself kills the child through `Shared.child` — that is what
//! unwedges a driver blocked in a pipe `write_all` on a live-but-
//! not-reading child (the kill turns the write into EPIPE) — and
//! pending waiters wake `Err(HostError::Ipc)`.

pub(crate) mod data;
pub(crate) mod shm;

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::process::{Child, ChildStdin, ChildStdout, Command as ProcCommand, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, HostError, HostInput, HostOutput, LifecycleState, PluginDescriptor, PrepareError,
    ProcessConfig, ProcessResult, TransportInfo,
};

/// The driver's crash-watch and command-drain poll interval
/// (~200 µs per the design; the effective granularity is the OS
/// scheduler's, which is coarser — the interval paces a `try_wait`
/// poll, it is not a deadline).
const DRIVER_POLL_INTERVAL: Duration = Duration::from_micros(200);

/// Poll cadence for the bounded driver join in
/// [`L1Instance::join_threads`].
const DRIVER_JOIN_POLL: Duration = Duration::from_millis(2);

/// Extra time past `command_timeout` (the shutdown grace) that
/// `join_threads` waits for the driver before force-killing a wedged
/// child — covers the driver's own watchdog kill + reap turnaround.
const DRIVER_JOIN_MARGIN: Duration = Duration::from_secs(1);

/// Process-wide instance counter feeding [`shm::unique_key`]'s `slot`
/// component (the composite's public slot numbers live in
/// `crate::composite` and are not shared here — this only needs to
/// discriminate instances for key uniquing).
static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The dedicated route's fixed registry handle: wire-v2 stamps every
/// instance-scoped envelope of a capacity-1 child with it (`Scan`
/// bypasses it with `WIRE_INSTANCE_NONE`). T2/T3's route pools mint
/// per-bundle handles; `L1Instance::instance` then carries the mint
/// instead of this constant.
const DEDICATED_INSTANCE: u32 = 0;

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
    /// [`L1Instance::spawn`] (`Err(Ipc)`): the just-submitted seq
    /// could never already have a result, so the leg would pass
    /// through forever. `D` also travels as `CreateInstance`'s
    /// `in_depth`/`out_depth`, so the codec's
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
/// stamped with plus the unbounded channel the reader sends the
/// child's reply (or the converger sends `Err(Ipc)`) onto. The
/// reply's `instance` must echo the expected value — a mismatch is
/// an attribution failure, converged `Failed` like any other
/// child-scope violation (Issue #117 demux table).
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
    /// converging thread force-kills the child itself, so the driver
    /// always gets to reap a real status and the notification can
    /// never dangle).
    Pending,
    /// The one-shot notification was already queued.
    Sent,
}

/// Mutex-protected proxy state shared by the caller, driver and
/// reader threads. Locking is legal everywhere here — none of these
/// threads is an RT thread (the RT leg in `l1::data` never takes
/// this mutex; it reads the [`Shared::rt_state`] snapshot instead).
struct Inner {
    /// In-flight commands by correlation seq.
    waiters: HashMap<u64, Waiter>,
    /// Local lifecycle mirror — updated on successful lifecycle
    /// commands and by convergence; the child's `Lifecycle` stays the
    /// authority, this mirror only reflects reported outcomes.
    /// Kept in lock-step with `Shared::rt_state` via
    /// [`Shared::set_mirror`].
    mirror: LifecycleState,
    /// Doorbell seqs the driver has written `BlockIn` for and the
    /// reader has not yet completed — the `BlockDone` / flagged
    /// `Reply` legality set. Seqs are partitioned by
    /// [`data::BLOCK_SEQ_FLAG`], so they can never alias `waiters`
    /// keys.
    block_in_flight: HashSet<u64>,
    /// The armed data-plane staging ring — `Some` after a successful
    /// `Activate`, replaced on re-activation. `Arc` so the I/O
    /// threads borrow it under the lock without holding the lock
    /// across shm work.
    staging: Option<Arc<data::Staging>>,
    /// `Some` once convergence ran — exactly once.
    terminal: Option<Terminal>,
    /// `Shutdown` was written on the wire — a later child exit is
    /// clean teardown rather than `Failed`. Set only on the driver,
    /// immediately before the `Shutdown` write and only while no
    /// terminal condition has converged, so a child that crashed
    /// before the wire `Shutdown` still converges `Failed` and keeps
    /// its notification.
    shutdown_initiated: bool,
    /// The reader observed stdout EOF (one `Failed` path).
    seen_eof: bool,
    /// One-shot `Failed` notification state.
    notify: Notify,
}

/// Shared proxy state: the mutex-guarded [`Inner`], the shared child
/// handle and the notification queue endpoint threads push
/// `ChildExit` into.
struct Shared {
    inner: Mutex<Inner>,
    /// The const-cheap lifecycle snapshot the RT leg inspects —
    /// written with `Release` by [`Shared::set_mirror`] at every
    /// `Inner::mirror` mutation, read lock-free with `Acquire` by
    /// `L1Instance::process`. `Arc` so a detached endpoint holds
    /// the same atomic and observes identical mirror updates.
    /// Fails closed: the RT leg never needs the mutex.
    rt_state: Arc<AtomicU8>,
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
    notify_tx: mpsc::Sender<ChildExit>,
    /// The result-channel producer — shared because *two* non-RT
    /// threads complete seqs: the reader pushes real `BlockDone`/
    /// rejection completions, and the driver pushes a synthesized
    /// `mark_failed` completion for every `submit` it drops without
    /// a doorbell (a stale-generation or post-terminal seq never
    /// enters `block_in_flight`, so it can never produce a real
    /// result — and an unanswered seq wedges its slot's
    /// strict-alternation gate forever). `Producer` is
    /// single-owner, hence the `Mutex`: both lockers are non-RT
    /// threads and the critical section is a single bounded
    /// `try_push`. A leaf lock — never held while acquiring
    /// `inner` or `child` — so the disjoint-sections rule in the
    /// module docs is preserved.
    result_tx: Mutex<ruviform_rt::Producer<u64>>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the installed child handle (poison-tolerant like
    /// [`Shared::lock`]). `None` only before the driver installs the
    /// child — unreachable once `driver_main` runs.
    fn child_lock(&self) -> Option<MutexGuard<'_, Child>> {
        self.child.get().map(|child| {
            child
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        })
    }

    /// The single mirror-mutation point: updates `Inner::mirror` and
    /// publishes the same state to the lock-free `rt_state` snapshot
    /// the RT leg reads. `Release` pairs the `Acquire` load in
    /// `L1Instance::process`.
    fn set_mirror(&self, inner: &mut Inner, state: LifecycleState) {
        inner.mirror = state;
        self.rt_state
            .store(data::mirror_encode(state), Ordering::Release);
    }

    /// Pushes one element onto the RT leg's result channel — the
    /// shared-producer entry point both I/O threads use (the reader
    /// for real completions, the driver for synthesized
    /// `mark_failed` completions on doorbell-less drops). Channel
    /// capacity is `D` and at most `D` seqs are ever in flight —
    /// each yields exactly one completion element — so `try_push`
    /// cannot overflow in-protocol; a `Full` is tolerated like the
    /// reader's own pushes. Poison-tolerant like [`Shared::lock`].
    fn push_result(&self, el: u64) {
        let _ = self
            .result_tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .try_push(el);
    }
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

/// Orders for the driver thread — it serializes outgoing frames and
/// runs the process lifecycle operations through this channel.
enum DriverCmd {
    /// Serialize and write a `Command` frame — `instance` is the
    /// stamped wire-v2 envelope (`WIRE_INSTANCE_NONE` for `Scan`,
    /// the dedicated handle `0` for everything else under T1).
    Command {
        seq: u64,
        instance: u32,
        cmd: wire::Command,
    },
    /// Write `Shutdown`, then give the child `command_timeout` of
    /// grace before `kill()`.
    Shutdown,
}

/// The idempotent convergence every terminal path funnels through.
/// `exit` is `Some` when the caller observed the final `ExitStatus`
/// (the driver's `try_wait`/`reap`) and `None` when it did not
/// (stdout EOF, protocol violation, write error, timeout).
///
/// First caller wins the transition; on `Failed` it queues the
/// single `ChildExit` notification — immediately when the status is
/// already known, or once the driver reaps it ([`Notify::Pending`]),
/// so the notification always carries the real exit status
/// regardless of which crash path fired first. Either way pending
/// waiters are woken with `Err(HostError::Ipc)` — a command can never
/// hang on a dead context.
///
/// A `Failed` transition also force-kills the child *here* — through
/// `Shared.child`, not via an order on the driver's channel: a wedged
/// child that stops draining stdin leaves the driver blocked in
/// `write_all`
/// (frames can exceed the ~64 KiB pipe capacity), and this direct
/// kill is what turns that write into EPIPE and lets the driver
/// unwind. `kill` is uncatchable on both OSes, so the driver's
/// `try_wait`/`reap` then observes the real exit status and a
/// `Pending` notification completes with it.
fn converge(shared: &Shared, exit: Option<ChildExit>) {
    let failed = {
        let mut inner = shared.lock();
        if inner.terminal.is_some() {
            // Already converged — but a late exit status still
            // completes a pending notification, exactly once.
            if matches!(inner.terminal, Some(Terminal::Failed))
                && inner.notify == Notify::Pending
                && let Some(exit) = exit
            {
                inner.notify = Notify::Sent;
                let _ = shared.notify_tx.send(exit);
            }
            return;
        }
        let failed = !inner.shutdown_initiated;
        inner.terminal = if failed {
            Some(Terminal::Failed)
        } else {
            Some(Terminal::Clean)
        };
        shared.set_mirror(
            &mut inner,
            if failed {
                LifecycleState::Failed
            } else {
                LifecycleState::Destroyed
            },
        );
        // Unbounded-channel sends never block — safe under the lock,
        // and keeping them inside the critical section makes
        // "exactly once" trivially atomic.
        for (_, waiter) in inner.waiters.drain() {
            let _ = waiter.tx.send(Err(HostError::Ipc));
        }
        if failed {
            inner.notify = match exit {
                Some(exit) => {
                    let _ = shared.notify_tx.send(exit);
                    Notify::Sent
                }
                None => Notify::Pending,
            };
        }
        failed
    };
    if failed {
        // Disjoint critical sections only — `inner` is already
        // released before `child` is taken (module lock order).
        kill_child(shared);
    }
}

/// Spawns the configured child and takes its piped stdio — run on the
/// driver thread so `Child` ownership never crosses a thread boundary.
/// A failure after a successful `spawn` (theoretically unreachable
/// with `Stdio::piped`) kills + reaps before reporting, so no child
/// ever leaks.
fn spawn_child(command: &mut ProcCommand) -> Result<(Child, ChildStdin, ChildStdout), ()> {
    let mut child = command.spawn().map_err(|_| ())?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(());
    };
    Ok((child, stdin, stdout))
}

/// The reader thread: first frame must be `Hello`, afterwards it
/// demultiplexes three reply families — unflagged `Reply` frames
/// wake their keyed command waiters, `BlockDone` completes a
/// doorbell (shm out-slot read plus result-channel push), and a
/// `Reply` whose seq carries [`data::BLOCK_SEQ_FLAG`] is the
/// child's block-rejection path (slot parse failure, lifecycle
/// rejection, plugin panic) — also a block completion, marked so
/// the RT leg passes through. Any other message kind, an unreadable
/// stream or EOF converges — EOF is the `seen_eof` `Failed` path.
///
/// `view` is this thread's own shm mapping (the `Shmem` handle is
/// `!Send`, so each I/O thread opens the owner `Segment`'s key
/// locally); `in_depth`/`out_depth` are the spawn-time ring sizes;
/// `self_instance` is the wire-v2 handle this child serves (the only
/// legal `instance` on block frames — `Scan` replies still echo
/// `WIRE_INSTANCE_NONE`, attributed by the waiter's stamp). Result
/// pushes go through the shared [`Shared::result_tx`]
/// producer — the driver completes dropped seqs on the same
/// channel.
fn reader_main(
    mut stdout: ChildStdout,
    shared: &Shared,
    hello_tx: &mpsc::Sender<Result<u16, ()>>,
    view: &shm::View,
    in_depth: u32,
    out_depth: u32,
    self_instance: u32,
) {
    // One scratch allocation reused across frames — the wire layer
    // owns the framing; EOF and protocol violations share the
    // `Err`/`Failed` path here.
    let mut frame_buf = Vec::new();
    if let Ok(wire::Message::Hello { version, .. }) =
        wire::read_message(&mut stdout, &mut frame_buf)
    {
        let _ = hello_tx.send(Ok(version));
    } else {
        // Missing `Hello` or a dead pipe before it — handshake
        // failure surfaces through `hello_tx` and convergence.
        let _ = hello_tx.send(Err(()));
        converge(shared, None);
        return;
    }
    loop {
        match wire::read_message(&mut stdout, &mut frame_buf) {
            Ok(wire::Message::Reply { seq, instance, .. }) if data::is_block_seq(seq) => {
                // A block rejection: the child answered the flagged
                // seq with an `Err` `Reply` instead of `BlockDone` —
                // the block still completes (marked failed) so the
                // strict alternation can proceed. A `BlockIn` was
                // only ever stamped with this child's `instance`
                // handle — a foreign echo is inert, not a violation.
                if instance != self_instance {
                    continue;
                }
                let known = shared.lock().block_in_flight.remove(&seq);
                if !known {
                    converge(shared, None);
                    return;
                }
                // Channel capacity is `depth` and at most `depth`
                // seqs are ever in flight — `try_push` cannot
                // overflow; a full push means the RT side already
                // drained past this seq, which is equally harmless.
                shared.push_result(data::mark_failed(seq));
            }
            Ok(wire::Message::Reply {
                seq,
                instance,
                result,
            }) => {
                let waiter = shared.lock().waiters.remove(&seq);
                if let Some(waiter) = waiter {
                    // A stale/unknown seq (timed-out waiter already
                    // removed) is dropped — the reply can no longer
                    // hurt anyone. A live waiter attributes the
                    // reply by `instance`: the echo must match the
                    // request's stamped envelope — a mismatch is the
                    // demux table's child-scope violation, so the
                    // waiter is gone and the instance converges
                    // `Failed` rather than satisfying the wrong
                    // command.
                    if waiter.instance != instance {
                        converge(shared, None);
                        return;
                    }
                    let _ = waiter.tx.send(result);
                }
            }
            Ok(wire::Message::BlockDone {
                seq,
                instance,
                result,
            }) => {
                // A doorbell reply must carry a flagged seq this
                // driver actually rang — an unflagged or unknown one
                // is a protocol violation. Like the flagged `Reply`
                // arm, a foreign `instance` echo is inert: the
                // doorbell was stamped `self_instance` on the way
                // out, so an off-instance `BlockDone` cannot belong
                // to this ring and is dropped.
                if instance != self_instance {
                    continue;
                }
                let known = data::is_block_seq(seq) && shared.lock().block_in_flight.remove(&seq);
                if !known {
                    converge(shared, None);
                    return;
                }
                // Acquire edge — pairs the child's `fence(Release)`
                // before `BlockDone`; nothing below may be reordered
                // above it. A real atomic fence (not a compiler
                // fence): the cross-process ordering rests on this
                // hardware fence plus the pipe-read syscall boundary.
                fence(Ordering::Acquire);
                if let Some(staging) = shared.lock().staging.clone() {
                    let slot = usize::try_from(seq % u64::from(out_depth))
                        .expect("slot index always fits usize");
                    let _published = data::read_out_slot(
                        view.out_slot(in_depth, out_depth, seq),
                        seq,
                        &staging.outs[slot],
                        staging.frame_capacity,
                        result,
                    );
                    // `_published == false` means the child wrote a
                    // mismatched slot seq — the data is withheld and
                    // the RT leg's tag check fails closed; the done
                    // still completes the alternation either way.
                }
                shared.push_result(seq);
            }
            Ok(_) => {
                // `Hello` twice or a parent→child kind arriving
                // child→parent: protocol violation — the execution
                // context is failed.
                converge(shared, None);
                return;
            }
            Err(_) => {
                {
                    let mut inner = shared.lock();
                    inner.seen_eof = true;
                }
                converge(shared, None);
                return;
            }
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
                    kill_child(shared);
                    return shared
                        .child_lock()
                        .and_then(|mut child| child.wait().ok())
                        .map(ChildExit::from_status)
                        .unwrap_or_default();
                }
            }
            Err(()) => return ChildExit::default(),
        }
        thread::sleep(DRIVER_POLL_INTERVAL);
    }
}

/// The driver thread: operates the shared `Child` handle and owns
/// the child's stdin. Drains the command channel on a
/// `DRIVER_POLL_INTERVAL` cadence, watches `try_wait` for the crash
/// path, runs the shutdown watchdog and drains the data-plane
/// `submit` channel — every popped seq becomes an in-slot shm write
/// plus a `BlockIn` doorbell (this thread does all the blocking
/// pipe work; the RT caller only `try_push`es seqs).
///
/// `view` is this thread's own shm mapping (`Shmem` is `!Send` —
/// each I/O thread opens the owner `Segment`'s key locally),
/// `in_depth` sizes the in-slot ring and `self_instance` is the
/// wire-v2 handle every `BlockIn` doorbell is stamped with.
// Thread-entry plumbing carries the whole per-child context — the
// same allow `spawn_driver`/`spawn_reader` record.
#[allow(clippy::too_many_arguments)]
fn driver_main(
    shared: &Shared,
    mut stdin: ChildStdin,
    rx: &mpsc::Receiver<DriverCmd>,
    shutdown_grace: Duration,
    submit: &mut ruviform_rt::Consumer<data::Submit>,
    view: &mut shm::View,
    in_depth: u32,
    self_instance: u32,
) {
    let mut shutdown_deadline: Option<Instant> = None;
    // One encode scratch reused for every outbound frame — commands,
    // the `Shutdown`, and each `BlockIn` doorbell.
    let mut encode_buf = Vec::new();
    loop {
        match rx.recv_timeout(DRIVER_POLL_INTERVAL) {
            Ok(DriverCmd::Command { seq, instance, cmd }) => {
                // A converged instance (or teardown in flight) gets no
                // new wire messages — the spec forbids writes after
                // `Failed`; wake the waiter `Err(Ipc)` instead of
                // writing a doomed frame.
                let reject = shutdown_deadline.is_some() || shared.lock().terminal.is_some();
                if reject {
                    let waiter = shared.lock().waiters.remove(&seq);
                    if let Some(waiter) = waiter {
                        let _ = waiter.tx.send(Err(HostError::Ipc));
                    }
                } else if wire::write_message(
                    &mut stdin,
                    &wire::Message::Command { seq, instance, cmd },
                    &mut encode_buf,
                )
                .is_err()
                {
                    converge(shared, None);
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
                    let dead = matches!(try_wait_child(shared), Ok(Some(_)));
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
                converge(shared, Some(exit));
                return;
            }
        }
        // Data plane: every seq the RT caller submitted becomes an
        // in-slot write plus a `BlockIn` doorbell — bounded by the
        // channel capacity, never blocking on the RT side.
        if !drain_submits(
            shared,
            &mut stdin,
            submit,
            view,
            in_depth,
            self_instance,
            &mut encode_buf,
        ) {
            return;
        }
        match try_wait_child(shared) {
            Ok(Some(status)) => {
                converge(shared, Some(ChildExit::from_status(status)));
                return;
            }
            Ok(None) => {}
            Err(()) => {
                // `try_wait` itself failed — the child's state is
                // unknown. Force the death so POSIX never leaves a
                // zombie, then collect the status best-effort (a
                // second failure converges with the default
                // `ChildExit`, still completing `Notify::Pending`).
                kill_child(shared);
                let exit = reap(shared, Some(Instant::now()));
                converge(shared, Some(exit));
                return;
            }
        }
        if shutdown_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            let exit = reap(shared, Some(Instant::now()));
            converge(shared, Some(exit));
            return;
        }
    }
}

/// One driver poll's worth of `submit` drains: every popped seq
/// becomes an in-slot shm write plus a `BlockIn` doorbell stamped
/// with `self_instance`. A seq that is dropped instead (stale
/// generation, dead or shutting-down child) still owes the leg a
/// completion — the synthesized `mark_failed` below. Returns `false`
/// when a doorbell write failed and convergence ran — the driver
/// exits with it.
fn drain_submits(
    shared: &Shared,
    stdin: &mut ChildStdin,
    submit: &mut ruviform_rt::Consumer<data::Submit>,
    view: &mut shm::View,
    in_depth: u32,
    self_instance: u32,
    encode_buf: &mut Vec<u8>,
) -> bool {
    while let Ok(submit) = submit.try_pop() {
        let seq = submit.seq;
        let staging = {
            let mut inner = shared.lock();
            if inner.terminal.is_some() || inner.shutdown_initiated {
                // A dead or shutting-down child takes no new
                // doorbells — the seq is consumed here.
                None
            } else {
                let staging = data::armed_for(inner.staging.as_ref(), submit);
                if staging.is_some() {
                    // The doorbell goes in flight BEFORE the
                    // write — the reader's `BlockDone` /
                    // flagged-`Reply` must always find it here.
                    inner.block_in_flight.insert(seq);
                }
                // `None` here is either "no ring armed yet" or a
                // stale-generation submit — queued before a
                // re-activation swapped `inner.staging`, so
                // serializing it would doorbell the new ring's
                // zeroed stage under a real seq. Dropped without
                // a doorbell: `block_in_flight` never sees it.
                staging
            }
        };
        let Some(staging) = staging else {
            // A dropped seq was never doorbell'd — no shm write
            // happened, so no out-slot race exists — but the leg
            // still waits on a completion to clear the slot's
            // alternation gate (`cleared[i] >= last_pushed[i]`).
            // Left unanswered it would wedge the slot (then the
            // whole ring) forever: a fully-synced leg passing
            // through 100%. Synthesize the `mark_failed`
            // completion — one seq, exactly one element — so the
            // normal `drain_to` path frees the slot and resolves
            // the block as the defined pass-through.
            shared.push_result(data::mark_failed(seq));
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
        if wire::write_message(
            stdin,
            &wire::Message::BlockIn {
                seq,
                instance: self_instance,
            },
            encode_buf,
        )
        .is_err()
        {
            converge(shared, None);
            return false;
        }
    }
    true
}

/// The driver's spawn hand-off: `Ok(stdout)` carries the child's piped
/// stdout across to the caller (which hands it to the reader thread);
/// `Err(())` reports a spawn-time failure.
type SpawnRx = mpsc::Receiver<Result<ChildStdout, ()>>;

/// The reader's handshake channel: `Ok(version)` carries the child's
/// `Hello` version; `Err(())` reports a broken handshake/data plane.
type HelloRx = mpsc::Receiver<Result<u16, ()>>;

/// Spawns the `l1-driver` thread — the child process itself is spawned
/// *inside* the thread and handed to [`L1Instance::spawn`] through
/// `spawn_rx`: keeping the `Child` created and installed on the driver
/// from birth means no failure path between thread creation and
/// handshake can leak a running worker — every post-driver error
/// funnels through channel disconnect plus the shared-child kill, and
/// a driver-spawn failure happens before the child exists.
#[allow(clippy::too_many_arguments)]
fn spawn_driver(
    shared: &Arc<Shared>,
    mut command: ProcCommand,
    driver_rx: mpsc::Receiver<DriverCmd>,
    mut submit_rx: ruviform_rt::Consumer<data::Submit>,
    shm_key: String,
    depth: u32,
    grace: Duration,
    self_instance: u32,
) -> Result<(thread::JoinHandle<()>, SpawnRx), HostError> {
    let (spawn_tx, spawn_rx) = mpsc::channel();
    let driver_shared = Arc::clone(shared);
    let join = thread::Builder::new()
        .name("l1-driver".to_owned())
        .spawn(move || {
            let Ok((child, stdin, stdout)) = spawn_child(&mut command) else {
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
                let mut child = still_owned
                    .into_inner()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let _ = child.kill();
                let _ = child.wait();
                let _ = spawn_tx.send(Err(()));
                return;
            }
            // This thread's own shm mapping for the in-slot
            // writes — `Shmem` is `!Send`, so it opens the key
            // here rather than moving the owner `Segment`.
            let Ok(mut view) = shm::View::attach(&shm_key) else {
                kill_child(&driver_shared);
                if let Some(mut child) = driver_shared.child_lock() {
                    let _ = child.wait();
                }
                let _ = spawn_tx.send(Err(()));
                return;
            };
            // Ownership of `stdout` crosses to the reader thread
            // through the caller; the child itself never leaves.
            if spawn_tx.send(Ok(stdout)).is_err() {
                kill_child(&driver_shared);
                if let Some(mut child) = driver_shared.child_lock() {
                    let _ = child.wait();
                }
                return;
            }
            driver_main(
                &driver_shared,
                stdin,
                &driver_rx,
                grace,
                &mut submit_rx,
                &mut view,
                depth,
                self_instance,
            );
        })
        .map_err(|_| HostError::Ipc)?;
    Ok((join, spawn_rx))
}

/// Spawns the `l1-reader` thread: attaches its own shm `View` by key
/// *before* `Hello` so a broken data plane fails the spawn rather than
/// the first `BlockDone`, then runs [`reader_main`]. Result pushes go
/// through the shared [`Shared::result_tx`] producer.
fn spawn_reader(
    shared: &Arc<Shared>,
    stdout: ChildStdout,
    shm_key: String,
    depth: u32,
    self_instance: u32,
) -> Result<(thread::JoinHandle<()>, HelloRx), HostError> {
    let (hello_tx, hello_rx) = mpsc::channel();
    let reader_shared = Arc::clone(shared);
    let join = thread::Builder::new()
        .name("l1-reader".to_owned())
        .spawn(move || {
            let Ok(view) = shm::View::attach(&shm_key) else {
                let _ = hello_tx.send(Err(()));
                converge(&reader_shared, None);
                return;
            };
            reader_main(
                stdout,
                &reader_shared,
                &hello_tx,
                &view,
                depth,
                depth,
                self_instance,
            );
        })
        .map_err(|_| HostError::Ipc)?;
    Ok((join, hello_rx))
}

/// One L1 instance: a spawned child plus its `driver`/`reader` threads,
/// the notification queue and the local lifecycle mirror.
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
    shared: Arc<Shared>,
    driver_tx: Option<mpsc::Sender<DriverCmd>>,
    driver_join: Option<thread::JoinHandle<()>>,
    reader_join: Option<thread::JoinHandle<()>>,
    notify_rx: mpsc::Receiver<ChildExit>,
    command_timeout: Duration,
    next_seq: AtomicU64,
    /// The wire-v2 `instance` envelope handle this child's registry
    /// entry lives under — `0` under the T1 dedicated route
    /// (capacity-1: one child serves exactly one instance). T2/T3's
    /// route pools mint per-bundle handles here; `Scan` bypasses the
    /// handle with `WIRE_INSTANCE_NONE` instead.
    instance: u32,
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
    /// Spawns the child, starts `driver`/`reader` and runs the
    /// handshake: `Hello` (version must equal [`wire::WIRE_VERSION`])
    /// → `Scan` → the reported `wire::Descriptor`s must field-match
    /// `spec.descriptor`. Any failure — spawn error, timeout, version
    /// or descriptor mismatch — is `Err(HostError::Ipc)` with the
    /// child killed and reaped.
    ///
    /// **RT contract:** non-RT only — blocks up to
    /// `3 × command_timeout` in the worst case (the spawn hand-off,
    /// the `Hello` wait and the `Scan` reply each carry one).
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
    pub(crate) fn spawn(spec: &SpawnSpec) -> Result<Self, HostError> {
        if spec.lookahead_blocks == 0 || spec.lookahead_blocks >= wire::WIRE_MAX_ELEMENTS {
            return Err(HostError::Ipc);
        }
        // `lookahead_blocks < WIRE_MAX_ELEMENTS` here, so `+ 1`
        // cannot overflow and `depth` stays inside the codec cap.
        let depth = spec.lookahead_blocks + 1;
        let slot =
            u32::try_from(INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap_or(u32::MAX);
        let segment = shm::Segment::create(slot, depth, depth).map_err(|()| HostError::Ipc)?;
        // The data-plane SPSC legs — capacity `D`, so at most a full
        // ring of doorbells/results is ever in flight. `submit`:
        // RT caller → driver; `result`: reader → RT caller.
        let (submit_tx, submit_rx) =
            ruviform_rt::channel::<data::Submit>(usize::try_from(depth).unwrap_or(usize::MAX));
        let (result_tx, result_rx) =
            ruviform_rt::channel::<u64>(usize::try_from(depth).unwrap_or(usize::MAX));
        // Wire v2: no `--ruviform-*` bootstrap flags — the plugin id
        // and shm key travel in the `CreateInstance` payload during
        // `handshake`. `spec.args` still prepends the runner's own
        // flags (e.g. the test worker's fault knobs).
        let mut command = ProcCommand::new(&spec.program);
        command
            .args(&spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // stderr piping is a deferred design concern — inherit
            // keeps child diagnostics visible without extra plumbing.
            .stderr(Stdio::inherit());
        let (driver_tx, driver_rx) = mpsc::channel();
        let (notify_tx, notify_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                waiters: HashMap::new(),
                mirror: LifecycleState::Created,
                block_in_flight: HashSet::new(),
                staging: None,
                terminal: None,
                shutdown_initiated: false,
                seen_eof: false,
                notify: Notify::Idle,
            }),
            rt_state: Arc::new(AtomicU8::new(data::mirror_encode(LifecycleState::Created))),
            child: OnceLock::new(),
            notify_tx,
            result_tx: Mutex::new(result_tx),
        });
        // `Shmem` is `!Send` — each I/O thread opens its own `View`
        // of the owner segment by key on its own stack frame.
        let (driver_join, spawn_rx) = spawn_driver(
            &shared,
            command,
            driver_rx,
            submit_rx,
            segment.key().to_owned(),
            depth,
            spec.command_timeout,
            DEDICATED_INSTANCE,
        )?;
        let Ok(Ok(stdout)) = spawn_rx.recv_timeout(spec.command_timeout) else {
            // Child spawn failed or timed out: disconnecting the
            // command channel makes the driver kill/reap whatever
            // it started (bounded); it may not have spawned at all.
            drop(driver_tx);
            let _ = driver_join.join();
            return Err(HostError::Ipc);
        };
        let reader_spawn = spawn_reader(
            &shared,
            stdout,
            segment.key().to_owned(),
            depth,
            DEDICATED_INSTANCE,
        );
        let (reader_join, hello_rx) = match reader_spawn {
            Ok(pair) => pair,
            Err(error) => {
                // The driver is already running — kill the child
                // through the shared handle and disconnect; the
                // driver reaps and exits (bounded — `kill` is
                // forceful on both OSes).
                kill_child(&shared);
                drop(driver_tx);
                let _ = driver_join.join();
                return Err(error);
            }
        };

        let shm_key = segment.key().to_owned();
        let mut instance = Self {
            shared,
            driver_tx: Some(driver_tx),
            driver_join: Some(driver_join),
            reader_join: Some(reader_join),
            notify_rx,
            command_timeout: spec.command_timeout,
            next_seq: AtomicU64::new(0),
            instance: DEDICATED_INSTANCE,
            depth,
            rt: Some(data::RtLeg::new(
                depth,
                spec.lookahead_blocks,
                submit_tx,
                result_rx,
            )),
            staging_generation: 0,
            arm_tx: None,
            pending_arm: None,
            return_rx: None,
            arm_collector: None,
            endpoint_bound: None,
            activated_sample_rate: None,
            segment,
        };
        match instance.handshake(spec, &hello_rx, &shm_key) {
            Ok(()) => Ok(instance),
            Err(error) => {
                // Handshake failure: kill + reap, no notification —
                // the instance never went live.
                instance.abort();
                Err(error)
            }
        }
    }

    /// `Hello` (version check) → `Scan` → descriptor field-match →
    /// `CreateInstance` — the wire-v2 handshake sequence fixed by the
    /// Issue #117 design. `shm_key` names the segment this spawn
    /// already created; `CreateInstance` is the point the child's
    /// registry learns the handle.
    fn handshake(
        &mut self,
        spec: &SpawnSpec,
        hello_rx: &HelloRx,
        shm_key: &str,
    ) -> Result<(), HostError> {
        match hello_rx.recv_timeout(spec.command_timeout) {
            Ok(Ok(version)) if version == wire::WIRE_VERSION => {}
            _ => return Err(HostError::Ipc),
        }
        // Every handshake failure is `Ipc` — including a `Failed`
        // surfaced by `request` when the child died between `Hello`
        // and `Scan`.
        match self
            .request(wire::Command::Scan)
            .map_err(|_| HostError::Ipc)?
        {
            wire::Reply::Scanned(descriptors) => {
                if !descriptors.iter().any(|d| d.matches(&spec.descriptor)) {
                    return Err(HostError::Ipc);
                }
            }
            _ => return Err(HostError::Ipc),
        }
        // Registry registration carries the v1 argv bootstrap fields
        // — plugin selection + shm attachment — as the `CreateInstance`
        // payload. The reply is `Ok` on success; every defined
        // failure (`UnknownPluginId` after a field-match Scan means
        // the child is lying, a duplicate handle the host never
        // mints, `InstanceLimit` at capacity 1, `Ipc`, `PluginFailed`)
        // folds into the handshake's uniform `Err(Ipc)` — the spawn
        // is unusable, never a live-but-empty child.
        match self
            .request(wire::Command::CreateInstance {
                plugin_id: spec.descriptor.id.to_owned(),
                shm_key: shm_key.to_owned(),
                in_depth: self.depth,
                out_depth: self.depth,
            })
            .map_err(|_| HostError::Ipc)?
        {
            wire::Reply::Ok => Ok(()),
            _ => Err(HostError::Ipc),
        }
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
    /// `Err(HostError::Ipc)` and converges the instance to `Failed`.
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
            // the mirror `Failed`, never `Activated`).
            if inner.terminal.is_none() {
                self.shared.set_mirror(&mut inner, state);
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

    /// Publishes a freshly activated staging ring: `Inner::staging`
    /// for the I/O threads, then the RT consumer — `RtLeg::arm`
    /// when the leg is local, the arm channel while an endpoint
    /// holds it (`pending_arm` latest-wins on `Full`). Also records
    /// the activation's sample rate for endpoint scratch sizing.
    fn arm_staging(&mut self, staging: Arc<data::Staging>, config: ProcessConfig) {
        self.shared.lock().staging = Some(Arc::clone(&staging));
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

    /// Registers the waiter, queues the frame and waits — the part of
    /// `execute` the handshake `Scan`/`CreateInstance` shares. The
    /// wire-v2 envelope `instance` is derived from the verb scope:
    /// `Scan` is child-scoped (`WIRE_INSTANCE_NONE`); everything else
    /// is stamped with this instance's dedicated handle — the same
    /// value the reader's waiter attribution checks the echo against.
    fn request(&mut self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        let instance = match cmd {
            wire::Command::Scan => wire::WIRE_INSTANCE_NONE,
            _ => self.instance,
        };
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
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
            inner.waiters.insert(seq, Waiter { instance, tx });
        }
        let Some(driver_tx) = &self.driver_tx else {
            self.shared.lock().waiters.remove(&seq);
            return Err(HostError::Ipc);
        };
        if driver_tx
            .send(DriverCmd::Command { seq, instance, cmd })
            .is_err()
        {
            self.shared.lock().waiters.remove(&seq);
            return Err(HostError::Ipc);
        }
        match reply_rx.recv_timeout(self.command_timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.shared.lock().waiters.remove(&seq);
                // A reply timeout is `Ipc` plus `Failed` — the child
                // is presumed lost; `converge` kills it (which is
                // also what unblocks a driver stuck mid-write).
                converge(&self.shared, None);
                Err(HostError::Ipc)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(HostError::Ipc),
        }
    }

    /// `Destroy`'s control-plane meaning: wire v2's teardown is
    /// `DestroyInstance` (reclaims the child's registry slot) followed
    /// by `Shutdown` — neither is sent to a `Failed` instance (it is
    /// killed instead), then reap and join. Always succeeds and is
    /// idempotent — the defined rejection is `Destroy → Ok` for live,
    /// `Failed` and already-destroyed instances alike, so there is no
    /// `Err` to return. This primitive is unconditional: rejecting a
    /// `HostCommand::Destroy` from `Activated`/`Processing` (the L0
    /// legality rule) is `crate::composite::PluginHost`'s gate — it
    /// mirrors `lifecycle_state()` through `LifecycleEvent::Destroy`
    /// before calling this — not this layer's.
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
        let failed = {
            let inner = self.shared.lock();
            inner.mirror.is_failed() || matches!(inner.terminal, Some(Terminal::Failed))
        };
        if failed {
            // Kill through the shared handle — a channel message
            // could sit behind a wedged `write_all` forever.
            kill_child(&self.shared);
        } else if self.driver_tx.is_some() {
            // The registry slot is reclaimed before the wire
            // `Shutdown` — teardown is unconditional, so a dead or
            // wedged reply path (`Err(Ipc)`/timeout → convergence)
            // still proceeds to `Shutdown` below. `request` no-ops
            // to `Err(Ipc)` on a `test_fixture` (no driver channel)
            // — harmless.
            let _ = self.request(wire::Command::DestroyInstance);
        }
        if let Some(driver_tx) = self.driver_tx.take() {
            if !failed {
                // `shutdown_initiated` is set only on the driver —
                // immediately before it writes the wire `Shutdown` —
                // so a child that crashed before that point still
                // converges `Failed` and keeps its notification.
                let _ = driver_tx.send(DriverCmd::Shutdown);
            }
            drop(driver_tx);
        }
        self.join_threads();
        {
            let mut inner = self.shared.lock();
            self.shared
                .set_mirror(&mut inner, LifecycleState::Destroyed);
            inner.terminal.get_or_insert(Terminal::Clean);
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
    /// conditions.
    ///
    /// **RT contract:** non-RT only — takes the shared mutex (the RT
    /// leg reads the const-cheap [`Shared::rt_state`] snapshot
    /// instead).
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        self.shared.lock().mirror
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
        let mirror = data::mirror_decode(self.shared.rt_state.load(Ordering::Acquire));
        rt.process(mirror, inputs, outputs, events, transport)
    }

    /// The attach-side lifecycle gate, shared by the public prelude
    /// ([`PluginHost::prepare_endpoint`](crate::PluginHost::prepare_endpoint)'s
    /// L1 arm runs it for its own state step — the design's
    /// rejection order puts state before every other check) and
    /// [`L1Instance::prepare_endpoint`] itself.
    ///
    /// `shared.lock().mirror` — the control plane's committed state
    /// — decides: `Activated`/`Processing` pass; `Failed` is
    /// `Err(HostError::Failed)`; anything below `Activated` is
    /// `Err(HostError::Lifecycle)`, the same shape a rejected
    /// `Process` event takes. No convergence runs here — the mirror
    /// is already the committed authority on this side.
    ///
    /// **RT contract:** non-RT only — takes the shared mutex.
    ///
    /// # Errors
    ///
    /// [`HostError::Failed`] for a `Failed` instance and
    /// [`HostError::Lifecycle`] below `Activated`.
    pub(crate) fn check_attachable_state(&self) -> Result<(), HostError> {
        let inner = self.shared.lock();
        match inner.mirror {
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
            .staging
            .as_ref()
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
        // (5) seed arm — `Inner::staging` clone, same generation as
        // the leg's armed ring in the normal path (a normalized
        // no-op for the endpoint drain), legitimately newer after a
        // detached `Activate`.
        if let Some(staging) = self.shared.lock().staging.clone() {
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
            mirror: Arc::clone(&self.shared.rt_state),
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
            // onto the current `Inner::staging` or every later
            // submit fails `armed_for` and the instance passes
            // through forever (with `latency_frames` diverging
            // too). `RtLeg::arm`'s `last_output` resize fits the
            // attach-time `reserve_last_output` (the detached
            // `Activate` bound gate caps the ring at
            // `bound.max_frames`), so it never reallocs here.
            let staging = self.shared.lock().staging.clone();
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
        let terminal =
            data::mirror_decode(self.shared.rt_state.load(Ordering::Acquire)).is_destroyed();
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
        let inner = self.shared.lock();
        match inner.mirror {
            LifecycleState::Activated | LifecycleState::Processing => {}
            _ => return None,
        }
        Self::armed_latency(&inner, self.depth)
    }

    /// The armed ring's latency figure with no lifecycle gate —
    /// `Some` while a ring is armed, `None` before `Activate`. The
    /// composite's attach path reads it for endpoint bookkeeping:
    /// the endpoint is minted carrying the armed horizon its leg
    /// still holds — a mid-attach mirror flip (convergence) does
    /// not disarm the ring, so this internal read must not gate on
    /// the mirror the way the public query does.
    ///
    /// **RT contract:** non-RT only — takes the shared mutex.
    pub(crate) fn armed_latency_frames(&self) -> Option<u32> {
        Self::armed_latency(&self.shared.lock(), self.depth)
    }

    /// `(depth − 1) × armed frame_capacity` — `None` while no ring
    /// is armed. The shared read behind [`L1Instance::latency_frames`]
    /// (lifecycle-gated) and [`L1Instance::armed_latency_frames`]
    /// (ungated).
    fn armed_latency(inner: &Inner, depth: u32) -> Option<u32> {
        inner.staging.as_ref().map(|staging| {
            u32::try_from((depth - 1) as usize * staging.frame_capacity).unwrap_or(u32::MAX)
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
    ///
    /// **RT contract:** non-RT only — `try_recv`, never blocks.
    pub(crate) fn try_recv_notification(&self) -> Option<ChildExit> {
        self.notify_rx.try_recv().ok()
    }

    /// Handshake-failure teardown: kill + reap + join, bounded by
    /// [`join_threads`]. No notification is emitted — the instance
    /// never went live.
    fn abort(&mut self) {
        kill_child(&self.shared);
        drop(self.driver_tx.take());
        self.join_threads();
    }

    /// Waits for the driver thread to finish — bounded. The driver
    /// exits on `try_wait`/`Disconnected`/the shutdown watchdog (all
    /// prompt once the child is dead), but a wedged child can leave
    /// it blocked in `write_all` where it never observes the dropped
    /// channel — so the join polls `is_finished` for the shutdown
    /// grace plus a reap margin, then force-kills through the shared
    /// handle (the kill turns the write into EPIPE; the post-kill
    /// `join` then completes once the driver unwinds and reaps).
    ///
    /// The reader is *detached*, not joined: a POSIX child can leave
    /// a stdio-inheriting grandchild holding the stdout pipe open
    /// after its own death, so the reader's `wire::read_message` may
    /// never see EOF — joining could hang forever. Dropping the
    /// `JoinHandle` detaches; the reader touches only `Arc`-shared
    /// state (kept alive by its own Arc) and process exit reaps the
    /// thread.
    fn join_threads(&mut self) {
        if let Some(join) = self.driver_join.take() {
            // Grace for the shutdown watchdog's `command_timeout`
            // window plus a reap margin; past it the driver can only
            // be wedged — force-kill before the blocking join.
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
        drop(self.reader_join.take());
    }
}

impl Drop for L1Instance {
    /// Dropping without `Destroy` disconnects the command channel —
    /// the driver's `Disconnected` arm kills and reaps the child and
    /// converges (any notification lands on the dropped receiver). A
    /// `Shutdown` already in flight keeps its grace. Bounded by the
    /// driver's kill guarantee — the endpoint-channel drain is the
    /// same bounded `try_*` subset, never a wait on endpoint
    /// liveness.
    fn drop(&mut self) {
        self.drain_endpoint_channels();
        drop(self.driver_tx.take());
        self.join_threads();
    }
}

/// The lifecycle state a successful command leaves the instance in —
/// `None` for commands that do not transition (`Scan`, state verbs,
/// GUI). `Deactivate` and `Initialize` share the `Initialized`
/// destination; `Activate` and `StopProcessing` share `Activated`.
fn post_command_state(cmd: &wire::Command) -> Option<LifecycleState> {
    match cmd {
        wire::Command::Initialize | wire::Command::Deactivate => Some(LifecycleState::Initialized),
        wire::Command::Activate { .. } | wire::Command::StopProcessing => {
            Some(LifecycleState::Activated)
        }
        wire::Command::StartProcessing => Some(LifecycleState::Processing),
        _ => None,
    }
}

/// The lifecycle state an `Err` reply still leaves the instance in —
/// `Some` only for `StopProcessing`, the one commit-then-advisory
/// verb in the runner's dispatch: it applies
/// `Processing → Activated` *before* running the
/// `stop_processing()` hook (`ruviform_plugin_export::Runner`'s exact
/// ordering, matching L0's `InProcessHost::stop_processing`), so a
/// hook/plugin-class `Err` arrives after the transition and the
/// child stays `Activated`. `Err(Lifecycle)` is always the
/// pre-commit legality rejection — the caller filters it, not this
/// table.
///
/// The full-verb audit behind `Some`/`None`: `Initialize` and
/// `Activate` either reject pre-commit (`Err(Lifecycle)`) or take
/// the `InitFailed`/`ActivateFailed` loop back to the pre-command
/// state; `StartProcessing` commits, then rolls a hook failure back
/// to `Activated` — likewise its pre-command state; `Deactivate`'s
/// transition *is* its dispatch, so every `Err` it reports is
/// pre-commit; `SaveState`, `RestoreState` and the GUI verbs carry
/// no lifecycle transition at all. Every other `Err` therefore
/// leaves the child — and the mirror — in the pre-command state.
fn err_committed_state(cmd: &wire::Command) -> Option<LifecycleState> {
    match cmd {
        wire::Command::StopProcessing => Some(LifecycleState::Activated),
        _ => None,
    }
}

#[cfg(test)]
impl L1Instance {
    /// Child-free fixture for the endpoint-machinery tests — this
    /// module's and `crate::composite`'s. Real `Shared`, channel
    /// legs and shm segment, but no spawned child: `driver_tx`/
    /// `driver_join`/`reader_join` are `None`, so `destroy`/`Drop`
    /// run the bounded teardown without touching a process and
    /// `request` fails `Err(Ipc)` past the defined rejections (the
    /// detached `Activate` gate is still exercised — it runs
    /// before the wire).
    ///
    /// `mirror` is the state `Inner.mirror` and `rt_state` start
    /// at; `activated_frames` arms a gen-1 staging ring of that
    /// capacity and records a 48 kHz activation.
    pub(crate) fn test_fixture(mirror: LifecycleState, activated_frames: Option<u32>) -> Self {
        const DEPTH: u32 = 3;
        let (submit_tx, _submit_rx) = ruviform_rt::channel::<data::Submit>(DEPTH as usize);
        let (result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let (notify_tx, notify_rx) = mpsc::channel();
        let staging = activated_frames.map(|frames| Arc::new(data::Staging::new(DEPTH, frames, 1)));
        let mut leg = data::RtLeg::new(DEPTH, DEPTH - 1, submit_tx, result_rx);
        if let Some(staging) = &staging {
            leg.arm(Arc::clone(staging));
        }
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                waiters: HashMap::new(),
                mirror,
                block_in_flight: HashSet::new(),
                staging,
                terminal: None,
                shutdown_initiated: false,
                seen_eof: false,
                notify: Notify::Idle,
            }),
            rt_state: Arc::new(AtomicU8::new(data::mirror_encode(mirror))),
            child: OnceLock::new(),
            notify_tx,
            result_tx: Mutex::new(result_tx),
        });
        Self {
            shared,
            driver_tx: None,
            driver_join: None,
            reader_join: None,
            notify_rx,
            command_timeout: Duration::from_millis(50),
            next_seq: AtomicU64::new(0),
            instance: 0,
            depth: DEPTH,
            rt: Some(leg),
            staging_generation: u64::from(activated_frames.is_some()),
            arm_tx: None,
            pending_arm: None,
            return_rx: None,
            arm_collector: None,
            endpoint_bound: None,
            activated_sample_rate: activated_frames.map(|_| 48_000),
            segment: shm::Segment::create(
                u32::try_from(INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap_or(u32::MAX),
                DEPTH,
                DEPTH,
            )
            .expect("fixture segment"),
        }
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
        assert!(Arc::ptr_eq(&parts.mirror, &instance.shared.rt_state));
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
    /// generation — or a terminal/shutting-down child) still owes
    /// the leg exactly one completion: the synthesized
    /// `mark_failed` result `push_result` delivers. Without it the
    /// slot's `cleared[i] >= last_pushed[i]` gate wedges
    /// permanently — every later seq for that slot fails the gate.
    /// This pins the leg-side half: a pushed synthesized
    /// completion frees the slot through the normal `drain_to`.
    #[test]
    fn dropped_submit_result_frees_alternation_slot() {
        const DEPTH: u32 = 3;
        // Live channel ends so `try_push` lands and `last_pushed`
        // records each seq — the wedge is real only while the
        // submit actually pushed.
        let (submit_tx, mut submit_rx) = ruviform_rt::channel::<data::Submit>(DEPTH as usize);
        let (result_tx, result_rx) = ruviform_rt::channel::<u64>(DEPTH as usize);
        let (notify_tx, _notify_rx) = mpsc::channel();
        let shared = Shared {
            inner: Mutex::new(Inner {
                waiters: HashMap::new(),
                mirror: LifecycleState::Processing,
                block_in_flight: HashSet::new(),
                staging: Some(Arc::new(data::Staging::new(DEPTH, 512, 1))),
                terminal: None,
                shutdown_initiated: false,
                seen_eof: false,
                notify: Notify::Idle,
            }),
            rt_state: Arc::new(AtomicU8::new(data::mirror_encode(
                LifecycleState::Processing,
            ))),
            child: OnceLock::new(),
            notify_tx,
            result_tx: Mutex::new(result_tx),
        };
        let mut leg = data::RtLeg::new(DEPTH, DEPTH - 1, submit_tx, result_rx);
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
            shared.push_result(data::mark_failed(data::BLOCK_SEQ_FLAG | block));
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
}
