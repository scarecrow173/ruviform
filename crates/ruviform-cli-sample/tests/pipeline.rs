//! Issue #155 acceptance tests for the shared headless pipeline:
//! media loading (`probe`) and one offline process (`run`) across
//! success / failure / cancellation / progress — over synthesized
//! in-memory WAV (Symphonia) and AVI (ruviform-io-avi) fixtures.
//!
//! Everything is exercised through the same entry points the CLI (and
//! a hypothetical GUI adapter) calls: [`probe`] and [`spawn_analyze`].
// Fixed-width fixture casts are the point of the RIFF grammar.
#![allow(clippy::cast_possible_truncation, clippy::trivially_copy_pass_by_ref)]

use std::io::Cursor;

use ruviform_cli_sample::{AnalyzeOptions, PipelineError, SampleScale, probe, spawn_analyze};
use ruviform_exec::{JobError, JobEvent, JobOutcome};

// ---------- generated WAV fixture (PCM s16, stereo, sine) ----------

const RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
const BITS: u16 = 16;

fn wav_bytes(frames: usize) -> Vec<u8> {
    let block_align = CHANNELS * (BITS / 8);
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM tag
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for ch in 0..CHANNELS {
            // 0.5-amplitude sine at 440 Hz × (channel + 1).
            let phase = 2.0 * std::f64::consts::PI * 440.0 * f64::from(i as u32) / f64::from(RATE)
                * f64::from(ch + 1);
            let sample = (0.5 * 32767.0 * phase.sin()) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
    }
    bytes
}

// ---------- generated AVI fixture (audio-only, PCM s16) ----------

fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

