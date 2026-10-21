//! Rational media time: [`TimeBase`], [`FrameRate`], [`MediaTime`],
//! [`MediaDuration`], [`MediaRange`] and the [`Rounding`] policy applied
//! by inexact conversions.
//!
//! All arithmetic is exact rational integer math with `i128`/`u128`
//! intermediates; `f64` never appears on the timekeeping path (the only
//! exceptions are the display-side [`MediaTime::to_seconds_f64`] and
//! [`MediaDuration::to_seconds_f64`]). Negative instants are legal and
//! represent pre-roll.
//!
//! Every conversion and arithmetic call applies at most one rounding —
//! the final step, never per intermediate. Chained conversions
//! therefore accumulate rounding error: keep values in their native
//! base as the stable anchor and convert at display/IO boundaries —
//! repeated base-hopping is discouraged.

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

/// A rational frame rate in frames per second, stored as a normalized
/// `numerator/denominator` fraction.
///
/// `FrameRate` is semantically distinct from [`TimeBase`]: it stores an
/// fps-oriented rational (`numerator` frames per `denominator` seconds),
/// whereas `TimeBase` stores the duration of one tick — the reciprocal.
/// The stored value is therefore **not** exposed as a `TimeBase`; the
/// only accessor returning a `TimeBase` is [`FrameRate::frame_duration`],
/// which yields the true per-frame tick length (the reciprocal).
/// Components are available via [`FrameRate::numerator`] and
/// [`FrameRate::denominator`]. `to_time_base()` and `From` conversions
/// between the two types are deliberately absent so an fps value cannot
/// silently be used as a tick duration.
///
/// The value is normalized at construction (`60_000/2_002` reduces to
/// `30_000/1_001`), so equality is exact rational equality.
///
/// **RT contract:** RT-suitable — `Copy` value type; every operation is
/// `const`, pure and allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{FrameRate, TimeBase};
///
/// let rate = FrameRate::new(30_000, 1_001).unwrap(); // 29.97 fps
/// assert_eq!(rate.numerator(), 30_000);
/// assert_eq!(rate.denominator(), 1_001);
/// assert_eq!(rate.nominal_fps(), 30);
/// assert_eq!(rate.frame_duration(), TimeBase::new(1_001, 30_000).unwrap());
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FrameRate(TimeBase);

impl FrameRate {
    /// Creates a normalized `numerator/denominator` fps rational.
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with `"frame_rate"` when `numerator` or
    /// `denominator` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::FrameRate;
    ///
    /// assert!(FrameRate::new(0, 1).is_err());
    /// assert!(FrameRate::new(1, 0).is_err());
    /// // Normalization reduces to lowest terms:
    /// assert_eq!(
    ///     FrameRate::new(60_000, 2_002).unwrap(),
    ///     FrameRate::new(30_000, 1_001).unwrap()
    /// );
    /// ```
    pub const fn new(numerator: u32, denominator: u32) -> Result<Self> {
        match TimeBase::new(numerator, denominator) {
            Ok(fps) => Ok(Self(fps)),
            Err(_) => Err(Error::InvalidValue("frame_rate")),
        }
    }

    /// Shorthand for an integer frame rate (`fps/1` frames per second).
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with `"frame_rate"` when `fps` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::FrameRate;
    ///
    /// assert_eq!(
    ///     FrameRate::from_fps(24).unwrap(),
    ///     FrameRate::new(24, 1).unwrap()
    /// );
    /// ```
    pub const fn from_fps(fps: u32) -> Result<Self> {
        Self::new(fps, 1)
    }

