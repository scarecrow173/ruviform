//! Internal coefficient derivation for the YUV↔RGB conversion kernels:
//! [`MatrixCoefficients`] × [`ColorRange`] are resolved under the spec's
//! default rules into rows/columns of a Q16 fixed-point coefficient
//! table baked at prepare time.
//!
//! ## Resolution rules
//!
//! - `Unspecified` matrix is interpreted as `Bt601` (the SD-era
//!   historical default) and `Unspecified` range as `Limited` — the
//!   spec's documented defaults for the metadata-carriage + non-
//!   destructive HDR scope.
//! - `Identity` — content already in RGB, no YCbCr matrixing applies —
//!   and any variant a future `#[non_exhaustive]` extension adds resolve
//!   to [`Error::InvalidValue`] through a graceful `_` arm, never
//!   `unreachable!()`.
//!
//! Contract source: `docs/specs/implement-video-foundation-api.md`
//! (Requirements「色変換の規則」) and
//! `docs/adr/video-proc-kernel-api-and-color-hdr-scope.md` (D3-a).

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{ColorRange, MatrixCoefficients};

/// Fractional bits of the kernel's fixed-point coefficients — Q16.
pub(crate) const FRAC_BITS: u32 = 16;

/// Bias added before the final Q16 right shift — round-half-up.
const HALF: i32 = 1 << (FRAC_BITS - 1);

/// Neutral chroma code value for 8-bit samples.
const CHROMA_CENTER: i32 = 128;

/// Rows of the coefficient table — must match [`matrix_slot`].
pub(crate) const MATRIX_SLOTS: usize = 3;

/// Columns of the coefficient table — must match [`range_slot`].
pub(crate) const RANGE_SLOTS: usize = 2;

/// Resolves a frame's `matrix` to its coefficient-table row
/// (`0` = Bt601, `1` = Bt709, `2` = Bt2020); `Unspecified` defaults to
/// the Bt601 row.
///
/// `Identity` (RGB content — no YCbCr matrix to undo) and unknown future
/// variants are rejected with [`Error::InvalidValue`].
pub(crate) fn matrix_slot(matrix: MatrixCoefficients) -> Result<usize> {
    match matrix {
        MatrixCoefficients::Bt601 | MatrixCoefficients::Unspecified => Ok(0),
        MatrixCoefficients::Bt709 => Ok(1),
        MatrixCoefficients::Bt2020 => Ok(2),
        // `Identity` and every future `#[non_exhaustive]` variant.
        _ => Err(Error::InvalidValue("color matrix")),
    }
}

/// Resolves a frame's `range` to its coefficient-table column
/// (`0` = Limited, `1` = Full); `Unspecified` defaults to Limited.
///
/// Unknown future variants are rejected with [`Error::InvalidValue`].
pub(crate) fn range_slot(range: ColorRange) -> Result<usize> {
    match range {
        ColorRange::Limited | ColorRange::Unspecified => Ok(0),
        ColorRange::Full => Ok(1),
        _ => Err(Error::InvalidValue("color range")),
    }
}

/// The applied matrix per coefficient-table row — the inverse of
/// [`matrix_slot`]'s default rules (`Unspecified` resolves to `Bt601`,
/// so the recorded value is never `Unspecified`). Used by kernels that
/// fix the matrix at construction time to report the applied conversion
/// in an output `ColorInfo`.
pub(crate) const RESOLVED_MATRIX: [MatrixCoefficients; MATRIX_SLOTS] = [
    MatrixCoefficients::Bt601,
    MatrixCoefficients::Bt709,
    MatrixCoefficients::Bt2020,
];

/// The applied range per coefficient-table column — the inverse of
/// [`range_slot`]'s default rules, same purpose as [`RESOLVED_MATRIX`].
pub(crate) const RESOLVED_RANGE: [ColorRange; RANGE_SLOTS] =
    [ColorRange::Limited, ColorRange::Full];

/// Q16 fixed-point coefficients of the `Yuv*p → Rgba8` integer pipeline
/// for one resolved matrix × range pair.
///
/// Per output pixel the kernel evaluates:
///
/// ```text
/// y' = y_scale · (Y − y_offset)
/// R  = clip8(y' + cr_to_r · Cr')
/// G  = clip8(y' − cb_to_g · Cb' − cr_to_g · Cr')
/// B  = clip8(y' + cb_to_b · Cb')
/// ```
///
/// with `Cb' = Cb − 128`, `Cr' = Cr − 128` and `clip8` = round-half-up
/// out of Q16 then saturate to `[0, 255]`. All integer math — bit-exact.
#[derive(Copy, Clone, Debug)]
pub(crate) struct YuvCoef {
    /// Luma code-value offset (16 Limited, 0 Full).
    y_offset: i32,
    /// Luma gain, Q16 (≈1.1644 Limited → 76309; 1.0 Full → 65536).
    y_scale: i32,
    /// Cr gain into R, Q16.
    cr_to_r: i32,
    /// Cb gain into G (subtracted), Q16.
    cb_to_g: i32,
    /// Cr gain into G (subtracted), Q16.
    cr_to_g: i32,
    /// Cb gain into B, Q16.
    cb_to_b: i32,
}

