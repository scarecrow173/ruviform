//! `multiversion` 0.9 candidate kernels (task T3) — autovectorization column.
//!
//! `#[multiversion]` clones each body per listed target and installs a runtime
//! dispatcher (`x86_64+avx2+fma` first, then `x86_64+sse4.2`, `aarch64+neon`;
//! the plain body is the fallback clone). Bodies are the scalar reference's
//! own loops — this column measures **compiler autovectorization under the
//! clone's target features**, not hand-written intrinsics.
//!
//! Structural limitation recorded for the matrix/ADR: the generated clones are
//! private and `multiversion` exposes no per-clone forcing API, so the
//! registry has a single `"dispatched"` column — the environment-selected
//! clone is the only level this backend can be measured at.

use multiversion::multiversion;
use ruviform_math::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

use super::KernelImpl;

/// `dst[i] = a[i] + b[i]`.
#[multiversion(targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"))]
pub(crate) fn add(dst: &mut [f32], a: &[f32], b: &[f32]) {
    debug_assert_eq!(dst.len(), a.len());
    debug_assert_eq!(a.len(), b.len());
    for ((d, &x), &y) in dst.iter_mut().zip(a.iter()).zip(b.iter()) {
        *d = x + y;
    }
}

/// `dst[i] = src[i] * gain`.
#[multiversion(targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"))]
pub(crate) fn scale(dst: &mut [f32], src: &[f32], gain: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, &x) in dst.iter_mut().zip(src.iter()) {
        *d = x * gain;
    }
}

/// `dst[i] = clamp(src[i], -1, 1)`; NaN propagates.
#[multiversion(targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"))]
pub(crate) fn clamp_unit(dst: &mut [f32], src: &[f32]) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, &x) in dst.iter_mut().zip(src.iter()) {
        *d = x.clamp(-1.0, 1.0);
    }
}

/// `dst[i] = src[i] * (g0 + i·step)` — mul-then-add kept (no FMA; contract).
#[multiversion(
    targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"),
    attrs(
        expect(
            clippy::cast_precision_loss,
            reason = "`i as f32` fits exactly for bench-scale lengths"
        ),
        expect(
            clippy::suboptimal_flops,
            reason = "`g0 + i*step` must stay separate mul/add — FMA is forbidden by the contract"
        )
    )
)]
pub(crate) fn fade(dst: &mut [f32], src: &[f32], g0: f32, step: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (i, (d, &x)) in dst.iter_mut().zip(src.iter()).enumerate() {
        *d = x * (g0 + i as f32 * step);
    }
}

/// `Σ a[i]·b[i]` — finite inputs only.
#[multiversion(
    targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"),
    attrs(allow(
        clippy::suboptimal_flops,
        reason = "sequential multiply-then-add accumulation is the reference rounding; \
              allow not expect — undetected by MSRV clippy 1.89"
    ))
)]
pub(crate) fn dot(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut acc = 0.0_f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        acc += x * y;
    }
    acc
}

/// Direct-form FIR; `taps.len() == FIR_TAPS`; zero history.
#[multiversion(
    targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"),
    attrs(allow(
        clippy::suboptimal_flops,
        reason = "sequential multiply-then-add accumulation is the reference rounding; \
              allow not expect — undetected by MSRV clippy 1.89"
    ))
)]
pub(crate) fn fir(dst: &mut [f32], src: &[f32], taps: &[f32]) {
    debug_assert!(dst.len() <= src.len());
    for (n, d) in dst.iter_mut().enumerate() {
        let mut acc = 0.0_f32;
        for (&t, &x) in taps.iter().zip(src[..=n].iter().rev()) {
            acc += t * x;
        }
        *d = acc;
    }
}

/// Planar `channels`-channel DF-II transposed biquad; `SoA` `state`.
#[multiversion(
    targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"),
    attrs(expect(
        clippy::suboptimal_flops,
        reason = "separate multiply/add/sub ops are the reference rounding"
    ))
)]
pub(crate) fn biquad(
    dst: &mut [f32],
    src: &[f32],
    c: &BiquadCoeffs,
    state: &mut [f32],
    channels: usize,
) {
    debug_assert_eq!(state.len(), 2 * channels);
    debug_assert_eq!(src.len() % channels, 0);
    debug_assert_eq!(dst.len(), src.len());
    let frames = src.len() / channels;
    let (z1s, z2s) = state.split_at_mut(channels);
    for ch in 0..channels {
        let mut z1 = z1s[ch];
        let mut z2 = z2s[ch];
        for f in 0..frames {
            let x = src[ch * frames + f];
            let y = c.b0 * x + z1;
            z1 = c.b1 * x - c.a1 * y + z2;
            z2 = c.b2 * x - c.a2 * y;
            dst[ch * frames + f] = y;
        }
        z1s[ch] = z1;
        z2s[ch] = z2;
    }
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` recursive oscillator.
#[multiversion(
    targets("x86_64+avx2+fma", "x86_64+sse4.2", "aarch64+neon"),
    attrs(expect(
        clippy::suboptimal_flops,
        reason = "sequential multiply-then-add recurrence is the reference rounding"
    ))
)]
pub(crate) fn sine(dst: &mut [f32], c: &SineCoeffs, state: &mut SineState) {
    let mut y1 = state.y1;
    let mut y2 = state.y2;
    for d in dst.iter_mut() {
        let y = c.c1 * y1 + c.c2 * y2;
        y2 = y1;
        y1 = y;
        *d = y;
    }
    state.y1 = y1;
    state.y2 = y2;
}

/// Registry column — dispatched only (see the file header for the per-clone
/// forcing limitation, recorded for the equivalence matrix).
pub(crate) const COLUMN: KernelImpl = KernelImpl {
    name: "multiversion",
    backend: "multiversion",
    level: "dispatched",
    probe: || true,
    add,
    scale,
    clamp_unit,
    fade,
    dot,
    fir,
    biquad,
    sine,
};
