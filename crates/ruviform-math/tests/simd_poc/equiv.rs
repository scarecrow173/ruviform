//! Equivalence-contract helpers for the SIMD `PoC` matrix (task T3).
//!
//! The per-kernel contract (design baseline: bit-exact or ulp-bounded vs the
//! scalar reference) is centralized in [`contract_for`] so the T4 matrix tests
//! and the failure reporter share one policy:
//! - bit-exact (`to_bits`): `add`, `scale`, `clamp_unit`, `fade`, `sine`.
//! - ulp-bounded: `dot` ([`MAX_ULPS_DOT`]), `fir` ([`MAX_ULPS_FIR`]), `biquad`
//!   ([`MAX_ULPS_BIQUAD`]) — accumulation-order differences are allowed; NaN
//!   lanes still must match `is_nan`-wise.
//!
//! [`max_ulps`] aggregates the observed max ulp across a compared pair — the
//! matrix tests print it per ulp-bounded kernel as adoption-ADR evidence.
//!
//! NaN handling is uniform across both contracts: a pair counts as equal iff
//! `is_nan` agrees on both sides (payload bits are ignored — backends may
//! produce a different NaN payload than the scalar `f32::clamp`).

use approx::UlpsEq;

/// Ulp bound for `dot` — **revised** from the design-doc initial value of 8
/// after the T4 matrix investigation: lane-split reduction (`n` partial
/// accumulators + horizontal sum) plus FMA reorder the rounding *relative to
/// the result magnitude*, so result-cancellation cells amplify a legitimate
/// few-ulp-of-intermediate-scale difference into tens of ulps of the result
/// (observed max over the matrix: 46 ulps — verified against an f64 ground
/// truth: impl and scalar sit within ~1e-7 absolute of the true sum, i.e.
/// contract misfit, not impl bug). 256 keeps ≈5× margin over the observed
/// max and headroom for lane widths not probed on this machine (NEON 4-lane,
/// AVX-512 16-lane); the adoption ADR finalizes the bound from the recorded
/// observed-max evidence. Sequential (`pulp_scalar`) and wide-lane impls
/// share this same bound.
pub(crate) const MAX_ULPS_DOT: u32 = 256;

/// Ulp bound for `fir` — the spec's initial value, unchanged: the candidate
/// impls keep the scalar reference's per-output tap order with *unfused*
/// multiply-add (FMA skipped for the same reason as `biquad` — a fused step
/// re-rounds and drift reached 3994 ulps under cancellation, verified vs f64
/// truth), so lane-split outputs stay bit-exact and this bound is met with
/// margin (observed 0).
pub(crate) const MAX_ULPS_FIR: u32 = 8;

/// Initial ulp bound for `biquad` — equal to the reduction kernels' spec
/// starting value: the design doc sets `ulp ≤ 8`（初期値）for `biquad` too,
/// with FMA rounding differences allowed. Because the recursive state can
/// compound rounding differences, a *legitimate* measured excess may switch
/// to the design doc's alternate contract form (first-N-frames ulp +
/// relative-error / energy bound, or a frame-length-dependent bound) —
/// never a post-hoc relaxation. Kept as a separate constant so the bound
/// stays independently revisable per kernel.
pub(crate) const MAX_ULPS_BIQUAD: u32 = 8;

/// Per-kernel equivalence contract.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Contract {
    /// `f32::to_bits` equality per element; NaN pairs match via `is_nan`.
    BitExact,
    /// `approx::UlpsEq` with the given `max_ulps`; NaN pairs match via `is_nan`.
    Ulp(u32),
}

/// The equivalence contract for kernel `name`.
///
/// # Panics
///
/// Unknown kernel name — callers contract to the kernel list (`"add"`,
/// `"scale"`, `"clamp_unit"`, `"fade"`, `"dot"`, `"fir"`, `"biquad"`, `"sine"`).
#[must_use]
pub(crate) fn contract_for(name: &str) -> Contract {
    match name {
        "add" | "scale" | "clamp_unit" | "fade" | "sine" => Contract::BitExact,
        "dot" => Contract::Ulp(MAX_ULPS_DOT),
        "fir" => Contract::Ulp(MAX_ULPS_FIR),
        "biquad" => Contract::Ulp(MAX_ULPS_BIQUAD),
        _ => panic!("unknown kernel name {name}"),
    }
}

