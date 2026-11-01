//! SIMD `PoC` benchmark harness (Issue #19, task T5).
//!
//! Measures the design-doc bench matrix — `kernel × impl column × input ×
//! len` — as one `BenchmarkGroup` per kernel and one `bench_function` per
//! cell:
//!
//! - **columns**: `registry()` minus the equivalence-only `fearless_sse2` /
//!   `pulp_v2` (`SSE2` is the `x86_64` baseline, so `fearless_sse4_2` is the v2
//!   tier's bench representative). Columns whose forced level is
//!   unconstructible on the host are skipped via `probe`, never panicked.
//! - **sizes**: `64 / 256 / 1024 / 4096` plus the ragged `100 / 1000 / 1023`.
//!   `biquad` fixes `channels = 8` and varies frames over the same series.
//! - **inputs**: `normal` for every kernel plus `denormal` for the
//!   accumulating kernels (`dot` / `fir` / `biquad` / `sine`) — the
//!   denormal-cliff observation variant. `edge` is equivalence-only and is
//!   never benched.
//! - **ids**: `BenchmarkId::new(<impl>, "{input}/{len}")` with
//!   `Throughput::Elements(len)` per cell — criterion full ids read
//!   `dot/fearless_avx2/normal/1024` (criterion sanitizes `/` inside each id
//!   component when naming output directories).
//! - **stateful kernels**: `biquad` / `sine` use `iter_batched` so every
//!   iteration starts from a fresh state — a deterministic non-denormal
//!   state for `normal` (otherwise the filter output decays into the
//!   denormal/zero region and pollutes the baseline), and a re-injected
//!   denormal state for `denormal` so the cliff stays observable. `dst` is
//!   allocated once per cell and reused (the writes are idempotent).
//! - **time budget**: reduced `warm_up_time` / `measurement_time` /
//!   `sample_size` on every group so the full matrix fits the ≈20–30 min
//!   `PoC` budget instead of criterion's default >1 h.
//! - **filter**: `cargo bench -p ruviform-math -- <filter>` (e.g. `dot`,
//!   `1024`) selects matching benchmarks — the quick-run convention.
//!
//! Run-start environment metadata (OS / CPU model / core count / rustc /
//! criterion / detected dispatch levels / skipped columns) is printed to
//! stdout exactly once, for transcription into the experiment log.

#[allow(dead_code)]
#[path = "../tests/simd_poc/mod.rs"]
mod simd_poc_support;

use std::hint::black_box;
use std::sync::Once;
use std::time::Duration;

use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use fearless_simd::{Level, dispatch};
use ruviform_math::simd_poc::{
    BIQUAD_CHANNELS, BIQUAD_COEFFS, SineCoeffs, SineState, denormal_f32, fir_taps,
};
use simd_poc_support::{KernelImpl, inputs, matrix_cells, registry};

/// Bench size series — the practical band plus the lane-boundary ragged
/// lengths (for `biquad` these are frames per channel at `channels = 8`).
const LENS: &[usize] = &[64, 256, 1024, 4096, 100, 1000, 1023];

/// Elementwise kernels measure the `normal` input set only.
const ELEMENTWISE_INPUTS: &[&str] = &["normal"];

/// Accumulating kernels additionally measure `denormal` (cliff observation).
const CUMULATIVE_INPUTS: &[&str] = &["normal", "denormal"];

/// Registry columns that stay in the equivalence matrix but are excluded
/// from benches: `fearless_sse2` / `pulp_v2` duplicate the `x86_64` `SSE2`
/// baseline (design methodology — `fearless_sse4_2` is the v2 representative).
const EQUIV_ONLY: &[&str] = &["fearless_sse2", "pulp_v2"];

/// Deterministic input seeds (same convention as the equivalence tests).
const SEED_A: u64 = 11;
/// `fir` source-stream seed.
const SEED_FIR: u64 = 19;
/// `biquad` source-stream seed.
const SEED_BIQUAD: u64 = 23;
/// `biquad` non-denormal initial-state seed.
const SEED_BIQUAD_INIT: u64 = 29;

/// `fade`'s fixed gain-ramp parameters (deterministic; matches the
/// equivalence smoke test).
const FADE_G0: f32 = 0.25;
const FADE_STEP: f32 = 0.01;
/// `scale`'s fixed gain.
const SCALE_GAIN: f32 = -0.5;
/// `sine`'s normalized angular frequency (undamped oscillator).
const SINE_OMEGA: f32 = 0.31;

