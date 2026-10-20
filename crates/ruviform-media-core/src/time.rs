//! Rational media time: [`TimeBase`], [`MediaTime`], [`MediaDuration`],
//! [`MediaRange`] and the [`Rounding`] policy applied by inexact
//! conversions.
//!
//! All arithmetic is exact rational integer math with `i128`/`u128`
//! intermediates; `f64` never appears on the timekeeping path (the only
//! exceptions are the display-side [`MediaTime::to_seconds_f64`] and
//! [`MediaDuration::to_seconds_f64`]). Negative instants are legal and
//! represent pre-roll.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::error::{Error, Result};

/// A rational `numerator/denominator` time base describing the duration of
/// one tick of a [`MediaTime`] or [`MediaDuration`] value.
///
/// `TimeBase` is always stored normalized (reduced to lowest terms), so
/// derived field equality is exact rational equality.
///
/// **RT contract:** RT-suitable — `Copy` value type; every operation is
/// `const`, pure and allocation-free.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeBase {
    numerator: u32,
    denominator: u32,
}

impl TimeBase {
    /// The common `1/90000` MPEG time base (90 kHz clock).
    pub const COMMON_90K: Self = Self {
        numerator: 1,
        denominator: 90_000,
    };

    /// `1/1` — one tick per second; the base of [`MediaTime::ZERO`] and
    /// [`MediaDuration::ZERO`].
    const UNIT: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// Creates a normalized `numerator/denominator` time base.
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTimeBase`] when `numerator` or `denominator` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::TimeBase;
    ///
    /// let base = TimeBase::new(1_001, 30_000).unwrap();
    /// assert_eq!(base.numerator(), 1_001);
    /// // Normalization reduces to lowest terms:
    /// assert_eq!(TimeBase::new(2, 4).unwrap(), TimeBase::new(1, 2).unwrap());
    /// ```
    pub const fn new(numerator: u32, denominator: u32) -> Result<Self> {
        if numerator == 0 || denominator == 0 {
            return Err(Error::InvalidTimeBase);
        }
        // Normalize to lowest terms (Euclid) so equality is exact.
        let (mut gcd, mut rem) = (numerator, denominator);
        while rem != 0 {
            let r = gcd % rem;
            gcd = rem;
            rem = r;
        }
        Ok(Self {
            numerator: numerator / gcd,
            denominator: denominator / gcd,
        })
    }

    /// The numerator of the normalized base.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    /// The denominator of the normalized base.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    /// The `1/rate` time base of an audio sample clock (`rate` in Hz).
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTimeBase`] when `rate` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::TimeBase;
    ///
    /// let base = TimeBase::sample_rate(48_000).unwrap();
    /// assert_eq!((base.numerator(), base.denominator()), (1, 48_000));
    /// ```
    pub const fn sample_rate(rate: u32) -> Result<Self> {
        if rate == 0 {
            return Err(Error::InvalidTimeBase);
        }
        Ok(Self {
            numerator: 1,
            denominator: rate,
        })
    }
}

/// An instant on the media timeline: `value` ticks of `base`
/// (`numerator/denominator` seconds each). Negative values are legal and
/// represent pre-roll.
///
/// Equality, ordering and hashing are exact cross-base rational
/// comparisons: `90000 @ 1/90000` equals `1 @ 1/1`, and a zero in any
/// [`TimeBase`] equals [`MediaTime::ZERO`].
///
/// `Add`/`Sub` trait impls are deliberately absent: arithmetic can fail
/// (base conversion or `i64` range), and that failure must surface as
/// `Err`, never panic — use [`MediaTime::checked_add`] /
/// [`MediaTime::checked_sub`].
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free; all operations
/// are exact integer math.
#[derive(Copy, Clone, Debug)]
pub struct MediaTime {
    value: i64,
    base: TimeBase,
}

impl MediaTime {
    /// The zero instant, expressed at a `1/1` base. Cross-base equality
    /// makes `ZERO` equal to a zero in any [`TimeBase`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TimeBase};
    ///
    /// assert_eq!(MediaTime::ZERO, MediaTime::new(0, TimeBase::COMMON_90K));
    /// ```
    pub const ZERO: Self = Self {
        value: 0,
        base: TimeBase::UNIT,
    };

