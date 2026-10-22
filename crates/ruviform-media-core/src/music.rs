//! Musical time: [`Ppqn`], [`Tick`], [`TickDuration`], [`TempoEvent`],
//! [`TimeSignatureEvent`], [`MusicalPosition`], [`MusicalTimeInfo`],
//! [`TempoMapBuilder`] and [`TempoMap`].
//!
//! Musical time is a coordinate system separate from media time
//! ([`MediaTime`]); the two convert only through a compiled
//! [`TempoMap`]. The core representation is integer ticks at
//! [`PPQN`] = 960 ticks per quarter note — the same exact-rational
//! policy as [`crate::time`], with `i128`/`u128` intermediates and no
//! `f64` anywhere on the conversion path.
//!
//! A [`TempoMap`] is compiled by [`TempoMapBuilder`] on a non-RT thread
//! and is immutable afterwards. All query APIs are pure, allocation-free
//! binary searches (`O(log n)`, bounded cost), so they are RT-suitable.
//! RT code is expected to receive the map through the engine's snapshot
//! exchange (`arc-swap`/`RtPlan`-style), never by mutating a builder on
//! the audio thread.
//!
//! Contract source: `docs/designs/music-time-model.md` (Issue #33 design)
//! and `docs/adr/musical-time-model.md` (accepted).

use crate::time::round_quotient;
use crate::{Error, MediaTime, Result, Rounding, TimeBase};
use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Sub};

/// Ticks-per-quarter-note resolution: the single source of truth for
/// musical-time resolution. Only the constant [`PPQN`] (960) can be
/// constructed, ruling out variable resolutions at the type level.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ppqn(u16);

/// The fixed [`Ppqn`] resolution: 960 ticks per quarter note.
pub const PPQN: Ppqn = Ppqn(960);

impl Ppqn {
    /// Ticks per quarter note (always 960).
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn ticks_per_quarter(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Ppqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A musical-time position in [`PPQN`] ticks. Negative values are legal
/// and denote pre-roll before the tempo map's first event.
///
/// **RT contract:** RT-suitable — `Copy` value type; comparisons are
/// exact integer math.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(i64);

/// A span between two [`Tick`] positions — the tick analogue of
/// `MediaDuration`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickDuration(i64);

impl Tick {
    /// The zero position.
    pub const ZERO: Self = Self(0);

    /// Creates a tick position from a raw `i64` count.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// The raw tick count.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

impl TickDuration {
    /// The zero duration.
    pub const ZERO: Self = Self(0);

    /// Creates a tick duration from a raw `i64` count.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// The raw tick count.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

impl Add<TickDuration> for Tick {
    type Output = Self;
    fn add(self, rhs: TickDuration) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub<TickDuration> for Tick {
    type Output = Self;
    fn sub(self, rhs: TickDuration) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Sub<Tick> for Tick {
    type Output = TickDuration;
    fn sub(self, rhs: Self) -> TickDuration {
        TickDuration(self.0 - rhs.0)
    }
}

impl Add<TickDuration> for TickDuration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub<TickDuration> for TickDuration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}t", self.0)
    }
}

/// A tempo change point: from `tick` onward the tempo is `bpm_milli`.
///
/// `bpm_milli` is a milli-BPM fixed-point value (120.000 BPM is
/// `120_000`); the valid range `1..=999_999_999` is enforced at
/// [`TempoMapBuilder::build`], not on construction.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TempoEvent {
    /// Tick at which the tempo takes effect.
    pub tick: Tick,
    /// Tempo in milli-BPM fixed point (120.000 BPM → `120_000`).
    pub bpm_milli: u32,
}

/// A time-signature change point: from `tick` onward the signature is
/// `numerator`/`denominator`. `numerator` must be ≥ 1 and `denominator`
/// a power of two — both enforced at [`TempoMapBuilder::build`], not on
/// construction.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeSignatureEvent {
    /// Tick at which the signature takes effect.
    pub tick: Tick,
    /// Beats per bar (≥ 1, checked at build).
    pub numerator: u8,
    /// Beat unit as a power-of-two denominator (checked at build).
    pub denominator: u8,
}

/// A tick decomposed into bar/beat/offset for the display layer, under
/// an applicable time-signature map. `bar` and `beat` are 0-based;
/// `tick_offset` counts within the beat.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MusicalPosition {
    /// 0-based bar index, counted from the first applicable signature
    /// span (a signature change always starts a new bar).
    pub bar: u32,
    /// 0-based beat index within the bar.
    pub beat: u8,
    /// Ticks elapsed within the current beat (`< 3840/denominator`).
    pub tick_offset: u16,
}

