//! Inspection snapshot vocabulary — the bounded, typed, explicitly
//! non-transactional result of one
//! [`inspect`](crate::debug::DebugInspector::inspect) call (Issue
//! #292, OBS-10).
//!
//! An [`InspectionSnapshot`] carries:
//!
//! - [`InspectionSection`]s — one per requested
//!   [`InspectionTarget`](crate::debug::InspectionTarget), each with
//!   a typed [`SectionStatus`]. A section that could not be produced
//!   is *reported*, never silently dropped: missing inputs, deadline
//!   expiry, port failures and disclosure withholding are all typed.
//! - [`InspectionConsistency`] — generation/clock alignment across
//!   the collector domain (snapshot/batch/diagnostics generations)
//!   and each probe deck, reported worst-of with
//!   [`ConsistencyLevel::Unknown`] for anything that cannot be
//!   established. Different generation *domains* are never compared
//!   numerically — a collector poll generation and a probe plan
//!   generation are unrelated counters.
//! - [`InspectionTruncation`] — structured accounting of everything
//!   the budget removed or withheld.
//!
//! The snapshot is **not** a transactional single-instant view:
//! sections are produced by independent reads and the consistency
//! block says exactly how far apart those reads may be. No section
//! payload can carry secrets, media bytes, paths, credentials,
//! prompts, or raw stderr — every field is a closed vocabulary of
//! observability types.

use std::time::Instant;

use crate::diagnostics::{DiagnosticCode, DomainHealth};
use crate::diagnostics::{EvidenceRef, FindingCertainty, FindingSeverity, FindingStatus};
use crate::metrics::{MeasurementIssue, MeasurementValidity, MetricValue};
use crate::source::{
    Confidentiality, MetricKind, ObservationScope, SourceIdentity, SourceLifecycle,
};

use super::invariant::InvariantResult;
use super::probe::{ProbeClockMark, ProbeMode, ProbeStatusKind};
use super::request::{InspectionTarget, RequestEcho};
use super::session::DebugSessionState;

/// Schema version of [`InspectionSnapshot`] — bumped on any breaking
/// change to the snapshot shape.
pub const INSPECTION_SCHEMA_VERSION: u16 = 1;

/// Per-section delivery status — truncation, unavailability and
/// withholding are typed, never silent.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionStatus {
    /// Section produced normally (possibly with truncated entries).
    Ok,
    /// Budget cut item collection short — see
    /// [`InspectionTruncation`].
    Truncated,
    /// The section could not be produced — input absent, deadline
    /// exceeded, or a registered port failed.
    Unavailable {
        /// Why the section is unavailable.
        reason: UnavailableReason,
    },
    /// Every item exceeded the request's disclosure ceiling — the
    /// section exists but carries no visible items.
    Withheld,
}

/// Why a [`SectionStatus::Unavailable`] section could not be
/// produced.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnavailableReason {
    /// The caller did not supply the required input (e.g.
    /// `InspectionInputs::metrics` was `None`).
    InputNotProvided,
    /// `budget.timeout` elapsed before this section ran.
    DeadlineExceeded,
    /// A registered port ([`ProbeSource`](crate::debug::ProbeSource)
    /// or [`InvariantCheck`](crate::debug::InvariantCheck)) failed or
    /// panicked — counted in
    /// [`DebugHealth::probe_source_failures_total`](crate::debug::DebugHealth)
    /// or `invariant_panics_total`; never fabricated.
    PortFailure,
}

/// How aligned the inspected inputs' generations are — the worst
/// level across every assessed domain.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ConsistencyLevel {
    /// Every assessed domain agreed internally (vacuously true when
    /// nothing generation-bearing was inspected).
    #[default]
    Aligned,
    /// At least one domain spans multiple generations — declared,
    /// never hidden.
    Skewed,
    /// At least one domain's alignment cannot be established —
    /// e.g. a probe deck whose clock mark is `UNKNOWN`.
    Unknown,
}

/// Collector-domain generation stamps seen during the inspect —
/// `None` fields mark inputs that were not provided. All present
/// values belong to the *same* generation domain (collector poll
/// generations), so they are compared with each other — never with
/// probe plan generations.
#[derive(Debug, Clone, Copy, Default)]
pub struct CollectorGenerations {
    /// `MetricsSnapshot::generation()`, if provided.
    pub snapshot: Option<u64>,
    /// `ObservationBatch::generation()`, if provided.
    pub batch: Option<u64>,
    /// `DiagnosticReport::source_generation`, if provided.
    pub diagnostics: Option<u64>,
    /// `true` iff every present pair within this domain is equal.
    pub aligned: bool,
}

