//! Opt-in `tracing` export layer — Issue #278 (OBS-03).
//!
//! This module is the non-RT conversion boundary between the
//! authoritative observability state and the standard `tracing`
//! ecosystem:
//!
//! - [`TracingExporter`] turns each [`crate::collector::ObservationBatch`]
//!   into `tracing` events under a fixed, versioned schema
//!   (`metric_observation` per eligible measurement +
//!   `observation_window` per processed poll). Eligibility and severity
//!   come from the domain-declared [`crate::source::ReportingHint`] /
//!   [`crate::source::ReportingImpact`] — `SnapshotOnly` never becomes
//!   an event, and counter deltas never infer severity.
//! - [`TracingFindingSink`] adapts [`crate::diagnostics::DiagnosticsEvent`]s
//!   into `tracing` events for `Diagnostics::set_sink`.
//! - [`TracingRecordExporter`] turns drained
//!   [`crate::record::EventRecord`]s into `rt_record` events, resolving
//!   each record's [`crate::record::CorrelationLink`] through a
//!   [`crate::record::CorrelationBook`] and optionally attaching W3C
//!   trace fields from a [`TraceContext`]. Slice 1 event schemas
//!   (`metric_observation`/`observation_window`/
//!   `diagnostics.health_transition`) are unchanged — trace fields are
//!   `rt_record`-only in this revision.
//!
//! # Ownership and gating
//!
//! - Available only with the `tracing-export` cargo feature, which
//!   enables `dep:tracing` alone — no subscriber construction, no
//!   `tracing-subscriber`/`tracing-appender` requirement, no separate
//!   crate.
//! - The host injects a [`tracing::Dispatch`] (typically built by the
//!   `logging` feature) into both types. Nothing here installs a
//!   global default or writes to stdout; `Dispatch::none()` is safe.
//! - The [`crate::collector::Collector`], metrics snapshot, and
//!   diagnostics engine never see `tracing` types — emission is a
//!   pull-side read of `ObservationBatch` plus scoped
//!   `dispatcher::with_default` calls.
//!
//! # RT contract
//!
//! Every public operation here is **non-RT only**: `emit` and
//! `emit_records` allocate (`HashMap`, `String`, slice iteration) and
//! run subscriber code under a scoped dispatch. RT writer paths must
//! never call into this module or into `tracing` at all — including
//! inside a `RecordSink::drain` callback.
//!
//! # Failure isolation
//!
//! The exporter cannot affect Collector/Metrics/Diagnostics/Source/RT
//! state — it only reads the batch it is handed. Subscriber-side
//! failures (filters, writers) are the host pipeline's concern; the
//! [`TracingFindingSink`] additionally isolates panics via
//! `catch_unwind` and counts them without recursive logging.

mod exporter;
mod records;
mod schema;
mod sink;
mod trace_context;

#[cfg(test)]
mod tests;

pub use exporter::{DEFAULT_MAX_EMISSIONS_PER_POLL, EmitStats, TracingExporter};
pub use records::{DEFAULT_MAX_EVENTS_PER_DRAIN, RecordEmitStats, TracingRecordExporter};
pub use sink::TracingFindingSink;
pub use trace_context::TraceContext;
