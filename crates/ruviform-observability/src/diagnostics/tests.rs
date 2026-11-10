//! Contract tests for `diagnostics` — Feature/Rule/Example coverage
//! per `docs/test-specs/issue-291-observability-diagnostics-tests.md`.
//!
//! These tests run in the default feature set (`logging` OFF): every
//! diagnostics API must work without any subscriber or logging
//! backend.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::collector::MetricsSnapshot;
use crate::metrics::{Measurement, MeasurementValidity, MetricValue};
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

fn gauge_measurement(
    key: &'static str,
    identity: SourceIdentity,
    value: u64,
    validity: MeasurementValidity,
) -> Measurement {
    Measurement {
        identity,
        descriptor: descriptor(key, Confidentiality::Internal),
        scope: ObservationScope::Instance,
        lifecycle: SourceLifecycle::Active,
        observed_at: Instant::now(),
        last_change_at: Instant::now(),
        validity,
        issue: None,
        loss: None,
        value: Some(MetricValue::Gauge {
            value,
            previous: None,
        }),
    }
}

fn snapshot_of(measurements: Vec<Measurement>) -> MetricsSnapshot {
    MetricsSnapshot::new(1, Instant::now(), measurements)
}

fn meta(domain: &'static str, subject: Option<&'static str>) -> EvidenceMeta {
    EvidenceMeta {
        domain,
        subject,
        time: EvidenceTime::Unstamped,
        validity: MeasurementValidity::Valid,
        confidentiality: Confidentiality::Internal,
        payload: EvidencePayload::None,
    }
}

fn code(name: &'static str) -> DiagnosticCode {
    DiagnosticCode {
        name,
        schema_version: 1,
    }
}

fn gauge_rule(
    key: &'static str,
    degrade_at: f64,
    unhealthy_at: Option<f64>,
    enter_after: u32,
    recover_after: u32,
) -> HealthRule {
    HealthRule {
        code: code("test.rule"),
        metric_key: key,
        signal: SignalKind::GaugeValue,
        direction: SignalDirection::Above,
        degrade_at: Some(degrade_at),
        unhealthy_at,
        reduced_validity_is: HealthState::Degraded,
        missing_is: HealthState::Unknown,
        valueless_is: HealthState::Unknown,
        hysteresis: Hysteresis {
            enter_after,
            recover_after,
        },
    }
}

fn finding(
    name: &'static str,
    domain: &'static str,
    severity: FindingSeverity,
    certainty: FindingCertainty,
    refs: &[EvidenceRef],
    now: Instant,
) -> DiagnosticFinding {
    DiagnosticFinding::new(
        code(name),
        domain,
        None,
        severity,
        certainty,
        Confidentiality::Internal,
        "test finding",
        refs.to_vec(),
        now,
    )
    .expect("finding construction")
}

struct VecSink {
    events: Arc<Mutex<Vec<DiagnosticsEvent>>>,
}

impl FindingSink for VecSink {
    fn emit(&mut self, event: DiagnosticsEvent) {
        self.events.lock().expect("sink mutex").push(event);
    }
}

struct PanicSink;

impl FindingSink for PanicSink {
    fn emit(&mut self, _event: DiagnosticsEvent) {
        panic!("sink failure");
    }
}

// ------------------------------------------------------------------
// Evidence — golden classification tests
// ------------------------------------------------------------------

#[test]
fn evidence_from_measurement_preserves_all_fields() {
    let identity = ident(7, 3);
    let mut m = gauge_measurement("l1.queue.depth", identity, 42, MeasurementValidity::Stale);
    m.descriptor.semantics.confidentiality = Confidentiality::Confidential;
    let e = Evidence::from_measurement(&m);

    assert_eq!(e.kind(), EvidenceKind::Observed);
    assert_eq!(e.source(), Some(identity));
    assert_eq!(e.subject(), Some("l1.queue.depth"));
    assert_eq!(e.domain(), m.descriptor.semantics.origin);
    assert_eq!(e.validity(), MeasurementValidity::Stale);
    assert_eq!(e.confidentiality(), Confidentiality::Confidential);
    assert_eq!(e.basis(), &[] as &[EvidenceRef]);
    assert!(matches!(e.time(), EvidenceTime::Monotonic(_)));
    assert!(matches!(e.payload(), EvidencePayload::Metric(_)));
}

