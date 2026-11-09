//! Diagnostics — the logging-independent layer that separates
//! verifiable facts from derived, reported, and inferred claims
//! (Issue #291, OBS-09).
//!
//! Built on top of the OBS-07 collector's typed [`Measurement`]s and
//! [`MetricsSnapshot`]s and the OBS-02 [`SourceIdentity`] contract,
//! this module provides:
//!
//! - [`evidence`]: classified [`Evidence`] (`Observed` / `Derived` /
//!   `Reported` / `Inferred`) with source id + generation, clock
//!   domain, validity, confidentiality, and a mandatory basis for
//!   derived/inferred claims.
//! - [`finding`]: [`DiagnosticFinding`]s with stable domain-owned
//!   [`DiagnosticCode`]s, severity, certainty
//!   (`Confirmed`/`Inferred`/`NotEvaluable`), and evidence references.
//! - [`health`]: [`HealthState`] (`Healthy`/`Degraded`/`Unhealthy`/
//!   `Unknown` — `Unknown` is neither normal nor abnormal) and
//!   [`HealthEvaluator`] applying domain-declared thresholds,
//!   hysteresis, recovery, and stale/missing mappings.
//! - [`report`]: bounded [`DiagnosticSnapshot`] views and explicitly
//!   generated [`DiagnosticReport`]s with findings/bytes/deadline
//!   limits, structured truncation, and disclosure-class filtering.
//! - [`sink`]: the [`FindingSink`] port for important state changes —
//!   bounded by [`SinkPolicy`], independent of any subscriber.
//! - [`engine`]: the [`Diagnostics`] aggregator tying it together.
//!
//! # Contracts
//!
//! - **Logging OFF**: the default build has no `tracing` dependency
//!   path here — every API works with the `logging` feature disabled.
//! - **Non-RT only**: every public operation allocates or locks.
//!   Never call from an RT writer path.
//! - **No domain-error duplication**: [`DiagnosticCode`] is a domain
//!   string + schema version, not a central enum; domain `Error` types
//!   and registries such as the plugin host's `DiagRegistry` remain
//!   authoritative.
//! - **Bounded disclosure**: reports reject unauthorized disclosure
//!   classes and exclude over-classified items wholesale — secrets,
//!   media bytes, prompts, credentials, and raw stderr have no typed
//!   carrier here.

pub mod engine;
pub mod evidence;
pub mod finding;
pub mod health;
pub mod report;
pub mod sink;

#[cfg(test)]
mod tests;

pub use engine::{Diagnostics, DiagnosticsHealth};
pub use evidence::{
    Evidence, EvidenceEntry, EvidenceError, EvidenceKind, EvidenceMeta, EvidencePayload,
    EvidenceRef, EvidenceTime,
};
pub use finding::{
    DiagnosticCode, DiagnosticFinding, FindingCertainty, FindingError, FindingSeverity,
    FindingStatus,
};
pub use health::{
    DomainEvaluation, DomainHealth, HealthEvaluator, HealthRule, HealthState, Hysteresis,
    RuleError, RuleHealth, RuleInput, RuleTransition, SignalDirection, SignalKind,
};
pub use report::{
    DiagnosticReport, DiagnosticSnapshot, DiagnosticsConfig, DiagnosticsConfigError,
    REPORT_SCHEMA_VERSION, ReportError, ReportLimits, ReportRequest, ReportTruncation,
    SnapshotConsistency,
};
pub use sink::{DiagnosticsEvent, FindingSink, SinkPolicy};
