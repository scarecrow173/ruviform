//! [`DemoWaveform`] — a deterministic synthetic [`WaveformSource`].
//!
//! Generates a two-tone signal on the fly inside `read` — no backing
//! store — which makes it a working demonstration that the source port
//! does not require pre-rendered data: frames exist only for the
//! ranges a view asks about. It backs the `viz_cli` example, the
//! benches, and integration tests that need large or gap-bearing
//! inputs without allocating them.

use std::ops::Range;

use crate::source::{PcmBlock, WaveformInfo, WaveformSource};
use crate::{Result, VizError};

/// Synthesizes `0.5·sin(2π·440·t + φ_c) + 0.3·sin(2π·997·t)` per
/// channel `c`, with an optional gap range reported as `NaN`.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug)]
pub struct DemoWaveform {
    info: WaveformInfo,
    gap: Option<Range<u64>>,
}

impl DemoWaveform {
    /// A gap-free source of `frames` frames.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `channels == 0` or `sample_rate == 0`.
    pub fn new(sample_rate: u32, channels: usize, frames: u64) -> Result<Self> {
        Self::with_gap(sample_rate, channels, frames, None)
    }

    /// A source reporting `NaN` inside `gap` (frames).
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `channels == 0` or `sample_rate == 0`.
    pub fn with_gap(
        sample_rate: u32,
        channels: usize,
        frames: u64,
        gap: Option<Range<u64>>,
    ) -> Result<Self> {
        if channels == 0 {
            return Err(VizError::Invalid("channels"));
        }
        if sample_rate == 0 {
            return Err(VizError::Invalid("sample_rate"));
        }
        Ok(Self {
            info: WaveformInfo {
                sample_rate,
                channels,
                frames,
            },
            gap,
        })
    }

    /// The synthesized value at `(frame, channel)`.
    #[expect(clippy::cast_precision_loss)]
    fn sample_at(&self, frame: u64, channel: usize) -> f64 {
        if self
            .gap
            .as_ref()
            .is_some_and(|g| g.start <= frame && frame < g.end)
        {
            return f64::NAN;
        }
        let t = frame as f64 / f64::from(self.info.sample_rate);
        let ph = channel as f64 * 0.5;
        0.5 * (2.0 * std::f64::consts::PI * 440.0 * t + ph).sin()
            + 0.3 * (2.0 * std::f64::consts::PI * 997.0 * t).sin()
    }
}

impl WaveformSource for DemoWaveform {
    fn info(&self) -> WaveformInfo {
        self.info
    }

    fn read(&self, start: u64, frames: usize) -> Result<PcmBlock> {
        let avail = self.info.frames.saturating_sub(start);
        let n = usize::try_from(avail).unwrap_or(usize::MAX).min(frames);
        let ch = self.info.channels;
        let mut samples = Vec::with_capacity(n * ch);
        for f in 0..n {
            let frame = start + f as u64;
            for c in 0..ch {
                samples.push(self.sample_at(frame, c));
            }
        }
        Ok(PcmBlock {
            start,
            frames: n,
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_deterministically() {
        let s = DemoWaveform::new(48_000, 2, 100).unwrap();
        let a = s.read(0, 50).unwrap();
        let b = s.read(0, 50).unwrap();
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.samples.len(), 100);
        assert!(a.samples.iter().all(|v| v.abs() <= 0.8 + 1e-9));
    }

    #[test]
    fn gap_reports_nan() {
        let s = DemoWaveform::with_gap(48_000, 1, 200, Some(50..60)).unwrap();
        let b = s.read(45, 20).unwrap();
        assert!(b.samples[4].is_finite()); // frame 49
        assert!(b.samples[5].is_nan()); // frame 50
        assert!(b.samples[14].is_nan()); // frame 59
        assert!(b.samples[15].is_finite()); // frame 60
    }
}
