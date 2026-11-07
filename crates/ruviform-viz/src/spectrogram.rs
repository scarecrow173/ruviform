//! Generic STFT analysis and viewport-shaped spectrum tiles.
//!
//! [`Stft::analyze`] streams a [`WaveformSource`] channel through
//! `ruviform-numeric`'s f64 real FFT and produces a [`Spectrogram`]
//! backed by complex bins ([`SpectrumData::Complex`]). The same
//! [`Spectrogram`] type also wraps any application-provided
//! [`SpectrumSource`] — stored intermediate spectra are reused as-is,
//! never re-analyzed — so tile serving works identically either way.
//!
//! [`Spectrogram::tile`] answers a [`TileRequest`]: a source frame
//! range decimated to `cols` columns and a [`RowSpec`] (bin ranges per
//! output row, precomputed by `display::FreqAxis`) aggregated to f64
//! magnitudes. Frequency-axis scale, dB mapping and coloring are
//! display concerns that stay in `display`; the tile keeps precise
//! values.
//!
//! Missing-data contract: an analysis window containing a non-finite
//! sample yields an all-`NaN` column; out-of-range samples at the
//! sequence edges are zero-padded. `NaN` propagates into tiles as gap
//! cells — never interpolated away.

use std::ops::Range;
use std::sync::Arc;

use ruviform_exec::CancellationToken;
use ruviform_numeric::fft::RealFft;
use ruviform_numeric::window::Window;

use crate::source::{BinAxis, SpectrumData, SpectrumInfo, SpectrumSource, WaveformSource};
use crate::timeline::FrameGrid;
use crate::{Result, VizError};

/// STFT analysis parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct StftConfig {
    /// FFT length (power of two — the `ruviform-numeric` plan
    /// requirement).
    pub fft_size: usize,
    /// Samples between successive frame starts (`>= 1`).
    pub hop: usize,
    /// Analysis window shape (periodic, `ruviform-numeric`
    /// convention).
    pub window: Window,
    /// `true` anchors frame 0's window centered on sample 0
    /// (`first_start = -fft/2`); `false` starts it at sample 0.
    pub center: bool,
    /// Frames read per [`WaveformSource::read`] during analysis.
    pub read_chunk: u64,
}

impl Default for StftConfig {
    /// 2048-point Hann, 50 % overlap, centered, 1<<16-frame reads.
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop: 1024,
            window: Window::Hann,
            center: true,
            read_chunk: 1 << 16,
        }
    }
}

impl StftConfig {
    /// Validates the parameters.
    ///
    /// # Errors
    /// [`VizError::Invalid`] when `fft_size` is not a power of two,
    /// `hop == 0`, or `read_chunk == 0`.
    pub fn validate(&self) -> Result<()> {
        if !self.fft_size.is_power_of_two() {
            return Err(VizError::Invalid("fft_size"));
        }
        if self.hop == 0 {
            return Err(VizError::Invalid("hop"));
        }
        if self.read_chunk == 0 {
            return Err(VizError::Invalid("read_chunk"));
        }
        Ok(())
    }

    /// Frame ↔ sample grid for this analysis over `frames` samples.
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn grid(&self) -> FrameGrid {
        let first = if self.center {
            -(self.fft_size as i64) / 2
        } else {
            0
        };
        FrameGrid {
            window: self.fft_size as u64,
            hop: self.hop as u64,
            first_start: first,
        }
    }

    /// Number of frames produced for `frames` input samples.
    #[must_use]
    pub fn frame_count(&self, frames: u64) -> u64 {
        self.grid().frame_count(frames)
    }
}

/// How source cells merge into one output tile cell.
///
/// `#[non_exhaustive]` — additional reducers may be added.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Aggregate {
    /// Maximum magnitude in the covered range (peak-preserving — the
    /// usual spectrogram decimation).
    #[default]
    Max,
    /// Arithmetic mean of magnitudes (smoother, energy-averaging).
    Mean,
}

/// Bin ranges per output frequency row.
///
/// `bins.len()` is the output row count; row 0 is the *lowest*
/// frequency. Ranges may be empty — `tile` emits `NaN` for rows with
/// no covering bins — and may overlap (log axes share bins in the
/// low end). Build via `display::FreqAxis::row_spec` or directly.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSpec {
    /// Source-bin range per output row (row 0 = lowest Hz).
    pub bins: Vec<Range<usize>>,
}

