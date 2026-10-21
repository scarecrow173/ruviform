//! `pulp` 0.22 candidate kernels + dispatch-level columns (task T3).
//!
//! Each kernel is a direct [`WithSimd`] op struct (the baseline form; the
//! `macro` feature's `#[with_simd]` codegen produces the same shape). Forced
//! columns run `Simd::vectorize(token, op)` — the token's `vectorize` enters
//! the `#[target_feature]` context the intrinsics need (calling
//! `op.with_simd(token)` directly would skip it); `pulp_scalar` dispatches the
//! scalar lane via `Arch::Scalar`, `pulp_dispatched` via `Arch::new()`. Forced
//! `x86::V*` tokens come from `try_new()` (`Option`; unavailable → `probe` is
//! `false` → the column is skipped, never panics). x86 columns are
//! `#[cfg]`-gated.
//!
//! Equivalence notes (same decisions as `fearless_impl`):
//! - `clamp_unit` is compare + select so NaN lanes propagate like
//!   `f32::clamp` (pulp's `min_f32s`/`max_f32s` are `_mm_min_ps`-style on x86
//!   and would not preserve NaN propagation portably).
//! - `dot` uses `mul_add_f32s` (FMA) — allowed by the ulp contract (revised
//!   bound; see `equiv.rs`). `fir` / `biquad` deliberately do *not*: a fused
//!   product skips one rounding and the scalar reference's per-output
//!   accumulation order is preserved by these impls, so dropping FMA keeps
//!   them *bit-exact* — measured FMA drift otherwise reaches thousands of
//!   ulps under cancellation (`fir` len ≤ 4096: up to 3994 ulps, f64-truth
//!   verified as pure rounding, not a bug). Unfused vector ops are exact
//!   per lane.
//! - `fade` keeps `idx·step + g0` a separate mul/add — the contract forbids
//!   FMA here.
//! - `biquad` is lane = channel on a *planar* layout: `x`/`y` are
//!   gathered/scattered per frame through a `[f32; MAX_LANES]` scratch —
//!   the honest cost of channel-parallel IIR here. Tail blocks
//!   (`channels % LANES != 0`) run padded via `partial_*` load/store.
//! - `sine` is a sequential scalar recurrence — bit-exact contract; the
//!   column records dispatch cost, not vectorization.

// `#[inline(always)]` on `with_simd` is required by `Simd::vectorize`: the op
// body must inline into the token's `#[target_feature]` trampoline for the
// vectorized code path to take effect (pulp's documented contract).
#![expect(
    clippy::inline_always,
    reason = "pulp::Simd::vectorize requires the WithSimd op body to inline into \
              the token's target_feature trampoline — pulp's documented contract"
)]

use pulp::{Arch, Simd, WithSimd};
use ruviform_math::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

use super::KernelImpl;

/// Widest `f32` vector `pulp` 0.22 produces on x86 (`x86::V4` = 512-bit) —
/// the scratch width for `biquad`'s gather/scatter and `fade`'s lane index.
const MAX_LANES: usize = 16;

/// `dst[i] = a[i] + b[i]`.
struct Add<'a> {
    dst: &'a mut [f32],
    a: &'a [f32],
    b: &'a [f32],
}

impl WithSimd for Add<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { dst, a, b } = self;
        let (d_head, d_tail) = S::as_mut_simd_f32s(dst);
        let (a_head, a_tail) = S::as_simd_f32s(a);
        let (b_head, b_tail) = S::as_simd_f32s(b);
        for ((d, &va), &vb) in d_head.iter_mut().zip(a_head).zip(b_head) {
            *d = simd.add_f32s(va, vb);
        }
        for ((d, &x), &y) in d_tail.iter_mut().zip(a_tail).zip(b_tail) {
            *d = x + y;
        }
    }
}

/// `dst[i] = src[i] * gain`.
struct Scale<'a> {
    dst: &'a mut [f32],
    src: &'a [f32],
    gain: f32,
}

impl WithSimd for Scale<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { dst, src, gain } = self;
        let g = simd.splat_f32s(gain);
        let (d_head, d_tail) = S::as_mut_simd_f32s(dst);
        let (s_head, s_tail) = S::as_simd_f32s(src);
        for (d, &x) in d_head.iter_mut().zip(s_head) {
            *d = simd.mul_f32s(x, g);
        }
        for (d, &x) in d_tail.iter_mut().zip(s_tail) {
            *d = x * gain;
        }
    }
}

