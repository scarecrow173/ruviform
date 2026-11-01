//! Async driving surface for non-RT offline operations: [`OfflineJob`].
//!
//! [`OfflineDriver`](crate::OfflineDriver) is deliberately synchronous:
//! the caller's thread drives the whole run. UI shells and CLIs,
//! however, need the dual shape — start a bounded operation, keep the
//! initiating thread free, observe status/progress/events from outside
//! and be able to stop it. `OfflineJob` provides exactly that glue and
//! nothing more: it owns one worker thread, hands the operation body a
//! [`JobScope`] (`CancellationToken` + `Progress` clones), and reports
//! lifecycle through a bounded event channel plus a polled
//! [`state`](OfflineJob::state).
//!
//! Deliberately absent — those responsibilities belong to the
//! application on top:
//!
//! - **Job registries / named IDs.** The job is a `Send` handle;
//!   whichever collection the app uses (a `HashMap<JobId, _>` for a
//!   `status`/`cancel <id>` CLI, a UI job panel) is app-owned.
//! - **Async executors.** `OfflineJob` is a plain std-thread
//!   spawn-join-observe handle, not a `Future`; an async runtime can
//!   wrap the same poll/wait/events surface or bridge [`JobEvent`]s.
//! - **Result persistence / manifests, retry, resume, exit codes,
//!   output policy.** All app concerns.
//!
//! # Lifecycle
//!
//! ```text
//! spawn ──► Running ──cancel()──► Cancelling ──body exits──► Finished
//!           └──────────────body exits───────────────────────► Finished
//! ```
//!
//! - [`cancel`](OfflineJob::cancel) flips the shared token and moves
//!   the state to `Cancelling`; the actual stop is cooperative — the
//!   body (typically an `OfflineDriver` run checking the token at
//!   chunk boundaries, or an I/O loop calling
//!   [`JobScope::is_cancelled`]) decides when to unwind.
//! - The state reaches `Finished` only when the worker thread has
//!   exited — all resources the body owned are released by then.
//! - [`wait`](OfflineJob::wait) joins the worker and yields the
//!   operation result; [`try_wait`](OfflineJob::try_wait) is the
//!   non-blocking variant, [`detach`](OfflineJob::detach) abandons the
//!   handle without joining.
//! - Dropping a job handle without `wait`/`detach` requests
//!   cancellation and detaches: a leaked handle never keeps running
//!   work alive silently, and never blocks the dropping thread.
//!
//! # Events vs. polling
//!
//! Two observation channels exist side by side and stay
//! authoritative-independent: `state()` + `progress()` give an
//! always-current snapshot (pollable), while [`events`](OfflineJob::events)
//! delivers `Started` → `Progress`* → `Finished` over a bounded channel.
//! Event delivery is lossy under back-pressure (`Progress` events are
//! coalesced by dropping the oldest updates — the absolute counters in
//! `progress()` remain exact — and even `Finished` can be dropped when
//! the channel is saturated); the reliable end-of-stream signal is the
//! channel closing when the worker exits.
//!
//! **RT contract:** non-RT only — the whole module is thread
//! spawn/join, atomics and channel operations.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::JoinHandle;

use crate::{CancellationToken, Progress};

/// Event-stream capacity: enough to absorb a short burst of chunk
/// completions while a consumer drains.
const EVENT_CAPACITY: usize = 64;

/// Coarse lifecycle state of an [`OfflineJob`].
///
/// `Finished` is terminal and means the worker thread has exited; it is
/// set before the operation result is published, so a `Finished` job is
/// immediately `wait`-able without blocking.
///
/// **RT contract:** non-RT only — `Copy` value.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum JobState {
    /// The operation body is executing.
    #[default]
    Running,
    /// Cancellation was requested; the body has not unwound yet.
    Cancelling,
    /// The worker thread exited; the result is retrievable.
    Finished,
}