/// Creates the per-kernel group with the reduced `PoC` time budget:
/// ≈1.3 s timed per case plus analysis overhead — the full matrix (~800
/// cases) lands inside the ≈20–30 min budget instead of criterion's
/// default >1 h.
fn poc_group<'a>(c: &'a mut Criterion, kernel: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(kernel);
    group
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1))
        .sample_size(20);
    group
}

/// Registers one `bench_function` for the `(column, input, len)` cell:
/// `Throughput::Elements(len)` and `BenchmarkId::new(col.name,
/// "{input}/{len}")`. The per-kernel iteration body is deferred to `f`.
fn bench_cell<F>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    col: &KernelImpl,
    input: &str,
    len: usize,
    f: F,
) where
    F: FnMut(&mut criterion::Bencher<'_>),
{
    group.throughput(Throughput::Elements(len as u64));
    group.bench_function(BenchmarkId::new(col.name, format!("{input}/{len}")), f);
}

/// Runs `register` on every measured `(column × input × len)` cell in
/// registry order. Equivalence-only columns and columns whose forced level
/// is unconstructible (`probe() == false`) are skipped — the skipped set is
/// reported once by [`print_env_metadata`].
fn for_each_cell<F>(input_kinds: &[&'static str], mut register: F)
where
    F: FnMut(&KernelImpl, &'static str, usize),
{
    for cell in matrix_cells(input_kinds, LENS, &[0]) {
        let col = cell.column;
        if EQUIV_ONLY.contains(&col.name) || !(col.probe)() {
            continue;
        }
        register(&col, cell.input, cell.len);
    }
}

/// `denormal` initial filter state — alternating `±denormal` values so the
/// recursive kernels re-enter the denormal range on every iteration.
fn denormal_state(len: usize) -> Vec<f32> {
    let d = denormal_f32();
    (0..len)
        .map(|i| if i.is_multiple_of(2) { d } else { -d })
        .collect()
}

/// Prints the run-start environment block to stdout exactly once; the
/// experiment log transcribes it.
fn print_env_metadata() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        println!("=== simd_poc bench environment ===");
        println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        println!("cpu: {}", cpu_model());
        println!("cores: {}", core_count());
        println!("rustc: {}", rustc_version());
        println!("criterion: {}", criterion_version());
        println!("fearless_simd Level::new(): {:?}", Level::new());
        println!("pulp::Arch::new(): {:?}", pulp::Arch::new());
        print_forced_levels();
        let skipped: Vec<&str> = registry()
            .iter()
            .filter(|c| !(c.probe)())
            .map(|c| c.name)
            .collect();
        println!("skipped columns (level unconstructible): {skipped:?}");
        println!("equivalence-only columns (not benched): {EQUIV_ONLY:?}");
    });
}

/// Forced-level constructibility report — x86 tier tokens.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn print_forced_levels() {
    let level = Level::new();
    println!(
        "fearless_simd forced levels: sse2={} sse4_2={} avx2={} avx512={}",
        level.as_sse2().is_some(),
        level.as_sse4_2().is_some(),
        level.as_avx2().is_some(),
        level.as_avx512().is_some()
    );
    println!(
        "pulp forced levels: v2={} v3={} v4={}",
        pulp::x86::V2::try_new().is_some(),
        pulp::x86::V3::try_new().is_some(),
        pulp::x86::V4::try_new().is_some()
    );
}

/// Forced-level constructibility report — aarch64 NEON token.
#[cfg(target_arch = "aarch64")]
fn print_forced_levels() {
    println!(
        "fearless_simd forced levels: neon={}",
        Level::new().as_neon().is_some()
    );
}

/// Forced-level constructibility report — no tier tokens exist elsewhere.
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
fn print_forced_levels() {}

/// CPU model name for the environment block (brand string when available).
#[cfg(target_os = "windows")]
fn cpu_model() -> String {
    // `HKLM\...\CentralProcessor\0\ProcessorNameString` holds the brand
    // string ("Intel(R) Core(TM) …"); `reg` is a system tool on Windows.
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
    // Fallback: family/model/stepping, not the marketing name.
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string())
}

/// CPU model name for the environment block — `/proc/cpuinfo` `model name`.
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

/// CPU model name for the environment block — `sysctl` brand string.
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