/// `dst[i] = clamp(src[i], -1, 1)` — compare + select so NaN propagates.
struct ClampUnit<'a> {
    dst: &'a mut [f32],
    src: &'a [f32],
}

impl WithSimd for ClampUnit<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { dst, src } = self;
        let lo = simd.splat_f32s(-1.0);
        let hi = simd.splat_f32s(1.0);
        let (d_head, d_tail) = S::as_mut_simd_f32s(dst);
        let (s_head, s_tail) = S::as_simd_f32s(src);
        for (d, &x) in d_head.iter_mut().zip(s_head) {
            // Mirrors `f32::clamp`: `x < lo → lo`, `x > hi → hi`, else `x` —
            // comparisons are false for NaN so NaN lanes propagate.
            let c = simd.select_f32s(simd.less_than_f32s(x, lo), lo, x);
            *d = simd.select_f32s(simd.greater_than_f32s(c, hi), hi, c);
        }
        for (d, &x) in d_tail.iter_mut().zip(s_tail) {
            *d = x.clamp(-1.0, 1.0);
        }
    }
}

/// `dst[i] = src[i] * (g0 + i·step)` — gain stays mul-then-add (no FMA).
struct Fade<'a> {
    dst: &'a mut [f32],
    src: &'a [f32],
    g0: f32,
    step: f32,
}

impl WithSimd for Fade<'_> {
    type Output = ();

    #[inline(always)]
    #[expect(
        clippy::cast_precision_loss,
        clippy::suboptimal_flops,
        reason = "`i as f32` fits exactly for bench-scale lengths and `g0 + i*step` \
                  must stay separate mul/add — FMA is forbidden by the equivalence contract"
    )]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { dst, src, g0, step } = self;
        let lanes = S::F32_LANES;
        debug_assert!(lanes <= MAX_LANES);
        let mut iota_buf = [0.0_f32; MAX_LANES];
        for (j, e) in iota_buf[..lanes].iter_mut().enumerate() {
            *e = j as f32;
        }
        let iota = simd.partial_load_f32s(&iota_buf[..lanes]);
        let step_v = simd.splat_f32s(step);
        let g0_v = simd.splat_f32s(g0);
        let (d_head, d_tail) = S::as_mut_simd_f32s(dst);
        let (s_head, s_tail) = S::as_simd_f32s(src);
        for (i, (d, &x)) in d_head.iter_mut().zip(s_head).enumerate() {
            let idx = simd.add_f32s(iota, simd.splat_f32s((i * lanes) as f32));
            let gain = simd.add_f32s(simd.mul_f32s(idx, step_v), g0_v);
            *d = simd.mul_f32s(x, gain);
        }
        let head_elems = d_head.len() * lanes;
        for (i, (d, &x)) in d_tail.iter_mut().zip(s_tail).enumerate() {
            let i = head_elems + i;
            *d = x * (g0 + i as f32 * step);
        }
    }
}

/// `Σ a[i]·b[i]` — lane-wise FMA accumulation, ordered tail (ulp ≤ 8).
struct Dot<'a> {
    a: &'a [f32],
    b: &'a [f32],
}

impl WithSimd for Dot<'_> {
    type Output = f32;

    #[inline(always)]
    #[expect(
        clippy::suboptimal_flops,
        reason = "the tail loop mirrors the scalar reference's sequential multiply-then-add"
    )]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { a, b } = self;
        let (a_head, a_tail) = S::as_simd_f32s(a);
        let (b_head, b_tail) = S::as_simd_f32s(b);
        let mut acc = simd.splat_f32s(0.0);
        for (&va, &vb) in a_head.iter().zip(b_head) {
            acc = simd.mul_add_f32s(va, vb, acc);
        }
        let mut sum = simd.reduce_sum_f32s(acc);
        for (&x, &y) in a_tail.iter().zip(b_tail) {
            sum += x * y;
        }
        sum
    }
}

