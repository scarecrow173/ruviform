//! The common plugin lifecycle state machine: [`LifecycleState`], the
//! [`LifecycleEvent`]s that drive it, the [`Lifecycle`] stateful driver
//! and the [`LifecycleError`] that surfaces illegal transitions.
//!
//! ## Transition table
//!
//! The machine is defined *as a transition table*. Rows not listed are
//! illegal and surface as
//! [`LifecycleError::IllegalTransition`]:
//!
//! | From | Event | To | Notes |
//! | --- | --- | --- | --- |
//! | `Created` | `Initialize` | `Initialized` | init failure → `InitFailed` |
//! | `Created` | `InitFailed` | `Created` | init failure destination |
//! | `Initialized` | `Activate` | `Activated` | activate failure → `ActivateFailed` |
//! | `Initialized` | `ActivateFailed` | `Initialized` | activate failure destination |
//! | `Activated` | `StartProcessing` | `Processing` | |
//! | `Processing` | `StopProcessing` | `Activated` | processing stopped, stays activated |
//! | `Activated` | `Deactivate` | `Initialized` | frees activation resources |
//! | `Processing` | `Process` | `Processing` | processing self-loop |
//! | `Created` | `Destroy` | `Destroyed` | uninitialized instance may be destroyed |
//! | `Initialized` | `Destroy` | `Destroyed` | destroy requires the deactivated state |
//! | `Created`/`Initialized`/`Activated`/`Processing` | `Failed` | `Failed` | reported terminal execution-context failure |
//! | `Failed` | `Destroy` | `Destroyed` | the only escape from `Failed` |
//! | *any other pair* | | | `Err(IllegalTransition)` |
//!
//! Defined rules, in words:
//!
//! - **Forward chain** — `create → init → activate → processing →
//!   deactivate → destroy` is the happy path.
//! - **`Processing` is activity, not an RT guarantee** — entering
//!   `Processing` describes control-plane activity; it does not imply
//!   real-time-safe execution. Video ports (e.g. `VideoPluginInstance`) run
//!   all operations NONRT inside this state.
//!
//! - **Processing stop is not a deactivate** — `StopProcessing` lands
//!   in `Activated` (activated but not processing; CLAP
//!   `stop_processing` / VST3 `setProcessing(false)` map here).
//!   `Deactivate` is legal only from `Activated`, i.e. it requires
//!   processing to be stopped first.
//! - **Re-activation cycles are legal** — `Activated ⇄ Processing`
//!   round-trips freely, and a deactivated plugin (`Activated →
//!   Deactivate → Initialized`) may be activated again
//!   (`Initialized → Activate`).
//! - **Failure destinations** — a failed init stays in `Created`
//!   (`InitFailed`), a failed activate stays in `Initialized`
//!   (`ActivateFailed`); the operation may be retried.
//! - **`Failed` is a terminal semi-state** — a *reported*
//!   execution-context failure (an out-of-process child dying, losing
//!   its response or breaking the protocol — injected by the host,
//!   never emitted by the plugin itself). Only `Destroy` escapes it;
//!   every other event is illegal. Unlike the retryable
//!   `InitFailed`/`ActivateFailed` destinations there is no recovery —
//!   the instance can only be torn down.
//! - **`Destroy` requires prior deactivate** — `Destroy` is legal only
//!   from `Initialized` (the post-deactivate state), `Created` (never
//!   initialized) and `Failed` (the failed context is only torn down,
//!   never recovered); destroying an `Activated` or `Processing`
//!   instance is a protocol violation.
//! - **Protocol violations are rejected** — `Process` before
//!   activation (`Created`/`Initialized`), `Process` while activated
//!   but not started, `Deactivate` while `Processing`, `Deactivate`
//!   before any activation, and *any* event on `Destroyed` all return
//!   `Err(IllegalTransition)`.

use core::fmt;

/// A node of the lifecycle state machine.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LifecycleState {
    /// Instantiated; `Initialize` has not (successfully) run.
    Created,
    /// Initialized and not activated — also the post-deactivate
    /// state (`Deactivate` lands here).
    Initialized,
    /// Activated: processing resources allocated, not yet processing.
    Activated,
    /// Processing activity is running (`Process` is legal) —
    /// activity only, not an RT guarantee.
    Processing,
    /// Terminal execution-context failure: the context the instance
    /// ran in is gone (e.g. an out-of-process child died or broke the
    /// protocol). Reached only through the reported
    /// [`LifecycleEvent::Failed`] event; the only legal way out is
    /// `Destroy` → `Destroyed`. Unlike the retryable
    /// `InitFailed`/`ActivateFailed` destinations, `Failed` is not
    /// recoverable — `accepts_processing()` is `false`.
    Failed,
    /// Terminal state — every event is illegal from here.
    Destroyed,
}

