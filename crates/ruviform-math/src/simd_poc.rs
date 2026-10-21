//! Experimental scalar reference kernels for the DSP/SIMD comparison PoC
//! (Issue #19 — canonical design: `docs/designs/dsp-simd-poc.md`).
//!
//! This module is a temporary experiment bridge for the PoC harness
//! (`tests/simd_poc/` shared support and `benches/simd_poc.rs`, reachable via
//! `#[doc(hidden)] pub mod simd_poc`). It is **not** part of the crate's
//! public API contract: the PoC adoption ADR will decide whether any item is
//! promoted into a real API or removed, and production code must not depend
//! on items in this module.
//!
//! RT contract: the kernel functions operate only on caller-provided slices
//! and perform no allocation (the intended production call shape), but this
//! experimental module is **not** covered by the production RT contract
//! declared in the crate-level docs. In particular the deterministic
//! input-generation helpers allocate `Vec`s and exist for tests/benches only.
//!
//! Conventions shared by all kernels:
//!
//! - Slice-length contracts are enforced with `debug_assert`; an
//!   out-of-contract call is a caller bug, not a recoverable error.
//! - Scalar bodies are plain `std` loops: autovectorization is intentionally
//!   not suppressed (the SSE2-baseline autovec result *is* the baseline being
//!   measured), no `#[inline]` hints, no `unsafe`.
//! - Fused multiply-add is never used: every kernel rounds as a sequential
//!   multiply-then-add loop, which defines the reference rounding that
//!   backend kernels are compared against (per-kernel equivalence contract
//!   in `docs/designs/dsp-simd-poc.md` — "カーネルセットと等価性契約").
//! - No `pub enum` lives in this module (repository public-enum policy);
//!   kernel identifiers used by the harness are `&'static str`.

/// Number of taps of the `fir` reference kernel (`taps.len() == FIR_TAPS`).
pub const FIR_TAPS: usize = 64;

/// Fixed planar channel count used by the `biquad` equivalence matrix and
/// benches (`channels = 8`); frames vary while channels stay fixed.
pub const BIQUAD_CHANNELS: usize = 8;

/// Seed for the deterministic `fir_taps` fixture. Distinct from
/// [`Xorshift64Star::DEFAULT_SEED`] so fixture taps never share a sequence
/// with generated inputs.
const FIR_TAPS_SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// `biquad` coefficients: Direct Form II transposed difference equation
/// `y[n] = b0·x[n] + z1`, `z1' = b1·x[n] - a1·y[n] + z2`,
/// `z2' = b2·x[n] - a2·y[n]`.
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoeffs {
    /// Feed-forward coefficient applied to `x[n]`.
    pub b0: f32,
    /// Feed-forward coefficient applied to `x[n]` inside the `z1` recurrence.
    pub b1: f32,
    /// Feed-forward coefficient applied to `x[n]` inside the `z2` recurrence.
    pub b2: f32,
    /// Feedback coefficient applied to `y[n]` inside the `z1` recurrence.
    pub a1: f32,
    /// Feedback coefficient applied to `y[n]` inside the `z2` recurrence.
    pub a2: f32,
}

/// `sine` coefficients for `y[n] = c1·y[n-1] + c2·y[n-2]`
/// (`c1 = 2·cos(ω)`, `c2 = -1` for an undamped oscillator).
#[derive(Clone, Copy, Debug)]
pub struct SineCoeffs {
    /// Recurrence coefficient on `y[n-1]` (`2·cos(ω)`).
    pub c1: f32,
    /// Recurrence coefficient on `y[n-2]` (`-1` for undamped output).
    pub c2: f32,
}

/// `sine` recurrence state carried across calls (`y[n-1]`, `y[n-2]`).
#[derive(Clone, Copy, Debug)]
pub struct SineState {
    /// Previous output `y[n-1]`.
    pub y1: f32,
    /// Output two samples back `y[n-2]`.
    pub y2: f32,
}

