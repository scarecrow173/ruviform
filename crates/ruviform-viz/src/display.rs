//! Display-side mappings, kept strictly separate from stored data.
//!
//! Everything in this module converts precise `f64` values into
//! *display* values: normalized units `0..=1`, colormap RGBA, quantized
//! steps. Nothing here mutates source or analysis data — the split is
//! the crate's precision contract.
//!
//! - [`DbRange`]: magnitude → dB → normalized unit, for spectrogram
//!   color ramps and waveform dB scales.
//! - [`LinearRange`]: value → unit for linear axes.
//! - [`FreqAxis`]: Hz-range → output-row bin mapping, linear or log,
//!   producing [`RowSpec`] for `Spectrogram::tile`.
//! - [`ColorMap`]: unit → RGBA (NaN → transparent), builtin
//!   perceptual maps plus caller LUTs.
//! - [`Quantization`]: explicit display rounding (`f64`/`f32`/`u8`)
//!   applied only when serializing.

use std::ops::Range;

use crate::source::BinAxis;
use crate::spectrogram::RowSpec;
use crate::{Result, VizError};

/// Linear value → unit mapping (amplitude axes, meters).
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LinearRange {
    /// Value mapped to `0.0`.
    pub min: f64,
    /// Value mapped to `1.0`.
    pub max: f64,
}

impl LinearRange {
    /// Creates a range; requires `max > min`.
    ///
    /// # Errors
    /// [`VizError::Invalid`] otherwise.
    pub fn new(min: f64, max: f64) -> Result<Self> {
        if max <= min || !min.is_finite() || !max.is_finite() {
            return Err(VizError::Invalid("range"));
        }
        Ok(Self { min, max })
    }

    /// `(v − min) / (max − min)` clamped to `0..=1`; `NaN` propagates.
    #[must_use]
    pub fn unit(&self, v: f64) -> f64 {
        if !v.is_finite() {
            return v;
        }
        ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
}

/// Magnitude → dB → unit mapping.
///
/// `db = 20·log10(mag / ref)`; `ref` is the full-scale reference
/// (typically `1.0` for PCM in `−1..1`). `min_db`/`max_db` set the
/// visible window — values outside clamp at the edges.
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DbRange {
    /// dB mapped to `0.0` (the floor of the visible ramp).
    pub min_db: f64,
    /// dB mapped to `1.0`.
    pub max_db: f64,
    /// Full-scale reference magnitude (`> 0`).
    pub reference: f64,
}

impl Default for DbRange {
    /// −80 dB .. 0 dBFS — a common spectrogram window.
    fn default() -> Self {
        Self {
            min_db: -80.0,
            max_db: 0.0,
            reference: 1.0,
        }
    }
}

impl DbRange {
    /// Creates a dB window; requires `max_db > min_db` and
    /// `reference > 0`.
    ///
    /// # Errors
    /// [`VizError::Invalid`] otherwise.
    pub fn new(min_db: f64, max_db: f64, reference: f64) -> Result<Self> {
        if max_db <= min_db {
            return Err(VizError::Invalid("db_range"));
        }
        if reference <= 0.0 {
            return Err(VizError::Invalid("reference"));
        }
        Ok(Self {
            min_db,
            max_db,
            reference,
        })
    }

    /// `20·log10(mag / reference)`; non-positive or non-finite
    /// magnitudes yield `−inf`/`NaN` (callers treat `−inf` as the
    /// floor, `NaN` as missing).
    #[must_use]
    pub fn db_of(&self, mag: f64) -> f64 {
        20.0 * (mag / self.reference).log10()
    }

    /// Unit position of a dB value within the window, clamped to
    /// `0..=1` (`−inf` → `0.0`); `NaN` propagates.
    #[must_use]
    pub fn unit(&self, db: f64) -> f64 {
        if db.is_nan() {
            return f64::NAN;
        }
        ((db - self.min_db) / (self.max_db - self.min_db)).clamp(0.0, 1.0)
    }

