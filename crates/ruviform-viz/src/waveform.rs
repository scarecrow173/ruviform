//! Waveform LOD pyramid and viewport-driven decimation.
//!
//! [`WaveformLod`] is a precomputed pyramid of [`LodBucket`]s —
//! `min`/`max`/`rms`/`count` per bucket — built from chunked
//! [`WaveformSource`] reads (the full PCM is never held at once).
//! [`WaveformSeries::view`] serves a `[start, end)` range decimated to
//! `bins` drawing bins: at coarse resolutions it merges pyramid
//! buckets, and at extreme zoom-in (`samples_per_bin < base_bucket`)
//! it falls through to bounded direct source reads. Without a pyramid
//! ([`WaveformSeries::on_demand`]) every view reduces the source
//! directly — the "on-demand" half of the Issue #157 contract.
//!
//! Precision: buckets keep `f64` statistics and an exact
//! sum-of-squares so coarser levels never re-approximate RMS. `NaN`
//! marks missing samples (skipped, tracked by `count`) and empty
//! bins/buckets ([`LodBucket::EMPTY`]).

use std::sync::Arc;

use ruviform_exec::CancellationToken;

use crate::source::{WaveformInfo, WaveformSource};
use crate::{Result, VizError};

/// One LOD bucket: statistics over a covered sample range.
///
/// `count` is the number of *finite* samples aggregated; an
/// all-missing range yields [`LodBucket::EMPTY`]. `rms` is exact over
/// the covered samples — coarser levels propagate the sum-of-squares
/// internally instead of re-approximating.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LodBucket {
    /// Minimum finite sample (`NaN` when empty).
    pub min: f64,
    /// Maximum finite sample (`NaN` when empty).
    pub max: f64,
    /// Root-mean-square over finite samples (`NaN` when empty).
    pub rms: f64,
    /// Finite samples aggregated (`0` = nothing covered / all missing).
    pub count: u64,
}

impl LodBucket {
    /// Empty bucket: `NaN` statistics, zero count.
    pub const EMPTY: Self = Self {
        min: f64::NAN,
        max: f64::NAN,
        rms: f64::NAN,
        count: 0,
    };

    /// `max(|min|, |max|)` — `0.0` for an empty bucket.
    #[must_use]
    pub fn peak(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.min.abs().max(self.max.abs())
        }
    }

    /// Fraction of `covered` samples that were finite (`0..=1`).
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn coverage(&self, covered: u64) -> f64 {
        if covered == 0 {
            0.0
        } else {
            self.count as f64 / covered as f64
        }
    }
}

/// Internal accumulator carrying the exact sum-of-squares that lets
/// levels merge without re-approximating RMS.
#[derive(Copy, Clone, Default)]
struct Acc {
    min: f64,
    max: f64,
    sum_sq: f64,
    count: u64,
}

impl Acc {
    fn push(&mut self, v: f64) {
        if !v.is_finite() {
            return;
        }
        if self.count == 0 {
            self.min = v;
            self.max = v;
        } else {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
        self.sum_sq += v * v;
        self.count += 1;
    }

    fn merge(&mut self, o: &Self) {
        if o.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = *o;
            return;
        }
        self.min = self.min.min(o.min);
        self.max = self.max.max(o.max);
        self.sum_sq += o.sum_sq;
        self.count += o.count;
    }

    #[expect(clippy::cast_precision_loss)]
    fn bucket(&self) -> LodBucket {
        if self.count == 0 {
            LodBucket::EMPTY
        } else {
            LodBucket {
                min: self.min,
                max: self.max,
                rms: (self.sum_sq / self.count as f64).sqrt(),
                count: self.count,
            }
        }
    }

    /// Rebuilds the accumulator from a stored bucket — `sum_sq` is
    /// recovered as `rms² · count` so merged statistics stay exact to
    /// the stored precision.
    #[expect(clippy::cast_precision_loss)]
    fn from_bucket(b: &LodBucket) -> Self {
        if b.count == 0 {
            Self::default()
        } else {
            Self {
                min: b.min,
                max: b.max,
                sum_sq: b.rms * b.rms * b.count as f64,
                count: b.count,
            }
        }
    }
}

