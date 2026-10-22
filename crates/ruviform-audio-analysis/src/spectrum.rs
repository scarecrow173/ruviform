//! Spectrum analysis: windowed FFT magnitude over a mono `f32`
//! block ([`SpectrumAnalyzer`]).

use ruviform_dsp::{Fft, Window};
use ruviform_media_core::Result;

/// FFT magnitude analyzer: prepared with an FFT size and window,
/// then `process` rewrites the internal `n/2+1`-bin magnitude
/// snapshot every block.
///
/// Input longer than `fft_size` is truncated to the first `n`
/// samples; shorter input is zero-padded — block-boundary calls get
/// deterministic behavior instead of a length error. Magnitudes are
/// raw `|z|` — normalization/scaling is the consumer's concern.
///
/// **RT contract:** `process` is RT-suitable — it reuses the
/// `prepare`-time `re`/`im`/magnitude buffers; no allocation or
/// blocking. Construction is non-RT (allocation).
///
/// # Examples
///
/// ```
/// use ruviform_audio_analysis::SpectrumAnalyzer;
/// use ruviform_dsp::Window;
///
/// let mut analyzer = SpectrumAnalyzer::new(4, Window::Rectangular).unwrap();
/// analyzer.process(&[1.0, 1.0, 1.0, 1.0]);
/// assert!(analyzer.magnitudes()[0] > 3.9); // DC bin
/// ```
pub struct SpectrumAnalyzer {
    fft: Fft,
    coeffs: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    mags: Vec<f32>,
}

impl SpectrumAnalyzer {
    /// Prepares an analyzer for `fft_size`-point FFTs with `window`,
    /// allocating the coefficient, work and magnitude buffers.
    ///
    /// **RT contract:** non-RT only (allocation).
    ///
    /// # Errors
    ///
    /// `Error::InvalidValue` when `fft_size` is not a power of two
    /// ≥ 2 (the [`Fft`] contract).
    pub fn new(fft_size: usize, window: Window) -> Result<Self> {
        let fft = Fft::new(fft_size)?;
        let mut coeffs = vec![0.0; fft_size];
        window.fill(&mut coeffs);
        Ok(Self {
            fft,
            coeffs,
            re: vec![0.0; fft_size],
            im: vec![0.0; fft_size],
            mags: vec![0.0; fft_size / 2 + 1],
        })
    }

    /// The FFT size this analyzer was prepared for.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub fn fft_size(&self) -> usize {
        self.fft.len()
    }

    /// Processes one block: windowed FFT magnitudes into the
    /// preallocated `n/2+1` bin snapshot. `samples` longer than
    /// `fft_size` is truncated; shorter input is zero-padded.
    ///
    /// **RT contract:** RT-suitable — reuses `prepare`-time buffers;
    /// no allocation or blocking.
    pub fn process(&mut self, samples: &[f32]) {
        let n = self.fft.len();
        self.re.fill(0.0);
        self.im.fill(0.0);
        let take = samples.len().min(n);
        for (dst, &x) in self.re[..take].iter_mut().zip(&samples[..take]) {
            *dst = x;
        }
        for (x, &w) in self.re.iter_mut().zip(&self.coeffs) {
            *x *= w;
        }
        // Buffer sizes are tied to `fft.len()` at construction.
        let _ = self.fft.process(&mut self.re, &mut self.im, false);
        for (k, mag) in self.mags.iter_mut().enumerate() {
            *mag = self.re[k]
                .mul_add(self.re[k], self.im[k] * self.im[k])
                .sqrt();
        }
    }

    /// The latest block's `n/2+1` magnitude bins (unnormalized `|z|`).
    ///
    /// **RT contract:** RT-suitable — borrow, pure.
    #[must_use]
    pub fn magnitudes(&self) -> &[f32] {
        &self.mags
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;
    use ruviform_media_core::Error;

    use super::*;

    #[test]
    fn rejects_non_power_of_two_size() {
        for n in [0, 1, 3, 5, 6, 12] {
            assert!(matches!(
                SpectrumAnalyzer::new(n, Window::Rectangular),
                Err(Error::InvalidValue(_))
            ));
        }
    }

    #[test]
    fn dc_signal_concentrates_in_bin_zero() {
        let mut analyzer = SpectrumAnalyzer::new(4, Window::Rectangular).unwrap();
        analyzer.process(&[1.0, 1.0, 1.0, 1.0]);
        let mags = analyzer.magnitudes();
        assert_eq!(mags.len(), 3);
        assert!((mags[0] - 4.0).abs() < 1e-6);
        assert!(mags[1].abs() < 1e-6);
        assert!(mags[2].abs() < 1e-6);
    }

    #[test]
    fn short_input_zero_pads_and_long_input_truncates() {
        let mut analyzer = SpectrumAnalyzer::new(4, Window::Rectangular).unwrap();
        analyzer.process(&[1.0]);
        // [1,0,0,0] → every bin has |z| = 1.
        assert!(
            analyzer
                .magnitudes()
                .iter()
                .all(|&m| (m - 1.0).abs() < 1e-6)
        );
        analyzer.process(&[1.0, 1.0, 1.0, 1.0, 99.0, 99.0]);
        assert!((analyzer.magnitudes()[0] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn nyquist_tone_lands_in_last_bin() {
        let mut analyzer = SpectrumAnalyzer::new(4, Window::Rectangular).unwrap();
        analyzer.process(&[1.0, -1.0, 1.0, -1.0]);
        assert!((analyzer.magnitudes()[2] - 4.0).abs() < 1e-6);
        assert!(analyzer.magnitudes()[0].abs() < 1e-6);
    }

    #[test]
    fn rt_process_allocates_nothing() {
        let mut analyzer = SpectrumAnalyzer::new(8, Window::Hann).unwrap();
        let samples = [0.5_f32; 8];
        assert_no_alloc(|| analyzer.process(&samples));
    }
}
