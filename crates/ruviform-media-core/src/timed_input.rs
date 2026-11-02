//! Timed input vocabulary: clock-domain identity, native input stamps
//! ([`InputStamp`]), stamped payload envelopes ([`TimedInputEvent`]),
//! the [`ClockAnchor`] stamp↔frame calibration snapshot and
//! [`map_stamp_to_frame`] — the single authority converting a native
//! input timestamp into an [`EngineFrame`].
//!
//! Design intent ([`docs/designs/issue-188-timed-midi-detailed.md`]
//! §2): the native stamp is *preserved, never quantized*. A stamp
//! carries its own `ticks` + [`TimeBase`] + [`ClockDomainId`] +
//! [`TimestampProvenance`]; conversion to the engine frame happens
//! exactly once, here, via anchor delta → frame-base conversion with a
//! single [`Rounding::NearestEven`] step — the same "one authority, one
//! rounding" policy as `media_time_ticks`/`event_engine_frame`. No
//! PPQN960 or other consumer grid is ever forced on the source.
//!
//! **RT contract:** `map_stamp_to_frame` and all accessors are pure
//! integer math — no allocation, no blocking, no I/O. Clock-anchor
//! *calibration* (deciding which frame a stamp corresponds to) is a
//! non-RT responsibility of the caller; this module only maps.

use crate::observe::TimestampProvenance;
use crate::time::{MediaDuration, Rounding, TimeBase};
use crate::transport_schedule::EngineFrame;

/// An opaque clock-domain identity for input timestamps.
///
/// Stamps from different domains are different clocks — they must never
/// be subtracted, averaged or compared against each other (the same
/// rule [`EngineFrame`] applies to epochs). A domain may be a device's
/// sample clock, a backend monotonic clock, or `UNKNOWN` when the
/// source cannot identify its clock.
///
/// Well-known ids are provided as associated constants; backends may
/// allocate their own ids outside this range.
///
/// **RT contract:** RT-suitable — `const`, pure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClockDomainId(u64);

impl ClockDomainId {
    /// The identity-less domain for sources that cannot identify their
    /// clock. Stamps in this domain can still be delivered but carry no
    /// synchronisation guarantee.
    pub const UNKNOWN: Self = Self(0);
    /// The engine audio/sample clock — the domain [`EngineFrame`]s
    /// live in.
    pub const SAMPLE_CLOCK: Self = Self(1);
    /// A backend host-monotonic clock (e.g. `mach_absolute_time`,
    /// `QueryPerformanceCounter`, `CLOCK_MONOTONIC`).
    pub const MONOTONIC: Self = Self(2);

    /// Creates a domain id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A native timestamp carried by an input source.
///
/// `ticks` + `base` express the instant in the source's own clock —
/// the raw precision the source produced, before any conversion.
/// `domain` names which clock produced it; `provenance` says how much
/// of it is a carried fact versus an estimate (reusing the observation
/// vocabulary: `Carried` > `Derived` > `Estimated`).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputStamp {
    /// Tick count in `base` units on `domain`'s clock.
    pub ticks: u64,
    /// The timebase `ticks` is denominated in.
    pub base: TimeBase,
    /// The clock domain this stamp belongs to.
    pub domain: ClockDomainId,
    /// How much of this stamp is a carried fact versus an estimate.
    pub provenance: TimestampProvenance,
}

/// A stamped payload — the envelope consumers receive.
///
/// The generic `P` is typically a [`crate::MidiEvent`] today, but the
/// envelope is payload-agnostic so future input kinds (audio-rate
/// control, remote sync) reuse the same stamping contract.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimedInputEvent<P> {
    /// The native timestamp the source produced.
    pub stamp: InputStamp,
    /// The input payload.
    pub event: P,
}

/// A calibrated stamp↔frame pair for one [`ClockDomainId`].
///
/// An anchor asserts "stamp `stamp_ticks` (in `stamp_base`) on `domain`
/// corresponds to engine frame `frame`". Mapping any other stamp of the
/// same domain is a delta from this pair — the only authority for the
/// conversion. Anchors are *calibrated* outside the RT path (e.g. at
/// stream-start by sampling both clocks) and then published as a
/// snapshot; [`map_stamp_to_frame`] consumes them read-only.
///
/// `calibration` records how the pair was established (`Carried` = both
/// clocks read in one synchronous sample, `Derived` = computed from
/// declared rates, `Estimated` = extrapolated); the mapped result's
/// provenance is the weaker of the stamp's and this value.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClockAnchor {
    /// The domain this anchor calibrates.
    pub domain: ClockDomainId,
    /// The anchor stamp's tick count (in `stamp_base`).
    pub stamp_ticks: u64,
    /// The anchor stamp's timebase (must match input stamps' `base`
    /// for the delta to be meaningful — mismatch is [`MapRejection`]).
    pub stamp_base: TimeBase,
    /// The engine frame the anchor stamp corresponds to.
    pub frame: EngineFrame,
    /// How the anchor was established.
    pub calibration: TimestampProvenance,
}

