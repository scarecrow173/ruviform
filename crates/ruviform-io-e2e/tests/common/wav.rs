//! WAV→`StagedFile` fixture helpers shared by the golden/fault
//! composition test binaries (RF305 slice).
//!
//! Expectations are built from the public API surface plus the
//! RIFF/WAVE and `f32 → wire` *specification* — never from
//! `ruviform-io-wav`'s private helpers — so the golden oracle stays
//! independent of the implementation under test.
#![allow(dead_code)]

use std::io::Cursor;

use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};
use ruviform_io_wav::WavOutputSpec;
use ruviform_media_core::{StreamId, TickTimestamp};
use ruviform_media_io::{ContainerFormat, Demuxer, Packet, PacketFlags};

/// The fixture sample rate shared by the WAV golden/fault slices.
pub const RATE: u32 = 48_000;

/// Stereo S16 spec — the same declaration `wav_staged.rs` uses.
#[must_use]
pub fn s16_stereo_spec() -> WavOutputSpec {
    WavOutputSpec {
        sample_rate: SampleRate::new(RATE).expect("rate"),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::S16,
    }
}

/// Spec for an arbitrary wire format + layout at [`RATE`].
#[must_use]
pub fn spec_for(sample_format: SampleFormat, layout: ChannelLayout) -> WavOutputSpec {
    WavOutputSpec {
        sample_rate: SampleRate::new(RATE).expect("rate"),
        layout,
        sample_format,
    }
}

/// Deterministic interleaved S16 PCM payload for `frames` frames —
/// the same formula `wav_staged.rs` uses.
#[must_use]
pub fn s16_payload(frames: usize, channels: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(frames * channels * 2);
    for i in 0..frames {
        // i64 intermediate keeps the value in i16 range (-16384..=16382).
        let v = i16::try_from((i64::try_from(i).expect("frame index") * 97) % 32_767 - 16_384)
            .expect("sample fits i16");
        for _ in 0..channels {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    data
}

/// A whole-frames packet for `block_align`-aligned payloads.
#[must_use]
pub fn packet(index: u64, block_align: usize, data: Vec<u8>) -> Packet {
    Packet {
        stream: StreamId(0),
        index,
        pts: Some(TickTimestamp::carried(
            i64::try_from(index).expect("index fits i64"),
        )),
        dts: None,
        duration: Some(u64::try_from(data.len() / block_align).expect("duration fits u64")),
        trim_start: 0,
        trim_end: 0,
        flags: PacketFlags::KEYFRAME,
        data,
    }
}

/// Independent decode oracle: symphonia demuxes the bytes and
/// concatenates recovered PCM payload — a parser this crate did not
/// write. Comparing payload *bytes* (not just the count) catches
/// same-length corruption.
pub fn independent_decode(bytes: &[u8]) -> ruviform_media_io::Result<(ContainerFormat, Vec<u8>)> {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes.to_vec()))?;
    let format = demuxer.container().format;
    let mut payload = Vec::new();
    while let Some(pkt) = demuxer.read_packet()? {
        payload.extend_from_slice(&pkt.data);
    }
    Ok((format, payload))
}

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// The complete expected WAV file for `spec` + `payload`, assembled
/// from the RIFF/WAVE specification: `RIFF` size = `file_len` - 8,
/// `fmt ` chunk (`WAVEFORMATEX` 16-byte body for Mono/Stereo,
/// `WAVEFORMATEXTENSIBLE` 40-byte body otherwise), `data` chunk with
/// the declared payload, one zero pad byte when the payload is odd.
#[must_use]
pub fn expected_wav_bytes(spec: &WavOutputSpec, payload: &[u8]) -> Vec<u8> {
    let channels = u16::try_from(spec.layout.channels()).expect("channels fit u16");
    let bits = spec.sample_format.bits_per_sample();
    let bytes_per_sample = u64::from(bits) / 8;
    let block_align =
        u16::try_from(u64::from(channels) * bytes_per_sample).expect("block align fits u16");
    let byte_rate = u32::try_from(u64::from(spec.sample_rate.get()) * u64::from(block_align))
        .expect("byte rate fits u32");
    let extensible = !matches!(spec.layout, ChannelLayout::Mono | ChannelLayout::Stereo);
    let (tag, subtype): (u16, [u8; 16]) = match spec.sample_format {
        SampleFormat::U8 | SampleFormat::S16 | SampleFormat::S24 | SampleFormat::S32 => {
            (0x0001, SUBTYPE_PCM)
        }
        SampleFormat::F32 => (0x0003, SUBTYPE_IEEE_FLOAT),
        _ => panic!(
            "sample format {:?} is not a WAV wire format",
            spec.sample_format
        ),
    };

    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&(if extensible { 0xFFFE_u16 } else { tag }).to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&spec.sample_rate.get().to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&bits.to_le_bytes());
    if extensible {
        body.extend_from_slice(&22_u16.to_le_bytes()); // cbSize
        body.extend_from_slice(&bits.to_le_bytes()); // wValidBitsPerSample
        body.extend_from_slice(&spec.layout.speaker_mask().bits().to_le_bytes());
        body.extend_from_slice(&subtype);
    }

    let mut out = Vec::with_capacity(44 + payload.len() + 1);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&0_u32.to_le_bytes()); // riff size — patched below
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&u32::try_from(body.len()).expect("fmt len").to_le_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(b"data");
    let data_size_at = out.len();
    out.extend_from_slice(&0_u32.to_le_bytes()); // data size — patched below
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    let riff_size = u32::try_from(out.len() - 8).expect("riff size fits u32");
    out[4..8].copy_from_slice(&riff_size.to_le_bytes());
    let data_size = u32::try_from(payload.len()).expect("data size fits u32");
    out[data_size_at..data_size_at + 4].copy_from_slice(&data_size.to_le_bytes());
    out
}

