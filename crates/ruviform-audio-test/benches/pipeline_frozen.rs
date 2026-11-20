//! Frozen E2E pipeline bench harness (Issue #807 — E-layer, slice 2).
//!
//! Measures the deterministic production audio-pipeline legs of
//! `ruviform-audio-test` end to end — no mocks in the measured path:
//!
//! - `output/session` — scripted output device → production
//!   `ruviform_audio::Session::open` → `pump_output_next` fires the
//!   production `plan.drive` callback (the [`fixture_graph`] node renders
//!   a [`PcmFixture`] into `AudioBufferViewMut`) → captured PCM vs the
//!   independent oracle.
//! - `input/typed-rt` — scripted input stream → `InputCapture` SPSC
//!   handoff → production typed runtime (`input_writer` → `process` →
//!   `output_reader` over a Source→GainNode→Sink graph) → observed PCM
//!   vs the independent oracle scaled by the gain under test.
//!
//! The gate metric is the **normalized wall time** per leg:
//! `norm = median(rep wall ns) / median(ref ns)`, where `ref` is a
//! frozen in-process yardstick kernel (splitmix64 fill + gain over one
//! rep's sample volume) measured in the same run. Normalizing to an
//! in-process reference removes machine-speed variance; the pinned
//! baseline holds a `max_norm_ratio` threshold per cell plus the
//! `expected_digest` fingerprint of the leg's observed PCM — never
//! absolute times. Absolute ns and environment metadata are emitted as
//! informational records only. Correctness is a separate fail-closed
//! layer: oracle PCM mismatches, device errors, capture-pool loss,
//! pipeline errors and rep-over-rep digest divergence each count as
//! errors and fail the cell before the threshold check.
//!
//! Everything runs on the test driver's thread against the scripted
//! backend — **OS-L1 deterministic evidence only** (not WASAPI/WinMM,
//! real threads, GUI, or plugin-ABI evidence).
//!
//! Modes (argv after `--`):
//!
//! - *(none)* — real run: [`WARMUP_REPS`] untimed + [`REPS`] timed reps
//!   per cell, median + min–max range, error counts, work count, limiter
//!   note and environment metadata — one JSON object per line on
//!   stdout — then the pinned-baseline gate. Exit `0` on gate pass,
//!   `1` on gate fail / harness error.
//! - `--test` — smoke compatibility path: [`SMOKE_REPS`] timed rep per
//!   cell, same output shape, gate evaluated report-only
//!   (`gate_mode: "report_only"`): the perf-class failure
//!   (`norm_above_threshold`) stays a per-cell diagnostic and the gate
//!   reports `not_run` — the perf gate never reaches the exit code on
//!   this path. Correctness failures (PCM mismatch, device errors,
//!   capture loss, pipeline errors, divergence, digest drift) are
//!   defects, not machine variance, and still exit `1`.
//! - `--selftest` — injects two real degradations: each rep's measured
//!   wall is floored to `(pinned max_norm_ratio + 1) × ref median`
//!   (deterministic threshold breach) *and* a `StreamFault::Error`
//!   `DeviceError::Xrun` is injected at callback 1 of every rep
//!   (deterministic correctness breach). The gate must fail
//!   (exit `1`) — proves fail-closed behaviour without waiting for a
//!   real regression.
//! - `--write-baseline <path>` — runs the measurement and writes a
//!   baseline *candidate* JSON to `<path>` (`"approval": null`), then
//!   exits `0` without gating. Promotion is manual only: copy the file
//!   over the pinned fixture and fill `approval`. The gate refuses an
//!   un-promoted baseline, so a gate run can never update its own
//!   thresholds.
//! - `--baseline <path>` — gate against `<path>` instead of the pinned
//!   fixture (candidate inspection, staging checks).
//! - `--help` / `-h` — usage; exit `0`.
//!
//! Positional argv is a cell filter (substring of the cell key) — the
//! `cargo bench -- <filter>` convention; a filter matching no cell
//! yields `gate: "not_run"`, exit `0`.
//!
//! Exit codes: `0` gate pass / smoke / no cells selected, `1` gate
//! fail / correctness failure / harness error, `2` CLI misuse.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_audio_core::device::{
    BufferSize, DeviceError, InputStreamConfig, OutputStreamConfig, SupportedBufferSize,
    SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_test::backend::{
    DeviceSpec, InputSpec, OutputSpec, ScriptedBackend, ScriptedDeviceHandle, StreamFault,
    StreamScript,
};
use ruviform_audio_test::clock::{JitterSpec, VirtualClock, VirtualClockConfig};
use ruviform_audio_test::oracle::PcmCompare;
use ruviform_audio_test::pcm::{PcmBlock, PcmFixture, PcmPattern, PcmScenario, PcmSegment};
use ruviform_audio_test::pipeline::{
    InputPipeline, PlaybackPipeline, compare_pcm_block, compare_planar, expected_block,
    expected_fixture_block, fixture_graph, pipeline_compiler_config, scale_expected,
};

/// Timed reps per cell in a real run (`>= 5`, same convention as
/// `dsp_frozen`).
const REPS: usize = 7;
/// Timed reps per cell in `--test` smoke mode.
const SMOKE_REPS: usize = 1;
/// Untimed warmup reps per cell (allocator/page-in warmth before timing).
const WARMUP_REPS: usize = 1;
/// Device callbacks per rep — matches the pipelines' preallocated
/// report/observation capacity (64), so steady-state pumping stays
/// inside the pre-reserved driver buffers.
const BLOCKS: usize = 64;
/// Frames per callback (the stream script's `default_frames`).
const FRAMES: u32 = 256;
/// Stereo legs — the canonical negotiated layout.
const CHANNELS: u16 = 2;
/// Canonical rate (also the `VirtualClock` rate).
const SAMPLE_RATE: u32 = 48_000;
/// Deterministic fixture seed — frozen with the benchmark.
const NOISE_SEED: u64 = 0x0000_0807_c0ff_ee00;
/// Fixture amplitude.
const AMPLITUDE: f32 = 0.5;
/// `GainNode` coefficient for the input leg.
const GAIN: f32 = 0.5;
/// SPSC capture pool depth (recommended range 4–8; drain-per-pump
/// needs only the minimum).
const CAPTURE_POOL: usize = 4;
/// Typed-runtime frame budget — must cover `FRAMES`.
const COMPILER_MAX_FRAMES: usize = 1024;
/// Samples driven through one rep (`BLOCKS × FRAMES × CHANNELS`) —
/// also the ref-yardstick buffer length.
const REP_SAMPLES: usize = BLOCKS * FRAMES as usize * CHANNELS as usize;