impl LifecycleState {
    /// The transition table as a pure function: the legal
    /// `self --event--> next` pairs of the module-doc table.
    ///
    /// **RT contract:** RT-suitable — pure match, allocation-free.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::IllegalTransition`] for every pair not in the
    /// table.
    pub const fn transition(self, event: LifecycleEvent) -> Result<Self, LifecycleError> {
        use LifecycleEvent as E;
        use LifecycleState as S;
        let next = match (self, event) {
            (S::Created, E::InitFailed) => S::Created,
            (S::Created, E::Initialize)
            | (S::Initialized, E::ActivateFailed)
            | (S::Activated, E::Deactivate) => S::Initialized,
            (S::Initialized, E::Activate) | (S::Processing, E::StopProcessing) => S::Activated,
            (S::Activated, E::StartProcessing) | (S::Processing, E::Process) => S::Processing,
            (S::Created | S::Initialized | S::Activated | S::Processing, E::Failed) => S::Failed,
            (S::Created | S::Initialized | S::Failed, E::Destroy) => S::Destroyed,
            _ => {
                return Err(LifecycleError::IllegalTransition { from: self, event });
            }
        };
        Ok(next)
    }

    /// Whether `Process` events are legal from this state — exactly
    /// [`LifecycleState::Processing`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn accepts_processing(self) -> bool {
        matches!(self, Self::Processing)
    }

    /// Whether this is the terminal [`LifecycleState::Destroyed`]
    /// state.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        matches!(self, Self::Destroyed)
    }

    /// Whether this is the [`LifecycleState::Failed`] semi-terminal
    /// state — a dead execution context that only `Destroy` escapes.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// An operation that drives the lifecycle machine — the standard-neutral
/// events adapters feed in.
///
/// `InitFailed` / `ActivateFailed` / `Failed` are *reported*
/// transitions: a failure destination is reported explicitly instead
/// of silently staying. `Process` is the per-block self-loop that
/// makes "process before activation" a defined, rejectable
/// transition.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LifecycleEvent {
    /// `init` — `Created → Initialized` on success.
    Initialize,
    /// Reported init failure — `Created → Created`.
    InitFailed,
    /// `activate` (allocate/prepare) — `Initialized → Activated`.
    Activate,
    /// Reported activate failure — `Initialized → Initialized`.
    ActivateFailed,
    /// Begin RT processing — `Activated → Processing`.
    StartProcessing,
    /// End RT processing — `Processing → Activated` (the plugin stays
    /// activated; deactivation needs a separate `Deactivate`).
    StopProcessing,
    /// Free activation resources — `Activated → Initialized`;
    /// requires processing to be stopped first.
    Deactivate,
    /// Per-block processing — `Processing → Processing`; illegal from
    /// every other state.
    Process,
    /// Terminal transition — legal from `Created`/`Initialized` (and
    /// from `Failed`, where it is the only legal event); an activated
    /// plugin must be deactivated first.
    Destroy,
    /// Reported terminal execution-context failure — `Created` /
    /// `Initialized` / `Activated` / `Processing` → `Failed`.
    ///
    /// The host injects this event when the instance's execution
    /// context is lost (an L1 child process dying, losing its
    /// response or breaking the protocol); plugins never emit it.
    /// Unlike the retryable `InitFailed`/`ActivateFailed`
    /// destinations, `Failed` is terminal — only `Destroy` escapes it.
    Failed,
}

/// Rejection of an illegal lifecycle transition.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{LifecycleError, LifecycleEvent, LifecycleState};
///
/// // Process before activation is a defined rejection.
/// let err = LifecycleState::Created
///     .transition(LifecycleEvent::Process)
///     .unwrap_err();
/// assert_eq!(
///     err,
///     LifecycleError::IllegalTransition {
///         from: LifecycleState::Created,
///         event: LifecycleEvent::Process,
///     }
/// );
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LifecycleError {
    /// `event` is not legal from `from` — a host protocol violation
    /// (or an adapter mapping bug).
    IllegalTransition {
        /// The state the event was attempted from.
        from: LifecycleState,
        /// The rejected event.
        event: LifecycleEvent,
    },
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalTransition { from, event } => write!(
                f,
                "illegal lifecycle transition: `{event:?}` is not legal from `{from:?}`"
            ),
        }
    }
}

impl core::error::Error for LifecycleError {}