impl RowSpec {
    /// Uniform `rows`-way split of `[0, total_bins)` (linear axis).
    #[must_use]
    pub fn uniform(total_bins: usize, rows: usize) -> Self {
        let rows = rows.max(1);
        Self {
            bins: (0..rows)
                .map(|r| total_bins * r / rows..total_bins * (r + 1) / rows)
                .collect(),
        }
    }

    /// Number of output rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.bins.len()
    }

    /// Lowest and highest bin covered (the read span for the source).
    #[must_use]
    pub fn covered(&self) -> Range<usize> {
        let lo = self.bins.iter().map(|r| r.start).min().unwrap_or(0);
        let hi = self.bins.iter().map(|r| r.end).max().unwrap_or(0);
        lo..hi
    }
}

/// One viewport-shaped spectrum tile: `rows × cols` f64 magnitudes.
///
/// `values[r * cols + c]` — row 0 is the *lowest* frequency (renderers
/// typically flip vertically). `NaN` marks cells covering no data
/// (gaps, empty row ranges, `NaN` source bins).
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    /// Source frame range actually covered (after clamping).
    pub frames: Range<u64>,
    /// Output time columns.
    pub cols: usize,
    /// Output frequency rows.
    pub rows: usize,
    /// Magnitudes, row-major (`rows * cols`, row 0 = lowest Hz).
    pub values: Vec<f64>,
}

/// One time column of a [`Tile`]: the source frames it covers, how
/// many of its cells are finite, and its magnitudes.
#[derive(Clone, Debug, PartialEq)]
pub struct TileColumn {
    /// Source frames the column covers (`[start, end)`, within
    /// [`Tile::frames`]).
    pub frames: Range<u64>,
    /// Finite cells in `magnitudes` (`0..=rows`). `0` marks a gap
    /// column: no output cell in it holds data.
    pub count: u64,
    /// Magnitudes, row 0 = lowest Hz (`rows` values).
    pub magnitudes: Vec<f64>,
}

impl Tile {
    /// Time columns in order, one per output column.
    ///
    /// The frame ranges cover [`Tile::frames`] without gaps. They
    /// partition it exactly when `cols <= frames.len()`; with more
    /// columns than frames, each column still covers at least one frame
    /// and neighbouring columns share frames. An empty tile yields
    /// empty frame ranges.
    pub fn columns(&self) -> impl Iterator<Item = TileColumn> + '_ {
        let span = (self.frames.end - self.frames.start).max(1);
        (0..self.cols).map(move |c| {
            let offsets = column_offsets(span, self.cols, c);
            let end = self.frames.end;
            let frames = (self.frames.start + offsets.start).min(end)
                ..(self.frames.start + offsets.end).min(end);
            let magnitudes: Vec<f64> = (0..self.rows)
                .map(|r| self.values[r * self.cols + c])
                .collect();
            let count = magnitudes.iter().filter(|v| v.is_finite()).count() as u64;
            TileColumn {
                frames,
                count,
                magnitudes,
            }
        })
    }
}

/// Relative frame offsets `[cf0, cf1)` of output column `c` when a span
/// of `span` frames is split into `cols` columns. This is the single
/// definition of the column rule, shared by [`Spectrogram::tile`] and
/// [`Tile::columns`]. Every column covers at least one frame.
fn column_offsets(span: u64, cols: usize, c: usize) -> Range<u64> {
    let (c, cols) = (c as u64, cols as u64);
    let cf0 = span * c / cols;
    let cf1 = (span * (c + 1) / cols).max(cf0 + 1);
    cf0..cf1
}

/// A bounded tile query: which source frames, at what output
/// resolution, aggregated how.
#[derive(Clone, Debug, PartialEq)]
pub struct TileRequest {
    /// Source frames to cover (clamped to the source's frame count).
    pub frames: Range<u64>,
    /// Output frequency rows (row 0 = lowest Hz).
    pub rows: RowSpec,
    /// Output time columns (`>= 1`).
    pub cols: usize,
    /// Cell aggregation.
    pub aggregate: Aggregate,
}

