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
use ruviform_media_core::{MediaDuration, MediaTime, StreamId, StreamKind, TimeBase};
use ruviform_media_io::{CodecFormat, CodecId, Decoder, Demuxer, ErrorKind, Frame};

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
        TimeBase::sample_rate(RATE).unwrap(),
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
    for (packet, (pts, frames)) in packets.iter().zip(expected) {
        assert_eq!(packet.stream, stream);
        assert_eq!(packet.pts.map(MediaTime::value), Some(pts));
        assert_eq!(
            packet.pts.map(MediaTime::base),
            Some(TimeBase::sample_rate(RATE).unwrap())
        );
        assert_eq!(
            packet.duration.map(MediaDuration::value),
            Some(i64::try_from(frames).unwrap())
        );
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
        // stream's 1/rate time base.
        assert_eq!(audio.pts().value(), i64::try_from(expected).unwrap());
        assert_eq!(audio.pts().base(), TimeBase::sample_rate(RATE).unwrap());
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
            pts: None,
            dts: None,
            duration: None,
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
    // Default-build codecs the boundary does not advertise.
    assert!(!caps.can_decode(CodecId::Aac));
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
        packet.pts.map(MediaTime::value),
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
