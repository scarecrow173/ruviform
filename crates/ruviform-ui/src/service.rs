//! Service port — the op surface application code registers with the
//! runtime.
//!
//! A [`Service`] answers intents either synchronously
//! ([`CallOutcome::Result`]) or by spawning a cancellable job
//! ([`CallOutcome::Job`]). Job workers run on their own threads and
//! report through [`JobContext`] — they never touch the UI thread or
//! the webview directly, which is what keeps the GUI ↔ non-UI
//! processing boundary identical to the CLI path (Issue #156 contract).

use serde_json::Value;

use crate::data::DataRegistry;
use crate::ipc::ErrorBody;

/// Metadata for one registered op (shown in `sys.capabilities`).
#[derive(Debug, Clone, Copy)]
pub struct OpInfo {
    /// Op name (`ns.name`; `sys.*` is reserved to the runtime).
    pub name: &'static str,
    /// One-line human description.
    pub summary: &'static str,
}

/// What [`Service::call`] decided for an intent.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum CallOutcome {
    /// Answer immediately with a `result` payload.
    Result(Value),
    /// Spawn a job; the intent gets a `result` ack carrying
    /// `{job, label}` and lifecycle follows as `job` events.
    Job(JobSpec),
}

/// Context handed to [`Service::call`].
pub struct ServiceContext<'a> {
    /// Shared data registry — services may publish [`crate::data::DataSource`]s
    /// for the pull path.
    pub data: &'a DataRegistry,
}

/// Worker body type of a [`JobSpec`].
type JobBody = dyn FnOnce(&JobContext) -> Result<Value, ErrorBody> + Send;

/// A unit of long-running work the runtime drives on a worker thread.
pub struct JobSpec {
    /// Human-readable label (diagnostics, `sys.capabilities` pending
    /// listings).
    pub label: &'static str,
    run: Box<JobBody>,
}

impl std::fmt::Debug for JobSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobSpec")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl JobSpec {
    /// Creates a job spec from its worker body.
    pub fn new(
        label: &'static str,
        run: impl FnOnce(&JobContext) -> Result<Value, ErrorBody> + Send + 'static,
    ) -> Self {
        Self {
            label,
            run: Box::new(run),
        }
    }

    /// Executes the job body (called once, on the worker thread).
    pub(crate) fn execute(self, ctx: &JobContext) -> Result<Value, ErrorBody> {
        (self.run)(ctx)
    }
}

/// Per-job handle visible to the worker: cancellation, coarse progress
/// reporting and the shared data registry.
///
/// Progress is latest-wins: reports travel a bounded mailbox and may
/// be dropped/coalesced under backpressure (mirroring the eval-push
/// coalescing measured in `poc/ui`). Terminal outcomes are never
/// dropped — return them from the job body. Each report may carry an
/// optional `detail` label (a stage name or the item being
/// processed) via [`progress_detail`](Self::progress_detail), which
/// the emitted `job` progress event exposes to the frontend
/// (Issue #172).
///
/// **RT contract:** non-RT only — worker threads.
pub struct JobContext {
    pub(crate) job: u64,
    pub(crate) cancel: ruviform_exec::CancellationToken,
    pub(crate) progress_tx: std::sync::mpsc::SyncSender<JobSignal>,
    pub(crate) data: DataRegistry,
}

impl JobContext {
    /// This job's id (correlates `job` events).
    #[must_use]
    pub fn job_id(&self) -> u64 {
        self.job
    }

    /// Whether cancellation was requested (observe at chunk
    /// boundaries; see [`JobContext::check_cancel`]).
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Cancellation token clone — pass into shared non-UI execution
    /// primitives (for example `ruviform_exec::OfflineDriver`) so the
    /// same cancel request governs job and driver alike.
    #[must_use]
    pub fn cancel_token(&self) -> ruviform_exec::CancellationToken {
        self.cancel.clone()
    }

    /// Chunk-boundary cancellation check — returns the `cancelled`
    /// [`ErrorBody`] the job body should propagate when a cancel
    /// request is pending.
    ///
    /// # Errors
    /// [`ErrorBody`] with code `cancelled` when cancellation was
    /// requested.
    pub fn check_cancel(&self) -> Result<(), ErrorBody> {
        if self.is_cancelled() {
            Err(ErrorBody::new(
                crate::ipc::codes::CANCELLED,
                "job cancelled",
            ))
        } else {
            Ok(())
        }
    }

    /// Reports progress; latest-wins under backpressure (dropped, not
    /// queued, when the mailbox is full).
    ///
    /// The report carries no detail label — use
    /// [`progress_detail`](Self::progress_detail) to attach one.
    pub fn progress(&self, done: u64, total: u64) {
        self.report(done, total, None);
    }

    /// Reports progress with a per-update `detail` label — a stage
    /// name, the item being processed, or any short human-readable
    /// string the frontend can show next to the counters (Issue #172).
    ///
    /// The detail rides this single report: it is attached to the
    /// emitted `job` progress event's optional `detail` field and
    /// does not carry over to later reports. A job that wants a
    /// persistent stage display passes the current stage name on
    /// every report; a frontend showing "which stage is running"
    /// keeps the detail from the most recent progress event that
    /// carried one until a terminal `job` event arrives.
    ///
    /// Under backpressure the same latest-wins coalescing as
    /// [`progress`](Self::progress) applies — the newest queued
    /// report (and its detail) wins.
    pub fn progress_detail(&self, done: u64, total: u64, detail: impl Into<String>) {
        self.report(done, total, Some(detail.into()));
    }

    fn report(&self, done: u64, total: u64, detail: Option<String>) {
        let _ = self.progress_tx.try_send(JobSignal::Progress {
            job: self.job,
            done,
            total,
            detail,
        });
    }

    /// Shared data registry — jobs publish display payloads here for
    /// the `rvf://data` pull path.
    #[must_use]
    pub fn data(&self) -> &DataRegistry {
        &self.data
    }
}

/// Internal job→runtime signal.
#[derive(Debug)]
pub(crate) enum JobSignal {
    Progress {
        job: u64,
        done: u64,
        total: u64,
        /// Per-report detail label (`None` for `progress()` reports).
        detail: Option<String>,
    },
    Finished {
        job: u64,
        result: JobResult,
    },
}

/// Terminal outcome of a job worker.
#[derive(Debug)]
pub(crate) enum JobResult {
    Ok(Value),
    Err(ErrorBody),
    Panic,
}

/// The op surface an application registers.
///
/// Implementations own app semantics (which ops exist, what `params`
/// mean); the runtime owns routing, `sys.*` ops, job lifecycle and the
/// wire schema. Services must not perform UI work — they return data
/// and jobs; the runtime emits events.
///
/// **RT contract:** non-RT only.
pub trait Service {
    /// Registered ops. `sys.*` names are rejected at registration.
    fn ops(&self) -> &'static [OpInfo];

    /// Handles `op` with `params`. Return [`CallOutcome::Result`] for
    /// synchronous answers or [`CallOutcome::Job`] for cancellable
    /// long-running work; `Err` becomes a `result` failure event.
    ///
    /// # Errors
    /// [`ErrorBody`] describing the failure (`invalid_params`,
    /// `service_error`, …).
    fn call(
        &mut self,
        op: &str,
        params: &Value,
        ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody>;
}
