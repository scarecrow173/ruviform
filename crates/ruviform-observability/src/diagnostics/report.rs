//! Snapshot and report vocabulary — bounded, non-transactional views
//! and explicitly generated diagnostic reports.
//!
//! [`DiagnosticSnapshot`] is the pull-side view returned by
//! [`Diagnostics::evaluate`](crate::diagnostics::Diagnostics::evaluate):
//! current domain health, findings, and evidence entries as of the
//! latest evaluation. Its [`SnapshotConsistency::IndependentReads`]
//! marker is a type-level promise that the contents come from
//! independent per-series reads — **not** a transactional
//! single-instant snapshot.
//!
//! [`DiagnosticReport`] is the export-side artifact produced by
//! [`Diagnostics::report`](crate::diagnostics::Diagnostics::report) —
//! an explicit, non-RT generation carrying schema version, applied
//! disclosure class, findings/health/evidence, structured
//! [`ReportTruncation`], and static limitation notes. Reports enforce
//! `max_findings` / `max_bytes` / `deadline` bounds and reject requests
//! whose [`Confidentiality`] exceeds the engine's configured export
//! ceiling with [`ReportError::Unauthorized`]. Items above the
//! requested disclosure are excluded wholesale — never partially
//! redacted — so secrets, media bytes, prompts, credentials, and raw
//! stderr cannot leak through a report.
//!
//! Host authorization itself (capability issuance, audit) is OBS-11
//! (#293); this module models it as the `max_export_disclosure`
//! policy parameter only.

use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use crate::source::Confidentiality;

use super::evidence::EvidenceEntry;
use super::finding::{DiagnosticFinding, confidentiality_rank};
use super::health::DomainHealth;
use super::sink::SinkPolicy;

/// Schema version of [`DiagnosticReport`] — bumped on any breaking
/// change to the report shape.
pub const REPORT_SCHEMA_VERSION: u16 = 1;

/// How the contents of a [`DiagnosticSnapshot`] were read.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotConsistency {
    /// Each series/atomic was read independently; values may come
    /// from different instants. This is **not** a transactional
    /// single-time snapshot.
    IndependentReads,
}

/// Bounded view of diagnostics state as of the latest evaluation.
#[derive(Debug, Clone)]
pub struct DiagnosticSnapshot {
    /// The collector snapshot generation this view was evaluated
    /// against.
    pub generation: u64,
    /// When the underlying `MetricsSnapshot` was committed.
    pub taken_at: Instant,
    /// Read-consistency declaration — always
    /// [`SnapshotConsistency::IndependentReads`].
    pub consistency: SnapshotConsistency,
    /// Per-domain health, ordered by domain label.
    pub domains: Vec<DomainHealth>,
    /// Retained findings (open and resolved), bounded by
    /// `DiagnosticsConfig::max_findings`.
    pub findings: Vec<DiagnosticFinding>,
    /// Retained evidence entries with superseded marking, bounded by
    /// `DiagnosticsConfig::max_evidence`.
    pub evidence: Vec<EvidenceEntry>,
    /// Number of declared rules whose latest input was `Missing`.
    pub missing_rules: usize,
}

/// Output bounds for [`DiagnosticReport`] generation.
///
/// `0` means *unlimited* for `max_findings`/`max_bytes` — the engine's
/// own retention caps (`DiagnosticsConfig::max_findings`,
/// `max_evidence`) still bound what can appear. `deadline: None`
/// disables the time bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportLimits {
    /// Maximum findings included (severity-descending, stable order).
    pub max_findings: usize,
    /// Approximate byte budget across findings/evidence/health —
    /// estimation is `size_of` plus payload lengths, documented as
    /// approximate, not exact.
    pub max_bytes: usize,
    /// Maximum wall time the generator may spend; checked before
    /// each item is appended. `None` disables.
    pub deadline: Option<Duration>,
}

impl Default for ReportLimits {
    fn default() -> Self {
        Self {
            max_findings: 64,
            max_bytes: 65_536,
            deadline: None,
        }
    }
}

/// Engine configuration — retention bounds, report limits, export
/// disclosure ceiling, and sink policy.
#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Maximum retained evidence entries; `0` disables evidence
    /// recording entirely.
    pub max_evidence: usize,
    /// Maximum retained findings; `0` disables finding recording.
    pub max_findings: usize,
    /// Report output bounds.
    pub report: ReportLimits,
    /// Highest [`Confidentiality`] a [`ReportRequest`] may ask for.
    /// Requests above this are rejected as
    /// [`ReportError::Unauthorized`]; items above the *requested*
    /// level are excluded from the report body.
    pub max_export_disclosure: Confidentiality,
    /// Suppression policy for [`FindingSink`](super::sink::FindingSink)
    /// notifications.
    pub sink_policy: SinkPolicy,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            max_evidence: 1024,
            max_findings: 256,
            report: ReportLimits::default(),
            max_export_disclosure: Confidentiality::Internal,
            sink_policy: SinkPolicy::default(),
        }
    }
}

