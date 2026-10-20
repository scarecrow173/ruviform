//! Stream-level audio format description: [`AudioFormat`] pairs a
//! [`SampleRate`], a [`ChannelLayout`] (which carries the channel count)
//! and a [`SampleFormat`].

use crate::layout::ChannelLayout;
use crate::time::SampleRate;

/// Stream-format description of a PCM sample encoding.
///
/// `F32` is the canonical RT processing format; `F64` exists only because
/// the design baseline permits a sample type parameter. The enum is
/// `#[non_exhaustive]` — integer formats may be added later.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SampleFormat;
///
/// assert_eq!(SampleFormat::default(), SampleFormat::F32);
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
