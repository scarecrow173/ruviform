//! `rvf.ipc.v1` message schema — the typed contract between the
//! frontend and Rust.
//!
//! Two directions share one versioned envelope:
//!
//! - Frontend → Rust: [`Intent`] JSON over `window.ipc.postMessage`
//!   (string payloads only — the wry `with_ipc_handler` contract).
//! - Rust → frontend: [`Event`] JSON injected via `evaluate_script`
//!   (see [`crate::bridge::dispatch_script`]).
//!
//! Bulk/binary payloads do **not** travel these channels — they are
//! pulled through `rvf://data` ([`crate::data`]). This is the 3-path
//! boundary measured in `poc/ui` (RESULTS.md): intents/results as
//! JSON, state pushes as eval scripts, large data as binary fetches.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema version carried in every envelope (`"v": 1`).
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum accepted byte length of an intent body. Oversized payloads
/// are rejected with [`codes::MESSAGE_TOO_LARGE`] — bulk data belongs
/// to the `rvf://data` pull path.
pub const MAX_INTENT_BYTES: usize = 1024 * 1024;

/// Maximum byte length of a single op name (`ns.name`).
pub const MAX_OP_LEN: usize = 128;

/// Stable error codes carried in [`ErrorBody::code`].
pub mod codes {
    /// Intent body was not valid `rvf.ipc.v1` JSON or had a wrong
    /// version/kind/op shape.
    pub const MALFORMED_MESSAGE: &str = "malformed_message";
    /// Intent body exceeded [`crate::ipc::MAX_INTENT_BYTES`].
    pub const MESSAGE_TOO_LARGE: &str = "message_too_large";
    /// No service registered the requested op.
    pub const UNKNOWN_OP: &str = "unknown_op";
    /// The op exists but `params` failed validation.
    pub const INVALID_PARAMS: &str = "invalid_params";
    /// The requested job id is not running.
    pub const UNKNOWN_JOB: &str = "unknown_job";
    /// The service returned an error for the call.
    pub const SERVICE_ERROR: &str = "service_error";
    /// A job worker panicked.
    pub const WORKER_FAILED: &str = "worker_failed";
    /// The job observed cancellation.
    pub const CANCELLED: &str = "cancelled";
    /// The requested capability is unavailable in this build/config.
    pub const UNAVAILABLE: &str = "unavailable";
}

/// Structured error carried in-band in `result`/`job`/`error` events.
///
/// Codes are stable strings (see [`codes`]); `message` is
/// human-readable and `details` an optional machine-readable payload.
/// Do not leak internal paths or secrets into `message`/`details`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Stable machine-readable code (see [`codes`]).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional machine-readable detail payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ErrorBody {
    /// Creates an error body with a stable code and message.
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Whether this body reports job cancellation.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.code == codes::CANCELLED
    }
}

impl std::fmt::Display for ErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ErrorBody {}

/// Frontend → Rust call. One intent yields exactly one `result` event
/// correlated by [`Intent::id`]; long-running calls additionally emit
/// `job` events under a fresh job id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Schema version; must equal [`SCHEMA_VERSION`].
    pub v: u32,
    /// Message kind; must be `"intent"`.
    pub kind: String,
    /// Caller-chosen correlation id echoed back in the `result` event.
    pub id: String,
    /// Operation name (`ns.name` — `sys.*` is reserved to the runtime).
    pub op: String,
    /// Operation arguments (validated by the owning service).
    #[serde(default)]
    pub params: Value,
}

/// Job lifecycle states reported via `job` events.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Progress update (`done`/`total` fields present).
    Progress,
    /// Completed successfully (`data` present).
    Finished,
    /// Cancellation observed.
    Cancelled,
    /// Failed (`error` present).
    Failed,
}

/// Host lifecycle notifications pushed to the frontend.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// The window is about to close.
    Closing,
}

/// Rust → frontend event. Serialized inside [`Envelope`] as
/// `{"v":1,"kind":"<variant>","...":...}`.
///
/// - `result`: synchronous reply to an intent (correlated by `id`).
///   `ok` selects `data` vs `error`.
/// - `job`: asynchronous job lifecycle under a job id.
/// - `notify`: latest-wins state push (`seq` is monotonic — receivers
///   must keep only the newest; eval pushes may coalesce, see the
///   module docs and `poc/ui` RESULTS).
/// - `lifecycle`: host lifecycle notifications.
/// - `error`: message-level failure with no intent id to correlate
///   (for example an unparsable body).
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// Synchronous reply correlated by [`Intent::id`].
    Result {
        /// Correlation id of the intent this answers.
        id: String,
        /// Whether the call succeeded.
        ok: bool,
        /// Success payload.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        /// Failure payload.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<ErrorBody>,
    },
    /// Job lifecycle event.
    Job {
        /// Job id issued at spawn.
        job: u64,
        /// Lifecycle state.
        state: JobState,
        /// Completed units (`state == Progress`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done: Option<u64>,
        /// Total units (`state == Progress`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total: Option<u64>,
        /// Result payload (`state == Finished`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        /// Failure payload (`state == Failed`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<ErrorBody>,
    },
    /// Latest-wins state push; only the newest `seq` is meaningful.
    Notify {
        /// Monotonic sequence number.
        seq: u64,
        /// State payload.
        data: Value,
    },
    /// Host lifecycle notification.
    Lifecycle {
        /// Lifecycle state.
        state: LifecycleState,
    },
    /// Uncorrelated protocol error (no intent id available).
    Error {
        /// Failure payload.
        error: ErrorBody,
    },
}

