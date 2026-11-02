//! Prefetch pipeline acceptance tests — RD-02 (mapping + generation),
//! RD-03 (variable callbacks + dual sources + single-owner pool),
//! RD-04 (fault classes), RD-05 (cache pressure, close/reopen, leaks).

use std::io::Cursor;
use std::time::{Duration, Instant};

use ruviform_media_io::{DecodeMode, Demuxer, ErrorKind};
use ruviform_media_reader::{
    EngineFrame, FrameMap, Prefetch, PrefetchConfig, PrefetchConsumer, PrefetchProducer,
    PumpOutcome, SourceFault, SourceFrame, SourceReader,
};
use ruviform_rt::RtContext;

const RATE: u32 = 48_000;
const FRAMES: usize = 4800;
const CHANNELS: u16 = 2;

fn sample_at(i: usize) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        (i as i32 * 7 % 5000 - 2500) as i16
    }
}

fn wav_bytes(channels: u16, frames: usize) -> Vec<u8> {
    let block_align = channels * 2;
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for _ in 0..channels {
            bytes.extend_from_slice(&sample_at(i).to_le_bytes());
        }
    }
    bytes
}

fn expected(i: usize) -> f32 {
    f32::from(sample_at(i)) / 32768.0
}

fn config() -> PrefetchConfig {
    PrefetchConfig {
        block_frames: 256,
        block_count: 8,
        channels: usize::from(CHANNELS),
        sample_rate: RATE,
    }
}

fn open_reader(frames: usize) -> SourceReader {
    let demuxer =
        ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(CHANNELS, frames))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap()
}

/// Alternates one producer pump with one consumer fill (variable
/// callback sizes) until EOF, returning the delivered samples per
/// channel. Mirrors the real topology: producer non-RT, consumer RT.
fn drain_all(producer: &mut PrefetchProducer, consumer: &mut PrefetchConsumer) -> Vec<Vec<f32>> {
    let ctx = RtContext::mint();
    let mut out = vec![Vec::new(); usize::from(CHANNELS)];
    let sizes = [97usize, 512, 33, 256, 1000, 64];
    let mut cycle = sizes.iter().cycle();
    let mut dst = [vec![0.0f32; 1024], vec![0.0f32; 1024]];
    for _ in 0..20_000 {
        producer.pump().ok();
        let n = *cycle.next().unwrap();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(|d| &mut d[..n]).collect();
        let report = consumer.fill(&mut slices, consumer.position(), &ctx);
        for (ch, buf) in out.iter_mut().enumerate() {
            buf.extend_from_slice(&slices[ch][..report.written]);
        }
        if report.eof {
            return out;
        }
    }
    panic!("pipeline never reached EOF");
}

#[test]
fn rd02_frame_map_engine_to_source_with_clip_and_placement() {
    // clip_offset=100, placement=1000: engine 1000 ↔ source 100.
    let map = FrameMap::linear(100, 1000);
    assert_eq!(
        map.engine_to_source(EngineFrame(1000)),
        Some(SourceFrame(100))
    );
    assert_eq!(
        map.engine_to_source(EngineFrame(1256)),
        Some(SourceFrame(356))
    );
    // Before placement the map still resolves to valid source
    // content; only a mapping below source frame 0 is rejected.
    assert_eq!(
        map.engine_to_source(EngineFrame(999)),
        Some(SourceFrame(99))
    );
    assert_eq!(map.engine_to_source(EngineFrame(50)), None);
    let range = map.source_range(EngineFrame(1000), 64).unwrap();
    assert_eq!(range.start, SourceFrame(100));
    assert_eq!(range.frames, 64);
}

