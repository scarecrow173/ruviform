//! Collector contract tests — the Issue #289 golden cases: baseline/
//! delta/rate/discontinuity, gauge zero-vs-unavailable, histogram
//! buckets, lifecycle drain and forced-drain validity, window
//! semantics, staleness, batch limits, bounded history, series
//! tombstones and `PollRunner` lifecycle.
//!
//! Time control: `poll` takes a caller `now: Instant`, so tests drive
//! deterministic windows with `Instant::now() + Duration` offsets —
//! the registration `Instant::now()` anchors are unknown only for the
//! first poll, which these tests never assert a rate on.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::collector::{
    Collector, CollectorConfig, CollectorConfigError, CollectorState, PollError,
};
use crate::core::{Counter, Gauge, Histogram, counter, counter_reader_on_cell, gauge, histogram};
use crate::metrics::{MeasurementIssue, MeasurementValidity, MetricValue};
use crate::source::{
    AggregationPolicy, CounterDescriptor, CounterSource, MetricDescriptor, MetricKind,
    MetricReader, MetricSource, ObservationRegistrar, ObservationScope, ObservationUnit,
    RegistrationError, RegistrationLease, ReportingHint, SourceLifecycle, SourceSemantics,
};

fn config() -> CollectorConfig {
    CollectorConfig {
        poll_interval: Duration::from_millis(10),
        ..CollectorConfig::default()
    }
}

fn counter_source(key: &'static str) -> (Counter, MetricSource) {
    let (counter, reader) = counter();
    let source = MetricSource::from(CounterSource {
        descriptor: CounterDescriptor {
            key,
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
        },
        scope: ObservationScope::Process,
        reader,
    });
    (counter, source)
}

fn gauge_source(key: &'static str, initial: u64) -> (Gauge, MetricSource) {
    let (gauge, reader) = gauge(initial);
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Gauge,
            key,
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::LatestValue,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Gauge(reader),
    )
    .unwrap();
    (gauge, source)
}

fn histogram_source(key: &'static str, boundaries: &'static [u64]) -> (Histogram, MetricSource) {
    let (histogram, reader) = histogram(boundaries).unwrap();
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Histogram,
            key,
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::BucketCounts,
            buckets: Some(boundaries),
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Histogram(reader),
    )
    .unwrap();
    (histogram, source)
}

fn counter_value(m: &crate::metrics::Measurement) -> (u64, u64, Option<u64>, Option<f64>) {
    match &m.value {
        Some(MetricValue::Counter {
            raw,
            cumulative_since_registration,
            delta,
            rate_per_sec,
        }) => (*raw, *cumulative_since_registration, *delta, *rate_per_sec),
        other => panic!("expected counter measurement, got {other:?}"),
    }
}

// --- configuration -----------------------------------------------------------

#[test]
fn config_rejects_zero_poll_interval() {
    let mut c = config();
    c.poll_interval = Duration::ZERO;
    assert_eq!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::ZeroPollInterval
    );
}

#[test]
fn config_rejects_zero_limits() {
    let mut c = config();
    c.max_sources = 0;
    assert_eq!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::ZeroMaxSources
    );
    let mut c = config();
    c.max_series = 0;
    assert_eq!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::ZeroMaxSeries
    );
    let mut c = config();
    c.max_batch_size = 0;
    assert_eq!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::ZeroMaxBatchSize
    );
}

#[test]
fn config_rejects_zero_stale_threshold_and_series_below_sources() {
    let mut c = config();
    c.stale_after = Some(Duration::ZERO);
    assert_eq!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::ZeroStaleThreshold
    );
    let mut c = config();
    c.max_series = c.max_sources - 1;
    assert!(matches!(
        Collector::new(c).unwrap_err(),
        CollectorConfigError::SeriesBelowSources { .. }
    ));
}

#[test]
fn zero_history_budgets_disable_history_without_error() {
    let mut c = config();
    c.history_max_samples = 0;
    c.history_budget_bytes = 0;
    let mut collector = Collector::new(c).unwrap();
    let (_, source) = counter_source("test.counter");
    let port = collector.registration_port();
    port.register(source).unwrap();
    collector.poll(Instant::now()).unwrap();
    assert!(collector.history().is_empty());
    assert_eq!(collector.health().history_len, 0);
}

// --- registration --------------------------------------------------------------

#[test]
fn registration_rejects_empty_key() {
    let collector = Collector::new(config()).unwrap();
    let (_, reader) = counter();
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Counter(reader),
    )
    .unwrap();
    let port = collector.registration_port();
    assert!(matches!(
        port.register(source).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));
    drop(collector);
}

#[test]
fn registration_rejects_wrong_aggregation_and_stray_buckets() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();

    let (_, reader) = gauge(0);
    let gauge_as_cumulative = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Gauge,
            key: "test.gauge.bad.agg",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Gauge(reader),
    )
    .unwrap();
    assert!(matches!(
        port.register(gauge_as_cumulative).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));

    // Counter with buckets declared — meaningless for non-histograms.
    let (_counter, reader) = counter();
    let stray = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "test.counter.buckets",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::Cumulative,
            buckets: Some(&[1, 10]),
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Counter(reader),
    )
    .unwrap();
    assert!(matches!(
        port.register(stray).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));
    drop(collector);
}

