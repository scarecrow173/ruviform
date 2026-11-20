//! Frozen DSP regression bench harness (Issue #807 — E-layer).
//!
//! Measures the production `ruviform_math::dsp` dispatch layer as an
//! in-process A/B pair: side A = `SimdToken::force_fallback()` (scalar
//! path), side B = `SimdToken::detect()` (the token production builds
//! once per block boundary). The gate metric is the per-cell ratio
//! `median(A ns) / median(B ns)` of per-call nanoseconds — measured in
//! one process on one machine, so it is machine-independent. The pinned
//! baseline holds a `min_ratio` threshold per `kernel/len` cell, never
//! absolute times; absolute ns and environment metadata are emitted as
//! informational records only.
//!
//! Cells: `dot` and `fir` — consumer hot paths (ruviform-dsp `fir`,
//! audio-analysis spectrum) with no scalar-routing floor, so the A/B
//! contrast measures the token path itself rather than the small-N
//! scalar routing `add`/`scale`/`clamp_unit` have. Inputs come from the
//! deterministic `tests/simd_poc/inputs.rs` generators (`#[path]`
//! include — the `benches/simd_poc.rs` technique); the scalar reference
//! is `ruviform_math::simd_poc` and the error bound is the shared
//! `tests/simd_poc/equiv.rs` contract (`dot` ulp ≤ 256, `fir` ulp ≤ 8).
//!
//! Modes (argv after `--`):
//!
//! - *(none)* — real run: [`REPS`] timed reps per side, A-B-A-B
//!   alternating, median + min–max range, error count vs the scalar
//!   reference, work count, limiter note and environment metadata — one
//!   JSON object per line on stdout — then the pinned-baseline gate.
//!   Exit `0` on gate pass, `1` on gate fail / harness error.
//! - `--test` — `bench-smoke` compatibility path: `1` rep per side,
//!   same output shape, gate evaluated report-only (`gate_mode:
//!   "report_only"`), exit `0` unless the harness itself errors or argv
//!   is malformed. `cargo bench -- --test` reaches every `[[bench]]`
//!   target, so this path is what CI's `bench` job runs.
//! - `--selftest` — injects a real regression: side B is timed with
//!   `SimdToken::force_fallback()` instead of `detect()`, simulating a
//!   dispatch layer that silently degrades to scalar. The gate must
//!   fail (exit `1`) — proves fail-closed behaviour without waiting for
//!   a real regression.
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
//! Positional argv is a cell filter (substring of `kernel/len`) — the
//! `cargo bench -- <filter>` convention; a filter matching no cell
//! yields `gate: "not_run"`, exit `0`.
//!
//! Exit codes: `0` gate pass / smoke / no cells selected, `1` gate
//! fail or harness error, `2` CLI misuse.

#[allow(dead_code)]
#[path = "../tests/simd_poc/equiv.rs"]
mod equiv;
#[allow(dead_code)]
#[path = "../tests/simd_poc/inputs.rs"]
mod inputs;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use ruviform_math::dsp::{self, SimdToken};
use ruviform_math::simd_poc;

/// Bench cells: `KERNELS × LENS` on the practical band (tail sizes are
/// equivalence-test territory; the frozen gate tracks the common cases).
const KERNELS: &[&str] = &["dot", "fir"];
/// Cell lengths — all above every `*_SCALAR_BELOW` routing floor, and
/// both kernels route unconditionally anyway.
const LENS: &[usize] = &[256, 1024, 4096];

/// Timed reps per side in a real run — `>= 5` per the A/B contract.
const REPS: usize = 7;
/// Reps per side in `--test` smoke mode.
const SMOKE_REPS: usize = 1;

/// Target wall time for one timed rep: long enough to dwarf the
/// `Instant` resolution (~100 ns on Windows QPC), short enough to keep
/// a full run in seconds.
const TARGET_REP_NS: u128 = 300_000;
/// Upper bound for the adaptive per-rep call count.
const MAX_INNER: usize = 100_000;

/// Deterministic `a`-stream seed (same convention as the `PoC` bench).
const SEED_A: u64 = 11;
/// `fir` source-stream seed (same convention as the `PoC` bench).
const SEED_FIR: u64 = 19;

