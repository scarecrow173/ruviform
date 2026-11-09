//! WAV → `StagedFile` byte-exact golden fixtures (RF305 slice).
//!
//! `wav_staged.rs` proves the hand-off *order* (finalize → borrow
//! release → seal → independent decode → publish → reopen); this
//! file pins the *bytes*: the published destination must equal a
//! RIFF/WAVE file assembled independently from the specification —
//! chunk layout, LE fields, size backpatch, pad rule — not merely
//! decode to the same payload. Encoder-path edge samples pin the
//! declared `f32 → wire` conversion policy end to end, and the 4 GiB
//! boundary is exercised on a discard sink so no real 4 GiB file is
//! ever created.

mod common;

use std::io::{Seek, SeekFrom, Write};

use common::wav::{
    edge_samples, expected_edge_payload, expected_wav_bytes, independent_decode, packet,
    s16_payload, s16_stereo_spec, spec_for,
};
use common::{TestDir, read};
use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleFormat};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
use ruviform_io_wav::{WavMuxer, WavOutputSpec, make_encoder};
use ruviform_media_core::{MediaTime, MediaTimestamp, TimeBase};
use ruviform_media_io::{AudioFrame, ContainerFormat, Encoder, Frame, Muxer};

const S16_STEREO_BLOCK_ALIGN: usize = 4; // 2ch * 2B

/// Publish options shared by the golden fixtures.
fn publish_options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

/// Runs the full `WavMuxer → &mut StagedFile → finalize → seal →
/// publish` sequence for `spec` and the given packets, returning the
/// published destination bytes.
fn publish_wav(
    dir: &TestDir,
    name: &str,
    spec: &WavOutputSpec,
    packets: Vec<ruviform_media_io::Packet>,
) -> Vec<u8> {
    let dest = dir.dest(name);
    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut muxer = WavMuxer::new(&mut staged, &[spec.stream_info()]).expect("muxer");
        for pkt in packets {
            muxer.write_packet(pkt).expect("write packet");
        }
        muxer.finalize().expect("finalize");
        let _staged_ref = muxer.into_sink();
    }
    staged.seal().publish(publish_options()).expect("publish");
    read(&dest).expect("published bytes")
}

/// Zero frames: header-only WAV must be byte-exact — RIFF size 36,
/// `fmt ` = 16-byte `WAVEFORMATEX`, `data` size 0.
#[test]
fn zero_frames_wav_is_byte_exact_waveformatex() {
    let dir = TestDir::new("wav-golden-zero");
    let spec = s16_stereo_spec();
    let payload: Vec<u8> = Vec::new();

    let published = publish_wav(&dir, "empty.wav", &spec, Vec::new());

    let expected = expected_wav_bytes(&spec, &payload);
    assert_eq!(
        published, expected,
        "zero-frame WAV must be the exact 44-byte RIFF/WAVE header"
    );
    assert_eq!(published.len(), 44);
    let (format, decoded) = independent_decode(&published).expect("independent decode");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(decoded.len(), 0, "zero frames decode to no payload");
}

/// S16 stereo payload: the published file must equal header + the
/// exact packet bytes, and the independent decoder must recover the
/// same payload bytes.
#[test]
fn s16_payload_is_byte_exact_and_decodes() {
    let dir = TestDir::new("wav-golden-s16");
    let spec = s16_stereo_spec();
    let frames = 1_024;
    let payload = s16_payload(frames, 2);

    let published = publish_wav(
        &dir,
        "tone.wav",
        &spec,
        vec![packet(0, S16_STEREO_BLOCK_ALIGN, payload.clone())],
    );

    let expected = expected_wav_bytes(&spec, &payload);
    assert_eq!(published, expected, "published WAV must be byte-exact");
    let (format, decoded) = independent_decode(&published).expect("independent decode");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(decoded, payload, "decoded payload must match byte-exact");
}

