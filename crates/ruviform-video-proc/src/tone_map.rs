//! HDR→SDR tone-mapping kernel: [`ToneMapSdr`] maps an absolute-
//! luminance working-space frame onto the fixed 100-nit SDR domain by
//! applying the ITU-R BT.2390-8 §5.4 EETF (a Hermite spline in the
//! PQ-signal domain) to the BT.2020 luma and scaling RGB by the luma
//! ratio.
//!
//! ## Input convention
//!
//! Per `docs/designs/color-conversion-kernels.md` the kernel consumes
//! the pipeline's `RgbaF32` linear working space — channel values are
//! linear light on an absolute-luminance scale where `1.0` = 100 cd/m²
//! (see [`crate::transfer`]). Both `src` and `dst` must be `RgbaF32`.
//! Whether `src.color()` actually describes that working space is
//! **not** validated — `primaries`/`transfer`/`matrix`/`range` tags
//! are carried to `dst` verbatim, and metadata interpretation is the
//! consumer's responsibility (the same convention as the composite
//! kernels). Feeding a frame whose channels are not BT.2020 linear
//! absolute luminance is defined-level misuse.
//!
//! ## `hdr` consumption
//!
//! `src.color().hdr` is read on every call and drives the EETF
//! parameters: the source peak `L_HS` derives from
//! `mastering_display.max_luminance` (ST 2086 units of 1/10 000 cd/m²,
//! `> 0` only), falling back to `content_light_level.max_cll`
//! (cd/m² direct, `> 0` only), then to the default 1000 nits when both
//! are absent or invalid. The source black `L_min` derives from
//! `mastering_display.min_luminance` (same scale), adopted only as a
//! consistent value below `max_luminance` — absent metadata or an
//! inverted `min ≥ max` ordering falls back to `b = 0` (no black
//! lift).
//!
//! Because the frame is SDR-range on output, the consumed `hdr` is
//! **not** propagated: `dst.color()` records `src`'s color description
//! with `hdr: None` so a downstream stage cannot re-apply the same
//! metadata and double tone-map.
//!
//! ## Equivalence contract
//!
//! This scalar implementation is the reference per the baseline
//! DSP/SIMD decision: each channel's output is the `f64` evaluation of
//! the documented formulas rounded to `binary32`; any optimized
//! backend must match within **≤ 4 ulp (`binary32`)** per channel.
//! The `Y_src ≤ 0` and `ks ≥ 1` passthrough paths are bit-exact.
//! Non-finite channels propagate per `f32`/`f64` semantics — the
//! contract bounds finite outputs only. Finite inputs far above the
//! declared `L_HS` extrapolate the Hermite spline unclamped and may
//! likewise yield non-finite output.
//!
//! Contract source: `docs/designs/color-conversion-kernels.md`
//! (「`ToneMapSdr` 規約」) and
//! `docs/test-specs/color-working-space-kernel-tests.md`.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{ColorInfo, HdrMetadata, PixelFormat, VideoFrame};

use crate::transfer::{L_DS, SDR_SCALE, pq_eotf, pq_eotf_inv};

/// Fallback source peak `L_HS` (cd/m²) when `hdr` carries no usable
/// mastering-display or `MaxCLL` information — the BT.2390 reference
/// mastering peak of 1000 nits.
const DEFAULT_L_HS: f64 = 1_000.0;

/// BT.2020 luma weights applied to the working-space RGB triple:
/// `Y = 0.2627·R + 0.6780·G + 0.0593·B`.
const LUMA_WEIGHTS: [f64; 3] = [0.2627, 0.6780, 0.0593];

/// Per-call EETF parameters derived from `src.color().hdr` — the
/// BT.2390-8 §5.4 normalization of the fixed [`L_DS`] target against
/// this frame's derived source peak `L_HS`, plus the black-lift
/// coefficient.
struct Eetf {
    /// `pq_eotf_inv(L_HS)` — the PQ signal of the source peak.
    e_src: f64,
    /// `pq_eotf_inv(L_DS) / e_src` — the target peak normalized to the
    /// source peak's signal domain.
    max_lum: f64,
    /// Knee start `1.5·max_lum − 0.5`. `ks ≥ 1` (equivalently
    /// `L_HS ≤ L_DS`, an SDR-equivalent source) selects all-domain
    /// passthrough — the defined non-compression, not a no-op.
    ks: f64,
    /// Black-lift coefficient `pq_eotf_inv(L_min)/e_src`; `0` when no
    /// consistent `min_luminance` was signaled.
    b: f64,
}