impl YuvCoef {
    /// One `(Y, Cb, Cr)` sample → `(R, G, B)` code values through the
    /// integer pipeline. Pure integer math; never panics.
    pub(crate) fn apply(&self, y: u8, cb: u8, cr: u8) -> [u8; 3] {
        let y_term = self.y_scale * (i32::from(y) - self.y_offset);
        let blue_term = i32::from(cb) - CHROMA_CENTER;
        let red_term = i32::from(cr) - CHROMA_CENTER;
        [
            clip8(y_term + self.cr_to_r * red_term),
            clip8(y_term - self.cb_to_g * blue_term - self.cr_to_g * red_term),
            clip8(y_term + self.cb_to_b * blue_term),
        ]
    }
}

/// `(Kr, Kb)` luma/chroma weightings per coefficient-table row
/// (ITU-T H.273 labels; `Kg = 1 − Kr − Kb`).
const MATRIX_WEIGHTS: [(f64, f64); MATRIX_SLOTS] = [
    (0.299, 0.114),   // Bt601
    (0.2126, 0.0722), // Bt709
    (0.2627, 0.0593), // Bt2020
];

/// `(y_offset, luma gain, chroma gain)` per coefficient-table column —
/// the 8-bit code-space scaling: Limited quantizes `Y'` into
/// `[16, 235]` (219 steps) and `C` into `[16, 240]` (224 steps), Full
/// uses the whole code space.
const RANGE_SCALE: [(i32, f64, f64); RANGE_SLOTS] = [
    (16, 255.0 / 219.0, 255.0 / 224.0), // Limited
    (0, 1.0, 1.0),                      // Full
];

/// Builds the whole `Yuv*p → Rgba8` coefficient table: rows indexed by
/// [`matrix_slot`] (Bt601/Bt709/Bt2020), columns by [`range_slot`]
/// (Limited/Full). Prepare-time only — the derivation below uses
/// floating-point math once per pair.
pub(crate) fn yuv_to_rgb_table() -> [[YuvCoef; RANGE_SLOTS]; MATRIX_SLOTS] {
    std::array::from_fn(|matrix| std::array::from_fn(|range| coef_pair(matrix, range)))
}

/// Derives the Q16 coefficient set for table slot `(matrix, range)` —
/// the normalized H.273 weighting scaled into the 8-bit code space.
/// Internal callers only produce in-range slots (see [`matrix_slot`] /
/// [`range_slot`]), so the table indexing never goes out of bounds.
fn coef_pair(matrix: usize, range: usize) -> YuvCoef {
    let (kr, kb) = MATRIX_WEIGHTS[matrix];
    let kg = 1.0 - kr - kb;
    let (y_offset, luma_gain, chroma_gain) = RANGE_SCALE[range];
    // Normalized (full-range) chroma → RGB gains.
    let blue_gain = 2.0 * (1.0 - kb); // Cb → B
    let red_gain = 2.0 * (1.0 - kr); //  Cr → R
    YuvCoef {
        y_offset,
        y_scale: q16(luma_gain),
        cr_to_r: q16(red_gain * chroma_gain),
        cb_to_g: q16(kb / kg * blue_gain * chroma_gain),
        cr_to_g: q16(kr / kg * red_gain * chroma_gain),
        cb_to_b: q16(blue_gain * chroma_gain),
    }
}

