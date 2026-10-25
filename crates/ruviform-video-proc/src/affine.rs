//! Integer-rectangle transform kernel: [`AffineTransform`] maps a
//! source rectangle onto a destination rectangle through [`RectMap`]
//! — translate + per-axis scale with cell-center nearest sampling —
//! and writes a complete output frame every call: `dst` pixels outside
//! `map.dst` become transparent black.
//!
//! ## Formats
//!
//! [`PixelFormat::Rgba8`], [`PixelFormat::RgbaF16`] and
//! [`PixelFormat::RgbaF32`] only, and `src`/`dst` must share the
//! format. The kernel never inspects channel values — it copies whole
//! pixels or writes zero bytes — so all three formats share a single
//! code path (`sample_bytes` sized pixel copies). Planar YUV is out
//! of scope.
//!
//! ## Mapping rule
//!
//! For a `dst` pixel `(x, y)` inside `map.dst`, let `offset = x −
//! map.dst.x` (`0 ≤ offset ≤ map.dst.w − 1`). The sampled source
//! coordinate is `sx = map.src.x + (2·offset·map.src.w + map.src.w) /
//! (2·map.dst.w)` — the exact integer evaluation of `floor((offset +
//! 0.5)·map.src.w/map.dst.w)`, i.e. cell-center aligned nearest
//! sampling (`sy` is the same formula on `y`/`h`). Because `offset ≤
//! map.dst.w − 1` the quotient is always `< map.src.w`, so `sx`/`sy`
//! stay inside `map.src` structurally and never need a clamp
//! (example: a 1→2 upscale maps offsets `[0, 1]` to source pixels
//! `[0, 0]` — pure replication). All scalar mapping math runs in
//! `u128`/`i128`, and the bounds checks (`x + w ≤ width` etc.) run in
//! `i64` after widening, so no frame dimension or rectangle value can
//! overflow.
//!
//! ## Canvas clipping
//!
//! `map.dst` may sit partially or fully off the `dst` canvas —
//! negative `x`/`y` included — and may be empty (`w == 0`/`h == 0`):
//! none of these is an error. The clipped intersection is what gets
//! sampled; an empty or non-intersecting `map.dst` is a defined result
//! where every `dst` pixel becomes transparent black. `dst` is fully
//! overwritten either way.
//!
//! `map.src` must be non-empty and contained in the `src` frame —
//! violations are [`Error::InvalidValue`]. `src`/`dst` dimensions do
//! not have to match: transforming is the point of the kernel (a
//! 640×360 source can map into a 1920×1080 canvas).
//!
//! ## `ColorInfo` and timestamps
//!
//! `dst` receives `src`'s `ColorInfo`/`pts`/`duration` — a geometric
//! transform of a single frame keeps the frame's identity and timing.
//!
//! ## Equivalence contract
//!
//! The kernel performs no pixel arithmetic — mapping and copying only
//! — so it is **bit-exact** on every accepted format: every conforming
//! backend must emit identical bytes.
//!
//! Rows are processed through `Plane::row`/`PlaneMut::row_mut` only,
//! so `stride > row_bytes` padding is never read or written.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{PixelFormat, VideoFrame};

/// `floor((offset + 0.5)·src_len/dst_len)` — the exact integer form
/// `(2·offset·src_len + src_len)/(2·dst_len)` evaluated in `u128`, so
/// no `u32`/`i32` field combination can overflow the multiplication.
/// The caller guarantees `0 < dst_len` and `offset < dst_len`, which
/// bounds the quotient to `< src_len` structurally — the `u128 → u32`
/// cast is lossless and the result can never step outside the source
/// rect.
#[allow(clippy::cast_possible_truncation)]
fn cell_center_offset(offset: u128, src_len: u32, dst_len: u32) -> u32 {
    let num = 2 * offset * u128::from(src_len) + u128::from(src_len);
    (num / (2 * u128::from(dst_len))) as u32
}

/// Integer-pixel rectangle on a frame grid — `x`/`y` is the top-left
/// corner (x rightward, y downward) in whole pixels and `w`/`h` the
/// extent in pixels.
///
/// Plain data: invariants are enforced by
/// [`AffineTransform::process`] validation, not by construction. On
/// the `src` side the rect must be non-empty and inside the frame
/// (`x`/`y` ≥ 0); on the `dst` side `x`/`y` may be negative, placing
/// the corner off-canvas.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Left edge in pixels — may be negative on the `dst` side
    /// (off-canvas placement).
    pub x: i32,
    /// Top edge in pixels — may be negative on the `dst` side.
    pub y: i32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