#[test]
fn rd03_pipeline_delivers_content_in_source_order() {
    let reader = open_reader(FRAMES);
    let (_handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let channels = drain_all(&mut producer, &mut consumer);
    assert_eq!(channels[0].len(), FRAMES);
    for (ch, buf) in channels.iter().enumerate() {
        for (i, &s) in buf.iter().enumerate() {
            assert_eq!(s, expected(i), "ch{ch} frame {i}");
        }
    }
}

#[test]
fn rd03_underrun_is_classified_not_silence_success() {
    let reader = open_reader(FRAMES);
    let (handle, _producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    // Nothing pumped — a fill must report underrun, not content.
    let mut dst = [vec![9.0f32; 128], vec![9.0f32; 128]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let report = consumer.fill(&mut slices, SourceFrame(0), &ctx);
    assert_eq!(report.written, 0);
    assert_eq!(report.underrun, 128);
    assert!(!report.eof);
    assert!(
        slices[0].iter().all(|&s| s == 0.0),
        "fallback is explicit zeros"
    );
    assert_eq!(handle.telemetry().underrun_frames(), 128);
    assert!(handle.telemetry().underrun_events() >= 1);
}

#[test]
fn rd03_two_sources_own_independent_pools() {
    let reader_a = open_reader(FRAMES);
    let reader_b = open_reader(2400);
    let (_ha, mut pa, mut ca) = Prefetch::manual(reader_a, config()).unwrap();
    let (_hb, mut pb, mut cb) = Prefetch::manual(reader_b, config()).unwrap();
    // Interleave: each pipeline progresses independently.
    let a = drain_all(&mut pa, &mut ca);
    let b = drain_all(&mut pb, &mut cb);
    assert_eq!(a[0].len(), FRAMES);
    assert_eq!(b[0].len(), 2400);
    assert_eq!(a[0][0], expected(0));
    assert_eq!(b[0][0], expected(0));
}

#[test]
fn rd02_seek_invalidates_stale_generation() {
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();

    // Deliver some content, then seek mid-stream.
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    for _ in 0..4 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }

    handle.seek(SourceFrame(2000)).unwrap();
    // The seek is an ordered boundary: queued old-generation content
    // still in flight plays first (it precedes the boundary in the
    // ring), then the generation flips and fresh content resumes
    // exactly at the seek target — never interleaved.
    let mut landed_at = None;
    for _ in 0..20_000 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        let report = consumer.fill(&mut slices, consumer.position(), &ctx);
        if report.written > 0 {
            // A fill straddling the boundary mixes generations —
            // position < written there; skip it.
            let Some(first) = consumer.position().0.checked_sub(report.written as u64) else {
                continue;
            };
            if first == 2000 {
                landed_at = Some(report.written);
                break;
            }
        }
    }
    let written = landed_at.expect("post-seek content never landed at 2000");
    let slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    for (ch, buf) in slices.iter().enumerate() {
        for (i, &s) in buf[..written].iter().enumerate() {
            assert_eq!(s, expected(2000 + i), "post-seek ch{ch} {i}");
        }
    }
}

#[test]
fn rd02_generation_isolates_two_seeks() {
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    let g0 = consumer.generation();

    handle.seek(SourceFrame(100)).unwrap();
    // The consumer's generation advances when the Boundary item is
    // delivered through a fill — not when the command is sent.
    for _ in 0..64 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }
    let g1 = consumer.generation();

    handle.seek(SourceFrame(500)).unwrap();
    for _ in 0..64 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }
    let g2 = consumer.generation();
    assert!(g1 > g0, "first seek opened a new generation");
    assert!(g2 > g1, "each seek is a new generation");
}

#[test]
fn rd05_cache_pressure_is_bounded_and_backpressured() {
    // 8 blocks × 256 frames = 2048 frames of capacity; the source has
    // 4800 — the producer must backpressure, not grow.
    let reader = open_reader(FRAMES);
    let (_handle, mut producer, _consumer) = Prefetch::manual(reader, config()).unwrap();
    let mut pushed = 0usize;
    let mut backpressured = false;
    for _ in 0..64 {
        match producer.pump().unwrap() {
            PumpOutcome::Pushed => pushed += 1,
            PumpOutcome::Backpressured => {
                backpressured = true;
                break;
            }
            _ => {}
        }
    }
    assert!(backpressured, "a full cache must backpressure");
    assert!(
        pushed <= 8 + 8,
        "bounded by pool + marker slack, got {pushed}"
    );
}