/// Pinned baseline schema id and fixture path — read-only gate input.
const BASELINE_SCHEMA: &str = "rvf.dsp-frozen-baseline/v1";
/// Pinned baseline fixture, resolved against the crate at compile time.
const PINNED_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/benches/frozen/dsp_baseline.json"
);
/// Issue reference recorded in baseline files.
const BASELINE_ISSUE: &str = "https://github.com/scarecrow173/ruviform/issues/807";

/// Arches where `detect()` must never resolve to `fallback` — SSE2 is
/// the `x86_64` baseline and NEON is mandatory on `AArch64`, so a
/// `fallback` level here means the dispatch layer itself regressed.
const SIMD_EXPECTED: bool = cfg!(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64"
));

/// Per-cell limiter note — why the metric stops where it does
/// (benchmark-checklist Q1). Recorded next to the metric so each JSONL
/// row is self-describing; the identification procedure lives in
/// `docs/test-specs/issue-807-frozen-dsp-bench.md`.
fn limiter(kernel: &str, len: usize) -> String {
    let set_kib = len * 4 / 1024;
    match kernel {
        // dot: FMA-free reference order, SIMD lanes split the
        // accumulator — throughput-bound on mul+add, not dispatch.
        "dot" => format!(
            "compute-bound: fp mul+add throughput on ~{set_kib} KiB resident streams; \
             dispatch cost amortized (no scalar-routing floor on this kernel)"
        ),
        // fir: O(len*taps) MAC chain, taps L1-resident at 256 B.
        "fir" => format!(
            "compute-bound: len*64-tap MAC loop on ~{set_kib} KiB src (taps L1-resident); \
             zero-history head is a fixed cost"
        ),
        _ => "unknown kernel".to_string(),
    }
}

/// One bench cell — inputs plus the working buffer for one
/// `kernel/len` pair.
enum Cell {
    Dot {
        len: usize,
        a: Vec<f32>,
        b: Vec<f32>,
    },
    Fir {
        len: usize,
        src: Vec<f32>,
        taps: Vec<f32>,
        dst: Vec<f32>,
    },
}

impl Cell {
    /// Builds deterministic inputs for `kernel/len` via the shared `PoC`
    /// generators (`"normal"` class — boundary classes are
    /// equivalence-test inputs, not bench inputs).
    fn new(kernel: &str, len: usize) -> Self {
        match kernel {
            "dot" => Self::Dot {
                len,
                a: inputs::generate("normal", len, 0, SEED_A),
                b: inputs::generate("normal", len, 0, inputs::SEED_B),
            },
            "fir" => Self::Fir {
                len,
                src: inputs::generate("normal", len, 0, SEED_FIR),
                taps: simd_poc::fir_taps().to_vec(),
                dst: vec![0.0; len],
            },
            _ => unreachable!("KERNELS is a closed set"),
        }
    }

    /// `"<kernel>/<len>"` — the baseline cell key.
    fn key(&self) -> String {
        format!("{}/{}", self.kernel(), self.len())
    }

    /// Kernel name.
    fn kernel(&self) -> &'static str {
        match self {
            Self::Dot { .. } => "dot",
            Self::Fir { .. } => "fir",
        }
    }

    /// Element count (also the elements-per-call work unit).
    fn len(&self) -> usize {
        match self {
            Self::Dot { len, .. } | Self::Fir { len, .. } => *len,
        }
    }

    /// One timed kernel call under `tok`. `black_box` keeps the work
    /// inside the timed region (benchmark-checklist Q7).
    fn call(&mut self, tok: SimdToken) {
        match self {
            Self::Dot { a, b, .. } => {
                black_box(dsp::dot(tok, black_box(&a[..]), black_box(&b[..])));
            }
            Self::Fir { src, taps, dst, .. } => {
                dsp::fir(
                    tok,
                    black_box(&mut dst[..]),
                    black_box(&src[..]),
                    black_box(&taps[..]),
                );
                // Keep the written output observable so the calls are
                // not optimized away.
                black_box(&dst[..]);
            }
        }
    }

    /// Output-mismatch count vs the `simd_poc` scalar reference under
    /// the shared equivalence contract (benchmark-checklist Q4 — the
    /// timed path must also be *correct*). Deterministic inputs make
    /// one check per side per cell sufficient.
    fn errors(&mut self, tok: SimdToken) -> usize {
        match self {
            Self::Dot { a, b, .. } => {
                let expected = simd_poc::dot(&a[..], &b[..]);
                let actual = dsp::dot(tok, &a[..], &b[..]);
                usize::from(!equiv::element_ok(
                    expected,
                    actual,
                    equiv::contract_for("dot"),
                ))
            }
            Self::Fir {
                len,
                src,
                taps,
                dst,
            } => {
                let mut expected = vec![0.0_f32; *len];
                simd_poc::fir(&mut expected, &src[..], &taps[..]);
                dsp::fir(tok, dst, &src[..], &taps[..]);
                expected
                    .iter()
                    .zip(dst.iter())
                    .filter(|&(&e, &a)| !equiv::element_ok(e, a, equiv::contract_for("fir")))
                    .count()
            }
        }
    }
}