#[test]
fn registration_rejects_histogram_descriptor_errors() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();

    // Missing buckets entirely.
    let (_, reader) = histogram(&[1, 10]).unwrap();
    let missing = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Histogram,
            key: "test.hist.missing",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::BucketCounts,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Histogram(reader),
    )
    .unwrap();
    assert!(matches!(
        port.register(missing).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));

    // Same bucket *count*, different boundary *values* — only the
    // by-value check catches this.
    let (_, reader) = histogram(&[1, 10]).unwrap();
    let mismatched = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Histogram,
            key: "test.hist.mismatch",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::BucketCounts,
            buckets: Some(&[1, 20]),
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Histogram(reader),
    )
    .unwrap();
    assert!(matches!(
        port.register(mismatched).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));
    drop(collector);
}

#[test]
fn registration_rejects_live_key_conflict() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, s1) = counter_source("test.counter.dup");
    let (_, s2) = counter_source("test.counter.dup");
    port.register(s1).unwrap();
    assert!(matches!(
        port.register(s2).unwrap_err(),
        RegistrationError::KeyConflict { .. }
    ));
    drop(collector);
}

#[test]
fn registration_rejects_cell_reuse_while_live() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_counter, reader) = counter();
    // Same reader cell under a *different* key — the cell is still
    // claimed by the live registration, so this is SourceReused. (The
    // same key would hit KeyConflict first.)
    let make = |key: &'static str, reader: &crate::core::CounterReader| {
        MetricSource::from(CounterSource {
            descriptor: CounterDescriptor {
                key,
                unit: ObservationUnit::Count,
                reporting: ReportingHint::SnapshotOnly,
            },
            scope: ObservationScope::Process,
            reader: reader.clone(),
        })
    };
    port.register(make("test.counter.a", &reader)).unwrap();
    assert!(matches!(
        port.register(make("test.counter.b", &reader)).unwrap_err(),
        RegistrationError::SourceReused
    ));
    drop(collector);
}

#[test]
fn registration_enforces_max_sources() {
    let mut c = config();
    c.max_sources = 1;
    let collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, s1) = counter_source("test.counter.one");
    let (_, s2) = counter_source("test.counter.two");
    port.register(s1).unwrap();
    assert!(matches!(
        port.register(s2).unwrap_err(),
        RegistrationError::CapacityExceeded { .. }
    ));
    drop(collector);
}

// --- counter semantics ---------------------------------------------------------

#[test]
fn counter_baseline_delta_rate_and_cumulative() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.baseline");
    counter.increment();
    counter.increment();
    let lease = port.register(source).unwrap();

    // Baseline established at registration: pre-registration
    // increments are not re-reported.
    let t0 = Instant::now();
    let b1 = collector.poll(t0).unwrap();
    let m1 = b1.measurements().first().unwrap();
    let (raw, cumulative, delta, _rate) = counter_value(m1);
    assert_eq!(raw, 2);
    assert_eq!(delta, Some(0));
    assert_eq!(cumulative, 0);

    // Second poll: 3 increments over a deterministic 2-second window.
    counter.increment();
    counter.increment();
    counter.increment();
    let b2 = collector.poll(t0 + Duration::from_secs(2)).unwrap();
    let m2 = b2.measurements().first().unwrap();
    let (raw, cumulative, delta, rate) = counter_value(m2);
    assert_eq!(raw, 5);
    assert_eq!(delta, Some(3));
    assert_eq!(cumulative, 3);
    let rate = rate.unwrap();
    assert!((rate - 1.5).abs() < 0.01, "rate ≈ 1.5/s, got {rate}");
    assert_eq!(m2.validity, MeasurementValidity::Valid);
    drop(lease);
    drop(collector);
}

#[test]
fn counter_decrease_is_discontinuous_and_rebaselines() {
    let cell = Arc::new(AtomicU64::new(10));
    let reader = counter_reader_on_cell(&cell);
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "test.counter.decrease",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Counter(reader),
    )
    .unwrap();

    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();

    // Cell decreases — reset/wrap suspicion: typed, never a huge
    // wrapped delta.
    cell.store(3, Ordering::Relaxed);
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    let m = b.measurements().first().unwrap();
    let (raw, cumulative, delta, rate) = counter_value(m);
    assert_eq!(raw, 3);
    assert_eq!(delta, None);
    assert_eq!(rate, None);
    assert_eq!(cumulative, 0);
    assert_eq!(m.validity, MeasurementValidity::Discontinuous);
    assert_eq!(m.issue, Some(MeasurementIssue::CounterDecreased));
    assert_eq!(collector.health().discontinuities_total, 1);

    // Re-baselined — next increase reports a normal delta again, and
    // `cumulative_since_registration` keeps accumulating across the
    // discontinuity rather than resetting.
    cell.store(8, Ordering::Relaxed);
    let b = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    let m = b.measurements().first().unwrap();
    let (raw, cumulative, delta, _rate) = counter_value(m);
    assert_eq!(raw, 8);
    assert_eq!(delta, Some(5));
    assert_eq!(cumulative, 5);
    assert_eq!(m.validity, MeasurementValidity::Valid);
    drop(lease);
    drop(collector);
}

