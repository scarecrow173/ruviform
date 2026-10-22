//! Waveform peak data for UI drawing: min/max decimation
//! ([`WaveformPeaks`], [`PeakPair`]).

use ruviform_media_core::{Error, Result};

/// One drawing bin: the minimum and maximum sample inside the bin's
/// input range.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PeakPair {
    /// Minimum sample value in the bin (`0` for empty input).
    pub min: f32,
    /// Maximum sample value in the bin (`0` for empty input).
    pub max: f32,
}

/// Waveform peak-data generator: decimates a sample region into
/// `bins` [`PeakPair`]s for UI drawing.
///
/// Bins cover the input with a ceiling division (`bin_len =
/// ceil(len / bins)`), so the last bin may be shorter. Empty input
/// yields all-zero pairs; `bins > len` yields one pair per sample
/// followed by zero pairs.
///
/// **RT contract:** non-RT only — `peaks` allocates its `Vec`
/// result. Drawing data is consumed on the non-RT side per the
/// baseline contract.
#[derive(Copy, Clone, Debug, Default)]
pub struct WaveformPeaks;

impl WaveformPeaks {
    /// Decimates `samples` into `bins` min/max pairs.
    ///
    /// **RT contract:** non-RT only (allocation).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `bins` is `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_analysis::WaveformPeaks;
    ///
    /// let peaks = WaveformPeaks::peaks(&[1.0, -2.0, 3.0, -4.0, 5.0, -6.0], 3).unwrap();
    /// assert_eq!(peaks[0].min, -2.0);
    /// assert_eq!(peaks[0].max, 1.0);
    /// ```
    pub fn peaks(samples: &[f32], bins: usize) -> Result<Vec<PeakPair>> {
        if bins == 0 {
            return Err(Error::InvalidValue("bins"));
        }
        let bin_len = samples.len().div_ceil(bins).max(1);
        Ok((0..bins)
            .map(|i| {
                let start = i * bin_len;
                let end = ((i + 1) * bin_len).min(samples.len());
                let mut pair = PeakPair::default();
                for &x in samples.get(start..end).unwrap_or(&[]) {
                    pair.min = pair.min.min(x);
                    pair.max = pair.max.max(x);
                }
                pair
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_bins() {
        assert!(matches!(
            WaveformPeaks::peaks(&[1.0], 0),
            Err(Error::InvalidValue("bins"))
        ));
    }

    #[test]
    fn decimates_into_min_max_pairs() {
        let peaks = WaveformPeaks::peaks(&[1.0, -2.0, 3.0, -4.0, 5.0, -6.0], 3).unwrap();
        assert_eq!(
            peaks,
            [
                PeakPair {
                    min: -2.0,
                    max: 1.0
                },
                PeakPair {
                    min: -4.0,
                    max: 3.0
                },
                PeakPair {
                    min: -6.0,
                    max: 5.0
                },
            ]
        );
    }

    #[test]
    fn empty_input_yields_zero_pairs() {
        let peaks = WaveformPeaks::peaks(&[], 4).unwrap();
        assert_eq!(peaks.len(), 4);
        assert!(peaks.iter().all(|p| *p == PeakPair::default()));
    }

    #[test]
    fn more_bins_than_samples_pads_with_zero_pairs() {
        let peaks = WaveformPeaks::peaks(&[0.5, -0.5], 4).unwrap();
        assert_eq!(
            peaks,
            [
                PeakPair { min: 0.0, max: 0.5 },
                PeakPair {
                    min: -0.5,
                    max: 0.0
                },
                PeakPair::default(),
                PeakPair::default(),
            ]
        );
    }

    #[test]
    fn uneven_tail_bin_is_shorter() {
        let peaks = WaveformPeaks::peaks(&[1.0, 2.0, 3.0], 2).unwrap();
        // Pairs always include the zero baseline: an all-positive bin
        // reports min 0, an all-negative bin reports max 0.
        assert_eq!(peaks[0], PeakPair { min: 0.0, max: 2.0 });
        assert_eq!(peaks[1], PeakPair { min: 0.0, max: 3.0 });
    }
}
