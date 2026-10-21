//! SIMD equivalence-matrix test target (Issue #19, task T3).
//!
//! Currently hosts the T3 smoke suite: every constructible registry column
//! runs all eight kernels once against the scalar reference on small
//! deterministic inputs. The full `(kernel × column × input × len × offset)`
//! equivalence matrix — including per-kernel exclusion cells (`dot`'s
//! finite-input contract) and the documented report format — is task T4.

mod simd_poc;

use ruviform_math::simd_poc::{BIQUAD_CHANNELS, BIQUAD_COEFFS, SineCoeffs, SineState, fir_taps};
use simd_poc::{KernelImpl, equiv, inputs, matrix_cells, registry};

/// Smoke-case length: `> FIR_TAPS` so the FIR vector body runs; `33`-class
/// lengths are covered by the offset matrix in T4.
const LEN: usize = 100;
const OFFSET: usize = 1;

/// One comparison cell's `(input, len, offset)` context, reported verbatim.
#[derive(Clone, Copy)]
struct Case {
    input: &'static str,
    len: usize,
    offset: usize,
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
    // Length-family sanity (T4's full matrix consumes these constants).
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

#[test]
fn columns_match_scalar_on_normal_input() {
    let mut failures = Vec::new();
    let mut skipped = Vec::new();
    for col in registry() {
        if !(col.probe)() {
            skipped.push(col.name);
            continue;
        }
        compare_elementwise(&col, &mut failures);
        compare_dot(&col, &mut failures);
        compare_fir(&col, &mut failures);
        compare_biquad(&col, &mut failures);
        compare_sine(&col, &mut failures);
    }
    eprintln!("skipped columns (level unavailable): {skipped:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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

/// `add` / `scale` / `clamp_unit` / `fade` on every input kind (the `edge`
/// set's NaN propagation is part of the contract here).
fn compare_elementwise(col: &KernelImpl, failures: &mut Vec<String>) {
    for &input in inputs::INPUT_KINDS {
        let case = Case {
            input,
            len: LEN,
            offset: OFFSET,
        };
        let a_buf = inputs::generate(input, LEN, OFFSET, 11);
        let b_buf = inputs::generate(input, LEN, OFFSET, inputs::SEED_B);
        let a = &a_buf[OFFSET..OFFSET + LEN];
        let b = &b_buf[OFFSET..OFFSET + LEN];
        let mut want = vec![0.0; LEN];
        let mut got = vec![0.0; LEN];

        ruviform_math::simd_poc::add(&mut want, a, b);
        (col.add)(&mut got, a, b);
        check(failures, "add", col, case, &want, &got);

        ruviform_math::simd_poc::scale(&mut want, a, -0.5);
        (col.scale)(&mut got, a, -0.5);
        check(failures, "scale", col, case, &want, &got);

        ruviform_math::simd_poc::clamp_unit(&mut want, a);
        (col.clamp_unit)(&mut got, a);
        check(failures, "clamp_unit", col, case, &want, &got);

        ruviform_math::simd_poc::fade(&mut want, a, 0.25, 0.01);
        (col.fade)(&mut got, a, 0.25, 0.01);
        check(failures, "fade", col, case, &want, &got);
    }
}

/// `dot` — finite-input contract: the `edge` cell is excluded per the
/// equivalence-matrix spec (NaN/Inf are not part of `dot`'s contract).
fn compare_dot(col: &KernelImpl, failures: &mut Vec<String>) {
    for input in ["normal", "denormal"] {
        let case = Case {
            input,
            len: LEN,
            offset: OFFSET,
        };
        let a_buf = inputs::generate(input, LEN, OFFSET, 11);
        let b_buf = inputs::generate(input, LEN, OFFSET, inputs::SEED_B);
        let a = &a_buf[OFFSET..OFFSET + LEN];
        let b = &b_buf[OFFSET..OFFSET + LEN];
        check(
            failures,
            "dot",
            col,
            case,
            &[ruviform_math::simd_poc::dot(a, b)],
            &[(col.dot)(a, b)],
        );
    }
}

/// `fir` on `normal` + `edge` (NaN/Inf windows must produce NaN agreement).
fn compare_fir(col: &KernelImpl, failures: &mut Vec<String>) {
    let taps = fir_taps();
    for input in ["normal", "edge"] {
        let case = Case {
            input,
            len: LEN,
            offset: OFFSET,
        };
        let src_buf = inputs::generate(input, LEN, OFFSET, 19);
        let src = &src_buf[OFFSET..OFFSET + LEN];
        let mut want = vec![0.0; LEN];
        let mut got = vec![0.0; LEN];
        ruviform_math::simd_poc::fir(&mut want, src, &taps);
        (col.fir)(&mut got, src, &taps);
        check(failures, "fir", col, case, &want, &got);
    }
}

/// `biquad` — output and carried state both compared (`edge` input exercises
/// NaN contamination agreement).
fn compare_biquad(col: &KernelImpl, failures: &mut Vec<String>) {
    let frames = LEN;
    for input in ["normal", "edge"] {
        let src = inputs::generate(input, BIQUAD_CHANNELS * frames, 0, 23);
        let init = inputs::normal(2 * BIQUAD_CHANNELS, 29);
        let mut want_state = init.clone();
        let mut got_state = init;
        let mut want = vec![0.0; src.len()];
        let mut got = vec![0.0; src.len()];
        ruviform_math::simd_poc::biquad(
            &mut want,
            &src,
            &BIQUAD_COEFFS,
            &mut want_state,
            BIQUAD_CHANNELS,
        );
        (col.biquad)(
            &mut got,
            &src,
            &BIQUAD_COEFFS,
            &mut got_state,
            BIQUAD_CHANNELS,
        );
        let out = Case {
            input,
            len: frames,
            offset: OFFSET,
        };
        check(failures, "biquad", col, out, &want, &got);
        let state = Case {
            input: "state",
            len: frames,
            offset: OFFSET,
        };
        check(failures, "biquad", col, state, &want_state, &got_state);
    }
}

/// `sine` — bit-exact contract; output and carried state both compared.
fn compare_sine(col: &KernelImpl, failures: &mut Vec<String>) {
    let c = SineCoeffs::for_omega(0.31);
    let init = SineState { y1: 0.5, y2: -0.25 };
    let mut want_state = init;
    let mut got_state = init;
    let mut want = vec![0.0; LEN];
    let mut got = vec![0.0; LEN];
    ruviform_math::simd_poc::sine(&mut want, &c, &mut want_state);
    (col.sine)(&mut got, &c, &mut got_state);
    let out = Case {
        input: "normal",
        len: LEN,
        offset: OFFSET,
    };
    check(failures, "sine", col, out, &want, &got);
    let state = Case {
        input: "state",
        len: LEN,
        offset: OFFSET,
    };
    check(
        failures,
        "sine",
        col,
        state,
        &[want_state.y1, want_state.y2],
        &[got_state.y1, got_state.y2],
    );
}
