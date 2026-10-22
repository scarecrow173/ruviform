//! `FirFilter` — streaming FIR convolution driven by
//! `ruviform_math::dsp::fir`, carrying `taps.len() - 1` samples of
//! history across `process` calls.

use ruviform_math::dsp::{SimdToken, fir};
use ruviform_media_core::{Error, Result};

/// Streaming FIR filter: `dst[n] = Σ_k taps[k]·x[n−k]` with the
/// previous call's tail samples supplying the `n−k < 0` region.
///
/// `prepare` allocates the taps copy plus two scratch buffers sized
/// `taps.len() - 1 + max_frames`; `process` then only moves data —
/// no allocation, no blocking.
///
/// **RT contract:** `process` is RT-suitable given `src.len() <=
/// max_frames` (the prepare-time bound; larger inputs are rejected
/// rather than reallocating).
pub struct FirFilter {
    simd: SimdToken,
    taps: Vec<f32>,
    /// `[delay ‖ src]` staging for the kernel call.
    in_scratch: Vec<f32>,
    /// Kernel output before slicing the `delay` prefix off.
    out_scratch: Vec<f32>,
    /// `taps.len() - 1`.
    delay: usize,
    max_frames: usize,
}

impl FirFilter {
    /// Prepares a stream: copies `taps` and allocates scratch for
    /// inputs up to `max_frames` samples per `process` call.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `taps` is empty or `max_frames`
    /// is zero.
    pub fn prepare(taps: &[f32], max_frames: usize) -> Result<Self> {
        if taps.is_empty() {
            return Err(Error::InvalidValue("taps"));
        }
        if max_frames == 0 {
            return Err(Error::InvalidValue("max_frames"));
        }
        let delay = taps.len() - 1;
        Ok(Self {
            simd: SimdToken::detect(),
            taps: taps.to_vec(),
            in_scratch: vec![0.0; delay + max_frames],
            out_scratch: vec![0.0; delay + max_frames],
            delay,
            max_frames,
        })
    }

    /// The FIR order — `taps.len() - 1` history samples carried
    /// between calls.
    #[must_use]
    pub fn taps(&self) -> &[f32] {
        &self.taps
    }

    /// Zeroes the carried history.
    ///
    /// **RT contract:** RT-suitable — bounded memset.
    pub fn reset(&mut self) {
        self.in_scratch[..self.delay].fill(0.0);
    }

    /// Convolves `src` into `dst` (`dst.len() == src.len() <=
    /// max_frames`). History from prior calls supplies samples
    /// before `src[0]`; after the call the last `delay` input
    /// samples become the new history.
    ///
    /// **RT contract:** RT-suitable — bounded copies plus the
    /// dispatched kernel; nothing allocates.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `src.len() != dst.len()` or the
    /// input exceeds the `prepare`-time `max_frames` bound.
    pub fn process(&mut self, dst: &mut [f32], src: &[f32]) -> Result<()> {
        if dst.len() != src.len() {
            return Err(Error::InvalidValue("dst"));
        }
        let frames = src.len();
        if frames > self.max_frames {
            return Err(Error::InvalidValue("frames"));
        }
        let total = self.delay + frames;
        self.in_scratch[self.delay..total].copy_from_slice(src);
        fir(
            self.simd,
            &mut self.out_scratch[..total],
            &self.in_scratch[..total],
            &self.taps,
        );
        dst.copy_from_slice(&self.out_scratch[self.delay..total]);
        // Carry: last `delay` samples of the staged input become the
        // next call's history.
        self.in_scratch.copy_within(total - self.delay..total, 0);
        Ok(())
    }
}
