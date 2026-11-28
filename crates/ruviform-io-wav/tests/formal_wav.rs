//! Issue #806 slice 4 (D-layer formal verification) — PCM WAV backend
//! property suite plus `#[cfg(kani)]` bounded proof harnesses.
//!
//! Target contracts (product source of truth:
//! `crates/ruviform-io-wav/src/{lib,wav,convert,encoder,muxer}.rs`):
//!
//! - `wav_capabilities().sample_formats` is the exact accepted wire
//!   set; `Wire::new` additionally gates `rate * block_align` on the
//!   RIFF `u32` byte-rate field (`InvalidValue` on overflow).
//! - The `fmt ` chunk is the 16-byte `WAVEFORMATEX` for Mono/Stereo
//!   and the 40-byte `WAVEFORMATEXTENSIBLE` (tag `0xFFFE`, `cbSize`
//!   22, valid bits, speaker mask, subtype GUID) for every other
//!   layout — including a 2-channel `Custom` mask.
//! - `open_muxer` writes `RIFF`/`WAVE`/`fmt `/`data` with zeroed size
//!   placeholders immediately and normalizes the declared stream
//!   (`time_base` = `1/rate`, `decodable` = `NotDeclared`).
//! - Canonical `f32` → wire conversion follows the documented policy:
//!   `NaN` → silence, `±Inf`/`|x| > 1` saturate to full scale,
//!   integer formats round half away from zero at their documented
//!   scale (`U8` `x*128+128`, `S16` `x*32768`, `S24` `x*8388608`,
//!   `S32` `x*2147483648`), `F32` writes verbatim, no dither.
//! - `WavEncoder` emits one packet per nonempty frame — monotone
//!   index, `KEYFRAME`, `duration == frames`, pts converted to stream
//!   ticks with provenance preserved; rejections are `InvalidData`
//!   and do not poison the encoder.
//! - `WavMuxer::write_packet` rejects wrong-stream and
//!   non-whole-frame payloads (`InvalidData`) without poisoning;
//!   `finalize` pads the `data` chunk to even, backpatches
//!   `riff size = len - 8` and `data size = payload`, flushes, and
//!   returns the only completion `WavReceipt` — a second finalize
//!   is `InvalidData` and never retracts the receipt.
//! - Error classification is stable: declaration problems are
//!   `Unsupported`/`InvalidValue`, stream/data problems are
//!   `InvalidData`, sink problems are `Io` — never a panic.
//!
//! The `#[cfg(kani)]` module at the bottom carries a bounded proof of
//! the `open_muxer` public-boundary contract; the `pub(crate)`
//! `write_sample`/`Wire`/`fmt `/`header` proofs live in-source
//! (`src/convert.rs`, `src/wav.rs`). All run via
//! `mise run test-formal-kani`; bounds and assumptions are documented
//! in `docs/specs/formal-verification.md`.

use proptest::prelude::*;
use ruviform_audio_core::{
    AudioBuffer, AudioFormat, BufferSpec, ChannelLayout, SampleFormat, SampleRate, SpeakerMask,
};
use ruviform_io_wav::{WavOutputSpec, make_encoder, open_muxer};
use ruviform_media_core::{
    Fact, MediaTime, MediaTimestamp, Rounding, StreamId, StreamKind, TickTimestamp,
};
use ruviform_media_io::{
    AudioFrame, CodecFormat, CodecId, Encoder, ErrorKind, Frame, Muxer, NativeAudioFrame, Packet,
    PacketFlags, StreamInfo,
};
use std::io::Cursor;

/// Every `SampleFormat` variant — the membership oracle's domain.
const ALL_FORMATS: &[SampleFormat] = &[
    SampleFormat::F32,
    SampleFormat::F64,
    SampleFormat::U8,
    SampleFormat::S8,
    SampleFormat::U16,
    SampleFormat::S16,
    SampleFormat::U24,
    SampleFormat::S24,
    SampleFormat::U32,
    SampleFormat::S32,
];

/// The advertised wire set — kept identical to `wav_capabilities()`
/// on purpose: the suite asserts the product's declaration, so the
/// list is duplicated rather than borrowed.
const WIRE_FORMATS: &[SampleFormat] = &[
    SampleFormat::U8,
    SampleFormat::S16,
    SampleFormat::S24,
    SampleFormat::S32,
    SampleFormat::F32,
];

/// `SampleFormat` variants outside the advertised wire set.
const NONWIRE_FORMATS: &[SampleFormat] = &[
    SampleFormat::F64,
    SampleFormat::S8,
    SampleFormat::U16,
    SampleFormat::U24,
    SampleFormat::U32,
];

/// Named layouts — the layouts `WAVEFORMATEX`-class readers model
/// without consulting `dwChannelMask`.
const NAMED_LAYOUTS: &[ChannelLayout] = &[
    ChannelLayout::Mono,
    ChannelLayout::Stereo,
    ChannelLayout::Quad,
    ChannelLayout::FivePointOne,
    ChannelLayout::SevenPointOne,
];

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const PCM_GUID: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const FLOAT_GUID: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

// --- generators -------------------------------------------------------------

fn arb_sample_format() -> impl Strategy<Value = SampleFormat> {
    prop::sample::select(ALL_FORMATS)
}

fn arb_wire_format() -> impl Strategy<Value = SampleFormat> {
    prop::sample::select(WIRE_FORMATS)
}

fn arb_named_layout() -> impl Strategy<Value = ChannelLayout> {
    prop::sample::select(NAMED_LAYOUTS)
}

/// Layouts across both `fmt ` forms: named classics plus arbitrary
/// `Custom` speaker masks (bit 0 forced set so the non-empty
/// `SpeakerMaskSet` invariant always holds — 1 to 32 channels).
fn arb_layout() -> impl Strategy<Value = ChannelLayout> {
    prop_oneof![
        3 => arb_named_layout(),
        2 => any::<u32>().prop_map(|bits| {
            ChannelLayout::custom(SpeakerMask::from_bits(bits | 0x1))
                .expect("bit 0 forces a non-empty mask")
        }),
    ]
}

/// A spec the backend must accept: `rate <= 192_000` keeps
/// `rate * block_align <= 192_000 * 128 < u32::MAX` for every layout.
fn arb_valid_spec() -> impl Strategy<Value = WavOutputSpec> {
    (
        (1u32..=192_000).prop_map(|r| SampleRate::new(r).expect("nonzero rate")),
        arb_layout(),
        arb_wire_format(),
    )
        .prop_map(|(rate, layout, format)| WavOutputSpec {
            sample_rate: rate,
            layout,
            sample_format: format,
        })
}

