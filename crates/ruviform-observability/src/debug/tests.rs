//! Contract tests for `debug` — Feature/Rule/Example coverage per
//! `docs/test-specs/issue-292-observability-debug-inspection-tests.md`.
//!
//! These tests run in the default feature set (`logging` OFF): every
//! debug API must work without any subscriber or logging backend.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ruviform_media_core::{ClockDomainId, ClockEpochId};

use super::*;
use crate::collector::{MetricsSnapshot, ObservationBatch, ObservationWindow};
use crate::diagnostics::{
    DiagnosticCode, Diagnostics, DiagnosticsConfig, FindingCertainty, FindingSeverity, HealthRule,
    HealthState, Hysteresis, ReportRequest, SignalDirection, SignalKind,
};
use crate::metrics::{Measurement, MeasurementValidity, MetricValue};
use crate::record::{EventId, EventRecord, RecordHeader};
use crate::rt_bridge;
use crate::source::{
    AggregationPolicy, Confidentiality, MetricDescriptor, MetricKind, ObservationScope,
    ObservationUnit, ReportingHint, SourceGeneration, SourceId, SourceIdentity, SourceLifecycle,
    SourceSemantics,
};

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn ident(id: u64, generation: u64) -> SourceIdentity {
    SourceIdentity::new(
        SourceId::new(id).expect("non-zero id"),
        SourceGeneration::new(generation),
    )
}

fn descriptor(key: &'static str, confidentiality: Confidentiality) -> MetricDescriptor {
    MetricDescriptor {
        kind: MetricKind::Gauge,
        key,
        unit: ObservationUnit::None,
        reporting: ReportingHint::SnapshotOnly,
        aggregation: AggregationPolicy::LatestValue,
        buckets: None,
        semantics: SourceSemantics {
            confidentiality,
            ..SourceSemantics::default()
        },
    }
}

fn measurement(
    key: &'static str,
    identity: SourceIdentity,
    confidentiality: Confidentiality,
    validity: MeasurementValidity,
) -> Measurement {
    Measurement {
        identity,
        descriptor: descriptor(key, confidentiality),
        scope: ObservationScope::Instance,
        lifecycle: SourceLifecycle::Active,
        observed_at: Instant::now(),
        last_change_at: Instant::now(),
        validity,
        issue: None,
        loss: None,
        value: Some(MetricValue::Gauge {
            value: 7,
            previous: None,
        }),
    }
}

fn snapshot_of(generation: u64, measurements: Vec<Measurement>) -> MetricsSnapshot {
    MetricsSnapshot::new(generation, Instant::now(), measurements)
}

fn code(name: &'static str) -> DiagnosticCode {
    DiagnosticCode {
        name,
        schema_version: 1,
    }
}

fn request(targets: &[InspectionTarget]) -> InspectionRequest {
    InspectionRequest {
        targets: targets.to_vec(),
        detail: InspectionDetail::Standard,
        disclosure: Confidentiality::Internal,
        budget: InspectionBudget {
            max_items: 64,
            max_bytes: 65_536,
            timeout: None,
        },
    }
}

fn no_inputs<'a>() -> InspectionInputs<'a> {
    InspectionInputs::default()
}

fn section(snapshot: &InspectionSnapshot, target: InspectionTarget) -> &InspectionSection {
    snapshot
        .sections
        .iter()
        .find(|s| s.target() == target)
        .expect("section present")
}

fn metrics_section(snapshot: &InspectionSnapshot) -> &MetricsInspection {
    let InspectionSection::Metrics(m) = section(snapshot, InspectionTarget::Metrics) else {
        panic!("metrics section")
    };
    m
}

fn diagnostics_section(snapshot: &InspectionSnapshot) -> &DiagnosticsInspection {
    let InspectionSection::Diagnostics(d) = section(snapshot, InspectionTarget::Diagnostics) else {
        panic!("diagnostics section")
    };
    d
}

fn invariants_section(snapshot: &InspectionSnapshot) -> &InvariantsInspection {
    let InspectionSection::Invariants(i) = section(snapshot, InspectionTarget::Invariants) else {
        panic!("invariants section")
    };
    i
}

fn session_section(snapshot: &InspectionSnapshot) -> &SessionInspection {
    let InspectionSection::Session(s) = section(snapshot, InspectionTarget::Session) else {
        panic!("session section")
    };
    s
}

fn probes_section(snapshot: &InspectionSnapshot) -> &ProbesInspection {
    let InspectionSection::Probes(p) = section(snapshot, InspectionTarget::Probes) else {
        panic!("probes section")
    };
    p
}

fn diagnostics_report(generation: u64) -> crate::diagnostics::DiagnosticReport {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).expect("config");
    d.evaluate(&snapshot_of(generation, Vec::new()), Instant::now());
    d.report(
        &ReportRequest {
            disclosure: Confidentiality::Internal,
        },
        Instant::now(),
    )
    .expect("report")
}

// ------------------------------------------------------------------
// Invariant check fixtures
// ------------------------------------------------------------------

struct FixedCheck {
    domain: &'static str,
    code: DiagnosticCode,
    verdict: InvariantVerdict,
}

impl InvariantCheck for FixedCheck {
    fn domain(&self) -> &'static str {
        self.domain
    }

    fn code(&self) -> DiagnosticCode {
        self.code
    }

    fn evaluate(&mut self, _ctx: &InspectionContext<'_>) -> InvariantVerdict {
        self.verdict.clone()
    }
}

struct PanicCheck;

impl InvariantCheck for PanicCheck {
    fn domain(&self) -> &'static str {
        "test.panic"
    }

    fn code(&self) -> DiagnosticCode {
        code("test.panic")
    }

    fn evaluate(&mut self, _ctx: &InspectionContext<'_>) -> InvariantVerdict {
        panic!("invariant check exploded");
    }
}

struct SlowCheck;

impl InvariantCheck for SlowCheck {
    fn domain(&self) -> &'static str {
        "test.slow"
    }

    fn code(&self) -> DiagnosticCode {
        code("test.slow")
    }

    fn evaluate(&mut self, _ctx: &InspectionContext<'_>) -> InvariantVerdict {
        std::thread::sleep(Duration::from_millis(30));
        InvariantVerdict::satisfied("slow but satisfied")
    }
}