/// Aggregated per-side timing for one cell.
struct SideStats {
    /// Token level label (`"fallback"` / `"avx2"` / …).
    level: &'static str,
    /// Calls per timed rep.
    inner: usize,
    /// Reps measured.
    reps: usize,
    /// Per-call nanoseconds: median / min / max over reps.
    median_ns: f64,
    min_ns: f64,
    max_ns: f64,
}

/// Elapsed nanos for `inner` calls of `cell.call(tok)`.
fn time_rep(cell: &mut Cell, tok: SimdToken, inner: usize) -> u128 {
    let t0 = Instant::now();
    for _ in 0..inner {
        cell.call(tok);
    }
    t0.elapsed().as_nanos()
}

/// Picks `inner` so one rep lands near [`TARGET_REP_NS`]: doubles the
/// call count until the rep is long enough. Calibration runs also act
/// as warmup — the alternating A-B-A-B rep order then keeps residual
/// drift from favouring either side.
fn calibrate(cell: &mut Cell, tok: SimdToken) -> usize {
    let mut inner = 1_usize;
    loop {
        let ns = time_rep(cell, tok, inner);
        if ns >= TARGET_REP_NS || inner >= MAX_INNER {
            return inner;
        }
        inner = inner.saturating_mul(4).min(MAX_INNER);
    }
}

/// Median / min / max of per-call nanoseconds (`rep_ns / inner`).
#[expect(
    clippy::cast_precision_loss,
    reason = "rep nanoseconds and inner counts are far below f64 precision limits"
)]
fn summarize(samples: &[u128], inner: usize) -> (f64, f64, f64) {
    let mut ns: Vec<f64> = samples.iter().map(|&t| t as f64 / inner as f64).collect();
    ns.sort_by(f64::total_cmp);
    (ns[ns.len() / 2], ns[0], ns[ns.len() - 1])
}

/// Measures both sides of one cell: `reps` A-B-A-B alternating timed
/// reps, then the per-side error count.
fn measure(cell: &mut Cell, a_tok: SimdToken, b_tok: SimdToken, reps: usize) -> CellReport {
    let inner_a = calibrate(cell, a_tok);
    let inner_b = calibrate(cell, b_tok);
    let mut a_times = Vec::with_capacity(reps);
    let mut b_times = Vec::with_capacity(reps);
    for _ in 0..reps {
        a_times.push(time_rep(cell, a_tok, inner_a));
        b_times.push(time_rep(cell, b_tok, inner_b));
    }
    let (a_med, a_min, a_max) = summarize(&a_times, inner_a);
    let (b_med, b_min, b_max) = summarize(&b_times, inner_b);
    let a = SideStats {
        level: a_tok.level(),
        inner: inner_a,
        reps,
        median_ns: a_med,
        min_ns: a_min,
        max_ns: a_max,
    };
    let b = SideStats {
        level: b_tok.level(),
        inner: inner_b,
        reps,
        median_ns: b_med,
        min_ns: b_min,
        max_ns: b_max,
    };
    let errors_a = cell.errors(a_tok);
    let errors_b = cell.errors(b_tok);
    CellReport {
        key: cell.key(),
        kernel: cell.kernel(),
        len: cell.len(),
        a,
        b,
        ratio: a_med / b_med,
        errors_a,
        errors_b,
        work_count: u64::try_from(cell.len() * (inner_a + inner_b) * reps).unwrap_or(u64::MAX),
        limiter: limiter(cell.kernel(), cell.len()),
        min_ratio: None,
        verdict: "not_run",
        reason: None,
    }
}

