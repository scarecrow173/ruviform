//! Bounded non-RT job executor: [`JobExecutor`] + [`QueuedJob`] (Issue
//! #744, API contract fixed by
//! `docs/designs/issue-727-bounded-nonrt-job-executor.md`).
//!
//! [`OfflineJob`](crate::OfflineJob) dedicates one OS thread per job.
//! `JobExecutor` instead runs job bodies on a fixed pool of worker
//! threads fed by a bounded queue, so `workers + queue_capacity` bounds
//! the number of in-flight jobs. Job bodies share the
//! [`OfflineJob`](crate::OfflineJob) signature —
//! `FnOnce(&JobScope) -> Result<T, E>` — and the
//! [`QueuedJob`](QueuedJob) handle mirrors `OfflineJob`'s
//! observe/cancel/join surface (`state`/`is_finished`/`cancel`/
//! `cancel_token`/`progress`/`events`/`try_wait`/`wait`/`detach`).
//!
//! Deliberately absent — those responsibilities belong to the
//! application on top (same boundary as `OfflineJob`):
//!
//! - **Job registries / job IDs / persistence / retry / resume.** The
//!   executor keeps only an internal live-job cancel-token bookkeeping
//!   for `shutdown`; it is not a product Job Registry.
//! - **Priorities, multiple pools, dynamic worker counts, work
//!   stealing.** `ExecutorConfig` fixes both bounds at construction
//!   (YAGNI — revisit when a consumer asks).
//! - **Forced preemption.** Cancellation is cooperative only: a body
//!   that never observes its [`CancellationToken`](crate::CancellationToken)
//!   keeps `shutdown` waiting, exactly like `std::thread` join
//!   semantics. Safe forced interruption of arbitrary Rust code is not
//!   provided.
//!
//! # Lifecycle
//!
//! ```text
//! submit ──► Queued ──worker picks up──► Running ──cancel()──► Cancelling ──► Finished
//!                │                            └──────────body exits──────────► Finished
//!                └──cancel()──► Cancelling ──worker skips body──► Finished
//! ```
//!
//! - A job cancelled while `Queued` never runs its body and resolves to
//!   [`JobError::Cancelled`]; a running job unwinds at its next
//!   cooperative check and classifies like `OfflineJob` (`Completed` /
//!   `Cancelled` / `Failed` via [`JobOutcome`]).
//! - [`JobExecutor::shutdown`] (also the `Drop` behavior) rejects new
//!   submissions, cancels every live job, drains the queue and joins
//!   all workers.
//!
//! # Deadlock contract
//!
//! [`JobExecutor::submit`] blocks while the queue is full. Calling it
//! from a job body running on this executor's own workers can deadlock
//! the whole pool once every worker waits on a full queue — job bodies
//! that submit more work must use [`JobExecutor::try_submit`] instead.
//!
//! **RT contract:** every API in this module is `non-RT only` — thread
//! spawn/join, mutex/condvar and channel operations. Never call from an
//! RT audio path.

use std::collections::{HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;

use crate::job::{
    EVENT_CAPACITY, JobError, JobEvent, JobOutcome, JobScope, JobState, STATE_FINISHED,
    STATE_QUEUED, request_cancel, state_from_u8,
};
use crate::{CancellationToken, Progress};

/// Configuration for [`JobExecutor`]: fixed worker count and bounded
/// queue capacity. Both must be `>= 1`; the in-flight bound is
/// `workers + queue_capacity`.
///
/// **RT contract:** non-RT only — `Copy` value.
#[derive(Debug, Clone, Copy)]
pub struct ExecutorConfig {
    /// Fixed worker thread count. Must be >= 1.
    pub workers: usize,
    /// Maximum number of submitted-but-not-yet-started jobs waiting in
    /// the queue. Must be >= 1.
    pub queue_capacity: usize,
}

/// Executor construction failure.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutorError {
    /// `workers == 0` or `queue_capacity == 0`.
    InvalidConfig,
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig => {
                f.write_str("executor config requires workers >= 1 and queue_capacity >= 1")
            }
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Submission rejection from [`JobExecutor::submit`] /
/// [`JobExecutor::try_submit`].
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubmitError {
    /// `try_submit` found `queue_capacity` jobs already waiting.
    QueueFull,
    /// The executor is shut down (or being dropped).
    Shutdown,
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => f.write_str("executor queue is full"),
            Self::Shutdown => f.write_str("executor is shut down"),
        }
    }
}

impl std::error::Error for SubmitError {}

/// Type-erased queued work item: runs the full job lifecycle on a pool
/// worker (cancel check → body under `catch_unwind` → result/event
/// delivery → live-bookkeeping teardown).
type JobTask = Box<dyn FnOnce() + Send>;