fn list(out: &mut Vec<u8>, ty: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(b"LIST");
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(ty);
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

fn strh(
    fcc: &[u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    sample_size: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(fcc);
    body.extend_from_slice(fcc);
    body.extend_from_slice(&0u32.to_le_bytes()); // dwFlags
    body.extend_from_slice(&0u16.to_le_bytes()); // wPriority
    body.extend_from_slice(&0u16.to_le_bytes()); // wLanguage
    body.extend_from_slice(&0u32.to_le_bytes()); // dwInitialFrames
    body.extend_from_slice(&scale.to_le_bytes());
    body.extend_from_slice(&rate.to_le_bytes());
    body.extend_from_slice(&start.to_le_bytes());
    body.extend_from_slice(&length.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // dwSuggestedBufferSize
    body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // dwQuality
    body.extend_from_slice(&sample_size.to_le_bytes());
    body.extend_from_slice(&[0_u8; 16]); // rcFrame
    body
}

fn waveformatex(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let block = channels * (bits / 8);
    let mut body = Vec::new();
    body.extend_from_slice(&tag.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&rate.to_le_bytes());
    body.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    body.extend_from_slice(&block.to_le_bytes());
    body.extend_from_slice(&bits.to_le_bytes());
    body
}

/// Single audio stream (`00wb`), one movi packet holding `frames`
/// interleaved s16 frames.
fn avi_bytes(frames: usize) -> Vec<u8> {
    let mut samples = Vec::new();
    for i in 0..frames {
        for ch in 0..CHANNELS {
            let phase = 2.0 * std::f64::consts::PI * 440.0 * f64::from(i as u32) / f64::from(RATE)
                * f64::from(ch + 1);
            let sample = (0.5 * 32767.0 * phase.sin()) as i16;
            samples.extend_from_slice(&sample.to_le_bytes());
        }
    }
    let block = u32::from(CHANNELS * (BITS / 8));
    let mut strl_body = Vec::new();
    chunk(
        &mut strl_body,
        b"strh",
        &strh(b"auds", block, RATE * block, 0, frames as u32, block),
    );
    chunk(
        &mut strl_body,
        b"strf",
        &waveformatex(1, CHANNELS, RATE, BITS),
    );
    let mut hdrl_body = Vec::new();
    list(&mut hdrl_body, b"strl", &strl_body);
    let mut movi_body = Vec::new();
    chunk(&mut movi_body, b"00wb", &samples);
    let mut riff_body = Vec::new();
    list(&mut riff_body, b"hdrl", &hdrl_body);
    list(&mut riff_body, b"movi", &movi_body);
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((riff_body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(&riff_body);
    out
}

// ---------- probe: media loading ----------

#[test]
fn probe_wav_reports_container_backend_and_stream_facts() {
    let report = probe(Cursor::new(wav_bytes(4_800))).unwrap();
    assert_eq!(report.container.format.name(), "wav");
    assert_eq!(report.backend.name, "symphonia");
    assert_eq!(report.streams.len(), 1);
    let stream = &report.streams[0];
    assert_eq!(stream.codec.codec.name(), "pcm");
    assert_eq!(stream.facts.decodable, ruviform_media_core::Fact::known(()));
}

#[test]
fn probe_avi_routes_to_the_avi_backend() {
    let report = probe(Cursor::new(avi_bytes(4_800))).unwrap();
    assert_eq!(report.container.format.name(), "avi");
    assert_eq!(report.backend.name, "ruviform-io-avi");
    assert_eq!(report.streams.len(), 1);
    assert_eq!(report.streams[0].codec.codec.name(), "pcm");
}

#[test]
fn probe_rejects_garbage_with_a_normalized_error() {
    let bytes = vec![0xde_u8, 0xad, 0xbe, 0xef, 0, 1, 2, 3, 4, 5, 6, 7];
    match probe(Cursor::new(bytes)) {
        Err(PipelineError::Media(e)) => {
            assert_eq!(e.kind(), ruviform_media_io::ErrorKind::Unsupported);
        }
        other => panic!("expected media error, got {other:?}"),
    }
}

// ---------- run: success + one offline process ----------

#[test]
fn run_decodes_and_meters_the_wav_stream() {
    let job = spawn_analyze(Cursor::new(wav_bytes(4_800)), AnalyzeOptions::default());
    let report = job.wait().unwrap();
    assert_eq!(report.scale, SampleScale::CanonicalF32);
    assert_eq!(report.workers, 1);
    assert_eq!(report.stats.packets, report.stats.forwarded);
    assert!(report.stats.frames >= 1);
    assert_eq!(report.summary.len(), 2, "stereo levels");
    for level in &report.summary {
        // 0.5-amplitude sine: peak ≈ 0.5, rms ≈ 0.5/√2.
        assert!((level.peak - 0.5).abs() < 0.001, "peak {}", level.peak);
        assert!((level.rms - 0.3536).abs() < 0.01, "rms {}", level.rms);
    }
    // Per-chunk levels are filled in-place by the kernel.
    for chunk in &report.chunks {
        assert_eq!(chunk.levels.len(), 2);
        assert!(chunk.levels.iter().all(|l| l.peak > 0.0));
    }
}

#[test]
fn run_native_reports_source_scale_levels_and_transforms() {
    let options = AnalyzeOptions {
        native: true,
        ..AnalyzeOptions::default()
    };
    let job = spawn_analyze(Cursor::new(wav_bytes(4_800)), options);
    let report = job.wait().unwrap();
    assert_eq!(report.scale, SampleScale::NativeF64);
    assert!(!report.lossy);
    assert_ne!(report.transforms.len(), 0);
    // s16 native scale: peak ≈ 0.5·32767 ≈ 16383.
    for level in &report.summary {
        assert!(
            (level.peak - 16_383.0).abs() < 2.0,
            "native peak {}",
            level.peak
        );
    }
}

#[test]
fn run_parallel_matches_the_sequential_reference_output() {
    let sequential = spawn_analyze(Cursor::new(wav_bytes(4_800)), AnalyzeOptions::default())
        .wait()
        .unwrap();
    let parallel = spawn_analyze(
        Cursor::new(wav_bytes(4_800)),
        AnalyzeOptions {
            workers: 4,
            ..AnalyzeOptions::default()
        },
    )
    .wait()
    .unwrap();
    assert_eq!(parallel.chunks.len(), sequential.chunks.len());
    for (seq, par) in sequential.chunks.iter().zip(&parallel.chunks) {
        assert_eq!(seq.index, par.index);
        assert_eq!(seq.pts, par.pts);
        assert_eq!(seq.levels, par.levels, "chunk {} diverged", seq.index);
    }
    assert_eq!(sequential.summary.len(), parallel.summary.len());
}

#[test]
fn run_on_the_avi_backend_meters_identical_signal() {
    let report = spawn_analyze(Cursor::new(avi_bytes(4_800)), AnalyzeOptions::default())
        .wait()
        .unwrap();
    assert_eq!(report.backend.name, "ruviform-io-avi");
    assert_eq!(report.stats.frames, 1, "one movi packet → one chunk");
    for level in &report.summary {
        assert!((level.peak - 0.5).abs() < 0.001, "peak {}", level.peak);
    }
}

// ---------- run: failure ----------

#[test]
fn run_unknown_stream_id_is_a_usage_error() {
    let options = AnalyzeOptions {
        stream: Some(99),
        ..AnalyzeOptions::default()
    };
    match spawn_analyze(Cursor::new(wav_bytes(4_800)), options).wait() {
        Err(JobError::Operation(PipelineError::Usage(reason))) => {
            assert!(reason.contains("stream"), "{reason}");
        }
        other => panic!("expected usage error, got {other:?}"),
    }
}

#[test]
fn run_on_corrupt_input_is_a_media_error() {
    let bytes = vec![0xAA_u8; 64];
    match spawn_analyze(Cursor::new(bytes), AnalyzeOptions::default()).wait() {
        Err(JobError::Operation(PipelineError::Media(e))) => {
            assert_eq!(e.kind(), ruviform_media_io::ErrorKind::Unsupported);
        }
        other => panic!("expected media error, got {other:?}"),
    }
}

// ---------- run: cancellation + progress events ----------

#[test]
fn cancellation_through_the_job_token_stops_the_run() {
    // A cancelled-before-start token is observed at the first packet
    // boundary — deterministic for any input.
    let job = spawn_analyze(Cursor::new(wav_bytes(48_000)), AnalyzeOptions::default());
    job.cancel();
    let outcomes: Vec<JobOutcome> = job
        .events()
        .iter()
        .filter_map(|e| match e {
            JobEvent::Finished { outcome } => Some(outcome),
            _ => None,
        })
        .collect();
    match job.wait() {
        Err(JobError::Operation(PipelineError::Cancelled)) => {}
        other => panic!("expected cancelled, got {other:?}"),
    }
    assert_eq!(outcomes, vec![JobOutcome::Cancelled]);
}

#[test]
fn progress_events_cover_decode_and_process_phases() {
    let job = spawn_analyze(Cursor::new(wav_bytes(4_800)), AnalyzeOptions::default());
    let events: Vec<JobEvent> = job.events().iter().collect();
    // `iter()` ends on channel-close = worker exit, so the report is
    // ready and the counters settled before `wait`.
    let completed = job.progress().completed();
    let report = job.wait().unwrap();

    assert_eq!(events.first(), Some(&JobEvent::Started));
    assert_eq!(
        events.last(),
        Some(&JobEvent::Finished {
            outcome: JobOutcome::Completed
        })
    );

    let progress: Vec<(u64, u64)> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Progress { completed, total } => Some((*completed, *total)),
            _ => None,
        })
        .collect();
    // Decode phase reports indeterminate counts (total 0); the driver
    // phase then reports `completed/chunks`.
    assert!(
        progress.iter().any(|&(_, total)| total == 0),
        "{progress:?}"
    );
    let n = report.chunks.len() as u64;
    assert!(
        progress
            .iter()
            .any(|&(completed, total)| completed == n && total == n),
        "terminal progress snapshot missing: {progress:?}"
    );
    assert_eq!(completed, n);
}

#[test]
fn mid_decode_cancellation_reports_cancelled_progress() {
    // Cancel after the first observed progress event — the decode loop
    // honors the token at the next packet boundary.
    let job = spawn_analyze(Cursor::new(wav_bytes(48_000)), AnalyzeOptions::default());
    let mut saw_progress = false;
    for event in job.events() {
        if let JobEvent::Progress { .. } = event
            && !saw_progress
        {
            saw_progress = true;
            job.cancel();
        }
    }
    match job.wait() {
        Err(JobError::Operation(PipelineError::Cancelled)) | Ok(_) => {}
        other => panic!("unexpected outcome {other:?}"),
    }
    assert!(saw_progress);
}
