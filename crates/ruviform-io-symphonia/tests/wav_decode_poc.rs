//! End-to-end proof of concept: an in-memory synthesized WAV stream driven through
//! the media-io boundary — `open_demuxer` → `read_packet` →
//! `make_decoder` → `send_packet`/`receive_frame`.
//!
//! The fixture is a plain RIFF/WAVE header built byte-by-byte in the
//! test — no LFS objects, no external files. Sample values are chosen
//! so the i16→f32 conversion is exact (`s / 32768`, a dyadic rational),
//! letting the assertions compare decoded samples for equality rather
//! than tolerance.

use std::io::Cursor;

use ruviform_audio_core::{ChannelLayout, SampleFormat};
use ruviform_media_core::{
    Fact, MediaDuration, MediaTime, StreamId, StreamKind, TimeBase, TimestampProvenance,
};
use ruviform_media_io::{
    CodecFormat, CodecId, ContainerFormat, Decoder, Demuxer, ErrorKind, Frame, PcmTransform,
};

/// Nominal sample rate of the synthesized stream.
const RATE: u32 = 48_000;
/// Stereo.
const CHANNELS: u16 = 2;
/// Bits per sample — signed 16-bit little-endian PCM.
const BITS: u16 = 16;
/// Total frames in the stream: two full 1152-frame packets plus a tail.
const FRAMES: usize = 2_500;
/// The backend's WAV packetization cap — frames per packet.
const PACKET_FRAMES: usize = 1_152;

/// Deterministic left-channel sample at frame `i` — exact in f32.
fn sample_left(i: usize) -> i16 {
    i16::try_from(i % 89).unwrap() * 256 - 8192
}

/// Deterministic right-channel sample at frame `i` — the mirror of the
/// left channel, so a planar/interleaved mix-up is detectable.
fn sample_right(i: usize) -> i16 {
    8192 - i16::try_from(i % 89).unwrap() * 256
}

/// Builds a complete RIFF/WAVE byte stream in memory.
fn wav_bytes(frames: usize) -> Vec<u8> {
    let block_align = CHANNELS * (BITS / 8);
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    // fmt chunk — 16-byte PCM descriptor.
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // format tag: PCM
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    // data chunk — interleaved little-endian samples.
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        bytes.extend_from_slice(&sample_left(i).to_le_bytes());
        bytes.extend_from_slice(&sample_right(i).to_le_bytes());
    }
    bytes
}

/// Extracts the audio frame payload or fails the test.
fn audio_payload(frame: &Frame) -> &ruviform_media_io::AudioFrame {
    match frame {
        Frame::Audio(audio) => audio,
        other => unreachable!("expected an audio frame, got {other:?}"),
    }
}

#[test]
fn demux_enumerates_the_wav_stream() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();

    let streams = demuxer.streams();
    assert_eq!(streams.len(), 1);
    let info = &streams[0];
    assert_eq!(info.stream.kind, StreamKind::Audio);
    assert_eq!(info.codec.codec, CodecId::Pcm);
    assert_eq!(
        info.stream.time_base,
        Fact::known(TimeBase::sample_rate(RATE).unwrap()),
        "WAV stream time base must be 1/rate"
    );
    assert_eq!(
        info.stream.duration.map(MediaDuration::value),
        Some(i64::try_from(FRAMES).unwrap())
    );
    match &info.codec.format {
        CodecFormat::Audio(format) => {
            assert_eq!(format.sample_rate.get(), RATE);
            assert_eq!(format.layout, ChannelLayout::Stereo);
            assert_eq!(format.sample_format, SampleFormat::F32);
        }
        other => unreachable!("expected an audio codec format, got {other:?}"),
    }

    // No stream lookup misses / spurious extra streams.
    assert!(demuxer.stream(info.stream.id).is_some());
    assert!(demuxer.stream(StreamId(99)).is_none());
}

