//! Display-side [`Timecode`] conversion: SMPTE labels derived from
//! [`MediaTime`], including NTSC drop-frame numbering.
//!
//! A timecode is a *labeling rule*, never a timing source: it is computed
//! from a [`MediaTime`] for display and sanity checks. Drop-frame
//! numbering skips labels at the top of each minute (except multiples of
//! ten); it does not skip media frames. All math is integer
//! (`i128`/`u128`); `f64` never appears here.

use crate::error::{Error, Result};
use crate::time::{FrameRate, MediaTime};

/// A SMPTE-style timecode label: `HH:MM:SS:FF`, plus the drop-frame flag.
///
/// Produced from a [`MediaTime`] by [`Timecode::from_media_time`] (non-drop
/// labeling) or [`Timecode::from_media_time_drop`] (NTSC drop-frame
/// labeling, `30000/1001` or `60000/1001` only). `hours` is unbounded —
/// labels are not wrapped at 24 h.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Timecode {
    hours: u32,
    minutes: u32,
    seconds: u32,
    frames: u32,
    drop_frame: bool,
}

impl Timecode {
    /// Builds a non-drop timecode label for `t` counted on the
    /// `frame_rate` grid.
    ///
    /// The frame count is `floor(t * frame_rate)` computed exactly. Labels
    /// decompose against the nominal (rounded-integer) rate — for an
    /// NTSC-family rate this produces non-drop labels that drift against
    /// the clock; use [`Timecode::from_media_time_drop`] for SMPTE
    /// drop-frame labeling.
    ///
    /// A negative [`MediaTime`] (pre-roll) has no timecode label and is
    /// rejected with [`Error::InvalidValue`] rather than silently clamped.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidValue`] — negative `t`, or a `frame_rate` that
    ///   rounds to zero and cannot form a label grid.
    /// * [`Error::TimeOverflow`] — `i128` intermediate or `hours` range
    ///   overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{FrameRate, MediaTime, Timecode};
    ///
    /// let rate = FrameRate::from_fps(25).unwrap();
    /// let frame_base = rate.frame_duration();
    /// // frame 90_000 @ 25 fps == exactly one hour of labels.
    /// let tc = Timecode::from_media_time(MediaTime::new(90_000, frame_base), rate).unwrap();
    /// assert_eq!((tc.hours(), tc.minutes(), tc.seconds(), tc.frames()), (1, 0, 0, 0));
    /// assert!(!tc.is_drop_frame());
    /// ```
    pub fn from_media_time(t: MediaTime, frame_rate: FrameRate) -> Result<Self> {
        // Nominal label grid: the integer rate nearest to `frame_rate`,
        // rounded half-up. A rate rounding to zero is no usable grid.
        let nominal = u128::from(frame_rate.nominal_fps());
        if nominal == 0 {
            return Err(Error::InvalidValue("frame_rate"));
        }

        let frames_total = elapsed_frames(t, frame_rate)?;
        let frames = frames_total % nominal;
        let total_seconds = frames_total / nominal;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = u32::try_from(total_minutes / 60)
            .ok()
            .ok_or(Error::TimeOverflow)?;

        Ok(Self {
            hours,
            minutes: narrow_u32(minutes),
            seconds: narrow_u32(seconds),
            frames: narrow_u32(frames),
            drop_frame: false,
        })
    }