impl SineCoeffs {
    /// Builds coefficients for an undamped oscillator at normalized angular
    /// frequency `omega` (`ω = 2π·f/f_s`, rad/sample).
    #[must_use]
    pub fn for_omega(omega: f32) -> Self {
        Self {
            c1: 2.0 * omega.cos(),
            c2: -1.0,
        }
    }
}

/// Deterministic `biquad` coefficients (stable gentle low-pass,
/// 2nd-order Butterworth at ≈`0.1·f_s`) shared by every matrix/bench column.
pub const BIQUAD_COEFFS: BiquadCoeffs = BiquadCoeffs {
    b0: 0.067_455_3,
    b1: 0.134_910_6,
    b2: 0.067_455_3,
    a1: -1.142_980_5,
    a2: 0.412_801_6,
};

/// Deterministic FIR coefficients shared by every equivalence-matrix /
/// bench column (uniform pseudo-random taps from a dedicated seed).
#[must_use]
pub fn fir_taps() -> [f32; FIR_TAPS] {
    let mut rng = Xorshift64Star::new(FIR_TAPS_SEED);
    let mut taps = [0.0; FIR_TAPS];
    rng.fill_uniform(&mut taps);
    taps
}

/// `dst[i] = a[i] + b[i]` — elementwise add.
///
/// Contract: `dst.len() == a.len() == b.len()`. Equivalence: bit-exact for
/// non-NaN input; NaN input propagates (`is_nan` match is the contract).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); release builds simply
/// stop at the shortest input length.
pub fn add(dst: &mut [f32], a: &[f32], b: &[f32]) {
    debug_assert_eq!(dst.len(), a.len());
    debug_assert_eq!(a.len(), b.len());
    for ((d, &x), &y) in dst.iter_mut().zip(a.iter()).zip(b.iter()) {
        *d = x + y;
    }
}

/// `dst[i] = src[i] * gain` — single-input elementwise scale.
///
/// Contract: `dst.len() == src.len()`. Equivalence: bit-exact for non-NaN
/// input; NaN input propagates (`is_nan` match).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); release builds stop
/// at the shortest input length.
pub fn scale(dst: &mut [f32], src: &[f32], gain: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, &x) in dst.iter_mut().zip(src.iter()) {
        *d = x * gain;
    }
}

/// `dst[i] = clamp(src[i], -1.0, 1.0)` via `f32::clamp` — the reference NaN
/// semantics (NaN input → NaN output, payload ignored).
///
/// Contract: `dst.len() == src.len()`. Equivalence: bit-exact for non-NaN
/// input; NaN input → NaN output (`is_nan` match).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); release builds stop
/// at the shortest input length.
pub fn clamp_unit(dst: &mut [f32], src: &[f32]) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, &x) in dst.iter_mut().zip(src.iter()) {
        *d = x.clamp(-1.0, 1.0);
    }
}

/// `dst[i] = src[i] * (g0 + i·step)` — elementwise ramp gain.
///
/// The gain term is evaluated per element as `g0 + (i as f32) * step`:
/// fused multiply-add must not be used (equivalence contract), and the
/// closed-form `(i)` evaluation — not incremental `g += step` accumulation —
/// defines the reference rounding.
///
/// Contract: `dst.len() == src.len()`. Equivalence: bit-exact for non-NaN
/// input; NaN input propagates (`is_nan` match).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); release builds stop
/// at the shortest input length.
#[expect(
    clippy::cast_precision_loss,
    clippy::suboptimal_flops,
    reason = "`i as f32` fits exactly for bench-scale lengths and `g0 + i*step` \
              must stay a separate multiply-then-add (the contract forbids FMA)"
)]
pub fn fade(dst: &mut [f32], src: &[f32], g0: f32, step: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (i, (d, &x)) in dst.iter_mut().zip(src.iter()).enumerate() {
        *d = x * (g0 + i as f32 * step);
    }
}

