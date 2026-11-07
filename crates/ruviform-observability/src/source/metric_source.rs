//! Metric sources — the registration-time bundle of an immutable
//! descriptor, an [`ObservationScope`], and the observing
//! [`MetricReader`] the registry will hold for the source's lifetime.
//!
//! The reader cell is transferred to the registrar at registration:
//! the domain keeps only its writing handles and the
//! [`RegistrationLease`](crate::source::RegistrationLease), so a lease
//! `Drop` can never destroy unreported observations.

use std::error::Error;
use std::fmt;

use crate::core::{CounterReader, GaugeReader, HistogramReader};

use super::descriptor::{CounterDescriptor, MetricDescriptor, MetricKind};
use super::scope::ObservationScope;

/// A counter source ready for registration — descriptor + scope +
/// the [`CounterReader`] transferred to the registry.
///
/// The reader field carries its own kind, so a `CounterSource` cannot
/// express a kind mismatch by construction.
#[derive(Debug)]
pub struct CounterSource {
    /// Immutable counter metadata.
    pub descriptor: CounterDescriptor,
    /// Where the source belongs in the running system.
    pub scope: ObservationScope,
    /// Observing handle transferred to the registry at registration.
    pub reader: CounterReader,
}

/// A metric source ready for registration — descriptor + scope +
/// reader.
///
/// `descriptor.kind` always matches `reader.kind()`:
/// [`MetricSource::new`] rejects mismatches and [`From<CounterSource>`]
/// is consistent by construction.
#[derive(Debug)]
pub struct MetricSource {
    descriptor: MetricDescriptor,
    scope: ObservationScope,
    reader: MetricReader,
}

/// Error returned by [`MetricSource::new`] when the descriptor's
/// `kind` does not match the reader's kind.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBuildError {
    /// `descriptor.kind` and `reader.kind()` disagree.
    KindMismatch,
}

impl fmt::Display for SourceBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::KindMismatch => {
                f.write_str("metric descriptor kind does not match the reader kind")
            }
        }
    }
}

impl Error for SourceBuildError {}

impl MetricSource {
    /// Bundles `descriptor`, `scope` and `reader` into a source, or
    /// returns [`SourceBuildError::KindMismatch`] when the descriptor
    /// kind disagrees with the reader kind.
    pub fn new(
        descriptor: MetricDescriptor,
        scope: ObservationScope,
        reader: MetricReader,
    ) -> Result<Self, SourceBuildError> {
        if descriptor.kind == reader.kind() {
            Ok(Self {
                descriptor,
                scope,
                reader,
            })
        } else {
            Err(SourceBuildError::KindMismatch)
        }
    }

    /// The immutable metric descriptor.
    #[must_use]
    pub fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }

    /// Where the source belongs in the running system.
    #[must_use]
    pub fn scope(&self) -> ObservationScope {
        self.scope
    }

    /// The observing handle the registry holds for this source.
    #[must_use]
    pub fn reader(&self) -> &MetricReader {
        &self.reader
    }
}

impl From<CounterSource> for MetricSource {
    /// Promotes a counter source — consistent by construction, so
    /// infallible.
    fn from(s: CounterSource) -> Self {
        Self {
            descriptor: MetricDescriptor::from(s.descriptor),
            scope: s.scope,
            reader: MetricReader::Counter(s.reader),
        }
    }
}

/// The observing handle of a registered source, tagged by metric
/// kind.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum MetricReader {
    /// Counter observing handle.
    Counter(CounterReader),
    /// Gauge observing handle.
    Gauge(GaugeReader),
    /// Histogram observing handle.
    Histogram(HistogramReader),
}

impl MetricReader {
    /// The metric kind this reader observes.
    #[must_use]
    pub fn kind(&self) -> MetricKind {
        match *self {
            Self::Counter(_) => MetricKind::Counter,
            Self::Gauge(_) => MetricKind::Gauge,
            Self::Histogram(_) => MetricKind::Histogram,
        }
    }

    /// The raw shared-cell address — used by the collector to reject
    /// re-registration of an already-registered cell
    /// (`SourceReused`). Valid only while the cell is alive, which the
    /// registry guarantees by holding this reader for the claim's
    /// duration.
    pub(crate) fn shared_cell_ptr(&self) -> usize {
        match self {
            Self::Counter(r) => r.shared_cell_ptr(),
            Self::Gauge(r) => r.shared_cell_ptr(),
            Self::Histogram(r) => r.shared_cell_ptr(),
        }
    }
}

impl From<CounterReader> for MetricReader {
    fn from(r: CounterReader) -> Self {
        Self::Counter(r)
    }
}

impl From<GaugeReader> for MetricReader {
    fn from(r: GaugeReader) -> Self {
        Self::Gauge(r)
    }
}

impl From<HistogramReader> for MetricReader {
    fn from(r: HistogramReader) -> Self {
        Self::Histogram(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{counter, gauge, histogram};
    use crate::source::descriptor::{
        AggregationPolicy, ObservationUnit, ReportingHint, SourceSemantics,
    };

    fn counter_descriptor() -> CounterDescriptor {
        CounterDescriptor {
            key: "test.counter",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
        }
    }

    fn metric_descriptor(kind: MetricKind) -> MetricDescriptor {
        MetricDescriptor {
            kind,
            key: "test.metric",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::LatestValue,
            buckets: None,
            semantics: SourceSemantics::default(),
        }
    }

    #[test]
    fn counter_source_promotes_into_consistent_metric_source() {
        let (_, reader) = counter();
        let source = MetricSource::from(CounterSource {
            descriptor: counter_descriptor(),
            scope: ObservationScope::Instance,
            reader,
        });
        assert_eq!(source.descriptor().kind, MetricKind::Counter);
        assert_eq!(source.reader().kind(), MetricKind::Counter);
        assert_eq!(source.scope(), ObservationScope::Instance);
    }

    #[test]
    fn metric_source_rejects_kind_mismatch() {
        let (_, gauge_reader) = gauge(0);
        let err = MetricSource::new(
            metric_descriptor(MetricKind::Histogram),
            ObservationScope::Process,
            MetricReader::Gauge(gauge_reader),
        )
        .unwrap_err();
        assert_eq!(err, SourceBuildError::KindMismatch);
    }

    #[test]
    fn metric_source_accepts_matching_kinds() {
        let (_, histogram_reader) = histogram(&[1, 10]).unwrap();
        let source = MetricSource::new(
            metric_descriptor(MetricKind::Histogram),
            ObservationScope::Process,
            MetricReader::Histogram(histogram_reader),
        )
        .unwrap();
        assert_eq!(source.descriptor().kind, MetricKind::Histogram);
    }
}
