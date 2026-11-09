//! Fixed event schema — the single source of truth for event names,
//! `event_version`, field keys, enum→string mappings, severity mapping,
//! and monotonic-time encoding used by [`super::TracingExporter`],
//! [`super::TracingFindingSink`], and [`super::TracingRecordExporter`].
//!
//! # Schema contract
//!
//! Every emitted event carries `event_code` (which event this is),
//! `event_version` (schema revision — bump on any field/key/meaning
//! change), and `target = "ruviform.observability"`. Field keys are
//! `snake_case` and stable; consumers must tolerate unknown *values* but
//! may rely on the documented key set per `event_code`.
//!
//! # Time encoding
//!
//! [`std::time::Instant`] has no wall-clock epoch, so timestamps are
//! emitted as `*_mono_us`: microseconds relative to the exporter's
//! first observed instant (its `t0`). They are monotonic, honest
//! relative offsets inside the host clock domain — never fabricated
//! wall-clock time and never converted across clock domains.

use std::time::Instant;

use ruviform_media_core::TimestampProvenance;
use tracing::Level;

use crate::diagnostics::HealthState;
use crate::metrics::{MeasurementIssue, MeasurementValidity};
use crate::record::UnlinkedReason;
use crate::source::{
    Confidentiality, MetricKind, ObservationScope, ObservationUnit, ReportingImpact,
    SourceLifecycle,
};

/// `tracing` target shared by every event this module emits.
pub(crate) const TARGET: &str = "ruviform.observability";

/// `event_code` of a per-measurement observation event.
pub(crate) const EVENT_CODE_MEASUREMENT: &str = "metric_observation";
/// `event_code` of the per-poll window aggregate event.
pub(crate) const EVENT_CODE_WINDOW: &str = "observation_window";
/// `event_code` of a diagnostics health-transition event.
pub(crate) const EVENT_CODE_HEALTH_TRANSITION: &str = "diagnostics.health_transition";
/// `event_code` of an RT [`crate::record::EventRecord`] event.
pub(crate) const EVENT_CODE_RECORD: &str = "rt_record";

/// Schema revision carried by every event's `event_version` field.
pub(crate) const EVENT_VERSION: u64 = 1;

/// `kind` field value for [`MetricKind::Counter`].
pub(crate) const fn kind_str(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Counter => "counter",
        MetricKind::Gauge => "gauge",
        MetricKind::Histogram => "histogram",
    }
}

/// `unit` field value for an [`ObservationUnit`].
pub(crate) const fn unit_str(unit: ObservationUnit) -> &'static str {
    match unit {
        ObservationUnit::None => "none",
        ObservationUnit::Count => "count",
        ObservationUnit::Bytes => "bytes",
        ObservationUnit::Nanoseconds => "nanoseconds",
        ObservationUnit::Samples => "samples",
        ObservationUnit::Frames => "frames",
    }
}

/// `scope` field value for an [`ObservationScope`].
pub(crate) const fn scope_str(scope: ObservationScope) -> &'static str {
    match scope {
        ObservationScope::Process => "process",
        ObservationScope::Instance => "instance",
        ObservationScope::Session => "session",
    }
}

/// `lifecycle` field value for a [`SourceLifecycle`].
pub(crate) const fn lifecycle_str(lifecycle: SourceLifecycle) -> &'static str {
    match lifecycle {
        SourceLifecycle::Active => "active",
        SourceLifecycle::Retiring => "retiring",
        SourceLifecycle::FinalDrain => "final_drain",
        SourceLifecycle::Removed => "removed",
    }
}

/// `validity` field value for a [`MeasurementValidity`].
pub(crate) const fn validity_str(validity: MeasurementValidity) -> &'static str {
    match validity {
        MeasurementValidity::Valid => "valid",
        MeasurementValidity::Stale => "stale",
        MeasurementValidity::Partial => "partial",
        MeasurementValidity::Discontinuous => "discontinuous",
        MeasurementValidity::Unavailable => "unavailable",
        MeasurementValidity::Unsupported => "unsupported",
    }
}

/// `issue` field value for a [`MeasurementIssue`]; `None` maps to the
/// stable `"none"` sentinel so the field is always present.
pub(crate) const fn issue_str(issue: Option<MeasurementIssue>) -> &'static str {
    match issue {
        None => "none",
        Some(MeasurementIssue::CounterDecreased) => "counter_decreased",
        Some(MeasurementIssue::ClockAnomaly) => "clock_anomaly",
        Some(MeasurementIssue::WriterNotQuiesced) => "writer_not_quiesced",
        Some(MeasurementIssue::HistogramInconsistent) => "histogram_inconsistent",
    }
}

/// `correlation_status` field values for `rt_record` events: the
/// record carried no link (`none`), the book classified the link as
/// usable (`linked`), or the link is stale/unknown (`unlinked`).
pub(crate) const CORRELATION_STATUS_NONE: &str = "none";
pub(crate) const CORRELATION_STATUS_LINKED: &str = "linked";
pub(crate) const CORRELATION_STATUS_UNLINKED: &str = "unlinked";

/// `clock_status` field values for `rt_record` events.
pub(crate) const CLOCK_STATUS_STAMPED: &str = "stamped";
pub(crate) const CLOCK_STATUS_UNSTAMPED: &str = "unstamped";