#[test]
fn evidence_kinds_are_distinct_and_never_promote() {
    let i = ident(1, 0);
    let observed = Evidence::observed(i, meta("d", Some("k")));
    let reported = Evidence::reported(Some(i), meta("d", Some("k")));
    let r = EvidenceRef::mint(9);
    let derived = Evidence::derived(vec![r], Some(i), meta("d", Some("k"))).unwrap();
    let inferred = Evidence::inferred([r], None, meta("d", Some("k"))).unwrap();

    assert_eq!(observed.kind(), EvidenceKind::Observed);
    assert_eq!(reported.kind(), EvidenceKind::Reported);
    assert_eq!(derived.kind(), EvidenceKind::Derived);
    assert_eq!(inferred.kind(), EvidenceKind::Inferred);
    // Basis contract: observed/reported carry none; derived/inferred do.
    assert!(observed.basis().is_empty() && reported.basis().is_empty());
    assert_eq!(derived.basis(), &[r]);
    assert_eq!(inferred.basis(), &[r]);
}

#[test]
fn evidence_clock_domain_stamp_is_preserved_verbatim() {
    use ruviform_media_core::{
        ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance,
    };

    use crate::record::ClockContext;

    // A domain-native clock reading must round-trip untouched: domain,
    // ticks, timebase, epoch, and provenance are carried verbatim with
    // no conversion — and `Estimated` provenance stays distinct from
    // `Unstamped`.
    let ctx = ClockContext::new(
        InputStamp {
            ticks: 96_000,
            base: TimeBase::new(1, 48_000).expect("valid timebase"),
            domain: ClockDomainId::SAMPLE_CLOCK,
            provenance: TimestampProvenance::Estimated,
        },
        ClockEpochId::new(3),
    );
    let e = Evidence::reported(
        None,
        EvidenceMeta {
            time: EvidenceTime::Clock(ctx),
            ..meta("clock.domain", Some("clock.key"))
        },
    );

    assert_eq!(e.time(), EvidenceTime::Clock(ctx));
    match e.time() {
        EvidenceTime::Clock(c) => {
            assert_eq!(c.domain(), ClockDomainId::SAMPLE_CLOCK);
            assert_eq!(c.position(), 96_000);
            assert_eq!(c.time_base(), TimeBase::new(1, 48_000).unwrap());
            assert_eq!(c.epoch(), ClockEpochId::new(3));
            assert_eq!(c.provenance(), TimestampProvenance::Estimated);
        }
        other => panic!("expected EvidenceTime::Clock, got {other:?}"),
    }
}

#[test]
fn derived_and_inferred_require_non_empty_basis() {
    let empty: Vec<EvidenceRef> = Vec::new();
    assert!(matches!(
        Evidence::derived(empty.clone(), None, meta("d", None)),
        Err(EvidenceError::MissingBasis)
    ));
    assert!(matches!(
        Evidence::inferred(empty, None, meta("d", None)),
        Err(EvidenceError::MissingBasis)
    ));
}

#[test]
fn evidence_payload_cannot_carry_arbitrary_bytes() {
    // The vocabulary is closed: Metric (typed) / Claim (static str) /
    // None. There is no Vec<u8>, path, or token variant by construction.
    let e = Evidence::reported(
        None,
        EvidenceMeta {
            payload: EvidencePayload::Claim("peer-reported"),
            ..meta("d", None)
        },
    );
    assert!(matches!(
        e.payload(),
        EvidencePayload::Claim("peer-reported")
    ));
}

// ------------------------------------------------------------------
// Finding — certainty and upsert tests
// ------------------------------------------------------------------

