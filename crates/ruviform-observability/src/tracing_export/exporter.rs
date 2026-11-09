//! [`TracingExporter`] — converts one [`ObservationBatch`] into
//! `tracing` events under the fixed schema in [`super::schema`].
//!
//! # Contract
//!
//! - **Pull, non-RT only.** [`TracingExporter::emit`] reads the batch
//!   and allocates (`HashMap` state, `String` keys); it must never be
//!   called from an RT writer path, and no RT path may call `tracing`.
//! - **`ObservationBatch` is the only boundary.** The exporter never
//!   touches the [`crate::collector::Collector`], the registry, or any
//!   metrics/diagnostics state — it cannot mutate the authoritative
//!   copies.
//! - **Host-owned dispatch.** The `tracing::Dispatch` is injected at
//!   construction; nothing here installs a global default, forces a
//!   subscriber, or writes to stdout. `Dispatch::none()` (or a
//!   subscriber that filters everything out) simply drops events.
//! - **Reporting policy lives in the descriptor.** `SnapshotOnly`
//!   measurements never become events; `OnIncrease` gates on the
//!   domain-declared `min_delta`/`min_interval`/`impact` — severity is
//!   never inferred from counter deltas.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tracing::{Dispatch, Level, dispatcher};

use crate::collector::ObservationBatch;
use crate::metrics::{Measurement, MeasurementIssue, MeasurementValidity, MetricValue};
use crate::source::{ReportingHint, ReportingImpact, SourceLifecycle};

use super::schema::{
    EVENT_CODE_MEASUREMENT, EVENT_CODE_WINDOW, EVENT_VERSION, TARGET, issue_str, kind_str,
    lifecycle_str, measurement_level, mono_us_since, scope_str, unit_str, validity_str,
};

/// Default cap on measurement events emitted per [`emit`](Self::emit)
/// call. Generous enough for normal fan-out, bounded enough that a
/// pathological batch cannot flood a subscriber.
pub const DEFAULT_MAX_EMISSIONS_PER_POLL: usize = 64;

/// Per-call accounting returned by [`TracingExporter::emit`].
///
/// Every skipped/dropped category is counted separately so loss of
/// different kinds is never conflated: policy suppression (domain
/// declared), snapshot suppression, valueless measurements, and the
/// per-poll emission cap are distinct fields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmitStats {
    /// Measurement events actually emitted this call.
    pub emitted: u64,
    /// Measurements skipped because their descriptor declares
    /// [`ReportingHint::SnapshotOnly`] (or a non-increase hint).
    pub snapshot_suppressed: u64,
    /// `OnIncrease` measurements that did not pass the
    /// `min_delta`/`min_interval` gate — including first observations,
    /// which only establish the baseline.
    pub policy_suppressed: u64,
    /// `OnIncrease` measurements with `value == None` — no reportable
    /// value exists, so no event is fabricated.
    pub valueless_skipped: u64,
    /// Measurements that passed the policy gate but exceeded
    /// `max_emissions_per_poll` — dropped without updating their
    /// baseline, so they remain eligible on the next poll.
    pub cap_dropped: u64,
}

/// Per-source `OnIncrease` pacing state.
///
/// `baseline_signal` is the report-signal value at the last emission
/// (or at first observation); `last_emit_at` is the corresponding
/// instant used for `min_interval` pacing; `last_*` record the
/// observation state at that point so an unchanged re-observation is
/// never re-reported. Keyed by
/// [`crate::source::SourceIdentity`] — registration-unique, so a
/// re-registered source starts fresh.
#[derive(Debug, Clone)]
struct SourcePolicy {
    baseline_signal: u64,
    last_emit_at: Instant,
    last_validity: MeasurementValidity,
    last_issue: Option<MeasurementIssue>,
    last_lifecycle: SourceLifecycle,
}

impl SourcePolicy {
    fn new(m: &Measurement, signal: u64) -> Self {
        Self {
            baseline_signal: signal,
            last_emit_at: m.observed_at,
            last_validity: m.validity,
            last_issue: m.issue,
            last_lifecycle: m.lifecycle,
        }
    }
}

