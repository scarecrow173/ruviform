//! Reference [`Service`] implementation shared by `demo_cli`
//! (headless) and `demo_app` (GUI) — proof that the same non-UI
//! computation drives both entry points.
//!
//! `demo.synthesize` generates a deterministic tone, computes min/max
//! peak pairs per window through `ruviform_exec::OfflineDriver`
//! (real chunk-boundary cancellation + progress), and publishes the
//! result as a `rvf://data`-pullable [`VecDataSource`]. The remaining
//! ops exist to exercise the protocol's error/cancel paths.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ruviform_exec::{OfflineConfig, OfflineDriver, OfflineError, Progress};
use ruviform_rt::{Processor, RtContext, RtProcessor};
use serde_json::{Value, json};

use crate::data::VecDataSource;
use crate::ipc::{ErrorBody, codes};
use crate::service::{CallOutcome, JobSpec, OpInfo, Service, ServiceContext};

const OPS: &[OpInfo] = &[
    OpInfo {
        name: "demo.echo",
        summary: "Echo params back (connectivity check)",
    },
    OpInfo {
        name: "demo.synthesize",
        summary: "Synthesize a tone and publish min/max peaks via rvf://data",
    },
    OpInfo {
        name: "demo.sleep",
        summary: "Sleep job with progress (cancellation testing)",
    },
    OpInfo {
        name: "demo.fail",
        summary: "Immediate service error (error-path testing)",
    },
    OpInfo {
        name: "demo.panic",
        summary: "Job that panics (crash handling testing)",
    },
];

const PEAK_WINDOW: usize = 480;

const MAX_SECONDS: u64 = 600;
const MAX_SAMPLE_RATE: u64 = 384_000;
const MIN_SAMPLE_RATE: u64 = 8_000;
const MAX_SLEEP_MS: u64 = 300_000;

/// The shared demo service. See module docs.
pub struct DemoService;

impl DemoService {
    /// Creates the demo service.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DemoService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for DemoService {
    fn ops(&self) -> &'static [OpInfo] {
        OPS
    }

    fn call(
        &mut self,
        op: &str,
        params: &Value,
        _ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        match op {
            "demo.echo" => Ok(CallOutcome::Result(json!({ "echo": params }))),
            "demo.fail" => Err(ErrorBody::new(
                codes::SERVICE_ERROR,
                "intentional demo failure",
            )),
            "demo.panic" => Ok(CallOutcome::Job(JobSpec::new("demo.panic", |_| {
                panic!("intentional demo panic");
            }))),
            "demo.sleep" => {
                let ms = param_u64(params, "ms")?.unwrap_or(1_000);
                if ms > MAX_SLEEP_MS {
                    return Err(invalid(format!("ms must be <= {MAX_SLEEP_MS}")));
                }
                Ok(CallOutcome::Job(sleep_job(ms)))
            }
            "demo.synthesize" => {
                let seconds = param_u64(params, "seconds")?.unwrap_or(30);
                let rate = param_u64(params, "sample_rate")?.unwrap_or(48_000);
                let frames = param_u64(params, "chunk_frames")?.map_or(Ok(rate), |f| {
                    if f == 0 || f > rate {
                        Err(invalid("chunk_frames must be in 1..=sample_rate"))
                    } else {
                        Ok(f)
                    }
                })?;
                if seconds == 0 || seconds > MAX_SECONDS {
                    return Err(invalid(format!("seconds must be in 1..={MAX_SECONDS}")));
                }
                if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&rate) {
                    return Err(invalid(format!(
                        "sample_rate must be in {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE}"
                    )));
                }
                let frames = usize::try_from(frames)
                    .map_err(|_| invalid("chunk_frames does not fit usize"))?;
                Ok(CallOutcome::Job(synthesize_job(seconds, rate, frames)))
            }
            _ => Err(ErrorBody::new(
                codes::UNKNOWN_OP,
                format!("unregistered demo op {op:?}"),
            )),
        }
    }
}

fn invalid(message: impl Into<String>) -> ErrorBody {
    ErrorBody::new(codes::INVALID_PARAMS, message)
}

fn param_u64(params: &Value, key: &str) -> Result<Option<u64>, ErrorBody> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| invalid(format!("{key} must be a non-negative integer"))),
    }
}

fn sleep_job(ms: u64) -> JobSpec {
    JobSpec::new("demo.sleep", move |ctx| {
        const STEP_MS: u64 = 10;
        let mut elapsed = 0u64;
        while elapsed < ms {
            ctx.check_cancel()?;
            ctx.progress_detail(elapsed, ms, "sleeping");
            thread::sleep(Duration::from_millis(STEP_MS));
            elapsed += STEP_MS;
        }
        ctx.progress_detail(ms, ms, "sleeping");
        Ok(json!({ "slept_ms": ms }))
    })
}

