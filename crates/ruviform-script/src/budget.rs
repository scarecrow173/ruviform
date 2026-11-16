//! C4 budget and cancellation vocabulary.
//!
//! All budget fields are `Option`: `None` means "unlimited by explicit
//! consumer choice". A backend that cannot honor a requested budget
//! must fail at [`crate::ScriptBackend::open_session`] with
//! [`crate::SessionError::UnsupportedBudget`] — never silently ignore
//! it. Cancellation is cooperative: [`CancelToken`] is a shared atomic
//! flag a consumer can trip from another thread while evaluation runs;
//! the backend observes it at its advertised
//! [`crate::CancelGranularity`] and reports
//! [`crate::ScriptErrorKind::Cancelled`]. All APIs are non-RT.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Which [`ExecutionBudget`] dimension a [`crate::SessionError::UnsupportedBudget`]
/// refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum BudgetKind {
    /// `max_instructions`.
    Instructions,
    /// `max_wall_time`.
    WallTime,
    /// `max_memory_bytes`.
    MemoryBytes,
    /// `max_output_bytes`.
    OutputBytes,
    /// `max_host_calls`.
    HostCalls,
}

/// Resource limits for a script session. Every field is optional;
/// `None` means unlimited by explicit consumer choice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionBudget {
    /// Maximum executed instructions, if limited.
    pub max_instructions: Option<u64>,
    /// Maximum wall-clock duration, if limited.
    pub max_wall_time: Option<Duration>,
    /// Maximum memory footprint in bytes, if limited.
    pub max_memory_bytes: Option<u64>,
    /// Maximum output bytes, if limited.
    pub max_output_bytes: Option<u64>,
    /// Maximum host binding calls, if limited.
    pub max_host_calls: Option<u64>,
}

impl ExecutionBudget {
    /// An unlimited budget — every dimension `None`.
    #[must_use]
    pub fn unlimited() -> Self {
        Self::default()
    }
}

/// How promptly a backend observes cancellation.
///
/// This is advertised capability, not a guarantee of preemption: a
/// blocking host call may not be interruptible until it returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CancelGranularity {
    /// Cancellation is observed cooperatively at safe points
    /// (statement boundaries, host-call returns).
    #[default]
    Cooperative,
    /// Cancellation may additionally be observed at instruction
    /// granularity (higher runtime overhead).
    Instruction,
}

/// Cloneable cooperative cancellation flag shared between a consumer
/// and a [`crate::ScriptSession`].
///
/// Cancellation is idempotent: [`CancelToken::cancel`] may be called
/// any number of times and from any thread. A backend holding the
/// token polls [`CancelToken::is_cancelled`] at its advertised
/// [`CancelGranularity`]; a consumer holding a clone can cancel an
/// in-flight evaluation without waiting for the session call to
/// return.
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// Creates a fresh, non-cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the token cancelled. Idempotent.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Returns `true` once [`CancelToken::cancel`] has been called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Consumer hint for where a session's evaluation should run.
///
/// A hint only — the backend advertises its actual placement
/// capabilities via [`crate::BackendCapabilities`] and may ignore or
/// reject an incompatible hint at `open_session`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlacementHint {
    /// No preference.
    #[default]
    Any,
    /// Prefer running on the caller's thread.
    CallerThread,
    /// Prefer a dedicated thread owned by the session.
    DedicatedThread,
}
