//! Observation window — the per-poll interval descriptor carried by
//! every [`ObservationBatch`](crate::collector::ObservationBatch).

use std::time::{Duration, Instant};

use ruviform_media_core::ClockDomainId;

/// The observation interval one [`crate::collector::Collector::poll`]
/// covered.
///
/// `started`/`ended` describe the *covered observation interval*, not
/// the poll's execution time: `started` is the previous batch's
/// `ended` (generation 1: the collector's `created_at`) and `ended` is
/// the caller-supplied `now`. `measured` is the real elapsed width —
/// a delayed poll records its *measured* interval, not the scheduled
/// `expected` one.
#[derive(Debug, Clone)]
pub struct ObservationWindow {
    /// The poll generation this window belongs to (matches the
    /// batch/snapshot generation).
    pub generation: u64,
    /// Start of the covered interval — previous batch's `ended`, or
    /// the collector's creation instant for generation 1.
    pub started: Instant,
    /// End of the covered interval — the caller's `now`.
    pub ended: Instant,
    /// The configured `poll_interval` — the scheduled width.
    pub expected: Duration,
    /// `ended - started` — the measured interval actually covered.
    /// `Duration::ZERO` when `ended` precedes `started` (a
    /// `clock_anomaly` window).
    pub measured: Duration,
    /// The clock domain all window instants come from —
    /// [`ClockDomainId::MONOTONIC`].
    pub clock: ClockDomainId,
    /// Sources considered for this poll (`Active` + retiring/draining).
    pub planned: usize,
    /// Sources actually observed into this batch's measurements.
    pub observed: usize,
    /// `Active` sources deferred by `max_batch_size` this poll.
    pub skipped: usize,
    /// Sources that completed their final drain this poll.
    pub drained: usize,
    /// History entries evicted by this poll's commits.
    pub history_evicted: usize,
    /// A clock anomaly was observed — either the window itself ran
    /// backwards (`ended < started`) or at least one source's elapsed
    /// anchor failed.
    pub clock_anomaly: bool,
}
