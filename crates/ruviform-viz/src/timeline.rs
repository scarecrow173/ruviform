//! Timeline coordinate conversion and layout primitives.
//!
//! Two families:
//!
//! - **Exact conversion**: [`SampleTimebase`] (sample index ↔ seconds,
//!   exact rational math via `media-core`'s `MediaTime`) and
//!   [`FrameGrid`] (window/hop frame ↔ sample). Sample-domain math is
//!   integer; `f64` seconds appear only at display boundaries.
//! - **Layout**: [`Viewport`] (axis span ↔ pixel), [`LaneItem`],
//!   [`assign_lanes`] (greedy lane packing for overlapping items),
//!   [`Selection`], [`items_in`], [`hit_test`]. Items carry generic
//!   payloads — `viz` assigns geometry only and holds no diagnostic or
//!   annotation semantics.

use std::cmp::Ordering;

use ruviform_media_core::{MediaTime, Rounding, TimeBase};

use crate::{Result, VizError};

/// Sample index ↔ seconds conversion anchored on an integer sample rate.
///
/// Sample positions stay `u64`/`i64` integers; seconds are `f64` at
/// display boundaries only. [`SampleTimebase::instant`] exposes the
/// exact `MediaTime` path for consumers that need rational precision.
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SampleTimebase {
    rate: u32,
    base: TimeBase,
}

impl SampleTimebase {
    /// Anchors a `1/rate` time base.
    ///
    /// # Errors
    /// [`VizError::Invalid`] when `sample_rate == 0`.
    pub fn new(sample_rate: u32) -> Result<Self> {
        let base = TimeBase::sample_rate(sample_rate)?;
        Ok(Self {
            rate: sample_rate,
            base,
        })
    }

    /// The integer sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// The underlying `1/rate` [`TimeBase`].
    #[must_use]
    pub fn time_base(&self) -> TimeBase {
        self.base
    }

    /// The exact instant of sample `s` (`None` beyond the `i64` tick
    /// range — practically unreachable at audio rates).
    #[must_use]
    pub fn instant(&self, s: u64) -> Option<MediaTime> {
        i64::try_from(s).ok().map(|v| MediaTime::new(v, self.base))
    }

    /// Display-side seconds for sample `s` (`s / rate`, `f64`).
    ///
    /// Use [`SampleTimebase::instant`] + [`MediaTime::convert`] when the
    /// result must stay exact; this helper is for pixel/label paths.
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn sample_to_seconds(&self, s: u64) -> f64 {
        s as f64 / f64::from(self.rate)
    }

    /// Sample index for `seconds`, rounded to the nearest representable
    /// sample (ties to even — the `media-core` [`Rounding::NearestEven`]
    /// default). Negative or non-finite input clamps to `0`; results
    /// saturate at `u64::MAX`.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn seconds_to_sample(&self, seconds: f64) -> u64 {
        let t = seconds * f64::from(self.rate);
        if !t.is_finite() || t <= 0.0 {
            return 0;
        }
        t.round_ties_even().min(u64::MAX as f64) as u64
    }

    /// Converts sample `s` into an instant at `to` — for example a
    /// `1/1000` base for millisecond labels. At most one rounding is
    /// applied, per `rounding`.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `i64` tick-range overflow.
    pub fn sample_to_time(&self, s: u64, to: TimeBase, rounding: Rounding) -> Result<MediaTime> {
        self.instant(s)
            .ok_or(VizError::Invalid("sample"))?
            .convert(to, rounding)
            .map_err(|_| VizError::Invalid("sample"))
    }
}

/// Frame ↔ sample grid for windowed series (STFT frames, analysis hops).
///
/// Frame `f` occupies `[first_start + f*hop, first_start + f*hop + window)`
/// samples. `first_start` may be negative for conventions that center
/// frame 0 on sample 0.
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameGrid {
    /// Samples per window.
    pub window: u64,
    /// Samples between successive frame starts.
    pub hop: u64,
    /// Sample index where frame 0 starts (may be negative).
    pub first_start: i64,
}

impl FrameGrid {
    /// Creates a grid; `window` and `hop` must be non-zero.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on `window == 0` or `hop == 0`.
    pub fn new(window: u64, hop: u64, first_start: i64) -> Result<Self> {
        if window == 0 {
            return Err(VizError::Invalid("window"));
        }
        if hop == 0 {
            return Err(VizError::Invalid("hop"));
        }
        Ok(Self {
            window,
            hop,
            first_start,
        })
    }