fn synthesize_job(seconds: u64, rate: u64, frames: usize) -> JobSpec {
    JobSpec::new("demo.synthesize", move |ctx| {
        let chunks: Vec<PcmChunk> = (0..seconds)
            .map(|s| PcmChunk::synth(s, frames, rate))
            .collect();
        // Drive the compute on a helper thread so this worker can relay
        // OfflineDriver's progress + cancellation through JobContext —
        // the same contract any app service gets.
        let progress = Progress::new();
        let cfg = OfflineConfig {
            workers: 4,
            queue_capacity: 0,
        };
        let worker = {
            let cancel = ctx.cancel_token();
            let progress = progress.clone();
            thread::spawn(move || {
                OfflineDriver::run_parallel(
                    cfg,
                    &PeaksStage,
                    PEAK_WINDOW,
                    chunks,
                    &cancel,
                    &progress,
                )
            })
        };
        while !worker.is_finished() {
            let total = progress.total();
            if total > 0 {
                ctx.progress_detail(progress.completed(), total, "peaks");
            }
            thread::sleep(Duration::from_millis(20));
        }
        ctx.progress_detail(progress.completed(), progress.total(), "peaks");
        let chunks = worker
            .join()
            .map_err(|_| ErrorBody::new(codes::WORKER_FAILED, "peak worker panicked"))?
            .map_err(|e| map_offline_err(&e))?;

        let points: usize = chunks.iter().map(|c| c.peaks.len()).sum();
        let mut bytes = Vec::with_capacity(points * 8);
        for chunk in &chunks {
            for [lo, hi] in &chunk.peaks {
                bytes.extend_from_slice(&lo.to_le_bytes());
                bytes.extend_from_slice(&hi.to_le_bytes());
            }
        }
        // Publish the display payload on the pull path — never inline
        // bulk bytes into a JSON event.
        ctx.data()
            .register("demo/peaks", Arc::new(VecDataSource::new(bytes)));
        Ok(json!({
            "source": "demo/peaks",
            "format": "f32le-pair",
            "points": points,
            "bytes": points * 8,
            "seconds": seconds,
            "sample_rate": rate,
        }))
    })
}

fn map_offline_err(e: &OfflineError) -> ErrorBody {
    match e {
        OfflineError::Cancelled => ErrorBody::new(codes::CANCELLED, "synthesis cancelled"),
        _ => ErrorBody::new(codes::WORKER_FAILED, e.to_string()),
    }
}

/// One synthesis chunk: PCM in, min/max peak pairs out (same `Io`
/// type for `OfflineDriver`'s in-place processing contract).
struct PcmChunk {
    pcm: Vec<f32>,
    peaks: Vec<[f32; 2]>,
}

impl PcmChunk {
    /// Deterministic amplitude-modulated tone. `f32::sin` is used for
    /// simplicity — bit-exactness across platforms is not a demo
    /// contract (determinism holds within a build).
    fn synth(index: u64, frames: usize, rate: u64) -> Self {
        let len = u64::try_from(frames).unwrap_or(u64::MAX);
        let base = index.saturating_mul(len);
        let pcm = (0..len).map(|i| sample(base + i, rate)).collect();
        Self {
            pcm,
            peaks: Vec::new(),
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "sample indices stay far below the f32 exact-integer range at demo rates"
)]
fn sample(n: u64, rate: u64) -> f32 {
    let t = n as f32 / rate as f32;
    0.6 * (t * 440.0 * std::f32::consts::TAU).sin()
        * (0.5 + 0.5 * (t * 0.5 * std::f32::consts::TAU).sin())
}

#[derive(Clone)]
struct PeaksStage;

impl Processor for PeaksStage {
    type Setup = usize;
    type Rt = PeaksKernel;
    type Error = std::convert::Infallible;

    fn prepare(&mut self, window: usize) -> Result<PeaksKernel, std::convert::Infallible> {
        Ok(PeaksKernel { window })
    }
}

struct PeaksKernel {
    window: usize,
}

impl RtProcessor for PeaksKernel {
    type Io = PcmChunk;

    fn process(&mut self, io: &mut PcmChunk, _ctx: &RtContext) {
        let window = self.window.max(1);
        for w in io.pcm.chunks(window) {
            let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
            for &x in w {
                if x < lo {
                    lo = x;
                }
                if x > hi {
                    hi = x;
                }
            }
            io.peaks.push([lo, hi]);
        }
        // The PCM has served its purpose — free the bulk before the
        // chunk is queued back to the collector.
        io.pcm = Vec::new();
    }
}
