//! Debug inspector engine — the non-RT aggregator tying inspection
//! budgets, invariant checks, probe ports, session lifecycle, and
//! authorized reporting together (Issue #292, OBS-10).
//!
//! # Contracts
//!
//! - **Non-RT only**: every public method allocates and/or locks.
//!   Never call from an RT writer path. The module exposes *no*
//!   RT-suitable operations — RT-adjacent value types
//!   ([`SessionRecord`](crate::debug::SessionRecord),
//!   [`ProbeReading`](crate::debug::ProbeReading),
//!   [`ProbeClockMark`](crate::debug::ProbeClockMark),
//!   [`ProbeStatusKind`](crate::debug::ProbeStatusKind)) are `Copy`
//!   and alloc-free by construction.
//! - **Logging OFF**: nothing here touches `tracing`, subscribers,
//!   or the `logging` feature.
//! - **Bounded**: every collection runs under
//!   [`InspectionBudget`](crate::debug::InspectionBudget) /
//!   [`ExportLimits`](crate::debug::ExportLimits) /
//!   [`SessionRequest::max_records`](crate::debug::SessionRequest);
//!   overflow is counted, never silently dropped.
//! - **Honest uncertainty**: missing inputs, generation skew, clock
//!   `UNKNOWN`, port failures and check panics are typed statuses —
//!   never fabricated certainty.
//! - **Non-recursive self-observation**: [`DebugHealth`] is a
//!   fixed-size counter struct, never registered as a source.

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::collector::MetricsSnapshot;
use crate::diagnostics::report::exceeds_disclosure;
use crate::diagnostics::{
    DiagnosticFinding, Diagnostics, Evidence, EvidenceMeta, EvidencePayload, EvidenceTime,
    FindingCertainty, FindingSeverity, FindingStatus,
};
use crate::metrics::MeasurementValidity;
use crate::source::Confidentiality;

use super::invariant::{
    InconclusiveReason, InspectionContext, InvariantCheck, InvariantOutcome,
    InvariantPublishReport, InvariantResult, InvariantVerdict,
};
use super::probe::{ProbeDeckSnapshot, ProbeSource};
use super::report::{
    DebugReport, DebugReportRequest, ExportError, ExportTruncation, REPORT_NOTES, RecordSummary,
    SessionReportView, exceeds as report_exceeds,
};
use super::request::{
    InspectionDetail, InspectionError, InspectionInputs, InspectionRequest, InspectionTarget,
    RequestEcho, canonical_targets, withheld,
};
use super::session::{
    DebugAuthority, DebugSessionState, SessionCore, SessionDrain, SessionDrainStatus, SessionError,
    SessionPermit, SessionRecord, SessionRequest, SessionView,
};
use super::snapshot::{
    CollectorGenerations, ConsistencyLevel, DiagnosticsInspection, DrainAccounting, FindingEntry,
    INSPECTION_SCHEMA_VERSION, InspectionConsistency, InspectionSection, InspectionSnapshot,
    InspectionTruncation, InvariantSummary, InvariantsInspection, MetricEntry, MetricsInspection,
    ProbeCounters, ProbeDeckAlignment, ProbeDeckView, ProbeEntry, ProbesInspection, SectionStatus,
    SessionInspection, UnavailableReason, ValidityCounts,
};

/// Engine configuration — disclosure ceilings, external-export
/// policy, and the session drain deadline.
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Highest [`Confidentiality`] an
    /// [`InspectionRequest::disclosure`](crate::debug::InspectionRequest)
    /// may ask for — requests above are rejected as
    /// [`InspectionError::Unauthorized`].
    pub max_inspection_disclosure: Confidentiality,
    /// Highest [`Confidentiality`] a
    /// [`DebugReportRequest::disclosure`](crate::debug::DebugReportRequest)
    /// may ask for.
    pub max_export_disclosure: Confidentiality,
    /// Whether `DebugReportRequest::external` may be granted.
    pub allow_external_export: bool,
    /// Session drain deadline — `None` means Draining persists until
    /// quiescence or explicit abandon; elapsed time is reported via
    /// `deadline_exceeded`, never forced.
    pub drain_timeout: Option<Duration>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            max_inspection_disclosure: Confidentiality::Internal,
            max_export_disclosure: Confidentiality::Internal,
            allow_external_export: false,
            drain_timeout: None,
        }
    }
}

/// The debug inspector — own one per process/consumer. Session
/// control additionally requires [`with_authority`](Self::with_authority).
pub struct DebugInspector {
    config: DebugConfig,
    authority_token: Option<Arc<()>>,
    session: SessionCore,
    invariant_checks: Vec<Box<dyn InvariantCheck>>,
    probe_sources: Vec<Box<dyn ProbeSource>>,
    health: DebugHealth,
}