struct QueueState {
    /// Waiting jobs; `jobs.len() <= capacity` is the bounded invariant.
    jobs: VecDeque<JobTask>,
    /// Set by `shutdown`: rejects new submissions and ends worker loops
    /// once the queue drains.
    shutdown: bool,
}

struct Inner {
    /// Bounded queue + shutdown flag, serialized by one mutex so a
    /// submission and the shutdown transition cannot interleave.
    queue: Mutex<QueueState>,
    /// Workers sleep here while the queue is empty.
    has_job: Condvar,
    /// Blocking `submit` sleeps here while the queue is full; also
    /// woken by `shutdown` so a parked submitter fails fast.
    has_space: Condvar,
    /// `queue_capacity`, fixed at construction.
    capacity: usize,
    /// Live-job bookkeeping: job id → shared cancel token, used only to
    /// propagate `shutdown` cancellation. Entries are removed when the
    /// job finishes — internal bookkeeping, NOT a product Job Registry.
    live: Mutex<HashMap<u64, CancellationToken>>,
    next_job_id: AtomicU64,
    /// Worker threads; drained and joined by `shutdown`.
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl Inner {
    /// Marks the queue closed, wakes every waiter, cancels all live
    /// jobs, then joins all workers. Idempotent; concurrent callers
    /// serialize on the workers mutex so each returns only after the
    /// workers have exited.
    fn shutdown(&self) {
        {
            let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            queue.shutdown = true;
            self.has_job.notify_all();
            self.has_space.notify_all();
        }
        for token in self
            .live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
        {
            token.cancel();
        }
        // The guard is held across the joins: a second concurrent
        // `shutdown` blocks here and observes the drained (empty) vec
        // only after the workers have actually exited.
        let mut workers = self.workers.lock().unwrap_or_else(PoisonError::into_inner);
        for worker in workers.drain(..) {
            let _ = worker.join();
        }
    }
}

/// Pool worker loop: pop tasks until the queue is empty AND shutdown
/// was requested, then exit. `shutdown` therefore drains queued jobs
/// (each sees its cancelled token and skips its body) before workers
/// terminate.
fn worker_loop(inner: &Inner) {
    loop {
        let task = {
            let mut queue = inner.queue.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(task) = queue.jobs.pop_front() {
                    // A queue slot freed: wake one blocked `submit`.
                    inner.has_space.notify_one();
                    break task;
                }
                if queue.shutdown {
                    return;
                }
                queue = inner
                    .has_job
                    .wait(queue)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        // The job body already maps its own panics to `JobError::Panicked`;
        // this guard additionally contains a panic in the delivery path
        // itself so one bad task cannot silently shrink the pool.
        let _ = catch_unwind(AssertUnwindSafe(task));
    }
}

/// Runs one queued job to completion on a pool worker.
///
/// Ordering contract: the result is published to `result_tx` BEFORE the
/// `Finished` state is stored, so [`QueuedJob::try_wait`] — which keys
/// on the state — can never observe a finished job whose result is not
/// yet in the channel.
fn run_job<F, T, E>(
    inner: &Inner,
    job_id: u64,
    operation: F,
    scope: &JobScope,
    state: &AtomicU8,
    events_tx: &SyncSender<JobEvent>,
    result_tx: &SyncSender<Result<T, JobError<E>>>,
) where
    F: FnOnce(&JobScope) -> Result<T, E>,
    T: Send + 'static,
    E: Send + 'static,
{
    let (outcome, result) = if scope.is_cancelled() {
        // Cancelled while still queued: skip the body entirely — the
        // whole point of queue-level cancellation. There is no
        // operation error to preserve, hence `JobError::Cancelled`.
        // No `Started` event: the body never ran.
        (JobOutcome::Cancelled, Err(JobError::Cancelled))
    } else {
        // Start marker: this worker is the only sender on the fresh
        // per-job channel, so `send` cannot block; it fails only when
        // the handle was already dropped.
        let _ = events_tx.send(JobEvent::Started);
        let body = catch_unwind(AssertUnwindSafe(|| operation(scope)));
        let cancelled = scope.cancel_token().is_cancelled();
        match body {
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
        }
    };
    let _ = result_tx.send(result);
    state.store(STATE_FINISHED, Ordering::Release);
    // Best-effort like OfflineJob: channel-close is the terminal marker.
    let _ = events_tx.try_send(JobEvent::Finished { outcome });
    inner
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&job_id);
}

/// Bounded non-RT job executor: a fixed pool of worker threads fed by a
/// bounded queue.
///
/// `JobExecutor` accepts job bodies of the same shape as
/// [`OfflineJob::spawn`](crate::OfflineJob::spawn) —
/// `FnOnce(&JobScope) -> Result<T, E>` — and runs them on pool workers
/// instead of one dedicated thread per job. `workers + queue_capacity`
/// bounds the number of in-flight jobs; submitting beyond that either
/// blocks ([`submit`](Self::submit)) or fails fast
/// ([`try_submit`](Self::try_submit) → [`SubmitError::QueueFull`]).
///
/// `Send + Sync`, no `Clone`: share through `Arc`; [`shutdown`](Self::shutdown)
/// is callable through the shared reference.
///
/// **RT contract:** non-RT only — thread spawn/join, mutex/condvar and
/// channel operations; never call from an RT audio path.
pub struct JobExecutor {
    inner: Arc<Inner>,
}

impl JobExecutor {
    /// Creates an executor with `config.workers` worker threads and a
    /// queue bounded to `config.queue_capacity` jobs.
    ///
    /// **RT contract:** non-RT only — allocates and spawns threads.
    ///
    /// # Errors
    ///
    /// [`ExecutorError::InvalidConfig`] when `workers` or
    /// `queue_capacity` is `0`.
    ///
    /// # Panics
    ///
    /// Panics when the OS fails to spawn a worker thread (same
    /// contract as [`std::thread::spawn`]). Workers already spawned are
    /// shut down and joined before the panic propagates.
    pub fn new(config: ExecutorConfig) -> Result<Self, ExecutorError> {
        if config.workers == 0 || config.queue_capacity == 0 {
            return Err(ExecutorError::InvalidConfig);
        }
        let inner = Arc::new(Inner {
            queue: Mutex::new(QueueState {
                jobs: VecDeque::new(),
                shutdown: false,
            }),
            has_job: Condvar::new(),
            has_space: Condvar::new(),
            capacity: config.queue_capacity,
            live: Mutex::new(HashMap::new()),
            next_job_id: AtomicU64::new(0),
            workers: Mutex::new(Vec::new()),
        });
        for index in 0..config.workers {
            let worker_inner = Arc::clone(&inner);
            match std::thread::Builder::new()
                .name(format!("ruviform-job-executor-{index}"))
                .spawn(move || worker_loop(&worker_inner))
            {
                Ok(handle) => inner
                    .workers
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(handle),
                Err(error) => {
                    // Stop already-spawned workers before propagating:
                    // detached workers would otherwise wait on the
                    // condvar forever — no shutdown signal would ever
                    // reach them.
                    inner.shutdown();
                    panic!("failed to spawn ruviform job executor worker {index}: {error}");
                }
            }
        }
        Ok(Self { inner })
    }