#[test]
fn re_registered_key_gets_new_identity_and_generation() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, s1) = counter_source("test.counter.regen");
    let lease1 = port.register(s1).unwrap();
    let id1 = lease1.identity();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let ticket = lease1.retire();
    collector.poll(t0 + Duration::from_millis(10)).unwrap();
    assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);

    // Same key, same kind — new registration with generation + 1.
    let (_, s2) = counter_source("test.counter.regen");
    let lease2 = port.register(s2).unwrap();
    let id2 = lease2.identity();
    assert_ne!(id1.id(), id2.id());
    assert_eq!(id1.generation().get() + 1, id2.generation().get());

    // The new registration starts with a fresh baseline — the cell
    // value resets conceptually.
    let b = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    let m = b.measurements().iter().find(|m| m.identity == id2).unwrap();
    assert_eq!(m.validity, MeasurementValidity::Valid);
    drop(lease2);
    drop(collector);
}

#[test]
fn re_registering_removed_key_with_different_kind_is_rejected() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, s1) = counter_source("test.key.kind");
    let lease = port.register(s1).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let _ = lease.retire();
    collector.poll(t0 + Duration::from_millis(10)).unwrap();

    let (_, gauge_as_key) = gauge_source("test.key.kind", 0);
    assert!(matches!(
        port.register(gauge_as_key).unwrap_err(),
        RegistrationError::InvalidDescriptor { .. }
    ));
    drop(collector);
}

#[test]
fn removed_cell_may_register_again_as_new_series() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_counter, reader) = counter();
    let descriptor = || CounterDescriptor {
        key: "test.counter.cell",
        unit: ObservationUnit::Count,
        reporting: ReportingHint::SnapshotOnly,
    };
    let s1 = MetricSource::from(CounterSource {
        descriptor: descriptor(),
        scope: ObservationScope::Process,
        reader: reader.clone(),
    });
    let lease = port.register(s1).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let _ = lease.retire();
    collector.poll(t0 + Duration::from_millis(10)).unwrap();

    // The cell claim was released at Removed — the same cell now
    // registers as a brand-new series (fresh baseline).
    let s2 = MetricSource::from(CounterSource {
        descriptor: descriptor(),
        scope: ObservationScope::Process,
        reader,
    });
    port.register(s2).unwrap();
    drop(collector);
}

// --- gauge ---------------------------------------------------------------------

#[test]
fn gauge_zero_is_an_observed_value_not_unavailable() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_gauge, source) = gauge_source("test.gauge.zero", 0);
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    let b = collector.poll(t0).unwrap();
    let m = b.measurements().first().unwrap();
    match &m.value {
        Some(MetricValue::Gauge { value, previous }) => {
            assert_eq!(*value, 0);
            assert_eq!(*previous, None);
        }
        other => panic!("expected gauge, got {other:?}"),
    }
    assert_eq!(m.validity, MeasurementValidity::Valid);
    drop(lease);
    drop(collector);
}

#[test]
fn gauge_tracks_previous_and_latest() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (gauge, source) = gauge_source("test.gauge.prev", 1);
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    gauge.set(7);
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    let m = b.measurements().first().unwrap();
    match &m.value {
        Some(MetricValue::Gauge { value, previous }) => {
            assert_eq!(*value, 7);
            assert_eq!(*previous, Some(1));
        }
        other => panic!("expected gauge, got {other:?}"),
    }
    drop(lease);
    drop(collector);
}

#[test]
fn removed_gauge_reads_unavailable_in_snapshot_but_keeps_history() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_gauge, source) = gauge_source("test.gauge.removed", 42);
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let ticket = lease.retire();
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    assert!(b.removed().contains(&identity));

    // Snapshot tombstone: Unavailable, value None — not zero.
    let snap = b.snapshot();
    let tomb = snap.get(identity).unwrap();
    assert_eq!(tomb.validity, MeasurementValidity::Unavailable);
    assert!(tomb.value.is_none());
    assert_eq!(tomb.lifecycle, SourceLifecycle::Removed);

    // History retains the final observed value.
    let history = collector.series(identity);
    let last = history.last().unwrap();
    match &last.value {
        Some(MetricValue::Gauge { value, .. }) => assert_eq!(*value, 42),
        other => panic!("expected gauge, got {other:?}"),
    }
    drop(collector);
}

// --- histogram -------------------------------------------------------------------

#[test]
fn histogram_reports_bucket_counts_and_approx_total() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (histogram, source) = histogram_source("test.hist.basic", &[1, 10, 100]);
    histogram.observe(5);
    histogram.observe(50);
    histogram.observe(50);
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    let b = collector.poll(t0).unwrap();
    let m = b.measurements().first().unwrap();
    match &m.value {
        Some(MetricValue::Histogram {
            buckets,
            total_approx,
        }) => {
            assert_eq!(&**buckets, &[0, 1, 2, 0]);
            assert_eq!(*total_approx, 3);
        }
        other => panic!("expected histogram, got {other:?}"),
    }
    drop(lease);
    drop(collector);
}

// --- window ----------------------------------------------------------------------

