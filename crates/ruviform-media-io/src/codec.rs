//! Codec description at the media-I/O boundary: [`CodecId`] (the
//! Ruviform-managed codec identifier), [`CodecFormat`] (the decoded
//! stream-format mapping) and [`CodecDescriptor`] (the pairing of both
//! plus opaque codec-private data).
//!
//! [`ruviform_media_core::StreamDescriptor`] is deliberately
//! codec-agnostic; everything codec-specific lives here instead. A
//! [`CodecDescriptor`] answers three questions (spec AC11): *which*
//! codec ([`CodecId`] + canonical [`CodecId::name`]), *what
//! codec-specific parameters* the container carried (opaque
//! [`CodecDescriptor::codec_private`] bytes — extradata such as an AAC
//! `AudioSpecificConfig`, held verbatim and never parsed by this
//! crate), and *what format* decoded output takes
//! ([`CodecDescriptor::audio_format`] / [`CodecDescriptor::video_format`]
//! via [`CodecFormat`]).

use ruviform_audio_core::AudioFormat;
use ruviform_video_core::VideoFormat;

/// Ruviform-managed codec identifier.
///
/// The variant set is the registry: Ruviform assigns an ID to every
/// codec the boundary can name, independent of any backend's own ID
/// space (a backend maps its native IDs at the edge). The enum is
/// `#[non_exhaustive]` per the unified public-enum API policy — new
/// codecs are added as new variants without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — part of the media-I/O boundary
/// (`Copy` value type; the contract regulates the API context it is
/// used in, not the value itself).
///
/// # Examples
///
/// ```
/// use ruviform_media_io::CodecId;
///
/// assert_eq!(CodecId::Flac.name(), "flac");
/// assert_ne!(CodecId::Aac, CodecId::Mp3);
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum CodecId {
    /// No Ruviform ID assigned — the codec is unrecognized by (or not
    /// yet registered in) the Ruviform registry. The default: a
    /// descriptor built without probing still distinguishes "known
    /// codec" from "could not be identified".
    #[default]
    Other,
    /// Linear PCM family (integer and float encodings; the wire
    /// encoding detail lives in [`CodecDescriptor::codec_private`] and
    /// the decoded format in [`CodecFormat::Audio`]).
    Pcm,
    /// AAC (Advanced Audio Coding).
    Aac,
    /// MP3 (MPEG audio layer III).
    Mp3,
    /// Vorbis.
    Vorbis,
    /// Opus.
    Opus,
    /// FLAC (Free Lossless Audio Codec).
    Flac,
    /// ALAC (Apple Lossless Audio Codec).
    Alac,
    /// H.264 / AVC.
    H264,
    /// H.265 / HEVC.
    Hevc,
    /// VP9.
    Vp9,
    /// AV1.
    Av1,
}

impl CodecId {
    /// The canonical lowercase name of the codec — a stable, Ruviform-
    /// managed label (never a backend's native name).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_io::CodecId;
    ///
    /// assert_eq!(CodecId::Other.name(), "other");
    /// assert_eq!(CodecId::Pcm.name(), "pcm");
    /// ```
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Other => "other",
            Self::Pcm => "pcm",
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
            Self::Vorbis => "vorbis",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Alac => "alac",
            Self::H264 => "h264",
            Self::Hevc => "hevc",
            Self::Vp9 => "vp9",
            Self::Av1 => "av1",
        }
    }
}

/// The decoded stream format a codec carries — the mapping from a
/// [`CodecDescriptor`] to a domain format type.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — part of the media-I/O boundary.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
/// use ruviform_media_io::CodecFormat;
///
/// let format = CodecFormat::Audio(AudioFormat {
///     sample_rate: SampleRate::new(48_000).unwrap(),
///     layout: ChannelLayout::Stereo,
///     sample_format: SampleFormat::F32,
/// });
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CodecFormat {
    /// Decoded audio stream format (canonical planar `f32` target
    /// described by [`AudioFormat`]).
    Audio(AudioFormat),
    /// Decoded video stream format described by [`VideoFormat`].
    Video(VideoFormat),
    /// Non-A/V payload stream ([`StreamKind::Data`] — subtitles,
    /// metadata, …): no domain format mapping applies.
    ///
    /// [`StreamKind::Data`]: ruviform_media_core::StreamKind::Data
    Data,
}

/// Codec-level description of one stream: identification, private
/// parameters and the decoded-format mapping.
///
/// Paired with the codec-agnostic [`StreamDescriptor`] via
/// [`StreamInfo`](crate::StreamInfo); `StreamDescriptor` itself is
/// never made codec-aware.
///
/// [`CodecDescriptor::codec_private`] is an *opaque* byte string: this
/// crate defines the slot but never interprets its content — the
/// backend copies the container's codec-specific parameters verbatim
/// (e.g. codec init records, `AudioSpecificConfig`) and a decoder
/// implementation consumes them.
///
/// **Send contract:** `CodecDescriptor` is `Send` — stream metadata
/// moves with the objects that produce it across the thread boundary.
///
/// **RT contract:** non-RT only — owns a `Vec<u8>`; construction,
/// cloning and dropping may allocate.
///
/// [`StreamDescriptor`]: ruviform_media_core::StreamDescriptor
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
/// use ruviform_media_io::{CodecDescriptor, CodecFormat, CodecId};
///
/// let descriptor = CodecDescriptor {
///     codec: CodecId::Aac,
///     codec_private: vec![0x11, 0x90], // AudioSpecificConfig, held opaque.
///     format: CodecFormat::Audio(AudioFormat {
///         sample_rate: SampleRate::new(48_000).unwrap(),
///         layout: ChannelLayout::Stereo,
///         sample_format: SampleFormat::F32,
///     }),
/// };
/// assert_eq!(descriptor.codec, CodecId::Aac);
/// assert_eq!(descriptor.codec_private, [0x11, 0x90]);
/// assert_eq!(descriptor.audio_format().unwrap().sample_rate.get(), 48_000);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodecDescriptor {
    /// The Ruviform-managed codec identifier.
    pub codec: CodecId,
    /// Codec-private parameters (extradata), copied verbatim from the
    /// container — opaque to this crate; empty when the codec needs
    /// none.
    pub codec_private: Vec<u8>,
    /// The decoded stream-format mapping.
    pub format: CodecFormat,
}

