//! Debug report vocabulary — the explicitly generated, bounded,
//! disclosure-authorized export artifact (Issue #292, OBS-10).
//!
//! A [`DebugReport`] summarizes an [`InspectionSnapshot`] (optional)
//! plus session/drain accounting into a single exportable document.
//! It enforces two authorization boundaries before producing
//! anything:
//!
//! - `request.disclosure` must not exceed
//!   [`DebugConfig::max_export_disclosure`](crate::debug::DebugConfig)
//!   — otherwise [`ExportError::Unauthorized`].
//! - `request.external` requires
//!   [`DebugConfig::allow_external_export`](crate::debug::DebugConfig)
//!   — otherwise [`ExportError::ExternalNotPermitted`].
//!
//! Neither failure silently downgrades: the report is refused
//! entirely. When authorized, items above the requested disclosure
//! are excluded wholesale (never partially redacted) and counted in
//! [`ExportTruncation::withheld_confidential`]; `max_items`/
//! `max_bytes`/`deadline` bounds produce structured truncation.
//!
//! # Non-leakage
//!
//! Every field is a closed vocabulary — `EventRecord` *metadata*
//! (event id, version, source identity, generation, clock presence,
//! confidentiality, positional loss), typed inspection echoes, and
//! `&'static str` notes. Raw `u64` payload values, secrets, media
//! bytes, paths, credentials, prompts, and stderr have no carrier
//! here — the shape itself is the guarantee.

use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use crate::diagnostics::report::exceeds_disclosure;
use crate::record::EventId;
use crate::source::{Confidentiality, SourceIdentity};

use super::request::InspectionDetail;
use super::session::SessionView;
use super::snapshot::{InspectionSnapshot, SectionStatus};

/// Schema version of [`DebugReport`] — bumped on any breaking
/// change to the report shape.
pub const DEBUG_REPORT_SCHEMA_VERSION: u16 = 1;

/// Output bounds for report generation — the same `0`/`None`
/// convention as [`InspectionBudget`](crate::debug::InspectionBudget)
/// and diagnostics [`ReportLimits`](crate::diagnostics::ReportLimits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportLimits {
    /// Maximum drained-record summaries carried; `0` = summaries
    /// only (no record entries).
    pub max_items: usize,
    /// Approximate byte budget; `0` = unlimited.
    pub max_bytes: usize,
    /// Generation deadline checked per item; `None` = unlimited.
    pub deadline: Option<Duration>,
}

impl Default for ExportLimits {
    fn default() -> Self {
        Self {
            max_items: 64,
            max_bytes: 65_536,
            deadline: None,
        }
    }
}

/// One report request — disclosure ceiling, external-export flag,
/// and output bounds.
#[derive(Debug, Clone)]
pub struct DebugReportRequest {
    /// Maximum [`Confidentiality`] the caller may receive — record
    /// summaries above this are excluded wholesale.
    pub disclosure: Confidentiality,
    /// `true` = external export (file/agent hand-off); requires
    /// [`DebugConfig::allow_external_export`](crate::debug::DebugConfig).
    pub external: bool,
    /// Output bounds.
    pub limits: ExportLimits,
}

/// Errors generating a [`DebugReport`] — request-level rejection;
/// per-item problems are structured truncation instead.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportError {
    /// `request.disclosure` exceeds `max_export_disclosure` —
    /// refused entirely rather than silently downgraded.
    Unauthorized {
        /// What was requested.
        requested: Confidentiality,
        /// What the engine permits.
        permitted: Confidentiality,
    },
    /// `request.external` was set but
    /// `allow_external_export` is `false`.
    ExternalNotPermitted,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unauthorized {
                requested,
                permitted,
            } => write!(
                f,
                "report disclosure {requested:?} exceeds permitted {permitted:?}"
            ),
            Self::ExternalNotPermitted => {
                write!(f, "external export is not permitted")
            }
        }
    }
}

impl Error for ExportError {}

/// Structured record of everything the report bounds removed or
/// withheld.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportTruncation {
    /// Record summaries not carried (item cap, byte budget, or
    /// deadline).
    pub items_dropped: usize,
    /// Record summaries excluded because their confidentiality
    /// exceeded the requested disclosure.
    pub withheld_confidential: usize,
    /// Approximate bytes accounted for included items.
    pub bytes_estimated: usize,
    /// `true` when the deadline stopped generation early.
    pub deadline_exceeded: bool,
}

/// Metadata-only summary of one drained [`SessionRecord`](crate::debug::SessionRecord)
/// — the payload `value` is deliberately absent (payload semantics
/// are domain-owned; the report carries identity/loss/classification
/// only).
#[derive(Debug, Clone, Copy)]
pub struct RecordSummary {
    /// Event kind.
    pub event: EventId,
    /// Payload-semantics version.
    pub version: u16,
    /// Producing registration identity.
    pub source: SourceIdentity,
    /// Domain-declared generation (`0` = none asserted).
    pub generation: u64,
    /// `true` when the record carried a clock stamp.
    pub clock_stamped: bool,
    /// Clock domain id when stamped (`0` = `ClockDomainId::UNKNOWN`).
    pub clock_domain: u64,
    /// Clock epoch when stamped.
    pub clock_epoch: u64,
    /// Record confidentiality classification.
    pub confidentiality: Confidentiality,
    /// Records dropped immediately before this one.
    pub dropped_before: u64,
}