#[test]
fn window_generations_chain_started_at_previous_ended() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.window");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    let b1 = collector.poll(t0).unwrap();
    assert_eq!(b1.generation(), 1);
    assert_eq!(b1.window().ended, t0);
    assert!(b1.window().started <= t0);
    assert_eq!(b1.window().measured, t0 - b1.window().started);
    let b2 = collector.poll(t0 + Duration::from_millis(30)).unwrap();
    assert_eq!(b2.generation(), 2);
    assert_eq!(b2.window().started, b1.window().ended);
    assert_eq!(b2.window().measured, Duration::from_millis(30));
    drop(collector);
}

#[test]
fn overdue_poll_is_counted() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.overdue");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    // measured = 50ms > expected = 10ms.
    collector.poll(t0 + Duration::from_millis(50)).unwrap();
    let health = collector.health();
    assert_eq!(health.polls_total, 2);
    assert_eq!(health.overdue_polls_total, 1);
    drop(collector);
}

#[test]
fn backwards_now_is_a_clock_anomaly_not_a_panic() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.clock");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let b = collector
        .poll(t0.checked_sub(Duration::from_millis(10)).unwrap())
        .unwrap();
    assert!(b.window().clock_anomaly);
    assert_eq!(b.window().measured, Duration::ZERO);
    assert_eq!(collector.health().clock_anomalies_total, 1);
    let m = b.measurements().first().unwrap();
    assert_eq!(m.issue, Some(MeasurementIssue::ClockAnomaly));
    drop(collector);
}

// --- staleness --------------------------------------------------------------------

#[test]
fn unchanged_source_turns_stale_past_threshold() {
    let mut c = config();
    c.stale_after = Some(Duration::from_millis(10));
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.stale");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let b = collector.poll(t0 + Duration::from_millis(30)).unwrap();
    let m = b.measurements().first().unwrap();
    assert_eq!(m.validity, MeasurementValidity::Stale);
    drop(collector);
}

#[test]
fn changed_source_stays_valid() {
    let mut c = config();
    c.stale_after = Some(Duration::from_millis(10));
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.fresh");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    counter.increment();
    let b = collector.poll(t0 + Duration::from_millis(30)).unwrap();
    let m = b.measurements().first().unwrap();
    assert_eq!(m.validity, MeasurementValidity::Valid);
    drop(collector);
}

#[test]
fn no_stale_threshold_means_never_stale() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.nostale");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let b = collector.poll(t0 + Duration::from_secs(3600)).unwrap();
    let m = b.measurements().first().unwrap();
    assert_eq!(m.validity, MeasurementValidity::Valid);
    drop(collector);
}

// --- lifecycle drain ----------------------------------------------------------------

#[test]
fn domain_retired_source_drains_valid_final_value() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.retire");
    counter.increment();
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    counter.increment();
    let ticket = lease.retire();
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    let m = b.measurements().first().unwrap();
    // Domain-declared quiescence → the final value is fully Valid.
    assert_eq!(m.validity, MeasurementValidity::Valid);
    assert_eq!(m.lifecycle, SourceLifecycle::Removed);
    let (_, _, delta, _) = counter_value(m);
    assert_eq!(delta, Some(1));
    drop(collector);
}

#[test]
fn abandoned_source_drains_as_partial_writer_not_quiesced() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.abandoned");
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    drop(lease); // abandon — no quiescence declaration.
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    let m = b.measurements().first().unwrap();
    assert_eq!(m.validity, MeasurementValidity::Partial);
    assert_eq!(m.issue, Some(MeasurementIssue::WriterNotQuiesced));
    assert_eq!(collector.health().abandoned_sources_total, 1);
    drop(collector);
}

#[test]
fn shutdown_forces_partial_drain_stops_polls_and_registrations() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.shutdown");
    counter.increment();
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let handle = collector.handle();

    collector.shutdown();
    assert_eq!(handle.state(), CollectorState::Stopped);

    // Forced drain committed the last value as Partial — quiescence
    // was never confirmed. The lease drop afterwards is a no-op.
    let history = collector.series(identity);
    let last = history.last().unwrap();
    assert_eq!(last.validity, MeasurementValidity::Partial);
    assert_eq!(last.issue, Some(MeasurementIssue::WriterNotQuiesced));
    drop(lease);

    // Poll and register are now typed rejections.
    assert!(matches!(
        collector.poll(Instant::now()),
        Err(PollError::CollectorStopped)
    ));
    let (_, source2) = counter_source("test.counter.late");
    assert!(matches!(
        port.register(source2).unwrap_err(),
        RegistrationError::RegistrarStopped
    ));

    // The final snapshot remains readable.
    let snap = handle.snapshot();
    let tomb = snap.get(identity).unwrap();
    assert_eq!(tomb.validity, MeasurementValidity::Unavailable);
    // latest_batch is still the last committed batch.
    assert!(handle.latest_batch().is_some());
    drop(collector);
}

#[test]
fn collector_drop_drains_sources() {
    let (handle, port) = {
        let collector = Collector::new(config()).unwrap();
        let handle = collector.handle();
        let port = collector.registration_port();
        let (_, source) = counter_source("test.counter.drop");
        port.register(source).unwrap();
        (handle, port)
    }; // collector dropped here — shutdown drains all sources.
    assert_eq!(handle.state(), CollectorState::Stopped);
    assert_eq!(handle.health().sources_removed_total, 1);
    let (_, source) = counter_source("test.counter.postdrop");
    assert!(matches!(
        port.register(source).unwrap_err(),
        RegistrationError::RegistrarStopped
    ));
}