    /// The numerator of the normalized fps rational.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.0.numerator()
    }

    /// The denominator of the normalized fps rational.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.0.denominator()
    }

    /// The duration of one frame as a [`TimeBase`] — the reciprocal of
    /// the stored fps rational (`denominator/numerator` seconds).
    ///
    /// This is the only accessor exposing the rate as a `TimeBase`, and
    /// it returns the true tick length, never the fps-oriented stored
    /// value.
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{FrameRate, TimeBase};
    ///
    /// let rate = FrameRate::from_fps(25).unwrap();
    /// assert_eq!(rate.frame_duration(), TimeBase::new(1, 25).unwrap());
    /// ```
    #[must_use]
    pub const fn frame_duration(self) -> TimeBase {
        // Components are non-zero and normalized (enforced at
        // construction), and gcd is symmetric, so the reciprocal is
        // already a valid normalized base.
        TimeBase {
            numerator: self.denominator(),
            denominator: self.numerator(),
        }
    }

    /// The nominal whole-number frame rate used for timecode labelling,
    /// rounded half-up (`(2*n + d) / (2*d)`).
    ///
    /// NTSC rates map onto their broadcast nominals: `30000/1001` → `30`,
    /// `60000/1001` → `60`, `24000/1001` → `24`. The half-up comparison is
    /// written as `remainder >= denominator - remainder` so no widened
    /// intermediate is needed and extreme rationals cannot overflow.
    ///
    /// **RT contract:** RT-suitable — pure `const` integer math.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::FrameRate;
    ///
    /// assert_eq!(FrameRate::new(30_000, 1_001).unwrap().nominal_fps(), 30);
    /// assert_eq!(FrameRate::new(60_000, 1_001).unwrap().nominal_fps(), 60);
    /// assert_eq!(FrameRate::from_fps(25).unwrap().nominal_fps(), 25);
    /// ```
    #[must_use]
    pub const fn nominal_fps(self) -> u32 {
        let numerator = self.numerator();
        let denominator = self.denominator();
        // Round half-up as `q + [2r >= d]`. `2 * r` could overflow `u32`
        // when `d` is large, so the comparison is rewritten as
        // `r >= d - r` (valid since `0 <= r < d`). The result fits `u32`:
        // `q + 1 <= n / d + 1 <= n` for `d >= 2`, and `r == 0` when `d == 1`.
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        if remainder >= denominator - remainder {
            quotient + 1
        } else {
            quotient
        }
    }
}

