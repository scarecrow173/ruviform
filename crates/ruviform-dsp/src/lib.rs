//! `ruviform-dsp` — typed DSP kernel layer above
//! `ruviform-math::dsp`.
//!
//! # Responsibility
//!
//! Audio-domain DSP kernels expressed over `ruviform-audio-core` /
//! `ruviform-media-core` types (`SampleRate`, `Result`), each
//! following the `prepare` (allocating, non-RT) / `process`
//! (pre-allocated, RT-suitable) separation:
//!
//! - [`Window`] — Hann/Hamming/Blackman coefficient generation.
//! - [`BiquadDesign`]/[`BiquadFilter`] — RBJ cookbook design plus
//!   planar multi-channel filtering on the dispatched
//!   `math::dsp::biquad` kernel.
//! - [`FirFilter`] — streaming FIR on `math::dsp::fir` with carried
//!   delay-line history.
//! - [`Fft`] — radix-2 complex FFT over planar re/im slices.
//! - [`LinearResampler`] — streaming linear-interpolation resampler.
//!
//! Low-level SIMD dispatch stays in `ruviform-math::dsp` (typed by
//! raw slices, caller-owned state); this crate owns domain
//! convenience — coefficient design, streaming state, scratch
//! buffers. No I/O, device access or graph execution lives here.
//!
//! # Permitted dependencies
//!
//! `ruviform-audio-core`, `ruviform-media-core` (domain types) and
//! `ruviform-math` (Primitives kernels). Same-tier foundations
//! (`ruviform-video-proc`, `ruviform-media-io`, …) must not be
//! depended on.
//!
//! # RT contract
//!
//! `process`-family methods are RT-suitable: they operate on
//! buffers and state allocated at `prepare` time and never allocate,
//! block, or perform I/O. `prepare`-family methods and coefficient
//! designers are non-RT only.

pub mod biquad;
pub mod fft;
pub mod fir;
pub mod resample;
pub mod window;

