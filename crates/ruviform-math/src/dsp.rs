//! `ruviform_math::dsp` — SIMD-dispatched DSP kernels (Issue #53).
//!
//! The production dispatch layer: callers build a [`SimdToken`] once at the
//! prepare/block boundary and pass it to every kernel call. Per-call cost is
//! one predictable tag match plus the selected kernel — no `dispatch!`,
//! `Level::new()`, or feature detection runs on the RT path (measured
//! per-call dispatch overhead ≈1.3 ns; see
//! `docs/impl/exp/dispatched-overhead-20261021.md`).
//!
//! Two-layer structure (adoption ADR, decision 1): the scalar reference
//! implementations in `crate::simd_poc` are the contract reference; every
//! SIMD path is verified against them per kernel. Backend types
//! (`fearless_simd`) are an implementation detail and never appear in this
//! public API.
//!
//! ## Routing
//!
//! Small buffers route to the scalar reference when the fixed SIMD call
//! overhead is not amortized (measured thresholds — see each `*_SCALAR_BELOW`
//! const). Routing affects performance only, never results: all paths honor
//! the same precision contract.
//!
//! ## Precision contract
//!
//! Inputs are finite `f32`. NaN/Inf inputs propagate but are outside the
//! accuracy contract; strongly cancelling inputs can exceed any fixed ulp
//! bound — a guarantee for that domain requires a contract-form change
//! (ADR `simd-ulp.md`), never post-hoc relaxation.
//!
//! ## RT constraints
//!
//! All functions are allocation-free and panic-free: contract violations
//! (mismatched lengths, `channels == 0`, short `state`) are caught by
//! `debug_assert` in debug builds; release builds process only the
//! well-defined prefix.

#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
use fearless_simd::Level;
#[cfg(target_arch = "aarch64")]
use fearless_simd::Neon;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use fearless_simd::{Avx2, Sse2, Sse4_2};
use fearless_simd::{Fallback, Select, Simd, SimdBase, SimdFloat};
use fearless_simd_macros::simd;

pub use crate::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

/// Resolved dispatch level inside a [`SimdToken`]. Internal — backend token
/// types never leak into the public API.
#[derive(Clone, Copy, Debug)]
enum Sel {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2(Avx2),
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Sse4_2(Sse4_2),
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Sse2(Sse2),
    #[cfg(target_arch = "aarch64")]
    Neon(Neon),
    /// Scalar token — terminal for targets without a constructible SIMD level
    /// and the forced path used by tests.
    Fallback(Fallback),
}

/// Runtime-detected SIMD capability token. Build once at prepare/block
/// boundary via [`SimdToken::detect`]; pass to every `dsp` kernel call.
///
/// `Copy + Send + Sync` — a ZST wrapper, free to store in processor structs
/// and share across threads.
#[derive(Clone, Copy, Debug)]
pub struct SimdToken {
    sel: Sel,
}