/// Versioned wire envelope wrapping an [`Event`].
#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    /// Schema version; always [`SCHEMA_VERSION`].
    pub v: u32,
    /// Event payload (flattened into the envelope on the wire).
    #[serde(flatten)]
    pub event: Event,
}

impl Event {
    /// `result` event answering `id` successfully.
    pub fn result_ok(id: impl Into<String>, data: Value) -> Self {
        Self::Result {
            id: id.into(),
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// `result` event answering `id` with a failure.
    pub fn result_err(id: impl Into<String>, error: ErrorBody) -> Self {
        Self::Result {
            id: id.into(),
            ok: false,
            data: None,
            error: Some(error),
        }
    }

    /// `job` progress event.
    #[must_use]
    pub fn job_progress(job: u64, done: u64, total: u64) -> Self {
        Self::Job {
            job,
            state: JobState::Progress,
            done: Some(done),
            total: Some(total),
            data: None,
            error: None,
        }
    }

    /// `job` finished event.
    #[must_use]
    pub fn job_finished(job: u64, data: Value) -> Self {
        Self::Job {
            job,
            state: JobState::Finished,
            done: None,
            total: None,
            data: Some(data),
            error: None,
        }
    }

    /// `job` failed event.
    #[must_use]
    pub fn job_failed(job: u64, error: ErrorBody) -> Self {
        Self::Job {
            job,
            state: JobState::Failed,
            done: None,
            total: None,
            data: None,
            error: Some(error),
        }
    }

    /// `job` cancelled event.
    #[must_use]
    pub fn job_cancelled(job: u64) -> Self {
        Self::Job {
            job,
            state: JobState::Cancelled,
            done: None,
            total: None,
            data: None,
            error: None,
        }
    }

    /// `lifecycle` event.
    #[must_use]
    pub fn lifecycle(state: LifecycleState) -> Self {
        Self::Lifecycle { state }
    }

    /// Uncorrelated `error` event.
    #[must_use]
    pub fn error(error: ErrorBody) -> Self {
        Self::Error { error }
    }

    /// Wraps this event in the versioned [`Envelope`].
    #[must_use]
    pub fn envelope(&self) -> Envelope {
        Envelope {
            v: SCHEMA_VERSION,
            event: self.clone(),
        }
    }

    /// Serializes the versioned envelope to a JSON string.
    ///
    /// Serializing this closed set of types cannot fail; a defensive
    /// fallback keeps `evaluate_script` payloads well-formed regardless.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.envelope()).unwrap_or_else(|_| {
            r#"{"v":1,"kind":"error","error":{"code":"internal","message":"event serialization failed"}}"#
                .to_string()
        })
    }
}

/// Whether `op` is a syntactically valid op name (`ns.name`, segments
/// non-empty, `[a-z0-9_.-]` only, within [`MAX_OP_LEN`]).
#[must_use]
pub fn valid_op(op: &str) -> bool {
    !op.is_empty()
        && op.len() <= MAX_OP_LEN
        && op.contains('.')
        && op
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
        && op.split('.').all(|seg| !seg.is_empty())
}

/// Parses a raw IPC body into an [`Intent`], enforcing the envelope
/// contract (size cap, version, kind, op shape).
///
/// # Errors
/// Returns [`ErrorBody`] with [`codes::MESSAGE_TOO_LARGE`] or
/// [`codes::MALFORMED_MESSAGE`].
pub fn parse_intent(body: &str) -> Result<Intent, ErrorBody> {
    if body.len() > MAX_INTENT_BYTES {
        return Err(ErrorBody::new(
            codes::MESSAGE_TOO_LARGE,
            format!(
                "intent body {} exceeds {} bytes",
                body.len(),
                MAX_INTENT_BYTES
            ),
        ));
    }
    let intent: Intent = serde_json::from_str(body)
        .map_err(|e| ErrorBody::new(codes::MALFORMED_MESSAGE, format!("invalid intent: {e}")))?;
    if intent.v != SCHEMA_VERSION {
        return Err(ErrorBody::new(
            codes::MALFORMED_MESSAGE,
            format!("unsupported schema version {}", intent.v),
        ));
    }
    if intent.kind != "intent" {
        return Err(ErrorBody::new(
            codes::MALFORMED_MESSAGE,
            format!("unsupported kind {:?}", intent.kind),
        ));
    }
    if !valid_op(&intent.op) {
        return Err(ErrorBody::new(
            codes::MALFORMED_MESSAGE,
            format!("invalid op name {:?}", intent.op),
        ));
    }
    Ok(intent)
}