    /// Magnitude → unit in one step (`db_of` then `unit`).
    #[must_use]
    pub fn unit_of(&self, mag: f64) -> f64 {
        self.unit(self.db_of(mag))
    }
}

/// How output frequency rows map to Hz.
///
/// `#[non_exhaustive]` — additional scales (mel, bark) may be added.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FreqAxis {
    /// Uniform Hz per row.
    Linear {
        /// Lowest rendered Hz (inclusive).
        min_hz: f64,
        /// Highest rendered Hz (exclusive).
        max_hz: f64,
    },
    /// Geometric Hz per row (`min_hz` must be `> 0`).
    Log {
        /// Lowest rendered Hz (inclusive, `> 0`).
        min_hz: f64,
        /// Highest rendered Hz (exclusive).
        max_hz: f64,
    },
}

impl FreqAxis {
    /// The Hz range the axis covers.
    #[must_use]
    pub fn hz(&self) -> Range<f64> {
        match *self {
            Self::Linear { min_hz, max_hz } | Self::Log { min_hz, max_hz } => min_hz..max_hz,
        }
    }

    /// Validates the range (`max > min`, finite, log `min > 0`).
    ///
    /// # Errors
    /// [`VizError::Invalid`] otherwise.
    pub fn validate(&self) -> Result<()> {
        let (lo, hi) = (self.hz().start, self.hz().end);
        if hi <= lo || !lo.is_finite() || !hi.is_finite() {
            return Err(VizError::Invalid("hz"));
        }
        if matches!(self, Self::Log { .. }) && lo <= 0.0 {
            return Err(VizError::Invalid("min_hz"));
        }
        Ok(())
    }

    /// Hz boundary between output row `r−1` and `r` (`r` in
    /// `0..=rows`; row 0 is the lowest frequency).
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn row_edge_hz(&self, r: usize, rows: usize) -> f64 {
        let t = r as f64 / rows.max(1) as f64;
        match *self {
            Self::Linear { min_hz, max_hz } => min_hz + t * (max_hz - min_hz),
            Self::Log { min_hz, max_hz } => min_hz * (max_hz / min_hz).powf(t),
        }
    }

    /// Bin ranges per output row for a [`BinAxis`] — the
    /// [`RowSpec`] input of `Spectrogram::tile`.
    ///
    /// Rows whose Hz span covers no bin fall back to the single bin
    /// nearest the span center (log axes out-resolve the linear bin
    /// grid in the low end; those rows share bins).
    ///
    /// # Errors
    /// [`VizError::Invalid`] on an invalid axis or `rows == 0`.
    pub fn row_spec(&self, axis: &BinAxis, rows: usize) -> Result<RowSpec> {
        self.validate()?;
        if rows == 0 {
            return Err(VizError::Invalid("rows"));
        }
        let mut bins = Vec::with_capacity(rows);
        for r in 0..rows {
            let lo = self.row_edge_hz(r, rows);
            let hi = self.row_edge_hz(r + 1, rows);
            let mut range = bins_covering(axis, lo, hi);
            if range.is_empty() {
                // No bin inside the span: take the bin nearest its center.
                let mid = f64::midpoint(lo, hi);
                let k = nearest_bin(axis, mid);
                range = k..k + 1;
            }
            bins.push(range);
        }
        Ok(RowSpec { bins })
    }
}

/// Bins whose center Hz falls inside `[lo, hi)`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn bins_covering(axis: &BinAxis, lo: f64, hi: f64) -> Range<usize> {
    match axis {
        BinAxis::Uniform { first_hz, step_hz } => {
            if *step_hz <= 0.0 {
                return 0..0;
            }
            let k = |hz: f64| ((hz - first_hz) / step_hz).ceil().max(0.0) as usize;
            let a = k(lo);
            let b = k(hi).max(a);
            a..b
        }
        BinAxis::Centers(v) => {
            let a = v.partition_point(|&c| c < lo);
            let b = v.partition_point(|&c| c < hi).max(a);
            a..b
        }
    }
}

/// Index of the bin nearest `hz` (for empty-row fallback).
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest_bin(axis: &BinAxis, hz: f64) -> usize {
    match axis {
        BinAxis::Uniform { first_hz, step_hz } => {
            ((hz - first_hz) / step_hz.max(f64::MIN_POSITIVE))
                .round_ties_even()
                .max(0.0) as usize
        }
        BinAxis::Centers(v) => {
            if v.is_empty() {
                return 0;
            }
            let i = v.partition_point(|&c| c < hz).min(v.len() - 1);
            if i > 0 && (hz - v[i - 1]).abs() <= (v[i] - hz).abs() {
                i - 1
            } else {
                i
            }
        }
    }
}