impl DiagnosticsConfig {
    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// [`DiagnosticsConfigError::ZeroSinkWindow`] when
    /// `sink_policy.window` is zero — a suppression window of zero
    /// would either suppress everything or nothing depending on
    /// interpretation, so it is rejected rather than guessed.
    pub fn validate(&self) -> Result<(), DiagnosticsConfigError> {
        if self.sink_policy.window.is_zero() {
            return Err(DiagnosticsConfigError::ZeroSinkWindow);
        }
        Ok(())
    }
}

/// What disclosure class a report request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportRequest {
    /// The maximum [`Confidentiality`] the caller is authorized to
    /// receive. Items classified above this are excluded.
    pub disclosure: Confidentiality,
}

/// Errors generating a [`DiagnosticReport`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportError {
    /// The requested disclosure exceeds the engine's configured
    /// `max_export_disclosure` — the report is refused entirely
    /// rather than silently downgraded.
    Unauthorized {
        /// What was requested.
        requested: Confidentiality,
        /// What the engine permits.
        permitted: Confidentiality,
    },
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unauthorized {
                requested,
                permitted,
            } => write!(
                f,
                "report disclosure {requested:?} exceeds permitted {permitted:?}"
            ),
        }
    }
}

impl Error for ReportError {}

/// Structured record of everything the report bounds removed or
/// withheld — truncation is always observable, never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportTruncation {
    /// Findings dropped by `max_findings`.
    pub findings_dropped: usize,
    /// Evidence entries dropped by `max_bytes`/`deadline`.
    pub evidence_dropped: usize,
    /// Domain-health entries dropped by `max_bytes`/`deadline`.
    pub health_dropped: usize,
    /// Items (findings + evidence + health) excluded because their
    /// confidentiality exceeded the requested disclosure.
    pub withheld_confidential: usize,
    /// Evidence references cited by included findings that point to
    /// evidence not included in this report.
    pub unresolved_evidence_refs: usize,
    /// Approximate bytes accounted for included items.
    pub bytes_estimated: usize,
    /// `true` when the deadline stopped generation early.
    pub deadline_exceeded: bool,
}

/// An explicitly generated diagnostic report.
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// Report schema version — [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u16,
    /// Caller-supplied generation instant.
    pub generated_at: Instant,
    /// The collector snapshot generation the diagnostics state was
    /// last evaluated against (`0` if never evaluated).
    pub source_generation: u64,
    /// The disclosure class actually applied (the request's, after
    /// authorization).
    pub disclosure: Confidentiality,
    /// Included findings — severity-descending, stable within equal
    /// severity, bounded by `max_findings`.
    pub findings: Vec<DiagnosticFinding>,
    /// Included evidence entries.
    pub evidence: Vec<EvidenceEntry>,
    /// Included domain health.
    pub health: Vec<DomainHealth>,
    /// What was dropped or withheld and why.
    pub truncation: ReportTruncation,
    /// Static limitation notes — e.g. read consistency, superseded
    /// evidence semantics, and the boundary against domain error
    /// types. `&'static str` keeps notes allocation-free.
    pub notes: Vec<&'static str>,
}

/// Errors validating [`DiagnosticsConfig`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsConfigError {
    /// `sink_policy.window` was zero.
    ZeroSinkWindow,
}

impl fmt::Display for DiagnosticsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSinkWindow => write!(f, "sink policy window must be non-zero"),
        }
    }
}

impl Error for DiagnosticsConfigError {}

/// Static limitation notes attached to every report.
pub(crate) const REPORT_NOTES: &[&str] = &[
    "Snapshot contents are independent per-series reads, not a transactional single-instant view.",
    "Evidence from retired or re-registered sources is marked superseded and never rebound.",
    "Findings summarize declared health rules; domain Error/Result types and existing diagnostic registries remain the authoritative failure channel.",
];

/// Estimated serialized size of one finding — `size_of` plus the
/// variable-length parts. Approximate by design.
pub(crate) fn estimate_finding_bytes(f: &DiagnosticFinding) -> usize {
    size_of::<DiagnosticFinding>() + f.summary().len() + size_of_val(f.evidence_refs())
}

/// Estimated serialized size of one evidence entry.
pub(crate) fn estimate_evidence_bytes(e: &EvidenceEntry) -> usize {
    size_of::<EvidenceEntry>()
        + size_of::<super::evidence::Evidence>()
        + match e.evidence.payload() {
            super::evidence::EvidencePayload::Metric(v) => metric_value_bytes(v),
            _ => 0,
        }
}

fn metric_value_bytes(v: &crate::metrics::MetricValue) -> usize {
    match v {
        crate::metrics::MetricValue::Counter { .. } | crate::metrics::MetricValue::Gauge { .. } => {
            0
        }
        crate::metrics::MetricValue::Histogram { buckets, .. } => buckets.len() * size_of::<u64>(),
    }
}

/// Estimated serialized size of one domain-health entry.
pub(crate) fn estimate_health_bytes(h: &DomainHealth) -> usize {
    size_of::<DomainHealth>() + h.rules.len() * size_of::<super::health::RuleHealth>()
}

/// Whether `item`'s confidentiality exceeds the requested disclosure.
pub(crate) fn exceeds_disclosure(item: Confidentiality, requested: Confidentiality) -> bool {
    confidentiality_rank(item) > confidentiality_rank(requested)
}