#[test]
fn shutdown_is_idempotent_and_concurrent() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.concurrent_shutdown");
    let _lease = port.register(source).unwrap();
    let handle = collector.handle();

    let mut threads = Vec::new();
    for _ in 0..4 {
        let h = handle.clone();
        threads.push(thread::spawn(move || h.shutdown()));
    }
    handle.shutdown();
    collector.shutdown();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(handle.state(), CollectorState::Stopped);
    assert_eq!(handle.health().sources_removed_total, 1);
    // A repeated shutdown is a no-op — the terminal generation must
    // not mint again.
    let generation = handle.latest_batch().unwrap().generation();
    handle.shutdown();
    collector.shutdown();
    assert_eq!(handle.latest_batch().unwrap().generation(), generation);
    drop(collector);
}

// --- batch bounds / round-robin ---------------------------------------------------------

#[test]
fn max_batch_size_defers_active_sources_fairly() {
    let mut c = config();
    c.max_batch_size = 1;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (c1, s1) = counter_source("test.counter.rr1");
    let (_c2, s2) = counter_source("test.counter.rr2");
    let _l1 = port.register(s1).unwrap();
    let _l2 = port.register(s2).unwrap();

    let t0 = Instant::now();
    let b1 = collector.poll(t0).unwrap();
    assert_eq!(b1.measurements().len(), 1);
    assert_eq!(b1.window().skipped, 1);
    let seen1 = b1.measurements()[0].identity;

    c1.increment();
    let b2 = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    assert_eq!(b2.measurements().len(), 1);
    let seen2 = b2.measurements()[0].identity;
    // Round-robin: the *other* source was observed this poll.
    assert_ne!(seen1, seen2);

    let b3 = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    assert_eq!(b3.measurements()[0].identity, seen1);
    drop(collector);
}

#[test]
fn retiring_source_is_always_drained_over_batch_limit() {
    let mut c = config();
    c.max_batch_size = 1;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, s1) = counter_source("test.counter.drain1");
    let (_, s2) = counter_source("test.counter.drain2");
    let l1 = port.register(s1).unwrap();
    let _l2 = port.register(s2).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let ticket = l1.retire();
    let b = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    // The retiree drains regardless of the batch budget.
    assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    assert!(b.removed().contains(&ticket.identity()));
    // One drain + the single budgeted active.
    assert_eq!(b.measurements().len(), 2);
    drop(collector);
}

// --- history ------------------------------------------------------------------------

#[test]
fn history_is_bounded_by_sample_count() {
    let mut c = config();
    c.history_max_samples = 2;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.hist");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    for i in 0..4u64 {
        collector.poll(t0 + Duration::from_millis(10 * i)).unwrap();
    }
    let history = collector.history();
    assert_eq!(history.len(), 2);
    assert_eq!(collector.health().history_evicted_total, 2);
    drop(collector);
}

#[test]
fn oversized_history_entry_is_dropped_but_batched() {
    let mut c = config();
    c.history_budget_bytes = 1; // every entry exceeds this.
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.oversized");
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    let b = collector.poll(t0).unwrap();
    assert_eq!(b.measurements().len(), 1);
    assert!(collector.history().is_empty());
    assert_eq!(collector.health().history_oversized_dropped_total, 1);
    drop(collector);
}

// --- series tombstones ----------------------------------------------------------------

#[test]
fn tombstones_evict_oldest_when_series_budget_is_full() {
    let mut c = config();
    c.max_sources = 2;
    c.max_series = 2;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let t0 = Instant::now();

    let (_, s1) = counter_source("test.series.a");
    let l1 = port.register(s1).unwrap();
    let id_a = l1.identity();
    collector.poll(t0).unwrap();
    let _ = l1.retire();
    collector.poll(t0 + Duration::from_millis(10)).unwrap();

    let (_, s2) = counter_source("test.series.b");
    let l2 = port.register(s2).unwrap();
    collector.poll(t0 + Duration::from_millis(20)).unwrap();
    let _ = l2.retire();
    collector.poll(t0 + Duration::from_millis(30)).unwrap();

    let (_, s3) = counter_source("test.series.c");
    let l3 = port.register(s3).unwrap();
    collector.poll(t0 + Duration::from_millis(40)).unwrap();
    let _ = l3.retire();
    collector.poll(t0 + Duration::from_millis(50)).unwrap();

    let snap = collector.snapshot();
    // Series A's tombstone was evicted; B and C remain.
    assert!(snap.get(id_a).is_none());
    assert_eq!(snap.len(), 2);
    assert_eq!(collector.health().removed_series_evicted_total, 1);
    drop(collector);
}

// --- handle / batch sharing --------------------------------------------------------------

#[test]
fn latest_batch_is_none_before_first_poll() {
    let collector = Collector::new(config()).unwrap();
    assert!(collector.latest_batch().is_none());
    assert!(collector.handle().latest_batch().is_none());
    assert_eq!(collector.snapshot().generation(), 0);
    drop(collector);
}

