//! Audio time contract: an integer [`SampleIndex`] paired with a
//! [`SampleRate`] (timebase `1/rate`) locates audio on the media timeline
//! exactly — no `f64` anywhere.
//!
//! The audio sample clock is the A/V master: audio↔video conversion is
//! expressed in sample indices at the audio rate and converted via exact
//! [`MediaTime`] rational arithmetic (`1/rate` ↔ the foreign timebase).

use core::cmp::Ordering;
use core::fmt;

use ruviform_media_core::{Error, MediaTime, Result, Rounding, TimeBase};

/// Nominal sample rate in Hz (`44_100`, `48_000`, `96_000`, …).
///
/// Stored as its [`TimeBase`] `1/rate`, so the `rate > 0` invariant is
/// enforced structurally at construction — a sample rate *is* a
/// non-degenerate timebase.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SampleRate;
/// use ruviform_media_core::Error;
///
/// let rate = SampleRate::new(48_000).unwrap();
/// assert_eq!(rate.get(), 48_000);
/// assert_eq!(SampleRate::new(0), Err(Error::InvalidValue("sample_rate")));
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SampleRate(TimeBase);

impl SampleRate {
    /// Creates a sample rate.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `rate == 0`.
    pub const fn new(rate: u32) -> Result<Self> {
        match TimeBase::new(1, rate) {
            Ok(base) => Ok(Self(base)),
            // `TimeBase::new` fails only on a zero denominator.
            Err(_) => Err(Error::InvalidValue("sample_rate")),
        }
    }

    /// The rate in Hz.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.denominator()
    }

    /// The rational [`TimeBase`] `1/rate` carried by sample-index times.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::SampleRate;
    /// use ruviform_media_core::TimeBase;
    ///
    /// let rate = SampleRate::new(48_000).unwrap();
    /// let base = rate.time_base();
    /// assert_eq!((base.numerator(), base.denominator()), (1, 48_000));
    /// ```
    #[must_use]
    pub const fn time_base(self) -> TimeBase {
        self.0
    }
}

/// Compact `SampleRate(48000)`-style output rather than the internal
/// `TimeBase` fields.
impl fmt::Debug for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SampleRate").field(&self.get()).finish()
    }
}

impl PartialOrd for SampleRate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Rates order by Hz — `44_100 < 48_000`.
impl Ord for SampleRate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

/// A signed index into the audio sample timeline.
///
/// Signed to allow pre-roll positions before media time zero — parity
/// with [`MediaTime`], which permits negative values for the same reason.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{SampleIndex, SampleRate};
/// use ruviform_media_core::MediaTime;
///
/// let rate = SampleRate::new(48_000).unwrap();
/// // One second of audio at 48 kHz is sample index 48_000.
/// assert_eq!(
///     SampleIndex(48_000).to_media_time(rate),
///     MediaTime::new(1, ruviform_media_core::TimeBase::new(1, 1).unwrap()),
/// );
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SampleIndex(pub i64);

impl SampleIndex {
    /// Converts to [`MediaTime`] in base `1/rate` — always exact, so no
    /// rounding mode is needed.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    #[must_use]
    pub fn to_media_time(self, rate: SampleRate) -> MediaTime {
        MediaTime::new(self.0, rate.time_base())
    }

