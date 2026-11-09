//! RT timing record vocabulary — the fixed-size measurement record that
//! travels [`crate::rt_bridge`] between a real-time writer and a non-RT
//! sink (Issue #290 / OBS-08 RT Timing slice).
//!
//! A [`TimingSample`] is the caller-built input: one measured interval
//! (`start_tick`/`end_tick`) on a caller-declared clock, with an
//! optional callback `budget_ticks` (`0` = no budget asserted) and a
//! domain-declared `generation`. [`TimingRecord`] is what actually
//! crosses the bridge — an OBS-05 [`RecordHeader`] (event identity,
//! source identity, generation, [`ClockContext`] stamped at
//! `start_tick`, confidentiality/impact) plus `end_tick` and
//! `budget_ticks`.
//!
//! The clock identity is **fixed per bridge**: [`TimingClock`] is
//! resolved once at [`crate::rt_timing::timing_bridge`] construction
//! and stamped into every record's [`ClockContext`]. `TimingRecord`
//! values are built only by [`crate::rt_timing::TimingRecorder`], so a
//! record carrying a foreign clock domain/epoch is unreachable by
//! construction — changing the clock means building a new bridge
//! (non-RT). This module performs no tick→time conversion: durations
//! stay in the record's [`TimeBase`] ticks; `ClockAnchor`-based
//! conversion is the domain's responsibility.

use ruviform_media_core::{ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance};

use crate::record::{ClockContext, EventId, RecordHeader};
use crate::source::SourceIdentity;

/// Bridge record schema version stamped into [`RecordHeader::version`]
/// for every [`TimingRecord`].
///
/// **RT contract:** RT-suitable — `const`.
pub const TIMING_SCHEMA_VERSION: u16 = 1;

/// Clock identity fixed when a timing bridge is prepared.
///
/// Composes the OBS-05 clock vocabulary without a tick reading: the
/// [`ClockDomainId`], the [`TimeBase`] the ticks are denominated in,
/// the [`TimestampProvenance`] of the readings, and the
/// [`ClockEpochId`] of the domain incarnation. [`crate::rt_timing::
/// TimingRecorder`] stamps every record's [`ClockContext`] from this
/// value with `position = start_tick`.
///
/// **RT contract:** RT-suitable — `Copy` value type; `const`, pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimingClock {
    domain: ClockDomainId,
    base: TimeBase,
    provenance: TimestampProvenance,
    epoch: ClockEpochId,
}

impl TimingClock {
    /// Fixes the clock identity for one timing bridge.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(
        domain: ClockDomainId,
        base: TimeBase,
        provenance: TimestampProvenance,
        epoch: ClockEpochId,
    ) -> Self {
        Self {
            domain,
            base,
            provenance,
            epoch,
        }
    }

    /// The clock domain the ticks belong to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn domain(self) -> ClockDomainId {
        self.domain
    }

    /// The timebase `start_tick`/`end_tick`/`budget_ticks` are
    /// denominated in (e.g. `1/48000` for a sample clock, `1/rate` for
    /// a monotonic tick rate).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn time_base(self) -> TimeBase {
        self.base
    }

    /// How much of a tick reading is a carried fact versus derived or
    /// estimated — stamped verbatim into each record's
    /// [`ClockContext`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn provenance(self) -> TimestampProvenance {
        self.provenance
    }

    /// The clock-domain incarnation the ticks belong to. Stamps from
    /// different epochs are never comparable — an epoch change requires
    /// a new bridge.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn epoch(self) -> ClockEpochId {
        self.epoch
    }

    /// Stamps a [`ClockContext`] for a measurement starting at
    /// `start_tick` — position carries the interval's *start* reading.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub(crate) const fn clock_context(self, start_tick: u64) -> ClockContext {
        ClockContext::new(
            InputStamp {
                ticks: start_tick,
                base: self.base,
                domain: self.domain,
                provenance: self.provenance,
            },
            self.epoch,
        )
    }
}

/// One measured interval supplied by the caller — the RT-side input to
/// [`crate::rt_timing::TimingRecorder::record`].
///
/// Field semantics:
///
/// - `point`: the measurement point's [`EventId`] — which block,
///   callback, plugin or graph node was timed. Domains allocate their
///   own id space; `0` is valid.
/// - `generation`: a *domain-declared* generation (plan/session/graph
///   incarnation) the measurement was taken under. `0` means "no
///   generation asserted" — the same convention as
///   [`RecordHeader::generation`].
/// - `budget_ticks`: the asserted callback budget **in the clock's
///   ticks**. `0` asserts no budget — utilization and overrun
///   accounting are skipped for that record rather than treated as a
///   zero budget.
/// - `start_tick`/`end_tick`: raw clock readings around the measured
///   interval, in [`TimingClock::time_base`] ticks. `end_tick <
///   start_tick` is a clock anomaly: counted by the sink, never
///   interpreted as a (wrapped) duration.
///
/// The caller owns tick reading and budget selection; this crate only
/// transports and accounts for the interval.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimingSample {
    /// Measurement point identity ([`EventId`]).
    pub point: EventId,
    /// Domain-declared generation (`0` = none asserted).
    pub generation: u64,
    /// Asserted budget in clock ticks; `0` = no budget asserted.
    pub budget_ticks: u64,
    /// Clock reading at interval start.
    pub start_tick: u64,
    /// Clock reading at interval end.
    pub end_tick: u64,
}

