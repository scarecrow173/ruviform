//! Shared support for the Issue #19 DSP/SIMD `PoC` (task T3).
//!
//! Cargo only treats `tests/*.rs` and `tests/*/main.rs` as test targets, so
//! this directory is never a target of its own: `tests/simd_equiv.rs` includes
//! it with `mod simd_poc;` and `benches/simd_poc.rs` shares the same files via
//! `#[path = "../tests/simd_poc/mod.rs"]` (with `#[allow(dead_code)]` on the
//! `mod` item, covering helpers the bench does not call). All items are
//! `pub(crate)` so they resolve from each including target's crate root.
//!
//! Contents:
//! - [`KernelImpl`] / [`registry`] / [`Cell`] / [`matrix_cells`] — the
//!   implementation-column registry and the `(column × input × len × offset)`
//!   runner data shared by the equivalence matrix (T4) and benches (T5).
//! - `inputs` — `normal` / `edge` / `denormal` boundary-input generation.
//! - `equiv` — the per-kernel equivalence contract (bit-exact vs ulp-bounded)
//!   and the mismatch report format.
//! - `fearless_impl` / `pulp_impl` / `multiversion_impl` — the three candidate
//!   backends, each behind the same uniform call interface.
//!
//! `unsafe` is not used anywhere in this tree: forced dispatch levels are
//! constructed through `Option`-returning detection APIs only
//! (`fearless_simd::Fallback::new()`, `fearless_simd::Level::new().as_*()`,
//! `pulp::x86::V*::try_new()`); unconstructible columns are skipped via
//! [`KernelImpl::probe`], never panicked.

pub(crate) mod equiv;
pub(crate) mod fearless_impl;
pub(crate) mod inputs;
pub(crate) mod multiversion_impl;
pub(crate) mod prod_impl;
pub(crate) mod pulp_impl;

use ruviform_math::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

/// One registry column: a complete kernel set for one backend at one dispatch
/// level behind a uniform call interface.
///
/// `probe()` reports column availability: `false` means the forced level
/// cannot be constructed on this machine (e.g. AVX-512 on a plain x64 CPU) and
/// the caller must skip the column — the matrix records it as *skipped*, never
/// panics. Kernel fns are guaranteed not to panic when called anyway: forced-
/// level closures simply return without touching `dst` (`dot` returns `0.0`).
#[derive(Clone, Copy)]
pub(crate) struct KernelImpl {
    /// Column identifier used in reports and bench ids, e.g. `"fearless_avx2"`.
    pub name: &'static str,
    /// Backend family: `"scalar"` / `"fearless"` / `"pulp"` / `"multiversion"`.
    pub backend: &'static str,
    /// Dispatch-level label: `"reference"`, `"fallback"`, `"sse2"`, `"avx2"`,
    /// `"dispatched"`, `"scalar"`, `"v3"`, …
    pub level: &'static str,
    /// Whether the forced level is constructible on this machine (`false` →
    /// the column is skipped, per the equivalence-matrix skip convention).
    pub probe: fn() -> bool,
    /// `dst[i] = a[i] + b[i]`.
    pub add: fn(&mut [f32], &[f32], &[f32]),
    /// `dst[i] = src[i] * gain`.
    pub scale: fn(&mut [f32], &[f32], f32),
    /// `dst[i] = clamp(src[i], -1, 1)`; NaN propagates.
    pub clamp_unit: fn(&mut [f32], &[f32]),
    /// `dst[i] = src[i] * (g0 + i·step)` — no FMA (equivalence contract).
    pub fade: fn(&mut [f32], &[f32], f32, f32),
    /// `Σ a[i]·b[i]` — finite inputs only (kernel contract).
    pub dot: fn(&[f32], &[f32]) -> f32,
    /// Direct-form FIR; `taps.len() == FIR_TAPS`; zero history before `src[0]`.
    pub fir: fn(&mut [f32], &[f32], &[f32]),
    /// Planar `channels`-channel DF-II transposed biquad; `SoA` `state`
    /// (`len == 2·channels`, `[z1 × ch | z2 × ch]`).
    pub biquad: fn(&mut [f32], &[f32], &BiquadCoeffs, &mut [f32], usize),
    /// `y[n] = c1·y[n-1] + c2·y[n-2]` recursive oscillator.
    pub sine: fn(&mut [f32], &SineCoeffs, &mut SineState),
}