/// Fixed-size self-observation counters — never registered as a
/// source, so debug cannot recurse into itself.
#[derive(Debug, Default, Clone)]
pub struct DebugHealth {
    /// `inspect` calls that passed authorization.
    pub inspections_total: u64,
    /// Inspect requests rejected as unauthorized.
    pub inspection_errors_total: u64,
    /// Items dropped by inspection budgets, total.
    pub items_truncated_total: u64,
    /// Sections reported unavailable, total.
    pub sections_unavailable_total: u64,
    /// Items withheld by disclosure ceilings, total.
    pub withheld_confidential_total: u64,
    /// `ProbeSource::decks` calls that panicked.
    pub probe_source_failures_total: u64,
    /// Invariant checks evaluated, total.
    pub invariant_checks_total: u64,
    /// Checks returning `Violated`, total.
    pub invariant_violated_total: u64,
    /// Checks returning `Inconclusive`, total.
    pub invariant_inconclusive_total: u64,
    /// Checks that panicked (isolated), total.
    pub invariant_panics_total: u64,
    /// Sessions prepared, total.
    pub sessions_prepared_total: u64,
    /// Sessions activated, total.
    pub sessions_activated_total: u64,
    /// Sessions reaching `Stopped` via confirmed drain, total.
    pub sessions_stopped_total: u64,
    /// Sessions reaching `Stopped` via `abandon`, total.
    pub sessions_abandoned_total: u64,
    /// Drained records delivered into session retention, total.
    pub drain_records_delivered_total: u64,
    /// Drained records exceeding `max_records`, total.
    pub drain_records_overflowed_total: u64,
    /// Reports successfully generated, total.
    pub reports_generated_total: u64,
    /// Report requests rejected, total.
    pub reports_rejected_total: u64,
    /// Findings published into `Diagnostics`, total.
    pub findings_published_total: u64,
    /// Evidence entries published into `Diagnostics`, total.
    pub evidence_published_total: u64,
}

/// Running budget state shared by section builders — `truncation`
/// is the snapshot-global ledger; each section diffs its own
/// contribution for status purposes.
struct Budget<'a> {
    detail: InspectionDetail,
    disclosure: Confidentiality,
    max_items: usize,
    max_bytes: usize,
    deadline: Option<Instant>,
    truncation: &'a mut InspectionTruncation,
}

/// Per-section diff of the global truncation ledger.
#[derive(Clone, Copy, Default)]
struct SectionTruncation {
    items_dropped: usize,
    withheld_confidential: usize,
}

impl Budget<'_> {
    fn expired(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }

    /// Remaining byte room — `usize::MAX` when unlimited.
    fn room(&self) -> usize {
        if self.max_bytes == 0 {
            usize::MAX
        } else {
            self.max_bytes
                .saturating_sub(self.truncation.bytes_estimated)
        }
    }

    fn charge(&mut self, bytes: usize) {
        self.truncation.bytes_estimated = self.truncation.bytes_estimated.saturating_add(bytes);
    }

    /// Marks `count` items as budget-dropped.
    fn drop_items(&mut self, count: usize) {
        self.truncation.items_dropped = self.truncation.items_dropped.saturating_add(count);
    }

    /// Whether `confidentiality` exceeds the request disclosure.
    fn withheld(&self, c: Confidentiality) -> bool {
        withheld(c, self.disclosure)
    }

    /// Per-item entry admission at the requested detail level.
    /// Returns `false` — without accounting a drop — when entries
    /// are not collected at all (`Summary` detail or
    /// `max_items == 0`, both deliberate modes per
    /// [`InspectionBudget`](crate::debug::InspectionBudget)).
    /// Otherwise a budget refusal is counted in `items_dropped`.
    fn admit_entry(&mut self, kept: usize, est_bytes: usize) -> bool {
        if self.detail == InspectionDetail::Summary || self.max_items == 0 {
            return false;
        }
        if kept >= self.max_items || est_bytes > self.room() {
            self.truncation.items_dropped += 1;
            return false;
        }
        true
    }
}