    /// Builds a SMPTE drop-frame timecode label for `t` on an NTSC-family
    /// `frame_rate` (`30000/1001` ≈ 29.97 fps or `60000/1001` ≈ 59.94 fps).
    ///
    /// Standard NTSC drop-frame numbering on the nominal grid (30 or 60):
    /// frame labels `00..=01` (29.97) or `00..=03` (59.94) are skipped at
    /// the top of every minute except multiples of ten. Only labels are
    /// skipped — the underlying frame count `floor(t * frame_rate)` is
    /// exact.
    ///
    /// A negative [`MediaTime`] (pre-roll) has no timecode label and is
    /// rejected with [`Error::InvalidValue`] rather than silently clamped.
    ///
    /// **RT contract:** RT-suitable — pure integer math, no allocation.
    ///
    /// # Errors
    ///
    /// * [`Error::UnsupportedDropFrameRate`] — `frame_rate` is not
    ///   `30000/1001` or `60000/1001` (after normalization).
    /// * [`Error::InvalidValue`] — negative `t`.
    /// * [`Error::TimeOverflow`] — `i128` intermediate or `hours` range
    ///   overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{FrameRate, MediaTime, Timecode};
    ///
    /// let rate = FrameRate::new(30_000, 1_001).unwrap();
    /// let frame_base = rate.frame_duration();
    /// // Frame 1_800 is the first frame of minute 1: labels 00/01 are
    /// // dropped, so it reads 00:01:00:02.
    /// let tc = Timecode::from_media_time_drop(MediaTime::new(1_800, frame_base), rate).unwrap();
    /// assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (1, 0, 2));
    /// assert!(tc.is_drop_frame());
    /// ```
    pub fn from_media_time_drop(t: MediaTime, frame_rate: FrameRate) -> Result<Self> {
        // SMPTE drop-frame labeling exists only for the NTSC-family rates:
        // (nominal label rate, labels dropped per skipped minute).
        let (nominal, dropped) = match (frame_rate.numerator(), frame_rate.denominator()) {
            (30_000, 1_001) => (30_u128, 2_u128),
            (60_000, 1_001) => (60, 4),
            _ => return Err(Error::UnsupportedDropFrameRate),
        };

        let frames_total = elapsed_frames(t, frame_rate)?;
        let frames_per_minute = nominal * 60 - dropped;
        let frames_per_10min = nominal * 600 - dropped * 9;
        let frames_per_hour = frames_per_10min * 6;

        let hours = u32::try_from(frames_total / frames_per_hour)
            .ok()
            .ok_or(Error::TimeOverflow)?;
        let mut rem = frames_total % frames_per_hour;
        let tens = rem / frames_per_10min;
        rem %= frames_per_10min;
        // Inside a ten-minute block, the first minute carries
        // `nominal * 60` labels; the following nine minutes each skip the
        // first `dropped` labels.
        let (units, inner) = if rem < nominal * 60 {
            (0_u128, rem)
        } else {
            let past_first_minute = rem - nominal * 60;
            (
                1 + past_first_minute / frames_per_minute,
                past_first_minute % frames_per_minute + dropped,
            )
        };
        let minutes = tens * 10 + units;
        let seconds = inner / nominal;
        let frames = inner % nominal;

        Ok(Self {
            hours,
            minutes: narrow_u32(minutes),
            seconds: narrow_u32(seconds),
            frames: narrow_u32(frames),
            drop_frame: true,
        })
    }

    /// Hours field of the label (unbounded; not wrapped at 24 h).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn hours(self) -> u32 {
        self.hours
    }

    /// Minutes field of the label (`0..=59`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn minutes(self) -> u32 {
        self.minutes
    }

    /// Seconds field of the label (`0..=59`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        self.seconds
    }

    /// Frames field of the label (`0..nominal rate - 1`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frames(self) -> u32 {
        self.frames
    }

    /// Whether this label uses SMPTE drop-frame numbering.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_drop_frame(self) -> bool {
        self.drop_frame
    }
}

/// Exact elapsed frame count `floor(t * frame_rate)` for a non-negative
/// [`MediaTime`], in `i128`/`u128` integer math.
fn elapsed_frames(t: MediaTime, rate: FrameRate) -> Result<u128> {
    if t.value() < 0 {
        // Negative (pre-roll) media time has no SMPTE label; reported as
        // an error rather than silently clamped.
        return Err(Error::InvalidValue("negative time"));
    }
    let num = i128::from(t.value())
        .checked_mul(i128::from(t.base().numerator()))
        .and_then(|n| n.checked_mul(i128::from(rate.numerator())))
        .ok_or(Error::TimeOverflow)?;
    let den = u128::from(t.base().denominator()) * u128::from(rate.denominator());
    let num = u128::try_from(num).ok().ok_or(Error::TimeOverflow)?;
    Ok(num / den)
}