/// One cell's measurement + gate outcome (one JSONL row).
struct CellReport {
    key: String,
    kernel: &'static str,
    len: usize,
    a: SideStats,
    b: SideStats,
    /// `median(A)/median(B)` — B's speedup over scalar (> 1 = faster).
    ratio: f64,
    errors_a: usize,
    errors_b: usize,
    /// Elements processed: `len * (inner_a + inner_b) * reps`.
    work_count: u64,
    limiter: String,
    min_ratio: Option<f64>,
    verdict: &'static str,
    reason: Option<String>,
}

/// CLI mode after parsing.
enum Mode {
    /// Real run + gate.
    Run,
    /// `bench-smoke` compatibility: 1 rep, gate report-only.
    Smoke,
    /// Injected regression: B side forced to `force_fallback()`.
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
    "dsp_frozen — frozen DSP regression bench (Issue #807)\n\
     usage: dsp_frozen [--test|--selftest|--write-baseline <path>] [--baseline <path>] \
     [filter]...\n\
     \x20 (no args)             real run: A/B alternating reps + pinned-baseline gate \
     (exit 1 on fail)\n\
     \x20 --test                smoke: 1 rep per side, gate report-only, exit 0\n\
     \x20 --selftest            inject B=force_fallback regression; gate must fail (exit 1)\n\
     \x20 --write-baseline <p>  write a baseline candidate JSON to <p>, exit 0\n\
     \x20 --baseline <p>        gate against <p> instead of the pinned fixture\n\
     \x20 -h, --help            this text\n\
     \x20 filter               cell filter substring (kernel/len), cargo-bench convention\n\
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

// ---- tiny strict JSON reader for the pinned baseline -------------------
// serde is intentionally not a dev-dependency; the baseline schema is a
// small fixed shape, parsed by a recursive-descent reader that rejects
// anything malformed (a corrupt baseline fails the gate closed).

struct Json<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Json<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            b: s.as_bytes(),
            i: 0,
        }
    }

    fn ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.peek() == Some(c) {
            self.i += 1;
            return Ok(());
        }
        Err(format!("expected '{}' at byte {}", char::from(c), self.i))
    }

    /// `"..."` with standard escapes (no surrogate-pair handling —
    /// baseline files never carry astral text).
    fn string(&mut self) -> Result<String, String> {
        self.ws();
        if self.peek() != Some(b'"') {
            return Err(format!("expected string at byte {}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(c) = self.peek() {
                if c == b'"' || c == b'\\' {
                    break;
                }
                self.i += 1;
            }
            if self.i > start {
                let chunk = std::str::from_utf8(&self.b[start..self.i])
                    .map_err(|e| format!("utf8 in string: {e}"))?;
                out.push_str(chunk);
            }
            match self.peek() {
                None => return Err("unterminated string".to_string()),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let e = self.peek().ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let hex = self
                                .b
                                .get(self.i..self.i + 4)
                                .ok_or("truncated \\u escape")?;
                            let hex = std::str::from_utf8(hex).map_err(|e| e.to_string())?;
                            let cp = u32::from_str_radix(hex, 16)
                                .map_err(|e| format!("bad \\u escape: {e}"))?;
                            self.i += 4;
                            out.push(char::from_u32(cp).ok_or("surrogate \\u escape")?);
                        }
                        _ => return Err(format!("bad escape \\{}", char::from(e))),
                    }
                }
                Some(_) => unreachable!(),
            }
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        self.ws();
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.b[start..self.i])
            .map_err(|e| e.to_string())?
            .parse::<f64>()
            .map_err(|e| format!("bad number at byte {start}: {e}"))
    }

    /// Skip any JSON value (string-aware so braces inside strings do
    /// not miscount).
    fn value(&mut self) -> Result<(), String> {
        self.ws();
        match self.peek() {
            Some(b'"') => {
                self.string()?;
            }
            Some(b'{') => {
                self.i += 1;
                loop {
                    self.ws();
                    match self.peek() {
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(());
                        }
                        Some(b'"') => {}
                        _ => return Err(format!("expected object key at byte {}", self.i)),
                    }
                    self.string()?;
                    self.expect(b':')?;
                    self.value()?;
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("expected ',' or '}}' at byte {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                loop {
                    self.ws();
                    match self.peek() {
                        Some(b']') => {
                            self.i += 1;
                            return Ok(());
                        }
                        Some(_) => self.value()?,
                        None => return Err("unterminated array".to_string()),
                    }
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
                    }
                }
            }
            Some(_) => {
                while let Some(c) = self.peek() {
                    if matches!(c, b',' | b'}' | b']') || c.is_ascii_whitespace() {
                        break;
                    }
                    self.i += 1;
                }
            }
            None => return Err("unexpected end of input".to_string()),
        }
        Ok(())
    }
}

