//! Diagnostics engine — the non-RT aggregator that retains bounded
//! [`Evidence`]/[`DiagnosticFinding`]s, evaluates per-domain
//! [`HealthEvaluator`]s against collector [`MetricsSnapshot`]s, and
//! produces [`DiagnosticSnapshot`]s and [`DiagnosticReport`]s.
//!
//! # Contracts
//!
//! - **Logging OFF**: nothing here touches `tracing`, subscribers, or
//!   the `logging` feature — every operation works in the default
//!   build.
//! - **Non-RT only**: every public method allocates and/or locks
//!   (`Vec`/`VecDeque`/`BTreeMap`, `catch_unwind` around sink calls).
//!   Never call from an RT writer path.
//! - **Non-recursive self-observation**: [`DiagnosticsHealth`] is a
//!   fixed-size counter struct, never registered as a source — the
//!   engine cannot observe itself into unbounded recursion.
//! - **No domain-error duplication**: findings cite domain-owned
//!   [`DiagnosticCode`]s and [`EvidenceRef`]s; domain `Error`/`Result`
//!   types and registries like the plugin host's `DiagRegistry`
//!   remain authoritative.
//! - **Bounded**: evidence/findings evict FIFO past their caps;
//!   reports enforce findings/bytes/deadline limits with structured
//!   [`ReportTruncation`]; sink emissions obey [`SinkPolicy`].

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::time::Instant;

use crate::collector::MetricsSnapshot;
use crate::source::{Confidentiality, SourceIdentity};

use super::evidence::{Evidence, EvidenceEntry, EvidenceError, EvidenceRef};
use super::finding::{
    DiagnosticCode, DiagnosticFinding, FindingCertainty, FindingError, FindingSeverity,
    FindingStatus,
};
use super::health::{
    DomainEvaluation, HealthEvaluator, HealthRule, HealthState, RuleError, RuleInput,
    RuleTransition,
};
use super::report::{
    DiagnosticReport, DiagnosticSnapshot, DiagnosticsConfig, DiagnosticsConfigError, REPORT_NOTES,
    REPORT_SCHEMA_VERSION, ReportError, ReportLimits, ReportRequest, ReportTruncation,
    SnapshotConsistency, estimate_evidence_bytes, estimate_finding_bytes, estimate_health_bytes,
    exceeds_disclosure,
};
use super::sink::{DiagnosticsEvent, FindingSink};

/// Maximum evidence references attached to a transition-generated
/// finding — keeps findings explainable without unbounded growth.
const MAX_FINDING_REFS: usize = 4;

/// The diagnostics engine — own one per process/consumer.
pub struct Diagnostics {
    config: DiagnosticsConfig,
    evaluators: BTreeMap<&'static str, HealthEvaluator>,
    evidence: VecDeque<EvidenceEntry>,
    next_evidence_ref: u64,
    findings: VecDeque<DiagnosticFinding>,
    sink: Option<Box<dyn FindingSink>>,
    sink_window_start: Option<Instant>,
    sink_window_count: u32,
    last_domain_states: BTreeMap<&'static str, HealthState>,
    last_generation: u64,
    health: DiagnosticsHealth,
}

/// Fixed-size self-observation counters — never registered as a
/// source, so diagnostics cannot recurse into itself.
#[derive(Debug, Default, Clone)]
pub struct DiagnosticsHealth {
    /// Currently retained evidence entries.
    pub evidence_retained: usize,
    /// Evidence entries evicted by the retention bound, total.
    pub evidence_evicted_total: u64,
    /// Currently retained findings.
    pub findings_retained: usize,
    /// Findings evicted by the retention bound, total.
    pub findings_evicted_total: u64,
    /// `evaluate` calls completed.
    pub evaluations_total: u64,
    /// Confirmed rule-level transitions observed.
    pub rule_transitions_total: u64,
    /// Confirmed domain-level transitions observed.
    pub domain_transitions_total: u64,
    /// Events actually delivered to the sink.
    pub sink_emitted_total: u64,
    /// Events dropped by the sink suppression policy.
    pub sink_suppressed_total: u64,
    /// Sink `emit` calls that panicked (isolated via `catch_unwind`).
    pub sink_panics_total: u64,
    /// Reports successfully generated.
    pub reports_generated_total: u64,
    /// Report requests rejected as unauthorized.
    pub reports_rejected_total: u64,
}