    /// Creates an instant of `value` ticks at `base`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TimeBase};
    ///
    /// // 90_000 ticks at 1/90_000 == exactly one second.
    /// let t = MediaTime::new(90_000, TimeBase::COMMON_90K);
    /// assert_eq!(t, MediaTime::new(1, TimeBase::new(1, 1).unwrap()));
    /// ```
    #[must_use]
    pub const fn new(value: i64, base: TimeBase) -> Self {
        Self { value, base }
    }

    /// The raw tick count.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.value
    }

    /// The [`TimeBase`] the tick count is expressed in.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn base(self) -> TimeBase {
        self.base
    }

    /// Converts this instant into another [`TimeBase`]: exact rational
    /// math via `i128` intermediates — exact when the value is
    /// representable in `to`, otherwise rounded per `rounding`.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the result (or an `i128` intermediate)
    /// overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, Rounding, TimeBase};
    ///
    /// let t = MediaTime::new(1, TimeBase::new(1, 2).unwrap()); // 0.5 s
    /// let thirds = TimeBase::new(1, 3).unwrap();
    /// assert_eq!(t.convert(thirds, Rounding::Floor).unwrap().value(), 1);
    /// assert_eq!(t.convert(thirds, Rounding::NearestEven).unwrap().value(), 2);
    /// ```
    pub fn convert(self, to: TimeBase, rounding: Rounding) -> Result<Self> {
        convert_value(self.value, self.base, to, rounding).map(|v| Self::new(v, to))
    }

    /// Converts this instant into another [`TimeBase`] only when the value
    /// is exactly representable; `None` otherwise (and on overflow).
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TimeBase};
    ///
    /// let one_sec = MediaTime::new(1, TimeBase::new(1, 1).unwrap());
    /// assert_eq!(
    ///     one_sec.convert_exact(TimeBase::COMMON_90K),
    ///     Some(MediaTime::new(90_000, TimeBase::COMMON_90K)),
    /// );
    /// // Half a second is not representable on a 1/3-tick grid:
    /// let half_sec = MediaTime::new(1, TimeBase::new(1, 2).unwrap());
    /// assert_eq!(half_sec.convert_exact(TimeBase::new(1, 3).unwrap()), None);
    /// ```
    #[must_use]
    pub fn convert_exact(self, to: TimeBase) -> Option<Self> {
        convert_value_exact(self.value, self.base, to).map(|v| Self::new(v, to))
    }

    /// Adds a [`MediaDuration`], first converting it into this instant's
    /// base with [`Rounding::NearestEven`]; an inexact duration is rounded.
    ///
    /// This is the deliberate replacement for an `Add` impl: the contract
    /// is total — no `f64`, no panic — and failure surfaces as `Err`.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] on conversion or `i64` range overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    ///
    /// let t = MediaTime::new(4, TimeBase::new(1, 4).unwrap()); // 1 s
    /// let d = MediaDuration::new(1, TimeBase::new(1, 1).unwrap()); // 1 s
    /// assert_eq!(t.checked_add(d), Ok(MediaTime::new(8, TimeBase::new(1, 4).unwrap())));
    /// ```
    pub fn checked_add(self, duration: MediaDuration) -> Result<Self> {
        let delta = duration.convert(self.base, Rounding::NearestEven)?;
        let value = self
            .value
            .checked_add(delta.value)
            .ok_or(Error::TimeOverflow)?;
        Ok(Self::new(value, self.base))
    }

    /// Subtracts a [`MediaDuration`], first converting it into this
    /// instant's base with [`Rounding::NearestEven`]; an inexact duration
    /// is rounded.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] on conversion or `i64` range overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    ///
    /// let t = MediaTime::new(4, TimeBase::new(1, 4).unwrap()); // 1 s
    /// let d = MediaDuration::new(1, TimeBase::new(1, 1).unwrap()); // 1 s
    /// assert_eq!(t.checked_sub(d), Ok(MediaTime::ZERO));
    /// ```
    pub fn checked_sub(self, duration: MediaDuration) -> Result<Self> {
        let delta = duration.convert(self.base, Rounding::NearestEven)?;
        let value = self
            .value
            .checked_sub(delta.value)
            .ok_or(Error::TimeOverflow)?;
        Ok(Self::new(value, self.base))
    }

    /// Approximate seconds as `f64` — **display/UI convenience only**.
    ///
    /// Per the data-model contract, `f64` seconds are never a stored or
    /// core time representation; this exists for presentation and must not
    /// feed back into timekeeping. Precision loss is inherent for large
    /// values.
    ///
    /// **RT contract:** RT-suitable (pure, allocation-free), but
    /// display-only by contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TimeBase};
    ///
    /// let t = MediaTime::new(45_000, TimeBase::COMMON_90K);
    /// assert!((t.to_seconds_f64() - 0.5).abs() < 1e-9);
    /// ```
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    // `self` by value like every other `MediaTime` accessor (`Copy` type);
    // the `to_` prefix reads correctly for a value conversion.
    #[expect(clippy::cast_precision_loss, clippy::float_arithmetic)]
    // Both are inherent to this documented display-only `f64` conversion.
    pub fn to_seconds_f64(self) -> f64 {
        self.value as f64 * f64::from(self.base.numerator) / f64::from(self.base.denominator)
    }
}