/// Explicit display quantization — applied once at the encode/
/// rasterize boundary, never to stored data.
///
/// `#[non_exhaustive]` — additional steps may be added.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Quantization {
    /// Keep `f64` — no precision loss, 8 bytes per value.
    #[default]
    F64,
    /// `f32` rounding — 4 bytes per value, display-sufficient.
    F32,
    /// 256-step `u8` over the unit interval — 1 byte per value.
    U8,
}

impl Quantization {
    /// Bytes per stored value.
    #[must_use]
    pub fn bytes(&self) -> usize {
        match self {
            Self::F64 => 8,
            Self::F32 => 4,
            Self::U8 => 1,
        }
    }

    /// Quantizes a unit value (`0..=1` input for [`Quantization::U8`];
    /// other variants clamp nothing). `NaN` propagates.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn apply(&self, unit: f64) -> f64 {
        match self {
            Self::F64 => unit,
            Self::F32 => f64::from(unit as f32),
            Self::U8 => {
                if !unit.is_finite() {
                    return f64::NAN;
                }
                (unit.clamp(0.0, 1.0) * 255.0).round() / 255.0
            }
        }
    }
}

/// Unit → RGBA colormaps.
///
/// Builtin maps are piecewise-linear interpolations over the published
/// matplotlib anchor samples (8 stops for `Viridis`/`Magma`) — display
/// fidelity, not analysis data. `NaN` units map to fully transparent
/// RGBA — gaps render as absence, not black.
///
/// `#[non_exhaustive]` — additional maps may be added.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum ColorMap {
    /// Black → white.
    Gray,
    /// Black → red → yellow → white.
    Heat,
    /// matplotlib `viridis` (8-anchor piecewise).
    Viridis,
    /// matplotlib `magma` (8-anchor piecewise).
    Magma,
    /// Application LUT: `(index, stops)` — `stops` are evenly spaced
    /// over `0..=1` and linearly interpolated.
    Custom(&'static [[u8; 3]]),
}

impl ColorMap {
    /// `unit` (`0..=1`) → RGBA. Out-of-range clamps; `NaN` →
    /// `[0, 0, 0, 0]` (transparent gap).
    #[must_use]
    pub fn at(&self, unit: f64) -> [u8; 4] {
        if !unit.is_finite() {
            return [0, 0, 0, 0];
        }
        let t = unit.clamp(0.0, 1.0);
        let stops: &[[u8; 3]] = match self {
            Self::Gray => &[[0, 0, 0], [255, 255, 255]],
            Self::Heat => &[[0, 0, 0], [180, 0, 0], [255, 220, 0], [255, 255, 255]],
            Self::Viridis => &VIRIDIS,
            Self::Magma => &MAGMA,
            Self::Custom(stops) => stops,
        };
        lerp_stops(stops, t)
    }
}

/// Interpolates `stops` (evenly spaced over `0..=1`) at `pos`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn lerp_stops(stops: &[[u8; 3]], pos: f64) -> [u8; 4] {
    if stops.is_empty() {
        return [0, 0, 0, 0];
    }
    if stops.len() == 1 {
        let [red, green, blue] = stops[0];
        return [red, green, blue, 255];
    }
    let scaled = pos * (stops.len() - 1) as f64;
    let idx = (scaled as usize).min(stops.len() - 2);
    let frac = scaled - idx as f64;
    let (lo, hi) = (stops[idx], stops[idx + 1]);
    let mix = |va: u8, vb: u8| (f64::from(va) + frac * (f64::from(vb) - f64::from(va))) as u8;
    [mix(lo[0], hi[0]), mix(lo[1], hi[1]), mix(lo[2], hi[2]), 255]
}

/// matplotlib `viridis` anchors at eighths (published LUT samples).
const VIRIDIS: [[u8; 3]; 8] = [
    [68, 1, 84],
    [72, 35, 116],
    [64, 67, 135],
    [49, 104, 142],
    [33, 145, 140],
    [53, 183, 121],
    [144, 215, 67],
    [253, 231, 37],
];

