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
//! | `driver` | `Shared.child` (`Mutex<Child>` — operated only here, but shared so a converging thread can `kill` a wedged child even while this thread is blocked in `write_all`) · child stdin | serializes outgoing frames · drains the data-plane `submit` channel (dropping stale-generation seqs): each popped seq becomes a shm in-slot write + `BlockIn` doorbell · `try_wait` crash watch every [`DRIVER_POLL_INTERVAL`] · shutdown watchdog (grace → `child.kill()`) |
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
//! a long hold.
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
use std::io::{self, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command as ProcCommand, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_plugin_core::wire::{self, WIRE_FRAME_HEADER_LEN, WIRE_MAX_PAYLOAD};
use ruviform_plugin_core::{
    EventList, HostError, HostInput, HostOutput, LifecycleState, PluginDescriptor, PrepareError,
    ProcessResult, TransportInfo,
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
    /// Registration-supplied `'static` descriptor: supplies the argv
    /// `--ruviform-plugin-id` and the field-equality oracle for the
    /// child's `Scanned` reply.
    pub(crate) descriptor: PluginDescriptor,
    /// Runner executable.
    pub(crate) program: OsString,
    /// Extra arguments passed before the `--ruviform-*` bootstrap flags.
    pub(crate) args: Vec<OsString>,
    /// Lookahead depth in blocks (default 2) — fixes the shm slot
    /// depth `D = lookahead_blocks + 1`, both SPSC channel
    /// capacities and the RT leg's result horizon
    /// (`expected = block − lookahead`). `0` is rejected by
    /// [`L1Instance::spawn`] (`Err(Ipc)`): the just-submitted seq
    /// could never already have a result, so the leg would pass
    /// through forever.
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

/// One pending command: the reader sends the child's reply (or the
/// converger sends `Err(Ipc)`) onto this unbounded channel.
type Waiter = mpsc::Sender<Result<wire::Reply, HostError>>;

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
    /// `L1Instance::process`. Fails closed: the RT leg never needs
    /// the mutex.
    rt_state: AtomicU8,
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
    /// Serialize and write a `Command` frame.
    Command { seq: u64, cmd: wire::Command },
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
            let _ = waiter.send(Err(HostError::Ipc));
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

/// Reads one frame: `[u32 LE payload_len][u8 kind][payload]`. Any I/O
/// error — EOF included — or a malformed/oversized frame is `Err`;
/// the caller converges on it (EOF and protocol violation share the
/// `Failed` path).
fn read_message(reader: &mut impl Read) -> io::Result<wire::Message> {
    let mut header = [0_u8; WIRE_FRAME_HEADER_LEN];
    reader.read_exact(&mut header)?;
    let payload_len =
        u32::from_le_bytes(header[..4].try_into().expect("u32 length is 4 bytes")) as usize;
    if payload_len > WIRE_MAX_PAYLOAD as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "declared payload_len exceeds WIRE_MAX_PAYLOAD",
        ));
    }
    let mut frame = Vec::with_capacity(WIRE_FRAME_HEADER_LEN + payload_len);
    frame.extend_from_slice(&header);
    frame.resize(WIRE_FRAME_HEADER_LEN + payload_len, 0);
    reader.read_exact(&mut frame[WIRE_FRAME_HEADER_LEN..])?;
    wire::decode(&frame).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Writes one framed message.