fn fixed_check(verdict: InvariantVerdict) -> Box<dyn InvariantCheck> {
    fixed_check_named("test.invariant", verdict)
}

fn fixed_check_named(name: &'static str, verdict: InvariantVerdict) -> Box<dyn InvariantCheck> {
    Box::new(FixedCheck {
        domain: "test.domain",
        code: code(name),
        verdict,
    })
}

// ------------------------------------------------------------------
// Probe fixtures
// ------------------------------------------------------------------

fn known_clock() -> ProbeClockMark {
    ProbeClockMark {
        domain: ClockDomainId::new(1),
        epoch: ClockEpochId::new(0),
    }
}

fn reading(probe: u64, generation: u64, clock: ProbeClockMark) -> ProbeReading {
    ProbeReading {
        probe,
        node: 1,
        port: 2,
        mode: ProbeMode::Monitor,
        availability: ProbeAvailability::Configured,
        generation,
        clock,
        last_status: None,
        recorded: 3,
        dropped: 1,
        lost_blocks: 0,
    }
}

fn deck(
    generation: u64,
    clock: ProbeClockMark,
    probes: Vec<ProbeReading>,
    confidentiality: Confidentiality,
) -> ProbeDeckSnapshot {
    ProbeDeckSnapshot {
        generation,
        clock,
        confidentiality,
        probes,
    }
}

struct FakeProbes(Vec<ProbeDeckSnapshot>);

impl ProbeSource for FakeProbes {
    fn decks(&mut self) -> Vec<ProbeDeckSnapshot> {
        self.0.clone()
    }
}

struct PanicProbes;

impl ProbeSource for PanicProbes {
    fn decks(&mut self) -> Vec<ProbeDeckSnapshot> {
        panic!("probe source exploded");
    }
}

// ------------------------------------------------------------------
// Session fixtures
// ------------------------------------------------------------------

struct MockDrain {
    quiesced: bool,
    in_flight: u64,
    records: VecDeque<SessionRecord>,
}

impl SessionDrain for MockDrain {
    fn writer_quiesced(&self) -> bool {
        self.quiesced
    }

    fn in_flight(&self) -> u64 {
        self.in_flight
    }

    fn drain_final(&mut self, emit: &mut dyn FnMut(SessionRecord)) -> DrainReport {
        let mut delivered = 0usize;
        while let Some(r) = self.records.pop_front() {
            emit(r);
            delivered += 1;
        }
        DrainReport {
            delivered,
            dropped_total: 0,
            lag: self.in_flight,
        }
    }
}

fn event(confidentiality: Confidentiality) -> EventRecord {
    EventRecord::new(
        RecordHeader::new(EventId::new(7), 1, ident(1, 1))
            .with_generation(3)
            .with_confidentiality(confidentiality),
        42,
    )
}

fn session_record(confidentiality: Confidentiality) -> SessionRecord {
    SessionRecord {
        record: event(confidentiality),
        dropped_before: 0,
    }
}

fn report_request(disclosure: Confidentiality) -> DebugReportRequest {
    DebugReportRequest {
        disclosure,
        external: false,
        limits: ExportLimits::default(),
    }
}

// ------------------------------------------------------------------
// Metrics section
// ------------------------------------------------------------------

#[test]
fn metrics_section_entries_and_aggregates() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let ms = [
        measurement(
            "k.valid",
            ident(1, 1),
            Confidentiality::Internal,
            MeasurementValidity::Valid,
        ),
        measurement(
            "k.stale",
            ident(2, 1),
            Confidentiality::Internal,
            MeasurementValidity::Stale,
        ),
        measurement(
            "k.disc",
            ident(3, 1),
            Confidentiality::Internal,
            MeasurementValidity::Discontinuous,
        ),
    ];
    let snap = snapshot_of(9, ms.to_vec());
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics]),
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert_eq!(m.status, SectionStatus::Ok);
    assert_eq!(m.generation, 9);
    assert_eq!(m.total_series, 3);
    assert_eq!(m.validity_counts.valid, 1);
    assert_eq!(m.validity_counts.stale, 1);
    assert_eq!(m.validity_counts.discontinuous, 1);
    assert_eq!(m.entries.len(), 3);
    // Standard detail carries no payload values.
    assert!(m.entries.iter().all(|e| e.value.is_none()));
    assert_eq!(out.consistency.collector.snapshot, Some(9));
    assert_eq!(out.consistency.level, ConsistencyLevel::Aligned);
}

#[test]
fn metrics_verbose_detail_carries_values() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(
        1,
        vec![measurement(
            "k.g",
            ident(1, 1),
            Confidentiality::Internal,
            MeasurementValidity::Valid,
        )],
    );
    let mut req = request(&[InspectionTarget::Metrics]);
    req.detail = InspectionDetail::Verbose;
    let out = insp
        .inspect(
            &req,
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert!(m.entries[0].value.is_some());
}

#[test]
fn metrics_summary_detail_has_no_entries() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(
        1,
        vec![
            measurement(
                "k.a",
                ident(1, 1),
                Confidentiality::Internal,
                MeasurementValidity::Valid,
            ),
            measurement(
                "k.b",
                ident(2, 1),
                Confidentiality::Internal,
                MeasurementValidity::Valid,
            ),
        ],
    );
    let mut req = request(&[InspectionTarget::Metrics]);
    req.detail = InspectionDetail::Summary;
    let out = insp
        .inspect(
            &req,
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert_eq!(m.status, SectionStatus::Ok);
    assert_eq!(m.total_series, 2);
    assert!(m.entries.is_empty());
    assert_eq!(out.truncation.items_dropped, 0);
}

