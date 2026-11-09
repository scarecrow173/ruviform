//! `tracing_export` contract tests — Issue #278 (OBS-03) slice.
//!
//! Events are captured through a JSON-formatting `tracing-subscriber`
//! layer writing into a shared buffer (dev-dependency only — the
//! feature itself never requires a subscriber). Collector-driven tests
//! use the real `Collector::poll` so baseline/delta/window semantics
//! come from OBS-07, not from fixtures.

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ruviform_media_core::ClockDomainId;
use serde_json::Value;
use tracing::{Dispatch, Event, Metadata};
use tracing_subscriber::Registry;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use crate::collector::{
    Collector, CollectorConfig, MetricsSnapshot, ObservationBatch, ObservationWindow,
};
use crate::core::{counter_reader_on_cell, gauge, histogram};
use crate::diagnostics::{
    Diagnostics, DiagnosticsConfig, DiagnosticsEvent, FindingSink, HealthState,
};
use crate::metrics::{Measurement, MeasurementValidity};
use crate::source::{
    AggregationPolicy, MetricDescriptor, MetricKind, MetricReader, MetricSource,
    ObservationRegistrar, ObservationScope, ObservationUnit, ReportingHint, ReportingImpact,
    SourceGeneration, SourceId, SourceIdentity, SourceLifecycle, SourceSemantics,
};

use super::{TracingExporter, TracingFindingSink};

// --- capture harness ---------------------------------------------------------

/// `io::Write` sink shared between the test and the fmt layer.
#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedBuf {
    /// Drains the buffer — returns only events captured since the
    /// previous `take`.
    fn take(&self) -> Vec<Value> {
        let mut data = self.0.lock().unwrap();
        let text = String::from_utf8_lossy(&data).into_owned();
        data.clear();
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("captured line is JSON"))
            .collect()
    }
}

/// A dispatch whose fmt/JSON layer writes into `buf` — no global
/// default is ever installed.
fn capture_dispatch() -> (Dispatch, SharedBuf) {
    let buf = SharedBuf::default();
    let layer = fmt::layer().json().with_writer({
        let b = buf.clone();
        move || b.clone()
    });
    let subscriber = Registry::default().with(layer);
    (Dispatch::new(subscriber), buf)
}

fn fields(event: &Value) -> &Value {
    &event["fields"]
}

fn field_str<'a>(event: &'a Value, key: &str) -> &'a str {
    fields(event)[key].as_str().expect("field is a string")
}

fn field_u64(event: &Value, key: &str) -> u64 {
    fields(event)[key].as_u64().expect("field is a u64")
}

fn field_f64(event: &Value, key: &str) -> f64 {
    fields(event)[key].as_f64().expect("field is a number")
}

fn field_bool(event: &Value, key: &str) -> bool {
    fields(event)[key].as_bool().expect("field is a bool")
}

fn level(event: &Value) -> &str {
    event["level"].as_str().expect("level is a string")
}

fn measurement_events(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| field_str(e, "event_code") == "metric_observation")
        .collect()
}

fn window_events(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| field_str(e, "event_code") == "observation_window")
        .collect()
}

// --- collector fixtures ------------------------------------------------------

fn config() -> CollectorConfig {
    CollectorConfig {
        poll_interval: Duration::from_millis(10),
        ..CollectorConfig::default()
    }
}

/// A counter source on a shared atomic cell so tests control the raw
/// value exactly (including decreases for discontinuity cases).
fn cell_counter_source(
    key: &'static str,
    reporting: ReportingHint,
) -> (Arc<AtomicU64>, MetricSource) {
    let cell = Arc::new(AtomicU64::new(0));
    let reader = counter_reader_on_cell(&cell);
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Counter,
            key,
            unit: ObservationUnit::Count,
            reporting,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Counter(reader),
    )
    .unwrap();
    (cell, source)
}

fn onincrease(min_delta: u64, min_interval: Duration, impact: ReportingImpact) -> ReportingHint {
    ReportingHint::OnIncrease {
        min_delta,
        min_interval,
        impact,
    }
}

// --- exporter: schema / policy goldens ----------------------------------------

