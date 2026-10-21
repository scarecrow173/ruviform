//! Planar-f32 summing mixer — the terminal accumulation node of the
//! output path (baseline: "ミキサー: 全入力を out に和算 (planar f32)").

use ruviform_audio_core::AudioBufferViewMut;
use ruviform_rt::RtContext;

use crate::graph::AudioNode;

/// Accumulates a fixed set of `f32` input buffers into `out`.
///
/// Inputs are raw planar planes delivered per block — the sources are
/// owned/prepared by the caller (e.g. rendered upstream of the mixer).
/// RT-safe: pure adds over pre-borrowed slices, no allocation.
///
/// **RT contract:** `pull` is RT-suitable — no allocation, no blocking.
pub struct Mixer {
    /// `(channel-major)` input planes, indexed `input × channel`.
    /// Set per block by the caller before `pull` — borrowed slices only.
    inputs: Vec<Vec<f32>>,
}

impl Mixer {
    /// Creates a mixer over `n_inputs` sources, each `channels × frames`
    /// planar. Buffers are allocated once here (non-RT call).
    #[must_use]
    pub fn new(n_inputs: usize, channels: usize, frames: usize) -> Self {
        Self {
            inputs: vec![vec![0.0; channels * frames]; n_inputs],
        }
    }

    /// Mutable access to input plane `i` for the upstream renderer to
    /// fill. Non-RT caller-side setup.
    #[must_use]
    pub fn input_mut(&mut self, i: usize) -> Option<&mut [f32]> {
        self.inputs.get_mut(i).map(Vec::as_mut_slice)
    }

    /// Zeroes every input plane (call once per block before rendering).
    pub fn clear_inputs(&mut self) {
        for input in &mut self.inputs {
            input.fill(0.0);
        }
    }
}

impl AudioNode for Mixer {
    /// Adds all input planes into `out` (channel-major, wrapping
    /// `frames × channels` into the buffer's planar view). Mismatched
    /// lengths are truncated to the common range — never panics.
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
        let frames = out.frames();
        let channels = out.channels();
        for input in &self.inputs {
            for ch in 0..channels.min(input.len() / frames.max(1)) {
                let src = &input[ch * frames..ch * frames + frames];
                if let Some(dst) = out.channel_mut(ch) {
                    for (d, s) in dst.iter_mut().zip(src.iter()) {
                        *d += *s;
                    }
                }
            }
        }
    }
}
