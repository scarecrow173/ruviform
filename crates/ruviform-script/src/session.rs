//! C2 runtime port: the [`ScriptBackend`] factory trait and the
//! [`ScriptSession`] evaluation trait.
//!
//! Neither trait requires `Send`/`Sync` unconditionally — placement is
//! advertised via [`crate::BackendCapabilities`] (`send`/`sync`/
//! `owner_thread_only`) and requested via
//! [`SessionConfig::placement_hint`]. All session APIs are non-RT;
//! invoking them from a real-time context is a contract violation
//! owned by the caller.

use std::collections::BTreeSet;
use std::fmt;

use crate::budget::{CancelToken, ExecutionBudget, PlacementHint};
use crate::host::HostApiSurface;
use crate::id::{ScriptBackendId, ScriptLanguageId};
use crate::registry::BackendDescriptor;
use crate::value::{ArtifactKind, ScriptError, ScriptResult, ScriptSource, SourceDigest};

/// Configuration passed to [`ScriptBackend::open_session`].
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Resource limits; unsupported dimensions must fail closed with
    /// [`SessionError::UnsupportedBudget`].
    pub budget: ExecutionBudget,
    /// Declared host API surface. The backend must reject a surface
    /// that fails [`HostApiSurface::validate_neutral`] or carries an
    /// unsupported `schema_version`.
    pub host_bindings: HostApiSurface,
    /// Artifact kinds the consumer expects this session to produce.
    pub artifact_kinds: BTreeSet<ArtifactKind>,
    /// The script source to evaluate.
    pub source: ScriptSource,
    /// Optional placement hint (advisory).
    pub placement_hint: Option<PlacementHint>,
    /// Expected source digest when resuming an existing session's
    /// source lineage; a mismatch fails with
    /// [`SessionError::SourceDigestMismatch`] (fail-closed F-08).
    pub resume: Option<SourceDigest>,
}

/// Errors from [`ScriptBackend::open_session`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionError {
    /// The backend cannot honor a requested budget dimension.
    UnsupportedBudget {
        /// Which budget dimension is unsupported.
        kind: crate::budget::BudgetKind,
    },
    /// The host API surface is invalid or unsupported.
    InvalidHostSurface {
        /// Human-readable reason.
        reason: String,
    },
    /// `resume` digest does not match the session's source lineage.
    SourceDigestMismatch {
        /// Digest the consumer expected.
        expected: SourceDigest,
        /// Digest the backend actually holds.
        actual: SourceDigest,
    },
    /// Source exceeds the backend's size limit.
    SourceTooLarge {
        /// Source size in bytes.
        bytes: u64,
        /// Backend limit in bytes.
        max: u64,
    },
    /// The source language does not match the backend's language.
    LanguageMismatch {
        /// Language the backend implements.
        expected: ScriptLanguageId,
        /// Language the source declares.
        actual: ScriptLanguageId,
    },
    /// The backend cannot provide the session isolation the consumer
    /// requires (F-16 — e.g. `shared_vm_state` backends where the
    /// consumer needs per-session isolation).
    IsolationViolation {
        /// Human-readable detail.
        detail: String,
    },
    /// A resource limit was hit while opening the session.
    ResourceExhausted {
        /// Human-readable detail.
        detail: String,
    },
    /// Backend-internal failure.
    Backend {
        /// Human-readable detail.
        detail: String,
    },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBudget { kind } => {
                write!(f, "unsupported budget dimension: {kind:?}")
            }
            Self::InvalidHostSurface { reason } => {
                write!(f, "invalid host surface: {reason}")
            }
            Self::SourceDigestMismatch { expected, actual } => {
                write!(
                    f,
                    "source digest mismatch: expected {expected}, got {actual}"
                )
            }
            Self::SourceTooLarge { bytes, max } => {
                write!(f, "source too large: {bytes} bytes > {max}")
            }
            Self::LanguageMismatch { expected, actual } => {
                write!(
                    f,
                    "language mismatch: backend is {expected}, source is {actual}"
                )
            }
            Self::IsolationViolation { detail } => {
                write!(f, "isolation violation: {detail}")
            }
            Self::ResourceExhausted { detail } => {
                write!(f, "resource exhausted: {detail}")
            }
            Self::Backend { detail } => write!(f, "backend error: {detail}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// A script runtime backend: a factory for [`ScriptSession`]s.
///
/// Implementations are registered in a [`crate::ScriptBackendRegistry`]
/// as `Box<dyn ScriptBackend>`. `open_session` is non-RT.
pub trait ScriptBackend {
    /// Returns the backend's descriptor (identity, language, version,
    /// capabilities).
    fn descriptor(&self) -> &BackendDescriptor;

    /// Opens a session for `config`.
    ///
    /// Must fail closed: unknown languages, unsupported budgets,
    /// invalid host surfaces, and isolation violations are
    /// [`SessionError`]s — never silent fallbacks.
    fn open_session(&self, config: SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError>;
}

/// An open script evaluation session.
///
/// `evaluate`, `cancel`, and `close` are non-RT. Cancellation is
/// cooperative via [`CancelToken`]; a blocking host call may not be
/// interruptible until it returns, and consumers may discard results
/// produced after cancellation.
pub trait ScriptSession {
    /// This session's identifier.
    fn session_id(&self) -> crate::id::SessionId;

    /// The backend that owns this session.
    fn backend_id(&self) -> &ScriptBackendId;

    /// Evaluates the session source (or `entry` override, when the
    /// backend supports named entry points).
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError`] on parse/compile/type/runtime/budget/
    /// cancellation failures. Success does not grant artifact trust —
    /// consumers verify digests and provenance themselves.
    fn evaluate(&mut self, entry: Option<&str>) -> Result<ScriptResult, ScriptError>;

    /// Returns the session's shared cancellation token.
    ///
    /// The default implementation delegates to [`Self::cancel_token`];
    /// backends may override to also signal in-flight work.
    fn cancel(&mut self) {
        self.cancel_token().cancel();
    }

    /// Returns a clone of the session's cancellation token so a
    /// consumer can cancel from another thread.
    fn cancel_token(&self) -> CancelToken;

    /// Closes the session, releasing backend resources. Idempotent —
    /// implementations must tolerate repeated calls.
    fn close(&mut self);
}