/// Pinned baseline schema id.
const BASELINE_SCHEMA: &str = "rvf.e2e-frozen-baseline/v1";
/// Pinned baseline fixture, resolved against the crate at compile time.
const PINNED_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/benches/frozen/pipeline_baseline.json"
);
/// Issue reference recorded in baseline files.
const BASELINE_ISSUE: &str = "https://github.com/scarecrow173/ruviform/issues/807";

/// FNV-1a 64-bit offset basis / prime for the rep digest.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

// ---- frozen fixture/device builders ------------------------------------

/// Stereo F32 @ 48 kHz — the frozen negotiated format.
fn stereo_format() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(SAMPLE_RATE).expect("48 kHz is a valid rate"),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
}

/// `BufferSize::Default` output config — the stream script's
/// `default_frames` then governs frames per callback.
fn out_config() -> OutputStreamConfig {
    OutputStreamConfig {
        format: stereo_format(),
        buffer_size: BufferSize::Default,
    }
}

/// Input-direction config.
fn in_config() -> InputStreamConfig {
    InputStreamConfig {
        format: stereo_format(),
        buffer_size: BufferSize::Default,
    }
}

/// Canonical clock — same convention as the pipeline tests
/// (48 kHz / epoch 7 / no drift / no jitter).
fn bench_clock() -> Arc<VirtualClock> {
    Arc::new(
        VirtualClock::new(VirtualClockConfig {
            sample_rate: SAMPLE_RATE,
            epoch: 7,
            offset_ticks: 0,
            drift_ppm: 0,
            jitter: JitterSpec::DISABLED,
        })
        .expect("clock config is valid"),
    )
}

/// The frozen fixture: deterministic per-channel `Noise` —
/// non-transcendental, so the oracle comparison is bit-exact
/// (`PcmCompare::Exact`).
fn bench_fixture() -> PcmFixture {
    PcmFixture::new(
        SampleRate::new(SAMPLE_RATE).expect("48 kHz is a valid rate"),
        CHANNELS,
        PcmPattern::Noise {
            seed: NOISE_SEED,
            amplitude: AMPLITUDE,
        },
    )
    .expect("fixture params are valid")
}

/// One contiguous segment covering exactly one rep's frame window.
fn bench_scenario() -> PcmScenario {
    PcmScenario::new(vec![PcmSegment {
        frame_start: 0,
        frame_len: u64::from(FRAMES) * BLOCKS as u64,
        fixture: bench_fixture(),
    }])
    .expect("scenario is valid")
}

/// Output capability covering stereo F32 44.1k–96k, any buffer size.
fn output_spec() -> OutputSpec {
    OutputSpec {
        default_format: stereo_format(),
        supported_formats: vec![SupportedOutputFormat {
            channels: u32::from(CHANNELS),
            min_sample_rate: 44_100,
            max_sample_rate: 96_000,
            buffer_size: SupportedBufferSize::Unknown,
            sample_format: SampleFormat::F32,
        }],
        stream: StreamScript::new(FRAMES),
        record_output: true,
    }
}

/// Input capability replaying `pcm`, with capture reporting on.
fn input_spec(pcm: PcmScenario) -> InputSpec {
    InputSpec {
        default_format: stereo_format(),
        supported_formats: vec![SupportedInputFormat {
            channels: u32::from(CHANNELS),
            min_sample_rate: 44_100,
            max_sample_rate: 96_000,
            buffer_size: SupportedBufferSize::Unknown,
            sample_format: SampleFormat::F32,
        }],
        pcm,
        stream: StreamScript::new(FRAMES),
        capture_reporting: true,
    }
}

/// A fresh backend + single-device handle for `spec` — each rep builds
/// its own so reps are fully independent.
fn device_handle(spec: DeviceSpec) -> (ScriptedBackend, ScriptedDeviceHandle) {
    let mut backend = ScriptedBackend::new(bench_clock());
    let handle = backend.add_device(spec).expect("spec is valid");
    (backend, handle)
}

// ---- rep digest ---------------------------------------------------------

fn fnv_u64(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(FNV_PRIME);
    }
    h
}

fn fnv_f32(h: u64, v: f32) -> u64 {
    fnv_u64(h, u64::from(v.to_bits()))
}

/// Fingerprint of the output leg's observed surface: block positions,
/// shapes and PCM bits in pump order.
fn digest_output(blocks: &[PcmBlock]) -> u64 {
    let mut h = FNV_OFFSET;
    for b in blocks {
        h = fnv_u64(h, b.start_frame);
        h = fnv_u64(h, u64::from(b.frames));
        h = fnv_u64(h, u64::from(b.channels));
        for &s in &b.data {
            h = fnv_f32(h, s);
        }
    }
    h
}

/// Fingerprint of the input leg's observed surface: reported frame
/// positions (`None` → `u64::MAX` sentinel, never synthesized), shapes
/// and PCM bits in pump order.
fn digest_input(pipeline: &InputPipeline) -> u64 {
    let mut h = FNV_OFFSET;
    for (i, b) in pipeline.observed_blocks().iter().enumerate() {
        h = fnv_u64(h, b.start_frame.unwrap_or(u64::MAX));
        h = fnv_u64(h, u64::from(b.frames));
        h = fnv_u64(h, u64::from(b.channels));
        if let Some(data) = pipeline.observed_block_data(i) {
            for &s in data {
                h = fnv_f32(h, s);
            }
        }
    }
    h
}