/// matplotlib `magma` anchors at eighths (published LUT samples).
const MAGMA: [[u8; 3]; 8] = [
    [0, 0, 4],
    [28, 16, 68],
    [79, 18, 123],
    [129, 37, 129],
    [181, 54, 122],
    [229, 80, 100],
    [251, 136, 97],
    [252, 253, 191],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_range_normalizes() {
        let r = LinearRange::new(-1.0, 1.0).unwrap();
        assert_eq!(r.unit(-1.0), 0.0);
        assert_eq!(r.unit(1.0), 1.0);
        assert_eq!(r.unit(0.0), 0.5);
        assert_eq!(r.unit(5.0), 1.0); // clamps
        assert!(r.unit(f64::NAN).is_nan());
        assert!(LinearRange::new(1.0, 1.0).is_err());
    }

    #[test]
    fn db_range_maps_full_scale_and_silence() {
        let d = DbRange::default();
        assert_eq!(d.db_of(1.0), 0.0);
        assert_eq!(d.unit_of(1.0), 1.0); // 0 dBFS → top
        assert_eq!(d.unit_of(0.0001), 0.0); // -80 dB → floor
        assert_eq!(d.unit_of(0.0), 0.0); // -inf → floor
        assert_eq!(d.db_of(0.5), 20.0 * 0.5f64.log10());
        assert!(DbRange::new(0.0, -80.0, 1.0).is_err());
        assert!(DbRange::new(-80.0, 0.0, 0.0).is_err());
    }

    #[test]
    fn freq_axis_row_specs_linear_and_log() {
        let axis = BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 100.0,
        };
        // Linear: 0..1000 Hz, 5 rows → 2 bins each.
        let lin = FreqAxis::Linear {
            min_hz: 0.0,
            max_hz: 1000.0,
        };
        let spec = lin.row_spec(&axis, 5).unwrap();
        assert_eq!(spec.bins[0], 0..2);
        assert_eq!(spec.bins[4], 8..10);
        // Log: 10..1000, 2 rows → edges 10, 100, 1000 → rows [0.1..1)bin, [1..10)bin
        let log = FreqAxis::Log {
            min_hz: 10.0,
            max_hz: 1000.0,
        };
        let spec = log.row_spec(&axis, 2).unwrap();
        assert_eq!(spec.bins[0], 1..2); // empty span → nearest-bin fallback (mid 55 Hz → bin 1)
        assert_eq!(spec.bins[1], 1..10);
    }

    #[test]
    fn freq_axis_validates() {
        assert!(
            FreqAxis::Linear {
                min_hz: 100.0,
                max_hz: 50.0
            }
            .validate()
            .is_err()
        );
        assert!(
            FreqAxis::Log {
                min_hz: 0.0,
                max_hz: 100.0
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn quantization_rounds_at_display_only() {
        assert_eq!(Quantization::F64.apply(0.123_456_789), 0.123_456_789);
        let f32v = Quantization::F32.apply(0.123_456_789);
        assert!((f32v - 0.123_456_789).abs() < 1e-7);
        assert_ne!(f32v, 0.123_456_789); // actually rounded
        let u8v = Quantization::U8.apply(0.5);
        assert_eq!(u8v, 128.0 / 255.0);
        assert!(Quantization::U8.apply(f64::NAN).is_nan());
        assert_eq!(Quantization::U8.bytes(), 1);
    }

    #[test]
    fn colormap_endpoints_and_nan() {
        assert_eq!(ColorMap::Gray.at(0.0), [0, 0, 0, 255]);
        assert_eq!(ColorMap::Gray.at(1.0), [255, 255, 255, 255]);
        assert_eq!(ColorMap::Gray.at(f64::NAN), [0, 0, 0, 0]);
        assert_eq!(ColorMap::Viridis.at(0.0)[..3], [68, 1, 84]);
        assert_eq!(ColorMap::Viridis.at(1.0)[..3], [253, 231, 37]);
        let custom = ColorMap::Custom(&[[10, 20, 30], [50, 60, 70]]);
        assert_eq!(custom.at(0.5), [30, 40, 50, 255]);
        assert_eq!(ColorMap::Custom(&[]).at(0.5), [0, 0, 0, 0]);
    }
}