pub use biquad::{BiquadDesign, BiquadFilter};
pub use fft::Fft;
pub use fir::FirFilter;
pub use resample::LinearResampler;
pub use window::Window;

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;
    use ruviform_audio_core::SampleRate;

    use super::*;

    const SR: SampleRate = match SampleRate::new(48_000) {
        Ok(r) => r,
        Err(_) => unreachable!(),
    };

    #[test]
    fn window_coefficients_match_definitions() {
        let hann = Window::Hann.coeffs(5);
        assert!((hann[0] - 0.0).abs() < 1e-6);
        assert!((hann[1] - 0.5).abs() < 1e-6);
        assert!((hann[2] - 1.0).abs() < 1e-6);
        let rect = Window::Rectangular.coeffs(8);
        assert!(rect.iter().all(|&v| (v - 1.0).abs() < 1e-7));
    }

    #[test]
    fn biquad_design_rejects_out_of_range() {
        assert!(BiquadDesign::lowpass(SR, 24_001.0, 0.707).is_err());
        assert!(BiquadDesign::lowpass(SR, 1_000.0, 0.0).is_err());
        assert!(BiquadDesign::peaking(SR, 1_000.0, 1.0, f64::NAN).is_err());
        assert!(BiquadDesign::lowpass(SR, 1_000.0, 0.707).is_ok());
    }

    #[test]
    fn biquad_filter_passes_dc_through_lowpass() {
        // DC gain of a normalized RBJ lowpass is 1.
        let coeffs = BiquadDesign::lowpass(SR, 1_000.0, 0.707).unwrap();
        let mut f = BiquadFilter::prepare(coeffs, 1);
        let src = vec![1.0_f32; 256];
        let mut dst = vec![0.0_f32; 256];
        f.process(&mut dst, &src);
        // Settled tail ≈ DC input.
        assert!((dst[255] - 1.0).abs() < 0.05, "tail: {}", dst[255]);
        f.reset();
        let mut dst2 = vec![0.0_f32; 64];
        f.process(&mut dst2, &src[..64]);
        assert!((dst2[0] - coeffs.b0).abs() < 1e-6);
    }

    #[test]
    fn biquad_process_multi_channel() {
        let coeffs = BiquadDesign::highpass(SR, 1_000.0, 0.707).unwrap();
        let mut f = BiquadFilter::prepare(coeffs, 2);
        let mut src = vec![0.0_f32; 128];
        src[0] = 1.0;
        src[64] = 0.5; // ch1 impulse
        let mut dst = vec![0.0_f32; 128];
        f.process(&mut dst, &src);
        // ch1 impulse scaled by 0.5 → first output ≈ 0.5·b0.
        assert!((dst[64] - 0.5 * coeffs.b0).abs() < 1e-5);
    }

    #[test]
    fn fir_matches_kernel_semantics_with_history() {
        // taps [1, 2]: out[n] = x[n] + 2·x[n−1].
        let mut f = FirFilter::prepare(&[1.0, 2.0], 4).unwrap();
        let src1 = [1.0, 0.0, 0.0, 0.0];
        let mut dst1 = [0.0; 4];
        f.process(&mut dst1, &src1).unwrap();
        assert_eq!(dst1, [1.0, 2.0, 0.0, 0.0]);
        // Next block: history carries the last sample (0.0? last delay
        // = taps-1 = 1 sample = src1[3] = 0.0... carry tail).
        let src2 = [0.0, 0.0, 0.0, 0.0];
        let mut dst2 = [0.0; 4];
        f.process(&mut dst2, &src2).unwrap();
        assert_eq!(dst2, [0.0; 4]);
        // Impulse at block boundary: history = last sample of src3.
        let mut f2 = FirFilter::prepare(&[1.0, 2.0], 4).unwrap();
        let src3 = [0.0, 0.0, 0.0, 5.0];
        let mut dst3 = [0.0; 4];
        f2.process(&mut dst3, &src3).unwrap();
        let src4 = [0.0, 0.0, 0.0, 0.0];
        let mut dst4 = [0.0; 4];
        f2.process(&mut dst4, &src4).unwrap();
        assert_eq!(dst4[0], 10.0);
    }

    #[test]
    fn fir_rejects_overlength_and_mismatch() {
        let mut f = FirFilter::prepare(&[1.0, 1.0], 4).unwrap();
        let src5 = [0.0; 5];
        let mut dst5 = [0.0; 5];
        assert!(f.process(&mut dst5, &src5).is_err());
        let src4 = [0.0; 4];
        let mut dst3 = [0.0; 3];
        assert!(f.process(&mut dst3, &src4).is_err());
    }

    #[test]
    fn fft_roundtrips_impulse_and_constant() {
        let mut fft = Fft::new(8).unwrap();
        // Impulse → flat spectrum.
        let mut re = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut im = [0.0; 8];
        fft.process(&mut re, &mut im, false).unwrap();
        assert!(re.iter().all(|&v| (v - 1.0).abs() < 1e-5));
        assert!(im.iter().all(|&v| v.abs() < 1e-5));
        // Inverse roundtrip on an arbitrary signal.
        let mut re2 = [0.0, 1.0, 0.5, -0.25, 0.0, 0.0, 0.0, 0.75];
        let orig = re2;
        let mut im2 = [0.0; 8];
        fft.process(&mut re2, &mut im2, false).unwrap();
        fft.process(&mut re2, &mut im2, true).unwrap();
        for (a, b) in re2.iter().zip(orig.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
        assert!(im2.iter().all(|&v| v.abs() < 1e-5));
        assert!(Fft::new(6).is_err());
    }

    #[test]
    fn resampler_identity_and_downsample() {
        // step = 1 → identity over the producible region (the final
        // sample is kept as the next call's x0 tap).
        let mut r = LinearResampler::new(1.0).unwrap();
        let src = [0.0, 1.0, 2.0, 3.0, 4.0];
        let mut out = [0.0; 4];
        let (c, p) = r.process(&src, &mut out);
        assert_eq!((c, p), (4, 4));
        assert_eq!(out, src[..4]);
        // step = 2 → half rate.
        let mut r2 = LinearResampler::new(2.0).unwrap();
        let mut out2 = [0.0; 8];
        let (_c2, p2) = r2.process(&[0.0, 10.0, 20.0, 30.0], &mut out2);
        assert_eq!(p2, 2);
        assert!((out2[0] - 0.0).abs() < 1e-6);
        assert!((out2[1] - 20.0).abs() < 1e-6);
        assert!(LinearResampler::new(0.0).is_err());
        assert!(LinearResampler::new(f64::NAN).is_err());
    }

    #[test]
    fn resampler_block_processing_equals_one_shot() {
        #[expect(clippy::cast_possible_truncation)]
        let input: Vec<f32> = (0..16).map(|i| (f64::from(i) * 0.7).sin() as f32).collect();
        let mut one_shot = LinearResampler::new(0.5).unwrap();
        let mut full = vec![0.0; 64];
        let (_c, produced_full) = one_shot.process(&input, &mut full);
        let mut blocked = LinearResampler::new(0.5).unwrap();
        let mut acc = Vec::new();
        let mut base = 0usize;
        while base < input.len() {
            let end = (base + 5).min(input.len());
            let mut out = [0.0; 64];
            let (c, p) = blocked.process(&input[base..end], &mut out);
            acc.extend_from_slice(&out[..p]);
            // The final unconsumed sample cannot produce alone.
            base += if c == 0 { 1 } else { c };
        }
        assert_eq!(acc.len(), produced_full);
        for (a, b) in acc.iter().zip(full[..produced_full].iter()) {
            assert!((a - b).abs() < 1e-5, "{a} vs {b}");
        }
    }

    /// RT contract: every `process` path runs with zero allocations on
    /// prepared state.
    #[test]
    fn rt_process_paths_do_not_allocate() {
        let coeffs = BiquadDesign::lowpass(SR, 1_000.0, 0.707).unwrap();
        let mut biquad_f = BiquadFilter::prepare(coeffs, 2);
        let mut fir_f = FirFilter::prepare(&[0.5, 0.5], 8).unwrap();
        let mut fft = Fft::new(8).unwrap();
        let mut resampler = LinearResampler::new(1.0).unwrap();
        let mut win = [0.0_f32; 16];

        let src = vec![0.5_f32; 16];
        let mut dst = vec![0.0_f32; 16];
        let mut re = [0.0_f32; 8];
        let mut im = [0.0_f32; 8];
        let mut r_out = [0.0_f32; 16];

        assert_no_alloc(|| {
            biquad_f.process(&mut dst, &src);
            fir_f.process(&mut dst[..8], &src[..8]).unwrap();
            fft.process(&mut re, &mut im, false).unwrap();
            resampler.process(&src, &mut r_out);
            Window::Hann.fill(&mut win);
        });
        assert!(dst.iter().any(|&v| v != 0.0));
    }
}
