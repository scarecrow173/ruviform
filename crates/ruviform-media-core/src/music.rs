//! Musical time: [`BeatPosition`], [`Ppqn`], [`Tick`], [`TickDuration`],
//! [`TickPosition`], [`TempoEvent`], [`TimeSignatureEvent`],
//! [`MusicalPosition`], [`MusicalTimeInfo`], [`TempoMapBuilder`] and
//! [`TempoMap`].
//!
//! Musical time is a coordinate system separate from media time
//! ([`MediaTime`]); the two convert only through a compiled
//! [`TempoMap`]. The canonical position is [`BeatPosition`] — a
//! normalized rational number of quarter notes that does not depend on
//! any tick grid — while [`Tick`] counts are only meaningful together
//! with their resolution ([`TickPosition`]). Conversion math follows the
//! same exact-rational policy as [`crate::time`], with `i128`/`u128`
//! intermediates and no `f64` anywhere on the conversion path.
//!
//! A [`TempoMap`] is compiled by [`TempoMapBuilder`] on a non-RT thread
//! and is immutable afterwards. All query APIs are pure, allocation-free
//! binary searches (`O(log n)`, bounded cost), so they are RT-suitable.
//! RT code is expected to receive the map through the engine's snapshot
//! exchange (`arc-swap`/`RtPlan`-style), never by mutating a builder on
//! the audio thread.
//!
//! Contract source: `docs/adr/configurable-ppqn-resolution-independent-musical-time.md`
//! (accepted; supersedes the fixed-960 `docs/adr/musical-time-model.md`)
//! and `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`
//! (Issue #189 M3 design).

use crate::time::round_quotient;
use crate::{Error, MediaTime, Result, Rounding, TimeBase};
use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Sub};

/// Ticks-per-quarter-note resolution. Every value `1..=u16::MAX` is a
/// valid resolution — [`Ppqn::new`] validates — while [`PPQN`] (960)
/// remains the conventional default retained for the legacy fixed-960
/// contract.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ppqn(u16);

/// The conventional resolution: 960 ticks per quarter note.
///
/// Retained as the compatibility constant for pre-[`BeatPosition`]
/// data and plugin snapshots; new code may construct any validated
/// resolution via [`Ppqn::new`].
pub const PPQN: Ppqn = Ppqn(960);

impl Ppqn {
    /// Validated resolution: `1..=u16::MAX` ticks per quarter note.
    /// `0` is rejected — a resolution must count at least one tick per
    /// quarter note.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `value` is `0`.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    pub const fn new(value: u16) -> Result<Self> {
        if value == 0 {
            return Err(Error::InvalidValue("ppqn"));
        }
        Ok(Self(value))
    }

    /// Ticks per quarter note.
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

/// Canonical musical position: a normalized rational number of quarter
/// notes (`num/den` beats), independent of any tick grid.
///
/// Invariants enforced at construction: `den >= 1`,
/// `gcd(|num|, den) == 1`, and the sign is carried only by `num`.
/// Negative positions are legal and denote pre-roll before the tempo
/// map's first event. `den` is bounded by `u32` so `i64 × u32`
/// cross-products always fit `i128` — ordering is total and exact, and
/// arithmetic that cannot be represented returns
/// [`Error::TimeOverflow`] instead of silently quantizing to a tick
/// grid (positions such as `1/7` beat stay exact).
///
/// **RT contract:** RT-suitable — `Copy` value type; comparisons and
/// the `checked_*` arithmetic are exact rational math with bounded
/// `i128`/`u128` intermediates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BeatPosition {
    num: i64,
    den: u32,
}

/// `const` gcd on `u64` — Euclid's algorithm; `gcd(0, b) = b`.
const fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Normalizes a wide rational into a [`BeatPosition`]:
/// [`Error::TimeOverflow`] when the reduced numerator or denominator
/// exceeds `i64`/`u32` — the explicit rejection required when a result
/// cannot be represented without quantizing.
fn beat_from_wide(num: i128, den: u128) -> Result<BeatPosition> {
    if den == 0 {
        return Err(Error::TimeOverflow);
    }
    let (mut a, mut b) = (num.unsigned_abs(), den);
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    // `a = gcd(|num|, den) >= 1` since `den > 0`; `a <= |num|` always.
    let g = i128::try_from(a).map_err(|_| Error::TimeOverflow)?;
    Ok(BeatPosition {
        num: i64::try_from(num / g).map_err(|_| Error::TimeOverflow)?,
        den: u32::try_from(den / a).map_err(|_| Error::TimeOverflow)?,
    })
}

/// Exact `a - b` on beat positions as an unnormalized `(i128, u128)`
/// pair — infallible: `|num| < 2⁶³` and `den < 2³²` keep every cross
/// product inside `i128`.
fn beat_sub(a: BeatPosition, b: BeatPosition) -> (i128, u128) {
    (
        i128::from(a.num) * i128::from(b.den) - i128::from(b.num) * i128::from(a.den),
        u128::from(a.den) * u128::from(b.den),
    )
}

/// `ceil(a/b)` for `b > 0` — truncating `i128` division already rounds
/// toward zero, which is `ceil` for non-positive `a`.
fn div_ceil(a: i128, b: i128) -> i128 {
    debug_assert!(b > 0);
    let q = a / b;
    if a % b == 0 {
        q
    } else if a >= 0 {
        q + 1
    } else {
        q
    }
}

impl BeatPosition {
    /// The zero position.
    pub const ZERO: Self = Self { num: 0, den: 1 };

    /// Creates a normalized rational beat position `num/den`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `den` is `0`.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free gcd.
    // `as` casts: `const fn` cannot call `TryFrom::try_from`; the gcd is
    // `<= den <= u32::MAX` and never grows the operands, so the
    // narrowing cannot fail.
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    pub const fn new(num: i64, den: u32) -> Result<Self> {
        if den == 0 {
            return Err(Error::InvalidValue("beat_denominator"));
        }
        let g = gcd_u64(num.unsigned_abs(), den as u64);
        // `g >= 1` since `den != 0`; the reduced halves never grow.
        Ok(Self {
            num: num / g as i64,
            den: den / g as u32,
        })
    }

    /// A whole-beat position (`beats/1`).
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn from_int(beats: i64) -> Self {
        Self { num: beats, den: 1 }
    }