/// The result of a successful [`map_stamp_to_frame`].
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MappedStamp {
    /// The mapped engine frame (anchor's epoch).
    pub frame: EngineFrame,
    /// The effective provenance — the weaker of the stamp's and the
    /// anchor's `calibration`.
    pub provenance: TimestampProvenance,
}

/// The reason [`map_stamp_to_frame`] refused a stamp. Mapping failures
/// are defined rejections — the caller must not silently drop or
/// mis-place the event (report + handle explicitly).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapRejection {
    /// The stamp's domain has no matching anchor — including `UNKNOWN`
    /// domain stamps with no anchor installed for `UNKNOWN`.
    UnmappedDomain,
    /// Stamp `base` ≠ anchor `stamp_base`: the source changed its
    /// timebase mid-stream (a new anchor is required).
    BaseMismatch,
    /// The anchor's epoch ≠ `now`'s epoch: a device restart invalidated
    /// the anchor. Frames of different epochs are never subtracted.
    EpochMismatch,
    /// The stamp is earlier than the anchor (negative delta). The
    /// caller decides whether to drop or clamp — never silently
    /// delivers it at a wrong position.
    BeforeAnchor,
    /// Tick delta or frame conversion overflowed.
    Overflow,
}

/// Maps a native input stamp to an [`EngineFrame`] via `anchor` — the
/// single stamp→frame authority.
///
/// `frame_base` is the engine frame timebase (e.g.
/// `TimeBase::sample_rate(48_000)` — one tick = one sample frame);
/// `now` supplies the current epoch so a stale anchor (installed before
/// a device restart) is detected as [`MapRejection::EpochMismatch`]
/// rather than producing a frame in a dead epoch.
///
/// The result's [`MappedStamp::provenance`] is the weaker of
/// `stamp.provenance` and `anchor.calibration` — an estimated anchor
/// can never produce a carried-fact frame.
///
/// **RT contract:** RT-suitable — pure integer math, single rounding
/// step ([`Rounding::NearestEven`]), no allocation.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{
///     ClockAnchor, ClockDomainId, EngineFrame, ClockEpochId, InputStamp,
///     TimeBase, TimestampProvenance, map_stamp_to_frame,
/// };
///
/// let ns = TimeBase::new(1, 1_000_000_000).unwrap();
/// let audio = TimeBase::sample_rate(48_000).unwrap();
/// let epoch = ClockEpochId::new(1);
/// let anchor = ClockAnchor {
///     domain: ClockDomainId::MONOTONIC,
///     stamp_ticks: 0,
///     stamp_base: ns,
///     frame: EngineFrame::new(epoch, 0),
///     calibration: TimestampProvenance::Carried,
/// };
/// let stamp = InputStamp {
///     ticks: 1_000_000, // 1 ms after the anchor
///     base: ns,
///     domain: ClockDomainId::MONOTONIC,
///     provenance: TimestampProvenance::Carried,
/// };
/// let mapped = map_stamp_to_frame(
///     &stamp,
///     &anchor,
///     audio,
///     EngineFrame::new(epoch, 96),
/// )
/// .unwrap();
/// assert_eq!(mapped.frame.frame(), 48); // 1 ms at 48 kHz
/// ```
pub fn map_stamp_to_frame(
    stamp: &InputStamp,
    anchor: &ClockAnchor,
    frame_base: TimeBase,
    now: EngineFrame,
) -> Result<MappedStamp, MapRejection> {
    if stamp.domain != anchor.domain {
        return Err(MapRejection::UnmappedDomain);
    }
    if stamp.base != anchor.stamp_base {
        return Err(MapRejection::BaseMismatch);
    }
    if !anchor.frame.same_epoch(now) {
        return Err(MapRejection::EpochMismatch);
    }
    let delta = stamp
        .ticks
        .checked_sub(anchor.stamp_ticks)
        .ok_or(MapRejection::BeforeAnchor)?;
    let delta_i64 = i64::try_from(delta).map_err(|_| MapRejection::Overflow)?;
    let frames = MediaDuration::new(delta_i64, stamp.base)
        .convert(frame_base, Rounding::NearestEven)
        .map_err(|_| MapRejection::Overflow)?
        .value();
    let frames = u64::try_from(frames).map_err(|_| MapRejection::Overflow)?;
    Ok(MappedStamp {
        frame: anchor.frame.advance(frames),
        provenance: weaker_provenance(stamp.provenance, anchor.calibration),
    })
}