/// Direct-form FIR — lane = output position, ordered tap loop *without* FMA:
/// each output's tap order already matches the scalar reference, so unfused
/// `t*w + acc` is bit-exact per lane (see the file header — the fused form
/// drifted past the ulp ≤ 8 contract under cancellation). The zero-history
/// head and the tail run sequentially.
struct Fir<'a> {
    dst: &'a mut [f32],
    src: &'a [f32],
    taps: &'a [f32],
}

impl WithSimd for Fir<'_> {
    type Output = ();

    #[inline(always)]
    #[expect(
        clippy::suboptimal_flops,
        reason = "the head/tail loops mirror the scalar reference's sequential multiply-then-add"
    )]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self { dst, src, taps } = self;
        let lanes = S::F32_LANES;
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
        while i + lanes <= dst.len() {
            let mut acc = simd.splat_f32s(0.0);
            for (k, &t) in taps.iter().enumerate() {
                // Lane l reads src[i + l - k] — a contiguous, shifted window.
                // `t*w + acc` stays unfused (see the struct comment): the
                // tap loop order equals the scalar reference per output, so
                // each lane is bit-exact — FMA would drift past the bound.
                let w = simd.partial_load_f32s(&src[i - k..i - k + lanes]);
                acc = simd.add_f32s(simd.mul_f32s(simd.splat_f32s(t), w), acc);
            }
            simd.partial_store_f32s(&mut dst[i..i + lanes], acc);
            i += lanes;
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
}

/// Planar `channels`-channel DF-II transposed biquad — lane = channel with a
/// per-frame gather/scatter through scratch (see the file header; ulp ≤ 8).
struct Biquad<'a> {
    dst: &'a mut [f32],
    src: &'a [f32],
    c: &'a BiquadCoeffs,
    state: &'a mut [f32],
    channels: usize,
}