#[test]
fn rd05_close_audit_reclaims_every_block() {
    let reader = open_reader(FRAMES);
    let (handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let mut dst = [vec![0.0f32; 128], vec![0.0f32; 128]];
    for _ in 0..16 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }
    let audit = handle.close(consumer);
    let close = producer.close_audit();
    // Manual pipeline has no worker thread to join.
    assert!(!audit.worker_joined);
    assert_eq!(audit.dropped, 0, "no leaked blocks");
    assert_eq!(
        close.free_reclaimed, audit.block_count,
        "producer audit sees the whole pool back in the free ring"
    );
}

#[test]
fn rd05_reopen_restarts_under_new_generation() {
    let reader = open_reader(1024);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    for _ in 0..8 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        consumer.fill(&mut slices, consumer.position(), &ctx);
    }
    let g0 = consumer.generation();

    handle
        .reopen(
            Box::new(Cursor::new(wav_bytes(CHANNELS, 2048))),
            Box::new(|source| {
                ruviform_io_symphonia::open_demuxer(source).map(|d| Box::new(d) as Box<dyn Demuxer>)
            }),
        )
        .unwrap();
    // Drain past the reopen boundary (queued old-generation content
    // plays first, in order); fresh content then starts at source 0.
    let mut first_written = None;
    for _ in 0..20_000 {
        producer.pump().ok();
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        let report = consumer.fill(&mut slices, consumer.position(), &ctx);
        if report.written > 0 {
            let Some(first) = consumer.position().0.checked_sub(report.written as u64) else {
                continue;
            };
            if first == 0 {
                first_written = Some(());
                break;
            }
        }
    }
    assert_eq!(first_written, Some(()), "reopen restarts at source 0");
    assert!(consumer.generation() > g0);
}