/// Parses the pinned baseline: `schema` must match, `approval` must be
/// an object (a `null` approval is an un-promoted candidate — refuse),
/// and `cells` maps `"<kernel>/<len>"` to `{"min_ratio": <f64>}`.
fn parse_baseline(text: &str) -> Result<BTreeMap<String, f64>, String> {
    let mut p = Json::new(text);
    p.expect(b'{')?;
    let mut schema_ok = false;
    let mut approved = false;
    let mut cells = BTreeMap::new();
    loop {
        p.ws();
        match p.peek() {
            Some(b'}') => break,
            Some(b'"') => {}
            _ => return Err(format!("expected object key at byte {}", p.i)),
        }
        let key = p.string()?;
        p.expect(b':')?;
        match key.as_str() {
            "schema" => schema_ok = p.string()? == BASELINE_SCHEMA,
            "cells" => cells = parse_cells(&mut p)?,
            "approval" => {
                p.ws();
                if p.b[p.i..].starts_with(b"null") {
                    p.i += 4;
                } else {
                    approved = true;
                    p.value()?;
                }
            }
            _ => p.value()?,
        }
        p.ws();
        match p.peek() {
            Some(b',') => p.i += 1,
            Some(b'}') => break,
            _ => return Err(format!("expected ',' or '}}' at byte {}", p.i)),
        }
    }
    if !schema_ok {
        return Err(format!("missing or wrong schema (want {BASELINE_SCHEMA})"));
    }
    if !approved {
        return Err(
            "baseline has no approval — an un-promoted candidate is not a gate input \
             (promotion is manual: copy the candidate file and fill `approval`)"
                .to_string(),
        );
    }
    Ok(cells)
}

/// `"cells": {"<key>": {"min_ratio": <f64>, …}, …}` — extra keys inside
/// a cell object are skipped.
fn parse_cells(p: &mut Json<'_>) -> Result<BTreeMap<String, f64>, String> {
    let mut cells = BTreeMap::new();
    p.expect(b'{')?;
    loop {
        p.ws();
        match p.peek() {
            Some(b'}') => {
                p.i += 1;
                return Ok(cells);
            }
            Some(b'"') => {}
            _ => return Err(format!("expected cell key at byte {}", p.i)),
        }
        let cell = p.string()?;
        p.expect(b':')?;
        p.expect(b'{')?;
        let mut min_ratio = None;
        loop {
            p.ws();
            match p.peek() {
                Some(b'}') => {
                    p.i += 1;
                    break;
                }
                Some(b'"') => {}
                _ => return Err(format!("expected cell field at byte {}", p.i)),
            }
            let field = p.string()?;
            p.expect(b':')?;
            if field == "min_ratio" {
                min_ratio = Some(p.number()?);
            } else {
                p.value()?;
            }
            p.ws();
            match p.peek() {
                Some(b',') => p.i += 1,
                Some(b'}') => {
                    p.i += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", p.i)),
            }
        }
        let Some(r) = min_ratio else {
            return Err(format!("cell {cell} has no min_ratio"));
        };
        cells.insert(cell, r);
        p.ws();
        match p.peek() {
            Some(b',') => p.i += 1,
            Some(b'}') => {
                p.i += 1;
                return Ok(cells);
            }
            _ => return Err(format!("expected ',' or '}}' at byte {}", p.i)),
        }
    }
}

// ---- JSONL output ------------------------------------------------------

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

/// `Option<f64>` as a JSON number (2 decimals) or `null`.
fn num_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "null".to_string(), |x| format!("{x:.2}"))
}