impl DebugInspector {
    /// Creates an inspector without session authority — session
    /// operations return [`SessionError::NoAuthority`].
    #[must_use]
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            authority_token: None,
            session: SessionCore::new(),
            invariant_checks: Vec::new(),
            probe_sources: Vec::new(),
            health: DebugHealth::default(),
        }
    }

    /// Creates an inspector with session control enabled by
    /// `authority` — the authority's binding token is stored so only
    /// permits *it* mints are accepted.
    #[must_use]
    pub fn with_authority(config: DebugConfig, authority: &DebugAuthority) -> Self {
        Self {
            authority_token: Some(Arc::clone(authority.token())),
            ..Self::new(config)
        }
    }

    /// Registers an invariant check — evaluation order is
    /// registration order (deterministic).
    pub fn add_invariant_check(&mut self, check: Box<dyn InvariantCheck>) {
        self.invariant_checks.push(check);
    }

    /// Registers a probe source — deck collection order is
    /// registration order.
    pub fn add_probe_source(&mut self, source: Box<dyn ProbeSource>) {
        self.probe_sources.push(source);
    }

    /// Runs one bounded, non-RT inspection.
    ///
    /// # Errors
    ///
    /// [`InspectionError::Unauthorized`] when
    /// `request.disclosure` exceeds `max_inspection_disclosure`.
    pub fn inspect(
        &mut self,
        request: &InspectionRequest,
        inputs: &InspectionInputs<'_>,
        now: Instant,
    ) -> Result<InspectionSnapshot, InspectionError> {
        if exceeds_disclosure(request.disclosure, self.config.max_inspection_disclosure) {
            self.health.inspection_errors_total += 1;
            return Err(InspectionError::Unauthorized {
                requested: request.disclosure,
                permitted: self.config.max_inspection_disclosure,
            });
        }
        self.health.inspections_total += 1;

        let start = Instant::now();
        let deadline = request.budget.timeout.and_then(|d| start.checked_add(d));
        let targets = canonical_targets(&request.targets);

        // Probe decks are collected once, before sections, so both
        // the Probes section and Invariant checks see the same
        // readings. Needed when either target is requested.
        let need_probes = targets.contains(&InspectionTarget::Probes)
            || targets.contains(&InspectionTarget::Invariants);
        let mut decks: Vec<ProbeDeckSnapshot> = Vec::new();
        let mut probe_sources_ok = 0usize;
        if need_probes {
            for source in &mut self.probe_sources {
                let result = catch_unwind(AssertUnwindSafe(|| source.decks()));
                match result {
                    Ok(d) => {
                        probe_sources_ok += 1;
                        decks.extend(d);
                    }
                    Err(_) => self.health.probe_source_failures_total += 1,
                }
            }
        }
        let probes_failed = need_probes && !self.probe_sources.is_empty() && probe_sources_ok == 0;
        let probes_absent =
            targets.contains(&InspectionTarget::Probes) && self.probe_sources.is_empty();

        let session_view = self.session.view();
        let ctx = InspectionContext {
            now,
            metrics: inputs.metrics,
            batch: inputs.batch,
            diagnostics: inputs.diagnostics,
            probes: &decks,
            session: &session_view,
        };

        let mut truncation = InspectionTruncation::default();
        let mut sections = Vec::with_capacity(targets.len());
        for target in &targets {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                truncation.deadline_exceeded = true;
                sections.push(unavailable_section(
                    *target,
                    UnavailableReason::DeadlineExceeded,
                ));
                continue;
            }
            let mut budget = Budget {
                detail: request.detail,
                disclosure: request.disclosure,
                max_items: request.budget.max_items,
                max_bytes: request.budget.max_bytes,
                deadline,
                truncation: &mut truncation,
            };
            let section = match target {
                InspectionTarget::Metrics => Self::build_metrics(inputs.metrics, &mut budget),
                InspectionTarget::Diagnostics => {
                    Self::build_diagnostics(inputs.diagnostics, &mut budget)
                }
                InspectionTarget::Invariants => self.build_invariants(&ctx, &mut budget),
                InspectionTarget::Session => Self::build_session(&session_view),
                InspectionTarget::Probes => {
                    Self::build_probes(&decks, probes_failed, probes_absent, &mut budget)
                }
            };
            sections.push(section);
        }

        // Consistency is computed from the raw generation stamps —
        // independent of whether individual sections succeeded.
        let consistency = Self::consistency(inputs, &decks, need_probes);
        truncation.sections_unavailable = sections
            .iter()
            .filter(|s| matches!(s.status(), SectionStatus::Unavailable { .. }))
            .count();

        self.health.items_truncated_total += truncation.items_dropped as u64;
        self.health.sections_unavailable_total += truncation.sections_unavailable as u64;
        self.health.withheld_confidential_total += truncation.withheld_confidential as u64;

        Ok(InspectionSnapshot {
            schema_version: INSPECTION_SCHEMA_VERSION,
            taken_at: now,
            request: RequestEcho {
                targets: targets.clone(),
                detail: request.detail,
                disclosure: request.disclosure,
                budget: request.budget,
            },
            consistency,
            sections,
            truncation,
        })
    }

    fn build_metrics(
        input: Option<&MetricsSnapshot>,
        budget: &mut Budget<'_>,
    ) -> InspectionSection {
        let base = SectionTruncation {
            items_dropped: budget.truncation.items_dropped,
            withheld_confidential: budget.truncation.withheld_confidential,
        };
        let Some(snapshot) = input else {
            return unavailable_section(
                InspectionTarget::Metrics,
                UnavailableReason::InputNotProvided,
            );
        };
        let mut counts = ValidityCounts::default();
        let mut entries = Vec::new();
        let mut included = 0usize;
        let total = snapshot.measurements().len();
        for (i, m) in snapshot.measurements().iter().enumerate() {
            if budget.expired() {
                budget.truncation.deadline_exceeded = true;
                budget.drop_items(total - i);
                break;
            }
            let conf = m.descriptor.semantics.confidentiality;
            if budget.withheld(conf) {
                budget.truncation.withheld_confidential += 1;
                continue;
            }
            included += 1;
            count_validity(&mut counts, m.validity);
            let est = metric_entry_bytes(m, budget.detail);
            if budget.admit_entry(entries.len(), est) {
                entries.push(metric_entry(m, budget.detail));
                budget.charge(est);
            }
        }
        let status = section_status(
            included,
            budget.truncation.items_dropped - base.items_dropped,
            budget.truncation.withheld_confidential - base.withheld_confidential,
            total,
        );
        InspectionSection::Metrics(MetricsInspection {
            status,
            generation: snapshot.generation(),
            taken_at: snapshot.taken_at(),
            total_series: included,
            validity_counts: counts,
            entries,
        })
    }

    fn build_diagnostics(
        input: Option<&crate::diagnostics::DiagnosticReport>,
        budget: &mut Budget<'_>,
    ) -> InspectionSection {
        let base = SectionTruncation {
            items_dropped: budget.truncation.items_dropped,
            withheld_confidential: budget.truncation.withheld_confidential,
        };
        let Some(report) = input else {
            return unavailable_section(
                InspectionTarget::Diagnostics,
                UnavailableReason::InputNotProvided,
            );
        };
        let mut entries = Vec::new();
        let mut visible = 0usize;
        let mut findings_visible = 0usize;
        let mut open = 0usize;
        let findings_total = report.findings.len();
        for (i, f) in report.findings.iter().enumerate() {
            if budget.expired() {
                budget.truncation.deadline_exceeded = true;
                budget.drop_items(findings_total - i);
                break;
            }
            if budget.withheld(f.confidentiality()) {
                budget.truncation.withheld_confidential += 1;
                continue;
            }
            visible += 1;
            findings_visible += 1;
            if f.status() == FindingStatus::Open {
                open += 1;
            }
            let est = finding_entry_bytes(f.summary().len(), f.evidence_refs().len());
            if budget.admit_entry(entries.len(), est) {
                entries.push(finding_entry(f));
                budget.charge(est);
            }
        }
        // Domain-health entries are section-level summaries already
        // bounded by the diagnostics engine's domain count —
        // disclosure filtered and byte-charged, never item-capped.
        let mut health = Vec::new();
        let health_total = report.health.len();
        for (i, h) in report.health.iter().enumerate() {
            if budget.expired() {
                budget.truncation.deadline_exceeded = true;
                budget.drop_items(health_total - i);
                break;
            }
            if budget.withheld(h.confidentiality) {
                budget.truncation.withheld_confidential += 1;
                continue;
            }
            visible += 1;
            let est = size_of::<crate::diagnostics::DomainHealth>()
                + h.rules.len() * size_of::<crate::diagnostics::RuleHealth>();
            if est > budget.room() {
                budget.truncation.items_dropped += 1;
                continue;
            }
            budget.charge(est);
            health.push(h.clone());
        }
        let status = section_status(
            visible,
            budget.truncation.items_dropped - base.items_dropped,
            budget.truncation.withheld_confidential - base.withheld_confidential,
            findings_total + health_total,
        );
        InspectionSection::Diagnostics(DiagnosticsInspection {
            status,
            source_generation: report.source_generation,
            report_disclosure: report.disclosure,
            findings_total: findings_visible,
            open_findings: open,
            health,
            entries,
        })
    }

    fn build_invariants(
        &mut self,
        ctx: &InspectionContext<'_>,
        budget: &mut Budget<'_>,
    ) -> InspectionSection {
        let base = SectionTruncation {
            items_dropped: budget.truncation.items_dropped,
            withheld_confidential: budget.truncation.withheld_confidential,
        };
        let mut results: Vec<InvariantResult> = Vec::new();
        let mut summary = InvariantSummary::default();
        let mut evaluated = 0usize;
        let mut checks = std::mem::take(&mut self.invariant_checks);
        let total_checks = checks.len();
        // Evaluate each check once; a panic is isolated into
        // Inconclusive{CheckPanicked} and counted, never propagated.
        let mut next = 0usize;
        while next < checks.len() {
            if budget.expired() {
                budget.truncation.deadline_exceeded = true;
                budget.drop_items(checks.len() - next);
                break;
            }
            let (domain, code) = {
                let c = &checks[next];
                (c.domain(), c.code())
            };
            let outcome =
                if let Ok(v) = catch_unwind(AssertUnwindSafe(|| checks[next].evaluate(ctx))) {
                    v
                } else {
                    self.health.invariant_panics_total += 1;
                    InvariantVerdict::inconclusive(
                        InconclusiveReason::CheckPanicked,
                        "invariant check panicked",
                    )
                };
            evaluated += 1;
            self.health.invariant_checks_total += 1;
            match outcome.outcome() {
                InvariantOutcome::Satisfied => summary.satisfied += 1,
                InvariantOutcome::Violated => {
                    summary.violated += 1;
                    self.health.invariant_violated_total += 1;
                }
                InvariantOutcome::Inconclusive => {
                    summary.inconclusive += 1;
                    self.health.invariant_inconclusive_total += 1;
                }
            }
            let result = InvariantResult {
                domain,
                code,
                outcome: outcome.outcome(),
                inconclusive: outcome.inconclusive_reason(),
                severity: outcome.severity(),
                summary: outcome.summary().to_owned(),
                basis: outcome.basis().into(),
                evaluated_at: ctx.now,
            };
            let est = invariant_result_bytes(&result);
            if budget.admit_entry(results.len(), est) {
                results.push(result);
                budget.charge(est);
            }
            next += 1;
        }
        self.invariant_checks = checks;
        let status = section_status(
            evaluated,
            budget.truncation.items_dropped - base.items_dropped,
            0,
            total_checks,
        );
        InspectionSection::Invariants(InvariantsInspection {
            status,
            checks_run: evaluated,
            summary,
            results,
        })
    }

    fn build_session(view: &SessionView) -> InspectionSection {
        InspectionSection::Session(SessionInspection {
            status: SectionStatus::Ok,
            state: view.state,
            generation: view.generation,
            label: view.label,
            drains_attached: view.drains_attached,
            drain: DrainAccounting {
                records_retained: view.records_retained,
                records_overflowed: view.records_overflowed,
                pending_quiescence: view.pending_quiescence,
                pending_records: view.pending_records,
                deadline_exceeded: view.deadline_exceeded,
            },
            abandoned: view.abandoned,
        })
    }

    fn build_probes(
        decks: &[ProbeDeckSnapshot],
        failed: bool,
        absent: bool,
        budget: &mut Budget<'_>,
    ) -> InspectionSection {
        let base = SectionTruncation {
            items_dropped: budget.truncation.items_dropped,
            withheld_confidential: budget.truncation.withheld_confidential,
        };
        if absent {
            return unavailable_section(
                InspectionTarget::Probes,
                UnavailableReason::InputNotProvided,
            );
        }
        if failed {
            return unavailable_section(InspectionTarget::Probes, UnavailableReason::PortFailure);
        }
        let mut views = Vec::new();
        let mut visible_decks = 0usize;
        // `max_items` is a per-section bound — probe entries across
        // all decks share it.
        let mut probe_entries_kept = 0usize;
        // Items are probe *readings* — an unassessed deck contributes
        // its reading count, never a deck-count unit mismatch.
        let remaining_readings =
            |from: usize| -> usize { decks[from..].iter().map(|d| d.probes.len()).sum() };
        'decks: for (di, deck) in decks.iter().enumerate() {
            if budget.expired() {
                budget.truncation.deadline_exceeded = true;
                budget.drop_items(remaining_readings(di));
                break;
            }
            if budget.withheld(deck.confidentiality) {
                budget.truncation.withheld_confidential += 1;
                continue;
            }
            let clock_known = !deck.clock.is_unknown();
            let mismatched = deck
                .probes
                .iter()
                .filter(|r| r.generation != deck.generation)
                .count();
            let mut probes = Vec::new();
            let mut hit_deadline = false;
            for (pi, reading) in deck.probes.iter().enumerate() {
                if budget.expired() {
                    budget.truncation.deadline_exceeded = true;
                    budget.drop_items(deck.probes.len() - pi);
                    hit_deadline = true;
                    break;
                }
                // Readings inherit the deck's declared
                // confidentiality — already admitted above.
                let est = probe_entry_bytes(budget.detail);
                if budget.admit_entry(probe_entries_kept, est) {
                    probes.push(probe_entry(reading, budget.detail));
                    probe_entries_kept += 1;
                    budget.charge(est);
                }
            }
            // The partially collected deck is still reported — its
            // admitted entries are kept and the section status says
            // Truncated.
            visible_decks += 1;
            views.push(ProbeDeckView {
                deck_generation: deck.generation,
                clock: deck.clock,
                clock_known,
                aligned: clock_known && mismatched == 0,
                mismatched,
                confidentiality: deck.confidentiality,
                probes,
            });
            if hit_deadline {
                budget.drop_items(remaining_readings(di + 1));
                break 'decks;
            }
        }
        let status = section_status(
            visible_decks,
            budget.truncation.items_dropped - base.items_dropped,
            budget.truncation.withheld_confidential - base.withheld_confidential,
            decks.len(),
        );
        InspectionSection::Probes(ProbesInspection {
            status,
            decks: views,
        })
    }

    /// Generation/clock alignment across all read inputs — computed
    /// from raw stamps, independent of section success.
    fn consistency(
        inputs: &InspectionInputs<'_>,
        decks: &[ProbeDeckSnapshot],
        probes_assessed: bool,
    ) -> InspectionConsistency {
        let mut collector = CollectorGenerations::default();
        if let Some(m) = inputs.metrics {
            collector.snapshot = Some(m.generation());
        }
        if let Some(b) = inputs.batch {
            collector.batch = Some(b.generation());
        }
        if let Some(d) = inputs.diagnostics {
            // `0` is the diagnostics sentinel for "never evaluated"
            // (`DiagnosticReport::source_generation`), not a real
            // collector generation — treating it as one would
            // fabricate skew against every evaluated input.
            if d.source_generation != 0 {
                collector.diagnostics = Some(d.source_generation);
            }
        }
        let present: Vec<u64> = [collector.snapshot, collector.batch, collector.diagnostics]
            .into_iter()
            .flatten()
            .collect();
        collector.aligned = present.iter().all(|g| Some(*g) == present.first().copied());

        let mut probe_decks = Vec::new();
        // Confirmed inconsistency outranks unverifiable clocks:
        // `Skewed` is positive evidence, `Unknown` is absent
        // evidence — an unknown probe clock must not mask a known
        // generation mismatch (same rule as OBS-09 health: `Unknown`
        // never erases a known-bad state).
        let mut skewed = !collector.aligned;
        let mut unknown_clock = false;
        if probes_assessed {
            for deck in decks {
                let clock_known = !deck.clock.is_unknown();
                let mismatched = deck
                    .probes
                    .iter()
                    .filter(|r| r.generation != deck.generation)
                    .count();
                probe_decks.push(ProbeDeckAlignment {
                    generation: deck.generation,
                    clock_known,
                    readings: deck.probes.len(),
                    mismatched,
                });
                if mismatched > 0 {
                    skewed = true;
                }
                if !clock_known {
                    unknown_clock = true;
                }
            }
        }
        let level = if skewed {
            ConsistencyLevel::Skewed
        } else if unknown_clock {
            ConsistencyLevel::Unknown
        } else {
            ConsistencyLevel::Aligned
        };
        InspectionConsistency {
            level,
            collector,
            probe_decks,
        }
    }

    /// Publishes invariant results from `snapshot` into OBS-09
    /// `Diagnostics` — evidence for every evaluated check, findings
    /// for `Violated`/`Inconclusive` only.
    ///
    /// Non-RT only. Skips are counted in the returned report, never
    /// silent.
    pub fn publish_invariants(
        &mut self,
        diagnostics: &mut Diagnostics,
        snapshot: &InspectionSnapshot,
        now: Instant,
    ) -> InvariantPublishReport {
        let mut report = InvariantPublishReport::default();
        let Some(InspectionSection::Invariants(inv)) = snapshot
            .sections
            .iter()
            .find(|s| matches!(s, InspectionSection::Invariants(_)))
        else {
            report.skipped_no_section = true;
            return report;
        };
        for result in &inv.results {
            report.results_examined += 1;
            let meta = EvidenceMeta {
                domain: result.domain,
                subject: Some(result.code.name),
                time: EvidenceTime::Monotonic(result.evaluated_at),
                validity: MeasurementValidity::Valid,
                confidentiality: Confidentiality::Internal,
                payload: EvidencePayload::Claim(result.code.name),
            };
            let evidence = if result.basis.is_empty() {
                Evidence::reported(None, meta)
            } else {
                // basis is non-empty by construction — derived cannot
                // fail here; the reported fallback is defensive only.
                Evidence::derived(result.basis.clone(), None, meta.clone())
                    .unwrap_or_else(|_| Evidence::reported(None, meta))
            };
            let mut finding_refs: Vec<crate::diagnostics::EvidenceRef> = result.basis.to_vec();
            match diagnostics.record_evidence(evidence) {
                Ok(ev_ref) => {
                    report.evidence_recorded += 1;
                    self.health.evidence_published_total += 1;
                    finding_refs.push(ev_ref);
                }
                Err(_) => report.skipped_retention += 1,
            }
            let (severity, certainty) = match result.outcome {
                InvariantOutcome::Violated => (
                    result.severity.unwrap_or(FindingSeverity::Warning),
                    FindingCertainty::Confirmed,
                ),
                InvariantOutcome::Inconclusive => {
                    (FindingSeverity::Info, FindingCertainty::NotEvaluable)
                }
                InvariantOutcome::Satisfied => continue,
            };
            let Ok(finding) = DiagnosticFinding::new(
                result.code,
                result.domain,
                None,
                severity,
                certainty,
                Confidentiality::Internal,
                result.summary.clone(),
                finding_refs,
                now,
            ) else {
                report.skipped_retention += 1;
                continue;
            };
            match diagnostics.record_finding(finding) {
                Ok(()) => {
                    report.findings_recorded += 1;
                    self.health.findings_published_total += 1;
                }
                Err(_) => report.skipped_retention += 1,
            }
        }
        report
    }

    /// Generates a bounded, disclosure-authorized [`DebugReport`].
    ///
    /// # Errors
    ///
    /// [`ExportError::Unauthorized`] when `request.disclosure`
    /// exceeds `max_export_disclosure`;
    /// [`ExportError::ExternalNotPermitted`] when `external` is set
    /// without `allow_external_export`.
    pub fn report(
        &mut self,
        request: &DebugReportRequest,
        snapshot: Option<&InspectionSnapshot>,
        now: Instant,
    ) -> Result<DebugReport, ExportError> {
        if exceeds_disclosure(request.disclosure, self.config.max_export_disclosure) {
            self.health.reports_rejected_total += 1;
            return Err(ExportError::Unauthorized {
                requested: request.disclosure,
                permitted: self.config.max_export_disclosure,
            });
        }
        if request.external && !self.config.allow_external_export {
            self.health.reports_rejected_total += 1;
            return Err(ExportError::ExternalNotPermitted);
        }
        self.health.reports_generated_total += 1;

        let session_view = self.session.view();
        let inspection = snapshot.map(super::report::inspection_echo);
        let limits = request.limits;
        let start = Instant::now();
        let mut truncation = ExportTruncation::default();

        // Newest-first record summaries under disclosure + bounds.
        // `max_items == 0` is the deliberate summaries-only mode per
        // [`ExportLimits`] — no record entries, no drop accounting.
        let mut records = Vec::new();
        if limits.max_items > 0 {
            let total = self.session.records().len();
            let mut seen = 0usize;
            for record in self.session.records().iter().rev() {
                seen += 1;
                if limits.deadline.is_some_and(|d| start.elapsed() >= d) {
                    truncation.deadline_exceeded = true;
                    truncation.items_dropped += total - seen + 1;
                    break;
                }
                let header = record.record.header();
                if report_exceeds(header.confidentiality(), request.disclosure) {
                    truncation.withheld_confidential += 1;
                    continue;
                }
                if records.len() >= limits.max_items {
                    truncation.items_dropped += 1;
                    continue;
                }
                let est = size_of::<RecordSummary>();
                if limits.max_bytes > 0
                    && truncation.bytes_estimated.saturating_add(est) > limits.max_bytes
                {
                    truncation.items_dropped += 1;
                    continue;
                }
                truncation.bytes_estimated = truncation.bytes_estimated.saturating_add(est);
                let clock = header.clock();
                records.push(RecordSummary {
                    event: header.event(),
                    version: header.version(),
                    source: header.source(),
                    generation: header.generation(),
                    clock_stamped: clock.is_some(),
                    clock_domain: clock.map_or(0, |c| c.domain().get()),
                    clock_epoch: clock.map_or(0, |c| c.epoch().get()),
                    confidentiality: header.confidentiality(),
                    dropped_before: record.dropped_before,
                });
            }
        }

        Ok(DebugReport {
            schema_version: super::report::DEBUG_REPORT_SCHEMA_VERSION,
            generated_at: now,
            disclosure: request.disclosure,
            external: request.external,
            inspection,
            session: SessionReportView::from(&session_view),
            records,
            truncation,
            notes: REPORT_NOTES.to_vec(),
        })
    }

    // ---- session API (thin delegations; permit checks live here) ----

    /// Current session state.
    #[must_use]
    pub fn session_state(&self) -> DebugSessionState {
        self.session.state()
    }

    /// Owned session view for inspection/report consumers.
    #[must_use]
    pub fn session_view(&self) -> SessionView {
        self.session.view()
    }

    /// Retained drained records, oldest first.
    #[must_use]
    pub fn drained_records(&self) -> &VecDeque<SessionRecord> {
        self.session.records()
    }

    /// `Disabled | Stopped → Prepared` under a valid permit.
    ///
    /// # Errors
    ///
    /// [`SessionError::NoAuthority`] / [`SessionError::PermitDenied`] /
    /// [`SessionError::SessionBusy`] / [`SessionError::InvalidRequest`].
    pub fn prepare_session(
        &mut self,
        permit: SessionPermit,
        request: SessionRequest,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_permit(permit)?;
        self.session
            .prepare(request.label, request.max_records, now)?;
        self.health.sessions_prepared_total += 1;
        Ok(())
    }

    /// Attaches a drain endpoint (`Prepared | Active` only).
    ///
    /// # Errors
    ///
    /// [`SessionError::InvalidTransition`].
    pub fn attach_drain(&mut self, drain: Box<dyn SessionDrain>) -> Result<(), SessionError> {
        self.session.attach(drain)
    }

    /// `Prepared → Active`; requires at least one drain.
    ///
    /// # Errors
    ///
    /// [`SessionError::InvalidTransition`] / [`SessionError::NoDrains`].
    pub fn activate_session(&mut self, now: Instant) -> Result<(), SessionError> {
        self.session.activate(now)?;
        self.health.sessions_activated_total += 1;
        Ok(())
    }

    /// `Prepared | Active → Draining` under a valid permit.
    ///
    /// # Errors
    ///
    /// [`SessionError::NoAuthority`] / [`SessionError::PermitDenied`] /
    /// [`SessionError::InvalidTransition`].
    pub fn request_stop(
        &mut self,
        permit: SessionPermit,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_permit(permit)?;
        self.session.request_stop(now)
    }

    /// Advances draining — see [`SessionCore::poll_drain`]. Reaching
    /// `Stopped` releases drain endpoints (rings/`Arc`s).
    #[must_use]
    pub fn poll_drain(&mut self, now: Instant) -> SessionDrainStatus {
        let before = self.session.drain_totals();
        let status = self.session.poll_drain(self.config.drain_timeout, now);
        let after = self.session.drain_totals();
        self.health.drain_records_delivered_total += after.0 - before.0;
        self.health.drain_records_overflowed_total += after.1 - before.1;
        if matches!(status, SessionDrainStatus::Stopped { .. }) {
            self.health.sessions_stopped_total += 1;
        }
        status
    }

    /// Explicit abandon path — `Prepared | Active | Draining →
    /// Stopped` without confirmed quiescence; counted and reported.
    ///
    /// # Errors
    ///
    /// [`SessionError::NoAuthority`] / [`SessionError::PermitDenied`] /
    /// [`SessionError::InvalidTransition`].
    pub fn abandon_session(
        &mut self,
        permit: SessionPermit,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_permit(permit)?;
        self.session.abandon(now)?;
        self.health.sessions_abandoned_total += 1;
        Ok(())
    }

    /// Fixed-size self-observation counters.
    #[must_use]
    pub fn health(&self) -> DebugHealth {
        self.health.clone()
    }

    /// Permit validation shared by session control ops — consumes
    /// the permit: presenting a capability spends it whether the
    /// operation then succeeds or fails.
    fn check_permit(&self, permit: SessionPermit) -> Result<(), SessionError> {
        let Some(token) = &self.authority_token else {
            return Err(SessionError::NoAuthority);
        };
        let authorized = Arc::ptr_eq(token, permit.token());
        drop(permit);
        if authorized {
            Ok(())
        } else {
            Err(SessionError::PermitDenied)
        }
    }
}