/// `Σ a[i]·b[i]` — sequential left-to-right accumulation defines the
/// reference rounding.
///
/// Contract: `a.len() == b.len()`; finite inputs (reduction order may change
/// rounding into `Inf`/`NaN`, so edge inputs are out of contract).
/// Equivalence: ulp ≤ 256 (finalized in the adoption ADR from the observed
/// 46-ulp max; reduction-order / FMA differences are expected and measured
/// — see `MAX_ULPS_DOT` in `tests/simd_poc/equiv.rs`).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); release builds
/// accumulate over the shorter input.
#[must_use]
#[expect(
    clippy::suboptimal_flops,
    reason = "sequential multiply-then-add accumulation is the reference rounding; \
              FMA on backends is a measured difference, not the reference"
)]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut acc = 0.0_f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        acc += x * y;
    }
    acc
}

/// `dst[n] = Σ_{k=0}^{FIR_TAPS-1} taps[k]·src[n−k]` — direct-form FIR.
///
/// Contract: `dst.len() == src.len()` and `taps.len() == FIR_TAPS`; samples
/// before `src[0]` are implicit zeros (zero history). Coefficients and input
/// are finite. Equivalence: ulp ≤ 8; `NaN`/`Inf` inside the tap window
/// propagates order-independently (`is_nan` match).
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); in release builds a
/// `dst` longer than `src` panics via slicing.
#[expect(
    clippy::suboptimal_flops,
    reason = "sequential multiply-then-add accumulation is the reference rounding; \
              FMA on backends is a measured difference, not the reference"
)]
pub fn fir(dst: &mut [f32], src: &[f32], taps: &[f32]) {
    debug_assert_eq!(dst.len(), src.len());
    debug_assert_eq!(taps.len(), FIR_TAPS);
    for (n, d) in dst.iter_mut().enumerate() {
        let mut acc = 0.0_f32;
        for (&t, &x) in taps.iter().zip(src[..=n].iter().rev()) {
            acc += t * x;
        }
        *d = acc;
    }
}

/// Planar `channels`-channel IIR biquad — Direct Form II transposed with
/// persistent state (`y[n] = b0·x[n] + z1`, `z1' = b1·x[n] - a1·y[n] + z2`,
/// `z2' = b2·x[n] - a2·y[n]`).
///
/// Layout: `src`/`dst` are planar `[ch0 frames][ch1 frames]…` with
/// `frames = src.len() / channels` per channel; `state` is `SoA`
/// `[z1×channels][z2×channels]` with `state.len() == 2 * channels`. Channels
/// are independent (lane = channel), frames sequential.
///
/// Contract: `dst.len() == src.len()`, `src.len() % channels == 0`,
/// `channels > 0`. Equivalence: ulp ≤ 8 initial bound (FMA differences
/// allowed); NaN/Inf input contaminates the recursive state, so `is_nan`
/// match is the contract there.
///
/// # Panics
///
/// `debug_assert` on contract violation (debug builds); in release builds
/// non-conforming lengths panic via channel-range slicing.
#[expect(
    clippy::suboptimal_flops,
    reason = "separate multiply/add/sub ops are the reference rounding; \
              FMA on backends is a measured difference covered by the ulp contract"
)]
pub fn biquad(dst: &mut [f32], src: &[f32], c: &BiquadCoeffs, state: &mut [f32], channels: usize) {
    debug_assert!(channels > 0);
    debug_assert_eq!(dst.len(), src.len());
    debug_assert_eq!(src.len() % channels, 0);
    debug_assert_eq!(state.len(), 2 * channels);
    let frames = src.len() / channels;
    let (z1s, z2s) = state.split_at_mut(channels);
    for (ch, (z1_slot, z2_slot)) in z1s.iter_mut().zip(z2s.iter_mut()).enumerate() {
        let mut z1 = *z1_slot;
        let mut z2 = *z2_slot;
        let src_ch = &src[ch * frames..(ch + 1) * frames];
        let dst_ch = &mut dst[ch * frames..(ch + 1) * frames];
        for (d, &x) in dst_ch.iter_mut().zip(src_ch.iter()) {
            let y = c.b0 * x + z1;
            z1 = c.b1 * x - c.a1 * y + z2;
            z2 = c.b2 * x - c.a2 * y;
            *d = y;
        }
        *z1_slot = z1;
        *z2_slot = z2;
    }
}