/// `confidentiality` field value for a [`Confidentiality`]. The value
/// is the record's own classification only — this exporter never
/// enforces or interprets it.
pub(crate) const fn confidentiality_str(confidentiality: Confidentiality) -> &'static str {
    match confidentiality {
        Confidentiality::Public => "public",
        Confidentiality::Internal => "internal",
        Confidentiality::Confidential => "confidential",
    }
}

/// `unlinked_reason` field value; `None` (linked or intentionally
/// uncorrelated records) maps to the stable `"none"` sentinel so the
/// field is always present.
pub(crate) const fn unlinked_reason_str(reason: Option<UnlinkedReason>) -> &'static str {
    match reason {
        None => "none",
        Some(UnlinkedReason::UnknownToken) => "unknown_token",
        Some(UnlinkedReason::SupersededEpoch) => "superseded_epoch",
        Some(UnlinkedReason::Expired) => "expired",
    }
}

/// `clock_provenance` field value for a [`TimestampProvenance`].
///
/// `TimestampProvenance` is `#[non_exhaustive]` in
/// `ruviform-media-core`, so a future variant added there maps to the
/// safe `"unknown"` sentinel here instead of failing to compile.
pub(crate) const fn clock_provenance_str(provenance: TimestampProvenance) -> &'static str {
    match provenance {
        TimestampProvenance::Carried => "carried",
        TimestampProvenance::Derived => "derived",
        TimestampProvenance::Estimated => "estimated",
        _ => "unknown",
    }
}

/// `health_state`/`from_state`/`to_state` field value for a
/// [`HealthState`].
pub(crate) const fn health_state_str(state: HealthState) -> &'static str {
    match state {
        HealthState::Healthy => "healthy",
        HealthState::Unknown => "unknown",
        HealthState::Degraded => "degraded",
        HealthState::Unhealthy => "unhealthy",
    }
}

/// Level contribution of the domain-declared [`ReportingImpact`].
///
/// This is the *only* severity input derived from reporting metadata —
/// counter deltas and retry/poll frequency never infer severity.
/// `ERROR` is never produced for measurement events.
pub(crate) const fn impact_level(impact: ReportingImpact) -> Level {
    match impact {
        ReportingImpact::Low => Level::DEBUG,
        ReportingImpact::Normal => Level::INFO,
        ReportingImpact::High => Level::WARN,
    }
}

/// Severity floor contributed by [`MeasurementValidity`].
///
/// `Valid` contributes `TRACE` (i.e. never raises the level); reduced
/// confidence raises it so a discontinuity can never be emitted below
/// `WARN` regardless of the declared impact.
pub(crate) const fn validity_floor(validity: MeasurementValidity) -> Level {
    match validity {
        MeasurementValidity::Valid => Level::TRACE,
        MeasurementValidity::Stale => Level::INFO,
        MeasurementValidity::Partial
        | MeasurementValidity::Discontinuous
        | MeasurementValidity::Unavailable
        | MeasurementValidity::Unsupported => Level::WARN,
    }
}

/// Event level for a measurement event: the greater of the declared
/// impact level and the validity floor.
///
/// `tracing::Level`'s `Ord` is *verbosity* order — `TRACE` is the
/// greatest and `ERROR` the least — so the higher-*severity* of two
/// levels is the `Ord` **minimum**, not the maximum.
pub(crate) fn measurement_level(impact: ReportingImpact, validity: MeasurementValidity) -> Level {
    std::cmp::min(impact_level(impact), validity_floor(validity))
}

/// Level for a diagnostics health-transition event, derived from the
/// *destination* [`HealthState`] — consistent with the OBS-09 finding
/// severity mapping (`Degraded` → WARN, `Unhealthy` → ERROR,
/// `Unknown`/`Healthy` → INFO).
pub(crate) const fn health_transition_level(to: HealthState) -> Level {
    match to {
        HealthState::Healthy | HealthState::Unknown => Level::INFO,
        HealthState::Degraded => Level::WARN,
        HealthState::Unhealthy => Level::ERROR,
    }
}

/// Level for an `rt_record` event: the declared
/// [`ReportingImpact`] level, raised to at least `WARN` when the
/// envelope reports positional loss (`dropped_before > 0` — the RT
/// bridge dropped records immediately before this one). `ERROR` is
/// never produced; unlinked correlation and `Estimated` provenance are
/// reported as fields and never raise severity.
///
/// Same `Ord`-is-verbosity rule as [`measurement_level`]: the higher
/// *severity* of two levels is the `Ord` minimum.
pub(crate) fn record_level(impact: ReportingImpact, dropped_before: u64) -> Level {
    let loss_floor = if dropped_before > 0 {
        Level::WARN
    } else {
        Level::TRACE
    };
    std::cmp::min(impact_level(impact), loss_floor)
}

/// Microseconds between `t` and `t0`, saturating at zero and clamped
/// to `u64`. `t0` is the exporter's first observed instant, so results
/// are stable monotonic offsets within the exporter lifetime.
pub(crate) fn mono_us_since(t: Instant, t0: Instant) -> u64 {
    u64::try_from(t.saturating_duration_since(t0).as_micros()).unwrap_or(u64::MAX)
}