/// Independent encoding of the declared `f32 → wire` policy
/// (NaN→0, clamp ±1, round half away from zero, `F32` verbatim, no
/// dither) — written from the crate-level documentation, not from
/// the private `convert` module.
// The `f64` → int casts below mirror the documented policy: `clipped`
// is clamped to `[-1.0, 1.0]` first, so every scaled+rounded value
// lands inside the destination type's range before the cast.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn expected_wire_sample(format: SampleFormat, value: f32) -> Vec<u8> {
    let clipped = if value.is_nan() {
        0.0_f64
    } else {
        f64::from(value).clamp(-1.0, 1.0)
    };
    match format {
        SampleFormat::U8 => {
            vec![((clipped * 128.0 + 128.0).round().clamp(0.0, 255.0)) as u8]
        }
        SampleFormat::S16 => ((clipped * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16)
            .to_le_bytes()
            .to_vec(),
        SampleFormat::S24 => {
            let sample = (clipped * 8_388_608.0)
                .round()
                .clamp(-8_388_608.0, 8_388_607.0) as i32;
            sample.to_le_bytes()[..3].to_vec()
        }
        SampleFormat::S32 => {
            let sample = (clipped * 2_147_483_648.0)
                .round()
                .clamp(-2_147_483_648.0, 2_147_483_647.0) as i64;
            (sample as i32).to_le_bytes().to_vec()
        }
        SampleFormat::F32 => value.to_le_bytes().to_vec(),
        _ => panic!("sample format {format:?} is not a WAV wire format"),
    }
}

/// The canonical `f32` edge samples the encoder-path golden feeds
/// through `WavEncoder`: NaN, infinities, out-of-range clip, `.5`
/// LSB rounding boundaries for every integer wire format (all
/// dyadic-exact in `f32`), mid-scale and full-scale ends.
#[must_use]
pub fn edge_samples() -> Vec<f32> {
    vec![
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2.0,
        -2.0,
        0.5,
        -0.5,
        1.0 / 256.0, // U8: x*128+128 lands on .5 LSB
        -1.0 / 256.0,
        1.5 / 32_768.0, // S16: x*32768 lands on .5 LSB
        -1.5 / 32_768.0,
        1.5 / 8_388_608.0, // S24: x*8388608 lands on .5 LSB
        -1.5 / 8_388_608.0,
        1.5 / 2_147_483_648.0, // S32: x*2^31 lands on .5 LSB
        -1.5 / 2_147_483_648.0,
        0.0,
        1.0,
        -1.0,
    ]
}

/// Interleaved wire payload for `samples` on a single channel,
/// encoded by the independent policy oracle.
#[must_use]
pub fn expected_edge_payload(format: SampleFormat, samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 4);
    for &sample in samples {
        out.extend_from_slice(&expected_wire_sample(format, sample));
    }
    out
}