/// The `OnIncrease` gate decision for one measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyGate {
    /// First observation of this source — establish the baseline, do
    /// not emit.
    Baseline,
    /// Increase/interval thresholds not met — suppress.
    Suppress,
    /// Thresholds met — eligible to emit.
    Emit,
}

/// Converts [`ObservationBatch`]es into `tracing` events on a
/// host-injected [`Dispatch`].
///
/// Non-RT only: [`emit`](Self::emit) allocates and may call into the
/// subscriber. Construct once per poll owner (e.g. the OBS-11 runner)
/// and reuse across polls so `OnIncrease` baselines persist.
///
/// # Examples
///
/// ```
/// use ruviform_observability::tracing_export::TracingExporter;
/// use tracing::Dispatch;
///
/// // `Dispatch::none()` drops every event — safe with no subscriber
/// // installed and no global default set.
/// let exporter = TracingExporter::new(Dispatch::none());
/// ```
pub struct TracingExporter {
    dispatch: Dispatch,
    max_emissions_per_poll: u64,
    /// First instant ever observed — origin of every `*_mono_us`
    /// field. `None` until the first `emit`.
    t0: Option<Instant>,
    /// Per-source `OnIncrease` baselines/pacing.
    states: HashMap<crate::source::SourceIdentity, SourcePolicy>,
}

impl TracingExporter {
    /// Creates an exporter emitting at most
    /// [`DEFAULT_MAX_EMISSIONS_PER_POLL`] measurement events per poll.
    #[must_use]
    pub fn new(dispatch: Dispatch) -> Self {
        Self::with_max_emissions_per_poll(dispatch, DEFAULT_MAX_EMISSIONS_PER_POLL)
    }

    /// Creates an exporter with an explicit per-poll emission cap.
    /// `max_emissions_per_poll == 0` emits no measurement events (all
    /// eligible measurements are `cap_dropped`).
    #[must_use]
    pub fn with_max_emissions_per_poll(dispatch: Dispatch, max_emissions_per_poll: usize) -> Self {
        Self {
            dispatch,
            max_emissions_per_poll: u64::try_from(max_emissions_per_poll).unwrap_or(u64::MAX),
            t0: None,
            states: HashMap::new(),
        }
    }

    /// The injected dispatch.
    #[must_use]
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Emits `tracing` events for one observation batch and returns
    /// per-call accounting.
    ///
    /// Non-RT only — allocates and may run subscriber code. Never call
    /// from an RT writer path.
    ///
    /// Emission is scoped to the injected dispatch via
    /// `tracing::dispatcher::with_default`: the caller's ambient
    /// dispatcher (if any) is restored afterwards, and no global
    /// default is consulted or installed.
    #[must_use]
    pub fn emit(&mut self, batch: &ObservationBatch) -> EmitStats {
        let mut stats = EmitStats::default();
        let t0 = *self.t0.get_or_insert_with(|| batch.window().started);
        let mut eligible = 0_u64;

        for m in batch.measurements() {
            let ReportingHint::OnIncrease {
                min_delta,
                min_interval,
                impact,
            } = m.descriptor.reporting
            else {
                // SnapshotOnly — and any future non-increase hint —
                // never becomes an event.
                stats.snapshot_suppressed += 1;
                continue;
            };
            let Some(value) = m.value.as_ref() else {
                stats.valueless_skipped += 1;
                continue;
            };
            eligible += 1;

            let signal = report_signal(value);
            match self.gate(m, signal, min_delta, min_interval) {
                PolicyGate::Baseline => {
                    self.states.insert(m.identity, SourcePolicy::new(m, signal));
                    stats.policy_suppressed += 1;
                }
                PolicyGate::Suppress => {
                    stats.policy_suppressed += 1;
                }
                PolicyGate::Emit => {
                    if stats.emitted >= self.max_emissions_per_poll {
                        // Do not touch baseline state — the increase
                        // remains pending for the next poll.
                        stats.cap_dropped += 1;
                    } else {
                        self.states.insert(m.identity, SourcePolicy::new(m, signal));
                        self.emit_measurement(m, value, impact, batch.generation(), t0);
                        stats.emitted += 1;
                    }
                }
            }
        }

        // Bound policy state by registration lifetime: identities that
        // reached `Removed` will not be observed again.
        for identity in batch.removed() {
            self.states.remove(identity);
        }

        if eligible > 0 {
            self.emit_window(batch, &stats, t0);
        }

        stats
    }