impl Diagnostics {
    /// Creates an engine with `config`.
    ///
    /// # Errors
    ///
    /// [`DiagnosticsConfigError::ZeroSinkWindow`] when the sink
    /// policy window is zero.
    pub fn new(config: DiagnosticsConfig) -> Result<Self, DiagnosticsConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            evaluators: BTreeMap::new(),
            evidence: VecDeque::new(),
            next_evidence_ref: 0,
            findings: VecDeque::new(),
            sink: None,
            sink_window_start: None,
            sink_window_count: 0,
            last_domain_states: BTreeMap::new(),
            last_generation: 0,
            health: DiagnosticsHealth::default(),
        })
    }

    /// The engine configuration.
    #[must_use]
    pub const fn config(&self) -> &DiagnosticsConfig {
        &self.config
    }

    /// Records evidence, minting its [`EvidenceRef`].
    ///
    /// Retention is FIFO-bounded by `max_evidence`; overflow evicts
    /// the oldest entry and counts it. Evidence whose source later
    /// retires or re-registers is marked `superseded` at the next
    /// [`evaluate`](Self::evaluate) — never rebound.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::RetentionDisabled`] when `max_evidence == 0`.
    pub fn record_evidence(&mut self, evidence: Evidence) -> Result<EvidenceRef, EvidenceError> {
        if self.config.max_evidence == 0 {
            return Err(EvidenceError::RetentionDisabled);
        }
        let reference = EvidenceRef::mint(self.next_evidence_ref);
        self.next_evidence_ref += 1;
        self.evidence.push_back(EvidenceEntry {
            reference,
            evidence,
            superseded: false,
        });
        self.trim_evidence();
        Ok(reference)
    }

    fn trim_evidence(&mut self) {
        while self.evidence.len() > self.config.max_evidence {
            self.evidence.pop_front();
            self.health.evidence_evicted_total += 1;
        }
        self.health.evidence_retained = self.evidence.len();
    }

    /// Records a finding, upserting into an existing `Open` finding
    /// with the same `(domain, code)` — repeated observations merge
    /// rather than accumulate.
    ///
    /// # Errors
    ///
    /// [`FindingError::RetentionDisabled`] when `max_findings == 0`.
    pub fn record_finding(&mut self, finding: DiagnosticFinding) -> Result<(), FindingError> {
        if self.config.max_findings == 0 {
            return Err(FindingError::RetentionDisabled);
        }
        let merged = finding.status() == FindingStatus::Open
            && self
                .findings
                .iter_mut()
                .find(|f| {
                    f.status() == FindingStatus::Open
                        && f.domain() == finding.domain()
                        && f.code() == finding.code()
                })
                .is_some_and(|existing| {
                    existing.observe_again(&finding);
                    true
                });
        if !merged {
            self.findings.push_back(finding);
        }
        self.trim_findings();
        Ok(())
    }

    fn trim_findings(&mut self) {
        while self.findings.len() > self.config.max_findings {
            self.findings.pop_front();
            self.health.findings_evicted_total += 1;
        }
        self.health.findings_retained = self.findings.len();
    }

    /// Declares a health rule for `domain` (creating the domain's
    /// evaluator on first use).
    ///
    /// # Errors
    ///
    /// Propagates [`RuleError`] from rule validation.
    pub fn add_health_rule(
        &mut self,
        domain: &'static str,
        rule: HealthRule,
    ) -> Result<(), RuleError> {
        self.evaluators
            .entry(domain)
            .or_insert_with(|| HealthEvaluator::new(domain))
            .add_rule(rule)
    }

    /// Removes one rule from `domain`; returns whether it existed.
    pub fn remove_health_rule(&mut self, domain: &'static str, code: DiagnosticCode) -> bool {
        self.evaluators
            .get_mut(domain)
            .is_some_and(|e| e.remove_rule(code))
    }

    /// Removes a whole domain evaluator and its transition history;
    /// returns whether it existed.
    pub fn remove_domain(&mut self, domain: &'static str) -> bool {
        self.last_domain_states.remove(domain);
        self.evaluators.remove(domain).is_some()
    }

    /// Installs the event sink for important state changes.
    pub fn set_sink(&mut self, sink: Box<dyn FindingSink>) {
        self.sink = Some(sink);
    }

    /// Evaluates all declared health rules against `snapshot` at `now`,
    /// updates hysteresis/transition state, upserts transition
    /// findings, emits sink events under policy, refreshes evidence
    /// `superseded` marks, and returns the bounded
    /// [`DiagnosticSnapshot`].
    ///
    /// Non-RT only.
    pub fn evaluate(&mut self, snapshot: &MetricsSnapshot, now: Instant) -> DiagnosticSnapshot {
        self.health.evaluations_total += 1;
        self.last_generation = snapshot.generation();

        // Refresh superseded marks: evidence whose recorded source
        // identity is absent from this snapshot (retired, removed, or
        // superseded by a new generation) is marked — never rebound.
        let identities: HashSet<SourceIdentity> =
            snapshot.measurements().iter().map(|m| m.identity).collect();
        for entry in &mut self.evidence {
            entry.superseded = entry
                .evidence
                .source()
                .is_some_and(|s| !identities.contains(&s));
        }

        // Evaluate first (mutably borrows evaluators), then process
        // outcomes (needs &mut self for findings/sink/health).
        let mut evaluations = Vec::with_capacity(self.evaluators.len());
        for evaluator in self.evaluators.values_mut() {
            evaluations.push(evaluator.evaluate(snapshot.measurements(), now));
        }

        let mut domains = Vec::with_capacity(evaluations.len());
        let mut missing_rules = 0usize;
        for eval in evaluations {
            let domain = eval.health.domain;
            let prev = self
                .last_domain_states
                .get(domain)
                .copied()
                .unwrap_or(HealthState::Unknown);
            if prev != eval.health.state {
                self.health.domain_transitions_total += 1;
                self.last_domain_states.insert(domain, eval.health.state);
                self.maybe_emit(
                    DiagnosticsEvent::HealthTransition {
                        domain,
                        from: prev,
                        to: eval.health.state,
                        at: now,
                    },
                    now,
                );
            }
            self.health.rule_transitions_total += eval.transitions.len() as u64;
            for t in &eval.transitions {
                if t.to == HealthState::Healthy {
                    self.resolve_finding(domain, t.code, now);
                } else {
                    self.upsert_transition_finding(domain, t, &eval, now);
                }
            }
            missing_rules += eval
                .health
                .rules
                .iter()
                .filter(|r| matches!(r.input, RuleInput::Missing))
                .count();
            domains.push(eval.health);
        }

        DiagnosticSnapshot {
            generation: snapshot.generation(),
            taken_at: snapshot.taken_at(),
            consistency: SnapshotConsistency::IndependentReads,
            domains,
            findings: self.findings.iter().cloned().collect(),
            evidence: self.evidence.iter().cloned().collect(),
            missing_rules,
        }
    }

    /// Generates a [`DiagnosticReport`] under `request`'s disclosure.
    ///
    /// Rejects requests above `max_export_disclosure` with
    /// [`ReportError::Unauthorized`]; otherwise applies disclosure
    /// filtering (over-classified items are excluded and counted),
    /// severity ordering, and findings/bytes/deadline bounds with
    /// structured [`ReportTruncation`].
    ///
    /// Non-RT only.
    ///
    /// # Errors
    ///
    /// [`ReportError::Unauthorized`] when `request.disclosure` exceeds
    /// the configured export ceiling.
    pub fn report(
        &mut self,
        request: &ReportRequest,
        now: Instant,
    ) -> Result<DiagnosticReport, ReportError> {
        if exceeds_disclosure(request.disclosure, self.config.max_export_disclosure) {
            self.health.reports_rejected_total += 1;
            return Err(ReportError::Unauthorized {
                requested: request.disclosure,
                permitted: self.config.max_export_disclosure,
            });
        }
        self.health.reports_generated_total += 1;

        let limits = self.config.report;
        let start = Instant::now();
        let mut truncation = ReportTruncation::default();

        // Findings: disclosure filter → stable severity-desc sort →
        // count cap. `max_findings == 0` means unlimited (retention
        // still bounds storage).
        let mut findings: Vec<DiagnosticFinding> = self
            .findings
            .iter()
            .filter(|f| {
                if exceeds_disclosure(f.confidentiality(), request.disclosure) {
                    truncation.withheld_confidential += 1;
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        findings.sort_by_key(|f| Reverse(f.severity().rank()));
        if limits.max_findings > 0 && findings.len() > limits.max_findings {
            truncation.findings_dropped = findings.len() - limits.max_findings;
            findings.truncate(limits.max_findings);
        }

        let mut bytes = findings.iter().map(estimate_finding_bytes).sum::<usize>();
        let evidence = self.collect_report_evidence(
            request.disclosure,
            &limits,
            start,
            &mut bytes,
            &mut truncation,
        );
        let health = self.collect_report_health(
            request.disclosure,
            &limits,
            start,
            &mut bytes,
            &mut truncation,
        );

        let included: HashSet<EvidenceRef> = evidence.iter().map(|e| e.reference).collect();
        truncation.unresolved_evidence_refs = findings
            .iter()
            .flat_map(|f| f.evidence_refs().iter())
            .filter(|r| !included.contains(r))
            .count();
        truncation.bytes_estimated = bytes;

        Ok(DiagnosticReport {
            schema_version: REPORT_SCHEMA_VERSION,
            generated_at: now,
            source_generation: self.last_generation,
            disclosure: request.disclosure,
            findings,
            evidence,
            health,
            truncation,
            notes: REPORT_NOTES.to_vec(),
        })
    }

    /// Evidence entries newest-first under the disclosure and
    /// byte/deadline budget; `bytes` carries the running total shared
    /// with the other report sections.
    fn collect_report_evidence(
        &self,
        disclosure: Confidentiality,
        limits: &ReportLimits,
        start: Instant,
        bytes: &mut usize,
        truncation: &mut ReportTruncation,
    ) -> Vec<EvidenceEntry> {
        let mut evidence = Vec::new();
        for entry in self.evidence.iter().rev() {
            if truncation.deadline_exceeded {
                truncation.evidence_dropped += 1;
                continue;
            }
            if exceeds_disclosure(entry.evidence.confidentiality(), disclosure) {
                truncation.withheld_confidential += 1;
                continue;
            }
            let cost = estimate_evidence_bytes(entry);
            if limits.max_bytes > 0 && bytes.saturating_add(cost) > limits.max_bytes {
                truncation.evidence_dropped += 1;
                continue;
            }
            if limits.deadline.is_some_and(|d| start.elapsed() >= d) {
                truncation.deadline_exceeded = true;
                truncation.evidence_dropped += 1;
                continue;
            }
            *bytes += cost;
            evidence.push(entry.clone());
        }
        evidence
    }

    /// Per-domain health under the same budget rules as evidence.
    fn collect_report_health(
        &self,
        disclosure: Confidentiality,
        limits: &ReportLimits,
        start: Instant,
        bytes: &mut usize,
        truncation: &mut ReportTruncation,
    ) -> Vec<super::health::DomainHealth> {
        let mut health = Vec::new();
        for evaluator in self.evaluators.values() {
            let h = evaluator.current_health();
            if truncation.deadline_exceeded {
                truncation.health_dropped += 1;
                continue;
            }
            if exceeds_disclosure(h.confidentiality, disclosure) {
                truncation.withheld_confidential += 1;
                continue;
            }
            let cost = estimate_health_bytes(&h);
            if limits.max_bytes > 0 && bytes.saturating_add(cost) > limits.max_bytes {
                truncation.health_dropped += 1;
                continue;
            }
            if limits.deadline.is_some_and(|d| start.elapsed() >= d) {
                truncation.deadline_exceeded = true;
                truncation.health_dropped += 1;
                continue;
            }
            *bytes += cost;
            health.push(h);
        }
        health
    }

    /// Fixed-size self-observation counters.
    #[must_use]
    pub fn health(&self) -> DiagnosticsHealth {
        self.health.clone()
    }

    /// Number of retained evidence entries.
    #[must_use]
    pub fn evidence_len(&self) -> usize {
        self.evidence.len()
    }

    /// Whether no evidence is retained.
    #[must_use]
    pub fn evidence_is_empty(&self) -> bool {
        self.evidence.is_empty()
    }

    /// Number of retained findings.
    #[must_use]
    pub fn findings_len(&self) -> usize {
        self.findings.len()
    }

    /// Whether no findings are retained.
    #[must_use]
    pub fn findings_is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// Emits a sink event under the leaky-bucket policy, isolating
    /// sink panics.
    fn maybe_emit(&mut self, event: DiagnosticsEvent, now: Instant) {
        if self.sink.is_none() {
            return;
        }
        let window = self.config.sink_policy.window;
        match self.sink_window_start {
            Some(start) if now.saturating_duration_since(start) < window => {}
            _ => {
                self.sink_window_start = Some(now);
                self.sink_window_count = 0;
            }
        }
        if self.sink_window_count >= self.config.sink_policy.max_events {
            self.health.sink_suppressed_total += 1;
            return;
        }
        self.sink_window_count += 1;
        // The sink is observability infrastructure — a panic inside it
        // must not take the evaluation down, and diagnostics never
        // recursively reports its own failure.
        if let Some(sink) = self.sink.as_mut() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                sink.emit(event);
            }));
            match result {
                Ok(()) => self.health.sink_emitted_total += 1,
                Err(_) => self.health.sink_panics_total += 1,
            }
        }
    }

    /// Evidence refs explaining a transition — newest entries whose
    /// subject matches the rule's metric key and whose source matches
    /// when the transition carries one.
    fn transition_evidence_refs(
        &self,
        source: Option<SourceIdentity>,
        metric_key: &'static str,
    ) -> Box<[EvidenceRef]> {
        self.evidence
            .iter()
            .rev()
            .filter(|e| {
                e.evidence.subject() == Some(metric_key)
                    && source.is_none_or(|s| e.evidence.source() == Some(s))
            })
            .take(MAX_FINDING_REFS)
            .map(|e| e.reference)
            .collect()
    }

    /// Upserts a finding for a confirmed non-`Healthy` rule
    /// transition.
    fn upsert_transition_finding(
        &mut self,
        domain: &'static str,
        t: &RuleTransition,
        eval: &DomainEvaluation,
        now: Instant,
    ) {
        let severity = match t.to {
            HealthState::Degraded => FindingSeverity::Warning,
            HealthState::Unhealthy => FindingSeverity::Critical,
            HealthState::Unknown => FindingSeverity::Info,
            HealthState::Healthy => return,
        };
        let certainty = if t.to == HealthState::Unknown {
            FindingCertainty::NotEvaluable
        } else {
            FindingCertainty::Confirmed
        };
        let confidentiality = eval
            .health
            .rules
            .iter()
            .find(|r| r.code == t.code)
            .map_or(Confidentiality::Internal, |r| r.confidentiality);
        let refs = self.transition_evidence_refs(t.source, t.metric_key);
        let summary = format!("{} {:?} -> {:?}", t.metric_key, t.from, t.to);
        if let Ok(finding) = DiagnosticFinding::new(
            t.code,
            domain,
            t.source,
            severity,
            certainty,
            confidentiality,
            summary,
            refs,
            now,
        ) {
            let _ = self.record_finding(finding);
        }
    }

    /// Resolves the open finding for a rule that recovered to
    /// `Healthy`.
    fn resolve_finding(&mut self, domain: &'static str, code: DiagnosticCode, now: Instant) {
        if let Some(f) = self
            .findings
            .iter_mut()
            .find(|f| f.domain() == domain && f.code() == code && f.status() == FindingStatus::Open)
        {
            f.resolve(now);
        }
    }
}