/// CPU model name fallback on other targets.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_model() -> String {
    "unknown".to_string()
}

/// Logical core count for the environment block.
fn core_count() -> String {
    std::thread::available_parallelism()
        .map_or_else(|_| "unknown".to_string(), |n| n.get().to_string())
}

/// `rustc --version` of the toolchain on `PATH` (the `mise`-pinned one when
/// run via `mise exec` / `mise run`).
fn rustc_version() -> String {
    std::process::Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |out| String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
}

/// criterion version actually linked — read from the workspace `Cargo.lock`
/// (the reproducibility anchor) rather than the `Cargo.toml` requirement.
fn criterion_version() -> String {
    let lock = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let Ok(text) = std::fs::read_to_string(lock) else {
        return "unknown".to_string();
    };
    let mut found = false;
    for line in text.lines().map(str::trim) {
        if found && let Some(rest) = line.strip_prefix("version = ") {
            return rest.trim_matches('"').to_string();
        }
        found = line == "name = \"criterion\"";
    }
    "unknown".to_string()
}

/// `dst[i] = a[i] + b[i]` — elementwise, `normal` only.
fn bench_add(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "add");
    for_each_cell(ELEMENTWISE_INPUTS, |col, input, len| {
        let a = inputs::generate(input, len, 0, SEED_A);
        let b = inputs::generate(input, len, 0, inputs::SEED_B);
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| {
                (col.add)(black_box(&mut dst), black_box(&a), black_box(&b));
            });
        });
    });
    group.finish();
}

/// `dst[i] = src[i] * gain` — elementwise, `normal` only.
fn bench_scale(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "scale");
    for_each_cell(ELEMENTWISE_INPUTS, |col, input, len| {
        let src = inputs::generate(input, len, 0, SEED_A);
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| {
                (col.scale)(black_box(&mut dst), black_box(&src), black_box(SCALE_GAIN));
            });
        });
    });
    group.finish();
}

/// `dst[i] = clamp(src[i], -1, 1)` — elementwise, `normal` only.
fn bench_clamp_unit(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "clamp_unit");
    for_each_cell(ELEMENTWISE_INPUTS, |col, input, len| {
        let src = inputs::generate(input, len, 0, SEED_A);
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| {
                (col.clamp_unit)(black_box(&mut dst), black_box(&src));
            });
        });
    });
    group.finish();
}

/// `dst[i] = src[i] * (g0 + i·step)` — elementwise, `normal` only.
fn bench_fade(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "fade");
    for_each_cell(ELEMENTWISE_INPUTS, |col, input, len| {
        let src = inputs::generate(input, len, 0, SEED_A);
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| {
                (col.fade)(
                    black_box(&mut dst),
                    black_box(&src),
                    black_box(FADE_G0),
                    black_box(FADE_STEP),
                );
            });
        });
    });
    group.finish();
}

/// `Σ a[i]·b[i]` — reduction, `normal` + `denormal`.
fn bench_dot(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "dot");
    for_each_cell(CUMULATIVE_INPUTS, |col, input, len| {
        let a = inputs::generate(input, len, 0, SEED_A);
        let b = inputs::generate(input, len, 0, inputs::SEED_B);
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| (col.dot)(black_box(&a), black_box(&b)));
        });
    });
    group.finish();
}

/// 64-tap direct-form FIR — reduction, `normal` + `denormal`.
fn bench_fir(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "fir");
    let taps = fir_taps();
    for_each_cell(CUMULATIVE_INPUTS, |col, input, len| {
        let src = inputs::generate(input, len, 0, SEED_FIR);
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter(|| {
                (col.fir)(black_box(&mut dst), black_box(&src), black_box(&taps[..]));
            });
        });
    });
    group.finish();
}