#[test]
fn batch_and_snapshot_share_one_generation_across_clones() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.shared");
    let _lease = port.register(source).unwrap();
    let b = collector.poll(Instant::now()).unwrap();
    let b2 = b.clone();
    let snap1 = b.snapshot();
    let snap2 = collector.snapshot();
    assert_eq!(snap1.generation(), snap2.generation());
    assert_eq!(b.generation(), b2.generation());
    assert_eq!(snap1.len(), snap2.len());
    drop(collector);
}

#[test]
fn handles_are_cloneable_send_sync_and_read_only() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<crate::collector::CollectorHandle>();
    assert_send_sync::<crate::collector::CollectorRegistrar>();
    assert_send_sync::<crate::collector::MetricsSnapshot>();
    assert_send_sync::<crate::collector::ObservationBatch>();
    assert_send_sync::<RegistrationLease>();

    let collector = Collector::new(config()).unwrap();
    let h1 = collector.handle();
    let h2 = h1.clone();
    assert_eq!(h1.state(), h2.state());
    drop(collector);
}

// --- PollRunner ----------------------------------------------------------------------------

#[test]
fn poll_runner_polls_in_background_and_stops() {
    let mut c = config();
    c.poll_interval = Duration::from_millis(5);
    let collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.runner");
    let _lease = port.register(source).unwrap();
    let runner = collector.into_poll_runner();
    let handle = runner.handle();
    thread::sleep(Duration::from_millis(40));
    runner.stop().unwrap();
    assert!(handle.snapshot().generation() >= 1);
    assert!(handle.latest_batch().is_some());
    assert_eq!(handle.state(), CollectorState::Stopped);
}

#[test]
fn poll_runner_drains_remaining_sources_on_stop() {
    let mut c = config();
    c.poll_interval = Duration::from_millis(5);
    let collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.runner_drain");
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let runner = collector.into_poll_runner();
    let handle = runner.handle();
    runner.stop().unwrap();
    assert_eq!(handle.state(), CollectorState::Stopped);
    assert_eq!(handle.health().sources_removed_total, 1);
    let snap = handle.snapshot();
    let tomb = snap.get(identity).unwrap();
    assert_eq!(tomb.validity, MeasurementValidity::Unavailable);
}

#[test]
fn runner_exposes_registration_port() {
    let collector = Collector::new(config()).unwrap();
    let runner = collector.into_poll_runner();
    let port = runner.registration_port();
    let (_, source) = counter_source("test.counter.runner_reg");
    port.register(source).unwrap();
    runner.stop().unwrap();
}

// --- concurrency -------------------------------------------------------------------------

#[test]
fn register_retire_and_poll_race_without_deadlock() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let handle = collector.handle();

    let mut threads = Vec::new();
    // Registrar threads: register → retire in a fixed-size loop.
    for n in 0..3usize {
        let port = port.clone();
        threads.push(thread::spawn(move || {
            // Keys are 'static; cycle a small fixed set.
            const KEYS: &[&str] = &["test.race.a", "test.race.b", "test.race.c", "test.race.d"];
            for i in 0..30usize {
                let (_, source) = counter_source(KEYS[(n + i) % KEYS.len()]);
                if let Ok(lease) = port.register(source) {
                    let _ = lease.retire();
                }
            }
        }));
    }
    // Poll concurrently — yields give the spawned threads CPU time so
    // registrations actually interleave with drains.
    let t0 = Instant::now();
    for i in 0..50u64 {
        let _ = collector.poll(t0 + Duration::from_millis(i));
        thread::yield_now();
    }
    for t in threads {
        t.join().unwrap();
    }
    // Registrations succeeded somewhere in the interleave — no
    // deadlock, and health stays coherent.
    assert!(handle.health().registered_total > 0);
    drop(collector);
}

#[test]
fn abandoned_leases_racing_shutdown_still_drain_partial() {
    // Issue #275 acceptance: quiescence → final drain → unregister
    // stays safe *including races*. Every lease here is dropped
    // without `retire` (abandon) on its own thread while the main
    // thread shuts the collector down — whichever side of the race
    // each drop lands on, the source must drain as `Partial` +
    // `WriterNotQuiesced` (quiescence was never declared) and reach
    // `Removed` exactly once.
    const KEYS: &[&str] = &[
        "test.abandon.race.a",
        "test.abandon.race.b",
        "test.abandon.race.c",
        "test.abandon.race.d",
        "test.abandon.race.e",
        "test.abandon.race.f",
        "test.abandon.race.g",
        "test.abandon.race.h",
    ];
    const SOURCES: usize = KEYS.len();

    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let handle = collector.handle();

    let mut leases = Vec::with_capacity(SOURCES);
    for key in KEYS {
        let (_, source) = counter_source(key);
        leases.push(port.register(source).unwrap());
    }

    let t0 = Instant::now();
    collector.poll(t0).unwrap(); // baseline for every source

    let mut threads = Vec::with_capacity(SOURCES);
    for lease in leases {
        threads.push(thread::spawn(move || {
            // Widen the race window — the drop may land before,
            // during, or after the shutdown drain pass.
            thread::yield_now();
            drop(lease);
        }));
    }
    thread::yield_now();
    handle.shutdown();
    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(handle.state(), CollectorState::Stopped);
    assert_eq!(handle.health().sources_removed_total, SOURCES as u64);

    // The terminal batch carries one final measurement per source —
    // all Partial because no lease ever declared quiescence.
    let terminal = handle.latest_batch().expect("terminal drain batch");
    assert_eq!(terminal.removed().len(), SOURCES);
    let drained: Vec<_> = terminal
        .measurements()
        .iter()
        .filter(|m| m.lifecycle == SourceLifecycle::Removed)
        .collect();
    assert_eq!(drained.len(), SOURCES);
    for m in drained {
        assert_eq!(m.validity, MeasurementValidity::Partial);
        assert_eq!(m.issue, Some(MeasurementIssue::WriterNotQuiesced));
    }
    drop(collector);
}

