//! Non-RT timing sink — drains the bridge, accounts for measured
//! intervals, and reports per-poll windows plus cumulative snapshots
//! (Issue #290 / OBS-08 RT Timing slice).
//!
//! [`TimingSink::poll`] consumes every queued [`TimingRecord`] and
//! returns a [`TimingWindow`]; [`TimingSink::snapshot`] returns the
//! cumulative [`TimingSnapshot`]. Both are **non-RT only** — the sink
//! allocates nothing per record but is not part of the RT contract.
//!
//! # Accounting rules
//!
//! - `end_tick < start_tick` → `clock_anomalies`, excluded from the
//!   duration distribution (a wrapped/reversed reading is never
//!   interpreted as a duration).
//! - Otherwise `duration_ticks` feeds the fixed-bucket
//!   [`Histogram`] (registered with OBS-02/OBS-07 via
//!   [`histogram_reader`](TimingSink::histogram_reader)) and the exact
//!   scalar statistics (count/min/max/mean).
//! - `budget_ticks > 0` enables budget accounting: `duration >
//!   budget` counts `over_budget`, and `duration * 1000 / budget`
//!   (permille) feeds `max_utilization_permille`. This is the
//!   **measured-interval budget utilization** — it is *not* a
//!   deadline overrun, XRUN, or device-scheduling verdict; those are
//!   domain/engine concepts outside this crate.
//! - `budget_ticks == 0` asserts no budget: utilization fields stay
//!   `None` rather than reading as 0% utilization.
//! - Loss is never hidden: `dropped_before` envelopes sum into the
//!   window's `dropped` (positional evidence), while
//!   [`TimingSnapshot::dropped_total`] reports the authoritative
//!   bridge counter including drops whose carrier record has not been
//!   consumed yet. A window with `dropped == 0` is *not* proven
//!   complete — in-flight carriers may attribute loss to a later
//!   window.
//! - Generation is tracked over every consumed record (anomalies
//!   included): `min_generation`/`max_generation`/
//!   `generation_transitions` make generation mixing explicit.
//!
//! # Exact vs estimated
//!
//! Exact scalar statistics cover every accepted sample. The
//! per-sample distribution itself is intentionally not retained
//! (bounded memory): quantiles come from the fixed-bucket histogram
//! as [`QuantileEstimate`] **intervals** at bucket resolution —
//! structurally distinct from the exact statistics.

use crate::core::{CounterReader, Histogram, HistogramReader};
use crate::rt_bridge::{ReceivedRecord, RecordSink};

use super::record::{TimingClock, TimingRecord};

/// Cumulative exact statistics plus the live histogram reader —
/// everything the non-RT side needs to report on a timing bridge.
///
/// `clock` echoes the [`TimingClock`] every record was stamped with.
/// All duration statistics are in the clock's [`TimeBase`] ticks —
/// this type performs no tick→time conversion.
#[derive(Clone, Debug)]
pub struct TimingSnapshot {
    /// The fixed clock identity all records were stamped with.
    pub clock: TimingClock,
    /// Records consumed by `poll` so far.
    pub records: u64,
    /// Valid duration samples (`end_tick >= start_tick`).
    pub samples: u64,
    /// Records rejected as clock anomalies (`end_tick < start_tick`).
    pub clock_anomalies: u64,
    /// Authoritative bridge-wide drop total — includes drops whose
    /// `dropped_before` carrier has not been consumed yet. Per-window
    /// positional evidence lives in [`TimingWindow::dropped`].
    pub dropped_total: u64,
    /// Samples whose duration exceeded their asserted budget
    /// (`budget_ticks > 0` only).
    pub over_budget: u64,
    /// Smallest domain-declared generation seen (`None` before the
    /// first record).
    pub min_generation: Option<u64>,
    /// Largest domain-declared generation seen.
    pub max_generation: Option<u64>,
    /// Generation changes between consecutive consumed records —
    /// attributed to the window containing the later record.
    pub generation_transitions: u64,
    /// Smallest valid duration in ticks (`None` when `samples == 0`).
    pub min_ticks: Option<u64>,
    /// Largest valid duration in ticks.
    pub max_ticks: Option<u64>,
    /// Mean valid duration in ticks (`sum / samples`, saturated).
    pub mean_ticks: Option<u64>,
    /// Largest observed `duration * 1000 / budget` (permille) over
    /// budget-asserted samples (`None` when no sample asserted a
    /// budget).
    pub max_utilization_permille: Option<u64>,
    histogram: HistogramReader,
}

