//! Input data-access ports: [`WaveformSource`] and [`SpectrumSource`].
//!
//! These are the "chunked/random-access source" ports the visualization
//! primitives pull from. Applications implement them over their own
//! storage (decoded PCM, analysis datasets, computed buffers); `viz`
//! reads only the ranges a view actually needs, so whole-dataset loads
//! are never required. In-memory reference implementations
//! ([`SliceWaveform`], [`VecSpectrum`]) cover tests, CLI use and small
//! buffers.
//!
//! Contract notes shared by both ports:
//!
//! - Sources are immutable once published; new data is published as a
//!   new source instance.
//! - `NaN`/non-finite values mark missing or invalid data. Consumers
//!   skip them in statistics and render gaps — they are never
//!   interpolated away.
//! - Failures report [`VizError::Source`] with a human-readable
//!   message (mirroring `ruviform-ui`'s `DataSource` error shape).

use std::ops::Range;

use crate::timeline::FrameGrid;
use crate::{Result, VizError};

/// Static layout of a [`WaveformSource`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformInfo {
    /// Samples per second (integer Hz).
    pub sample_rate: u32,
    /// Channel count (`>= 1`).
    pub channels: usize,
    /// Per-channel frame count.
    pub frames: u64,
}

/// One bounded PCM read: interleaved `f64` samples, frame-major.
///
/// `samples[frame_index * channels + channel]` — `frames * channels`
/// elements. Non-finite values mark missing samples.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmBlock {
    /// Index of the first frame covered.
    pub start: u64,
    /// Per-channel frames actually returned (short reads only at
    /// end-of-data).
    pub frames: usize,
    /// Interleaved samples, `frames * channels` elements.
    pub samples: Vec<f64>,
}

/// Chunked random-access PCM port.
///
/// `read(start, frames)` returns up to `frames` frames starting at
/// `start`; a short read is legal only at end-of-data, and
/// `start >= info().frames` must return `frames == 0` (never an error —
/// callers clamp viewports against `info().frames` and rely on empty
/// reads for out-of-range pans). `frames == 0` is a no-op.
///
/// Implementations should be cheap for bounded windows; both the LOD
/// builder and viewport queries issue sequential chunked reads.
///
/// **RT contract:** non-RT only.
pub trait WaveformSource: Send + Sync {
    /// Static source layout.
    fn info(&self) -> WaveformInfo;

    /// Reads up to `frames` frames starting at `start`.
    ///
    /// # Errors
    /// [`VizError::Source`] on backend failure.
    fn read(&self, start: u64, frames: usize) -> Result<PcmBlock>;
}

/// How bin indices map to frequency on a [`SpectrumSource`].
///
/// `#[non_exhaustive]` — additional axis kinds may be added.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum BinAxis {
    /// Uniform linear grid: `hz(k) = first_hz + k * step_hz` (the STFT
    /// bin grid).
    Uniform {
        /// Hz of bin 0 (usually `0.0` = DC).
        first_hz: f64,
        /// Hz spacing between consecutive bins (`sample_rate / fft_size`).
        step_hz: f64,
    },
    /// Explicit center frequency per bin — band aggregates (octave
    /// bands etc.) whose bins are not uniformly spaced.
    Centers(Vec<f64>),
}

impl BinAxis {
    /// Center frequency of `bin` (edges interpolate for uniform axes).
    ///
    /// Returns `f64::NAN` for out-of-range bins.
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn hz(&self, bin: usize) -> f64 {
        match self {
            Self::Uniform { first_hz, step_hz } => bin as f64 * step_hz + first_hz,
            Self::Centers(v) => v.get(bin).copied().unwrap_or(f64::NAN),
        }
    }
}

/// Static layout of a [`SpectrumSource`].
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumInfo {
    /// Frame (time-column) count.
    pub frames: u64,
    /// Frequency bins per frame.
    pub bins: usize,
    /// Timebase of the frame index (window/hop grid in samples).
    pub grid: FrameGrid,
    /// Bin → Hz mapping.
    pub axis: BinAxis,
    /// Sample rate of the underlying capture.
    pub sample_rate: u32,
}

/// Spectral values of a [`SpectrumTile`].
///
/// `#[non_exhaustive]` — additional layouts may be added.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumData {
    /// Complex bins as split re/im planes (the `ruviform-numeric` FFT
    /// convention). Both vectors are `frames * bins`, frame-major.
    /// This variant is what lets stored intermediate spectra be reused
    /// without re-analysis.
    Complex {
        /// Real parts.
        re: Vec<f64>,
        /// Imaginary parts.
        im: Vec<f64>,
    },
    /// Magnitude bins `|z|`, `frames * bins`, frame-major.
    Magnitude(Vec<f64>),
}