    /// Submits `operation` and blocks until it fits in the bounded
    /// queue, returning the [`QueuedJob`] handle.
    ///
    /// Back-pressure: while `workers + queue_capacity` jobs are already
    /// in flight the call waits for a worker to free queue space — or
    /// for [`shutdown`](Self::shutdown), which wakes parked submitters
    /// with [`SubmitError::Shutdown`].
    ///
    /// **Do not call this from a job body running on this executor** —
    /// the type-level deadlock contract applies; use
    /// [`try_submit`](Self::try_submit) there.
    ///
    /// **RT contract:** non-RT only — may block.
    ///
    /// # Errors
    ///
    /// [`SubmitError::Shutdown`] when the executor is shut down.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_exec::{ExecutorConfig, JobExecutor};
    ///
    /// let executor = JobExecutor::new(ExecutorConfig {
    ///     workers: 2,
    ///     queue_capacity: 8,
    /// })
    /// .unwrap();
    /// let job = executor
    ///     .submit(|_scope| Ok::<u64, std::io::Error>(42))
    ///     .unwrap();
    /// assert_eq!(job.wait().unwrap(), 42);
    /// executor.shutdown();
    /// ```
    pub fn submit<F, T, E>(&self, operation: F) -> Result<QueuedJob<T, E>, SubmitError>
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        self.submit_impl(operation, false)
    }

