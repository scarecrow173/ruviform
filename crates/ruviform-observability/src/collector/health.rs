//! Collector health — a logging-independent, non-recursive view of
//! the collector's own state, error and loss counters.

use std::time::{Duration, Instant};

/// Top-level state of a [`crate::collector::Collector`] — readable
/// without the registry lock (the state lives in an atomic).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectorState {
    /// Accepting registrations and polls.
    Running,
    /// `shutdown()` in progress — registrations race in with
    /// [`crate::source::RegistrationError::FinalDrainInProgress`],
    /// polls fail `CollectorStopped`.
    Draining,
    /// Fully drained — the collector answered its last poll.
    Stopped,
}

/// A point-in-time read of the collector's health counters.
///
/// Health is produced on demand from internal counters and never
/// registers itself as a source — collector self-observation cannot
/// recurse.
#[derive(Debug, Clone)]
pub struct CollectorHealth {
    /// Collector lifecycle state at read time.
    pub state: CollectorState,
    /// Completed polls.
    pub polls_total: u64,
    /// Polls whose measured window exceeded `poll_interval`.
    pub overdue_polls_total: u64,
    /// `now` of the last completed poll.
    pub last_poll_at: Option<Instant>,
    /// How long the last poll's commit took.
    pub last_poll_duration: Option<Duration>,
    /// Successful registrations.
    pub registered_total: u64,
    /// Rejected registrations (any [`RegistrationError`](crate::source::RegistrationError)).
    pub registration_rejected_total: u64,
    /// Currently `Active` sources.
    pub sources_active: usize,
    /// Currently `Retiring`/`FinalDrain` sources awaiting drain.
    pub sources_retiring: usize,
    /// Sources that completed `Removed` since collector start.
    pub sources_removed_total: u64,
    /// Drains committed for abandoned registrations (lease dropped
    /// without `retire`).
    pub abandoned_sources_total: u64,
    /// Measurements committed across all polls.
    pub samples_total: u64,
    /// `Active` sources deferred by `max_batch_size` across all polls.
    pub samples_skipped_total: u64,
    /// Counter decreases/wrap suspicions detected.
    pub discontinuities_total: u64,
    /// Timestamp-ordering anomalies detected (window or per-source
    /// anchor failures).
    pub clock_anomalies_total: u64,
    /// Descriptor/reader histogram mismatches caught at observe time
    /// (registration validates boundaries — this is defensive).
    pub histogram_inconsistencies_total: u64,
    /// Lifecycle transitions that returned `InvalidTransition` during
    /// commit — defensive; zero in a correct implementation.
    pub lifecycle_errors_total: u64,
    /// Measurements currently held in the history ring.
    pub history_len: usize,
    /// Approximate bytes held in the history ring.
    pub history_bytes: usize,
    /// History entries evicted by capacity since collector start.
    pub history_evicted_total: u64,
    /// History entries refused because one entry exceeds
    /// `history_budget_bytes` (still present in batch/snapshot).
    pub history_oversized_dropped_total: u64,
    /// Retained-removed tombstones evicted to reclaim series budget —
    /// a tombstone disappearing from snapshots without a batch
    /// reaching a lagging subscriber.
    pub removed_series_evicted_total: u64,
    /// Times the registry mutex was recovered from poisoning — a
    /// panic inside a critical section did not brick observability.
    pub poison_recoveries_total: u64,
}

/// The mutable counters [`CollectorHealth`] snapshots — collector
/// internal, zeroed at start.
#[derive(Debug, Default)]
pub(crate) struct HealthCounters {
    pub polls_total: u64,
    pub overdue_polls_total: u64,
    pub last_poll_at: Option<Instant>,
    pub last_poll_duration: Option<Duration>,
    pub registered_total: u64,
    pub registration_rejected_total: u64,
    pub sources_removed_total: u64,
    pub abandoned_sources_total: u64,
    pub samples_total: u64,
    pub samples_skipped_total: u64,
    pub discontinuities_total: u64,
    pub clock_anomalies_total: u64,
    pub histogram_inconsistencies_total: u64,
    pub lifecycle_errors_total: u64,
    pub history_evicted_total: u64,
    pub history_oversized_dropped_total: u64,
    pub removed_series_evicted_total: u64,
    pub poison_recoveries_total: u64,
}
