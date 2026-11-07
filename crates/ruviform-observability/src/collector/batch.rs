//! Observation batch — the owned result of one
//! [`Collector::poll`](crate::collector::Collector::poll), `Arc`-shared
//! so several pull consumers read the same generation.

use std::sync::Arc;

use crate::collector::{MetricsSnapshot, ObservationWindow};
use crate::metrics::Measurement;
use crate::source::SourceIdentity;

/// The typed result of one poll: the window it covered, the
/// measurements observed, the registrations that reached `Removed`,
/// and a reference to the post-commit [`MetricsSnapshot`].
///
/// `Clone` is an `Arc` bump — every clone sees the same generation.
/// Missed batches are diagnosable by `generation()` gaps between
/// consecutive [`CollectorHandle::latest_batch`](crate::collector::CollectorHandle::latest_batch)
/// reads.
#[derive(Debug, Clone)]
pub struct ObservationBatch {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    generation: u64,
    window: ObservationWindow,
    measurements: Vec<Measurement>,
    removed: Vec<SourceIdentity>,
    snapshot: MetricsSnapshot,
}

impl ObservationBatch {
    /// Builds a batch — collector-internal.
    pub(crate) fn new(
        generation: u64,
        window: ObservationWindow,
        measurements: Vec<Measurement>,
        removed: Vec<SourceIdentity>,
        snapshot: MetricsSnapshot,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                generation,
                window,
                measurements,
                removed,
                snapshot,
            }),
        }
    }

    /// The poll generation — consecutive polls yield consecutive
    /// generations.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.generation
    }

    /// The covered observation interval.
    #[must_use]
    pub fn window(&self) -> &ObservationWindow {
        &self.inner.window
    }

    /// The measurements committed this poll — one per observed or
    /// drained source.
    #[must_use]
    pub fn measurements(&self) -> &[Measurement] {
        &self.inner.measurements
    }

    /// Registrations that reached `Removed` in this poll — each also
    /// appears in `measurements` as the final drain commit.
    #[must_use]
    pub fn removed(&self) -> &[SourceIdentity] {
        &self.inner.removed
    }

    /// The post-commit snapshot this batch produced — all tracked
    /// series' latest measurements at this generation.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.inner.snapshot.clone()
    }
}
