//! Production dispatch columns: the routed `dsp` public API (Issue #53).
//!
//! Each column captures a [`SimdToken`] once via a `OnceLock` — mirroring the
//! production pattern of preparing the token at a block boundary and reusing
//! it for every call. `probe` reports constructibility of the forced level;
//! kernel fns early-return when the level is unavailable (skip convention).

use std::sync::OnceLock;

use ruviform_math::dsp::{self, SimdToken};
use ruviform_math::simd_poc::{BiquadCoeffs, SineCoeffs, SineState};

use super::KernelImpl;

macro_rules! prod_column {
    ($col:ident, $name:literal, $level:literal, $mod:ident, $init:expr) => {
        mod $mod {
            use super::*;

            pub static TOKEN: OnceLock<Option<SimdToken>> = OnceLock::new();

            pub fn get() -> Option<SimdToken> {
                *TOKEN.get_or_init(|| $init)
            }

            pub fn probe() -> bool {
                get().is_some()
            }

            pub fn add(d: &mut [f32], a: &[f32], b: &[f32]) {
                if let Some(t) = get() {
                    dsp::add(t, d, a, b);
                }
            }
            pub fn scale(d: &mut [f32], s: &[f32], g: f32) {
                if let Some(t) = get() {
                    dsp::scale(t, d, s, g);
                }
            }
            pub fn clamp_unit(d: &mut [f32], s: &[f32]) {
                if let Some(t) = get() {
                    dsp::clamp_unit(t, d, s);
                }
            }
            pub fn fade(d: &mut [f32], s: &[f32], g0: f32, st: f32) {
                if let Some(t) = get() {
                    dsp::fade(t, d, s, g0, st);
                }
            }
            pub fn dot(a: &[f32], b: &[f32]) -> f32 {
                get().map_or(0.0, |t| dsp::dot(t, a, b))
            }
            pub fn fir(d: &mut [f32], s: &[f32], taps: &[f32]) {
                if let Some(t) = get() {
                    dsp::fir(t, d, s, taps);
                }
            }
            pub fn biquad(d: &mut [f32], s: &[f32], c: &BiquadCoeffs, st: &mut [f32], ch: usize) {
                if let Some(t) = get() {
                    dsp::biquad(t, d, s, c, st, ch);
                }
            }
            pub fn sine(d: &mut [f32], c: &SineCoeffs, st: &mut SineState) {
                if let Some(t) = get() {
                    dsp::sine(t, d, c, st);
                }
            }
        }

        #[doc = concat!("`", $name, "` — routed production path at `", $level, "`.")]
        pub(crate) const $col: KernelImpl = KernelImpl {
            name: $name,
            backend: "prod",
            level: $level,
            probe: $mod::probe,
            add: $mod::add,
            scale: $mod::scale,
            clamp_unit: $mod::clamp_unit,
            fade: $mod::fade,
            dot: $mod::dot,
            fir: $mod::fir,
            biquad: $mod::biquad,
            sine: $mod::sine,
        };
    };
}

prod_column!(
    DETECTED,
    "prod_detected",
    "detected",
    detected,
    Some(SimdToken::detect())
);
prod_column!(
    FALLBACK,
    "prod_fallback",
    "fallback",
    fallback,
    Some(SimdToken::force_fallback())
);
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
prod_column!(SSE2, "prod_sse2", "sse2", sse2, SimdToken::force_sse2());
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
prod_column!(
    SSE4_2,
    "prod_sse4_2",
    "sse4_2",
    sse4_2,
    SimdToken::force_sse4_2()
);
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
prod_column!(AVX2, "prod_avx2", "avx2", avx2, SimdToken::force_avx2());
#[cfg(target_arch = "aarch64")]
prod_column!(NEON, "prod_neon", "neon", neon, SimdToken::force_neon());