impl PartialEq for MediaTime {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for MediaTime {}

impl PartialOrd for MediaTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MediaTime {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_time(self.value, self.base, other.value, other.base)
    }
}

impl Hash for MediaTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        normalized_pair(self.value, self.base).hash(state);
    }
}

/// A span on the media timeline — same shape as [`MediaTime`] but a
/// relative quantity.
///
/// Durations are kept signed for simplicity: negative values are permitted
/// (e.g. reversed ranges); enforcing non-negativity is the caller's
/// policy, not this type's invariant. Equality, ordering and hashing are
/// the same exact cross-base rational comparisons as [`MediaTime`].
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free; all operations
/// are exact integer math.
#[derive(Copy, Clone, Debug)]
pub struct MediaDuration {
    value: i64,
    base: TimeBase,
}

impl MediaDuration {
    /// The zero duration, expressed at a `1/1` base.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, TimeBase};
    ///
    /// assert_eq!(
    ///     MediaDuration::ZERO,
    ///     MediaDuration::new(0, TimeBase::COMMON_90K),
    /// );
    /// ```
    pub const ZERO: Self = Self {
        value: 0,
        base: TimeBase::UNIT,
    };

    /// Creates a duration of `value` ticks at `base`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, TimeBase};
    ///
    /// let half_sec = MediaDuration::new(45_000, TimeBase::COMMON_90K);
    /// assert_eq!(half_sec, MediaDuration::new(1, TimeBase::new(1, 2).unwrap()));
    /// ```
    #[must_use]
    pub const fn new(value: i64, base: TimeBase) -> Self {
        Self { value, base }
    }

    /// The raw tick count (signed; negative durations are legal).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.value
    }

    /// The [`TimeBase`] the tick count is expressed in.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn base(self) -> TimeBase {
        self.base
    }

    /// Converts this duration into another [`TimeBase`]: exact rational
    /// math via `i128` intermediates — exact when representable, otherwise
    /// rounded per `rounding`.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the result (or an `i128` intermediate)
    /// overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, Rounding, TimeBase};
    ///
    /// let one_sec = MediaDuration::new(1, TimeBase::new(1, 1).unwrap());
    /// let d = one_sec.convert(TimeBase::COMMON_90K, Rounding::NearestEven).unwrap();
    /// assert_eq!(d.value(), 90_000);
    /// ```
    pub fn convert(self, to: TimeBase, rounding: Rounding) -> Result<Self> {
        convert_value(self.value, self.base, to, rounding).map(|v| Self::new(v, to))
    }

    /// Converts this duration into another [`TimeBase`] only when the
    /// value is exactly representable; `None` otherwise (and on overflow).
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, TimeBase};
    ///
    /// let one_sec = MediaDuration::new(1, TimeBase::new(1, 1).unwrap());
    /// assert_eq!(
    ///     one_sec.convert_exact(TimeBase::COMMON_90K),
    ///     Some(MediaDuration::new(90_000, TimeBase::COMMON_90K)),
    /// );
    /// ```
    #[must_use]
    pub fn convert_exact(self, to: TimeBase) -> Option<Self> {
        convert_value_exact(self.value, self.base, to).map(|v| Self::new(v, to))
    }

    /// Approximate seconds as `f64` — **display/UI convenience only**.
    ///
    /// Per the data-model contract, `f64` seconds are never a stored or
    /// core time representation; this exists for presentation and must not
    /// feed back into timekeeping. Precision loss is inherent for large
    /// values.
    ///
    /// **RT contract:** RT-suitable (pure, allocation-free), but
    /// display-only by contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, TimeBase};
    ///
    /// let d = MediaDuration::new(45_000, TimeBase::COMMON_90K);
    /// assert!((d.to_seconds_f64() - 0.5).abs() < 1e-9);
    /// ```
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    // `self` by value like every other `MediaDuration` accessor (`Copy`
    // type); the `to_` prefix reads correctly for a value conversion.
    #[expect(clippy::cast_precision_loss, clippy::float_arithmetic)]
    // Both are inherent to this documented display-only `f64` conversion.
    pub fn to_seconds_f64(self) -> f64 {
        self.value as f64 * f64::from(self.base.numerator) / f64::from(self.base.denominator)
    }
}

