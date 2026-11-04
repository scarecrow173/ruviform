//! Registration lease — the domain-side handle that owns a source's
//! *registration* (not its data) for the registration's lifetime.
//!
//! Ownership contract:
//!
//! - the lease never owns the reader or the observed cell — those
//!   moved into the [`MetricSource`](crate::source::MetricSource) the
//!   registry holds — so **dropping a lease cannot erase unreported
//!   deltas**; it can only mark the registration [`abandoned`]
//!   (observably, not silently);
//! - explicit [`RegistrationLease::retire`] consumes the lease and
//!   returns a [`RetiredSource`] ticket for observing the drain;
//! - `Drop` before `retire` calls [`SourceState::abandon`], making
//!   the missing quiescence declaration visible to the collector.
//!
//! Every operation is **non-RT only** — the last `Drop` of a lease or
//! ticket must never run on an RT path.

use std::sync::Arc;

use super::identity::SourceIdentity;
use super::lifecycle::{SourceLifecycle, SourceState};

/// Domain-side handle owning one source registration.
///
/// Obtained only through
/// [`ObservationRegistrar::register`](crate::source::ObservationRegistrar::register);
/// [`RegistrationLease::new`] exists for registrar implementations
/// (Issue #289's registry), not for domain code.
#[derive(Debug)]
pub struct RegistrationLease {
    identity: SourceIdentity,
    state: Arc<SourceState>,
}

impl RegistrationLease {
    /// Creates a lease over `identity` sharing `state` with the
    /// registry.
    ///
    /// Intended for registrar implementations — domains receive
    /// leases only from
    /// [`ObservationRegistrar::register`](crate::source::ObservationRegistrar::register).
    #[must_use]
    pub fn new(identity: SourceIdentity, state: Arc<SourceState>) -> Self {
        Self { identity, state }
    }

    /// The registrar-assigned identity of this registration.
    #[must_use]
    pub fn identity(&self) -> SourceIdentity {
        self.identity
    }

    /// The current lifecycle state of the registration.
    #[must_use]
    pub fn lifecycle(&self) -> SourceLifecycle {
        self.state.lifecycle()
    }

    /// Whether the registration was abandoned without `retire`.
    #[must_use]
    pub fn is_abandoned(&self) -> bool {
        self.state.is_abandoned()
    }

    /// Declares the domain's RT writer quiescent and begins
    /// `Active → Retiring`, consuming the lease.
    ///
    /// Call this only after every RT writer for the source has
    /// stopped — the collector may begin its final drain as soon as
    /// the transition lands. Returns a [`RetiredSource`] ticket for
    /// observing `FinalDrain → Removed` progress.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn retire(self) -> RetiredSource {
        let _ = self.state.request_retire();
        RetiredSource {
            identity: self.identity,
            state: Arc::clone(&self.state),
        }
    }
}

impl Drop for RegistrationLease {
    /// A lease dropped without [`retire`](Self::retire) abandons the
    /// registration — observable via
    /// [`SourceState::is_abandoned`], never a silent loss: the reader
    /// cell is owned by the registry and outlives the lease.
    ///
    /// A lease dropped after `retire` (or after a registrar-forced
    /// transition) is a no-op — the shared state has already left
    /// `Active`.
    fn drop(&mut self) {
        if self.state.lifecycle() == SourceLifecycle::Active {
            self.state.abandon();
        }
    }
}

/// Ticket returned by [`RegistrationLease::retire`] — a non-blocking
/// observation handle for the registration's remaining lifecycle
/// (`FinalDrain → Removed`, driven by the collector).
///
/// `Drop` is an explicit no-op: after `retire`, completion authority
/// belongs to the collector and dropping the ticket changes nothing.
/// **Non-RT only.**
#[derive(Debug)]
pub struct RetiredSource {
    identity: SourceIdentity,
    state: Arc<SourceState>,
}

impl RetiredSource {
    /// The registrar-assigned identity of the retired registration.
    #[must_use]
    pub fn identity(&self) -> SourceIdentity {
        self.identity
    }

    /// The current lifecycle state — poll for `Removed` to confirm
    /// the collector released the reader.
    #[must_use]
    pub fn lifecycle(&self) -> SourceLifecycle {
        self.state.lifecycle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::identity::{SourceGeneration, SourceId};

    fn identity() -> SourceIdentity {
        SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0))
    }

    #[test]
    fn retire_transitions_to_retiring_and_returns_ticket() {
        let state = Arc::new(SourceState::new());
        let lease = RegistrationLease::new(identity(), Arc::clone(&state));
        assert_eq!(lease.lifecycle(), SourceLifecycle::Active);

        let ticket = lease.retire();
        assert_eq!(ticket.lifecycle(), SourceLifecycle::Retiring);
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
        assert!(!state.is_abandoned());

        state.begin_final_drain().unwrap();
        assert_eq!(ticket.lifecycle(), SourceLifecycle::FinalDrain);
        state.mark_removed().unwrap();
        assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    }

    #[test]
    fn dropping_a_lease_without_retire_abandons_observably() {
        let state = Arc::new(SourceState::new());
        let lease = RegistrationLease::new(identity(), Arc::clone(&state));
        drop(lease);
        assert!(state.is_abandoned());
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
    }

    #[test]
    fn dropping_a_retired_lease_is_a_no_op() {
        let state = Arc::new(SourceState::new());
        let lease = RegistrationLease::new(identity(), Arc::clone(&state));
        let ticket = lease.retire();
        drop(ticket);
        assert!(!state.is_abandoned());
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
    }

    #[test]
    fn lease_drop_after_registrar_forced_transition_is_a_no_op() {
        let state = Arc::new(SourceState::new());
        let lease = RegistrationLease::new(identity(), Arc::clone(&state));
        // The registrar/collector may drive the lifecycle on its own
        // clone (e.g. collector shutdown forcing drains).
        assert!(state.request_retire());
        state.begin_final_drain().unwrap();
        drop(lease);
        assert!(!state.is_abandoned());
        assert_eq!(state.lifecycle(), SourceLifecycle::FinalDrain);
    }

    #[test]
    fn lease_and_ticket_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<RegistrationLease>();
        assert_send::<RetiredSource>();
    }
}