#[test]
fn packets_carry_stream_pts_and_duration() {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let stream = demuxer.streams()[0].stream.id;

    let mut packets = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        packets.push(packet);
    }
    // 2500 frames split at the 1152-frame packetization cap → 3 packets.
    assert_eq!(packets.len(), 3);

    let tail = FRAMES - 2 * PACKET_FRAMES;
    let expected = [
        (0_i64, PACKET_FRAMES),
        (i64::try_from(PACKET_FRAMES).unwrap(), PACKET_FRAMES),
        (i64::try_from(2 * PACKET_FRAMES).unwrap(), tail),
    ];
    for (expected_index, (packet, (pts, frames))) in packets.iter().zip(expected).enumerate() {
        assert_eq!(packet.stream, stream);
        assert_eq!(packet.index, expected_index as u64);
        // Timestamps stay raw ticks with Carried provenance — they
        // resolve against the stream's declared 1/rate base.
        let pts_ts = packet.pts.unwrap();
        assert_eq!(pts_ts.ticks, pts);
        assert_eq!(pts_ts.provenance, TimestampProvenance::Carried);
        assert_eq!(packet.dts.unwrap().ticks, pts);
        assert_eq!(packet.duration, Some(frames as u64));
        assert_eq!((packet.trim_start, packet.trim_end), (0, 0));
        assert!(
            packet.flags.is_empty(),
            "backend reports no packet attributes"
        );
        assert_eq!(packet.data.len(), frames * 4); // 2ch × s16
    }
}

#[test]
fn decode_yields_planar_f32_frames_with_progressing_pts() {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut decoder = demuxer.make_decoder(stream).unwrap();

    let mut frames = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            frames.push(frame);
        }
    }
    decoder.drain().unwrap();
    assert!(
        decoder.receive_frame().unwrap().is_none(),
        "queue drained to empty"
    );

    assert_eq!(frames.len(), 3);

    let mut expected = 0_usize; // running global frame index
    for (index, frame) in frames.iter().enumerate() {
        let audio = audio_payload(frame);
        let count = audio.buffer().frames();
        // PTS advances by exactly the previous frame's duration — the
        // stream's 1/rate time base, provenance carried through.
        assert_eq!(audio.pts().time.value(), i64::try_from(expected).unwrap());
        assert_eq!(
            audio.pts().time.base(),
            TimeBase::sample_rate(RATE).unwrap()
        );
        assert_eq!(audio.pts().provenance, TimestampProvenance::Carried);
        assert_eq!(
            audio.duration().map(MediaDuration::value),
            Some(i64::try_from(count).unwrap())
        );

        // Planar f32: one Vec per channel, channel-major.
        let buffer = audio.buffer();
        assert_eq!(buffer.channels(), 2);
        assert_eq!(buffer.layout(), ChannelLayout::Stereo);
        assert_eq!(buffer.sample_rate().get(), RATE);
        let left = buffer.channel(0).unwrap();
        let right = buffer.channel(1).unwrap();
        assert_eq!(left.len(), count);
        assert_eq!(right.len(), count);
        for (offset, (&l, &r)) in left.iter().zip(right.iter()).enumerate() {
            let global = expected + offset;
            assert_eq!(
                l,
                f32::from(sample_left(global)) / 32768.0,
                "L ch mismatch at frame {global}"
            );
            assert_eq!(
                r,
                f32::from(sample_right(global)) / 32768.0,
                "R ch mismatch at frame {global}"
            );
        }
        expected += count;
        let _ = index;
    }
    assert_eq!(expected, FRAMES);

    // flush resets the pump state.
    decoder.flush().unwrap();
    assert!(decoder.receive_frame().unwrap().is_none());
}