/// A non-Mono/Stereo layout emits `WAVEFORMATEXTENSIBLE`: 40-byte
/// `fmt ` body with `wFormatTag` 0xFFFE, `cbSize` 22, the layout's
/// speaker mask and the PCM subtype GUID.
#[test]
fn five_point_one_layout_is_byte_exact_waveformatextensible() {
    let dir = TestDir::new("wav-golden-51");
    let spec = spec_for(SampleFormat::S16, ChannelLayout::FivePointOne);
    let payload: Vec<u8> = Vec::new();

    let published = publish_wav(&dir, "surround.wav", &spec, Vec::new());

    let expected = expected_wav_bytes(&spec, &payload);
    assert_eq!(
        published, expected,
        "5.1 WAV must carry the extensible fmt chunk"
    );
    assert_eq!(published.len(), 68, "extensible header is 68 bytes");
    // Spot-check the fields the golden builder derived: tag, cbSize,
    // channel mask (5.1 = FL|FR|FC|LFE|BL|BR = 0x3F), PCM subtype.
    assert_eq!(&published[20..22], &0xFFFE_u16.to_le_bytes());
    assert_eq!(&published[36..38], &22_u16.to_le_bytes());
    assert_eq!(&published[40..44], &0x3F_u32.to_le_bytes());
    assert_eq!(&published[44..48], &[0x01, 0x00, 0x00, 0x00]);
    assert_eq!(
        &published[48..60],
        &[
            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71
        ]
    );
    let (format, decoded) = independent_decode(&published).expect("independent decode");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(decoded.len(), 0);
}

/// The `f32 → wire` policy pinned against *literal* bytes — this
/// validates the independent oracle itself so the encoder golden
/// below cannot drift with the implementation.
#[test]
fn wire_sample_policy_literals() {
    use ruviform_audio_core::SampleFormat as F;
    let cases: &[(F, f32, &[u8])] = &[
        // NaN → 0, ±Inf → full scale, out-of-range clamps.
        (F::U8, f32::NAN, &[128]),
        (F::U8, f32::INFINITY, &[255]),
        (F::U8, f32::NEG_INFINITY, &[0]),
        (F::U8, 2.0, &[255]),
        (F::U8, -2.0, &[0]),
        // .5-LSB rounding — half away from zero.
        (F::U8, 1.0 / 256.0, &[129]),  // 128.5 → 129
        (F::U8, -1.0 / 256.0, &[128]), // 127.5 → 128
        (F::S16, 1.5 / 32_768.0, &[0x02, 0x00]),
        (F::S16, -1.5 / 32_768.0, &[0xFE, 0xFF]),
        (F::S24, 1.5 / 8_388_608.0, &[0x02, 0x00, 0x00]),
        (F::S32, 1.5 / 2_147_483_648.0, &[0x02, 0x00, 0x00, 0x00]),
        (F::S16, 0.5, &[0x00, 0x40]),
        (F::S16, -0.5, &[0x00, 0xC0]),
        (F::S16, 1.0, &[0xFF, 0x7F]),
        (F::S16, -1.0, &[0x00, 0x80]),
        (F::S16, f32::INFINITY, &[0xFF, 0x7F]),
        (F::S16, f32::NEG_INFINITY, &[0x00, 0x80]),
        (F::S16, f32::NAN, &[0x00, 0x00]),
        (F::S24, 0.5, &[0x00, 0x00, 0x40]),
        (F::S24, -0.5, &[0x00, 0x00, 0xC0]),
        (F::S24, 1.0, &[0xFF, 0xFF, 0x7F]),
        (F::S32, 0.5, &[0x00, 0x00, 0x00, 0x40]),
        (F::S32, -0.5, &[0x00, 0x00, 0x00, 0xC0]),
        (F::S32, 1.0, &[0xFF, 0xFF, 0xFF, 0x7F]),
        (F::S32, -1.0, &[0x00, 0x00, 0x00, 0x80]),
        (F::S32, f32::INFINITY, &[0xFF, 0xFF, 0xFF, 0x7F]),
        (F::S32, f32::NEG_INFINITY, &[0x00, 0x00, 0x00, 0x80]),
        (F::S32, f32::NAN, &[0x00, 0x00, 0x00, 0x00]),
        // F32 is verbatim IEEE-754 — NaN/Inf/out-of-range preserved.
        (F::F32, 0.5, &[0x00, 0x00, 0x00, 0x3F]),
        (F::F32, -1.0, &[0x00, 0x00, 0x80, 0xBF]),
        (F::F32, 2.0, &[0x00, 0x00, 0x00, 0x40]),
        (F::F32, -2.0, &[0x00, 0x00, 0x00, 0xC0]),
        (F::F32, f32::NAN, &[0x00, 0x00, 0xC0, 0x7F]),
        (F::F32, f32::INFINITY, &[0x00, 0x00, 0x80, 0x7F]),
        (F::F32, f32::NEG_INFINITY, &[0x00, 0x00, 0x80, 0xFF]),
    ];
    for &(format, value, expected) in cases {
        assert_eq!(
            common::wav::expected_wire_sample(format, value),
            expected,
            "{format:?} {value} must encode to {expected:?}"
        );
    }
}

