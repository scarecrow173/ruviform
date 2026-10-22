//! Level metering: per-channel peak and RMS over one processed block
//! ([`LevelMeter`], [`ChannelLevel`]).

use ruviform_audio_core::PlanarBufferView;
use ruviform_media_core::{Error, Result};

/// Per-channel level measurement for one processed block: absolute
/// peak and root-mean-square amplitude.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ChannelLevel {
    /// `max |x|` over the block.
    pub peak: f32,
    /// `sqrt(Σx² / n)` over the block (`0` for an empty channel).
    pub rms: f32,
}

/// Per-channel level meter: prepared with a channel count, then
/// `process` rewrites the internal snapshot every block and
/// `levels()` exposes it.
///
/// Values describe the *latest* processed block — no ballistics or
/// hold decay lives here (deferred to the meter-UI layer).
///
/// **RT contract:** `process` is RT-suitable — it writes only into
/// the snapshot allocated by [`LevelMeter::new`]. Construction is
/// non-RT (allocation).
///
/// # Examples
///
/// ```
/// use ruviform_audio_analysis::LevelMeter;
/// use ruviform_audio_core::PlanarBufferView;
///
/// let left = [1.0_f32, -0.5];
/// let right = [0.25_f32, -0.25];
/// let channels: [&[f32]; 2] = [&left, &right];
/// let view = PlanarBufferView::new(&channels).unwrap();
/// let mut meter = LevelMeter::new(2).unwrap();
/// meter.process(&view).unwrap();
/// assert_eq!(meter.levels()[0].peak, 1.0);
/// ```
#[derive(Debug)]
pub struct LevelMeter {
    levels: Vec<ChannelLevel>,
}

impl LevelMeter {
    /// Prepares a meter for `channels` channels, allocating the
    /// internal snapshot.
    ///
    /// **RT contract:** non-RT only (allocation).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `channels` is `0`.
    pub fn new(channels: usize) -> Result<Self> {
        if channels == 0 {
            return Err(Error::InvalidValue("channels"));
        }
        Ok(Self {
            levels: vec![ChannelLevel::default(); channels],
        })
    }

    /// Number of channels this meter was prepared for.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(&self) -> usize {
        self.levels.len()
    }

    /// Processes one block: `peak = max |x|` and
    /// `rms = sqrt(Σx² / n)` per channel, written into the
    /// preallocated snapshot. An empty channel yields `0`/`0`.
    ///
    /// **RT contract:** RT-suitable — writes only into the
    /// preallocated snapshot; no allocation or blocking.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `view`'s channel count differs
    /// from the prepared count.
    pub fn process(&mut self, view: &PlanarBufferView<'_, f32>) -> Result<()> {
        if view.channels() != self.levels.len() {
            return Err(Error::InvalidValue("channels"));
        }
        for (i, level) in self.levels.iter_mut().enumerate() {
            let samples = view.channel(i).unwrap_or(&[]);
            let mut peak = 0.0_f32;
            let mut sum_sq = 0.0_f32;
            for &x in samples {
                let a = x.abs();
                if a > peak {
                    peak = a;
                }
                sum_sq += x * x;
            }
            let n = u32::try_from(samples.len()).unwrap_or(u32::MAX);
            level.peak = peak;
            level.rms = if n == 0 {
                0.0
            } else {
                #[expect(clippy::cast_precision_loss)]
                let mean = sum_sq / n as f32;
                mean.sqrt()
            };
        }
        Ok(())
    }

    /// The latest block's per-channel levels.
    ///
    /// **RT contract:** RT-suitable — `const`-eligible borrow, pure.
    #[must_use]
    pub fn levels(&self) -> &[ChannelLevel] {
        &self.levels
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::*;

    #[test]
    fn rejects_zero_channels() {
        assert!(matches!(
            LevelMeter::new(0),
            Err(Error::InvalidValue("channels"))
        ));
    }

    #[test]
    fn measures_peak_and_rms_per_channel() {
        let left = [1.0_f32, -0.5];
        let right = [0.25_f32, -0.25];
        let channels: [&[f32]; 2] = [&left, &right];
        let view = PlanarBufferView::new(&channels).unwrap();
        let mut meter = LevelMeter::new(2).unwrap();
        meter.process(&view).unwrap();
        let levels = meter.levels();
        assert_eq!(levels[0].peak, 1.0);
        assert!((levels[0].rms - (0.625_f32).sqrt()).abs() < 1e-7);
        assert_eq!(levels[1].peak, 0.25);
        assert!((levels[1].rms - 0.25).abs() < 1e-7);
    }

    #[test]
    fn empty_channel_yields_zero_levels() {
        let ch: [f32; 0] = [];
        let channels: [&[f32]; 1] = [&ch];
        let view = PlanarBufferView::new(&channels).unwrap();
        let mut meter = LevelMeter::new(1).unwrap();
        meter.process(&view).unwrap();
        assert_eq!(
            meter.levels()[0],
            ChannelLevel {
                peak: 0.0,
                rms: 0.0
            }
        );
    }

    #[test]
    fn rejects_channel_count_mismatch() {
        let ch = [0.0_f32; 4];
        let channels: [&[f32]; 1] = [&ch];
        let view = PlanarBufferView::new(&channels).unwrap();
        let mut meter = LevelMeter::new(2).unwrap();
        assert!(matches!(
            meter.process(&view),
            Err(Error::InvalidValue("channels"))
        ));
    }

    #[test]
    fn rt_process_allocates_nothing() {
        let ch = [0.5_f32; 8];
        let channels: [&[f32]; 1] = [&ch];
        let view = PlanarBufferView::new(&channels).unwrap();
        let mut meter = LevelMeter::new(1).unwrap();
        assert_no_alloc(|| meter.process(&view).unwrap());
    }
}