    /// Start sample of frame `f` (may be negative).
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn frame_start(&self, f: u64) -> i64 {
        self.first_start + (f * self.hop) as i64
    }

    /// End sample (exclusive) of frame `f`.
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn frame_end(&self, f: u64) -> i64 {
        self.frame_start(f) + self.window as i64
    }

    /// Center sample of frame `f`.
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn frame_center(&self, f: u64) -> i64 {
        self.frame_start(f) + self.window as i64 / 2
    }

    /// Number of frames covering `[first_start, total)`: every frame
    /// `f` whose window *starts* before `total`
    /// (`first_start + f·hop < total`). Edge frames may extend past
    /// `total` — callers zero-pad them; this is the usual STFT frame
    /// count convention.
    #[must_use]
    pub fn frame_count(&self, total: u64) -> u64 {
        let rem = if self.first_start >= 0 {
            total.saturating_sub(self.first_start.unsigned_abs())
        } else {
            total.saturating_add(self.first_start.unsigned_abs())
        };
        rem.div_ceil(self.hop)
    }

    /// Index of the frame whose start is at-or-before `sample`
    /// (`floor((s - first_start) / hop)`; negative when `s < first_start`).
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn frame_at(&self, sample: i64) -> i64 {
        (sample - self.first_start).div_euclid(self.hop as i64)
    }
}

/// A visible span on one axis mapped to pixels.
///
/// The axis unit is caller-chosen (seconds, samples, frames); `viz` only
/// maps positions. `px` is an integer pixel count, `start`/`end` are
/// `f64` axis positions (`end` exclusive).
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Viewport {
    /// Axis position at the near edge (inclusive).
    pub start: f64,
    /// Axis position at the far edge (exclusive).
    pub end: f64,
    /// Pixels the span maps to (`> 0`).
    pub px: u32,
}

impl Viewport {
    /// Creates a viewport; requires `end > start` and `px > 0`.
    ///
    /// # Errors
    /// [`VizError::Invalid`] otherwise.
    pub fn new(start: f64, end: f64, px: u32) -> Result<Self> {
        if end <= start || !start.is_finite() || !end.is_finite() {
            return Err(VizError::Invalid("range"));
        }
        if px == 0 {
            return Err(VizError::Invalid("px"));
        }
        Ok(Self { start, end, px })
    }

    /// `end - start` axis units covered.
    #[must_use]
    pub fn span(&self) -> f64 {
        self.end - self.start
    }

    /// Axis units per pixel.
    #[must_use]
    pub fn units_per_px(&self) -> f64 {
        self.span() / f64::from(self.px)
    }

    /// Fractional pixel position of `u` (`0` = near edge; may fall
    /// outside `[0, px)` when `u` is outside the span).
    #[must_use]
    pub fn to_px(&self, u: f64) -> f64 {
        (u - self.start) / self.units_per_px()
    }

    /// Axis position at the center of pixel `px` (`0..px`).
    #[must_use]
    pub fn to_unit(&self, px: f64) -> f64 {
        self.start + (px + 0.5) * self.units_per_px()
    }

    /// A viewport zoomed by `factor` around `anchor` (the anchor's
    /// pixel position is preserved). `factor > 1` zooms in.
    ///
    /// # Errors
    /// [`VizError::Invalid`] on non-positive or non-finite factor.
    pub fn zoom(&self, factor: f64, anchor: f64) -> Result<Self> {
        if factor <= 0.0 || !factor.is_finite() {
            return Err(VizError::Invalid("factor"));
        }
        let span = self.span() / factor;
        let t = (anchor - self.start) / self.span();
        Self::new(anchor - span * t, anchor + span * (1.0 - t), self.px)
    }

    /// A viewport panned by `dpx` pixels (positive = content moves
    /// right, i.e. earlier axis positions become visible).
    #[must_use]
    pub fn pan(&self, dpx: f64) -> Self {
        let d = dpx * self.units_per_px();
        Self {
            start: self.start - d,
            end: self.end - d,
            px: self.px,
        }
    }
}

/// One item on the timeline axis with an application payload.
///
/// `start`/`end` are axis positions in the caller's unit (seconds or
/// samples); `end == start` marks a point event (marker). The payload
/// is opaque to `viz` — labels, ids, diagnostic references all live on
/// the application side.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneItem<T> {
    /// Axis position where the item starts (inclusive).
    pub start: f64,
    /// Axis position where the item ends (inclusive; `== start` for
    /// point markers).
    pub end: f64,
    /// Application payload — never interpreted by `viz`.
    pub payload: T,
}