impl TimingSnapshot {
    /// Bucket-resolution **interval estimate** of the duration
    /// quantile at `q_permille` (0..=1000; p50 = 500, p95 = 950,
    /// p99 = 990). Returns `None` when no samples exist or
    /// `q_permille > 1000`.
    ///
    /// This is an *estimate*, not an exact percentile: the true
    /// quantile lies inside the returned interval whose width is one
    /// histogram bucket. The per-sample distribution is not retained,
    /// so no exact quantile is available — use the exact scalar
    /// statistics ([`min_ticks`](Self::min_ticks),
    /// [`max_ticks`](Self::max_ticks), [`mean_ticks`](Self::mean_ticks))
    /// for precise bounds.
    ///
    /// The estimate reads the histogram's latest non-transactional
    /// bucket counts (same caveat as [`HistogramReader::snapshot`]).
    ///
    /// **Non-RT only** — allocates the bucket snapshot.
    #[must_use]
    pub fn estimated_quantile(&self, q_permille: u16) -> Option<QuantileEstimate> {
        if q_permille > 1000 {
            return None;
        }
        let buckets = self.histogram.snapshot();
        let total = buckets.iter().map(|&c| u128::from(c)).sum::<u128>();
        if total == 0 {
            return None;
        }
        // Ceil(q * total / 1000), clamped to ≥1 — the rank of the first
        // sample at-or-above the quantile.
        let target = (u128::from(q_permille) * total).div_ceil(1000).max(1);
        let boundaries = self.histogram.boundaries();
        let mut cumulative = 0u128;
        for (index, &count) in buckets.iter().enumerate() {
            cumulative += u128::from(count);
            if cumulative >= target {
                // `index <= boundaries.len()` always (buckets =
                // boundaries + 1); `get` keeps this panic-free — the
                // top bucket's `upper` is `None` by construction.
                let lower = index
                    .checked_sub(1)
                    .and_then(|i| boundaries.get(i))
                    .copied()
                    .unwrap_or(0);
                return Some(QuantileEstimate {
                    lower_ticks: lower,
                    upper_ticks: boundaries.get(index).copied(),
                });
            }
        }
        None
    }

    /// The live histogram reader behind [`estimated_quantile`](Self::estimated_quantile)
    /// — the same cell [`TimingSink::histogram_reader`] hands to OBS-02/OBS-07
    /// registration.
    #[must_use]
    pub fn histogram_reader(&self) -> &HistogramReader {
        &self.histogram
    }
}

/// A bucket-resolution interval estimate of a duration quantile.
///
/// The true quantile lies within `[lower_ticks, upper_ticks)`;
/// `upper_ticks == None` means the top (unbounded) bucket. Bounds are
/// in the record clock's [`TimeBase`](ruviform_media_core::TimeBase)
/// ticks — no time conversion is performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuantileEstimate {
    /// Lower bound of the estimate interval (inclusive), in ticks.
    pub lower_ticks: u64,
    /// Upper bound of the estimate interval (exclusive), in ticks —
    /// `None` for the unbounded top bucket.
    pub upper_ticks: Option<u64>,
}

/// Per-poll accounting returned by [`TimingSink::poll`].
///
/// `dropped` is the positional sum of `dropped_before` over the
/// records consumed by this poll — loss *evidence*, attributed to the
/// window where its marker arrived. `dropped == 0` does not prove the
/// window complete (in-flight carriers may attribute loss later); the
/// authoritative cumulative total is
/// [`TimingSnapshot::dropped_total`]. Distribution fields describe
/// only surviving samples and must not be presented as a complete
/// distribution when `dropped > 0`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingWindow {
    /// Poll generation — 1-based, increments per `poll` call.
    pub index: u64,
    /// Records consumed by this poll.
    pub records: u64,
    /// Valid duration samples this poll.
    pub samples: u64,
    /// Clock anomalies (`end_tick < start_tick`) this poll.
    pub clock_anomalies: u64,
    /// Positional loss evidence: sum of `dropped_before` over consumed
    /// records.
    pub dropped: u64,
    /// Samples exceeding their asserted budget this poll.
    pub over_budget: u64,
    /// Smallest valid duration this poll (`None` when `samples == 0`).
    pub min_ticks: Option<u64>,
    /// Largest valid duration this poll.
    pub max_ticks: Option<u64>,
    /// Mean valid duration this poll (`None` when `samples == 0`).
    pub mean_ticks: Option<u64>,
    /// Largest budget utilization (permille) this poll (`None` when no
    /// sample asserted a budget).
    pub max_utilization_permille: Option<u64>,
    /// Smallest generation among this poll's records (`None` when
    /// `records == 0`).
    pub min_generation: Option<u64>,
    /// Largest generation among this poll's records.
    pub max_generation: Option<u64>,
    /// Generation changes detected while consuming this poll's
    /// records — including a change from the previous poll's last
    /// record (attributed to this window).
    pub generation_transitions: u64,
}