#[test]
fn inferred_finding_requires_evidence_basis() {
    let now = Instant::now();
    let err = DiagnosticFinding::new(
        code("inf"),
        "dom",
        None,
        FindingSeverity::Warning,
        FindingCertainty::Inferred,
        Confidentiality::Internal,
        "guess",
        Vec::<EvidenceRef>::new(),
        now,
    );
    assert!(matches!(err, Err(FindingError::MissingBasis)));

    let ok = DiagnosticFinding::new(
        code("inf"),
        "dom",
        None,
        FindingSeverity::Warning,
        FindingCertainty::Inferred,
        Confidentiality::Internal,
        "guess",
        vec![EvidenceRef::mint(1)],
        now,
    );
    assert!(ok.is_ok());
}

#[test]
fn confirmed_finding_needs_no_refs_but_carries_certainty() {
    let f = finding(
        "conf",
        "dom",
        FindingSeverity::Critical,
        FindingCertainty::Confirmed,
        &[],
        Instant::now(),
    );
    assert_eq!(f.certainty(), FindingCertainty::Confirmed);
    assert_eq!(f.status(), FindingStatus::Open);
    assert_eq!(f.code().name, "conf");
    assert_eq!(f.code().schema_version, 1);
}

#[test]
fn finding_upsert_merges_worst_severity_and_latest_fields() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    let t0 = Instant::now();
    let f1 = finding(
        "dup",
        "dom",
        FindingSeverity::Info,
        FindingCertainty::Confirmed,
        &[],
        t0,
    );
    d.record_finding(f1).unwrap();
    let f2 = finding(
        "dup",
        "dom",
        FindingSeverity::Critical,
        FindingCertainty::Confirmed,
        &[EvidenceRef::mint(3)],
        t0 + Duration::from_secs(1),
    );
    d.record_finding(f2).unwrap();

    assert_eq!(d.findings_len(), 1);
    // Inspect via report (Public disclosure includes Internal items? no —
    // Internal > Public; request Internal).
    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            Instant::now(),
        )
        .unwrap();
    assert_eq!(report.findings.len(), 1);
    let f = &report.findings[0];
    assert_eq!(f.severity(), FindingSeverity::Critical);
    assert_eq!(f.evidence_refs(), &[EvidenceRef::mint(3)]);
    assert_eq!(f.first_seen_at(), t0);
    assert_eq!(f.last_seen_at(), t0 + Duration::from_secs(1));
}

// ------------------------------------------------------------------
// HealthEvaluator — threshold/hysteresis/recovery/unknown tests
// ------------------------------------------------------------------

#[test]
fn hysteresis_requires_consecutive_verdicts_before_transition() {
    let mut ev = HealthEvaluator::new("dom");
    ev.add_rule(gauge_rule("k", 5.0, Some(9.0), 2, 2)).unwrap();
    let bad = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    let good = vec![gauge_measurement(
        "k",
        ident(1, 0),
        0,
        MeasurementValidity::Valid,
    )];
    let now = Instant::now();

    // First bad eval: streak 1 < enter_after 2 → still Healthy.
    let e1 = ev.evaluate(&bad, now);
    assert_eq!(e1.health.state, HealthState::Healthy);
    assert!(e1.transitions.is_empty());

    // Second consecutive bad eval → Degraded confirmed.
    let e2 = ev.evaluate(&bad, now + Duration::from_millis(1));
    assert_eq!(e2.health.state, HealthState::Degraded);
    assert_eq!(e2.transitions.len(), 1);
    assert_eq!(e2.transitions[0].from, HealthState::Healthy);
    assert_eq!(e2.transitions[0].to, HealthState::Degraded);

    // One good eval is not enough to recover (recover_after = 2).
    let e3 = ev.evaluate(&good, now + Duration::from_millis(2));
    assert_eq!(e3.health.state, HealthState::Degraded);
    // Second consecutive good eval → Healthy.
    let e4 = ev.evaluate(&good, now + Duration::from_millis(3));
    assert_eq!(e4.health.state, HealthState::Healthy);
    assert_eq!(e4.transitions[0].to, HealthState::Healthy);
}