#[test]
fn decoder_rejects_packets_for_other_streams() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut decoder = demuxer.make_decoder(stream).unwrap();

    let error = decoder
        .send_packet(ruviform_media_io::Packet {
            stream: StreamId(99),
            index: 0,
            pts: None,
            dts: None,
            duration: None,
            trim_start: 0,
            trim_end: 0,
            flags: ruviform_media_io::PacketFlags::NONE,
            data: Vec::new(),
        })
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn make_decoder_rejects_unknown_streams_and_capabilities_split() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let error = demuxer.make_decoder(StreamId(99)).err().unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);

    // Query half: demux + decode only.
    let caps = ruviform_io_symphonia::capabilities();
    assert!(caps.can_demux());
    assert!(caps.can_decode(CodecId::Pcm));
    assert!(caps.can_decode(CodecId::Flac));
    assert!(caps.can_decode(CodecId::Vorbis));
    assert!(!caps.can_mux());
    assert!(!caps.can_encode(CodecId::Pcm));
    // Default-build codecs the boundary does not advertise. `aac` is an
    // opt-in feature (Issue #154), so its capability follows the build.
    assert_eq!(caps.can_decode(CodecId::Aac), cfg!(feature = "aac"));
    assert!(!caps.can_decode(CodecId::Opus));
    assert!(!caps.can_decode(CodecId::Mp3));

    // Request half: acquisition returns normalized Unsupported errors.
    let error = ruviform_io_symphonia::make_muxer().err().unwrap();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
    let error = ruviform_io_symphonia::make_encoder().err().unwrap();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
}

#[test]
fn seek_positions_the_next_packet() {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();

    // Seek to frame 2_000 of 2_500. The backend's WAV reader lands on
    // packet boundaries (1152-frame packets), so the observable landing
    // is the boundary at-or-before the request: packet 1 at pts 1152.
    let target = MediaTime::new(2_000, TimeBase::sample_rate(RATE).unwrap());
    demuxer.seek(target).unwrap();
    let packet = demuxer.read_packet().unwrap().unwrap();
    assert_eq!(
        packet.pts.map(|pts| pts.ticks),
        Some(i64::try_from(PACKET_FRAMES).unwrap())
    );
    assert_eq!(packet.data.len(), PACKET_FRAMES * 4);
}

#[test]
fn probe_rejects_non_container_bytes() {
    let result = ruviform_io_symphonia::open_demuxer(Cursor::new(vec![0xAB_u8; 64]));
    let error = result.err().unwrap();
    // Probing garbage is a normalized error — never a panic.
    assert!(matches!(
        error.kind(),
        ErrorKind::InvalidData | ErrorKind::Unsupported | ErrorKind::UnexpectedEos
    ));
}

#[test]
fn stream_and_container_facts_are_populated() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();

    // Container identification comes from real magic bytes.
    assert_eq!(demuxer.container().format, ContainerFormat::Wav);
    assert_eq!(demuxer.backend_info().name, "symphonia");

    let info = &demuxer.streams()[0];
    assert_eq!(info.facts.index, 0);
    assert_eq!(info.facts.container_id, Some(0));
    assert_eq!(info.facts.start, Fact::known(0));
    assert_eq!(
        info.facts.declared_units,
        Fact::known(u64::try_from(FRAMES).unwrap())
    );
    assert_eq!(info.facts.decodable, Fact::known(()));
    // WAV declares no codec delay/padding — facts stay absent, not 0.
    assert_eq!(info.facts.leading_trim, None);
    assert_eq!(info.facts.trailing_trim, None);
    // The codec tag concept has no backend source → absent, not fabricated.
    assert_eq!(info.facts.codec_tag, None);

    // The native format reports the wire encoding (S16), distinct from
    // the canonical F32 decode target.
    match info.codec.native_format {
        Fact::Known(CodecFormat::Audio(format)) => {
            assert_eq!(format.sample_format, SampleFormat::S16);
            assert_eq!(format.sample_rate.get(), RATE);
            assert_eq!(format.layout, ChannelLayout::Stereo);
        }
        ref other => unreachable!("expected a known native audio format, got {other:?}"),
    }
}