    /// Converts a [`MediaTime`] into a sample index at `rate` via exact
    /// rational arithmetic — exact when the instant lands on a sample
    /// boundary, rounded per `rounding` otherwise.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the conversion overflows the `i64`
    /// sample range.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{SampleIndex, SampleRate};
    /// use ruviform_media_core::{MediaTime, Rounding, TimeBase};
    ///
    /// let rate = SampleRate::new(48_000).unwrap();
    /// // 1 tick of the 90 kHz container timebase is 8/15 of a 48 kHz
    /// // sample — not representable, so the rounding mode applies.
    /// let tick = MediaTime::new(1, TimeBase::COMMON_90K);
    /// assert_eq!(
    ///     SampleIndex::from_media_time(tick, rate, Rounding::Floor),
    ///     Ok(SampleIndex(0))
    /// );
    /// assert_eq!(
    ///     SampleIndex::from_media_time(tick, rate, Rounding::Ceil),
    ///     Ok(SampleIndex(1))
    /// );
    /// ```
    pub fn from_media_time(time: MediaTime, rate: SampleRate, rounding: Rounding) -> Result<Self> {
        let converted = time.convert(rate.time_base(), rounding)?;
        Ok(Self(converted.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(numerator: u32, denominator: u32) -> TimeBase {
        TimeBase::new(numerator, denominator).unwrap()
    }

    #[test]
    fn sample_index_to_media_time_is_exact() {
        let rate = SampleRate::new(48_000).unwrap();
        // Cross-base equality: 48_000 samples at 1/48_000 == 1 second.
        assert_eq!(
            SampleIndex(48_000).to_media_time(rate),
            MediaTime::new(1, base(1, 1))
        );
        // Negative indices are legal (pre-roll parity with MediaTime).
        assert_eq!(
            SampleIndex(-48_000).to_media_time(rate),
            MediaTime::new(-1, base(1, 1))
        );
    }

    #[test]
    fn media_time_to_sample_index_exact_at_44100() {
        let rate = SampleRate::new(44_100).unwrap();
        let two_seconds = MediaTime::new(2, base(1, 1));
        assert_eq!(
            SampleIndex::from_media_time(two_seconds, rate, Rounding::NearestEven),
            Ok(SampleIndex(88_200))
        );
    }

    #[test]
    fn media_time_to_sample_index_rounds_per_mode() {
        let rate = SampleRate::new(48_000).unwrap();
        // Half a sample — exercises NearestEven's round-half-to-even both ways.
        let half = MediaTime::new(1, base(1, 96_000));
        assert_eq!(
            SampleIndex::from_media_time(half, rate, Rounding::NearestEven),
            Ok(SampleIndex(0)) // 0.5 → 0 (even)
        );
        let three_halves = MediaTime::new(3, base(1, 96_000));
        assert_eq!(
            SampleIndex::from_media_time(three_halves, rate, Rounding::NearestEven),
            Ok(SampleIndex(2)) // 1.5 → 2 (even)
        );
        assert_eq!(
            SampleIndex::from_media_time(half, rate, Rounding::Ceil),
            Ok(SampleIndex(1))
        );
    }

    #[test]
    fn media_time_to_sample_index_negative_rounding() {
        let rate = SampleRate::new(48_000).unwrap();
        let minus_one_tick = MediaTime::new(-1, TimeBase::COMMON_90K); // ≈ −0.53 samples
        assert_eq!(
            SampleIndex::from_media_time(minus_one_tick, rate, Rounding::Floor),
            Ok(SampleIndex(-1))
        );
        assert_eq!(
            SampleIndex::from_media_time(minus_one_tick, rate, Rounding::TowardZero),
            Ok(SampleIndex(0))
        );
        assert_eq!(
            SampleIndex::from_media_time(minus_one_tick, rate, Rounding::AwayFromZero),
            Ok(SampleIndex(-1))
        );
    }

    #[test]
    fn media_time_to_sample_index_overflow_is_error() {
        let rate = SampleRate::new(48_000).unwrap();
        let huge = MediaTime::new(i64::MAX, base(1, 1));
        assert_eq!(
            SampleIndex::from_media_time(huge, rate, Rounding::NearestEven),
            Err(Error::TimeOverflow)
        );
    }

    #[test]
    fn sample_index_media_time_roundtrip() {
        let rate = SampleRate::new(44_100).unwrap();
        let index = SampleIndex(123_456);
        let back =
            SampleIndex::from_media_time(index.to_media_time(rate), rate, Rounding::NearestEven);
        assert_eq!(back, Ok(index));
    }
}