impl core::fmt::Debug for FrameRate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FrameRate({}/{})", self.numerator(), self.denominator())
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
/// (the resulting instant may fall outside the `i64` range), and that
/// failure must surface as `Err`, never panic — use
/// [`MediaTime::checked_add`] / [`MediaTime::checked_sub`].
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
    /// One call applies at most one rounding — the final step. Chained
    /// conversions accumulate rounding error: prefer converting from a
    /// stable anchor (keep values in their native base, convert at
    /// display/IO boundaries); repeated base-hopping is discouraged.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the converted value exceeds the
    /// `i64` range.
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
    /// base with [`Rounding::NearestEven`]; an inexact duration is rounded
    /// once.
    ///
    /// The converted delta is held as an exact `i128` rational quotient,
    /// so a duration whose conversion alone exceeds the `i64` range does
    /// not fail early — only the final `self + delta` instant is checked
    /// against the representable range.
    ///
    /// This is the deliberate replacement for an `Add` impl: the contract
    /// is total — no `f64`, no panic — and failure surfaces as `Err`.
    ///
    /// One call applies at most one rounding — repeated
    /// add-and-convert loops accumulate rounding error, so accumulate
    /// in a stable base (see the module docs).
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the resulting instant falls outside
    /// the `i64` value range.
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
        let delta = convert_value_wide(
            duration.value,
            duration.base,
            self.base,
            Rounding::NearestEven,
        );
        let value = i64::try_from(i128::from(self.value) + delta)
            .ok()
            .ok_or(Error::TimeOverflow)?;
        Ok(Self::new(value, self.base))
    }

    /// Subtracts a [`MediaDuration`], first converting it into this
    /// instant's base with [`Rounding::NearestEven`]; an inexact duration
    /// is rounded once.
    ///
    /// The converted delta is held as an exact `i128` rational quotient
    /// and negated at `i128` width, so an `i64::MIN` duration is handled
    /// exactly — only the final `self - delta` instant is checked against
    /// the representable range.
    ///
    /// One call applies at most one rounding — repeated
    /// subtract-and-convert loops accumulate rounding error, so accumulate
    /// in a stable base (see the module docs).
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the resulting instant falls outside
    /// the `i64` value range.
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
        let delta = convert_value_wide(
            duration.value,
            duration.base,
            self.base,
            Rounding::NearestEven,
        );
        let value = i64::try_from(i128::from(self.value) - delta)
            .ok()
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
    /// [`Error::TimeOverflow`] when the converted value exceeds the
    /// `i64` range.
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
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
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
    /// The delta is kept as an exact `i128` rational quotient, so a
    /// duration whose conversion alone exceeds the `i64` range still
    /// yields the true end whenever `start + delta` is representable.
    ///
    /// This function is total: if the final `start + duration` exceeds
    /// the `i64` value range, the result saturates at
    /// `i64::MIN`/`i64::MAX` in `start`'s base — never a panic.
    /// Saturation happens only on true final overflow; use
    /// [`MediaRange::checked_end`] to observe it as an `Err` instead.
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
        let end = self.end_wide(Rounding::NearestEven);
        let clamped = end.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
        // `clamped` is in `i64` range by construction; the fallback is unreachable.
        MediaTime::new(i64::try_from(clamped).unwrap_or(i64::MAX), self.start.base)
    }

    /// Checked variant of [`MediaRange::end`] with an explicit `rounding`
    /// applied to the duration conversion.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] only when the final `start + delta`
    /// instant falls outside the `i64` value range — an intermediate
    /// delta that exceeds `i64` on its own is not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, Rounding, TimeBase};
    ///
    /// let base = TimeBase::new(1, 1).unwrap();
    /// let range = MediaRange::new(MediaTime::new(i64::MAX - 1, base), MediaDuration::new(1, base));
    /// assert_eq!(range.checked_end(Rounding::NearestEven).map(|t| t.value()), Ok(i64::MAX));
    /// let overflowing = MediaRange::new(
    ///     MediaTime::new(i64::MAX, base),
    ///     MediaDuration::new(2, base),
    /// );
    /// assert!(overflowing.checked_end(Rounding::NearestEven).is_err());
    /// ```
    pub fn checked_end(self, rounding: Rounding) -> Result<MediaTime> {
        let value = i64::try_from(self.end_wide(rounding))
            .ok()
            .ok_or(Error::TimeOverflow)?;
        Ok(MediaTime::new(value, self.start.base))
    }

    /// `start + duration` in `start`'s base as an `i128`, with `rounding`
    /// applied once to the duration's conversion. The exact intermediate
    /// keeps full precision, so `i64` range checks apply only to this
    /// final value; `|start| + |delta| <= 2^63 + 2^63 * (2^32 - 1)^2`
    /// still fits `i128`.
    fn end_wide(self, rounding: Rounding) -> i128 {
        let delta = convert_value_wide(
            self.duration.value,
            self.duration.base,
            self.start.base,
            rounding,
        );
        i128::from(self.start.value) + delta
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
/// `to` (`den > 0` always). `|num| <= 2^63 * (2^32 - 1)^2 < 2^127`, so the
/// `i128` intermediate cannot overflow.
fn scaled_fraction(value: i64, from: TimeBase, to: TimeBase) -> (i128, u128) {
    let num = i128::from(value) * i128::from(from.numerator) * i128::from(to.denominator);
    let den = u128::from(from.denominator) * u128::from(to.numerator);
    (num, den)
}

/// Applies `rounding` to the exact rational `num / den` (`den > 0`),
/// returning the quotient as `i128` with no `i64` range check. Rounding
/// acts on the magnitude and the sign is re-applied afterwards, so all
/// modes stay symmetric for pre-roll values.
fn round_quotient(num: i128, den: u128, rounding: Rounding) -> i128 {
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
    // `q <= |num| + 1 <= 2^63 * (2^32 - 1)^2 + 1 < i128::MAX`, so the
    // narrowing cannot fail; the fallback is unreachable.
    let magnitude = i128::try_from(q).unwrap_or(i128::MAX);
    if negative { -magnitude } else { magnitude }
}

/// Converts `value` ticks of `from` into ticks of `to` as an `i128`,
/// applying `rounding` once — with no `i64` range check on the converted
/// value itself. End/add/sub arithmetic uses this so range validity is
/// judged only on the final instant.
fn convert_value_wide(value: i64, from: TimeBase, to: TimeBase, rounding: Rounding) -> i128 {
    let (num, den) = scaled_fraction(value, from, to);
    round_quotient(num, den, rounding)
}

/// Converts `value` ticks of `from` into ticks of `to`; exact when
/// representable, otherwise rounded per `rounding`. Overflow surfaces as
/// [`Error::TimeOverflow`].
fn convert_value(value: i64, from: TimeBase, to: TimeBase, rounding: Rounding) -> Result<i64> {
    let (num, den) = scaled_fraction(value, from, to);
    i64::try_from(round_quotient(num, den, rounding))
        .ok()
        .ok_or(Error::TimeOverflow)
}

/// Converts `value` ticks of `from` into ticks of `to` only when exactly
/// representable; `None` on any remainder or overflow.
fn convert_value_exact(value: i64, from: TimeBase, to: TimeBase) -> Option<i64> {
    let (num, den) = scaled_fraction(value, from, to);
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

    fn rate(numerator: u32, denominator: u32) -> FrameRate {
        FrameRate::new(numerator, denominator).unwrap()
    }

    #[test]
    fn frame_rate_normalizes_and_validates() {
        assert_eq!(rate(60_000, 2_002), rate(30_000, 1_001));
        assert_eq!(rate(48, 2), rate(24, 1));
        assert_eq!(FrameRate::new(0, 1), Err(Error::InvalidValue("frame_rate")));
        assert_eq!(FrameRate::new(1, 0), Err(Error::InvalidValue("frame_rate")));
        assert_eq!(
            FrameRate::from_fps(0),
            Err(Error::InvalidValue("frame_rate"))
        );
        assert_eq!(FrameRate::from_fps(24), Ok(rate(24, 1)));
    }

    #[test]
    fn frame_duration_is_the_reciprocal_tick_length() {
        assert_eq!(rate(30_000, 1_001).frame_duration(), base(1_001, 30_000));
        assert_eq!(
            FrameRate::from_fps(25).unwrap().frame_duration(),
            base(1, 25)
        );
        // The stored fps rational itself is never exposed as a `TimeBase`:
        // only the reciprocal frame duration is.
        let ntsc = rate(30_000, 1_001);
        assert_eq!(
            (
                ntsc.frame_duration().numerator(),
                ntsc.frame_duration().denominator()
            ),
            (ntsc.denominator(), ntsc.numerator())
        );
    }

    #[test]
    fn nominal_fps_rounds_half_up_to_broadcast_nominals() {
        assert_eq!(FrameRate::from_fps(24).unwrap().nominal_fps(), 24);
        assert_eq!(FrameRate::from_fps(25).unwrap().nominal_fps(), 25);
        assert_eq!(rate(30_000, 1_001).nominal_fps(), 30);
        assert_eq!(rate(60_000, 1_001).nominal_fps(), 60);
        assert_eq!(rate(24_000, 1_001).nominal_fps(), 24);
        // Exact half rounds up.
        assert_eq!(rate(5, 2).nominal_fps(), 3);
        // Extreme rationals must not overflow the `2*num` intermediate.
        assert_eq!(rate(u32::MAX, 1).nominal_fps(), u32::MAX);
        assert_eq!(rate(1, u32::MAX).nominal_fps(), 0);
    }

    /// |a - b| in seconds as a `(numerator, denominator)` rational —
    /// exact `i128`/`u128` math, no `f64` on the assertion path.
    fn err_seconds(a: MediaTime, b: MediaTime) -> (u128, u128) {
        let lhs = i128::from(a.value())
            * i128::from(a.base().numerator())
            * i128::from(b.base().denominator());
        let rhs = i128::from(b.value())
            * i128::from(b.base().numerator())
            * i128::from(a.base().denominator());
        (
            (lhs - rhs).unsigned_abs(),
            u128::from(a.base().denominator()) * u128::from(b.base().denominator()),
        )
    }

    /// Sum of `0.5 * numerator/denominator` seconds over `bases`, as a
    /// `(numerator, denominator)` rational over the common denominator
    /// `2 * prod(d_i)`. Valid for the small base sets used in these
    /// tests (each product fits `u128`).
    fn half_tick_bound_seconds(bases: &[TimeBase]) -> (u128, u128) {
        let product: u128 = bases.iter().map(|b| u128::from(b.denominator())).product();
        let numerator: u128 = bases
            .iter()
            .map(|b| u128::from(b.numerator()) * (product / u128::from(b.denominator())))
            .sum();
        (numerator, product * 2)
    }

    /// Asserts the exact rational error `|a - b|` does not exceed the
    /// rational `bound` (both in seconds).
    fn assert_error_within(a: MediaTime, b: MediaTime, bound: (u128, u128)) {
        let (err_num, err_den) = err_seconds(a, b);
        assert!(
            err_num * bound.1 <= bound.0 * err_den,
            "error {err_num}/{err_den}s exceeds bound {}/{}s (a={a:?} b={b:?})",
            bound.0,
            bound.1
        );
    }

    /// Audio clocks and video frame grids named in the rounding-policy
    /// matrix (the video side is derived from [`FrameRate`]).
    fn precision_matrix_bases() -> ([TimeBase; 3], [TimeBase; 4]) {
        let audio = [base(1, 44_100), base(1, 48_000), base(1, 90_000)];
        let video = [
            rate(24, 1).frame_duration(),
            rate(25, 1).frame_duration(),
            rate(30_000, 1_001).frame_duration(),
            rate(60_000, 1_001).frame_duration(),
        ];
        (audio, video)
    }

    #[test]
    fn single_conversion_error_is_bounded_by_half_tick() {
        let (audio, video) = precision_matrix_bases();
        // Values stay small enough that every base pair stays
        // representable (worst: 1e12 ticks at 1/24 s -> ~4e15 ticks at
        // 1/90000); overflow is a separate contract (`convert` errs).
        let values = [0_i64, 1, -1, 1_000, 1_000_000_000_000, -1_000_000_000_000];
        for &from in audio.iter().chain(video.iter()) {
            for &to in audio.iter().chain(video.iter()) {
                let bound = half_tick_bound_seconds(&[to]);
                for v in values {
                    let t = MediaTime::new(v, from);
                    let converted = t.convert(to, Rounding::NearestEven).unwrap();
                    // |converted - t| <= 0.5 tick of the target base.
                    // (v=1000 at 1/48000 -> 1/24 is an exact-half tie.)
                    assert_error_within(t, converted, bound);
                }
            }
        }
    }

    #[test]
    fn round_trip_error_is_bounded_by_summed_half_ticks() {
        // t -> B -> original: error <= 0.5 tick of B + 0.5 tick of the
        // original base, both expressed in seconds. A coarse hop through
        // 1/24 may round 1000 ticks of 1/48000 to 0 — that must still sit
        // inside the summed bound even though it is huge in 1/48000 units.
        let original = base(1, 48_000);
        let coarse = base(1, 24);
        let t = MediaTime::new(1_000, original);
        let round_tripped = t
            .convert(coarse, Rounding::NearestEven)
            .unwrap()
            .convert(original, Rounding::NearestEven)
            .unwrap();
        assert_eq!(round_tripped.value(), 0); // 1000 tick error at 1/48000
        assert_error_within(
            t,
            round_tripped,
            half_tick_bound_seconds(&[original, coarse]),
        );

        // Same bound must hold across the audio x video matrix.
        let (audio, video) = precision_matrix_bases();
        for &from in audio.iter().chain(video.iter()) {
            for &via in audio.iter().chain(video.iter()) {
                let bound = half_tick_bound_seconds(&[from, via]);
                for v in [0_i64, 1, -1, 1_000_000, -999_999] {
                    let t = MediaTime::new(v, from);
                    let round_tripped = t
                        .convert(via, Rounding::NearestEven)
                        .unwrap()
                        .convert(from, Rounding::NearestEven)
                        .unwrap();
                    assert_error_within(t, round_tripped, bound);
                }
            }
        }
    }

    #[test]
    fn chained_conversion_error_is_bounded_by_per_hop_half_ticks() {
        // N-hop cycle: accumulated error <= sum over hops of 0.5 tick of
        // each destination base (in seconds). Contrast: a direct
        // anchor-to-destination conversion stays within 0.5 tick of the
        // destination base — the chained cost is the policy's price.
        let cycles: &[&[TimeBase]] = &[
            &[
                base(1, 48_000),
                base(1_001, 30_000),
                base(1, 25),
                base(1, 48_000),
            ],
            &[
                base(1, 44_100),
                base(1, 90_000),
                base(1_001, 60_000),
                base(1, 44_100),
            ],
            &[
                base(1, 48_000),
                base(1, 44_100),
                base(1, 24),
                base(1, 48_000),
            ],
        ];
        for cycle in cycles {
            let bound = half_tick_bound_seconds(&cycle[1..]);
            for v in [0_i64, 1, -1, 1_000_000, -1_000_000] {
                let start = MediaTime::new(v, cycle[0]);
                let mut t = start;
                for &to in &cycle[1..] {
                    t = t.convert(to, Rounding::NearestEven).unwrap();
                }
                assert_error_within(start, t, bound);
                // Control: the direct conversion is much tighter.
                let direct = start
                    .convert(*cycle.last().unwrap(), Rounding::NearestEven)
                    .unwrap();
                assert_error_within(
                    start,
                    direct,
                    half_tick_bound_seconds(&[*cycle.last().unwrap()]),
                );
            }
        }
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

    #[test]
    fn range_end_delta_wider_than_i64() {
        // Regression (Issue #41): the duration converts to a delta beyond
        // the `i64` range in `start`'s base, yet `start + delta` is
        // representable — `end()` must return the true instant, not the
        // sum of a saturated delta.
        //
        // delta = 2 * i64::MAX ticks; i64::MIN + 2 * i64::MAX == i64::MAX - 1.
        let range = MediaRange::new(
            MediaTime::new(i64::MIN, base(1, 1)),
            MediaDuration::new(i64::MAX, base(2, 1)),
        );
        assert_eq!(range.end(), MediaTime::new(i64::MAX - 1, base(1, 1)));

        // Mirror with a huge negative delta and a large positive start:
        // delta = 2 * (i64::MIN + 1) ticks; i64::MAX + delta == i64::MIN + 1.
        let mirror = MediaRange::new(
            MediaTime::new(i64::MAX, base(1, 1)),
            MediaDuration::new(i64::MIN + 1, base(2, 1)),
        );
        assert_eq!(mirror.end(), MediaTime::new(i64::MIN + 1, base(1, 1)));

        // Only a genuinely unrepresentable final value saturates.
        let high = MediaRange::new(
            MediaTime::new(i64::MAX, base(1, 1)),
            MediaDuration::new(i64::MAX, base(2, 1)),
        );
        assert_eq!(high.end().value(), i64::MAX);
        let low = MediaRange::new(
            MediaTime::new(i64::MIN, base(1, 1)),
            MediaDuration::new(i64::MIN, base(2, 1)),
        );
        assert_eq!(low.end().value(), i64::MIN);
    }

    #[test]
    fn checked_add_sub_delta_wider_than_i64() {
        // Same shape through checked_add: the converted delta overflows
        // `i64` but the final instant is representable, so the operation
        // must succeed instead of surfacing the conversion's Err.
        let start = MediaTime::new(i64::MIN, base(1, 1));
        let dur = MediaDuration::new(i64::MAX, base(2, 1));
        assert_eq!(
            start.checked_add(dur),
            Ok(MediaTime::new(i64::MAX - 1, base(1, 1)))
        );

        // Negative-duration mirror through both operations.
        let neg_dur = MediaDuration::new(i64::MIN + 1, base(2, 1));
        assert_eq!(
            MediaTime::new(i64::MAX, base(1, 1)).checked_add(neg_dur),
            Ok(MediaTime::new(i64::MIN + 1, base(1, 1)))
        );
        assert_eq!(
            start.checked_sub(neg_dur),
            Ok(MediaTime::new(i64::MAX - 1, base(1, 1)))
        );

        // An `i64::MIN` duration must not trip the negation inside sub:
        // -1 - i64::MIN == i64::MAX.
        assert_eq!(
            MediaTime::new(-1, base(1, 1)).checked_sub(MediaDuration::new(i64::MIN, base(1, 1))),
            Ok(MediaTime::new(i64::MAX, base(1, 1)))
        );

        // Truly unrepresentable finals still surface Err.
        assert_eq!(
            MediaTime::new(i64::MAX, base(1, 1)).checked_add(dur),
            Err(Error::TimeOverflow)
        );
        assert_eq!(start.checked_sub(dur), Err(Error::TimeOverflow));
    }

    #[test]
    fn checked_end_errors_only_on_final_overflow() {
        // Representable through an `i64`-overflowing delta: Ok.
        let range = MediaRange::new(
            MediaTime::new(i64::MIN, base(1, 1)),
            MediaDuration::new(i64::MAX, base(2, 1)),
        );
        assert_eq!(
            range.checked_end(Rounding::NearestEven),
            Ok(MediaTime::new(i64::MAX - 1, base(1, 1)))
        );

        // Final sum beyond i64: Err in the correct direction of signs.
        let high = MediaRange::new(
            MediaTime::new(i64::MAX, base(1, 1)),
            MediaDuration::new(i64::MAX, base(2, 1)),
        );
        assert_eq!(
            high.checked_end(Rounding::NearestEven),
            Err(Error::TimeOverflow)
        );
        let low = MediaRange::new(
            MediaTime::new(i64::MIN, base(1, 1)),
            MediaDuration::new(i64::MIN, base(2, 1)),
        );
        assert_eq!(
            low.checked_end(Rounding::NearestEven),
            Err(Error::TimeOverflow)
        );
    }

    #[test]
    fn checked_end_applies_each_rounding_mode_once() {
        // delta = 1.5 ticks in start's 1/1 base: each mode applies a
        // single rounding to the delta (same rules as `convert`).
        let start = MediaTime::new(10, base(1, 1));
        let one_and_half = MediaDuration::new(3, base(1, 2));
        let cases = [
            (Rounding::Floor, 11),
            (Rounding::Ceil, 12),
            (Rounding::TowardZero, 11),
            (Rounding::AwayFromZero, 12),
            // 1.5 -> 2: exact half, quotient 1 is odd -> rounds up.
            (Rounding::NearestEven, 12),
        ];
        for (mode, expected) in cases {
            assert_eq!(
                MediaRange::new(start, one_and_half).checked_end(mode),
                Ok(MediaTime::new(expected, base(1, 1))),
                "mode {mode:?}"
            );
        }

        // 2.5 -> NearestEven stays at 2 (even); AwayFromZero reaches 3.
        let two_and_half = MediaDuration::new(5, base(1, 2));
        assert_eq!(
            MediaRange::new(start, two_and_half).checked_end(Rounding::NearestEven),
            Ok(MediaTime::new(12, base(1, 1)))
        );
        assert_eq!(
            MediaRange::new(start, two_and_half).checked_end(Rounding::AwayFromZero),
            Ok(MediaTime::new(13, base(1, 1)))
        );

        // delta = -1.5 ticks: rounding stays symmetric for pre-roll.
        let neg = MediaDuration::new(-3, base(1, 2));
        let neg_cases = [
            (Rounding::Floor, 8),
            (Rounding::Ceil, 9),
            (Rounding::TowardZero, 9),
            (Rounding::AwayFromZero, 8),
            // -1.5 -> -2: exact half, quotient 1 is odd -> rounds away.
            (Rounding::NearestEven, 8),
        ];
        for (mode, expected) in neg_cases {
            assert_eq!(
                MediaRange::new(start, neg).checked_end(mode),
                Ok(MediaTime::new(expected, base(1, 1))),
                "mode {mode:?}"
            );
        }
    }

    #[test]
    fn negative_duration_moves_both_directions() {
        // A negative duration puts end before start.
        let backwards = MediaRange::new(
            MediaTime::new(4, base(1, 4)),
            MediaDuration::new(-1, base(1, 1)),
        );
        assert_eq!(backwards.end(), MediaTime::new(0, base(1, 4)));

        // checked_add of a negative duration moves backwards;
        // checked_sub of the same duration moves forwards.
        let t = MediaTime::new(4, base(1, 4));
        let neg = MediaDuration::new(-1, base(1, 1));
        assert_eq!(t.checked_add(neg), Ok(MediaTime::new(0, base(1, 4))));
        assert_eq!(t.checked_sub(neg), Ok(MediaTime::new(8, base(1, 4))));
    }

    #[test]
    fn checked_end_agrees_with_checked_add() {
        // `end` arithmetic and `checked_add` share the exact-delta
        // semantics: they agree on Ok *and* on Err for the same inputs.
        let starts = [
            MediaTime::new(0, base(1, 1)),
            MediaTime::new(i64::MIN, base(1, 1)),
            MediaTime::new(i64::MAX, base(1, 1)),
            MediaTime::new(10, base(1, 4)),
        ];
        let durations = [
            MediaDuration::new(1, base(1, 1)),
            MediaDuration::new(-1, base(1, 1)),
            MediaDuration::new(i64::MAX, base(2, 1)),
            MediaDuration::new(i64::MIN, base(1, 1)),
            MediaDuration::new(3, base(1, 2)),
        ];
        for s in starts {
            for d in durations {
                assert_eq!(
                    MediaRange::new(s, d).checked_end(Rounding::NearestEven),
                    s.checked_add(d),
                    "start={s:?} dur={d:?}"
                );
            }
        }
    }
}
