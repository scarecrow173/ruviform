//! RT timing contract tests — Issue #290 RT Timing slice.
//!
//! Covers the test-spec rules: RT-side `record` no-alloc (R1),
//! drop-on-full loss accounting (R2/R7), clock anomaly separation
//! (R3), fixed clock stamping (R4), budget semantics (R5), histogram +
//! collector integration (R6), window loss identification (R7),
//! quantile interval estimates vs exact stats (R8), generation
//! accounting (R9), and record vocabulary (R10).

use std::time::Instant;

use ruviform_media_core::{ClockDomainId, ClockEpochId, TimeBase, TimestampProvenance};

use crate::collector::{Collector, CollectorConfig};
use crate::metrics::MetricValue;
use crate::record::EventId;
use crate::source::{
    AggregationPolicy, Confidentiality, MetricDescriptor, MetricKind, MetricReader, MetricSource,
    ObservationRegistrar, ObservationScope, ObservationUnit, ReportingHint, ReportingImpact,
    SourceGeneration, SourceId, SourceIdentity, SourceSemantics,
};

use super::*;

/// Fixed histogram boundaries shared by the bridge and the
/// collector-facing `MetricDescriptor` (`&'static` for registration).
const BOUNDS: &[u64] = &[100, 500, 1_000, 5_000];

fn clock() -> TimingClock {
    TimingClock::new(
        ClockDomainId::MONOTONIC,
        TimeBase::new(1, 1_000_000).unwrap(),
        TimestampProvenance::Carried,
        ClockEpochId::new(1),
    )
}

fn source() -> SourceIdentity {
    SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0))
}

fn sample(start_tick: u64, end_tick: u64) -> TimingSample {
    TimingSample {
        point: EventId::new(1),
        generation: 0,
        budget_ticks: 0,
        start_tick,
        end_tick,
    }
}

fn budgeted_sample(
    start_tick: u64,
    end_tick: u64,
    budget_ticks: u64,
    generation: u64,
) -> TimingSample {
    TimingSample {
        point: EventId::new(2),
        generation,
        budget_ticks,
        start_tick,
        end_tick,
    }
}

fn make_bridge() -> (TimingRecorder, TimingSink) {
    timing_bridge(64, clock(), source(), BOUNDS).unwrap()
}

// --- R1: RT-side record is allocation-free -------------------------------

/// RT lane: `record` (accept and drop paths) plus the counter reads
/// perform no allocation inside the measurement window.
#[test]
fn rt_record_allocates_nothing() {
    use assert_no_alloc::assert_no_alloc;

    let (mut recorder, _sink) = timing_bridge(4, clock(), source(), BOUNDS).unwrap();
    assert_no_alloc(|| {
        for i in 0..4_u64 {
            assert!(recorder.record(sample(i * 100, i * 100 + 50)));
        }
        // Full ring: the drop path is also alloc-free.
        assert!(!recorder.record(sample(400, 450)));
        let _ = recorder.dropped();
        let _ = recorder.recorded();
    });
    assert_eq!(recorder.dropped(), 1);
    assert_eq!(recorder.recorded(), 4);
}

// --- R2/R7: drop-on-full loss accounting ----------------------------------

#[test]
fn drop_on_full_is_counted_and_attributed() {
    let (mut recorder, mut sink) = timing_bridge(2, clock(), source(), BOUNDS).unwrap();
    assert!(recorder.record(sample(0, 100)));
    assert!(recorder.record(sample(100, 200)));
    // The ring stays full until the sink drains — every further
    // record drops (drop-on-full, never blocking).
    assert!(!recorder.record(sample(200, 300)));
    assert!(!recorder.record(sample(300, 400)));
    assert!(!recorder.record(sample(400, 500)));
    assert_eq!(recorder.dropped(), 3);
    assert_eq!(recorder.recorded(), 2);

    // First poll drains the two survivors — no loss carrier consumed.
    let first = sink.poll();
    assert_eq!(first.records, 2);
    assert_eq!(first.dropped, 0);

    // The next accepted record carries all three drops.
    assert!(recorder.record(sample(500, 600)));
    let second = sink.poll();
    assert_eq!(second.records, 1);
    assert_eq!(second.dropped, 3);
    assert_eq!(second.samples, 1);

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.records, 3);
    // Authoritative bridge total — equals the positional sum here
    // because every drop found a carrier within this poll.
    assert_eq!(snapshot.dropped_total, 3);
    // The distribution describes the 3 surviving samples only.
    assert_eq!(snapshot.samples, 3);
    let buckets = sink.histogram_reader().snapshot();
    assert_eq!(buckets.iter().sum::<u64>(), 3);
}