/// Q16 fixed-point coefficients of the `Rgba8 → Yuv*p` integer pipeline
/// for one resolved matrix × range pair — the inverse direction of
/// [`YuvCoef`].
///
/// Per luma pixel the kernel evaluates the normalized H.273 model
///
/// ```text
/// Y'  = Kr·R + Kg·G + Kb·B
/// Cb' = (B − Y') / (2·(1 − Kb))
/// Cr' = (R − Y') / (2·(1 − Kr))
/// ```
///
/// with `R`/`G`/`B` the channel code values rescaled into `[0, 1]`,
/// expanded into per-channel gains so each output sample is one fused
/// multiply-add chain in Q16:
///
/// ```text
/// Y       = clip8(y_base + r_to_y·R + g_to_y·G + b_to_y·B)
/// Cb(q16) = CHROMA_CENTER·2¹⁶ + r_to_cb·R + g_to_cb·G + b_to_cb·B
/// Cr(q16) = CHROMA_CENTER·2¹⁶ + r_to_cr·R + g_to_cr·G + b_to_cr·B
/// ```
///
/// The chroma values stay in the Q16 domain — pre-round, pre-clip — so a
/// subsampled output cell can average the per-pixel values before
/// quantization (see [`RgbToYuv`](crate::RgbToYuv)); the code-space
/// gains `b_to_cb`/`r_to_cr` already carry the
/// `1 / (2·(1 − Kb|Kr))` normalization. All integer math — bit-exact.
#[derive(Copy, Clone, Debug)]
pub(crate) struct RgbCoef {
    /// Luma code-value offset, pre-shifted to Q16 (`16 << 16` Limited,
    /// `0` Full).
    y_base: i32,
    /// R gain into Y, Q16 (`Kr · 219/255` Limited, `Kr` Full).
    r_to_y: i32,
    /// G gain into Y, Q16.
    g_to_y: i32,
    /// B gain into Y, Q16.
    b_to_y: i32,
    /// R gain into Cb, Q16 (negative).
    r_to_cb: i32,
    /// G gain into Cb, Q16 (negative).
    g_to_cb: i32,
    /// B gain into Cb, Q16.
    b_to_cb: i32,
    /// R gain into Cr, Q16.
    r_to_cr: i32,
    /// G gain into Cr, Q16 (negative).
    g_to_cr: i32,
    /// B gain into Cr, Q16 (negative).
    b_to_cr: i32,
}

impl RgbCoef {
    /// One `(r, g, b)` pixel → quantized `Y` code value. Pure integer
    /// math; never panics.
    pub(crate) fn luma(&self, r: u8, g: u8, b: u8) -> u8 {
        clip8(
            self.y_base
                + self.r_to_y * i32::from(r)
                + self.g_to_y * i32::from(g)
                + self.b_to_y * i32::from(b),
        )
    }

    /// One `(r, g, b)` pixel → `[Cb, Cr]` in the Q16 domain —
    /// pre-round, pre-clip, so the caller can average a downsampling
    /// cell before quantizing with [`clip8`]. Pure integer math; never
    /// panics.
    pub(crate) fn chroma(&self, r: u8, g: u8, b: u8) -> [i32; 2] {
        let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
        [
            CHROMA_CENTER * (1 << FRAC_BITS)
                + self.r_to_cb * r
                + self.g_to_cb * g
                + self.b_to_cb * b,
            CHROMA_CENTER * (1 << FRAC_BITS)
                + self.r_to_cr * r
                + self.g_to_cr * g
                + self.b_to_cr * b,
        ]
    }
}

/// Builds the whole `Rgba8 → Yuv*p` coefficient table: rows indexed by
/// [`matrix_slot`] (Bt601/Bt709/Bt2020), columns by [`range_slot`]
/// (Limited/Full). Prepare-time only — the derivation below uses
/// floating-point math once per pair.
pub(crate) fn rgb_to_yuv_table() -> [[RgbCoef; RANGE_SLOTS]; MATRIX_SLOTS] {
    std::array::from_fn(|matrix| std::array::from_fn(|range| rgb_coef_pair(matrix, range)))
}

/// Derives the Q16 coefficient set for table slot `(matrix, range)` —
/// the normalized H.273 weighting inverted and scaled into the 8-bit
/// code space, mirroring [`coef_pair`].
///
/// The code-space gains are the reciprocals of [`RANGE_SCALE`]'s
/// decode-side columns: luma `219/255` (Limited) or unity (Full),
/// chroma `224/255` (Limited) or unity (Full). The chroma
/// normalizations `1 / (2·(1 − Kb))` and `1 / (2·(1 − Kr))` are folded
/// into the per-channel gains.
fn rgb_coef_pair(matrix: usize, range: usize) -> RgbCoef {
    let (kr, kb) = MATRIX_WEIGHTS[matrix];
    let kg = 1.0 - kr - kb;
    let (y_offset, luma_gain, chroma_gain) = RANGE_SCALE[range];
    // Encode-side code-space gains — inverse of the decode-side scaling.
    let y_gain = 1.0 / luma_gain;
    let chroma_scale = 1.0 / chroma_gain;
    // Cb' = (B − Y') / (2·(1 − Kb)) and Cr' = (R − Y') / (2·(1 − Kr))
    // expanded into per-channel gains — `blue_norm`/`red_norm` carry
    // the per-axis 1/(2·(1 − K)) normalizers.
    let blue_norm = chroma_scale / (2.0 * (1.0 - kb));
    let red_norm = chroma_scale / (2.0 * (1.0 - kr));
    RgbCoef {
        y_base: y_offset << FRAC_BITS,
        r_to_y: q16(y_gain * kr),
        g_to_y: q16(y_gain * kg),
        b_to_y: q16(y_gain * kb),
        r_to_cb: q16(-blue_norm * kr),
        g_to_cb: q16(-blue_norm * kg),
        // blue_norm·(1−Kb) == red_norm·(1−Kr) == chroma_scale/2 — the
        // dominant channel gain is written in the exact form so the
        // float noise cannot diverge between the two paths.
        b_to_cb: q16(chroma_scale / 2.0),
        r_to_cr: q16(chroma_scale / 2.0),
        g_to_cr: q16(-red_norm * kg),
        b_to_cr: q16(-red_norm * kb),
    }
}

