//! L1 child diagnostics — per-attempt/per-child stderr capture and
//! the host's bounded failure-record retention (Issue #121).
//!
//! Two layers, deliberately separated:
//!
//! - [`ActiveDiagnostics`] is the *live* capture owned by one child
//!   (or one spawn/create attempt): the drain thread pushes stderr
//!   bytes, the spawn/converge paths advance `phase` and record the
//!   first-observed `cause`. It is never directly queryable — the
//!   queryable artifact is the frozen [`DiagnosticRecord`] produced
//!   by [`ActiveDiagnostics::freeze`].
//! - [`DiagRegistry`] is the host-side bounded retention: at most
//!   [`RETAINED_RECORDS_MAX`] completed records and
//!   [`RETAINED_STDERR_MAX`] total retained stderr bytes. Records
//!   are minted a contiguous, never-reused `diagnostics_id` under
//!   the registry lock at registration time — never before the
//!   attempt — so FIFO eviction is id-ordered and the
//!   `evicted_watermark`/`issued_max` pair resolves
//!   [`DiagnosticQuery`] into `Found`/`Evicted`/`NotFound`.
//!
//! stderr is arbitrary bytes — retained as `Vec<u8>`, never decoded
//! as text and never implicitly echoed to stdout/log/console;
//! retention and query are its only exits.
//!
//! **Harness note:** this file is reachable through the
//! `#[path = "../src/l1/mod.rs"]` graft in `tests/l1_proxy_e2e.rs`
//! where `crate::` resolves to the test crate root — references use
//! crate-name paths (`ruviform_plugin_core::`), `std::`, `self::` or
//! `super::`, never `crate::`.
//
// The drain/convergence wiring lands in T6–T8; until then the
// capture/registry types have no non-test consumer.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::Read;
use std::process::ChildStderr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_plugin_core::FailureCause;

use super::Shared;

/// Tail bytes retained per capture/snapshot — 64 KiB
/// (design: "stderr 保持の暫定契約").
pub(crate) const STDERR_RING_CAPACITY: usize = 64 * 1024;

/// Completed failure records retained per registry.
pub(crate) const RETAINED_RECORDS_MAX: usize = 8;

/// Total retained stderr payload across retained records — 512 KiB
/// (active captures, snapshot copies taken by callers and record
/// metadata are accounted separately, outside this bound).
pub(crate) const RETAINED_STDERR_MAX: usize = 512 * 1024;

/// Post-exit drain window (design `STDERR_DRAIN_GRACE` pin): starts
/// at the first child-exit observation (`Inner.terminal` set by any
/// convergence path), is never extended and ends early on pipe EOF.
/// Sized for a full pipe buffer plus scheduling jitter — far below
/// `DRIVER_JOIN_MARGIN`/`command_timeout`, so the drain can never
/// stretch the teardown bound. It is a target, not a hard
/// guarantee: an OS scheduling stall can delay the stop.
pub(crate) const STDERR_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Idle wait inside the drain loop — the poll/peek cadence for a
/// quiet-but-open pipe (also bounds how soon child-exit observation
/// is noticed).
const DRAIN_IDLE_WAIT: Duration = Duration::from_millis(10);

/// Read chunk for one drain iteration — bounded; a noisy child just
/// takes more iterations (this thread is off the RT path).
const DRAIN_READ_CHUNK: usize = 8 * 1024;

/// Where in the spawn/create lifecycle an attempt or child stood.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — it is re-exported with the diagnostics query surface.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpawnPhase {
    /// Attempt record created; spec/descriptor validation and
    /// argument assembly still in flight (no OS process yet).
    PreSpawn,
    /// `Command::spawn` in flight or just returned.
    Spawn,
    /// stderr pipe hand-off and drain-thread startup.
    DrainInit,
    /// `Hello`/version, `Scan` and `CreateInstance`/
    /// `CreateVideoInstance` negotiation — the mint window.
    Handshake,
    /// Handle minted — the attempt succeeded; the record continues
    /// for the child's life until convergence.
    Live,
}

/// Frozen view of one stderr capture, assembled under a single
/// lock hold so the counters and bytes always describe one instant.
///
/// **Invariant:** `total_read_bytes == bytes.len() + dropped_bytes`.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StderrSnapshot {
    /// Retained tail bytes — arbitrary bytes, never decoded as text.
    pub bytes: Vec<u8>,
    /// Bytes actually read from the pipe over the capture's life —
    /// output produced after the drain's deadline stop is not
    /// counted unless it was actually read.
    pub total_read_bytes: u64,
    /// Read bytes that could not be retained (ring capacity).
    pub dropped_bytes: u64,
    /// The drain observed pipe EOF.
    pub eof: bool,
    /// EOF was not reached before the drain deadline and the drain
    /// stopped voluntarily — independent of whether unread bytes
    /// were observed.
    pub drain_deadline_exceeded: bool,
    /// Unread pipe data was actually observed at the deadline or the
    /// final check.
    pub unread_bytes_observed: bool,
}