#[test]
fn mutex_poison_recovers_and_is_counted() {
    let mut collector = Collector::new(config()).unwrap();
    // A panic while holding the guard poisons the mutex; once
    // poisoned, EVERY subsequent lock recovers via `into_inner` and
    // is counted, so the counter tracks lock-recovery events.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        collector.poison_state_for_test();
    }));
    // health() locks once → 1 recovery; poll() locks once → 2; the
    // final health() locks once more → 3, proving the collector stays
    // fully operational after the poison.
    assert_eq!(collector.health().poison_recoveries_total, 1);
    assert!(collector.poll(Instant::now()).is_ok());
    assert_eq!(collector.health().poison_recoveries_total, 3);
    drop(collector);
}

#[test]
fn mid_poll_retire_still_drains_this_poll() {
    // A lock-free retire between enumerate and commit must not be
    // stranded until the next poll.
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.midpoll");
    let lease = port.register(source).unwrap();
    let ticket = lease.retire(); // already Retiring when enumerated.
    let b = collector.poll(Instant::now()).unwrap();
    assert_eq!(ticket.lifecycle(), SourceLifecycle::Removed);
    assert!(b.removed().contains(&ticket.identity()));
    drop(collector);
}

// --- terminal drain batch -----------------------------------------------------
// The shutdown/drop final-drain pass is an observation commit like any
// poll: it mints one generation carrying the drain measurements, the
// `removed` identities and the post-commit snapshot — so the final
// delta/`Removed` event stays recoverable through `latest_batch()`/
// `snapshot()` even when history retention is disabled
// (`history_max_samples == 0` / `history_budget_bytes == 0`).

#[test]
fn shutdown_drain_publishes_terminal_batch_with_history_disabled() {
    let mut c = config();
    c.history_max_samples = 0;
    c.history_budget_bytes = 0;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.term_hist0");
    counter.increment();
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let t0 = Instant::now();
    let b1 = collector.poll(t0).unwrap();
    assert_eq!(b1.generation(), 1);
    counter.increment();
    counter.increment();
    let ticket = lease.retire(); // domain retire — writer quiesced.
    let handle = collector.handle();
    // Keep the drain's elapsed anchor measurably positive on coarse
    // timers so the final delta/rate is not ClockAnomaly-suppressed.
    thread::sleep(Duration::from_millis(2));

    collector.shutdown();
    assert_eq!(handle.state(), CollectorState::Stopped);

    // Terminal generation minted — the final delta and the Removed
    // event are recoverable from the batch itself.
    let terminal = handle.latest_batch().expect("terminal drain batch");
    assert_eq!(terminal.generation(), 2);
    assert_eq!(terminal.removed(), &[identity]);
    assert_eq!(terminal.measurements().len(), 1);
    let m = &terminal.measurements()[0];
    assert_eq!(m.lifecycle, SourceLifecycle::Removed);
    assert_eq!(m.validity, MeasurementValidity::Valid); // domain-retired
    let (raw, _cumulative, delta, _rate) = counter_value(m);
    assert_eq!(raw, 3);
    assert_eq!(delta, Some(2));
    let w = terminal.window();
    assert_eq!(w.generation, 2);
    assert_eq!(w.planned, 1);
    assert_eq!(w.drained, 1);
    assert_eq!(w.observed, 1);
    assert_eq!(w.skipped, 0);
    assert!(!w.clock_anomaly);
    assert!(w.ended >= w.started);

    // Snapshot is rebuilt at the same terminal generation — live
    // series gone, tombstone retained.
    let snap = handle.snapshot();
    assert_eq!(snap.generation(), 2);
    assert_eq!(terminal.snapshot().generation(), 2);
    let tomb = snap.get(identity).unwrap();
    assert_eq!(tomb.validity, MeasurementValidity::Unavailable);

    // History retention is disabled — nothing there; the batch is the
    // carrier that makes the terminal loss impossible.
    assert!(handle.series(identity).is_empty());
    assert!(handle.history().is_empty());
    drop(ticket);
    drop(collector);
}

