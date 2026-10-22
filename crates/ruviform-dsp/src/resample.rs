//! `LinearResampler` — streaming linear-interpolation resampler.

use ruviform_media_core::{Error, Result};

/// Linear-interpolation resampler for a continuous stream.
///
/// `step` is the input-sample distance between consecutive outputs
/// (`input_rate / output_rate`): `step < 1` upsamples, `step > 1`
/// downsamples. `process` consumes a prefix of `input` and writes a
/// prefix of `output`, returning `(input_consumed, output_produced)`;
/// the fractional position and the boundary sample are carried into
/// the next call, so block-wise processing equals one-shot
/// processing of the concatenated stream.
///
/// Output position `pos` counts in input samples relative to the
/// current call's `input[0]`; production needs taps at `floor(pos)`
/// and `floor(pos) + 1`, so it stops when `floor(pos) + 1` would
/// index past `input` — the last sample is left unconsumed for the
/// next call.
///
/// **RT contract:** `process` is RT-suitable — state update only,
/// no allocation, no blocking.
pub struct LinearResampler {
    step: f64,
    pos: f64,
}

impl LinearResampler {
    /// Creates a resampler with the given `step` (finite, `> 0`).
    ///
    /// **RT contract:** non-RT only — setup.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on non-finite or non-positive `step`.
    pub fn new(step: f64) -> Result<Self> {
        if !(step.is_finite() && step > 0.0) {
            return Err(Error::InvalidValue("step"));
        }
        Ok(Self { step, pos: 0.0 })
    }

    /// Resets the fractional position to the stream head.
    ///
    /// **RT contract:** non-RT only by convention — pure write.
    pub fn reset(&mut self) {
        self.pos = 0.0;
    }

    /// Resamples `input` into `output`, returning
    /// `(input_consumed, output_produced)`. Samples at indices `>=
    /// input_consumed` must be resubmitted on the next call together
    /// with the following input.
    ///
    /// **RT contract:** RT-suitable — writes into caller buffers
    /// only, no allocation.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> (usize, usize) {
        let mut produced = 0;
        let mut pos = self.pos;
        while produced < output.len() {
            let i0 = pos.floor() as usize;
            // Need taps at input[i0] and input[i0 + 1].
            if i0 + 1 >= input.len() {
                break;
            }
            let t = (pos - i0 as f64) as f32;
            output[produced] = input[i0] + (input[i0 + 1] - input[i0]) * t;
            produced += 1;
            pos += self.step;
        }
        // Keep `input[len - 1]` (or `input[i0]`) as the next call's
        // x0 tap: never consume the last index.
        let consumed = (pos.floor() as usize).min(input.len().saturating_sub(1));
        self.pos = pos - consumed as f64;
        (consumed, produced)
    }
}