/// `probe` for unconditionally constructible columns.
fn probe_true() -> bool {
    true
}

/// `scalar` — the plain-`std` reference implementation from
/// `src/simd_poc.rs`; the comparison base for every other column.
pub(crate) const SCALAR: KernelImpl = KernelImpl {
    name: "scalar",
    backend: "scalar",
    level: "reference",
    probe: probe_true,
    add: ruviform_math::simd_poc::add,
    scale: ruviform_math::simd_poc::scale,
    clamp_unit: ruviform_math::simd_poc::clamp_unit,
    fade: ruviform_math::simd_poc::fade,
    dot: ruviform_math::simd_poc::dot,
    fir: ruviform_math::simd_poc::fir,
    biquad: ruviform_math::simd_poc::biquad,
    sine: ruviform_math::simd_poc::sine,
};

/// Every registry column in the design-doc matrix order.
///
/// The set is arch-dependent: x86-only forced levels exist only under
/// `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` and
/// `fearless_neon` only on aarch64 — callers must not assume a fixed length.
/// Columns whose `probe` fails remain listed; skip them when iterating.
#[must_use]
pub(crate) fn registry() -> Vec<KernelImpl> {
    let mut cols = vec![SCALAR, fearless_impl::FALLBACK];
    #[cfg(target_arch = "aarch64")]
    cols.push(fearless_impl::NEON);
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    cols.extend([
        fearless_impl::SSE2,
        fearless_impl::SSE4_2,
        fearless_impl::AVX2,
        fearless_impl::AVX512,
    ]);
    cols.push(fearless_impl::DISPATCHED);
    cols.push(pulp_impl::SCALAR);
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    cols.extend([pulp_impl::V2, pulp_impl::V3, pulp_impl::V4]);
    cols.extend([pulp_impl::DISPATCHED, multiversion_impl::COLUMN]);
    // Issue #53 production dispatch layer — the routed public API at the
    // detected level plus each forced level the machine supports.
    cols.extend([prod_impl::DETECTED, prod_impl::FALLBACK]);
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    cols.extend([prod_impl::SSE2, prod_impl::SSE4_2, prod_impl::AVX2]);
    #[cfg(target_arch = "aarch64")]
    cols.push(prod_impl::NEON);
    cols
}

/// One `(column × input × len × offset)` cell of the equivalence/bench matrix.
#[derive(Clone, Copy)]
pub(crate) struct Cell {
    /// The backend column under test.
    pub column: KernelImpl,
    /// Input-set name (`"normal"` / `"edge"` / `"denormal"` — see `inputs`).
    pub input: &'static str,
    /// Logical element count (for `biquad`: frames per channel).
    pub len: usize,
    /// Head offset into the backing buffer (unaligned-start coverage).
    pub offset: usize,
}

/// Builds the full `(column × input × len × offset)` matrix in registry order.
///
/// Unavailable columns are *kept* so callers can report the skipped set rather
/// than silently shrinking the matrix — check `cell.column.probe` per cell.
#[must_use]
pub(crate) fn matrix_cells(
    inputs: &[&'static str],
    lens: &[usize],
    offsets: &[usize],
) -> Vec<Cell> {
    let cols = registry();
    let mut cells = Vec::with_capacity(cols.len() * inputs.len() * lens.len() * offsets.len());
    for &column in &cols {
        for &input in inputs {
            for &len in lens {
                for &offset in offsets {
                    cells.push(Cell {
                        column,
                        input,
                        len,
                        offset,
                    });
                }
            }
        }
    }
    cells
}