/// Canonical `f32` samples biased toward the contract's interesting
/// regions: specials, the ±1.0 full-scale edges, and the clipping
/// band just outside them.
fn arb_f32() -> impl Strategy<Value = f32> {
    prop_oneof![
        4 => any::<f32>(),
        3 => -2.0f32..=2.0,
        2 => prop::sample::select(vec![
            0.0,
            -0.0,
            0.5,
            -0.5,
            1.0,
            -1.0,
            0.999_999_9,
            -0.999_999_9,
            1.000_000_1,
            -1.000_000_1,
            1.5,
            -1.5,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE,
        ]),
    ]
}

/// Packet/frame pts ticks — mostly small positive, with extremes so
/// the fallback and `first_pts` paths see non-trivial values.
fn arb_ticks() -> impl Strategy<Value = i64> {
    prop_oneof![
        5 => 0i64..=1_000_000,
        3 => any::<i64>(),
        2 => prop::sample::select(vec![0i64, -1, 1, i64::MAX / 2, i64::MIN / 2]),
    ]
}

// --- model / oracle helpers -------------------------------------------------

/// `nBlockAlign` — `channels * bits_per_sample / 8`.
fn block_align(spec: &WavOutputSpec) -> u64 {
    u64::from(spec.layout.channels()) * u64::from(spec.sample_format.bits_per_sample()) / 8
}

/// `nAvgBytesPerSec` — `rate * block_align`.
fn byte_rate(spec: &WavOutputSpec) -> u64 {
    u64::from(spec.sample_rate.get()) * block_align(spec)
}

/// Whether `layout` serializes as `WAVEFORMATEXTENSIBLE` — every
/// layout but the two classics, per `wav::use_extensible`.
fn is_extensible(spec: &WavOutputSpec) -> bool {
    !matches!(spec.layout, ChannelLayout::Mono | ChannelLayout::Stereo)
}

fn fmt_body_len(spec: &WavOutputSpec) -> usize {
    if is_extensible(spec) { 40 } else { 16 }
}

fn header_len(spec: &WavOutputSpec) -> usize {
    // RIFF(4) + size(4) + WAVE(4) + fmt (4) + size(4) + body + data(4) + size(4)
    28 + fmt_body_len(spec)
}

fn data_size_offset(spec: &WavOutputSpec) -> usize {
    24 + fmt_body_len(spec)
}

/// `wFormatTag` — `0xFFFE` whenever the extensible form carries the
/// real tag in the subtype GUID; `1` integer PCM / `3` IEEE float
/// otherwise.
fn expected_tag(spec: &WavOutputSpec) -> u16 {
    if is_extensible(spec) {
        0xFFFE
    } else if spec.sample_format == SampleFormat::F32 {
        0x0003
    } else {
        0x0001
    }
}

fn u16le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Parses a freshly-written header (zero placeholders) and asserts
/// every serialized field against the spec-derived model.
fn check_header(bytes: &[u8], spec: &WavOutputSpec) -> Result<(), TestCaseError> {
    prop_assert_eq!(u32le(&bytes[4..8]), 0, "riff size stays a zero placeholder");
    let body_len = fmt_body_len(spec);
    prop_assert_eq!(
        u32le(&bytes[24 + body_len..28 + body_len]),
        0,
        "data size stays a zero placeholder"
    );
    check_header_fields(bytes, spec)
}

/// The header fields that stay invariant across `finalize`'s size
/// backpatch — everything but the two placeholder `u32`s.
fn check_header_fields(bytes: &[u8], spec: &WavOutputSpec) -> Result<(), TestCaseError> {
    let body_len = fmt_body_len(spec);
    prop_assert_eq!(bytes.len(), header_len(spec), "header length");
    prop_assert_eq!(&bytes[0..4], b"RIFF");
    prop_assert_eq!(&bytes[8..12], b"WAVE");
    prop_assert_eq!(&bytes[12..16], b"fmt ");
    prop_assert_eq!(
        u32le(&bytes[16..20]),
        u32::try_from(body_len).expect("body len fits u32"),
        "fmt chunk length"
    );
    let body = &bytes[20..20 + body_len];
    prop_assert_eq!(u16le(&body[0..2]), expected_tag(spec), "wFormatTag");
    prop_assert_eq!(
        u32::from(u16le(&body[2..4])),
        spec.layout.channels(),
        "nChannels"
    );
    prop_assert_eq!(u32le(&body[4..8]), spec.sample_rate.get(), "nSamplesPerSec");
    prop_assert_eq!(
        u64::from(u32le(&body[8..12])),
        byte_rate(spec),
        "nAvgBytesPerSec"
    );
    prop_assert_eq!(
        u64::from(u16le(&body[12..14])),
        block_align(spec),
        "nBlockAlign"
    );
    prop_assert_eq!(
        u16le(&body[14..16]),
        spec.sample_format.bits_per_sample(),
        "wBitsPerSample"
    );
    if body_len == 40 {
        prop_assert_eq!(u16le(&body[16..18]), 22, "cbSize");
        prop_assert_eq!(
            u16le(&body[18..20]),
            spec.sample_format.bits_per_sample(),
            "wValidBitsPerSample"
        );
        prop_assert_eq!(
            u32le(&body[20..24]),
            spec.layout.speaker_mask().bits(),
            "dwChannelMask"
        );
        let want_guid: &[u8] = if spec.sample_format == SampleFormat::F32 {
            &FLOAT_GUID
        } else {
            &PCM_GUID
        };
        prop_assert_eq!(&body[24..40], want_guid, "SubFormat GUID");
    }
    prop_assert_eq!(&bytes[20 + body_len..24 + body_len], b"data");
    Ok(())
}

/// The conversion policy as documented in `src/convert.rs` and the
/// crate-level contract — restated so the property compares the
/// implementation against the *declared* semantics, not against
/// itself.
// The `f64` → int casts are range-safe by the same argument as the
// product: `clipped` sits in [-1.0, 1.0], so every scaled+rounded
// value lands inside the destination range (`as` also saturates).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn model_sample_bytes(format: SampleFormat, value: f32) -> Vec<u8> {
    let clipped = if value.is_nan() {
        0.0_f64
    } else {
        f64::from(value).clamp(-1.0, 1.0)
    };
    match format {
        SampleFormat::U8 => vec![(clipped * 128.0 + 128.0).round().clamp(0.0, 255.0) as u8],
        SampleFormat::S16 => ((clipped * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16)
            .to_le_bytes()
            .to_vec(),
        SampleFormat::S24 => ((clipped * 8_388_608.0)
            .round()
            .clamp(-8_388_608.0, 8_388_607.0) as i32)
            .to_le_bytes()[..3]
            .to_vec(),
        SampleFormat::S32 => (((clipped * 2_147_483_648.0)
            .round()
            .clamp(-2_147_483_648.0, 2_147_483_647.0) as i64) as i32)
            .to_le_bytes()
            .to_vec(),
        SampleFormat::F32 => value.to_le_bytes().to_vec(),
        _ => Vec::new(),
    }
}

