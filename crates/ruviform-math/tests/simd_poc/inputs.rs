//! Boundary-input generation for the SIMD `PoC` matrix (task T3).
//!
//! Three input kinds per the equivalence-matrix contract:
//! - `"normal"` — uniform pseudo-random `f32` in `[-1, 1)` (finite).
//! - `"edge"` — uniform values with every 3rd element replaced by a boundary
//!   special: denormals, `±0.0`, `NaN`, `±Inf`, `±f32::MAX`, `±MIN_POSITIVE`.
//! - `"denormal"` — uniform values with denormals mixed in every
//!   [`DENORMAL_PERIOD`]-th element (denormal-cost coverage).
//!
//! All generators are deterministic (`Xorshift64Star`); the caller passes a
//! fixed seed per stream (`a`/`b`/`src` use distinct seeds).

use ruviform_math::simd_poc::{denormal_f32, denormal_mix, uniform_f32};

/// Kernel-matrix lengths stressing tail paths (odd, `lanes ± 1`, odd multiples).
pub(crate) const LEN_SHORT: &[usize] = &[7, 15, 16, 17, 31, 33];

/// Kernel-matrix lengths covering multi-block bodies (incl. `1024`/`4096` —
/// the main bench sizes).
pub(crate) const LEN_LONG: &[usize] = &[100, 255, 1023, 1024, 4096];

/// Every matrix length (`LEN_SHORT` then `LEN_LONG`).
pub(crate) const LEN_ALL: &[usize] = &[7, 15, 16, 17, 31, 33, 100, 255, 1023, 1024, 4096];

/// Head offsets into the backing buffer (`0`/`1`/`3` cover unaligned starts).
pub(crate) const OFFSETS: &[usize] = &[0, 1, 3];

/// Input kinds in matrix order.
pub(crate) const INPUT_KINDS: &[&str] = &["normal", "edge", "denormal"];

/// Denormal-injection period for the `"denormal"` input set.
pub(crate) const DENORMAL_PERIOD: usize = 8;

/// Secondary deterministic seed for the `b` input stream (`a` uses `seed`).
pub(crate) const SEED_B: u64 = 0x5DEE_CE66_D5D3_9EA5;

/// Uniform input in `[-1, 1)` — the `"normal"` set.
#[must_use]
pub(crate) fn normal(len: usize, seed: u64) -> Vec<f32> {
    uniform_f32(len, seed)
}

/// Edge-value mix — the `"edge"` set: uniform values with every 3rd element
/// replaced by a boundary special, cycling so every special occurs within the
/// first 33 elements (the smallest matrix length still sees each one).
#[must_use]
pub(crate) fn edge(len: usize, seed: u64) -> Vec<f32> {
    let mut v = uniform_f32(len, seed);
    let d = denormal_f32();
    let specials = [
        d,
        -d,
        0.0,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for (i, x) in v.iter_mut().enumerate() {
        if i % 3 == 0 {
            *x = specials[(i / 3) % specials.len()];
        }
    }
    v
}

/// Denormal-mixed uniform input — the `"denormal"` set.
#[must_use]
pub(crate) fn denormal(len: usize, seed: u64) -> Vec<f32> {
    denormal_mix(len, seed, DENORMAL_PERIOD)
}

/// Allocates `len + offset` elements of input `kind`; callers slice
/// `&buf[offset..offset + len]` so `offset` exercises unaligned heads.
///
/// # Panics
///
/// Unknown `kind` — callers contract to [`INPUT_KINDS`] values.
#[must_use]
pub(crate) fn generate(kind: &str, len: usize, offset: usize, seed: u64) -> Vec<f32> {
    let total = len + offset;
    match kind {
        "normal" => normal(total, seed),
        "edge" => edge(total, seed),
        "denormal" => denormal(total, seed),
        _ => panic!("unknown input kind {kind}"),
    }
}