#[test]
fn unhealthy_threshold_is_worse_side_of_degrade() {
    let mut ev = HealthEvaluator::new("dom");
    ev.add_rule(gauge_rule("k", 5.0, Some(9.0), 1, 1)).unwrap();
    let now = Instant::now();
    let severe = vec![gauge_measurement(
        "k",
        ident(1, 0),
        10,
        MeasurementValidity::Valid,
    )];
    let e = ev.evaluate(&severe, now);
    assert_eq!(e.health.state, HealthState::Unhealthy);
    assert_eq!(e.health.rules[0].state, HealthState::Unhealthy);
}

#[test]
fn missing_stale_and_valueless_map_to_declared_states() {
    let mut ev = HealthEvaluator::new("dom");
    ev.add_rule(gauge_rule("k", 5.0, None, 1, 1)).unwrap();
    let now = Instant::now();

    // Missing → missing_is (Unknown here).
    let e = ev.evaluate(&[], now);
    assert_eq!(e.health.state, HealthState::Unknown);
    assert!(matches!(e.health.rules[0].input, RuleInput::Missing));

    // Stale → reduced_validity_is (Degraded here).
    let stale = vec![gauge_measurement(
        "k",
        ident(1, 0),
        1,
        MeasurementValidity::Stale,
    )];
    let e = ev.evaluate(&stale, now);
    assert_eq!(e.health.rules[0].state, HealthState::Degraded);
    assert!(matches!(
        e.health.rules[0].input,
        RuleInput::ReducedValidity(MeasurementValidity::Stale)
    ));

    // Valueless (value = None) → valueless_is (Unknown here).
    let mut valueless = gauge_measurement("k", ident(1, 0), 0, MeasurementValidity::Unavailable);
    valueless.value = None;
    let e = ev.evaluate(&[valueless], now);
    assert!(matches!(
        e.health.rules[0].input,
        RuleInput::Valueless(MeasurementValidity::Unavailable)
    ));
}

#[test]
fn unevaluable_signal_is_unknown_not_zero() {
    let mut ev = HealthEvaluator::new("dom");
    let mut rule = gauge_rule("c", 5.0, None, 1, 1);
    rule.signal = SignalKind::CounterDelta;
    rule.metric_key = "counter.key";
    ev.add_rule(rule).unwrap();

    // Counter whose delta is suppressed (decrease) → signal
    // unevaluable → raw Unknown, never treated as 0.
    let mut m = gauge_measurement("counter.key", ident(1, 0), 0, MeasurementValidity::Valid);
    m.value = Some(MetricValue::Counter {
        raw: 10,
        cumulative_since_registration: 10,
        delta: None,
        rate_per_sec: None,
    });
    let e = ev.evaluate(&[m], Instant::now());
    assert_eq!(e.health.rules[0].state, HealthState::Unknown);
    assert!(matches!(e.health.rules[0].input, RuleInput::Unevaluable));
}

#[test]
fn source_generation_change_resets_hysteresis() {
    let mut ev = HealthEvaluator::new("dom");
    ev.add_rule(gauge_rule("k", 5.0, None, 2, 1)).unwrap();
    let now = Instant::now();

    // Gen 0: one bad streak (needs 2).
    let gen0 = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    assert_eq!(ev.evaluate(&gen0, now).health.state, HealthState::Healthy);

    // Gen 1 appears: streak resets — still needs 2 bad evals.
    let gen1 = vec![gauge_measurement(
        "k",
        ident(1, 1),
        6,
        MeasurementValidity::Valid,
    )];
    assert_eq!(
        ev.evaluate(&gen1, now + Duration::from_millis(1))
            .health
            .state,
        HealthState::Healthy
    );
    // Second gen-1 eval confirms.
    assert_eq!(
        ev.evaluate(&gen1, now + Duration::from_millis(2))
            .health
            .state,
        HealthState::Degraded
    );
}