    /// Evaluates the `OnIncrease` gate without mutating state.
    ///
    /// Emission requires the declared `min_delta`/`min_interval`
    /// thresholds *and* an actual change since the last emission (or
    /// baseline): a positive signal increment or a
    /// validity/issue/lifecycle transition. The change requirement is
    /// what makes `min_delta == 0`/`min_interval == ZERO` mean
    /// "report every change" rather than "re-report every poll", and
    /// it lets degraded-validity measurements (whose report signal is
    /// flat — e.g. a counter decrease leaves
    /// `cumulative_since_registration` unchanged) still surface
    /// instead of being deduplicated away.
    fn gate(
        &self,
        m: &Measurement,
        signal: u64,
        min_delta: u64,
        min_interval: Duration,
    ) -> PolicyGate {
        let Some(state) = self.states.get(&m.identity) else {
            return PolicyGate::Baseline;
        };
        let accumulated = signal.saturating_sub(state.baseline_signal);
        let elapsed = m.observed_at.saturating_duration_since(state.last_emit_at);
        let unchanged = accumulated == 0
            && m.validity == state.last_validity
            && m.issue == state.last_issue
            && m.lifecycle == state.last_lifecycle;
        if accumulated >= min_delta && elapsed >= min_interval && !unchanged {
            PolicyGate::Emit
        } else {
            PolicyGate::Suppress
        }
    }

    /// Emits one measurement event under the fixed schema. All value
    /// fields are always present; fields not applicable to the
    /// measurement's `kind` carry `0`/`false` sentinels documented in
    /// [`super::schema`].
    ///
    /// `tracing::event!` callsites carry a *static* level, so the
    /// computed severity is dispatched through one callsite per level.
    fn emit_measurement(
        &self,
        m: &Measurement,
        value: &MetricValue,
        impact: ReportingImpact,
        poll_generation: u64,
        t0: Instant,
    ) {
        let level = measurement_level(impact, m.validity);
        let fields = ValueFields::from(value);
        macro_rules! go {
            ($lvl:expr) => {
                dispatcher::with_default(&self.dispatch, || {
                    tracing::event!(
                        target: TARGET,
                        $lvl,
                        event_code = EVENT_CODE_MEASUREMENT,
                        event_version = EVENT_VERSION,
                        observation_key = m.identity.to_string(),
                        source_id = m.identity.id().get(),
                        source_generation = m.identity.generation().get(),
                        metric_key = m.descriptor.key,
                        scope = scope_str(m.scope),
                        lifecycle = lifecycle_str(m.lifecycle),
                        kind = kind_str(m.descriptor.kind),
                        unit = unit_str(m.descriptor.unit),
                        validity = validity_str(m.validity),
                        issue = issue_str(m.issue),
                        loss = m.loss.unwrap_or(0),
                        loss_quantified = m.loss.is_some(),
                        observed_at_mono_us = mono_us_since(m.observed_at, t0),
                        last_change_at_mono_us = mono_us_since(m.last_change_at, t0),
                        poll_generation = poll_generation,
                        counter_raw = fields.counter_raw,
                        counter_cumulative = fields.counter_cumulative,
                        counter_delta = fields.counter_delta,
                        counter_delta_present = fields.counter_delta_present,
                        counter_rate_per_sec = fields.counter_rate_per_sec,
                        counter_rate_present = fields.counter_rate_present,
                        gauge_value = fields.gauge_value,
                        gauge_previous = fields.gauge_previous,
                        gauge_previous_present = fields.gauge_previous_present,
                        histogram_bucket_count = fields.histogram_bucket_count,
                        histogram_total_approx = fields.histogram_total_approx,
                    );
                })
            };
        }
        match level {
            Level::DEBUG => go!(Level::DEBUG),
            Level::INFO => go!(Level::INFO),
            // `measurement_level` only yields DEBUG/INFO/WARN; anything
            // else fails safe upward rather than under-reporting.
            _ => go!(Level::WARN),
        }
    }