impl SimdToken {
    /// Detect the best validated SIMD level for this CPU.
    ///
    /// x86 order: AVX2 → SSE4.2 → SSE2 → fallback. aarch64: NEON → fallback.
    /// AVX-512 is intentionally not selected: it is unmeasured (adoption ADR
    /// — enablement waits for Issue #55 hardware/CI measurement); `as_avx2`
    /// downgrades from stronger supersets, so AVX-512 hosts still get AVX2.
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let level = Level::new();
            if let Some(t) = level.as_avx2() {
                return Self { sel: Sel::Avx2(t) };
            }
            if let Some(t) = level.as_sse4_2() {
                return Self {
                    sel: Sel::Sse4_2(t),
                };
            }
            if let Some(t) = level.as_sse2() {
                return Self { sel: Sel::Sse2(t) };
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            if let Some(t) = Level::new().as_neon() {
                return Self { sel: Sel::Neon(t) };
            }
        }
        Self {
            sel: Sel::Fallback(Fallback::new()),
        }
    }

    /// Name of the selected ISA level (`"avx2"`, `"sse4_2"`, `"sse2"`,
    /// `"neon"`, `"fallback"`). Diagnostics and tests only.
    #[must_use]
    pub fn level(&self) -> &'static str {
        match self.sel {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Sel::Avx2(_) => "avx2",
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Sel::Sse4_2(_) => "sse4_2",
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Sel::Sse2(_) => "sse2",
            #[cfg(target_arch = "aarch64")]
            Sel::Neon(_) => "neon",
            Sel::Fallback(_) => "fallback",
        }
    }

    /// Scalar-path token. Always constructible.
    #[doc(hidden)]
    #[must_use]
    pub fn force_fallback() -> Self {
        Self {
            sel: Sel::Fallback(Fallback::new()),
        }
    }

    /// Forced SSE2 token (`None` when unsupported). Test/bench only.
    #[doc(hidden)]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[must_use]
    pub fn force_sse2() -> Option<Self> {
        Level::new().as_sse2().map(|t| Self { sel: Sel::Sse2(t) })
    }

    /// Forced SSE4.2 token (`None` when unsupported). Test/bench only.
    #[doc(hidden)]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[must_use]
    pub fn force_sse4_2() -> Option<Self> {
        Level::new().as_sse4_2().map(|t| Self {
            sel: Sel::Sse4_2(t),
        })
    }

    /// Forced AVX2 token (`None` when unsupported). Test/bench only.
    #[doc(hidden)]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[must_use]
    pub fn force_avx2() -> Option<Self> {
        Level::new().as_avx2().map(|t| Self { sel: Sel::Avx2(t) })
    }

    /// Forced NEON token (`None` when unsupported). Test/bench only.
    #[doc(hidden)]
    #[cfg(target_arch = "aarch64")]
    #[must_use]
    pub fn force_neon() -> Option<Self> {
        Level::new().as_neon().map(|t| Self { sel: Sel::Neon(t) })
    }
}

// ---- routing thresholds (determined values) --------------------------------
// Provenance: `docs/impl/exp/dispatched-overhead-20261021.md` 3-run medians,
// scalar vs forced-avx2 on i9-9900K. Scalar is routed while it is not slower
// than the token path; these are performance heuristics, not contract.

/// `add`: scalar wins at n≤32 (0.62–0.79x) and is at parity through n=100;
/// AVX2 first wins clearly at n=256 (1.36x). 128 keeps the whole non-losing
/// band on scalar.
const ADD_SCALAR_BELOW: usize = 128;
/// `scale`: AVX2 already wins at the smallest measured size (n=64, 1.17x).
/// Conservative floor protecting only the unmeasured sub-64 band where the
/// ~2–3 ns fixed call overhead would dominate.
const SCALE_SCALAR_BELOW: usize = 64;
/// `clamp_unit`: scalar wins at n=64–100 (0.76–0.79x) and stays at parity
/// afterwards (memory-bandwidth-bound — SIMD gains nothing).
const CLAMP_SCALAR_BELOW: usize = 256;

// ---- scalar reference path (contract "truth") -------------------------------

fn scalar_add(dst: &mut [f32], a: &[f32], b: &[f32]) {
    crate::simd_poc::add(dst, a, b);
}
fn scalar_scale(dst: &mut [f32], src: &[f32], gain: f32) {
    crate::simd_poc::scale(dst, src, gain);
}
fn scalar_clamp_unit(dst: &mut [f32], src: &[f32]) {
    crate::simd_poc::clamp_unit(dst, src);
}

// ---- `#[simd]` kernel bodies ------------------------------------------------
// Ported from the PoC (`tests/simd_poc/fearless_impl.rs`). Equivalence notes:
// - `clamp_unit` uses compare + select (matching `f32::clamp` order) so NaN
//   lanes propagate — `min_precise`/`max_precise` would drop them.
// - `dot` uses `mul_add` (FMA) — allowed by its ulp ≤ 256 contract.
// - `fir`/`biquad`/`fade`/`sine` deliberately avoid FMA: a fused product
//   skips one rounding and measured drift exceeded the ulp ≤ 8 / bit-exact
//   contracts under cancellation (fir: up to 3994 ulps, f64-verified).
// - `biquad` is lane = channel under a planar layout, so `x`/`y` are
//   gathered/scattered per frame via `from_fn`/`as_slice`.
// - `sine` is a sequential recurrence inside `#[simd]` — bit-exact contract.

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

/// `dst[i] = clamp(src[i], -1, 1)` — compare + select so NaN propagates.
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

/// `Σ a[i]·b[i]` — lane-wise FMA accumulation, ordered tail (ulp ≤ 256).
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

