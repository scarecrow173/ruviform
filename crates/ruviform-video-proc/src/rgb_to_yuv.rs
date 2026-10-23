//! `Rgba8 → Yuv*p` color-conversion kernel: [`RgbToYuv`] converts packed
//! RGBA8 frames into planar YUV ([`PixelFormat::Yuv420p`],
//! [`PixelFormat::Yuv422p`], [`PixelFormat::Yuv444p`]), applying a
//! constructor-fixed matrix (`Bt601`/`Bt709`/`Bt2020`) and quantization
//! range (`Limited`/`Full`) and downsampling chroma onto the
//! destination's subsampled grid.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{ColorRange, MatrixCoefficients, PixelFormat, VideoFrame};

use crate::coef::{self, RgbCoef, clip8};

/// Prepared `Rgba8 → Yuv*p` conversion kernel — the two-phase API shape
/// of ADR D1-b: [`new`](Self::new) is the non-RT `prepare` step that
/// resolves and bakes the Q16 coefficient set for one matrix × range
/// pair, and [`process`](Self::process) is the allocation-free
/// conversion step reusable for any number of frames.
///
/// ## Conversion rules
///
/// Unlike [`YuvToRgb`](crate::YuvToRgb) — which reads the conversion
/// from the input frame's `ColorInfo` — this kernel takes the target
/// YUV interpretation from constructor arguments (plan open question
/// Q2): an RGB input's `matrix` is naturally `Identity`/`Unspecified`,
/// so reading `src.color().matrix` could not drive the conversion:
///
/// - `matrix`: `Bt601`/`Bt709`/`Bt2020` select the corresponding
///   H.273-style weighting; `Unspecified` resolves to `Bt601` (the
///   SD-era historical default). `Identity` — the output would not be
///   YCbCr — and unknown future variants make [`new`](Self::new) fail
///   with [`Error::InvalidValue`] instead of panicking or silently
///   mis-converting.
/// - `range`: `Limited`/`Full` select the code-space scaling;
///   `Unspecified` resolves to `Limited`.
///
/// The math is code-value arithmetic only: `primaries`/`transfer`/`hdr`
/// are never consulted — no transfer-function application (e.g. PQ/HLG
/// linearization) and no tone mapping. HDR scope = metadata carriage +
/// non-destructive (ADR D3-a).
///
/// ## Chroma downsampling
///
/// `dst`'s pixel format selects the subsampling: `Yuv444p` keeps one
/// chroma sample per pixel, `Yuv422p` averages each horizontal 1×2 luma
/// pair, and `Yuv420p` box-averages each 2×2 luma cell — the exact
/// inverse of the siting documented for [`YuvToRgb`](crate::YuvToRgb):
/// **horizontally co-sited, vertically centered**, i.e. chroma cell
/// `(cx, cy)` covers luma columns `2cx..=2cx+1` (and rows
/// `2cy..=2cy+1` for 4:2:0). `PixelFormat`/`ColorInfo` cannot express
/// siting, so this assumption is fixed here; differently-sited content
/// may show a half-pixel chroma shift (known limitation).
///
/// Odd luma dimensions use `div_ceil` chroma grids: the last chroma
/// column/row covers only the luma samples that exist (e.g. a 5-wide
/// right-edge cell averages 1 column, never a phantom neighbor) — a
/// cell therefore holds 1–4 luma pixels (1–2 for 4:2:2).
///
/// The quantization contract is *average, then quantize*: each luma
/// pixel's chroma is computed in the Q16 domain (pre-round, pre-clip),
/// the cell's values are averaged with round-half-up
/// (`(sum + n/2) / n`), and the result is quantized by the shared
/// `clip8` (round-half-up out of Q16, saturate to `[0, 255]`).
///
/// ## Output frame
///
/// `src` must be `PixelFormat::Rgba8`; `dst` must be one of
/// `Yuv420p`/`Yuv422p`/`Yuv444p` at the same `width`×`height` and
/// exclusively owned — a shared `dst` (`plane_mut(0..3)` → `None`) is
/// rejected before a single byte is written. Alpha is dropped: the YUV
/// formats carry no alpha channel. Rows are processed through
/// `Plane::row`/`PlaneMut::row_mut` only, so `stride > row_bytes`
/// padding is never read or written.
///
/// The output `ColorInfo` records the applied conversion: `matrix` →
/// the resolved matrix and `range` → the resolved range (constructor
/// arguments after `Unspecified` default resolution — never left
/// `Unspecified`); `primaries`/`transfer`/`hdr` propagate unchanged.
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
/// use ruviform_video_core::{ColorRange, MatrixCoefficients, PixelFormat, VideoFrame};
/// use ruviform_video_proc::RgbToYuv;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// // Pure BT.601 red.
/// src.plane_mut(0).unwrap().data_mut().copy_from_slice(&[255, 0, 0, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts).unwrap();
///
/// let mut kernel = RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited).unwrap();
/// kernel.process(&src, &mut dst).unwrap();
///
/// // The BT.601 bar color: (Y, Cb, Cr) = (81, 90, 240).
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[81]);
/// assert_eq!(dst.plane(1).unwrap().row(0).unwrap(), &[90]);
/// assert_eq!(dst.plane(2).unwrap().row(0).unwrap(), &[240]);
/// assert_eq!(dst.color().matrix, MatrixCoefficients::Bt601);
/// assert_eq!(dst.color().range, ColorRange::Limited);
/// ```
#[derive(Debug)]
pub struct RgbToYuv {
    /// Baked Q16 coefficients for the constructor-resolved matrix ×
    /// range pair.
    coef: RgbCoef,
    /// The resolved matrix actually applied — recorded on `dst`.
    matrix: MatrixCoefficients,
    /// The resolved range actually applied — recorded on `dst`.
    range: ColorRange,
}