#[test]
fn generation_change_never_fabricates_healthy_over_confirmed_bad() {
    let mut ev = HealthEvaluator::new("dom");
    // enter_after=1 so a single bad verdict confirms Degraded.
    ev.add_rule(gauge_rule("k", 5.0, None, 1, 1)).unwrap();
    let now = Instant::now();

    // Gen 0 confirms Degraded.
    let gen0 = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    assert_eq!(ev.evaluate(&gen0, now).health.state, HealthState::Degraded);

    // Gen 1 appears still bad: the confirmed state must NOT silently
    // reset to Healthy — a re-registration is not evidence of health.
    // The streak restarts, so with enter_after=1 the first gen-1 bad
    // verdict re-confirms Degraded with no spurious transition.
    let gen1 = vec![gauge_measurement(
        "k",
        ident(1, 1),
        6,
        MeasurementValidity::Valid,
    )];
    let e = ev.evaluate(&gen1, now + Duration::from_millis(1));
    assert_eq!(e.health.state, HealthState::Degraded);
    assert!(
        e.transitions.is_empty(),
        "no fabricated transition on generation change"
    );

    // Gen 1 recovers: recovery still requires its own streak.
    let good = vec![gauge_measurement(
        "k",
        ident(1, 1),
        0,
        MeasurementValidity::Valid,
    )];
    // recover_after=1 → single healthy verdict recovers.
    let e = ev.evaluate(&good, now + Duration::from_millis(2));
    assert_eq!(e.health.state, HealthState::Healthy);
    assert_eq!(e.transitions.len(), 1);
    assert_eq!(e.transitions[0].from, HealthState::Degraded);
    assert_eq!(e.transitions[0].to, HealthState::Healthy);
}

#[test]
fn rollup_partial_failure_is_degraded_not_unhealthy() {
    let mut ev = HealthEvaluator::new("dom");
    ev.add_rule(gauge_rule("a", 5.0, Some(9.0), 1, 1)).unwrap();
    let mut rule_b = gauge_rule("b", 5.0, Some(9.0), 1, 1);
    rule_b.code = code("test.rule.b");
    rule_b.metric_key = "b";
    ev.add_rule(rule_b).unwrap();
    let now = Instant::now();

    // a=10 (Unhealthy), b absent (Unknown via missing_is) → known =
    // {Unhealthy} only → all known unhealthy → Unhealthy.
    let ms = vec![gauge_measurement(
        "a",
        ident(1, 0),
        10,
        MeasurementValidity::Valid,
    )];
    let e = ev.evaluate(&ms, now);
    assert_eq!(e.health.state, HealthState::Unhealthy);

    // a healthy, b missing → known={Healthy}, unknown mixed → Degraded.
    let ms = vec![gauge_measurement(
        "a",
        ident(1, 0),
        0,
        MeasurementValidity::Valid,
    )];
    let e = ev.evaluate(&ms, now + Duration::from_millis(1));
    assert_eq!(e.health.state, HealthState::Degraded);

    // Both healthy → Healthy.
    let ms = vec![
        gauge_measurement("a", ident(1, 0), 0, MeasurementValidity::Valid),
        gauge_measurement("b", ident(2, 0), 0, MeasurementValidity::Valid),
    ];
    let e = ev.evaluate(&ms, now + Duration::from_millis(2));
    assert_eq!(e.health.state, HealthState::Healthy);
}

#[test]
fn rule_validation_rejects_invalid_declarations() {
    let mut ev = HealthEvaluator::new("dom");
    let mut bad = gauge_rule("k", 5.0, None, 0, 1);
    assert_eq!(ev.add_rule(bad.clone()), Err(RuleError::InvalidHysteresis));
    bad.hysteresis = Hysteresis {
        enter_after: 1,
        recover_after: 1,
    };
    bad.degrade_at = None;
    bad.unhealthy_at = None;
    assert_eq!(ev.add_rule(bad.clone()), Err(RuleError::MissingThreshold));
    bad.degrade_at = Some(f64::NAN);
    assert_eq!(ev.add_rule(bad.clone()), Err(RuleError::NonFiniteThreshold));
    bad.degrade_at = Some(5.0);
    bad.unhealthy_at = Some(3.0); // worse side for Above must be >=
    assert_eq!(ev.add_rule(bad.clone()), Err(RuleError::ThresholdOrder));
    bad.unhealthy_at = None;
    ev.add_rule(bad.clone()).unwrap();
    assert_eq!(ev.add_rule(bad), Err(RuleError::DuplicateCode));
}

// ------------------------------------------------------------------
// Diagnostics engine — evidence/finding/health/report integration
// ------------------------------------------------------------------