    /// Signed numerator — carries the position's sign.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn numerator(self) -> i64 {
        self.num
    }

    /// Denominator — always `>= 1`.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.den
    }

    /// Exact `self + rhs`.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the reduced result exceeds the
    /// `i64`/`u32` representation.
    ///
    /// **RT contract:** RT-suitable — `i128`/`u128` intermediates only.
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let n =
            i128::from(self.num) * i128::from(rhs.den) + i128::from(rhs.num) * i128::from(self.den);
        let d = u128::from(self.den) * u128::from(rhs.den);
        beat_from_wide(n, d)
    }

    /// Exact `self - rhs`; same overflow contract as
    /// [`checked_add`](Self::checked_add).
    ///
    /// **RT contract:** RT-suitable — `i128`/`u128` intermediates only.
    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        let n =
            i128::from(self.num) * i128::from(rhs.den) - i128::from(rhs.num) * i128::from(self.den);
        let d = u128::from(self.den) * u128::from(rhs.den);
        beat_from_wide(n, d)
    }

    /// Exact `-self`.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] on `num == i64::MIN`.
    ///
    /// **RT contract:** RT-suitable.
    pub const fn checked_neg(self) -> Result<Self> {
        match self.num.checked_neg() {
            Some(num) => Ok(Self { num, den: self.den }),
            None => Err(Error::TimeOverflow),
        }
    }

    /// Lossy `f64` projection for display/UI — explicitly approximate;
    /// never used on the canonical conversion path.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // documented lossy projection
    pub fn to_f64(self) -> f64 {
        self.num as f64 / f64::from(self.den)
    }
}

impl Ord for BeatPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        // `|num| < 2⁶³` and `den < 2³²` keep cross-products below 2⁹⁵ —
        // always inside `i128`, so this is a total, exact order.
        (i128::from(self.num) * i128::from(other.den))
            .cmp(&(i128::from(other.num) * i128::from(self.den)))
    }
}

impl PartialOrd for BeatPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for BeatPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// A raw tick count in some declared resolution — meaningful only
/// together with its [`Ppqn`]; [`TickPosition`] binds the two so a bare
/// `Tick` can never be compared or converted across resolutions
/// silently. Negative values are legal and denote pre-roll before the
/// tempo map's first event.
///
/// **RT contract:** RT-suitable — `Copy` value type; comparisons are
/// exact integer math *within one resolution*.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(i64);

/// A span between two [`Tick`] positions *in the same resolution* —
/// the tick analogue of `MediaDuration`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickDuration(i64);

/// A [`Tick`] bound to the [`Ppqn`] resolution it counts in.
///
/// Tick counts from different resolutions are *different* musical
/// positions, so [`TickPosition`] carries its context: equality
/// compares `(tick, ppqn)` pairs — `960t@960` never equals `480t@480`
/// even though both denote one beat — and ordering exists only within
/// one resolution via [`cmp_same_resolution`](Self::cmp_same_resolution).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TickPosition {
    tick: Tick,
    ppqn: Ppqn,
}

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

impl TickPosition {
    /// Binds `tick` to the `ppqn` resolution it counts in.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn new(tick: Tick, ppqn: Ppqn) -> Self {
        Self { tick, ppqn }
    }

    /// The raw tick count.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn tick(self) -> Tick {
        self.tick
    }

    /// The resolution [`tick`](Self::tick) counts in.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn ppqn(self) -> Ppqn {
        self.ppqn
    }

    /// The exact rational beat position — `tick / ppqn` quarter notes.
    /// Infallible: a [`Ppqn`] is always `>= 1`.
    ///
    /// # Panics
    ///
    /// Never — `ppqn >= 1` is a `Ppqn` invariant, so the
    /// [`BeatPosition::new`] call cannot return `Err`.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn to_beat(self) -> BeatPosition {
        match BeatPosition::new(self.tick.0, self.ppqn.0 as u32) {
            Ok(b) => b,
            // `ppqn >= 1` is a `Ppqn` invariant — `new` cannot fail here.
            Err(_) => panic!("unreachable: ppqn denominator is non-zero"),
        }
    }

    /// Projects a [`BeatPosition`] onto the `ppqn` tick grid with an
    /// explicit `rounding` policy — the caller owns the resulting
    /// quantization error (≤ 0.5 target ticks for `NearestEven`, ≤ 1
    /// target tick for directed roundings).
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the rounded tick exceeds `i64`.
    ///
    /// **RT contract:** RT-suitable — `i128` intermediates only.
    pub fn from_beat(beat: BeatPosition, ppqn: Ppqn, rounding: Rounding) -> Result<Self> {
        let num = i128::from(beat.num) * i128::from(ppqn.0);
        let ticks = round_quotient(num, u128::from(beat.den), rounding);
        let ticks = i64::try_from(ticks).map_err(|_| Error::TimeOverflow)?;
        Ok(Self {
            tick: Tick::new(ticks),
            ppqn,
        })
    }

    /// Re-expresses this position on the `target` grid: exact when
    /// `tick * target / self.ppqn` is integral, otherwise rounded per
    /// `rounding`.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the result exceeds `i64`.
    ///
    /// **RT contract:** RT-suitable.
    pub fn rescale(self, target: Ppqn, rounding: Rounding) -> Result<Self> {
        Self::from_beat(self.to_beat(), target, rounding)
    }

    /// Ordering is only meaningful inside one resolution: compares tick
    /// counts iff `self.ppqn == other.ppqn`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on a cross-resolution comparison — an
    /// implicit mixed-PPQN ordering is a bug and is never allowed
    /// silently.
    ///
    /// **RT contract:** RT-suitable.
    pub fn cmp_same_resolution(&self, other: &Self) -> Result<Ordering> {
        if self.ppqn != other.ppqn {
            return Err(Error::InvalidValue("tick_ppqn_mismatch"));
        }
        Ok(self.tick.cmp(&other.tick))
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

/// A tempo change point: from `position` onward the tempo is
/// `bpm_milli`.
///
/// `position` is the canonical [`BeatPosition`]; tick-based maps anchor
/// events via [`TempoEvent::at_tick`]. `bpm_milli` is a milli-BPM
/// fixed-point value (120.000 BPM is `120_000`); the valid range
/// `1..=999_999_999` is enforced at [`TempoMapBuilder::build`], not on
/// construction.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TempoEvent {
    /// Beat position at which the tempo takes effect.
    pub position: BeatPosition,
    /// Tempo in milli-BPM fixed point (120.000 BPM → `120_000`).
    pub bpm_milli: u32,
}

impl TempoEvent {
    /// Anchors the event at a [`BeatPosition`].
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn at(position: BeatPosition, bpm_milli: u32) -> Self {
        Self {
            position,
            bpm_milli,
        }
    }

    /// Anchors the event at a [`TickPosition`] — the explicit tick +
    /// resolution context required for tick-based tempo maps.
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn at_tick(position: TickPosition, bpm_milli: u32) -> Self {
        Self {
            position: position.to_beat(),
            bpm_milli,
        }
    }
}

