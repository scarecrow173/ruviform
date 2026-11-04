//! Observation source contract — descriptors, identity, registration
//! port and lifecycle types.
//!
//! This module defines the *contract* a collector (Issue #289's
//! registry) implements and instrumented domains consume. It
//! deliberately contains **no registry, no polling, no metrics
//! aggregation and no tracing/logging types**.
//!
//! # Boundary
//!
//! - Registration, deregistration and descriptor changes are
//!   **non-RT only**: an RT writer never holds a registrar handle or
//!   registry lock; readers and metadata needed during RT operation
//!   are prepared beforehand.
//! - The reader cell of a registered source is owned by the registry
//!   — a dropped [`RegistrationLease`] can mark the registration
//!   `abandoned` but can never erase unreported observations.
//! - Failed registrations are typed [`RegistrationError`]s; they never
//!   implicitly decide whether an audio callback (or other RT path)
//!   may continue.

mod descriptor;
mod identity;
mod lease;
mod lifecycle;
mod metric_source;
mod registrar;
mod scope;

pub use descriptor::{
    AggregationPolicy, Confidentiality, CounterDescriptor, MetricDescriptor, MetricKind,
    ObservationUnit, ReportingHint, ReportingImpact, SourceSemantics, SourceTemperature,
};
pub use identity::{SourceGeneration, SourceId, SourceIdentity};
pub use lease::{RegistrationLease, RetiredSource};
pub use lifecycle::{LifecycleError, SourceLifecycle, SourceState};
pub use metric_source::{CounterSource, MetricReader, MetricSource, SourceBuildError};
pub use registrar::{ObservationRegistrar, RegistrationError};
pub use scope::ObservationScope;