#[test]
fn missing_inputs_report_unavailable() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let out = insp
        .inspect(
            &request(&[
                InspectionTarget::Metrics,
                InspectionTarget::Diagnostics,
                InspectionTarget::Probes,
            ]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    assert!(matches!(
        metrics_section(&out).status,
        SectionStatus::Unavailable {
            reason: UnavailableReason::InputNotProvided
        }
    ));
    assert!(matches!(
        diagnostics_section(&out).status,
        SectionStatus::Unavailable {
            reason: UnavailableReason::InputNotProvided
        }
    ));
    assert!(matches!(
        probes_section(&out).status,
        SectionStatus::Unavailable {
            reason: UnavailableReason::InputNotProvided
        }
    ));
    assert_eq!(out.truncation.sections_unavailable, 3);
}

// ------------------------------------------------------------------
// Budget bounds
// ------------------------------------------------------------------

#[test]
fn max_items_truncates_entries_observably() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let ms: Vec<Measurement> = (1..=3u64)
        .map(|i| {
            measurement(
                "k.x",
                ident(i, 1),
                Confidentiality::Internal,
                MeasurementValidity::Valid,
            )
        })
        .collect();
    let snap = snapshot_of(1, ms);
    let mut req = request(&[InspectionTarget::Metrics]);
    req.budget.max_items = 1;
    let out = insp
        .inspect(
            &req,
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert_eq!(m.entries.len(), 1);
    assert_eq!(m.total_series, 3);
    assert_eq!(m.status, SectionStatus::Truncated);
    assert_eq!(out.truncation.items_dropped, 2);
    assert_eq!(insp.health().items_truncated_total, 2);
}

#[test]
fn max_bytes_truncates_entries_observably() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let ms: Vec<Measurement> = (1..=4u64)
        .map(|i| {
            measurement(
                "k.x",
                ident(i, 1),
                Confidentiality::Internal,
                MeasurementValidity::Valid,
            )
        })
        .collect();
    let snap = snapshot_of(1, ms);
    let mut req = request(&[InspectionTarget::Metrics]);
    // Enough for roughly one entry.
    req.budget.max_bytes = size_of::<MetricEntry>() + 8;
    let out = insp
        .inspect(
            &req,
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert!(m.entries.len() < 4);
    assert!(out.truncation.items_dropped > 0);
    assert!(out.truncation.bytes_estimated > 0);
    assert_eq!(m.status, SectionStatus::Truncated);
}

#[test]
fn max_items_zero_is_deliberate_not_truncation() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(
        1,
        vec![measurement(
            "k.a",
            ident(1, 1),
            Confidentiality::Internal,
            MeasurementValidity::Valid,
        )],
    );
    let mut req = request(&[InspectionTarget::Metrics]);
    req.budget.max_items = 0;
    let out = insp
        .inspect(
            &req,
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert!(m.entries.is_empty());
    assert_eq!(m.total_series, 1);
    assert_eq!(m.status, SectionStatus::Ok);
    assert_eq!(out.truncation.items_dropped, 0);
}

#[test]
fn zero_timeout_marks_all_sections_unavailable() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let mut req = request(&[
        InspectionTarget::Metrics,
        InspectionTarget::Invariants,
        InspectionTarget::Session,
    ]);
    req.budget.timeout = Some(Duration::ZERO);
    let out = insp
        .inspect(&req, &no_inputs(), Instant::now())
        .expect("inspect");
    for s in &out.sections {
        assert!(
            matches!(
                s.status(),
                SectionStatus::Unavailable {
                    reason: UnavailableReason::DeadlineExceeded
                }
            ),
            "section {:?} should be deadline-unavailable",
            s.target()
        );
    }
    assert!(out.truncation.deadline_exceeded);
    assert_eq!(out.truncation.sections_unavailable, 3);
}

#[test]
fn deadline_mid_collection_truncates_invariants() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_invariant_check(Box::new(SlowCheck));
    insp.add_invariant_check(fixed_check(InvariantVerdict::satisfied("fast")));
    let mut req = request(&[InspectionTarget::Invariants]);
    req.budget.timeout = Some(Duration::from_millis(5));
    let out = insp
        .inspect(&req, &no_inputs(), Instant::now())
        .expect("inspect");
    let inv = invariants_section(&out);
    // The slow check finished before the deadline was re-checked;
    // the second check was never evaluated.
    assert_eq!(inv.checks_run, 1);
    assert_eq!(inv.status, SectionStatus::Truncated);
    assert!(out.truncation.deadline_exceeded);
    assert_eq!(out.truncation.items_dropped, 1);
}

// ------------------------------------------------------------------
// Disclosure / authorization
// ------------------------------------------------------------------

#[test]
fn confidential_items_are_withheld_not_leaked() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(
        1,
        vec![
            measurement(
                "k.pub",
                ident(1, 1),
                Confidentiality::Internal,
                MeasurementValidity::Valid,
            ),
            measurement(
                "k.sec",
                ident(2, 1),
                Confidentiality::Confidential,
                MeasurementValidity::Valid,
            ),
        ],
    );
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics]),
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let m = metrics_section(&out);
    assert_eq!(m.entries.len(), 1);
    assert_eq!(m.total_series, 1);
    assert_eq!(out.truncation.withheld_confidential, 1);
    assert_eq!(insp.health().withheld_confidential_total, 1);
}

#[test]
fn all_withheld_section_reports_withheld() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(
        1,
        vec![measurement(
            "k.sec",
            ident(1, 1),
            Confidentiality::Confidential,
            MeasurementValidity::Valid,
        )],
    );
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics]),
            &InspectionInputs {
                metrics: Some(&snap),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    assert_eq!(metrics_section(&out).status, SectionStatus::Withheld);
}

#[test]
fn disclosure_above_ceiling_is_unauthorized() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let mut req = request(&[InspectionTarget::Session]);
    req.disclosure = Confidentiality::Confidential;
    let err = insp
        .inspect(&req, &no_inputs(), Instant::now())
        .unwrap_err();
    assert!(matches!(
        err,
        InspectionError::Unauthorized {
            requested: Confidentiality::Confidential,
            permitted: Confidentiality::Internal,
        }
    ));
    assert_eq!(insp.health().inspection_errors_total, 1);
    assert_eq!(insp.health().inspections_total, 0);
}

// ------------------------------------------------------------------
// Generation / clock consistency
// ------------------------------------------------------------------

#[test]
fn generation_skew_is_declared_not_hidden() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let snap = snapshot_of(1, Vec::new());
    let diag = diagnostics_report(2);
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics, InspectionTarget::Diagnostics]),
            &InspectionInputs {
                metrics: Some(&snap),
                diagnostics: Some(&diag),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    assert_eq!(out.consistency.level, ConsistencyLevel::Skewed);
    assert!(!out.consistency.collector.aligned);
    assert_eq!(out.consistency.collector.snapshot, Some(1));
    assert_eq!(out.consistency.collector.diagnostics, Some(2));
}