/// Tail-priority byte ring — the newest bytes win at capacity.
/// All fields advance only under the owning record's lock.
#[derive(Debug)]
struct StderrRing {
    bytes: Vec<u8>,
    total_read_bytes: u64,
    dropped_bytes: u64,
}

impl StderrRing {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            total_read_bytes: 0,
            dropped_bytes: 0,
        }
    }

    /// Appends `buf`, evicting the *oldest* retained bytes first —
    /// and counting every byte read-but-not-retained as dropped, so
    /// `total_read_bytes == bytes.len() + dropped_bytes` always holds.
    fn push(&mut self, buf: &[u8]) {
        self.total_read_bytes = self.total_read_bytes.saturating_add(buf.len() as u64);
        // At most `CAPACITY` bytes of `buf` itself can be retained.
        let keep = buf.len().min(STDERR_RING_CAPACITY);
        // Retained-after-append would be `len + keep`; evict the
        // excess from the front (`evict <= bytes.len()` because
        // `keep <= CAPACITY`).
        let evict = (self.bytes.len() + keep).saturating_sub(STDERR_RING_CAPACITY);
        self.dropped_bytes = self
            .dropped_bytes
            .saturating_add((buf.len() - keep + evict) as u64);
        self.bytes.drain(..evict);
        self.bytes.extend_from_slice(&buf[buf.len() - keep..]);
    }
}

#[derive(Debug)]
struct ActiveInner {
    ring: StderrRing,
    eof: bool,
    drain_deadline_exceeded: bool,
    unread_bytes_observed: bool,
    phase: SpawnPhase,
    /// First-observed failure cause — fixed under this lock; later
    /// observations never overwrite it.
    cause: Option<FailureCause>,
    /// First cleanup (kill/reap) error text, if any.
    cleanup_error: Option<String>,
}

impl ActiveInner {
    /// The single-lock snapshot — the only place a [`StderrSnapshot`]
    /// is assembled, so its invariant holds by construction.
    fn stderr_snapshot(&self) -> StderrSnapshot {
        StderrSnapshot {
            bytes: self.ring.bytes.clone(),
            total_read_bytes: self.ring.total_read_bytes,
            dropped_bytes: self.ring.dropped_bytes,
            eof: self.eof,
            drain_deadline_exceeded: self.drain_deadline_exceeded,
            unread_bytes_observed: self.unread_bytes_observed,
        }
    }
}

/// Live per-child capture shared (`Arc`) between the drain thread,
/// the spawn/converge paths and failure registration. Reaching the
/// drain thread the moment the child exists is what lets draining
/// start before the `Hello` wait.
#[derive(Debug)]
pub(crate) struct ActiveDiagnostics {
    inner: Mutex<ActiveInner>,
}

impl ActiveDiagnostics {
    /// A fresh capture at `phase` — typically
    /// [`SpawnPhase::PreSpawn`] for a spawn attempt.
    pub(crate) fn new(phase: SpawnPhase) -> Self {
        Self {
            inner: Mutex::new(ActiveInner {
                ring: StderrRing::new(),
                eof: false,
                drain_deadline_exceeded: false,
                unread_bytes_observed: false,
                phase,
                cause: None,
                cleanup_error: None,
            }),
        }
    }

    /// Drain thread: append bytes read from the child's stderr pipe.
    pub(crate) fn record_bytes(&self, buf: &[u8]) {
        super::lock_mutex(&self.inner).ring.push(buf);
    }

    /// Drain thread: pipe EOF observed.
    pub(crate) fn mark_eof(&self) {
        super::lock_mutex(&self.inner).eof = true;
    }

    /// Drain thread: the drain deadline was reached without EOF and
    /// the drain is stopping voluntarily.
    pub(crate) fn mark_deadline_exceeded(&self) {
        super::lock_mutex(&self.inner).drain_deadline_exceeded = true;
    }

    /// Drain thread: unread pipe data was actually observed at the
    /// deadline/final check.
    pub(crate) fn mark_unread_observed(&self) {
        super::lock_mutex(&self.inner).unread_bytes_observed = true;
    }