/// A time-signature change point: from `position` onward the signature
/// is `numerator`/`denominator`. `numerator` must be ≥ 1 and
/// `denominator` a power of two — both enforced at
/// [`TempoMapBuilder::build`], not on construction.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeSignatureEvent {
    /// Beat position at which the signature takes effect.
    pub position: BeatPosition,
    /// Beats per bar (≥ 1, checked at build).
    pub numerator: u8,
    /// Beat unit as a power-of-two denominator (checked at build).
    pub denominator: u8,
}

impl TimeSignatureEvent {
    /// Anchors the event at a [`BeatPosition`].
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn at(position: BeatPosition, numerator: u8, denominator: u8) -> Self {
        Self {
            position,
            numerator,
            denominator,
        }
    }

    /// Anchors the event at a [`TickPosition`] — see
    /// [`TempoEvent::at_tick`].
    ///
    /// **RT contract:** RT-suitable — `const`, allocation-free.
    #[must_use]
    pub const fn at_tick(position: TickPosition, numerator: u8, denominator: u8) -> Self {
        Self {
            position: position.to_beat(),
            numerator,
            denominator,
        }
    }
}

/// A beat position decomposed into bar/beat/offset for the display
/// layer, under an applicable time-signature map. `bar` and `beat` are
/// 0-based; `beat_offset` is the exact beat remainder within the beat.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MusicalPosition {
    /// 0-based bar index, counted from the first applicable signature
    /// span (a signature change always starts a new bar).
    pub bar: u32,
    /// 0-based beat index within the bar.
    pub beat: u8,
    /// Exact beats elapsed within the current beat (`< 4/sig_den`
    /// quarter notes — `< 1` in `x/4`, `< 1/2` in `x/8`).
    pub beat_offset: BeatPosition,
}

/// A `Copy` snapshot of musical transport state for plugin ABI
/// boundaries (24 bytes). When no time signature applies,
/// `sig_num`/`sig_den` are `0` and `bar_start_tick` is `0` — the "unset"
/// representation; no implicit 4/4 is assumed (ADR decision 4).
///
/// `position_ticks` and `bar_start_tick` are projections of the exact
/// [`BeatPosition`] state onto the resolution reported by `ppqn`
/// ([`Rounding::NearestEven`]); `ppqn` always reports the actual
/// resolution of the tick fields, so wire/ABI consumers can rescale
/// without ambiguity.
///
/// **RT contract:** RT-suitable — `Copy` value type (24 bytes).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MusicalTimeInfo {
    /// Position in `ppqn` ticks — the nearest tick to the queried
    /// beat position.
    pub position_ticks: i64,
    /// Tick at which the bar containing `position_ticks` starts, or `0`
    /// when no signature applies.
    pub bar_start_tick: i64,
    /// Current tempo in milli-BPM fixed point.
    pub bpm_milli: u32,
    /// Resolution of the tick fields — the [`Ppqn`] supplied to
    /// [`TempoMap::time_info`].
    pub ppqn: u16,
    /// Current signature numerator, or `0` when unset.
    pub sig_num: u8,
    /// Current signature denominator, or `0` when unset.
    pub sig_den: u8,
}

/// One compiled tempo segment: `bpm_milli` holds from `start` until
/// the next segment. `start_num`/`start_den` is the accumulated rational
/// seconds at `start` (the first segment anchors its beat to 0 s).
#[derive(Copy, Clone, Debug)]
struct TempoSegment {
    start: BeatPosition,
    bpm_milli: u32,
    start_num: i128,
    start_den: u128,
}

/// One compiled signature span with its precomputed bar geometry.
#[derive(Copy, Clone, Debug)]
struct SignatureSpan {
    start: BeatPosition,
    numerator: u8,
    denominator: u8,
    /// Beats in one bar — `numerator * 4 / denominator`, normalized.
    bar_len_num: u32,
    bar_len_den: u32,
    /// 0-based bar index of the bar that starts at `start`.
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
/// use ruviform_media_core::{PPQN, TempoEvent, TempoMapBuilder, Tick, TickPosition};
///
/// let mut builder = TempoMapBuilder::new();
/// builder
///     .add_tempo_event(TempoEvent::at_tick(TickPosition::new(Tick::new(0), PPQN), 120_000))
///     .add_tempo_event(TempoEvent::at_tick(TickPosition::new(Tick::new(7_680), PPQN), 60_000));
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
    /// rejects duplicate positions (compared rationally: `1/2` beat and
    /// `480/960` beat are the same position).
    ///
    /// **RT contract:** non-RT only (allocation).
    pub fn add_tempo_event(&mut self, event: TempoEvent) -> &mut Self {
        self.tempo.push(event);
        self
    }

    /// Appends a time-signature event. Order is irrelevant — `build()`
    /// sorts and rejects duplicate positions.
    ///
    /// **RT contract:** non-RT only (allocation).
    pub fn add_time_signature_event(&mut self, event: TimeSignatureEvent) -> &mut Self {
        self.signature.push(event);
        self
    }

    /// Compiles the map: sorts events, validates ranges (at least one
    /// tempo event; `bpm_milli` in `1..=999_999_999`; signature
    /// `numerator` ≥ 1 and `denominator` a power of two; no duplicate
    /// positions in either list) and precomputes accumulated rational
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
        tempo.sort_by_key(|e| e.position);
        if tempo.windows(2).any(|w| w[0].position == w[1].position) {
            return Err(Error::InvalidValue("tempo_event_tick"));
        }
        signature.sort_by_key(|e| e.position);
        if signature.windows(2).any(|w| w[0].position == w[1].position) {
            return Err(Error::InvalidValue("time_signature_tick"));
        }

        // Accumulated seconds per segment — exact rational forward
        // accumulation; the first segment anchors its beat to 0 s and
        // segment boundaries are stable anchors in both directions.
        let mut segments = Vec::with_capacity(tempo.len());
        let (mut acc_num, mut acc_den): (i128, u128) = (0, 1);
        let mut prev_position = tempo[0].position;
        let mut prev_bpm = tempo[0].bpm_milli;
        for e in &tempo {
            if !segments.is_empty() {
                // Δbeats × 60_000 ms / bpm — exact rational seconds.
                let (dtn, dtd) = beat_sub(e.position, prev_position);
                let dn = dtn.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
                let dd = dtd
                    .checked_mul(u128::from(prev_bpm))
                    .ok_or(Error::TimeOverflow)?;
                let (n, d) = ratio_add(acc_num, acc_den, dn, dd)?;
                acc_num = n;
                acc_den = d;
            }
            segments.push(TempoSegment {
                start: e.position,
                bpm_milli: e.bpm_milli,
                start_num: acc_num,
                start_den: acc_den,
            });
            prev_position = e.position;
            prev_bpm = e.bpm_milli;
        }