fn write_message(writer: &mut impl Write, message: &wire::Message) -> io::Result<()> {
    let mut frame = Vec::new();
    wire::encode(message, &mut frame);
    writer.write_all(&frame)?;
    writer.flush()
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
/// locally); `in_depth`/`out_depth` are the spawn-time ring sizes.
fn reader_main(
    mut stdout: ChildStdout,
    shared: &Shared,
    hello_tx: &mpsc::Sender<Result<u16, ()>>,
    mut result_tx: ruviform_rt::Producer<u64>,
    view: &shm::View,
    in_depth: u32,
    out_depth: u32,
) {
    if let Ok(wire::Message::Hello { version, .. }) = read_message(&mut stdout) {
        let _ = hello_tx.send(Ok(version));
    } else {
        // Missing `Hello` or a dead pipe before it — handshake
        // failure surfaces through `hello_tx` and convergence.
        let _ = hello_tx.send(Err(()));
        converge(shared, None);
        return;
    }
    loop {
        match read_message(&mut stdout) {
            Ok(wire::Message::Reply { seq, .. }) if data::is_block_seq(seq) => {
                // A block rejection: the child answered the flagged
                // seq with an `Err` `Reply` instead of `BlockDone` —
                // the block still completes (marked failed) so the
                // strict alternation can proceed.
                let known = shared.lock().block_in_flight.remove(&seq);
                if !known {
                    converge(shared, None);
                    return;
                }
                // Channel capacity is `depth` and at most `depth`
                // doorbells are ever in flight — `try_push` cannot
                // overflow; a full push means the RT side already
                // drained past this seq, which is equally harmless.
                let _ = result_tx.try_push(data::mark_failed(seq));
            }
            Ok(wire::Message::Reply { seq, result }) => {
                let waiter = shared.lock().waiters.remove(&seq);
                if let Some(waiter) = waiter {
                    // A stale/unknown seq (timed-out waiter already
                    // removed) is dropped — the reply can no longer
                    // hurt anyone.
                    let _ = waiter.send(result);
                }
            }
            Ok(wire::Message::BlockDone { seq, result }) => {
                // A doorbell reply must carry a flagged seq this
                // driver actually rang — an unflagged or unknown one
                // is a protocol violation.
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
                let _ = result_tx.try_push(seq);
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
/// each I/O thread opens the owner `Segment`'s key locally) and
/// `in_depth` sizes the in-slot ring.
fn driver_main(
    shared: &Shared,
    mut stdin: ChildStdin,
    rx: &mpsc::Receiver<DriverCmd>,
    shutdown_grace: Duration,
    submit: &mut ruviform_rt::Consumer<data::Submit>,
    view: &mut shm::View,
    in_depth: u32,
) {
    let mut shutdown_deadline: Option<Instant> = None;
    loop {
        match rx.recv_timeout(DRIVER_POLL_INTERVAL) {
            Ok(DriverCmd::Command { seq, cmd }) => {
                // A converged instance (or teardown in flight) gets no
                // new wire messages — the spec forbids writes after
                // `Failed`; wake the waiter `Err(Ipc)` instead of
                // writing a doomed frame.
                let reject = shutdown_deadline.is_some() || shared.lock().terminal.is_some();
                if reject {
                    let waiter = shared.lock().waiters.remove(&seq);
                    if let Some(waiter) = waiter {
                        let _ = waiter.send(Err(HostError::Ipc));
                    }
                } else if write_message(&mut stdin, &wire::Message::Command { seq, cmd }).is_err() {
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
                let _ = write_message(&mut stdin, &wire::Message::Shutdown);
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
        while let Ok(submit) = submit.try_pop() {
            let seq = submit.seq;
            let staging = {
                let mut inner = shared.lock();
                if inner.terminal.is_some() || inner.shutdown_initiated {
                    // A dead or shutting-down child takes no new
                    // doorbells — the seq is consumed, its result
                    // simply never arrives (the RT leg passes
                    // through).
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
                    // a doorbell: `block_in_flight` never sees it and
                    // the RT leg resolves the absent result as
                    // pass-through.
                    staging
                }
            };
            let Some(staging) = staging else {
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
            if write_message(&mut stdin, &wire::Message::BlockIn { seq }).is_err() {
                converge(shared, None);
                return;
            }
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
            );
        })
        .map_err(|_| HostError::Ipc)?;
    Ok((join, spawn_rx))
}

/// Spawns the `l1-reader` thread: attaches its own shm `View` by key
/// *before* `Hello` so a broken data plane fails the spawn rather than
/// the first `BlockDone`, then runs [`reader_main`].
fn spawn_reader(
    shared: &Arc<Shared>,
    stdout: ChildStdout,
    result_tx: ruviform_rt::Producer<u64>,
    shm_key: String,
    depth: u32,
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
                result_tx,
                &view,
                depth,
                depth,
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
    /// The shm slot count `D` (`lookahead_blocks + 1`) — sizes the
    /// segment, both SPSC channels and the staging ring.
    depth: u32,
    /// The RT leg: caller-thread end of the data plane (submit
    /// producer, result consumer, staging, pass-through history).
    rt: data::RtLeg,
    /// Counts successful `Activate`s — stamped onto each new
    /// [`data::Staging`] ring so the driver can drop `submit`s
    /// queued under a superseded ring (see [`data::Submit`]).
    staging_generation: u64,
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
    /// silence). The spec is rejected like any other unusable
    /// spawn configuration rather than silently degraded.
    pub(crate) fn spawn(spec: &SpawnSpec) -> Result<Self, HostError> {
        if spec.lookahead_blocks == 0 {
            return Err(HostError::Ipc);
        }
        let depth = spec.lookahead_blocks.saturating_add(1);
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
        let mut command = ProcCommand::new(&spec.program);
        command
            .args(&spec.args)
            .arg("--ruviform-plugin-id")
            .arg(spec.descriptor.id)
            .arg("--ruviform-shm-key")
            .arg(segment.key())
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
            rt_state: AtomicU8::new(data::mirror_encode(LifecycleState::Created)),
            child: OnceLock::new(),
            notify_tx,
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
        )?;
        let Ok(Ok(stdout)) = spawn_rx.recv_timeout(spec.command_timeout) else {
            // Child spawn failed or timed out: disconnecting the
            // command channel makes the driver kill/reap whatever
            // it started (bounded); it may not have spawned at all.
            drop(driver_tx);
            let _ = driver_join.join();
            return Err(HostError::Ipc);
        };
        let reader_spawn =
            spawn_reader(&shared, stdout, result_tx, segment.key().to_owned(), depth);
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

        let mut instance = Self {
            shared,
            driver_tx: Some(driver_tx),
            driver_join: Some(driver_join),
            reader_join: Some(reader_join),
            notify_rx,
            command_timeout: spec.command_timeout,
            next_seq: AtomicU64::new(0),
            depth,
            rt: data::RtLeg::new(depth, spec.lookahead_blocks, submit_tx, result_rx),
            staging_generation: 0,
            segment,
        };
        match instance.handshake(spec, &hello_rx) {
            Ok(()) => Ok(instance),
            Err(error) => {
                // Handshake failure: kill + reap, no notification —
                // the instance never went live.
                instance.abort();
                Err(error)
            }
        }
    }

    /// `Hello` (version check) → `Scan` → descriptor field-match —
    /// the handshake sequence fixed by the design.
    fn handshake(&mut self, spec: &SpawnSpec, hello_rx: &HelloRx) -> Result<(), HostError> {
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
                if descriptors.iter().any(|d| d.matches(&spec.descriptor)) {
                    Ok(())
                } else {
                    Err(HostError::Ipc)
                }
            }
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
        let post = post_command_state(&cmd);
        // The mirror state an `Err` reply still leaves behind —
        // `Some` only for `StopProcessing` (see
        // `err_committed_state`).
        let on_err = err_committed_state(&cmd);
        let activate_frames = match &cmd {
            wire::Command::Activate { config } => Some(config.max_frames()),
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
        if let Some(frames) = activate_frames {
            // A successful `Activate` arms the data plane: build the
            // staging ring (non-RT allocation) and publish it to the
            // I/O threads plus the RT leg in one step. A concurrent
            // convergence leaves the armed ring unobserved — harmless.
            // The bumped `generation` lets the driver drop submits
            // queued under the superseded ring — see `data::Submit`.
            self.staging_generation += 1;
            let staging = Arc::new(data::Staging::new(
                self.depth,
                frames,
                self.staging_generation,
            ));
            self.shared.lock().staging = Some(Arc::clone(&staging));
            self.rt.arm(staging);
        }
        Ok(reply)
    }

    /// Registers the waiter, queues the frame and waits — the part of
    /// `execute` the handshake `Scan` shares.
    fn request(&mut self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let (waiter, reply_rx) = mpsc::channel();
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
            inner.waiters.insert(seq, waiter);
        }
        let Some(driver_tx) = &self.driver_tx else {
            self.shared.lock().waiters.remove(&seq);
            return Err(HostError::Ipc);
        };
        if driver_tx.send(DriverCmd::Command { seq, cmd }).is_err() {
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

    /// `Destroy`'s control-plane meaning: `Shutdown` on the wire (not
    /// sent to a `Failed` instance — it is killed instead), reap and
    /// join. Always succeeds and is idempotent — the defined
    /// rejection is `Destroy → Ok` for live, `Failed` and
    /// already-destroyed instances alike, so there is no `Err` to
    /// return. This primitive is unconditional: rejecting a
    /// `HostCommand::Destroy` from `Activated`/`Processing` (the L0
    /// legality rule) is `crate::composite::PluginHost`'s gate — it
    /// mirrors `lifecycle_state()` through `LifecycleEvent::Destroy`
    /// before calling this — not this layer's.
    ///
    /// **RT contract:** non-RT only — bounded by `command_timeout`
    /// of shutdown grace plus the kill guarantee.
    pub(crate) fn destroy(&mut self) {
        let failed = {
            let inner = self.shared.lock();
            inner.mirror.is_failed() || matches!(inner.terminal, Some(Terminal::Failed))
        };
        if let Some(driver_tx) = self.driver_tx.take() {
            if failed {
                // Kill through the shared handle — a channel message
                // could sit behind a wedged `write_all` forever.
                kill_child(&self.shared);
            } else {
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
    /// allocation (the `rt_l1_*` contract tests assert this).
    pub(crate) fn process(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        let mirror = data::mirror_decode(self.shared.rt_state.load(Ordering::Acquire));
        self.rt.process(mirror, inputs, outputs, events, transport)
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
    /// after its own death, so the reader's `read_exact` may never
    /// see EOF — joining could hang forever. Dropping the
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
    /// driver's kill guarantee.
    fn drop(&mut self) {
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
