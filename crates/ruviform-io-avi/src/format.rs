//! AVI stream-header (`strh`) and stream-format (`strf`) body parsing.
//!
//! Two `strf` shapes are owned here: `WAVEFORMATEX` for `auds` streams
//! (the container's codec + PCM layout declaration) and
//! `BITMAPINFOHEADER` for `vids` streams (DIB compression fourCC + pixel
//! layout). Each parser classifies the body the boundary's `Fact`
//! vocabulary needs — *decodable* vs *unsupported* vs *malformed* — so
//! the demuxer can report honest per-stream capability instead of
//! silently succeeding on inputs it cannot decode.

use ruviform_audio_core::{ChannelLayout, SampleFormat, SpeakerMask};
use ruviform_media_io::CodecId;

/// `AVISTREAMHEADER` (`strh`) fields the demuxer uses. Bodies shorter
/// than `STRH_MIN` are malformed — `dwSampleSize` ends at offset 48.
#[derive(Copy, Clone, Debug)]
pub(crate) struct StreamHead {
    /// `fccType` — `vids` / `auds` / other.
    pub fcc_type: [u8; 4],
    /// `dwScale` — ticks per unit of `dwRate`; the stream tick length.
    pub scale: u32,
    /// `dwRate` — units per second; `scale/rate` seconds per tick.
    pub rate: u32,
    /// `dwStart` — stream clock origin in `scale`-tick units.
    pub start: u32,
    /// `dwLength` — declared unit count (video frames / audio sample
    /// blocks).
    pub length: u32,
    /// `dwSampleSize` — bytes per audio sample block (0 for video).
    pub sample_size: u32,
}

/// Minimum `strh` body length — through `dwSampleSize`.
const STRH_MIN: usize = 48;

/// Parses an `AVISTREAMHEADER` body. `None` when the body is shorter
/// than the fixed header — the malformed case the demuxer reports.
pub(crate) fn stream_head(body: &[u8]) -> Option<StreamHead> {
    if body.len() < STRH_MIN {
        return None;
    }
    let u32_at = |off: usize| u32::from_le_bytes(body[off..off + 4].try_into().unwrap());
    Some(StreamHead {
        fcc_type: body[0..4].try_into().unwrap(),
        scale: u32_at(20),
        rate: u32_at(24),
        start: u32_at(28),
        length: u32_at(32),
        sample_size: u32_at(44),
    })
}

/// `WAVEFORMATEX` PCM decode parameters — the fields needed to decode
/// interleaved PCM bodies plus the native [`SampleFormat`].
#[derive(Copy, Clone, Debug)]
pub(crate) struct WavePcm {
    /// Container-declared channel count.
    pub channels: u16,
    /// Boundary speaker layout for the declared channel count.
    pub layout: ChannelLayout,
    /// `nSamplesPerSec`.
    pub rate: u32,
    /// `nBlockAlign` — bytes per channel-interleaved sample frame.
    pub block_align: u16,
    /// Native sample encoding — `U8`/`S16`/`S24`/`S32` or `F32`/`F64`.
    pub sample: SampleFormat,
}

/// `strf` (audio) classification: what the demuxer/decoder may claim.
#[derive(Copy, Clone, Debug)]
pub(crate) enum WaveClass {
    /// Layout-consistent PCM the decoder supports (tag 1 + 8/16/24/32
    /// bits, or tag 3 + 32/64 bits).
    Pcm(WavePcm),
    /// A syntactically valid `WAVEFORMATEX` declaring an encoding this
    /// backend cannot decode (non-PCM tag, or a PCM bit depth outside
    /// the supported set — e.g. `WAVE_FORMAT_EXTENSIBLE`, ADPCM, 12-bit).
    Unsupported,
    /// Structurally inconsistent body — too short, zero channels/rate,
    /// or `nBlockAlign` not equal to `channels * bytes_per_sample`.
    Malformed,
}

/// WAVE format tags (values of `wFormatTag`).
const TAG_PCM: u16 = 0x0001;
const TAG_IEEE_FLOAT: u16 = 0x0003;

/// Parses a `WAVEFORMATEX` body (the `auds` `strf` chunk).
pub(crate) fn wave_format(body: &[u8]) -> WaveClass {
    if body.len() < 16 {
        return WaveClass::Malformed;
    }
    let tag = u16::from_le_bytes(body[0..2].try_into().unwrap());
    let channels = u16::from_le_bytes(body[2..4].try_into().unwrap());
    let rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
    let block_align = u16::from_le_bytes(body[12..14].try_into().unwrap());
    let bits = u16::from_le_bytes(body[14..16].try_into().unwrap());
    if channels == 0 || rate == 0 {
        return WaveClass::Malformed;
    }
    let sample = match (tag, bits) {
        (TAG_PCM, 8) => SampleFormat::U8,
        (TAG_PCM, 16) => SampleFormat::S16,
        (TAG_PCM, 24) => SampleFormat::S24,
        (TAG_PCM, 32) => SampleFormat::S32,
        (TAG_IEEE_FLOAT, 32) => SampleFormat::F32,
        (TAG_IEEE_FLOAT, 64) => SampleFormat::F64,
        // A well-formed header for an encoding outside the supported
        // PCM set — a capability gap, not corruption.
        _ => return WaveClass::Unsupported,
    };
    if block_align != channels.saturating_mul(bits / 8) || block_align == 0 {
        return WaveClass::Malformed;
    }
    // Channel count the boundary's speaker-mask model cannot express —
    // a capability gap, not corruption.
    let Some(layout) = channel_layout(channels) else {
        return WaveClass::Unsupported;
    };
    WaveClass::Pcm(WavePcm {
        channels,
        layout,
        rate,
        block_align,
        sample,
    })
}