/// The fixed-size record that crosses the timing bridge — `Copy +
/// Send + 'static`, bounded like [`crate::record::EventRecord`].
///
/// Layout: a [`RecordHeader`] carrying `event = sample.point`,
/// `version = TIMING_SCHEMA_VERSION`, the bridge's `source`, the
/// sample's `generation`, `clock = Some(ClockContext)` stamped from
/// the bridge's [`TimingClock`] with `position = start_tick`, and the
/// default `Internal`/`Normal` classification — plus `end_tick` and
/// `budget_ticks`. The header's clock position marks the interval
/// **start**; `end_tick` is the only additional tick the record needs.
///
/// Built only by [`crate::rt_timing::TimingRecorder::record`]; consumers
/// read it through the accessors below.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimingRecord {
    header: RecordHeader,
    end_tick: u64,
    budget_ticks: u64,
}

impl TimingRecord {
    /// Builds the record for `sample` under `clock`/`source`. Called by
    /// the RT recorder — `const`, pure, no allocation.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub(crate) const fn new(
        clock: TimingClock,
        source: SourceIdentity,
        sample: TimingSample,
    ) -> Self {
        let header = RecordHeader::new(sample.point, TIMING_SCHEMA_VERSION, source)
            .with_generation(sample.generation)
            .with_clock(Some(clock.clock_context(sample.start_tick)));
        Self {
            header,
            end_tick: sample.end_tick,
            budget_ticks: sample.budget_ticks,
        }
    }

    /// The record's metadata header — event/version/source/generation/
    /// clock/confidentiality/impact.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn header(self) -> RecordHeader {
        self.header
    }

    /// The clock reading at interval end (`TimingSample::end_tick`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn end_tick(self) -> u64 {
        self.end_tick
    }

    /// The asserted budget in clock ticks (`0` = none asserted).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn budget_ticks(self) -> u64 {
        self.budget_ticks
    }

    /// The clock reading at interval start — the header clock
    /// context's `position`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn start_tick(self) -> u64 {
        match self.header.clock() {
            Some(clock) => clock.position(),
            None => 0,
        }
    }

    /// The measured duration in clock ticks — `end_tick - start_tick`,
    /// or `None` when `end_tick < start_tick` (clock anomaly; the sink
    /// counts it separately and never treats it as a duration).
    ///
    /// **RT contract:** RT-suitable — bounded integer ops, no alloc.
    #[must_use]
    pub fn duration_ticks(self) -> Option<u64> {
        self.end_tick.checked_sub(self.start_tick())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceGeneration, SourceId};

    fn clock() -> TimingClock {
        TimingClock::new(
            ClockDomainId::MONOTONIC,
            TimeBase::new(1, 48_000).unwrap(),
            TimestampProvenance::Carried,
            ClockEpochId::new(3),
        )
    }

    fn source() -> SourceIdentity {
        SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0))
    }

    #[test]
    fn record_stamps_clock_generation_and_point() {
        let sample = TimingSample {
            point: EventId::new(9),
            generation: 4,
            budget_ticks: 480,
            start_tick: 1_000,
            end_tick: 1_240,
        };
        let record = TimingRecord::new(clock(), source(), sample);
        let header = record.header();
        assert_eq!(header.event(), EventId::new(9));
        assert_eq!(header.version(), TIMING_SCHEMA_VERSION);
        assert_eq!(header.source(), source());
        assert_eq!(header.generation(), 4);
        assert_eq!(
            header.confidentiality(),
            crate::source::Confidentiality::Internal
        );
        assert_eq!(header.impact(), crate::source::ReportingImpact::Normal);
        let clock_ctx = header.clock().unwrap();
        assert_eq!(clock_ctx.domain(), ClockDomainId::MONOTONIC);
        assert_eq!(clock_ctx.position(), 1_000);
        assert_eq!(clock_ctx.time_base(), TimeBase::new(1, 48_000).unwrap());
        assert_eq!(clock_ctx.provenance(), TimestampProvenance::Carried);
        assert_eq!(clock_ctx.epoch(), ClockEpochId::new(3));
        assert_eq!(record.end_tick(), 1_240);
        assert_eq!(record.budget_ticks(), 480);
        assert_eq!(record.start_tick(), 1_000);
        assert_eq!(record.duration_ticks(), Some(240));
    }

    #[test]
    fn duration_is_none_on_clock_anomaly() {
        let sample = TimingSample {
            point: EventId::new(0),
            generation: 0,
            budget_ticks: 0,
            start_tick: 500,
            end_tick: 400,
        };
        let record = TimingRecord::new(clock(), source(), sample);
        assert_eq!(record.duration_ticks(), None);
    }

    #[test]
    fn record_is_small_fixed_size_copy() {
        let record = TimingRecord::new(
            clock(),
            source(),
            TimingSample {
                point: EventId::new(1),
                generation: 0,
                budget_ticks: 0,
                start_tick: 0,
                end_tick: 1,
            },
        );
        assert!(size_of::<TimingRecord>() <= 128);
        let copied = record;
        assert_eq!(copied.header(), record.header());
        assert_eq!(copied.end_tick(), record.end_tick());
    }

    #[test]
    fn rt_record_ops_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let clock = clock();
        let source = source();
        let sample = TimingSample {
            point: EventId::new(2),
            generation: 1,
            budget_ticks: 100,
            start_tick: 10,
            end_tick: 60,
        };
        assert_no_alloc(|| {
            let record = TimingRecord::new(clock, source, sample);
            let _ = record.header();
            let _ = record.end_tick();
            let _ = record.budget_ticks();
            let _ = record.start_tick();
            let _ = record.duration_ticks();
            let _ = clock.domain();
            let _ = clock.time_base();
            let _ = clock.provenance();
            let _ = clock.epoch();
        });
    }
}