/// How a finished [`OfflineJob`] ended — the classification the app
/// maps to its own diagnostics/exit semantics.
///
/// **RT contract:** non-RT only — `Copy` value.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum JobOutcome {
    /// The body returned `Ok`.
    #[default]
    Completed,
    /// The body returned `Err` while the cancellation token was
    /// cancelled — the cooperative-cancel path. Note the heuristic:
    /// an operation that fails *and* is cancelled concurrently
    /// classifies as `Cancelled`; the preserved [`JobError`] stays
    /// available for apps that need the distinction.
    Cancelled,
    /// The body returned `Err` or panicked without cancellation.
    Failed,
}

/// One lifecycle/progress notification from an [`OfflineJob`].
///
/// Delivered through [`OfflineJob::events`]. `Progress` events are
/// best-effort (coalesced under back-pressure); `Started` is sent
/// before the worker spawns so it is delivered unless the receiver was
/// already dropped; `Finished` shares the channel and can also be
/// coalesced out — consumers must treat channel-close as the terminal
/// marker.
///
/// **RT contract:** non-RT only — `Copy` value.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum JobEvent {
    /// The worker thread is about to start the operation body.
    Started,
    /// Progress counters changed; carries the *absolute* snapshot so a
    /// coalesced stream never loses the latest position.
    Progress {
        /// Units completed so far.
        completed: u64,
        /// Total units scheduled (`0` = indeterminate).
        total: u64,
    },
    /// The worker exited; carries the coarse outcome.
    Finished {
        /// How the job ended.
        outcome: JobOutcome,
    },
}

/// Failure wrapper distinguishing an operation error from a worker
/// panic — the same split [`OfflineDriver::run`] makes between
/// `Err(e)` and `OfflineError::WorkerPanicked`, kept at the job
/// boundary so an app can propagate `E` unchanged or report panics
/// separately.
///
/// **RT contract:** non-RT only.
///
/// [`OfflineDriver::run`]: crate::OfflineDriver::run
/// [`OfflineError::WorkerPanicked`]: crate::OfflineError::WorkerPanicked
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum JobError<E> {
    /// The operation body returned `Err`.
    Operation(E),
    /// The operation body panicked; the panic payload was consumed by
    /// `catch_unwind` (the panic message still reaches the default
    /// panic hook / stderr).
    Panicked,
}

impl<E: std::fmt::Display> std::fmt::Display for JobError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operation(e) => write!(f, "{e}"),
            Self::Panicked => f.write_str("operation panicked"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for JobError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            Self::Panicked => None,
        }
    }
}

/// The operation-side half of a job: clones of the shared
/// [`CancellationToken`] and [`Progress`] handed to the body.
///
/// The body uses them to (a) observe cancellation — either by passing
/// the token straight into [`OfflineDriver::run`]/[`run_parallel`], or
/// by checking [`is_cancelled`](Self::is_cancelled) in its own I/O and
/// setup loops — and (b) report progress phases the driver does not
/// own, via [`Progress::reset`]/[`Progress::bump`].
///
/// **Lifetime:** the scope is valid only inside the body closure.
///
/// [`run_parallel`]: crate::OfflineDriver::run_parallel
#[derive(Debug)]
pub struct JobScope {
    cancel: CancellationToken,
    progress: Progress,
}