/// The stateful lifecycle driver: current state plus checked
/// transitions.
///
/// Adapters hold one `Lifecycle` per plugin instance and feed it
/// [`LifecycleEvent`]s as host calls arrive; illegal host sequences
/// surface as [`LifecycleError`] instead of corrupting plugin state.
///
/// **RT contract:** transitions are `const` table lookups —
/// allocation-free and non-blocking; `Process` is safe to feed once
/// per block.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{Lifecycle, LifecycleEvent, LifecycleState};
///
/// let mut lifecycle = Lifecycle::new();
/// for event in [
///     LifecycleEvent::Initialize,
///     LifecycleEvent::Activate,
///     LifecycleEvent::StartProcessing,
/// ] {
///     lifecycle.transition(event).unwrap();
/// }
/// assert_eq!(lifecycle.state(), LifecycleState::Processing);
///
/// // Deactivate while processing is a defined rejection.
/// assert!(lifecycle.transition(LifecycleEvent::Deactivate).is_err());
///
/// lifecycle.transition(LifecycleEvent::StopProcessing).unwrap();
/// lifecycle.transition(LifecycleEvent::Deactivate).unwrap();
/// lifecycle.transition(LifecycleEvent::Destroy).unwrap();
/// assert_eq!(lifecycle.state(), LifecycleState::Destroyed);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lifecycle {
    state: LifecycleState,
}

impl Default for Lifecycle {
    /// [`Lifecycle::new`] — starts in [`LifecycleState::Created`].
    fn default() -> Self {
        Self::new()
    }
}

