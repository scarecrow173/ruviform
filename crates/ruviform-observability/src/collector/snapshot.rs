//! Metrics snapshot — the latest measurement per tracked series as of
//! one poll generation, `Arc`-shared so multiple pull consumers read
//! the same generation.

use std::sync::Arc;
use std::time::Instant;

use crate::metrics::Measurement;
use crate::source::SourceIdentity;

/// A generation-tagged, shareable view of the latest
/// [`Measurement`] of every tracked series — live sources plus
/// retained-removed tombstones (`validity: Unavailable`).
///
/// `Clone` is an `Arc` bump — every clone sees the same generation.
/// Multiple subscribers can hold snapshots of different generations;
/// a missed batch is diagnosable by comparing `generation()` gaps.
///
/// The snapshot stays readable after the collector stops — the last
/// committed generation remains the final answer, never erased.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    generation: u64,
    taken_at: Instant,
    measurements: Vec<Measurement>,
}

impl MetricsSnapshot {
    /// Builds a snapshot — collector-internal.
    pub(crate) fn new(generation: u64, taken_at: Instant, measurements: Vec<Measurement>) -> Self {
        Self {
            inner: Arc::new(Inner {
                generation,
                taken_at,
                measurements,
            }),
        }
    }

    /// The poll generation that produced this snapshot — `0` before
    /// the first poll.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.generation
    }

    /// When this snapshot was committed.
    #[must_use]
    pub fn taken_at(&self) -> Instant {
        self.inner.taken_at
    }

    /// The latest measurement of every tracked series — live sources
    /// plus retained-removed tombstones (`validity: Unavailable`).
    #[must_use]
    pub fn measurements(&self) -> &[Measurement] {
        &self.inner.measurements
    }

    /// The latest measurement of one registration, if tracked.
    #[must_use]
    pub fn get(&self, identity: SourceIdentity) -> Option<&Measurement> {
        self.inner
            .measurements
            .iter()
            .find(|m| m.identity == identity)
    }

    /// The number of tracked series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.measurements.len()
    }

    /// Whether any series is tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.measurements.is_empty()
    }
}
