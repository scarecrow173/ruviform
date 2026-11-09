//! Timing bridge endpoints — [`timing_bridge`] prepares the bounded
//! SPSC path and returns the RT writer ([`TimingRecorder`]) and the
//! non-RT collector ([`TimingSink`], in [`super::sink`]).
//!
//! Construction is the module's only allocation point (ring buffer +
//! histogram buckets + shared counter cells) and is **non-RT only**.
//! After preparation the RT side performs exactly one operation —
//! [`TimingRecorder::record`] — which is a `const` header build plus
//! [`crate::rt_bridge::RtRecorder::try_record`]: no allocation, no
//! lock, no panic path beyond the inner bridge, no tracing or
//! subscriber call, no I/O.

use crate::core::{HistogramError, histogram};
use crate::rt_bridge::{RtRecorder, bridge};
use crate::source::SourceIdentity;

use super::record::{TimingClock, TimingRecord, TimingSample};
use super::sink::TimingSink;

/// Prepares a timing bridge — the module's only allocation point.
///
/// Returns the RT-side [`TimingRecorder`] and the non-RT
/// [`TimingSink`]. `capacity` is the ring's slot count (same contract
/// as [`crate::rt_bridge::bridge`]: `0` panics). `clock` fixes the
/// [`TimingClock`] stamped into every record; `source` is the
/// registration identity carried in every record's header.
/// `duration_boundaries` are the fixed histogram bucket edges **in
/// clock ticks** — strictly ascending and non-empty, per
/// [`histogram`]'s contract; `Err(HistogramError)` is returned for
/// invalid boundaries.
///
/// One bridge serves one clock domain and one coherent duration
/// distribution: build one bridge per measurement point (or per clock
/// domain) and register each sink's [`crate::core::HistogramReader`]
/// as its own OBS-02/OBS-07 metric source. [`EventId`] in each
/// [`TimingSample`] still distinguishes points inside a shared
/// distribution.
///
/// **Non-RT only** — allocates the ring, histogram buckets and shared
/// counter cells.
///
/// # Panics
///
/// Panics when `capacity == 0` (same contract as
/// [`crate::rt_bridge::bridge`]).
///
/// # Examples
///
/// ```no_run
/// use ruviform_media_core::{ClockDomainId, ClockEpochId, TimeBase, TimestampProvenance};
/// use ruviform_observability::record::EventId;
/// use ruviform_observability::rt_timing::{TimingClock, TimingSample, timing_bridge};
/// use ruviform_observability::source::{SourceGeneration, SourceId, SourceIdentity};
///
/// let clock = TimingClock::new(
///     ClockDomainId::MONOTONIC,
///     TimeBase::new(1, 1_000_000).unwrap(),
///     TimestampProvenance::Carried,
///     ClockEpochId::new(1),
/// );
/// let source = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0));
/// let (mut recorder, mut sink) =
///     timing_bridge(256, clock, source, &[100, 500, 1_000, 5_000]).unwrap();
///
/// // RT side (inside the measured callback's caller):
/// let sample = TimingSample {
///     point: EventId::new(7),
///     generation: 0,
///     budget_ticks: 500,
///     start_tick: 1_000,
///     end_tick: 1_240,
/// };
/// assert!(recorder.record(sample));
///
/// // Non-RT side:
/// let _window = sink.poll();
/// let _snapshot = sink.snapshot();
/// ```
pub fn timing_bridge(
    capacity: usize,
    clock: TimingClock,
    source: SourceIdentity,
    duration_boundaries: &[u64],
) -> Result<(TimingRecorder, TimingSink), HistogramError> {
    let (histogram, _reader) = histogram(duration_boundaries)?;
    let (inner, sink_inner) = bridge::<TimingRecord>(capacity);
    Ok((
        TimingRecorder {
            inner,
            clock,
            source,
        },
        TimingSink::new(sink_inner, histogram, clock),
    ))
}

/// The RT writer endpoint of a timing bridge — single-producer only.
///
/// `Send` (the recorder is meant to cross to its producing thread),
/// `!Sync`, not `Clone` — the bridge is SPSC. Only [`record`](Self::record),
/// [`dropped`](Self::dropped) and [`recorded`](Self::recorded) are
/// RT-suitable; moving the recorder into/out of the RT path and
/// dropping it are non-RT.
pub struct TimingRecorder {
    inner: RtRecorder<TimingRecord>,
    clock: TimingClock,
    source: SourceIdentity,
}

impl TimingRecorder {
    /// Records one measured interval.
    ///
    /// Builds the [`TimingRecord`] — a `const`, allocation-free header
    /// construction — and offers it to the bounded bridge. Returns
    /// `true` when accepted; `false` when the ring was full, in which
    /// case the record is **dropped** (never recoverable): the loss is
    /// counted by [`dropped`](Self::dropped) and surfaced to the sink
    /// via `dropped_before` on the next accepted record.
    ///
    /// **RT contract:** RT-suitable — one bounded `try_push` plus at
    /// most two `Relaxed` counter updates inside the inner bridge; the
    /// header build is pure `const` arithmetic. No alloc, no lock, no
    /// panic path, no tracing/subscriber call, no I/O.
    pub fn record(&mut self, sample: TimingSample) -> bool {
        self.inner
            .try_record(TimingRecord::new(self.clock, self.source, sample))
    }

    /// Total records dropped on a full ring — shared `Relaxed` counter.
    ///
    /// **RT contract:** RT-suitable — wait-free `Relaxed` load.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.inner.dropped()
    }

    /// Total records accepted — shared `Relaxed` counter.
    ///
    /// **RT contract:** RT-suitable — wait-free `Relaxed` load.
    #[must_use]
    pub fn recorded(&self) -> u64 {
        self.inner.recorded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceGeneration, SourceId};
    use ruviform_media_core::{ClockDomainId, ClockEpochId, TimeBase, TimestampProvenance};

    fn clock() -> TimingClock {
        TimingClock::new(
            ClockDomainId::MONOTONIC,
            TimeBase::new(1, 1_000_000).unwrap(),
            TimestampProvenance::Carried,
            ClockEpochId::new(1),
        )
    }

    fn source() -> SourceIdentity {
        SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0))
    }

    #[test]
    fn timing_bridge_rejects_invalid_boundaries() {
        assert_eq!(
            timing_bridge(4, clock(), source(), &[]).err(),
            Some(HistogramError::EmptyBoundaries)
        );
        assert_eq!(
            timing_bridge(4, clock(), source(), &[10, 10]).err(),
            Some(HistogramError::UnsortedBoundaries)
        );
    }

    #[test]
    fn endpoints_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TimingRecorder>();
        assert_send::<TimingSink>();
    }
}