/// Alignment assessment of one probe deck.
#[derive(Debug, Clone, Copy)]
pub struct ProbeDeckAlignment {
    /// Deck plan generation.
    pub generation: u64,
    /// `false` when the deck clock mark is `UNKNOWN` — alignment
    /// cannot be established for an unwired deck.
    pub clock_known: bool,
    /// Readings assessed.
    pub readings: usize,
    /// Readings whose generation differs from the deck's.
    pub mismatched: usize,
}

/// Generation/clock alignment across everything the inspect read —
/// reported independently of section success so a failed section
/// never hides skew elsewhere.
#[derive(Debug, Clone, Default)]
pub struct InspectionConsistency {
    /// Worst level across assessed domains
    /// (`Unknown` > `Skewed` > `Aligned`).
    pub level: ConsistencyLevel,
    /// Collector-domain generation stamps.
    pub collector: CollectorGenerations,
    /// Per-deck probe alignment assessments, in adapter order.
    pub probe_decks: Vec<ProbeDeckAlignment>,
}

/// Structured record of everything the inspection budget removed or
/// withheld — truncation is always observable, never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InspectionTruncation {
    /// Items not collected because `max_items`/`max_bytes`/deadline
    /// removed them.
    pub items_dropped: usize,
    /// Items excluded because their confidentiality exceeded the
    /// request's disclosure ceiling.
    pub withheld_confidential: usize,
    /// Approximate bytes accounted for included entries.
    pub bytes_estimated: usize,
    /// `true` when `budget.timeout` stopped collection early.
    pub deadline_exceeded: bool,
    /// Sections reported [`SectionStatus::Unavailable`].
    pub sections_unavailable: usize,
}

/// Validity histogram over the *included* (non-withheld) metrics
/// series — `Discontinuous`/`Stale`/`Partial`/`Unavailable` are
/// counted, never flattened into `Valid`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ValidityCounts {
    /// `MeasurementValidity::Valid`.
    pub valid: usize,
    /// `MeasurementValidity::Stale`.
    pub stale: usize,
    /// `MeasurementValidity::Partial`.
    pub partial: usize,
    /// `MeasurementValidity::Discontinuous`.
    pub discontinuous: usize,
    /// `MeasurementValidity::Unavailable`.
    pub unavailable: usize,
    /// `MeasurementValidity::Unsupported`.
    pub unsupported: usize,
    /// Variants added after this revision
    /// (`MeasurementValidity` is `#[non_exhaustive]`).
    pub other: usize,
}

/// One metrics-series entry — identity, classification, validity and
/// lifecycle preserved verbatim from the [`Measurement`].
/// `value` is `Some` only at
/// [`InspectionDetail::Verbose`](crate::debug::InspectionDetail::Verbose).
#[derive(Debug, Clone)]
pub struct MetricEntry {
    /// Registration-unique source identity (id + generation).
    pub identity: SourceIdentity,
    /// Statically bound metric key.
    pub key: &'static str,
    /// Metric kind.
    pub kind: MetricKind,
    /// Declared observation scope.
    pub scope: ObservationScope,
    /// Lifecycle state the observation was committed under.
    pub lifecycle: SourceLifecycle,
    /// Typed validity — never flattened.
    pub validity: MeasurementValidity,
    /// Reason for reduced confidence, when any.
    pub issue: Option<MeasurementIssue>,
    /// Quantified loss when the collector bounded it.
    pub loss: Option<u64>,
    /// When this observation was committed.
    pub observed_at: Instant,
    /// When the raw value last changed.
    pub last_change_at: Instant,
    /// Declared confidentiality of the series.
    pub confidentiality: Confidentiality,
    /// Typed value — `Some` only at `Verbose` detail.
    pub value: Option<MetricValue>,
}

/// The Metrics section payload.
#[derive(Debug, Clone)]
pub struct MetricsInspection {
    /// Section status.
    pub status: SectionStatus,
    /// `MetricsSnapshot::generation()`.
    pub generation: u64,
    /// When the snapshot was committed.
    pub taken_at: Instant,
    /// Number of *included* (non-withheld) series.
    pub total_series: usize,
    /// Validity histogram over included series.
    pub validity_counts: ValidityCounts,
    /// Per-series entries — bounded by budget and detail level.
    pub entries: Vec<MetricEntry>,
}