#[test]
fn forced_shutdown_drain_is_recoverable_without_history() {
    let mut c = config();
    c.history_max_samples = 0;
    c.history_budget_bytes = 0;
    let mut collector = Collector::new(c).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.term_forced");
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    counter.increment();
    counter.increment();
    counter.increment();
    let handle = collector.handle();
    thread::sleep(Duration::from_millis(2));

    // Writer never quiesced — the collector forces the drain and the
    // final delta is still recoverable, marked Partial.
    collector.shutdown();

    let terminal = handle.latest_batch().expect("terminal drain batch");
    assert_eq!(terminal.generation(), 2);
    assert_eq!(terminal.removed(), &[identity]);
    let m = &terminal.measurements()[0];
    assert_eq!(m.lifecycle, SourceLifecycle::Removed);
    assert_eq!(m.validity, MeasurementValidity::Partial);
    assert_eq!(m.issue, Some(MeasurementIssue::WriterNotQuiesced));
    let (raw, _cumulative, delta, _rate) = counter_value(m);
    assert_eq!(raw, 3);
    assert_eq!(delta, Some(3));
    assert!(handle.series(identity).is_empty());
    // Health parity with the drain commit — forced (unquiesced)
    // drain is not abandonment.
    let health = handle.health();
    assert_eq!(health.sources_removed_total, 1);
    assert_eq!(health.abandoned_sources_total, 0);
    assert!(health.samples_total >= 2);
    drop(lease);
    drop(collector);
}

#[test]
fn shutdown_terminal_batch_covers_mixed_sources() {
    // History enabled — the terminal batch still carries every drain
    // (retired → Valid; still-Active → Partial), not just what the
    // ring happened to retain.
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (counter_a, source_a) = counter_source("test.counter.term_a");
    let (_counter_b, source_b) = counter_source("test.counter.term_b");
    let (gauge_c, source_c) = gauge_source("test.gauge.term_c", 7);
    counter_a.increment();
    let lease_a = port.register(source_a).unwrap();
    let lease_b = port.register(source_b).unwrap();
    let lease_c = port.register(source_c).unwrap();
    let id_a = lease_a.identity();
    let id_b = lease_b.identity();
    let id_c = lease_c.identity();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    gauge_c.set(9);
    let _ticket_a = lease_a.retire();
    let handle = collector.handle();
    thread::sleep(Duration::from_millis(2));

    collector.shutdown();

    let terminal = handle.latest_batch().expect("terminal drain batch");
    assert_eq!(terminal.generation(), 2);
    assert_eq!(terminal.removed().len(), 3);
    for id in [id_a, id_b, id_c] {
        assert!(terminal.removed().contains(&id));
    }
    let by_id = |id| {
        terminal
            .measurements()
            .iter()
            .find(|m| m.identity == id)
            .unwrap()
    };
    let ma = by_id(id_a);
    assert_eq!(ma.validity, MeasurementValidity::Valid); // domain retire
    let mb = by_id(id_b);
    assert_eq!(mb.validity, MeasurementValidity::Partial);
    assert_eq!(mb.issue, Some(MeasurementIssue::WriterNotQuiesced));
    let mc = by_id(id_c);
    assert_eq!(mc.validity, MeasurementValidity::Partial);
    assert!(matches!(
        mc.value,
        Some(MetricValue::Gauge { value: 9, .. })
    ));
    assert_eq!(handle.snapshot().generation(), 2);
    drop(lease_b);
    drop(lease_c);
    drop(collector);
}

#[test]
fn shutdown_without_live_sources_mints_empty_terminal_batch() {
    // The terminal pass is still an observation commit — it mints a
    // generation even when nothing was live, so `latest_batch` is
    // always the defined terminal generation after `Stopped`.
    let collector = Collector::new(config()).unwrap();
    let handle = collector.handle();
    collector.shutdown();
    let terminal = handle.latest_batch().expect("empty terminal batch");
    assert_eq!(terminal.generation(), 1);
    assert_eq!(terminal.measurements().len(), 0);
    assert_eq!(terminal.removed(), []);
    assert_eq!(handle.snapshot().generation(), 1);
    drop(collector);
}

#[test]
fn shutdown_after_fully_drained_poll_still_mints_terminal_generation() {
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (_, source) = counter_source("test.counter.term_empty");
    let lease = port.register(source).unwrap();
    let t0 = Instant::now();
    collector.poll(t0).unwrap();
    let ticket = lease.retire();
    let b2 = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    assert!(b2.removed().contains(&ticket.identity()));
    drop(ticket);
    let handle = collector.handle();

    collector.shutdown();

    // The drain batch exists (generation 3) even though the source
    // was already Removed — the terminal generation is defined.
    let terminal = handle.latest_batch().unwrap();
    assert_eq!(terminal.generation(), 3);
    assert_eq!(terminal.removed(), []);
    assert_eq!(terminal.measurements().len(), 0);
    drop(collector);
}

#[test]
fn poll_runner_stop_publishes_terminal_batch() {
    let collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let (counter, source) = counter_source("test.counter.runner_term");
    counter.increment();
    let lease = port.register(source).unwrap();
    let identity = lease.identity();
    let runner = collector.into_poll_runner();
    let handle = runner.handle();

    runner.stop().unwrap();

    assert_eq!(handle.state(), CollectorState::Stopped);
    let terminal = handle.latest_batch().expect("terminal drain batch");
    assert!(terminal.removed().contains(&identity));
    let last = terminal
        .measurements()
        .iter()
        .find(|m| m.identity == identity)
        .unwrap();
    assert_eq!(last.lifecycle, SourceLifecycle::Removed);
    drop(lease);
}