#[test]
fn counter_onincrease_emits_fixed_schema_event() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) = cell_counter_source(
        "test.tracing.counter",
        onincrease(2, Duration::from_secs(1), ReportingImpact::High),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let lease = port.register(source).unwrap();
    let identity: SourceIdentity = lease.identity();

    let t0 = Instant::now();

    // First observation: baseline only — no measurement event, but the
    // poll still produces a window aggregate.
    let b1 = collector.poll(t0).unwrap();
    let stats = exporter.emit(&b1);
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // Increase below min_delta: still suppressed.
    cell.store(1, Ordering::Relaxed);
    let b2 = collector.poll(t0 + Duration::from_millis(1500)).unwrap();
    let stats = exporter.emit(&b2);
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // Accumulated increase >= min_delta after min_interval: one event.
    cell.store(4, Ordering::Relaxed);
    let b3 = collector.poll(t0 + Duration::from_millis(1600)).unwrap();
    let stats = exporter.emit(&b3);
    assert_eq!(stats.emitted, 1);
    assert_eq!(stats.cap_dropped, 0);

    // Re-emitting the same batch must not duplicate the measurement
    // event (baseline was consumed by the first emit).
    let stats = exporter.emit(&b3);
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // An unchanged poll stays suppressed (no increase, interval short).
    let b4 = collector.poll(t0 + Duration::from_millis(1700)).unwrap();
    let stats = exporter.emit(&b4);
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // Enough accumulated increase + interval elapsed: second event.
    cell.store(6, Ordering::Relaxed);
    let b5 = collector.poll(t0 + Duration::from_millis(2700)).unwrap();
    let stats = exporter.emit(&b5);
    assert_eq!(stats.emitted, 1);

    let events = buf.take();
    let measurement = measurement_events(&events);
    assert_eq!(measurement.len(), 2);
    let e = measurement[0];

    // Golden schema assertions — every documented key is present with
    // the expected type; inapplicable value fields carry 0/false.
    assert_eq!(field_str(e, "event_code"), "metric_observation");
    assert_eq!(field_u64(e, "event_version"), 1);
    assert_eq!(
        field_str(e, "observation_key"),
        identity.to_string().as_str()
    );
    assert_eq!(field_u64(e, "source_id"), identity.id().get());
    assert_eq!(
        field_u64(e, "source_generation"),
        identity.generation().get()
    );
    assert_eq!(field_str(e, "metric_key"), "test.tracing.counter");
    assert_eq!(field_str(e, "scope"), "process");
    assert_eq!(field_str(e, "lifecycle"), "active");
    assert_eq!(field_str(e, "kind"), "counter");
    assert_eq!(field_str(e, "unit"), "count");
    assert_eq!(field_str(e, "validity"), "valid");
    assert_eq!(field_str(e, "issue"), "none");
    assert_eq!(field_u64(e, "loss"), 0);
    assert!(!field_bool(e, "loss_quantified"));
    assert!(fields(e)["observed_at_mono_us"].is_u64());
    assert!(fields(e)["last_change_at_mono_us"].is_u64());
    assert_eq!(field_u64(e, "poll_generation"), b3.generation());
    assert_eq!(field_u64(e, "counter_raw"), 4);
    assert_eq!(field_u64(e, "counter_cumulative"), 4);
    assert_eq!(field_u64(e, "counter_delta"), 3);
    assert!(field_bool(e, "counter_delta_present"));
    assert!(field_bool(e, "counter_rate_present"));
    assert!(field_f64(e, "counter_rate_per_sec") > 0.0);
    assert_eq!(field_u64(e, "gauge_value"), 0);
    assert!(!field_bool(e, "gauge_previous_present"));
    assert_eq!(field_u64(e, "histogram_bucket_count"), 0);
    assert_eq!(field_u64(e, "histogram_total_approx"), 0);

    // High impact + Valid → WARN (never ERROR for measurements).
    assert_eq!(level(e), "WARN");

    // Second event carries the later window delta/cumulative.
    let e2 = measurement[1];
    assert_eq!(field_u64(e2, "counter_raw"), 6);
    assert_eq!(field_u64(e2, "counter_cumulative"), 6);
    assert_eq!(field_u64(e2, "counter_delta"), 2);
    assert_eq!(level(e2), "WARN");

    // Window aggregate: one per emit call that saw eligible work —
    // six emit calls above (including the deliberate re-emit of b3).
    let windows = window_events(&events);
    assert_eq!(windows.len(), 6);
    let w = windows[2];
    assert_eq!(field_str(w, "event_code"), "observation_window");
    assert_eq!(field_u64(w, "poll_generation"), b3.generation());
    assert_eq!(field_u64(w, "emitted"), 1);
    assert_eq!(field_u64(w, "policy_suppressed"), 0);
    assert_eq!(field_u64(w, "cap_dropped"), 0);
    assert!(fields(w)["window_measured_us"].is_u64());
    assert!(fields(w)["window_started_mono_us"].is_u64());
    assert!(fields(w)["window_ended_mono_us"].is_u64());
    assert!(!field_bool(w, "clock_anomaly"));
    // `ClockDomainId::MONOTONIC`.
    assert_eq!(field_u64(w, "clock_domain"), 2);
}