#[test]
fn record_evidence_mints_refs_and_evicts_fifo() {
    let config = DiagnosticsConfig {
        max_evidence: 2,
        ..Default::default()
    };
    let mut d = Diagnostics::new(config).unwrap();

    let r0 = d
        .record_evidence(Evidence::reported(None, meta("d", None)))
        .unwrap();
    let r1 = d
        .record_evidence(Evidence::reported(None, meta("d", None)))
        .unwrap();
    let r2 = d
        .record_evidence(Evidence::reported(None, meta("d", None)))
        .unwrap();

    assert_eq!(d.evidence_len(), 2);
    assert_eq!(d.health().evidence_evicted_total, 1);
    let snap = d.evaluate(&snapshot_of(vec![]), Instant::now());
    let refs: Vec<EvidenceRef> = snap.evidence.iter().map(|e| e.reference).collect();
    assert_eq!(refs, vec![r1, r2]);
    assert_ne!(r0, r1);
}

#[test]
fn evidence_retention_disabled_is_typed_error() {
    let config = DiagnosticsConfig {
        max_evidence: 0,
        max_findings: 0,
        ..Default::default()
    };
    let mut d = Diagnostics::new(config).unwrap();
    assert_eq!(
        d.record_evidence(Evidence::reported(None, meta("d", None))),
        Err(EvidenceError::RetentionDisabled)
    );
    assert_eq!(
        d.record_finding(finding(
            "x",
            "d",
            FindingSeverity::Info,
            FindingCertainty::Confirmed,
            &[],
            Instant::now()
        )),
        Err(FindingError::RetentionDisabled)
    );
}

#[test]
fn evaluate_marks_superseded_evidence_never_rebinds() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    let a = ident(1, 0);
    let e = Evidence::observed(a, meta("dom", Some("k")));
    let r = d.record_evidence(e).unwrap();

    // Snapshot containing the source → not superseded.
    let ms = vec![gauge_measurement("k", a, 1, MeasurementValidity::Valid)];
    let snap = d.evaluate(&snapshot_of(ms.clone()), Instant::now());
    let entry = snap.evidence.iter().find(|e| e.reference == r).unwrap();
    assert!(!entry.superseded);
    assert_eq!(entry.evidence.source(), Some(a));

    // Snapshot where the source is gone (removed) → superseded.
    let snap = d.evaluate(&snapshot_of(vec![]), Instant::now());
    let entry = snap.evidence.iter().find(|e| e.reference == r).unwrap();
    assert!(entry.superseded);
    // The identity is preserved verbatim — never rebound to another.
    assert_eq!(entry.evidence.source(), Some(a));
}

#[test]
fn evaluate_produces_findings_for_rule_transitions() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    d.add_health_rule("dom", gauge_rule("k", 5.0, None, 1, 1))
        .unwrap();
    let bad = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    let snap = d.evaluate(&snapshot_of(bad), Instant::now());

    assert_eq!(snap.domains.len(), 1);
    assert_eq!(snap.domains[0].state, HealthState::Degraded);
    assert_eq!(snap.domains[0].domain, "dom");
    assert_eq!(snap.consistency, SnapshotConsistency::IndependentReads);
    assert_eq!(snap.generation, 1);

    // A Warning/Confirmed finding was created for the transition.
    let f = snap
        .findings
        .iter()
        .find(|f| f.code().name == "test.rule")
        .expect("transition finding");
    assert_eq!(f.severity(), FindingSeverity::Warning);
    assert_eq!(f.certainty(), FindingCertainty::Confirmed);
    assert_eq!(f.status(), FindingStatus::Open);
    assert_eq!(f.domain(), "dom");
}

#[test]
fn recovery_resolves_open_finding() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    d.add_health_rule("dom", gauge_rule("k", 5.0, None, 1, 1))
        .unwrap();
    let now = Instant::now();
    let bad = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    let good = vec![gauge_measurement(
        "k",
        ident(1, 0),
        0,
        MeasurementValidity::Valid,
    )];

    d.evaluate(&snapshot_of(bad), now);
    d.evaluate(&snapshot_of(good.clone()), now + Duration::from_millis(1));
    let snap = d.evaluate(&snapshot_of(good), now + Duration::from_millis(2));

    let f = snap
        .findings
        .iter()
        .find(|f| f.code().name == "test.rule")
        .expect("finding retained");
    assert_eq!(f.status(), FindingStatus::Resolved);
    assert_eq!(snap.domains[0].state, HealthState::Healthy);
}

