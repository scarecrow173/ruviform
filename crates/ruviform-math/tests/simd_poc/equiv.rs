//! Equivalence-contract helpers for the SIMD `PoC` matrix (task T3).
//!
//! The per-kernel contract (design baseline: bit-exact or ulp-bounded vs the
//! scalar reference) is centralized in [`contract_for`] so the T4 matrix tests
//! and the failure reporter share one policy:
//! - bit-exact (`to_bits`): `add`, `scale`, `clamp_unit`, `fade`, `sine`.
//! - ulp ≤ [`MAX_ULPS`]: `dot`, `fir`, `biquad` — accumulation-order
//!   differences are allowed; NaN lanes still must match `is_nan`-wise.
//!
//! NaN handling is uniform across both contracts: a pair counts as equal iff
//! `is_nan` agrees on both sides (payload bits are ignored — backends may
//! produce a different NaN payload than the scalar `f32::clamp`).

use approx::UlpsEq;

/// Initial ulp bound for the reduction kernels (`dot`, `fir`).
pub(crate) const MAX_ULPS: u32 = 8;

/// Initial ulp bound for `biquad` — higher than the reduction kernels because
/// the recursive state update compounds rounding differences (the contract's
/// starting threshold; see the equivalence-matrix spec).
pub(crate) const MAX_ULPS_BIQUAD: u32 = 16;

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
        "dot" | "fir" => Contract::Ulp(MAX_ULPS),
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
