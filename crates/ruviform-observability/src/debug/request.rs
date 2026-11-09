//! Inspection request vocabulary — what to inspect, how much detail
//! to carry, and the input bundle an
//! [`inspect`](crate::debug::DebugInspector::inspect) call consumes
//! (Issue #292, OBS-10).
//!
//! The request is deliberately a *closed vocabulary*: targets name
//! the five inspectable section kinds, detail names three
//! granularity levels, and inputs are typed references to existing
//! observability artifacts. There is no path for arbitrary internal
//! state, filesystem access, or untyped blobs — a debug inspection
//! can only look at what the observability stack already produced.

use std::error::Error;
use std::fmt;

use crate::collector::{MetricsSnapshot, ObservationBatch};
use crate::diagnostics::DiagnosticReport;
use crate::diagnostics::report::exceeds_disclosure;
use crate::source::Confidentiality;

use super::budget::InspectionBudget;

/// One inspectable section kind — the closed vocabulary bounding
/// what `inspect` can produce.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InspectionTarget {
    /// Collector [`MetricsSnapshot`] measurements — per-series
    /// entries with validity/lifecycle preserved.
    Metrics,
    /// OBS-09 [`DiagnosticReport`] findings and domain health.
    Diagnostics,
    /// Registered [`InvariantCheck`](crate::debug::InvariantCheck)s
    /// evaluated against the assembled context.
    Invariants,
    /// Debug session state and drain accounting.
    Session,
    /// [`ProbeSource`](crate::debug::ProbeSource) deck readings with
    /// generation/clock alignment.
    Probes,
}

/// How much detail section entries carry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InspectionDetail {
    /// Aggregate counts only — per-item `entries` stay empty;
    /// validity/findings/deck histograms and totals are still
    /// reported. This is the *requested* detail level, distinct from
    /// a budget-imposed truncation.
    Summary,
    /// Per-item entries carrying identity, classification, validity
    /// and status — **no payload values**.
    Standard,
    /// Entries additionally carry typed payloads
    /// ([`MetricValue`](crate::metrics::MetricValue) for metrics,
    /// cumulative counters for probes). Still a closed vocabulary —
    /// no raw bytes, paths, or media.
    Verbose,
}

/// One inspection request — targets, detail, disclosure ceiling, and
/// budget. Carried by value into [`inspect`](crate::debug::DebugInspector::inspect);
/// the snapshot echoes it back via [`RequestEcho`].
#[derive(Debug, Clone)]
pub struct InspectionRequest {
    /// Sections to produce. Duplicates are removed and sections are
    /// evaluated in canonical (enum declaration) order regardless of
    /// request order — output is deterministic. Empty produces a
    /// snapshot with no sections.
    pub targets: Vec<InspectionTarget>,
    /// Detail granularity for per-item entries.
    pub detail: InspectionDetail,
    /// Maximum [`Confidentiality`] the caller may see — items above
    /// this are withheld wholesale (the same rule diagnostics
    /// reports apply). Requests above the engine's
    /// `max_inspection_disclosure` are rejected as
    /// [`InspectionError::Unauthorized`].
    pub disclosure: Confidentiality,
    /// Bounds applied to this inspect.
    pub budget: InspectionBudget,
}

/// Borrowed observability artifacts an inspect may read — every
/// field is optional because a caller may legitimately inspect a
/// subset (e.g. session state only, or metrics without a diagnostics
/// report). A requested target whose input is `None` reports
/// [`SectionStatus::Unavailable`](crate::debug::SectionStatus)
/// rather than silently producing an empty section.
#[derive(Debug, Default)]
pub struct InspectionInputs<'a> {
    /// Latest collector metrics snapshot, if available.
    pub metrics: Option<&'a MetricsSnapshot>,
    /// Latest observation batch, if available.
    pub batch: Option<&'a ObservationBatch>,
    /// Latest diagnostics report, if available.
    pub diagnostics: Option<&'a DiagnosticReport>,
}

/// Deterministic, deduplicated echo of the request a snapshot
/// answered — targets in canonical order.
#[derive(Debug, Clone)]
pub struct RequestEcho {
    /// Requested targets, deduplicated, canonical order.
    pub targets: Vec<InspectionTarget>,
    /// Requested detail level.
    pub detail: InspectionDetail,
    /// Applied disclosure ceiling.
    pub disclosure: Confidentiality,
    /// Applied budget.
    pub budget: InspectionBudget,
}

/// Errors an [`inspect`](crate::debug::DebugInspector::inspect) call
/// can return — the snapshot itself reports per-section failures as
/// typed statuses; this error is reserved for request-level
/// rejection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionError {
    /// `request.disclosure` exceeds the engine's configured
    /// `max_inspection_disclosure` — refused entirely rather than
    /// silently downgraded.
    Unauthorized {
        /// What was requested.
        requested: Confidentiality,
        /// What the engine permits.
        permitted: Confidentiality,
    },
}

impl fmt::Display for InspectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unauthorized {
                requested,
                permitted,
            } => write!(
                f,
                "inspection disclosure {requested:?} exceeds permitted {permitted:?}"
            ),
        }
    }
}

impl Error for InspectionError {}

/// Whether `item`'s confidentiality exceeds `requested` — delegates
/// to the diagnostics module's single rank implementation so the
/// disclosure ordering cannot drift between modules.
pub(crate) fn withheld(item: Confidentiality, requested: Confidentiality) -> bool {
    exceeds_disclosure(item, requested)
}

/// The canonical target order — deduplicated, declaration order.
pub(crate) fn canonical_targets(requested: &[InspectionTarget]) -> Vec<InspectionTarget> {
    const ORDER: [InspectionTarget; 5] = [
        InspectionTarget::Metrics,
        InspectionTarget::Diagnostics,
        InspectionTarget::Invariants,
        InspectionTarget::Session,
        InspectionTarget::Probes,
    ];
    ORDER
        .into_iter()
        .filter(|t| requested.contains(t))
        .collect()
}