/// Rounds a real coefficient to Q16 fixed-point.
// All gains are bounded (|gain| ≤ 2·255/224 ≈ 2.28 → |value| < 2¹⁸), so
// the rounded result always fits `i32` and the cast only drops the
// (already integer-valued) fraction.
#[allow(clippy::cast_possible_truncation)]
fn q16(gain: f64) -> i32 {
    (gain * f64::from(1 << FRAC_BITS)).round() as i32
}

/// Rounds a Q16-domain value half-up and saturates to a `u8` code value.
pub(crate) fn clip8(q16_value: i32) -> u8 {
    let shifted = (q16_value + HALF) >> FRAC_BITS;
    // Provably within [0, 255] after `clamp`; the `unwrap_or_default`
    // fallback is unreachable but keeps this panic-free by construction.
    u8::try_from(shifted.clamp(0, i32::from(u8::MAX))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_resolution_applies_default_rules() {
        assert_eq!(matrix_slot(MatrixCoefficients::Unspecified), Ok(0));
        assert_eq!(matrix_slot(MatrixCoefficients::Bt601), Ok(0));
        assert_eq!(matrix_slot(MatrixCoefficients::Bt709), Ok(1));
        assert_eq!(matrix_slot(MatrixCoefficients::Bt2020), Ok(2));
        assert_eq!(range_slot(ColorRange::Unspecified), Ok(0));
        assert_eq!(range_slot(ColorRange::Limited), Ok(0));
        assert_eq!(range_slot(ColorRange::Full), Ok(1));
    }

    #[test]
    fn slot_resolution_rejects_unusable_values() {
        // `Identity` cannot drive a YCbCr→RGB conversion; future
        // `#[non_exhaustive]` variants share the `_` arm.
        assert_eq!(
            matrix_slot(MatrixCoefficients::Identity),
            Err(Error::InvalidValue("color matrix"))
        );
    }

    #[test]
    fn coefficient_table_matches_bt601_limited_spot_check() {
        // Bt601 Limited reference values (Q16): y_scale = round(255/219·
        // 2¹⁶) = 76309, cr_to_r = round(1.402·255/224·2¹⁶) = 104597.
        let table = yuv_to_rgb_table();
        let bt601_limited = table[0][0];
        assert_eq!(bt601_limited.y_scale, 76_309);
        assert_eq!(bt601_limited.cr_to_r, 104_597);
        assert_eq!(bt601_limited.y_offset, 16);
        // Full range: unity gains, no luma offset.
        let bt601_full = table[0][1];
        assert_eq!(bt601_full.y_scale, 65_536);
        assert_eq!(bt601_full.y_offset, 0);
    }

    #[test]
    fn rgb_coefficient_table_matches_bt601_limited_spot_check() {
        // Bt601 Limited reference values (Q16): r_to_y =
        // round(0.299·219/255·2¹⁶) = 16829, the dominant chroma gain
        // b_to_cb = r_to_cr = round(112/255·2¹⁶) = 28784.
        let table = rgb_to_yuv_table();
        let bt601_limited = table[0][0];
        assert_eq!(bt601_limited.r_to_y, 16_829);
        assert_eq!(bt601_limited.b_to_cb, 28_784);
        assert_eq!(bt601_limited.r_to_cr, 28_784);
        assert_eq!(bt601_limited.y_base, 16 << FRAC_BITS);
        // Full range: unity gains, no luma offset.
        let bt601_full = table[0][1];
        assert_eq!(bt601_full.r_to_y, q16(0.299));
        assert_eq!(bt601_full.b_to_cb, 1 << (FRAC_BITS - 1));
        assert_eq!(bt601_full.y_base, 0);
    }

    #[test]
    fn rgb_coefficients_keep_gray_chroma_neutral() {
        // R = G = B must leave Cb = Cr = 128: the per-row coefficient
        // sums −Kr, −Kg, (1−Kb) / (1−Kr), −Kg, −Kb cancel to (numerically)
        // zero — assert the Q16 rounding never drifts more than a few
        // quantum, so neutral content stays neutral in every pair.
        for coef in rgb_to_yuv_table().into_iter().flatten() {
            assert!(
                (coef.r_to_cb + coef.g_to_cb + coef.b_to_cb).abs() <= 8,
                "Cb row sum drift"
            );
            assert!(
                (coef.r_to_cr + coef.g_to_cr + coef.b_to_cr).abs() <= 8,
                "Cr row sum drift"
            );
        }
    }
}
