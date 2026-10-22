//! `BiquadFilter` — planar multi-channel IIR biquad driven by
//! `ruviform_math::dsp::biquad`, plus RBJ cookbook coefficient
//! designers (`BiquadDesign`).

use ruviform_audio_core::SampleRate;
use ruviform_math::dsp::{BiquadCoeffs, SimdToken, biquad};
use ruviform_media_core::{Error, Result};

/// RBJ Audio-EQ-Cookbook coefficient designer — returns kernel-ready
/// normalized [`BiquadCoeffs`] (`b*`/`a*` already divided by `a0`,
/// matching the DF-II transposed recurrence of
/// `ruviform_math::dsp::biquad`).
///
/// Design math runs in `f64`; the stored coefficients are `f32`.
///
/// **RT contract:** non-RT only — called at prepare/reconfigure time.
pub struct BiquadDesign;

impl BiquadDesign {
    /// Second-order Butterworth-style lowpass at `f0_hz` with quality
    /// `q` (must be finite, `0 < f0 < fs/2`, `q > 0`).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on out-of-range `f0_hz`/`q`.
    pub fn lowpass(sample_rate: SampleRate, f0_hz: f64, q: f64) -> Result<BiquadCoeffs> {
        let (w0, alpha) = Self::params(sample_rate, f0_hz, q)?;
        let cos = w0.cos();
        let a0 = 1.0 + alpha;
        Ok(Self::finish(
            a0,
            f64::midpoint(1.0, -cos),
            1.0 - cos,
            f64::midpoint(1.0, -cos),
            -2.0 * cos,
            1.0 - alpha,
        ))
    }

    /// Second-order highpass — mirrors [`BiquadDesign::lowpass`].
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on out-of-range `f0_hz`/`q`.
    pub fn highpass(sample_rate: SampleRate, f0_hz: f64, q: f64) -> Result<BiquadCoeffs> {
        let (w0, alpha) = Self::params(sample_rate, f0_hz, q)?;
        let cos = w0.cos();
        let a0 = 1.0 + alpha;
        Ok(Self::finish(
            a0,
            f64::midpoint(1.0, cos),
            -(1.0 + cos),
            f64::midpoint(1.0, cos),
            -2.0 * cos,
            1.0 - alpha,
        ))
    }

    /// Second-order peaking EQ at `f0_hz` with `gain_db` boost/cut.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] on out-of-range arguments.
    pub fn peaking(
        sample_rate: SampleRate,
        f0_hz: f64,
        q: f64,
        gain_db: f64,
    ) -> Result<BiquadCoeffs> {
        let (w0, alpha) = Self::params(sample_rate, f0_hz, q)?;
        if !gain_db.is_finite() {
            return Err(Error::InvalidValue("gain_db"));
        }
        let a = 10.0_f64.powf(gain_db / 40.0);
        let cos = w0.cos();
        let a0 = 1.0 + alpha / a;
        Ok(Self::finish(
            a0,
            1.0 + alpha * a,
            -2.0 * cos,
            1.0 - alpha * a,
            -2.0 * cos,
            1.0 - alpha / a,
        ))
    }

    fn params(sample_rate: SampleRate, f0_hz: f64, q: f64) -> Result<(f64, f64)> {
        let fs = f64::from(sample_rate.get());
        if !(f0_hz.is_finite() && f0_hz > 0.0 && f0_hz < fs / 2.0) {
            return Err(Error::InvalidValue("f0_hz"));
        }
        if !(q.is_finite() && q > 0.0) {
            return Err(Error::InvalidValue("q"));
        }
        let w0 = 2.0 * core::f64::consts::PI * f0_hz / fs;
        Ok((w0, w0.sin() / (2.0 * q)))
    }

    #[expect(clippy::cast_possible_truncation)]
    fn finish(a0: f64, b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> BiquadCoeffs {
        BiquadCoeffs {
            b0: (b0 / a0) as f32,
            b1: (b1 / a0) as f32,
            b2: (b2 / a0) as f32,
            a1: (a1 / a0) as f32,
            a2: (a2 / a0) as f32,
        }
    }
}

/// Planar multi-channel biquad: `prepare` allocates per-channel
/// DF-II-transposed state once; `process` runs the dispatched
/// `math::dsp` kernel over a planar buffer
/// (`[ch0 frames][ch1 frames]…`).
///
/// **RT contract:** `process` is RT-suitable — pre-allocated state
/// only, no allocation, no blocking. `prepare`/`set_coeffs` are
/// non-RT.
pub struct BiquadFilter {
    simd: SimdToken,
    coeffs: BiquadCoeffs,
    state: Vec<f32>,
    channels: usize,
}

impl BiquadFilter {
    /// Allocates zeroed state for `channels` channels of `coeffs`.
    ///
    /// **RT contract:** non-RT only — allocates.
    #[must_use]
    pub fn prepare(coeffs: BiquadCoeffs, channels: usize) -> Self {
        Self {
            simd: SimdToken::detect(),
            coeffs,
            state: vec![0.0; 2 * channels],
            channels,
        }
    }

    /// Replaces the coefficients (reconfigure; state is kept).
    ///
    /// **RT contract:** non-RT only by convention — pure write, but
    /// changing coefficients mid-stream is a host decision.
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    /// Zeroes the filter state (history flush).
    ///
    /// **RT contract:** RT-suitable — bounded memset, no allocation.
    pub fn reset(&mut self) {
        self.state.fill(0.0);
    }

    /// Filters planar `src` into `dst` (`[ch0][ch1]…`, frame count =
    /// `len / channels`).
    ///
    /// **RT contract:** RT-suitable — the kernel processes the
    /// `frames * channels` prefix common to both buffers; nothing
    /// allocates or blocks.
    pub fn process(&mut self, dst: &mut [f32], src: &[f32]) {
        biquad(
            self.simd,
            dst,
            src,
            &self.coeffs,
            &mut self.state,
            self.channels,
        );
    }
}