/// Semantic (non-byte-model) invariants for a signed wire sample —
/// stated directly from the documented policy so a wrong-but-
/// self-consistent implementation still trips them.
fn check_signed_sample(code: i64, value: f32, min: i64, max: i64) -> Result<(), TestCaseError> {
    prop_assert!(
        code >= min && code <= max,
        "wire code {code} inside [{min}, {max}]"
    );
    if value.is_nan() {
        prop_assert_eq!(code, 0, "NaN must produce silence");
    }
    if value >= 1.0 {
        prop_assert_eq!(code, max, "+full-scale saturation");
    }
    if value <= -1.0 {
        prop_assert_eq!(code, min, "-full-scale saturation");
    }
    if !value.is_nan() && value > 0.0 {
        prop_assert!(code >= 0, "positive input must not decode negative");
    }
    if !value.is_nan() && value < 0.0 {
        prop_assert!(code <= 0, "negative input must not decode positive");
    }
    Ok(())
}

/// The pts the encoder stamps on a packet — mirrors
/// `WavEncoder::pts_ticks`: exact rational conversion first,
/// seconds-based rounding on overflow.
// `f64` → `i64` after `.round()` — the documented fallback; the lint
// can't see the saturation semantics of `as`.
#[allow(clippy::cast_possible_truncation)]
fn model_pts_ticks(pts: MediaTime, rate: SampleRate) -> i64 {
    pts.convert(rate.time_base(), Rounding::NearestEven)
        .map_or_else(
            |_| (pts.to_seconds_f64() * f64::from(rate.get())).round() as i64,
            MediaTime::value,
        )
}

/// Builds an `AudioBuffer` for `spec` whose interleaved sample order
/// is `samples` cycling — `samples[f * channels + c]` lands in
/// `channel c`, `frame f`, matching the wire interleave order.
fn buffer_with(spec: &WavOutputSpec, frames: usize, samples: &[f32]) -> AudioBuffer {
    let channels = spec.layout.channels() as usize;
    let mut buffer = AudioBuffer::new(BufferSpec {
        layout: spec.layout,
        frames,
        sample_rate: spec.sample_rate,
    });
    let take = |index: usize| {
        samples
            .get(index % samples.len().max(1))
            .copied()
            .unwrap_or(0.0)
    };
    for channel in 0..channels {
        let plane = buffer.channel_mut(channel).expect("channel exists");
        for (frame_index, sample) in plane.iter_mut().enumerate() {
            *sample = take(frame_index * channels + channel);
        }
    }
    buffer
}

/// Writes `frames` through encoder → muxer → `finalize` over an
/// in-memory sink and returns the file bytes plus the receipt.
fn encode_to_file(info: &StreamInfo, frames: Vec<Frame>) -> (Vec<u8>, ruviform_io_wav::WavReceipt) {
    let mut encoder = make_encoder(info).expect("spec must be accepted");
    for frame in frames {
        encoder.send_frame(frame).expect("valid frame accepted");
    }
    encoder.drain().expect("drain");
    let mut muxer = open_muxer(Cursor::new(Vec::new()), std::slice::from_ref(info))
        .expect("spec must be accepted");
    while let Some(packet) = encoder.receive_packet().expect("packet read") {
        muxer.write_packet(packet).expect("encoder packet accepted");
    }
    let receipt = muxer.finalize().expect("finalize");
    (muxer.into_sink().into_inner(), receipt)
}

/// Maximum per-sample error the wire format's quantization permits —
/// the same tolerance table `tests/roundtrip.rs` uses.
fn tolerance(wire: SampleFormat) -> f32 {
    match wire {
        SampleFormat::U8 => 1.0 / 128.0 + 1e-6,
        SampleFormat::S16 => 1.0 / 32768.0 + 1e-6,
        SampleFormat::S24 => 1.0 / 8_388_608.0 + 1e-7,
        SampleFormat::S32 => 1.0 / 2_147_483_648.0 + 1e-8,
        SampleFormat::F32 => 1e-7,
        _ => 1e-6,
    }
}

/// Reopens `bytes` through `ruviform-io-symphonia` and decodes the
/// single stream to canonical `f32` planes.
fn decode_planes(bytes: &[u8]) -> (StreamInfo, usize, Vec<Vec<f32>>) {
    use ruviform_media_io::{Decoder, Demuxer};

    let mut demuxer =
        ruviform_io_symphonia::open_demuxer(Cursor::new(bytes.to_vec())).expect("wav must reopen");
    let info = {
        let streams = demuxer.streams();
        assert_eq!(streams.len(), 1, "wav carries exactly one stream");
        streams[0].clone()
    };
    let mut decoder = demuxer
        .make_decoder(info.stream.id)
        .expect("decoder builds");
    let mut planes: Vec<Vec<f32>> = Vec::new();
    let mut frames = 0usize;
    let mut take = |frame: &Frame| {
        if let Frame::Audio(audio) = frame {
            let buffer = audio.buffer();
            if planes.is_empty() {
                planes = vec![Vec::new(); buffer.channels()];
            }
            for (channel, plane) in planes.iter_mut().enumerate() {
                plane.extend_from_slice(buffer.channel(channel).expect("channel exists"));
            }
            frames += buffer.frames();
        }
    };
    while let Some(packet) = demuxer.read_packet().expect("packet read") {
        decoder.send_packet(packet).expect("packet accepted");
        while let Some(frame) = decoder.receive_frame().expect("frame read") {
            take(&frame);
        }
    }
    decoder.drain().expect("drain");
    while let Some(frame) = decoder.receive_frame().expect("frame read") {
        take(&frame);
    }
    (info, frames, planes)
}

/// The shell half of the rf#1015 slice-18 boundary: a non-wire
/// declaration is rejected `Unsupported`, and the lazy diagnostic source
/// remains an `InvalidInput` `io::Error` saying that the format is outside
/// the `wav_capabilities` wire set. The exact sentence is not promoted to
/// a wire contract; the assertions pin only the stable kind and meaning.
#[test]
fn unsupported_wire_format_rejection_carries_invalid_input_diagnostic() {
    let spec = WavOutputSpec {
        sample_rate: SampleRate::new(48_000).expect("nonzero rate"),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F64,
    };
    let info = spec.stream_info();
    let results: [ruviform_media_io::Result<()>; 2] = [
        make_encoder(&info).map(|_| ()),
        open_muxer(Cursor::new(Vec::new()), &[info]).map(|_| ()),
    ];
    for result in results {
        let error = result.expect_err("non-wire format must be rejected");
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        let source = std::error::Error::source(&error).expect("io diagnostic source");
        let io_error = source
            .downcast_ref::<std::io::Error>()
            .expect("unsupported diagnostic is an io::Error");
        assert_eq!(io_error.kind(), std::io::ErrorKind::InvalidInput);
        let diagnostic = io_error.to_string();
        assert!(
            diagnostic.contains("F64"),
            "diagnostic names the format: {diagnostic}"
        );
        assert!(
            diagnostic.contains("is not in wav_capabilities"),
            "diagnostic explains the wire-set rejection: {diagnostic}"
        );
    }
}

