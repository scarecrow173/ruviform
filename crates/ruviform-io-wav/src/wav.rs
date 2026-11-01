//! RIFF/WAVE serialization primitives (little-endian) — private to the
//! backend.
//!
//! Classic RIFF/WAVE only: 32-bit chunk sizes, `WAVE` form type, `fmt`
//! and `data` chunks. No `RF64`/`ds64`, no `bext`/BWF metadata — both
//! are out of the declared capability set
//! ([`crate::wav_capabilities`]).

use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat};
use ruviform_media_io::{Error, ErrorKind, Result};

/// RIFF `WAVE_FORMAT_PCM` — integer PCM.
const TAG_PCM: u16 = 0x0001;
/// RIFF `WAVE_FORMAT_IEEE_FLOAT` — IEEE-754 float PCM.
const TAG_IEEE_FLOAT: u16 = 0x0003;
/// RIFF `WAVE_FORMAT_EXTENSIBLE` — subformat + channel mask form.
const TAG_EXTENSIBLE: u16 = 0xFFFE;

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// The wire sample formats this backend emits — advertised verbatim in
/// [`crate::WavCapabilities::sample_formats`].
pub(crate) const WIRE_FORMATS: &[SampleFormat] = &[
    SampleFormat::U8,
    SampleFormat::S16,
    SampleFormat::S24,
    SampleFormat::S32,
    SampleFormat::F32,
];

/// Whether `layout` is emitted as plain `WAVEFORMATEX` (16-byte `fmt`)
/// or `WAVEFORMATEXTENSIBLE` (40-byte `fmt` with channel mask).
///
/// Mono and Stereo take the classic form — every RIFF reader handles
/// it — while any other layout (including a 2-channel `Custom` mask
/// that is not plain stereo) needs the mask-carrying extensible form.
pub(crate) fn use_extensible(layout: ChannelLayout) -> bool {
    !matches!(layout, ChannelLayout::Mono | ChannelLayout::Stereo)
}

/// The `wFormatTag` for a wire sample format — `None` when the format
/// is outside [`WIRE_FORMATS`].
pub(crate) fn format_tag(format: SampleFormat) -> Option<u16> {
    match format {
        SampleFormat::U8 | SampleFormat::S16 | SampleFormat::S24 | SampleFormat::S32 => {
            Some(TAG_PCM)
        }
        SampleFormat::F32 => Some(TAG_IEEE_FLOAT),
        _ => None,
    }
}

/// The `KSDATAFORMAT_SUBTYPE_*` GUID for a wire sample format.
fn subtype(format: SampleFormat) -> [u8; 16] {
    match format_tag(format) {
        Some(TAG_IEEE_FLOAT) => SUBTYPE_IEEE_FLOAT,
        _ => SUBTYPE_PCM,
    }
}

/// Validated wire parameters shared by encoder and muxer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Wire {
    /// Wire sample encoding (one of [`WIRE_FORMATS`]).
    pub format: SampleFormat,
    /// Channel count (`layout.channels()`), u16-safe.
    pub channels: u16,
    /// Samples per second.
    pub rate: u32,
    /// `channels * bits_per_sample / 8`.
    pub block_align: u16,
    /// `rate * block_align`.
    pub byte_rate: u32,
    /// Speaker mask for `WAVEFORMATEXTENSIBLE` (`dwChannelMask`).
    pub channel_mask: u32,
    /// Whether the `fmt` chunk is the extensible form.
    pub extensible: bool,
}

impl Wire {
    /// Builds wire parameters from a declared wire [`AudioFormat`];
    /// rejects sample formats outside the advertised set and channel
    /// counts/byte rates that overflow the RIFF fields.
    pub(crate) fn new(format: &AudioFormat) -> Result<Self> {
        let sample_format = format.sample_format;
        if format_tag(sample_format).is_none() {
            return Err(Error::with_source(
                ErrorKind::Unsupported,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("wire sample format {sample_format:?} is not in wav_capabilities"),
                ),
            ));
        }
        let channels = u16::try_from(format.layout.channels())
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        let bytes_per_sample = u64::from(sample_format.bits_per_sample()) / 8;
        let block_align = u16::try_from(u64::from(channels) * bytes_per_sample)
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        let byte_rate = u32::try_from(u64::from(format.sample_rate.get()) * u64::from(block_align))
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        Ok(Self {
            format: sample_format,
            channels,
            rate: format.sample_rate.get(),
            block_align,
            byte_rate,
            channel_mask: format.layout.speaker_mask().bits(),
            extensible: use_extensible(format.layout),
        })
    }
}

/// Bytes of a 16-bit little-endian value.
fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Bytes of a 32-bit little-endian value.
fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// The complete `fmt ` chunk body (`WAVEFORMATEX` 16 bytes or
/// `WAVEFORMATEXTENSIBLE` 40 bytes).
pub(crate) fn fmt_chunk_body(wire: &Wire) -> Vec<u8> {
    let mut body = Vec::with_capacity(40);
    if wire.extensible {
        push_u16(&mut body, TAG_EXTENSIBLE);
    } else {
        push_u16(&mut body, format_tag(wire.format).unwrap_or(TAG_PCM));
    }
    push_u16(&mut body, wire.channels);
    push_u32(&mut body, wire.rate);
    push_u32(&mut body, wire.byte_rate);
    push_u16(&mut body, wire.block_align);
    push_u16(&mut body, wire.format.bits_per_sample());
    if wire.extensible {
        push_u16(&mut body, 22); // cbSize
        push_u16(&mut body, wire.format.bits_per_sample()); // wValidBitsPerSample
        push_u32(&mut body, wire.channel_mask);
        body.extend_from_slice(&subtype(wire.format));
    }
    body
}

/// The complete file header — `RIFF` + size placeholder + `WAVE` +
/// `fmt ` chunk + `data` + size placeholder. Returns the byte offset
/// of the `data` chunk's size field for later backpatching.
pub(crate) fn header(wire: &Wire) -> (Vec<u8>, u64) {
    let body = fmt_chunk_body(wire);
    let mut header = Vec::with_capacity(12 + 8 + body.len() + 8);
    header.extend_from_slice(b"RIFF");
    push_u32(&mut header, 0); // backpatched: riff size = file len - 8
    header.extend_from_slice(b"WAVE");
    header.extend_from_slice(b"fmt ");
    push_u32(&mut header, u32::try_from(body.len()).unwrap_or(u32::MAX));
    header.extend_from_slice(&body);
    header.extend_from_slice(b"data");
    let data_size_offset = header.len() as u64;
    push_u32(&mut header, 0); // backpatched: data chunk byte count
    (header, data_size_offset)
}

/// Byte offset of the `RIFF` chunk size field (always 4).
pub(crate) const RIFF_SIZE_OFFSET: u64 = 4;