#[test]
fn sink_receives_domain_transitions_and_suppresses_over_limit() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut config = DiagnosticsConfig::default();
    config.sink_policy.max_events = 1;
    config.sink_policy.window = Duration::from_secs(60);
    let mut d = Diagnostics::new(config).unwrap();
    d.set_sink(Box::new(VecSink {
        events: events.clone(),
    }));

    // Two domains transition in one evaluate → 2 events, policy allows 1.
    d.add_health_rule("dom.a", {
        let mut r = gauge_rule("ka", 5.0, None, 1, 1);
        r.code = code("rule.a");
        r
    })
    .unwrap();
    d.add_health_rule("dom.b", {
        let mut r = gauge_rule("kb", 5.0, None, 1, 1);
        r.code = code("rule.b");
        r
    })
    .unwrap();
    let bad = vec![
        gauge_measurement("ka", ident(1, 0), 6, MeasurementValidity::Valid),
        gauge_measurement("kb", ident(2, 0), 6, MeasurementValidity::Valid),
    ];
    d.evaluate(&snapshot_of(bad), Instant::now());

    let got = events.lock().expect("events");
    assert_eq!(got.len(), 1, "policy max_events=1 per window");
    assert!(matches!(got[0], DiagnosticsEvent::HealthTransition { .. }));
    drop(got);
    assert_eq!(d.health().sink_emitted_total, 1);
    assert_eq!(d.health().sink_suppressed_total, 1);
}

#[test]
fn sink_panic_is_isolated_and_counted() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    d.set_sink(Box::new(PanicSink));
    d.add_health_rule("dom", gauge_rule("k", 5.0, None, 1, 1))
        .unwrap();
    let bad = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    let snap = d.evaluate(&snapshot_of(bad), Instant::now());
    // Evaluation completed despite the panicking sink.
    assert_eq!(snap.domains[0].state, HealthState::Degraded);
    assert_eq!(d.health().sink_panics_total, 1);
    assert_eq!(d.health().sink_emitted_total, 0);
}

#[test]
fn report_rejects_unauthorized_disclosure() {
    let config = DiagnosticsConfig {
        max_export_disclosure: Confidentiality::Internal,
        ..Default::default()
    };
    let mut d = Diagnostics::new(config).unwrap();
    d.record_evidence(Evidence::reported(None, meta("d", None)))
        .unwrap();

    let err = d.report(
        &ReportRequest {
            disclosure: Confidentiality::Confidential,
        },
        Instant::now(),
    );
    assert!(matches!(
        err,
        Err(ReportError::Unauthorized {
            requested: Confidentiality::Confidential,
            permitted: Confidentiality::Internal,
        })
    ));
    assert_eq!(d.health().reports_rejected_total, 1);
    assert_eq!(d.health().reports_generated_total, 0);
}

#[test]
fn report_withholds_over_disclosure_items_without_leaking() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    // Confidential evidence must not appear in a Public report; the
    // Public-classified item must (Internal would also be withheld).
    let confidential_meta = EvidenceMeta {
        confidentiality: Confidentiality::Confidential,
        ..meta("secret.domain", Some("secret.key"))
    };
    let public_meta = EvidenceMeta {
        confidentiality: Confidentiality::Public,
        ..meta("open.domain", Some("open.key"))
    };
    d.record_evidence(Evidence::reported(None, confidential_meta))
        .unwrap();
    d.record_evidence(Evidence::reported(None, public_meta))
        .unwrap();

    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Public,
            },
            Instant::now(),
        )
        .unwrap();
    assert_eq!(report.truncation.withheld_confidential, 1);
    assert_eq!(report.evidence.len(), 1);
    assert_eq!(
        report.evidence[0].evidence.domain(),
        "open.domain",
        "confidential item excluded wholesale, not redacted"
    );
    assert_eq!(report.disclosure, Confidentiality::Public);
    assert_eq!(report.schema_version, REPORT_SCHEMA_VERSION);
    assert_ne!(report.notes.len(), 0);
}