proptest! {
    /// The stream-declaration gate is a biconditional: a spec is
    /// accepted iff its wire format is advertised AND
    /// `rate * block_align <= u32::MAX` — rejections are `Unsupported`
    /// for the format and `InvalidValue` for the overflow, at both
    /// `make_encoder` and `open_muxer`. `mode == 1` generates rates in
    /// a ±4 window around the exact overflow threshold so the
    /// boundary is exercised densely (slice-2 boundary-density
    /// lesson).
    #[test]
    fn wire_gate_is_exact_membership_plus_byte_rate(
        layout in arb_layout(),
        format in arb_sample_format(),
        mode in 0u8..=1,
        rate_raw in 1u32..=u32::MAX,
        delta in -4i64..=4,
    ) {
        let align = u64::from(layout.channels())
            * u64::from(format.bits_per_sample())
            / 8;
        let max_rate = u64::from(u32::MAX) / align;
        let rate_value = if mode == 0 {
            rate_raw
        } else {
            let wide = i64::try_from(max_rate).unwrap_or(i64::MAX) + delta;
            u32::try_from(wide.clamp(1, i64::from(u32::MAX))).unwrap_or(1)
        };
        let spec = WavOutputSpec {
            sample_rate: SampleRate::new(rate_value).expect("clamped nonzero rate"),
            layout,
            sample_format: format,
        };
        let info = spec.stream_info();
        let advertised = WIRE_FORMATS.contains(&format);
        let fits = u32::try_from(u64::from(rate_value) * align).is_ok();

        let encoder_result = make_encoder(&info).map(|_| ());
        let muxer_result = open_muxer(Cursor::new(Vec::new()), &[info]).map(|_| ());
        for result in [encoder_result, muxer_result] {
            match result {
                Ok(()) => {
                    prop_assert!(advertised, "non-wire {format:?} accepted");
                    prop_assert!(fits, "accepted with overflowing byte_rate");
                }
                Err(error) => {
                    if advertised {
                        prop_assert!(!fits, "wire {format:?} rejected with fitting byte_rate");
                        prop_assert_eq!(error.kind(), ErrorKind::InvalidValue);
                    } else {
                        prop_assert_eq!(error.kind(), ErrorKind::Unsupported);
                    }
                }
            }
        }
    }

    /// Every single-field violation of the stream-declaration rule
    /// keeps its documented `ErrorKind` — both at `make_encoder` and
    /// `open_muxer`. `multi_stream` additionally asserts the
    /// muxer-only single-stream gate.
    #[test]
    fn declaration_mutations_keep_stable_error_kinds(
        spec in arb_valid_spec(),
        mutation in 0u8..=6,
        sel in any::<u8>(),
    ) {
        let mut info = spec.stream_info();
        let expected = match mutation {
            // codec is not Pcm
            0 => {
                info.codec.codec = match sel % 5 {
                    0 => CodecId::Flac,
                    1 => CodecId::Aac,
                    2 => CodecId::Mp3,
                    3 => CodecId::Uncompressed,
                    _ => CodecId::Other,
                };
                ErrorKind::Unsupported
            }
            // canonical format is not Audio(F32)
            1 => {
                info.codec.format = match sel % 3 {
                    0 => CodecFormat::Data,
                    1 => CodecFormat::Audio(AudioFormat {
                        sample_rate: spec.sample_rate,
                        layout: spec.layout,
                        sample_format: SampleFormat::S16,
                    }),
                    _ => CodecFormat::Audio(AudioFormat {
                        sample_rate: spec.sample_rate,
                        layout: spec.layout,
                        sample_format: SampleFormat::F64,
                    }),
                };
                ErrorKind::Unsupported
            }
            // native_format is not Fact::Known(Audio)
            2 => {
                info.codec.native_format = match sel % 4 {
                    0 => Fact::NotDeclared,
                    1 => Fact::Unsupported,
                    2 => Fact::Malformed,
                    _ => Fact::known(CodecFormat::Data),
                };
                ErrorKind::InvalidValue
            }
            // native wire format outside the advertised set
            3 => {
                info.codec.native_format = Fact::known(CodecFormat::Audio(AudioFormat {
                    sample_rate: spec.sample_rate,
                    layout: spec.layout,
                    sample_format: NONWIRE_FORMATS[usize::from(sel % 5)],
                }));
                ErrorKind::Unsupported
            }
            // canonical/wire rate-layout disagreement
            4 => {
                let Fact::Known(CodecFormat::Audio(mut native)) = info.codec.native_format else {
                    unreachable!("spec builds a known audio native_format")
                };
                if sel % 2 == 0 {
                    native.sample_rate =
                        SampleRate::new(spec.sample_rate.get() + 1).expect("rate + 1 nonzero");
                } else {
                    native.layout = match spec.layout {
                        ChannelLayout::Stereo => ChannelLayout::Mono,
                        _ => ChannelLayout::Stereo,
                    };
                }
                info.codec.native_format = Fact::known(CodecFormat::Audio(native));
                ErrorKind::InvalidValue
            }
            // non-audio stream kind
            5 => {
                info.stream.kind = if sel % 2 == 0 {
                    StreamKind::Video
                } else {
                    StreamKind::Data
                };
                ErrorKind::InvalidValue
            }
            // the muxer's single-stream gate (encoder takes one
            // StreamInfo — nothing to violate there)
            _ => {
                let mut second = info.clone();
                second.stream.id = StreamId(1);
                let error = open_muxer(Cursor::new(Vec::new()), &[info.clone(), second])
                    .expect_err("two streams must fail");
                prop_assert_eq!(error.kind(), ErrorKind::Unsupported);
                return Ok(());
            }
        };
        prop_assert_eq!(make_encoder(&info).expect_err("mutant rejected").kind(), expected);
        prop_assert_eq!(
            open_muxer(Cursor::new(Vec::new()), &[info])
                .expect_err("mutant rejected")
                .kind(),
            expected
        );
    }

    /// `open_muxer` serializes the complete RIFF/WAVE header
    /// immediately: `RIFF`+zero size+`WAVE`+`fmt `(16 or 40 bytes by
    /// layout)+`data`+zero size, with every `fmt ` field derived from
    /// the spec — and normalizes the declared stream (`1/rate` base,
    /// `decodable` reset to `NotDeclared`).
    #[test]
    fn open_muxer_writes_complete_header(spec in arb_valid_spec()) {
        let info = spec.stream_info();
        let muxer =
            open_muxer(Cursor::new(Vec::new()), &[info]).expect("valid spec opens");
        let declared = &muxer.streams()[0];
        prop_assert_eq!(declared.stream.id, StreamId(0));
        prop_assert_eq!(declared.stream.kind, StreamKind::Audio);
        prop_assert_eq!(
            declared.stream.time_base,
            Fact::known(spec.sample_rate.time_base())
        );
        prop_assert_eq!(declared.facts.decodable, Fact::NotDeclared);
        let bytes = muxer.into_sink().into_inner();
        check_header(&bytes, &spec)?;
    }

    /// Canonical `f32` planes become exactly the documented wire
    /// bytes: packet length `frames * block_align`, byte-exact model
    /// match per sample, plus the independent semantic invariants
    /// (NaN → silence code point, ±full-scale saturation endpoints,
    /// sign preservation, F32 bit-verbatim).
    #[test]
    fn sample_conversion_matches_documented_policy(
        spec in arb_valid_spec(),
        frames in 1usize..=8,
        ticks in arb_ticks(),
        samples in prop::collection::vec(arb_f32(), 0..=256),
    ) {
        let info = spec.stream_info();
        let channels = spec.layout.channels() as usize;
        let need = frames * channels;
        let interleaved: Vec<f32> = (0..need)
            .map(|index| {
                samples
                    .get(index % samples.len().max(1))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();
        let buffer = buffer_with(&spec, frames, &interleaved);
        let pts = MediaTimestamp::carried(MediaTime::new(
            ticks,
            spec.sample_rate.time_base(),
        ));

        let mut encoder = make_encoder(&info).expect("spec accepted");
        encoder
            .send_frame(Frame::Audio(AudioFrame::new(buffer, pts)))
            .expect("frame accepted");
        let packet = encoder
            .receive_packet()
            .expect("packet read")
            .expect("one packet per nonempty frame");

        let align = usize::try_from(block_align(&spec)).expect("align fits usize");
        prop_assert_eq!(packet.data.len(), frames * align);
        prop_assert_eq!(packet.duration, Some(frames as u64));
        prop_assert_eq!(packet.flags, PacketFlags::KEYFRAME);
        prop_assert_eq!(packet.stream, StreamId(0));
        prop_assert_eq!(packet.pts, Some(TickTimestamp::carried(ticks)));

        // Byte-exact model: interleaved little-endian, same order the
        // encoder writes (frame-major, channel-minor).
        let expected: Vec<u8> = interleaved
            .iter()
            .flat_map(|&value| model_sample_bytes(spec.sample_format, value))
            .collect();
        prop_assert_eq!(packet.data.len(), expected.len());
        prop_assert_eq!(packet.data.as_slice(), expected.as_slice());

        // Independent semantic invariants (not the byte model).
        let sample_bytes = usize::from(spec.sample_format.bits_per_sample()) / 8;
        for (index, &value) in interleaved.iter().enumerate() {
            let wire = &packet.data[index * sample_bytes..(index + 1) * sample_bytes];
            match spec.sample_format {
                SampleFormat::U8 => {
                    if value.is_nan() {
                        prop_assert_eq!(wire[0], 128, "NaN -> unsigned midpoint");
                    }
                    if value >= 1.0 {
                        prop_assert_eq!(wire[0], 255);
                    }
                    if value <= -1.0 {
                        prop_assert_eq!(wire[0], 0);
                    }
                }
                SampleFormat::S16 => {
                    check_signed_sample(
                        i64::from(i16::from_le_bytes([wire[0], wire[1]])),
                        value,
                        -32_768,
                        32_767,
                    )?;
                }
                SampleFormat::S24 => {
                    let sign = if wire[2] & 0x80 == 0 { 0 } else { 0xFF };
                    check_signed_sample(
                        i64::from(i32::from_le_bytes([wire[0], wire[1], wire[2], sign])),
                        value,
                        -8_388_608,
                        8_388_607,
                    )?;
                }
                SampleFormat::S32 => {
                    check_signed_sample(
                        i64::from(i32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]])),
                        value,
                        -2_147_483_648,
                        2_147_483_647,
                    )?;
                }
                SampleFormat::F32 => {
                    let want = value.to_le_bytes();
                    prop_assert_eq!(wire, want.as_slice(), "F32 verbatim");
                }
                other => unreachable!("{other:?} is not a wire format"),
            }
        }
    }

    /// The packet stream is exactly the frame stream with wire
    /// payloads: monotone index from 0, `duration == frames`,
    /// `KEYFRAME`, pts carried through `pts_ticks` (same-base exact,
    /// millisecond-base converted, both with `Carried` provenance),
    /// zero-frame buffers emit nothing, `drain` is sticky, and
    /// `receive_packet` reports `Ok(None)` forever at end of stream.
    #[test]
    fn encoder_packet_stream_is_the_frame_stream(
        spec in arb_valid_spec(),
        frames in prop::collection::vec((0usize..=16, arb_ticks(), any::<bool>()), 0..=6),
    ) {
        let info = spec.stream_info();
        let align = usize::try_from(block_align(&spec)).expect("align fits usize");
        let mut encoder = make_encoder(&info).expect("spec accepted");
        for &(count, ticks, ms_base) in &frames {
            let buffer = buffer_with(&spec, count, &[]);
            let base = if ms_base {
                ruviform_media_core::TimeBase::new(1, 1_000).expect("1/1000 base")
            } else {
                spec.sample_rate.time_base()
            };
            let pts = MediaTimestamp::carried(MediaTime::new(ticks, base));
            encoder
                .send_frame(Frame::Audio(AudioFrame::new(buffer, pts)))
                .expect("frame accepted");
        }
        encoder.drain().expect("drain");

        for (expected_index, &(count, ticks, ms_base)) in
            frames.iter().filter(|&&(c, _, _)| c > 0).enumerate()
        {
            let packet = encoder
                .receive_packet()
                .expect("packet read")
                .expect("packet per nonempty frame");
            let base = if ms_base {
                ruviform_media_core::TimeBase::new(1, 1_000).expect("1/1000 base")
            } else {
                spec.sample_rate.time_base()
            };
            prop_assert_eq!(packet.stream, StreamId(0));
            prop_assert_eq!(
                packet.index,
                u64::try_from(expected_index).expect("packet index fits u64")
            );
            prop_assert_eq!(packet.flags, PacketFlags::KEYFRAME);
            prop_assert_eq!(packet.duration, Some(count as u64));
            prop_assert_eq!(packet.data.len(), count * align);
            prop_assert_eq!(
                packet.pts,
                Some(TickTimestamp::carried(model_pts_ticks(
                    MediaTime::new(ticks, base),
                    spec.sample_rate,
                )))
            );
            prop_assert_eq!(packet.dts, None);
        }
        prop_assert!(encoder.receive_packet().expect("eos").is_none());
        prop_assert!(encoder.receive_packet().expect("eos repeats").is_none());
    }

    /// Frame-level rejections are `InvalidData` and never poison the
    /// encoder: a wrong channel count, a wrong sample rate, or a
    /// non-`Audio` (`NativeAudio`) frame each fails with the
    /// documented kind, after which a valid frame still encodes.
    /// `send_frame` after `drain` is `InvalidData` too.
    #[test]
    fn encoder_rejections_are_invalid_data_and_not_sticky(
        spec in arb_valid_spec(),
        case in 0u8..=3,
    ) {
        let info = spec.stream_info();
        let mut encoder = make_encoder(&info).expect("spec accepted");
        let valid_pts = MediaTimestamp::carried(MediaTime::new(0, spec.sample_rate.time_base()));
        let valid_frame = || Frame::Audio(AudioFrame::new(buffer_with(&spec, 4, &[0.25]), valid_pts));

        match case {
            0 => {
                // Channel count mismatch — deterministic different
                // count via a custom mask.
                let wrong_channels = if spec.layout.channels() >= 31 { 30 } else { spec.layout.channels() + 1 };
                let mask = SpeakerMask::from_bits((1u32 << wrong_channels) - 1);
                let wrong_layout = ChannelLayout::custom(mask).expect("nonempty mask");
                let mut buffer = AudioBuffer::new(BufferSpec {
                    layout: wrong_layout,
                    frames: 4,
                    sample_rate: spec.sample_rate,
                });
                buffer.channel_mut(0).expect("channel").fill(0.1);
                let result = encoder.send_frame(Frame::Audio(AudioFrame::new(buffer, valid_pts)));
                prop_assert_eq!(result.expect_err("mismatch rejected").kind(), ErrorKind::InvalidData);
                encoder.send_frame(valid_frame()).expect("encoder not poisoned");
            }
            1 => {
                // Sample-rate mismatch.
                let wrong_rate = SampleRate::new(spec.sample_rate.get() + 1).expect("rate + 1");
                let mut buffer = AudioBuffer::new(BufferSpec {
                    layout: spec.layout,
                    frames: 4,
                    sample_rate: wrong_rate,
                });
                buffer.channel_mut(0).expect("channel").fill(0.1);
                let result = encoder.send_frame(Frame::Audio(AudioFrame::new(buffer, valid_pts)));
                prop_assert_eq!(result.expect_err("mismatch rejected").kind(), ErrorKind::InvalidData);
                encoder.send_frame(valid_frame()).expect("encoder not poisoned");
            }
            2 => {
                // Non-Audio frame variant — the native-scale path is
                // not what the encoder consumes.
                let native = NativeAudioFrame::new(
                    AudioFormat {
                        sample_rate: spec.sample_rate,
                        layout: spec.layout,
                        sample_format: SampleFormat::S16,
                    },
                    vec![0.0; spec.layout.channels() as usize * 4],
                    TickTimestamp::carried(0),
                    Vec::new(),
                );
                let result = encoder.send_frame(Frame::NativeAudio(native));
                prop_assert_eq!(result.expect_err("native rejected").kind(), ErrorKind::InvalidData);
                encoder.send_frame(valid_frame()).expect("encoder not poisoned");
            }
            _ => {
                encoder.drain().expect("drain");
                let result = encoder.send_frame(valid_frame());
                prop_assert_eq!(
                    result.expect_err("post-drain rejected").kind(),
                    ErrorKind::InvalidData
                );
            }
        }
    }

    /// `write_packet` → `finalize` produces the only completion
    /// receipt: `total_bytes` is the payload sum, `total_frames` is
    /// `bytes / block_align`, `duration` sits in the `1/rate` base,
    /// `first_pts_ticks` is the first pts-carrying packet's ticks,
    /// the `data` chunk pads to even with a zero byte, `riff size`
    /// backpatches to `len - 8`, and `data size` to the payload. A
    /// second finalize is `InvalidData` and never retracts the
    /// receipt; `Muxer::finish` is the same checked path.
    #[test]
    fn muxer_finalize_receipt_backpatch_one_shot(
        spec in arb_valid_spec(),
        writes in prop::collection::vec(
            (0u16..=32, prop::option::of(arb_ticks())),
            0..=8,
        ),
        use_finish in any::<bool>(),
    ) {
        let info = spec.stream_info();
        let align = usize::try_from(block_align(&spec)).expect("align fits usize");
        let mut muxer =
            open_muxer(Cursor::new(Vec::new()), &[info]).expect("valid spec opens");
        let mut expected_bytes = 0_u64;
        for (index, &(frame_count, pts)) in writes.iter().enumerate() {
            let len = usize::from(frame_count) * align;
            muxer
                .write_packet(Packet {
                    stream: StreamId(0),
                    index: index as u64,
                    pts: pts.map(TickTimestamp::carried),
                    dts: None,
                    duration: Some(u64::from(frame_count)),
                    trim_start: 0,
                    trim_end: 0,
                    flags: PacketFlags::KEYFRAME,
                    data: vec![0xAB; len],
                })
                .expect("valid packet accepted");
            expected_bytes += len as u64;
        }

        let receipt = if use_finish {
            Muxer::finish(&mut muxer).expect("finish");
            *muxer.receipt().expect("finish yields a receipt")
        } else {
            muxer.finalize().expect("finalize")
        };
        let expected_frames = expected_bytes / block_align(&spec);
        prop_assert_eq!(receipt.total_bytes, expected_bytes);
        prop_assert_eq!(receipt.total_frames, expected_frames);
        prop_assert_eq!(
            receipt.duration.value(),
            i64::try_from(expected_frames).expect("frames fit i64")
        );
        prop_assert_eq!(receipt.duration.base(), spec.sample_rate.time_base());
        prop_assert_eq!(
            receipt.first_pts_ticks,
            writes.iter().find_map(|&(_, pts)| pts)
        );
        prop_assert_eq!(muxer.receipt(), Some(&receipt));

        // One-shot: later finalize/write fail closed, receipt stands.
        prop_assert_eq!(
            muxer.finalize().expect_err("second finalize").kind(),
            ErrorKind::InvalidData
        );
        prop_assert_eq!(
            muxer
                .write_packet(Packet {
                    stream: StreamId(0),
                    index: 99,
                    pts: None,
                    dts: None,
                    duration: None,
                    trim_start: 0,
                    trim_end: 0,
                    flags: PacketFlags::KEYFRAME,
                    data: vec![0; align],
                })
                .expect_err("post-finalize write")
                .kind(),
            ErrorKind::InvalidData
        );
        prop_assert_eq!(muxer.receipt(), Some(&receipt));

        let bytes = muxer.into_sink().into_inner();
        let pad = usize::from(expected_bytes % 2 == 1);
        let want_len = header_len(&spec)
            + usize::try_from(expected_bytes).expect("payload fits usize")
            + pad;
        prop_assert_eq!(bytes.len(), want_len, "file length = header + data + pad");
        prop_assert_eq!(
            u32le(&bytes[4..8]),
            u32::try_from(bytes.len() - 8).expect("riff size fits u32"),
            "riff size backpatched to len - 8"
        );
        let offset = data_size_offset(&spec);
        prop_assert_eq!(
            u64::from(u32le(&bytes[offset..offset + 4])),
            expected_bytes,
            "data size backpatched to payload bytes"
        );
        if pad == 1 {
            prop_assert_eq!(bytes[bytes.len() - 1], 0, "even-align pad is a zero byte");
        }
        // The backpatch touched only the size fields — the rest of
        // the header is byte-identical to the open-time skeleton.
        check_header_fields(&bytes[..header_len(&spec)], &spec)?;
    }

    /// Packet-level rejections are `InvalidData` and never poison the
    /// muxer: a foreign stream id or a payload that is not a whole
    /// number of frames fails with the documented kind, and the file
    /// still finalizes holding only the accepted bytes.
    #[test]
    fn muxer_rejections_are_invalid_data_and_not_sticky(
        spec in arb_valid_spec(),
        case in 0u8..=1,
        foreign_id in 1u32..=u32::MAX,
        extra in 1usize..=64,
    ) {
        let info = spec.stream_info();
        let align = usize::try_from(block_align(&spec)).expect("align fits usize");
        let mut muxer =
            open_muxer(Cursor::new(Vec::new()), &[info]).expect("valid spec opens");
        let rejected = Packet {
            stream: StreamId(0),
            index: 0,
            pts: None,
            dts: None,
            duration: None,
            trim_start: 0,
            trim_end: 0,
            flags: PacketFlags::KEYFRAME,
            data: Vec::new(),
        };
        let bad_packet = if case == 0 {
            Packet {
                stream: StreamId(foreign_id),
                data: vec![0; align],
                ..rejected
            }
        } else {
            prop_assume!(align > 1, "block_align 1 cannot make a partial frame");
            Packet {
                // `align * k + r` with `0 < r < align`.
                data: vec![0; align + 1 + (extra % (align - 1))],
                ..rejected
            }
        };
        prop_assert_eq!(
            muxer.write_packet(bad_packet).expect_err("bad packet rejected").kind(),
            ErrorKind::InvalidData
        );

        // Not poisoned: a valid packet still writes and lands in the
        // file, and the rejected packet contributed nothing.
        let good = vec![0x5A; align * 2];
        muxer
            .write_packet(Packet {
                stream: StreamId(0),
                index: 1,
                pts: None,
                dts: None,
                duration: Some(2),
                trim_start: 0,
                trim_end: 0,
                flags: PacketFlags::KEYFRAME,
                data: good.clone(),
            })
            .expect("muxer not poisoned");
        let receipt = muxer.finalize().expect("finalize");
        prop_assert_eq!(receipt.total_bytes, good.len() as u64);
        prop_assert_eq!(receipt.total_frames, 2);
        let bytes = muxer.into_sink().into_inner();
        let body = &bytes[header_len(&spec)..header_len(&spec) + good.len()];
        prop_assert_eq!(body, good.as_slice());
    }

    /// End-to-end: every named layout × wire format encodes, muxes,
    /// finalizes, and reopens through the Symphonia reader — the
    /// written file is a real WAV that decodes to the same channel
    /// geometry, frame count, and samples within one quantization
    /// LSB (F32 is bit-faithful modulo decode epsilon). `Custom`
    /// masks stay out of this property deliberately — a generic
    /// reader is not required to interpret arbitrary masks.
    #[test]
    fn wav_roundtrip_decodes_through_symphonia(
        rate_value in prop::sample::select(vec![8_000u32, 44_100, 48_000, 96_000]),
        layout in arb_named_layout(),
        format in arb_wire_format(),
        frames in 1usize..=128,
        samples in prop::collection::vec(-1.5f32..=1.5, 1..=4096),
    ) {
        let spec = WavOutputSpec {
            sample_rate: SampleRate::new(rate_value).expect("nonzero rate"),
            layout,
            sample_format: format,
        };
        let info = spec.stream_info();
        let channels = layout.channels() as usize;
        let interleaved: Vec<f32> = (0..frames * channels)
            .map(|index| samples[index % samples.len()])
            .collect();
        let buffer = buffer_with(&spec, frames, &interleaved);
        let frame = Frame::Audio(AudioFrame::new(
            buffer,
            MediaTimestamp::carried(MediaTime::new(0, spec.sample_rate.time_base())),
        ));

        let (bytes, receipt) = encode_to_file(&info, vec![frame]);
        prop_assert_eq!(receipt.total_frames, frames as u64);

        let (decoded, decoded_frames, planes) = decode_planes(&bytes);
        prop_assert_eq!(decoded_frames, frames);
        prop_assert_eq!(planes.len(), channels);
        if let Fact::Known(CodecFormat::Audio(native)) = decoded.codec.native_format
        {
            prop_assert_eq!(native.sample_rate, spec.sample_rate);
            prop_assert_eq!(native.layout.channels(), spec.layout.channels());
        }
        let tol = tolerance(format);
        for channel in 0..channels {
            for frame_index in 0..frames {
                let source = interleaved[frame_index * channels + channel];
                let expected = if format == SampleFormat::F32 {
                    source
                } else {
                    source.clamp(-1.0, 1.0)
                };
                let actual = planes[channel][frame_index];
                prop_assert!(
                    (expected - actual).abs() <= tol,
                    "ch {channel} frame {frame_index}: expected {expected}, got {actual} (tol {tol})"
                );
            }
        }
    }
}