#[test]
fn unknown_probe_clock_marks_consistency_unknown() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_probe_source(Box::new(FakeProbes(vec![deck(
        5,
        ProbeClockMark::UNKNOWN,
        vec![reading(1, 5, ProbeClockMark::UNKNOWN)],
        Confidentiality::Internal,
    )])));
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Probes]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    let p = probes_section(&out);
    assert_eq!(p.decks.len(), 1);
    assert!(!p.decks[0].clock_known);
    assert!(!p.decks[0].aligned);
    assert_eq!(out.consistency.level, ConsistencyLevel::Unknown);
    assert_eq!(out.consistency.probe_decks.len(), 1);
    assert!(!out.consistency.probe_decks[0].clock_known);
}

#[test]
fn probe_generation_mismatch_is_counted() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_probe_source(Box::new(FakeProbes(vec![deck(
        5,
        known_clock(),
        vec![
            reading(1, 5, known_clock()),
            reading(2, 4, known_clock()), // stale generation
        ],
        Confidentiality::Internal,
    )])));
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Probes]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    let p = probes_section(&out);
    assert_eq!(p.decks[0].mismatched, 1);
    assert!(!p.decks[0].aligned);
    assert_eq!(out.consistency.level, ConsistencyLevel::Skewed);
    assert_eq!(out.consistency.probe_decks[0].mismatched, 1);
}

#[test]
fn probe_source_failure_is_unavailable_not_fabricated() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_probe_source(Box::new(PanicProbes));
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Probes]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    assert!(matches!(
        probes_section(&out).status,
        SectionStatus::Unavailable {
            reason: UnavailableReason::PortFailure
        }
    ));
    assert_eq!(insp.health().probe_source_failures_total, 1);
}

#[test]
fn probe_verbose_counters_and_confidential_deck_withheld() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_probe_source(Box::new(FakeProbes(vec![
        deck(
            5,
            known_clock(),
            vec![reading(1, 5, known_clock())],
            Confidentiality::Internal,
        ),
        deck(
            5,
            known_clock(),
            vec![reading(2, 5, known_clock())],
            Confidentiality::Confidential,
        ),
    ])));
    let mut req = request(&[InspectionTarget::Probes]);
    req.detail = InspectionDetail::Verbose;
    let out = insp
        .inspect(&req, &no_inputs(), Instant::now())
        .expect("inspect");
    let p = probes_section(&out);
    assert_eq!(p.decks.len(), 1);
    assert!(p.decks[0].probes[0].counters.is_some());
    assert_eq!(out.truncation.withheld_confidential, 1);
}

// ------------------------------------------------------------------
// Invariants
// ------------------------------------------------------------------

#[test]
fn invariant_three_valued_outcomes() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_invariant_check(fixed_check(InvariantVerdict::satisfied("ok")));
    insp.add_invariant_check(fixed_check(InvariantVerdict::violated(
        FindingSeverity::Critical,
        "broken",
    )));
    insp.add_invariant_check(fixed_check(InvariantVerdict::inconclusive(
        InconclusiveReason::MissingObservation,
        "no data",
    )));
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Invariants]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    let inv = invariants_section(&out);
    assert_eq!(inv.checks_run, 3);
    assert_eq!(inv.summary.satisfied, 1);
    assert_eq!(inv.summary.violated, 1);
    assert_eq!(inv.summary.inconclusive, 1);
    assert_eq!(inv.results.len(), 3);
    let inconclusive = inv
        .results
        .iter()
        .find(|r| r.outcome == InvariantOutcome::Inconclusive)
        .expect("inconclusive result");
    assert_eq!(
        inconclusive.inconclusive,
        Some(InconclusiveReason::MissingObservation)
    );
    assert_eq!(out.consistency.level, ConsistencyLevel::Aligned);
}

#[test]
fn invariant_panic_is_isolated_as_inconclusive() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_invariant_check(Box::new(PanicCheck));
    insp.add_invariant_check(fixed_check(InvariantVerdict::satisfied("after panic")));
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Invariants]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    let inv = invariants_section(&out);
    assert_eq!(inv.checks_run, 2);
    assert_eq!(inv.summary.inconclusive, 1);
    assert_eq!(inv.summary.satisfied, 1);
    assert_eq!(
        inv.results[0].inconclusive,
        Some(InconclusiveReason::CheckPanicked)
    );
    assert_eq!(insp.health().invariant_panics_total, 1);
}

#[test]
fn publish_invariants_maps_to_diagnostics() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_invariant_check(fixed_check_named(
        "test.satisfied",
        InvariantVerdict::satisfied("fine"),
    ));
    insp.add_invariant_check(fixed_check_named(
        "test.violated",
        InvariantVerdict::violated(FindingSeverity::Warning, "invariant broken"),
    ));
    insp.add_invariant_check(fixed_check_named(
        "test.inconclusive",
        InvariantVerdict::inconclusive(InconclusiveReason::CheckDeclined, "cannot tell"),
    ));
    let now = Instant::now();
    let out = insp
        .inspect(&request(&[InspectionTarget::Invariants]), &no_inputs(), now)
        .expect("inspect");
    let mut diag = Diagnostics::new(DiagnosticsConfig::default()).expect("config");
    let pub_report = insp.publish_invariants(&mut diag, &out, now);
    assert_eq!(pub_report.results_examined, 3);
    // Every evaluated check produced evidence; only
    // Violated/Inconclusive produced findings.
    assert_eq!(pub_report.evidence_recorded, 3);
    assert_eq!(pub_report.findings_recorded, 2);
    assert!(!pub_report.skipped_no_section);
    let report = diag
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            now,
        )
        .expect("report");
    assert_eq!(report.findings.len(), 2);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.certainty() == FindingCertainty::Confirmed)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.certainty() == FindingCertainty::NotEvaluable)
    );
    assert_eq!(insp.health().findings_published_total, 2);
    assert_eq!(insp.health().evidence_published_total, 3);
}