impl PartialEq for MediaDuration {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for MediaDuration {}

impl PartialOrd for MediaDuration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MediaDuration {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_time(self.value, self.base, other.value, other.base)
    }
}

impl Hash for MediaDuration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        normalized_pair(self.value, self.base).hash(state);
    }
}

/// Rounding policy applied when a value cannot be represented exactly in
/// the target [`TimeBase`].
///
/// [`Rounding::NearestEven`] (round-half-to-even) is the documented
/// default for media arithmetic because it is unbiased across repeated
/// conversions.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceil,
    /// Toward zero (truncation).
    TowardZero,
    /// Away from zero.
    AwayFromZero,
    /// To the nearest representable value; exact halves go to the even
    /// neighbour (round-half-to-even). This is the default.
    #[default]
    NearestEven,
}

/// A start instant plus a duration — the half-open interval
/// `[start, start + duration)` on the media timeline.
///
/// No validation is performed: a negative [`MediaDuration`] yields a
/// backwards range in which [`MediaRange::end`] sits before
/// [`MediaRange::start`].
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaRange {
    start: MediaTime,
    duration: MediaDuration,
}

impl MediaRange {
    /// Creates a range from `start` and `duration` (no validation; see the
    /// type-level docs).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, TimeBase};
    ///
    /// let range = MediaRange::new(
    ///     MediaTime::new(4, TimeBase::new(1, 4).unwrap()),
    ///     MediaDuration::new(1, TimeBase::new(1, 1).unwrap()),
    /// );
    /// assert_eq!(range.end(), MediaTime::new(8, TimeBase::new(1, 4).unwrap()));
    /// ```
    #[must_use]
    pub const fn new(start: MediaTime, duration: MediaDuration) -> Self {
        Self { start, duration }
    }

    /// The start instant.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn start(self) -> MediaTime {
        self.start
    }

    /// The duration of the range (signed; see the type-level docs).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn duration(self) -> MediaDuration {
        self.duration
    }

    /// The end instant: `start + duration`, expressed in `start`'s base.
    /// The duration is converted with [`Rounding::NearestEven`] first.
    ///
    /// This function is total: if the conversion or the sum exceeds the
    /// `i64` value range, the result saturates at `i64::MIN`/`i64::MAX` in
    /// `start`'s base — never a panic.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, TimeBase};
    ///
    /// let range = MediaRange::new(MediaTime::ZERO, MediaDuration::new(1, TimeBase::new(1, 1).unwrap()));
    /// assert_eq!(range.end(), MediaTime::new(1, TimeBase::new(1, 1).unwrap()));
    /// ```
    #[must_use]
    pub fn end(self) -> MediaTime {
        let delta = match self
            .duration
            .convert(self.start.base, Rounding::NearestEven)
        {
            Ok(d) => i128::from(d.value),
            // Conversion overflow saturates in the duration's own direction.
            Err(_) => i128::from(if self.duration.value < 0 {
                i64::MIN
            } else {
                i64::MAX
            }),
        };
        let end = i128::from(self.start.value) + delta;
        let clamped = end.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
        // `clamped` is in `i64` range by construction; the fallback is unreachable.
        MediaTime::new(i64::try_from(clamped).unwrap_or(i64::MAX), self.start.base)
    }
}

