//! Timestamp provenance and frame-rate-agnostic timing observation.
//!
//! [`TickTimestamp`] and [`MediaTimestamp`] pair a timestamp with
//! [`TimestampProvenance`] so consumers can distinguish container-carried
//! values from derived or estimated ones. [`observe_timing`] consumes a
//! raw tick sequence and reports interval statistics, drift, gaps,
//! duplicates, and drop candidates without ever assuming a nominal frame
//! rate — the *interpretation* of those facts (validity, confidence,
//! anomaly labels) belongs to the caller.

use crate::{MediaTime, TimeBase};

/// Where a timestamp value came from.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimestampProvenance {
    /// The value was stored explicitly in the container bitstream.
    Carried,
    /// The value was computed from container-declared structure (sample
    /// tables, mux order, declared rates) — a container fact, not a
    /// backend guess.
    Derived,
    /// The value was extrapolated or synthesized by the backend or
    /// decoder (e.g. anchor advancement past a missing timestamp) — an
    /// estimate that must not be trusted as a stored fact.
    Estimated,
}

/// A timestamp expressed as a raw tick count in the containing stream's
/// time base, plus its provenance.
///
/// Raw packets and native (non-normalized) frames carry ticks rather than
/// [`MediaTime`]: they remain observable even when the stream's
/// [`TimeBase`] is an unavailable [`Fact`](crate::Fact), and they keep the
/// exact container value without an attached (possibly fabricated) base.
/// Resolve ticks into a [`MediaTime`] by pairing with
/// `StreamDescriptor::time_base` via [`TickTimestamp::at`].
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{TickTimestamp, TimeBase, TimestampProvenance};
///
/// let ts = TickTimestamp::carried(90_000);
/// let time = ts.at(TimeBase::COMMON_90K);
/// assert_eq!(time.to_seconds_f64(), 1.0);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TickTimestamp {
    /// Tick count in the containing stream's time base.
    pub ticks: i64,
    /// Where the value came from.
    pub provenance: TimestampProvenance,
}

impl TickTimestamp {
    /// A timestamp carried by the container.
    #[must_use]
    pub const fn carried(ticks: i64) -> Self {
        Self {
            ticks,
            provenance: TimestampProvenance::Carried,
        }
    }

    /// A timestamp derived from container-declared structure.
    #[must_use]
    pub const fn derived(ticks: i64) -> Self {
        Self {
            ticks,
            provenance: TimestampProvenance::Derived,
        }
    }

    /// A timestamp estimated by the backend or decoder.
    #[must_use]
    pub const fn estimated(ticks: i64) -> Self {
        Self {
            ticks,
            provenance: TimestampProvenance::Estimated,
        }
    }

    /// Resolves the tick count into a [`MediaTime`] under `base`.
    ///
    /// **RT contract:** RT-suitable — `Copy`, allocation-free.
    #[must_use]
    pub const fn at(self, base: TimeBase) -> MediaTime {
        MediaTime::new(self.ticks, base)
    }
}

/// A self-contained [`MediaTime`] plus its provenance.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaTimestamp {
    /// The timestamp value.
    pub time: MediaTime,
    /// Where the value came from.
    pub provenance: TimestampProvenance,
}

impl MediaTimestamp {
    /// A timestamp carried by the container.
    #[must_use]
    pub const fn carried(time: MediaTime) -> Self {
        Self {
            time,
            provenance: TimestampProvenance::Carried,
        }
    }

    /// A timestamp derived from container-declared structure.
    #[must_use]
    pub const fn derived(time: MediaTime) -> Self {
        Self {
            time,
            provenance: TimestampProvenance::Derived,
        }
    }

    /// A timestamp estimated by the backend or decoder.
    #[must_use]
    pub const fn estimated(time: MediaTime) -> Self {
        Self {
            time,
            provenance: TimestampProvenance::Estimated,
        }
    }
}

/// A single discontinuity candidate in a timestamp sequence.
///
/// **RT contract:** non-RT only — produced by the allocating
/// [`observe_timing`] analysis path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discontinuity {
    /// Index of the interval (between `ticks[i]` and `ticks[i+1]`).
    pub interval_index: usize,
    /// Observed interval in ticks.
    pub observed_ticks: i64,
    /// Median interval used as the local reference.
    pub reference_ticks: i64,
}

/// Distribution summary of the interval series.
///
/// **RT contract:** non-RT only — produced by the allocating
/// [`observe_timing`] analysis path.
#[derive(Clone, Debug, PartialEq)]
pub struct IntervalStats {
    /// Smallest observed interval (ticks).
    pub min_ticks: i64,
    /// Largest observed interval (ticks).
    pub max_ticks: i64,
    /// Median interval (ticks).
    pub median_ticks: i64,
    /// Mean interval (ticks).
    pub mean_ticks: f64,
}