impl SpectrumData {
    /// Whether complex (phase-capable) data is present.
    #[must_use]
    pub fn is_complex(&self) -> bool {
        matches!(self, Self::Complex { .. })
    }

    /// Magnitudes `|z|`, `frames * bins`, frame-major.
    ///
    /// For [`SpectrumData::Complex`] this computes `sqrt(re²+im²)` per
    /// bin (allocates). `NaN` propagates.
    #[must_use]
    pub fn magnitudes(&self) -> Vec<f64> {
        match self {
            Self::Magnitude(v) => v.clone(),
            Self::Complex { re, im } => re.iter().zip(im).map(|(r, i)| r.hypot(*i)).collect(),
        }
    }
}

/// One bounded time×frequency read, frame-major.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumTile {
    /// First frame index covered.
    pub frame_start: u64,
    /// Frames covered (`data` rows).
    pub frames: usize,
    /// First bin index covered.
    pub bin_start: usize,
    /// Bins covered (`data` columns per frame).
    pub bins: usize,
    /// Values, `frames * bins` frame-major.
    pub data: SpectrumData,
}

/// Chunked random-access spectrum port.
///
/// `read_tile(frames, bins)` returns exactly the requested rectangle
/// clamped to [`SpectrumInfo`] (random-access stores have no meaningful
/// short read — requests clamped to empty return empty tiles).
///
/// **RT contract:** non-RT only.
pub trait SpectrumSource: Send + Sync {
    /// Static source layout.
    fn info(&self) -> SpectrumInfo;

    /// Reads a frame×bin rectangle (clamped to the declared shape).
    ///
    /// # Errors
    /// [`VizError::Source`] on backend failure.
    fn read_tile(&self, frames: Range<u64>, bins: Range<usize>) -> Result<SpectrumTile>;
}

/// `WaveformSource` over an in-memory interleaved `f64` buffer.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug)]
pub struct SliceWaveform {
    info: WaveformInfo,
    samples: Vec<f64>,
}

impl SliceWaveform {
    /// Wraps interleaved `samples` (`frames = samples.len() / channels`).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError::Invalid`] when `channels == 0`, `sample_rate == 0`,
    /// or the buffer length is not a multiple of `channels`.
    pub fn new(sample_rate: u32, channels: usize, samples: Vec<f64>) -> Result<Self> {
        if channels == 0 {
            return Err(VizError::Invalid("channels"));
        }
        if sample_rate == 0 {
            return Err(VizError::Invalid("sample_rate"));
        }
        if !samples.len().is_multiple_of(channels) {
            return Err(VizError::Invalid("samples"));
        }
        let frames =
            u64::try_from(samples.len() / channels).map_err(|_| VizError::Invalid("samples"))?;
        Ok(Self {
            info: WaveformInfo {
                sample_rate,
                channels,
                frames,
            },
            samples,
        })
    }
}

impl WaveformSource for SliceWaveform {
    fn info(&self) -> WaveformInfo {
        self.info
    }

    fn read(&self, start: u64, frames: usize) -> Result<PcmBlock> {
        let channels = self.info.channels;
        let avail = self.info.frames.saturating_sub(start);
        let n = usize::try_from(avail).unwrap_or(usize::MAX).min(frames);
        let begin = usize::try_from(start)
            .ok()
            .and_then(|s| s.checked_mul(channels))
            .ok_or(VizError::Invalid("start"))?;
        let end = begin + n * channels;
        Ok(PcmBlock {
            start,
            frames: n,
            samples: self.samples.get(begin..end).unwrap_or(&[]).to_vec(),
        })
    }
}

/// `SpectrumSource` over an in-memory frame-major store.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug)]
pub struct VecSpectrum {
    info: SpectrumInfo,
    data: SpectrumData,
}