impl Eetf {
    /// Derives this call's EETF parameters from `hdr` per the rules on
    /// the module doc: `L_HS` = `mastering_display.max_luminance`
    /// (`/10_000` → nits, `> 0` only) → `content_light_level.max_cll`
    /// (`> 0` only) → [`DEFAULT_L_HS`]; `b` = `pq_eotf_inv(L_min) /
    /// e_src` only for a consistent `min < max` mastering pair, else
    /// `0`.
    fn from_hdr(hdr: Option<HdrMetadata>) -> Self {
        let mastering = hdr.and_then(|h| h.mastering_display);
        let l_hs = mastering
            .map(|m| f64::from(m.max_luminance) / 10_000.0)
            .filter(|l| *l > 0.0)
            .or_else(|| {
                hdr.and_then(|h| h.content_light_level)
                    .map(|c| f64::from(c.max_cll))
                    .filter(|l| *l > 0.0)
            })
            .unwrap_or(DEFAULT_L_HS);
        let e_src = pq_eotf_inv(l_hs);
        let max_lum = pq_eotf_inv(L_DS) / e_src;
        Self {
            e_src,
            max_lum,
            // `1.5·max_lum − 0.5` in mul_add form.
            ks: 1.5f64.mul_add(max_lum, -0.5),
            b: match mastering {
                Some(m) if m.min_luminance < m.max_luminance => {
                    pq_eotf_inv(f64::from(m.min_luminance) / 10_000.0) / e_src
                }
                // Absent `mastering_display` or an inverted
                // `min ≥ max` ordering — the documented `b = 0`
                // fallback (no black lift).
                _ => 0.0,
            },
        }
    }
}

/// Reads one `RgbaF32` pixel's channels — raw little-endian `binary32`
/// quads widened to `f64` (every `f32` is exact in `f64`; `NaN`/`±∞`
/// propagate).
fn rgba_f32_signal(px: &[u8; 16]) -> [f64; 4] {
    std::array::from_fn(|i| {
        f64::from(f32::from_le_bytes([
            px[4 * i],
            px[4 * i + 1],
            px[4 * i + 2],
            px[4 * i + 3],
        ]))
    })
}

/// Serializes one `f64` channel quad into a 16-byte `RgbaF32` pixel —
/// each channel narrowed to `binary32` little-endian. The reference
/// definition is "the `f64` evaluation rounded to `binary32`", so the
/// `as f32` narrowing performs exactly the contract's rounding.
#[allow(clippy::cast_possible_truncation)]
fn f32_px_bytes(px: [f64; 4]) -> [u8; 16] {
    let mut out = [0_u8; 16];
    for (i, ch) in px.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&(*ch as f32).to_le_bytes());
    }
    out
}

/// The BT.2390-8 §5.4 EETF on one working-space pixel: luma is mapped
/// through the Hermite spline in the PQ-signal domain and RGB scales
/// by the luma ratio; alpha passes through. `src_px` is the raw
/// 16-byte pixel — passthrough returns it verbatim.
fn tone_map_px(eetf: &Eetf, src_px: &[u8; 16]) -> [u8; 16] {
    let [red, green, blue, alpha] = rgba_f32_signal(src_px);
    // BT.2020 luma of the working-space value, in mul_add form
    // (pedantic `suboptimal_flops`).
    let y_src =
        LUMA_WEIGHTS[0].mul_add(red, LUMA_WEIGHTS[1].mul_add(green, LUMA_WEIGHTS[2] * blue));
    // `ks ≥ 1` is the defined `L_HS ≤ L_DS` non-compression — the
    // whole domain passes through. A non-positive luma cannot be
    // ratio-scaled — it likewise passes through. Both copy the pixel
    // verbatim; a `NaN` luma fails `<= 0.0` and keeps to the formula
    // path, where it propagates.
    if eetf.ks >= 1.0 || y_src <= 0.0 {
        return *src_px;
    }
    // EETF in the PQ-signal domain (`pq_eotf_inv`/`pq_eotf` convert
    // nits ↔ signal):
    //   `e = pq_eotf_inv(Y_nits)`, `E = e / e_src`.
    let e = pq_eotf_inv(y_src * SDR_SCALE);
    let e_norm = e / eetf.e_src;
    // `E ≤ ks` → `E' = E` (identity below the knee); `E > ks` → the
    // Hermite interpolation `t = (E−ks)/(1−ks)`,
    // `E' = (2t³−3t²+1)·ks + (t³−2t²+t)·(1−ks) + (−2t³+3t²)·maxLum`,
    // each term in mul_add form.
    let e1 = if e_norm <= eetf.ks {
        e_norm
    } else {
        let t = (e_norm - eetf.ks) / (1.0 - eetf.ks);
        let t2 = t * t;
        let t3 = t2 * t;
        (2.0f64.mul_add(t3, (-3.0f64).mul_add(t2, 1.0))).mul_add(
            eetf.ks,
            t2.mul_add(t - 2.0, t)
                .mul_add(1.0 - eetf.ks, t2 * 2.0f64.mul_add(-t, 3.0) * eetf.max_lum),
        )
    };
    // Black lift: `E'' = b·(1−E')⁴ + E'`.
    let lift = 1.0 - e1;
    let lift = lift * lift;
    let e2 = eetf.b.mul_add(lift * lift, e1);
    // Signal → nits → working scale (`/100`), then the ratio scale.
    let y_out = pq_eotf(e2 * eetf.e_src) / SDR_SCALE;
    let scale = y_out / y_src;
    f32_px_bytes([red * scale, green * scale, blue * scale, alpha])
}