/// Cumulative exact statistics maintained by [`TimingSink`].
#[derive(Debug, Default)]
struct TimingStats {
    records: u64,
    samples: u64,
    clock_anomalies: u64,
    over_budget: u64,
    sum_ticks: u128,
    min_ticks: Option<u64>,
    max_ticks: Option<u64>,
    max_utilization_permille: Option<u64>,
    min_generation: Option<u64>,
    max_generation: Option<u64>,
    generation_transitions: u64,
    last_generation: Option<u64>,
}

/// Per-window accumulators for the duration/generation statistics that
/// [`TimingWindow`] reports as `Option`s — kept separate from the
/// window's counter fields so `account` stays a single pass.
#[derive(Debug, Default)]
struct WindowAccum {
    sum_ticks: u128,
    min_ticks: Option<u64>,
    max_ticks: Option<u64>,
    max_utilization: Option<u64>,
    min_generation: Option<u64>,
    max_generation: Option<u64>,
}

/// The non-RT collector endpoint of a timing bridge — sole consumer
/// and ring owner.
///
/// `Send` when the inner sink is `Send`; `!Sync`, not `Clone`. Every
/// method is **non-RT only**. Dropping the sink frees the ring — never
/// drop it on an RT thread.
pub struct TimingSink {
    inner: RecordSink<TimingRecord>,
    histogram: Histogram,
    clock: TimingClock,
    stats: TimingStats,
    polls: u64,
}

impl TimingSink {
    /// Wraps the non-RT half of a prepared bridge. Called by
    /// [`super::timing_bridge`] — non-RT only.
    pub(crate) fn new(
        inner: RecordSink<TimingRecord>,
        histogram: Histogram,
        clock: TimingClock,
    ) -> Self {
        Self {
            inner,
            histogram,
            clock,
            stats: TimingStats::default(),
            polls: 0,
        }
    }

    /// Drains every queued record and returns this poll's accounting.
    ///
    /// **Non-RT only** — bounded drain plus fixed-size accounting; no
    /// blocking. Safe to call while the writer is live; records the
    /// writer pushes concurrently may or may not be included.
    #[must_use]
    pub fn poll(&mut self) -> TimingWindow {
        self.polls += 1;
        let mut window = TimingWindow {
            index: self.polls,
            ..TimingWindow::default()
        };
        let mut accum = WindowAccum::default();

        let Self {
            inner,
            histogram,
            stats,
            ..
        } = self;
        inner.drain(|received| {
            Self::account(received, histogram, stats, &mut window, &mut accum);
        });

        window.min_ticks = accum.min_ticks;
        window.max_ticks = accum.max_ticks;
        window.mean_ticks = (window.samples > 0)
            .then(|| saturate_u64(accum.sum_ticks / u128::from(window.samples)));
        window.max_utilization_permille = accum.max_utilization;
        window.min_generation = accum.min_generation;
        window.max_generation = accum.max_generation;
        window
    }