/// One finding entry — the explainable summary of a
/// [`DiagnosticFinding`], bounded to identity/classification/status
/// fields.
#[derive(Debug, Clone)]
pub struct FindingEntry {
    /// Stable domain-owned code.
    pub code: DiagnosticCode,
    /// Domain that owns the finding.
    pub domain: &'static str,
    /// Subject source, if bound.
    pub source: Option<SourceIdentity>,
    /// Attention level.
    pub severity: FindingSeverity,
    /// Certainty classification.
    pub certainty: FindingCertainty,
    /// Lifetime state.
    pub status: FindingStatus,
    /// Confidentiality classification.
    pub confidentiality: Confidentiality,
    /// Human-readable summary.
    pub summary: String,
    /// Evidence references explaining the finding.
    pub evidence_refs: Vec<EvidenceRef>,
    /// When first/last observed.
    pub first_seen_at: Instant,
    /// When first/last observed.
    pub last_seen_at: Instant,
}

/// The Diagnostics section payload — findings and domain health from
/// a [`DiagnosticReport`](crate::diagnostics::DiagnosticReport).
#[derive(Debug, Clone)]
pub struct DiagnosticsInspection {
    /// Section status.
    pub status: SectionStatus,
    /// `DiagnosticReport::source_generation` — the collector
    /// generation diagnostics was last evaluated against.
    pub source_generation: u64,
    /// Disclosure the report itself was generated under.
    pub report_disclosure: Confidentiality,
    /// Number of *included* findings.
    pub findings_total: usize,
    /// Included findings still `Open`.
    pub open_findings: usize,
    /// Per-domain health carried verbatim (already disclosure-
    /// filtered by the diagnostics report; re-filtered against this
    /// request's ceiling).
    pub health: Vec<DomainHealth>,
    /// Per-finding entries — bounded by budget and detail level.
    pub entries: Vec<FindingEntry>,
}

/// Aggregate invariant outcomes across evaluated checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvariantSummary {
    /// Checks that returned `Satisfied`.
    pub satisfied: usize,
    /// Checks that returned `Violated`.
    pub violated: usize,
    /// Checks that returned `Inconclusive`.
    pub inconclusive: usize,
}

/// The Invariants section payload.
#[derive(Debug, Clone)]
pub struct InvariantsInspection {
    /// Section status.
    pub status: SectionStatus,
    /// Checks actually evaluated this inspect.
    pub checks_run: usize,
    /// Outcome histogram.
    pub summary: InvariantSummary,
    /// Per-check results — bounded by budget and detail level.
    pub results: Vec<InvariantResult>,
}

/// Cumulative drain accounting retained from the session's
/// [`SessionDrain`](crate::debug::SessionDrain) endpoints.
#[derive(Debug, Clone, Copy, Default)]
pub struct DrainAccounting {
    /// Records accepted into session retention.
    pub records_retained: usize,
    /// Records that exceeded `SessionRequest::max_records`.
    pub records_overflowed: u64,
    /// Endpoints still awaiting writer quiescence.
    pub pending_quiescence: usize,
    /// Records still in flight across endpoints (last view).
    pub pending_records: u64,
    /// `true` once `drain_timeout` elapsed while still `Draining` —
    /// the session keeps draining honestly rather than forcing a
    /// stop.
    pub deadline_exceeded: bool,
}

/// The Session section payload — the debug session's own state,
/// always available (it is this crate's machinery, not an external
/// input).
#[derive(Debug, Clone)]
pub struct SessionInspection {
    /// Section status — `Ok` unless the budget cut collection short
    /// (the payload itself is small and always produced).
    pub status: SectionStatus,
    /// Current lifecycle state.
    pub state: DebugSessionState,
    /// Session instance generation (`0` while `Disabled`).
    pub generation: u64,
    /// Domain-declared session label, if prepared.
    pub label: Option<&'static str>,
    /// Attached drain endpoints.
    pub drains_attached: usize,
    /// Drain accounting.
    pub drain: DrainAccounting,
    /// `true` when the session reached `Stopped` via explicit
    /// `abandon` without confirmed quiescence.
    pub abandoned: bool,
}