/// One pyramid level: `base_bucket << level` samples per bucket.
#[derive(Clone, Debug, PartialEq)]
pub struct LodLevel {
    /// Samples aggregated per bucket at this level.
    pub samples_per_bucket: u64,
    /// `channels[c]` = bucket series for channel `c`.
    pub channels: Vec<Vec<LodBucket>>,
}

/// Build parameters for [`WaveformLod`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LodConfig {
    /// Samples per bucket at the finest level (`>= 1`).
    pub base_bucket: u64,
    /// Number of levels; each level is 2× coarser (`>= 1`).
    pub levels: usize,
    /// Frames read per `WaveformSource::read` call during the build.
    pub read_chunk: u64,
}

impl Default for LodConfig {
    /// `256` base samples/bucket, `10` levels (coarsest bucket ≈ 131k
    /// samples — covers day-scale inputs), `1<<16` frames per read.
    fn default() -> Self {
        Self {
            base_bucket: 256,
            levels: 10,
            read_chunk: 1 << 16,
        }
    }
}

impl LodConfig {
    /// Validates the parameters.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `base_bucket == 0`, `levels == 0` or
    /// `read_chunk == 0`.
    pub fn validate(&self) -> Result<()> {
        if self.base_bucket == 0 {
            return Err(VizError::Invalid("base_bucket"));
        }
        if self.levels == 0 {
            return Err(VizError::Invalid("levels"));
        }
        if self.read_chunk == 0 {
            return Err(VizError::Invalid("read_chunk"));
        }
        Ok(())
    }

    /// A config whose coarsest level keeps at most `top_target` buckets
    /// for `frames` — level count grows with input length instead of
    /// being fixed.
    #[must_use]
    pub fn fit(frames: u64, top_target: u64) -> Self {
        let mut cfg = Self::default();
        let target = top_target.max(1);
        while cfg.levels < 62 && frames.div_ceil(cfg.base_bucket << (cfg.levels - 1)) > target {
            cfg.levels += 1;
        }
        cfg
    }
}

/// Precomputed waveform LOD pyramid.
///
/// `levels[0]` aggregates `base_bucket` samples per bucket; each
/// further level merges adjacent pairs. Built by streaming the source
/// in `read_chunk` blocks — memory stays proportional to the pyramid,
/// never the input.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug)]
pub struct WaveformLod {
    info: WaveformInfo,
    levels: Vec<LodLevel>,
}

impl WaveformLod {
    /// Streams `src` into the pyramid. `cancel` is observed at every
    /// read boundary (`Err(Cancelled)` aborts); `progress(done, total)`
    /// reports completed reads when given.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError`] on invalid config/source reads, or cancellation.
    #[expect(clippy::cast_possible_truncation)]
    pub fn build(
        src: &dyn WaveformSource,
        cfg: &LodConfig,
        cancel: Option<&CancellationToken>,
        progress: Option<&dyn Fn(u64, u64)>,
    ) -> Result<Self> {
        cfg.validate()?;
        let info = src.info();
        if info.channels == 0 {
            return Err(VizError::Invalid("channels"));
        }
        let base = cfg.base_bucket;
        let n_base =
            usize::try_from(info.frames.div_ceil(base)).map_err(|_| VizError::Invalid("frames"))?;
        let mut accs: Vec<Vec<Acc>> = vec![vec![Acc::default(); n_base]; info.channels];

        let mut start = 0u64;
        let total_reads = info.frames.div_ceil(cfg.read_chunk).max(1);
        let mut done = 0u64;
        while start < info.frames {
            if cancel.is_some_and(CancellationToken::is_cancelled) {
                return Err(VizError::Cancelled);
            }
            let want =
                usize::try_from((info.frames - start).min(cfg.read_chunk)).unwrap_or(usize::MAX);
            let block = src.read(start, want)?;
            let ch = info.channels;
            if block.samples.len() < block.frames * ch {
                return Err(VizError::Source(format!(
                    "short block: {} samples for {} frames x {ch} ch",
                    block.samples.len(),
                    block.frames
                )));
            }
            for (i, &v) in block.samples.iter().enumerate() {
                let frame = block.start + (i / ch) as u64;
                accs[i % ch][(frame / base) as usize].push(v);
            }
            start += block.frames as u64;
            done += 1;
            if let Some(p) = progress {
                p(done, total_reads);
            }
        }

        // Coarser levels merge adjacent bucket pairs.
        let mut levels = Vec::with_capacity(cfg.levels);
        let mut current = accs;
        for _ in 0..cfg.levels {
            levels.push(LodLevel {
                samples_per_bucket: base << (levels.len() as u64),
                channels: current
                    .iter()
                    .map(|c| c.iter().map(Acc::bucket).collect())
                    .collect(),
            });
            current = current
                .iter()
                .map(|c| {
                    c.chunks(2)
                        .map(|pair| {
                            let mut m = pair[0];
                            if let Some(b) = pair.get(1) {
                                m.merge(b);
                            }
                            m
                        })
                        .collect()
                })
                .collect();
        }
        Ok(Self { info, levels })
    }

