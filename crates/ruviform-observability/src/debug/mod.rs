//! Debug inspection — the bounded, non-RT, explicitly authorized
//! debug layer of the observability stack (Issue #292, OBS-10).
//!
//! Debug owns the *additional-cost* part of debug instrumentation:
//! inspection budgets, invariant verdicts, debug sessions with
//! bounded drain, and disclosure-authorized reports. It consumes —
//! never duplicates — the shared foundations:
//!
//! - **Budgets** ([`InspectionBudget`], [`ExportLimits`]): every
//!   inspect/report runs under item/byte/deadline bounds; every
//!   bound that removes data is counted in
//!   [`InspectionTruncation`]/[`ExportTruncation`] — truncation is
//!   observable, never silent.
//! - **Inspection** ([`DebugInspector::inspect`] →
//!   [`InspectionSnapshot`]): five typed sections
//!   ([`InspectionTarget`]) each carrying a [`SectionStatus`] —
//!   `Ok`/`Truncated`/`Unavailable{reason}`/`Withheld` — plus
//!   [`InspectionConsistency`] generation/clock alignment. A
//!   snapshot is *not* a transactional single-instant view; the
//!   consistency block declares how far apart the reads may be.
//! - **Invariants** ([`InvariantCheck`] port →
//!   [`InvariantOutcome`]): three-valued `Satisfied`/`Violated`/
//!   `Inconclusive` verdicts with typed [`InconclusiveReason`]s —
//!   uncertainty is never collapsed into a definite answer.
//!   [`DebugInspector::publish_invariants`] maps results onto OBS-09
//!   [`Diagnostics`](crate::diagnostics::Diagnostics) evidence and
//!   findings.
//! - **Probe port** ([`ProbeSource`]): a read-only vocabulary +
//!   `Box<dyn>` port over #185 Generic Probe state — this crate
//!   cannot depend on `ruviform-audio` (dependency cycle), so the
//!   concrete adapter is Issue #293's responsibility. This module
//!   re-declares the probe vocabulary ([`ProbeMode`],
//!   [`ProbeAvailability`], [`ProbeClockMark`], …) and never
//!   re-implements OFF/Monitor/Capture behavior.
//! - **Sessions** ([`DebugAuthority`]/[`SessionPermit`]/
//!   [`SessionDrain`]): an explicit
//!   `Disabled → Prepared → Active → Draining → Stopped` lifecycle
//!   with capability-gated control, bounded record retention, and
//!   final RT-buffer recovery only after confirmed writer
//!   quiescence — or an explicit, counted `abandon`.
//! - **Reports** ([`DebugInspector::report`] → [`DebugReport`]):
//!   explicitly generated, disclosure-authorized exports whose
//!   closed vocabulary cannot carry secrets, media bytes, paths,
//!   credentials, prompts, or raw stderr.
//!
//! # Contracts
//!
//! - **Non-RT only**: every public operation allocates and/or locks.
//!   Never call from an RT writer path; RT-adjacent value types are
//!   `Copy` and alloc-free by construction.
//! - **Logging OFF**: the default build has no `tracing` path here.
//! - **Debug OFF is free**: with no session prepared and no checks
//!   registered, inspection reads existing observability artifacts
//!   only — production invariants elsewhere are untouched.
//! - **Honest uncertainty**: missing inputs, generation skew, clock
//!   `UNKNOWN`, port failures, and check panics are typed statuses —
//!   never fabricated certainty.

mod budget;
mod engine;
mod invariant;
mod probe;
mod report;
mod request;
mod session;
mod snapshot;

#[cfg(test)]
mod tests;

pub use budget::InspectionBudget;
pub use engine::{DebugConfig, DebugHealth, DebugInspector};
pub use invariant::{
    InconclusiveReason, InspectionContext, InvariantCheck, InvariantOutcome,
    InvariantPublishReport, InvariantResult, InvariantVerdict,
};
pub use probe::{
    ProbeAvailability, ProbeClockMark, ProbeDeckSnapshot, ProbeMode, ProbeReading, ProbeSource,
    ProbeStatusKind, ProbeStopReason, ProbeUnobservedReason,
};
pub use report::{
    DEBUG_REPORT_SCHEMA_VERSION, DebugReport, DebugReportRequest, ExportError, ExportLimits,
    ExportTruncation, InspectionEcho, RecordSummary, SessionReportView,
};
pub use request::{
    InspectionDetail, InspectionError, InspectionInputs, InspectionRequest, InspectionTarget,
    RequestEcho,
};
pub use session::{
    DebugAuthority, DebugSessionState, DrainReport, EventRecordDrain, SessionDrain,
    SessionDrainStatus, SessionError, SessionPermit, SessionRecord, SessionRequest, SessionView,
};
pub use snapshot::{
    CollectorGenerations, ConsistencyLevel, DiagnosticsInspection, DrainAccounting, FindingEntry,
    INSPECTION_SCHEMA_VERSION, InspectionConsistency, InspectionSection, InspectionSnapshot,
    InspectionTruncation, InvariantSummary, InvariantsInspection, MetricEntry, MetricsInspection,
    ProbeCounters, ProbeDeckAlignment, ProbeDeckView, ProbeEntry, ProbesInspection, SectionStatus,
    SessionInspection, UnavailableReason, ValidityCounts,
};