/// Section status from scan totals — `Withheld` when every input
/// item was disclosure-blocked, `Truncated` when the budget removed
/// items, else `Ok`.
fn section_status(
    visible: usize,
    items_dropped: usize,
    withheld_count: usize,
    total_input: usize,
) -> SectionStatus {
    if total_input > 0 && visible == 0 && withheld_count > 0 {
        SectionStatus::Withheld
    } else if items_dropped > 0 {
        SectionStatus::Truncated
    } else {
        SectionStatus::Ok
    }
}

fn unavailable_section(target: InspectionTarget, reason: UnavailableReason) -> InspectionSection {
    match target {
        InspectionTarget::Metrics => InspectionSection::Metrics(MetricsInspection {
            status: SectionStatus::Unavailable { reason },
            generation: 0,
            taken_at: Instant::now(),
            total_series: 0,
            validity_counts: ValidityCounts::default(),
            entries: Vec::new(),
        }),
        InspectionTarget::Diagnostics => InspectionSection::Diagnostics(DiagnosticsInspection {
            status: SectionStatus::Unavailable { reason },
            source_generation: 0,
            report_disclosure: Confidentiality::Internal,
            findings_total: 0,
            open_findings: 0,
            health: Vec::new(),
            entries: Vec::new(),
        }),
        InspectionTarget::Invariants => InspectionSection::Invariants(InvariantsInspection {
            status: SectionStatus::Unavailable { reason },
            checks_run: 0,
            summary: InvariantSummary::default(),
            results: Vec::new(),
        }),
        InspectionTarget::Session => InspectionSection::Session(SessionInspection {
            status: SectionStatus::Unavailable { reason },
            state: DebugSessionState::Disabled,
            generation: 0,
            label: None,
            drains_attached: 0,
            drain: DrainAccounting::default(),
            abandoned: false,
        }),
        InspectionTarget::Probes => InspectionSection::Probes(ProbesInspection {
            status: SectionStatus::Unavailable { reason },
            decks: Vec::new(),
        }),
    }
}