/// The weaker of two provenances — the ordering is
/// `Carried` > `Derived` > `Estimated`; any non-`Carried` input drags
/// the result down.
///
/// **RT contract:** RT-suitable — pure.
#[must_use]
pub fn weaker_provenance(a: TimestampProvenance, b: TimestampProvenance) -> TimestampProvenance {
    use TimestampProvenance as P;
    match (a, b) {
        (P::Carried, P::Carried) => P::Carried,
        (P::Estimated, _) | (_, P::Estimated) => P::Estimated,
        _ => P::Derived,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport_schedule::ClockEpochId;

    fn ns() -> TimeBase {
        TimeBase::new(1, 1_000_000_000).unwrap()
    }

    fn audio() -> TimeBase {
        TimeBase::sample_rate(48_000).unwrap()
    }

    fn anchor() -> ClockAnchor {
        ClockAnchor {
            domain: ClockDomainId::MONOTONIC,
            stamp_ticks: 0,
            stamp_base: ns(),
            frame: EngineFrame::new(ClockEpochId::new(1), 0),
            calibration: TimestampProvenance::Carried,
        }
    }

    fn stamp(ticks: u64) -> InputStamp {
        InputStamp {
            ticks,
            base: ns(),
            domain: ClockDomainId::MONOTONIC,
            provenance: TimestampProvenance::Carried,
        }
    }

    #[test]
    fn maps_ns_delta_to_sample_frames() {
        let now = EngineFrame::new(ClockEpochId::new(1), 10_000);
        let m = map_stamp_to_frame(&stamp(1_000_000), &anchor(), audio(), now).unwrap();
        assert_eq!(m.frame.frame(), 48);
        assert_eq!(m.provenance, TimestampProvenance::Carried);
    }

    #[test]
    fn anchor_offset_adds_to_anchor_frame() {
        let mut a = anchor();
        a.stamp_ticks = 1_000_000;
        a.frame = EngineFrame::new(ClockEpochId::new(1), 100);
        let now = EngineFrame::new(ClockEpochId::new(1), 200);
        let m = map_stamp_to_frame(&stamp(2_000_000), &a, audio(), now).unwrap();
        assert_eq!(m.frame.frame(), 148);
    }

    #[test]
    fn rejects_defined_failures() {
        let now = EngineFrame::new(ClockEpochId::new(1), 100);
        // Wrong domain.
        let mut s = stamp(1);
        s.domain = ClockDomainId::UNKNOWN;
        assert_eq!(
            map_stamp_to_frame(&s, &anchor(), audio(), now),
            Err(MapRejection::UnmappedDomain)
        );
        // Wrong base.
        let mut s2 = stamp(1);
        s2.base = audio();
        assert_eq!(
            map_stamp_to_frame(&s2, &anchor(), audio(), now),
            Err(MapRejection::BaseMismatch)
        );
        // Before anchor.
        let mut a = anchor();
        a.stamp_ticks = 10;
        assert_eq!(
            map_stamp_to_frame(&stamp(5), &a, audio(), now),
            Err(MapRejection::BeforeAnchor)
        );
        // Stale epoch: anchor was calibrated in epoch 1, now is epoch 2.
        let now2 = EngineFrame::new(ClockEpochId::new(2), 0);
        assert_eq!(
            map_stamp_to_frame(&stamp(1), &a, audio(), now2),
            Err(MapRejection::EpochMismatch)
        );
    }

    #[test]
    fn provenance_takes_the_weaker_side() {
        let mut a = anchor();
        a.calibration = TimestampProvenance::Estimated;
        let now = EngineFrame::new(ClockEpochId::new(1), 100);
        let m = map_stamp_to_frame(&stamp(1_000), &a, audio(), now).unwrap();
        assert_eq!(m.provenance, TimestampProvenance::Estimated);
    }

    /// RT contract: the mapper is pure integer math — no allocation.
    #[test]
    fn rt_map_stamp_no_alloc() {
        let a = anchor();
        let s = stamp(1_000_000);
        let now = EngineFrame::new(ClockEpochId::new(1), 96);
        let _ = assert_no_alloc::assert_no_alloc(|| {
            core::hint::black_box(map_stamp_to_frame(&s, &a, audio(), now)).unwrap()
        });
    }
}