    /// Accounts one received record into cumulative stats, the
    /// histogram, and the per-window accumulators.
    fn account(
        received: ReceivedRecord<TimingRecord>,
        histogram: &Histogram,
        stats: &mut TimingStats,
        window: &mut TimingWindow,
        accum: &mut WindowAccum,
    ) {
        window.records += 1;
        window.dropped += received.dropped_before();

        let record = received.record();
        let generation = record.header().generation();
        stats.records += 1;

        // Generation accounting covers every consumed record —
        // anomalies included — so generation mixing stays explicit.
        stats.min_generation = Some(match stats.min_generation {
            Some(min) => min.min(generation),
            None => generation,
        });
        stats.max_generation = Some(match stats.max_generation {
            Some(max) => max.max(generation),
            None => generation,
        });
        if let Some(previous) = stats.last_generation
            && previous != generation
        {
            stats.generation_transitions += 1;
            window.generation_transitions += 1;
        }
        stats.last_generation = Some(generation);
        accum.min_generation = Some(
            accum
                .min_generation
                .map_or(generation, |min| min.min(generation)),
        );
        accum.max_generation = Some(
            accum
                .max_generation
                .map_or(generation, |max| max.max(generation)),
        );

        let Some(duration) = record.duration_ticks() else {
            stats.clock_anomalies += 1;
            window.clock_anomalies += 1;
            return;
        };

        stats.samples += 1;
        window.samples += 1;
        histogram.observe(duration);
        accum.sum_ticks += u128::from(duration);
        stats.sum_ticks += u128::from(duration);
        stats.min_ticks = Some(stats.min_ticks.map_or(duration, |min| min.min(duration)));
        stats.max_ticks = Some(stats.max_ticks.map_or(duration, |max| max.max(duration)));
        accum.min_ticks = Some(accum.min_ticks.map_or(duration, |min| min.min(duration)));
        accum.max_ticks = Some(accum.max_ticks.map_or(duration, |max| max.max(duration)));

        let budget = record.budget_ticks();
        if budget == 0 {
            // No budget asserted — utilization/overrun accounting is
            // skipped rather than read as zero utilization.
            return;
        }
        if duration > budget {
            stats.over_budget += 1;
            window.over_budget += 1;
        }
        let utilization = saturate_u64((u128::from(duration) * 1000) / u128::from(budget));
        stats.max_utilization_permille = Some(
            stats
                .max_utilization_permille
                .map_or(utilization, |max| max.max(utilization)),
        );
        accum.max_utilization = Some(
            accum
                .max_utilization
                .map_or(utilization, |max| max.max(utilization)),
        );
    }

    /// Cumulative statistics since construction — exact scalars only;
    /// the full per-sample distribution is not retained (see module
    /// docs). Cheap: no heap data beyond the histogram reader's `Arc`
    /// bump.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn snapshot(&self) -> TimingSnapshot {
        TimingSnapshot {
            clock: self.clock,
            records: self.stats.records,
            samples: self.stats.samples,
            clock_anomalies: self.stats.clock_anomalies,
            dropped_total: self.inner.dropped(),
            over_budget: self.stats.over_budget,
            min_generation: self.stats.min_generation,
            max_generation: self.stats.max_generation,
            generation_transitions: self.stats.generation_transitions,
            min_ticks: self.stats.min_ticks,
            max_ticks: self.stats.max_ticks,
            mean_ticks: (self.stats.samples > 0)
                .then(|| saturate_u64(self.stats.sum_ticks / u128::from(self.stats.samples))),
            max_utilization_permille: self.stats.max_utilization_permille,
            histogram: self.histogram.reader(),
        }
    }

    /// Test-only access to the raw record sink — contract tests inspect
    /// bridged [`TimingRecord`]s (stamps, envelopes) before aggregation.
    #[cfg(test)]
    pub(crate) fn inner_sink(&mut self) -> &mut RecordSink<TimingRecord> {
        &mut self.inner
    }

    /// A [`HistogramReader`] on the duration distribution — cloneable
    /// for OBS-02/OBS-07 `MetricSource` registration; the collector
    /// reads it as `MetricValue::Histogram`.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn histogram_reader(&self) -> HistogramReader {
        self.histogram.reader()
    }

    /// A [`CounterReader`] on the accepted-record count — for OBS-02
    /// `CounterSource` registration.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn recorded_reader(&self) -> CounterReader {
        self.inner.recorded_reader()
    }

    /// A [`CounterReader`] on the bridge-wide dropped-record count —
    /// for OBS-02 `CounterSource` registration. This is the
    /// authoritative total (includes drops whose `dropped_before`
    /// carrier has not been consumed yet).
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn dropped_reader(&self) -> CounterReader {
        self.inner.dropped_reader()
    }
}

/// Saturating `u128 → u64` conversion for statistics that must stay
/// bounded even under adversarial tick values.
fn saturate_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