/// A `Copy` snapshot of musical transport state for plugin ABI
/// boundaries (24 bytes). When no time signature applies,
/// `sig_num`/`sig_den` are `0` and `bar_start_tick` is `0` — the "unset"
/// representation; no implicit 4/4 is assumed (ADR decision 4).
///
/// **RT contract:** RT-suitable — `Copy` value type (24 bytes).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MusicalTimeInfo {
    /// Position in [`PPQN`] ticks.
    pub position_ticks: i64,
    /// Tick at which the bar containing `position_ticks` starts, or `0`
    /// when no signature applies.
    pub bar_start_tick: i64,
    /// Current tempo in milli-BPM fixed point.
    pub bpm_milli: u32,
    /// Resolution — always 960 ([`PPQN`]).
    pub ppqn: u16,
    /// Current signature numerator, or `0` when unset.
    pub sig_num: u8,
    /// Current signature denominator, or `0` when unset.
    pub sig_den: u8,
}

/// One compiled tempo segment: `bpm_milli` holds from `start_tick` until
/// the next segment. `start_num`/`start_den` is the accumulated rational
/// seconds at `start_tick` (the first segment anchors its tick to 0 s).
#[derive(Copy, Clone, Debug)]
struct TempoSegment {
    start_tick: i64,
    bpm_milli: u32,
    start_num: i128,
    start_den: u128,
}

/// One compiled signature span with its precomputed bar geometry.
#[derive(Copy, Clone, Debug)]
struct SignatureSpan {
    start_tick: i64,
    numerator: u8,
    denominator: u8,
    /// Ticks in one bar: `numerator * 4 * PPQN / denominator`.
    bar_len_ticks: u64,
    /// 0-based bar index of the bar that starts at `start_tick`.
    first_bar: u64,
}

/// Non-RT builder for a [`TempoMap`]: collects events, then `build()`
/// sorts, validates and precomputes the immutable segment tables.
///
/// **RT contract:** non-RT only — allocation and validation happen here
/// so the compiled [`TempoMap`] can stay allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{TempoEvent, TempoMapBuilder, Tick};
///
/// let mut builder = TempoMapBuilder::new();
/// builder
///     .add_tempo_event(TempoEvent { tick: Tick::new(0), bpm_milli: 120_000 })
///     .add_tempo_event(TempoEvent { tick: Tick::new(7_680), bpm_milli: 60_000 });
/// let map = builder.build().unwrap();
/// ```
#[derive(Debug, Default)]
pub struct TempoMapBuilder {
    tempo: Vec<TempoEvent>,
    signature: Vec<TimeSignatureEvent>,
}

impl TempoMapBuilder {
    /// An empty builder.
    ///
    /// **RT contract:** non-RT only (allocation).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a tempo event. Order is irrelevant — `build()` sorts and
    /// rejects duplicate ticks.
    ///
    /// **RT contract:** non-RT only (allocation).
    pub fn add_tempo_event(&mut self, event: TempoEvent) -> &mut Self {
        self.tempo.push(event);
        self
    }

    /// Appends a time-signature event. Order is irrelevant — `build()`
    /// sorts and rejects duplicate ticks.
    ///
    /// **RT contract:** non-RT only (allocation).
    pub fn add_time_signature_event(&mut self, event: TimeSignatureEvent) -> &mut Self {
        self.signature.push(event);
        self
    }