    /// Source layout the pyramid was built from.
    #[must_use]
    pub fn info(&self) -> &WaveformInfo {
        &self.info
    }

    /// All levels, finest first.
    #[must_use]
    pub fn levels(&self) -> &[LodLevel] {
        &self.levels
    }

    /// Samples per bucket at `level`.
    #[must_use]
    pub fn samples_per_bucket(&self, level: usize) -> u64 {
        self.levels[level].samples_per_bucket
    }

    /// The coarsest level whose bucket still fits inside
    /// `samples_per_bin` — `None` when even level 0 is coarser than the
    /// bin (caller reads the source directly).
    #[expect(clippy::cast_precision_loss)]
    fn best_level(&self, samples_per_bin: f64) -> Option<usize> {
        let mut best = None;
        for (i, l) in self.levels.iter().enumerate() {
            if (l.samples_per_bucket as f64) <= samples_per_bin {
                best = Some(i);
            } else {
                break;
            }
        }
        best
    }
}

/// One drawing bin from [`WaveformSeries::view`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ViewBin {
    /// First sample the bin covers (inclusive).
    pub start: u64,
    /// Last sample the bin covers (exclusive; `> start` except for
    /// clamped tail bins).
    pub end: u64,
    /// Aggregated statistics (`EMPTY` = no finite data in range).
    pub bucket: LodBucket,
}

/// Channel-addressable waveform facade — LOD-accelerated when a
/// pyramid is attached, on-demand otherwise.
///
/// **RT contract:** non-RT only.
pub struct WaveformSeries {
    source: Arc<dyn WaveformSource>,
    lod: Option<WaveformLod>,
    read_chunk: u64,
}

/// Frames read per source call on the direct-read path.
const VIEW_CHUNK: u64 = 1 << 16;

impl WaveformSeries {
    /// On-demand series: every [`view`](Self::view) reduces the source
    /// directly (bounded reads; no pyramid memory).
    #[must_use]
    pub fn on_demand(source: Arc<dyn WaveformSource>) -> Self {
        Self {
            source,
            lod: None,
            read_chunk: VIEW_CHUNK,
        }
    }

    /// LOD-accelerated series (build with [`WaveformLod::build`]).
    #[must_use]
    pub fn with_lod(source: Arc<dyn WaveformSource>, lod: WaveformLod) -> Self {
        Self {
            source,
            lod: Some(lod),
            read_chunk: VIEW_CHUNK,
        }
    }

    /// The backing source.
    #[must_use]
    pub fn source(&self) -> &Arc<dyn WaveformSource> {
        &self.source
    }

    /// Attached pyramid, when built.
    #[must_use]
    pub fn lod(&self) -> Option<&WaveformLod> {
        self.lod.as_ref()
    }

    /// Source layout.
    #[must_use]
    pub fn info(&self) -> WaveformInfo {
        self.source.info()
    }

    /// Decimates `[start, end)` of `channel` into `bins` drawing bins.
    ///
    /// Each [`ViewBin`] reports its exact sample range and the
    /// aggregated [`LodBucket`]. The range is clamped to
    /// `info().frames`; bins past end-of-data are `EMPTY` (pan beyond
    /// the edge shows gaps, not errors).
    ///
    /// Resolution selection: the coarsest pyramid level with
    /// `samples_per_bucket <= samples_per_bin` serves the query;
    /// below `base_bucket` per bin the source is read directly
    /// (bounded by `bins * base_bucket` samples).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `channel >= channels` or `bins == 0`;
    /// [`VizError::Source`] on read failures.
    #[expect(clippy::cast_precision_loss)]
    pub fn view(&self, channel: usize, start: u64, end: u64, bins: usize) -> Result<Vec<ViewBin>> {
        let info = self.source.info();
        if channel >= info.channels {
            return Err(VizError::Invalid("channel"));
        }
        if bins == 0 {
            return Err(VizError::Invalid("bins"));
        }
        let frames = info.frames;
        let end = end.min(frames);
        let start = start.min(end);
        let span = end - start;
        if span == 0 {
            return Ok((0..bins)
                .map(|_| ViewBin {
                    start,
                    end: start,
                    bucket: LodBucket::EMPTY,
                })
                .collect());
        }
        let spp = span as f64 / bins as f64;
        if let Some(lod) = &self.lod
            && let Some(i) = lod.best_level(spp)
        {
            return Ok(Self::view_lod(channel, start, span, bins, &lod.levels()[i]));
        }
        self.view_direct(channel, start, span, bins)
    }