impl RgbToYuv {
    /// Builds the prepared kernel, resolving and baking the Q16
    /// coefficient set for `matrix` × `range`.
    ///
    /// `Unspecified` fields resolve under the documented default rules
    /// (matrix → `Bt601`, range → `Limited`); `Identity` or an unknown
    /// future enum variant is rejected.
    ///
    /// **RT contract:** non-RT only — coefficient derivation uses
    /// floating-point math; run on the setup path, not the media path.
    /// (The coefficients are stored inline — even here no allocation
    /// happens.)
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `matrix` is `Identity` or an unknown
    /// future variant, or `range` is an unknown future variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_video_core::{ColorRange, MatrixCoefficients};
    /// use ruviform_video_proc::RgbToYuv;
    ///
    /// let kernel = RgbToYuv::new(MatrixCoefficients::Bt709, ColorRange::Full).unwrap();
    /// // `Identity` cannot produce YCbCr output.
    /// assert!(RgbToYuv::new(MatrixCoefficients::Identity, ColorRange::Full).is_err());
    /// ```
    pub fn new(matrix: MatrixCoefficients, range: ColorRange) -> Result<Self> {
        let matrix = coef::matrix_slot(matrix)?;
        let range = coef::range_slot(range)?;
        Ok(Self {
            coef: coef::rgb_to_yuv_table()[matrix][range],
            matrix: coef::RESOLVED_MATRIX[matrix],
            range: coef::RESOLVED_RANGE[range],
        })
    }

