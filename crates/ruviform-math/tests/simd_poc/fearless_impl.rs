//! `fearless_simd` 1.1 candidate kernels + dispatch-level columns (task T3).
//!
//! Each `#[simd]` fn is the portable kernel — instantiated per level by the
//! registry columns below. Forced columns pass `Level::new().as_*()` tokens
//! (`Option`-returning detection; no `unsafe`, no `assume_supported`);
//! `fearless_fallback` uses `Fallback::new()` (enabled by the
//! `force_support_fallback` dev-dependency feature) and `fearless_dispatched`
//! routes through `dispatch!(Level::new(), …)` — the production-shaped path.
//! x86-only columns are `#[cfg]`-gated; `fearless_neon` exists only on
//! aarch64.
//!
//! Equivalence notes:
//! - `clamp_unit` is compare + select, matching `f32::clamp` order
//!   (`x < lo → lo`, `x > hi → hi`, else `x`): comparisons are false for NaN
//!   so NaN lanes propagate with their payload, which `min_precise` /
//!   `max_precise` would *drop* by selecting the non-NaN operand.
//! - `dot` / `fir` use `mul_add` (FMA) — allowed by the ulp ≤ 8 contract.
//!   `biquad` deliberately does *not*: a fused product skips one rounding and
//!   the recursive state update compounds it (measured 18–351 ulps at len 100
//!   vs the contract's ≤16), so its vector body keeps the scalar reference's
//!   exact mul/sub/add order instead — unfused ops are bit-exact per lane.
//! - `fade` keeps `idx·step + g0` a separate mul/add — the contract forbids
//!   FMA here.
//! - `biquad` is lane = channel under a *planar* layout, so `x`/`y` are
//!   gathered/scattered per frame via `from_fn`/`as_slice` — the honest cost
//!   of channel-parallel IIR on this layout. Tail blocks
//!   (`channels % LEN != 0`, e.g. 8 ch on 16 lanes) run as one zero-padded
//!   vector block; padded lanes never store back.
//! - `sine` is a sequential scalar recurrence inside `#[simd]` — the
//!   equivalence contract is bit-exact and the matrix records this column's
//!   codegen as a dispatch-cost data point, not a vectorization claim.

use fearless_simd::{Fallback, Level, Select, Simd, SimdBase, SimdFloat, dispatch};
use fearless_simd_macros::simd;
use ruviform_math::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

use super::KernelImpl;

/// `dst[i] = a[i] + b[i]`.
#[simd]
fn add_impl<S: Simd>(simd: S, dst: &mut [f32], a: &[f32], b: &[f32]) {
    let n = S::f32s::LEN;
    let head = dst.len() - dst.len() % n;
    let (d_head, d_tail) = dst.split_at_mut(head);
    for ((d, va), vb) in d_head
        .chunks_exact_mut(n)
        .zip(a.chunks_exact(n))
        .zip(b.chunks_exact(n))
    {
        (S::f32s::from_slice(simd, va) + S::f32s::from_slice(simd, vb)).store_slice(d);
    }
    for (i, d) in d_tail.iter_mut().enumerate() {
        *d = a[head + i] + b[head + i];
    }
}

/// `dst[i] = src[i] * gain`.
#[simd]
fn scale_impl<S: Simd>(simd: S, dst: &mut [f32], src: &[f32], gain: f32) {
    let n = S::f32s::LEN;
    let head = dst.len() - dst.len() % n;
    let (d_head, d_tail) = dst.split_at_mut(head);
    let g = S::f32s::splat(simd, gain);
    for (d, v) in d_head.chunks_exact_mut(n).zip(src.chunks_exact(n)) {
        (S::f32s::from_slice(simd, v) * g).store_slice(d);
    }
    for (i, d) in d_tail.iter_mut().enumerate() {
        *d = src[head + i] * gain;
    }
}