/// Rectangle mapping for [`AffineTransform`]: the `src` rect is
/// sampled onto the `dst` rect. The per-axis scale is the rational
/// ratio `dst.w/src.w` (resp. `dst.h/src.h`) — not restricted to
/// integer ratios — and equal extents produce a pure translation.
///
/// Plain data: invariants are enforced by
/// [`AffineTransform::process`] validation, not by construction.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RectMap {
    /// Source rectangle — must be non-empty and inside the `src`
    /// frame when passed to [`AffineTransform::process`].
    pub src: Rect,
    /// Destination rectangle in `dst` canvas coordinates — may be
    /// empty or reach off-canvas (clipped; defined transparent-black
    /// results, never an error).
    pub dst: Rect,
}

/// Prepared integer-rectangle transform kernel — the composite
/// family's two-phase shape of ADR D1-b: [`new`](Self::new) is the
/// trivial non-RT `prepare` step (no coefficients to bake — the same
/// unit shape as [`Rgba8ToRgbaF16`](crate::Rgba8ToRgbaF16)), and
/// [`process`](Self::process) is the allocation-free transform step
/// carrying the per-call [`RectMap`]. See the module doc for the
/// mapping, clipping, propagation and equivalence contracts.
///
/// `process` rewrites `dst` completely — sampled pixels inside the
/// mapped span, transparent black outside — which is what makes the
/// NLE compositing pattern `AffineTransform` → `CompositeOver` work on
/// a canvas-sized intermediate (the caller allocates the
/// intermediate; the kernel does not).
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::{AffineTransform, Rect, RectMap};
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts).unwrap();
/// src.plane_mut(0)
///     .unwrap()
///     .data_mut()
///     .copy_from_slice(&[10, 20, 30, 255, 40, 50, 60, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 4, 1, pts).unwrap();
///
/// let mut kernel = AffineTransform::new();
/// kernel
///     .process(
///         &src,
///         &mut dst,
///         RectMap {
///             src: Rect { x: 0, y: 0, w: 2, h: 1 },
///             dst: Rect { x: 0, y: 0, w: 4, h: 1 },
///         },
///     )
///     .unwrap();
///
/// // The 2→4 upscale replicates each source pixel twice: `[0,0,1,1]`.
/// assert_eq!(
///     dst.plane(0).unwrap().row(0).unwrap(),
///     &[10, 20, 30, 255, 10, 20, 30, 255, 40, 50, 60, 255, 40, 50, 60, 255],
/// );
/// ```
#[derive(Debug)]
pub struct AffineTransform;