/// Direct-form FIR — lane = output position, ordered tap loop *without* FMA
/// (unfused `t*w + acc` keeps per-output tap order identical to the scalar
/// reference → bit-exact per lane; fused accumulation measured at up to
/// 3994 ulps under cancellation). Zero-history head and tail run sequential.
#[simd]
#[expect(
    clippy::suboptimal_flops,
    reason = "unfused multiply-then-add mirrors the scalar reference's exact rounding"
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
            acc = S::f32s::splat(simd, t) * w + acc;
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
/// per-frame gather/scatter (ulp ≤ 8; no FMA — recursive state compounds
/// fused-rounding drift, measured 18–351 ulps at len 100).
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

/// `y[n] = c1·y[n-1] + c2·y[n-2]` — sequential recurrence; the token is
/// consumed by the `#[simd]` machinery and unused in the body (bit-exact).
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

// ---- public routed kernels --------------------------------------------------

/// `dst[i] = a[i] + b[i]` — elementwise add.
///
/// Contract: `dst.len() == a.len() == b.len()`. Precision: **bit-exact** with
/// the scalar reference for finite input; NaN input propagates (`is_nan`
/// agreement is the contract). Inputs are finite `f32` — NaN/Inf propagate
/// but are outside the accuracy contract (ADR `simd-ulp.md`).
///
/// Buffers shorter than `dst` process the common prefix only; `debug_assert`
/// catches length mismatches in debug builds, release builds never panic.
pub fn add(simd: SimdToken, dst: &mut [f32], a: &[f32], b: &[f32]) {
    debug_assert_eq!(dst.len(), a.len());
    debug_assert_eq!(a.len(), b.len());
    let n = dst.len().min(a.len()).min(b.len());
    let (dst, a, b) = (&mut dst[..n], &a[..n], &b[..n]);
    if n < ADD_SCALAR_BELOW {
        return scalar_add(dst, a, b);
    }
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => add_impl(t, dst, a, b),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => add_impl(t, dst, a, b),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => add_impl(t, dst, a, b),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => add_impl(t, dst, a, b),
        Sel::Fallback(t) => add_impl(t, dst, a, b),
    }
}

/// `dst[i] = src[i] * gain` — elementwise gain.
///
/// Contract: `dst.len() == src.len()`. Precision: **bit-exact** with the
/// scalar reference; NaN propagates (`is_nan` agreement).
///
/// Non-panicking on length mismatch (common prefix only); `debug_assert` in
/// debug builds.
pub fn scale(simd: SimdToken, dst: &mut [f32], src: &[f32], gain: f32) {
    debug_assert_eq!(dst.len(), src.len());
    let n = dst.len().min(src.len());
    let (dst, src) = (&mut dst[..n], &src[..n]);
    if n < SCALE_SCALAR_BELOW {
        return scalar_scale(dst, src, gain);
    }
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => scale_impl(t, dst, src, gain),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => scale_impl(t, dst, src, gain),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => scale_impl(t, dst, src, gain),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => scale_impl(t, dst, src, gain),
        Sel::Fallback(t) => scale_impl(t, dst, src, gain),
    }
}

/// `dst[i] = clamp(src[i], -1.0, 1.0)` — NaN input → NaN output.
///
/// Contract: `dst.len() == src.len()`. Precision: **bit-exact**; NaN
/// propagates (`is_nan` agreement).
///
/// Non-panicking on length mismatch (common prefix only); `debug_assert` in
/// debug builds.
pub fn clamp_unit(simd: SimdToken, dst: &mut [f32], src: &[f32]) {
    debug_assert_eq!(dst.len(), src.len());
    let n = dst.len().min(src.len());
    let (dst, src) = (&mut dst[..n], &src[..n]);
    if n < CLAMP_SCALAR_BELOW {
        return scalar_clamp_unit(dst, src);
    }
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => clamp_unit_impl(t, dst, src),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => clamp_unit_impl(t, dst, src),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => clamp_unit_impl(t, dst, src),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => clamp_unit_impl(t, dst, src),
        Sel::Fallback(t) => clamp_unit_impl(t, dst, src),
    }
}

