//! Round-trip verification: PCM WAV written by `ruviform-io-wav`
//! reopened and decoded by `ruviform-io-symphonia` — the acceptance
//! `PoC` for `Issue #191` (AC1, AC3, AC4).

use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleFormat, SampleRate};
use ruviform_io_wav::{WavOutputSpec, WavReceipt, make_encoder, open_muxer};
use ruviform_media_core::{MediaTime, MediaTimestamp};
use ruviform_media_io::{AudioFrame, Decoder, Demuxer, Encoder, Frame, Muxer, StreamInfo};
use std::io::Cursor;

fn spec(rate: u32, layout: ChannelLayout, wire: SampleFormat) -> WavOutputSpec {
    WavOutputSpec {
        sample_rate: SampleRate::new(rate).unwrap(),
        layout,
        sample_format: wire,
    }
}

/// Deterministic per-channel test signal: channel `c`, frame `i` → a
/// bounded value in (-0.75, 0.75) — inside full scale so integer
/// formats exercise real quantization rather than clipping.
// Values stay under 100, so the usize→f32 cast is exact; the lint
// can't see the bound.
#[allow(clippy::cast_precision_loss)]
fn signal(channels: usize, frames: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|channel| {
            (0..frames)
                .map(|i| {
                    let phase = ((i * 7 + channel * 31) % 97) as f32 / 97.0;
                    phase * 1.5 - 0.75
                })
                .collect()
        })
        .collect()
}

fn buffer_of(rate: u32, layout: ChannelLayout, planes: &[Vec<f32>]) -> AudioBuffer {
    let mut buffer = AudioBuffer::new(BufferSpec {
        layout,
        frames: planes[0].len(),
        sample_rate: SampleRate::new(rate).unwrap(),
    });
    for (index, plane) in planes.iter().enumerate() {
        buffer.channel_mut(index).unwrap().copy_from_slice(plane);
    }
    buffer
}

/// Encodes `frames` through `WavEncoder` into `WavMuxer` over an
/// in-memory sink and finalizes — the full
/// `Frame → Packet → container` path.
fn write_wav(info: &StreamInfo, frames: Vec<Frame>) -> (Vec<u8>, WavReceipt) {
    let mut encoder = make_encoder(info).unwrap();
    for frame in frames {
        encoder.send_frame(frame).unwrap();
    }
    encoder.drain().unwrap();

    let sink = Cursor::new(Vec::new());
    let mut muxer = open_muxer(sink, std::slice::from_ref(info)).unwrap();
    while let Some(packet) = encoder.receive_packet().unwrap() {
        muxer.write_packet(packet).unwrap();
    }
    let receipt = muxer.finalize().unwrap();
    (muxer.into_sink().into_inner(), receipt)
}

/// Reopens `bytes` through `ruviform-io-symphonia` and decodes the
/// first stream to canonical `f32` planes.
fn decode_wav(bytes: &[u8]) -> (StreamInfo, usize, usize, Vec<Vec<f32>>) {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes.to_vec()))
        .expect("written wav must reopen");
    let (info, stream_count) = {
        let streams = demuxer.streams();
        assert_eq!(streams.len(), 1, "wav must carry exactly one stream");
        (streams[0].clone(), streams.len())
    };
    let id = info.stream.id;
    let mut decoder = demuxer.make_decoder(id).expect("wav stream must decode");

    let mut planes: Vec<Vec<f32>> = Vec::new();
    let mut total_frames = 0usize;
    while let Some(packet) = demuxer.read_packet().unwrap() {
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            if let Frame::Audio(audio) = frame {
                let buffer = audio.buffer();
                if planes.is_empty() {
                    planes = vec![Vec::new(); buffer.channels()];
                }
                for (channel, plane) in planes.iter_mut().enumerate() {
                    plane.extend_from_slice(buffer.channel(channel).unwrap());
                }
                total_frames += buffer.frames();
            }
        }
    }
    decoder.drain().unwrap();
    while let Some(frame) = decoder.receive_frame().unwrap() {
        if let Frame::Audio(audio) = frame {
            let buffer = audio.buffer();
            if planes.is_empty() {
                planes = vec![Vec::new(); buffer.channels()];
            }
            for (channel, plane) in planes.iter_mut().enumerate() {
                plane.extend_from_slice(buffer.channel(channel).unwrap());
            }
            total_frames += buffer.frames();
        }
    }
    (info, stream_count, total_frames, planes)
}

/// Maximum per-sample error allowed by the wire format's quantization
/// (one LSB of the destination encoding plus a small epsilon).
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