impl<T> LaneItem<T> {
    /// An item covering `[start, end]` (endpoints normalize to
    /// `start <= end`). Non-finite endpoints are stored as-is and
    /// ignored by the geometric queries.
    pub fn new(start: f64, end: f64, payload: T) -> Self {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self {
            start,
            end,
            payload,
        }
    }

    /// A point event at `at`.
    pub fn marker(at: f64, payload: T) -> Self {
        Self {
            start: at,
            end: at,
            payload,
        }
    }

    /// Axis length covered (`end - start`; `0` for markers).
    pub fn span(&self) -> f64 {
        self.end - self.start
    }
}

/// Result of [`assign_lanes`]: items partitioned into display lanes.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneAssignment {
    /// `lanes[l]` = indices of the items assigned to lane `l`, in
    /// input order.
    pub lanes: Vec<Vec<usize>>,
    /// `lane_of[i]` = the lane item `i` was assigned to.
    pub lane_of: Vec<usize>,
}

/// Greedy first-fit lane packing: sorted by `(start, end)`, each item
/// takes the first lane whose last item ends at least `padding` units
/// before it starts; otherwise a new lane opens.
///
/// This is the general "multiple lanes" primitive — overlapping items
/// are stacked without any semantics attached to the lanes.
#[must_use]
pub fn assign_lanes<T>(items: &[LaneItem<T>], padding: f64) -> LaneAssignment {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| {
        items[a]
            .start
            .partial_cmp(&items[b].start)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                items[a]
                    .end
                    .partial_cmp(&items[b].end)
                    .unwrap_or(Ordering::Equal)
            })
    });
    let mut lanes: Vec<Vec<usize>> = Vec::new();
    let mut lane_of = vec![0usize; items.len()];
    // `last_end[l]` = end of the last item in lane l.
    let mut last_end: Vec<f64> = Vec::new();
    for i in order {
        let it = &items[i];
        let lane = last_end
            .iter()
            .position(|&e| e + padding <= it.start)
            .unwrap_or_else(|| {
                lanes.push(Vec::new());
                last_end.push(f64::NEG_INFINITY);
                lanes.len() - 1
            });
        lanes[lane].push(i);
        last_end[lane] = it.end;
        lane_of[i] = lane;
    }
    LaneAssignment { lanes, lane_of }
}

/// A normalized selection `[start, end]` on the timeline axis.
///
/// Endpoint convention is inclusive on both sides — selections are
/// user gestures, not half-open intervals.
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Selection {
    /// Earlier endpoint (inclusive).
    pub start: f64,
    /// Later endpoint (inclusive).
    pub end: f64,
}

impl Selection {
    /// A selection spanning `a..=b` (endpoint order normalized).
    #[must_use]
    pub fn new(a: f64, b: f64) -> Self {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        Self { start, end }
    }

    /// Whether the selection covers no axis length (`start == end`
    /// still contains that single point).
    #[must_use]
    pub fn is_point(&self) -> bool {
        // Endpoints are normalized `start <= end`, so equality is exact.
        self.start >= self.end
    }

    /// Whether `t` is inside (`start <= t <= end`).
    #[must_use]
    pub fn contains(&self, t: f64) -> bool {
        self.start <= t && t <= self.end
    }

    /// Whether `[a, b]` overlaps this selection.
    #[must_use]
    pub fn intersects(&self, a: f64, b: f64) -> bool {
        a <= self.end && b >= self.start
    }
}

/// Indices of the items overlapping `sel` (input order).
#[must_use]
pub fn items_in<T>(items: &[LaneItem<T>], sel: Selection) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| sel.intersects(it.start, it.end))
        .map(|(i, _)| i)
        .collect()
}