impl JobScope {
    /// The shared cancellation token — pass to the driver or poll
    /// directly.
    #[must_use]
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// The shared progress counters — pass to the driver or drive
    /// directly for body-owned phases.
    #[must_use]
    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// `true` once cancellation was requested — cheap atomic read for
    /// hand-rolled loops.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// Handle to a non-RT offline operation running on its own worker
/// thread.
///
/// `T` is the operation's success value (`Send` + `'static` — it crosses
/// the thread boundary), `E` its error type. The handle is `Send`: the
/// app may move it into a registry, a GUI job panel or another thread.
///
/// **Threading/memory:** exactly one `std::thread` per job, plus the
/// bounded event channel and shared atomics; there is no executor or
/// thread pool. **Determinism:** identical to whatever the body does —
/// the job adds scheduling/lifecycle but never reorders or alters the
/// operation's output. **Reentrancy:** a job is not reusable; spawn a
/// new job per operation. Concurrent `spawn`s are independent.
/// **Testability:** all handles are in-process; tests drive the same
/// surface without signals or subprocesses.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct OfflineJob<T, E> {
    state: Arc<AtomicU8>,
    cancel: CancellationToken,
    progress: Progress,
    events: Receiver<JobEvent>,
    worker: Option<JoinHandle<Result<T, JobError<E>>>>,
    result: Option<Result<T, JobError<E>>>,
}

const STATE_RUNNING: u8 = 0;
const STATE_CANCELLING: u8 = 1;
const STATE_FINISHED: u8 = 2;

impl<T, E> OfflineJob<T, E> {
    /// Observed lifecycle state — non-blocking atomic read.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn state(&self) -> JobState {
        match self.state.load(Ordering::Acquire) {
            STATE_RUNNING => JobState::Running,
            STATE_CANCELLING => JobState::Cancelling,
            _ => JobState::Finished,
        }
    }

    /// `true` when [`state`](Self::state) reports `Finished`.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state() == JobState::Finished
    }

    /// Requests cooperative cancellation: sets the shared token and
    /// transitions `Running` → `Cancelling`. Idempotent; a `Finished`
    /// job ignores it. The operation stops when its body next observes
    /// the token.
    ///
    /// **RT contract:** non-RT only.
    pub fn cancel(&self) {
        Self::request_cancel(&self.cancel, &self.state);
    }

    /// Shared [`cancel`](Self::cancel) body — also invoked by the
    /// `spawn_with_cancel_after` progress trigger inside the worker.
    fn request_cancel(cancel: &CancellationToken, state: &AtomicU8) {
        cancel.cancel();
        // Only the Running state can become Cancelling; Finished and an
        // already-set Cancelling are left alone.
        let _ = state.compare_exchange(
            STATE_RUNNING,
            STATE_CANCELLING,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    /// The shared cancellation token — cloneable, e.g. to hand a signal
    /// handler (`ctrlc`) or another app thread.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// The shared progress counters — pollable snapshot of the
    /// operation's position. The same counters the operation body sees;
    /// an `OfflineDriver` run resets and bumps them per chunk.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// The event stream: `Started` → `Progress`* → `Finished`, then the
    /// channel closes. Bounded and lossy under back-pressure — see the
    /// module docs and [`JobEvent`] for the delivery contract.
    ///
    /// Typical consumer: `for ev in job.events().iter() { … }` until the
    /// channel closes, or `recv_timeout` in a polling UI loop.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn events(&self) -> &Receiver<JobEvent> {
        &self.events
    }
}

impl<T: Send + 'static, E: Send + 'static> OfflineJob<T, E> {
    /// Spawns `operation` on a dedicated worker thread and returns the
    /// handle immediately.
    ///
    /// `operation` receives the [`JobScope`]; whatever it returns is the
    /// job result. `catch_unwind` maps a panic to
    /// [`JobError::Panicked`] so a faulting body can never poison the
    /// handle.
    ///
    /// **RT contract:** non-RT only — allocates (channel, `Arc`s) and
    /// spawns a thread.
    ///
    /// # Panics
    ///
    /// Panics when the OS fails to spawn the worker thread (same
    /// contract as [`std::thread::spawn`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_exec::{JobError, JobOutcome, OfflineJob};
    ///
    /// let job = OfflineJob::<u64, std::io::Error>::spawn(|scope| {
    ///     scope.progress().reset(3);
    ///     for _ in 0..3 {
    ///         scope.progress().bump();
    ///     }
    ///     Ok(3)
    /// });
    /// assert_eq!(job.wait().unwrap(), 3);
    /// ```
    #[must_use]
    pub fn spawn<F>(operation: F) -> Self
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
    {
        Self::spawn_impl(operation, None)
    }