/// `dst[i] = clamp(src[i], -1, 1)` — compare + select so NaN propagates
/// (see the file header; `_precise` min/max would drop NaN lanes).
#[simd]
fn clamp_unit_impl<S: Simd>(simd: S, dst: &mut [f32], src: &[f32]) {
    let n = S::f32s::LEN;
    let head = dst.len() - dst.len() % n;
    let (d_head, d_tail) = dst.split_at_mut(head);
    let lo = S::f32s::splat(simd, -1.0);
    let hi = S::f32s::splat(simd, 1.0);
    for (d, v) in d_head.chunks_exact_mut(n).zip(src.chunks_exact(n)) {
        let v = S::f32s::from_slice(simd, v);
        let c = v.simd_lt(lo).select(lo, v);
        let c = c.simd_gt(hi).select(hi, c);
        c.store_slice(d);
    }
    for (i, d) in d_tail.iter_mut().enumerate() {
        *d = src[head + i].clamp(-1.0, 1.0);
    }
}

/// `dst[i] = src[i] * (g0 + i·step)` — gain stays mul-then-add (no FMA).
#[simd]
#[expect(
    clippy::cast_precision_loss,
    clippy::suboptimal_flops,
    reason = "`i as f32` fits exactly for bench-scale lengths and `g0 + i*step` \
              must stay separate mul/add — FMA is forbidden by the equivalence contract"
)]
fn fade_impl<S: Simd>(simd: S, dst: &mut [f32], src: &[f32], g0: f32, step: f32) {
    let n = S::f32s::LEN;
    let head = dst.len() - dst.len() % n;
    let (d_head, d_tail) = dst.split_at_mut(head);
    let iota = S::f32s::from_fn(simd, |j| j as f32);
    let step_v = S::f32s::splat(simd, step);
    let g0_v = S::f32s::splat(simd, g0);
    for (chunk, (d, v)) in d_head
        .chunks_exact_mut(n)
        .zip(src.chunks_exact(n))
        .enumerate()
    {
        let idx = iota + S::f32s::splat(simd, (chunk * n) as f32);
        let gain = idx * step_v + g0_v;
        (S::f32s::from_slice(simd, v) * gain).store_slice(d);
    }
    for (i, d) in d_tail.iter_mut().enumerate() {
        let i = head + i;
        *d = src[i] * (g0 + i as f32 * step);
    }
}

/// `Σ a[i]·b[i]` — lane-wise FMA accumulation, ordered tail (ulp ≤ 8).
#[simd]
#[expect(
    clippy::suboptimal_flops,
    reason = "the tail loop mirrors the scalar reference's sequential multiply-then-add"
)]
fn dot_impl<S: Simd>(simd: S, a: &[f32], b: &[f32]) -> f32 {
    let n = S::f32s::LEN;
    let head = a.len() - a.len() % n;
    let mut acc = S::f32s::splat(simd, 0.0);
    for (va, vb) in a[..head].chunks_exact(n).zip(b[..head].chunks_exact(n)) {
        acc = S::f32s::from_slice(simd, va).mul_add(S::f32s::from_slice(simd, vb), acc);
    }
    let mut sum = acc.reduce_sum();
    for (&x, &y) in a[head..].iter().zip(&b[head..]) {
        sum += x * y;
    }
    sum
}

/// Direct-form FIR — lane = output position, ordered tap loop with FMA;
/// the zero-history head and the tail run sequentially (ulp ≤ 8).
#[simd]
#[expect(
    clippy::suboptimal_flops,
    reason = "the head/tail loops mirror the scalar reference's sequential multiply-then-add"
)]
fn fir_impl<S: Simd>(simd: S, dst: &mut [f32], src: &[f32], taps: &[f32]) {
    let n = S::f32s::LEN;
    // Outputs `o < taps.len() - 1` see implicit zero history.
    let warmup = taps.len().saturating_sub(1).min(dst.len());
    for (o, d) in dst[..warmup].iter_mut().enumerate() {
        let mut acc = 0.0_f32;
        for (k, &t) in taps.iter().enumerate().take(o + 1) {
            acc += t * src[o - k];
        }
        *d = acc;
    }
    let mut i = warmup;
    while i + n <= dst.len() {
        let mut acc = S::f32s::splat(simd, 0.0);
        for (k, &t) in taps.iter().enumerate() {
            // Lane l reads src[i + l - k] — a contiguous, shifted window.
            let w = S::f32s::from_slice(simd, &src[i - k..i - k + n]);
            acc = S::f32s::splat(simd, t).mul_add(w, acc);
        }
        acc.store_slice(&mut dst[i..i + n]);
        i += n;
    }
    for (o, d) in dst[i..].iter_mut().enumerate() {
        let o = i + o;
        let mut acc = 0.0_f32;
        for (k, &t) in taps.iter().enumerate() {
            acc += t * src[o - k];
        }
        *d = acc;
    }
}