    /// Compiles the map: sorts events, validates ranges (at least one
    /// tempo event; `bpm_milli` in `1..=999_999_999`; signature
    /// `numerator` ≥ 1 and `denominator` a power of two; no duplicate
    /// ticks in either list) and precomputes accumulated rational
    /// seconds and bar indices.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on validation failure,
    /// [`Error::TimeOverflow`] if the accumulated rational arithmetic
    /// exceeds `i128`/`u128` intermediates.
    ///
    /// **RT contract:** non-RT only (allocation).
    pub fn build(self) -> Result<TempoMap> {
        let Self {
            mut tempo,
            mut signature,
        } = self;
        if tempo.is_empty() {
            return Err(Error::InvalidValue("tempo_events"));
        }
        if tempo
            .iter()
            .any(|e| !(1..=999_999_999).contains(&e.bpm_milli))
        {
            return Err(Error::InvalidValue("bpm_milli"));
        }
        for e in &signature {
            if e.numerator == 0 {
                return Err(Error::InvalidValue("time_signature_numerator"));
            }
            if !e.denominator.is_power_of_two() {
                return Err(Error::InvalidValue("time_signature_denominator"));
            }
        }
        tempo.sort_by_key(|e| e.tick);
        if tempo.windows(2).any(|w| w[0].tick == w[1].tick) {
            return Err(Error::InvalidValue("tempo_event_tick"));
        }
        signature.sort_by_key(|e| e.tick);
        if signature.windows(2).any(|w| w[0].tick == w[1].tick) {
            return Err(Error::InvalidValue("time_signature_tick"));
        }

        // Accumulated seconds per segment — exact rational forward
        // accumulation; the first segment anchors its tick to 0 s and
        // segment boundaries are stable anchors in both directions.
        let mut segments = Vec::with_capacity(tempo.len());
        let (mut acc_num, mut acc_den): (i128, u128) = (0, 1);
        let mut prev_tick = tempo[0].tick.0;
        let mut prev_bpm = tempo[0].bpm_milli;
        for e in &tempo {
            if !segments.is_empty() {
                let dt = i128::from(e.tick.0) - i128::from(prev_tick);
                let dn = dt.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
                let dd = u128::from(prev_bpm)
                    .checked_mul(960)
                    .ok_or(Error::TimeOverflow)?;
                let (n, d) = ratio_add(acc_num, acc_den, dn, dd)?;
                acc_num = n;
                acc_den = d;
            }
            segments.push(TempoSegment {
                start_tick: e.tick.0,
                bpm_milli: e.bpm_milli,
                start_num: acc_num,
                start_den: acc_den,
            });
            prev_tick = e.tick.0;
            prev_bpm = e.bpm_milli;
        }

        // Signature spans: each signature change starts a new bar at its
        // own tick (a mid-bar change truncates the previous bar).
        let mut spans: Vec<SignatureSpan> = Vec::with_capacity(signature.len());
        for e in &signature {
            let bar_len_ticks = u64::from(e.numerator) * (3840 / u64::from(e.denominator));
            let first_bar = match spans.last() {
                None => 0,
                Some(prev) => {
                    let span_ticks = u64::try_from(e.tick.0 - prev.start_tick)
                        .map_err(|_| Error::InvalidValue("time_signature_tick"))?;
                    prev.first_bar + span_ticks.div_ceil(prev.bar_len_ticks)
                }
            };
            spans.push(SignatureSpan {
                start_tick: e.tick.0,
                numerator: e.numerator,
                denominator: e.denominator,
                bar_len_ticks,
                first_bar,
            });
        }

        Ok(TempoMap {
            tempo: segments.into_boxed_slice(),
            signature: spans.into_boxed_slice(),
        })
    }
}

/// An immutable, compiled tempo + time-signature map. Shared between
/// threads by `Arc`; produced by [`TempoMapBuilder::build`].
///
/// All queries are pure, allocation-free binary searches — `O(log n)`
/// bounded cost. The rounding policy matches the media-time model: at
/// most one rounding per call, applied at the final unit conversion;
/// tempo change points are stable anchors that are exactly
/// representable in both ticks and seconds, so round-trips through them
/// are lossless.
///
/// **RT contract:** every query API is RT-suitable (allocation-free,
/// bounded); the map itself is immutable once built.
#[derive(Debug)]
pub struct TempoMap {
    tempo: Box<[TempoSegment]>,
    signature: Box<[SignatureSpan]>,
}