    /// Spawn/converge path: advance the lifecycle phase.
    pub(crate) fn set_phase(&self, phase: SpawnPhase) {
        super::lock_mutex(&self.inner).phase = phase;
    }

    /// Spawn/converge path: record the failure cause — first
    /// observed wins; a later observation never overwrites (the
    /// caller's convergence lock fixes the ordering).
    pub(crate) fn record_cause(&self, cause: FailureCause) {
        let mut inner = super::lock_mutex(&self.inner);
        if inner.cause.is_none() {
            inner.cause = Some(cause);
        }
    }

    /// Cleanup path: record a kill/reap error — first observed wins.
    pub(crate) fn record_cleanup_error(&self, error: impl fmt::Display) {
        let mut inner = super::lock_mutex(&self.inner);
        if inner.cleanup_error.is_none() {
            inner.cleanup_error = Some(error.to_string());
        }
    }

    /// The frozen stderr view at this instant.
    pub(crate) fn stderr_snapshot(&self) -> StderrSnapshot {
        super::lock_mutex(&self.inner).stderr_snapshot()
    }

    /// The current phase/cause pair — for tests and for the
    /// notification payload's cause propagation.
    #[cfg(test)]
    pub(crate) fn phase(&self) -> SpawnPhase {
        super::lock_mutex(&self.inner).phase
    }

    /// The recorded cause, if any.
    #[cfg(test)]
    pub(crate) fn cause(&self) -> Option<FailureCause> {
        super::lock_mutex(&self.inner).cause
    }

    /// Freezes the record for registry registration — the live
    /// record keeps mutating afterwards (a pooled live child's
    /// attempt record is a snapshot copy of this instant, separate
    /// from the child's continuing capture).
    pub(crate) fn freeze(&self, handle: Option<u32>) -> PendingRecord {
        let inner = super::lock_mutex(&self.inner);
        PendingRecord {
            handle,
            phase: inner.phase,
            cause: inner.cause,
            cleanup_error: inner.cleanup_error.clone(),
            stderr: inner.stderr_snapshot(),
        }
    }
}

/// A completed failure record before registry mint — produced by
/// [`ActiveDiagnostics::freeze`] or built directly for
/// attempt-scope failures that never had a live record.
#[derive(Debug)]
pub(crate) struct PendingRecord {
    /// The minted instance handle for minted failures — the
    /// handle-based correlation for notifications (`None` for
    /// pre-mint attempt records).
    pub(crate) handle: Option<u32>,
    /// Lifecycle phase at freeze.
    pub(crate) phase: SpawnPhase,
    /// First-observed failure cause (`None` for attempt records
    /// whose classification travels only in the `Err`).
    pub(crate) cause: Option<FailureCause>,
    /// First cleanup (kill/reap) error text, if any.
    pub(crate) cleanup_error: Option<String>,
    /// Frozen stderr view at freeze.
    pub(crate) stderr: StderrSnapshot,
}

/// A frozen failure record retained by [`DiagRegistry`] —
/// immutable once minted.
#[non_exhaustive]
#[derive(Debug)]
pub struct DiagnosticRecord {
    /// Registry-minted at registration — contiguous, never reused.
    pub diagnostics_id: u64,
    /// The minted instance handle for minted failures (`None` for
    /// pre-mint attempt records).
    pub handle: Option<u32>,
    /// Lifecycle phase at freeze.
    pub phase: SpawnPhase,
    /// First-observed failure cause.
    pub cause: Option<FailureCause>,
    /// First cleanup (kill/reap) error text, if any.
    pub cleanup_error: Option<String>,
    /// Frozen stderr view at freeze.
    pub stderr: StderrSnapshot,
}

/// Result of [`DiagRegistry::query`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
#[non_exhaustive]
#[derive(Debug)]
pub enum DiagnosticQuery {
    /// The record is retained.
    Found(Arc<DiagnosticRecord>),
    /// A record with that id was minted but evicted — `id` is at or
    /// below the eviction watermark (FIFO eviction is id-ordered).
    Evicted,
    /// The id was never minted (`id > issued_max`) — or, an
    /// internal-inconsistency case the three-state query reports
    /// honestly, the id sits inside the issued window but is absent
    /// from retention.
    NotFound,
}

#[derive(Debug, Default)]
struct RegistryInner {
    /// FIFO retention — front is oldest, so eviction order is
    /// registration order, which is id order.
    records: VecDeque<Arc<DiagnosticRecord>>,
    /// Minted-failure index: handle → `diagnostics_id` (`None`-handle
    /// records never appear here).
    by_handle: HashMap<u32, u64>,
    /// Largest id ever minted.
    issued_max: u64,
    /// Largest id ever evicted.
    evicted_watermark: u64,
    /// Lifetime eviction count.
    evicted_records: u64,
    /// Sum of `stderr.bytes.len()` across retained records.
    stderr_bytes: usize,
}