impl AffineTransform {
    /// Builds the prepared kernel. No coefficients are needed — the
    /// mapping rule is fixed and the rectangles arrive per `process`
    /// call — so construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::AffineTransform::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Transforms `src` into `dst` per the rules on the module doc —
    /// `map.src` is sampled onto `map.dst` with cell-center nearest
    /// sampling, and every `dst` pixel outside the mapped span becomes
    /// transparent black.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `Rgba8`/`RgbaF16`/`RgbaF32`
    /// (`"src format"`), `dst` has a different format (`"dst format"`),
    /// `map.src` is empty or reaches outside the `src` frame
    /// (`"src rect"`), or `dst`'s backing is still shared
    /// (`dst.plane_mut(0)` → `None`, `"dst plane"`). Every check runs
    /// before any write — a failed call never partially fills `dst`.
    /// An empty or fully off-canvas `map.dst` is *not* an error: it
    /// produces the defined all-transparent-black frame.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking
    /// and bounded (exactly `width`×`height` pixel iterations over
    /// borrowed rows). Dropping `src`/`dst` stays the caller's non-RT
    /// concern.
    // The `as` conversions in this body are lossless by construction:
    // `map.src` is validated inside `src` (so `map.src.x`/`y` ≥ 0 and
    // every `sx`/`sy` stays `< width`/`height`), and in the `Some`
    // arm of `span` the clip bounds satisfy `0 ≤ x0 < x1 ≤ width`,
    // `0 ≤ y0 < y1 ≤ height` with non-negative offsets.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame, map: RectMap) -> Result<()> {
        match src.format() {
            PixelFormat::Rgba8 | PixelFormat::RgbaF16 | PixelFormat::RgbaF32 => {}
            _ => return Err(Error::InvalidValue("src format")),
        }
        if dst.format() != src.format() {
            return Err(Error::InvalidValue("dst format"));
        }
        // `map.src` must be non-empty and inside `src` — the `i64`
        // widening keeps `x + w`-style additions overflow-free for any
        // `i32`/`u32` field combination.
        let src_w = i64::from(src.width());
        let src_h = i64::from(src.height());
        let rx = i64::from(map.src.x);
        let ry = i64::from(map.src.y);
        let rw = i64::from(map.src.w);
        let rh = i64::from(map.src.h);
        if map.src.w == 0
            || map.src.h == 0
            || rx < 0
            || ry < 0
            || rx + rw > src_w
            || ry + rh > src_h
        {
            return Err(Error::InvalidValue("src rect"));
        }
        let src_color = src.color();
        let src_plane = src.plane(0).ok_or(Error::InvalidValue("src plane"))?;
        // Captured before the mutable borrow: `dst`'s canvas extent.
        let dst_w = i64::from(dst.width());
        let dst_h = dst.height();
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            // Clip `map.dst` against the canvas in `i64`.
            let dx = i64::from(map.dst.x);
            let dy = i64::from(map.dst.y);
            let clip_left = dx.max(0);
            let clip_top = dy.max(0);
            let clip_right = (dx + i64::from(map.dst.w)).min(dst_w);
            let clip_bottom = (dy + i64::from(map.dst.h)).min(i64::from(dst_h));
            // In-canvas column/row ranges of the mapped span plus the
            // `map.dst` offsets (`clip − dst` origin ≥ 0) of their
            // first columns/rows — `None` when `map.dst` is empty or
            // fully off-canvas; the fallback folds that into empty
            // ranges so every row takes the black-fill path: the
            // defined all-transparent-black result.
            let span = (clip_left < clip_right && clip_top < clip_bottom).then(|| {
                (
                    clip_left as u32..clip_right as u32,
                    clip_top as u32..clip_bottom as u32,
                    (clip_left - dx) as u128,
                    (clip_top - dy) as u128,
                )
            });
            let (cols, rows, ox, oy) = span.unwrap_or((0..0, 0..0, 0, 0));
            // Bytes per pixel — 4 for `Rgba8`, 8 for `RgbaF16`, 16 for
            // `RgbaF32`; the copy path is format-agnostic.
            let px = src_plane.sample_bytes();
            let head_bytes = cols.start as usize * px;
            let tail_start = cols.end as usize * px;
            for y in 0..dst_h {
                // These row borrows cannot fail: `y < height`, and
                // `sy < map.src.y + map.src.h ≤ height` structurally —
                // `ok_or` keeps the contract panic-free regardless.
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                if !rows.contains(&y) {
                    // Off-span row (or no span at all): transparent
                    // black is zero bytes in every format.
                    dst_row.fill(0);
                    continue;
                }
                let sy = map.src.y as u32
                    + cell_center_offset(oy + u128::from(y - rows.start), map.src.h, map.dst.h);
                let src_row = src_plane.row(sy).ok_or(Error::InvalidValue("src plane"))?;
                // Columns before/after the mapped span on this row:
                // transparent black.
                dst_row[..head_bytes].fill(0);
                dst_row[tail_start..].fill(0);
                for (i, dst_px) in dst_row[head_bytes..tail_start]
                    .chunks_exact_mut(px)
                    .enumerate()
                {
                    let sx =
                        map.src.x as u32 + cell_center_offset(ox + i as u128, map.src.w, map.dst.w);
                    let si = sx as usize * px;
                    dst_px.copy_from_slice(&src_row[si..si + px]);
                }
            }
        }
        // Geometric transform of a single frame: `dst` takes `src`'s
        // `ColorInfo`/`pts`/`duration` — interpretation and timing are
        // unchanged.
        dst.set_color(src_color);
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for AffineTransform {
    /// Equivalent to [`AffineTransform::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{ColorInfo, ColorPrimaries, PixelFormat, SharedPlane, VideoFrame};

    use super::*;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Builds an `Rgba8` frame holding `pixels` (tightly packed).
    fn rgba8_frame(width: u32, height: u32, pixels: &[[u8; 4]]) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::Rgba8, width, height, pts()).unwrap();
        frame
            .plane_mut(0)
            .unwrap()
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(pixels.iter())
            .for_each(|(dst, src)| dst.copy_from_slice(src));
        frame
    }

    /// Builds an `RgbaF16` frame of `pixels` — channels given as
    /// `binary16` bit patterns, written little-endian.
    fn f16_frame(width: u32, height: u32, pixels: &[[u16; 4]]) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::RgbaF16, width, height, pts()).unwrap();
        {
            let mut plane = frame.plane_mut(0).unwrap();
            for (dst_px, src_px) in plane
                .data_mut()
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .zip(pixels.iter())
            {
                for (dst_ch, value) in dst_px.as_chunks_mut::<2>().0.iter_mut().zip(src_px.iter()) {
                    dst_ch.copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        frame
    }

    /// Reads the `x`-th `RgbaF16` pixel of row `y` back as `binary16`
    /// bit patterns.
    fn f16_px(frame: &VideoFrame, x: usize, y: u32) -> [u16; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let mut out = [0_u16; 4];
        for (dst, ch) in out
            .iter_mut()
            .zip(row[x * 8..x * 8 + 8].as_chunks::<2>().0.iter())
        {
            *dst = u16::from_le_bytes(*ch);
        }
        out
    }

    /// Builds an `RgbaF32` frame of `pixels` — channels given as
    /// `binary32` bit patterns, written little-endian (bit-exact
    /// coverage wants arbitrary patterns: NaN payloads, `−0.0`,
    /// subnormals).
    fn f32_frame(width: u32, height: u32, pixels: &[[u32; 4]]) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::RgbaF32, width, height, pts()).unwrap();
        {
            let mut plane = frame.plane_mut(0).unwrap();
            for (dst_px, src_px) in plane
                .data_mut()
                .as_chunks_mut::<16>()
                .0
                .iter_mut()
                .zip(pixels.iter())
            {
                for (dst_ch, bits) in dst_px.as_chunks_mut::<4>().0.iter_mut().zip(src_px.iter()) {
                    dst_ch.copy_from_slice(&bits.to_le_bytes());
                }
            }
        }
        frame
    }

    /// Reads the `x`-th `RgbaF32` pixel of row `y` back as `binary32`
    /// bit patterns.
    fn f32_px(frame: &VideoFrame, x: usize, y: u32) -> [u32; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let mut out = [0_u32; 4];
        for (dst, ch) in out
            .iter_mut()
            .zip(row[x * 16..x * 16 + 16].as_chunks::<4>().0.iter())
        {
            *dst = u32::from_le_bytes(*ch);
        }
        out
    }

    /// Reads a `dst` frame's `Rgba8` pixels row-major as a `Vec` —
    /// one 4-tuple per pixel, padding excluded.
    fn rgba8_px(frame: &VideoFrame) -> Vec<[u8; 4]> {
        let plane = frame.plane(0).unwrap();
        let mut out = Vec::new();
        for y in 0..plane.height() {
            out.extend(plane.row(y).unwrap().as_chunks::<4>().0.iter().copied());
        }
        out
    }

    /// Builds a `RectMap` from bare rect fields — a convenience that
    /// keeps each assertion to one line.
    #[allow(clippy::too_many_arguments)]
    fn map(sx: i32, sy: i32, sw: u32, sh: u32, dx: i32, dy: i32, dw: u32, dh: u32) -> RectMap {
        RectMap {
            src: Rect {
                x: sx,
                y: sy,
                w: sw,
                h: sh,
            },
            dst: Rect {
                x: dx,
                y: dy,
                w: dw,
                h: dh,
            },
        }
    }

    /// Identity mapping — same-size `map.src` covering all of `src`
    /// onto all of `dst` — is a bit-exact copy.
    #[test]
    fn identity_map_is_bit_exact_copy() {
        let src = rgba8_frame(
            2,
            2,
            &[
                [1, 2, 3, 4],
                [5, 6, 7, 8],
                [9, 10, 11, 12],
                [13, 14, 15, 16],
            ],
        );
        let mut dst = rgba8_frame(2, 2, &[[200; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 2, 0, 0, 2, 2))
            .unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), src.plane(0).unwrap().data());
    }

    /// Translation: same-size rects at different origins — `map.src`
    /// crops a sub-rect of `src`, `map.dst` places it inside `dst`,
    /// and the uncovered columns stay transparent black.
    #[test]
    fn sub_rect_crop_and_placement() {
        let src = rgba8_frame(
            4,
            1,
            &[[1, 0, 0, 1], [2, 0, 0, 2], [3, 0, 0, 3], [4, 0, 0, 4]],
        );
        let mut dst = rgba8_frame(4, 1, &[[200; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(1, 0, 2, 1, 2, 0, 2, 1))
            .unwrap();
        assert_eq!(
            rgba8_px(&dst),
            vec![[0, 0, 0, 0], [0, 0, 0, 0], [2, 0, 0, 2], [3, 0, 0, 3]]
        );
    }

    /// Trap vector — 1→2 upscale: both output pixels are replications
    /// of source pixel 0 (`offset` `[0,1]` → `sx` `[0,0]`).
    #[test]
    fn upscale_1_to_2_replicates_source_pixel() {
        let src = rgba8_frame(1, 1, &[[9, 8, 7, 6]]);
        let mut dst = rgba8_frame(2, 1, &[[0; 4]; 2]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 0, 0, 2, 1))
            .unwrap();
        assert_eq!(rgba8_px(&dst), vec![[9, 8, 7, 6], [9, 8, 7, 6]]);
    }

    /// Trap vector — 2→4 upscale replicates `[0,0,1,1]`: the last
    /// output column lands on `src.x + 1`, proving the quotient never
    /// reaches `map.src.w` (the structural in-range guarantee).
    #[test]
    fn upscale_2_to_4_replicates_pairs_last_column_in_range() {
        let src = rgba8_frame(2, 1, &[[10, 0, 0, 10], [20, 0, 0, 20]]);
        let mut dst = rgba8_frame(4, 1, &[[0; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 4, 1))
            .unwrap();
        assert_eq!(
            rgba8_px(&dst),
            vec![
                [10, 0, 0, 10],
                [10, 0, 0, 10],
                [20, 0, 0, 20],
                [20, 0, 0, 20]
            ]
        );
    }

    /// Downscale — 4→2 decimates by cell centers: `offset` `[0,1]` →
    /// `floor((0.5)·4/2) = 1`, `floor((1.5)·4/2) = 3`.
    #[test]
    fn downscale_4_to_2_decimates_by_cell_centers() {
        let src = rgba8_frame(
            4,
            1,
            &[[1, 0, 0, 1], [2, 0, 0, 2], [3, 0, 0, 3], [4, 0, 0, 4]],
        );
        let mut dst = rgba8_frame(2, 1, &[[0; 4]; 2]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 4, 1, 0, 0, 2, 1))
            .unwrap();
        assert_eq!(rgba8_px(&dst), vec![[2, 0, 0, 2], [4, 0, 0, 4]]);
    }

    /// Vertical mapping works on `h` the same way — 1→2 upscale
    /// replicates the single source row, and a `map.dst` inset writes
    /// black rows around it.
    #[test]
    fn vertical_mapping_uses_height_axis() {
        let src = rgba8_frame(1, 1, &[[5, 6, 7, 8]]);
        let mut dst = rgba8_frame(1, 4, &[[0; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 0, 1, 1, 2))
            .unwrap();
        assert_eq!(
            rgba8_px(&dst),
            vec![[0, 0, 0, 0], [5, 6, 7, 8], [5, 6, 7, 8], [0, 0, 0, 0]]
        );
    }

    /// `map.dst.x < 0` places the mapped region partially off-canvas
    /// left: only the intersecting columns are written, and they keep
    /// the offset the clipped-away columns started — `dst.x = −1`,
    /// `w = 2` on a 2-wide canvas writes only `x = 0` with `offset =
    /// 1` → `floor(1.5·2/2) = 1` (source pixel 1), while `x = 1`
    /// falls outside `map.dst` and becomes transparent black.
    #[test]
    fn negative_dst_x_clips_to_canvas() {
        let src = rgba8_frame(2, 1, &[[10, 0, 0, 10], [20, 0, 0, 20]]);
        let mut dst = rgba8_frame(2, 1, &[[200; 4]; 2]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 1, -1, 0, 2, 1))
            .unwrap();
        assert_eq!(rgba8_px(&dst), vec![[20, 0, 0, 20], [0, 0, 0, 0]]);
    }

    /// `map.dst` reaching past the canvas right/bottom edges is clipped
    /// the same way: `dst.x = 1`/`w = 3` and `dst.y = 1`/`h = 3` on a
    /// 3×3 canvas clip to the in-canvas 2×2 (`offset` `[0,1]` → `sx`/
    /// `sy` `[0,1]` — the clipped `offset` 2 column and row never
    /// sample), while the uncovered top row and left column become
    /// transparent black.
    #[test]
    fn right_bottom_overhang_clips_to_canvas() {
        let src = rgba8_frame(
            2,
            2,
            &[[1, 0, 0, 1], [2, 0, 0, 2], [3, 0, 0, 3], [4, 0, 0, 4]],
        );
        let mut dst = rgba8_frame(3, 3, &[[200; 4]; 9]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 2, 1, 1, 3, 3))
            .unwrap();
        assert_eq!(
            rgba8_px(&dst),
            vec![
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [1, 0, 0, 1],
                [2, 0, 0, 2],
                [0, 0, 0, 0],
                [3, 0, 0, 3],
                [4, 0, 0, 4],
            ]
        );
    }

    /// `dst` is fully overwritten: pixels of `map.dst` that fall
    /// outside the canvas leave transparent black behind, and so does
    /// the canvas area the rect never covered.
    #[test]
    fn dst_fully_overwritten_outside_mapped_rect() {
        let src = rgba8_frame(1, 1, &[[9, 8, 7, 6]]);
        let mut dst = rgba8_frame(3, 1, &[[200; 4]; 3]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 1, 0, 1, 1))
            .unwrap();
        assert_eq!(
            rgba8_px(&dst),
            vec![[0, 0, 0, 0], [9, 8, 7, 6], [0, 0, 0, 0]]
        );
    }

    /// An empty `map.dst` (`w == 0` or `h == 0`) is not an error —
    /// the defined result is an all-transparent-black `dst`.
    #[test]
    fn empty_dst_rect_writes_all_transparent_black() {
        let src = rgba8_frame(1, 1, &[[9, 8, 7, 6]]);
        let mut dst = rgba8_frame(2, 2, &[[200; 4]; 4]);
        let mut kernel = AffineTransform::new();
        for zero in [map(0, 0, 1, 1, 0, 0, 0, 2), map(0, 0, 1, 1, 0, 0, 2, 0)] {
            kernel.process(&src, &mut dst, zero).unwrap();
            assert_eq!(rgba8_px(&dst), vec![[0, 0, 0, 0]; 4]);
        }
    }

    /// A `map.dst` fully outside the canvas — beyond the right/bottom
    /// edge or entirely left/above it — is not an error either: same
    /// defined all-transparent-black result, bounded and panic-free.
    #[test]
    fn off_canvas_dst_rect_writes_all_transparent_black() {
        let src = rgba8_frame(1, 1, &[[9, 8, 7, 6]]);
        let mut dst = rgba8_frame(2, 1, &[[200; 4]; 2]);
        let mut kernel = AffineTransform::new();
        for off in [
            map(0, 0, 1, 1, 2, 0, 2, 1),  // x ≥ width
            map(0, 0, 1, 1, 0, 1, 2, 1),  // y ≥ height
            map(0, 0, 1, 1, -4, 0, 2, 1), // x + w ≤ 0
            map(0, 0, 1, 1, 0, -4, 2, 2), // y + h ≤ 0
        ] {
            kernel.process(&src, &mut dst, off).unwrap();
            assert_eq!(rgba8_px(&dst), vec![[0, 0, 0, 0]; 2]);
        }
    }

    /// `map.src` validation — empty (`w == 0`/`h == 0`), negative
    /// origin, and `x + w`/`y + h` past the frame edges all reject as
    /// `InvalidValue("src rect")` before any write; `dst` is untouched.
    #[test]
    fn rejects_src_rects_outside_frame() {
        let src = rgba8_frame(2, 2, &[[1; 4]; 4]);
        let mut dst = rgba8_frame(2, 2, &[[9; 4]; 4]);
        let mut kernel = AffineTransform::new();
        for bad in [
            map(0, 0, 0, 2, 0, 0, 2, 2),  // empty: w == 0
            map(0, 0, 2, 0, 0, 0, 2, 2),  // empty: h == 0
            map(-1, 0, 2, 2, 0, 0, 2, 2), // x < 0
            map(0, -1, 2, 2, 0, 0, 2, 2), // y < 0
            map(1, 0, 2, 2, 0, 0, 2, 2),  // x + w > width
            map(0, 1, 2, 2, 0, 0, 2, 2),  // y + h > height
        ] {
            assert_eq!(
                kernel.process(&src, &mut dst, bad),
                Err(Error::InvalidValue("src rect")),
                "{bad:?}"
            );
        }
        assert_eq!(rgba8_px(&dst), vec![[9; 4]; 4]);
    }

    /// `map.src` may sit anywhere inside `src` including the far edges
    /// — the `x + w == width` boundary is in-range.
    #[test]
    fn src_rect_at_far_edges_is_in_range() {
        let src = rgba8_frame(
            2,
            2,
            &[[1, 1, 1, 1], [2, 2, 2, 2], [3, 3, 3, 3], [4, 4, 4, 4]],
        );
        let mut dst = rgba8_frame(1, 1, &[[0; 4]]);
        AffineTransform::new()
            .process(&src, &mut dst, map(1, 1, 1, 1, 0, 0, 1, 1))
            .unwrap();
        assert_eq!(rgba8_px(&dst), vec![[4, 4, 4, 4]]);
    }

    /// Format violations — non-RGBA `src`, `dst` format disagreement —
    /// and a shared `dst` backing are rejected before any write.
    #[test]
    fn rejects_invalid_frames() {
        let src = rgba8_frame(1, 1, &[[1, 2, 3, 4]]);
        let mut dst = rgba8_frame(1, 1, &[[9; 4]]);
        let mut kernel = AffineTransform::new();
        let full = map(0, 0, 1, 1, 0, 0, 1, 1);

        // Non-RGBA source.
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_yuv, &mut dst, full),
            Err(Error::InvalidValue("src format"))
        );

        // src↔dst format disagreement — dimension mismatch is allowed
        // for this kernel, so the f16 dst is 2×1 on purpose.
        let mut dst16 = f16_frame(2, 1, &[[0x3C00; 4]; 2]);
        assert_eq!(
            kernel.process(&src, &mut dst16, full),
            Err(Error::InvalidValue("dst format"))
        );

        // Shared `dst` backing — including a clone of `src` itself.
        let sharing = dst.clone();
        assert_eq!(
            kernel.process(&src, &mut dst, full),
            Err(Error::InvalidValue("dst plane"))
        );
        drop(sharing);
        let mut dst_aliasing_src = src.clone();
        assert_eq!(
            kernel.process(&src, &mut dst_aliasing_src, full),
            Err(Error::InvalidValue("dst plane"))
        );

        // No rejection wrote anything.
        assert_eq!(rgba8_px(&dst), vec![[9; 4]]);
    }

    /// `RgbaF16` path — same mapping as `Rgba8`, pixels copied as
    /// opaque bytes (bit-exact): a 2→4 upscale replicates
    /// `[0,0,1,1]`, `binary16` patterns and all.
    #[test]
    fn f16_upscale_copies_pixels_bit_exact() {
        let src = f16_frame(
            2,
            1,
            &[
                [0x3C00, 0x3800, 0x3400, 0x3C00],
                [0x4000, 0x3C00, 0x3800, 0x3400],
            ],
        );
        let mut dst = f16_frame(4, 1, &[[0x0000; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 4, 1))
            .unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x3C00, 0x3800, 0x3400, 0x3C00]);
        assert_eq!(f16_px(&dst, 1, 0), [0x3C00, 0x3800, 0x3400, 0x3C00]);
        assert_eq!(f16_px(&dst, 2, 0), [0x4000, 0x3C00, 0x3800, 0x3400]);
        assert_eq!(f16_px(&dst, 3, 0), [0x4000, 0x3C00, 0x3800, 0x3400]);
    }

    /// `RgbaF32` path — same mapping as `Rgba8`/`RgbaF16`, pixels
    /// copied as opaque bytes (bit-exact): a 2→4 upscale replicates
    /// `[0,0,1,1]`, `binary32` patterns and all — including a NaN
    /// payload, `−0.0` and a subnormal, which proves no arithmetic is
    /// applied to the channel values.
    #[test]
    fn f32_upscale_copies_pixels_bit_exact() {
        let src = f32_frame(
            2,
            1,
            &[
                [0x3F80_0000, 0x8000_0000, 0x7FC0_0001, 0x0000_0001],
                [0x4000_0000, 0x3F00_0000, 0x3E80_0000, 0x3E00_0000],
            ],
        );
        let mut dst = f32_frame(4, 1, &[[0x0000_0000; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 4, 1))
            .unwrap();
        assert_eq!(
            f32_px(&dst, 0, 0),
            [0x3F80_0000, 0x8000_0000, 0x7FC0_0001, 0x0000_0001]
        );
        assert_eq!(
            f32_px(&dst, 1, 0),
            [0x3F80_0000, 0x8000_0000, 0x7FC0_0001, 0x0000_0001]
        );
        assert_eq!(
            f32_px(&dst, 2, 0),
            [0x4000_0000, 0x3F00_0000, 0x3E80_0000, 0x3E00_0000]
        );
        assert_eq!(
            f32_px(&dst, 3, 0),
            [0x4000_0000, 0x3F00_0000, 0x3E80_0000, 0x3E00_0000]
        );
    }

    /// `map.dst` partially or fully off-canvas on `RgbaF32`: uncovered
    /// canvas becomes transparent black — zero bytes, i.e. `+0.0`
    /// `binary32` channels.
    #[test]
    fn f32_off_canvas_fills_transparent_black() {
        let src = f32_frame(1, 1, &[[0x3F80_0000; 4]]);
        let mut dst = f32_frame(3, 1, &[[0x7F80_0000; 4]; 3]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 1, 0, 1, 1))
            .unwrap();
        assert_eq!(f32_px(&dst, 0, 0), [0; 4]);
        assert_eq!(f32_px(&dst, 1, 0), [0x3F80_0000; 4]);
        assert_eq!(f32_px(&dst, 2, 0), [0; 4]);

        // Fully off-canvas `map.dst`: every pixel transparent black.
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 3, 0, 1, 1))
            .unwrap();
        for x in 0..3 {
            assert_eq!(f32_px(&dst, x, 0), [0; 4], "x {x}");
        }
    }

    /// Format disagreements on `RgbaF32` — `dst` in another format —
    /// and a shared `dst` backing are rejected before any write.
    #[test]
    fn f32_rejects_mismatched_formats() {
        let src32 = f32_frame(1, 1, &[[0x3F80_0000; 4]]);
        let mut dst8 = rgba8_frame(1, 1, &[[9; 4]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst32 = f32_frame(1, 1, &[[0; 4]]);
        let mut kernel = AffineTransform::new();
        let full = map(0, 0, 1, 1, 0, 0, 1, 1);

        assert_eq!(
            kernel.process(&src32, &mut dst8, full),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&src32, &mut dst16, full),
            Err(Error::InvalidValue("dst format"))
        );

        // Shared `dst` backing.
        let sharing = dst32.clone();
        assert_eq!(
            kernel.process(&src32, &mut dst32, full),
            Err(Error::InvalidValue("dst plane"))
        );
        drop(sharing);

        // No rejection wrote anything.
        assert_eq!(f32_px(&dst32, 0, 0), [0; 4]);
    }

    /// `dst` takes `src`'s `ColorInfo`/`pts`/`duration` — a transform
    /// keeps the source frame's identity.
    #[test]
    fn propagates_src_metadata() {
        let mut src = rgba8_frame(1, 1, &[[9, 8, 7, 6]]);
        let src_pts = MediaTime::new(7, TimeBase::COMMON_90K);
        let duration = MediaDuration::new(9_000, TimeBase::COMMON_90K);
        src.set_pts(src_pts);
        src.set_duration(Some(duration));
        src.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        let mut dst = rgba8_frame(2, 1, &[[0; 4]; 2]);
        dst.set_pts(MediaTime::ZERO);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 0, 0, 2, 1))
            .unwrap();
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), src_pts);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Same propagation on the `RgbaF32` arm — the transform keeps
    /// the source frame's identity there too.
    #[test]
    fn f32_propagates_src_metadata() {
        let mut src = f32_frame(1, 1, &[[0x3F80_0000; 4]]);
        let src_pts = MediaTime::new(7, TimeBase::COMMON_90K);
        let duration = MediaDuration::new(9_000, TimeBase::COMMON_90K);
        src.set_pts(src_pts);
        src.set_duration(Some(duration));
        src.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        let mut dst = f32_frame(2, 1, &[[0; 4]; 2]);
        dst.set_pts(MediaTime::ZERO);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 1, 0, 0, 2, 1))
            .unwrap();
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), src_pts);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// `stride > row_bytes` on both sides: rows are read/written
    /// through `row`/`row_mut` only — inter-row padding is never
    /// touched on `dst` and never sampled on `src`.
    #[test]
    fn padded_strides_are_never_touched() {
        // 1×2 `Rgba8`: row_bytes 4, padded stride 6 → usable
        // `6·(2−1) + 4` = 10 bytes; offsets 4–5 are sentinels.
        let padded = |fill: u8| {
            VideoFrame::from_shared_planes(
                PixelFormat::Rgba8,
                1,
                2,
                pts(),
                vec![SharedPlane {
                    data: Arc::from(
                        vec![fill, fill, fill, 255, 0xEE, 0xEE, fill, fill, fill, 255]
                            .into_boxed_slice(),
                    ),
                    stride: 6,
                }],
            )
            .unwrap()
        };

        let src = padded(9);
        let mut dst = padded(0);
        assert_eq!(src.plane(0).unwrap().stride(), 6);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 1, 2, 0, 0, 1, 2))
            .unwrap();
        // Identity transform copies row bytes only — padding bytes
        // keep their `0xEE` sentinels on `dst`.
        assert_eq!(
            dst.plane(0).unwrap().data(),
            &[9, 9, 9, 255, 0xEE, 0xEE, 9, 9, 9, 255]
        );
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_affine_transform_is_repeatable_and_deterministic() {
        let src = rgba8_frame(2, 1, &[[10, 0, 0, 10], [20, 0, 0, 20]]);
        let mut dst = rgba8_frame(4, 1, &[[0; 4]; 4]);
        let mut kernel = AffineTransform::new();
        for _ in 0..64 {
            kernel
                .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 4, 1))
                .unwrap();
            assert_eq!(
                rgba8_px(&dst),
                vec![
                    [10, 0, 0, 10],
                    [10, 0, 0, 10],
                    [20, 0, 0, 20],
                    [20, 0, 0, 20]
                ]
            );
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// `src` stays byte-identical across the call (a borrow, not a
    /// drain).
    #[test]
    fn rt_process_affine_transform_never_mutates_source() {
        let src = rgba8_frame(
            2,
            2,
            &[
                [1, 2, 3, 4],
                [5, 6, 7, 8],
                [9, 10, 11, 12],
                [13, 14, 15, 16],
            ],
        );
        let before: [u8; 16] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = rgba8_frame(2, 2, &[[0; 4]; 4]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 2, 0, 0, 1, 1))
            .unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }

    /// RT contract on the `RgbaF32` arm: `process` is allocation-free,
    /// non-blocking and bounded — observably: repeated calls on the
    /// same kernel succeed and stay deterministic, never panic.
    #[test]
    fn rt_process_affine_transform_f32_is_repeatable_and_deterministic() {
        let src = f32_frame(2, 1, &[[0x3F80_0000; 4], [0x4000_0000; 4]]);
        let mut dst = f32_frame(4, 1, &[[0; 4]; 4]);
        let mut kernel = AffineTransform::new();
        for _ in 0..64 {
            kernel
                .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 4, 1))
                .unwrap();
            assert_eq!(
                (0..4).map(|x| f32_px(&dst, x, 0)).collect::<Vec<_>>(),
                vec![
                    [0x3F80_0000; 4],
                    [0x3F80_0000; 4],
                    [0x4000_0000; 4],
                    [0x4000_0000; 4],
                ]
            );
        }
    }

    /// RT contract on the `RgbaF32` arm: `process` writes only the
    /// caller-provided `dst` — `src` stays byte-identical across the
    /// call.
    #[test]
    fn rt_process_affine_transform_f32_never_mutates_source() {
        let src = f32_frame(
            2,
            1,
            &[
                [0x3F80_0000, 0x8000_0000, 0x7FC0_0001, 0x0000_0001],
                [0x4000_0000; 4],
            ],
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = f32_frame(2, 1, &[[0; 4]; 2]);
        AffineTransform::new()
            .process(&src, &mut dst, map(0, 0, 2, 1, 0, 0, 2, 1))
            .unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }
}