/// `BITMAPINFOHEADER` (`strf` video) fields the demuxer uses.
#[derive(Copy, Clone, Debug)]
pub(crate) struct DibHeader {
    /// `biWidth` — column count (> 0 required).
    pub width: i32,
    /// `biHeight` — signed: negative marks a top-down DIB; the
    /// magnitude is the row count.
    pub height: i32,
    /// `biBitCount` — bits per pixel (24 or 32 supported).
    pub bit_count: u16,
    /// `biCompression` — 0 = `BI_RGB` uncompressed; otherwise a codec
    /// fourCC (`MJPG`, `XVID`, `H264`, …).
    pub compression: u32,
}

/// `strf` (video) classification.
#[derive(Copy, Clone, Debug)]
pub(crate) enum DibClass {
    /// Uncompressed `BI_RGB` DIB at 24 or 32 bpp — decodable.
    Rgb(DibHeader),
    /// A parseable header declaring something this backend cannot
    /// decode — compressed fourCCs, bit depths outside {24, 32}, or
    /// `BI_RLE*` encodings (nonzero compression ids).
    Unsupported(DibHeader),
    /// Body too short for `BITMAPINFOHEADER`, or invalid geometry
    /// (`width <= 0`, `height == 0`).
    Malformed,
}

/// `BI_RGB` — `biCompression` value for uncompressed RGB DIBs.
const BI_RGB: u32 = 0;

/// Parses a `BITMAPINFOHEADER` body (the `vids` `strf` chunk).
pub(crate) fn dib_header(body: &[u8]) -> DibClass {
    if body.len() < 40 {
        return DibClass::Malformed;
    }
    let header = DibHeader {
        width: i32::from_le_bytes(body[4..8].try_into().unwrap()),
        height: i32::from_le_bytes(body[8..12].try_into().unwrap()),
        bit_count: u16::from_le_bytes(body[14..16].try_into().unwrap()),
        compression: u32::from_le_bytes(body[16..20].try_into().unwrap()),
    };
    if header.width <= 0 || header.height == 0 {
        return DibClass::Malformed;
    }
    if header.compression != BI_RGB {
        return DibClass::Unsupported(header);
    }
    match header.bit_count {
        24 | 32 => DibClass::Rgb(header),
        _ => DibClass::Unsupported(header),
    }
}

/// Maps an uncompressed-DIB row to its decode geometry: decoded frame
/// width/height (absolute height; sign only selects row order) and
/// bytes per source row (rows are 4-byte-aligned in the classic
/// packed-DIB layout this backend owns).
impl DibHeader {
    /// Decoded frame width.
    pub(crate) const fn frame_width(self) -> u32 {
        self.width.unsigned_abs()
    }

    /// Decoded frame height — `biHeight` magnitude.
    pub(crate) const fn frame_height(self) -> u32 {
        self.height.unsigned_abs()
    }

    /// `true` when rows are stored bottom-up (`biHeight` positive).
    pub(crate) const fn bottom_up(self) -> bool {
        self.height > 0
    }

    /// Bytes per DIB source row, 4-byte aligned.
    pub(crate) const fn row_stride(self) -> u64 {
        let bits = self.width.unsigned_abs() as u64 * self.bit_count as u64;
        bits.div_ceil(32) * 4
    }

    /// Expected body bytes for one full DIB frame.
    pub(crate) const fn frame_bytes(self) -> u64 {
        self.row_stride() * self.frame_height() as u64
    }
}

/// Maps a `biCompression` fourCC to a registry [`CodecId`]. Only the
/// codec names the AVI support matrix requires are claimed — `MJPG`
/// family → [`CodecId::Mjpeg`], AVC fourCCs → [`CodecId::H264`],
/// uncompressed → [`CodecId::Uncompressed`]; every other fourCC stays
/// [`CodecId::Other`] (its verbatim tag still travels via
/// `codec_tag = FourCc`).
pub(crate) fn dib_codec_id(compression: u32) -> CodecId {
    let cc = compression.to_le_bytes();
    match &cc {
        b"MJPG" | b"MJPA" | b"mjpg" | b"mjpa" | b"dmb1" => CodecId::Mjpeg,
        b"H264" | b"h264" | b"X264" | b"x264" | b"AVC1" | b"avc1" => CodecId::H264,
        _ if compression == BI_RGB => CodecId::Uncompressed,
        _ => CodecId::Other,
    }
}

/// Channel count → boundary [`ChannelLayout`]: the standard speaker
/// sets for 1/2/4/6/8 channels; for other counts a custom mask of
/// reserved bits (above the named `SpeakerMask` range — the same
/// convention `ruviform-io-symphonia` uses for unnamed channel sets).
/// `None` when the count cannot be expressed (more than the 24
/// available reserved mask bits).
pub(crate) fn channel_layout(count: u16) -> Option<ChannelLayout> {
    match count {
        1 => Some(ChannelLayout::Mono),
        2 => Some(ChannelLayout::Stereo),
        4 => Some(ChannelLayout::Quad),
        6 => Some(ChannelLayout::FivePointOne),
        8 => Some(ChannelLayout::SevenPointOne),
        n if n <= 24 => {
            // Reserved bits 8.. keep `mask.channels() == n`.
            let mut mask = SpeakerMask::empty();
            for index in 0..u32::from(n) {
                mask = mask.union(SpeakerMask::from_bits(0x100 << index));
            }
            ChannelLayout::custom(mask).ok()
        }
        _ => None,
    }
}