impl RegistryInner {
    /// Evicts oldest-first until both retention bounds hold — a
    /// record exceeding the stderr cap alone cannot happen (the
    /// ring caps at 64 KiB) but would evict itself and stop.
    fn enforce_bounds(&mut self) {
        while self.records.len() > RETAINED_RECORDS_MAX || self.stderr_bytes > RETAINED_STDERR_MAX {
            let Some(oldest) = self.records.pop_front() else {
                break;
            };
            self.stderr_bytes -= oldest.stderr.bytes.len();
            if let Some(handle) = oldest.handle {
                self.by_handle.remove(&handle);
            }
            self.evicted_watermark = oldest.diagnostics_id;
            self.evicted_records += 1;
        }
    }
}

/// Host-side bounded retention of completed failure records and the
/// contiguous `diagnostics_id` space — `PluginHost` holds one
/// `Arc<DiagRegistry>`; spawn/create paths take `&DiagRegistry` and
/// the function returning the final `Err` registers the record (the
/// record's `Arc` moves to the registry and outlives the dropped
/// `Shared`). No global/static registry — per-host retention and id
/// semantics.
#[derive(Debug, Default)]
pub struct DiagRegistry {
    inner: Mutex<RegistryInner>,
}

impl DiagRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mints the next `diagnostics_id`, retains the frozen record
    /// and evicts oldest-first until both bounds hold. Returns the
    /// retained record — the caller reads `.diagnostics_id` for the
    /// `SpawnFailed` payload.
    pub(crate) fn register(&self, pending: PendingRecord) -> Arc<DiagnosticRecord> {
        let mut inner = super::lock_mutex(&self.inner);
        inner.issued_max += 1;
        let record = Arc::new(DiagnosticRecord {
            diagnostics_id: inner.issued_max,
            handle: pending.handle,
            phase: pending.phase,
            cause: pending.cause,
            cleanup_error: pending.cleanup_error,
            stderr: pending.stderr,
        });
        inner.stderr_bytes += record.stderr.bytes.len();
        if let Some(handle) = record.handle {
            inner.by_handle.insert(handle, record.diagnostics_id);
        }
        inner.records.push_back(record.clone());
        inner.enforce_bounds();
        record
    }

    /// Resolves `id` against retention and the watermark pair.
    pub fn query(&self, id: u64) -> DiagnosticQuery {
        let inner = super::lock_mutex(&self.inner);
        if let Some(record) = inner
            .records
            .iter()
            .find(|record| record.diagnostics_id == id)
        {
            return DiagnosticQuery::Found(record.clone());
        }
        if id <= inner.evicted_watermark {
            return DiagnosticQuery::Evicted;
        }
        debug_assert!(
            id > inner.issued_max,
            "diagnostics id {id} inside the issued window is absent from retention"
        );
        DiagnosticQuery::NotFound
    }

    /// Handle-based correlation for minted failures — notifications
    /// carry no diagnostics id.
    pub fn record_for_handle(&self, handle: u32) -> Option<Arc<DiagnosticRecord>> {
        let inner = super::lock_mutex(&self.inner);
        let id = inner.by_handle.get(&handle)?;
        inner
            .records
            .iter()
            .find(|record| record.diagnostics_id == *id)
            .cloned()
    }

    /// Lifetime eviction count.
    pub fn evicted_records(&self) -> u64 {
        super::lock_mutex(&self.inner).evicted_records
    }

    /// Largest id ever minted (`0` before the first registration).
    pub fn issued_max(&self) -> u64 {
        super::lock_mutex(&self.inner).issued_max
    }
}

/// One drain-poll outcome — the platform FFI's answer to "is there
/// stderr to read right now".
#[derive(Debug, PartialEq, Eq)]
enum Poll {
    /// At least one byte can be read without blocking.
    Data,
    /// The writer side is gone (pipe EOF).
    Eof,
    /// Nothing readable within the idle window.
    Idle,
    /// The poll/peek itself failed — the drain ends quietly (the
    /// convergence path owns the failure's classification).
    Error,
}