#[test]
fn native_decode_yields_exact_native_scale_samples() {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut decoder = demuxer.make_native_decoder(stream).unwrap();

    let mut frames = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            frames.push(frame);
        }
    }

    assert_eq!(frames.len(), 3);
    let mut expected = 0_usize;
    for frame in &frames {
        let Frame::NativeAudio(native) = frame else {
            unreachable!("expected a native audio frame, got {frame:?}")
        };
        let format = native.format();
        assert_eq!(format.sample_format, SampleFormat::S16);
        assert_eq!(format.layout, ChannelLayout::Stereo);
        assert_eq!(format.sample_rate.get(), RATE);
        // Interleaved S16-scale f64 — every decoded value is the exact
        // i16 wire value, no [-1, 1] normalization.
        assert_eq!(native.transforms(), [PcmTransform::IntegerWidenedToF64]);
        assert!(!native.is_lossy());
        assert_eq!(native.pts().ticks, i64::try_from(expected).unwrap());
        assert_eq!(native.pts().provenance, TimestampProvenance::Carried);
        let count = native.frames();
        assert_eq!(native.samples().len(), count * 2);
        for index in 0..count {
            let global = expected + index;
            assert_eq!(
                native.samples()[index * 2],
                f64::from(sample_left(global)),
                "L ch native-scale mismatch at frame {global}"
            );
            assert_eq!(
                native.samples()[index * 2 + 1],
                f64::from(sample_right(global)),
                "R ch native-scale mismatch at frame {global}"
            );
        }
        expected += count;
    }
    assert_eq!(expected, FRAMES);
}

/// Builds a RIFF/WAVE stream in an arbitrary PCM encoding — mono, 64
/// frames of deterministic content — for native-scale exactness tests.
///
/// `format_tag`/`bits` are the wire fields: 1 + bits for integer PCM,
/// 3 + 32/64 for IEEE float. `sample(i)` yields the encoded value for
/// frame `i` as raw bytes.
fn wav_bytes_encoded(format_tag: u16, bits: u16, sample: impl Fn(usize) -> Vec<u8>) -> Vec<u8> {
    const FRAMES_N: usize = 64;
    let block_align = CHANNELS_1 * (bits / 8);
    let data_len = u32::try_from(FRAMES_N * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&format_tag.to_le_bytes());
    bytes.extend_from_slice(&CHANNELS_1.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..FRAMES_N {
        bytes.extend_from_slice(&sample(i));
    }
    bytes
}

/// Mono channel count for the parameterized fixtures.
const CHANNELS_1: u16 = 1;

/// Native-decodes `bytes` and returns every emitted frame.
fn native_frames(bytes: Vec<u8>) -> Vec<ruviform_media_io::NativeAudioFrame> {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes)).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut decoder = demuxer.make_native_decoder(stream).unwrap();
    let mut frames = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            let Frame::NativeAudio(native) = frame else {
                unreachable!("expected native audio frames")
            };
            frames.push(native);
        }
    }
    frames
}

/// Collects the interleaved native samples of a whole stream.
fn native_samples_of(bytes: Vec<u8>) -> Vec<f64> {
    let mut samples = Vec::new();
    for frame in native_frames(bytes) {
        samples.extend_from_slice(frame.samples());
    }
    samples
}

#[test]
fn native_decode_is_exact_for_every_integer_and_float_encoding() {
    // Unsigned 8-bit — signed-offset convention: wire 128 → 0.0.
    let samples = native_samples_of(wav_bytes_encoded(1, 8, |i| {
        vec![u8::try_from(i % 4).unwrap() + 126]
    }));
    assert_eq!(samples.len(), 64);
    for (i, &v) in samples.iter().enumerate() {
        assert_eq!(
            v,
            f64::from(u8::try_from(i % 4).unwrap() + 126) - 128.0,
            "u8 at {i}"
        );
    }

    // Signed 24-bit — the extremes of the range must round-trip exactly.
    let values = [-8_388_608_i32, -1, 0, 1, 8_388_607];
    let samples = native_samples_of(wav_bytes_encoded(1, 24, |i| {
        let v = values[i % values.len()];
        (v & 0xFF_FFFF).to_le_bytes()[..3].to_vec()
    }));
    for (i, &v) in samples.iter().enumerate() {
        assert_eq!(v, f64::from(values[i % values.len()]), "s24 at {i}");
    }

    // Signed 32-bit — full-range values stay exact in f64.
    let values = [i32::MIN, -1, 0, 1, i32::MAX];
    let samples = native_samples_of(wav_bytes_encoded(1, 32, |i| {
        values[i % values.len()].to_le_bytes().to_vec()
    }));
    for (i, &v) in samples.iter().enumerate() {
        assert_eq!(v, f64::from(values[i % values.len()]), "s32 at {i}");
    }
}

