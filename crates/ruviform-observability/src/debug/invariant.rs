//! Invariant inspection vocabulary — the three-valued verdict
//! contract, the [`InvariantCheck`] port domain owners implement, and
//! the OBS-09 [`Diagnostics`] publishing path (Issue #292, OBS-10).
//!
//! # Three-valued honesty
//!
//! [`InvariantOutcome`] distinguishes `Satisfied` / `Violated` /
//! `Inconclusive` — an invariant that could not be evaluated is
//! **never** collapsed into either definite answer. Every
//! `Inconclusive` carries a typed [`InconclusiveReason`] (missing
//! observation, generation mismatch, unsynchronized read, check
//! decline, or isolated panic) so the *cause* stays explicit rather
//! than guessed.
//!
//! # Diagnostics connection
//!
//! [`DebugInspector::publish_invariants`](crate::debug::DebugInspector::publish_invariants)
//! maps each [`InvariantResult`] onto OBS-09 vocabulary: every
//! evaluated check becomes an [`Evidence`] (`Derived` when the check
//! cited a non-empty basis, `Reported` otherwise), `Violated`
//! becomes a `Confirmed` [`DiagnosticFinding`] at the check's
//! declared severity, and `Inconclusive` becomes a
//! `NotEvaluable`/`Info` finding. `Satisfied` produces evidence
//! only — never a finding.
//!
//! **Known limitation:** `Diagnostics::record_finding` upserts only
//! into `Open` findings and the public API has no resolve path for
//! externally published findings, so a `Violated → Satisfied`
//! recovery leaves the `Open` finding retained (bounded by
//! `max_findings` FIFO). Resolving it is deferred to a diagnostics
//! API revision — see the spec's deferred concerns.

use std::time::Instant;

use crate::collector::{MetricsSnapshot, ObservationBatch};
use crate::diagnostics::{DiagnosticCode, DiagnosticReport, EvidenceRef, FindingSeverity};

use super::probe::ProbeDeckSnapshot;
use super::session::SessionView;

/// The three-valued verdict of one invariant check.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantOutcome {
    /// The declared invariant was verified against observations.
    Satisfied,
    /// The declared invariant was observed violated.
    Violated,
    /// The invariant could not be evaluated — the reason is carried
    /// in [`InconclusiveReason`], never fabricated.
    Inconclusive,
}

/// Why an invariant could not be decided — typed, never a guessed
/// cause.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InconclusiveReason {
    /// Required observation absent (no measurement, probe reading,
    /// or report entry the check needed).
    MissingObservation,
    /// Generation/source-identity mismatch — e.g. a probe index or
    /// source identity differs from the inspected generation.
    GenerationMismatch,
    /// Inputs were not read atomically; a concurrent mutation may
    /// invalidate the verdict.
    UnsynchronizedRead,
    /// The check itself reported it cannot evaluate.
    CheckDeclined,
    /// The check panicked — isolated via `catch_unwind`, counted in
    /// `DebugHealth::invariant_panics_total`.
    CheckPanicked,
}

/// What a check returns — outcome plus optional detail. Construct
/// through [`satisfied`](Self::satisfied) /
/// [`violated`](Self::violated) /
/// [`inconclusive`](Self::inconclusive) so the outcome↔payload
/// invariants (severity required for `Violated`, reason required for
/// `Inconclusive`) cannot be bypassed.
#[derive(Debug, Clone)]
pub struct InvariantVerdict {
    outcome: InvariantOutcome,
    severity: Option<FindingSeverity>,
    inconclusive: Option<InconclusiveReason>,
    summary: String,
    basis: Box<[EvidenceRef]>,
}

impl InvariantVerdict {
    /// The invariant held.
    #[must_use]
    pub fn satisfied(summary: impl Into<String>) -> Self {
        Self {
            outcome: InvariantOutcome::Satisfied,
            severity: None,
            inconclusive: None,
            summary: summary.into(),
            basis: Box::new([]),
        }
    }

    /// The invariant was violated — `severity` is the domain's
    /// declared attention level for the published finding.
    #[must_use]
    pub fn violated(severity: FindingSeverity, summary: impl Into<String>) -> Self {
        Self {
            outcome: InvariantOutcome::Violated,
            severity: Some(severity),
            inconclusive: None,
            summary: summary.into(),
            basis: Box::new([]),
        }
    }

    /// The invariant could not be evaluated — `reason` is the typed
    /// cause, carried verbatim into the result and the published
    /// finding summary.
    #[must_use]
    pub fn inconclusive(reason: InconclusiveReason, summary: impl Into<String>) -> Self {
        Self {
            outcome: InvariantOutcome::Inconclusive,
            severity: None,
            inconclusive: Some(reason),
            summary: summary.into(),
            basis: Box::new([]),
        }
    }