#[test]
fn pending_drops_attribute_to_the_carrier_window() {
    let (mut recorder, mut sink) = timing_bridge(2, clock(), source(), BOUNDS).unwrap();
    assert!(recorder.record(sample(0, 100)));
    assert!(recorder.record(sample(100, 200)));
    // Window 1 drains clean — no loss evidence yet.
    let first = sink.poll();
    assert_eq!(first.records, 2);
    assert_eq!(first.dropped, 0);

    // Refill, then drop while full: the pending loss has no carrier
    // until the next *accepted* record.
    assert!(recorder.record(sample(200, 300)));
    assert!(recorder.record(sample(300, 400)));
    assert!(!recorder.record(sample(400, 500)));
    assert!(!recorder.record(sample(500, 600)));
    assert!(!recorder.record(sample(600, 700)));

    // Window 2 drains the two survivors — still no carrier consumed.
    let second = sink.poll();
    assert_eq!(second.records, 2);
    assert_eq!(second.dropped, 0);

    // The carrier lands in window 3 — positional attribution, not
    // temporal: the drops are reported where their evidence arrived.
    assert!(recorder.record(sample(700, 800)));
    let third = sink.poll();
    assert_eq!(third.records, 1);
    assert_eq!(third.dropped, 3);

    // A `dropped == 0` window is not proven complete — the
    // authoritative total already counts the loss.
    let snapshot = sink.snapshot();
    assert_eq!(snapshot.dropped_total, 3);
    assert_eq!(snapshot.records, 5);
    assert_eq!(snapshot.samples, 5);
}

// --- R3: clock anomaly separation -----------------------------------------

#[test]
fn clock_anomaly_is_counted_not_distributed() {
    let (mut recorder, mut sink) = make_bridge();
    assert!(recorder.record(sample(1_000, 500))); // end < start
    assert!(recorder.record(sample(0, 100)));
    assert!(recorder.record(sample(100, 300)));

    let window = sink.poll();
    assert_eq!(window.records, 3);
    assert_eq!(window.samples, 2);
    assert_eq!(window.clock_anomalies, 1);
    assert_eq!(window.min_ticks, Some(100));
    assert_eq!(window.max_ticks, Some(200));
    assert_eq!(window.mean_ticks, Some(150));

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.samples, 2);
    assert_eq!(snapshot.clock_anomalies, 1);
    let buckets = sink.histogram_reader().snapshot();
    assert_eq!(buckets.iter().sum::<u64>(), 2);
}

// --- R4/R10: fixed clock stamping + record vocabulary ----------------------

#[test]
fn bridged_records_carry_the_fixed_clock_stamp() {
    let (mut recorder, mut sink) = make_bridge();
    assert!(recorder.record(budgeted_sample(10, 60, 100, 4)));

    let mut got = Vec::new();
    sink.inner_sink().drain(|received| {
        got.push((received.record(), received.dropped_before()));
    });
    assert_eq!(got.len(), 1);
    let (record, dropped_before) = got[0];
    assert_eq!(dropped_before, 0);

    let header = record.header();
    assert_eq!(header.event(), EventId::new(2));
    assert_eq!(header.version(), TIMING_SCHEMA_VERSION);
    assert_eq!(header.source(), source());
    assert_eq!(header.generation(), 4);
    assert_eq!(header.confidentiality(), Confidentiality::Internal);
    assert_eq!(header.impact(), ReportingImpact::Normal);
    let clock_ctx = header.clock().unwrap();
    assert_eq!(clock_ctx.domain(), ClockDomainId::MONOTONIC);
    assert_eq!(clock_ctx.position(), 10);
    assert_eq!(clock_ctx.time_base(), TimeBase::new(1, 1_000_000).unwrap());
    assert_eq!(clock_ctx.provenance(), TimestampProvenance::Carried);
    assert_eq!(clock_ctx.epoch(), ClockEpochId::new(1));
    assert_eq!(record.end_tick(), 60);
    assert_eq!(record.budget_ticks(), 100);
    assert_eq!(record.start_tick(), 10);
    assert_eq!(record.duration_ticks(), Some(50));
}