/// Exact cross-base comparison of `value * numerator / denominator`.
/// Magnitudes are `|value| * numerator * other_denominator` in `u128`,
/// bounded by `2^63 * (2^32 - 1)^2 < 2^128`, so this cannot overflow.
fn cmp_time(v1: i64, b1: TimeBase, v2: i64, b2: TimeBase) -> Ordering {
    match (v1 < 0, v2 < 0) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => scaled_magnitude(v1, b1.numerator, b2.denominator)
            .cmp(&scaled_magnitude(v2, b2.numerator, b1.denominator)),
        (true, true) => scaled_magnitude(v2, b2.numerator, b1.denominator).cmp(&scaled_magnitude(
            v1,
            b1.numerator,
            b2.denominator,
        )),
    }
}

#[allow(clippy::cast_lossless)] // `as` casts: `const fn` cannot call `From::from`
const fn scaled_magnitude(value: i64, numerator: u32, other_denominator: u32) -> u128 {
    (value.unsigned_abs() as u128) * (numerator as u128) * (other_denominator as u128)
}

/// Canonical `(numerator, denominator)` of the represented rational in
/// lowest terms (sign carried by the numerator), so `Hash` stays
/// consistent with cross-base `Eq` — equal rationals at different bases
/// hash identically, and zero hashes as `0/1` in any base.
fn normalized_pair(value: i64, base: TimeBase) -> (i128, u128) {
    let num = i128::from(value) * i128::from(base.numerator);
    let den = u128::from(base.denominator);
    let mag = num.unsigned_abs();
    let (mut a, mut b) = (mag, den);
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    // `a == gcd(mag, den)`; `reduced_mag <= 2^95`, so the narrowing below
    // cannot actually fail.
    let reduced_num = i128::try_from(mag / a).unwrap_or(i128::MAX);
    (if num < 0 { -reduced_num } else { reduced_num }, den / a)
}

/// Rescales `value` ticks of `from` into the fraction `num/den` ticks of
/// `to` (`den > 0` always). Returns `None` only on `i128` intermediate
/// overflow, reachable solely at `i64::MIN`-times-`u32::MAX` extremes.
fn scaled_fraction(value: i64, from: TimeBase, to: TimeBase) -> Option<(i128, u128)> {
    let num = i128::from(value)
        .checked_mul(i128::from(from.numerator))?
        .checked_mul(i128::from(to.denominator))?;
    let den = u128::from(from.denominator) * u128::from(to.numerator);
    Some((num, den))
}

/// Converts `value` ticks of `from` into ticks of `to`; exact when
/// representable, otherwise rounded per `rounding`. Overflow surfaces as
/// [`Error::TimeOverflow`].
fn convert_value(value: i64, from: TimeBase, to: TimeBase, rounding: Rounding) -> Result<i64> {
    let (num, den) = scaled_fraction(value, from, to).ok_or(Error::TimeOverflow)?;

    // Divide with explicit rounding on the magnitude; the sign is
    // re-applied afterwards so all modes stay symmetric for pre-roll.
    let negative = num < 0;
    let mut q = num.unsigned_abs() / den;
    let r = num.unsigned_abs() % den;
    let round_up = match rounding {
        Rounding::Floor => negative && r > 0,
        Rounding::Ceil => !negative && r > 0,
        Rounding::TowardZero => false,
        Rounding::AwayFromZero => r > 0,
        // Round-half-to-even.
        Rounding::NearestEven => {
            let twice = r * 2;
            twice > den || (twice == den && q % 2 == 1)
        }
    };
    if round_up {
        q += 1;
    }
    // The result must fit `i64`: `|i64::MIN| == 2^63` on the negative side.
    let limit: u128 = if negative { 1 << 63 } else { (1 << 63) - 1 };
    if q > limit {
        return Err(Error::TimeOverflow);
    }
    let magnitude = i128::try_from(q).ok().ok_or(Error::TimeOverflow)?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).ok().ok_or(Error::TimeOverflow)
}