/// `dst[i] = src[i] * (g0 + i·step)` — elementwise ramp gain.
///
/// The gain term is evaluated per element as `g0 + (i as f32) * step` (no
/// FMA — the contract forbids it; the closed-form `(i)` evaluation, not
/// incremental accumulation, defines the reference rounding).
///
/// Contract: `dst.len() == src.len()`. Precision: **bit-exact**; NaN
/// propagates (`is_nan` agreement).
///
/// Non-panicking on length mismatch (common prefix only); `debug_assert` in
/// debug builds.
pub fn fade(simd: SimdToken, dst: &mut [f32], src: &[f32], g0: f32, step: f32) {
    debug_assert_eq!(dst.len(), src.len());
    let n = dst.len().min(src.len());
    let (dst, src) = (&mut dst[..n], &src[..n]);
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => fade_impl(t, dst, src, g0, step),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => fade_impl(t, dst, src, g0, step),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => fade_impl(t, dst, src, g0, step),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => fade_impl(t, dst, src, g0, step),
        Sel::Fallback(t) => fade_impl(t, dst, src, g0, step),
    }
}

/// `Σ a[i]·b[i]` — dot product.
///
/// Contract: `a.len() == b.len()`; finite inputs. Precision: **ulp ≤ 256**
/// vs the scalar reference (lane-split reduction reorders accumulation —
/// measured max 46 ulps on `PoC` input classes; strongly cancelling inputs
/// can exceed any fixed ulp bound and are out of contract).
///
/// Accumulates over the common prefix on length mismatch; `debug_assert` in
/// debug builds.
#[must_use]
pub fn dot(simd: SimdToken, a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => dot_impl(t, a, b),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => dot_impl(t, a, b),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => dot_impl(t, a, b),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => dot_impl(t, a, b),
        Sel::Fallback(t) => dot_impl(t, a, b),
    }
}

/// `dst[n] = Σ_k taps[k]·src[n−k]` — direct-form FIR, arbitrary `taps` length.
/// Samples before `src[0]` are implicit zeros (zero history).
///
/// Contract: `dst.len() == src.len()`; coefficients and input finite.
/// Precision: **ulp ≤ 8** vs the scalar reference (tap order is preserved and
/// FMA is deliberately not used).
///
/// Processes `min(dst.len(), src.len())` outputs; `debug_assert` on length
/// mismatch in debug builds, never panics in release.
pub fn fir(simd: SimdToken, dst: &mut [f32], src: &[f32], taps: &[f32]) {
    debug_assert_eq!(dst.len(), src.len());
    let n = dst.len().min(src.len());
    let (dst, src) = (&mut dst[..n], &src[..n]);
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => fir_impl(t, dst, src, taps),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => fir_impl(t, dst, src, taps),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => fir_impl(t, dst, src, taps),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => fir_impl(t, dst, src, taps),
        Sel::Fallback(t) => fir_impl(t, dst, src, taps),
    }
}