    /// [`submit`](Self::submit) without blocking: returns
    /// [`SubmitError::QueueFull`] immediately when `queue_capacity`
    /// jobs are already waiting. This is the non-blocking submission
    /// path for event loops and GUI threads — and the only safe submit
    /// from a job body running on this executor's own workers.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SubmitError::QueueFull`] when the queue is full;
    /// [`SubmitError::Shutdown`] when the executor is shut down.
    pub fn try_submit<F, T, E>(&self, operation: F) -> Result<QueuedJob<T, E>, SubmitError>
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        self.submit_impl(operation, true)
    }

    /// Shuts the executor down: rejects new submissions with
    /// [`SubmitError::Shutdown`], requests cooperative cancellation on
    /// every live job — queued jobs are skipped before their body runs,
    /// running jobs unwind at their next cooperative check — then
    /// blocks until the queue drains and all workers exit.
    ///
    /// Idempotent and callable through `Arc`/`&self`; also the [`Drop`]
    /// behavior, so an executor dropped without an explicit `shutdown`
    /// still cancels outstanding jobs and joins its workers. A job that
    /// never observes its cancellation token keeps `shutdown` waiting —
    /// there is no forced preemption. Calling `shutdown` from inside a
    /// job body running on this executor deadlocks: the worker would
    /// wait on itself.
    ///
    /// **RT contract:** non-RT only — blocks.
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Shared submit body; `try_only` selects the non-blocking
    /// [`try_submit`](Self::try_submit) path.
    fn submit_impl<F, T, E>(
        &self,
        operation: F,
        try_only: bool,
    ) -> Result<QueuedJob<T, E>, SubmitError>
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        // Build the job's shared surface before queueing, mirroring
        // `OfflineJob::spawn_impl`: the handle keeps the observe half
        // (Receiver side + listener-free Progress), the worker task
        // gets the emit half.
        let (events_tx, events_rx) = sync_channel::<JobEvent>(EVENT_CAPACITY);
        let (result_tx, result_rx) = sync_channel::<Result<T, JobError<E>>>(1);
        let cancel = CancellationToken::new();
        let state = Arc::new(AtomicU8::new(STATE_QUEUED));
        let progress = Progress::new();
        let mut scope_progress = progress.clone();
        let progress_tx = events_tx.clone();
        scope_progress.set_listener(move |completed, total| {
            // Coalesce like OfflineJob: an absolute snapshot supersedes
            // queued updates; the counters in `progress()` stay exact.
            let _ = progress_tx.try_send(JobEvent::Progress { completed, total });
        });
        let scope = JobScope::new(cancel.clone(), scope_progress);

        // Register for shutdown cancellation BEFORE the task becomes
        // visible to workers: `Inner::shutdown` cancels every token in
        // `live`, so a queued job can never slip past the sweep. The
        // queue mutex then serializes the push against the shutdown
        // flag — a job is either fully queued (and therefore
        // registered + cancellable) or rejected here.
        let job_id = self.inner.next_job_id.fetch_add(1, Ordering::Relaxed);
        self.inner
            .live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(job_id, cancel.clone());

        let task_inner = Arc::clone(&self.inner);
        let task_state = Arc::clone(&state);
        let task = Box::new(move || {
            run_job(
                &task_inner,
                job_id,
                operation,
                &scope,
                &task_state,
                &events_tx,
                &result_tx,
            );
        }) as JobTask;

        match self.enqueue(task, try_only) {
            Ok(()) => Ok(QueuedJob {
                state,
                cancel,
                progress,
                events: events_rx,
                result: Some(result_rx),
            }),
            Err(error) => {
                // Rejected before reaching a worker: drop the
                // live-bookkeeping entry. The task (carrying the
                // operation and both channel senders) drops with the
                // block, so nothing ever runs.
                self.inner
                    .live
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&job_id);
                Err(error)
            }
        }
    }

    /// Pushes `task` onto the bounded queue. With `try_only` the call
    /// fails fast on a full queue or shutdown; otherwise it blocks
    /// until a worker frees queue space or [`shutdown`](Self::shutdown)
    /// wakes it with [`SubmitError::Shutdown`]. The queue mutex
    /// serializes the push against the shutdown flag, so a pushed task
    /// is always registered in `live` and therefore cancellable.
    fn enqueue(&self, task: JobTask, try_only: bool) -> Result<(), SubmitError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if try_only {
            if queue.shutdown {
                return Err(SubmitError::Shutdown);
            }
            if queue.jobs.len() >= self.inner.capacity {
                return Err(SubmitError::QueueFull);
            }
        } else {
            while !queue.shutdown && queue.jobs.len() >= self.inner.capacity {
                queue = self
                    .inner
                    .has_space
                    .wait(queue)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            if queue.shutdown {
                return Err(SubmitError::Shutdown);
            }
        }
        queue.jobs.push_back(task);
        self.inner.has_job.notify_one();
        Ok(())
    }
}

impl Drop for JobExecutor {
    fn drop(&mut self) {
        self.inner.shutdown();
    }
}