/// HDR→SDR tone-mapping kernel — `RgbaF32` → `RgbaF32` luma mapping
/// through the ITU-R BT.2390-8 §5.4 EETF, with the source peak
/// `L_HS`/black `L_min` derived from the frame's [`HdrMetadata`] per
/// call. The two-phase API shape of ADR D1-b: [`new`](Self::new) is
/// the non-RT `prepare` step (the kernel bakes nothing — there is no
/// configuration surface; the frame's own `hdr` drives each call), and
/// [`process`](Self::process) is the allocation-free mapping step.
///
/// ## Mapping rules
///
/// Per pixel, the BT.2020 luma `Y = 0.2627·R + 0.6780·G + 0.0593·B` of
/// the working-space value (`1.0` = 100 nits) is mapped by the EETF in
/// the PQ-signal domain: `e = pq_eotf_inv(Y·100)` normalized by the
/// source-peak signal `e_src = pq_eotf_inv(L_HS)` gives `E`; below the
/// knee `ks = 1.5·maxLum − 0.5` (with `maxLum =
/// pq_eotf_inv(L_DS)/e_src`, `L_DS` = 100 nits) the map is the
/// identity, above it the Hermite spline
/// `t = (E−ks)/(1−ks)`, `E' = (2t³−3t²+1)·ks + (t³−2t²+t)·(1−ks) +
/// (−2t³+3t²)·maxLum` compresses onto the SDR peak; the black-lift
/// term `E'' = b·(1−E')⁴ + E'` with `b = pq_eotf_inv(L_min)/e_src`
/// lifts the near-black range toward the signaled source minimum.
/// `Y_out = pq_eotf(E''·e_src)/100` back on the working scale, and RGB
/// scales by the ratio `Y_out/Y_src` — hue/saturation preserving.
/// Alpha passes through unconverted.
///
/// Passthrough rules: a pixel whose luma is `≤ 0` (negative or zero —
/// no ratio can scale it) copies verbatim, and `ks ≥ 1` — an
/// SDR-equivalent source with `L_HS ≤ L_DS` — passes the whole domain
/// through as the defined non-compression (not a no-op: the pixels
/// still flow through the pass and `dst` still records `ColorInfo`).
/// `NaN`/`±∞` channels propagate per `f32`/`f64` semantics.
///
/// `hdr` absent (or carrying no usable values) maps with the default
/// `L_HS` = 1000 nits and no black lift — untagged HDR is treated as
/// a 1000-nit-mastered source.
///
/// ## Output frame
///
/// `dst` must be `RgbaF32` at the same `width`×`height` and
/// exclusively owned — a shared `dst` (`plane_mut(0)` → `None`) is
/// rejected before a single byte is written. Rows are processed
/// through `Plane::row`/`PlaneMut::row_mut` only, so `stride >
/// row_bytes` padding is never read or written.
///
/// `dst`'s `ColorInfo` propagates `src`'s
/// `primaries`/`transfer`/`matrix`/`range` verbatim — the output stays
/// in the `Bt2020`/`Linear` working-space description — with `hdr`
/// consumed: `hdr: None` always, so a downstream stage cannot re-apply
/// the metadata and double tone-map. `pts`/`duration` propagate from
/// `src`.
///
/// ## Equivalence contract
///
/// The module doc's contract applies: per-channel output is the `f64`
/// reference evaluation rounded to `binary32`; optimized backends must
/// match within ≤ 4 ulp (`binary32`). The `Y_src ≤ 0`/`ks ≥ 1`
/// passthrough paths are bit-exact.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{
///     ColorInfo, ColorPrimaries, ColorRange, MatrixCoefficients,
///     PixelFormat, TransferCharacteristics, VideoFrame,
/// };
/// use ruviform_video_proc::ToneMapSdr;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts).unwrap();
/// // A 1000-nit working-space gray (10.0 on the 1.0 = 100 nits scale).
/// let px: Vec<u8> = [10.0_f32, 10.0, 10.0, 1.0]
///     .iter()
///     .flat_map(|c| c.to_le_bytes())
///     .collect();
/// src.plane_mut(0).unwrap().data_mut().copy_from_slice(&px);
/// src.set_color(ColorInfo {
///     primaries: ColorPrimaries::Bt2020,
///     transfer: TransferCharacteristics::Linear,
///     matrix: MatrixCoefficients::Identity,
///     range: ColorRange::Full,
///     hdr: None, // absent → default L_HS = 1000 nits
/// });
/// let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts).unwrap();
///
/// ToneMapSdr::new().process(&src, &mut dst).unwrap();
///
/// // The source peak maps to the SDR peak — ≈ 1.0 = 100 nits.
/// let row = dst.plane(0).unwrap().row(0).unwrap();
/// let r = f32::from_le_bytes(row[0..4].try_into().unwrap());
/// assert!((r - 1.0).abs() < 1e-6);
/// assert!(dst.color().hdr.is_none());
/// ```
#[derive(Debug)]
pub struct ToneMapSdr;