    /// Pyramid path: merge the buckets overlapping each bin's range.
    /// Boundary buckets are merged whole — edges are approximate to
    /// bucket resolution (that is the LOD trade-off).
    #[expect(clippy::cast_possible_truncation)]
    fn view_lod(
        channel: usize,
        start: u64,
        span: u64,
        bins: usize,
        level: &LodLevel,
    ) -> Vec<ViewBin> {
        let spb = level.samples_per_bucket;
        let series = &level.channels[channel];
        (0..bins)
            .map(|i| {
                // Ceil edges match the direct path's sample→bin rule
                // `floor((f-start)*bins/span)` even when span < bins.
                let s = start + (span * i as u64).div_ceil(bins as u64);
                let e = start + (span * (i as u64 + 1)).div_ceil(bins as u64);
                let mut acc = Acc::default();
                if s < e {
                    let first = (s / spb) as usize;
                    let last = ((e - 1) / spb) as usize;
                    if let Some(slice) =
                        series.get(first..=last.min(series.len().saturating_sub(1)))
                    {
                        for b in slice {
                            acc.merge(&Acc::from_bucket(b));
                        }
                    }
                }
                ViewBin {
                    start: s,
                    end: e,
                    bucket: acc.bucket(),
                }
            })
            .collect()
    }

