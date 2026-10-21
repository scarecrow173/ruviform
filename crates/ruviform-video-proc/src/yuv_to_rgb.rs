//! `Yuv*p → Rgba8` color-conversion kernel: [`YuvToRgb`] converts planar
//! YUV frames ([`PixelFormat::Yuv420p`], [`PixelFormat::Yuv422p`],
//! [`PixelFormat::Yuv444p`]) into packed RGBA8, honoring the frame's
//! matrix (`Bt601`/`Bt709`/`Bt2020`) and quantization range
//! (`Limited`/`Full`) and reconstructing subsampled chroma onto the
//! luma grid.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{ColorRange, MatrixCoefficients, PixelFormat, VideoFrame};

use crate::coef::{self, MATRIX_SLOTS, RANGE_SLOTS, YuvCoef};

/// Prepared `Yuv*p → Rgba8` conversion kernel — the two-phase API shape
/// of ADR D1-b: [`new`](Self::new) is the non-RT `prepare` step that
/// bakes the Q16 coefficient table (all six matrix × range pairs), and
/// [`process`](Self::process) is the allocation-free conversion step
/// reusable for any number of frames.
///
/// ## Conversion rules
///
/// The effective interpretation is read from `src.color()` on every
/// call — the conversion references the input frame's [`ColorInfo`](
/// ruviform_video_core::ColorInfo), so one prepared kernel serves
/// streams whose signaled matrix/range changes mid-stream:
///
/// - `matrix`: `Bt601`/`Bt709`/`Bt2020` select the corresponding
///   H.273-style weighting; `Unspecified` is interpreted as `Bt601`
///   (the SD-era historical default). `Identity` — content already RGB —
///   and unknown future variants make `process` fail with
///   [`Error::InvalidValue`] instead of panicking or silently
///   mis-converting.
/// - `range`: `Limited`/`Full` select the code-space scaling;
///   `Unspecified` is interpreted as `Limited`.
///
/// The math is code-value arithmetic only: `primaries`/`transfer`/`hdr`
/// are never consulted — no transfer-function application (e.g. PQ/HLG
/// linearization) and no tone mapping. HDR scope = metadata carriage +
/// non-destructive (ADR D3-a).
///
/// ## Chroma upsampling
///
/// `Yuv444p` chroma is already per-pixel. For `Yuv420p`/`Yuv422p` each
/// output pixel replicates the chroma sample covering it — chroma sample
/// `(cx, cy)` serves luma columns `2cx..=2cx+1` (and rows `2cy..=2cy+1`
/// for 4:2:0) — i.e. nearest-sample upsampling under the documented
/// siting assumption: **horizontally co-sited, vertically centered**.
/// `PixelFormat`/`ColorInfo` cannot express siting, so this assumption
/// is fixed here; differently-sited content may show a half-pixel chroma
/// shift (known limitation). Odd luma dimensions use `div_ceil` chroma
/// grids — the last chroma column/row covers only the existing luma
/// samples; no nonexistent neighbor is synthesized.
///
/// ## Output frame
///
/// `dst` must be `PixelFormat::Rgba8` at the same `width`×`height` as
/// `src` and exclusively owned — a shared `dst` (`plane_mut(0)` →
/// `None`) is rejected. Alpha is written as 255 (opaque). Rows are
/// processed through `Plane::row`/`PlaneMut::row_mut` only, so
/// `stride > row_bytes` padding is never read or written.
///
/// The output `ColorInfo` records the applied conversion: `matrix` →
/// [`MatrixCoefficients::Identity`] (content is now RGB) and `range` →
/// [`ColorRange::Full`] (the emitted RGB always spans the full code
/// space); `primaries`/`transfer`/`hdr` propagate unchanged.
/// `dst`'s `pts`/`duration` are overwritten with `src`'s — a color
/// conversion never retimes the frame (plan open question Q4).
///
/// ## Equivalence contract
///
/// This scalar implementation is the reference implementation per the
/// baseline DSP/SIMD decision: it uses Q16 fixed-point integer math
/// only, so the contract for any future optimized backend is
/// **bit-exact** equality of every output byte.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{ColorInfo, MatrixCoefficients, PixelFormat, VideoFrame};
/// use ruviform_video_proc::YuvToRgb;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts).unwrap();
/// src.set_color(ColorInfo {
///     matrix: MatrixCoefficients::Bt709,
///     ..ColorInfo::default()
/// });
/// // Neutral gray sample: Y = Cb = Cr = 128.
/// for plane in 0..3 {
///     src.plane_mut(plane).unwrap().data_mut()[0] = 128;
/// }
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
///
/// let mut kernel = YuvToRgb::new();
/// kernel.process(&src, &mut dst).unwrap();
///
/// // Neutral YUV → gray RGB, alpha 255.
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[130, 130, 130, 255]);
/// assert_eq!(dst.color().matrix, MatrixCoefficients::Identity);
/// ```
#[derive(Debug)]
pub struct YuvToRgb {
    /// Baked Q16 coefficients, indexed by [`coef::matrix_slot`] ×
    /// [`coef::range_slot`].
    coef: [[YuvCoef; RANGE_SLOTS]; MATRIX_SLOTS],
}

