//! `ruviform-cli-sample` — the Issue #155 minimal CLI adapter over the
//! shared Ruviform headless service (`ruviform_cli_sample::*`).
//!
//! Everything in this file is *application-owned* per the Issue #155
//! responsibility split: command names, option spelling, argument
//! parsing (clap), the stdout/stderr policy and the exit-code mapping.
//! Ruviform supplies the execution control contract the commands sit
//! on — [`OfflineJob`]/`JobEvent`/`CancellationToken` — and the media
//! pipeline in the crate's library half.
//!
//! ## Output contract (sample-owned, JSON)
//!
//! - **stdout**: exactly one JSON document per invocation —
//!   the `probe` report, the `run` report, or a `{"status":"failed"|
//!   "cancelled","error":{…}}` error document. A machine consumer reads
//!   stdout only.
//! - **stderr**: human/machine side channel — one JSON object per
//!   [`JobEvent`] line (JSONL: `started` / `progress` / `finished`),
//!   plus clap's own usage diagnostics.
//! - **exit codes** (this sample's choice — the app owns the meaning):
//!   `0` completed · `1` operation failed (error document on stdout) ·
//!   `2` argument/usage errors (clap's own diagnostics) · `130`
//!   cancelled (SIGINT convention — also when `--cancel-after` fired).

use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ruviform_cli_sample::{
    AnalyzeOptions, PipelineError, error_json, event_json, probe, probe_json, report_json,
    spawn_analyze,
};
use ruviform_exec::{JobError, JobEvent};

/// Issue #155 headless-CLI sample: probe and offline-analyze media
/// files through the shared Ruviform service surface.
#[derive(Parser)]
#[command(
    name = "ruviform-cli-sample",
    version,
    about = "Headless media probe + offline analysis sample built on the shared Ruviform service surface (Issue #155)",
    long_about = "Headless media probe + offline analysis sample built on the shared Ruviform service surface (Issue #155).\n\nOutput contract (sample-owned): stdout carries exactly one JSON document (result or error), stderr carries JSONL progress events plus usage diagnostics. Exit codes: 0 completed, 1 failed, 2 argument errors, 130 cancelled."
)]
struct Cli {
    /// The command to run.
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Probe a media file: enumerate container, backend and per-stream
    /// facts — the read-only "status of the input" operation.
    Probe {
        /// Media file to probe (WAV/FLAC/Ogg/Matroska via Symphonia,
        /// AVI via the pure-Rust AVI backend; routed by signature).
        input: PathBuf,
    },
    /// Decode one audio stream and run the offline peak/RMS analysis
    /// over it, reporting a JSON document on stdout and JSONL progress
    /// on stderr.
    Run {
        /// Media file to analyze.
        input: PathBuf,
        /// Stream id to decode; defaults to the first decodable audio
        /// stream.
        #[arg(long)]
        stream: Option<u32>,
        /// Worker count for the offline driver; 1 is the sequential
        /// determinism-reference path.
        #[arg(long, default_value_t = 1)]
        workers: usize,
        /// Use the native-scale (non-destructive) decoder: levels are
        /// reported in the source PCM scale as f64 instead of
        /// normalized f32.
        #[arg(long)]
        native: bool,
        /// Deterministic cancellation hook for tests and demos: after
        /// N progress events, request job cancellation — the same path
        /// Ctrl-C takes.
        #[arg(long)]
        cancel_after: Option<u64>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Probe { input } => run_probe(&input),
        Commands::Run {
            input,
            stream,
            workers,
            native,
            cancel_after,
        } => run_analyze(&input, stream, workers, native, cancel_after),
    }
}

/// `probe <file>` — synchronous: no job, one document on stdout.
fn run_probe(input: &PathBuf) -> ExitCode {
    let source = match File::open(input) {
        Ok(file) => file,
        Err(e) => return fail_probe(&PipelineError::Usage("cannot open input file"), Some(&e)),
    };
    match probe(source) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&probe_json(&report)).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::from(0)
        }
        Err(e) => fail_probe(&e, None),
    }
}

fn fail_probe(error: &PipelineError, io: Option<&std::io::Error>) -> ExitCode {
    let doc = match io {
        Some(io) => serde_json::json!({
            "status": "failed",
            "operation": "probe",
            "error": {"kind": "io", "message": io.to_string()},
        }),
        None => error_json("probe", error),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
    );
    ExitCode::from(1)
}

/// `run <file>` — spawn the job, stream JSONL events on stderr, print
/// the terminal document on stdout.
fn run_analyze(
    input: &PathBuf,
    stream: Option<u32>,
    workers: usize,
    native: bool,
    cancel_after: Option<u64>,
) -> ExitCode {
    let source = match File::open(input) {
        Ok(file) => file,
        Err(e) => {
            let doc = serde_json::json!({
                "status": "failed",
                "operation": "run",
                "error": {"kind": "io", "message": format!("cannot open input file: {e}")},
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
            );
            return ExitCode::from(1);
        }
    };

    let options = AnalyzeOptions {
        stream,
        workers,
        native,
    };
    let job = spawn_analyze(source, options);

    // Ctrl-C (and SIGBREAK on Windows) flips the job's shared token —
    // the same cancellation path `--cancel-after` and `job.cancel()`
    // take. The handler only flips an AtomicBool; process exit stays
    // with `main`.
    let cancel = job.cancel_token().clone();
    if ctrlc::set_handler(move || cancel.cancel()).is_err() {
        eprintln!("warning: could not install Ctrl-C handler");
    }

    // Event consumer: every JobEvent becomes one stderr JSONL line.
    // `recv` ends when the channel closes on worker exit — the
    // reliable terminal signal (events themselves are best-effort).
    let mut cancelled_after = false;
    while let Ok(event) = job.events().recv() {
        eprintln!("{}", event_json(&event));
        if let JobEvent::Progress { completed, .. } = event
            && !cancelled_after
            && cancel_after.is_some_and(|n| completed >= n)
        {
            job.cancel();
            cancelled_after = true;
        }
    }

    match job.wait() {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report_json(&report)).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::from(0)
        }
        Err(JobError::Operation(e)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&error_json("run", &e))
                    .unwrap_or_else(|_| "{}".into())
            );
            if matches!(e, PipelineError::Cancelled) {
                ExitCode::from(130)
            } else {
                ExitCode::from(1)
            }
        }
        Err(JobError::Panicked) => {
            let doc = serde_json::json!({
                "status": "failed",
                "operation": "run",
                "error": {"kind": "panic", "message": "operation panicked"},
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::from(1)
        }
        Err(_) => {
            let doc = serde_json::json!({
                "status": "failed",
                "operation": "run",
                "error": {"kind": "other", "message": "unclassified job error"},
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::from(1)
        }
    }
}