// ---- measured reps ------------------------------------------------------

/// Per-rep observed error counts — the correctness layer. Any nonzero
/// total fails the cell before the perf threshold is consulted.
#[derive(Default)]
struct RepErrors {
    /// Blocks whose PCM failed the independent-oracle compare
    /// (a block-count shortfall counts once).
    pcm_mismatch: usize,
    /// Device errors observed on the error surface (output: per-pump
    /// `errors_fired`; input: drained error-callback values + overflow).
    device_errors: usize,
    /// Input capture handoff loss (starved/overflow/rejected-shape/
    /// recycle-dropped).
    capture_loss: usize,
    /// `PipelineError` aborts (open/pump/stop failures).
    pipeline_errors: usize,
    /// Reps whose observed digest differed from the first measured rep's.
    rep_divergence: usize,
}

impl RepErrors {
    fn total(&self) -> usize {
        self.pcm_mismatch
            + self.device_errors
            + self.capture_loss
            + self.pipeline_errors
            + self.rep_divergence
    }

    fn absorb(&mut self, other: &Self) {
        self.pcm_mismatch += other.pcm_mismatch;
        self.device_errors += other.device_errors;
        self.capture_loss += other.capture_loss;
        self.pipeline_errors += other.pipeline_errors;
        self.rep_divergence += other.rep_divergence;
    }

    /// First nonzero error class — the cell `reason` on a correctness
    /// failure.
    fn first_reason(&self) -> Option<&'static str> {
        if self.pcm_mismatch > 0 {
            Some("pcm_mismatch")
        } else if self.device_errors > 0 {
            Some("device_errors")
        } else if self.capture_loss > 0 {
            Some("capture_loss")
        } else if self.pipeline_errors > 0 {
            Some("pipeline_errors")
        } else if self.rep_divergence > 0 {
            Some("rep_divergence")
        } else {
            None
        }
    }
}

/// What one rep produced.
struct RepOutcome {
    /// Pump wall time; `None` when the rep never produced a measurement
    /// (e.g. an open failure).
    wall_ns: Option<u128>,
    /// Digest of the rep's observed output; `None` when the rep
    /// produced no output to fingerprint.
    digest: Option<u64>,
    errors: RepErrors,
}

/// Applies the `--selftest` wall floor: the rep's recorded wall is
/// raised to `floor` (sleeping the deficit) so `norm` deterministically
/// breaches the pinned threshold.
fn apply_wall_floor(wall: u128, wall_floor: Option<u128>) -> u128 {
    let Some(floor) = wall_floor else {
        return wall;
    };
    let deficit = floor.saturating_sub(wall);
    if deficit > 0 {
        std::thread::sleep(Duration::from_nanos(
            u64::try_from(deficit).unwrap_or(u64::MAX),
        ));
    }
    floor.max(wall)
}

/// One output-leg rep: fresh device → `Session::open` → timed
/// `pump_blocks(BLOCKS)` → captured PCM vs the oracle + digest.
fn output_rep(selftest: bool, wall_floor: Option<u128>) -> RepOutcome {
    let mut out = RepOutcome {
        wall_ns: None,
        digest: None,
        errors: RepErrors::default(),
    };
    let spec = DeviceSpec {
        name: "bench-out".to_owned(),
        id: Some("bench-out-id".to_owned()),
        clock: None,
        output: Some(output_spec()),
        input: None,
    };
    let (_backend, handle) = device_handle(spec);
    if selftest {
        handle.inject_output_fault(StreamFault::Error {
            at_callback: 1,
            error: DeviceError::Xrun,
        });
    }
    let fixture = bench_fixture();
    let Ok(mut pipeline) =
        PlaybackPipeline::open(&handle, out_config(), fixture_graph(Arc::new(fixture)))
    else {
        out.errors.pipeline_errors += 1;
        return out;
    };
    let t0 = Instant::now();
    let pumped = pipeline.pump_blocks(BLOCKS);
    let wall = t0.elapsed().as_nanos();
    out.wall_ns = Some(apply_wall_floor(wall, wall_floor));
    match pumped {
        Err(_) => out.errors.pipeline_errors += 1,
        Ok(reports) => {
            out.errors.device_errors += reports
                .iter()
                .map(|r| usize::try_from(r.errors_fired).unwrap_or(usize::MAX))
                .sum::<usize>();
            let captured = pipeline.captured();
            if captured.len() != BLOCKS {
                out.errors.pcm_mismatch += 1;
            }
            for block in &captured {
                let expected = expected_fixture_block(
                    &fixture,
                    block.start_frame,
                    block.frames,
                    block.channels,
                );
                if compare_pcm_block(block, &expected, PcmCompare::Exact).is_err() {
                    out.errors.pcm_mismatch += 1;
                }
            }
            out.digest = Some(digest_output(&captured));
        }
    }
    if pipeline.stop().is_err() {
        out.errors.pipeline_errors += 1;
    }
    out
}