    /// [`spawn`](Self::spawn) plus a deterministic cancellation
    /// trigger: the job behaves as if [`cancel`](Self::cancel) were
    /// invoked at the first progress emit reporting
    /// `completed >= cancel_after`. `0` fires at the very first emit —
    /// including a phase `reset` — so the run stops as early as its
    /// first cooperative check.
    ///
    /// The trigger runs on the worker thread inside the progress emit
    /// itself: delivery does not depend on an [`events`](Self::events)
    /// consumer observing the update and racing [`cancel`](Self::cancel)
    /// back, so a fast body cannot outrun it. The body observes the
    /// token at its next cooperative check (decode loop, driver chunk
    /// boundary) and unwinds as `Cancelled`; when no check remains
    /// after the triggering unit (e.g. `cancel_after` lands on the
    /// final unit of the last phase) the run completes normally.
    ///
    /// **RT contract:** non-RT only — allocates and spawns a thread.
    ///
    /// # Panics
    ///
    /// Panics when the OS fails to spawn the worker thread (same
    /// contract as [`std::thread::spawn`]).
    #[must_use]
    pub fn spawn_with_cancel_after<F>(operation: F, cancel_after: u64) -> Self
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
    {
        Self::spawn_impl(operation, Some(cancel_after))
    }

    /// Shared spawn body; `cancel_after` arms the worker-side trigger
    /// described on [`spawn_with_cancel_after`](Self::spawn_with_cancel_after).
    fn spawn_impl<F>(operation: F, cancel_after: Option<u64>) -> Self
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
    {
        let (events_tx, events_rx) = sync_channel::<JobEvent>(EVENT_CAPACITY);
        let cancel = CancellationToken::new();
        let state = Arc::new(AtomicU8::new(STATE_RUNNING));

        // The handle's `Progress` (no listener — it must not keep an
        // event sender alive, or the channel would never close) shares
        // counters with the scope-side clone, which alone carries the
        // event-emitting listener. When the worker exits, the scope
        // drops, the listener's sender drops with it, `events_tx` drops
        // — and the receiver sees channel-close as end-of-stream.
        let progress = Progress::new();
        let mut scope_progress = progress.clone();
        let progress_tx = events_tx.clone();
        let trigger_cancel = cancel.clone();
        let trigger_state = Arc::clone(&state);
        scope_progress.set_listener(move |completed, total| {
            // Coalesce: an absolute snapshot supersedes any queued one;
            // dropping updates under back-pressure never loses position.
            let _ = progress_tx.try_send(JobEvent::Progress { completed, total });
            // The spawn-armed trigger applies the same
            // `completed >= n` comparison an events() consumer would
            // make before calling `cancel()` — but on the emitting
            // thread, so the next cooperative check always observes it.
            if cancel_after.is_some_and(|n| completed >= n) {
                Self::request_cancel(&trigger_cancel, &trigger_state);
            }
        });

        // Guaranteed-delivery start marker: the channel is empty and the
        // receiver is alive unless the app dropped it already.
        let _ = events_tx.send(JobEvent::Started);

        let scope = JobScope {
            cancel: cancel.clone(),
            progress: scope_progress,
        };
        let worker_state = Arc::clone(&state);
        let worker_cancel = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("ruviform-offline-job".to_string())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| operation(&scope)));
                let cancelled = worker_cancel.is_cancelled();
                let (outcome, result) = match result {
                    Ok(Ok(value)) => (JobOutcome::Completed, Ok(value)),
                    Ok(Err(e)) => (
                        if cancelled {
                            JobOutcome::Cancelled
                        } else {
                            JobOutcome::Failed
                        },
                        Err(JobError::Operation(e)),
                    ),
                    Err(_panic) => (
                        if cancelled {
                            JobOutcome::Cancelled
                        } else {
                            JobOutcome::Failed
                        },
                        Err(JobError::Panicked),
                    ),
                };
                worker_state.store(STATE_FINISHED, Ordering::Release);
                // Best-effort like Progress events; consumers rely on
                // channel-close as the terminal signal.
                let _ = events_tx.try_send(JobEvent::Finished { outcome });
                result
            })
            .expect("failed to spawn ruviform offline job thread");

        Self {
            state,
            cancel,
            progress,
            events: events_rx,
            worker: Some(worker),
            result: None,
        }
    }

    /// Non-blocking result retrieval: joins the worker when it has
    /// already exited and returns the outcome; `None` while the job is
    /// still running.
    ///
    /// After a `Some` the result is consumed — subsequent calls return
    /// `None`.
    ///
    /// **RT contract:** non-RT only.
    pub fn try_wait(&mut self) -> Option<Result<T, JobError<E>>> {
        if !self.is_finished() {
            return None;
        }
        self.join()
    }

    /// Blocks until the worker exits and returns the operation result.
    ///
    /// Does not request cancellation: `wait` on a still-running job
    /// simply waits.
    ///
    /// # Panics
    ///
    /// Panics in the (internally impossible) case the worker exited
    /// without publishing a result.
    ///
    /// **RT contract:** non-RT only.
    pub fn wait(mut self) -> Result<T, JobError<E>> {
        self.join().expect("offline job result missing after join")
    }

    /// Consumes the handle without joining; the operation runs to
    /// completion (or until its own cancellation path fires) and its
    /// result is dropped. The worker thread is detached — no
    /// cancellation is requested, unlike dropping the handle.
    ///
    /// **RT contract:** non-RT only.
    pub fn detach(mut self) {
        // Taking the JoinHandle detaches the worker; the subsequent
        // Drop sees `worker == None` and does not request cancellation.
        drop(self.worker.take());
    }

    /// Joins the worker once and stashes the result.
    fn join(&mut self) -> Option<Result<T, JobError<E>>> {
        if let Some(worker) = self.worker.take() {
            let result = worker.join().unwrap_or_else(|_| Err(JobError::Panicked));
            self.result = Some(result);
        }
        self.result.take()
    }
}