/// `y[n] = c1·y[n-1] + c2·y[n-2]` — 2nd-order recursive oscillator.
///
/// Pure serial recurrence: SIMD cannot break the loop dependency, so every
/// backend is expected to run this same sequential loop via dispatch and the
/// contract is bit-exact (a difference would mean a dispatch-layer codegen
/// difference, recorded as a finding).
///
/// `state` is caller-owned and carried across calls (no allocation).
#[expect(
    clippy::suboptimal_flops,
    reason = "sequential multiply-then-add recurrence is the reference rounding; \
              the contract is bit-exact so the scalar form must stay unfused"
)]
pub fn sine(dst: &mut [f32], c: &SineCoeffs, state: &mut SineState) {
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

/// `xorshift64*` PRNG — deterministic input generation for the `PoC`
/// (reproducibility only; not cryptographic).
#[derive(Clone, Copy, Debug)]
pub struct Xorshift64Star {
    state: u64,
}

impl Xorshift64Star {
    /// Default fixed seed for shared input fixtures (`normal` input set).
    pub const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// Creates a generator from `seed`, which must be nonzero (state `0` is
    /// a `xorshift` fixed point that yields only zeros).
    ///
    /// # Panics
    ///
    /// `debug_assert` on `seed == 0` (debug builds).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        debug_assert_ne!(seed, 0);
        Self { state: seed }
    }

    /// Advances the state and returns the next `u64` (`xorshift64*`
    /// shifts `>>12`, `<<25`, `>>27` then the `0x2545F4914F6CDD1D`
    /// multiplicative scrambler).
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Next value as an `f32` uniformly distributed in `[-1.0, 1.0)` — the
    /// top 24 bits scaled by `2^-23`, then shifted by `-1.0`.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the shifted value occupies 24 bits, so the u64→f32 cast is exact"
    )]
    pub fn next_f32(&mut self) -> f32 {
        let v = (self.next_u64() >> 40) as f32;
        v * (1.0 / 8_388_608.0) - 1.0
    }

    /// Fills `dst` with uniform `[-1.0, 1.0)` values.
    pub fn fill_uniform(&mut self, dst: &mut [f32]) {
        for d in dst.iter_mut() {
            *d = self.next_f32();
        }
    }
}

/// Allocates a `len`-element buffer of deterministic uniform `[-1.0, 1.0)`
/// values from `seed` — the `normal` input set (test/bench helper;
/// allocates, not RT-suitable).
///
/// # Panics
///
/// `debug_assert` on `seed == 0` (debug builds).
#[must_use]
pub fn uniform_f32(len: usize, seed: u64) -> Vec<f32> {
    let mut rng = Xorshift64Star::new(seed);
    let mut v = vec![0.0; len];
    rng.fill_uniform(&mut v);
    v
}

/// Canonical denormal fixture value `2^-127` (≈ 5.9e-39).
///
/// Built via `f32::from_bits` on purpose: scaling `f32::MIN_POSITIVE` by a
/// small factor such as `1e-8` rounds to `+0.0`, so the bits are set
/// directly (`f32::MIN_POSITIVE * 0.5` yields this same value).
#[must_use]
pub fn denormal_f32() -> f32 {
    f32::from_bits(0x0040_0000)
}