fn roundtrip(rate: u32, layout: ChannelLayout, wire: SampleFormat, frames: usize) {
    let info = spec(rate, layout, wire).stream_info();
    let channels = layout.channels() as usize;
    let source = signal(channels, frames);
    let buffers = buffer_of(rate, layout, &source);
    let pts = MediaTimestamp::carried(MediaTime::new(
        0,
        SampleRate::new(rate).unwrap().time_base(),
    ));
    let frame = Frame::Audio(AudioFrame::new(buffers, pts));

    let (bytes, receipt) = write_wav(&info, vec![frame]);
    assert_eq!(receipt.total_frames, frames as u64);
    assert_eq!(
        receipt.total_bytes,
        frames as u64 * channels as u64 * u64::from(wire.bits_per_sample()) / 8
    );

    let (decoded_info, stream_count, decoded_frames, planes) = decode_wav(&bytes);
    assert_eq!(stream_count, 1);
    assert_eq!(decoded_frames, frames);
    assert_eq!(planes.len(), channels);

    let tol = tolerance(wire);
    for channel in 0..channels {
        for (index, (&expected, &actual)) in source[channel]
            .iter()
            .zip(planes[channel].iter())
            .enumerate()
        {
            assert!(
                (expected - actual).abs() <= tol,
                "channel {channel} frame {index}: expected {expected}, got {actual} (tol {tol})"
            );
        }
    }
    let _ = decoded_info;
}

#[test]
fn roundtrip_stereo_s16_48k() {
    roundtrip(48_000, ChannelLayout::Stereo, SampleFormat::S16, 4_800);
}

#[test]
fn roundtrip_mono_s24_44k1() {
    roundtrip(44_100, ChannelLayout::Mono, SampleFormat::S24, 4_410);
}

#[test]
fn roundtrip_five_point_one_f32_96k_extensible() {
    roundtrip(
        96_000,
        ChannelLayout::FivePointOne,
        SampleFormat::F32,
        9_600,
    );
}

#[test]
fn roundtrip_stereo_s32_48k() {
    roundtrip(48_000, ChannelLayout::Stereo, SampleFormat::S32, 2_048);
}

#[test]
fn roundtrip_short_file() {
    roundtrip(48_000, ChannelLayout::Stereo, SampleFormat::S16, 7);
}

#[test]
fn zero_frame_file_reopens_empty() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let (bytes, receipt) = write_wav(&info, Vec::new());
    assert_eq!(receipt.total_frames, 0);
    let (_, count, decoded_frames, _) = decode_wav(&bytes);
    assert_eq!(count, 1);
    assert_eq!(decoded_frames, 0);
}

#[test]
fn nan_inf_and_clip_values_follow_declared_policy() {
    // Wire S16: NaN→0, ±Inf→±full scale, |x|>1 saturates.
    let info = spec(48_000, ChannelLayout::Mono, SampleFormat::S16).stream_info();
    let plane = vec![0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 2.5, -2.5];
    let buffer = buffer_of(48_000, ChannelLayout::Mono, &[plane]);
    let frame = Frame::Audio(AudioFrame::new(
        buffer,
        MediaTimestamp::carried(MediaTime::new(
            0,
            SampleRate::new(48_000).unwrap().time_base(),
        )),
    ));
    let (bytes, _) = write_wav(&info, vec![frame]);
    let (_, _, decoded, planes) = decode_wav(&bytes);
    assert_eq!(decoded, 6);
    let tol = 1.0 / 32768.0 + 1e-4;
    let expected = [0.0_f32, 0.0, 1.0, -1.0, 1.0, -1.0];
    for (index, (&want, &got)) in expected.iter().zip(planes[0].iter()).enumerate() {
        assert!(
            (want - got).abs() <= tol,
            "sample {index}: expected {want}, got {got}"
        );
    }
}

#[test]
fn drain_then_receive_then_none() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    let planes = signal(2, 8);
    let buffer = buffer_of(48_000, ChannelLayout::Stereo, &planes);
    encoder
        .send_frame(Frame::Audio(AudioFrame::new(
            buffer,
            MediaTimestamp::carried(MediaTime::new(
                0,
                SampleRate::new(48_000).unwrap().time_base(),
            )),
        )))
        .unwrap();
    encoder.drain().unwrap();
    assert!(encoder.receive_packet().unwrap().is_some());
    assert!(encoder.receive_packet().unwrap().is_none());
}

#[test]
fn send_after_drain_is_invalid_data() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    encoder.drain().unwrap();
    let planes = signal(2, 4);
    let buffer = buffer_of(48_000, ChannelLayout::Stereo, &planes);
    let result = encoder.send_frame(Frame::Audio(AudioFrame::new(
        buffer,
        MediaTimestamp::carried(MediaTime::new(
            0,
            SampleRate::new(48_000).unwrap().time_base(),
        )),
    )));
    assert_eq!(
        result.unwrap_err().kind(),
        ruviform_media_io::ErrorKind::InvalidData
    );
}