/// Handle to a job accepted by [`JobExecutor`]: the same
/// observe/cancel/join surface as [`OfflineJob`](crate::OfflineJob),
/// except the body runs on a pool worker rather than a dedicated
/// thread, [`state`](Self::state) can report [`JobState::Queued`]
/// while the job waits for a worker, and a job cancelled before its
/// body starts resolves to [`JobError::Cancelled`].
///
/// `T` is the operation's success value, `E` its error type; the
/// handle is `Send` and may live in a registry, a GUI job panel or
/// another thread — registries themselves remain an application
/// concern.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct QueuedJob<T, E> {
    state: Arc<AtomicU8>,
    cancel: CancellationToken,
    progress: Progress,
    events: Receiver<JobEvent>,
    /// `None` once the result was consumed by `wait`/`try_wait` or the
    /// handle was `detach`ed — `Drop` reads it to decide whether
    /// cancellation still needs requesting (same role as
    /// `OfflineJob::worker: Option<JoinHandle>`).
    result: Option<Receiver<Result<T, JobError<E>>>>,
}

impl<T, E> QueuedJob<T, E> {
    /// Observed lifecycle state — `Queued` while waiting for a worker,
    /// then `Running`/`Cancelling`/`Finished` exactly like
    /// [`OfflineJob::state`](crate::OfflineJob::state). Non-blocking
    /// atomic read.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn state(&self) -> JobState {
        state_from_u8(self.state.load(Ordering::Acquire))
    }

    /// `true` when [`state`](Self::state) reports `Finished`.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state() == JobState::Finished
    }

    /// Requests cooperative cancellation: sets the shared token and
    /// moves `Queued`/`Running` → `Cancelling`. Idempotent; a
    /// `Finished` job ignores it. A job still in the queue is skipped
    /// by the worker before its body runs and resolves to
    /// [`JobError::Cancelled`]; a running job stops when its body next
    /// observes the token.
    ///
    /// **RT contract:** non-RT only.
    pub fn cancel(&self) {
        request_cancel(&self.cancel, &self.state);
    }

    /// The shared cancellation token — same contract as
    /// [`OfflineJob::cancel_token`](crate::OfflineJob::cancel_token).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// The shared progress counters — same contract as
    /// [`OfflineJob::progress`](crate::OfflineJob::progress).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// The event stream — same bounded, lossy contract as
    /// [`OfflineJob::events`](crate::OfflineJob::events), with one
    /// difference: `Started` is emitted by the pool worker when the
    /// body actually begins, so a job cancelled while queued emits
    /// only `Finished { outcome: Cancelled }` and no `Started`.
    /// Channel-close remains the terminal marker.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn events(&self) -> &Receiver<JobEvent> {
        &self.events
    }
}

impl<T: Send + 'static, E: Send + 'static> QueuedJob<T, E> {
    /// Non-blocking result retrieval — same contract as
    /// [`OfflineJob::try_wait`](crate::OfflineJob::try_wait): `Some`
    /// once the job reports `Finished` (the worker publishes the result
    /// before the state, so a finished job always has it), `None`
    /// while the job is still queued/running or after the result was
    /// consumed.
    ///
    /// **RT contract:** non-RT only.
    pub fn try_wait(&mut self) -> Option<Result<T, JobError<E>>> {
        if !self.is_finished() {
            return None;
        }
        let receiver = self.result.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.result = None;
                Some(result)
            }
            // Unreachable while the worker publishes the result before
            // the Finished state; keep the receiver so a later poll
            // retries instead of losing the result.
            Err(TryRecvError::Empty) => None,
            // All senders dropped without a result — an internal worker
            // fault, mapped like a panic.
            Err(TryRecvError::Disconnected) => {
                self.result = None;
                Some(Err(JobError::Panicked))
            }
        }
    }

    /// Blocks until the job finishes and returns the operation result.
    /// Does not request cancellation: `wait` on a still-queued or
    /// running job simply waits.
    ///
    /// # Panics
    ///
    /// Panics in the (internally impossible) case the worker exited
    /// without publishing a result, or when the result was already
    /// consumed by [`try_wait`](Self::try_wait) — same contract as
    /// [`OfflineJob::wait`](crate::OfflineJob::wait).
    ///
    /// **RT contract:** non-RT only.
    pub fn wait(mut self) -> Result<T, JobError<E>> {
        let receiver = self
            .result
            .take()
            .expect("queued job result already consumed");
        receiver
            .recv()
            .expect("queued job result missing after worker exit")
    }

    /// Consumes the handle without joining; the job keeps its place —
    /// a queued job still waits for a worker and runs to completion
    /// (or executor-shutdown cancellation) with the result dropped.
    /// Unlike dropping the handle, no cancellation is requested. The
    /// pool worker is never joined through the handle — the executor
    /// owns worker lifetimes.
    ///
    /// **RT contract:** non-RT only.
    pub fn detach(mut self) {
        // Taking the receiver marks the handle as intentionally
        // abandoned; `Drop` then skips the cancel request.
        drop(self.result.take());
    }
}

