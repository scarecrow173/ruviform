//! Registration port — the non-RT boundary between instrumented
//! domains and a collector/registry.
//!
//! [`ObservationRegistrar`] is the object-safe port a collector
//! (Issue #289's registry) implements and hands to domains at setup
//! time. Registration, deregistration and descriptor changes are
//! **non-RT only**: an RT writer never holds a registrar handle, a
//! registry lock, or performs any operation defined here. Every
//! reader/metadata an RT writer needs is prepared before RT operation
//! begins.
//!
//! A failed registration is returned as a typed [`RegistrationError`]
//! — it never implicitly changes whether the audio callback (or any
//! other RT path) may continue; that decision stays with the caller.

use std::error::Error;
use std::fmt;

use super::lease::RegistrationLease;
use super::metric_source::{CounterSource, MetricSource};

/// Errors a registrar can return from
/// [`ObservationRegistrar::register`].
#[non_exhaustive]
#[derive(Debug)]
pub enum RegistrationError {
    /// The registrar/collector is stopped — no further registrations
    /// are accepted.
    RegistrarStopped,
    /// The source-count or series cardinality limit is reached.
    CapacityExceeded {
        /// The configured limit that was exceeded.
        limit: usize,
    },
    /// A different live source instance already registered the same
    /// key.
    KeyConflict {
        /// The conflicting metric key.
        key: &'static str,
    },
    /// The same source cell was registered before — re-registration
    /// or reuse of an already-registered source.
    SourceReused,
    /// Registration raced the registrar's final drain.
    FinalDrainInProgress,
    /// The descriptor failed validation (e.g. a `Histogram` kind
    /// without bucket boundaries, or an empty key).
    InvalidDescriptor {
        /// Why the descriptor is invalid.
        reason: &'static str,
    },
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::RegistrarStopped => f.write_str("registrar is stopped"),
            Self::CapacityExceeded { limit } => {
                write!(f, "source capacity exceeded (limit {limit})")
            }
            Self::KeyConflict { key } => {
                write!(f, "metric key already registered by another source: {key}")
            }
            Self::SourceReused => f.write_str("source cell is already registered"),
            Self::FinalDrainInProgress => {
                f.write_str("registration raced the registrar's final drain")
            }
            Self::InvalidDescriptor { reason } => {
                write!(f, "invalid metric descriptor: {reason}")
            }
        }
    }
}

impl Error for RegistrationError {}

/// Non-RT registration port for observation sources.
///
/// Object-safe (`Box<dyn ObservationRegistrar>`/`&dyn
/// ObservationRegistrar`) so a collector can hand the port to
/// domains behind a trait object. `Send + Sync` lets the port cross
/// thread boundaries during setup.
///
/// Implementations own each accepted [`MetricSource`] (including its
/// reader cell) for the registration's lifetime; the returned
/// [`RegistrationLease`] is the domain's only handle over the
/// registration lifecycle.
///
/// **Non-RT only** — all methods may allocate and lock.
pub trait ObservationRegistrar: Send + Sync {
    /// Registers `source` and returns the lease owning the
    /// registration lifecycle.
    fn register(&self, source: MetricSource) -> Result<RegistrationLease, RegistrationError>;

    /// Convenience for the common counter case — equivalent to
    /// `register(source.into())`.
    fn register_counter(
        &self,
        source: CounterSource,
    ) -> Result<RegistrationLease, RegistrationError> {
        self.register(MetricSource::from(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::core::counter;
    use crate::source::descriptor::{CounterDescriptor, ObservationUnit, ReportingHint};
    use crate::source::identity::{SourceGeneration, SourceId, SourceIdentity};
    use crate::source::lifecycle::{SourceLifecycle, SourceState};
    use crate::source::scope::ObservationScope;

    /// Minimal in-memory registrar proving the port is implementable:
    /// mints identities, keeps sources and their state cells, and
    /// enforces a capacity limit — enough to exercise the lease
    /// contract deterministically without a real collector.
    struct TestRegistrar {
        next: AtomicU64,
        limit: usize,
        sources: std::sync::Mutex<Vec<(MetricSource, Arc<SourceState>)>>,
    }

    impl ObservationRegistrar for TestRegistrar {
        fn register(&self, source: MetricSource) -> Result<RegistrationLease, RegistrationError> {
            let mut sources = self.sources.lock().unwrap();
            if sources.len() >= self.limit {
                return Err(RegistrationError::CapacityExceeded { limit: self.limit });
            }
            let raw = self.next.fetch_add(1, Ordering::Relaxed) + 1;
            let identity =
                SourceIdentity::new(SourceId::new(raw).unwrap(), SourceGeneration::new(0));
            let state = Arc::new(SourceState::new());
            sources.push((source, Arc::clone(&state)));
            Ok(RegistrationLease::new(identity, state))
        }
    }

    fn counter_source(key: &'static str) -> CounterSource {
        let (_, reader) = counter();
        CounterSource {
            descriptor: CounterDescriptor {
                key,
                unit: ObservationUnit::Count,
                reporting: ReportingHint::SnapshotOnly,
            },
            scope: ObservationScope::Instance,
            reader,
        }
    }

    #[test]
    fn registrar_port_is_object_safe_and_send_sync() {
        let registrar: Box<dyn ObservationRegistrar> = Box::new(TestRegistrar {
            next: AtomicU64::new(0),
            limit: 8,
            sources: std::sync::Mutex::new(Vec::new()),
        });
        let registrar: Arc<dyn ObservationRegistrar> = registrar.into();
        let dyn_ref: &dyn ObservationRegistrar = &*registrar;
        let lease = dyn_ref
            .register_counter(counter_source("test.dyn"))
            .unwrap();
        assert_eq!(lease.identity().id().get(), 1);
        assert_eq!(lease.lifecycle(), SourceLifecycle::Active);
    }

    #[test]
    fn capacity_exceeded_is_typed() {
        let registrar = TestRegistrar {
            next: AtomicU64::new(0),
            limit: 1,
            sources: std::sync::Mutex::new(Vec::new()),
        };
        registrar
            .register_counter(counter_source("test.first"))
            .unwrap();
        let err = registrar
            .register_counter(counter_source("test.second"))
            .unwrap_err();
        assert!(matches!(
            err,
            RegistrationError::CapacityExceeded { limit: 1 }
        ));
    }

    #[test]
    fn registered_source_lifecycle_is_driven_through_shared_state() {
        let registrar = TestRegistrar {
            next: AtomicU64::new(0),
            limit: 8,
            sources: std::sync::Mutex::new(Vec::new()),
        };
        let lease = registrar
            .register_counter(counter_source("test.lifecycle"))
            .unwrap();

        // Registry side drives Retiring → FinalDrain → Removed on its
        // own clone once the domain retires.
        let ticket = lease.retire();
        {
            let sources = registrar.sources.lock().unwrap();
            let (_, state) = &sources[0];
            state.begin_final_drain().unwrap();
            state.mark_removed().unwrap();
        }
        assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    }
}