#[test]
fn clock_is_fixed_per_bridge() {
    // A second bridge with a different clock stamps its own identity —
    // clock changes are new-bridge (non-RT) events, never mixed.
    let other = TimingClock::new(
        ClockDomainId::SAMPLE_CLOCK,
        TimeBase::new(1, 48_000).unwrap(),
        TimestampProvenance::Estimated,
        ClockEpochId::new(9),
    );
    let (mut recorder, mut sink) = timing_bridge(4, other, source(), BOUNDS).unwrap();
    assert!(recorder.record(sample(7, 77)));

    let mut got = Vec::new();
    sink.inner_sink()
        .drain(|received| got.push(received.record()));
    let clock_ctx = got[0].header().clock().unwrap();
    assert_eq!(clock_ctx.domain(), ClockDomainId::SAMPLE_CLOCK);
    assert_eq!(clock_ctx.epoch(), ClockEpochId::new(9));
    assert_eq!(clock_ctx.provenance(), TimestampProvenance::Estimated);
    assert_eq!(clock_ctx.time_base(), TimeBase::new(1, 48_000).unwrap());
}

#[test]
fn record_shape_is_bounded_copy() {
    fn assert_copy_send<T: Copy + Send + 'static>() {}
    assert!(size_of::<TimingRecord>() <= 128);
    assert_copy_send::<TimingRecord>();
    assert_copy_send::<TimingSample>();
    assert_copy_send::<TimingClock>();
}

// --- R5: budget semantics ---------------------------------------------------

#[test]
fn budget_utilization_and_overrun_are_accounted() {
    let (mut recorder, mut sink) = make_bridge();
    // No budget asserted — utilization stays absent, never 0%.
    assert!(recorder.record(sample(0, 100)));
    // Within budget: duration 50 / budget 100 = 500 permille.
    assert!(recorder.record(budgeted_sample(1_000, 1_050, 100, 0)));
    // Over budget: duration 200 / budget 100 = 2000 permille.
    assert!(recorder.record(budgeted_sample(2_000, 2_200, 100, 0)));

    let window = sink.poll();
    assert_eq!(window.samples, 3);
    assert_eq!(window.over_budget, 1);
    assert_eq!(window.max_utilization_permille, Some(2_000));

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.over_budget, 1);
    assert_eq!(snapshot.max_utilization_permille, Some(2_000));
}

#[test]
fn unasserted_budget_reports_none_not_zero() {
    let (mut recorder, mut sink) = make_bridge();
    assert!(recorder.record(sample(0, 100)));
    assert!(recorder.record(sample(100, 200)));

    let window = sink.poll();
    assert_eq!(window.samples, 2);
    assert_eq!(window.over_budget, 0);
    assert_eq!(window.max_utilization_permille, None);

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.max_utilization_permille, None);
    assert_eq!(snapshot.over_budget, 0);
}

// --- R6: histogram + collector integration ---------------------------------

#[test]
fn duration_histogram_feeds_collector_metric_value() {
    let (mut recorder, mut sink) = make_bridge();
    for (start, end) in [
        (0, 50),
        (100, 250),
        (500, 1_100),
        (2_000, 4_000),
        (5_000, 14_000),
    ] {
        assert!(recorder.record(sample(start, end)));
    }
    // Aggregation happens on the non-RT side.
    let _ = sink.poll();

    let mut collector = Collector::new(CollectorConfig::default()).unwrap();
    let port = collector.registration_port();
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Histogram,
            key: "test.rt_timing.duration_ticks",
            unit: ObservationUnit::None,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::BucketCounts,
            buckets: Some(BOUNDS),
            semantics: SourceSemantics {
                meaning: "measured interval duration in clock ticks",
                ..SourceSemantics::default()
            },
        },
        ObservationScope::Instance,
        MetricReader::Histogram(sink.histogram_reader()),
    )
    .unwrap();
    let _lease = port.register(source).unwrap();

    let batch = collector.poll(Instant::now()).unwrap();
    let measurement = batch.measurements().first().unwrap();
    match &measurement.value {
        Some(MetricValue::Histogram {
            buckets,
            total_approx,
        }) => {
            // durations 50,150,600,2000,9000 → one per bucket.
            assert_eq!(&**buckets, &[1, 1, 1, 1, 1]);
            assert_eq!(*total_approx, 5);
        }
        other => panic!("expected histogram measurement, got {other:?}"),
    }
}

#[test]
fn histogram_buckets_match_observed_durations() {
    let (mut recorder, mut sink) = make_bridge();
    for (start, end) in [
        (0, 50),
        (100, 250),
        (500, 1_100),
        (2_000, 4_000),
        (5_000, 14_000),
    ] {
        assert!(recorder.record(sample(start, end)));
    }
    let _ = sink.poll();
    let buckets = sink.histogram_reader().snapshot();
    assert_eq!(&*buckets, &[1, 1, 1, 1, 1]);
}

// --- R8: quantile interval estimates vs exact stats ------------------------