        // Signature spans: each signature change starts a new bar at its
        // own position (a mid-bar change truncates the previous bar).
        let mut spans: Vec<SignatureSpan> = Vec::with_capacity(signature.len());
        for e in &signature {
            // Bar length in beats: `numerator * 4 / denominator`,
            // normalized (e.g. 6/8 → 3 beats, 4/4 → 4 beats).
            let g = gcd_u64(u64::from(e.numerator) * 4, u64::from(e.denominator));
            // `numerator * 4 <= 1020` and `denominator <= 255` — the
            // narrowings cannot fail.
            let bar_len_num =
                u32::try_from(u64::from(e.numerator) * 4 / g).map_err(|_| Error::TimeOverflow)?;
            let bar_len_den =
                u32::try_from(u64::from(e.denominator) / g).map_err(|_| Error::TimeOverflow)?;
            let first_bar = match spans.last() {
                None => 0,
                Some(prev) => {
                    // Beats into the previous span ÷ its bar length,
                    // rounded up — a partial bar still consumes a bar
                    // index.
                    let (dtn, dtd) = beat_sub(e.position, prev.start);
                    let bn = dtn
                        .checked_mul(i128::from(prev.bar_len_den))
                        .ok_or(Error::TimeOverflow)?;
                    let bd = dtd
                        .checked_mul(u128::from(prev.bar_len_num))
                        .ok_or(Error::TimeOverflow)?;
                    let bd = i128::try_from(bd).map_err(|_| Error::TimeOverflow)?;
                    let bars = u64::try_from(div_ceil(bn, bd)).map_err(|_| Error::TimeOverflow)?;
                    prev.first_bar
                        .checked_add(bars)
                        .ok_or(Error::TimeOverflow)?
                }
            };
            spans.push(SignatureSpan {
                start: e.position,
                numerator: e.numerator,
                denominator: e.denominator,
                bar_len_num,
                bar_len_den,
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
/// representable in both beats and seconds, so round-trips through them
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
    /// Index of the tempo segment governing `position` — the last
    /// segment with `start <= position`, or `0` when `position`
    /// precedes the first event (the first segment extrapolates to -∞
    /// for pre-roll).
    fn tempo_index(&self, position: BeatPosition) -> usize {
        self.tempo
            .partition_point(|s| s.start <= position)
            .saturating_sub(1)
    }

    /// Converts a [`BeatPosition`] into a [`MediaTime`] in `base`:
    /// beat → exact rational seconds → a single
    /// [`Rounding::NearestEven`] rounding into the target base (target
    /// error ≤ 0.5 base ticks).
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
    /// use ruviform_media_core::{BeatPosition, TempoEvent, TempoMapBuilder, TimeBase};
    ///
    /// let mut b = TempoMapBuilder::new();
    /// b.add_tempo_event(TempoEvent::at(BeatPosition::from_int(0), 120_000));
    /// let map = b.build().unwrap();
    /// // One beat at 120.000 BPM is exactly 0.5 s.
    /// let t = map
    ///     .beat_to_media(BeatPosition::from_int(1), TimeBase::new(1, 1_000).unwrap())
    ///     .unwrap();
    /// assert_eq!(t.value(), 500);
    /// ```
    pub fn beat_to_media(&self, position: BeatPosition, base: TimeBase) -> Result<MediaTime> {
        let seg = &self.tempo[self.tempo_index(position)];
        // Δbeats × 60_000 ms / bpm — exact rational seconds.
        let (dtn, dtd) = beat_sub(position, seg.start);
        let dn = dtn.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
        let dd = dtd
            .checked_mul(u128::from(seg.bpm_milli))
            .ok_or(Error::TimeOverflow)?;
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

    /// Converts a [`MediaTime`] into the exact governing
    /// [`BeatPosition`]: rational seconds → segment binary search →
    /// exact rational interpolation — no rounding on the result path
    /// (the returned position is the precise beat the media instant
    /// lands on).
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate overflows or the
    /// exact result's denominator exceeds the `u32` bound.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search +
    /// `i128`/`u128` rational math.
    pub fn media_to_beat(&self, time: MediaTime) -> Result<BeatPosition> {
        let (qn, qd) = norm_ratio(
            i128::from(time.value()) * i128::from(time.base().numerator()),
            u128::from(time.base().denominator()),
        );
        // Last segment with start_seconds <= q; when q precedes the
        // first segment, index 0 extrapolates (negative beats).
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
        // Δseconds × bpm / 60_000 — exact rational beats.
        let bn = dn
            .checked_mul(i128::from(seg.bpm_milli))
            .ok_or(Error::TimeOverflow)?;
        let bd = dd.checked_mul(60_000).ok_or(Error::TimeOverflow)?;
        let (n, d) = ratio_add(i128::from(seg.start.num), u128::from(seg.start.den), bn, bd)?;
        beat_from_wide(n, d)
    }

    /// Converts a [`TickPosition`] into a [`MediaTime`] in `base`:
    /// tick → exact [`BeatPosition`] → [`beat_to_media`](Self::beat_to_media)
    /// — a single [`Rounding::NearestEven`] rounding at the final base
    /// conversion, so a whole-beat position is lossless at any
    /// resolution.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate or the result
    /// exceeds the representable range.
    ///
    /// **RT contract:** RT-suitable.
    pub fn tick_to_media(&self, tick: TickPosition, base: TimeBase) -> Result<MediaTime> {
        self.beat_to_media(tick.to_beat(), base)
    }

    /// Converts a [`MediaTime`] into a [`TickPosition`] at resolution
    /// `ppqn`: exact [`media_to_beat`](Self::media_to_beat) → a single
    /// [`Rounding::Floor`] projection onto the tick grid — the legacy
    /// floor policy, kept so the result never lands *after* the media
    /// instant. Round-trips through
    /// [`tick_to_media`](Self::tick_to_media) land within ≤ 1 tick.
    /// For another policy use `media_to_beat` +
    /// [`TickPosition::from_beat`].
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate or the result
    /// exceeds the representable range.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search +
    /// `i128`/`u128` rational math.
    pub fn media_to_tick(&self, time: MediaTime, ppqn: Ppqn) -> Result<TickPosition> {
        TickPosition::from_beat(self.media_to_beat(time)?, ppqn, Rounding::Floor)
    }

    /// Decomposes `position` into bar/beat/offset under the signature
    /// map. `Ok(None)` when no signature applies — the map has no
    /// signature events, or `position` precedes the first one (no
    /// implicit 4/4).
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate overflows or the
    /// exact `beat_offset` exceeds the `u32` denominator bound.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary search +
    /// exact rational math.
    pub fn musical_position(&self, position: BeatPosition) -> Result<Option<MusicalPosition>> {
        let Some(idx) = self
            .signature
            .partition_point(|s| s.start <= position)
            .checked_sub(1)
        else {
            return Ok(None);
        };
        let span = &self.signature[idx];
        // Beats into the span (`>= 0` — position is at or past start).
        let (dn, dd) = beat_sub(position, span.start);
        // bars = floor(d / bar_len) = floor(dn * bar_len_den / (dd * bar_len_num)).
        let bn = dn
            .checked_mul(i128::from(span.bar_len_den))
            .ok_or(Error::TimeOverflow)?;
        let bd = dd
            .checked_mul(u128::from(span.bar_len_num))
            .ok_or(Error::TimeOverflow)?;
        let bd = i128::try_from(bd).map_err(|_| Error::TimeOverflow)?;
        let bars = bn / bd; // `bn >= 0` — truncating division is floor.
        let bar = span
            .first_bar
            .checked_add(u64::try_from(bars).map_err(|_| Error::TimeOverflow)?)
            .ok_or(Error::TimeOverflow)?;
        // Beats remaining within the bar: `d - bars * bar_len`.
        let bars_n = bars
            .checked_mul(i128::from(span.bar_len_num))
            .ok_or(Error::TimeOverflow)?;
        let (rn, rd) = ratio_add(dn, dd, -bars_n, u128::from(span.bar_len_den))?;
        // Beat index: floor(rem / beat_len); a `1/denominator` note is
        // `4/denominator` quarter notes.
        let in_n = rn
            .checked_mul(i128::from(span.denominator))
            .ok_or(Error::TimeOverflow)?;
        let in_d = rd.checked_mul(4).ok_or(Error::TimeOverflow)?;
        let in_d = i128::try_from(in_d).map_err(|_| Error::TimeOverflow)?;
        let beats = in_n / in_d; // `in_n >= 0` — truncating is floor.
        // Exact remainder within the beat.
        let off_n = beats.checked_mul(4).ok_or(Error::TimeOverflow)?;
        let (on, od) = ratio_add(rn, rd, -off_n, u128::from(span.denominator))?;
        let beat_offset = beat_from_wide(on, od)?;
        Ok(Some(MusicalPosition {
            bar: u32::try_from(bar).map_err(|_| Error::TimeOverflow)?,
            beat: u8::try_from(beats).map_err(|_| Error::TimeOverflow)?,
            beat_offset,
        }))
    }

    /// Builds the [`MusicalTimeInfo`] ABI snapshot at `position`,
    /// projected onto the `ppqn` grid: current tempo, current signature
    /// (`0`/`0` when none applies) and the containing bar's start (`0`
    /// when unset). Tick fields are the nearest ticks to the exact beat
    /// positions ([`Rounding::NearestEven`]); `ppqn` reports the
    /// supplied resolution.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when an intermediate overflows or a beat
    /// position cannot be projected into `ppqn` ticks.
    ///
    /// **RT contract:** RT-suitable — allocation-free binary searches +
    /// exact rational math.
    pub fn time_info(&self, position: BeatPosition, ppqn: Ppqn) -> Result<MusicalTimeInfo> {
        let seg = &self.tempo[self.tempo_index(position)];
        let position_ticks = TickPosition::from_beat(position, ppqn, Rounding::NearestEven)?
            .tick()
            .value();
        let (sig_num, sig_den, bar_start_tick) = match self
            .signature
            .partition_point(|s| s.start <= position)
            .checked_sub(1)
        {
            None => (0, 0, 0),
            Some(idx) => {
                let span = &self.signature[idx];
                let (dn, dd) = beat_sub(position, span.start);
                let bn = dn
                    .checked_mul(i128::from(span.bar_len_den))
                    .ok_or(Error::TimeOverflow)?;
                let bd = dd
                    .checked_mul(u128::from(span.bar_len_num))
                    .ok_or(Error::TimeOverflow)?;
                let bd = i128::try_from(bd).map_err(|_| Error::TimeOverflow)?;
                let bars = bn / bd;
                // Bar start = `start + bars * bar_len` — exact beats.
                let bars_n = bars
                    .checked_mul(i128::from(span.bar_len_num))
                    .ok_or(Error::TimeOverflow)?;
                let (sn, sd) = ratio_add(
                    i128::from(span.start.num),
                    u128::from(span.start.den),
                    bars_n,
                    u128::from(span.bar_len_den),
                )?;
                let bar_start = beat_from_wide(sn, sd)?;
                let ticks = TickPosition::from_beat(bar_start, ppqn, Rounding::NearestEven)?
                    .tick()
                    .value();
                (span.numerator, span.denominator, ticks)
            }
        };
        Ok(MusicalTimeInfo {
            position_ticks,
            bar_start_tick,
            bpm_milli: seg.bpm_milli,
            ppqn: ppqn.ticks_per_quarter(),
            sig_num,
            sig_den,
        })
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::*;

    fn base(num: u32, den: u32) -> TimeBase {
        TimeBase::new(num, den).unwrap()
    }

    /// Tick bound to a resolution.
    fn tp(tick: i64, ppqn: Ppqn) -> TickPosition {
        TickPosition::new(Tick::new(tick), ppqn)
    }

    /// Whole-beat position.
    fn beat(beats: i64) -> BeatPosition {
        BeatPosition::from_int(beats)
    }

    /// Rational beat position.
    fn beatq(num: i64, den: u32) -> BeatPosition {
        BeatPosition::new(num, den).unwrap()
    }

    fn tempo_960(tick: i64, bpm_milli: u32) -> TempoEvent {
        TempoEvent::at_tick(tp(tick, PPQN), bpm_milli)
    }

    fn sig_960(tick: i64, num: u8, den: u8) -> TimeSignatureEvent {
        TimeSignatureEvent::at_tick(tp(tick, PPQN), num, den)
    }

    fn map_120() -> TempoMap {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000));
        b.build().unwrap()
    }

    fn map_4_4() -> TempoMap {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_time_signature_event(sig_960(0, 4, 4));
        b.build().unwrap()
    }

    #[test]
    fn beat_position_normalizes() {
        let p = BeatPosition::new(2, 4).unwrap();
        assert_eq!((p.numerator(), p.denominator()), (1, 2));
        // 1/7 fits no PPQN grid — it stays exact.
        let seventh = beatq(1, 7);
        assert_eq!((seventh.numerator(), seventh.denominator()), (1, 7));
        assert_eq!(BeatPosition::new(0, 42).unwrap(), BeatPosition::ZERO);
        assert_eq!(BeatPosition::new(-4, 8).unwrap(), beatq(-1, 2));
        assert_eq!(
            BeatPosition::new(1, 0),
            Err(Error::InvalidValue("beat_denominator"))
        );
        // Sign lives only in the numerator.
        assert_eq!(beatq(-1, 2).to_f64(), -0.5);
        // Exact ordering across unrelated denominators.
        assert!(beatq(1, 7) < beatq(1, 6));
        assert!(beatq(-3, 7) < beatq(2, 5));
        assert_eq!(beatq(3, 9), beatq(1, 3));
        assert_eq!(format!("{}", beatq(1, 7)), "1/7");
        assert_eq!(format!("{}", beat(3)), "3");
    }

    #[test]
    fn beat_position_arithmetic() {
        assert_eq!(beatq(1, 7).checked_add(beatq(2, 5)).unwrap(), beatq(19, 35));
        assert_eq!(beat(3).checked_sub(beatq(1, 2)).unwrap(), beatq(5, 2));
        assert_eq!(beatq(1, 2).checked_neg().unwrap(), beatq(-1, 2));
        assert_eq!(
            beat(i64::MAX - 1).checked_add(beat(1)).unwrap(),
            beat(i64::MAX)
        );
        // Beyond `i64` or the `u32` denominator bound: explicit
        // overflow, never silent quantization.
        assert_eq!(
            beat(i64::MAX).checked_add(beat(1)),
            Err(Error::TimeOverflow)
        );
        assert_eq!(
            beatq(1, u32::MAX).checked_add(beatq(1, u32::MAX - 1)),
            Err(Error::TimeOverflow)
        );
        assert_eq!(beat(i64::MIN).checked_neg(), Err(Error::TimeOverflow));
    }

    #[test]
    fn ppqn_validates() {
        assert_eq!(Ppqn::new(0), Err(Error::InvalidValue("ppqn")));
        for v in [1u16, 480, 960, 1_000, 1_920, u16::MAX] {
            assert_eq!(Ppqn::new(v).unwrap().ticks_per_quarter(), v);
        }
        assert_eq!(PPQN.ticks_per_quarter(), 960);
    }

    #[test]
    fn tick_position_binds_resolution() {
        let p480 = Ppqn::new(480).unwrap();
        let a = tp(960, PPQN);
        let same_beat = tp(480, p480);
        // Same beat, different tick counts — unequal as TickPosition,
        // equal as BeatPosition.
        assert_ne!(a, same_beat);
        assert_eq!(a.to_beat(), beat(1));
        assert_eq!(same_beat.to_beat(), beat(1));
        // Same tick count, different resolution — different beats.
        assert_ne!(tp(960, PPQN).to_beat(), tp(960, p480).to_beat());
        // Cross-resolution ordering is an explicit error, not a guess.
        assert_eq!(
            a.cmp_same_resolution(&same_beat),
            Err(Error::InvalidValue("tick_ppqn_mismatch"))
        );
        assert_eq!(
            tp(10, PPQN).cmp_same_resolution(&tp(20, PPQN)),
            Ok(Ordering::Less)
        );
    }

    #[test]
    fn tick_position_projection() {
        let p480 = Ppqn::new(480).unwrap();
        // Exact: 1 beat = 480 ticks at 480 PPQN.
        assert_eq!(
            TickPosition::from_beat(beat(1), p480, Rounding::Floor).unwrap(),
            tp(480, p480)
        );
        // 1/7 beat is exact on a PPQN-7 grid.
        let p7 = Ppqn::new(7).unwrap();
        assert_eq!(
            TickPosition::from_beat(beatq(1, 7), p7, Rounding::Floor).unwrap(),
            tp(1, p7)
        );
        // 1/7 beat at 960 = 137.142… ticks — directed vs nearest.
        assert_eq!(
            TickPosition::from_beat(beatq(1, 7), PPQN, Rounding::Floor).unwrap(),
            tp(137, PPQN)
        );
        assert_eq!(
            TickPosition::from_beat(beatq(1, 7), PPQN, Rounding::NearestEven).unwrap(),
            tp(137, PPQN)
        );
        // rescale 960→480 halves the count; inexact cases honour the
        // chosen rounding.
        assert_eq!(
            tp(960, PPQN).rescale(p480, Rounding::Floor).unwrap(),
            tp(480, p480)
        );
        assert_eq!(
            tp(1, PPQN).rescale(p480, Rounding::Ceil).unwrap(),
            tp(1, p480)
        );
        assert_eq!(
            tp(1, PPQN).rescale(p480, Rounding::Floor).unwrap(),
            tp(0, p480)
        );
    }

    #[test]
    fn builder_sorts_and_validates() {
        // Unsorted input is sorted; single-tempo map converts.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(1_920, 60_000))
            .add_tempo_event(tempo_960(0, 120_000));
        let map = b.build().unwrap();
        // tick 1920 = beat 2 starts the 60 BPM segment: 2 beats at
        // 120 BPM = 1.0 s.
        assert_eq!(
            map.tick_to_media(tp(1_920, PPQN), base(1, 1)).unwrap(),
            MediaTime::new(1, base(1, 1))
        );

        for bpm in [0u32, 1_000_000_000] {
            let mut b = TempoMapBuilder::new();
            b.add_tempo_event(tempo_960(0, bpm));
            assert_eq!(b.build().unwrap_err(), Error::InvalidValue("bpm_milli"));
        }

        let mut b = TempoMapBuilder::new();
        b.add_time_signature_event(sig_960(0, 4, 3))
            .add_tempo_event(tempo_960(0, 120_000));
        assert_eq!(
            b.build().unwrap_err(),
            Error::InvalidValue("time_signature_denominator")
        );

        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_tempo_event(tempo_960(0, 60_000));
        assert_eq!(
            b.build().unwrap_err(),
            Error::InvalidValue("tempo_event_tick")
        );

        // Duplicate positions compare rationally: 1/2 beat == 480/960.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent::at(beatq(1, 2), 120_000))
            .add_tempo_event(tempo_960(480, 60_000));
        assert_eq!(
            b.build().unwrap_err(),
            Error::InvalidValue("tempo_event_tick")
        );

        let b = TempoMapBuilder::new();
        assert_eq!(b.build().unwrap_err(), Error::InvalidValue("tempo_events"));
    }

    #[test]
    fn beat_to_media_constant_tempo() {
        let map = map_120();
        // 120.000 BPM: one beat = 0.5 s; two beats = 1.0 s.
        assert_eq!(
            map.beat_to_media(beat(1), base(1, 1_000)).unwrap(),
            MediaTime::new(500, base(1, 1_000))
        );
        assert_eq!(
            map.beat_to_media(beat(2), base(1, 1)).unwrap(),
            MediaTime::new(1, base(1, 1))
        );
        // 1/7 beat = 1/14 s ≈ 71.43 ms — a single NearestEven rounding.
        assert_eq!(
            map.beat_to_media(beatq(1, 7), base(1, 1_000)).unwrap(),
            MediaTime::new(71, base(1, 1_000))
        );
        // Tick-based query delegates through the same path.
        assert_eq!(
            map.tick_to_media(tp(960, PPQN), base(1, 1_000)).unwrap(),
            MediaTime::new(500, base(1, 1_000))
        );
    }

    #[test]
    fn tempo_boundary_anchor_exact() {
        // Tempo change at beat 1: 0.5 s at 120 BPM, then 60 BPM.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent::at(beat(0), 120_000))
            .add_tempo_event(TempoEvent::at(beat(1), 60_000));
        let map = b.build().unwrap();
        // The change point maps to the accumulated seconds exactly.
        assert_eq!(
            map.beat_to_media(beat(1), base(1, 1_000_000)).unwrap(),
            MediaTime::new(500_000, base(1, 1_000_000))
        );
        // One more beat at 60 BPM adds exactly 1.0 s.
        assert_eq!(
            map.beat_to_media(beat(2), base(1, 1_000_000)).unwrap(),
            MediaTime::new(1_500_000, base(1, 1_000_000))
        );
        // media→beat recovers the anchor exactly.
        assert_eq!(
            map.media_to_beat(MediaTime::new(500_000, base(1, 1_000_000)))
                .unwrap(),
            beat(1)
        );
        // …and in ticks the boundary is `1 beat = ppqn` at any
        // resolution (the anchor is exactly representable).
        for res in [480u16, 960, 1_000, 1_920] {
            let p = Ppqn::new(res).unwrap();
            assert_eq!(
                map.media_to_tick(MediaTime::new(500_000, base(1, 1_000_000)), p)
                    .unwrap(),
                tp(i64::from(res), p)
            );
        }
    }

    #[test]
    fn variable_ppqn_media_roundtrip() {
        // Golden: the same tempo map converts identically at any
        // resolution — a tick→media→tick round-trip stays within the
        // target grid's quantization bound.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_tempo_event(tempo_960(4_800, 90_000))
            .add_tempo_event(tempo_960(9_600, 150_500));
        let map = b.build().unwrap();
        let base = base(1, 48_000);
        for res in [480u16, 960, 1_000, 1_920] {
            let p = Ppqn::new(res).unwrap();
            for &tick in &[0i64, 1, 480, 959, 960, 4_799, 4_800, 9_599, 9_600, 123_456] {
                let t = map.tick_to_media(tp(tick, p), base).unwrap();
                let back = map.media_to_tick(t, p).unwrap();
                assert!(
                    (back.tick().value() - tick).abs() <= 1,
                    "round-trip drift {back:?} for {tick}t@{p}"
                );
            }
        }
    }

    #[test]
    fn beat_position_off_grid() {
        // A tempo event anchored off every PPQN grid (1/7 beat) stays
        // exact through conversion.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent::at(beatq(1, 7), 120_000))
            .add_tempo_event(TempoEvent::at(beatq(4, 7), 60_000));
        let map = b.build().unwrap();
        // Boundary at 4/7 beat: (4/7 - 1/7) = 3/7 beats at 120 BPM =
        // 3/14 s — exact.
        assert_eq!(
            map.beat_to_media(beatq(4, 7), base(1, 14)).unwrap(),
            MediaTime::new(3, base(1, 14))
        );
        // media→beat returns the exact rational boundary.
        assert_eq!(
            map.media_to_beat(MediaTime::new(3, base(1, 14))).unwrap(),
            beatq(4, 7)
        );
    }

    #[test]
    fn tick_to_media_milli_bpm() {
        // 120.500 BPM stays on the exact-rational path.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_500));
        let map = b.build().unwrap();
        // 960 ticks = 1 beat = 60_000/120_500 s = 120/241 s — exact in
        // a 241-ticks-per-second base.
        let t = map.tick_to_media(tp(960, PPQN), base(1, 241)).unwrap();
        assert_eq!(t, MediaTime::new(120, base(1, 241)));
    }

    #[test]
    fn preroll_negative_position() {
        // First event at beat 1: earlier positions extrapolate at the
        // first segment's tempo.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(TempoEvent::at(beat(1), 120_000));
        let map = b.build().unwrap();
        // One beat before the first event = -0.5 s.
        assert_eq!(
            map.beat_to_media(beat(0), base(1, 1_000)).unwrap(),
            MediaTime::new(-500, base(1, 1_000))
        );
        // Fractional negative preroll stays exact: 3.5 beats early.
        assert_eq!(
            map.beat_to_media(beatq(-5, 2), base(1, 1_000)).unwrap(),
            MediaTime::new(-1_750, base(1, 1_000))
        );
        assert_eq!(
            map.media_to_beat(MediaTime::new(-500, base(1, 1_000)))
                .unwrap(),
            beat(0)
        );
        // Tick-level pre-roll: first event at 960t@960 → tick 0 = -0.5 s.
        let mut b2 = TempoMapBuilder::new();
        b2.add_tempo_event(tempo_960(960, 120_000));
        let map2 = b2.build().unwrap();
        assert_eq!(
            map2.tick_to_media(tp(0, PPQN), base(1, 1_000)).unwrap(),
            MediaTime::new(-500, base(1, 1_000))
        );
        assert_eq!(
            map2.media_to_tick(MediaTime::new(-500, base(1, 1_000)), PPQN)
                .unwrap(),
            tp(0, PPQN)
        );
    }

    #[test]
    fn conversion_overflow_is_explicit() {
        let map = map_120();
        // Enormous media time: the exact beat exceeds `i64` — the API
        // reports it instead of saturating.
        let t = MediaTime::new(i64::MAX, base(1, 1));
        assert_eq!(map.media_to_beat(t), Err(Error::TimeOverflow));
        assert_eq!(map.media_to_tick(t, PPQN), Err(Error::TimeOverflow));
    }

    /// Golden compatibility: every pre-migration fixed-960 case must
    /// produce identical results through the new Beat-based API.
    #[test]
    fn ppqn_960_golden_compat() {
        // Old `tick_to_media_constant_tempo` values.
        let map = map_120();
        assert_eq!(
            map.tick_to_media(tp(960, PPQN), base(1, 1_000)).unwrap(),
            MediaTime::new(500, base(1, 1_000))
        );
        assert_eq!(
            map.tick_to_media(tp(1_920, PPQN), base(1, 1)).unwrap(),
            MediaTime::new(1, base(1, 1))
        );

        // Old `tick_to_media_anchor_exact` values.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_tempo_event(tempo_960(960, 60_000));
        let map = b.build().unwrap();
        assert_eq!(
            map.tick_to_media(tp(960, PPQN), base(1, 1_000_000))
                .unwrap(),
            MediaTime::new(500_000, base(1, 1_000_000))
        );
        assert_eq!(
            map.tick_to_media(tp(1_920, PPQN), base(1, 1_000_000))
                .unwrap(),
            MediaTime::new(1_500_000, base(1, 1_000_000))
        );
        assert_eq!(
            map.media_to_tick(MediaTime::new(500_000, base(1, 1_000_000)), PPQN)
                .unwrap(),
            tp(960, PPQN)
        );

        // Old `preroll_negative_tick` values.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(960, 120_000));
        let map = b.build().unwrap();
        assert_eq!(
            map.tick_to_media(tp(0, PPQN), base(1, 1_000)).unwrap(),
            MediaTime::new(-500, base(1, 1_000))
        );
        assert_eq!(
            map.media_to_tick(MediaTime::new(-500, base(1, 1_000)), PPQN)
                .unwrap(),
            tp(0, PPQN)
        );

        // Old bar/beat decomposition: tick 3840+1920+30 → bar 1,
        // beat 2, offset 30t@960 = 1/32 beat.
        let map = map_4_4();
        assert_eq!(
            map.musical_position(tp(3_840 + 1_920 + 30, PPQN).to_beat())
                .unwrap(),
            Some(MusicalPosition {
                bar: 1,
                beat: 2,
                beat_offset: beatq(1, 32)
            })
        );

        // Old `time_info` snapshot fields at PPQN 960.
        let info = map.time_info(tp(5_760, PPQN).to_beat(), PPQN).unwrap();
        assert_eq!(info.position_ticks, 5_760);
        assert_eq!(info.bpm_milli, 120_000);
        assert_eq!((info.sig_num, info.sig_den), (4, 4));
        assert_eq!(info.bar_start_tick, 3_840);
        assert_eq!(info.ppqn, 960);
    }

    #[test]
    fn musical_position_decomposes() {
        let map = map_4_4();
        let mp = |ticks: i64| map.musical_position(tp(ticks, PPQN).to_beat()).unwrap();
        // 4/4 at 120 BPM: bar = 4 beats = 3840 ticks@960.
        assert_eq!(
            mp(0),
            Some(MusicalPosition {
                bar: 0,
                beat: 0,
                beat_offset: BeatPosition::ZERO
            })
        );
        assert_eq!(
            mp(3_840),
            Some(MusicalPosition {
                bar: 1,
                beat: 0,
                beat_offset: BeatPosition::ZERO
            })
        );
        assert_eq!(
            mp(3_840 + 1_920 + 30),
            Some(MusicalPosition {
                bar: 1,
                beat: 2,
                beat_offset: beatq(1, 32)
            })
        );

        // Signature change mid-bar truncates the previous bar and starts
        // a new one: 4/4 at 0, 3/4 at beat 5 (= bar 1, beat 1).
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_time_signature_event(sig_960(0, 4, 4))
            .add_time_signature_event(sig_960(4_800, 3, 4));
        let map = b.build().unwrap();
        let mp = |ticks: i64| map.musical_position(tp(ticks, PPQN).to_beat()).unwrap();
        assert_eq!(
            mp(4_800),
            Some(MusicalPosition {
                bar: 2,
                beat: 0,
                beat_offset: BeatPosition::ZERO
            })
        );
        // 3/4 bars are 3 beats = 2880 ticks@960.
        assert_eq!(
            mp(4_800 + 2_880),
            Some(MusicalPosition {
                bar: 3,
                beat: 0,
                beat_offset: BeatPosition::ZERO
            })
        );
    }

    #[test]
    fn musical_position_none_without_signature() {
        let map = map_120();
        assert_eq!(map.musical_position(beat(0)).unwrap(), None);

        // Position before the first signature event has no applicable sig.
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_time_signature_event(sig_960(960, 4, 4));
        let map = b.build().unwrap();
        assert_eq!(map.musical_position(beat(0)).unwrap(), None);
        assert!(map.musical_position(beat(1)).unwrap().is_some());
    }

    #[test]
    fn time_info_snapshot() {
        let map = map_4_4();
        let info = map.time_info(tp(5_760, PPQN).to_beat(), PPQN).unwrap();
        assert_eq!(info.position_ticks, 5_760);
        assert_eq!(info.bpm_milli, 120_000);
        assert_eq!((info.sig_num, info.sig_den), (4, 4));
        assert_eq!(info.bar_start_tick, 3_840);
        assert_eq!(info.ppqn, 960);

        // No signature events: sig fields are the unset representation.
        let info = map_120().time_info(beat(2), PPQN).unwrap();
        assert_eq!((info.sig_num, info.sig_den), (0, 0));
        assert_eq!(info.bar_start_tick, 0);
        assert_eq!(info.bpm_milli, 120_000);
    }

    #[test]
    fn time_info_variable_ppqn() {
        // Same map, projected at 480: every tick field halves and ppqn
        // reports the real resolution.
        let p480 = Ppqn::new(480).unwrap();
        let map = map_4_4();
        let info = map.time_info(tp(5_760, PPQN).to_beat(), p480).unwrap();
        assert_eq!(info.position_ticks, 2_880);
        assert_eq!(info.bar_start_tick, 1_920);
        assert_eq!(info.ppqn, 480);
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
    fn rt_queries_no_alloc() {
        let mut b = TempoMapBuilder::new();
        b.add_tempo_event(tempo_960(0, 120_000))
            .add_tempo_event(tempo_960(4_800, 90_000))
            .add_time_signature_event(sig_960(0, 4, 4));
        let map = b.build().unwrap();
        let base = base(1, 48_000);
        let p480 = Ppqn::new(480).unwrap();

        let (t, back, got, pos, info) = assert_no_alloc(|| {
            let t = map.tick_to_media(tp(9_600, PPQN), base).unwrap();
            let back = map.media_to_tick(t, PPQN).unwrap();
            let got = map.media_to_beat(t).unwrap();
            let pos = map.musical_position(beat(10)).unwrap();
            let info = map.time_info(beat(10), p480).unwrap();
            (t, back, got, pos, info)
        });
        assert!((back.tick().value() - 9_600).abs() <= 1);
        assert_eq!(got, beat(10));
        assert!(pos.is_some());
        assert_eq!(info.position_ticks, 4_800);
        let _ = t;
    }
}
