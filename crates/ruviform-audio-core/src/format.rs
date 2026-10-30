//! Stream-level audio format description: [`AudioFormat`] pairs a
//! [`SampleRate`], a [`ChannelLayout`] (which carries the channel count)
//! and a [`SampleFormat`].

use crate::layout::ChannelLayout;
use crate::time::SampleRate;

/// Stream-format description of a PCM sample encoding.
///
/// `F32` is the canonical RT processing format; `F64` exists only because
/// the design baseline permits a sample type parameter. The integer
/// variants exist so *native* (non-canonical) stream formats can be
/// described at the media-I/O boundary — `AudioBuffer` itself stays
/// `f32`/`f64`-only. The enum is `#[non_exhaustive]` — further encodings
/// (e.g. companded PCM) may be added later.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SampleFormat;
///
/// assert_eq!(SampleFormat::default(), SampleFormat::F32);
/// assert_eq!(SampleFormat::S24.bits_per_sample(), 24);
/// assert!(SampleFormat::S16.is_integer());
/// assert!(SampleFormat::F32.is_float());
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    /// 32-bit IEEE float — the canonical RT processing format.
    #[default]
    F32,
    /// 64-bit IEEE float — permitted type parameter, no further
    /// machinery.
    F64,
    /// Unsigned 8-bit integer PCM.
    U8,
    /// Signed 8-bit integer PCM.
    S8,
    /// Unsigned 16-bit integer PCM.
    U16,
    /// Signed 16-bit integer PCM.
    S16,
    /// Unsigned 24-bit integer PCM.
    U24,
    /// Signed 24-bit integer PCM.
    S24,
    /// Unsigned 32-bit integer PCM.
    U32,
    /// Signed 32-bit integer PCM.
    S32,
}

impl SampleFormat {
    /// Storage width of one sample in bits.
    ///
    /// This is the *storage* width of the encoding; a container may
    /// declare fewer *valid* bits (e.g. 20 valid bits in 24-bit storage)
    /// — that distinction belongs to the stream's declared facts, not to
    /// this encoding.
    ///
    /// **RT contract:** RT-suitable — `Copy`, allocation-free.
    #[must_use]
    pub const fn bits_per_sample(self) -> u16 {
        match self {
            Self::U8 | Self::S8 => 8,
            Self::U16 | Self::S16 => 16,
            Self::U24 | Self::S24 => 24,
            Self::U32 | Self::S32 | Self::F32 => 32,
            Self::F64 => 64,
        }
    }

    /// `true` for the floating-point encodings (`F32`, `F64`).
    ///
    /// **RT contract:** RT-suitable — `Copy`, allocation-free.
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// `true` for the integer encodings (`U8`…`S32`).
    ///
    /// **RT contract:** RT-suitable — `Copy`, allocation-free.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            Self::U8
                | Self::S8
                | Self::U16
                | Self::S16
                | Self::U24
                | Self::S24
                | Self::U32
                | Self::S32
        )
    }

    /// `true` for unsigned integer encodings (`U8`, `U16`, `U24`,
    /// `U32`). Decoded audio convention stores these as signed-offset
    /// values (wire value minus half the range); `false` for signed
    /// integer and float encodings.
    ///
    /// **RT contract:** RT-suitable — `Copy`, allocation-free.
    #[must_use]
    pub const fn is_unsigned(self) -> bool {
        matches!(self, Self::U8 | Self::U16 | Self::U24 | Self::U32)
    }
}

/// Stream-level audio format: rate + layout + sample encoding.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
///
/// let format = AudioFormat {
///     sample_rate: SampleRate::new(48_000).unwrap(),
///     layout: ChannelLayout::Stereo,
///     sample_format: SampleFormat::F32,
/// };
/// assert_eq!(format.layout.channels(), 2);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// Nominal sample rate.
    pub sample_rate: SampleRate,
    /// Channel layout — owns the channel count.
    pub layout: ChannelLayout,
    /// PCM sample encoding.
    pub sample_format: SampleFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_format_default_is_f32() {
        assert_eq!(SampleFormat::default(), SampleFormat::F32);
    }

    #[test]
    fn integer_encodings_report_width_and_sign() {
        for (format, bits, unsigned) in [
            (SampleFormat::U8, 8, true),
            (SampleFormat::S8, 8, false),
            (SampleFormat::U16, 16, true),
            (SampleFormat::S16, 16, false),
            (SampleFormat::U24, 24, true),
            (SampleFormat::S24, 24, false),
            (SampleFormat::U32, 32, true),
            (SampleFormat::S32, 32, false),
            (SampleFormat::F32, 32, false),
            (SampleFormat::F64, 64, false),
        ] {
            assert_eq!(format.bits_per_sample(), bits, "{format:?}");
            assert_eq!(format.is_unsigned(), unsigned, "{format:?}");
            assert_eq!(format.is_integer(), !format.is_float(), "{format:?}");
        }
        assert!(SampleFormat::S16.is_integer());
        assert!(!SampleFormat::S16.is_float());
        assert!(SampleFormat::F64.is_float());
        assert!(!SampleFormat::F64.is_integer());
    }

    /// The exactness invariant behind the non-destructive PCM contract:
    /// every ≤32-bit integer encoding value is exactly representable as
    /// `f64`, so `f64` storage never loses integer PCM precision.
    ///
    /// The `as` casts here are the contract under test — the lint
    /// allowances are intentional.
    #[test]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn all_integer_encoding_values_are_exact_in_f64() {
        for (min, max) in [
            (i64::from(i8::MIN), i64::from(i8::MAX)),
            (i64::from(i16::MIN), i64::from(i16::MAX)),
            (-8_388_608, 8_388_607), // s24
            (i64::from(i32::MIN), i64::from(i32::MAX)),
        ] {
            for value in [min, max] {
                let widened = value as f64;
                assert_eq!(widened as i64, value, "{value} must round-trip through f64");
            }
        }
        for max in [
            u64::from(u8::MAX),
            u64::from(u16::MAX),
            16_777_215,
            u64::from(u32::MAX),
        ] {
            let widened = max as f64;
            assert_eq!(widened as u64, max, "{max} must round-trip through f64");
        }
    }

    #[test]
    fn audio_format_is_a_copy_value_type() {
        let format = AudioFormat {
            sample_rate: SampleRate::new(96_000).unwrap(),
            layout: ChannelLayout::Mono,
            sample_format: SampleFormat::F64,
        };
        let copy = format; // `Copy` — the original stays usable.
        assert_eq!(format, copy);
        assert_eq!(format.sample_rate.get(), 96_000);
        assert_eq!(format.layout.channels(), 1);
        assert_eq!(format.sample_format, SampleFormat::F64);
    }
}