#[test]
fn snapshot_only_never_becomes_an_event() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) =
        cell_counter_source("test.tracing.snapshot_only", ReportingHint::SnapshotOnly);
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();

    let t0 = Instant::now();
    for i in 0..3_u64 {
        cell.store(i * 10, Ordering::Relaxed);
        let batch = collector.poll(t0 + Duration::from_millis(i * 100)).unwrap();
        let stats = exporter.emit(&batch);
        assert_eq!(stats.emitted, 0);
        assert_eq!(stats.snapshot_suppressed, 1);
    }

    // No measurement *and* no window event — SnapshotOnly work is not
    // eventized at all.
    assert_eq!(buf.take(), Vec::<Value>::new());
}

#[test]
fn discontinuity_emits_warn_with_typed_fields() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    // min_delta = 0 / min_interval = 0: every post-baseline valued
    // measurement is reportable, so the discontinuity itself surfaces.
    let (cell, source) = cell_counter_source(
        "test.tracing.discontinuity",
        onincrease(0, Duration::ZERO, ReportingImpact::Normal),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();

    let t0 = Instant::now();
    cell.store(9, Ordering::Relaxed);
    let _ = exporter.emit(&collector.poll(t0).unwrap()); // baseline
    // Unchanged poll: deduplicated — no measurement event even with
    // min_delta/min_interval at zero.
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // Counter decrease → Discontinuous + CounterDecreased, delta None.
    // The report signal is flat but the validity/issue transition is
    // a real change, so the WARN-floor event still emits.
    cell.store(2, Ordering::Relaxed);
    let b = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    let m = b.measurements().first().unwrap();
    assert_eq!(m.validity, MeasurementValidity::Discontinuous);
    let stats = exporter.emit(&b);
    assert_eq!(stats.emitted, 1);

    let events = buf.take();
    let measurement = measurement_events(&events);
    assert_eq!(measurement.len(), 1);
    let e = measurement[0];
    assert_eq!(field_str(e, "validity"), "discontinuous");
    assert_eq!(field_str(e, "issue"), "counter_decreased");
    assert!(!field_bool(e, "counter_delta_present"));
    assert!(!field_bool(e, "counter_rate_present"));
    // Normal impact alone would be INFO; the validity floor raises it.
    assert_eq!(level(e), "WARN");
}

#[test]
fn abandoned_lease_partial_drain_warns() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) = cell_counter_source(
        "test.tracing.partial",
        onincrease(0, Duration::ZERO, ReportingImpact::Low),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let lease = port.register(source).unwrap();

    let t0 = Instant::now();
    cell.store(5, Ordering::Relaxed);
    let _ = exporter.emit(&collector.poll(t0).unwrap()); // baseline

    // Lease dropped without retire → final drain is Partial +
    // WriterNotQuiesced; validity floor WARN beats Low→DEBUG. The
    // drain may take more than one poll to surface.
    drop(lease);
    for i in 0..3_u64 {
        let b = collector
            .poll(t0 + Duration::from_millis(10 + i * 10))
            .unwrap();
        let _ = exporter.emit(&b);
    }

    let events = buf.take();
    let e = measurement_events(&events)
        .into_iter()
        .find(|e| field_str(e, "validity") == "partial")
        .expect("partial drain event");
    assert_eq!(field_str(e, "issue"), "writer_not_quiesced");
    assert_eq!(level(e), "WARN");
}