fn count_validity(counts: &mut ValidityCounts, v: MeasurementValidity) {
    match v {
        MeasurementValidity::Valid => counts.valid += 1,
        MeasurementValidity::Stale => counts.stale += 1,
        MeasurementValidity::Partial => counts.partial += 1,
        MeasurementValidity::Discontinuous => counts.discontinuous += 1,
        MeasurementValidity::Unavailable => counts.unavailable += 1,
        MeasurementValidity::Unsupported => counts.unsupported += 1,
    }
}

fn metric_entry(m: &crate::metrics::Measurement, detail: InspectionDetail) -> MetricEntry {
    MetricEntry {
        identity: m.identity,
        key: m.descriptor.key,
        kind: m.descriptor.kind,
        scope: m.scope,
        lifecycle: m.lifecycle,
        validity: m.validity,
        issue: m.issue,
        loss: m.loss,
        observed_at: m.observed_at,
        last_change_at: m.last_change_at,
        confidentiality: m.descriptor.semantics.confidentiality,
        value: if detail == InspectionDetail::Verbose {
            m.value.clone()
        } else {
            None
        },
    }
}

fn metric_entry_bytes(m: &crate::metrics::Measurement, detail: InspectionDetail) -> usize {
    size_of::<MetricEntry>()
        + match (&m.value, detail) {
            (
                Some(crate::metrics::MetricValue::Histogram { buckets, .. }),
                InspectionDetail::Verbose,
            ) => buckets.len() * size_of::<u64>(),
            _ => 0,
        }
}