impl Lifecycle {
    /// A fresh instance in [`LifecycleState::Created`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: LifecycleState::Created,
        }
    }

    /// The current state.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn state(self) -> LifecycleState {
        self.state
    }

    /// Feeds `event` through the transition table: on success the
    /// machine moves and the new state is returned; on an illegal pair
    /// the machine stays put and `Err` is returned.
    ///
    /// **RT contract:** RT-suitable — pure match, allocation-free.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::IllegalTransition`] — `event` is not legal
    /// from the current state.
    pub const fn transition(
        &mut self,
        event: LifecycleEvent,
    ) -> Result<LifecycleState, LifecycleError> {
        match self.state.transition(event) {
            Ok(next) => {
                self.state = next;
                Ok(next)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LifecycleEvent as E;
    use LifecycleState as S;

    fn lifecycle_at(state: S) -> Lifecycle {
        // Drive a fresh machine to `state` along the legal path.
        let mut lifecycle = Lifecycle::new();
        let path = match state {
            S::Created => return lifecycle,
            S::Initialized => &[E::Initialize][..],
            S::Activated => &[E::Initialize, E::Activate][..],
            S::Processing => &[E::Initialize, E::Activate, E::StartProcessing][..],
            // `Failed` is reachable straight from `Created` — no init
            // required.
            S::Failed => &[E::Failed][..],
            S::Destroyed => &[E::Initialize, E::Destroy][..],
        };
        for event in path {
            lifecycle.transition(*event).unwrap();
        }
        assert_eq!(lifecycle.state(), state);
        lifecycle
    }

    #[test]
    fn forward_chain_is_legal() {
        let mut lifecycle = Lifecycle::new();
        assert_eq!(lifecycle.state(), S::Created);
        let chain = [
            (E::Initialize, S::Initialized),
            (E::Activate, S::Activated),
            (E::StartProcessing, S::Processing),
            (E::StopProcessing, S::Activated),
            (E::Deactivate, S::Initialized),
            (E::Destroy, S::Destroyed),
        ];
        for (event, expected) in chain {
            assert_eq!(lifecycle.transition(event), Ok(expected));
        }
    }

    #[test]
    fn stop_processing_lands_in_activated_not_initialized() {
        let mut lifecycle = lifecycle_at(S::Processing);
        assert_eq!(lifecycle.transition(E::StopProcessing), Ok(S::Activated));
        // Activated, not deactivated: Deactivate is still required to
        // release resources and is legal from here.
        assert_eq!(lifecycle.transition(E::Deactivate), Ok(S::Initialized));
    }

    #[test]
    fn reactivation_cycles_are_legal() {
        let mut lifecycle = lifecycle_at(S::Activated);
        for _ in 0..3 {
            assert_eq!(lifecycle.transition(E::StartProcessing), Ok(S::Processing));
            assert_eq!(lifecycle.transition(E::Process), Ok(S::Processing));
            assert_eq!(lifecycle.transition(E::StopProcessing), Ok(S::Activated));
        }
        // Deactivated plugins may be activated again.
        lifecycle.transition(E::Deactivate).unwrap();
        assert_eq!(lifecycle.transition(E::Activate), Ok(S::Activated));
    }

    #[test]
    fn failure_destinations_stay_put() {
        let mut created = Lifecycle::new();
        assert_eq!(created.transition(E::InitFailed), Ok(S::Created));
        assert_eq!(created.state(), S::Created);
        // InitFailed is only legal from Created.
        assert_eq!(
            lifecycle_at(S::Initialized).transition(E::InitFailed),
            Err(LifecycleError::IllegalTransition {
                from: S::Initialized,
                event: E::InitFailed,
            })
        );

        let mut initialized = lifecycle_at(S::Initialized);
        assert_eq!(
            initialized.transition(E::ActivateFailed),
            Ok(S::Initialized)
        );
        // A failed activate may be retried.
        assert_eq!(initialized.transition(E::Activate), Ok(S::Activated));
    }

    #[test]
    fn destroy_requires_prior_deactivate() {
        for state in [S::Activated, S::Processing] {
            assert_eq!(
                lifecycle_at(state).transition(E::Destroy),
                Err(LifecycleError::IllegalTransition {
                    from: state,
                    event: E::Destroy,
                })
            );
        }
        // Legal from Initialized (the post-deactivate state) and
        // Created (never initialized).
        for state in [S::Created, S::Initialized] {
            assert_eq!(lifecycle_at(state).transition(E::Destroy), Ok(S::Destroyed));
        }
    }

    #[test]
    fn protocol_violations_are_rejected() {
        // Process before activate / while activated-not-processing.
        for state in [S::Created, S::Initialized, S::Activated, S::Destroyed] {
            assert_eq!(
                lifecycle_at(state).transition(E::Process),
                Err(LifecycleError::IllegalTransition {
                    from: state,
                    event: E::Process,
                })
            );
        }
        // Deactivate while processing, or before any activation.
        for state in [S::Processing, S::Created, S::Initialized] {
            assert!(lifecycle_at(state).transition(E::Deactivate).is_err());
        }
        // Double init, re-init, activate before init.
        for (state, event) in [
            (S::Initialized, E::Initialize),
            (S::Processing, E::Initialize),
            (S::Created, E::Activate),
            (S::Created, E::StartProcessing),
            (S::Destroyed, E::Initialize),
        ] {
            assert!(lifecycle_at(state).transition(event).is_err());
        }
        // Destroyed is terminal for every event — including the
        // reported-failure events and a double destroy.
        for event in [
            E::Initialize,
            E::Activate,
            E::StartProcessing,
            E::Process,
            E::StopProcessing,
            E::Deactivate,
            E::Destroy,
            E::Failed,
            E::InitFailed,
            E::ActivateFailed,
        ] {
            assert_eq!(
                lifecycle_at(S::Destroyed).transition(event),
                Err(LifecycleError::IllegalTransition {
                    from: S::Destroyed,
                    event,
                })
            );
        }
    }

    #[test]
    fn failed_is_reachable_from_every_live_state() {
        for state in [S::Created, S::Initialized, S::Activated, S::Processing] {
            assert_eq!(lifecycle_at(state).transition(E::Failed), Ok(S::Failed));
        }
    }

    #[test]
    fn failed_is_terminal_and_only_destroy_escapes() {
        // Every non-`Destroy` event — including re-reporting `Failed`
        // — is illegal from `Failed`.
        for event in [
            E::Initialize,
            E::InitFailed,
            E::Activate,
            E::ActivateFailed,
            E::StartProcessing,
            E::Process,
            E::StopProcessing,
            E::Deactivate,
            E::Failed,
        ] {
            assert_eq!(
                lifecycle_at(S::Failed).transition(event),
                Err(LifecycleError::IllegalTransition {
                    from: S::Failed,
                    event,
                })
            );
        }
        // The single escape: teardown.
        let mut failed = lifecycle_at(S::Failed);
        assert_eq!(failed.transition(E::Destroy), Ok(S::Destroyed));
        assert!(failed.state().is_destroyed());
    }

    #[test]
    fn failed_reports_via_state_queries() {
        let failed = lifecycle_at(S::Failed).state();
        assert!(failed.is_failed());
        assert!(!failed.is_destroyed());
        assert!(!failed.accepts_processing());
        // `Destroyed` and the live states are not `Failed`.
        assert!(!S::Destroyed.is_failed());
        assert!(!S::Processing.is_failed());
    }

    #[test]
    fn display_formats_illegal_transition() {
        let error = LifecycleError::IllegalTransition {
            from: S::Created,
            event: E::Process,
        };
        let text = error.to_string();
        assert!(text.contains("Process"));
        assert!(text.contains("Created"));
    }
}