#[test]
fn gauge_onincrease_uses_latest_value_signal() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (g, reader) = gauge(10);
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Gauge,
            key: "test.tracing.gauge",
            unit: ObservationUnit::Count,
            reporting: onincrease(5, Duration::ZERO, ReportingImpact::Low),
            aggregation: AggregationPolicy::LatestValue,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Instance,
        MetricReader::Gauge(reader),
    )
    .unwrap();

    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();

    let t0 = Instant::now();
    let _ = exporter.emit(&collector.poll(t0).unwrap()); // baseline 10

    // +4 < min_delta 5 → suppressed.
    g.set(14);
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 1);

    // +6 >= 5 → emit at DEBUG (Low impact, Valid).
    g.set(20);
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(20)).unwrap());
    assert_eq!(stats.emitted, 1);

    let events = buf.take();
    let measurement = measurement_events(&events);
    assert_eq!(measurement.len(), 1);
    let e = measurement[0];
    assert_eq!(field_str(e, "kind"), "gauge");
    assert_eq!(field_str(e, "scope"), "instance");
    assert_eq!(field_u64(e, "gauge_value"), 20);
    assert!(field_bool(e, "gauge_previous_present"));
    assert_eq!(field_u64(e, "gauge_previous"), 14);
    assert_eq!(field_u64(e, "counter_raw"), 0);
    assert_eq!(level(e), "DEBUG");
}

#[test]
fn histogram_emits_bounded_fields_only() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (h, reader) = histogram(&[10, 100]).unwrap();
    let source = MetricSource::new(
        MetricDescriptor {
            kind: MetricKind::Histogram,
            key: "test.tracing.histogram",
            unit: ObservationUnit::Count,
            reporting: onincrease(1, Duration::ZERO, ReportingImpact::Normal),
            aggregation: AggregationPolicy::BucketCounts,
            buckets: Some(&[10, 100]),
            semantics: SourceSemantics::default(),
        },
        ObservationScope::Process,
        MetricReader::Histogram(reader),
    )
    .unwrap();

    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();

    let t0 = Instant::now();
    h.observe(5);
    h.observe(50);
    let _ = exporter.emit(&collector.poll(t0).unwrap()); // baseline total 2

    h.observe(500);
    h.observe(7);
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 1);

    let events = buf.take();
    let measurement = measurement_events(&events);
    assert_eq!(measurement.len(), 1);
    let e = measurement[0];
    assert_eq!(field_str(e, "kind"), "histogram");
    // Bounded schema: bucket count + approximate total only — the
    // bucket array itself is never expanded into fields.
    assert_eq!(field_u64(e, "histogram_bucket_count"), 3);
    assert_eq!(field_u64(e, "histogram_total_approx"), 4);
    assert_eq!(level(e), "INFO");
}

#[test]
fn normal_impact_valid_measurement_is_info() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) = cell_counter_source(
        "test.tracing.normal",
        onincrease(1, Duration::ZERO, ReportingImpact::Normal),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();

    let t0 = Instant::now();
    let _ = exporter.emit(&collector.poll(t0).unwrap());
    cell.store(3, Ordering::Relaxed);
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 1);

    let events = buf.take();
    let e = measurement_events(&events)[0];
    assert_eq!(level(e), "INFO");
}

#[test]
fn emission_cap_drops_and_reports_loss() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::with_max_emissions_per_poll(dispatch, 2);

    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let keys: [&'static str; 3] = [
        "test.tracing.cap.a",
        "test.tracing.cap.b",
        "test.tracing.cap.c",
    ];
    let mut cells = Vec::new();
    let mut leases = Vec::new();
    for key in keys {
        let (cell, source) =
            cell_counter_source(key, onincrease(0, Duration::ZERO, ReportingImpact::Normal));
        cells.push(cell);
        // Keep the lease alive — dropping it drains the source.
        leases.push(port.register(source).unwrap());
    }

    let t0 = Instant::now();
    for cell in &cells {
        cell.store(1, Ordering::Relaxed);
    }
    // First poll: all three baseline (policy_suppressed), window event
    // still emitted because eligible work was seen.
    let stats = exporter.emit(&collector.poll(t0).unwrap());
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.policy_suppressed, 3);

    for cell in &cells {
        cell.fetch_add(1, Ordering::Relaxed);
    }
    // Second poll: all three pass the gate; cap allows two.
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 2);
    assert_eq!(stats.cap_dropped, 1);

    let events = buf.take();
    assert_eq!(measurement_events(&events).len(), 2);
    let windows = window_events(&events);
    let last = windows.last().unwrap();
    assert_eq!(field_u64(last, "cap_dropped"), 1);
    assert_eq!(field_u64(last, "emitted"), 2);
    // Cap loss elevates the window aggregate to WARN.
    assert_eq!(level(last), "WARN");
}