/// Planar `channels`-channel DF-II transposed biquad — lane = channel with a
/// per-frame gather/scatter (see the file header; ulp ≤ 8).
#[simd]
fn biquad_impl<S: Simd>(
    simd: S,
    dst: &mut [f32],
    src: &[f32],
    c: &BiquadCoeffs,
    state: &mut [f32],
    channels: usize,
) {
    let n = S::f32s::LEN;
    let frames = src.len() / channels;
    let (z1s, z2s) = state.split_at_mut(channels);
    let b0 = S::f32s::splat(simd, c.b0);
    let b1 = S::f32s::splat(simd, c.b1);
    let b2 = S::f32s::splat(simd, c.b2);
    let a1 = S::f32s::splat(simd, c.a1);
    let a2 = S::f32s::splat(simd, c.a2);
    let mut ch = 0;
    while ch < channels {
        let rem = n.min(channels - ch);
        let mut z1 = S::f32s::from_fn(simd, |j| if j < rem { z1s[ch + j] } else { 0.0 });
        let mut z2 = S::f32s::from_fn(simd, |j| if j < rem { z2s[ch + j] } else { 0.0 });
        for f in 0..frames {
            let x = S::f32s::from_fn(simd, |j| {
                if j < rem {
                    src[(ch + j) * frames + f]
                } else {
                    0.0
                }
            });
            // y = b0·x + z1; z1' = (b1·x − a1·y) + z2; z2' = b2·x − a2·y —
            // separate mul/sub/add in the scalar reference's exact order.
            // FMA (`mul_add`/`mul_sub`) is *not* used here: a fused product
            // skips one rounding and the recursive state compounds the
            // difference — measured drift of 18–351 ulps at len 100 vs the
            // contract's ≤16. (Unfused vector ops are exact per lane.)
            let y = b0 * x + z1;
            let z1_next = b1 * x - a1 * y + z2;
            z2 = b2 * x - a2 * y;
            z1 = z1_next;
            for (j, &v) in y.as_slice()[..rem].iter().enumerate() {
                dst[(ch + j) * frames + f] = v;
            }
        }
        for (j, &v) in z1.as_slice()[..rem].iter().enumerate() {
            z1s[ch + j] = v;
        }
        for (j, &v) in z2.as_slice()[..rem].iter().enumerate() {
            z2s[ch + j] = v;
        }
        ch += rem;
    }
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` — sequential recurrence run under the token
/// (bit-exact contract; see the file header). The token itself is consumed by
/// the `#[simd]` machinery (`vectorize` context) and unused in the body.
#[simd]
#[expect(
    clippy::suboptimal_flops,
    reason = "sequential multiply-then-add recurrence is the reference rounding; \
              the contract is bit-exact so the body must stay unfused"
)]
fn sine_impl<S: Simd>(simd: S, dst: &mut [f32], c: SineCoeffs, state: &mut SineState) {
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

/// Builds a [`KernelImpl`] column from a module-like `$m` exposing `probe` and
/// the eight kernel fns (`add`, `scale`, `clamp_unit`, `fade`, `dot`, `fir`,
/// `biquad`, `sine`). Internal to this module's column macros.
macro_rules! column {
    ($name:literal, $level:literal, $probe:expr, {$($field:ident : $body:expr),* $(,)?}) => {
        KernelImpl {
            name: $name,
            backend: "fearless",
            level: $level,
            probe: $probe,
            $($field: $body),*
        }
    };
}

/// Forced-`Option`-token column: `probe` reports `($token).is_some()`; each
/// kernel forwards to its `#[simd]` body while the token constructs and
/// returns early otherwise (callers skip via `probe`, kernels never panic).
macro_rules! forced_column {
    ($col:ident, $name:literal, $level:literal, $token:expr) => {
        /// Registry column — forced dispatch level (`Option` token, probe-gated).
        pub(crate) const $col: KernelImpl = column!(
            $name,
            $level,
            || ($token).is_some(),
            {
                add: |dst, a, b| {
                    if let Some(simd) = $token {
                        add_impl(simd, dst, a, b);
                    }
                },
                scale: |dst, src, gain| {
                    if let Some(simd) = $token {
                        scale_impl(simd, dst, src, gain);
                    }
                },
                clamp_unit: |dst, src| {
                    if let Some(simd) = $token {
                        clamp_unit_impl(simd, dst, src);
                    }
                },
                fade: |dst, src, g0, step| {
                    if let Some(simd) = $token {
                        fade_impl(simd, dst, src, g0, step);
                    }
                },
                dot: |a, b| ($token).map_or(0.0, |simd| dot_impl(simd, a, b)),
                fir: |dst, src, taps| {
                    if let Some(simd) = $token {
                        fir_impl(simd, dst, src, taps);
                    }
                },
                biquad: |dst, src, c, state, channels| {
                    if let Some(simd) = $token {
                        biquad_impl(simd, dst, src, c, state, channels);
                    }
                },
                sine: |dst, c, state| {
                    if let Some(simd) = $token {
                        sine_impl(simd, dst, *c, state);
                    }
                },
            }
        );
    };
}

/// `fearless_fallback` — forced `Fallback` token (always constructible; the
/// `force_support_fallback` feature is enabled in dev-dependencies).
pub(crate) const FALLBACK: KernelImpl = column!(
    "fearless_fallback",
    "fallback",
    || true,
    {
        add: |dst, a, b| add_impl(Fallback::new(), dst, a, b),
        scale: |dst, src, gain| scale_impl(Fallback::new(), dst, src, gain),
        clamp_unit: |dst, src| clamp_unit_impl(Fallback::new(), dst, src),
        fade: |dst, src, g0, step| fade_impl(Fallback::new(), dst, src, g0, step),
        dot: |a, b| dot_impl(Fallback::new(), a, b),
        fir: |dst, src, taps| fir_impl(Fallback::new(), dst, src, taps),
        biquad: |dst, src, c, state, channels| {
            biquad_impl(Fallback::new(), dst, src, c, state, channels);
        },
        sine: |dst, c, state| sine_impl(Fallback::new(), dst, *c, state),
    }
);

/// `fearless_dispatched` — `dispatch!(Level::new(), …)`: runtime-detected best
/// level, the production-shaped entry path (always constructible).
pub(crate) const DISPATCHED: KernelImpl = column!(
    "fearless_dispatched",
    "dispatched",
    || true,
    {
        add: |dst, a, b| dispatch!(Level::new(), simd => add_impl(simd, dst, a, b)),
        scale: |dst, src, gain| {
            dispatch!(Level::new(), simd => scale_impl(simd, dst, src, gain));
        },
        clamp_unit: |dst, src| dispatch!(Level::new(), simd => clamp_unit_impl(simd, dst, src)),
        fade: |dst, src, g0, step| {
            dispatch!(Level::new(), simd => fade_impl(simd, dst, src, g0, step));
        },
        dot: |a, b| dispatch!(Level::new(), simd => dot_impl(simd, a, b)),
        fir: |dst, src, taps| dispatch!(Level::new(), simd => fir_impl(simd, dst, src, taps)),
        biquad: |dst, src, c, state, channels| {
            dispatch!(Level::new(), simd => biquad_impl(simd, dst, src, c, state, channels));
        },
        sine: |dst, c, state| dispatch!(Level::new(), simd => sine_impl(simd, dst, *c, state)),
    }
);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(SSE2, "fearless_sse2", "sse2", Level::new().as_sse2());
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(
    SSE4_2,
    "fearless_sse4_2",
    "sse4_2",
    Level::new().as_sse4_2()
);
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(AVX2, "fearless_avx2", "avx2", Level::new().as_avx2());
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(
    AVX512,
    "fearless_avx512",
    "avx512",
    Level::new().as_avx512()
);
#[cfg(target_arch = "aarch64")]
forced_column!(NEON, "fearless_neon", "neon", Level::new().as_neon());