    /// Converts `src` into `dst` per the rules on [`RgbToYuv`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `Rgba8`, `dst` is not a
    /// planar `Yuv*p` format, the frames disagree on dimensions, or any
    /// `dst` plane's backing is still shared (`dst.plane_mut(i)` →
    /// `None`). Every check runs before any write — a failed call never
    /// partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (`width`×`height` luma iterations plus one pass over the
    /// `div_ceil` chroma grid per chroma plane, all over borrowed rows;
    /// coefficients were baked by [`new`](Self::new)). Dropping
    /// `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        if src.format() != PixelFormat::Rgba8 {
            return Err(Error::InvalidValue("src format"));
        }
        let (sub_w, sub_h): (usize, u32) = match dst.format() {
            PixelFormat::Yuv420p => (2, 2),
            PixelFormat::Yuv422p => (2, 1),
            PixelFormat::Yuv444p => (1, 1),
            _ => return Err(Error::InvalidValue("dst format")),
        };
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        let rgba = src.plane(0).ok_or(Error::InvalidValue("src plane"))?;
        // Verify every dst plane is exclusively owned before a single
        // byte is written: `plane_mut` yields `None` while a clone still
        // shares the backing. Each borrow is released immediately —
        // `plane_mut` borrows the whole frame, so only one mutable plane
        // view can exist at a time.
        for plane in 0..dst.plane_count() {
            if dst.plane_mut(plane).is_none() {
                return Err(Error::InvalidValue("dst plane"));
            }
        }
        let width = src.width() as usize;
        let height = src.height();
        {
            // Luma pass — full-resolution plane 0. Alpha is dropped:
            // YUV carries no alpha channel.
            let mut luma_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..height {
                // These row borrows cannot fail: `y < height` on a
                // well-formed frame — `ok_or` keeps the contract
                // panic-free regardless.
                let src_row = rgba.row(y).ok_or(Error::InvalidValue("src plane"))?;
                let luma_row = luma_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                // `row_bytes == width * 4` for Rgba8, so exactly `width`
                // pixel chunks pair with the `width` luma samples —
                // padding never included.
                for (px, y_out) in src_row.as_chunks::<4>().0.iter().zip(luma_row.iter_mut()) {
                    *y_out = self.coef.luma(px[0], px[1], px[2]);
                }
            }
        }
        // Chroma passes — one per plane. Two mutable plane views of `dst`
        // cannot coexist (`plane_mut` borrows the whole frame), so Cb
        // and Cr are written in separate bounded passes; each luma
        // pixel's Q16 chroma is re-evaluated once per pass.
        for plane_index in 1..=2_usize {
            let mut chroma_plane = dst
                .plane_mut(plane_index)
                .ok_or(Error::InvalidValue("dst plane"))?;
            for cy in 0..chroma_plane.height() {
                let cell_row = chroma_plane
                    .row_mut(cy)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                let y0 = cy * sub_h;
                // `div_ceil` grids: the last cell row may hold fewer
                // than `sub_h` luma rows — clamp to what exists.
                let y1 = (y0 + sub_h).min(height);
                for (cx, cell_out) in cell_row.iter_mut().enumerate() {
                    let x0 = cx * sub_w;
                    let x1 = (x0 + sub_w).min(width);
                    // Average the cell's Q16-domain chroma with
                    // round-half-up, then quantize — `count` ∈ {1, 2, 4}
                    // (never 0: every cell covers ≥1 luma pixel).
                    let mut sum = 0_i32;
                    let mut count = 0_i32;
                    for y in y0..y1 {
                        let src_row = rgba.row(y).ok_or(Error::InvalidValue("src plane"))?;
                        for px in src_row.as_chunks::<4>().0.iter().skip(x0).take(x1 - x0) {
                            sum += self.coef.chroma(px[0], px[1], px[2])[plane_index - 1];
                            count += 1;
                        }
                    }
                    *cell_out = clip8((sum + count / 2) / count);
                }
            }
        }
        // Metadata: record the applied matrix/range (constructor args
        // after default resolution); primaries/transfer/hdr propagate
        // untouched, and a color conversion never retimes the frame.
        let mut dst_color = src_color;
        dst_color.matrix = self.matrix;
        dst_color.range = self.range;
        dst.set_color(dst_color);
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{
        ColorInfo, ColorPrimaries, ContentLightLevel, HdrMetadata, PixelFormat, SharedPlane,
        TransferCharacteristics, VideoFrame,
    };

    use super::*;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Converts one `Rgba8` pixel through a `Yuv444p` destination and
    /// returns its `(Y, Cb, Cr)` code values.
    fn pixel_444(kernel: &mut RgbToYuv, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        src.plane_mut(0)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[r, g, b, 255]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        (
            dst.plane(0).unwrap().row(0).unwrap()[0],
            dst.plane(1).unwrap().row(0).unwrap()[0],
            dst.plane(2).unwrap().row(0).unwrap()[0],
        )
    }

    /// Golden vectors — every supported matrix × range pair against
    /// known pixels, at the documented Q16 bit-exact contract.
    ///
    /// Expected values are derived from the H.273 weightings through the
    /// documented integer pipeline — worked example, Bt601/Limited pure
    /// red `(R, G, B) = (255, 0, 0)`:
    ///
    /// ```text
    /// Y  = clip8((16 << 16) + 16 829·255)                = 81
    /// Cb = clip8((128 << 16) − 9 714·255)                = 90
    /// Cr = clip8((128 << 16) + 28 784·255)               = 240
    /// ```
    ///
    /// i.e. exactly the BT.601 bar color `(81, 90, 240)` that
    /// `YuvToRgb`'s golden vectors decode — the same holds for green
    /// `(145, 54, 34)` and blue `(41, 240, 110)`. Limited `255`-bright
    /// primaries reach `Cr`/`Cb` = 240; Full-range primaries saturate
    /// the code space at 255 (e.g. Bt601 Full red `Cr` rounds to 256 →
    /// clamp 255 — the known out-of-range case the pipeline must
    /// saturate). All remaining expectations are derived identically.
    #[test]
    fn golden_vectors_cover_every_matrix_range_pair() {
        let pixels = [
            (128_u8, 128_u8, 128_u8), // neutral gray
            (255, 0, 0),              // saturated red
            (0, 255, 0),              // saturated green
            (0, 0, 255),              // saturated blue
            (0, 0, 0),                // code-space extremes
            (255, 255, 255),
        ];
        // Columns are (Y, Cb, Cr) per pixel in `pixels` order.
        let cases = [
            (
                MatrixCoefficients::Bt601,
                ColorRange::Limited,
                [
                    (126, 128, 128),
                    (81, 90, 240),
                    (145, 54, 34),
                    (41, 240, 110),
                    (16, 128, 128),
                    (235, 128, 128),
                ],
            ),
            (
                MatrixCoefficients::Bt601,
                ColorRange::Full,
                [
                    (128, 128, 128),
                    (76, 85, 255),
                    (150, 44, 21),
                    (29, 255, 107),
                    (0, 128, 128),
                    (255, 128, 128),
                ],
            ),
            (
                MatrixCoefficients::Bt709,
                ColorRange::Limited,
                [
                    (126, 128, 128),
                    (63, 102, 240),
                    (173, 42, 26),
                    (32, 240, 118),
                    (16, 128, 128),
                    (235, 128, 128),
                ],
            ),
            (
                MatrixCoefficients::Bt709,
                ColorRange::Full,
                [
                    (128, 128, 128),
                    (54, 99, 255),
                    (182, 30, 12),
                    (18, 255, 116),
                    (0, 128, 128),
                    (255, 128, 128),
                ],
            ),
            (
                MatrixCoefficients::Bt2020,
                ColorRange::Limited,
                [
                    (126, 128, 128),
                    (74, 97, 240),
                    (164, 47, 25),
                    (29, 240, 119),
                    (16, 128, 128),
                    (235, 128, 128),
                ],
            ),
            (
                MatrixCoefficients::Bt2020,
                ColorRange::Full,
                [
                    (128, 128, 128),
                    (67, 92, 255),
                    (173, 36, 11),
                    (15, 255, 118),
                    (0, 128, 128),
                    (255, 128, 128),
                ],
            ),
        ];
        for (matrix, range, expected) in cases {
            let mut kernel = RgbToYuv::new(matrix, range).unwrap();
            for ((r, g, b), want) in pixels.iter().zip(expected.iter()) {
                assert_eq!(
                    pixel_444(&mut kernel, *r, *g, *b),
                    *want,
                    "matrix {matrix:?} range {range:?} pixel ({r}, {g}, {b})"
                );
            }
        }
    }

    /// Fills a `Rgba8` frame from per-pixel RGB colors (alpha 255).
    fn rgba_frame(width: u32, height: u32, pixels: &[[u8; 3]]) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::Rgba8, width, height, pts()).unwrap();
        {
            let mut plane = frame.plane_mut(0).unwrap();
            for (dst_px, [r, g, b]) in plane
                .data_mut()
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(pixels.iter())
            {
                dst_px.copy_from_slice(&[*r, *g, *b, 255]);
            }
        }
        frame
    }

    /// 4:2:0 chroma downsampling — including the odd-dimension rule: a
    /// 5×3 luma grid pairs with a 3×2 chroma grid, and the last cell
    /// column/row must average only the luma samples that exist.
    ///
    /// Each chroma cell is filled with a uniform color so the emitted
    /// value is that color's chroma (Bt601/Limited):
    ///
    /// ```text
    /// cell (cx, cy): (0,0)   (1,0)    (2,0)      (3rd col = x4 only)
    ///                red     green    blue    → Cb 90/54/240, Cr 240/34/110
    ///                (0,1)   (1,1)    (2,1)
    ///                white   cyan     magenta → Cb 128/166/202, Cr 128/16/222
    /// ```
    ///
    /// e.g. cyan `(0, 255, 255)`: `Cb = clip8(8 388 608 − 19 067·255 +
    /// 28 784·255) = 166`. Cells (2, 0) and (·, 1) cover one luma
    /// column/row and still emit exactly their color's chroma — no
    /// phantom neighbor is averaged in.
    #[test]
    fn chroma_downsample_420_covers_existing_cells_only() {
        // Cell color of luma pixel (x, y) is `cells[y / 2][x / 2]`.
        let cells = [
            [[255, 0, 0], [0, 255, 0], [0, 0, 255]],
            [[255, 255, 255], [0, 255, 255], [255, 0, 255]],
        ];
        let pixels: Vec<[u8; 3]> = (0..3_usize)
            .flat_map(|y| (0..5_usize).map(move |x| cells[y / 2][x / 2]))
            .collect();
        let src = rgba_frame(5, 3, &pixels);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 5, 3, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();

        // Luma rows: each pixel's Y of its cell color.
        let expected_luma = [
            [81_u8, 81, 145, 145, 41],
            [81, 81, 145, 145, 41],
            [235, 235, 170, 170, 106],
        ];
        for (y, row) in expected_luma.iter().enumerate() {
            assert_eq!(
                dst.plane(0)
                    .unwrap()
                    .row(u32::try_from(y).unwrap())
                    .unwrap(),
                row,
                "luma row {y}"
            );
        }
        // Plane 1 = Cb cells, plane 2 = Cr cells.
        for (plane, expected) in [
            (1_usize, [[90_u8, 54, 240], [128, 166, 202]]),
            (2, [[240, 34, 110], [128, 16, 222]]),
        ] {
            for (cy, row) in expected.iter().enumerate() {
                assert_eq!(
                    dst.plane(plane)
                        .unwrap()
                        .row(u32::try_from(cy).unwrap())
                        .unwrap(),
                    row,
                    "chroma plane {plane} row {cy}"
                );
            }
        }
    }

    /// 4:2:2 chroma downsampling — vertical neighbor is per-row (chroma
    /// height == luma height), horizontal pairs average; the odd-width
    /// right cell holds a single pixel.
    #[test]
    fn chroma_downsample_422_averages_horizontal_pairs() {
        // Cell color of luma pixel (x, y) is `cells[y][x / 2]`.
        let cells = [[[255, 0, 0], [0, 0, 255]], [[0, 255, 0], [255, 0, 255]]];
        let pixels: Vec<[u8; 3]> = (0..2_usize)
            .flat_map(|y| (0..3_usize).map(move |x| cells[y][x / 2]))
            .collect();
        let src = rgba_frame(3, 2, &pixels);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv422p, 3, 2, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();

        let expected_luma = [[81_u8, 81, 41], [145, 145, 106]];
        for (y, row) in expected_luma.iter().enumerate() {
            assert_eq!(
                dst.plane(0)
                    .unwrap()
                    .row(u32::try_from(y).unwrap())
                    .unwrap(),
                row,
                "luma row {y}"
            );
        }
        // Plane 1 = Cb cells, plane 2 = Cr cells.
        for (plane, expected) in [
            (1_usize, [[90_u8, 240], [54, 202]]),
            (2, [[240, 110], [34, 222]]),
        ] {
            for (cy, row) in expected.iter().enumerate() {
                assert_eq!(
                    dst.plane(plane)
                        .unwrap()
                        .row(u32::try_from(cy).unwrap())
                        .unwrap(),
                    row,
                    "chroma plane {plane} row {cy}"
                );
            }
        }
    }

    /// The documented quantization contract is *average in the Q16
    /// domain, then `clip8`* — not "quantize each pixel, then average
    /// the bytes". A 1×2 luma frame makes one 4:2:0 cell of two pixels:
    /// black `(0,0,0)` and `(0,0,102)`, whose Q16 chroma sums are odd —
    /// averaging the pre-quantization values lands half a code step
    /// from averaging the quantized bytes:
    ///
    /// ```text
    /// Cb: (8 388 608 + 11 324 576 + 1) / 2 = 9 856 592 → clip8 = 150
    ///     (quantized first: (128 + 173) / 2 = 150.5 → 151)
    /// Cr: (8 388 608 + 7 911 146 + 1) / 2 = 8 149 877  → clip8 = 124
    ///     (quantized first: (128 + 120) / 2 = 124.5 → 125)
    /// ```
    #[test]
    fn chroma_downsample_averages_q16_before_quantizing() {
        let src = rgba_frame(1, 2, &[[0, 0, 0], [0, 0, 102]]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 2, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[16]);
        assert_eq!(dst.plane(0).unwrap().row(1).unwrap(), &[26]);
        assert_eq!(dst.plane(1).unwrap().row(0).unwrap(), &[150]);
        assert_eq!(dst.plane(2).unwrap().row(0).unwrap(), &[124]);
    }

    /// A 2×2 luma frame forms exactly one 4:2:0 cell — the four
    /// distinct pixels' chroma is box-averaged in the Q16 domain:
    ///
    /// ```text
    /// Cb: (8 388 608 + 11 324 576 + 5 911 538 + 3 525 503 + 2) / 4
    ///   = 7 287 556 → clip8 = 111
    /// Cr: (8 388 608 + 7 911 146 + 15 728 528 + 2 242 343 + 2) / 4
    ///   = 8 567 656 → clip8 = 131
    /// ```
    #[test]
    fn chroma_downsample_420_averages_the_whole_cell() {
        let src = rgba_frame(2, 2, &[[0, 0, 0], [0, 0, 102], [255, 0, 0], [0, 255, 0]]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), &[16_u8, 26, 81, 145]);
        assert_eq!(dst.plane(1).unwrap().data(), &[111_u8]);
        assert_eq!(dst.plane(2).unwrap().data(), &[131_u8]);
    }

    /// Round-trip cross-validation: the documented `YuvToRgb` decode of
    /// this kernel's BT.601 output returns the source bars, pixel for
    /// pixel — the chroma-average and nearest-sample-upsample paths are
    /// consistent inverses on uniform cells.
    #[test]
    fn rgb_yuv_roundtrip_recovers_uniform_cells() {
        // 4×4 frame of four 2×2-uniform colors → each 420 cell holds one
        // color's chroma, so YUV→RGB nearest-sample upsample recovers
        // the source pixel for pixel.
        //
        // ```text
        // cells:  (0,0)=red  (1,0)=green
        //         (0,1)=blue (1,1)=white
        // ```
        let colors = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
        let pixels: Vec<[u8; 3]> = (0..4_usize)
            .flat_map(|y| (0..4_usize).map(move |x| colors[y / 2 * 2 + x / 2]))
            .collect();
        let src = rgba_frame(4, 4, &pixels);
        let mut yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 4, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut yuv)
            .unwrap();
        let mut back = VideoFrame::new_owned(PixelFormat::Rgba8, 4, 4, pts()).unwrap();
        crate::YuvToRgb::new().process(&yuv, &mut back).unwrap();
        // Per-cell decodes through `YuvToRgb`'s Q16 path: red
        // (81, 90, 240) → (254, 0, 0), green (145, 54, 34) → (0, 255, 1),
        // blue (41, 240, 110) → (0, 0, 255), white (235, 128, 128) →
        // (255, 255, 255) — pure primaries round-trip within the 8-bit
        // quantization of both legs.
        let expected_rows = [
            [
                [254_u8, 0, 0, 255],
                [254, 0, 0, 255],
                [0, 255, 1, 255],
                [0, 255, 1, 255],
            ],
            [
                [254, 0, 0, 255],
                [254, 0, 0, 255],
                [0, 255, 1, 255],
                [0, 255, 1, 255],
            ],
            [
                [0, 0, 255, 255],
                [0, 0, 255, 255],
                [255, 255, 255, 255],
                [255, 255, 255, 255],
            ],
            [
                [0, 0, 255, 255],
                [0, 0, 255, 255],
                [255, 255, 255, 255],
                [255, 255, 255, 255],
            ],
        ];
        for (y, expected_row) in expected_rows.iter().enumerate() {
            let row = back
                .plane(0)
                .unwrap()
                .row(u32::try_from(y).unwrap())
                .unwrap();
            for (x, want) in expected_row.iter().enumerate() {
                assert_eq!(&row[x * 4..x * 4 + 4], want, "pixel ({x}, {y})");
            }
        }
    }

    /// A `stride > row_bytes` destination: conversion must write rows
    /// only — padding bytes keep their caller-chosen contents.
    #[test]
    fn padded_dst_stride_is_never_written() {
        // 1×2 Rgba8 source (neutral gray), 1×2 `Yuv444p` dst with a
        // padded stride of 3 on every plane: usable `3·(2−1) + 1` = 4
        // bytes per plane, offsets 1–2 are inter-row padding.
        let src = rgba_frame(1, 2, &[[128, 128, 128], [128, 128, 128]]);
        let mut dst = VideoFrame::from_shared_planes(
            PixelFormat::Yuv444p,
            1,
            2,
            pts(),
            (0..3)
                .map(|_| SharedPlane {
                    data: Arc::from(vec![0_u8, 0xEE, 0xEE, 0].into_boxed_slice()),
                    stride: 3,
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(dst.plane(0).unwrap().stride(), 3);
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        // Gray → (126, 128, 128): rows written, padding intact on all
        // three planes.
        for plane in 0..3 {
            let expected: &[u8] = if plane == 0 {
                &[126, 0xEE, 0xEE, 126]
            } else {
                &[128, 0xEE, 0xEE, 128]
            };
            assert_eq!(dst.plane(plane).unwrap().data(), expected);
        }
    }

    /// A shared `dst` (clone still holding the `Arc` backing) must be
    /// rejected — `plane_mut` → `None` — with no partial write.
    #[test]
    fn shared_dst_backing_is_rejected() {
        let src = rgba_frame(1, 1, &[[128, 128, 128]]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        let sharing_clone = dst.clone();
        assert!(dst.plane_mut(0).is_none());
        assert_eq!(
            RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
                .unwrap()
                .process(&src, &mut dst),
            Err(Error::InvalidValue("dst plane"))
        );
        // Rejection wrote nothing — including planes 1/2.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0]);
        assert_eq!(dst.plane(1).unwrap().row(0).unwrap(), &[0]);
        assert_eq!(dst.plane(2).unwrap().row(0).unwrap(), &[0]);
        drop(sharing_clone);
        assert!(
            RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
                .unwrap()
                .process(&src, &mut dst)
                .is_ok()
        );
    }

    /// Format/dimension mismatches and unusable constructor arguments
    /// are all rejected before any write.
    #[test]
    fn invalid_inputs_are_rejected() {
        let mut kernel = RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited).unwrap();
        let src = rgba_frame(1, 1, &[[128, 128, 128]]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();

        // Planar source.
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_yuv, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Packed destinations: `Rgba8` and `RgbaF16` are not Yuv*p.
        let mut dst_rgb = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_rgb),
            Err(Error::InvalidValue("dst format"))
        );
        let mut dst_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_f16),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::Yuv444p, 2, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // `Identity` cannot produce YCbCr output; unknown future
        // `#[non_exhaustive]` variants share the `_` arm.
        assert_eq!(
            RgbToYuv::new(MatrixCoefficients::Identity, ColorRange::Limited).unwrap_err(),
            Error::InvalidValue("color matrix")
        );

        // Nothing was written by any rejection.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0]);
    }

    /// `Unspecified` constructor arguments resolve under the documented
    /// default rules: matrix → Bt601, range → Limited — and the output
    /// `ColorInfo` records the resolved values, not `Unspecified`.
    #[test]
    fn unspecified_args_use_defaults() {
        let src = rgba_frame(1, 1, &[[255, 0, 0]]);
        let mut dst_default = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Unspecified, ColorRange::Unspecified)
            .unwrap()
            .process(&src, &mut dst_default)
            .unwrap();
        let mut dst_explicit = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst_explicit)
            .unwrap();

        for plane in 0..3 {
            assert_eq!(
                dst_default.plane(plane).unwrap().data(),
                dst_explicit.plane(plane).unwrap().data()
            );
        }
        let color = dst_default.color();
        assert_eq!(color.matrix, MatrixCoefficients::Bt601);
        assert_eq!(color.range, ColorRange::Limited);
    }

    /// Output `ColorInfo` records the applied conversion (matrix/range →
    /// the resolved arguments) while primaries/transfer/hdr and the
    /// timestamps propagate unchanged.
    #[test]
    fn output_color_and_time_propagate() {
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let mut src = rgba_frame(1, 1, &[[128, 128, 128]]);
        src.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Pq,
            matrix: MatrixCoefficients::Identity, // RGB input, as expected
            range: ColorRange::Full,
            hdr: Some(HdrMetadata {
                content_light_level: Some(ContentLightLevel {
                    max_cll: 1_000,
                    max_fall: 400,
                }),
                ..HdrMetadata::default()
            }),
        });
        src.set_duration(Some(duration));
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        dst.set_pts(MediaTime::ZERO);
        RgbToYuv::new(MatrixCoefficients::Bt709, ColorRange::Unspecified)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();

        let color = dst.color();
        assert_eq!(color.matrix, MatrixCoefficients::Bt709);
        assert_eq!(color.range, ColorRange::Limited); // resolved default
        // Non-destructive carriage: untouched fields propagate as-is
        // (PQ is *carried*, never applied) — even src's own RGB-side
        // matrix/range fields are overwritten with the applied values.
        assert_eq!(color.primaries, ColorPrimaries::Bt2020);
        assert_eq!(color.transfer, TransferCharacteristics::Pq);
        assert_eq!(
            color.hdr.unwrap().content_light_level.unwrap().max_cll,
            1_000
        );
        assert_eq!(dst.pts(), pts());
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Alpha is dropped: the YUV formats carry no alpha channel.
    #[test]
    fn alpha_is_dropped() {
        let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        src.plane_mut(0)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[255, 0, 0, 17]);
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        // Identical output to the opaque-alpha conversion of red.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[81]);
        assert_eq!(dst.plane(1).unwrap().row(0).unwrap(), &[90]);
        assert_eq!(dst.plane(2).unwrap().row(0).unwrap(), &[240]);
    }

    /// RT contract: a prepared kernel's `process` is allocation-free,
    /// non-blocking and bounded — observably: repeated calls on the same
    /// kernel succeed and stay deterministic, never panic.
    #[test]
    fn rt_process_is_repeatable_and_deterministic() {
        let src = rgba_frame(
            2,
            2,
            &[[255, 0, 0], [0, 255, 0], [0, 0, 255], [128, 128, 128]],
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, pts()).unwrap();
        let mut kernel = RgbToYuv::new(MatrixCoefficients::Bt709, ColorRange::Limited).unwrap();
        // Row 0 pixels are red and green: Y = 63 and 173. The cell's
        // mixed chroma averages near-neutral: Cb = Cr = 128.
        for _ in 0..64 {
            kernel.process(&src, &mut dst).unwrap();
            assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[63, 173]);
            assert_eq!(dst.plane(1).unwrap().row(0).unwrap(), &[128]);
            assert_eq!(dst.plane(2).unwrap().row(0).unwrap(), &[128]);
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// never allocates a new frame, and `src` stays byte-identical
    /// across the call (a borrow, not a drain).
    #[test]
    fn rt_process_never_mutates_source() {
        let src = rgba_frame(
            4,
            2,
            &[
                [100, 50, 200],
                [1, 2, 3],
                [4, 5, 6],
                [7, 8, 9],
                [10, 11, 12],
                [13, 14, 15],
                [16, 17, 18],
                [19, 20, 21],
            ],
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts()).unwrap();
        RgbToYuv::new(MatrixCoefficients::Bt601, ColorRange::Limited)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }
}