#[test]
fn valueless_measurement_is_counted_not_emitted() {
    let (dispatch, buf) = capture_dispatch();
    let mut exporter = TracingExporter::new(dispatch);

    // Synthetic batch: an OnIncrease measurement with no reportable
    // value (Unavailable) must be counted, never fabricated into an
    // event with a zero.
    let identity = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(1));
    let t0 = Instant::now();
    let measurement = Measurement {
        identity,
        descriptor: MetricDescriptor {
            kind: MetricKind::Counter,
            key: "test.tracing.valueless",
            unit: ObservationUnit::Count,
            reporting: onincrease(0, Duration::ZERO, ReportingImpact::High),
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        },
        scope: ObservationScope::Process,
        lifecycle: SourceLifecycle::Removed,
        observed_at: t0,
        last_change_at: t0,
        validity: MeasurementValidity::Unavailable,
        issue: None,
        loss: Some(3),
        value: None,
    };
    let window = ObservationWindow {
        generation: 1,
        started: t0,
        ended: t0 + Duration::from_millis(10),
        expected: Duration::from_millis(10),
        measured: Duration::from_millis(10),
        clock: ClockDomainId::MONOTONIC,
        planned: 1,
        observed: 1,
        skipped: 0,
        drained: 0,
        history_evicted: 0,
        clock_anomaly: false,
    };
    let batch = ObservationBatch::new(
        1,
        window,
        vec![measurement],
        Vec::new(),
        MetricsSnapshot::new(1, t0, Vec::new()),
    );

    let stats = exporter.emit(&batch);
    assert_eq!(stats.emitted, 0);
    assert_eq!(stats.valueless_skipped, 1);
    // Valueless-only polls produce no window event either — there was
    // no eligible work.
    assert_eq!(measurement_events(&buf.take()), Vec::<&Value>::new());
    assert_eq!(window_events(&buf.take()), Vec::<&Value>::new());
}

#[test]
fn dispatch_none_is_safe_without_subscriber() {
    let mut exporter = TracingExporter::new(Dispatch::none());
    let (cell, source) = cell_counter_source(
        "test.tracing.none",
        onincrease(0, Duration::ZERO, ReportingImpact::High),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();
    cell.store(1, Ordering::Relaxed);

    // No global default is installed anywhere in this test binary —
    // `Dispatch::none()` must simply drop events.
    let stats = exporter.emit(&collector.poll(t0).unwrap());
    assert_eq!(stats.policy_suppressed, 1); // baseline only
    cell.store(3, Ordering::Relaxed);
    let stats = exporter.emit(&collector.poll(t0 + Duration::from_millis(10)).unwrap());
    assert_eq!(stats.emitted, 1); // gate passed; events dropped
}

// --- subscriber-side failure / filtering --------------------------------------

/// A layer that disables every event — models subscriber-side
/// filtering. `on_event` must never run.
struct SuppressingLayer;

impl Layer<Registry> for SuppressingLayer {
    fn enabled(&self, _metadata: &Metadata<'_>, _ctx: Context<'_, Registry>) -> bool {
        false
    }

    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, Registry>) {
        panic!("filtered events must not reach on_event");
    }
}

/// A layer that panics on every event — models a failing subscriber.
struct PanicLayer;

impl Layer<Registry> for PanicLayer {
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, Registry>) {
        panic!("injected subscriber failure");
    }
}

#[test]
fn filter_suppression_does_not_mutate_exporter_or_collector() {
    let subscriber = Registry::default().with(SuppressingLayer);
    let dispatch = Dispatch::new(subscriber);
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) = cell_counter_source(
        "test.tracing.filtered",
        onincrease(0, Duration::ZERO, ReportingImpact::High),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();

    cell.store(2, Ordering::Relaxed);
    let batch = collector.poll(t0).unwrap();
    let stats = exporter.emit(&batch);
    // The exporter did its job — the subscriber's filter is the
    // pipeline's concern, not the exporter's.
    assert_eq!(stats.policy_suppressed, 1);

    cell.store(5, Ordering::Relaxed);
    let batch2 = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    let stats = exporter.emit(&batch2);
    assert_eq!(stats.emitted, 1); // counted even though filtered out

    // Collector state is untouched by either emit.
    let after = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    assert_eq!(after.measurements().len(), 1);
}