/// One input-leg rep: fresh device → `InputPipeline::open` → timed
/// `pump_blocks(BLOCKS)` (capture → `input_writer` → `process` →
/// `output_reader` per block) → observed PCM vs oracle × gain + digest.
fn input_rep(selftest: bool, wall_floor: Option<u128>) -> RepOutcome {
    let mut out = RepOutcome {
        wall_ns: None,
        digest: None,
        errors: RepErrors::default(),
    };
    let scenario = bench_scenario();
    let spec = DeviceSpec {
        name: "bench-in".to_owned(),
        id: Some("bench-in-id".to_owned()),
        clock: None,
        output: None,
        input: Some(input_spec(scenario.clone())),
    };
    let (_backend, handle) = device_handle(spec);
    if selftest {
        handle.inject_input_fault(StreamFault::Error {
            at_callback: 1,
            error: DeviceError::Xrun,
        });
    }
    let Ok(mut pipeline) = InputPipeline::open(
        &handle,
        in_config(),
        GAIN,
        pipeline_compiler_config(COMPILER_MAX_FRAMES),
        CAPTURE_POOL,
    ) else {
        out.errors.pipeline_errors += 1;
        return out;
    };
    let t0 = Instant::now();
    let pumped = pipeline.pump_blocks(BLOCKS);
    let wall = t0.elapsed().as_nanos();
    out.wall_ns = Some(apply_wall_floor(wall, wall_floor));
    if pumped.is_err() {
        out.errors.pipeline_errors += 1;
    } else {
        out.errors.device_errors += pipeline.errors().len();
        out.errors.device_errors +=
            usize::try_from(pipeline.error_overflow()).unwrap_or(usize::MAX);
        let c = pipeline.capture_counters();
        out.errors.capture_loss +=
            usize::try_from(c.starved + c.overflow + c.rejected_shape + c.recycle_dropped)
                .unwrap_or(usize::MAX);
        let blocks = pipeline.observed_blocks();
        if blocks.len() != BLOCKS {
            out.errors.pcm_mismatch += 1;
        }
        for (i, b) in blocks.iter().enumerate() {
            let start = b.start_frame.unwrap_or(u64::MAX);
            let Some(data) = pipeline.observed_block_data(i) else {
                out.errors.pcm_mismatch += 1;
                continue;
            };
            let expected = scale_expected(
                &expected_block(&scenario, start, b.frames, b.channels),
                GAIN,
            );
            if compare_planar(data, b.frames, b.channels, &expected, PcmCompare::Exact).is_err() {
                out.errors.pcm_mismatch += 1;
            }
        }
        out.digest = Some(digest_input(&pipeline));
    }
    if pipeline.stop().is_err() {
        out.errors.pipeline_errors += 1;
    }
    out
}

// ---- ref yardstick ------------------------------------------------------

/// Frozen reference-yardstick mixer (splitmix64-style) — frozen here so
/// the `norm` denominator never drifts with library internals.
fn ref_mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// One ref rep: splitmix64-hash fill + gain over one rep's sample
/// volume — the machine-speed yardstick the cell wall is normalized
/// against. `black_box` keeps the fill inside the timed region.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "top-24-bit hash -> f64 is exact enough; f64->f32 is the same narrowing spec as the fixture"
)]
fn ref_rep(buf: &mut [f32], seed: u64) -> u128 {
    let t0 = Instant::now();
    for (i, s) in buf.iter_mut().enumerate() {
        let h = ref_mix64(seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        *s = (((h >> 40) as f64) * (1.0 / 8_388_608.0) - 1.0) as f32 * GAIN;
    }
    black_box(&mut buf[..]);
    t0.elapsed().as_nanos()
}

/// Median / min / max of rep nanoseconds; `(0,0,0)` on empty input.
#[expect(
    clippy::cast_precision_loss,
    reason = "rep nanoseconds are far below f64 precision limits"
)]
fn summarize_ns(samples: &[u128]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut ns: Vec<f64> = samples.iter().map(|&t| t as f64).collect();
    ns.sort_by(f64::total_cmp);
    (ns[ns.len() / 2], ns[0], ns[ns.len() - 1])
}

// ---- cells --------------------------------------------------------------

/// One bench cell — one pipeline leg.
#[derive(Copy, Clone)]
enum CellKind {
    /// `output/session` — `Session::open` + `pump_output_next` +
    /// `plan.drive` + capture.
    Output,
    /// `input/typed-rt` — capture + `input_writer`/`process`/
    /// `output_reader` through the typed runtime.
    Input,
}

/// The frozen cell set.
const CELLS: &[CellKind] = &[CellKind::Output, CellKind::Input];

impl CellKind {
    fn key(self) -> &'static str {
        match self {
            Self::Output => "output/session",
            Self::Input => "input/typed-rt",
        }
    }

    fn leg(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Input => "input",
        }
    }

    /// Per-cell limiter note — what the metric is bounded by
    /// (benchmark-checklist Q1). Recorded next to the metric so each
    /// JSONL row is self-describing; the identification procedure lives
    /// in `docs/test-specs/issue-807-e2e-pipeline-bench.md`.
    fn limiter(self) -> String {
        match self {
            Self::Output => "driver pump + backend callback machinery + Session plan.drive \
             + fixture render + captured-output record — wall normalized to the in-bench \
             mix64 yardstick; regresses on added per-callback cost in the callback path"
                .to_string(),
            Self::Input => "driver pump + capture offer/drain + input_writer copy + typed \
             process (GainNode) + output_reader observe — normalized likewise; regresses \
             on added per-block cost in the capture/runtime path"
                .to_string(),
        }
    }

    fn rep(self, selftest: bool, wall_floor: Option<u128>) -> RepOutcome {
        match self {
            Self::Output => output_rep(selftest, wall_floor),
            Self::Input => input_rep(selftest, wall_floor),
        }
    }
}

/// One cell's measurement + gate outcome (one JSONL row).
struct CellReport {
    key: String,
    leg: &'static str,
    reps: usize,
    /// `median(rep wall) / median(ref)` — `None` when no rep produced a
    /// measurement.
    norm: Option<f64>,
    median_ns: f64,
    min_ns: f64,
    max_ns: f64,
    ref_median_ns: f64,
    /// First measured rep's digest — compared against the pinned
    /// `expected_digest`.
    digest: Option<u64>,
    errors: RepErrors,
    /// Samples driven through the leg: `REP_SAMPLES × reps`.
    work_count: u64,
    limiter: String,
    max_norm_ratio: Option<f64>,
    verdict: &'static str,
    reason: Option<String>,
}

/// The `--selftest` wall floor: `(pinned max_norm_ratio + 1) × ref
/// median` in nanoseconds — a guaranteed `norm_above_threshold` breach.
/// `4.0` stands in when the cell has no pin (that cell fails closed on
/// the baseline anyway).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "median * (pin + 1) is small positive nanoseconds"
)]
fn wall_floor_ns(pin: Option<&CellPin>, ref_median_ns: f64) -> u128 {
    let pin_v = pin.map_or(4.0, |p| p.max_norm_ratio);
    (ref_median_ns * (pin_v + 1.0)) as u128
}

