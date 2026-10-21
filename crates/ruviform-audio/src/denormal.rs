//! Opt-in denormal flush node (Issue #54).
//!
//! Denormal inputs cause a measured performance cliff in recursive
//! kernels (`fir` 5.1–31.8x, `biquad` 1.6–4.3x — see the backend
//! adoption ADR, decision 5). FTZ/DAZ is formally UB in Rust, so the
//! engine layer provides software flushing instead: insert
//! [`DenormalFlush`] into the pull order after any node whose output
//! may carry denormals.

use ruviform_audio_core::AudioBufferViewMut;
use ruviform_rt::RtContext;

use crate::AudioNode;

/// Flushes denormal samples in `out` to ±0 (sign preserved).
///
/// Stateless unit node — `graph.push(Box::new(DenormalFlush))` places
/// it in the pull order wherever protection is needed (typically
/// after IIR/feedback sources). Normal values, zero, NaN, and Inf are
/// passed through unchanged; kernels keep accepting denormals
/// contractually — this node only removes the performance cliff.
///
/// **RT contract:** `pull` is RT-suitable — no allocation, no
/// blocking, branch-and-store per sample.
pub struct DenormalFlush;

impl AudioNode for DenormalFlush {
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
        for ch in 0..out.channels() {
            if let Some(plane) = out.channel_mut(ch) {
                for s in plane.iter_mut() {
                    if *s != 0.0 && s.is_finite() && s.abs() < f32::MIN_POSITIVE {
                        *s = 0.0f32.copysign(*s);
                    }
                }
            }
        }
    }
}