/// Planar `channels`-channel IIR biquad — Direct Form II transposed,
/// persistent state (`y[n] = b0·x[n] + z1`, `z1' = b1·x[n] - a1·y[n] + z2`,
/// `z2' = b2·x[n] - a2·y[n]`).
///
/// Layout: `src`/`dst` are planar `[ch0 frames][ch1 frames]…` with
/// `frames = len / channels` per channel; `state` is `SoA`
/// `[z1×channels][z2×channels]` with `state.len() == 2 * channels`.
///
/// Contract: `dst.len() == src.len()`, `src.len() % channels == 0`,
/// `channels > 0`, `state.len() == 2 * channels`. Precision: **ulp ≤ 8** vs
/// the scalar reference (FMA deliberately unused — recursive state compounds
/// fused-rounding drift). NaN/Inf contaminates the recursive state; `is_nan`
/// agreement is the contract there.
///
/// Non-panicking on contract violation: `channels` is clamped to
/// `state.len() / 2`, `channels == 0` returns early, and only the
/// `frames * channels` prefix common to `dst`/`src` is processed.
/// `debug_assert` catches violations in debug builds.
pub fn biquad(
    simd: SimdToken,
    dst: &mut [f32],
    src: &[f32],
    c: &BiquadCoeffs,
    state: &mut [f32],
    channels: usize,
) {
    debug_assert!(channels > 0);
    debug_assert_eq!(dst.len(), src.len());
    debug_assert_eq!(src.len() % channels, 0);
    debug_assert_eq!(state.len(), 2 * channels);
    let channels = channels.min(state.len() / 2);
    if channels == 0 {
        return;
    }
    let n = dst.len().min(src.len());
    let frames = n / channels;
    if frames == 0 {
        return;
    }
    let n = frames * channels;
    let (dst, src) = (&mut dst[..n], &src[..n]);
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => biquad_impl(t, dst, src, c, state, channels),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => biquad_impl(t, dst, src, c, state, channels),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => biquad_impl(t, dst, src, c, state, channels),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => biquad_impl(t, dst, src, c, state, channels),
        Sel::Fallback(t) => biquad_impl(t, dst, src, c, state, channels),
    }
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` — 2nd-order recursive oscillator with
/// caller-owned state carried across calls (no allocation).
///
/// Pure serial recurrence: every level runs the same sequential loop.
/// Precision: **bit-exact** — a difference would indicate a dispatch-layer
/// codegen difference.
pub fn sine(simd: SimdToken, dst: &mut [f32], c: &SineCoeffs, state: &mut SineState) {
    match simd.sel {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Avx2(t) => sine_impl(t, dst, *c, state),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse4_2(t) => sine_impl(t, dst, *c, state),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Sel::Sse2(t) => sine_impl(t, dst, *c, state),
        #[cfg(target_arch = "aarch64")]
        Sel::Neon(t) => sine_impl(t, dst, *c, state),
        Sel::Fallback(t) => sine_impl(t, dst, *c, state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    #[test]
    fn detect_returns_constructible_level() {
        let tok = SimdToken::detect();
        // Every supported host must resolve to a concrete level; on this
        // dev machine (AVX2) it must be "avx2".
        assert_ne!(tok.level(), "");
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if std::arch::is_x86_feature_detected!("avx2") {
            assert_eq!(tok.level(), "avx2");
        }
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "fixture indices are far below the f32 exact-integer range"
    )]
    fn add_matches_scalar_reference() {
        let tok = SimdToken::detect();
        let a: Vec<f32> = (0..300).map(|i| i as f32 * 0.5).collect();
        let b: Vec<f32> = (0..300).map(|i| i as f32 - 10.0).collect();
        let mut d = vec![0.0; 300];
        let mut expect = vec![0.0; 300];
        add(tok, &mut d, &a, &b);
        crate::simd_poc::add(&mut expect, &a, &b);
        assert_eq!(d, expect);
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "fixture indices are far below the f32 exact-integer range"
    )]
    fn small_size_routes_scalar_and_matches_reference() {
        // Below the scalar-routing threshold the public path must still be
        // bit-identical to the scalar reference.
        let tok = SimdToken::detect();
        let a: Vec<f32> = (0..ADD_SCALAR_BELOW - 1).map(|i| i as f32).collect();
        let b = vec![1.0; ADD_SCALAR_BELOW - 1];
        let mut d = vec![0.0; ADD_SCALAR_BELOW - 1];
        let mut expect = vec![0.0; ADD_SCALAR_BELOW - 1];
        add(tok, &mut d, &a, &b);
        crate::simd_poc::add(&mut expect, &a, &b);
        assert_eq!(d, expect);
    }
    /// Constructible forced tokens, for direct `*_impl` verification that
    /// bypasses size routing.
    fn forced_tokens() -> Vec<SimdToken> {
        let mut v = vec![SimdToken::force_fallback()];
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        v.extend(
            [
                SimdToken::force_sse2(),
                SimdToken::force_sse4_2(),
                SimdToken::force_avx2(),
            ]
            .into_iter()
            .flatten(),
        );
        #[cfg(target_arch = "aarch64")]
        v.extend(SimdToken::force_neon());
        v
    }

    /// All 8 kernels via the concrete token — exercised at every size,
    /// including sizes below the routing thresholds.
    #[expect(
        clippy::many_single_char_names,
        clippy::cast_precision_loss,
        reason = "fixture values are small; short names mirror kernel signatures"
    )]
    fn verify_kernels<S: Simd>(simd: S, n: usize) {
        let a: Vec<f32> = (0..n).map(|i| (i as f32 * 0.5) - 3.0).collect();
        let b: Vec<f32> = (0..n).map(|i| (i as f32 * 0.25) + 1.0).collect();
        let taps = crate::simd_poc::fir_taps();

        let mut d = vec![0.0; n];
        let mut e = vec![0.0; n];
        add_impl(simd, &mut d, &a, &b);
        crate::simd_poc::add(&mut e, &a, &b);
        assert_eq!(d, e, "add n={n}");

        scale_impl(simd, &mut d, &a, 0.5);
        crate::simd_poc::scale(&mut e, &a, 0.5);
        assert_eq!(d, e, "scale n={n}");

        clamp_unit_impl(simd, &mut d, &a);
        crate::simd_poc::clamp_unit(&mut e, &a);
        assert_eq!(d, e, "clamp_unit n={n}");

        fade_impl(simd, &mut d, &a, 0.0, 0.01);
        crate::simd_poc::fade(&mut e, &a, 0.0, 0.01);
        assert_eq!(d, e, "fade n={n}");

        let got = dot_impl(simd, &a, &b);
        let want = crate::simd_poc::dot(&a, &b);
        // ulp-bounded contract (dot ≤ 256); bit-equality is not required.
        assert!(
            approx::ulps_eq!(got, want, max_ulps = 256),
            "dot n={n}: {got} vs {want}"
        );

        fir_impl(simd, &mut d, &a, &taps);
        crate::simd_poc::fir(&mut e, &a, &taps);
        assert_eq!(d, e, "fir n={n}");

        let channels = 2;
        let frames = n / channels;
        let src: Vec<f32> = (0..frames * channels)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();
        let mut d = vec![0.0; frames * channels];
        let mut e = vec![0.0; frames * channels];
        let mut st = vec![0.0; 2 * channels];
        let mut st_e = vec![0.0; 2 * channels];
        let c = crate::simd_poc::BIQUAD_COEFFS;
        biquad_impl(simd, &mut d, &src, &c, &mut st, channels);
        crate::simd_poc::biquad(&mut e, &src, &c, &mut st_e, channels);
        assert_eq!(d, e, "biquad n={n}");
        assert_eq!(st, st_e, "biquad state n={n}");

        let mut st = SineState { y1: 0.0, y2: 0.5 };
        let mut st_e = st;
        let c = SineCoeffs::for_omega(0.1);
        sine_impl(simd, &mut d, c, &mut st);
        crate::simd_poc::sine(&mut e, &c, &mut st_e);
        assert_eq!(d, e, "sine n={n}");
        assert_eq!(st.y1, st_e.y1);
    }

    #[test]
    fn forced_levels_match_scalar_reference_all_sizes() {
        for tok in forced_tokens() {
            for n in [1usize, 4, 16, 63, 64, 127, 128, 255, 256, 300, 1000] {
                let frames = n / 2 * 2; // keep biquad friendly
                let _ = frames;
                match tok.sel {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    Sel::Avx2(t) => verify_kernels(t, n),
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    Sel::Sse4_2(t) => verify_kernels(t, n),
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    Sel::Sse2(t) => verify_kernels(t, n),
                    #[cfg(target_arch = "aarch64")]
                    Sel::Neon(t) => verify_kernels(t, n),
                    Sel::Fallback(t) => verify_kernels(t, n),
                }
            }
        }
    }

    /// Issue #22 RT lane — every routed public kernel call must be
    /// allocation-free (fixture setup stays outside the measurement window).
    #[test]
    fn rt_dsp_kernels_no_alloc() {
        let tok = SimdToken::detect();
        let n = 1024;
        let a = vec![0.5f32; n];
        let b = vec![0.25f32; n];
        let taps = crate::simd_poc::fir_taps();
        let c = crate::simd_poc::BIQUAD_COEFFS;
        let mut dst = vec![0.0f32; n];
        let mut st2 = vec![0.0f32; 4];
        let sc = SineCoeffs::for_omega(0.1);
        let mut ss = SineState { y1: 0.0, y2: 0.0 };
        assert_no_alloc::assert_no_alloc(|| {
            add(tok, &mut dst, &a, &b);
            scale(tok, &mut dst, &a, 0.5);
            clamp_unit(tok, &mut dst, &a);
            fade(tok, &mut dst, &a, 0.0, 0.001);
            let _ = dot(tok, &a, &b);
            fir(tok, &mut dst, &a, &taps);
            biquad(tok, &mut dst, &a, &c, &mut st2, 2);
            sine(tok, &mut dst, &sc, &mut ss);
        });
    }
}