/// Runs `kind`'s warmup + timed reps and folds them into a report.
/// `wall_floor` (`Some` only under `--selftest`) raises each rep's
/// recorded wall to breach the pinned threshold deterministically.
fn run_cell(
    kind: CellKind,
    pin: Option<&CellPin>,
    reps: usize,
    ref_median_ns: f64,
    selftest: bool,
) -> CellReport {
    for _ in 0..WARMUP_REPS {
        black_box(kind.rep(false, None));
    }
    let wall_floor = selftest.then(|| wall_floor_ns(pin, ref_median_ns));
    let mut samples = Vec::with_capacity(reps);
    let mut digests = Vec::with_capacity(reps);
    let mut errors = RepErrors::default();
    for _ in 0..reps {
        let r = kind.rep(selftest, wall_floor);
        if let Some(w) = r.wall_ns {
            samples.push(w);
        }
        digests.push(r.digest);
        errors.absorb(&r.errors);
    }
    let reference = digests.iter().flatten().next().copied();
    errors.rep_divergence =
        reference.map_or(0, |r| digests.iter().flatten().filter(|&&d| d != r).count());
    let (median_ns, min_ns, max_ns) = summarize_ns(&samples);
    let norm = if samples.is_empty() || ref_median_ns <= 0.0 {
        None
    } else {
        Some(median_ns / ref_median_ns)
    };
    CellReport {
        key: kind.key().to_string(),
        leg: kind.leg(),
        reps,
        norm,
        median_ns,
        min_ns,
        max_ns,
        ref_median_ns,
        digest: reference,
        work_count: u64::try_from(REP_SAMPLES * reps).unwrap_or(u64::MAX),
        limiter: kind.limiter(),
        errors,
        max_norm_ratio: None,
        verdict: "not_run",
        reason: None,
    }
}

// ---- CLI ----------------------------------------------------------------

/// CLI mode after parsing.
enum Mode {
    /// Real run + gate.
    Run,
    /// Smoke: fewer reps, gate report-only.
    Smoke,
    /// Injected degradations: gate must fail.
    Selftest,
    /// Emit a baseline candidate file; no gate.
    WriteBaseline(PathBuf),
}

/// Parsed argv.
struct Args {
    mode: Mode,
    baseline: PathBuf,
    filters: Vec<String>,
}

/// Arg-parse failure — prints usage, exits `2` (`0` for `--help`).
struct Misuse(String);

fn usage() -> String {
    "pipeline_frozen — frozen E2E pipeline bench (Issue #807 slice 2)\n\
     usage: pipeline_frozen [--test|--selftest|--write-baseline <path>] \
     [--baseline <path>] [filter]...\n\
     \x20 (no args)             real run: warmup + timed reps + pinned-baseline gate \
     (exit 1 on fail)\n\
     \x20 --test                smoke: 1 timed rep per cell, gate report-only, exit 0 \
     (correctness failures still exit 1)\n\
     \x20 --selftest            inject wall-floor + Xrun degradations; gate must fail (exit 1)\n\
     \x20 --write-baseline <p>  write a baseline candidate JSON to <p>, exit 0\n\
     \x20 --baseline <p>        gate against <p> instead of the pinned fixture\n\
     \x20 -h, --help            this text\n\
     \x20 filter               cell filter substring (cell key), cargo-bench convention\n\
     exit: 0 pass/smoke/no cells, 1 gate fail or harness error, 2 misuse"
        .to_string()
}

fn parse_args(argv: &[String]) -> Result<Args, Misuse> {
    let mut mode: Option<Mode> = None;
    let mut baseline = PathBuf::from(PINNED_BASELINE);
    let mut filters = Vec::new();
    let mut set_mode = |m: Mode, flag: &str| -> Result<(), Misuse> {
        if mode.is_some() {
            return Err(Misuse(format!(
                "{flag} conflicts with an earlier mode flag"
            )));
        }
        mode = Some(m);
        Ok(())
    };
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--test" => set_mode(Mode::Smoke, "--test")?,
            "--selftest" => set_mode(Mode::Selftest, "--selftest")?,
            "--write-baseline" => {
                i += 1;
                let Some(p) = argv.get(i) else {
                    return Err(Misuse("--write-baseline needs a path".to_string()));
                };
                set_mode(Mode::WriteBaseline(PathBuf::from(p)), "--write-baseline")?;
            }
            "--baseline" => {
                i += 1;
                let Some(p) = argv.get(i) else {
                    return Err(Misuse("--baseline needs a path".to_string()));
                };
                baseline = PathBuf::from(p);
            }
            "--" | "--bench" | "--quiet" | "--exact" | "--nocapture" | "--show-output" => {}
            "-h" | "--help" => return Err(Misuse(String::new())),
            s if s.starts_with('-') => return Err(Misuse(format!("unknown flag {s}"))),
            s => filters.push(s.to_string()),
        }
        i += 1;
    }
    Ok(Args {
        mode: mode.unwrap_or(Mode::Run),
        baseline,
        filters,
    })
}

// ---- baseline -----------------------------------------------------------

/// One cell's pinned gate input.
struct CellPin {
    /// Normalized-wall ceiling for this cell.
    max_norm_ratio: f64,
    /// Frozen fingerprint of the leg's observed output.
    expected_digest: u64,
}