/// POSIX result mapping — separated from the FFI call so the
/// classification is unit-testable without injecting signals.
/// `EINTR` maps to `Idle` (the drain retries on its next cycle),
/// matching the read side's `ErrorKind::Interrupted` retry —
/// a caught-and-resumed signal must not kill the capture.
#[cfg(unix)]
fn classify_poll(rc: i32, errno: i32, revents: i16) -> Poll {
    if rc < 0 {
        return if errno == libc::EINTR {
            Poll::Idle
        } else {
            Poll::Error
        };
    }
    if rc == 0 {
        return Poll::Idle;
    }
    if revents & libc::POLLIN != 0 {
        return Poll::Data;
    }
    if revents & libc::POLLHUP != 0 {
        return Poll::Eof;
    }
    // POLLERR/POLLNVAL (or an unrecognised set) — the descriptor is
    // unusable either way.
    Poll::Error
}

/// POSIX wait: `poll` on the pipe read end for `POLLIN`, with
/// `POLLHUP` reported as EOF. `std` exposes no readiness API, so
/// the libc call stays inside this file's local unsafe allowance.
#[cfg(unix)]
#[expect(
    unsafe_code,
    reason = "libc::poll is the only waitable pipe-readiness primitive on POSIX"
)]
fn poll_stderr(stderr: &ChildStderr, wait: Duration) -> Poll {
    use std::os::fd::AsRawFd;

    // SAFETY: `stderr` is a live pipe read end owned by this thread
    // for the whole call; `poll` only inspects the descriptor and
    // `pfd` — an exclusively-owned local borrowed as `&raw mut` —
    // outlives the call. Timeout clamps to the API's `c_int` range.
    let mut pfd = libc::pollfd {
        fd: stderr.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let timeout = i32::try_from(wait.as_millis().min(i32::MAX as u128)).unwrap_or(i32::MAX);
    let rc = unsafe { libc::poll(&raw mut pfd, 1, timeout) };
    // `last_os_error` must be read before any other fallible call.
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    classify_poll(rc, errno, pfd.revents)
}

/// Windows check: pipes have no waitable readiness primitive in
/// `std`, so `PeekNamedPipe` answers non-blockingly and the caller's
/// bounded sleep supplies the idle wait.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "PeekNamedPipe is the only non-blocking pipe-readiness primitive on Windows"
)]
fn poll_stderr(stderr: &ChildStderr, wait: Duration) -> Poll {
    use std::os::windows::io::AsRawHandle;
    use std::ptr;

    use windows_sys::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_NO_DATA, GetLastError, HANDLE};
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;

    let mut avail: u32 = 0;
    // SAFETY: `stderr` is a live pipe read end owned by this thread
    // for the whole call; null is legal for every output pointer
    // except `lptotal_bytes_avail`, which is a valid out-pointer.
    let ok = unsafe {
        PeekNamedPipe(
            stderr.as_raw_handle() as HANDLE,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &raw mut avail,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        // The writer closed or broke the pipe — treated as EOF.
        // SAFETY: `GetLastError` takes no pointers and only reads
        // this thread's error slot.
        let err = unsafe { GetLastError() };
        return if err == ERROR_BROKEN_PIPE || err == ERROR_NO_DATA {
            Poll::Eof
        } else {
            Poll::Error
        };
    }
    if avail > 0 {
        return Poll::Data;
    }
    thread::sleep(wait);
    Poll::Idle
}