#[test]
fn estimated_quantile_returns_bucket_intervals() {
    let (mut recorder, mut sink) = make_bridge();
    // One sample per bucket: 50, 150, 600, 2000, 9000 ticks.
    for (start, end) in [
        (0, 50),
        (100, 250),
        (500, 1_100),
        (2_000, 4_000),
        (5_000, 14_000),
    ] {
        assert!(recorder.record(sample(start, end)));
    }
    let _ = sink.poll();
    let snapshot = sink.snapshot();

    // Exact stats are precise.
    assert_eq!(snapshot.samples, 5);
    assert_eq!(snapshot.min_ticks, Some(50));
    assert_eq!(snapshot.max_ticks, Some(9_000));
    assert_eq!(snapshot.mean_ticks, Some(2_360)); // (50+150+600+2000+9000)/5

    // p50: rank ceil(500*5/1000)=3 → third bucket [500, 1000).
    assert_eq!(
        snapshot.estimated_quantile(500),
        Some(QuantileEstimate {
            lower_ticks: 500,
            upper_ticks: Some(1_000),
        })
    );
    // p1000: rank 5 → top bucket [5000, ∞).
    assert_eq!(
        snapshot.estimated_quantile(1_000),
        Some(QuantileEstimate {
            lower_ticks: 5_000,
            upper_ticks: None,
        })
    );
    // p0 clamps to the first occupied bucket.
    assert_eq!(
        snapshot.estimated_quantile(0),
        Some(QuantileEstimate {
            lower_ticks: 0,
            upper_ticks: Some(100),
        })
    );
    // Out-of-range quantile and empty distribution report None.
    assert_eq!(snapshot.estimated_quantile(1_001), None);
    let (_recorder, empty_sink) = make_bridge();
    assert_eq!(empty_sink.snapshot().estimated_quantile(500), None);
}

// --- R9: generation accounting ----------------------------------------------

#[test]
fn generation_mixing_is_explicit() {
    let (mut recorder, mut sink) = make_bridge();
    assert!(recorder.record(budgeted_sample(0, 100, 0, 5)));
    assert!(recorder.record(budgeted_sample(100, 200, 0, 5)));
    assert!(recorder.record(budgeted_sample(200, 400, 0, 9)));
    assert!(recorder.record(budgeted_sample(400, 300, 0, 5))); // anomaly, gen still counted

    let window = sink.poll();
    assert_eq!(window.records, 4);
    assert_eq!(window.samples, 3);
    assert_eq!(window.clock_anomalies, 1);
    assert_eq!(window.min_generation, Some(5));
    assert_eq!(window.max_generation, Some(9));
    assert_eq!(window.generation_transitions, 2); // 5→9, 9→5

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.min_generation, Some(5));
    assert_eq!(snapshot.max_generation, Some(9));
    assert_eq!(snapshot.generation_transitions, 2);

    // An empty poll reports no records and no new transitions.
    let empty = sink.poll();
    assert_eq!(empty.index, 2);
    assert_eq!(empty.records, 0);
    assert_eq!(empty.min_generation, None);
    assert_eq!(empty.generation_transitions, 0);
}

// --- window/snapshot shape ---------------------------------------------------

#[test]
fn empty_poll_reports_empty_window() {
    let (_recorder, mut sink) = make_bridge();
    let window = sink.poll();
    assert_eq!(window.index, 1);
    assert_eq!(window.records, 0);
    assert_eq!(window.samples, 0);
    assert_eq!(window.min_ticks, None);
    assert_eq!(window.mean_ticks, None);
    assert_eq!(window.max_utilization_permille, None);

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.clock, clock());
    assert_eq!(snapshot.records, 0);
    assert_eq!(snapshot.samples, 0);
    assert_eq!(snapshot.dropped_total, 0);
    assert_eq!(snapshot.min_generation, None);
}

#[test]
fn cumulative_stats_accumulate_across_polls() {
    let (mut recorder, mut sink) = make_bridge();
    assert!(recorder.record(sample(0, 100)));
    let _ = sink.poll();
    assert!(recorder.record(sample(100, 400)));
    assert!(recorder.record(sample(400, 900)));
    let second = sink.poll();
    assert_eq!(second.index, 2);
    assert_eq!(second.records, 2);
    assert_eq!(second.samples, 2);
    assert_eq!(second.min_ticks, Some(300));
    assert_eq!(second.max_ticks, Some(500));

    let snapshot = sink.snapshot();
    assert_eq!(snapshot.records, 3);
    assert_eq!(snapshot.samples, 3);
    assert_eq!(snapshot.min_ticks, Some(100));
    assert_eq!(snapshot.max_ticks, Some(500));
    assert_eq!(snapshot.mean_ticks, Some(300)); // (100+300+500)/3
}