/// Allocates a `len`-element uniform buffer where every `period`-th element
/// is replaced by a denormal value (sign alternates between replacements) —
/// the `denormal` input variant used to observe the denormal performance
/// cliff (test/bench helper; allocates).
///
/// # Panics
///
/// `debug_assert` on `seed == 0` (debug builds). `period == 0` panics
/// unconditionally via integer division.
#[must_use]
pub fn denormal_mix(len: usize, seed: u64, period: usize) -> Vec<f32> {
    debug_assert!(period > 0);
    let mut v = uniform_f32(len, seed);
    let d = denormal_f32();
    for (i, x) in v.iter_mut().enumerate() {
        if i % period == 0 {
            *x = if (i / period) % 2 == 0 { d } else { -d };
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert_eq!(a.to_bits(), e.to_bits(), "mismatch at index {i}");
        }
    }

    #[test]
    fn add_produces_elementwise_sum() {
        let a = [1.0, -2.0, 0.5, -0.0];
        let b = [0.5, 2.0, -0.25, 0.0];
        let mut dst = [0.0; 4];
        add(&mut dst, &a, &b);
        // -0.0 + 0.0 == +0.0 (IEEE) — the expected pattern is bit-exact.
        assert_bits_eq(&dst, &[1.5, 0.0, 0.25, 0.0]);
    }

    #[test]
    fn scale_multiplies_by_gain() {
        let mut dst = [0.0; 3];
        scale(&mut dst, &[2.0, -4.0, 0.125], 0.5);
        assert_bits_eq(&dst, &[1.0, -2.0, 0.0625]);
    }

    #[test]
    fn clamp_unit_bounds_and_propagates_nan() {
        let src = [-2.0, -1.0, 0.25, 1.0, 7.0, f32::NAN];
        let mut dst = [0.0; 6];
        clamp_unit(&mut dst, &src);
        assert_bits_eq(&dst[..5], &[-1.0, -1.0, 0.25, 1.0, 1.0]);
        assert!(dst[5].is_nan());
    }

    #[test]
    fn fade_applies_index_ramp() {
        let src = [1.0; 4];
        let mut dst = [0.0; 4];
        fade(&mut dst, &src, 0.25, 0.25);
        assert_bits_eq(&dst, &[0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn dot_accumulates_products_in_order() {
        let result = dot(&[1.0, 2.0, 3.0], &[4.0, -5.0, 0.5]);
        assert_eq!(result.to_bits(), (-4.5_f32).to_bits());
    }

    #[test]
    fn fir_convolves_with_zero_history() {
        // dst[n] = 0.5·src[n] + 0.25·src[n-1], src[<0] = 0.
        let mut taps = [0.0; FIR_TAPS];
        taps[0] = 0.5;
        taps[1] = 0.25;
        let mut dst = [0.0; 3];
        fir(&mut dst, &[1.0, 1.0, 0.0], &taps);
        assert_bits_eq(&dst, &[0.5, 0.75, 0.25]);
    }

    #[test]
    fn fir_impulse_tap_copies_src() {
        let mut taps = [0.0; FIR_TAPS];
        taps[0] = 1.0;
        let src = [0.3, -0.2, 0.7, 1.0];
        let mut dst = [0.0; 4];
        fir(&mut dst, &src, &taps);
        assert_bits_eq(&dst, &src);
    }

    #[test]
    fn biquad_runs_direct_form_two_recurrence() {
        // b0=0.5, b1=0.25 (rest 0): y[n] = 0.5·x[n] + z1, z1' = 0.25·x[n].
        let c = BiquadCoeffs {
            b0: 0.5,
            b1: 0.25,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        let mut state = [0.0; 2];
        let mut dst = [0.0; 3];
        biquad(&mut dst, &[1.0, 1.0, 1.0], &c, &mut state, 1);
        assert_bits_eq(&dst, &[0.5, 0.75, 0.75]);
        assert_bits_eq(&state, &[0.25, 0.0]);
    }

    #[test]
    fn biquad_processes_planar_channels_independently() {
        // b0=1 only: y[n] = x[n] + z1, z1' = z2 — so the first frame adds the
        // seeded z1 and the recurrence then dies out.
        let c = BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        // SoA state layout [z1 ch0, z1 ch1 | z2 ch0, z2 ch1].
        let mut state = [0.5, -0.5, 0.0, 0.0];
        let src = [1.0, 1.0, 2.0, 2.0]; // [ch0: 2 frames][ch1: 2 frames]
        let mut dst = [0.0; 4];
        biquad(&mut dst, &src, &c, &mut state, 2);
        assert_bits_eq(&dst, &[1.5, 1.0, 1.5, 2.0]);
        assert_bits_eq(&state, &[0.0; 4]);
    }

    #[test]
    fn sine_recurrence_updates_output_and_state() {
        let c = SineCoeffs { c1: 1.0, c2: -1.0 };
        let mut st = SineState { y1: 1.0, y2: 0.5 };
        let mut dst = [0.0; 3];
        sine(&mut dst, &c, &mut st);
        assert_bits_eq(&dst, &[0.5, -0.5, -1.0]);
        assert_eq!(st.y1.to_bits(), (-1.0_f32).to_bits());
        assert_eq!(st.y2.to_bits(), (-0.5_f32).to_bits());
    }

    #[test]
    fn sine_coeffs_for_omega_builds_undamped_recurrence() {
        let c = SineCoeffs::for_omega(0.0);
        assert_eq!(c.c1.to_bits(), 2.0_f32.to_bits());
        assert_eq!(c.c2.to_bits(), (-1.0_f32).to_bits());
    }

    #[test]
    fn xorshift64_star_is_deterministic() {
        let mut a = Xorshift64Star::new(Xorshift64Star::DEFAULT_SEED);
        let mut b = Xorshift64Star::new(Xorshift64Star::DEFAULT_SEED);
        let mut c = Xorshift64Star::new(0xDEAD_BEEF);
        let mut differ = false;
        for _ in 0..16 {
            let va = a.next_u64();
            assert_eq!(va, b.next_u64());
            differ |= va != c.next_u64();
        }
        assert!(differ);
    }

    #[test]
    fn uniform_f32_is_deterministic_and_in_range() {
        let v1 = uniform_f32(64, 7);
        let v2 = uniform_f32(64, 7);
        assert_bits_eq(&v1, &v2);
        assert!(v1.iter().all(|&x| (-1.0..1.0).contains(&x)));
    }

    #[test]
    fn denormal_f32_is_subnormal() {
        let d = denormal_f32();
        assert!(d.is_subnormal());
        assert_eq!(d.to_bits(), 0x0040_0000);
        // f32::MIN_POSITIVE * 0.5 is the same denormal (2^-127); smaller
        // factors would round to +0.0.
        assert_eq!(d.to_bits(), (f32::MIN_POSITIVE * 0.5).to_bits());
        assert_eq!((f32::MIN_POSITIVE * 1e-8).to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn denormal_mix_replaces_periodically() {
        let v = denormal_mix(8, 3, 3);
        let d = denormal_f32();
        // i % 3 == 0 → replaced; sign alternates per replacement (i/period).
        assert_eq!(v[0].to_bits(), d.to_bits());
        assert!(v[0].is_subnormal());
        assert!(!v[1].is_subnormal());
        assert!(!v[2].is_subnormal());
        assert_eq!(v[3].to_bits(), (-d).to_bits());
        assert!(v[3].is_subnormal());
        assert!(!v[4].is_subnormal());
        assert!(!v[5].is_subnormal());
        assert_eq!(v[6].to_bits(), d.to_bits());
        // Non-replaced elements still match the plain uniform sequence.
        let plain = uniform_f32(8, 3);
        assert_eq!(v[1].to_bits(), plain[1].to_bits());
    }

    #[test]
    fn fir_taps_fixture_is_deterministic() {
        let t1 = fir_taps();
        let t2 = fir_taps();
        assert_bits_eq(&t1, &t2);
        assert!(t1.iter().all(|&x| (-1.0..1.0).contains(&x)));
    }
}