#[test]
fn native_decode_is_exact_for_float_encodings() {
    // Float32 — widened to f64 without loss.
    let values = [-1.5_f32, 0.0, 0.25, f32::MAX];
    let samples = native_samples_of(wav_bytes_encoded(3, 32, |i| {
        values[i % values.len()].to_le_bytes().to_vec()
    }));
    for (i, &v) in samples.iter().enumerate() {
        assert_eq!(v, f64::from(values[i % values.len()]), "f32 at {i}");
    }

    // Float64 — stored verbatim, empty transform history.
    let values = [-1.5_f64, 0.0, 0.1, f64::MAX];
    let bytes = wav_bytes_encoded(3, 64, |i| values[i % values.len()].to_le_bytes().to_vec());
    let frames = native_frames(bytes);
    let mut offset = 0_usize;
    for frame in &frames {
        assert_eq!(frame.format().sample_format, SampleFormat::F64);
        assert!(frame.transforms().is_empty(), "f64 path is verbatim");
        for (i, &v) in frame.samples().iter().enumerate() {
            assert_eq!(v, values[(offset + i) % values.len()], "f64 sample");
        }
        offset += frame.frames();
    }
}

// ---------- Issue #197 boundary additions ----------

/// Non-seekable source wrapper — `Seek::seek` always fails.
struct NoSeek(Cursor<Vec<u8>>);

impl std::io::Read for NoSeek {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Seek for NoSeek {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no seek",
        ))
    }
}

#[test]
fn boundary_seekable_fact_mirrors_source_capability() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    assert_eq!(demuxer.seekable(), Fact::known(()));

    let unseekable =
        ruviform_io_symphonia::open_demuxer(NoSeek(Cursor::new(wav_bytes(FRAMES)))).unwrap();
    assert_eq!(unseekable.seekable(), Fact::Unsupported);
}

#[test]
fn boundary_make_decoder_dispatches_both_modes() {
    use ruviform_media_io::DecodeMode;

    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let stream = demuxer.streams()[0].stream.id;

    // The generic port builds a canonical decoder — the same object
    // the inherent `make_decoder` constructor produces.
    let mut decoder = Demuxer::make_decoder(&demuxer, stream, DecodeMode::Canonical).unwrap();
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let packet = demuxer.read_packet().unwrap().unwrap();
    decoder.send_packet(packet).unwrap();
    match decoder.receive_frame().unwrap() {
        Some(Frame::Audio(_)) => {}
        other => unreachable!("expected canonical audio, got {other:?}"),
    }

    // Native mode yields the observation representation.
    let mut decoder = Demuxer::make_decoder(&demuxer, stream, DecodeMode::Native).unwrap();
    let packet = demuxer.read_packet().unwrap().unwrap();
    decoder.send_packet(packet).unwrap();
    match decoder.receive_frame().unwrap() {
        Some(Frame::NativeAudio(_)) => {}
        other => unreachable!("expected native audio, got {other:?}"),
    }
}

#[test]
fn boundary_make_decoder_reports_unknown_stream() {
    use ruviform_media_io::DecodeMode;

    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(FRAMES))).unwrap();
    let err = Demuxer::make_decoder(&demuxer, StreamId(99), DecodeMode::Canonical)
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::InvalidValue);
}