/// Converts `value` ticks of `from` into ticks of `to` only when exactly
/// representable; `None` on any remainder or overflow.
fn convert_value_exact(value: i64, from: TimeBase, to: TimeBase) -> Option<i64> {
    let (num, den) = scaled_fraction(value, from, to)?;
    let den = i128::try_from(den).ok()?;
    if num % den != 0 {
        return None;
    }
    i64::try_from(num / den).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn base(numerator: u32, denominator: u32) -> TimeBase {
        TimeBase::new(numerator, denominator).unwrap()
    }

    #[test]
    fn time_base_normalizes_and_validates() {
        assert_eq!(base(2, 4), base(1, 2));
        assert_eq!(TimeBase::new(0, 1), Err(Error::InvalidTimeBase));
        assert_eq!(TimeBase::new(1, 0), Err(Error::InvalidTimeBase));
        assert_eq!(TimeBase::sample_rate(0), Err(Error::InvalidTimeBase));
        assert_eq!(TimeBase::sample_rate(48_000), Ok(base(1, 48_000)));
        assert_eq!(
            (
                TimeBase::COMMON_90K.numerator(),
                TimeBase::COMMON_90K.denominator()
            ),
            (1, 90_000)
        );
    }

    #[test]
    fn conversion_exact_and_rounding() {
        let one_sec = MediaTime::new(1, base(1, 1));
        assert_eq!(
            one_sec.convert_exact(TimeBase::COMMON_90K),
            Some(MediaTime::new(90_000, TimeBase::COMMON_90K))
        );

        // 1/2 s does not land on a 1/3 tick (1.5 ticks).
        let half = MediaTime::new(1, base(1, 2));
        let thirds = base(1, 3);
        assert_eq!(half.convert_exact(thirds), None);
        assert_eq!(half.convert(thirds, Rounding::Floor).unwrap().value(), 1);
        assert_eq!(half.convert(thirds, Rounding::Ceil).unwrap().value(), 2);
        assert_eq!(
            half.convert(thirds, Rounding::TowardZero).unwrap().value(),
            1
        );
        assert_eq!(
            half.convert(thirds, Rounding::AwayFromZero)
                .unwrap()
                .value(),
            2
        );
        // 1.5 -> 2 (nearest even).
        assert_eq!(
            half.convert(thirds, Rounding::NearestEven).unwrap().value(),
            2
        );

        // Negative side (pre-roll): -1.5 ticks.
        let neg_half = MediaTime::new(-1, base(1, 2));
        assert_eq!(
            neg_half.convert(thirds, Rounding::Floor).unwrap().value(),
            -2
        );
        assert_eq!(
            neg_half.convert(thirds, Rounding::Ceil).unwrap().value(),
            -1
        );
        assert_eq!(
            neg_half
                .convert(thirds, Rounding::TowardZero)
                .unwrap()
                .value(),
            -1
        );
        assert_eq!(
            neg_half
                .convert(thirds, Rounding::AwayFromZero)
                .unwrap()
                .value(),
            -2
        );
        // -1.5 -> -2 (nearest even).
        assert_eq!(
            neg_half
                .convert(thirds, Rounding::NearestEven)
                .unwrap()
                .value(),
            -2
        );

        // NearestEven non-tie case rounds to nearest: 2/3 s in 1/4 ticks
        // is 8/3 = 2.66... -> 3.
        let two_thirds = MediaTime::new(2, base(1, 3));
        let quarters = base(1, 4);
        assert_eq!(
            two_thirds
                .convert(quarters, Rounding::NearestEven)
                .unwrap()
                .value(),
            3
        );
        assert_eq!(
            two_thirds
                .convert(quarters, Rounding::Floor)
                .unwrap()
                .value(),
            2
        );
    }

    #[test]
    fn conversion_overflow_surfaces() {
        let big = MediaTime::new(i64::MAX, base(1, 1));
        // ~9.2e18 s in 90 kHz ticks exceeds the i64 value range.
        assert_eq!(
            big.convert(TimeBase::COMMON_90K, Rounding::Floor),
            Err(Error::TimeOverflow)
        );
        assert_eq!(big.convert_exact(TimeBase::COMMON_90K), None);
    }

    #[test]
    fn cross_base_equality_ordering_and_hash() {
        let one_sec = MediaTime::new(1, base(1, 1));
        let ninety_k_ticks = MediaTime::new(90_000, TimeBase::COMMON_90K);
        assert_eq!(one_sec, ninety_k_ticks);
        assert!(MediaTime::new(89_999, TimeBase::COMMON_90K) < one_sec);
        assert!(MediaTime::new(90_001, TimeBase::COMMON_90K) > one_sec);
        assert_eq!(MediaTime::ZERO, MediaTime::new(0, base(1, 48_000)));

        // Hash stays consistent with cross-base Eq: a stored instant is
        // found via an equal value in any base — including zero.
        let mut set = HashSet::new();
        set.insert(one_sec);
        assert!(set.contains(&ninety_k_ticks));
        set.insert(MediaTime::ZERO);
        assert!(set.contains(&MediaTime::new(0, base(1, 48_000))));

        // MediaDuration follows the same cross-base contract.
        let one_sec_dur = MediaDuration::new(1, base(1, 1));
        assert_eq!(
            one_sec_dur,
            MediaDuration::new(90_000, TimeBase::COMMON_90K)
        );
        assert!(MediaDuration::new(89_999, TimeBase::COMMON_90K) < one_sec_dur);
        let mut dur_set = HashSet::new();
        dur_set.insert(one_sec_dur);
        assert!(dur_set.contains(&MediaDuration::new(90_000, TimeBase::COMMON_90K)));
    }

    #[test]
    fn negative_time_is_pre_roll() {
        let neg = MediaTime::new(-1, base(1, 1));
        assert!(neg < MediaTime::ZERO);
        assert_eq!(
            neg.convert(TimeBase::COMMON_90K, Rounding::Floor),
            Ok(MediaTime::new(-90_000, TimeBase::COMMON_90K))
        );
    }

    #[test]
    fn checked_add_sub() {
        let four_quarters = MediaTime::new(4, base(1, 4)); // 1 s
        let one_sec = MediaDuration::new(1, base(1, 1));
        assert_eq!(
            four_quarters.checked_add(one_sec),
            Ok(MediaTime::new(8, base(1, 4)))
        );
        assert_eq!(
            four_quarters.checked_sub(one_sec),
            Ok(MediaTime::new(0, base(1, 4)))
        );

        // Inexact conversion rounds with NearestEven: 1/3 s into 1/2-tick
        // base is 0.667 ticks -> 1.
        let third = MediaDuration::new(1, base(1, 3));
        let half = MediaTime::new(1, base(1, 2));
        assert_eq!(half.checked_add(third), Ok(MediaTime::new(2, base(1, 2))));

        // Overflow surfaces as Err, never a panic.
        assert_eq!(
            MediaTime::new(i64::MAX, base(1, 1)).checked_add(one_sec),
            Err(Error::TimeOverflow)
        );
        assert_eq!(
            MediaTime::new(i64::MIN, base(1, 1)).checked_sub(one_sec),
            Err(Error::TimeOverflow)
        );
    }

    #[test]
    fn duration_is_signed_and_converts() {
        let neg = MediaDuration::new(-3, base(1, 1));
        assert_eq!(neg.base(), base(1, 1));
        assert_eq!(
            neg.convert(TimeBase::COMMON_90K, Rounding::TowardZero),
            Ok(MediaDuration::new(-270_000, TimeBase::COMMON_90K))
        );
        assert!(neg < MediaDuration::ZERO);
    }

    #[test]
    fn duration_display_seconds() {
        let half_sec = MediaDuration::new(45_000, TimeBase::COMMON_90K);
        assert_eq!(half_sec.base(), TimeBase::COMMON_90K);
        assert!((half_sec.to_seconds_f64() - 0.5).abs() < 1e-9);
        // Signed durations produce signed seconds.
        let neg = MediaDuration::new(-90_000, TimeBase::COMMON_90K);
        assert!((neg.to_seconds_f64() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn range_end_in_starts_base() {
        let start = MediaTime::new(4, base(1, 4)); // 1 s
        let dur = MediaDuration::new(90_000, TimeBase::COMMON_90K); // 1 s
        let range = MediaRange::new(start, dur);
        assert_eq!(range.start(), start);
        assert_eq!(range.duration(), dur);
        let end = range.end();
        assert_eq!(end, MediaTime::new(8, base(1, 4)));
        assert_eq!(end.base(), base(1, 4));

        // Inexact conversion rounds with NearestEven: 1/3 s into 1/4-tick
        // base is 4/3 = 1.33... -> 1 tick.
        let r = MediaRange::new(
            MediaTime::new(4, base(1, 4)),
            MediaDuration::new(1, base(1, 3)),
        );
        assert_eq!(r.end(), MediaTime::new(5, base(1, 4)));
    }
}