impl<T, E> Drop for QueuedJob<T, E> {
    fn drop(&mut self) {
        if self.result.is_some() {
            // The handle was neither waited nor detached: request
            // cancellation like `OfflineJob`'s Drop — a leaked handle
            // never keeps queued/running work alive silently. Nothing
            // to join: pool workers are owned by the executor.
            request_cancel(&self.cancel, &self.state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
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

    fn executor(workers: usize, queue_capacity: usize) -> JobExecutor {
        JobExecutor::new(ExecutorConfig {
            workers,
            queue_capacity,
        })
        .unwrap()
    }

    /// A cooperative body that spins until cancelled, then reports
    /// `Err("stopped")`; sets `ran` once the body actually starts.
    fn spin_until_cancelled(
        scope: &JobScope,
        ran: &Arc<AtomicU64>,
    ) -> Result<&'static str, &'static str> {
        ran.fetch_add(1, Ordering::Relaxed);
        while !scope.is_cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        Err("stopped")
    }

    #[test]
    fn new_rejects_zero_workers_and_zero_capacity() {
        for config in [
            ExecutorConfig {
                workers: 0,
                queue_capacity: 1,
            },
            ExecutorConfig {
                workers: 1,
                queue_capacity: 0,
            },
            ExecutorConfig {
                workers: 0,
                queue_capacity: 0,
            },
        ] {
            assert!(matches!(
                JobExecutor::new(config),
                Err(ExecutorError::InvalidConfig)
            ));
        }
    }

    #[test]
    fn submit_runs_body_and_wait_returns_result() {
        let executor = executor(2, 4);
        let job = executor
            .submit(|scope| {
                scope.progress().reset(2);
                scope.progress().bump();
                scope.progress().bump();
                Ok::<u64, std::io::Error>(42)
            })
            .unwrap();
        let events: Vec<JobEvent> = job.events().iter().collect();
        assert_eq!(events.first(), Some(&JobEvent::Started));
        assert_eq!(
            events.last(),
            Some(&JobEvent::Finished {
                outcome: JobOutcome::Completed
            })
        );
        assert_eq!(job.wait().unwrap(), 42);
        executor.shutdown();
    }

    #[test]
    fn try_wait_returns_the_result_exactly_once() {
        let executor = executor(1, 1);
        let mut job = executor
            .submit(|_scope| Ok::<u64, std::io::Error>(7))
            .unwrap();
        assert!(poll_until(Duration::from_secs(5), || job.is_finished()));
        assert_eq!(job.try_wait().unwrap().unwrap(), 7);
        assert!(job.try_wait().is_none(), "result already consumed");
        executor.shutdown();
    }

    #[test]
    fn queued_job_reports_queued_until_a_worker_picks_it_up() {
        let executor = executor(1, 2);
        let gate = Arc::new(AtomicU64::new(0));
        let gate_flag = Arc::clone(&gate);
        // Holds the single worker until released.
        let blocker = executor
            .submit(move |_scope| {
                while gate_flag.load(Ordering::Relaxed) == 0 {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok::<(), std::io::Error>(())
            })
            .unwrap();
        let queued = executor
            .submit(|_scope| Ok::<(), std::io::Error>(()))
            .unwrap();
        // FIFO: the queued job cannot start while the blocker holds the
        // only worker, so `Queued` is deterministic here.
        assert_eq!(queued.state(), JobState::Queued);
        gate.store(1, Ordering::Relaxed);
        blocker.wait().unwrap();
        queued.wait().unwrap();
        executor.shutdown();
    }

    #[test]
    fn try_submit_fails_closed_when_the_queue_is_full() {
        let executor = executor(1, 1);
        let gate = Arc::new(AtomicU64::new(0));
        let gate_flag = Arc::clone(&gate);
        let blocker = executor
            .submit(move |_scope| {
                while gate_flag.load(Ordering::Relaxed) == 0 {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok::<(), std::io::Error>(())
            })
            .unwrap();
        // workers(1) running + queue_capacity(1) waiting = the bound.
        let waiting = executor
            .submit(|_scope| Ok::<(), std::io::Error>(()))
            .unwrap();
        assert!(matches!(
            executor.try_submit(|_scope| Ok::<(), std::io::Error>(())),
            Err(SubmitError::QueueFull)
        ));
        gate.store(1, Ordering::Relaxed);
        blocker.wait().unwrap();
        waiting.wait().unwrap();
        executor.shutdown();
    }

    #[test]
    fn submit_blocks_until_queue_space_frees() {
        let executor = executor(1, 1);
        let executor = Arc::new(executor);
        let gate = Arc::new(AtomicU64::new(0));
        let gate_flag = Arc::clone(&gate);
        let blocker = executor
            .submit(move |_scope| {
                while gate_flag.load(Ordering::Relaxed) == 0 {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok::<u64, std::io::Error>(1)
            })
            .unwrap();
        let waiting = executor
            .submit(|_scope| Ok::<u64, std::io::Error>(2))
            .unwrap();
        let submitted = Arc::new(AtomicU64::new(0));
        let flag = Arc::clone(&submitted);
        let worker_executor = Arc::new(executor);
        let submitter_executor = Arc::clone(&worker_executor);
        let submit_thread = std::thread::spawn(move || {
            let job = submitter_executor
                .submit(|_scope| Ok::<u64, std::io::Error>(3))
                .unwrap();
            flag.store(1, Ordering::Relaxed);
            job.wait().unwrap()
        });
        // Queue is full (blocker running + `waiting` queued): the
        // submitter must still be parked.
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(submitted.load(Ordering::Relaxed), 0);
        gate.store(1, Ordering::Relaxed);
        assert_eq!(blocker.wait().unwrap(), 1);
        assert_eq!(waiting.wait().unwrap(), 2);
        assert_eq!(submit_thread.join().unwrap(), 3);
        worker_executor.shutdown();
    }

    #[test]
    fn cancel_queued_job_skips_the_body_and_reports_cancelled() {
        let executor = executor(1, 1);
        let ran = Arc::new(AtomicU64::new(0));
        let ran_flag = Arc::clone(&ran);
        let gate = Arc::new(AtomicU64::new(0));
        let gate_flag = Arc::clone(&gate);
        let blocker = executor
            .submit(move |_scope| {
                while gate_flag.load(Ordering::Relaxed) == 0 {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok::<(), std::io::Error>(())
            })
            .unwrap();
        let victim = executor
            .submit(move |scope| spin_until_cancelled(scope, &ran_flag))
            .unwrap();
        victim.cancel();
        assert_eq!(victim.state(), JobState::Cancelling);
        gate.store(1, Ordering::Relaxed);
        blocker.wait().unwrap();
        // The worker dequeued the cancelled job, skipped the body and
        // published `JobError::Cancelled`. Collect events before
        // `wait` consumes the handle; channel-close marks completion.
        let events: Vec<JobEvent> = victim.events().iter().collect();
        match victim.wait() {
            Err(JobError::Cancelled) => {}
            other => panic!("expected JobError::Cancelled, got {other:?}"),
        }
        assert_eq!(ran.load(Ordering::Relaxed), 0, "body must not run");
        assert!(
            !events.iter().any(|e| matches!(e, JobEvent::Started)),
            "a skipped job never emits Started"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                JobEvent::Finished {
                    outcome: JobOutcome::Cancelled
                }
            )),
            "expected Finished {{ Cancelled }}, got {events:?}"
        );
        executor.shutdown();
    }

    #[test]
    fn cancel_running_job_unwinds_cooperatively() {
        let executor = executor(1, 1);
        let ran = Arc::new(AtomicU64::new(0));
        let ran_flag = Arc::clone(&ran);
        let mut job = executor
            .submit(move |scope| spin_until_cancelled(scope, &ran_flag))
            .unwrap();
        assert!(poll_until(Duration::from_secs(5), || {
            ran.load(Ordering::Relaxed) == 1
        }));
        job.cancel();
        assert!(poll_until(Duration::from_secs(5), || job.is_finished()));
        match job.try_wait() {
            Some(Err(JobError::Operation("stopped"))) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
        executor.shutdown();
    }

    #[test]
    fn shutdown_rejects_submits_and_cancels_live_jobs() {
        let executor = executor(1, 2);
        let ran = Arc::new(AtomicU64::new(0));
        let ran_flag = Arc::clone(&ran);
        // Holds the only worker until cancelled — keeps the queued job
        // queued deterministically.
        let blocker = executor
            .submit(move |scope| spin_until_cancelled(scope, &ran_flag))
            .unwrap();
        let queued = executor
            .submit(|_scope| -> Result<(), std::io::Error> { panic!("queued body must never run") })
            .unwrap();
        assert!(poll_until(Duration::from_secs(5), || {
            ran.load(Ordering::Relaxed) == 1
        }));
        executor.shutdown();
        // Second call is idempotent and returns promptly.
        executor.shutdown();
        assert!(matches!(
            executor.submit(|_scope| Ok::<(), std::io::Error>(())),
            Err(SubmitError::Shutdown)
        ));
        assert!(matches!(
            executor.try_submit(|_scope| Ok::<(), std::io::Error>(())),
            Err(SubmitError::Shutdown)
        ));
        match queued.wait() {
            Err(JobError::Cancelled) => {}
            other => panic!("expected JobError::Cancelled, got {other:?}"),
        }
        match blocker.wait() {
            Err(JobError::Operation("stopped")) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
    }

    #[test]
    fn panic_in_body_maps_to_panicked_and_the_pool_continues() {
        let executor = executor(1, 1);
        let panicker = executor
            .submit(|_scope| -> Result<(), std::io::Error> { panic!("boom") })
            .unwrap();
        match panicker.wait() {
            Err(JobError::Panicked) => {}
            other => panic!("expected JobError::Panicked, got {other:?}"),
        }
        let next = executor
            .submit(|_scope| Ok::<u64, std::io::Error>(9))
            .unwrap();
        assert_eq!(next.wait().unwrap(), 9);
        executor.shutdown();
    }

    #[test]
    fn detach_lets_the_job_run_to_completion() {
        let executor = executor(1, 1);
        let done = Arc::new(AtomicU64::new(0));
        let flag = Arc::clone(&done);
        let job = executor
            .submit(move |_scope| {
                std::thread::sleep(Duration::from_millis(10));
                flag.store(1, Ordering::Relaxed);
                Ok::<(), std::io::Error>(())
            })
            .unwrap();
        job.detach();
        assert!(poll_until(Duration::from_secs(5), || {
            done.load(Ordering::Relaxed) == 1
        }));
        executor.shutdown();
    }

    #[test]
    fn dropping_a_handle_requests_cancellation() {
        let executor = executor(1, 1);
        let started = Arc::new(AtomicU64::new(0));
        let start_flag = Arc::clone(&started);
        let exited = Arc::new(AtomicU64::new(0));
        let flag = Arc::clone(&exited);
        let job = executor
            .submit(move |scope| {
                start_flag.store(1, Ordering::Relaxed);
                while !scope.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                flag.store(1, Ordering::Relaxed);
                Err::<(), _>(std::io::Error::other("cancelled"))
            })
            .unwrap();
        // Wait until the body is actually running: a drop while the job
        // is still `Queued` would skip the body entirely (correct
        // `JobError::Cancelled` semantics) and never reach this loop.
        assert!(poll_until(Duration::from_secs(5), || {
            started.load(Ordering::Relaxed) == 1
        }));
        drop(job);
        assert!(
            poll_until(Duration::from_secs(5), || exited.load(Ordering::Relaxed)
                == 1),
            "dropping the handle must request cancellation"
        );
        executor.shutdown();
    }

    #[test]
    fn wait_does_not_request_cancellation() {
        let executor = executor(1, 1);
        let job = executor
            .submit(|scope| {
                scope.progress().reset(1);
                while !scope.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err::<(), _>(std::io::Error::other("cancelled"))
            })
            .unwrap();
        let token = job.cancel_token().clone();
        let waiter = std::thread::spawn(move || job.wait());
        // `wait` alone must not cancel: the token stays unset while the
        // waiter is parked.
        std::thread::sleep(Duration::from_millis(50));
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(waiter.join().unwrap().is_err());
        executor.shutdown();
    }

    #[test]
    fn executor_is_send_sync_and_handle_is_send() {
        fn assert_send<T: Send>() {}
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<JobExecutor>();
        // `QueuedJob` holds an mpsc `Receiver` — `Send` but not `Sync`,
        // exactly like `OfflineJob`.
        assert_send::<QueuedJob<u64, std::io::Error>>();
    }

    #[test]
    fn dropped_executor_shuts_down_and_cancels_live_jobs() {
        let ran = Arc::new(AtomicU64::new(0));
        let ran_flag = Arc::clone(&ran);
        let executor = executor(1, 1);
        let job = executor
            .submit(move |scope| spin_until_cancelled(scope, &ran_flag))
            .unwrap();
        // Ensure the body is running before the drop so the result is
        // deterministically `Operation`, not the queued-skip `Cancelled`.
        assert!(poll_until(Duration::from_secs(5), || {
            ran.load(Ordering::Relaxed) == 1
        }));
        // The executor is dropped without an explicit shutdown: Drop
        // must still cancel the live job so `wait` can resolve.
        drop(executor);
        match job.wait() {
            Err(JobError::Operation("stopped")) => {}
            other => panic!("expected operation error, got {other:?}"),
        }
    }

    #[test]
    fn concurrent_submitters_share_the_bounded_queue() {
        let executor = Arc::new(executor(2, 4));
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();
        for index in 0..4u64 {
            let executor = Arc::clone(&executor);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                executor
                    .submit(move |_scope| Ok::<u64, std::io::Error>(index))
                    .unwrap()
                    .wait()
                    .unwrap()
            }));
        }
        let mut results: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        results.sort_unstable();
        assert_eq!(results, vec![0, 1, 2, 3]);
        executor.shutdown();
    }
}