/// Planar 8-channel DF-II transposed biquad — stateful, `normal` +
/// `denormal`. `len` is frames per channel; `src`/`dst` hold
/// `channels × len` elements.
fn bench_biquad(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "biquad");
    for_each_cell(CUMULATIVE_INPUTS, |col, input, len| {
        let src = inputs::generate(input, BIQUAD_CHANNELS * len, 0, SEED_BIQUAD);
        let init_state = match input {
            "normal" => inputs::normal(2 * BIQUAD_CHANNELS, SEED_BIQUAD_INIT),
            "denormal" => denormal_state(2 * BIQUAD_CHANNELS),
            _ => panic!("unknown input kind {input}"),
        };
        let mut dst = vec![0.0; src.len()];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter_batched(
                || init_state.clone(),
                |mut state| {
                    (col.biquad)(
                        black_box(&mut dst),
                        black_box(&src),
                        black_box(&BIQUAD_COEFFS),
                        black_box(&mut state),
                        BIQUAD_CHANNELS,
                    );
                },
                BatchSize::SmallInput,
            );
        });
    });
    group.finish();
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` recursive oscillator — stateful,
/// `normal` + `denormal`. The `denormal` variant re-seeds the recurrence
/// with denormal `y1`/`y2` each iteration so the cliff stays observable.
fn bench_sine(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "sine");
    let coeffs = SineCoeffs::for_omega(SINE_OMEGA);
    for_each_cell(CUMULATIVE_INPUTS, |col, input, len| {
        let init_state = match input {
            "normal" => SineState { y1: 0.5, y2: -0.25 },
            "denormal" => SineState {
                y1: denormal_f32(),
                y2: -denormal_f32(),
            },
            _ => panic!("unknown input kind {input}"),
        };
        let mut dst = vec![0.0; len];
        bench_cell(&mut group, col, input, len, |bench| {
            bench.iter_batched(
                || init_state,
                |mut state| {
                    (col.sine)(
                        black_box(&mut dst),
                        black_box(&coeffs),
                        black_box(&mut state),
                    );
                },
                BatchSize::SmallInput,
            );
        });
    });
    group.finish();
}

/// 固定レベル列は arch 依存: x86 は avx2、aarch64 は neon。
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const PROBE_LEVEL_COL: &str = "fearless_avx2";
#[cfg(target_arch = "aarch64")]
const PROBE_LEVEL_COL: &str = "fearless_neon";
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
const PROBE_LEVEL_COL: &str = "fearless_fallback";

/// Issue #56 — direct per-call dispatch-cost probe. The `fearless_dispatched`
/// regression on light kernels decomposes into a *fixed* per-call cost
/// (LazyLock-cached `Level::new()` + `__dispatch_target` normalization +
/// dispatch arm) added to the kernel's own `t(N)`. This group measures the
/// pieces directly and adds a small-size `add` band (16/32) where the fixed
/// cost dominates, so the N-fit in the analysis can be cross-checked against
/// the probe value.
fn bench_dispatch_probe(c: &mut Criterion) {
    print_env_metadata();
    let mut group = poc_group(c, "dispatch_probe");

    // Component probes — fixed per-call costs in isolation.
    group.bench_function("level_new", |b| {
        b.iter(|| black_box(Level::new()));
    });
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    group.bench_function("token_avx2", |b| {
        b.iter(|| black_box(Level::new().as_avx2()));
    });
    #[cfg(target_arch = "aarch64")]
    group.bench_function("token_neon", |b| {
        b.iter(|| black_box(Level::new().as_neon()));
    });
    group.bench_function("dispatch_noop", |b| {
        b.iter(|| {
            // Each `dispatch!` arm yields a distinct opaque `impl Simd`
            // token type — borrow + `()` so the arms unify. The token is
            // still materialized and consumed through `black_box`, so the
            // dispatch path itself is measured.
            dispatch!(Level::new(), simd => { black_box(&simd); });
        });
    });

    // Small-size `add` band: at N=16/32 the kernel body is a handful of
    // lanes so the fixed dispatch cost dominates the ratio — the clearest
    // window onto the per-call component.
    let cols = registry();
    for probe_col in ["scalar", PROBE_LEVEL_COL, "fearless_dispatched"] {
        let Some(col) = cols.iter().find(|c| c.name == probe_col) else {
            continue;
        };
        for len in [16_usize, 32] {
            let a = inputs::generate("normal", len, 0, SEED_A);
            let b = inputs::generate("normal", len, 0, inputs::SEED_B);
            let mut dst = vec![0.0; len];
            group.throughput(Throughput::Elements(len as u64));
            group.bench_function(
                BenchmarkId::new(format!("add_{probe_col}"), len.to_string()),
                |bench| {
                    bench.iter(|| {
                        (col.add)(black_box(&mut dst), black_box(&a), black_box(&b));
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_add,
    bench_scale,
    bench_clamp_unit,
    bench_fade,
    bench_dot,
    bench_fir,
    bench_biquad,
    bench_sine,
    bench_dispatch_probe
);
criterion_main!(benches);