/// Reduces `num/den` (`den > 0`) to lowest terms; `0/x` becomes `0/1`.
fn norm_ratio(num: i128, den: u128) -> (i128, u128) {
    debug_assert!(den > 0);
    if num == 0 {
        return (0, 1);
    }
    let (mut a, mut b) = (num.unsigned_abs(), den);
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    // `a == gcd(|num|, den) <= |num| <= i128::MAX`.
    let g = i128::try_from(a).unwrap_or(i128::MAX);
    (num / g, den / a)
}

/// Exact `a + b` on normalized rationals; [`Error::TimeOverflow`] when an
/// `i128`/`u128` intermediate overflows.
fn ratio_add(an: i128, ad: u128, bn: i128, bd: u128) -> Result<(i128, u128)> {
    let cross = |x: i128, y: u128| -> Result<i128> {
        let y = i128::try_from(y).map_err(|_| Error::TimeOverflow)?;
        x.checked_mul(y).ok_or(Error::TimeOverflow)
    };
    let n = cross(an, bd)?
        .checked_add(cross(bn, ad)?)
        .ok_or(Error::TimeOverflow)?;
    let d = ad.checked_mul(bd).ok_or(Error::TimeOverflow)?;
    Ok(norm_ratio(n, d))
}

/// Compares `an/ad` against `bn/bd` exactly; [`Error::TimeOverflow`]
/// when a cross-multiply intermediate overflows.
fn ratio_cmp(an: i128, ad: u128, bn: i128, bd: u128) -> Result<Ordering> {
    let lhs = an
        .checked_mul(i128::try_from(bd).map_err(|_| Error::TimeOverflow)?)
        .ok_or(Error::TimeOverflow)?;
    let rhs = bn
        .checked_mul(i128::try_from(ad).map_err(|_| Error::TimeOverflow)?)
        .ok_or(Error::TimeOverflow)?;
    Ok(lhs.cmp(&rhs))
}