#[test]
fn publish_without_invariants_section_is_honest_skip() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Session]),
            &no_inputs(),
            Instant::now(),
        )
        .expect("inspect");
    let mut diag = Diagnostics::new(DiagnosticsConfig::default()).expect("config");
    let pub_report = insp.publish_invariants(&mut diag, &out, Instant::now());
    assert!(pub_report.skipped_no_section);
    assert_eq!(pub_report.results_examined, 0);
}

// ------------------------------------------------------------------
// Session lifecycle
// ------------------------------------------------------------------

#[test]
fn session_requires_authority_and_permit() {
    let now = Instant::now();
    // No authority installed.
    let mut no_auth = DebugInspector::new(DebugConfig::default());
    let foreign_authority = DebugAuthority::new();
    let foreign_permit = foreign_authority.permit().expect("permit");
    let err = no_auth
        .prepare_session(
            foreign_permit,
            SessionRequest {
                label: "t",
                max_records: 4,
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, SessionError::NoAuthority);

    // Permit minted by a different authority is denied.
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let other = DebugAuthority::new();
    let other_permit = other.permit().expect("permit");
    let err = insp
        .prepare_session(
            other_permit,
            SessionRequest {
                label: "t",
                max_records: 4,
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, SessionError::PermitDenied);
}

#[test]
fn session_full_lifecycle_with_drain() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();

    insp.prepare_session(
        authority.permit().expect("permit"),
        SessionRequest {
            label: "test.session",
            max_records: 4,
        },
        now,
    )
    .expect("prepare");
    assert_eq!(insp.session_state(), DebugSessionState::Prepared);

    // Activate without drains is rejected.
    let err = insp.activate_session(now).unwrap_err();
    assert_eq!(err, SessionError::NoDrains,);

    let mut records = VecDeque::new();
    records.push_back(session_record(Confidentiality::Internal));
    records.push_back(session_record(Confidentiality::Internal));
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    assert_eq!(insp.session_state(), DebugSessionState::Active);

    insp.request_stop(authority.permit().expect("permit"), now)
        .expect("stop");
    assert_eq!(insp.session_state(), DebugSessionState::Draining);

    let status = insp.poll_drain(now);
    assert!(matches!(
        status,
        SessionDrainStatus::Stopped {
            clean: true,
            records_collected: 2,
            records_overflowed: 0,
        }
    ));
    assert_eq!(insp.session_state(), DebugSessionState::Stopped);
    assert_eq!(insp.drained_records().len(), 2);
    let view = insp.session_view();
    assert_eq!(view.generation, 1);
    assert_eq!(view.label, Some("test.session"));

    let health = insp.health();
    assert_eq!(health.sessions_prepared_total, 1);
    assert_eq!(health.sessions_activated_total, 1);
    assert_eq!(health.sessions_stopped_total, 1);
    assert_eq!(health.drain_records_delivered_total, 2);

    // Re-prepare after Stopped mints a new generation.
    insp.prepare_session(
        authority.permit().expect("permit"),
        SessionRequest {
            label: "test.session",
            max_records: 4,
        },
        now,
    )
    .expect("re-prepare");
    assert_eq!(insp.session_view().generation, 2);
}

#[test]
fn session_busy_and_invalid_transitions_are_typed() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    let req = SessionRequest {
        label: "t",
        max_records: 4,
    };
    insp.prepare_session(authority.permit().expect("p"), req, now)
        .expect("prepare");
    // Second prepare while Prepared → SessionBusy.
    let err = insp
        .prepare_session(authority.permit().expect("p"), req, now)
        .unwrap_err();
    assert!(matches!(err, SessionError::SessionBusy { .. }));
    // request_stop from Prepared → Draining is allowed; then stop
    // again → InvalidTransition.
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let err = insp
        .request_stop(authority.permit().expect("p"), now)
        .unwrap_err();
    assert!(matches!(err, SessionError::InvalidTransition { .. }));
    // max_records == 0 rejected at prepare — but session is
    // Draining now, so first return to Disabled via abandon.
    insp.abandon_session(authority.permit().expect("p"), now)
        .expect("abandon");
    let err = insp
        .prepare_session(
            authority.permit().expect("p"),
            SessionRequest {
                label: "t",
                max_records: 0,
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, SessionError::InvalidRequest);
}

#[test]
fn abandon_is_counted_and_reported_never_silent() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 4,
        },
        now,
    )
    .expect("prepare");
    insp.abandon_session(authority.permit().expect("p"), now)
        .expect("abandon");
    assert_eq!(insp.session_state(), DebugSessionState::Stopped);
    assert!(insp.session_view().abandoned);
    assert_eq!(insp.health().sessions_abandoned_total, 1);
    // Abandon on Disabled is an invalid transition.
    let err = insp
        .abandon_session(authority.permit().expect("p"), now)
        .unwrap_err();
    assert!(matches!(err, SessionError::InvalidTransition { .. }));
}

#[test]
fn drain_timeout_reports_but_never_forces_stop() {
    let authority = DebugAuthority::new();
    let config = DebugConfig {
        drain_timeout: Some(Duration::from_millis(1)),
        ..DebugConfig::default()
    };
    let mut insp = DebugInspector::with_authority(config, &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 4,
        },
        now,
    )
    .expect("prepare");
    insp.attach_drain(Box::new(MockDrain {
        quiesced: false,
        in_flight: 2,
        records: VecDeque::new(),
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    std::thread::sleep(Duration::from_millis(5));
    let later = Instant::now();
    let status = insp.poll_drain(later);
    assert!(matches!(
        status,
        SessionDrainStatus::Draining {
            pending_quiescence: 1,
            deadline_exceeded: true,
            ..
        }
    ));
    // Still Draining — the deadline never forces a stop.
    assert_eq!(insp.session_state(), DebugSessionState::Draining);
    assert!(insp.session_view().deadline_exceeded);
}

#[test]
fn event_record_drain_reuses_rt_bridge() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "bridge",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");
    let (mut recorder, sink) = rt_bridge::bridge::<EventRecord>(8);
    assert!(recorder.try_record(event(Confidentiality::Internal)));
    assert!(recorder.try_record(event(Confidentiality::Internal)));
    let quiesced = Arc::new(AtomicBool::new(true));
    insp.attach_drain(Box::new(EventRecordDrain::new(sink, Arc::clone(&quiesced))))
        .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(
        status,
        SessionDrainStatus::Stopped {
            records_collected: 2,
            ..
        }
    ));
    assert_eq!(insp.drained_records().len(), 2);
    assert_eq!(insp.drained_records()[0].record.header().event().get(), 7);
}