/// Generic STFT analyzer.
///
/// **RT contract:** non-RT only.
pub struct Stft;

impl Stft {
    /// Analyzes `channel` of `src` into a complex-bin [`Spectrogram`].
    ///
    /// Reads stream in `read_chunk` blocks; `cancel` is observed per
    /// read; `progress(done, total)` reports completed frames.
    ///
    /// A window containing a non-finite sample produces an all-`NaN`
    /// column (missing data stays missing); sequence-edge samples
    /// outside `[0, frames)` are zero-padded.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError`] on invalid config/channel, source failures, or
    /// cancellation.
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    pub fn analyze(
        src: &dyn WaveformSource,
        channel: usize,
        cfg: &StftConfig,
        cancel: Option<&CancellationToken>,
        progress: Option<&dyn Fn(u64, u64)>,
    ) -> Result<Spectrogram> {
        cfg.validate()?;
        let info = src.info();
        if channel >= info.channels {
            return Err(VizError::Invalid("channel"));
        }
        let grid = cfg.grid();
        let n_frames = usize::try_from(grid.frame_count(info.frames))
            .map_err(|_| VizError::Invalid("frames"))?;
        let n_bins = cfg.fft_size / 2 + 1;
        let mut fft = RealFft::new(cfg.fft_size)?;
        let coeffs = cfg.window.coeffs(cfg.fft_size);
        let mut re = vec![0.0; n_frames * n_bins];
        let mut im = vec![0.0; n_frames * n_bins];
        let mut frame_buf = vec![0.0; cfg.fft_size];

        for f in 0..n_frames {
            if cancel.is_some_and(CancellationToken::is_cancelled) {
                return Err(VizError::Cancelled);
            }
            let s0 = grid.frame_start(f as u64);
            let (read_start, frames_wanted) = if s0 < 0 {
                // |s0| <= fft_size/2 (center mode), so the isize cast fits.
                (0u64, cfg.fft_size.saturating_add_signed(s0 as isize))
            } else {
                (s0 as u64, cfg.fft_size)
            };
            let block = src.read(read_start, frames_wanted.max(1))?;
            frame_buf.fill(0.0);
            let mut missing = false;
            for i in 0..cfg.fft_size {
                let s = s0 + i as i64;
                if s < 0 || s >= info.frames as i64 {
                    continue; // zero-pad outside the sequence
                }
                let off = usize::try_from(s - block.start as i64)
                    .ok()
                    .and_then(|o| o.checked_mul(info.channels))
                    .and_then(|o| o.checked_add(channel))
                    .ok_or(VizError::Source("block offset overflow".to_string()))?;
                let v = block.samples.get(off).copied().unwrap_or(0.0);
                if !v.is_finite() {
                    missing = true;
                    break;
                }
                frame_buf[i] = v * coeffs[i];
            }
            if missing {
                // Missing data stays missing: whole column is NaN.
                for b in 0..n_bins {
                    re[f * n_bins + b] = f64::NAN;
                    im[f * n_bins + b] = f64::NAN;
                }
            } else {
                let (r, i) = (
                    &mut re[f * n_bins..(f + 1) * n_bins],
                    &mut im[f * n_bins..(f + 1) * n_bins],
                );
                fft.forward(&frame_buf, r, i)?;
            }
            if let Some(p) = progress {
                p(f as u64 + 1, n_frames as u64);
            }
        }

        let spec = SpectrumInfo {
            frames: n_frames as u64,
            bins: n_bins,
            grid,
            axis: BinAxis::Uniform {
                first_hz: 0.0,
                step_hz: f64::from(info.sample_rate) / cfg.fft_size as f64,
            },
            sample_rate: info.sample_rate,
        };
        let inner = crate::source::VecSpectrum::new(spec, SpectrumData::Complex { re, im })?;
        Ok(Spectrogram::new(Arc::new(inner)))
    }
}

/// A spectrum tile provider over any [`SpectrumSource`].
///
/// Constructed from `Stft::analyze` (complex bins, in memory) or
/// directly over an application's source — tile serving is identical
/// either way, which is the no-re-analysis reuse path.
///
/// **RT contract:** non-RT only.
pub struct Spectrogram {
    source: Arc<dyn SpectrumSource>,
}