impl<T, E> Drop for OfflineJob<T, E> {
    fn drop(&mut self) {
        if self.worker.is_some() {
            // Leaking a handle must not leave work running silently:
            // request cancellation, then let the JoinHandle drop —
            // which detaches the worker. Never join on drop: the body
            // may be a blocking decode the caller abandoned.
            self.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::time::{Duration, Instant};

    fn poll_until(deadline: Duration, mut condition: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        condition()
    }

    /// A cooperative body that bumps progress, then spins until it is
    /// cancelled (or `give_up` fires) — returns `Ok` on cancel.
    fn spin_body(
        scope: &JobScope,
        give_up: &Arc<AtomicBool>,
    ) -> Result<&'static str, &'static str> {
        scope.progress().reset(1);
        scope.progress().bump();
        while !scope.is_cancelled() && !give_up.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(1));
        }
        if scope.is_cancelled() {
            Err("stopped")
        } else {
            Ok("done")
        }
    }

    #[test]
    fn spawn_wait_returns_the_body_result() {
        let job = OfflineJob::<u64, std::io::Error>::spawn(|scope| {
            scope.progress().reset(2);
            scope.progress().bump();
            scope.progress().bump();
            Ok(42)
        });
        assert_eq!(job.wait().unwrap(), 42);
    }