impl CodecDescriptor {
    /// The decoded [`AudioFormat`] when this is an audio codec stream;
    /// `None` otherwise.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
    /// use ruviform_media_io::{CodecDescriptor, CodecFormat, CodecId};
    ///
    /// let format = AudioFormat {
    ///     sample_rate: SampleRate::new(44_100).unwrap(),
    ///     layout: ChannelLayout::Stereo,
    ///     sample_format: SampleFormat::F32,
    /// };
    /// let descriptor = CodecDescriptor {
    ///     codec: CodecId::Flac,
    ///     codec_private: Vec::new(),
    ///     format: CodecFormat::Audio(format),
    /// };
    /// assert_eq!(descriptor.audio_format(), Some(format));
    /// assert_eq!(descriptor.video_format(), None);
    /// ```
    #[must_use]
    pub const fn audio_format(&self) -> Option<AudioFormat> {
        match self.format {
            CodecFormat::Audio(format) => Some(format),
            _ => None,
        }
    }

    /// The decoded [`VideoFormat`] when this is a video codec stream;
    /// `None` otherwise.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn video_format(&self) -> Option<VideoFormat> {
        match self.format {
            CodecFormat::Video(format) => Some(format),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};
    use ruviform_video_core::{ColorInfo, PixelFormat};

    fn check_send<T: Send>() {}

    fn audio_descriptor() -> CodecDescriptor {
        CodecDescriptor {
            codec: CodecId::Aac,
            codec_private: vec![0x11, 0x90],
            format: CodecFormat::Audio(AudioFormat {
                sample_rate: SampleRate::new(48_000).unwrap(),
                layout: ChannelLayout::Stereo,
                sample_format: SampleFormat::F32,
            }),
        }
    }

    /// AC11 (1): codec identification — the Ruviform-managed ID and its
    /// canonical name are kept on the descriptor.
    #[test]
    fn codec_id_is_carried_with_name() {
        let descriptor = audio_descriptor();
        assert_eq!(descriptor.codec, CodecId::Aac);
        assert_eq!(descriptor.codec.name(), "aac");
        // `Other` keeps "unidentified" representable without panicking.
        assert_eq!(CodecId::Other.name(), "other");
        // Names cover every registered variant and stay non-empty.
        for codec in [
            CodecId::Other,
            CodecId::Pcm,
            CodecId::Aac,
            CodecId::Mp3,
            CodecId::Vorbis,
            CodecId::Opus,
            CodecId::Flac,
            CodecId::Alac,
            CodecId::H264,
            CodecId::Hevc,
            CodecId::Vp9,
            CodecId::Av1,
        ] {
            assert_ne!(codec.name(), "");
        }
    }

    /// AC11 (2): codec-private parameters are held as opaque bytes —
    /// stored verbatim, readable back verbatim.
    #[test]
    fn private_bytes_are_kept_opaque() {
        let descriptor = audio_descriptor();
        assert_eq!(descriptor.codec_private, [0x11, 0x90]);
        let mut empty = descriptor.clone();
        empty.codec_private.clear();
        assert_eq!(empty.codec_private, Vec::<u8>::new());
    }

    /// AC11 (3): the format mapping yields `AudioFormat` / `VideoFormat`
    /// on the matching variant and `None` on the others — including a
    /// codec the PCM `PoC` path would never exercise.
    #[test]
    fn format_mapping_yields_domain_formats() {
        let audio = AudioFormat {
            sample_rate: SampleRate::new(96_000).unwrap(),
            layout: ChannelLayout::FivePointOne,
            sample_format: SampleFormat::F64,
        };
        let audio_side = CodecDescriptor {
            codec: CodecId::Opus,
            codec_private: vec![0x01],
            format: CodecFormat::Audio(audio),
        };
        assert_eq!(audio_side.audio_format(), Some(audio));
        assert_eq!(audio_side.video_format(), None);

        let video = VideoFormat {
            width: 1_920,
            height: 1_080,
            format: PixelFormat::Yuv420p,
            color: ColorInfo::default(),
        };
        let video_side = CodecDescriptor {
            codec: CodecId::Av1,
            codec_private: Vec::new(),
            format: CodecFormat::Video(video),
        };
        assert_eq!(video_side.video_format(), Some(video));
        assert_eq!(video_side.audio_format(), None);

        let data_side = CodecDescriptor {
            codec: CodecId::Other,
            codec_private: Vec::new(),
            format: CodecFormat::Data,
        };
        assert_eq!(data_side.audio_format(), None);
        assert_eq!(data_side.video_format(), None);
    }

    #[test]
    fn codec_send_bounds() {
        check_send::<CodecId>();
        check_send::<CodecFormat>();
        check_send::<CodecDescriptor>();
    }
}