    /// Direct path: stream the range in `read_chunk` blocks and fold
    /// every sample into its output bin.
    #[expect(clippy::cast_possible_truncation)]
    fn view_direct(
        &self,
        channel: usize,
        start: u64,
        span: u64,
        bins: usize,
    ) -> Result<Vec<ViewBin>> {
        let channels = self.source.info().channels;
        let mut accs = vec![Acc::default(); bins];
        let end = start + span;
        let mut at = start;
        while at < end {
            let want = usize::try_from((end - at).min(self.read_chunk)).unwrap_or(usize::MAX);
            let block = self.source.read(at, want)?;
            if block.frames == 0 {
                break;
            }
            for i in 0..block.frames {
                let frame = block.start + i as u64;
                let b = (((frame - start) * bins as u64) / span) as usize;
                accs[b.min(bins - 1)].push(block.samples[i * channels + channel]);
            }
            at = block.start + block.frames as u64;
        }
        Ok((0..bins)
            .map(|i| ViewBin {
                start: start + (span * i as u64).div_ceil(bins as u64),
                end: start + (span * (i as u64 + 1)).div_ceil(bins as u64),
                bucket: accs[i].bucket(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SliceWaveform;

    fn mono(samples: Vec<f64>) -> Arc<dyn WaveformSource> {
        Arc::new(SliceWaveform::new(48_000, 1, samples).unwrap())
    }

    #[test]
    fn lod_buckets_aggregate_and_merge() {
        let mut a = Acc::default();
        for v in [1.0, -2.0, 3.0] {
            a.push(v);
        }
        let mut b = Acc::default();
        for v in [0.5, -0.5] {
            b.push(v);
        }
        a.merge(&b);
        let bucket = a.bucket();
        assert_eq!(bucket.min, -2.0);
        assert_eq!(bucket.max, 3.0);
        assert_eq!(bucket.count, 5);
        let expect = ((1.0 + 4.0 + 9.0 + 0.25 + 0.25) / 5.0_f64).sqrt();
        assert!((bucket.rms - expect).abs() < 1e-12);
    }

    #[test]
    fn lod_bucket_nan_is_skipped_and_empty_is_nan() {
        let mut a = Acc::default();
        a.push(f64::NAN);
        a.push(1.0);
        let b = a.bucket();
        assert_eq!(b.count, 1);
        assert_eq!(b.min, 1.0);
        assert!(LodBucket::EMPTY.min.is_nan());
        assert_eq!(LodBucket::EMPTY.peak(), 0.0);
        assert_eq!(b.coverage(4), 0.25);
    }

    #[test]
    fn build_small_input_matches_direct_stats() {
        // 8 samples, base bucket 4 → 2 base buckets; level 1 merges both.
        let src = mono(vec![1.0, -1.0, 2.0, -2.0, 4.0, -4.0, 8.0, -8.0]);
        let cfg = LodConfig {
            base_bucket: 4,
            levels: 2,
            read_chunk: 4,
        };
        let lod = WaveformLod::build(src.as_ref(), &cfg, None, None).unwrap();
        let l0 = &lod.levels()[0].channels[0];
        assert_eq!(l0.len(), 2);
        assert_eq!(l0[0].min, -2.0); // {1,-1,2,-2}
        assert_eq!(l0[0].max, 2.0);
        assert_eq!(l0[1].min, -8.0); // {4,-4,8,-8}
        assert_eq!(l0[1].max, 8.0);
        let l1 = &lod.levels()[1].channels[0];
        assert_eq!(l1[0].min, -8.0);
        assert_eq!(l1[0].max, 8.0);
        assert_eq!(l1[0].count, 8);
    }

    #[test]
    fn view_coarse_uses_pyramid_and_zoom_reads_source() {
        // base 4, one channel of 32 known samples.
        let samples: Vec<f64> = (0..32).map(|i| f64::from(i) - 16.0).collect();
        let src = mono(samples);
        let cfg = LodConfig {
            base_bucket: 4,
            levels: 3,
            read_chunk: 8,
        };
        let lod = WaveformLod::build(src.as_ref(), &cfg, None, None).unwrap();
        let series = WaveformSeries::with_lod(src.clone(), lod);

        // Coarse: 32 samples → 4 bins (8 samples/bin, level-1 buckets).
        let v = series.view(0, 0, 32, 4).unwrap();
        assert_eq!(v.len(), 4);
        assert_eq!(v[0].bucket.min, -16.0);
        assert_eq!(v[3].bucket.max, 15.0);
        assert!(v.iter().all(|b| b.bucket.count > 0));

        // Extreme zoom-in: 8 samples → 16 bins < base_bucket → direct.
        let z = series.view(0, 8, 16, 16).unwrap();
        let filled: Vec<_> = z.iter().filter(|b| b.bucket.count > 0).collect();
        assert_eq!(filled.len(), 8); // one bin per sample
        assert_eq!(filled[0].bucket.min, -8.0);
    }

    #[test]
    fn on_demand_and_pan_past_end() {
        let src = mono(vec![1.0, 2.0, 3.0, 4.0]);
        let series = WaveformSeries::on_demand(src);
        let v = series.view(0, 0, 4, 2).unwrap();
        assert_eq!(v[0].bucket.min, 1.0);
        assert_eq!(v[1].bucket.max, 4.0);
        // Pan beyond end: range clamps to [2,4) (span 2 < 4 bins) —
        // ceil edges give bins [2,3),[3,3),[3,4),[4,4): samples land in
        // bins 0 and 2, the rest are EMPTY.
        let v = series.view(0, 2, 100, 4).unwrap();
        assert_eq!(v[0].bucket.count, 1);
        assert_eq!(v[1].bucket.count, 0);
        assert_eq!(v[2].bucket.count, 1);
        assert_eq!(v[3].bucket.count, 0);
        assert!(v[3].bucket.min.is_nan());
        // Zero-span → all EMPTY.
        assert_eq!(series.view(0, 4, 4, 2).unwrap()[0].bucket.count, 0);
        assert!(series.view(0, 0, 10, 0).is_err());
        assert!(series.view(1, 0, 10, 2).is_err());
    }

    #[test]
    fn build_cancels_at_chunk_boundary() {
        let src = mono(vec![1.0; 100]);
        let token = CancellationToken::new();
        token.cancel();
        let cfg = LodConfig {
            base_bucket: 4,
            levels: 1,
            read_chunk: 8,
        };
        assert!(matches!(
            WaveformLod::build(src.as_ref(), &cfg, Some(&token), None),
            Err(VizError::Cancelled)
        ));
    }
}