    /// Emits the per-poll window aggregate — once per `emit` call that
    /// processed at least one eligible measurement, so quiet polls do
    /// not spam events while cap losses stay observable.
    fn emit_window(&self, batch: &ObservationBatch, stats: &EmitStats, t0: Instant) {
        let w = batch.window();
        let level = if stats.cap_dropped > 0 {
            Level::WARN
        } else {
            Level::INFO
        };
        macro_rules! go {
            ($lvl:expr) => {
                dispatcher::with_default(&self.dispatch, || {
                    tracing::event!(
                        target: TARGET,
                        $lvl,
                        event_code = EVENT_CODE_WINDOW,
                        event_version = EVENT_VERSION,
                        poll_generation = batch.generation(),
                        window_generation = w.generation,
                        window_started_mono_us = mono_us_since(w.started, t0),
                        window_ended_mono_us = mono_us_since(w.ended, t0),
                        window_measured_us = duration_micros(w.measured),
                        window_expected_us = duration_micros(w.expected),
                        planned = w.planned as u64,
                        observed = w.observed as u64,
                        skipped = w.skipped as u64,
                        drained = w.drained as u64,
                        history_evicted = w.history_evicted as u64,
                        clock_anomaly = w.clock_anomaly,
                        clock_domain = w.clock.get(),
                        emitted = stats.emitted,
                        snapshot_suppressed = stats.snapshot_suppressed,
                        policy_suppressed = stats.policy_suppressed,
                        valueless_skipped = stats.valueless_skipped,
                        cap_dropped = stats.cap_dropped,
                    );
                })
            };
        }
        match level {
            Level::INFO => go!(Level::INFO),
            _ => go!(Level::WARN),
        }
    }
}

/// The kind-specific value fields of one measurement event. Fields
/// not applicable to the measurement's [`MetricKind`] stay `0`/`false`.
#[derive(Debug, Default)]
struct ValueFields {
    counter_raw: u64,
    counter_cumulative: u64,
    counter_delta: u64,
    counter_delta_present: bool,
    counter_rate_per_sec: f64,
    counter_rate_present: bool,
    gauge_value: u64,
    gauge_previous: u64,
    gauge_previous_present: bool,
    histogram_bucket_count: u64,
    histogram_total_approx: u64,
}

impl From<&MetricValue> for ValueFields {
    fn from(value: &MetricValue) -> Self {
        match value {
            MetricValue::Counter {
                raw,
                cumulative_since_registration,
                delta,
                rate_per_sec,
            } => Self {
                counter_raw: *raw,
                counter_cumulative: *cumulative_since_registration,
                counter_delta: delta.unwrap_or(0),
                counter_delta_present: delta.is_some(),
                counter_rate_per_sec: rate_per_sec.unwrap_or(0.0),
                counter_rate_present: rate_per_sec.is_some(),
                ..Self::default()
            },
            MetricValue::Gauge { value, previous } => Self {
                gauge_value: *value,
                gauge_previous: previous.unwrap_or(0),
                gauge_previous_present: previous.is_some(),
                ..Self::default()
            },
            MetricValue::Histogram {
                buckets,
                total_approx,
            } => Self {
                histogram_bucket_count: buckets.len() as u64,
                histogram_total_approx: *total_approx,
                ..Self::default()
            },
        }
    }
}

/// The monotonic report-signal an `OnIncrease` gate accumulates over —
/// per kind: counter cumulative-since-registration, gauge value,
/// histogram approximate total.
fn report_signal(value: &MetricValue) -> u64 {
    match value {
        MetricValue::Counter {
            cumulative_since_registration,
            ..
        } => *cumulative_since_registration,
        MetricValue::Gauge { value, .. } => *value,
        MetricValue::Histogram { total_approx, .. } => *total_approx,
    }
}

/// Microseconds of a [`Duration`], clamped to `u64`.
fn duration_micros(d: Duration) -> u64 {
    u64::try_from(d.as_micros()).unwrap_or(u64::MAX)
}