impl WithSimd for Biquad<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            dst,
            src,
            c,
            state,
            channels,
        } = self;
        let lanes = S::F32_LANES;
        let frames = src.len() / channels;
        let (z1s, z2s) = state.split_at_mut(channels);
        let b0 = simd.splat_f32s(c.b0);
        let b1 = simd.splat_f32s(c.b1);
        let b2 = simd.splat_f32s(c.b2);
        let a1 = simd.splat_f32s(c.a1);
        let a2 = simd.splat_f32s(c.a2);
        let mut ch = 0;
        while ch < channels {
            let rem = lanes.min(channels - ch);
            // z-state is contiguous in the SoA layout — partial_* covers the
            // padded tail block (`channels % LANES != 0`) directly.
            let mut z1 = simd.partial_load_f32s(&z1s[ch..ch + rem]);
            let mut z2 = simd.partial_load_f32s(&z2s[ch..ch + rem]);
            for f in 0..frames {
                // Planar x/y → per-frame channel gather/scatter.
                let mut xbuf = [0.0_f32; MAX_LANES];
                for (j, e) in xbuf[..rem].iter_mut().enumerate() {
                    *e = src[(ch + j) * frames + f];
                }
                let x = simd.partial_load_f32s(&xbuf[..lanes]);
                // y = b0·x + z1; z1' = (b1·x − a1·y) + z2; z2' = b2·x − a2·y —
                // separate mul/sub/add in the scalar reference's exact order.
                // FMA (`mul_add_f32s`) is *not* used here: a fused product
                // skips one rounding and the recursive state compounds the
                // difference — measured drift of 18–351 ulps at len 100 vs
                // the contract's ≤8. (Unfused vector ops are exact per lane.)
                let y = simd.add_f32s(simd.mul_f32s(b0, x), z1);
                let z1_next = simd.add_f32s(
                    simd.sub_f32s(simd.mul_f32s(b1, x), simd.mul_f32s(a1, y)),
                    z2,
                );
                z2 = simd.sub_f32s(simd.mul_f32s(b2, x), simd.mul_f32s(a2, y));
                z1 = z1_next;
                let mut ybuf = [0.0_f32; MAX_LANES];
                simd.partial_store_f32s(&mut ybuf[..rem], y);
                for (j, &v) in ybuf[..rem].iter().enumerate() {
                    dst[(ch + j) * frames + f] = v;
                }
            }
            simd.partial_store_f32s(&mut z1s[ch..ch + rem], z1);
            simd.partial_store_f32s(&mut z2s[ch..ch + rem], z2);
            ch += rem;
        }
    }
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` — sequential recurrence under the token
/// (bit-exact contract; see the file header).
struct Sine<'a> {
    dst: &'a mut [f32],
    c: &'a SineCoeffs,
    state: &'a mut SineState,
}

impl WithSimd for Sine<'_> {
    type Output = ();

    #[inline(always)]
    #[expect(
        clippy::suboptimal_flops,
        reason = "sequential multiply-then-add recurrence is the reference rounding; \
                  the contract is bit-exact so the body must stay unfused"
    )]
    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        let Self { dst, c, state } = self;
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
}

/// Shared column constructor (`backend` fixed to `"pulp"`).
macro_rules! column {
    ($name:literal, $level:literal, $probe:expr, {$($field:ident : $body:expr),* $(,)?}) => {
        KernelImpl {
            name: $name,
            backend: "pulp",
            level: $level,
            probe: $probe,
            $($field: $body),*
        }
    };
}

/// `Arch`-dispatch column: `$arch` is evaluated per call (`Arch::Scalar` for
/// the forced scalar lane, `Arch::new()` for the detected best level).
macro_rules! dispatch_column {
    ($col:ident, $name:literal, $level:literal, $arch:expr) => {
        /// Registry column — `pulp` `Arch` dispatch (always constructible).
        pub(crate) const $col: KernelImpl = column!(
            $name,
            $level,
            || true,
            {
                add: |dst, a, b| $arch.dispatch(Add { dst, a, b }),
                scale: |dst, src, gain| $arch.dispatch(Scale { dst, src, gain }),
                clamp_unit: |dst, src| $arch.dispatch(ClampUnit { dst, src }),
                fade: |dst, src, g0, step| $arch.dispatch(Fade { dst, src, g0, step }),
                dot: |a, b| $arch.dispatch(Dot { a, b }),
                fir: |dst, src, taps| $arch.dispatch(Fir { dst, src, taps }),
                biquad: |dst, src, c, state, channels| {
                    $arch.dispatch(Biquad {
                        dst,
                        src,
                        c,
                        state,
                        channels,
                    });
                },
                sine: |dst, c, state| $arch.dispatch(Sine { dst, c, state }),
            }
        );
    };
}

/// Forced-`Option`-token column (`pulp::x86::V*::try_new()`): the token's
/// `Simd::vectorize` enters its `#[target_feature]` context, then runs the op.
macro_rules! forced_column {
    ($col:ident, $name:literal, $level:literal, $token:expr) => {
        /// Registry column — forced `x86` level (`Option` token, probe-gated).
        pub(crate) const $col: KernelImpl = column!(
            $name,
            $level,
            || ($token).is_some(),
            {
                add: |dst, a, b| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, Add { dst, a, b });
                    }
                },
                scale: |dst, src, gain| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, Scale { dst, src, gain });
                    }
                },
                clamp_unit: |dst, src| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, ClampUnit { dst, src });
                    }
                },
                fade: |dst, src, g0, step| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, Fade { dst, src, g0, step });
                    }
                },
                dot: |a, b| ($token).map_or(0.0, |simd| Simd::vectorize(simd, Dot { a, b })),
                fir: |dst, src, taps| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, Fir { dst, src, taps });
                    }
                },
                biquad: |dst, src, c, state, channels| {
                    if let Some(simd) = $token {
                        Simd::vectorize(
                            simd,
                            Biquad {
                                dst,
                                src,
                                c,
                                state,
                                channels,
                            },
                        );
                    }
                },
                sine: |dst, c, state| {
                    if let Some(simd) = $token {
                        Simd::vectorize(simd, Sine { dst, c, state });
                    }
                },
            }
        );
    };
}

dispatch_column!(SCALAR, "pulp_scalar", "scalar", Arch::Scalar);
dispatch_column!(DISPATCHED, "pulp_dispatched", "dispatched", Arch::new());

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(V2, "pulp_v2", "v2", pulp::x86::V2::try_new());
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(V3, "pulp_v3", "v3", pulp::x86::V3::try_new());
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
forced_column!(V4, "pulp_v4", "v4", pulp::x86::V4::try_new());