/// A bounded proof of the `open_muxer` public-boundary contract,
/// verified exhaustively over symbolic spec fields by Kani
/// (`cargo kani --tests`; run via `mise run test-formal-kani`). The
/// `pub(crate)` conversion/serialization proofs live in-source
/// (`src/convert.rs`, `src/wav.rs`); bounds and assumptions are
/// documented in `docs/specs/formal-verification.md`.
///
/// Harness shape (slice-4 environment lesson, building on slices
/// 1–3): a fully concrete spec matrix on a concrete
/// (`Cursor`) skeleton — no symbolic streams or containers. A
/// symbolic `WavOutputSpec` was infeasible on this WSL2 box:
/// `SampleRate::new` normalizes via `TimeBase::new`'s Euclid
/// `while` loop whose nested symbolic `%` unrolls exhaust CBMC
/// memory. `Wire::new` now delegates its numeric/membership decision to
/// the pure scalar core in `src/wav.rs`; there is no `cfg(kani)`
/// substitution of production diagnostics. The concrete success matrix
/// never reaches the unsupported-diagnostic shell, while the decision
/// proofs avoid shell/error machinery entirely. Each `open_muxer`
/// unroll still costs ~1-2
/// GiB of SSA (std `Cursor`/`Write`/`io::Error` machinery plus the
/// stream-declaration builder), so the matrices below split per
/// outcome class: the PCM-form and extensible-form success + stream
/// declaration in `open_muxer_spec_contract_{pcm,extensible}`.
/// The byte-rate overflow rejection is proven at the `Wire::new`
/// gate by `wav_decl_gate_byte_rate_boundary_*` in `src/wav.rs` — its
/// boundary matrix covers this exact spec (7.1 F32 at
/// `u32::MAX / 32 + 1`). The composite Err-direction formula
/// exceeds the reference WSL2 envelope at *every* depth tried:
/// `open_muxer` pinned at ~31.7 GiB across 6 variants (slice 4),
/// and a residual-slice redesign that proved `validate_stream`
/// alone still pinned at ~31.9 GiB — the gate's own error-arm SSA
/// (`Error::with_source` + `format!`/`io::Error` machinery), not
/// `open_muxer`'s post-gate CFG, is the dominant cost. The `?`
/// propagation to the public entry therefore stays
/// proptest-covered
/// (`muxer_rejections_are_invalid_data_and_not_sticky`). The
/// 16-byte `fmt ` success path is proven at the serialization
/// layer by `wav_decl_gate_fmt_and_header_layout`; its `open_muxer` glue is
/// identical to the extensible form's and stays proptest-covered
/// (`open_muxer_writes_complete_header`).
#[cfg(kani)]
mod proofs {
    use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate, SpeakerMask};
    use ruviform_io_wav::{WavOutputSpec, open_muxer};
    use ruviform_media_core::{StreamId, StreamKind};
    use ruviform_media_io::{ErrorKind, Muxer};
    use std::io::{Seek, SeekFrom, Write};

    /// Fixed-buffer `Write + Seek` sink for the proofs. 80 bytes
    /// covers the largest open-time header (68-byte extensible
    /// form); no heap, no `Vec` splice/growth, trivial position
    /// arithmetic — the std `Cursor<Vec<u8>>` machinery is what made
    /// a single `open_muxer` unroll cost ~1-2 GiB of SSA on the
    /// reference WSL2 box.
    struct KaniSink {
        buf: [u8; 80],
        pos: usize,
    }

    impl KaniSink {
        fn bytes(&self) -> &[u8] {
            &self.buf[..self.pos]
        }
    }

    impl Write for KaniSink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let end = self.pos + bytes.len();
            if end > self.buf.len() {
                return Err(std::io::ErrorKind::WriteZero.into());
            }
            self.buf[self.pos..end].copy_from_slice(bytes);
            self.pos = end;
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for KaniSink {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            let next = match pos {
                SeekFrom::Start(n) => n as usize,
                SeekFrom::Current(n) => (self.pos as i64 + n) as usize,
                SeekFrom::End(n) => (self.buf.len() as i64 + n) as usize,
            };
            if next > self.buf.len() {
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
            self.pos = next;
            Ok(next as u64)
        }
    }

    /// For every concrete `WavOutputSpec` in the matrix,
    /// `open_muxer` either succeeds with a sink carrying a
    /// structurally complete RIFF/WAVE header plus a normalized
    /// stream declaration, or fails `InvalidValue` — the only
    /// rejection this wire-format domain can reach. Never another
    /// kind, never a panic. `unwind(96)` covers the builtin
    /// memmove loops inside the sink writes of the 68-byte header.
    ///
    /// Split into two disjoint success proofs — like the declaration-gate
    /// gate proofs, the split is solver-RAM, not semantics: each
    /// `open_muxer` unroll encodes its internal diagnostic paths,
    /// and the single 6-spec matrix exhausted the reference WSL2
    /// service (~2.5 GiB RSS before `Wsl/Service/E_UNEXPECTED`).
    /// The rejection half is covered at the `Wire::new` gate
    /// (`wav_decl_gate_byte_rate_boundary_*`); the composite `open_muxer`
    /// Err-direction proof stays out of the lane — it never
    /// produces a verdict in-envelope (see the module doc).
    fn check_open_muxer(specs: &[WavOutputSpec]) {
        for spec in specs {
            let info = spec.stream_info();
            match open_muxer(
                KaniSink {
                    buf: [0; 80],
                    pos: 0,
                },
                &[info],
            ) {
                Ok(muxer) => {
                    let declared = &muxer.streams()[0];
                    assert!(declared.stream.id == StreamId(0));
                    assert!(declared.stream.kind == StreamKind::Audio);
                    assert!(
                        declared.stream.time_base.get() == Some(&spec.sample_rate.time_base()),
                        "container time base normalizes to 1/rate"
                    );
                    let sink = muxer.into_sink();
                    let bytes = sink.bytes();
                    assert!(bytes.len() == 44 || bytes.len() == 68);
                    // Per-byte marker checks — slice equality routes
                    // through the memcmp builtin with byte-level
                    // unrolling; direct indexing constant-folds.
                    assert!(
                        bytes[0] == b'R'
                            && bytes[1] == b'I'
                            && bytes[2] == b'F'
                            && bytes[3] == b'F'
                    );
                    assert!(
                        bytes[8] == b'W'
                            && bytes[9] == b'A'
                            && bytes[10] == b'V'
                            && bytes[11] == b'E'
                    );
                    assert!(
                        bytes[12] == b'f'
                            && bytes[13] == b'm'
                            && bytes[14] == b't'
                            && bytes[15] == b' '
                    );
                    let body_len =
                        u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
                    assert!(body_len == 16 || body_len == 40);
                    assert!(bytes.len() == 28 + body_len);
                    let data_at = 20 + body_len;
                    assert!(
                        bytes[data_at] == b'd'
                            && bytes[data_at + 1] == b'a'
                            && bytes[data_at + 2] == b't'
                            && bytes[data_at + 3] == b'a'
                    );
                }
                Err(error) => {
                    assert!(error.kind() == ErrorKind::InvalidValue);
                }
            }
        }
    }

    /// Non-extensible (`WAVEFORMATEX`) success — the 16-byte `fmt `
    /// header form through `open_muxer` itself, on `KaniSink`.
    #[kani::proof]
    #[kani::unwind(96)]
    fn open_muxer_spec_contract_pcm() {
        check_open_muxer(&[WavOutputSpec {
            sample_rate: SampleRate::new(48_000).expect("nonzero rate"),
            layout: ChannelLayout::Mono,
            sample_format: SampleFormat::U8,
        }]);
    }

    /// Extensible (`WAVEFORMATEXTENSIBLE`) success cases — the
    /// arbitrary-mask layout exercises `cbSize`/speaker-mask/GUID.
    #[kani::proof]
    #[kani::unwind(96)]
    fn open_muxer_spec_contract_extensible() {
        check_open_muxer(&[
            WavOutputSpec {
                sample_rate: SampleRate::new(96_000).expect("nonzero rate"),
                layout: ChannelLayout::FivePointOne,
                sample_format: SampleFormat::S24,
            },
            WavOutputSpec {
                sample_rate: SampleRate::new(192_000).expect("nonzero rate"),
                layout: ChannelLayout::SevenPointOne,
                sample_format: SampleFormat::F32,
            },
            WavOutputSpec {
                sample_rate: SampleRate::new(8_000).expect("nonzero rate"),
                layout: ChannelLayout::custom(SpeakerMask::from_bits(0x0607))
                    .expect("non-empty mask"),
                sample_format: SampleFormat::S32,
            },
        ]);
    }
}