impl ToneMapSdr {
    /// Builds the kernel — the luma map has no configuration surface
    /// (the frame's `hdr` drives each call), so construction cannot
    /// fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::ToneMapSdr::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Maps `src` into `dst` per the rules on [`ToneMapSdr`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` or `dst` is not `RgbaF32`, the
    /// frames disagree on dimensions, or `dst`'s backing is still
    /// shared (`dst.plane_mut(0)` → `None`). All validation runs
    /// before any write; the in-loop row borrows are infallible under
    /// the `Plane` contract — a failed call never partially fills
    /// `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking
    /// and bounded (exactly `width`×`height` pixel iterations over
    /// borrowed rows; the only scratch is stack channel quads and the
    /// `Copy` [`HdrMetadata`] read). Dropping `src`/`dst` stays the
    /// caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        if src.format() != PixelFormat::RgbaF32 {
            return Err(Error::InvalidValue("src format"));
        }
        if dst.format() != PixelFormat::RgbaF32 {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        // The call-site contract: `hdr` is re-read and the EETF
        // parameters re-derived on every call — infallible, and done
        // ahead of the plane borrows like every other validation.
        let eetf = Eetf::from_hdr(src_color.hdr);
        let src_plane = src.plane(0).ok_or(Error::InvalidValue("src plane"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..src.height() {
                // These row borrows cannot fail: `y < height` —
                // `ok_or` keeps the contract panic-free regardless.
                let src_row = src_plane.row(y).ok_or(Error::InvalidValue("src plane"))?;
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                for (src_px, dst_px) in src_row
                    .as_chunks::<16>()
                    .0
                    .iter()
                    .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                {
                    *dst_px = tone_map_px(&eetf, src_px);
                }
            }
        }
        // `src`'s color description propagates verbatim except `hdr`:
        // consumed by this map, cleared so no downstream stage can
        // re-apply it and double tone-map. Timestamps propagate — a
        // conversion never retimes the frame.
        dst.set_color(ColorInfo {
            hdr: None,
            ..src_color
        });
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for ToneMapSdr {
    /// Equivalent to [`ToneMapSdr::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{
        ColorPrimaries, ColorRange, ContentLightLevel, MasteringDisplayInfo, MatrixCoefficients,
        SharedPlane, TransferCharacteristics,
    };

    use super::*;

    /// The documented per-channel bound on the `binary32` equivalence
    /// contract (f64 reference evaluation → `binary32` rounding).
    const ULP_BOUND: u32 = 4;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Asserts `actual` is within `ulp` `binary32` steps of `expected`
    /// (the `f64` reference value rounded to `binary32`). Sign-
    /// magnitude bit distance is the ulp distance for the same-sign
    /// comparisons used here.
    // The `f64 → f32` narrowing is the contract's own rounding step.
    #[allow(clippy::cast_possible_truncation)]
    fn assert_ulp(actual: f32, expected: f64, ulp: u32) {
        let expected = expected as f32;
        assert!(
            actual.to_bits().abs_diff(expected.to_bits()) <= ulp,
            "expected {expected} ± {ulp} ulp, got {actual}",
        );
    }

    /// Builds an `RgbaF32` frame of one row holding `pixels` as `f32`
    /// channels, tagged `color`.
    fn rgba_f32_frame(
        width: u32,
        height: u32,
        pixels: &[[f32; 4]],
        color: ColorInfo,
    ) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::RgbaF32, width, height, pts()).unwrap();
        frame
            .plane_mut(0)
            .unwrap()
            .data_mut()
            .as_chunks_mut::<16>()
            .0
            .iter_mut()
            .zip(pixels.iter())
            .for_each(|(dst, px)| *dst = f32_px_bytes_f32(*px));
        frame.set_color(color);
        frame
    }

    /// `f32` channel quad → 16-byte little-endian `binary32` pixel —
    /// the test-side writer (`f32` input, no `f64` round trip).
    fn f32_px_bytes_f32(px: [f32; 4]) -> [u8; 16] {
        let mut out = [0_u8; 16];
        for (i, ch) in px.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&ch.to_le_bytes());
        }
        out
    }

    /// Reads the `x`-th pixel of an `RgbaF32` frame's plane as four
    /// `f32` channels.
    fn f32_px(frame: &VideoFrame, x: usize, y: u32) -> [f32; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let px: &[u8; 16] = row[16 * x..16 * x + 16].try_into().unwrap();
        std::array::from_fn(|i| {
            f32::from_le_bytes([px[4 * i], px[4 * i + 1], px[4 * i + 2], px[4 * i + 3]])
        })
    }

    /// The working-space `ColorInfo` — `{Bt2020, Linear, Identity,
    /// Full}` — with the given `hdr`.
    fn working_tag(hdr: Option<HdrMetadata>) -> ColorInfo {
        ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Linear,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr,
        }
    }

    /// `HdrMetadata` carrying only a mastering display pair (raw
    /// 1/10 000 cd/m² units).
    fn mastering(max_luminance: u32, min_luminance: u32) -> HdrMetadata {
        HdrMetadata {
            mastering_display: Some(MasteringDisplayInfo {
                max_luminance,
                min_luminance,
                ..MasteringDisplayInfo::default()
            }),
            content_light_level: None,
        }
    }

    /// `HdrMetadata` carrying only a content light level (`max_cll` in
    /// cd/m² direct).
    fn cll(max_cll: u16) -> HdrMetadata {
        HdrMetadata {
            mastering_display: None,
            content_light_level: Some(ContentLightLevel {
                max_cll,
                max_fall: 0,
            }),
        }
    }

    /// Golden: `hdr` absent → default `L_HS` = 1000 nits. A 1000-nit
    /// gray (`10.0` on the working scale, BT.2020 weights sum to 1.0)
    /// sits at `E = 1` → Hermite `t = 1` → `E' = maxLum` → the SDR
    /// peak ≈ 1.0 — compressed from 10.0 as required.
    #[test]
    fn tone_map_sdr_maps_default_1000nit_source() {
        let src = rgba_f32_frame(1, 1, &[[10.0, 10.0, 10.0, 0.75]], working_tag(None));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        // `f64` reference: `pq_eotf(maxLum·e_src)/100` at `L_HS = 1000`.
        for ch in [r, g, b] {
            assert_ulp(ch, 1.000_000_000_000_052_4, ULP_BOUND);
            assert!(ch < 10.0);
        }
        // Alpha is the unmapped passthrough channel.
        assert_ulp(a, 0.75, ULP_BOUND);
    }

    /// EETF shape under the default `L_HS` = 1000: below the knee
    /// (`ks` ≈ 0.5137 → inputs up to ≈ 27.6 nits) `E' = E` is the
    /// luminance identity, above it the spline compresses — a 50-nit
    /// gray lands at ≈ 0.4614.
    #[test]
    fn tone_map_sdr_eetf_shape_below_and_above_the_knee() {
        let src = rgba_f32_frame(
            3,
            1,
            &[
                [0.1, 0.1, 0.1, 1.0],    // 10 nits — below the knee
                [0.25, 0.25, 0.25, 1.0], // 25 nits — still below
                [0.5, 0.5, 0.5, 1.0],    // 50 nits — compressed
            ],
            working_tag(None),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 3, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        // Below the knee the map is the luma identity (`b = 0` →
        // `E'' = E` → `Y_out = Y_src`): output ≈ input within the
        // `pq_eotf`∘`pq_eotf_inv` round-trip noise.
        for ch in &f32_px(&dst, 0, 0)[0..3] {
            assert_ulp(*ch, 0.099_999_999_999_999_5, ULP_BOUND);
        }
        for ch in &f32_px(&dst, 1, 0)[0..3] {
            assert_ulp(*ch, 0.249_999_999_999_990_15, ULP_BOUND);
        }
        for ch in &f32_px(&dst, 2, 0)[0..3] {
            assert_ulp(*ch, 0.461_429_547_546_645_8, ULP_BOUND);
        }
    }

    /// Golden: `mastering_display.max_luminance = 4_000_000` (400 nits
    /// in the 1/10 000 cd/m² ST 2086 units) wins over a simultaneously
    /// present `max_cll` — the priority order is mastering → `MaxCLL` →
    /// default. A 400-nit gray is the declared source peak and maps to
    /// the SDR peak ≈ 1.0 (with `L_HS` = 2000 it would be ≈ 0.94).
    #[test]
    fn tone_map_sdr_l_hs_prefers_mastering_display() {
        let src = rgba_f32_frame(
            1,
            1,
            &[[4.0, 4.0, 4.0, 1.0]],
            working_tag(Some(HdrMetadata {
                mastering_display: Some(MasteringDisplayInfo {
                    max_luminance: 4_000_000,
                    min_luminance: 0,
                    ..MasteringDisplayInfo::default()
                }),
                content_light_level: Some(ContentLightLevel {
                    max_cll: 2_000,
                    max_fall: 500,
                }),
            })),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        let [r, ..] = f32_px(&dst, 0, 0);
        assert_ulp(r, 1.000_000_000_000_052_4, ULP_BOUND);
    }

    /// Golden: `mastering_display` absent (or its `max_luminance`
    /// invalid at 0) → `content_light_level.max_cll` in cd/m² direct
    /// is the source peak — 2000 nits maps a 2000-nit gray to the SDR
    /// peak ≈ 1.0. Both derivations produce byte-identical output.
    #[test]
    fn tone_map_sdr_l_hs_falls_back_to_max_cll() {
        let pixels = [[20.0, 20.0, 20.0, 1.0], [2.0, 2.0, 2.0, 1.0]];
        let src_cll = rgba_f32_frame(2, 1, &pixels, working_tag(Some(cll(2_000))));
        let src_zero_max = rgba_f32_frame(
            2,
            1,
            &pixels,
            working_tag(Some(HdrMetadata {
                mastering_display: Some(MasteringDisplayInfo {
                    max_luminance: 0, // invalid — falls through to MaxCLL
                    min_luminance: 0,
                    ..MasteringDisplayInfo::default()
                }),
                content_light_level: Some(ContentLightLevel {
                    max_cll: 2_000,
                    max_fall: 500,
                }),
            })),
        );
        let mut dst_cll = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        let mut dst_zero = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src_cll, &mut dst_cll).unwrap();
        ToneMapSdr::new()
            .process(&src_zero_max, &mut dst_zero)
            .unwrap();
        assert_eq!(
            dst_cll.plane(0).unwrap().data(),
            dst_zero.plane(0).unwrap().data()
        );
        let [r0, ..] = f32_px(&dst_cll, 0, 0);
        let [r1, ..] = f32_px(&dst_cll, 1, 0);
        assert_ulp(r0, 1.000_000_000_000_052_4, ULP_BOUND);
        assert_ulp(r1, 0.799_883_421_466_246_4, ULP_BOUND);
    }

    /// Both derivations absent/0 → the default `L_HS` = 1000 nits —
    /// `hdr: Some(..)` with only a zero `max_cll` behaves like no
    /// `hdr` at all.
    #[test]
    fn tone_map_sdr_l_hs_defaults_to_1000_nits() {
        let src_none = rgba_f32_frame(1, 1, &[[10.0, 10.0, 10.0, 1.0]], working_tag(None));
        let src_zero = rgba_f32_frame(1, 1, &[[10.0, 10.0, 10.0, 1.0]], working_tag(Some(cll(0))));
        let mut dst_none = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let mut dst_zero = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src_none, &mut dst_none).unwrap();
        ToneMapSdr::new().process(&src_zero, &mut dst_zero).unwrap();
        assert_eq!(
            dst_none.plane(0).unwrap().data(),
            dst_zero.plane(0).unwrap().data()
        );
        assert_ulp(f32_px(&dst_zero, 0, 0)[0], 1.0, ULP_BOUND);
    }

    /// `L_min` drives the black-lift term: a consistent `min < max`
    /// mastering pair (`min_luminance` = `5_000` → 0.5 nits) gives
    /// `b = pq_eotf_inv(0.5)/e_src` ≈ 0.156, lifting a 1-nit gray to
    /// ≈ 2.36 nits — an absent/`min ≥ max` pair keeps `b = 0` and
    /// leaves the below-knee identity untouched.
    #[test]
    fn tone_map_sdr_l_min_drives_the_black_lift() {
        let lifted = rgba_f32_frame(
            1,
            1,
            &[[0.01, 0.01, 0.01, 1.0]],
            working_tag(Some(mastering(10_000_000, 5_000))),
        );
        let unlifted = rgba_f32_frame(
            1,
            1,
            &[[0.01, 0.01, 0.01, 1.0]],
            working_tag(Some(mastering(10_000_000, 0))),
        );
        let mut dst_lifted = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let mut dst_unlifted = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToneMapSdr::new().process(&lifted, &mut dst_lifted).unwrap();
        ToneMapSdr::new()
            .process(&unlifted, &mut dst_unlifted)
            .unwrap();
        let [r_lifted, ..] = f32_px(&dst_lifted, 0, 0);
        let [r_unlifted, ..] = f32_px(&dst_unlifted, 0, 0);
        assert_ulp(r_lifted, 0.023_590_624_182_081_78, ULP_BOUND);
        assert!(r_lifted > r_unlifted);
        // `min_luminance` = 0 is still a consistent `min < max` value —
        // adopted, so `b = pq_eotf_inv(0)/e_src` ≈ 9.5e-7 (not the
        // literal `b = 0` fallback).
        assert_ulp(r_unlifted, 0.010_000_058_893_875_088, ULP_BOUND);
    }

    /// An inverted `min ≥ max` ordering is ignored — `b = 0` — so the
    /// output matches the `b = 0` derivation byte-for-byte (same
    /// `L_HS` = 1000 through `MaxCLL`).
    #[test]
    fn tone_map_sdr_inverted_l_min_is_ignored() {
        let invalid = rgba_f32_frame(
            2,
            1,
            &[[0.01, 0.01, 0.01, 1.0], [5.0, 5.0, 5.0, 1.0]],
            working_tag(Some(mastering(10_000_000, 20_000_000))),
        );
        let reference = rgba_f32_frame(
            2,
            1,
            &[[0.01, 0.01, 0.01, 1.0], [5.0, 5.0, 5.0, 1.0]],
            working_tag(Some(cll(1_000))),
        );
        let mut dst_invalid = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        let mut dst_reference = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToneMapSdr::new()
            .process(&invalid, &mut dst_invalid)
            .unwrap();
        ToneMapSdr::new()
            .process(&reference, &mut dst_reference)
            .unwrap();
        assert_eq!(
            dst_invalid.plane(0).unwrap().data(),
            dst_reference.plane(0).unwrap().data()
        );
    }

    /// `L_HS ≤ L_DS` (100 nits, `max_luminance` = `1_000_000`) makes
    /// `ks` = 1 → all-domain passthrough, including pixels far above
    /// the declared peak — the defined non-compression, bit-exact.
    #[test]
    fn tone_map_sdr_sdr_equivalent_source_passes_through() {
        let src = rgba_f32_frame(
            3,
            1,
            &[
                [50.0, 50.0, 50.0, 1.0], // 5000 nits ≫ declared peak
                [0.5, 0.5, 0.5, 0.25],
                [-1.0, -1.0, -1.0, 0.5],
            ],
            working_tag(Some(mastering(1_000_000, 100))),
        );
        let before: Vec<u8> = src.plane(0).unwrap().data().to_vec();
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 3, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), &before[..]);
    }

    /// `Y_src ≤ 0` pixels cannot be ratio-scaled — negative- and
    /// zero-luma pixels copy verbatim while positive-luma neighbours
    /// still map (the kernel is not blanket-passthrough).
    #[test]
    fn tone_map_sdr_non_positive_luma_passes_through() {
        let pixels = [
            [-0.5, -0.5, -0.5, 0.75],
            [0.0, 0.0, 0.0, 1.0],
            [-2.0, 0.5, 0.0, 0.5], // mixed channels, luma ≈ −0.19
            [0.0, 2.0, 0.0, 1.0],  // positive luma → mapped
        ];
        let src = rgba_f32_frame(4, 1, &pixels, working_tag(None));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 4, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        // Strict passthrough: bit-exact channel copies, alpha included.
        for (x, px) in pixels[0..3].iter().enumerate() {
            assert_eq!(f32_px(&dst, x, 0).map(f32::to_bits), px.map(f32::to_bits));
        }
        // The positive-luma pixel was ratio-scaled (luma 2·0.678 →
        // compressed below input).
        let [_, g, _, _] = f32_px(&dst, 3, 0);
        assert!(g < 2.0);
        assert!(g > 0.0);
    }

    /// Ratio preservation: a non-gray pixel's luma compresses while
    /// each RGB channel scales by the same `Y_out/Y_src` — `f64`
    /// reference values for `[4, 1, 0.5]` (luma 1.75845) and
    /// `[-0.5, 2, 0]` (luma 1.22465, negative channel carried through
    /// unclamped) at `L_HS` = 1000.
    #[test]
    fn tone_map_sdr_scales_rgb_by_the_luma_ratio() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[4.0, 1.0, 0.5, 0.5], [-0.5, 2.0, 0.0, 1.0]],
            working_tag(None),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        let [r0, g0, b0, a0] = f32_px(&dst, 0, 0);
        assert_ulp(r0, 1.936_431_832_208_525_7, ULP_BOUND);
        assert_ulp(g0, 0.484_107_958_052_131_4, ULP_BOUND);
        assert_ulp(b0, 0.242_053_979_026_065_7, ULP_BOUND);
        assert_ulp(a0, 0.5, ULP_BOUND);
        let [r1, g1, b1, a1] = f32_px(&dst, 1, 0);
        assert_ulp(r1, -0.308_798_431_706_376_2, ULP_BOUND);
        assert_ulp(g1, 1.235_193_726_825_504_9, ULP_BOUND);
        assert_ulp(b1, 0.0, ULP_BOUND);
        assert_ulp(a1, 1.0, ULP_BOUND);
    }

    /// `src`'s working-space description is not validated — a frame
    /// tagged `Bt709`/`Bt709`/`Bt601`/`Limited` maps pixel-identically
    /// to the working-space tag (metadata interpretation is consumer
    /// responsibility) and the tags propagate to `dst` verbatim.
    #[test]
    fn tone_map_sdr_does_not_validate_the_working_space_tag() {
        let pixels = [[5.0, 1.0, 0.25, 1.0]];
        let tagged = rgba_f32_frame(1, 1, &pixels, working_tag(None));
        let mistagged = rgba_f32_frame(
            1,
            1,
            &pixels,
            ColorInfo {
                primaries: ColorPrimaries::Bt709,
                transfer: TransferCharacteristics::Bt709,
                matrix: MatrixCoefficients::Bt601,
                range: ColorRange::Limited,
                hdr: None,
            },
        );
        let mut dst_tagged = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let mut dst_mistagged = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToneMapSdr::new().process(&tagged, &mut dst_tagged).unwrap();
        ToneMapSdr::new()
            .process(&mistagged, &mut dst_mistagged)
            .unwrap();
        assert_eq!(
            dst_tagged.plane(0).unwrap().data(),
            dst_mistagged.plane(0).unwrap().data()
        );
        // The non-working-space tag propagated verbatim (modulo `hdr`).
        assert_eq!(dst_mistagged.color(), mistagged.color());
    }

    /// `hdr` consumption: the output records `src`'s color description
    /// with `hdr: None` — `primaries`/`transfer`/`matrix`/`range` and
    /// the timestamps propagate.
    #[test]
    fn tone_map_sdr_consumes_hdr_and_propagates_metadata() {
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let mut src = rgba_f32_frame(
            1,
            1,
            &[[10.0, 10.0, 10.0, 1.0]],
            working_tag(Some(mastering(10_000_000, 5_000))),
        );
        src.set_duration(Some(duration));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        // Scrub `dst`'s timestamps so the propagation is observable.
        dst.set_pts(MediaTime::ZERO);
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        assert_eq!(
            dst.color(),
            ColorInfo {
                hdr: None,
                ..src.color()
            }
        );
        assert_eq!(dst.pts(), src.pts());
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Format/dimension/shared-backing rejections all happen before
    /// any write — a failed call leaves `dst` untouched.
    #[test]
    fn tone_map_sdr_invalid_inputs_are_rejected() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 1.0, 1.0, 1.0]], working_tag(None));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();

        // Non-`RgbaF32` sources.
        let src_u8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            ToneMapSdr::new().process(&src_u8, &mut dst),
            Err(Error::InvalidValue("src format"))
        );
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        assert_eq!(
            ToneMapSdr::new().process(&src_yuv, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Non-`RgbaF32` destinations.
        let mut dst_u8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            ToneMapSdr::new().process(&src, &mut dst_u8),
            Err(Error::InvalidValue("dst format"))
        );
        // The rejected `dst` is untouched — pixels, `ColorInfo`, pts.
        assert_eq!(dst_u8.plane(0).unwrap().data(), &[0; 4]);
        assert_eq!(dst_u8.color(), ColorInfo::default());
        assert_eq!(dst_u8.pts(), pts());

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        assert_eq!(
            ToneMapSdr::new().process(&src, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // A shared `dst` (clone still holding the `Arc` backing) —
        // rejected before any write.
        let mut dst_shared = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let sharing_clone = dst_shared.clone();
        assert!(dst_shared.plane_mut(0).is_none());
        assert_eq!(
            ToneMapSdr::new().process(&src, &mut dst_shared),
            Err(Error::InvalidValue("dst plane"))
        );
        // Rejection wrote nothing.
        assert_eq!(dst_shared.plane(0).unwrap().data(), &[0; 16]);
        drop(sharing_clone);
        assert!(ToneMapSdr::new().process(&src, &mut dst_shared).is_ok());

        assert_eq!(dst.plane(0).unwrap().data(), &[0; 16]);
    }

    /// A `stride > row_bytes` destination: the map writes rows only —
    /// padding bytes keep their caller-chosen contents.
    #[test]
    fn tone_map_sdr_padded_dst_stride_is_never_written() {
        // 1×2 `RgbaF32` dst: row_bytes 16, padded stride 20 → usable
        // 20 + 16 = 36 bytes — row 0 at 0..16, inter-row padding at
        // 16..20, row 1 at 20..36.
        let src = rgba_f32_frame(
            1,
            2,
            &[[10.0, 10.0, 10.0, 1.0], [10.0, 10.0, 10.0, 1.0]],
            working_tag(None),
        );
        let mut dst = VideoFrame::from_shared_planes(
            PixelFormat::RgbaF32,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    vec![
                        0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xEE, 0xEE, 0xEE, 0xEE,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ]
                    .into_boxed_slice(),
                ),
                stride: 20,
            }],
        )
        .unwrap();
        assert_eq!(dst.plane(0).unwrap().stride(), 20);
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        let data = dst.plane(0).unwrap().data();
        // Both rows written (source peak → SDR peak ≈ 1.0); the
        // inter-row padding keeps 0xEE.
        assert_ulp(f32_px(&dst, 0, 0)[0], 1.0, ULP_BOUND);
        assert_eq!(&data[16..20], &[0xEE; 4]);
        assert_ulp(f32_px(&dst, 0, 1)[0], 1.0, ULP_BOUND);
    }

    /// A `stride > row_bytes` source: the map reads the row region
    /// only — padding bytes never enter the pipeline.
    #[test]
    fn tone_map_sdr_padded_src_stride_reads_rows_only() {
        // 1×2 `RgbaF32` src: row_bytes 16, padded stride 20 → usable
        // 20 + 16 = 36 bytes. 0xEE is not pixel data: a read straying
        // into padding would corrupt the outputs asserted below.
        let mut src = VideoFrame::from_shared_planes(
            PixelFormat::RgbaF32,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    [
                        10.0_f32.to_le_bytes(),
                        10.0_f32.to_le_bytes(),
                        10.0_f32.to_le_bytes(),
                        1.0_f32.to_le_bytes(),
                        [0xEE_u8; 4], // the inter-row padding
                        0.1_f32.to_le_bytes(),
                        0.1_f32.to_le_bytes(),
                        0.1_f32.to_le_bytes(),
                        1.0_f32.to_le_bytes(),
                    ]
                    .concat()
                    .into_boxed_slice(),
                ),
                stride: 20,
            }],
        )
        .unwrap();
        assert_eq!(src.plane(0).unwrap().stride(), 20);
        src.set_color(working_tag(None));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 2, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        // Row 0 = source peak → SDR peak; row 1 = below-knee identity.
        assert_ulp(f32_px(&dst, 0, 0)[0], 1.000_000_000_000_052_4, ULP_BOUND);
        assert_ulp(f32_px(&dst, 0, 1)[0], 0.099_999_999_999_999_5, ULP_BOUND);
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_tone_map_sdr_is_repeatable_and_deterministic() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[10.0, 4.0, 0.5, 1.0], [0.01, 0.01, 0.01, 0.5]],
            working_tag(Some(mastering(10_000_000, 5_000))),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        let mut kernel = ToneMapSdr::new();
        let mut first = [0_u8; 32];
        for i in 0..64 {
            kernel.process(&src, &mut dst).unwrap();
            let row = dst.plane(0).unwrap().row(0).unwrap();
            if i == 0 {
                first.copy_from_slice(row);
            } else {
                assert_eq!(row, &first);
            }
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// `src` stays byte-identical across the call (a borrow, not a
    /// drain).
    #[test]
    fn rt_process_tone_map_sdr_never_mutates_source() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.25, 0.5, 0.75, 1.0], [15.0, 20.0, 25.0, 0.5]],
            working_tag(Some(cll(4_000))),
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToneMapSdr::new().process(&src, &mut dst).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }
}