#[test]
fn rd04_unseekable_handle_reports_fault_class() {
    use std::io::{Read, Seek, SeekFrom};
    struct NoSeek(Cursor<Vec<u8>>);
    impl Read for NoSeek {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for NoSeek {
        fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
            Err(std::io::Error::other("unseekable"))
        }
    }
    let demuxer =
        ruviform_io_symphonia::open_demuxer(NoSeek(Cursor::new(wav_bytes(CHANNELS, 512)))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let reader = SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap();
    let (mut handle, _producer, _consumer) = Prefetch::manual(reader, config()).unwrap();
    let err = handle.seek(SourceFrame(10)).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn rd04_fill_marks_position_mismatch() {
    let reader = open_reader(FRAMES);
    let (_handle, mut producer, consumer_pair) = Prefetch::manual(reader, config()).unwrap();
    let mut consumer = consumer_pair;
    producer.pump().ok();
    let ctx = RtContext::mint();
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    // Caller expects a different source position than the pipeline has.
    let report = consumer.fill(&mut slices, SourceFrame(777), &ctx);
    assert!(report.mismatch, "expected_first mismatch must be reported");
    assert_eq!(report.written, 0, "mismatched fill writes no content");
}

#[test]
fn rd05_spawned_worker_delivers_and_closes() {
    let reader = open_reader(FRAMES);
    let Prefetch {
        handle,
        mut consumer,
    } = Prefetch::spawn(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let mut got = 0usize;
    let mut dst = [vec![0.0f32; 256], vec![0.0f32; 256]];
    // The worker pumps ~once per ms on its own thread; underruns while
    // it catches up are the classified early state, not failures.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut eof = false;
    while Instant::now() < deadline && !eof {
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        let report = consumer.fill(&mut slices, consumer.position(), &ctx);
        got += report.written;
        eof = report.eof;
        if report.written == 0 && !report.eof {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(got, FRAMES, "spawned worker served the full stream");
    assert!(eof, "End marker consumed");
    let audit = handle.close(consumer);
    assert!(audit.worker_joined);
    assert_eq!(audit.dropped, 0);
}

#[test]
fn rd04_fault_counts_have_a_slot_per_class() {
    // The report vocabulary exposes every fault class the issue lists.
    let faults = ruviform_media_reader::FaultCounts::default();
    assert_eq!(faults.truncated, 0);
    assert_eq!(faults.corrupt, 0);
    assert_eq!(faults.io, 0);
    assert_eq!(faults.invalid_sample, 0);
    assert_eq!(faults.unsupported, 0);
    assert_eq!(faults.unseekable, 0);
    let _ = SourceFault::Truncated;
    let _ = SourceFault::Corrupt;
    let _ = SourceFault::InvalidSample;
}

// ---- Issue #233: discard/drain + reopen-at contract tests ----

#[test]
fn rd02_seek_default_keeps_play_through() {
    // Existing `seek` contract: queued pre-seek items still play
    // before the boundary; nothing is purged.
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    for _ in 0..16 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    handle.seek(SourceFrame(2000)).unwrap();
    producer.pump().ok(); // boundary queues behind pre-seek items
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let report = consumer.fill(&mut slices, consumer.position(), &ctx);
    assert!(report.written > 0, "pre-seek content plays through");
    assert_eq!(slices[0][0], expected(0), "pre-seek stream content");
}

#[test]
fn rd02_discard_pending_drains_to_seek_boundary() {
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    for _ in 0..16 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    handle.seek_discard_pending(SourceFrame(2000)).unwrap();
    for _ in 0..16 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    let report = consumer.discard_pending(&ctx);
    assert_eq!(report.boundary, Some(2000));
    assert_eq!(consumer.position(), SourceFrame(2000));
    // After the boundary the pipeline serves only post-seek content.
    for _ in 0..4 {
        producer.pump().ok();
    }
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let fill = consumer.fill(&mut slices, consumer.position(), &ctx);
    assert!(fill.written > 0);
    assert_eq!(slices[0][0], expected(2000), "re-anchored content");
}

#[test]
fn rd02_discard_pending_without_boundary_reports_none() {
    let reader = open_reader(FRAMES);
    let (_handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    for _ in 0..8 {
        producer.pump().ok();
    }
    let report = consumer.discard_pending(&ctx);
    assert_eq!(report.boundary, None);
    assert!(report.items > 0, "queued items were processed");
    assert!(report.blocks > 0, "blocks were returned to the pool");
}

#[test]
fn rd02_seek_discard_pending_purges_stale_markers() {
    // Two seeks in one command batch: the second's discard flag
    // clears the first's queued boundary, so only the later target
    // is ever observed.
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let g0 = consumer.generation();
    handle.seek(SourceFrame(100)).unwrap();
    handle.seek_discard_pending(SourceFrame(500)).unwrap();
    for _ in 0..16 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    let report = consumer.discard_pending(&ctx);
    assert_eq!(report.boundary, Some(500), "first seek marker was purged");
    assert_eq!(consumer.position(), SourceFrame(500));
    assert_eq!(consumer.generation(), g0 + 2);
}

#[test]
fn rd02_reopen_at_lands_on_seek_target() {
    let reader = open_reader(1024);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    let g0 = consumer.generation();
    handle
        .reopen_at(
            Box::new(Cursor::new(wav_bytes(CHANNELS, 2048))),
            Box::new(|source| {
                ruviform_io_symphonia::open_demuxer(source).map(|d| Box::new(d) as Box<dyn Demuxer>)
            }),
            SourceFrame(500),
        )
        .unwrap();
    let mut landed = None;
    for _ in 0..64 {
        producer.pump().ok();
        let report = consumer.discard_pending(&ctx);
        if report.boundary.is_some() {
            landed = report.boundary;
            break;
        }
    }
    assert_eq!(landed, Some(500), "boundary reports landed position");
    assert_eq!(consumer.position(), SourceFrame(500));
    assert!(consumer.generation() > g0);
    for _ in 0..8 {
        producer.pump().ok();
    }
    let mut dst = [vec![0.0f32; 64], vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let fill = consumer.fill(&mut slices, consumer.position(), &ctx);
    assert!(fill.written > 0);
    assert_eq!(slices[0][0], expected(500), "reopened source at target");
}

#[test]
fn rd02_reopen_at_seek_failure_faults_then_reanchors() {
    use std::io::{Read, Seek, SeekFrom};
    struct NoSeek(Cursor<Vec<u8>>);
    impl Read for NoSeek {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for NoSeek {
        fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
            Err(std::io::Error::other("unseekable"))
        }
    }
    let reader = open_reader(1024);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    handle
        .reopen_at(
            Box::new(NoSeek(Cursor::new(wav_bytes(CHANNELS, 2048)))),
            Box::new(|source| {
                ruviform_io_symphonia::open_demuxer(source).map(|d| Box::new(d) as Box<dyn Demuxer>)
            }),
            SourceFrame(500),
        )
        .unwrap();
    let mut landed = None;
    for _ in 0..64 {
        producer.pump().ok();
        let report = consumer.discard_pending(&ctx);
        if report.boundary.is_some() {
            landed = report.boundary;
            break;
        }
    }
    // The seek failed: an Unseekable fault precedes a boundary that
    // re-anchors at the reopened position (frame 0). The fault is
    // tagged with the new generation, so the consumer drops it as
    // generation-stale ahead of the boundary — the observable report
    // is a stale drop plus the frame-0 re-anchor.
    assert_eq!(landed, Some(0));
    assert_eq!(consumer.position(), SourceFrame(0));
    assert!(handle.telemetry().stale_drops() >= 1);
}

#[test]
fn rd05_reopen_still_anchors_at_frame_zero() {
    // Existing `reopen` regression: unchanged frame-0 anchoring.
    let reader = open_reader(1024);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    handle
        .reopen(
            Box::new(Cursor::new(wav_bytes(CHANNELS, 2048))),
            Box::new(|source| {
                ruviform_io_symphonia::open_demuxer(source).map(|d| Box::new(d) as Box<dyn Demuxer>)
            }),
        )
        .unwrap();
    let mut landed = None;
    for _ in 0..64 {
        producer.pump().ok();
        let report = consumer.discard_pending(&ctx);
        if report.boundary.is_some() {
            landed = report.boundary;
            break;
        }
    }
    assert_eq!(landed, Some(0));
    assert_eq!(consumer.position(), SourceFrame(0));
}

#[test]
fn rd02_discard_pending_returns_inflight_block() {
    // Partially consume a block, then drain to a seek boundary:
    // the in-flight `current` is returned to the pool and counted.
    let reader = open_reader(FRAMES);
    let (mut handle, mut producer, mut consumer) = Prefetch::manual(reader, config()).unwrap();
    let ctx = RtContext::mint();
    for _ in 0..8 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    let mut dst = [vec![0.0f32; 32], vec![0.0f32; 32]];
    {
        let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
        let fill = consumer.fill(&mut slices, consumer.position(), &ctx);
        assert!(
            fill.written > 0 && fill.written < 256,
            "block left in-flight"
        );
    }
    handle.seek_discard_pending(SourceFrame(2000)).unwrap();
    for _ in 0..16 {
        if producer.pump().unwrap() != PumpOutcome::Pushed {
            break;
        }
    }
    let report = consumer.discard_pending(&ctx);
    assert_eq!(report.boundary, Some(2000));
    assert_eq!(consumer.position(), SourceFrame(2000));
    assert!(
        report.blocks >= 2,
        "in-flight + queued blocks returned, got {}",
        report.blocks
    );
}