/// Narrows a decomposed label field to `u32`. Every caller bounds `x`
/// below `2^32` by construction (`minutes`/`seconds` < 60, `frames` <
/// nominal rate), so the fallback is unreachable.
fn narrow_u32(x: u128) -> u32 {
    debug_assert!(x <= u128::from(u32::MAX));
    u32::try_from(x).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeBase;

    fn base(numerator: u32, denominator: u32) -> TimeBase {
        TimeBase::new(numerator, denominator).unwrap()
    }

    fn rate(numerator: u32, denominator: u32) -> FrameRate {
        FrameRate::new(numerator, denominator).unwrap()
    }

    #[test]
    fn non_drop_labels() {
        let rate = rate(25, 1);
        let frame_base = rate.frame_duration();
        let tc = Timecode::from_media_time(MediaTime::new(90_000, frame_base), rate).unwrap();
        assert_eq!(
            (tc.hours(), tc.minutes(), tc.seconds(), tc.frames()),
            (1, 0, 0, 0)
        );
        assert!(!tc.is_drop_frame());

        // Frame 26 -> 00:00:01:01.
        let tc = Timecode::from_media_time(MediaTime::new(26, frame_base), rate).unwrap();
        assert_eq!((tc.seconds(), tc.frames()), (1, 1));

        // A fractional media time floors onto the frame grid: 1.04 s at
        // 25 fps is frame 26 exactly.
        let sec_base = base(1, 1);
        let tc = Timecode::from_media_time(MediaTime::new(1, sec_base), rate).unwrap();
        assert_eq!((tc.seconds(), tc.frames()), (1, 0));
    }

    #[test]
    fn drop_labels_2997() {
        let rate = rate(30_000, 1_001);
        let frame_base = rate.frame_duration();
        let label = |frames: i64| {
            Timecode::from_media_time_drop(MediaTime::new(frames, frame_base), rate).unwrap()
        };

        assert_eq!(
            (
                label(0).hours(),
                label(0).minutes(),
                label(0).seconds(),
                label(0).frames()
            ),
            (0, 0, 0, 0)
        );
        assert!(label(0).is_drop_frame());
        // Last frame of minute 0 -> 00:00:59:29.
        let tc = label(1_799);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (0, 59, 29));
        // First frame of minute 1 -> 00:01:00:02 (labels 00/01 dropped).
        let tc = label(1_800);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (1, 0, 2));
        // Minute 9 boundary: labels dropped -> 00:09:00:02.
        // (1_800 + 8 * 1_798 real frames into the block.)
        let tc = label(16_184);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (9, 0, 2));
        // Ten-minute boundary: no drop -> 00:10:00:00.
        let tc = label(17_982);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (10, 0, 0));
        // One full hour of labels -> 01:00:00:00.
        let tc = label(107_892);
        assert_eq!(
            (tc.hours(), tc.minutes(), tc.seconds(), tc.frames()),
            (1, 0, 0, 0)
        );
    }

    #[test]
    fn drop_labels_5994() {
        let rate = rate(60_000, 1_001);
        let frame_base = rate.frame_duration();
        let label = |frames: i64| {
            Timecode::from_media_time_drop(MediaTime::new(frames, frame_base), rate).unwrap()
        };

        // Last frame of minute 0 -> 00:00:59:59.
        let tc = label(3_599);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (0, 59, 59));
        // First frame of minute 1 -> 00:01:00:04 (labels 00..03 dropped).
        let tc = label(3_600);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (1, 0, 4));
        // Ten-minute boundary: frames_per_10min = 36_000 - 36 = 35_964.
        let tc = label(35_964);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (10, 0, 0));
        // One full hour: 35_964 * 6 = 215_784 -> 01:00:00:00.
        let tc = label(215_784);
        assert_eq!(
            (tc.hours(), tc.minutes(), tc.seconds(), tc.frames()),
            (1, 0, 0, 0)
        );
    }

    #[test]
    fn drop_requires_ntsc_rate() {
        let t = MediaTime::ZERO;
        assert_eq!(
            Timecode::from_media_time_drop(t, rate(25, 1)),
            Err(Error::UnsupportedDropFrameRate)
        );
        assert_eq!(
            Timecode::from_media_time_drop(t, rate(24, 1)),
            Err(Error::UnsupportedDropFrameRate)
        );
        // A non-normalized spelling normalizes to 30000/1001 and works.
        let tc = Timecode::from_media_time_drop(t, rate(60_000, 2_002)).unwrap();
        assert!(tc.is_drop_frame());
    }

    #[test]
    fn negative_time_is_rejected() {
        let neg = MediaTime::new(-1, base(1, 1));
        assert_eq!(
            Timecode::from_media_time(neg, rate(25, 1)),
            Err(Error::InvalidValue("negative time"))
        );
        assert_eq!(
            Timecode::from_media_time_drop(neg, rate(30_000, 1_001)),
            Err(Error::InvalidValue("negative time"))
        );
    }

    #[test]
    fn sub_fps_rate_is_rejected() {
        // A rate rounding to zero cannot form a label grid.
        assert_eq!(
            Timecode::from_media_time(MediaTime::ZERO, rate(1, 1_000)),
            Err(Error::InvalidValue("frame_rate"))
        );
    }
}