/// Ordered bit distance ("ulps") between `a` and `b`; `u64::MAX` when either
/// side is NaN (NaN pairs are judged by `is_nan`, never by distance).
#[must_use]
pub(crate) fn ulps_between(a: f32, b: f32) -> u64 {
    if a.is_nan() || b.is_nan() {
        return u64::MAX;
    }
    // Map the sign-magnitude bit pattern to a monotone integer so `u64` order
    // equals ordered-`f32` distance (±0.0 map to the same integer).
    #[expect(
        clippy::cast_possible_wrap,
        reason = "reinterpreting the f32 sign-magnitude bit pattern is intended"
    )]
    let ord = |x: f32| {
        let i = i64::from(x.to_bits() as i32);
        if i < 0 { i64::from(i32::MIN) - i } else { i }
    };
    (ord(a) - ord(b)).unsigned_abs()
}

/// Max ordered-`f32` ulp distance across element pairs, skipping pairs where
/// either side is NaN (NaN agreement is judged by [`element_ok`], never by
/// distance). Used to aggregate the observed max ulp of each ulp-bounded
/// kernel over the whole matrix as adoption-ADR evidence.
///
/// # Panics
///
/// Length mismatch — a harness bug, not a candidate failure.
#[must_use]
pub(crate) fn max_ulps(expected: &[f32], actual: &[f32]) -> u64 {
    assert_eq!(expected.len(), actual.len());
    expected
        .iter()
        .zip(actual)
        .filter(|&(&e, &a)| !(e.is_nan() || a.is_nan()))
        .map(|(&e, &a)| ulps_between(e, a))
        .max()
        .unwrap_or(0)
}

/// One element failing the equivalence contract.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mismatch {
    /// Element index inside the compared slices.
    pub index: usize,
    /// Scalar-reference value.
    pub expected: f32,
    /// Candidate output.
    pub actual: f32,
    /// Measured ulp distance (`u64::MAX` on a NaN-agreement failure).
    pub ulps: u64,
}

/// Element-level contract check: NaN pairs require `is_nan` agreement on both
/// sides; otherwise `to_bits` equality or `UlpsEq` within `max_ulps`.
#[must_use]
pub(crate) fn element_ok(expected: f32, actual: f32, contract: Contract) -> bool {
    if expected.is_nan() || actual.is_nan() {
        return expected.is_nan() == actual.is_nan();
    }
    match contract {
        Contract::BitExact => expected.to_bits() == actual.to_bits(),
        Contract::Ulp(max) => expected.ulps_eq(&actual, 0.0, max),
    }
}

/// First contract mismatch between `expected` and `actual`, or `None`.
///
/// # Panics
///
/// Length mismatch — a harness bug, not a candidate failure.
#[must_use]
pub(crate) fn first_mismatch(
    expected: &[f32],
    actual: &[f32],
    contract: Contract,
) -> Option<Mismatch> {
    assert_eq!(expected.len(), actual.len());
    expected
        .iter()
        .zip(actual)
        .enumerate()
        .find_map(|(index, (&e, &a))| {
            if element_ok(e, a, contract) {
                None
            } else {
                Some(Mismatch {
                    index,
                    expected: e,
                    actual: a,
                    ulps: ulps_between(e, a),
                })
            }
        })
}

/// Formats a mismatch as the documented report tuple `(kernel, impl, level,
/// input, len, offset, index, expected_bits, actual_bits, ulps)`.
#[must_use]
pub(crate) fn format_mismatch(
    kernel: &str,
    impl_name: &str,
    level: &str,
    input: &str,
    len: usize,
    offset: usize,
    m: &Mismatch,
) -> String {
    format!(
        "kernel={kernel} impl={impl_name} level={level} input={input} \
         len={len} offset={offset} index={} expected_bits={:#010x} \
         actual_bits={:#010x} ulps={}",
        m.index,
        m.expected.to_bits(),
        m.actual.to_bits(),
        m.ulps
    )
}