    /// Attaches the [`EvidenceRef`]s this verdict rests on — makes
    /// the published evidence `Derived` instead of `Reported`.
    /// The refs are domain-managed: the check cites refs it recorded
    /// into the same [`Diagnostics`](crate::diagnostics::Diagnostics)
    /// engine itself; unresolved refs are counted by diagnostics
    /// reports (`unresolved_evidence_refs`), never silently dropped.
    #[must_use]
    pub fn with_basis(mut self, basis: impl Into<Box<[EvidenceRef]>>) -> Self {
        self.basis = basis.into();
        self
    }

    /// The verdict outcome.
    #[must_use]
    pub const fn outcome(&self) -> InvariantOutcome {
        self.outcome
    }

    /// Declared severity — `Some` iff `outcome == Violated`.
    #[must_use]
    pub const fn severity(&self) -> Option<FindingSeverity> {
        self.severity
    }

    /// Typed inconclusive reason — `Some` iff
    /// `outcome == Inconclusive`.
    #[must_use]
    pub const fn inconclusive_reason(&self) -> Option<InconclusiveReason> {
        self.inconclusive
    }

    /// Human summary — non-RT construction only; carries no
    /// dedicated fields for paths/bytes/tokens.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Cited evidence basis — empty publishes `Reported` evidence.
    #[must_use]
    pub fn basis(&self) -> &[EvidenceRef] {
        &self.basis
    }
}

/// Read-only view handed to [`InvariantCheck::evaluate`] — assembled
/// once per inspect before any check runs, so every check sees the
/// same inputs (probe decks are collected first so checks can cite
/// readings).
pub struct InspectionContext<'a> {
    /// The inspect's caller-supplied generation instant.
    pub now: Instant,
    /// Provided metrics snapshot, if any.
    pub metrics: Option<&'a MetricsSnapshot>,
    /// Provided observation batch, if any.
    pub batch: Option<&'a ObservationBatch>,
    /// Provided diagnostics report, if any.
    pub diagnostics: Option<&'a DiagnosticReport>,
    /// Probe deck snapshots collected for this inspect (empty when
    /// no `ProbeSource` is registered or the Probes target was not
    /// requested — checks must treat absence as
    /// `MissingObservation`, not as "no probes exist").
    pub probes: &'a [ProbeDeckSnapshot],
    /// Current debug session view.
    pub session: &'a SessionView,
}

/// Non-RT invariant check port — implemented by domain owners and
/// injected via
/// [`DebugInspector::add_invariant_check`](crate::debug::DebugInspector::add_invariant_check).
///
/// Implementations must be non-RT, bounded, and must not block.
/// `evaluate` is called once per inspect that includes
/// [`InspectionTarget::Invariants`](crate::debug::InspectionTarget),
/// in registration order; a panic is isolated by the caller
/// (`catch_unwind`) and reported as `Inconclusive{CheckPanicked}`.
pub trait InvariantCheck: Send {
    /// Stable domain-owned provenance label — findings and evidence
    /// published from this check carry it as their `domain`.
    fn domain(&self) -> &'static str;

    /// Stable domain-owned code — reuse [`DiagnosticCode`]
    /// (`name` + `schema_version`), never a central enum.
    fn code(&self) -> DiagnosticCode;

    /// Evaluates the invariant against `ctx`.
    fn evaluate(&mut self, ctx: &InspectionContext<'_>) -> InvariantVerdict;
}

/// One evaluated check's result as recorded in an
/// [`InspectionSnapshot`](crate::debug::InspectionSnapshot).
#[derive(Debug, Clone)]
pub struct InvariantResult {
    /// Domain provenance label.
    pub domain: &'static str,
    /// Stable domain-owned code.
    pub code: DiagnosticCode,
    /// The verdict outcome.
    pub outcome: InvariantOutcome,
    /// Typed inconclusive reason, when `outcome == Inconclusive`.
    pub inconclusive: Option<InconclusiveReason>,
    /// Declared severity, when `outcome == Violated`.
    pub severity: Option<FindingSeverity>,
    /// Human summary.
    pub summary: String,
    /// Cited evidence basis.
    pub basis: Box<[EvidenceRef]>,
    /// When the check was evaluated.
    pub evaluated_at: Instant,
}

/// What [`publish_invariants`](crate::debug::DebugInspector::publish_invariants)
/// did — every counter is honest about skips, nothing is silently
/// dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvariantPublishReport {
    /// Invariant results examined from the snapshot.
    pub results_examined: usize,
    /// Evidence entries recorded into `Diagnostics`.
    pub evidence_recorded: usize,
    /// Findings recorded into `Diagnostics`.
    pub findings_recorded: usize,
    /// Operations skipped because diagnostics retention is disabled
    /// (`max_evidence`/`max_findings == 0`).
    pub skipped_retention: usize,
    /// `true` when the snapshot carried no Invariants section —
    /// nothing was published.
    pub skipped_no_section: bool,
}
