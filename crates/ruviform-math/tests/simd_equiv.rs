//! SIMD equivalence-matrix test target (Issue #19, task T4).
//!
//! One `#[test]` per kernel aggregates that kernel's full
//! `(column × input × len × offset)` matrix against the scalar reference
//! under its equivalence contract ([`equiv::contract_for`]). All cell
//! mismatches are collected and reported by a single trailing assert; forced
//! dispatch levels that cannot be constructed on this machine
//! (`KernelImpl::probe() == false`) are recorded as skipped — never panicked
//! — so the matrix stays deterministic across environments. Each
//! ulp-bounded kernel additionally prints its observed max ulp for the
//! adoption-ADR evidence record.
//!
//! Canonical contracts: `docs/designs/dsp-simd-poc.md` (カーネルセットと等価性
//! 契約 / 等価性 matrix) and `docs/test-specs/simd-equivalence-matrix-contract.md`.

mod simd_poc;

use std::collections::BTreeSet;

use ruviform_math::simd_poc::{
    BIQUAD_CHANNELS, BIQUAD_COEFFS, SineCoeffs, SineState, denormal_f32, fir_taps,
};
use simd_poc::{Cell, KernelImpl, equiv, inputs, matrix_cells, registry};

/// One comparison cell's `(input, len, offset)` context, reported verbatim
/// into the failure tuple.
#[derive(Clone, Copy)]
struct Case {
    input: &'static str,
    len: usize,
    offset: usize,
}

impl Case {
    /// The output-comparison case of `cell` (for `biquad`, `len` is frames
    /// per channel — channels stay fixed at [`BIQUAD_CHANNELS`]).
    fn of(cell: &Cell) -> Self {
        Self {
            input: cell.input,
            len: cell.len,
            offset: cell.offset,
        }
    }

    /// The carried-state pseudo-case: `biquad` / `sine` compare the
    /// post-call state too, labelled `input: "state"` in the report tuple.
    fn state(self) -> Self {
        Self {
            input: "state",
            ..self
        }
    }
}

/// Probes `col`'s forced level once per cell; unconstructible levels are
/// recorded into `skipped` and the cell is bypassed — the matrix's
/// panic-free skip convention (an unavailable level is an environment fact,
/// recorded per test, not a failure).
fn probe_or_skip(col: &KernelImpl, skipped: &mut BTreeSet<&'static str>) -> bool {
    if (col.probe)() {
        true
    } else {
        skipped.insert(col.name);
        false
    }
}

/// Per-test evidence lines: the skipped-column set (levels not constructible
/// on this machine) and, for ulp-bounded kernels, the observed max ulp vs
/// the scalar reference next to its contract bound.
fn report(test: &str, skipped: &BTreeSet<&'static str>, ulp: Option<(u64, u32)>) {
    eprintln!("{test}: skipped columns (level unavailable): {skipped:?}");
    if let Some((observed, bound)) = ulp {
        eprintln!(
            "{test}: observed max ulp vs scalar reference: {observed} (contract bound ≤ {bound})"
        );
    }
}

/// Asserts `want`/`got` equality under `kernel`'s contract; records a
/// formatted mismatch into `failures`.
fn check(
    failures: &mut Vec<String>,
    kernel: &str,
    col: &KernelImpl,
    case: Case,
    want: &[f32],
    got: &[f32],
) {
    if let Some(m) = equiv::first_mismatch(want, got, equiv::contract_for(kernel)) {
        failures.push(equiv::format_mismatch(
            kernel,
            col.name,
            col.level,
            case.input,
            case.len,
            case.offset,
            &m,
        ));
    }
}

#[test]
fn registry_lists_expected_columns() {
    let cols = registry();
    assert!(
        cols.len() >= 6,
        "expected the backend columns, got {}",
        cols.len()
    );
    for name in [
        "scalar",
        "fearless_fallback",
        "fearless_dispatched",
        "pulp_scalar",
        "pulp_dispatched",
        "multiversion",
    ] {
        assert!(
            cols.iter().any(|c| c.name == name),
            "missing registry column {name}"
        );
    }
    assert!(
        cols.iter()
            .all(|c| !c.name.is_empty() && !c.level.is_empty())
    );
    assert!(
        cols.iter()
            .all(|c| matches!(c.backend, "scalar" | "fearless" | "pulp" | "multiversion"))
    );
    // Column naming: `{backend}_{level}` (scalar is `level = "reference"`).
    assert!(cols.iter().all(|c| c.name.starts_with(c.backend)));
}

#[test]
fn matrix_cells_cover_registry() {
    // Length-family sanity (the full matrix consumes these constants).
    assert!(
        inputs::LEN_ALL.starts_with(inputs::LEN_SHORT)
            && inputs::LEN_ALL.ends_with(inputs::LEN_LONG)
            && inputs::LEN_ALL.len() == inputs::LEN_SHORT.len() + inputs::LEN_LONG.len()
    );
    let cells = matrix_cells(inputs::INPUT_KINDS, inputs::LEN_SHORT, inputs::OFFSETS);
    assert_eq!(
        cells.len(),
        registry().len()
            * inputs::INPUT_KINDS.len()
            * inputs::LEN_SHORT.len()
            * inputs::OFFSETS.len()
    );
    let c = cells[0];
    assert_eq!(c.column.name, "scalar");
    assert_eq!(c.input, "normal");
    assert_eq!(c.len, inputs::LEN_SHORT[0]);
    assert_eq!(c.offset, 0);
}