/// Encoder path: `WavEncoder` converts canonical `f32` frames to the
/// declared wire format; the published file must be byte-exact
/// against the independent oracle for every wire format, and the
/// independent decoder must recover the same payload.
#[test]
fn encoder_edge_samples_are_byte_exact_per_wire_format() {
    let dir = TestDir::new("wav-golden-enc");
    let samples = edge_samples();
    let frames = samples.len();

    for format in [
        SampleFormat::U8,
        SampleFormat::S16,
        SampleFormat::S24,
        SampleFormat::S32,
        SampleFormat::F32,
    ] {
        let spec = spec_for(format, ChannelLayout::Mono);
        let mut encoder = make_encoder(&spec.stream_info()).expect("encoder");
        let buffer = AudioBuffer::from_interleaved(
            &samples,
            BufferSpec {
                layout: ChannelLayout::Mono,
                frames,
                sample_rate: spec.sample_rate,
            },
        )
        .expect("buffer");
        encoder
            .send_frame(Frame::Audio(AudioFrame::new(
                buffer,
                MediaTimestamp::carried(MediaTime::new(0, TimeBase::COMMON_90K)),
            )))
            .expect("send frame");
        encoder.drain().expect("drain");
        let mut packets = Vec::new();
        while let Some(pkt) = encoder.receive_packet().expect("receive") {
            packets.push(pkt);
        }
        assert_eq!(packets.len(), 1, "{format:?}: one packet per frame buffer");

        let published = publish_wav(&dir, &format!("{format:?}.wav"), &spec, packets);
        let expected_payload = expected_edge_payload(format, &samples);
        let expected = expected_wav_bytes(&spec, &expected_payload);
        assert_eq!(
            published, expected,
            "{format:?}: published WAV must be byte-exact"
        );
        let (decoded_format, decoded) = independent_decode(&published).expect("independent decode");
        assert_eq!(decoded_format, ContainerFormat::Wav);
        assert_eq!(
            decoded, expected_payload,
            "{format:?}: decoded payload must match the wire bytes"
        );
    }
}

/// Odd-length data chunks get the RIFF pad byte: U8 mono has
/// `block_align` 1 so a 3-byte payload exercises the rule end to end.
#[test]
fn odd_payload_gets_pad_byte() {
    let dir = TestDir::new("wav-golden-pad");
    let spec = spec_for(SampleFormat::U8, ChannelLayout::Mono);
    let payload = vec![0x80_u8, 0x40, 0xC0];

    let published = publish_wav(&dir, "odd.wav", &spec, vec![packet(0, 1, payload.clone())]);

    let expected = expected_wav_bytes(&spec, &payload);
    assert_eq!(published, expected, "odd payload must gain a zero pad byte");
    assert_eq!(published.len(), 44 + 3 + 1);
    // data_size field excludes the pad byte.
    assert_eq!(&published[40..44], &3_u32.to_le_bytes());
    assert_eq!(&published[44..47], &payload[..]);
    assert_eq!(published[47], 0, "pad byte is zero");
    let (_, decoded) = independent_decode(&published).expect("independent decode");
    assert_eq!(decoded, payload);
}

/// Discard sink tracking `len`/`pos` like a real file — exercises
/// the classic RIFF 4 GiB boundary without materializing the bytes.
struct DevNull {
    /// High-water mark of written bytes = `seek(End)` position.
    len: u64,
    /// Current cursor position.
    pos: u64,
}

impl DevNull {
    fn new() -> Self {
        Self { len: 0, pos: 0 }
    }
}