#[test]
fn record_overflow_is_counted_not_silent() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 1,
        },
        now,
    )
    .expect("prepare");
    let mut records = VecDeque::new();
    records.push_back(session_record(Confidentiality::Internal));
    records.push_back(session_record(Confidentiality::Internal));
    records.push_back(session_record(Confidentiality::Internal));
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(
        status,
        SessionDrainStatus::Stopped {
            records_collected: 1,
            records_overflowed: 2,
            ..
        }
    ));
    assert_eq!(insp.session_view().records_overflowed, 2);
    assert_eq!(insp.health().drain_records_overflowed_total, 2);
}

// ------------------------------------------------------------------
// Debug report
// ------------------------------------------------------------------

#[test]
fn report_requires_export_authorization() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let now = Instant::now();
    // Disclosure above max_export_disclosure → refused entirely.
    let err = insp
        .report(&report_request(Confidentiality::Confidential), None, now)
        .unwrap_err();
    assert!(matches!(
        err,
        ExportError::Unauthorized {
            requested: Confidentiality::Confidential,
            permitted: Confidentiality::Internal,
        }
    ));
    // External export not permitted by default.
    let mut external = report_request(Confidentiality::Internal);
    external.external = true;
    let err = insp.report(&external, None, now).unwrap_err();
    assert_eq!(err, ExportError::ExternalNotPermitted);
    assert_eq!(insp.health().reports_rejected_total, 2);
}

#[test]
fn report_carries_record_metadata_only() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");
    let mut records = VecDeque::new();
    records.push_back(session_record(Confidentiality::Internal));
    records.push_back(session_record(Confidentiality::Confidential));
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(status, SessionDrainStatus::Stopped { .. }));

    let report = insp
        .report(&report_request(Confidentiality::Internal), None, now)
        .expect("report");
    // Newest-first; the Confidential record is withheld wholesale.
    assert_eq!(report.records.len(), 1);
    assert_eq!(report.truncation.withheld_confidential, 1);
    assert_eq!(report.session.state, DebugSessionState::Stopped);
    assert_eq!(report.session.records_retained, 2);
    assert_ne!(report.notes.len(), 0);
    // Metadata-only: identity/classification/loss — no payload value
    // field exists on RecordSummary by construction.
    let r = &report.records[0];
    assert_eq!(r.event.get(), 7);
    assert_eq!(r.version, 1);
    assert_eq!(r.generation, 3);
    assert!(!r.clock_stamped);
    assert_eq!(r.confidentiality, Confidentiality::Internal);
}

#[test]
fn report_bounds_drop_items_observably() {
    let authority = DebugAuthority::new();
    let config = DebugConfig {
        max_export_disclosure: Confidentiality::Confidential,
        ..DebugConfig::default()
    };
    let mut insp = DebugInspector::with_authority(config, &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");
    let mut records = VecDeque::new();
    for _ in 0..4 {
        records.push_back(session_record(Confidentiality::Internal));
    }
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(status, SessionDrainStatus::Stopped { .. }));

    let mut req = report_request(Confidentiality::Confidential);
    req.limits.max_items = 2;
    let report = insp.report(&req, None, now).expect("report");
    assert_eq!(report.records.len(), 2);
    assert_eq!(report.truncation.items_dropped, 2);
    assert!(!report.external);
}

#[test]
fn report_echoes_inspection_bounded() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let now = Instant::now();
    let snap = insp
        .inspect(&request(&[InspectionTarget::Session]), &no_inputs(), now)
        .expect("inspect");
    let report = insp
        .report(&report_request(Confidentiality::Internal), Some(&snap), now)
        .expect("report");
    let echo = report.inspection.expect("echo");
    assert_eq!(echo.schema_version, INSPECTION_SCHEMA_VERSION);
    assert_eq!(echo.targets, vec![InspectionTarget::Session]);
    assert_eq!(echo.sections.len(), 1);
}

#[test]
fn session_section_reports_state_machine() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    let out = insp
        .inspect(&request(&[InspectionTarget::Session]), &no_inputs(), now)
        .expect("inspect");
    let s = session_section(&out);
    assert_eq!(s.status, SectionStatus::Ok);
    assert_eq!(s.state, DebugSessionState::Disabled);
    assert_eq!(s.generation, 0);

    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "lbl",
            max_records: 2,
        },
        now,
    )
    .expect("prepare");
    let out = insp
        .inspect(&request(&[InspectionTarget::Session]), &no_inputs(), now)
        .expect("inspect");
    let s = session_section(&out);
    assert_eq!(s.state, DebugSessionState::Prepared);
    assert_eq!(s.generation, 1);
    assert_eq!(s.label, Some("lbl"));
}

#[test]
fn empty_targets_produce_empty_snapshot() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let out = insp
        .inspect(&request(&[]), &no_inputs(), Instant::now())
        .expect("inspect");
    assert!(out.sections.is_empty());
    assert_eq!(out.request.targets.len(), 0);
    assert_eq!(out.consistency.level, ConsistencyLevel::Aligned);
    assert_eq!(out.schema_version, INSPECTION_SCHEMA_VERSION);
}

#[test]
fn canonical_target_order_is_deterministic() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let mut req = request(&[
        InspectionTarget::Probes,
        InspectionTarget::Metrics,
        InspectionTarget::Session,
    ]);
    req.targets.push(InspectionTarget::Metrics); // duplicate
    let out = insp
        .inspect(&req, &no_inputs(), Instant::now())
        .expect("inspect");
    let order: Vec<InspectionTarget> = out.sections.iter().map(InspectionSection::target).collect();
    assert_eq!(
        order,
        vec![
            InspectionTarget::Metrics,
            InspectionTarget::Session,
            InspectionTarget::Probes,
        ]
    );
    assert_eq!(out.request.targets.len(), 3);
}

