//! Source lifecycle — the `Active → Retiring → FinalDrain → Removed`
//! state machine shared between a [`RegistrationLease`] and the
//! registry that owns the source's reader.
//!
//! Transition authority is split by method:
//!
//! - the *domain* (via its lease) declares writer quiescence with
//!   [`SourceState::request_retire`], or abandons the source with
//!   [`SourceState::abandon`];
//! - the *collector* performs [`SourceState::begin_final_drain`] once
//!   the writer is quiescent, then [`SourceState::mark_removed`] once
//!   the last value has been committed and the reader released.
//!
//! `FinalDrain` cannot be entered from `Active`: the writer's
//! quiescence declaration (`Retiring`, explicit or abandoned) is a
//! hard precondition for finalizing the last value and releasing the
//! reader.

use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Lifecycle of an observation source registration.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLifecycle {
    /// Registered and being observed.
    Active,
    /// The domain declared writer quiescence (`retire`) or abandoned
    /// its lease; the collector should perform the final drain.
    Retiring,
    /// The collector is committing the last value(s); the reader is
    /// still alive.
    FinalDrain,
    /// Fully removed — the reader has been released by the registry.
    Removed,
}

impl SourceLifecycle {
    const fn to_u8(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Retiring => 1,
            Self::FinalDrain => 2,
            Self::Removed => 3,
        }
    }

    const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Active,
            1 => Self::Retiring,
            2 => Self::FinalDrain,
            _ => Self::Removed,
        }
    }
}

/// Error returned when a lifecycle transition is attempted from a
/// state that does not allow it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    /// The transition was attempted from a state that does not allow
    /// it — e.g. `begin_final_drain` before writer quiescence, or any
    /// transition after `Removed`.
    InvalidTransition {
        /// The state the transition expected as its precondition.
        expected: SourceLifecycle,
        /// The state actually found.
        actual: SourceLifecycle,
    },
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidTransition { expected, actual } => write!(
                f,
                "invalid source lifecycle transition: expected {expected:?}, found {actual:?}"
            ),
        }
    }
}

impl Error for LifecycleError {}

/// Shared lifecycle cell behind a [`RegistrationLease`] and the
/// registry's copy of the same registration.
///
/// The registrar creates one `SourceState` per registration (inside
/// an `Arc`), hands a clone to the [`RegistrationLease`], and keeps a
/// clone for the registry — so neither side owns the other and no
/// reference cycle exists. Lifecycle-driving calls are intended for
/// registrar/collector implementations: domains observe progress
/// through the lease and
/// [`RetiredSource`](crate::source::RetiredSource) only.
///
/// **Non-RT only** — every transition is part of
/// registration/deregistration control flow; an RT writer never
/// touches this cell.
#[derive(Debug)]
pub struct SourceState {
    lifecycle: AtomicU8,
    abandoned: AtomicBool,
}

impl SourceState {
    /// Creates a state cell in [`SourceLifecycle::Active`].
    ///
    /// **Non-RT only** — intended for registrar implementations.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lifecycle: AtomicU8::new(SourceLifecycle::Active.to_u8()),
            abandoned: AtomicBool::new(false),
        }
    }

    /// The current lifecycle state — a `Relaxed` load.
    #[must_use]
    pub fn lifecycle(&self) -> SourceLifecycle {
        SourceLifecycle::from_u8(self.lifecycle.load(Ordering::Relaxed))
    }

    /// Whether the domain dropped its lease without an explicit
    /// `retire` — i.e. the source is draining without a confirmed
    /// writer-quiescence declaration.
    #[must_use]
    pub fn is_abandoned(&self) -> bool {
        self.abandoned.load(Ordering::Relaxed)
    }

    /// `Active → Retiring` — the domain's explicit declaration that
    /// its RT writer has quiesced and the final drain may begin.
    ///
    /// Returns `true` when this call performed the transition, or
    /// `false` when the state was already past `Active` (e.g. a
    /// previous `abandon`, or a registrar-forced removal).
    pub fn request_retire(&self) -> bool {
        self.transition(SourceLifecycle::Active, SourceLifecycle::Retiring)
    }

    /// Marks the registration abandoned and best-effort transitions
    /// `Active → Retiring`.
    ///
    /// Abandonment is deliberately *observable*: instead of silently
    /// erasing a source whose lease was dropped without `retire`, the
    /// flag lets the collector still drain and report the loss.
    pub fn abandon(&self) {
        self.abandoned.store(true, Ordering::Relaxed);
        let _ = self.request_retire();
    }

    /// `Retiring → FinalDrain` — the collector begins committing the
    /// last value(s).
    ///
    /// Fails with [`LifecycleError::InvalidTransition`] unless the
    /// writer quiescence declaration has happened (`Retiring`) — the
    /// final value is never committed before the writer is known to
    /// be quiescent.
    pub fn begin_final_drain(&self) -> Result<(), LifecycleError> {
        self.expected_transition(SourceLifecycle::Retiring, SourceLifecycle::FinalDrain)
    }

    /// `FinalDrain → Removed` — the collector finished the final
    /// drain; the registry may now release the reader.
    ///
    /// Fails with [`LifecycleError::InvalidTransition`] unless the
    /// state is `FinalDrain`.
    pub fn mark_removed(&self) -> Result<(), LifecycleError> {
        self.expected_transition(SourceLifecycle::FinalDrain, SourceLifecycle::Removed)
    }

    fn expected_transition(
        &self,
        expected: SourceLifecycle,
        next: SourceLifecycle,
    ) -> Result<(), LifecycleError> {
        if self.transition(expected, next) {
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition {
                expected,
                actual: self.lifecycle(),
            })
        }
    }

    fn transition(&self, expected: SourceLifecycle, next: SourceLifecycle) -> bool {
        self.lifecycle
            .compare_exchange(
                expected.to_u8(),
                next.to_u8(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
    }
}

impl Default for SourceState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_runs_active_to_removed() {
        let state = SourceState::new();
        assert_eq!(state.lifecycle(), SourceLifecycle::Active);
        assert!(state.request_retire());
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
        state.begin_final_drain().unwrap();
        assert_eq!(state.lifecycle(), SourceLifecycle::FinalDrain);
        state.mark_removed().unwrap();
        assert_eq!(state.lifecycle(), SourceLifecycle::Removed);
    }

    #[test]
    fn final_drain_requires_quiescence_declaration() {
        let state = SourceState::new();
        let err = state.begin_final_drain().unwrap_err();
        assert_eq!(
            err,
            LifecycleError::InvalidTransition {
                expected: SourceLifecycle::Retiring,
                actual: SourceLifecycle::Active,
            }
        );
    }

    #[test]
    fn removal_requires_final_drain() {
        let state = SourceState::new();
        assert!(state.mark_removed().is_err());
        assert!(state.request_retire());
        assert!(state.mark_removed().is_err());
        state.begin_final_drain().unwrap();
        state.mark_removed().unwrap();
    }

    #[test]
    fn removed_is_terminal() {
        let state = SourceState::new();
        assert!(state.request_retire());
        state.begin_final_drain().unwrap();
        state.mark_removed().unwrap();
        assert!(!state.request_retire());
        assert!(state.begin_final_drain().is_err());
        assert!(state.mark_removed().is_err());
    }

    #[test]
    fn abandon_marks_and_retires() {
        let state = SourceState::new();
        state.abandon();
        assert!(state.is_abandoned());
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
    }

    #[test]
    fn retire_is_idempotent_after_abandon() {
        let state = SourceState::new();
        state.abandon();
        assert!(!state.request_retire());
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
    }
}