#[test]
fn report_truncates_findings_with_structured_count() {
    let mut config = DiagnosticsConfig::default();
    config.report.max_findings = 1;
    let mut d = Diagnostics::new(config).unwrap();
    let now = Instant::now();
    for name in ["f.a", "f.b", "f.c"] {
        d.record_finding(finding(
            name,
            "dom",
            FindingSeverity::Warning,
            FindingCertainty::Confirmed,
            &[],
            now,
        ))
        .unwrap();
    }
    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            now,
        )
        .unwrap();
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.truncation.findings_dropped, 2);
}

#[test]
fn report_enforces_byte_budget() {
    let mut config = DiagnosticsConfig::default();
    config.report.max_bytes = 1; // smaller than any single entry
    let mut d = Diagnostics::new(config).unwrap();
    d.record_evidence(Evidence::reported(None, meta("d", Some("k"))))
        .unwrap();
    d.record_evidence(Evidence::reported(None, meta("d", Some("k2"))))
        .unwrap();

    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            Instant::now(),
        )
        .unwrap();
    assert!(report.evidence.is_empty());
    assert_eq!(report.truncation.evidence_dropped, 2);
    assert!(!report.truncation.deadline_exceeded);
}

#[test]
fn report_enforces_deadline() {
    let mut config = DiagnosticsConfig::default();
    config.report.deadline = Some(Duration::ZERO);
    let mut d = Diagnostics::new(config).unwrap();
    d.record_evidence(Evidence::reported(None, meta("d", Some("k"))))
        .unwrap();

    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            Instant::now(),
        )
        .unwrap();
    assert!(report.evidence.is_empty());
    assert_eq!(report.truncation.evidence_dropped, 1);
    assert!(report.truncation.deadline_exceeded);
}

#[test]
fn report_counts_unresolved_evidence_refs() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    let now = Instant::now();
    // Finding cites a ref that was never recorded (or evicted).
    d.record_finding(finding(
        "orphan",
        "dom",
        FindingSeverity::Warning,
        FindingCertainty::Inferred,
        &[EvidenceRef::mint(999)],
        now,
    ))
    .unwrap();
    let report = d
        .report(
            &ReportRequest {
                disclosure: Confidentiality::Internal,
            },
            now,
        )
        .unwrap();
    assert_eq!(report.truncation.unresolved_evidence_refs, 1);
}

#[test]
fn config_rejects_zero_sink_window() {
    let mut config = DiagnosticsConfig::default();
    config.sink_policy.window = Duration::ZERO;
    assert!(matches!(
        Diagnostics::new(config),
        Err(DiagnosticsConfigError::ZeroSinkWindow)
    ));
}

#[test]
fn missing_rules_are_counted_in_snapshot() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    d.add_health_rule("dom", gauge_rule("absent.key", 5.0, None, 1, 1))
        .unwrap();
    let snap = d.evaluate(&snapshot_of(vec![]), Instant::now());
    assert_eq!(snap.missing_rules, 1);
    assert_eq!(snap.domains[0].state, HealthState::Unknown);
}

#[test]
fn diagnostics_health_is_fixed_size_and_non_recursive() {
    let mut d = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    d.add_health_rule("dom", gauge_rule("k", 5.0, None, 1, 1))
        .unwrap();
    let bad = vec![gauge_measurement(
        "k",
        ident(1, 0),
        6,
        MeasurementValidity::Valid,
    )];
    d.evaluate(&snapshot_of(bad), Instant::now());
    let h = d.health();
    assert_eq!(h.evaluations_total, 1);
    assert_eq!(h.domain_transitions_total, 1);
    assert_eq!(h.rule_transitions_total, 1);
    // DiagnosticsHealth itself is never registered as an observation
    // source — no self-feeding loop is possible.
    assert_eq!(h.evidence_retained, 0);
}