/// Parses the pinned baseline via `serde_json` (already a crate
/// dependency — unlike `dsp_frozen`, no hand-rolled reader is needed).
/// `schema` must match, `approval` must be non-null (a `null` approval
/// is an un-promoted candidate — refuse), and every cell must carry a
/// numeric `max_norm_ratio` and a `0x`-hex `expected_digest`.
fn parse_baseline(text: &str) -> Result<BTreeMap<String, CellPin>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("json: {e}"))?;
    if v.get("schema").and_then(serde_json::Value::as_str) != Some(BASELINE_SCHEMA) {
        return Err(format!("missing or wrong schema (want {BASELINE_SCHEMA})"));
    }
    if v.get("approval").is_none_or(serde_json::Value::is_null) {
        return Err(
            "baseline has no approval — an un-promoted candidate is not a gate input \
             (promotion is manual: copy the candidate file and fill `approval`)"
                .to_string(),
        );
    }
    let cells_obj = v
        .get("cells")
        .and_then(serde_json::Value::as_object)
        .ok_or("cells missing")?;
    let mut cells = BTreeMap::new();
    for (key, cell) in cells_obj {
        let max_norm_ratio = cell
            .get("max_norm_ratio")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("cell {key} has no max_norm_ratio"))?;
        let digest_s = cell
            .get("expected_digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("cell {key} has no expected_digest"))?;
        let expected_digest =
            u64::from_str_radix(digest_s.strip_prefix("0x").unwrap_or(digest_s), 16)
                .map_err(|e| format!("cell {key} expected_digest: {e}"))?;
        cells.insert(
            key.clone(),
            CellPin {
                max_norm_ratio,
                expected_digest,
            },
        );
    }
    Ok(cells)
}

// ---- JSONL output -------------------------------------------------------

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", u32::from(c as u8));
            }
            c => o.push(c),
        }
    }
    o
}

/// JSON-quoted string.
fn js(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

/// `Option<&str>` as a JSON string or `null`.
fn js_opt(s: Option<&str>) -> String {
    s.map_or_else(|| "null".to_string(), js)
}

/// `Option<f64>` as a JSON number (3 decimals) or `null`.
fn num_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "null".to_string(), |x| format!("{x:.3}"))
}

/// `Option<u64>` as a `0x`-hex JSON string or `null`.
fn digest_opt(d: Option<u64>) -> String {
    d.map_or_else(|| "null".to_string(), |x| js(&format!("0x{x:016x}")))
}