/// Timing observations for one raw timestamp sequence.
///
/// All fields are facts about the supplied sequence only. Whether a
/// discontinuity is a defect, which clock a sequence belongs to, and how
/// sequences relate across streams are caller-level interpretations —
/// observe each stream's sequence separately and compare them downstream.
///
/// **RT contract:** non-RT only — allocates.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingObservation {
    /// Number of timestamps observed.
    pub timestamp_count: usize,
    /// Consecutive deltas `ticks[i+1] - ticks[i]`.
    pub intervals: Vec<i64>,
    /// Interval distribution summary (`None` for empty sequences).
    pub interval_stats: Option<IntervalStats>,
    /// Measured-vs-nominal drift in ppm, when a nominal interval was
    /// given.
    pub drift_ppm: Option<f64>,
    /// Intervals far outside the local median (gap / jump candidates).
    pub discontinuities: Vec<Discontinuity>,
    /// Indices where `ticks[i] == ticks[i-1]` (duplicate-timestamp
    /// candidates).
    pub duplicates: Vec<usize>,
    /// Indices of *isolated* large intervals — at least twice the local
    /// median while both neighbours stay near it. Sustained regime shifts
    /// (a VFR section slowing down) surface as
    /// [`TimingObservation::discontinuities`], not per-unit drops.
    pub drop_candidates: Vec<usize>,
}