impl YuvToRgb {
    /// Builds the prepared kernel, baking the Q16 coefficient table for
    /// every supported matrix × range pair.
    ///
    /// **RT contract:** non-RT only — coefficient derivation uses
    /// floating-point math; run on the setup path, not the media path.
    /// (The table is stored inline — even here no allocation happens.)
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::YuvToRgb::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            coef: coef::yuv_to_rgb_table(),
        }
    }

    /// Converts `src` into `dst` per the rules on [`YuvToRgb`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not a planar `Yuv*p` format,
    /// `dst` is not `Rgba8`, the frames disagree on dimensions, `src`'s
    /// `color.matrix`/`color.range` is `Identity` or an unknown future
    /// variant, or `dst`'s backing is still shared
    /// (`dst.plane_mut(0)` → `None`). Every check runs before any write —
    /// a failed call never partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (exactly `width`×`height` pixel iterations over borrowed
    /// rows; the coefficient table was baked by [`new`](Self::new)).
    /// Dropping `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        let (sub_w, sub_h): (usize, u32) = match src.format() {
            PixelFormat::Yuv420p => (2, 2),
            PixelFormat::Yuv422p => (2, 1),
            PixelFormat::Yuv444p => (1, 1),
            _ => return Err(Error::InvalidValue("src format")),
        };
        if dst.format() != PixelFormat::Rgba8 {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        let coef =
            &self.coef[coef::matrix_slot(src_color.matrix)?][coef::range_slot(src_color.range)?];
        let luma = src.plane(0).ok_or(Error::InvalidValue("src planes"))?;
        let blue_plane = src.plane(1).ok_or(Error::InvalidValue("src planes"))?;
        let red_plane = src.plane(2).ok_or(Error::InvalidValue("src planes"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..src.height() {
                // These row borrows cannot fail: `y < height`, and the
                // `div_ceil` chroma grids keep `cy`/`cx` in range —
                // `ok_or` keeps the contract panic-free regardless.
                let cy = y / sub_h;
                let luma_row = luma.row(y).ok_or(Error::InvalidValue("src planes"))?;
                let blue_row = blue_plane
                    .row(cy)
                    .ok_or(Error::InvalidValue("src planes"))?;
                let red_row = red_plane.row(cy).ok_or(Error::InvalidValue("src planes"))?;
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                // `row_bytes == width * 4` for Rgba8, so exactly `width`
                // pixel chunks are produced — padding never included.
                for (x, px) in dst_row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let cx = x / sub_w;
                    let [r, g, b] = coef.apply(luma_row[x], blue_row[cx], red_row[cx]);
                    px.copy_from_slice(&[r, g, b, u8::MAX]);
                }
            }
        }
        // Metadata: the content is now RGB in the full code space;
        // primaries/transfer/hdr propagate untouched, and a color
        // conversion never retimes the frame (pts/duration pass through).
        let mut dst_color = src_color;
        dst_color.matrix = MatrixCoefficients::Identity;
        dst_color.range = ColorRange::Full;
        dst.set_color(dst_color);
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for YuvToRgb {
    /// Equivalent to [`YuvToRgb::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ruviform_media_core::{MediaTime, TimeBase};
    use ruviform_video_core::{
        ColorInfo, ColorPrimaries, ColorRange, ContentLightLevel, HdrMetadata, MatrixCoefficients,
        PixelFormat, SharedPlane, TransferCharacteristics, VideoFrame,
    };

    use super::*;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Builds a `Yuv444p` frame holding exactly one pixel.
    fn one_pixel_444(y: u8, cb: u8, cr: u8, color: ColorInfo) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        for (i, value) in [y, cb, cr].iter().enumerate() {
            frame.plane_mut(i).unwrap().data_mut()[0] = *value;
        }
        frame.set_color(color);
        frame
    }

    /// Converts a single `Yuv444p` pixel and returns its RGBA bytes.
    fn pixel_444(kernel: &mut YuvToRgb, y: u8, cb: u8, cr: u8, color: ColorInfo) -> [u8; 4] {
        let src = one_pixel_444(y, cb, cr, color);
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        [row[0], row[1], row[2], row[3]]
    }

    /// Golden vectors — every supported matrix × range pair against
    /// known pixels, at the documented Q16 bit-exact contract.
    ///
    /// Expected values are derived from the H.273 weightings through the
    /// documented integer pipeline — worked example, Bt601/Limited
    /// `(Y, Cb, Cr) = (81, 90, 240)` (the BT.601 "red" bar color):
    ///
    /// ```text
    /// y' = 76309 · (81 − 16) = 4 960 085              (Q16 domain)
    /// R  = (4 960 085 + 104 597·112 + 2¹⁵) >> 16 = 254
    /// G  = (4 960 085 − 25 675·(−38) − 53 279·112 + 2¹⁵) >> 16 = 0
    /// B  = (4 960 085 + 132 201·(−38) + 2¹⁵) >> 16 = −1 → clamp 0
    /// ```
    ///
    /// `(145, 54, 34)` is the BT.601 "green" bar color; all remaining
    /// expectations are derived identically (the full-range columns use
    /// unity gains — `y_offset` 0 and no code-space rescaling).
    /// Bt709/Bt2020 Limited clamp `R` and `B` on the bar colors — the
    /// known out-of-range case the integer pipeline must saturate.
    #[test]
    fn golden_vectors_cover_every_matrix_range_pair() {
        let pixels = [
            (128_u8, 128_u8, 128_u8), // neutral gray
            (81, 90, 240),            // saturated red-ish (Bt601 bar)
            (145, 54, 34),            // saturated green-ish (Bt601 bar)
            (255, 255, 255),          // code-space extremes
        ];
        let cases = [
            (
                MatrixCoefficients::Bt601,
                ColorRange::Limited,
                [
                    [130, 130, 130, 255],
                    [254, 0, 0, 255],
                    [0, 255, 1, 255],
                    [255, 125, 255, 255],
                ],
            ),
            (
                MatrixCoefficients::Bt601,
                ColorRange::Full,
                [
                    [128, 128, 128, 255],
                    [238, 14, 14, 255],
                    [13, 238, 14, 255],
                    [255, 121, 255, 255],
                ],
            ),
            (
                MatrixCoefficients::Bt709,
                ColorRange::Limited,
                [
                    [130, 130, 130, 255],
                    [255, 24, 0, 255],
                    [0, 216, 0, 255],
                    [255, 184, 255, 255],
                ],
            ),
            (
                MatrixCoefficients::Bt709,
                ColorRange::Full,
                [
                    [128, 128, 128, 255],
                    [255, 36, 10, 255],
                    [0, 203, 8, 255],
                    [255, 172, 255, 255],
                ],
            ),
            (
                MatrixCoefficients::Bt2020,
                ColorRange::Limited,
                [
                    [130, 130, 130, 255],
                    [255, 10, 0, 255],
                    [0, 225, 0, 255],
                    [255, 172, 255, 255],
                ],
            ),
            (
                MatrixCoefficients::Bt2020,
                ColorRange::Full,
                [
                    [128, 128, 128, 255],
                    [246, 23, 10, 255],
                    [6, 211, 6, 255],
                    [255, 162, 255, 255],
                ],
            ),
        ];
        let mut kernel = YuvToRgb::new();
        for (matrix, range, expected) in cases {
            for ((y, cb, cr), want) in pixels.iter().zip(expected.iter()) {
                let color = ColorInfo {
                    matrix,
                    range,
                    ..ColorInfo::default()
                };
                assert_eq!(
                    pixel_444(&mut kernel, *y, *cb, *cr, color),
                    *want,
                    "matrix {matrix:?} range {range:?} pixel ({y}, {cb}, {cr})"
                );
            }
        }
    }

    /// 4:2:0 chroma upsampling — including the odd-dimension rule: a
    /// 5×3 luma grid pairs with a 3×2 chroma grid, and the last chroma
    /// column/row must cover only the luma samples that exist.
    ///
    /// Six distinct `Cr` values make the covering cell directly visible
    /// in `R`: Bt601/Limited, `Y` = 128, `Cb` = 128:
    ///
    /// ```text
    /// cell (cx, cy):  (0,0) (1,0) (2,0)   Cr = 100, 140, 180 → R =  86, 150, 213
    ///                 (0,1) (1,1) (2,1)   Cr = 120, 160, 220 → R = 118, 181, 255
    /// ```
    ///
    /// e.g. cell (1,1): R = (76309·112 + 104 597·32 + 2¹⁵) >> 16 = 181.
    /// Luma columns 4 and row 2 must read cells (2, ·) and (·, 1) — the
    /// rightmost/bottom cells — never an imagined neighbor.
    #[test]
    fn chroma_upsample_420_covers_existing_cells_only() {
        let mut src = VideoFrame::new_owned(PixelFormat::Yuv420p, 5, 3, pts()).unwrap();
        src.plane_mut(0)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[128_u8; 15]);
        for (plane, values) in [
            (1_usize, [128_u8, 128, 128, 128, 128, 128]),
            (2, [100_u8, 140, 180, 120, 160, 220]),
        ] {
            src.plane_mut(plane)
                .unwrap()
                .data_mut()
                .copy_from_slice(&values);
        }
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 5, 3, pts()).unwrap();
        YuvToRgb::new().process(&src, &mut dst).unwrap();

        // Expected R per output row — chroma replicated across its
        // covering luma columns/rows; last column/row use cell (2, cy)
        // and (cx, 1) respectively.
        let expected_r = [
            [86_u8, 86, 150, 150, 213],
            [86, 86, 150, 150, 213],
            [118, 118, 181, 181, 255],
        ];
        for (y, row) in expected_r.iter().enumerate() {
            let dst_row = dst
                .plane(0)
                .unwrap()
                .row(u32::try_from(y).unwrap())
                .unwrap();
            for (x, want) in row.iter().enumerate() {
                assert_eq!(dst_row[x * 4], *want, "pixel ({x}, {y}) R channel");
            }
        }
    }

    /// 4:2:2 chroma upsampling — vertical neighbor is per-row (chroma
    /// height == luma height), horizontal pairs share a cell.
    #[test]
    fn chroma_upsample_422_shares_cells_horizontally() {
        let mut src = VideoFrame::new_owned(PixelFormat::Yuv422p, 3, 2, pts()).unwrap();
        src.plane_mut(0)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[128_u8; 6]);
        // Chroma grid: 2×2 (ceil(3/2) × 2). Cb cells all 128; Cr cells
        // 100/140 on row 0, 200/220 on row 1 → R = 86/150 / 245/255.
        for (plane, values) in [
            (1_usize, [128_u8, 128, 128, 128]),
            (2, [100_u8, 140, 200, 220]),
        ] {
            src.plane_mut(plane)
                .unwrap()
                .data_mut()
                .copy_from_slice(&values);
        }
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 3, 2, pts()).unwrap();
        YuvToRgb::new().process(&src, &mut dst).unwrap();

        // R = (76309·112 + 104 597·(Cr−128) + 2¹⁵) >> 16: Cr 200→245,
        // 220→255(clamped from 255.4).
        let expected_r = [[86_u8, 86, 150], [245, 245, 255]];
        for (y, row) in expected_r.iter().enumerate() {
            let dst_row = dst
                .plane(0)
                .unwrap()
                .row(u32::try_from(y).unwrap())
                .unwrap();
            for (x, want) in row.iter().enumerate() {
                assert_eq!(dst_row[x * 4], *want, "pixel ({x}, {y}) R channel");
            }
        }
    }

    /// A `stride > row_bytes` destination: conversion must write rows
    /// only — padding bytes keep their caller-chosen contents. (Two rows
    /// are needed: the usable region ends at the last row's `row_bytes`,
    /// so only *inter-row* padding is observable through the view.)
    #[test]
    fn padded_dst_stride_is_never_written() {
        // 1×2 `Yuv444p` source, uniform neutral gray.
        let mut src = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 2, pts()).unwrap();
        for i in 0..3 {
            src.plane_mut(i)
                .unwrap()
                .data_mut()
                .copy_from_slice(&[128_u8; 2]);
        }
        // 1×2 Rgba8 dst: row_bytes 4, padded stride 6 → usable
        // `6·(2−1) + 4` = 10 bytes; offsets 4–5 are inter-row padding
        // carrying sentinels, the 10-byte backing follows the padded
        // stride exactly.
        let mut dst = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(vec![0_u8, 0, 0, 0, 0xEE, 0xEE, 0, 0, 0, 0].into_boxed_slice()),
                stride: 6,
            }],
        )
        .unwrap();
        assert_eq!(dst.plane(0).unwrap().stride(), 6);
        YuvToRgb::new().process(&src, &mut dst).unwrap();
        // Both rows written (neutral → gray, alpha 255), padding intact.
        assert_eq!(
            dst.plane(0).unwrap().data(),
            &[130, 130, 130, 255, 0xEE, 0xEE, 130, 130, 130, 255]
        );
    }

    /// A shared `dst` (clone still holding the `Arc` backing) must be
    /// rejected — `plane_mut` → `None` — with no partial write.
    #[test]
    fn shared_dst_backing_is_rejected() {
        let src = one_pixel_444(128, 128, 128, ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let sharing_clone = dst.clone();
        assert!(dst.plane_mut(0).is_none());
        assert_eq!(
            YuvToRgb::new().process(&src, &mut dst),
            Err(Error::InvalidValue("dst plane"))
        );
        // Rejection wrote nothing.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0, 0, 0, 0]);
        drop(sharing_clone);
        assert!(YuvToRgb::new().process(&src, &mut dst).is_ok());
    }

    /// Format/dimension mismatches and `Identity`/unknown color metadata
    /// are all rejected before any write.
    #[test]
    fn invalid_inputs_are_rejected() {
        let mut kernel = YuvToRgb::new();
        let src_444 = one_pixel_444(128, 128, 128, ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();

        // Packed or RGB source.
        let src_rgb = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_rgb, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Non-Rgba8 destination.
        let mut dst_yuv = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_444, &mut dst_yuv),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_444, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // `Identity` matrix — RGB content has no YCbCr matrix to undo.
        let src_identity = one_pixel_444(
            128,
            128,
            128,
            ColorInfo {
                matrix: MatrixCoefficients::Identity,
                ..ColorInfo::default()
            },
        );
        assert_eq!(
            kernel.process(&src_identity, &mut dst),
            Err(Error::InvalidValue("color matrix"))
        );

        // Nothing was written by any rejection.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0, 0, 0, 0]);
    }

    /// `Unspecified` color fields resolve under the documented default
    /// rules: matrix → Bt601, range → Limited.
    #[test]
    fn unspecified_color_fields_use_defaults() {
        let mut kernel = YuvToRgb::new();
        let unspecified = ColorInfo::default();
        let explicit_601_limited = ColorInfo {
            matrix: MatrixCoefficients::Bt601,
            range: ColorRange::Limited,
            ..ColorInfo::default()
        };
        for pixel in [(128, 128, 128), (81, 90, 240)] {
            assert_eq!(
                pixel_444(&mut kernel, pixel.0, pixel.1, pixel.2, unspecified),
                pixel_444(&mut kernel, pixel.0, pixel.1, pixel.2, explicit_601_limited)
            );
        }
    }

    /// Output `ColorInfo` records the applied conversion (matrix →
    /// `Identity`, range → `Full`) while primaries/transfer/hdr and the
    /// timestamps propagate unchanged.
    #[test]
    fn output_color_and_time_propagate() {
        let duration = ruviform_media_core::MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let mut src = one_pixel_444(
            128,
            128,
            128,
            ColorInfo {
                primaries: ColorPrimaries::Bt2020,
                transfer: TransferCharacteristics::Pq,
                matrix: MatrixCoefficients::Bt709,
                range: ColorRange::Limited,
                hdr: Some(HdrMetadata {
                    content_light_level: Some(ContentLightLevel {
                        max_cll: 1_000,
                        max_fall: 400,
                    }),
                    ..HdrMetadata::default()
                }),
            },
        );
        src.set_duration(Some(duration));
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        dst.set_pts(MediaTime::ZERO);
        YuvToRgb::new().process(&src, &mut dst).unwrap();

        let color = dst.color();
        assert_eq!(color.matrix, MatrixCoefficients::Identity);
        assert_eq!(color.range, ColorRange::Full);
        // Non-destructive carriage: untouched fields propagate as-is
        // (PQ is *carried*, never applied).
        assert_eq!(color.primaries, ColorPrimaries::Bt2020);
        assert_eq!(color.transfer, TransferCharacteristics::Pq);
        assert_eq!(
            color.hdr.unwrap().content_light_level.unwrap().max_cll,
            1_000
        );
        assert_eq!(dst.pts(), pts());
        assert_eq!(dst.duration(), Some(duration));
    }

    /// RT contract: a prepared kernel's `process` is allocation-free,
    /// non-blocking and bounded — observably: repeated calls on the same
    /// kernel succeed and stay deterministic, never panic.
    #[test]
    fn rt_process_is_repeatable_and_deterministic() {
        let src = one_pixel_444(
            81,
            90,
            240,
            ColorInfo {
                matrix: MatrixCoefficients::Bt709,
                range: ColorRange::Limited,
                ..ColorInfo::default()
            },
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut kernel = YuvToRgb::new();
        for _ in 0..64 {
            kernel.process(&src, &mut dst).unwrap();
            assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[255, 24, 0, 255]);
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// never allocates a new frame, and `src` stays byte-identical
    /// across the call (a borrow, not a drain).
    #[test]
    fn rt_process_never_mutates_source() {
        let mut src = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts()).unwrap();
        src.plane_mut(0)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[100_u8; 8]);
        // 4×2 Yuv420p chroma planes are 2×1 = 2 bytes each.
        for i in 1..3 {
            src.plane_mut(i)
                .unwrap()
                .data_mut()
                .copy_from_slice(&[128_u8; 2]);
        }
        let luma_before: [u8; 8] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 4, 2, pts()).unwrap();
        YuvToRgb::new().process(&src, &mut dst).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &luma_before);
    }
}
