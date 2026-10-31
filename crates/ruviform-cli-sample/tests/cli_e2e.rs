//! Issue #155 end-to-end tests through the actual `ruviform-cli-sample`
//! binary: stdout carries exactly one JSON document, stderr carries
//! JSONL progress events, and the exit code maps the job outcome —
//! the sample-owned CLI adapter contract.
#![allow(clippy::cast_possible_truncation)]

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

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
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for ch in 0..CHANNELS {
            let phase = 2.0 * std::f64::consts::PI * 440.0 * f64::from(i as u32) / f64::from(RATE)
                * f64::from(ch + 1);
            let sample = (0.5 * 32767.0 * phase.sin()) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
    }
    bytes
}

/// A temp-dir fixture file; cleaned up on drop.
struct Fixture {
    dir: PathBuf,
    wav: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "ruviform-cli-sample-e2e-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("sine.wav");
        let mut file = std::fs::File::create(&wav).unwrap();
        file.write_all(&wav_bytes(4_800)).unwrap();
        Self { dir, wav }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_ruviform-cli-sample"))
        .args(args)
        .output()
        .expect("spawn cli");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn probe_prints_one_json_document_on_stdout() {
    let fixture = Fixture::new();
    let (code, stdout, stderr) = run_cli(&["probe", fixture.wav.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["status"], "ok");
    assert_eq!(doc["operation"], "probe");
    assert_eq!(doc["container"]["format"], "wav");
    assert_eq!(doc["streams"][0]["codec"], "pcm");
    assert_eq!(doc["streams"][0]["decodable"], "known");
    // No progress events for the synchronous probe path.
    assert!(!stderr.contains("\"event\":\"progress\""), "{stderr}");
}

#[test]
fn run_streams_jsonl_events_on_stderr_and_report_on_stdout() {
    let fixture = Fixture::new();
    let (code, stdout, stderr) = run_cli(&["run", fixture.wav.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");

    // stdout: exactly one document.
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["status"], "completed");
    assert_eq!(doc["operation"], "run");
    assert_eq!(doc["processing"]["scale"], "canonical-f32");
    assert_eq!(doc["processing"]["kernel"], "peak-rms-meter");
    assert_ne!(doc["chunks"].as_array().unwrap().len(), 0);
    let peak = doc["summary"][0]["peak"].as_f64().unwrap();
    assert!((peak - 0.5).abs() < 0.001, "peak {peak}");

    // stderr: one JSON object per line, first started / last finished.
    let lines: Vec<serde_json::Value> = stderr
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.first().unwrap()["event"], "started", "{stderr}");
    assert_eq!(lines.last().unwrap()["event"], "finished", "{stderr}");
    assert_eq!(lines.last().unwrap()["outcome"], "completed", "{stderr}");
    assert!(
        lines.iter().any(|l| l["event"] == "progress"),
        "progress events missing: {stderr}"
    );
}

#[test]
fn run_cancel_after_exits_130_with_cancelled_document() {
    let fixture = Fixture::new();
    let (code, stdout, stderr) =
        run_cli(&["run", fixture.wav.to_str().unwrap(), "--cancel-after", "2"]);
    assert_eq!(code, 130, "stderr: {stderr}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["status"], "cancelled");
    assert_eq!(doc["error"]["kind"], "cancelled");
    assert!(stderr.contains("\"outcome\":\"cancelled\""), "{stderr}");
}

#[test]
fn run_missing_file_is_a_failed_document_exit_1() {
    let (code, stdout, _stderr) = run_cli(&["run", "/definitely/missing/input.wav"]);
    assert_eq!(code, 1);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["status"], "failed");
    assert_eq!(doc["error"]["kind"], "io");
}

#[test]
fn run_unknown_stream_is_a_failed_document_exit_1() {
    let fixture = Fixture::new();
    let (code, stdout, _stderr) =
        run_cli(&["run", fixture.wav.to_str().unwrap(), "--stream", "99"]);
    assert_eq!(code, 1);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["status"], "failed");
    assert_eq!(doc["error"]["kind"], "usage");
}

#[test]
fn run_native_scale_reports_source_pcm_range() {
    let fixture = Fixture::new();
    let (code, stdout, _stderr) = run_cli(&["run", fixture.wav.to_str().unwrap(), "--native"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["processing"]["scale"], "native-f64");
    assert_eq!(doc["processing"]["lossy"], false);
    let peak = doc["summary"][0]["peak"].as_f64().unwrap();
    assert!((peak - 16_383.0).abs() < 2.0, "native peak {peak}");
}

#[test]
fn argument_errors_exit_2_with_clap_diagnostics() {
    let (code, _stdout, stderr) = run_cli(&["run"]);
    assert_eq!(code, 2);
    assert_ne!(stderr, "");
}