    #[test]
    fn events_deliver_started_progress_finished_then_close() {
        let job = OfflineJob::<(), std::io::Error>::spawn(|scope| {
            scope.progress().reset(2);
            scope.progress().bump();
            scope.progress().bump();
            Ok(())
        });
        let events: Vec<JobEvent> = job.events().iter().collect();
        assert_eq!(events.first(), Some(&JobEvent::Started));
        assert_eq!(
            events.last(),
            Some(&JobEvent::Finished {
                outcome: JobOutcome::Completed
            })
        );
        let progress_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                JobEvent::Progress { completed, total } => Some((*completed, *total)),
                _ => None,
            })
            .collect();
        // The reset(2) emit and both bumps all flow through; every
        // snapshot carries absolute counters.
        assert!(progress_events.contains(&(2, 2)), "{progress_events:?}");
        job.wait().unwrap();
    }

    #[test]
    fn state_and_progress_are_pollable_snapshots() {
        let released = Arc::new(AtomicBool::new(false));
        let give_up = Arc::clone(&released);
        let mut job = OfflineJob::<&'static str, &'static str>::spawn(move |scope| {
            spin_body(scope, &give_up)
        });
        assert!(matches!(
            job.state(),
            JobState::Running | JobState::Cancelling
        ));
        assert!(job.try_wait().is_none(), "body still spinning");
        released.store(true, Ordering::Relaxed);
        assert!(poll_until(Duration::from_secs(5), || job.is_finished()));
        assert_eq!(job.try_wait().unwrap().unwrap(), "done");
        assert!(job.try_wait().is_none(), "result consumed");
    }

    #[test]
    fn cancel_moves_to_cancelling_then_finished_cancelled() {
        let never = Arc::new(AtomicBool::new(false));
        let mut job =
            OfflineJob::<&'static str, &'static str>::spawn(move |scope| spin_body(scope, &never));
        job.cancel();
        assert!(poll_until(Duration::from_secs(5), || {
            matches!(job.state(), JobState::Cancelling | JobState::Finished)
        }));
        assert!(poll_until(Duration::from_secs(5), || job.is_finished()));
        match job.try_wait().unwrap() {
            Err(JobError::Operation("stopped")) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
        // The Finished event reports the classified outcome.
        let outcomes: Vec<JobOutcome> = job
            .events()
            .iter()
            .filter_map(|e| match e {
                JobEvent::Finished { outcome } => Some(outcome),
                _ => None,
            })
            .collect();
        assert_eq!(outcomes, vec![JobOutcome::Cancelled]);
    }

    /// A cooperative body with no suspension points: bumps, checks the
    /// token each unit, returns `Err` once cancelled.
    fn counted_body(scope: &JobScope, units: u64) -> Result<&'static str, &'static str> {
        scope.progress().reset(units);
        for _ in 0..units {
            if scope.is_cancelled() {
                return Err("stopped");
            }
            scope.progress().bump();
        }
        Ok("done")
    }

    #[test]
    fn cancel_after_fires_worker_side_without_a_consumer() {
        // The trigger fires inside the worker's emit, so a body that
        // never yields is still cancelled deterministically — an
        // events() consumer racing `cancel()` back could lose this.
        let job = OfflineJob::<&'static str, &'static str>::spawn_with_cancel_after(
            |scope| counted_body(scope, 5),
            2,
        );
        let outcomes: Vec<JobOutcome> = job
            .events()
            .iter()
            .filter_map(|e| match e {
                JobEvent::Finished { outcome } => Some(outcome),
                _ => None,
            })
            .collect();
        match job.wait() {
            Err(JobError::Operation("stopped")) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
        assert_eq!(outcomes, vec![JobOutcome::Cancelled]);
    }

    #[test]
    fn cancel_after_zero_cancels_at_the_first_progress_emit() {
        // `0` trips on the phase `reset` emit itself — the body's first
        // cooperative check already sees the token.
        let job = OfflineJob::<&'static str, &'static str>::spawn_with_cancel_after(
            |scope| counted_body(scope, 3),
            0,
        );
        match job.wait() {
            Err(JobError::Operation("stopped")) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
    }

    #[test]
    fn cancel_after_leaves_unreached_threshold_runs_completed() {
        // Threshold above the total never trips: the run completes.
        let job = OfflineJob::<&'static str, &'static str>::spawn_with_cancel_after(
            |scope| counted_body(scope, 3),
            4,
        );
        assert_eq!(job.wait().unwrap(), "done");
    }

    #[test]
    fn panic_in_body_surfaces_as_job_error_panicked() {
        let job = OfflineJob::<(), std::io::Error>::spawn(|_scope| {
            panic!("boom");
        });
        match job.wait() {
            Err(JobError::Panicked) => {}
            other => panic!("expected JobError::Panicked, got {other:?}"),
        }
    }

    #[test]
    fn detach_lets_the_body_run_to_completion() {
        let done = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let job = OfflineJob::<(), std::io::Error>::spawn(move |_scope| {
            std::thread::sleep(Duration::from_millis(10));
            flag.store(true, Ordering::Relaxed);
            Ok(())
        });
        job.detach();
        assert!(poll_until(Duration::from_secs(5), || done.load(Ordering::Relaxed)));
    }

    #[test]
    fn dropping_a_running_job_requests_cancellation() {
        let exited = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&exited);
        let job = OfflineJob::<(), std::io::Error>::spawn(move |scope| {
            while !scope.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, Ordering::Relaxed);
            Err(std::io::Error::other("cancelled"))
        });
        drop(job);
        assert!(
            poll_until(Duration::from_secs(5), || exited.load(Ordering::Relaxed)),
            "dropping the handle must request cancellation"
        );
    }

    #[test]
    fn scoped_progress_listener_fires_on_bump_and_reset() {
        let seen = Arc::new(AtomicU64::new(0));
        let flag = Arc::clone(&seen);
        let job = OfflineJob::<(), std::io::Error>::spawn(move |scope| {
            scope.progress().reset(3);
            scope.progress().bump();
            scope.progress().bump();
            scope.progress().bump();
            flag.store(1, Ordering::Relaxed);
            Ok(())
        });
        // The handle-side clone has no listener; the scope-side clone
        // emitted events into the job's channel. The channel closes on
        // worker exit, so `iter()` ends without `wait`.
        let events: Vec<JobEvent> = job.events().iter().collect();
        assert_eq!(seen.load(Ordering::Relaxed), 1);
        let last = events
            .iter()
            .rfind(|e| matches!(e, JobEvent::Progress { .. }));
        assert_eq!(
            last,
            Some(&JobEvent::Progress {
                completed: 3,
                total: 3
            })
        );
        job.wait().unwrap();
    }

    #[test]
    fn progress_listener_on_a_plain_progress_object() {
        let count = Arc::new(AtomicU64::new(0));
        let hits = Arc::clone(&count);
        let mut progress = Progress::new();
        progress.set_listener(move |completed, total| {
            hits.fetch_add(1, Ordering::Relaxed);
            assert!(completed <= total.max(1));
        });
        progress.reset(4);
        progress.bump();
        progress.bump();
        assert_eq!(count.load(Ordering::Relaxed), 3);
        progress.clear_listener();
        progress.bump();
        assert_eq!(count.load(Ordering::Relaxed), 3);
        assert_eq!(progress.completed(), 3);
    }

    #[test]
    fn job_handle_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<OfflineJob<u64, std::io::Error>>();
        assert_send::<JobEvent>();
        assert_send::<JobOutcome>();
        assert_send::<JobState>();
    }

    #[test]
    fn job_error_display_and_source() {
        let err: JobError<std::io::Error> =
            JobError::Operation(std::io::Error::other("decode failed"));
        assert_eq!(err.to_string(), "decode failed");
        assert!(std::error::Error::source(&err).is_some());
        assert_eq!(
            JobError::<std::io::Error>::Panicked.to_string(),
            "operation panicked"
        );
    }
}