/// Environment helpers — same reporting `dsp_frozen` prints for its
/// run records, kept local because bench targets cannot share modules.
#[cfg(target_os = "windows")]
fn cpu_model() -> String {
    if let Ok(out) = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
            "/v",
            "ProcessorNameString",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.contains("ProcessorNameString")
                && let Some((_, name)) = line.split_once("REG_SZ")
            {
                return name.trim().to_string();
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(target_os = "linux")]
fn cpu_model() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':')
                && key.trim() == "model name"
            {
                return value.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

#[cfg(target_os = "macos")]
fn cpu_model() -> String {
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        && out.status.success()
    {
        return String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    "unknown".to_string()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_model() -> String {
    "unknown".to_string()
}

fn core_count() -> u64 {
    std::thread::available_parallelism().map_or(0, |n| n.get() as u64)
}

fn rustc_version() -> String {
    std::process::Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |out| String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
}

fn repo_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |out| String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
}

/// Emits the `run` header line.
fn emit_run(mode: &str, reps: usize, baseline: Option<&Path>, selftest: bool) {
    let injected_json = if selftest {
        concat!(
            "[{\"kind\":\"wall_floor\",\"detail\":\"each rep wall floored to ",
            "(pinned max_norm_ratio + 1) * ref_median — deterministic threshold breach\"},",
            "{\"kind\":\"stream_fault_xrun\",\"detail\":\"StreamFault::Error ",
            "DeviceError::Xrun injected at callback 1 of every rep — deterministic ",
            "device-errors correctness breach\"}]"
        )
    } else {
        "null"
    };
    println!(
        "{{\"type\":\"run\",\"schema\":\"rvf.e2e-frozen.run/v1\",\"lane\":\"bench-frozen-e2e\",\
         \"tool\":\"pipeline_frozen\",\"mode\":{},\"os\":{},\"arch\":{},\"cpu\":{},\"cores\":{},\
         \"rustc\":{},\"repo_sha\":{},\"metric\":\"normalized_wall_vs_ref\",\
         \"ref_kernel\":\"mix64_fill_gain\",\"reps\":{},\"warmup_reps\":{},\
         \"blocks_per_rep\":{},\"frames_per_callback\":{},\"channels\":{},\
         \"sample_rate\":{},\"fixture\":{{\"pattern\":\"noise\",\"seed\":{},\"amplitude\":{}}},\
         \"gain\":{},\"baseline\":{},\"injected\":{}}}",
        js(mode),
        js(std::env::consts::OS),
        js(std::env::consts::ARCH),
        js(&cpu_model()),
        core_count(),
        js(&rustc_version()),
        js(&repo_sha()),
        reps,
        WARMUP_REPS,
        BLOCKS,
        FRAMES,
        CHANNELS,
        SAMPLE_RATE,
        NOISE_SEED,
        AMPLITUDE,
        GAIN,
        baseline.map_or_else(|| "null".to_string(), |p| js(&p.display().to_string())),
        injected_json,
    );
}

/// Emits the ref-yardstick record — the `norm` denominator.
fn emit_ref(reps: usize, median_ns: f64, min_ns: f64, max_ns: f64) {
    println!(
        "{{\"type\":\"ref_kernel\",\"kernel\":\"mix64_fill_gain\",\
         \"samples_per_rep\":{REP_SAMPLES},\"reps\":{reps},\"median_ns\":{median_ns:.1},\
         \"min_ns\":{min_ns:.1},\"max_ns\":{max_ns:.1}}}"
    );
}

fn emit_cell(r: &CellReport) {
    let e = &r.errors;
    println!(
        "{{\"type\":\"cell\",\"key\":{},\"leg\":{},\"norm\":{},\"max_norm_ratio\":{},\
         \"reps\":{},\"median_ns\":{:.1},\"min_ns\":{:.1},\"max_ns\":{:.1},\
         \"ref_median_ns\":{:.1},\"digest\":{},\"errors\":{},\
         \"error_detail\":{{\"pcm_mismatch\":{},\"device_errors\":{},\"capture_loss\":{},\
         \"pipeline_errors\":{},\"rep_divergence\":{}}},\
         \"work_count\":{},\"limiter\":{},\"verdict\":{},\"reason\":{}}}",
        js(&r.key),
        js(r.leg),
        num_opt(r.norm),
        num_opt(r.max_norm_ratio),
        r.reps,
        r.median_ns,
        r.min_ns,
        r.max_ns,
        r.ref_median_ns,
        digest_opt(r.digest),
        e.total(),
        e.pcm_mismatch,
        e.device_errors,
        e.capture_loss,
        e.pipeline_errors,
        e.rep_divergence,
        r.work_count,
        js(&r.limiter),
        js(r.verdict),
        js_opt(r.reason.as_deref()),
    );
}

// ---- gate ---------------------------------------------------------------

/// Per-cell gate evaluation: returns `(verdict, reason, max_norm_ratio)`.
fn judge(
    baseline: &Result<BTreeMap<String, CellPin>, String>,
    r: &CellReport,
) -> (&'static str, Option<String>, Option<f64>) {
    if r.errors.total() > 0 {
        return ("fail", r.errors.first_reason().map(str::to_string), None);
    }
    let Ok(map) = baseline else {
        return ("not_run", Some("baseline_unreadable".to_string()), None);
    };
    let Some(pin) = map.get(&r.key) else {
        return ("unsupported", Some("no_pinned_threshold".to_string()), None);
    };
    if r.digest != Some(pin.expected_digest) {
        return (
            "fail",
            Some("digest_mismatch".to_string()),
            Some(pin.max_norm_ratio),
        );
    }
    let Some(norm) = r.norm else {
        return ("not_run", Some("no_measurement".to_string()), None);
    };
    if norm <= pin.max_norm_ratio {
        ("pass", None, Some(pin.max_norm_ratio))
    } else {
        (
            "fail",
            Some(format!(
                "norm_above_threshold ({norm:.3} > {:.2})",
                pin.max_norm_ratio
            )),
            Some(pin.max_norm_ratio),
        )
    }
}

/// Whether a fail `reason` is perf-class (exempt from the smoke exit
/// code) — every other failure class is a correctness defect.
fn is_perf_reason(reason: Option<&str>) -> bool {
    reason.is_some_and(|r| r.starts_with("norm_above_threshold"))
}

/// Writes the baseline candidate JSON (tmp + rename, same atomic-write
/// convention as `dsp_frozen` / `BaselineStore`). `"approval": null`
/// keeps the file an un-promoted candidate — the gate refuses it until
/// a human fills `approval` and copies it over the pinned fixture. A
/// cell that produced no `norm` writes `max_norm_ratio: null`, which
/// the parser refuses — an un-measured cell can never be promoted.
fn write_candidate(path: &Path, results: &[CellReport], ref_median_ns: f64) -> std::io::Result<()> {
    let mut s = String::new();
    s.push_str("{\n");
    let _ = writeln!(s, "  \"schema\": {},", js(BASELINE_SCHEMA));
    let _ = writeln!(s, "  \"issue\": {},", js(BASELINE_ISSUE));
    s.push_str("  \"metric\": \"normalized_wall_vs_ref\",\n");
    s.push_str("  \"ref_kernel\": \"mix64_fill_gain\",\n");
    s.push_str(
        "  \"threshold_rule\": \"max_norm_ratio = ceil(measured_norm * 1.5, 0.01) — \
         fails when the leg's normalized wall grows 1.5x past its pin (machine \
         speed normalized out by the in-process ref yardstick; slack absorbs \
         run-to-run noise)\",\n",
    );
    s.push_str("  \"measured_env\": ");
    let _ = writeln!(
        s,
        "{{\"os\": {}, \"arch\": {}, \"cpu\": {}, \"cores\": {}, \"rustc\": {}, \
         \"repo_sha\": {}}},",
        js(std::env::consts::OS),
        js(std::env::consts::ARCH),
        js(&cpu_model()),
        core_count(),
        js(&rustc_version()),
        js(&repo_sha()),
    );
    s.push_str("  \"cells\": {\n");
    for (i, r) in results.iter().enumerate() {
        let sep = if i + 1 == results.len() { "" } else { "," };
        let pin_json = r.norm.map_or_else(
            || "null".to_string(),
            |n| format!("{:.2}", (n * 150.0).ceil() / 100.0),
        );
        let _ = writeln!(
            s,
            "    {}: {{\"max_norm_ratio\": {pin_json}, \"measured_norm\": {}, \
             \"measured_median_ns\": {:.1}, \"measured_ref_median_ns\": {ref_median_ns:.1}, \
             \"expected_digest\": {}, \"errors\": {}, \"work_count\": {}}}{sep}",
            js(&r.key),
            num_opt(r.norm),
            r.median_ns,
            digest_opt(r.digest),
            r.errors.total(),
            r.work_count,
        );
    }
    s.push_str("  },\n");
    s.push_str("  \"limiter\": {\n");
    for (i, r) in results.iter().enumerate() {
        let sep = if i + 1 == results.len() { "" } else { "," };
        let _ = writeln!(s, "    {}: {}{sep}", js(&r.key), js(&r.limiter));
    }
    s.push_str("  },\n");
    s.push_str("  \"approval\": null\n");
    s.push_str("}\n");
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

// ---- driver -------------------------------------------------------------

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&raw) {
        Ok(a) => a,
        Err(Misuse(m)) => {
            if m.is_empty() {
                println!("{}", usage());
                return ExitCode::SUCCESS;
            }
            eprintln!("{m}\n{}", usage());
            return ExitCode::from(2);
        }
    };
    ExitCode::from(run(&args))
}

/// Per-run verdict counters for the gate summary line.
struct Tally {
    cells: u32,
    passed: u32,
    failed: u32,
    unsupported: u32,
    not_run: u32,
    errors: usize,
}

/// Measures every selected cell and emits its record; returns the gate
/// counters plus the reports (`write-baseline` reuses them).
fn measure_cells(
    args: &Args,
    baseline: &Result<BTreeMap<String, CellPin>, String>,
    ref_median_ns: f64,
    gating: bool,
    reps: usize,
    selftest: bool,
) -> (Tally, Vec<CellReport>) {
    let mut t = Tally {
        cells: 0,
        passed: 0,
        failed: 0,
        unsupported: 0,
        not_run: 0,
        errors: 0,
    };
    let mut results: Vec<CellReport> = Vec::new();
    for &kind in CELLS {
        let key = kind.key();
        if !args.filters.is_empty() && !args.filters.iter().any(|f| key.contains(f.as_str())) {
            continue;
        }
        t.cells += 1;
        let pin = baseline.as_ref().ok().and_then(|m| m.get(key));
        let mut r = run_cell(kind, pin, reps, ref_median_ns, selftest);
        t.errors += r.errors.total();
        if gating {
            let (verdict, reason, max_norm_ratio) = judge(baseline, &r);
            r.verdict = verdict;
            r.reason = reason;
            r.max_norm_ratio = max_norm_ratio;
        } else {
            r.reason = Some("write_baseline_mode".to_string());
        }
        match r.verdict {
            "pass" => t.passed += 1,
            "fail" => t.failed += 1,
            "unsupported" => t.unsupported += 1,
            _ => t.not_run += 1,
        }
        emit_cell(&r);
        results.push(r);
    }
    (t, results)
}

/// Emits the `baseline_candidate` record and maps the write result to
/// the process exit code.
fn emit_candidate_result(path: &Path, cells: u32, outcome: std::io::Result<()>) -> u8 {
    match outcome {
        Ok(()) => {
            println!(
                "{{\"type\":\"baseline_candidate\",\"path\":{},\"cells\":{},\
                 \"approval\":null,\"verdict\":\"written\"}}",
                js(&path.display().to_string()),
                cells,
            );
            0
        }
        Err(e) => {
            println!(
                "{{\"type\":\"baseline_candidate\",\"path\":{},\"verdict\":\"fail\",\
                 \"reason\":{}}}",
                js(&path.display().to_string()),
                js(&e.to_string()),
            );
            1
        }
    }
}

/// Whole-mode driver — returns the process exit code.
fn run(args: &Args) -> u8 {
    let selftest = matches!(args.mode, Mode::Selftest);
    let reps = match args.mode {
        Mode::Smoke => SMOKE_REPS,
        _ => REPS,
    };
    let gating = !matches!(args.mode, Mode::WriteBaseline(_));
    let report_only = matches!(args.mode, Mode::Smoke);
    let mode_name = match args.mode {
        Mode::Run => "run",
        Mode::Smoke => "test",
        Mode::Selftest => "selftest",
        Mode::WriteBaseline(_) => "write-baseline",
    };

    let baseline: Result<BTreeMap<String, CellPin>, String> = if gating {
        std::fs::read_to_string(&args.baseline)
            .map_err(|e| format!("{}: {e}", args.baseline.display()))
            .and_then(|t| parse_baseline(&t))
    } else {
        Ok(BTreeMap::new())
    };

    // Ref-yardstick measurement — the same reps/warmup shape as the
    // cells, so `norm` compares like-for-like.
    let ref_reps = reps;
    let mut ref_buf = vec![0.0f32; REP_SAMPLES];
    for _ in 0..WARMUP_REPS {
        black_box(ref_rep(&mut ref_buf, NOISE_SEED));
    }
    let mut ref_samples = Vec::with_capacity(ref_reps);
    for _ in 0..ref_reps {
        ref_samples.push(ref_rep(&mut ref_buf, NOISE_SEED));
    }
    let (ref_median_ns, ref_min_ns, ref_max_ns) = summarize_ns(&ref_samples);

    emit_run(
        mode_name,
        reps,
        gating.then_some(args.baseline.as_path()),
        selftest,
    );
    emit_ref(ref_reps, ref_median_ns, ref_min_ns, ref_max_ns);

    let (tally, results) = measure_cells(args, &baseline, ref_median_ns, gating, reps, selftest);
    let Tally {
        cells,
        passed,
        failed,
        unsupported,
        not_run,
        errors: errors_total,
    } = tally;

    if let Mode::WriteBaseline(path) = &args.mode {
        return emit_candidate_result(path, cells, write_candidate(path, &results, ref_median_ns));
    }

    // Gate line: `fail` wins over `pass`; a baseline that cannot be
    // read or was never promoted fails the gate closed. In smoke
    // (`report_only`) the perf gate never reaches the exit code —
    // threshold cell failures stay per-cell diagnostics — but a
    // correctness failure is a defect, not machine variance, and still
    // fails hard.
    let hard_failures = results
        .iter()
        .filter(|r| r.verdict == "fail" && !is_perf_reason(r.reason.as_deref()))
        .count();
    let gate_mode = if report_only {
        "report_only"
    } else {
        "fail_closed"
    };
    let verdict = if hard_failures > 0 {
        "fail"
    } else if !gating || cells == 0 || report_only {
        "not_run"
    } else if failed > 0 || baseline.is_err() {
        // A cell failed, or the pinned baseline itself cannot be read —
        // the gate fails closed either way.
        "fail"
    } else if passed > 0 {
        "pass"
    } else if unsupported > 0 {
        "unsupported"
    } else {
        "not_run"
    };
    let baseline_status = baseline
        .as_ref()
        .map_or_else(|e| js(&format!("unreadable: {e}")), |_| js("ok"));
    println!(
        "{{\"type\":\"gate\",\"gate_mode\":{},\"baseline\":{},\"baseline_status\":{},\
         \"cells\":{},\"pass\":{},\"fail\":{},\"unsupported\":{},\"not_run\":{},\
         \"errors\":{},\"verdict\":{}}}",
        js(gate_mode),
        js(&args.baseline.display().to_string()),
        baseline_status,
        cells,
        passed,
        failed,
        unsupported,
        not_run,
        errors_total,
        js(verdict),
    );
    u8::from(verdict == "fail")
}