impl VecSpectrum {
    /// Wraps `data`, which must hold `info.frames * info.bins` values.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on shape mismatches.
    pub fn new(info: SpectrumInfo, data: SpectrumData) -> Result<Self> {
        let len = match &data {
            SpectrumData::Magnitude(v) => v.len(),
            SpectrumData::Complex { re, im } => {
                if re.len() != im.len() {
                    return Err(VizError::Invalid("data"));
                }
                re.len()
            }
        };
        let expect = usize::try_from(info.frames)
            .ok()
            .and_then(|f| f.checked_mul(info.bins))
            .ok_or(VizError::Invalid("frames"))?;
        if len != expect {
            return Err(VizError::Invalid("data"));
        }
        if info.bins == 0 {
            return Err(VizError::Invalid("bins"));
        }
        Ok(Self { info, data })
    }
}

impl SpectrumSource for VecSpectrum {
    fn info(&self) -> SpectrumInfo {
        self.info.clone()
    }

    fn read_tile(&self, frames: Range<u64>, bins: Range<usize>) -> Result<SpectrumTile> {
        let f0 = frames.start.min(self.info.frames);
        let f1 = frames.end.min(self.info.frames).max(f0);
        let b0 = bins.start.min(self.info.bins);
        let b1 = bins.end.min(self.info.bins).max(b0);
        let nf = usize::try_from(f1 - f0).map_err(|_| VizError::Invalid("frames"))?;
        let nb = b1 - b0;
        let data = match &self.data {
            SpectrumData::Magnitude(v) => {
                SpectrumData::Magnitude(copy_rect(v, self.info.bins, f0, b0, nf, nb))
            }
            SpectrumData::Complex { re, im } => SpectrumData::Complex {
                re: copy_rect(re, self.info.bins, f0, b0, nf, nb),
                im: copy_rect(im, self.info.bins, f0, b0, nf, nb),
            },
        };
        Ok(SpectrumTile {
            frame_start: f0,
            frames: nf,
            bin_start: b0,
            bins: nb,
            data,
        })
    }
}

/// Copies a `rows × cols` rectangle out of a frame-major `frames × stride`
/// plane (`src[f * stride + b]`).
fn copy_rect(src: &[f64], stride: usize, f0: u64, b0: usize, nf: usize, nb: usize) -> Vec<f64> {
    let f0 = usize::try_from(f0).unwrap_or(usize::MAX);
    let mut out = Vec::with_capacity(nf * nb);
    for f in f0..f0.saturating_add(nf) {
        let row = f * stride + b0;
        out.extend_from_slice(src.get(row..row + nb).unwrap_or(&[]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_waveform_bounds_reads() {
        let src = SliceWaveform::new(48_000, 2, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert_eq!(src.info().frames, 3);
        let b = src.read(1, 10).unwrap();
        assert_eq!((b.start, b.frames), (1, 2));
        assert_eq!(b.samples, vec![2.0, 3.0, 4.0, 5.0]);
        assert_eq!(src.read(3, 4).unwrap().frames, 0);
        assert_eq!(src.read(0, 0).unwrap().frames, 0);
    }

    #[test]
    fn slice_waveform_validates_shape() {
        assert!(SliceWaveform::new(48_000, 0, vec![]).is_err());
        assert!(SliceWaveform::new(0, 1, vec![]).is_err());
        assert!(SliceWaveform::new(48_000, 2, vec![0.0, 1.0, 2.0]).is_err());
    }

    #[test]
    fn vec_spectrum_reads_subrect() {
        let info = SpectrumInfo {
            frames: 2,
            bins: 3,
            grid: FrameGrid::new(4, 2, 0).unwrap(),
            axis: BinAxis::Uniform {
                first_hz: 0.0,
                step_hz: 100.0,
            },
            sample_rate: 48_000,
        };
        // Frame-major [f][b]: f0=[0,1,2], f1=[3,4,5].
        let spec = VecSpectrum::new(
            info,
            SpectrumData::Magnitude(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]),
        )
        .unwrap();
        let t = spec.read_tile(0..2, 1..3).unwrap();
        assert_eq!((t.frames, t.bins, t.bin_start), (2, 2, 1));
        assert_eq!(t.data.magnitudes(), vec![1.0, 2.0, 4.0, 5.0]);
        // Clamped reads.
        let t = spec.read_tile(1..99, 0..1).unwrap();
        assert_eq!((t.frame_start, t.frames), (1, 1));
        let t = spec.read_tile(5..9, 0..3).unwrap();
        assert_eq!(t.frames, 0);
    }

    #[test]
    fn complex_tile_reports_magnitudes() {
        let d = SpectrumData::Complex {
            re: vec![3.0, 0.0],
            im: vec![4.0, 0.0],
        };
        assert!(d.is_complex());
        assert_eq!(d.magnitudes(), vec![5.0, 0.0]);
    }
}