impl TempoMap {
    /// Index of the tempo segment governing `tick` — the last segment
    /// with `start_tick <= tick`, or `0` when `tick` precedes the first
    /// event (the first segment extrapolates to -∞ for pre-roll).
    fn tempo_index(&self, tick: i64) -> usize {
        self.tempo
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1)
    }

    /// Converts a [`Tick`] into a [`MediaTime`] in `base`: tick → exact
    /// rational seconds → a single [`Rounding::NearestEven`] rounding
    /// into the target base.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate or the result
    /// exceeds the representable range.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search +
    /// `i128`/`u128` rational math.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{TempoEvent, TempoMapBuilder, Tick, TimeBase};
    ///
    /// let mut b = TempoMapBuilder::new();
    /// b.add_tempo_event(TempoEvent { tick: Tick::new(0), bpm_milli: 120_000 });
    /// let map = b.build().unwrap();
    /// // One beat (960 ticks) at 120.000 BPM is exactly 0.5 s.
    /// let t = map.tick_to_media(Tick::new(960), TimeBase::new(1, 1_000).unwrap()).unwrap();
    /// assert_eq!(t.value(), 500);
    /// ```
    pub fn tick_to_media(&self, tick: Tick, base: TimeBase) -> Result<MediaTime> {
        let seg = &self.tempo[self.tempo_index(tick.0)];
        let dt = i128::from(tick.0) - i128::from(seg.start_tick);
        let dn = dt.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
        let dd = u128::from(seg.bpm_milli) * 960;
        let (sn, sd) = ratio_add(seg.start_num, seg.start_den, dn, dd)?;
        let num = sn
            .checked_mul(i128::from(base.denominator()))
            .ok_or(Error::TimeOverflow)?;
        let den = sd
            .checked_mul(u128::from(base.numerator()))
            .ok_or(Error::TimeOverflow)?;
        let value = round_quotient(num, den, Rounding::NearestEven);
        let value = i64::try_from(value).map_err(|_| Error::TimeOverflow)?;
        Ok(MediaTime::new(value, base))
    }

    /// Converts a [`MediaTime`] into a [`Tick`]: exact rational seconds →
    /// segment binary search → linear interpolation, with the inexact
    /// remainder truncated via [`Rounding::Floor`]. Round-trips through
    /// [`tick_to_media`](Self::tick_to_media) land within ≤ 1 tick.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate or the result
    /// exceeds the representable range.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search +
    /// `i128`/`u128` rational math.
    pub fn media_to_tick(&self, time: MediaTime) -> Result<Tick> {
        let (qn, qd) = norm_ratio(
            i128::from(time.value()) * i128::from(time.base().numerator()),
            u128::from(time.base().denominator()),
        );
        // Last segment with start_seconds <= q; when q precedes the
        // first segment, index 0 extrapolates (negative ticks).
        let mut lo = 0usize;
        let mut hi = self.tempo.len() - 1;
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            let s = &self.tempo[mid];
            if ratio_cmp(s.start_num, s.start_den, qn, qd)? == Ordering::Greater {
                hi = mid - 1;
            } else {
                lo = mid;
            }
        }
        let seg = &self.tempo[lo];
        let (dn, dd) = ratio_add(qn, qd, -seg.start_num, seg.start_den)?;
        let tn = dn
            .checked_mul(i128::from(seg.bpm_milli) * 960)
            .ok_or(Error::TimeOverflow)?;
        let td = dd.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
        let d = round_quotient(tn, td, Rounding::Floor);
        let t = i128::from(seg.start_tick)
            .checked_add(d)
            .ok_or(Error::TimeOverflow)?;
        Ok(Tick(i64::try_from(t).map_err(|_| Error::TimeOverflow)?))
    }

    /// Decomposes `tick` into bar/beat/offset under the signature map.
    /// `None` when no signature applies — the map has no signature
    /// events, or `tick` precedes the first one (no implicit 4/4).
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search.
    #[must_use]
    pub fn musical_position(&self, tick: Tick) -> Option<MusicalPosition> {
        let idx = self
            .signature
            .partition_point(|s| s.start_tick <= tick.0)
            .checked_sub(1)?;
        let span = &self.signature[idx];
        let d = u64::try_from(tick.0 - span.start_tick).ok()?;
        let bar = span.first_bar.checked_add(d / span.bar_len_ticks)?;
        let rem = d % span.bar_len_ticks;
        let ticks_per_beat = 3840 / u64::from(span.denominator);
        Some(MusicalPosition {
            bar: u32::try_from(bar).ok()?,
            beat: u8::try_from(rem / ticks_per_beat).ok()?,
            tick_offset: u16::try_from(rem % ticks_per_beat).ok()?,
        })
    }

    /// Builds the [`MusicalTimeInfo`] ABI snapshot at `tick`: current
    /// tempo, current signature (`0`/`0` when none applies) and the start
    /// tick of the containing bar (`0` when unset).
    ///
    /// **RT contract:** RT-suitable — allocation-free binary searches.
    #[must_use]
    pub fn time_info(&self, tick: Tick) -> MusicalTimeInfo {
        let seg = &self.tempo[self.tempo_index(tick.0)];
        let (sig_num, sig_den, bar_start_tick) = match self
            .signature
            .partition_point(|s| s.start_tick <= tick.0)
            .checked_sub(1)
        {
            None => (0, 0, 0),
            Some(idx) => {
                let span = &self.signature[idx];
                let d = i128::from(tick.0) - i128::from(span.start_tick);
                let bars = d / i128::from(span.bar_len_ticks);
                let start = i128::from(span.start_tick) + bars * i128::from(span.bar_len_ticks);
                (
                    span.numerator,
                    span.denominator,
                    i64::try_from(start).unwrap_or(0),
                )
            }
        };
        MusicalTimeInfo {
            position_ticks: tick.0,
            bar_start_tick,
            bpm_milli: seg.bpm_milli,
            ppqn: PPQN.0,
            sig_num,
            sig_den,
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::*;

    fn base(num: u32, den: u32) -> TimeBase {
        TimeBase::new(num, den).unwrap()
    }

    fn map_120() -> TempoMap {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        });
        b.build().unwrap()
    }

    fn map_4_4() -> TempoMap {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(0),
            numerator: 4,
            denominator: 4,
        });
        b.build().unwrap()
    }

    #[test]
    fn builder_sorts_and_validates() {
        // Unsorted input is sorted; single-tempo map converts.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(1_920),
            bpm_milli: 60_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        });
        let map = b.build().unwrap();
        // tick 1920 starts the 60 BPM segment: 2 beats at 120 BPM = 1.0 s.
        assert_eq!(
            map.tick_to_media(Tick::new(1_920), base(1, 1)).unwrap(),
            MediaTime::new(1, base(1, 1))
        );

        for bpm in [0u32, 1_000_000_000] {
            let mut b = TempoMapBuilder::new();
            b.add_tempo_event(TempoEvent {
                tick: Tick::new(0),
                bpm_milli: bpm,
            });
            assert_eq!(b.build().unwrap_err(), Error::InvalidValue("bpm_milli"));
        }

        let mut b = TempoMapBuilder::new();
        b.add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(0),
            numerator: 4,
            denominator: 3,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        });
        assert_eq!(
            b.build().unwrap_err(),
            Error::InvalidValue("time_signature_denominator")
        );

        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 60_000,
        });
        assert_eq!(
            b.build().unwrap_err(),
            Error::InvalidValue("tempo_event_tick")
        );

        let b = TempoMapBuilder::new();
        assert_eq!(b.build().unwrap_err(), Error::InvalidValue("tempo_events"));
    }

    #[test]
    fn tick_to_media_constant_tempo() {
        let map = map_120();
        // 120.000 BPM: one beat (960 ticks) = 0.5 s; 1920 = 1.0 s.
        assert_eq!(
            map.tick_to_media(Tick::new(960), base(1, 1_000)).unwrap(),
            MediaTime::new(500, base(1, 1_000))
        );
        assert_eq!(
            map.tick_to_media(Tick::new(1_920), base(1, 1)).unwrap(),
            MediaTime::new(1, base(1, 1))
        );
    }

    #[test]
    fn tick_to_media_anchor_exact() {
        // Tempo change at tick 960: 1 beat at 120 BPM (0.5 s), then 60.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(960),
            bpm_milli: 60_000,
        });
        let map = b.build().unwrap();
        // The change point maps to the accumulated seconds exactly.
        assert_eq!(
            map.tick_to_media(Tick::new(960), base(1, 1_000_000))
                .unwrap(),
            MediaTime::new(500_000, base(1, 1_000_000))
        );
        // One more beat at 60 BPM adds exactly 1.0 s.
        assert_eq!(
            map.tick_to_media(Tick::new(1_920), base(1, 1_000_000))
                .unwrap(),
            MediaTime::new(1_500_000, base(1, 1_000_000))
        );
        // Round-trip through the anchor is lossless.
        assert_eq!(
            map.media_to_tick(MediaTime::new(500_000, base(1, 1_000_000)))
                .unwrap(),
            Tick::new(960)
        );
    }

    #[test]
    fn media_to_tick_roundtrip() {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(4_800),
            bpm_milli: 90_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(9_600),
            bpm_milli: 150_500,
        });
        let map = b.build().unwrap();
        let base = base(1, 48_000);
        for &tick in &[0i64, 1, 480, 959, 960, 4_799, 4_800, 9_599, 9_600, 123_456] {
            let t = map.tick_to_media(Tick::new(tick), base).unwrap();
            let back = map.media_to_tick(t).unwrap();
            assert!(
                (back.value() - tick).abs() <= 1,
                "round-trip drift {back:?} for tick {tick}"
            );
        }
    }

    #[test]
    fn tick_to_media_milli_bpm() {
        // 120.500 BPM stays on the exact-rational path.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_500,
        });
        let map = b.build().unwrap();
        // 960 ticks = 1 beat = 60_000/120_500 s = 120/241 s — exact in
        // a 241-ticks-per-second base.
        let t = map.tick_to_media(Tick::new(960), base(1, 241)).unwrap();
        assert_eq!(t, MediaTime::new(120, base(1, 241)));
    }

    #[test]
    fn preroll_negative_tick() {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(960),
            bpm_milli: 120_000,
        });
        let map = b.build().unwrap();
        // One beat before the first event extrapolates to -0.5 s.
        assert_eq!(
            map.tick_to_media(Tick::new(0), base(1, 1_000)).unwrap(),
            MediaTime::new(-500, base(1, 1_000))
        );
        assert_eq!(
            map.media_to_tick(MediaTime::new(-500, base(1, 1_000)))
                .unwrap(),
            Tick::new(0)
        );
    }

    #[test]
    fn musical_position_decomposes() {
        let map = map_4_4();
        // 4/4 at 120 BPM: bar = 3840 ticks.
        assert_eq!(
            map.musical_position(Tick::new(0)),
            Some(MusicalPosition {
                bar: 0,
                beat: 0,
                tick_offset: 0
            })
        );
        assert_eq!(
            map.musical_position(Tick::new(3_840)),
            Some(MusicalPosition {
                bar: 1,
                beat: 0,
                tick_offset: 0
            })
        );
        assert_eq!(
            map.musical_position(Tick::new(3_840 + 1_920 + 30)),
            Some(MusicalPosition {
                bar: 1,
                beat: 2,
                tick_offset: 30
            })
        );

        // Signature change mid-bar truncates the previous bar and starts
        // a new one: 4/4 at 0, 3/4 at tick 4800 (= bar 1, beat 1).
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(0),
            numerator: 4,
            denominator: 4,
        })
        .add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(4_800),
            numerator: 3,
            denominator: 4,
        });
        let map = b.build().unwrap();
        assert_eq!(
            map.musical_position(Tick::new(4_800)),
            Some(MusicalPosition {
                bar: 2,
                beat: 0,
                tick_offset: 0
            })
        );
        // 3/4 bars are 2880 ticks.
        assert_eq!(
            map.musical_position(Tick::new(4_800 + 2_880)),
            Some(MusicalPosition {
                bar: 3,
                beat: 0,
                tick_offset: 0
            })
        );
    }

    #[test]
    fn musical_position_none_without_signature() {
        let map = map_120();
        assert_eq!(map.musical_position(Tick::new(0)), None);

        // Tick before the first signature event has no applicable sig.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(960),
            numerator: 4,
            denominator: 4,
        });
        let map = b.build().unwrap();
        assert_eq!(map.musical_position(Tick::new(0)), None);
        assert!(map.musical_position(Tick::new(960)).is_some());
    }

    #[test]
    fn time_info_snapshot() {
        let map = map_4_4();
        let info = map.time_info(Tick::new(5_760)); // bar 1, beat 2
        assert_eq!(info.position_ticks, 5_760);
        assert_eq!(info.bpm_milli, 120_000);
        assert_eq!((info.sig_num, info.sig_den), (4, 4));
        assert_eq!(info.bar_start_tick, 3_840);
        assert_eq!(info.ppqn, 960);

        // No signature events: sig fields are the unset representation.
        let info = map_120().time_info(Tick::new(1_920));
        assert_eq!((info.sig_num, info.sig_den), (0, 0));
        assert_eq!(info.bar_start_tick, 0);
        assert_eq!(info.bpm_milli, 120_000);
    }

    #[test]
    fn musical_time_info_is_copy_24b() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<MusicalTimeInfo>();
        assert_eq!(size_of::<MusicalTimeInfo>(), 24);
    }

    #[test]
    fn tick_arithmetic() {
        let a = Tick::new(1_000);
        let d = TickDuration::new(500);
        assert_eq!((a + d).value(), 1_500);
        assert_eq!((a - d).value(), 500);
        assert_eq!((a - Tick::new(250)).value(), 750);
        assert_eq!((d + d - TickDuration::new(100)).value(), 900);
    }

    /// RT contract: every `TempoMap` query is allocation-free inside the
    /// `assert_no_alloc` window (binary search + integer math only).
    #[test]
    fn rt_conversion_no_alloc() {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent {
            tick: Tick::new(0),
            bpm_milli: 120_000,
        })
        .add_tempo_event(TempoEvent {
            tick: Tick::new(4_800),
            bpm_milli: 90_000,
        })
        .add_time_signature_event(TimeSignatureEvent {
            tick: Tick::new(0),
            numerator: 4,
            denominator: 4,
        });
        let map = b.build().unwrap();
        let base = base(1, 48_000);

        let (t, back, pos, info) = assert_no_alloc(|| {
            let t = map.tick_to_media(Tick::new(9_600), base).unwrap();
            let back = map.media_to_tick(t).unwrap();
            let pos = map.musical_position(Tick::new(9_600));
            let info = map.time_info(Tick::new(9_600));
            (t, back, pos, info)
        });
        assert!((back.value() - 9_600).abs() <= 1);
        assert!(pos.is_some());
        assert_eq!(info.position_ticks, 9_600);
        let _ = t;
    }
}