/// Bounded echo of the inspection a report summarizes — statuses and
/// counts only, never item payloads (those were already
/// disclosure-filtered at inspect time; the echo cannot leak what it
/// does not carry).
#[derive(Debug, Clone)]
pub struct InspectionEcho {
    /// Snapshot schema version.
    pub schema_version: u16,
    /// When the inspection ran.
    pub taken_at: Instant,
    /// Requested targets (canonical order).
    pub targets: Vec<crate::debug::InspectionTarget>,
    /// Requested detail level.
    pub detail: InspectionDetail,
    /// Applied disclosure ceiling.
    pub disclosure: Confidentiality,
    /// Per-section (target, status) pairs in canonical order.
    pub sections: Vec<(crate::debug::InspectionTarget, SectionStatus)>,
    /// Worst consistency level observed.
    pub consistency: crate::debug::ConsistencyLevel,
    /// Inspection truncation accounting.
    pub truncation_items_dropped: usize,
    /// Inspection truncation accounting.
    pub truncation_withheld: usize,
}

/// Session state carried into every report — the session view is
/// small and always available.
#[derive(Debug, Clone)]
pub struct SessionReportView {
    /// Current lifecycle state.
    pub state: crate::debug::DebugSessionState,
    /// Session generation (`0` while `Disabled`).
    pub generation: u64,
    /// Domain label, if prepared.
    pub label: Option<&'static str>,
    /// Attached drains.
    pub drains_attached: usize,
    /// Retained records.
    pub records_retained: usize,
    /// Overflowed records.
    pub records_overflowed: u64,
    /// Endpoints awaiting quiescence.
    pub pending_quiescence: usize,
    /// Records in flight.
    pub pending_records: u64,
    /// Drain deadline elapsed.
    pub deadline_exceeded: bool,
    /// Reached `Stopped` via abandon.
    pub abandoned: bool,
}

impl From<&SessionView> for SessionReportView {
    fn from(view: &SessionView) -> Self {
        Self {
            state: view.state,
            generation: view.generation,
            label: view.label,
            drains_attached: view.drains_attached,
            records_retained: view.records_retained,
            records_overflowed: view.records_overflowed,
            pending_quiescence: view.pending_quiescence,
            pending_records: view.pending_records,
            deadline_exceeded: view.deadline_exceeded,
            abandoned: view.abandoned,
        }
    }
}

/// Static limitation notes attached to every report.
pub(crate) const REPORT_NOTES: &[&str] = &[
    "Report contents are independent reads, not a transactional single-instant view.",
    "Drained record summaries carry metadata only — payload values are domain-owned and never exported here.",
    "This report is a lossy diagnostic artifact; domain Error types and the authoritative diagnostics channel remain the failure source of truth.",
    "Secrets, media bytes, paths, credentials, prompts and raw stderr have no typed carrier in this report.",
];

/// An explicitly generated debug report.
#[derive(Debug, Clone)]
pub struct DebugReport {
    /// Report schema version — [`DEBUG_REPORT_SCHEMA_VERSION`].
    pub schema_version: u16,
    /// Caller-supplied generation instant.
    pub generated_at: Instant,
    /// The disclosure class actually applied (the request's, after
    /// authorization).
    pub disclosure: Confidentiality,
    /// Whether this report was generated for external export.
    pub external: bool,
    /// Bounded echo of the summarized inspection, when provided.
    pub inspection: Option<InspectionEcho>,
    /// Session state at generation time.
    pub session: SessionReportView,
    /// Drained-record summaries, newest first, bounded by
    /// `ExportLimits::max_items`.
    pub records: Vec<RecordSummary>,
    /// What was dropped or withheld and why.
    pub truncation: ExportTruncation,
    /// Static limitation notes.
    pub notes: Vec<&'static str>,
}

/// Builds the [`InspectionEcho`] for `snapshot` — statuses and
/// counts only.
pub(crate) fn inspection_echo(snapshot: &InspectionSnapshot) -> InspectionEcho {
    InspectionEcho {
        schema_version: snapshot.schema_version,
        taken_at: snapshot.taken_at,
        targets: snapshot.request.targets.clone(),
        detail: snapshot.request.detail,
        disclosure: snapshot.request.disclosure,
        sections: snapshot
            .sections
            .iter()
            .map(|s| (s.target(), s.status()))
            .collect(),
        consistency: snapshot.consistency.level,
        truncation_items_dropped: snapshot.truncation.items_dropped,
        truncation_withheld: snapshot.truncation.withheld_confidential,
    }
}

/// Whether `confidentiality` exceeds `disclosure` — delegates to the
/// diagnostics rank so ordering cannot drift.
pub(crate) fn exceeds(item: Confidentiality, disclosure: Confidentiality) -> bool {
    exceeds_disclosure(item, disclosure)
}