fn finding_entry(f: &DiagnosticFinding) -> FindingEntry {
    FindingEntry {
        code: f.code(),
        domain: f.domain(),
        source: f.source(),
        severity: f.severity(),
        certainty: f.certainty(),
        status: f.status(),
        confidentiality: f.confidentiality(),
        summary: f.summary().to_owned(),
        evidence_refs: f.evidence_refs().to_vec(),
        first_seen_at: f.first_seen_at(),
        last_seen_at: f.last_seen_at(),
    }
}

fn finding_entry_bytes(summary_len: usize, refs: usize) -> usize {
    size_of::<FindingEntry>() + summary_len + refs * size_of::<crate::diagnostics::EvidenceRef>()
}

fn invariant_result_bytes(r: &InvariantResult) -> usize {
    size_of::<InvariantResult>()
        + r.summary.len()
        + r.basis.len() * size_of::<crate::diagnostics::EvidenceRef>()
}

fn probe_entry(r: &super::probe::ProbeReading, detail: InspectionDetail) -> ProbeEntry {
    ProbeEntry {
        probe: r.probe,
        node: r.node,
        port: r.port,
        mode: r.mode,
        availability: r.availability,
        generation: r.generation,
        clock: r.clock,
        last_status: r.last_status,
        counters: if detail == InspectionDetail::Verbose {
            Some(ProbeCounters {
                recorded: r.recorded,
                dropped: r.dropped,
                lost_blocks: r.lost_blocks,
            })
        } else {
            None
        },
    }
}

fn probe_entry_bytes(detail: InspectionDetail) -> usize {
    size_of::<ProbeEntry>()
        + if detail == InspectionDetail::Verbose {
            size_of::<ProbeCounters>()
        } else {
            0
        }
}