impl Spectrogram {
    /// Wraps a spectrum source.
    #[must_use]
    pub fn new(source: Arc<dyn SpectrumSource>) -> Self {
        Self { source }
    }

    /// The backing source.
    #[must_use]
    pub fn source(&self) -> &Arc<dyn SpectrumSource> {
        &self.source
    }

    /// Static source layout.
    #[must_use]
    pub fn info(&self) -> SpectrumInfo {
        self.source.info()
    }

    /// Serves a bounded, viewport-shaped tile of f64 magnitudes.
    ///
    /// Output size is `rows × cols` regardless of input size — the
    /// load contract scales with the request, not the dataset. Cells
    /// covering no finite bins are `NaN`. Complex bins are reduced to
    /// magnitudes during aggregation only. Time columns follow the rule
    /// of [`Tile::columns`].
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `cols == 0` or empty [`RowSpec`];
    /// [`VizError::Source`] on read failures.
    #[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn tile(&self, req: &TileRequest) -> Result<Tile> {
        if req.cols == 0 {
            return Err(VizError::Invalid("cols"));
        }
        let rows = req.rows.rows();
        if rows == 0 {
            return Err(VizError::Invalid("rows"));
        }
        let info = self.source.info();
        let f0 = req.frames.start.min(info.frames);
        let f1 = req.frames.end.min(info.frames).max(f0);
        let covered = req.rows.covered();
        let b0 = covered.start.min(info.bins);
        let b1 = covered.end.min(info.bins).max(b0);
        let read = self.source.read_tile(f0..f1, b0..b1)?;
        let mag = read.data.magnitudes();
        let (rb0, rb1) = (read.bin_start, read.bin_start + read.bins);
        let nfr = read.frames.max(1) as u64;
        let span = (f1 - f0).max(1);
        let mut values = vec![f64::NAN; rows * req.cols];
        for (r, bins) in req.rows.bins.iter().enumerate() {
            let lo = bins.start.max(rb0);
            let hi = bins.end.min(rb1);
            for c in 0..req.cols {
                let offsets = column_offsets(span, req.cols, c);
                let mut acc = 0.0f64;
                let mut n = 0u64;
                let mut peak = f64::NEG_INFINITY;
                for f in offsets.start..offsets.end.min(nfr) {
                    for b in lo..hi {
                        let v = mag[f as usize * read.bins + (b - rb0)];
                        if !v.is_finite() {
                            continue;
                        }
                        match req.aggregate {
                            Aggregate::Max => peak = peak.max(v),
                            Aggregate::Mean => {
                                acc += v;
                                n += 1;
                            }
                        }
                    }
                }
                values[r * req.cols + c] = match req.aggregate {
                    Aggregate::Max => {
                        if peak.is_finite() {
                            peak
                        } else {
                            f64::NAN
                        }
                    }
                    Aggregate::Mean => {
                        if n > 0 {
                            acc / n as f64
                        } else {
                            f64::NAN
                        }
                    }
                };
            }
        }
        Ok(Tile {
            frames: f0..f1,
            cols: req.cols,
            rows,
            values,
        })
    }
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use crate::source::{SliceWaveform, VecSpectrum};

    fn mono(samples: Vec<f64>) -> Arc<dyn WaveformSource> {
        Arc::new(SliceWaveform::new(48_000, 1, samples).unwrap())
    }

    #[test]
    fn stft_finds_a_sine_bin() {
        // 4 Hz-per-bin resolution: sine at exactly bin 4 (4*23.4375 Hz
        // per bin at 48k/2048 → use k=8 → 187.5 Hz).
        let n = 2048;
        let rate = 48_000f64;
        let fft = 256usize;
        let k = 8f64;
        let freq = k * rate / fft as f64;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * f64::from(i) / rate).sin())
            .collect();
        let src = mono(samples);
        let cfg = StftConfig {
            fft_size: fft,
            hop: 128,
            window: Window::Rectangular,
            center: false,
            read_chunk: 1024,
        };
        let spec = Stft::analyze(src.as_ref(), 0, &cfg, None, None).unwrap();
        let info = spec.info();
        assert_eq!(info.bins, fft / 2 + 1);
        // Peak bin of frame 0 must be k.
        // `b..b + 1` stays exclusive — `RowSpec.bins` is `Vec<Range<usize>>`,
        // so the `b..=b` suggestion does not typecheck (1.89-only false positive).
        #[allow(clippy::range_plus_one)]
        let tile = spec
            .tile(&TileRequest {
                frames: 0..1,
                rows: RowSpec {
                    bins: (0..info.bins).map(|b| b..b + 1).collect(),
                },
                cols: 1,
                aggregate: Aggregate::Max,
            })
            .unwrap();
        let peak = tile
            .values
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(peak, 8);
    }

    #[test]
    fn stft_nan_input_makes_nan_column() {
        let mut samples = vec![0.5; 512];
        samples[100] = f64::NAN;
        let src = mono(samples);
        let cfg = StftConfig {
            fft_size: 128,
            hop: 128,
            window: Window::Rectangular,
            center: false,
            read_chunk: 128,
        };
        let spec = Stft::analyze(src.as_ref(), 0, &cfg, None, None).unwrap();
        let tile = spec.source().read_tile(0..4, 0..3).unwrap();
        let mag = tile.data.magnitudes();
        // Frame 0 covers samples 0..128 → contains NaN → NaN column.
        assert!(mag[0].is_nan() && mag[1].is_nan() && mag[2].is_nan());
        // Frame 1 covers 128..256 → clean.
        assert!(mag[3].is_finite());
    }

    #[test]
    fn tile_aggregates_frames_and_bins() {
        // 4 frames x 4 bins magnitudes: value = f*10 + b.
        let info = SpectrumInfo {
            frames: 4,
            bins: 4,
            grid: FrameGrid::new(4, 2, 0).unwrap(),
            axis: BinAxis::Uniform {
                first_hz: 0.0,
                step_hz: 100.0,
            },
            sample_rate: 48_000,
        };
        let data: Vec<f64> = (0..4)
            .flat_map(|f| (0..4).map(move |b| f64::from(f * 10 + b)))
            .collect();
        let spec = Spectrogram::new(Arc::new(
            VecSpectrum::new(info, SpectrumData::Magnitude(data)).unwrap(),
        ));
        let t = spec
            .tile(&TileRequest {
                frames: 0..4,
                rows: RowSpec {
                    bins: vec![0..2, 2..4],
                },
                cols: 2,
                aggregate: Aggregate::Max,
            })
            .unwrap();
        assert_eq!((t.cols, t.rows), (2, 2));
        // col0 = frames 0,1 ; col1 = frames 2,3 ; row0 = bins 0,1 ; row1 = bins 2,3
        // row0/col0 max = f1b1 = 11; row0/col1 = f3b1 = 31
        // row1/col0 = f1b3 = 13; row1/col1 = f3b3 = 33
        assert_eq!(t.values, vec![11.0, 31.0, 13.0, 33.0]);
        // Mean over the same tiling.
        let m = spec
            .tile(&TileRequest {
                frames: 0..4,
                rows: RowSpec {
                    bins: vec![0..2, 2..4],
                },
                cols: 2,
                aggregate: Aggregate::Mean,
            })
            .unwrap();
        assert_eq!(m.values[0], (0.0 + 1.0 + 10.0 + 11.0) / 4.0);
        assert_eq!(m.values[3], (22.0 + 23.0 + 32.0 + 33.0) / 4.0);
    }

    #[test]
    fn tile_clamps_and_marks_empty() {
        let info = SpectrumInfo {
            frames: 2,
            bins: 2,
            grid: FrameGrid::new(4, 2, 0).unwrap(),
            axis: BinAxis::Uniform {
                first_hz: 0.0,
                step_hz: 100.0,
            },
            sample_rate: 48_000,
        };
        let spec = Spectrogram::new(Arc::new(
            VecSpectrum::new(info, SpectrumData::Magnitude(vec![1.0, 2.0, 3.0, 4.0])).unwrap(),
        ));
        // Frames past end + row range covering bins 5..7 (out of range → empty).
        let t = spec
            .tile(&TileRequest {
                frames: 0..99,
                rows: RowSpec {
                    bins: vec![0..2, 5..7],
                },
                cols: 2,
                aggregate: Aggregate::Max,
            })
            .unwrap();
        assert_eq!(t.frames, 0..2);
        assert!(t.values[0].is_finite());
        assert!(t.values[2].is_nan()); // row 1 (bins 5..7): no data
        assert!(
            spec.tile(&TileRequest {
                frames: 0..2,
                rows: RowSpec::uniform(2, 2),
                cols: 0,
                aggregate: Aggregate::Max
            })
            .is_err()
        );
    }

    #[test]
    fn columns_partition_frames_for_every_span() {
        for span in 1..=24usize {
            for cols in 1..=span {
                let span = span as u64;
                let tile = Tile {
                    frames: 5..5 + span,
                    cols,
                    rows: 1,
                    values: vec![0.0; cols],
                };
                let mut next = tile.frames.start;
                for col in tile.columns() {
                    assert_eq!(col.frames.start, next, "span={span} cols={cols}");
                    assert!(!col.frames.is_empty(), "span={span} cols={cols}");
                    next = col.frames.end;
                }
                assert_eq!(next, tile.frames.end, "span={span} cols={cols}");
            }
        }
    }

    #[test]
    fn empty_tile_columns_have_empty_frame_ranges() {
        let tile = Tile {
            frames: 7..7,
            cols: 3,
            rows: 1,
            values: vec![f64::NAN; 3],
        };
        let cols: Vec<TileColumn> = tile.columns().collect();
        assert_eq!(cols.len(), 3);
        assert!(cols.iter().all(|c| c.frames == (7..7) && c.count == 0));
    }

    #[test]
    fn columns_cover_frames_when_cols_exceed_span() {
        let tile = Tile {
            frames: 0..2,
            cols: 4,
            rows: 1,
            values: vec![0.0; 4],
        };
        let frames: Vec<_> = tile.columns().map(|c| c.frames).collect();
        assert_eq!(frames, vec![0..1, 0..1, 1..2, 1..2]);
    }

    #[test]
    fn tile_columns_split_indivisible_span_and_mark_nan_gap() {
        // 10 frames into 3 columns: 0..3, 3..6, 6..10 (indivisible span).
        let info = SpectrumInfo {
            frames: 10,
            bins: 2,
            grid: FrameGrid::new(4, 2, 0).unwrap(),
            axis: BinAxis::Uniform {
                first_hz: 0.0,
                step_hz: 100.0,
            },
            sample_rate: 48_000,
        };
        let mut data = vec![1.0; 20];
        // Frames 0..3 (column 0) are entirely NaN.
        data[..6].fill(f64::NAN);
        let spec = Spectrogram::new(Arc::new(
            VecSpectrum::new(info, SpectrumData::Magnitude(data)).unwrap(),
        ));
        let tile = spec
            .tile(&TileRequest {
                frames: 0..10,
                rows: RowSpec::uniform(2, 2),
                cols: 3,
                aggregate: Aggregate::Max,
            })
            .unwrap();
        let cols: Vec<TileColumn> = tile.columns().collect();
        assert_eq!(
            cols.iter().map(|c| c.frames.clone()).collect::<Vec<_>>(),
            vec![0..3, 3..6, 6..10]
        );
        assert_eq!(
            cols.iter().map(|c| c.count).collect::<Vec<_>>(),
            vec![0, 2, 2]
        );
        assert!(cols[0].magnitudes.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn column_count_counts_finite_cells() {
        // Row-major 2×3: column 1 is all NaN (gap); column 2 has one finite cell.
        let tile = Tile {
            frames: 0..4,
            cols: 3,
            rows: 2,
            values: vec![1.0, f64::NAN, f64::NAN, 2.0, f64::NAN, 4.0],
        };
        let cols: Vec<TileColumn> = tile.columns().collect();
        assert_eq!(
            cols.iter().map(|c| c.count).collect::<Vec<_>>(),
            vec![2, 0, 1]
        );
        assert_eq!(cols[0].magnitudes, vec![1.0, 2.0]);
        assert_eq!(cols[2].frames, 2..4);
    }
}