/// The drain thread's main loop (design's adopted 案A — periodic
/// wake + self-bounded deadline). Pushes every read into the
/// capture's ring; ends on pipe EOF, or [`STDERR_DRAIN_GRACE`] after
/// the child's exit is first observed (`Inner.terminal` set — the
/// deadline starts at observation, is never extended, and an EOF at
/// any point ends the drain immediately). At the deadline a final
/// readiness check distinguishes "leftover bytes seen but not
/// taken" (`unread_bytes_observed`) from a clean stop
/// (`drain_deadline_exceeded` always sets — the stop itself is the
/// observed fact; test-spec semantics pin both flags independent).
/// Dropping `stderr` on return closes the read end, so a
/// stdio-inheriting grandchild's later writes fail.
fn drain_stderr(mut stderr: ChildStderr, diag: &ActiveDiagnostics, shared: &Shared) {
    let mut buf = [0u8; DRAIN_READ_CHUNK];
    // `Some(at)` from the first observed child exit — the deadline
    // `at + STDERR_DRAIN_GRACE` never moves afterwards.
    let mut exit_observed_at: Option<Instant> = None;
    loop {
        match poll_stderr(&stderr, DRAIN_IDLE_WAIT) {
            Poll::Data => match stderr.read(&mut buf) {
                Ok(0) => {
                    diag.mark_eof();
                    return;
                }
                Ok(n) => diag.record_bytes(&buf[..n]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return,
            },
            Poll::Eof => {
                diag.mark_eof();
                return;
            }
            Poll::Error => return,
            Poll::Idle => {}
        }
        if exit_observed_at.is_none() && shared.lock().terminal.is_some() {
            exit_observed_at = Some(Instant::now());
        }
        if let Some(at) = exit_observed_at
            && at.elapsed() >= STDERR_DRAIN_GRACE
        {
            if matches!(poll_stderr(&stderr, Duration::ZERO), Poll::Data) {
                diag.mark_unread_observed();
            }
            diag.mark_deadline_exceeded();
            return;
        }
    }
}

/// Spawns the `l1-stderr-drain` thread — detached by convention: the
/// caller drops the `JoinHandle` and never joins (the drain bounds
/// its own life; an stdio-inheriting grandchild could otherwise
/// stretch a join past the teardown bound). `Err` mirrors
/// [`thread::Builder::spawn`] — the caller rolls the spawn back
/// through the existing kill/reap abort path.
///
/// `pub(super)` — only `l1`'s spawn path names `Shared`.
pub(super) fn spawn_drain(
    stderr: ChildStderr,
    diag: Arc<ActiveDiagnostics>,
    shared: Arc<Shared>,
) -> std::io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("l1-stderr-drain".to_owned())
        .spawn(move || drain_stderr(stderr, &diag, &shared))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(stderr: StderrSnapshot) -> PendingRecord {
        PendingRecord {
            handle: None,
            phase: SpawnPhase::Handshake,
            cause: Some(FailureCause::Handshake),
            cleanup_error: None,
            stderr,
        }
    }

    fn snapshot_with_bytes(len: usize) -> StderrSnapshot {
        StderrSnapshot {
            bytes: vec![0xAB; len],
            total_read_bytes: len as u64,
            dropped_bytes: 0,
            eof: true,
            drain_deadline_exceeded: false,
            unread_bytes_observed: false,
        }
    }

    /// Issue #147: a `poll` returning EINTR must classify as `Idle`
    /// (retry on the next cycle — the read side's
    /// `ErrorKind::Interrupted` semantics), not `Error` (drain death
    /// → `ChildStderr` drop → child stderr writes hit EPIPE).
    #[cfg(unix)]
    #[test]
    fn poll_eintr_is_idle_not_error() {
        assert_eq!(classify_poll(-1, libc::EINTR, 0), Poll::Idle);
        assert_eq!(classify_poll(-1, libc::EIO, 0), Poll::Error);
        assert_eq!(classify_poll(0, 0, 0), Poll::Idle);
        assert_eq!(classify_poll(1, 0, libc::POLLIN), Poll::Data);
        assert_eq!(classify_poll(1, 0, libc::POLLHUP), Poll::Eof);
        assert_eq!(classify_poll(1, 0, libc::POLLERR), Poll::Error);
    }

    #[test]
    fn ring_retains_newest_tail_at_capacity() {
        let mut ring = StderrRing::new();
        // Push capacity + 10 bytes: the oldest 10 retained bytes are
        // evicted and the new tail survives.
        let first = vec![0x11; STDERR_RING_CAPACITY];
        ring.push(&first);
        ring.push(&[0x22; 10]);
        assert_eq!(ring.bytes.len(), STDERR_RING_CAPACITY);
        assert_eq!(&ring.bytes[..STDERR_RING_CAPACITY - 10], &first[10..]);
        assert_eq!(&ring.bytes[STDERR_RING_CAPACITY - 10..], &[0x22; 10]);
        assert_eq!(ring.total_read_bytes, (STDERR_RING_CAPACITY + 10) as u64);
        assert_eq!(ring.dropped_bytes, 10);
        assert_eq!(
            ring.total_read_bytes,
            ring.bytes.len() as u64 + ring.dropped_bytes
        );
    }

    #[test]
    fn ring_invariant_holds_across_varied_pushes() {
        let mut ring = StderrRing::new();
        // Mixed sizes — small, boundary, and over-capacity pushes.
        for size in [1usize, 4096, 1, 64 * 1024, 3, 70_000, 17, 64 * 1024, 2] {
            ring.push(&vec![0x5A; size]);
            assert_eq!(
                ring.total_read_bytes,
                ring.bytes.len() as u64 + ring.dropped_bytes,
                "invariant broken after push of {size} bytes"
            );
            assert!(ring.bytes.len() <= STDERR_RING_CAPACITY);
        }
    }

    #[test]
    fn ring_push_larger_than_capacity_keeps_only_tail() {
        let mut ring = StderrRing::new();
        ring.push(&[0x00; 5]);
        let big = vec![0x77u8; STDERR_RING_CAPACITY + 100];
        ring.push(&big);
        assert_eq!(ring.bytes.len(), STDERR_RING_CAPACITY);
        assert_eq!(ring.bytes, big[100..]);
        assert_eq!(
            ring.dropped_bytes,
            // The 5 prior bytes plus the 100 eldest bytes of `big`.
            (5 + 100) as u64
        );
    }

    #[test]
    fn snapshot_is_frozen_at_one_instant() {
        let active = ActiveDiagnostics::new(SpawnPhase::Spawn);
        active.record_bytes(&[0xAA; 16]);
        active.mark_eof();
        let first = active.stderr_snapshot();
        assert_eq!(first.bytes.len(), 16);
        assert!(first.eof);
        assert!(!first.drain_deadline_exceeded);
        // Later mutations never reach an already-taken snapshot.
        active.record_bytes(&[0xBB; 16]);
        active.mark_deadline_exceeded();
        let second = active.stderr_snapshot();
        assert_eq!(first.bytes.len(), 16);
        assert!(!first.drain_deadline_exceeded);
        assert_eq!(second.bytes.len(), 32);
        assert!(second.drain_deadline_exceeded);
        assert_eq!(second.total_read_bytes, 32);
    }

    #[test]
    fn first_observed_cause_wins() {
        let active = ActiveDiagnostics::new(SpawnPhase::Handshake);
        active.record_cause(FailureCause::Transport);
        active.record_cause(FailureCause::Timeout);
        assert_eq!(active.cause(), Some(FailureCause::Transport));
        let frozen = active.freeze(None);
        assert_eq!(frozen.cause, Some(FailureCause::Transport));
    }

    #[test]
    fn phase_advances_and_freeze_captures() {
        let active = ActiveDiagnostics::new(SpawnPhase::PreSpawn);
        assert_eq!(active.phase(), SpawnPhase::PreSpawn);
        active.set_phase(SpawnPhase::Handshake);
        active.set_phase(SpawnPhase::Live);
        assert_eq!(active.freeze(Some(7)).phase, SpawnPhase::Live);
        assert_eq!(active.freeze(Some(7)).handle, Some(7));
    }

    #[test]
    fn registry_mints_contiguous_ids_and_evicts_fifo() {
        let registry = DiagRegistry::new();
        let mut ids = Vec::new();
        for _ in 0..10 {
            ids.push(
                registry
                    .register(pending(snapshot_with_bytes(0)))
                    .diagnostics_id,
            );
        }
        assert_eq!(ids, (1..=10).collect::<Vec<_>>());
        // Retention is the newest 8; the two oldest are evicted.
        assert_eq!(registry.evicted_records(), 2);
        assert_eq!(registry.issued_max(), 10);
        // `id <= watermark` → Evicted; in-window & present → Found;
        // above issued_max → NotFound.
        assert!(matches!(registry.query(1), DiagnosticQuery::Evicted));
        assert!(matches!(registry.query(2), DiagnosticQuery::Evicted));
        match registry.query(5) {
            DiagnosticQuery::Found(record) => assert_eq!(record.diagnostics_id, 5),
            other => panic!("expected Found, got {other:?}"),
        }
        assert!(matches!(registry.query(11), DiagnosticQuery::NotFound));
    }

    #[test]
    fn stderr_payload_cap_evicts_beyond_count_bound() {
        let registry = DiagRegistry::new();
        // 9 records × 60 KiB = 540 KiB — the payload cap (512 KiB)
        // binds before the record-count bound for the last insert.
        for _ in 0..9 {
            registry.register(pending(snapshot_with_bytes(60 * 1024)));
        }
        assert_eq!(registry.evicted_records(), 1);
        assert!(matches!(registry.query(1), DiagnosticQuery::Evicted));
        assert!(matches!(registry.query(2), DiagnosticQuery::Found(_)));
        assert!(matches!(registry.query(9), DiagnosticQuery::Found(_)));
    }

    #[test]
    fn handle_index_resolves_minted_failures() {
        let registry = DiagRegistry::new();
        let mut record = pending(snapshot_with_bytes(4));
        record.handle = Some(7);
        let registered = registry.register(record);
        assert_eq!(
            registry.record_for_handle(7).map(|r| r.diagnostics_id),
            Some(registered.diagnostics_id)
        );
        assert!(registry.record_for_handle(8).is_none());
        // Eviction removes the handle index too — register 9 more
        // (10 total → id 1 evicted).
        for _ in 0..9 {
            registry.register(pending(snapshot_with_bytes(0)));
        }
        assert!(registry.record_for_handle(7).is_none());
        assert!(matches!(
            registry.query(registered.diagnostics_id),
            DiagnosticQuery::Evicted
        ));
    }

    /// `dropped_bytes` counts *only* bytes read-but-unretained: a
    /// deadline stop with leftover pipe data moves the flags, not
    /// the counters — output the drain never read is not a "drop",
    /// and the snapshot stays inside its invariant.
    #[test]
    fn dropped_bytes_counts_only_unretained() {
        let active = ActiveDiagnostics::new(SpawnPhase::Live);
        active.record_bytes(&[0x55; 32]);
        // Unread pipe data observed at the deadline + voluntary
        // stop — flags, not bytes.
        active.mark_unread_observed();
        active.mark_deadline_exceeded();
        let snapshot = active.stderr_snapshot();
        assert!(snapshot.unread_bytes_observed);
        assert!(snapshot.drain_deadline_exceeded);
        assert_eq!(snapshot.total_read_bytes, 32);
        assert_eq!(snapshot.dropped_bytes, 0);
        assert_eq!(
            snapshot.total_read_bytes,
            snapshot.bytes.len() as u64 + snapshot.dropped_bytes
        );
    }

    /// Retention eviction is the registry's own accounting — it
    /// never folds into a record's `dropped_bytes`: a retained
    /// record still carries exactly the capture it was frozen with.
    #[test]
    fn registry_eviction_does_not_count_as_dropped_bytes() {
        let registry = DiagRegistry::new();
        // 10 records × 4 bytes → 2 retention evictions.
        for _ in 0..10 {
            registry.register(pending(snapshot_with_bytes(4)));
        }
        assert_eq!(registry.evicted_records(), 2);
        let DiagnosticQuery::Found(record) = registry.query(10) else {
            panic!("the newest record is retained")
        };
        assert_eq!(record.stderr.dropped_bytes, 0);
        assert_eq!(record.stderr.total_read_bytes, 4);
    }

    /// The watermark pair resolves every id boundary-exactly:
    /// `id <= evicted_watermark` is `Evicted`, the retained window
    /// is `Found`, and `id > issued_max` is `NotFound` — the
    /// three-value contract, no gap in between.
    #[test]
    fn watermark_boundaries_resolve_three_way() {
        let registry = DiagRegistry::new();
        for _ in 0..10 {
            registry.register(pending(snapshot_with_bytes(0)));
        }
        // ids 1..=2 evicted (watermark 2), 3..=10 retained.
        assert!(matches!(registry.query(2), DiagnosticQuery::Evicted));
        assert!(matches!(registry.query(3), DiagnosticQuery::Found(_)));
        assert!(matches!(registry.query(10), DiagnosticQuery::Found(_)));
        assert!(matches!(registry.query(11), DiagnosticQuery::NotFound));
        // A far-future id is just as never-minted.
        assert!(matches!(
            registry.query(u64::MAX),
            DiagnosticQuery::NotFound
        ));
    }

    /// Concurrent registration keeps the id space gap-free — the
    /// mint serializes under the lock, so racing registrations
    /// still mint `1..=issued_max` contiguously and every issued
    /// id resolves `Found` or `Evicted` (never `NotFound` inside
    /// the issued window).
    #[test]
    fn concurrent_registration_mints_gap_free_ids() {
        const THREADS: usize = 4;
        const EACH: usize = 8;
        let registry = Arc::new(DiagRegistry::new());
        let mut joins = Vec::new();
        for _ in 0..THREADS {
            let registry = Arc::clone(&registry);
            joins.push(thread::spawn(move || {
                (0..EACH)
                    .map(|_| {
                        registry
                            .register(pending(snapshot_with_bytes(0)))
                            .diagnostics_id
                    })
                    .collect::<Vec<u64>>()
            }));
        }
        let mut ids: Vec<u64> = joins
            .into_iter()
            .flat_map(|join| join.join().expect("registration thread"))
            .collect();
        ids.sort_unstable();
        let total = (THREADS * EACH) as u64;
        assert_eq!(ids, (1..=total).collect::<Vec<_>>());
        assert_eq!(registry.issued_max(), total);
        // Retention holds the newest 8 by mint order — above the
        // watermark Found, at/below it Evicted, never NotFound.
        for id in 1..=total {
            let query = registry.query(id);
            if id <= total - RETAINED_RECORDS_MAX as u64 {
                assert!(matches!(query, DiagnosticQuery::Evicted), "id {id}");
            } else {
                assert!(matches!(query, DiagnosticQuery::Found(_)), "id {id}");
            }
        }
    }
}