/// Environment helpers — same reporting the `PoC` bench prints for its
/// experiment log, kept local because `benches/simd_poc.rs` is a
/// separate bench target (not a shared module).
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
fn emit_run(
    mode: &str,
    b_desc: &str,
    b_tok: SimdToken,
    reps: usize,
    baseline: Option<&Path>,
    injected: bool,
) {
    let injected_json = if injected {
        concat!(
            "{\"kind\":\"b_forced_fallback\",\"detail\":\"B side timed with ",
            "SimdToken::force_fallback() — simulates dispatch degraded to scalar\"}"
        )
    } else {
        "null"
    };
    println!(
        "{{\"type\":\"run\",\"schema\":\"rvf.dsp-frozen.run/v1\",\"lane\":\"bench-frozen\",\
         \"tool\":\"dsp_frozen\",\"mode\":{},\"os\":{},\"arch\":{},\"cpu\":{},\"cores\":{},\
         \"rustc\":{},\"repo_sha\":{},\"a\":\"SimdToken::force_fallback()\",\"b\":{},\
         \"b_level\":{},\"reps\":{},\"baseline\":{},\"injected\":{}}}",
        js(mode),
        js(std::env::consts::OS),
        js(std::env::consts::ARCH),
        js(&cpu_model()),
        core_count(),
        js(&rustc_version()),
        js(&repo_sha()),
        js(b_desc),
        js(b_tok.level()),
        reps,
        baseline.map_or_else(|| "null".to_string(), |p| js(&p.display().to_string())),
        injected_json,
    );
}

fn emit_cell(r: &CellReport) {
    println!(
        "{{\"type\":\"cell\",\"key\":{},\"kernel\":{},\"len\":{},\
         \"a\":{{\"level\":{},\"inner\":{},\"reps\":{},\"median_ns\":{:.1},\"min_ns\":{:.1},\
         \"max_ns\":{:.1}}},\
         \"b\":{{\"level\":{},\"inner\":{},\"reps\":{},\"median_ns\":{:.1},\"min_ns\":{:.1},\
         \"max_ns\":{:.1}}},\
         \"ratio\":{:.3},\"min_ratio\":{},\"errors_a\":{},\"errors_b\":{},\
         \"work_count\":{},\"limiter\":{},\"verdict\":{},\"reason\":{}}}",
        js(&r.key),
        js(r.kernel),
        r.len,
        js(r.a.level),
        r.a.inner,
        r.a.reps,
        r.a.median_ns,
        r.a.min_ns,
        r.a.max_ns,
        js(r.b.level),
        r.b.inner,
        r.b.reps,
        r.b.median_ns,
        r.b.min_ns,
        r.b.max_ns,
        r.ratio,
        num_opt(r.min_ratio),
        r.errors_a,
        r.errors_b,
        r.work_count,
        js(&r.limiter),
        js(r.verdict),
        js_opt(r.reason.as_deref()),
    );
}

// ---- gate --------------------------------------------------------------

/// Per-cell gate evaluation: returns `(verdict, reason, min_ratio)`.
/// `b_is_detect` distinguishes a real `detect()` side from the
/// `--selftest` injection (which must reach the threshold check to fail
/// for real).
fn judge(
    baseline: &Result<BTreeMap<String, f64>, String>,
    r: &CellReport,
    b_tok: SimdToken,
    b_is_detect: bool,
) -> (&'static str, Option<String>, Option<f64>) {
    if r.errors_a + r.errors_b > 0 {
        return ("fail", Some("output_mismatch".to_string()), None);
    }
    if b_is_detect && b_tok.level() == "fallback" {
        // A real `detect()` that fell back: broken dispatch on SIMD
        // arches, legitimately ungatable elsewhere.
        return if SIMD_EXPECTED {
            ("fail", Some("detect_returned_fallback".to_string()), None)
        } else {
            ("unsupported", Some("no_simd_level".to_string()), None)
        };
    }
    let Ok(map) = baseline else {
        return ("not_run", Some("baseline_unreadable".to_string()), None);
    };
    let Some(&min_ratio) = map.get(&r.key) else {
        return ("unsupported", Some("no_pinned_threshold".to_string()), None);
    };
    if r.ratio >= min_ratio {
        ("pass", None, Some(min_ratio))
    } else {
        (
            "fail",
            Some(format!(
                "ratio_below_threshold ({:.3} < {min_ratio:.2})",
                r.ratio
            )),
            Some(min_ratio),
        )
    }
}