#[test]
fn panicking_subscriber_does_not_affect_collector() {
    let subscriber = Registry::default().with(PanicLayer);
    let dispatch = Dispatch::new(subscriber);
    let mut exporter = TracingExporter::new(dispatch);

    let (cell, source) = cell_counter_source(
        "test.tracing.panicsubscriber",
        onincrease(0, Duration::ZERO, ReportingImpact::High),
    );
    let mut collector = Collector::new(config()).unwrap();
    let port = collector.registration_port();
    let _lease = port.register(source).unwrap();
    let t0 = Instant::now();

    // Every emit with eligible work also emits the window aggregate,
    // so the panicking subscriber fires on the very first emit.
    let batch = collector.poll(t0).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| exporter.emit(&batch))).is_err());

    cell.store(4, Ordering::Relaxed);
    let batch = collector.poll(t0 + Duration::from_millis(10)).unwrap();
    // The panic propagates to the *caller* (the host owns the
    // subscriber); it must not corrupt collector state.
    assert!(catch_unwind(AssertUnwindSafe(|| exporter.emit(&batch))).is_err());

    // Collector keeps working normally afterwards.
    cell.store(9, Ordering::Relaxed);
    let batch2 = collector.poll(t0 + Duration::from_millis(20)).unwrap();
    assert_eq!(batch2.measurements().len(), 1);
    let m = &batch2.measurements()[0];
    assert_eq!(m.lifecycle, SourceLifecycle::Active);
}

// --- FindingSink adapter -------------------------------------------------------

fn health_event(domain: &'static str, from: HealthState, to: HealthState) -> DiagnosticsEvent {
    DiagnosticsEvent::HealthTransition {
        domain,
        from,
        to,
        at: Instant::now(),
    }
}

#[test]
fn sink_emits_health_transition_with_stable_fields() {
    let (dispatch, buf) = capture_dispatch();
    let mut sink = TracingFindingSink::new(dispatch);

    sink.emit(health_event(
        "plugin_host.l1",
        HealthState::Healthy,
        HealthState::Degraded,
    ));
    sink.emit(health_event(
        "plugin_host.l1",
        HealthState::Degraded,
        HealthState::Unhealthy,
    ));
    sink.emit(health_event(
        "plugin_host.l1",
        HealthState::Unhealthy,
        HealthState::Unknown,
    ));

    let events = buf.take();
    let sink_events: Vec<&Value> = events
        .iter()
        .filter(|e| field_str(e, "event_code") == "diagnostics.health_transition")
        .collect();
    assert_eq!(sink_events.len(), 3);

    let e = sink_events[0];
    assert_eq!(field_u64(e, "event_version"), 1);
    assert_eq!(field_str(e, "domain"), "plugin_host.l1");
    assert_eq!(field_str(e, "from_state"), "healthy");
    assert_eq!(field_str(e, "to_state"), "degraded");
    assert!(fields(e)["at_mono_us"].is_u64());
    assert_eq!(level(e), "WARN");

    assert_eq!(level(sink_events[1]), "ERROR");
    assert_eq!(field_str(sink_events[1], "to_state"), "unhealthy");
    assert_eq!(level(sink_events[2]), "INFO");
    assert_eq!(field_str(sink_events[2], "to_state"), "unknown");
    assert!(fields(sink_events[2])["at_mono_us"].is_u64());
}

#[test]
fn sink_panic_is_isolated_and_counted() {
    let subscriber = Registry::default().with(PanicLayer);
    let dispatch = Dispatch::new(subscriber);
    let mut sink = TracingFindingSink::new(dispatch);

    // The panic is caught inside the adapter — `emit` returns
    // normally and the failure is counted, never recursively logged.
    sink.emit(health_event(
        "d",
        HealthState::Healthy,
        HealthState::Degraded,
    ));
    sink.emit(health_event(
        "d",
        HealthState::Degraded,
        HealthState::Healthy,
    ));
    assert_eq!(sink.panics(), 2);
}

#[test]
fn diagnostics_set_sink_accepts_tracing_sink() {
    let (dispatch, _buf) = capture_dispatch();
    let mut diagnostics = Diagnostics::new(DiagnosticsConfig::default()).unwrap();
    diagnostics.set_sink(Box::new(TracingFindingSink::new(dispatch)));
    // Engine-side suppression/panic isolation is OBS-09's contract;
    // here we only prove the adapter satisfies `FindingSink` as a
    // trait object.
}