impl Write for DevNull {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pos += u64::try_from(buf.len()).expect("len fits u64");
        self.len = self.len.max(self.pos);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for DevNull {
    fn seek(&mut self, target: SeekFrom) -> std::io::Result<u64> {
        let base = match target {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.pos) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.len) + i128::from(n),
        };
        let next = u64::try_from(base)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek below 0"))?;
        self.pos = next;
        Ok(next)
    }
}

const CHUNK: usize = 64 * 1024 * 1024; // 64 MiB per packet

/// Largest data chunk the classic RIFF container can still finalize:
/// `data_bytes` must be a whole number of S16-stereo frames and keep
/// `file_len - 8` (the RIFF size field) within `u32`.
/// 63 × 64 MiB + tail = `4_294_967_256` bytes; `file_len` = 44 +
/// that, `riff_size` = `4_294_967_292` ≤ `u32::MAX`.
const MAX_DATA_BYTES: u64 = 4_294_967_256;

/// Writes whole-frame S16-stereo packets totalling exactly
/// [`MAX_DATA_BYTES`] of payload — 63 × [`CHUNK`] plus a tail packet.
fn fill_to_boundary(muxer: &mut WavMuxer<DevNull>) {
    let full = usize::try_from(MAX_DATA_BYTES / u64::try_from(CHUNK).expect("chunk"))
        .expect("packet count");
    for i in 0..full {
        muxer
            .write_packet(packet(
                u64::try_from(i).expect("index"),
                S16_STEREO_BLOCK_ALIGN,
                vec![0_u8; CHUNK],
            ))
            .expect("boundary packet");
    }
    let tail = usize::try_from(MAX_DATA_BYTES - u64::try_from(full * CHUNK).expect("bytes"))
        .expect("tail fits usize");
    muxer
        .write_packet(packet(
            u64::try_from(full).expect("index"),
            S16_STEREO_BLOCK_ALIGN,
            vec![0_u8; tail],
        ))
        .expect("tail packet");
}

/// Positive boundary: the largest legal data chunk finalizes and
/// yields a receipt — proving the limit check is exact, not
/// conservative.
#[test]
fn data_chunk_at_classic_riff_limit_finalizes() {
    let spec = s16_stereo_spec();
    let mut muxer = WavMuxer::new(DevNull::new(), &[spec.stream_info()]).expect("muxer");
    fill_to_boundary(&mut muxer);
    let receipt = muxer.finalize().expect("finalize at RIFF limit");
    assert_eq!(receipt.total_bytes, MAX_DATA_BYTES);
    assert_eq!(
        receipt.total_frames,
        MAX_DATA_BYTES / u64::try_from(S16_STEREO_BLOCK_ALIGN).expect("align")
    );
}

/// Negative boundary: one more whole frame pushes `data_bytes` past
/// what the `data` size field can declare → `write_packet` rejects
/// with `InvalidData`; `finalize` independently rejects because the
/// `RIFF` size field would overflow `u32`. No receipt exists either
/// way — RF64 is honestly out of scope.
#[test]
fn data_chunk_past_classic_riff_limit_is_rejected() {
    let spec = s16_stereo_spec();
    let mut muxer = WavMuxer::new(DevNull::new(), &[spec.stream_info()]).expect("muxer");
    fill_to_boundary(&mut muxer);
    // +4 bytes keeps data_bytes ≤ u32::MAX, so the write itself is
    // accepted — the *finalize* RIFF-size check is what must reject.
    muxer
        .write_packet(packet(64, S16_STEREO_BLOCK_ALIGN, vec![0_u8; 4]))
        .expect("write still inside u32 data limit");
    // +64 more bytes crosses u32::MAX on the data chunk itself.
    let err = muxer
        .write_packet(packet(65, S16_STEREO_BLOCK_ALIGN, vec![0_u8; 64]))
        .expect_err("data chunk past u32::MAX must be rejected");
    assert_eq!(err.kind(), ruviform_media_io::ErrorKind::InvalidData);
    let err = muxer
        .finalize()
        .expect_err("RIFF size overflow must fail finalize");
    assert_eq!(err.kind(), ruviform_media_io::ErrorKind::InvalidData);
    assert!(
        muxer.receipt().is_none(),
        "no completion receipt may exist past the boundary"
    );
}
