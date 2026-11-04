//! Metric descriptors — the immutable, domain-declared metadata a
//! domain hands to an [`ObservationRegistrar`](crate::source::ObservationRegistrar)
//! at registration time.
//!
//! Every descriptor field is immutable and uses `&'static str` for
//! names, keys, meanings and origins: source keys and label-ish
//! strings are bound at compile time, which statically bounds the
//! cardinality a domain can mint (runtime cardinality limits — e.g.
//! `max_sources`/`max_series` — are the collector's own
//! configuration, see Issue #289).

use std::ops::RangeInclusive;
use std::time::Duration;

/// The kind of metric a [`MetricDescriptor`] describes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    /// Cumulative, monotonically non-decreasing count.
    Counter,
    /// Latest state value — may increase and decrease.
    Gauge,
    /// Distribution over fixed pre-declared buckets.
    Histogram,
}

/// Unit of the observed quantity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationUnit {
    /// No declared unit.
    None,
    /// Unit-less count of events or items.
    Count,
    /// Bytes.
    Bytes,
    /// Duration or latency in nanoseconds.
    Nanoseconds,
    /// Audio samples.
    Samples,
    /// Video frames.
    Frames,
}

/// How a collector should aggregate the raw reads of this metric.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregationPolicy {
    /// Cumulative count — deltas/rates are derived by the collector
    /// from a baseline it owns.
    Cumulative,
    /// Latest observed value is the metric (gauge semantics).
    LatestValue,
    /// Fixed bucket counts — distribution is derived by the collector.
    BucketCounts,
}

/// Domain-declared impact hint attached to an increase report. This
/// is declared metadata, **not** a severity inferred from observed
/// increments — this crate never derives severity from counter
/// deltas uniformly.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportingImpact {
    /// Routine signal — low attention.
    Low,
    /// Default attention level.
    Normal,
    /// Elevated attention — the domain considers increases notable.
    High,
}

/// Domain-declared reporting preference for a metric.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportingHint {
    /// Included in on-demand snapshots only; no proactive reporting.
    SnapshotOnly,
    /// Report when the metric increases by at least `min_delta`, no
    /// more often than `min_interval`, with the declared `impact`.
    OnIncrease {
        /// Minimum increment that justifies a report.
        min_delta: u64,
        /// Minimum interval between reports.
        min_interval: Duration,
        /// Domain-declared attention hint.
        impact: ReportingImpact,
    },
}

/// How promptly a source needs observation attention — a declared
/// hint, not a scheduling guarantee.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceTemperature {
    /// High-rate / attention-adjacent source.
    Hot,
    /// Default temperature.
    Warm,
    /// Rare, diagnostic-grade source.
    Cold,
}

/// Confidentiality classification of a source's observed values.
/// Export decisions remain the host's authorization call; this label
/// only carries the domain's declaration.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confidentiality {
    /// Safe to expose outside the process.
    Public,
    /// Process-internal use (default).
    Internal,
    /// Confidential — must not leave the host without explicit
    /// authorization.
    Confidential,
}

/// Domain-declared semantics of a metric — what the event means, how
/// fresh it needs to be, what counts as normal, where the data comes
/// from, and how confidential it is.
///
/// These fields are declarative metadata: the domain states them at
/// registration and consumers (collector/exporters/diagnostics)
/// interpret them. This crate performs no conversion and infers no
/// severity from them.
#[derive(Debug, Clone)]
pub struct SourceSemantics {
    /// Human-meaningful description of what the observed event or
    /// state means (e.g. `"submits dropped because the ring was full"`).
    pub meaning: &'static str,
    /// Declared observation-attention temperature.
    pub temperature: SourceTemperature,
    /// Warmup period during which abnormal readings are expected and
    /// should not be interpreted as faults (e.g. plugin startup).
    pub warmup: Option<Duration>,
    /// Range of values considered normal after `warmup` (inclusive).
    /// `None` means the domain declares no normal range.
    pub expected_range: Option<RangeInclusive<u64>>,
    /// Information source — the emitting crate/module/system path
    /// (e.g. `"ruviform-plugin-host::l1::audio"`).
    pub origin: &'static str,
    /// Confidentiality classification of the observed values.
    pub confidentiality: Confidentiality,
}

impl Default for SourceSemantics {
    fn default() -> Self {
        Self {
            meaning: "",
            temperature: SourceTemperature::Warm,
            warmup: None,
            expected_range: None,
            origin: "",
            confidentiality: Confidentiality::Internal,
        }
    }
}

/// Descriptor of a counter source — the minimal registration triple
/// (`key`, `unit`, `reporting`) per Issue #277.
#[derive(Debug, Clone)]
pub struct CounterDescriptor {
    /// Statically bound metric key — bounds series cardinality.
    pub key: &'static str,
    /// Unit of the observed quantity.
    pub unit: ObservationUnit,
    /// Domain-declared reporting preference.
    pub reporting: ReportingHint,
}

/// Descriptor of any metric kind — the domain-declared immutable
/// metadata attached to a [`MetricSource`](crate::source::MetricSource)
/// at registration.
///
/// `kind` must match the kind of the accompanying
/// [`MetricReader`](crate::source::MetricReader); `buckets` must carry
/// the fixed bucket boundaries for `MetricKind::Histogram` (and is
/// meaningless for other kinds).
#[derive(Debug, Clone)]
pub struct MetricDescriptor {
    /// The kind of metric.
    pub kind: MetricKind,
    /// Statically bound metric key — bounds series cardinality.
    pub key: &'static str,
    /// Unit of the observed quantity.
    pub unit: ObservationUnit,
    /// Domain-declared reporting preference.
    pub reporting: ReportingHint,
    /// Collector-side aggregation policy.
    pub aggregation: AggregationPolicy,
    /// Fixed bucket boundaries — required when `kind` is
    /// `MetricKind::Histogram`, ignored otherwise.
    pub buckets: Option<&'static [u64]>,
    /// Domain-declared semantics (meaning, temperature, warmup,
    /// normal range, origin, confidentiality).
    pub semantics: SourceSemantics,
}

impl From<CounterDescriptor> for MetricDescriptor {
    /// Promotes a counter descriptor — `kind`/`aggregation` are fixed
    /// by construction and semantics default to
    /// [`SourceSemantics::default`].
    fn from(d: CounterDescriptor) -> Self {
        Self {
            kind: MetricKind::Counter,
            key: d.key,
            unit: d.unit,
            reporting: d.reporting,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_descriptor_promotes_to_metric_descriptor() {
        let d = MetricDescriptor::from(CounterDescriptor {
            key: "l1.dropped_submits",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
        });
        assert_eq!(d.kind, MetricKind::Counter);
        assert_eq!(d.aggregation, AggregationPolicy::Cumulative);
        assert_eq!(d.key, "l1.dropped_submits");
        assert!(d.buckets.is_none());
    }
}
