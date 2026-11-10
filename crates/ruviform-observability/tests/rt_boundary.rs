//! Issue #275 contract test — RT-suitable operations emit no
//! `tracing` events and never reach a subscriber, even while a
//! dispatch is installed.
//!
//! This pins the `tracing`/Subscriber half of the acceptance row
//! "RT call path に alloc、blocking lock、OS IO、`tracing`/
//! Subscriber、無制限キューや制御されない `Drop` がない". The
//! no-alloc/no-lock halves are pinned by the `rt_*`
//! `assert_no_alloc` lanes; this file is the regression guard for
//! the per-operation RT contracts documented in `lib.rs` ("RT
//! contract" — e.g. [`ruviform_observability::rt_timing::TimingRecorder::record`]
//! declares "no tracing/subscriber call").
//!
//! `required-features = ["tracing-export"]` is the minimal feature
//! that enables `dep:tracing` without requiring subscriber
//! construction. The counting subscriber is a dev-dependency
//! `tracing-subscriber` layer scoped via `with_default` — the
//! library never installs a global default, and each test proves
//! the dispatch is live with a control `tracing::info!` emission
//! so a zero count cannot be a dead subscriber.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::{Dispatch, Event, dispatcher};
use tracing_subscriber::Registry;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use ruviform_media_core::{ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance};
use ruviform_observability::core::{counter, gauge, histogram};
use ruviform_observability::record::{
    ClockContext, CorrelationLink, CorrelationToken, EventId, EventRecord, RecordHeader,
};
use ruviform_observability::rt_bridge::bridge;
use ruviform_observability::rt_timing::{TimingClock, TimingSample, timing_bridge};
use ruviform_observability::source::{
    Confidentiality, ReportingImpact, SourceGeneration, SourceId, SourceIdentity,
};

/// Counts every event that reaches the subscriber.
#[derive(Clone, Default)]
struct EventCounter {
    events: Arc<AtomicU64>,
}

impl Layer<Registry> for EventCounter {
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, Registry>) {
        self.events.fetch_add(1, Ordering::Relaxed);
    }
}

/// A scoped dispatch wrapping a counting layer — never installed as
/// a global default.
fn counting_dispatch() -> (Dispatch, Arc<AtomicU64>) {
    let layer = EventCounter::default();
    let events = Arc::clone(&layer.events);
    let subscriber = Registry::default().with(layer);
    (Dispatch::new(subscriber), events)
}

fn control_emission(dispatch: &Dispatch) {
    dispatcher::with_default(dispatch, || {
        tracing::info!("rt-boundary control emission");
    });
}

#[test]
fn rt_core_primitive_ops_emit_no_tracing_events() {
    let (dispatch, events) = counting_dispatch();
    let (counter_w, counter_r) = counter();
    let (gauge_w, gauge_r) = gauge(0);
    let (hist_w, _hist_r) = histogram(&[10, 100]).unwrap();

    dispatcher::with_default(&dispatch, || {
        counter_w.increment();
        let _ = counter_w.read();
        let _ = counter_w.reader();
        let _ = counter_r.read();
        gauge_w.set(7);
        let _ = gauge_w.read();
        let _ = gauge_w.reader();
        let _ = gauge_r.read();
        hist_w.observe(5);
        hist_w.observe(50);
        let _ = hist_w.boundaries();
        let _ = hist_w.bucket_count();
        let _ = hist_w.reader();
    });
    assert_eq!(events.load(Ordering::Relaxed), 0);

    // Control: the dispatch is live, so the zero above is not a
    // dead subscriber.
    control_emission(&dispatch);
    assert_eq!(events.load(Ordering::Relaxed), 1);
}

#[test]
fn rt_bridge_and_record_ops_emit_no_tracing_events() {
    let (dispatch, events) = counting_dispatch();

    let source = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(2));
    let clock = ClockContext::new(
        InputStamp {
            ticks: 1,
            base: TimeBase::new(1, 48_000).unwrap(),
            domain: ClockDomainId::SAMPLE_CLOCK,
            provenance: TimestampProvenance::Carried,
        },
        ClockEpochId::new(1),
    );
    let token = CorrelationToken::new(9).unwrap();
    let link = CorrelationLink::new(token, 0);
    let header = RecordHeader::new(EventId::new(5), 1, source)
        .with_generation(4)
        .with_correlation(Some(link))
        .with_clock(Some(clock))
        .with_confidentiality(Confidentiality::Public)
        .with_impact(ReportingImpact::High);
    let record = EventRecord::new(header, 480);

    let (mut recorder, _sink) = bridge::<EventRecord>(8);

    dispatcher::with_default(&dispatch, || {
        // Record vocabulary — `Copy` value types with `const`
        // accessors (RT-suitable per `lib.rs`).
        let _ = clock.stamp();
        let _ = clock.epoch();
        let _ = clock.domain();
        let _ = clock.position();
        let _ = clock.time_base();
        let _ = clock.provenance();
        let _ = token.get();
        let _ = link.token();
        let _ = link.epoch();
        let _ = header.event();
        let _ = header.version();
        let _ = header.source();
        let _ = header.generation();
        let _ = header.correlation();
        let _ = header.clock();
        let _ = header.confidentiality();
        let _ = header.impact();
        let _ = record.header();
        let _ = record.value();
        let _ = record.reserved();
        // Bridge RT ops — one bounded `try_push` plus `Relaxed`
        // counter ops.
        assert!(recorder.try_record(record));
        let _ = recorder.dropped();
        let _ = recorder.recorded();
        assert!(recorder.try_record(record));
    });
    assert_eq!(events.load(Ordering::Relaxed), 0);

    control_emission(&dispatch);
    assert_eq!(events.load(Ordering::Relaxed), 1);
}

#[test]
fn rt_timing_ops_emit_no_tracing_events() {
    let (dispatch, events) = counting_dispatch();

    let clock = TimingClock::new(
        ClockDomainId::MONOTONIC,
        TimeBase::new(1, 1_000_000).unwrap(),
        TimestampProvenance::Carried,
        ClockEpochId::new(1),
    );
    let source = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0));
    let (mut recorder, _sink) =
        timing_bridge(64, clock, source, &[100, 500, 1_000, 5_000]).unwrap();
    let sample = TimingSample {
        point: EventId::new(7),
        generation: 0,
        budget_ticks: 500,
        start_tick: 1_000,
        end_tick: 1_240,
    };

    dispatcher::with_default(&dispatch, || {
        assert!(recorder.record(sample));
        assert!(recorder.record(sample));
        let _ = recorder.dropped();
        let _ = recorder.recorded();
    });
    assert_eq!(events.load(Ordering::Relaxed), 0);

    control_emission(&dispatch);
    assert_eq!(events.load(Ordering::Relaxed), 1);
}
