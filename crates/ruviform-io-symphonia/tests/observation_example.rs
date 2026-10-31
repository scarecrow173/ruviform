//! AudioLens-style usage example for the Issue #151 observation
//! contract.
//!
//! The code below is written the way a downstream consumer works: it
//! touches only `ruviform_media_io` / `ruviform_media_core` /
//! `ruviform_audio_core` boundary types — never a Symphonia type — and
//! collects the facts needed to analyze timestamp sequences and decode
//! non-destructive PCM. Whether a gap is a *defect*, and any
//! validity/confidence label, stays on the consumer side of the
//! boundary.

use std::collections::BTreeMap;
use std::io::Cursor;

use ruviform_audio_core::SampleFormat;
use ruviform_media_core::{Fact, StreamId, observe_timing};
use ruviform_media_io::{CodecFormat, ContainerFormat, Decoder, Demuxer, Frame};

/// Synthesizes a deterministic stereo S16 WAV stream — 400 frames of a
/// ±2000-amplitude ramp at 48 kHz.
fn wav_bytes() -> Vec<u8> {
    const FRAMES: usize = 400;
    const RATE: u32 = 48_000;
    const CHANNELS: u16 = 2;
    const BITS: u16 = 16;

    let block_align = CHANNELS * (BITS / 8);
    let data_len = u32::try_from(FRAMES * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..FRAMES {
        let v = (i16::try_from(i % 40).unwrap() - 20) * 100;
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

#[test]
fn consumer_collects_facts_and_observes_raw_timestamps() {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes())).unwrap();

    // Container and backend identity are observable facts.
    assert_eq!(demuxer.container().format, ContainerFormat::Wav);
    assert_eq!(demuxer.backend_info().name, "symphonia");

    // Stream facts: codec identity, time base, decodability, and the
    // codec's native output format — classified, never defaulted.
    let info = &demuxer.streams()[0];
    let stream = info.stream.id;
    assert!(info.stream.time_base.is_known());
    assert_eq!(info.facts.decodable, Fact::Known(()));
    let Some(CodecFormat::Audio(native)) = info.codec.native_format.value() else {
        panic!("PCM WAV has a known native format")
    };
    assert_eq!(native.sample_format, SampleFormat::S16);

    // One sequential pass collects the raw per-stream tick sequences —
    // keyed by `StreamId` so independent clocks never mix — while a
    // native decoder produces non-destructive PCM: original-scale S16
    // values in f64, with an explicit conversion history.
    let mut decoder = demuxer.make_native_decoder(stream).unwrap();
    let mut pts_by_stream: BTreeMap<StreamId, Vec<i64>> = BTreeMap::new();
    let mut decoded_frames = 0_usize;
    while let Some(packet) = demuxer.read_packet().unwrap() {
        if let Some(pts) = packet.pts {
            pts_by_stream
                .entry(packet.stream)
                .or_default()
                .push(pts.ticks);
        }
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            let Frame::NativeAudio(native) = frame else {
                panic!("native decoder must yield native frames")
            };
            assert_eq!(native.format().sample_format, SampleFormat::S16);
            for transform in native.transforms() {
                assert!(
                    !transform.is_lossy(),
                    "native path records only lossless steps"
                );
            }
            decoded_frames += native.frames();
        }
    }

    // The consumer now holds everything an analysis pass needs: the raw
    // tick series, the declared time base, and the *actual* decoded
    // frame count — without having fabricated or normalized anything.
    let pts = &pts_by_stream[&stream];
    let observation = observe_timing(pts, None);
    assert_eq!(observation.timestamp_count, pts.len());
    assert_eq!(observation.discontinuities, vec![]);
    assert_eq!(observation.duplicates, vec![]);
    assert_eq!(decoded_frames, 400);
}