/// One probe entry — the observable state of a single probe point.
/// Counter fields are `Some` only at
/// [`InspectionDetail::Verbose`](crate::debug::InspectionDetail::Verbose).
#[derive(Debug, Clone)]
pub struct ProbeEntry {
    /// Domain probe id (opaque).
    pub probe: u64,
    /// Observed node id (opaque).
    pub node: u64,
    /// Observed port id (opaque).
    pub port: u64,
    /// Declared observation mode.
    pub mode: ProbeMode,
    /// Availability classification — `Unknown`/`Unsupported` are
    /// preserved, never flattened.
    pub availability: crate::debug::probe::ProbeAvailability,
    /// Plan generation the binding resolved against.
    pub generation: u64,
    /// Clock identity at read time.
    pub clock: ProbeClockMark,
    /// Latest non-data status event.
    pub last_status: Option<ProbeStatusKind>,
    /// Cumulative counters — `Some` only at `Verbose`.
    pub counters: Option<ProbeCounters>,
}

/// Cumulative probe loss/throughput counters (Verbose detail).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProbeCounters {
    /// Telemetry records accepted.
    pub recorded: u64,
    /// Telemetry records dropped.
    pub dropped: u64,
    /// PCM blocks lost.
    pub lost_blocks: u64,
}

/// One probe deck's inspected view — deck-level alignment plus
/// bounded per-probe entries.
#[derive(Debug, Clone)]
pub struct ProbeDeckView {
    /// Deck plan generation.
    pub deck_generation: u64,
    /// Deck clock mark.
    pub clock: ProbeClockMark,
    /// `false` when the deck clock is `UNKNOWN`.
    pub clock_known: bool,
    /// `true` iff every reading's generation matches the deck's and
    /// the clock is known.
    pub aligned: bool,
    /// Readings whose generation differs from the deck's.
    pub mismatched: usize,
    /// Adapter-declared confidentiality of this deck.
    pub confidentiality: Confidentiality,
    /// Per-probe entries — bounded by budget and detail level.
    pub probes: Vec<ProbeEntry>,
}

/// The Probes section payload.
#[derive(Debug, Clone)]
pub struct ProbesInspection {
    /// Section status — `Unavailable{PortFailure}` when every
    /// registered [`ProbeSource`](crate::debug::ProbeSource) failed.
    pub status: SectionStatus,
    /// Per-deck views in adapter order, bounded by budget.
    pub decks: Vec<ProbeDeckView>,
}

/// One typed inspection section — the closed vocabulary of what an
/// inspect can return, in canonical target order.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum InspectionSection {
    /// [`MetricsInspection`] payload.
    Metrics(MetricsInspection),
    /// [`DiagnosticsInspection`] payload.
    Diagnostics(DiagnosticsInspection),
    /// [`InvariantsInspection`] payload.
    Invariants(InvariantsInspection),
    /// [`SessionInspection`] payload.
    Session(SessionInspection),
    /// [`ProbesInspection`] payload.
    Probes(ProbesInspection),
}

impl InspectionSection {
    /// The target this section answers.
    #[must_use]
    pub const fn target(&self) -> InspectionTarget {
        match self {
            Self::Metrics(_) => InspectionTarget::Metrics,
            Self::Diagnostics(_) => InspectionTarget::Diagnostics,
            Self::Invariants(_) => InspectionTarget::Invariants,
            Self::Session(_) => InspectionTarget::Session,
            Self::Probes(_) => InspectionTarget::Probes,
        }
    }

    /// The section's delivery status.
    #[must_use]
    pub const fn status(&self) -> SectionStatus {
        match self {
            Self::Metrics(s) => s.status,
            Self::Diagnostics(s) => s.status,
            Self::Invariants(s) => s.status,
            Self::Session(s) => s.status,
            Self::Probes(s) => s.status,
        }
    }
}

/// A bounded, typed, explicitly non-transactional inspection result.
#[derive(Debug, Clone)]
pub struct InspectionSnapshot {
    /// Schema version — [`INSPECTION_SCHEMA_VERSION`].
    pub schema_version: u16,
    /// Caller-supplied generation instant.
    pub taken_at: Instant,
    /// Echo of the request this snapshot answered.
    pub request: RequestEcho,
    /// Generation/clock alignment across inputs.
    pub consistency: InspectionConsistency,
    /// Sections in canonical target order.
    pub sections: Vec<InspectionSection>,
    /// What the budget removed or withheld.
    pub truncation: InspectionTruncation,
}
