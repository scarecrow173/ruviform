//! RT contract tests (`rt_` lane — `mise run test-rt`): the consumer
//! fill path must not allocate, take locks, or touch the OS.

use std::io::Cursor;

use ruviform_media_io::{DecodeMode, Demuxer};
use ruviform_media_reader::{Prefetch, PrefetchConfig, PumpOutcome, SourceFrame, SourceReader};
use ruviform_rt::RtContext;

const RATE: u32 = 48_000;

fn wav_bytes(frames: usize) -> Vec<u8> {
    let block_align = 2_u16 * 2;
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let s = i16::try_from(i % 97).unwrap();
        bytes.extend_from_slice(&s.to_le_bytes());
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    bytes
}

fn build() -> (
    ruviform_media_reader::PrefetchHandle,
    ruviform_media_reader::PrefetchProducer,
    ruviform_media_reader::PrefetchConsumer,
) {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(2048))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let reader = SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap();
    Prefetch::manual(
        reader,
        PrefetchConfig {
            block_frames: 256,
            block_count: 8,
            channels: 2,
            sample_rate: RATE,
        },
    )
    .unwrap()
}

#[test]
fn rt_consumer_fill_no_alloc() {
    let (_handle, mut producer, mut consumer) = build();
    // Pre-load the rings (non-RT).
    for _ in 0..16 {
        if producer.pump().unwrap() == PumpOutcome::Ended {
            break;
        }
    }
    let ctx = RtContext::mint();
    let mut dst = [vec![0.0f32; 128], vec![0.0f32; 128]];
    // Warm-up fill (first fill may lazily bind state).
    {
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }
    let mut ch0 = vec![0.0f32; 128];
    let mut ch1 = vec![0.0f32; 128];
    let mut slices: [&mut [f32]; 2] = [&mut ch0, &mut ch1];
    assert_no_alloc::assert_no_alloc(|| {
        let report = consumer.fill(&mut slices, consumer.position(), &ctx);
        assert!(report.written > 0 || report.underrun > 0 || report.eof);
    });
}

#[test]
fn rt_consumer_underrun_fill_no_alloc() {
    let (_handle, _producer, mut consumer) = build();
    let ctx = RtContext::mint();
    let mut ch0 = vec![0.0f32; 64];
    let mut ch1 = vec![0.0f32; 64];
    let mut slices: [&mut [f32]; 2] = [&mut ch0, &mut ch1];
    // Empty pipeline: the underrun zero-fill path is also alloc-free.
    {
        let mut warm: Vec<&mut [f32]> = slices.iter_mut().map(|s| &mut **s).collect();
        consumer.fill(&mut warm, SourceFrame(0), &ctx);
    }
    assert_no_alloc::assert_no_alloc(|| {
        let report = consumer.fill(&mut slices, SourceFrame(0), &ctx);
        assert_eq!(report.underrun, 64);
    });
}

#[test]
fn rt_consumer_discard_pending_no_alloc() {
    let (_handle, mut producer, mut consumer) = build();
    for _ in 0..16 {
        if producer.pump().unwrap() == PumpOutcome::Ended {
            break;
        }
    }
    let ctx = RtContext::mint();
    let _ = consumer.discard_pending(&ctx); // warm-up: first call may bind state
    for _ in 0..8 {
        producer.pump().ok();
    }
    assert_no_alloc::assert_no_alloc(|| {
        // No seek was issued: the drain consumes block backlog only.
        let report = consumer.discard_pending(&ctx);
        assert!(report.boundary.is_none());
    });
}