/// Writes the baseline candidate JSON (tmp + rename, same atomic-write
/// convention as `BaselineStore`). `"approval": null` keeps the file an
/// un-promoted candidate — the gate refuses it until a human fills
/// `approval` and copies it over the pinned fixture.
fn write_candidate(path: &Path, results: &[CellReport]) -> std::io::Result<()> {
    let mut s = String::new();
    s.push_str("{\n");
    let _ = writeln!(s, "  \"schema\": {},", js(BASELINE_SCHEMA));
    let _ = writeln!(s, "  \"issue\": {},", js(BASELINE_ISSUE));
    s.push_str("  \"metric\": \"in_process_ab_ratio\",\n");
    s.push_str("  \"a\": \"SimdToken::force_fallback()\",\n");
    s.push_str("  \"b\": \"SimdToken::detect()\",\n");
    s.push_str(
        "  \"threshold_rule\": \"min_ratio = floor(1 + (measured_ratio - 1) * 0.5, 0.1) — \
         fails when the SIMD advantage halves vs the pin (dispatch degraded to \
         scalar -> ratio ~1.0), with enough slack for machine noise\",\n",
    );
    s.push_str("  \"measured_env\": ");
    let _ = writeln!(
        s,
        "{{\"os\": {}, \"arch\": {}, \"cpu\": {}, \"cores\": {}, \"rustc\": {}, \
         \"repo_sha\": {}, \"detected_level\": {}}},",
        js(std::env::consts::OS),
        js(std::env::consts::ARCH),
        js(&cpu_model()),
        core_count(),
        js(&rustc_version()),
        js(&repo_sha()),
        js(SimdToken::detect().level()),
    );
    s.push_str("  \"cells\": {\n");
    for (i, r) in results.iter().enumerate() {
        let sep = if i + 1 == results.len() { "" } else { "," };
        let min_ratio = ((1.0 + (r.ratio - 1.0) * 0.5) * 10.0).floor() / 10.0;
        let _ = writeln!(
            s,
            "    {}: {{\"min_ratio\": {min_ratio:.1}, \"measured_ratio\": {:.3}, \
             \"measured_a_median_ns\": {:.1}, \"measured_b_median_ns\": {:.1}, \
             \"errors\": {}, \"work_count\": {}}}{sep}",
            js(&r.key),
            r.ratio,
            r.a.median_ns,
            r.b.median_ns,
            r.errors_a + r.errors_b,
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
    baseline: &Result<BTreeMap<String, f64>, String>,
    a_tok: SimdToken,
    b_tok: SimdToken,
    b_is_detect: bool,
    gating: bool,
    reps: usize,
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
    for &kernel in KERNELS {
        for &len in LENS {
            let key = format!("{kernel}/{len}");
            if !args.filters.is_empty() && !args.filters.iter().any(|f| key.contains(f.as_str())) {
                continue;
            }
            t.cells += 1;
            let mut cell = Cell::new(kernel, len);
            let mut r = measure(&mut cell, a_tok, b_tok, reps);
            t.errors += r.errors_a + r.errors_b;
            if gating {
                let (verdict, reason, min_ratio) = judge(baseline, &r, b_tok, b_is_detect);
                r.verdict = verdict;
                r.reason = reason;
                r.min_ratio = min_ratio;
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
    let b_is_detect = !matches!(args.mode, Mode::Selftest);
    let b_tok = if b_is_detect {
        SimdToken::detect()
    } else {
        SimdToken::force_fallback()
    };
    let a_tok = SimdToken::force_fallback();
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
    let b_desc = if b_is_detect {
        "SimdToken::detect()"
    } else {
        "SimdToken::force_fallback()"
    };

    let baseline: Result<BTreeMap<String, f64>, String> = if gating {
        std::fs::read_to_string(&args.baseline)
            .map_err(|e| format!("{}: {e}", args.baseline.display()))
            .and_then(|t| parse_baseline(&t))
    } else {
        Ok(BTreeMap::new())
    };

    emit_run(
        mode_name,
        b_desc,
        b_tok,
        reps,
        gating.then_some(args.baseline.as_path()),
        matches!(args.mode, Mode::Selftest),
    );

    let (tally, results) = measure_cells(args, &baseline, a_tok, b_tok, b_is_detect, gating, reps);
    let Tally {
        cells,
        passed,
        failed,
        unsupported,
        not_run,
        errors: errors_total,
    } = tally;

    if let Mode::WriteBaseline(path) = &args.mode {
        return emit_candidate_result(path, cells, write_candidate(path, &results));
    }

    // Gate line: `fail` wins over `pass`; a baseline that cannot be read
    // or was never promoted fails the gate closed (except in smoke,
    // where nothing hard-fails).
    let gate_mode = if report_only {
        "report_only"
    } else {
        "fail_closed"
    };
    let verdict = if !gating || cells == 0 || (baseline.is_err() && report_only) {
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