/// Index of the item nearest to `at` within `tolerance`, where distance
/// is `0` for items containing `at` and the gap to the nearer endpoint
/// otherwise. Ties go to the earliest input index; `None` when nothing
/// is within tolerance.
#[must_use]
pub fn hit_test<T>(items: &[LaneItem<T>], at: f64, tolerance: f64) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, it) in items.iter().enumerate() {
        let d = if it.start <= at && at <= it.end {
            0.0
        } else {
            (it.start - at).abs().min((it.end - at).abs())
        };
        if d > tolerance {
            continue;
        }
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timebase_converts_samples_and_seconds() {
        let tb = SampleTimebase::new(48_000).unwrap();
        assert!(tb.sample_to_seconds(48_000) - 1.0 < 1e-12);
        assert_eq!(tb.seconds_to_sample(1.5), 72_000);
        assert_eq!(tb.seconds_to_sample(-1.0), 0);
        assert_eq!(tb.seconds_to_sample(f64::NAN), 0);
        // Exact path: sample 1001 @ 48000 = exactly 1001/48000 s.
        let t = tb.instant(1001).unwrap();
        assert_eq!(
            t.convert(TimeBase::new(1, 48_000).unwrap(), Rounding::NearestEven)
                .unwrap()
                .value(),
            1001
        );
    }

    #[test]
    fn frame_grid_counts_and_positions() {
        let g = FrameGrid::new(2048, 1024, 0).unwrap();
        assert_eq!(g.frame_start(3), 3072);
        assert_eq!(g.frame_end(0), 2048);
        assert_eq!(g.frame_center(0), 1024);
        assert_eq!(g.frame_count(0), 0);
        assert_eq!(g.frame_count(1), 1);
        assert_eq!(g.frame_count(2048), 2);
        assert_eq!(g.frame_count(4096), 4);
        assert_eq!(g.frame_at(1024), 1);
        assert_eq!(g.frame_at(-5), -1);
        // Centered first frame: first_start = -window/2.
        let c = FrameGrid::new(2048, 1024, -1024).unwrap();
        assert_eq!(c.frame_center(0), 0);
        // f0 covers [-1024,1024), f1 covers [0,2048) → 2 frames for 1024 samples.
        assert_eq!(c.frame_count(1024), 2);
    }

    #[test]
    fn viewport_maps_and_transforms() {
        let v = Viewport::new(10.0, 20.0, 100).unwrap();
        assert!((v.units_per_px() - 0.1).abs() < 1e-12);
        assert!((v.to_px(15.0) - 50.0).abs() < 1e-9);
        assert!((v.to_unit(50.0) - 15.05).abs() < 1e-9);
        // Zoom ×2 around anchor 15 → span halves, anchor keeps its px.
        let z = v.zoom(2.0, 15.0).unwrap();
        assert!((z.span() - 5.0).abs() < 1e-12);
        assert!((z.to_px(15.0) - 50.0).abs() < 1e-9);
        // Pan +10px → start moves 10px worth of units earlier.
        let p = v.pan(10.0);
        assert!((p.start - 9.0).abs() < 1e-9);
        assert!(Viewport::new(5.0, 5.0, 10).is_err());
        assert!(Viewport::new(0.0, 5.0, 0).is_err());
    }

    #[test]
    fn lanes_pack_overlapping_items() {
        let items = vec![
            LaneItem::new(0.0, 2.0, "a"),
            LaneItem::new(1.0, 3.0, "b"), // overlaps a → lane 1
            LaneItem::new(2.5, 4.0, "c"), // overlaps b → lane 1? a ends 2.0, pad 0 → fits lane 0? a ends 2.0 <= 2.5 → lane 0
            LaneItem::marker(1.5, "m"),   // inside a → lane 2
        ];
        let a = assign_lanes(&items, 0.0);
        assert_eq!(a.lane_of, vec![0, 1, 0, 2]);
        assert_eq!(a.lanes[0], vec![0, 2]);
        assert_eq!(a.lanes[1], vec![1]);
        assert_eq!(a.lanes[2], vec![3]);
    }

    #[test]
    fn selection_and_hits() {
        let items = vec![
            LaneItem::new(0.0, 2.0, ()),
            LaneItem::marker(5.0, ()),
            LaneItem::new(8.0, 10.0, ()),
        ];
        let sel = Selection::new(4.0, 6.0);
        assert!(sel.contains(5.0));
        assert!(!sel.contains(6.5));
        assert_eq!(items_in(&items, sel), vec![1]);
        assert_eq!(items_in(&items, Selection::new(1.0, 9.0)), vec![0, 1, 2]);
        // Hit test: inside wins, then nearest within tolerance.
        assert_eq!(hit_test(&items, 5.0, 0.5), Some(1));
        assert_eq!(hit_test(&items, 4.0, 1.5), Some(1));
        assert_eq!(hit_test(&items, 1.0, 0.5), Some(0));
        assert_eq!(hit_test(&items, 3.0, 0.5), None);
    }
}