#[test]
fn report_max_items_zero_is_summaries_only() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");
    let mut records = VecDeque::new();
    records.push_back(session_record(Confidentiality::Internal));
    records.push_back(session_record(Confidentiality::Internal));
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(status, SessionDrainStatus::Stopped { .. }));

    // `ExportLimits::max_items == 0` is the deliberate summaries-only
    // mode — retained records exist but no entries are carried, and
    // nothing is counted as dropped.
    let mut req = report_request(Confidentiality::Internal);
    req.limits.max_items = 0;
    let report = insp.report(&req, None, now).expect("report");
    assert!(report.records.is_empty());
    assert_eq!(report.truncation.items_dropped, 0);
    assert_eq!(report.truncation.withheld_confidential, 0);
    assert_eq!(report.session.records_retained, 2);
}

#[test]
fn unevaluated_diagnostics_report_is_not_a_generation() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    // A report generated without `evaluate` carries the
    // `source_generation == 0` "never evaluated" sentinel — it must
    // not be compared as a real collector generation.
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).expect("config");
    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            Instant::now(),
        )
        .expect("report");
    assert_eq!(report.source_generation, 0);

    let snap = snapshot_of(1, Vec::new());
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics, InspectionTarget::Diagnostics]),
            &InspectionInputs {
                metrics: Some(&snap),
                diagnostics: Some(&report),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    assert_eq!(out.consistency.level, ConsistencyLevel::Aligned);
    assert_eq!(out.consistency.collector.diagnostics, None);
    assert_eq!(out.consistency.collector.snapshot, Some(1));
}

#[test]
fn confirmed_skew_outranks_unknown_probe_clock() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    insp.add_probe_source(Box::new(FakeProbes(vec![deck(
        5,
        ProbeClockMark::UNKNOWN,
        vec![reading(1, 5, ProbeClockMark::UNKNOWN)],
        Confidentiality::Internal,
    )])));
    // Collector inputs disagree (metrics gen 1 vs diagnostics gen 2)
    // — confirmed skew must not be masked by the unverifiable probe
    // clock.
    let snap = snapshot_of(1, Vec::new());
    let diag = diagnostics_report(2);
    let out = insp
        .inspect(
            &request(&[
                InspectionTarget::Metrics,
                InspectionTarget::Diagnostics,
                InspectionTarget::Probes,
            ]),
            &InspectionInputs {
                metrics: Some(&snap),
                diagnostics: Some(&diag),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    assert_eq!(out.consistency.level, ConsistencyLevel::Skewed);
    assert!(out.consistency.probe_decks.iter().any(|d| !d.clock_known));
}

#[test]
fn diagnostics_findings_total_excludes_domain_health() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).expect("config");
    d.add_health_rule(
        "dom",
        HealthRule {
            code: code("test.rule"),
            metric_key: "k",
            signal: SignalKind::GaugeValue,
            direction: SignalDirection::Above,
            degrade_at: Some(5.0),
            unhealthy_at: None,
            reduced_validity_is: HealthState::Degraded,
            missing_is: HealthState::Unknown,
            valueless_is: HealthState::Unknown,
            hysteresis: Hysteresis {
                enter_after: 1,
                recover_after: 1,
            },
        },
    )
    .expect("rule");
    // Gauge value 7 > degrade_at 5.0 → Degraded + one transition
    // finding, plus one domain-health entry.
    let ms = vec![measurement(
        "k",
        ident(1, 0),
        Confidentiality::Internal,
        MeasurementValidity::Valid,
    )];
    d.evaluate(&snapshot_of(1, ms), Instant::now());
    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            Instant::now(),
        )
        .expect("report");
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.health.len(), 1);

    let mut insp = DebugInspector::new(DebugConfig::default());
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Diagnostics]),
            &InspectionInputs {
                diagnostics: Some(&report),
                ..no_inputs()
            },
            Instant::now(),
        )
        .expect("inspect");
    let ds = diagnostics_section(&out);
    // `findings_total` counts findings only — domain-health entries
    // are reported separately under `health`.
    assert_eq!(ds.findings_total, 1);
    assert_eq!(ds.health.len(), 1);
    assert_eq!(ds.entries.len(), 1);
}

// ------------------------------------------------------------------
// Issue #292 acceptance re-audit additions (rf292-obs10 slice)
// ------------------------------------------------------------------

#[test]
fn batch_generation_skew_is_declared_not_hidden() {
    let mut insp = DebugInspector::new(DebugConfig::default());
    let now = Instant::now();
    // `ObservationBatch.generation` is a third stamp in the *same*
    // collector generation domain — a batch at generation 2 against a
    // metrics snapshot at generation 1 must surface as Skewed.
    let snap = snapshot_of(1, Vec::new());
    let window = ObservationWindow {
        generation: 2,
        started: now,
        ended: now,
        expected: Duration::from_millis(10),
        measured: Duration::ZERO,
        clock: ClockDomainId::MONOTONIC,
        planned: 0,
        observed: 0,
        skipped: 0,
        drained: 0,
        history_evicted: 0,
        clock_anomaly: false,
    };
    let batch = ObservationBatch::new(
        2,
        window,
        Vec::new(),
        Vec::new(),
        snapshot_of(2, Vec::new()),
    );
    let out = insp
        .inspect(
            &request(&[InspectionTarget::Metrics]),
            &InspectionInputs {
                metrics: Some(&snap),
                batch: Some(&batch),
                ..no_inputs()
            },
            now,
        )
        .expect("inspect");
    assert_eq!(out.consistency.level, ConsistencyLevel::Skewed);
    assert_eq!(out.consistency.collector.snapshot, Some(1));
    assert_eq!(out.consistency.collector.batch, Some(2));
    assert_eq!(out.consistency.collector.diagnostics, None);
    assert!(!out.consistency.collector.aligned);
}