/// `add` — bit-exact contract; every input kind applies (`edge`'s NaN/Inf
/// lanes propagate deterministically through IEEE add: `is_nan` agreement,
/// bit equality otherwise).
#[test]
fn matrix_add() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let a_buf = inputs::generate(cell.input, cell.len, cell.offset, 11);
        let b_buf = inputs::generate(cell.input, cell.len, cell.offset, inputs::SEED_B);
        let a = &a_buf[cell.offset..cell.offset + cell.len];
        let b = &b_buf[cell.offset..cell.offset + cell.len];
        // Every kernel-facing slice starts `offset` elements into its backing
        // `Vec`, covering unaligned stores as well as loads.
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::add(&mut want[cell.offset..], a, b);
        (col.add)(&mut got[cell.offset..], a, b);
        check(
            &mut failures,
            "add",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
    }
    report("matrix_add", &skipped, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `scale` — bit-exact contract; same input coverage as `add`.
#[test]
fn matrix_scale() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let src_buf = inputs::generate(cell.input, cell.len, cell.offset, 11);
        let src = &src_buf[cell.offset..cell.offset + cell.len];
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::scale(&mut want[cell.offset..], src, -0.5);
        (col.scale)(&mut got[cell.offset..], src, -0.5);
        check(
            &mut failures,
            "scale",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
    }
    report("matrix_scale", &skipped, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `clamp_unit` — bit-exact on non-NaN input; NaN input → NaN output
/// (`is_nan` agreement, payload ignored — the contract absorbs min/max NaN
/// semantics differences).
#[test]
fn matrix_clamp_unit() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let src_buf = inputs::generate(cell.input, cell.len, cell.offset, 11);
        let src = &src_buf[cell.offset..cell.offset + cell.len];
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::clamp_unit(&mut want[cell.offset..], src);
        (col.clamp_unit)(&mut got[cell.offset..], src);
        check(
            &mut failures,
            "clamp_unit",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
    }
    report("matrix_clamp_unit", &skipped, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `fade` — bit-exact contract: `dst[i] = src[i] * (g0 + i·step)` with the
/// gain kept a separate multiply-then-add on both sides (the contract
/// forbids FMA).
#[test]
fn matrix_fade() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let src_buf = inputs::generate(cell.input, cell.len, cell.offset, 11);
        let src = &src_buf[cell.offset..cell.offset + cell.len];
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::fade(&mut want[cell.offset..], src, 0.25, 0.01);
        (col.fade)(&mut got[cell.offset..], src, 0.25, 0.01);
        check(
            &mut failures,
            "fade",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
    }
    report("matrix_fade", &skipped, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `dot` — ulp ≤ [`equiv::MAX_ULPS_DOT`] contract (revised from the initial
/// 8 after T4 measurement — see the constant's doc). The `edge` input set is
/// *excluded*: `dot`'s kernel contract requires finite inputs — a different
/// reduction order can round an `Inf`/`NaN` intermediate differently, so the
/// NaN/Inf edge cells are out of contract (equivalence-matrix spec; see
/// `docs/test-specs/simd-equivalence-matrix-contract.md`).
#[test]
fn matrix_dot() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    let mut max_ulp = 0_u64;
    for cell in matrix_cells(&["normal", "denormal"], inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let a_buf = inputs::generate(cell.input, cell.len, cell.offset, 11);
        let b_buf = inputs::generate(cell.input, cell.len, cell.offset, inputs::SEED_B);
        let a = &a_buf[cell.offset..cell.offset + cell.len];
        let b = &b_buf[cell.offset..cell.offset + cell.len];
        let want = ruviform_math::simd_poc::dot(a, b);
        let got = (col.dot)(a, b);
        max_ulp = max_ulp.max(equiv::ulps_between(want, got));
        check(&mut failures, "dot", &col, case, &[want], &[got]);
    }
    report("matrix_dot", &skipped, Some((max_ulp, equiv::MAX_ULPS_DOT)));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `fir` — ulp ≤ [`equiv::MAX_ULPS_FIR`] contract; the candidate impls keep
/// the scalar reference's per-output tap order with unfused multiply-add, so
/// outputs are bit-exact on this matrix (the FMA variant's cancellation
/// drift was measured up to 3994 ulps — see `equiv.rs`). `edge` applies:
/// outputs whose 64-tap window contains NaN/Inf propagate it
/// order-independently, judged by `is_nan` agreement in [`equiv::element_ok`].
#[test]
fn matrix_fir() {
    let taps = fir_taps();
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    let mut max_ulp = 0_u64;
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let src_buf = inputs::generate(cell.input, cell.len, cell.offset, 19);
        let src = &src_buf[cell.offset..cell.offset + cell.len];
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::fir(&mut want[cell.offset..], src, &taps);
        (col.fir)(&mut got[cell.offset..], src, &taps);
        max_ulp = max_ulp.max(equiv::max_ulps(&want[cell.offset..], &got[cell.offset..]));
        check(
            &mut failures,
            "fir",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
    }
    report("matrix_fir", &skipped, Some((max_ulp, equiv::MAX_ULPS_FIR)));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `biquad` — ulp ≤ [`equiv::MAX_ULPS_BIQUAD`] contract (FMA rounding
/// differences allowed; a *legitimate* fixed-bound excess would switch to
/// the design doc's alternate contract form — none observed so far).
/// `channels = 8` fixed; the length axis is frames per channel. `edge`
/// applies: NaN/Inf input contaminates the recursive state and every later
/// output, so those lanes are judged by `is_nan` agreement; state is
/// re-initialized fresh per cell so contamination never leaks between cells.
/// Output *and* carried state are compared.
#[test]
fn matrix_biquad() {
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    let mut max_ulp = 0_u64;
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        // `len` is frames per channel; the planar buffer holds
        // `channels * frames` elements (`index` in a report is the flat
        // planar index).
        let elems = BIQUAD_CHANNELS * cell.len;
        let src_buf = inputs::generate(cell.input, elems, cell.offset, 23);
        let src = &src_buf[cell.offset..cell.offset + elems];
        let mut want_state = inputs::normal(2 * BIQUAD_CHANNELS + cell.offset, 29);
        let mut got_state = want_state.clone();
        let mut want = vec![0.0; elems + cell.offset];
        let mut got = vec![0.0; elems + cell.offset];
        ruviform_math::simd_poc::biquad(
            &mut want[cell.offset..],
            src,
            &BIQUAD_COEFFS,
            &mut want_state[cell.offset..],
            BIQUAD_CHANNELS,
        );
        (col.biquad)(
            &mut got[cell.offset..],
            src,
            &BIQUAD_COEFFS,
            &mut got_state[cell.offset..],
            BIQUAD_CHANNELS,
        );
        max_ulp = max_ulp
            .max(equiv::max_ulps(&want[cell.offset..], &got[cell.offset..]))
            .max(equiv::max_ulps(
                &want_state[cell.offset..],
                &got_state[cell.offset..],
            ));
        check(
            &mut failures,
            "biquad",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
        check(
            &mut failures,
            "biquad",
            &col,
            case.state(),
            &want_state[cell.offset..],
            &got_state[cell.offset..],
        );
    }
    report(
        "matrix_biquad",
        &skipped,
        Some((max_ulp, equiv::MAX_ULPS_BIQUAD)),
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `sine` — bit-exact contract (every backend runs the same sequential
/// recurrence through its dispatch path; a difference would be a
/// dispatch-layer codegen finding, recorded as such). The input-set axis
/// degenerates — `sine` consumes no input slice — so each input kind instead
/// selects the initial-state fixture ([`sine_state_for`]); `offset` still
/// applies to the `dst` head.
#[test]
fn matrix_sine() {
    let coeffs = SineCoeffs::for_omega(0.31);
    let mut failures = Vec::new();
    let mut skipped = BTreeSet::new();
    for cell in matrix_cells(inputs::INPUT_KINDS, inputs::LEN_ALL, inputs::OFFSETS) {
        let col = cell.column;
        if !probe_or_skip(&col, &mut skipped) {
            continue;
        }
        let case = Case::of(&cell);
        let mut want_state = sine_state_for(cell.input);
        let mut got_state = want_state;
        let mut want = vec![0.0; cell.len + cell.offset];
        let mut got = vec![0.0; cell.len + cell.offset];
        ruviform_math::simd_poc::sine(&mut want[cell.offset..], &coeffs, &mut want_state);
        (col.sine)(&mut got[cell.offset..], &coeffs, &mut got_state);
        check(
            &mut failures,
            "sine",
            &col,
            case,
            &want[cell.offset..],
            &got[cell.offset..],
        );
        check(
            &mut failures,
            "sine",
            &col,
            case.state(),
            &[want_state.y1, want_state.y2],
            &[got_state.y1, got_state.y2],
        );
    }
    report("matrix_sine", &skipped, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Initial `SineState` per input kind — `sine` takes no input slice, so the
/// input axis selects the state fixture: finite `normal`, subnormal
/// `denormal`, and NaN-contaminated `edge` (the NaN init propagates to every
/// output and is judged by `is_nan` agreement).
fn sine_state_for(input: &str) -> SineState {
    match input {
        "normal" => SineState { y1: 0.5, y2: -0.25 },
        "denormal" => SineState {
            y1: denormal_f32(),
            y2: -denormal_f32(),
        },
        "edge" => SineState {
            y1: f32::NAN,
            y2: 0.5,
        },
        _ => unreachable!("input kinds are inputs::INPUT_KINDS members"),
    }
}