fn median(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Observes a raw timestamp sequence without assuming a frame rate.
///
/// `ticks` is a monotonically non-decreasing sequence in the ticks of
/// whatever stream produced it; the caller knows the base and supplies it
/// only when joining against [`TimeBase`]. `nominal_interval_ticks` is
/// optional reference information only — it is used to report drift and
/// never to "fix" the sequence.
///
/// This is a pure computation over facts: it does not distinguish which
/// timestamps were [`TimestampProvenance::Carried`] versus
/// [`TimestampProvenance::Estimated`] — filter or annotate the sequence
/// before observation when provenance matters.
///
/// **RT contract:** non-RT only — allocates.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::observe_timing;
///
/// let observation = observe_timing(&[0, 1_000, 2_000, 3_000], Some(1_000.0));
/// assert_eq!(observation.timestamp_count, 4);
/// assert!(observation.discontinuities.is_empty());
/// assert_eq!(observation.drift_ppm, Some(0.0));
/// ```
#[must_use]
pub fn observe_timing(ticks: &[i64], nominal_interval_ticks: Option<f64>) -> TimingObservation {
    let intervals: Vec<i64> = ticks.windows(2).map(|w| w[1] - w[0]).collect();
    let med = median(&intervals);
    let stats = if intervals.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        let mean = intervals.iter().map(|v| *v as f64).sum::<f64>() / intervals.len() as f64;
        Some(IntervalStats {
            min_ticks: *intervals.iter().min().unwrap_or(&0),
            max_ticks: *intervals.iter().max().unwrap_or(&0),
            median_ticks: med,
            mean_ticks: mean,
        })
    };

    // A discontinuity is an interval at least 2x the median, or negative
    // when the median is positive. The median keeps VFR variation from
    // triggering on itself. A drop candidate additionally requires the
    // interval to be isolated — both neighbours near the median — so a
    // sustained VFR regime is not misread as dropped frames.
    let mut discontinuities = Vec::new();
    let mut duplicates = Vec::new();
    let mut drop_candidates = Vec::new();
    let near_median = |interval: i64| interval <= med + med / 2;
    for (i, &interval) in intervals.iter().enumerate() {
        if interval == 0 {
            duplicates.push(i + 1);
        } else if med > 0 && (interval >= med * 2 || interval < 0) {
            discontinuities.push(Discontinuity {
                interval_index: i,
                observed_ticks: interval,
                reference_ticks: med,
            });
            let isolated = (i == 0 || near_median(intervals[i - 1]))
                && (i + 1 >= intervals.len() || near_median(intervals[i + 1]));
            if interval >= med * 2 && isolated {
                drop_candidates.push(i);
            }
        }
    }

    let drift_ppm = nominal_interval_ticks.and_then(|nominal| {
        stats
            .as_ref()
            .and_then(|s| (nominal > 0.0).then(|| (s.mean_ticks - nominal) / nominal * 1e6))
    });

    TimingObservation {
        timestamp_count: ticks.len(),
        intervals,
        interval_stats: stats,
        drift_ppm,
        discontinuities,
        duplicates,
        drop_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_tags_survive_construction() {
        assert_eq!(
            TickTimestamp::carried(5).provenance,
            TimestampProvenance::Carried
        );
        assert_eq!(
            TickTimestamp::derived(5).provenance,
            TimestampProvenance::Derived
        );
        assert_eq!(
            TickTimestamp::estimated(5).provenance,
            TimestampProvenance::Estimated
        );
        let mt = MediaTimestamp::estimated(MediaTime::new(42, TimeBase::COMMON_90K));
        assert_eq!(mt.time.value(), 42);
        assert_eq!(mt.provenance, TimestampProvenance::Estimated);
    }

    #[test]
    fn tick_timestamp_resolves_under_a_base() {
        let ts = TickTimestamp::derived(48_000);
        let time = ts.at(TimeBase::sample_rate(48_000).unwrap());
        assert_eq!(time.value(), 48_000);
        assert_eq!(time.to_seconds_f64(), 1.0);
    }

    #[test]
    fn constant_sequence_has_no_anomalies() {
        let pts: Vec<i64> = (0..100).map(|i| i * 1_000).collect();
        let obs = observe_timing(&pts, Some(1_000.0));
        assert_eq!(obs.timestamp_count, 100);
        assert_eq!(obs.intervals.len(), 99);
        assert_eq!(obs.discontinuities, vec![]);
        assert_eq!(obs.duplicates, vec![]);
        assert_eq!(obs.drop_candidates, vec![]);
        assert_eq!(obs.drift_ppm, Some(0.0));
    }

    #[test]
    fn vfr_sequence_keeps_interval_distribution() {
        // Alternating 1000/2000-tick intervals (legitimate VFR cadence).
        let mut pts = vec![0_i64];
        for i in 0..50 {
            pts.push(pts[i] + if i % 2 == 0 { 1_000 } else { 2_000 });
        }
        let obs = observe_timing(&pts, None);
        let stats = obs.interval_stats.unwrap();
        assert_eq!(stats.min_ticks, 1_000);
        assert_eq!(stats.max_ticks, 2_000);
        // Median of a mixed series must not collapse to a nominal rate.
        assert_eq!(obs.discontinuities, vec![]);
    }

    #[test]
    fn gap_is_recorded_as_discontinuity_and_drop() {
        let mut pts: Vec<i64> = (0..30).map(|i| i * 1_000).collect();
        for p in pts.iter_mut().skip(15) {
            *p += 10_000; // jump mid-sequence
        }
        let obs = observe_timing(&pts, None);
        assert_eq!(obs.discontinuities.len(), 1);
        assert_eq!(obs.discontinuities[0].interval_index, 14);
        assert_eq!(obs.drop_candidates, vec![14]);
    }

    #[test]
    fn sustained_vfr_slowdown_is_not_a_drop() {
        // CFR section followed by a sustained slower section: the regime
        // change is a discontinuity but not dropped frames.
        let mut pts: Vec<i64> = (0..40).map(|i| i * 1_000).collect();
        let mut t = *pts.last().unwrap();
        for _ in 0..20 {
            t += 2_000;
            pts.push(t);
        }
        let obs = observe_timing(&pts, None);
        assert_ne!(obs.discontinuities, vec![]);
        assert_eq!(obs.drop_candidates, vec![]);
    }

    #[test]
    fn duplicate_timestamps_are_recorded() {
        let mut pts: Vec<i64> = (0..10).map(|i| i * 1_000).collect();
        pts[5] = pts[4];
        let obs = observe_timing(&pts, None);
        assert_eq!(obs.duplicates, vec![5]);
    }

    #[test]
    fn drift_is_reported_against_nominal() {
        // 1% slow clock: intervals of 1010 vs nominal 1000.
        let pts: Vec<i64> = (0..100).map(|i| i * 1_010).collect();
        let obs = observe_timing(&pts, Some(1_000.0));
        let ppm = obs.drift_ppm.unwrap();
        assert!((ppm - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn origin_offset_series_observes_first_tick_directly() {
        // A stream whose clock starts at a non-zero origin: the offset is
        // part of the raw series and does not perturb the intervals.
        let pts: Vec<i64> = (0..10).map(|i| 5_000 + i * 1_000).collect();
        let obs = observe_timing(&pts, Some(1_000.0));
        assert_eq!(obs.intervals, vec![1_000; 9]);
        assert_eq!(obs.discontinuities, vec![]);
        assert_eq!(obs.drift_ppm, Some(0.0));
    }

    #[test]
    fn negative_interval_is_a_discontinuity() {
        let pts = [0_i64, 1_000, 500, 1_500];
        let obs = observe_timing(&pts, None);
        assert_eq!(obs.discontinuities.len(), 1);
        assert_eq!(obs.discontinuities[0].observed_ticks, -500);
    }

    #[test]
    fn empty_and_single_sequences_are_explicit_empty_results() {
        let empty = observe_timing(&[], None);
        assert_eq!(empty.timestamp_count, 0);
        assert!(empty.interval_stats.is_none());
        assert_eq!(empty.intervals, vec![]);

        let single = observe_timing(&[42], Some(1_000.0));
        assert_eq!(single.timestamp_count, 1);
        assert!(single.interval_stats.is_none());
        assert_eq!(single.drift_ppm, None);
    }
}