#[test]
fn drain_waits_for_quiescence_then_recovers_rt_records() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "rt",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");

    // Real OBS-05 bridge: two records sit in flight while the writer
    // has not confirmed quiescence.
    let (mut recorder, sink) = rt_bridge::bridge::<EventRecord>(8);
    assert!(recorder.try_record(event(Confidentiality::Internal)));
    assert!(recorder.try_record(event(Confidentiality::Internal)));
    let quiesced = Arc::new(AtomicBool::new(false));
    insp.attach_drain(Box::new(EventRecordDrain::new(sink, Arc::clone(&quiesced))))
        .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");

    // Un-quiesced: the endpoint (ring/`Arc`) is retained, the in-flight
    // records are *not* collected, and repeated polls stay honestly
    // `Draining` — final recovery never runs early.
    for _ in 0..2 {
        let status = insp.poll_drain(now);
        assert!(matches!(
            status,
            SessionDrainStatus::Draining {
                pending_quiescence: 1,
                pending_records: 2,
                deadline_exceeded: false,
            }
        ));
    }
    assert!(insp.drained_records().is_empty());
    let view = insp.session_view();
    assert_eq!(view.drains_attached, 1);
    assert_eq!(view.pending_records, 2);

    // Quiescence confirmed → the bounded final drain recovers both
    // records and only then are the drain endpoints released.
    quiesced.store(true, Ordering::Relaxed);
    let status = insp.poll_drain(now);
    assert!(matches!(
        status,
        SessionDrainStatus::Stopped {
            clean: true,
            records_collected: 2,
            records_overflowed: 0,
        }
    ));
    assert_eq!(insp.drained_records().len(), 2);
    let view = insp.session_view();
    assert_eq!(view.drains_attached, 0);
    assert_eq!(view.pending_records, 0);
    assert_eq!(insp.health().drain_records_delivered_total, 2);
}

#[test]
fn draining_rejects_attach_activate_and_prepare() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 4,
        },
        now,
    )
    .expect("prepare");
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records: VecDeque::new(),
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    assert_eq!(insp.session_state(), DebugSessionState::Draining);

    // Stop race: while Draining no new drain may attach, the session
    // cannot re-activate, and a new session cannot be prepared — every
    // rejection is typed, never queued or reported as success.
    let err = insp
        .attach_drain(Box::new(MockDrain {
            quiesced: true,
            in_flight: 0,
            records: VecDeque::new(),
        }))
        .unwrap_err();
    assert!(matches!(
        err,
        SessionError::InvalidTransition {
            actual: DebugSessionState::Draining
        }
    ));
    let err = insp.activate_session(now).unwrap_err();
    assert!(matches!(
        err,
        SessionError::InvalidTransition {
            actual: DebugSessionState::Draining
        }
    ));
    let err = insp
        .prepare_session(
            authority.permit().expect("p"),
            SessionRequest {
                label: "t",
                max_records: 4,
            },
            now,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        SessionError::SessionBusy {
            state: DebugSessionState::Draining
        }
    ));
}

#[test]
fn report_external_export_permitted_when_allowed() {
    let config = DebugConfig {
        allow_external_export: true,
        ..DebugConfig::default()
    };
    let mut insp = DebugInspector::new(config);
    let now = Instant::now();
    let mut req = report_request(Confidentiality::Internal);
    req.external = true;
    let report = insp.report(&req, None, now).expect("authorized export");
    assert!(report.external);
    assert_eq!(report.disclosure, Confidentiality::Internal);
    assert_eq!(insp.health().reports_generated_total, 1);
}

#[test]
fn report_export_limits_bytes_and_deadline_truncate() {
    let authority = DebugAuthority::new();
    let mut insp = DebugInspector::with_authority(DebugConfig::default(), &authority);
    let now = Instant::now();
    insp.prepare_session(
        authority.permit().expect("p"),
        SessionRequest {
            label: "t",
            max_records: 8,
        },
        now,
    )
    .expect("prepare");
    let mut records = VecDeque::new();
    for _ in 0..4 {
        records.push_back(session_record(Confidentiality::Internal));
    }
    insp.attach_drain(Box::new(MockDrain {
        quiesced: true,
        in_flight: 0,
        records,
    }))
    .expect("attach");
    insp.activate_session(now).expect("activate");
    insp.request_stop(authority.permit().expect("p"), now)
        .expect("stop");
    let status = insp.poll_drain(now);
    assert!(matches!(status, SessionDrainStatus::Stopped { .. }));

    // `max_bytes` admits exactly one `RecordSummary`-sized entry; the
    // rest are counted as dropped, never silently carried.
    let mut req = report_request(Confidentiality::Internal);
    req.limits.max_bytes = size_of::<RecordSummary>();
    let report = insp.report(&req, None, now).expect("report");
    assert_eq!(report.records.len(), 1);
    assert_eq!(report.truncation.items_dropped, 3);
    assert_eq!(
        report.truncation.bytes_estimated,
        size_of::<RecordSummary>()
    );
    assert!(!report.truncation.deadline_exceeded);

    // A zero deadline expires before the first item — every retained
    // record is accounted as dropped and the flag is reported.
    let mut req = report_request(Confidentiality::Internal);
    req.limits.deadline = Some(Duration::ZERO);
    let report = insp.report(&req, None, now).expect("report");
    assert!(report.records.is_empty());
    assert_eq!(report.truncation.items_dropped, 4);
    assert!(report.truncation.deadline_exceeded);
}

/// RT lane (`cargo test --workspace rt_`): the `Copy` value types an
/// RT-adjacent path may hold — construct, copy, and read — allocate
/// nothing inside the measurement window.
#[test]
fn rt_debug_value_types_allocate_nothing() {
    use assert_no_alloc::assert_no_alloc;

    assert_no_alloc(|| {
        let clock = ProbeClockMark {
            domain: ClockDomainId::new(1),
            epoch: ClockEpochId::new(0),
        };
        let status = ProbeStatusKind::ModeChanged {
            to: ProbeMode::Capture,
        };
        let reading = ProbeReading {
            probe: 1,
            node: 2,
            port: 3,
            mode: ProbeMode::Monitor,
            availability: ProbeAvailability::Configured,
            generation: 4,
            clock,
            last_status: Some(status),
            recorded: 5,
            dropped: 6,
            lost_blocks: 7,
        };
        let record = SessionRecord {
            record: event(Confidentiality::Internal),
            dropped_before: 0,
        };
        // Copies and field reads — the only operations an RT-adjacent
        // consumer may perform on these types.
        let (clock, status, reading, record) = (clock, status, reading, record);
        std::hint::black_box((
            clock.domain,
            clock.epoch,
            matches!(status, ProbeStatusKind::ModeChanged { .. }),
            reading.generation,
            reading.last_status.is_some(),
            record.dropped_before,
            record.record.header().event().get(),
        ));
    });
}
