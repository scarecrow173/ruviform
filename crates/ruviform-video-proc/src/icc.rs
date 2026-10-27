//! ICC profile transform kernel: [`IccTransform`] converts packed RGBA
//! frames through a caller-supplied pair of ICC profiles — the
//! profile-driven color path complementing the CICP-driven
//! [`ToWorking`](crate::ToWorking)/[`FromWorking`](crate::FromWorking)
//! working-space kernels.
//!
//! This module exists only under Cargo feature `icc` (`default = []` —
//! the feature-off base path keeps zero external dependencies). The ICC
//! engine is a private implementation detail: no engine types, errors or
//! feature names appear in the public API — the `rtrb` precedent.
//!
//! ## Conversion rules
//!
//! [`new`](IccTransform::new) (the non-RT `prepare` step of the ADR D1-b
//! two-phase shape) parses both profile byte strings and bakes the
//! f32 executor with the contract-fixed options — rendering intent
//! mapped from [`IccIntent`], CICP-transfer reuse allowed, floating
//! point (never fixed point), extended-range RGB↔XYZ enabled,
//! default trilinear interpolation. [`process`](IccTransform::process)
//! accepts `Rgba8`/`RgbaF32` in any combination (same format included)
//! at matching `width`×`height`.
//!
//! Per pixel: the input normalizes into a `[f32; 4]` stack quad —
//! `Rgba8` channels map by `v / 255`, `RgbaF32` channels read as raw
//! little-endian `binary32` — and the executor runs on that single
//! pixel's slice (one 4-element call per pixel; no row buffer, so
//! `process` is allocation-free by construction). The output encodes
//! as raw `binary32` for `RgbaF32` — extended range preserved, values
//! below `0` and above `1` included — and as
//! `round(clamp(v, 0, 1) · 255)` for `Rgba8`. Alpha is never color
//! transformed: the engine's RGBA layout carries channel 3 through
//! unmodified (verified against the engine implementation and pinned
//! by `alpha_passthrough` tests below).
//!
//! ## Output frame
//!
//! `dst` must match `src`'s `width`×`height` and be exclusively owned —
//! a shared `dst` (`plane_mut(0)` → `None`) is rejected before a single
//! byte is written. Rows are processed through
//! `Plane::row`/`PlaneMut::row_mut` only, so `stride > row_bytes`
//! padding is never read or written.
//!
//! `dst`'s `ColorInfo` records `{primaries: Unspecified, transfer:
//! Unspecified, matrix: Identity, range: Full, hdr: None}` — an ICC
//! profile's described color space does not map onto the CICP enums,
//! so `Unspecified` is the honest record (a consumer that knows the
//! destination space can overwrite via `set_color`). `pts`/`duration`
//! propagate from `src`; a color conversion never retimes the frame.
//!
//! ## Equivalence contract
//!
//! The baked executor is deterministic per build configuration —
//! scalar engine paths only (no SIMD engine features are enabled),
//! but moxcms `mlaf` still selects fused FMA on `aarch64` (NEON
//! baseline) and `target_feature = "fma"` `x86`/`x86_64`, so bit output
//! can differ in the last ulp across target-features/architectures.
//! The normalization/quantization shims above are exact, so per-pixel
//! output is **bit-exact** against the executor output of the pinned
//! engine version + feature set on the same build configuration.
//! Literal golden vectors in this module's tests are pinned to the
//! `x86_64` non-FMA baseline (the CI `icc-verify` lane) and `cfg`-gated
//! accordingly. Bump the engine version or its enabled features and
//! the golden vectors must be re-derived; cross-engine equivalence
//! (lcms2/qcms etc.) is out of contract.
//!
//! Contract source: `docs/designs/color-conversion-kernels.md` and
//! `docs/test-specs/icc-transform-contract-tests.md`.

use std::sync::Arc;

use moxcms::{
    ColorProfile, DataColorSpace, Layout, RenderingIntent, TransformF32Executor, TransformOptions,
};
use ruviform_media_core::{Error, Result};
use ruviform_video_core::{
    ColorInfo, ColorPrimaries, ColorRange, MatrixCoefficients, PixelFormat,
    TransferCharacteristics, VideoFrame,
};

/// Reads one `Rgba8` pixel's normalized signal channels (`v / 255`) —
/// the `[0, 1]` input the f32 executor requires. A single `binary32`
/// division per channel: exact, deterministic.
fn rgba8_signal(px: [u8; 4]) -> [f32; 4] {
    std::array::from_fn(|i| f32::from(px[i]) / 255.0)
}

/// Reads one `RgbaF32` pixel's signal channels — raw little-endian
/// `binary32` quads (`NaN`/`±∞` pass through to the executor).
fn rgba_f32_signal(px: &[u8; 16]) -> [f32; 4] {
    std::array::from_fn(|i| {
        f32::from_le_bytes([px[4 * i], px[4 * i + 1], px[4 * i + 2], px[4 * i + 3]])
    })
}

/// Serializes one executor output quad into a 16-byte `RgbaF32`
/// pixel — raw `binary32` little-endian, extended range kept (no
/// clamp: HDR/wide-gamut profile transforms must not lose
/// below-`0`/above-`1` information).
fn f32_px_bytes(px: [f32; 4]) -> [u8; 16] {
    let mut out = [0_u8; 16];
    for (i, ch) in px.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&ch.to_le_bytes());
    }
    out
}

/// `round(clamp(v, 0, 1) · 255)` quantization (round-half-up, evaluated
/// in `f64` on the exactly-widened `f32` channel) for `Rgba8` output —
/// the same rule every `Rgba8`-emitting kernel in this crate applies.
/// Negative and `> 1` signals saturate; `NaN` folds to `0` (explicit —
/// the `as u8` cast would land on `0` anyway).
// `normalized * 255` is in `[0, 255]` and `.round()` makes it integral
// and non-negative — the cast can neither truncate nor lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize8(v: f32) -> u8 {
    let v = f64::from(v);
    let normalized = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    (normalized * 255.0).round() as u8
}

/// The ICC rendering intent — which of the profile's intents the baked
/// transform applies. Maps internally onto the engine's intent set;
/// the mapping is private, engine types never leave this module.
///
/// # Examples
///
/// ```
/// use ruviform_video_proc::IccIntent;
///
/// let intent = IccIntent::Perceptual;
/// assert_eq!(intent, IccIntent::Perceptual);
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IccIntent {
    /// Perceptual rendering — the profile's perceptual table (or its
    /// colorimetric fallback when the profile carries none).
    Perceptual,
    /// Relative colorimetric rendering — media-relative colorimetry
    /// with white-point adaptation.
    RelativeColorimetric,
}

impl IccIntent {
    /// Maps onto the engine's rendering-intent value. Exhaustive on
    /// purpose: adding an [`IccIntent`] variant must make this match
    /// fail to compile so the new mapping is decided deliberately.
    fn rendering_intent(self) -> RenderingIntent {
        match self {
            Self::Perceptual => RenderingIntent::Perceptual,
            Self::RelativeColorimetric => RenderingIntent::RelativeColorimetric,
        }
    }
}

/// Prepared ICC profile transform — `Rgba8`/`RgbaF32` →
/// `Rgba8`/`RgbaF32` conversion through the constructor-baked profile
/// pair. See the module doc for the conversion rules, the
/// unspecified-`ColorInfo` record and the bit-exact equivalence
/// contract.
///
/// The engine profile pair and executor are private implementation
/// details: [`new`](Self::new) is the non-RT `prepare` step that does
/// the allocating parse + executor construction, and
/// [`process`](Self::process) is the allocation-free conversion step.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::{IccIntent, IccTransform};
///
/// // Callers own the profile byte strings (media-io/app layer duty);
/// // a fixture sRGB profile stands in here.
/// let srgb = include_bytes!("../tests/fixture/srgb.icc");
/// let mut kernel = IccTransform::new(srgb, srgb, IccIntent::Perceptual).unwrap();
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// src.plane_mut(0)
///     .unwrap()
///     .data_mut()
///     .copy_from_slice(&[255, 0, 0, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// kernel.process(&src, &mut dst).unwrap();
/// ```
pub struct IccTransform {
    executor: Arc<TransformF32Executor>,
}

// The engine executor is not `Debug`; the kernel's debug form stays
// non-exhaustive so no engine internals leak.
impl std::fmt::Debug for IccTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IccTransform").finish_non_exhaustive()
    }
}

impl IccTransform {
    /// Builds the prepared kernel: parses `src_profile`/`dst_profile`
    /// (caller-owned ICC byte strings — acquiring them from media
    /// metadata is the media-io/app layer's job) and bakes the f32
    /// executor with the contract-fixed options — `intent` mapped
    /// onto the engine intent, CICP-transfer reuse allowed, floating
    /// point (never fixed point), extended-range RGB↔XYZ on, default
    /// trilinear interpolation.
    ///
    /// **RT contract:** non-RT — profile parsing and executor
    /// construction allocate; this is the `prepare` phase of the
    /// two-phase kernel shape.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `"icc profile"` when either byte
    /// string fails to parse as an ICC profile; `"icc transform"` when
    /// either profile's color space is not RGB or the profile pair
    /// cannot build an RGBA→RGBA transform. Engine errors never
    /// escape as engine types.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_video_proc::{IccIntent, IccTransform};
    ///
    /// let srgb = include_bytes!("../tests/fixture/srgb.icc");
    /// let _kernel = IccTransform::new(srgb, srgb, IccIntent::RelativeColorimetric).unwrap();
    /// // Not-ICC bytes are rejected before any executor exists.
    /// assert!(IccTransform::new(b"not a profile", srgb, IccIntent::Perceptual).is_err());
    /// ```
    pub fn new(src_profile: &[u8], dst_profile: &[u8], intent: IccIntent) -> Result<Self> {
        let src = ColorProfile::new_from_slice(src_profile)
            .map_err(|_| Error::InvalidValue("icc profile"))?;
        let dst = ColorProfile::new_from_slice(dst_profile)
            .map_err(|_| Error::InvalidValue("icc profile"))?;
        // `Layout::Rgba` is only honest for RGB-space profiles:
        // moxcms also accepts it for CMYK/Lab/multi-channel color
        // spaces, where channel 3 stops being the alpha passthrough
        // (a CMYK executor would consume it as the K ink and emit
        // opaque alpha) — reject before the executor is built.
        if src.color_space != DataColorSpace::Rgb || dst.color_space != DataColorSpace::Rgb {
            return Err(Error::InvalidValue("icc transform"));
        }
        let executor = src
            .create_transform_f32(
                Layout::Rgba,
                &dst,
                Layout::Rgba,
                TransformOptions {
                    rendering_intent: intent.rendering_intent(),
                    allow_use_cicp_transfer: true,
                    // The f32 executor's reference precision must not
                    // drop to the Q2.13 fixed-point path.
                    prefer_fixed_point: false,
                    // Matrix-shaper gamma evaluation keeps [0, 1]
                    // excursions — HDR/wide-gamut `RgbaF32` output must
                    // not clamp (the `extended_range` engine feature).
                    allow_extended_range_rgb_xyz: true,
                    // Contract-fixed defaults: trilinear interpolation,
                    // low barycentric weight scale.
                    ..TransformOptions::default()
                },
            )
            .map_err(|_| Error::InvalidValue("icc transform"))?;
        Ok(Self { executor })
    }

    /// Runs the baked executor on one normalized RGBA pixel — a
    /// single-pixel call (`src`/`dst` as 4-channel slices) per the
    /// allocation-free contract; alpha flows through the engine's
    /// RGBA passthrough unchanged.
    fn transform_px(&self, src_px: [f32; 4]) -> Result<[f32; 4]> {
        let mut dst_px = [0.0_f32; 4];
        self.executor
            .transform(&src_px, &mut dst_px)
            .map_err(|_| Error::InvalidValue("icc transform"))?;
        Ok(dst_px)
    }

    /// Converts `src` into `dst` per the rules on [`IccTransform`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src`/`dst` is not `Rgba8`/`RgbaF32`
    /// (planar `Yuv*p`, `RgbaF16` or an unknown format variant), the
    /// frames disagree on dimensions, `dst`'s backing is still shared
    /// (`dst.plane_mut(0)` → `None`), or the executor itself fails on
    /// a pixel (`"icc transform"` — unreachable for the baked RGBA
    /// layouts, kept panic-free). All input validation runs before
    /// any write — a validation failure never partially fills `dst` —
    /// but a mid-frame engine failure is not atomic: already-written
    /// rows keep their converted pixels.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking
    /// and bounded (exactly `width`×`height` single-pixel executor
    /// calls over borrowed rows; the only scratch is stack channel
    /// quads; the executor is a pure mapping on the pipeline the
    /// constructor pre-built). Dropping `src`/`dst` stays the
    /// caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        let src_format = src.format();
        match src_format {
            PixelFormat::Rgba8 | PixelFormat::RgbaF32 => {}
            // Planar `Yuv*p`, `RgbaF16` and every future
            // `#[non_exhaustive]` variant.
            _ => return Err(Error::InvalidValue("src format")),
        }
        let dst_format = dst.format();
        match dst_format {
            PixelFormat::Rgba8 | PixelFormat::RgbaF32 => {}
            _ => return Err(Error::InvalidValue("dst format")),
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
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
                // `src_format`/`dst_format` were captured before the
                // `plane_mut` borrow — `dst.format()` cannot be
                // called here.
                match (src_format, dst_format) {
                    (PixelFormat::Rgba8, PixelFormat::Rgba8) => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                        {
                            let out = self.transform_px(rgba8_signal(*src_px))?;
                            *dst_px = [
                                quantize8(out[0]),
                                quantize8(out[1]),
                                quantize8(out[2]),
                                quantize8(out[3]),
                            ];
                        }
                    }
                    (PixelFormat::Rgba8, PixelFormat::RgbaF32) => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px = f32_px_bytes(self.transform_px(rgba8_signal(*src_px))?);
                        }
                    }
                    (PixelFormat::RgbaF32, PixelFormat::Rgba8) => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                        {
                            let out = self.transform_px(rgba_f32_signal(src_px))?;
                            *dst_px = [
                                quantize8(out[0]),
                                quantize8(out[1]),
                                quantize8(out[2]),
                                quantize8(out[3]),
                            ];
                        }
                    }
                    (PixelFormat::RgbaF32, PixelFormat::RgbaF32) => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px = f32_px_bytes(self.transform_px(rgba_f32_signal(src_px))?);
                        }
                    }
                    // Validated above — this arm only keeps the match
                    // panic-free on future `PixelFormat` variants.
                    _ => return Err(Error::InvalidValue("dst format")),
                }
            }
        }
        // The ICC-described space has no honest CICP enum mapping —
        // record `Unspecified` (consumers may overwrite via
        // `set_color`); the timestamps propagate — a color conversion
        // never retimes the frame.
        dst.set_color(ColorInfo {
            primaries: ColorPrimaries::Unspecified,
            transfer: TransferCharacteristics::Unspecified,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: None,
        });
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use moxcms::{
        LocalizableString, LutDataType, LutMultidimensionalType, LutStore, LutType, LutWarehouse,
        Matrix3d, ProfileClass, ProfileText, ToneReprCurve, Vector3d, white_point_d50,
    };
    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{ContentLightLevel, HdrMetadata, SharedPlane};

    use super::*;

    /// The committed ICC fixtures — moxcms `encode()` output of the
    /// pinned version's built-in profiles (see `generate_fixtures`).
    const SRGB: &[u8] = include_bytes!("../tests/fixture/srgb.icc");
    const DISPLAY_P3: &[u8] = include_bytes!("../tests/fixture/display_p3.icc");
    const GRAY_GAMMA22: &[u8] = include_bytes!("../tests/fixture/gray_gamma22.icc");
    /// Synthetic LUT-tag fixtures — `LutWarehouse` A2B/B2A tags on
    /// hand-built RGB profiles, encoded by the pinned engine (see
    /// `generate_fixtures` and the builders below).
    ///
    /// - `LUT_INTENT_PAIR`: mft2 A2B0 = ×0.5 / A2B1 = identity and
    ///   B2A0 = ×0.5 / B2A1 = identity — Perceptual and
    ///   `RelativeColorimetric` bake observably different mappings.
    /// - `LUT_MFT1`: mft1 (`Store8`, 256-entry in/out tables) R↔B
    ///   channel swap in both directions and both intents.
    /// - `LUT_MFT2`: mft2 (`Store16`, 2-entry in/out tables) — A2B
    ///   ×0.5 and B2A ×0.75 scales in both intents.
    /// - `LUT_MAB`: `LutMultidimensionalType` — linear a/b/m curves,
    ///   a non-identity M matrix and a 2³ channel-rotation CLUT in
    ///   both directions and both intents.
    /// - `LUT_A2B0_ONLY`: a single mft2 A2B0 tag (×0.5) — no other
    ///   intent or direction tables, for the missing-tag rejection.
    const LUT_INTENT_PAIR: &[u8] = include_bytes!("../tests/fixture/lut_intent_pair.icc");
    const LUT_MFT1: &[u8] = include_bytes!("../tests/fixture/lut_mft1.icc");
    const LUT_MFT2: &[u8] = include_bytes!("../tests/fixture/lut_mft2.icc");
    const LUT_MAB: &[u8] = include_bytes!("../tests/fixture/lut_mab.icc");
    const LUT_A2B0_ONLY: &[u8] = include_bytes!("../tests/fixture/lut_a2b0_only.icc");

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Quantizes a normalized `[0, 1]` CLUT node value into a
    /// `Store8` sample — `round(v · 255)`.
    // `v` is clamped into `[0, 1]`, so `* 255` lands in `[0, 255]`
    // and `.round()` makes it integral — the cast is lossless.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn lut_u8(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Quantizes a normalized `[0, 1]` CLUT node value into a
    /// `Store16` sample — `round(v · 65535)`.
    // Same reasoning as `lut_u8`: the value is integral in
    // `[0, 65535]` after `.round()`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn lut_u16(v: f32) -> u16 {
        (v.clamp(0.0, 1.0) * 65535.0).round() as u16
    }

    /// Samples `map` over the 2×2×2 trilinear CLUT grid — node index
    /// `x·4 + y·2 + z` (`x`/`y`/`z` = R/G/B axes, the engine `Cube`
    /// ordering), three channels interleaved.
    fn clut2(map: impl Fn(f32, f32, f32) -> [f32; 3]) -> Vec<f32> {
        let mut out = Vec::with_capacity(8 * 3);
        for x in 0..2_u8 {
            for y in 0..2_u8 {
                for z in 0..2_u8 {
                    out.extend_from_slice(&map(f32::from(x), f32::from(y), f32::from(z)));
                }
            }
        }
        out
    }

    /// An `LutDataType` (mft1 `Lut8` / mft2 `Lut16`) 3×3 LUT tag —
    /// linear input/output tables and a 2³ CLUT driven by `map`.
    /// mft1 input/output tables are the format-fixed 256 entries per
    /// channel; mft2 uses the minimal 2-entry linear ramps.
    fn mft_lut(map: impl Fn(f32, f32, f32) -> [f32; 3], lut_type: LutType) -> LutWarehouse {
        let (input_table, clut_table, output_table, entries) = match lut_type {
            LutType::Lut8 => (
                LutStore::Store8((0..=255_u8).collect::<Vec<u8>>().repeat(3)),
                LutStore::Store8(clut2(map).iter().map(|v| lut_u8(*v)).collect()),
                LutStore::Store8((0..=255_u8).collect::<Vec<u8>>().repeat(3)),
                256_u16,
            ),
            _ => (
                LutStore::Store16(vec![0, u16::MAX, 0, u16::MAX, 0, u16::MAX]),
                LutStore::Store16(clut2(map).iter().map(|v| lut_u16(*v)).collect()),
                LutStore::Store16(vec![0, u16::MAX, 0, u16::MAX, 0, u16::MAX]),
                2_u16,
            ),
        };
        LutWarehouse::Lut(LutDataType {
            num_input_channels: 3,
            num_output_channels: 3,
            num_clut_grid_points: 2,
            matrix: Matrix3d::IDENTITY,
            num_input_table_entries: entries,
            num_output_table_entries: entries,
            input_table,
            clut_table,
            output_table,
            lut_type,
        })
    }

    /// An `LutMultidimensionalType` tag — mAB when installed under an
    /// A2B tag, mBA under a B2A tag. Linear a/b/m curves (b curves are
    /// mandatory — moxcms rejects an mAB without them), `matrix` in
    /// the M stage slot and a 2³ CLUT driven by `map` behind linear
    /// a curves.
    fn mab_lut(map: impl Fn(f32, f32, f32) -> [f32; 3], matrix: Matrix3d) -> LutWarehouse {
        // An empty `Lut` tone curve is the engine's linear curve.
        let linear = || ToneReprCurve::Lut(Vec::new());
        let mut grid_points = [0_u8; 16];
        grid_points[..3].copy_from_slice(&[2, 2, 2]);
        LutWarehouse::Multidimensional(LutMultidimensionalType {
            num_input_channels: 3,
            num_output_channels: 3,
            grid_points,
            clut: Some(LutStore::Store16(
                clut2(map).iter().map(|v| lut_u16(*v)).collect(),
            )),
            a_curves: vec![linear(), linear(), linear()],
            b_curves: vec![linear(), linear(), linear()],
            m_curves: vec![linear(), linear(), linear()],
            matrix,
            bias: Vector3d::default(),
        })
    }

    /// Encodes a synthetic RGB↔XYZ (PCS) ICC profile carrying the
    /// given per-intent LUT tags — `None` leaves the tag absent.
    fn lut_rgb_profile(
        description: &str,
        a2b0: Option<LutWarehouse>,
        a2b1: Option<LutWarehouse>,
        b2a0: Option<LutWarehouse>,
        b2a1: Option<LutWarehouse>,
    ) -> Vec<u8> {
        let mut profile = ColorProfile::default();
        profile.profile_class = ProfileClass::DisplayDevice;
        profile.color_space = DataColorSpace::Rgb;
        profile.pcs = DataColorSpace::Xyz;
        profile.rendering_intent = RenderingIntent::Perceptual;
        let white = white_point_d50().to_xyzd();
        profile.white_point = white;
        profile.media_white_point = Some(white);
        profile.lut_a_to_b_perceptual = a2b0;
        profile.lut_a_to_b_colorimetric = a2b1;
        profile.lut_b_to_a_perceptual = b2a0;
        profile.lut_b_to_a_colorimetric = b2a1;
        profile.description = Some(ProfileText::Localizable(vec![LocalizableString::new(
            "en".to_string(),
            "US".to_string(),
            description.to_string(),
        )]));
        fix_mft1_tags(&profile.encode().unwrap())
    }

    /// Rewrites a freshly-encoded profile so every `mft1`/`lut8Type`
    /// tag is spec-valid: moxcms 0.9.1 `write_lut_entry` emits the
    /// `lut16Type` 2×u16 entry-count fields for `Lut8` too — four
    /// bytes the `lut8Type` header does not carry (input tables begin
    /// at offset 48). The engine's own spec-correct reader then
    /// mis-parses the encoder's output, shifting every table four
    /// bytes early. This rewrites each `mft1` body as
    /// `body[..48] ++ body[52..]` and re-lays-out the tag directory —
    /// a fixture-side workaround pinned to the observed encoder
    /// layout, confined to the generator so committed fixture bytes
    /// are what a correct CMM would have written.
    fn fix_mft1_tags(encoded: &[u8]) -> Vec<u8> {
        // ICC layout: 128-byte header, u32 tag count, 12-byte
        // directory records (signature, offset, size), then aligned
        // tag bodies. Every tag body begins with its type signature.
        let tag_count = u32::from_be_bytes(encoded[128..132].try_into().unwrap()) as usize;
        let dir_end = 132 + tag_count * 12;
        let mut bodies = Vec::with_capacity(tag_count);
        for i in 0..tag_count {
            let e = 132 + i * 12;
            let off = u32::from_be_bytes(encoded[e + 4..e + 8].try_into().unwrap()) as usize;
            let size = u32::from_be_bytes(encoded[e + 8..e + 12].try_into().unwrap()) as usize;
            let body = &encoded[off..off + size];
            let fixed = if body[..4] == *b"mft1" {
                assert!(body.len() > 52, "mft1 tag too small for the 52-byte header");
                [&body[..48], &body[52..]].concat()
            } else {
                body.to_vec()
            };
            bodies.push(fixed);
        }
        let mut out = encoded[..dir_end].to_vec();
        for (i, body) in bodies.iter().enumerate() {
            while !out.len().is_multiple_of(4) {
                out.push(0);
            }
            let e = 132 + i * 12;
            let tag_offset = u32::try_from(out.len()).unwrap();
            let tag_size = u32::try_from(body.len()).unwrap();
            out[e + 4..e + 8].copy_from_slice(&tag_offset.to_be_bytes());
            out[e + 8..e + 12].copy_from_slice(&tag_size.to_be_bytes());
            out.extend_from_slice(body);
        }
        // Header bytes 0..4 hold the total profile size.
        let total = u32::try_from(out.len()).unwrap();
        out[..4].copy_from_slice(&total.to_be_bytes());
        out
    }

    // CLUT node maps — each maps one grid coordinate to its stored
    // output triple. All are linear, so trilinear interpolation
    // reproduces them (up to store quantization).
    /// ×0.5 darkening — the intent pair's Perceptual tables.
    fn halve3(x: f32, y: f32, z: f32) -> [f32; 3] {
        [x * 0.5, y * 0.5, z * 0.5]
    }
    /// ×0.75 darkening — the mft2 fixture's B2A tables.
    fn scale34_3(x: f32, y: f32, z: f32) -> [f32; 3] {
        [x * 0.75, y * 0.75, z * 0.75]
    }
    /// Identity — the intent pair's colorimetric tables.
    fn ident3(x: f32, y: f32, z: f32) -> [f32; 3] {
        [x, y, z]
    }
    /// R↔B channel swap — the mft1 fixture's tables.
    fn swap_rb3(x: f32, y: f32, z: f32) -> [f32; 3] {
        [z, y, x]
    }
    /// (x, y, z) → (z, x, y) channel rotation — the mAB fixture's CLUT.
    fn rotate3(x: f32, y: f32, z: f32) -> [f32; 3] {
        [z, x, y]
    }

    /// A non-identity M-stage matrix for the mAB fixture — exercises
    /// the matrix stage (engine skips it only for IDENTITY + zero
    /// bias + all-linear m curves).
    fn mab_matrix() -> Matrix3d {
        Matrix3d {
            v: [[1.0, 0.0, 0.0], [0.0, 0.9, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// One-shot fixture (re)generator — rewrites the committed `.icc`
    /// files under `tests/fixture/`: the pinned engine's built-in
    /// profiles plus the synthetic LUT-tag profiles from the builders
    /// above. Run explicitly:
    /// `cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored`.
    #[test]
    #[ignore = "one-shot fixture regeneration — run explicitly when the pinned engine changes"]
    fn generate_fixtures() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture");
        std::fs::create_dir_all(dir).unwrap();
        let halve = |t| mft_lut(t, LutType::Lut16);
        let fixtures: [(&str, Vec<u8>); 8] = [
            ("srgb.icc", ColorProfile::new_srgb().encode().unwrap()),
            (
                "display_p3.icc",
                ColorProfile::new_display_p3().encode().unwrap(),
            ),
            (
                "gray_gamma22.icc",
                ColorProfile::new_gray_with_gamma(2.2).encode().unwrap(),
            ),
            (
                "lut_intent_pair.icc",
                lut_rgb_profile(
                    "icc-lut-intent-pair",
                    Some(halve(halve3)),
                    Some(mft_lut(ident3, LutType::Lut16)),
                    Some(halve(halve3)),
                    Some(mft_lut(ident3, LutType::Lut16)),
                ),
            ),
            (
                "lut_mft1.icc",
                lut_rgb_profile(
                    "icc-lut-mft1",
                    Some(mft_lut(swap_rb3, LutType::Lut8)),
                    Some(mft_lut(swap_rb3, LutType::Lut8)),
                    Some(mft_lut(swap_rb3, LutType::Lut8)),
                    Some(mft_lut(swap_rb3, LutType::Lut8)),
                ),
            ),
            (
                "lut_mft2.icc",
                lut_rgb_profile(
                    "icc-lut-mft2",
                    Some(halve(halve3)),
                    Some(halve(halve3)),
                    Some(mft_lut(scale34_3, LutType::Lut16)),
                    Some(mft_lut(scale34_3, LutType::Lut16)),
                ),
            ),
            (
                "lut_mab.icc",
                lut_rgb_profile(
                    "icc-lut-mab",
                    Some(mab_lut(rotate3, mab_matrix())),
                    Some(mab_lut(rotate3, mab_matrix())),
                    Some(mab_lut(rotate3, mab_matrix())),
                    Some(mab_lut(rotate3, mab_matrix())),
                ),
            ),
            (
                "lut_a2b0_only.icc",
                lut_rgb_profile("icc-lut-a2b0-only", Some(halve(halve3)), None, None, None),
            ),
        ];
        for (name, bytes) in fixtures {
            // The generator validates its own output: every committed
            // fixture must parse back, and each synthetic LUT profile
            // must be an RGB device profile — a fixture that
            // `new_from_slice` rejects would fail every test that
            // consumes it.
            let parsed = ColorProfile::new_from_slice(&bytes).unwrap();
            if name.starts_with("lut_") {
                assert_eq!(parsed.color_space, DataColorSpace::Rgb, "{name}");
            }
            std::fs::write(format!("{dir}/{name}"), &bytes).unwrap();
        }
    }

    /// Builds an `Rgba8` frame of one row holding `pixels`, tagged
    /// `color`.
    fn rgba8_frame(width: u32, height: u32, pixels: &[[u8; 4]], color: ColorInfo) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::Rgba8, width, height, pts()).unwrap();
        frame
            .plane_mut(0)
            .unwrap()
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(pixels.iter())
            .for_each(|(dst, px)| *dst = *px);
        frame.set_color(color);
        frame
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
            .for_each(|(dst, px)| *dst = f32_px_bytes(*px));
        frame.set_color(color);
        frame
    }

    /// Reads the `x`-th pixel of an `RgbaF32` frame's row `y` as four
    /// `f32` channels.
    fn f32_px(frame: &VideoFrame, x: usize, y: u32) -> [f32; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let px: &[u8; 16] = row[16 * x..16 * x + 16].try_into().unwrap();
        std::array::from_fn(|i| {
            f32::from_le_bytes([px[4 * i], px[4 * i + 1], px[4 * i + 2], px[4 * i + 3]])
        })
    }

    /// The `ColorInfo` every `IccTransform` output records.
    fn icc_tag() -> ColorInfo {
        ColorInfo {
            primaries: ColorPrimaries::Unspecified,
            transfer: TransferCharacteristics::Unspecified,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: None,
        }
    }

    /// The raw engine executor for the same profile pair/options —
    /// the bit-exact oracle every golden/equivalence assertion
    /// compares against (moxcms 0.9.1,
    /// `default-features = false` + `lut` + `extended_range`, contract
    /// `TransformOptions` — deterministic per build configuration).
    fn raw_executor(
        src_profile: &[u8],
        dst_profile: &[u8],
        intent: RenderingIntent,
    ) -> Arc<TransformF32Executor> {
        let src = ColorProfile::new_from_slice(src_profile).unwrap();
        let dst = ColorProfile::new_from_slice(dst_profile).unwrap();
        src.create_transform_f32(
            Layout::Rgba,
            &dst,
            Layout::Rgba,
            TransformOptions {
                rendering_intent: intent,
                allow_use_cicp_transfer: true,
                prefer_fixed_point: false,
                allow_extended_range_rgb_xyz: true,
                ..TransformOptions::default()
            },
        )
        .unwrap()
    }

    /// ctor: unparsable byte strings reject with `"icc profile"` on
    /// either argument — garbage, empty and truncated inputs.
    #[test]
    fn icc_transform_ctor_rejects_unparsable_profiles() {
        for bad in [
            &b"not an icc profile"[..],
            &[][..],
            &SRGB[..128],            // header only, tag table cut off
            &SRGB[..SRGB.len() - 4], // truncated tail
        ] {
            assert_eq!(
                IccTransform::new(bad, SRGB, IccIntent::Perceptual).map(|_| ()),
                Err(Error::InvalidValue("icc profile"))
            );
            assert_eq!(
                IccTransform::new(SRGB, bad, IccIntent::Perceptual).map(|_| ()),
                Err(Error::InvalidValue("icc profile"))
            );
        }
    }

    /// ctor: a parseable profile pair that cannot build an RGBA→RGBA
    /// transform rejects with `"icc transform"` — a Gray color-space
    /// profile is incompatible with the baked `Layout::Rgba`.
    #[test]
    fn icc_transform_ctor_rejects_untransformable_pair() {
        assert_eq!(
            IccTransform::new(GRAY_GAMMA22, SRGB, IccIntent::Perceptual).map(|_| ()),
            Err(Error::InvalidValue("icc transform"))
        );
        assert_eq!(
            IccTransform::new(SRGB, GRAY_GAMMA22, IccIntent::Perceptual).map(|_| ()),
            Err(Error::InvalidValue("icc transform"))
        );
    }

    /// ctor: a parseable profile whose header color space is not RGB
    /// rejects with `"icc transform"` on either argument — moxcms also
    /// accepts `Layout::Rgba` for CMYK/Lab/multi-channel spaces, where
    /// channel 3 would cease to be alpha passthrough (consumed as the
    /// K ink for CMYK), so the RGB-only gate must fire before the
    /// executor exists. Bytes 16..20 of the profile are the header
    /// data color space field; the `b"CMYK"` patch keeps the rest of
    /// the sRGB fixture intact.
    #[test]
    fn icc_transform_ctor_rejects_non_rgb_color_space() {
        let mut cmyk = SRGB.to_vec();
        cmyk[16..20].copy_from_slice(b"CMYK");
        // The patched header still parses — the rejection is the
        // color-space gate, not the parser.
        assert_eq!(
            ColorProfile::new_from_slice(&cmyk).map(|p| p.color_space),
            Ok(DataColorSpace::Cmyk)
        );
        assert_eq!(
            IccTransform::new(&cmyk, SRGB, IccIntent::Perceptual).map(|_| ()),
            Err(Error::InvalidValue("icc transform"))
        );
        assert_eq!(
            IccTransform::new(SRGB, &cmyk, IccIntent::Perceptual).map(|_| ()),
            Err(Error::InvalidValue("icc transform"))
        );
    }

    /// Golden + oracle: sRGB→sRGB `Rgba8`→`Rgba8`. The engine's
    /// roundtrip is near-identity (float matrix noise of ≲1e-7), so
    /// quantized output is exactly the input codes for these pixels —
    /// asserted literally AND against a direct executor call.
    // Literal goldens are pinned to the x86_64 non-FMA baseline (the
    // CI `icc-verify` lane): moxcms `mlaf` selects fused FMA on
    // aarch64 (NEON baseline) and `target_feature = "fma"`
    // x86/x86_64, so executor bit output can diverge in the last ulp
    // across target-features/architectures.
    #[cfg(all(target_arch = "x86_64", not(target_feature = "fma")))]
    #[test]
    fn icc_transform_srgb_identity_rgba8_bit_exact() {
        let mut kernel = IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap();
        let src = rgba8_frame(
            4,
            1,
            &[
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [128, 200, 50, 255],
            ],
            ColorInfo::default(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 4, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        let data = dst.plane(0).unwrap().data();
        assert_eq!(
            data.as_chunks::<4>().0,
            [
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [128, 200, 50, 255]
            ]
        );
    }

    /// Oracle equivalence: the kernel's `Rgba8`→`Rgba8` output equals
    /// `quantize8(executor(normalize(px)))` computed by a raw engine
    /// executor on the same profiles/options — bit-exact per the
    /// equivalence contract (moxcms 0.9.1, `lut`+`extended_range`,
    /// scalar path).
    #[test]
    fn icc_transform_rgba8_output_matches_engine_bit_exact() {
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let executor = raw_executor(SRGB, DISPLAY_P3, RenderingIntent::Perceptual);
        let pixels: &[[u8; 4]] = &[
            [255, 0, 0, 255],
            [0, 255, 0, 200],
            [0, 0, 255, 128],
            [128, 200, 50, 64],
            [10, 20, 30, 0],
        ];
        let src = rgba8_frame(5, 1, pixels, ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 5, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        for (x, px) in pixels.iter().enumerate() {
            let mut expected = [0.0_f32; 4];
            executor
                .transform(&rgba8_signal(*px), &mut expected)
                .unwrap();
            let expected: [u8; 4] = std::array::from_fn(|i| quantize8(expected[i]));
            assert_eq!(
                &dst.plane(0).unwrap().data()[4 * x..4 * x + 4],
                &expected,
                "pixel {x} {px:?}"
            );
        }
    }

    /// Golden: sRGB→P3 `RgbaF32`→`RgbaF32` output bit patterns, derived
    /// from moxcms 0.9.1 (`default-features = false` + `lut` +
    /// `extended_range`, contract `TransformOptions`, scalar path).
    /// `[1.5, 2.0, -0.5]` keeps its extended range — below-`0` and
    /// above-`1` channels survive into `RgbaF32`.
    // Literal goldens are pinned to the x86_64 non-FMA baseline (the
    // CI `icc-verify` lane): moxcms `mlaf` selects fused FMA on
    // aarch64 (NEON baseline) and `target_feature = "fma"`
    // x86/x86_64, so executor bit output can diverge in the last ulp
    // across target-features/architectures.
    #[cfg(all(target_arch = "x86_64", not(target_feature = "fma")))]
    #[test]
    fn icc_transform_rgba_f32_bit_exact_golden() {
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        // `f32::to_bits` of the engine output, pinned-moxcms derived.
        let cases: &[([f32; 4], [u32; 4])] = &[
            (
                [1.0, 0.0, 0.0, 1.0],
                [1_063_967_789, 1_045_232_921, 1_041_100_570, 1_065_353_216],
            ),
            (
                [0.0, 1.0, 0.0, 1.0],
                [1_055_573_803, 1_065_106_107, 1_050_201_425, 1_065_353_216],
            ),
            (
                [0.0, 0.0, 1.0, 1.0],
                [963_750_034, 3_107_035_727, 1_064_674_539, 1_065_353_216],
            ),
            (
                [128.0 / 255.0, 200.0 / 255.0, 50.0 / 255.0, 1.0],
                [1_058_069_296, 1_061_611_902, 1_050_344_615, 1_065_353_216],
            ),
            (
                [1.5, 2.0, -0.5, 0.5],
                [1_070_425_554, 1_073_624_957, 1_059_312_130, 1_056_964_608],
            ),
            (
                [0.25, 0.5, 0.75, 128.0 / 255.0],
                [1_050_692_016, 1_056_766_605, 1_060_825_579, 1_056_997_505],
            ),
        ];
        let src = rgba_f32_frame(
            u32::try_from(cases.len()).unwrap(),
            1,
            &cases.iter().map(|c| c.0).collect::<Vec<_>>(),
            ColorInfo::default(),
        );
        let mut dst = VideoFrame::new_owned(
            PixelFormat::RgbaF32,
            u32::try_from(cases.len()).unwrap(),
            1,
            pts(),
        )
        .unwrap();
        kernel.process(&src, &mut dst).unwrap();
        for (x, (_, expected_bits)) in cases.iter().enumerate() {
            let out = f32_px(&dst, x, 0);
            assert_eq!(out.map(f32::to_bits), *expected_bits, "pixel {x}");
        }
        // Extended range explicitly: above-`1` channels survive into
        // `RgbaF32` (the P3 remapping does not clamp).
        let out = f32_px(&dst, 4, 0);
        assert!(out[0] > 1.0 && out[1] > 1.0);

        // Below-`0` channels also survive: the sRGB→sRGB executor
        // roundtrip carries `-0.5` through verbatim.
        let mut identity = IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap();
        let src = rgba_f32_frame(1, 1, &[[1.5, 2.0, -0.5, 0.5]], ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        identity.process(&src, &mut dst).unwrap();
        let out = f32_px(&dst, 0, 0);
        assert!(out[0] > 1.0 && out[1] > 1.0 && out[2] < 0.0);
        assert_eq!(out[2], -0.5);
    }

    /// Extended range survives `RgbaF32` output on every build
    /// configuration — the behavioral twin of the cfg-gated literal
    /// golden above: below-`0`/above-`1` channels are not clamped
    /// (loose bounds — `mlaf`/FMA-level noise may move the last ulp
    /// across target-features/architectures), and alpha passes
    /// through untransformed.
    #[test]
    fn icc_transform_rgba_f32_keeps_extended_range() {
        // sRGB→P3 remap: the [1.5, 2.0, -0.5] input keeps its
        // above-`1` channels; alpha stays verbatim.
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let src = rgba_f32_frame(1, 1, &[[1.5, 2.0, -0.5, 0.5]], ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        let out = f32_px(&dst, 0, 0);
        assert!(out[0] > 1.0 && out[1] > 1.0);
        assert_eq!(out[3], 0.5);

        // sRGB→sRGB roundtrip: the `-0.5` channel survives below `0`
        // and near-verbatim (tolerance covers cross-target-feature
        // last-ulp divergence — the exact-bit assert lives in the
        // gated golden).
        let mut identity = IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap();
        identity.process(&src, &mut dst).unwrap();
        let out = f32_px(&dst, 0, 0);
        assert!(out[0] > 1.0 && out[1] > 1.0 && out[2] < 0.0);
        assert!((out[2] + 0.5).abs() < 1e-6);
        assert_eq!(out[3], 0.5);
    }

    /// `Rgba8`→`RgbaF32` and `RgbaF32`→`Rgba8` endpoint mixes: the
    /// normalize/encode shims chain exactly around the same executor.
    #[test]
    fn icc_transform_mixed_endpoints() {
        let executor = raw_executor(SRGB, DISPLAY_P3, RenderingIntent::Perceptual);

        // Rgba8 → RgbaF32: output f32 bits equal the executor's.
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let src8 = rgba8_frame(1, 1, &[[128, 200, 50, 255]], ColorInfo::default());
        let mut dst_f32 = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        kernel.process(&src8, &mut dst_f32).unwrap();
        let mut expected = [0.0_f32; 4];
        executor
            .transform(&rgba8_signal([128, 200, 50, 255]), &mut expected)
            .unwrap();
        assert_eq!(
            f32_px(&dst_f32, 0, 0).map(f32::to_bits),
            expected.map(f32::to_bits)
        );

        // RgbaF32 → Rgba8: extended-range input quantizes/clamps on
        // output — the [1.5, 2.0, -0.5] executor result saturates.
        let mut kernel = IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap();
        let src_f32 = rgba_f32_frame(1, 1, &[[1.5, 2.0, -0.5, 1.0]], ColorInfo::default());
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        kernel.process(&src_f32, &mut dst8).unwrap();
        let mut out = [0.0_f32; 4];
        raw_executor(SRGB, SRGB, RenderingIntent::Perceptual)
            .transform(&[1.5, 2.0, -0.5, 1.0], &mut out)
            .unwrap();
        let expected: [u8; 4] = std::array::from_fn(|i| quantize8(out[i]));
        assert_eq!(&dst8.plane(0).unwrap().data()[..4], &expected);
        assert_eq!(&expected[..3], &[255, 255, 0]);
    }

    /// Alpha is never color-transformed: the engine's RGBA passthrough
    /// carries channel 3 through — `0.5` stays `0.5` bit-exact, and a
    /// quantized `Rgba8` alpha roundtrips its code.
    #[test]
    fn icc_transform_alpha_passes_through() {
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.2, 0.4, 0.6, 0.5], [1.0, 0.0, 0.0, 0.25]],
            ColorInfo::default(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        assert_eq!(f32_px(&dst, 0, 0)[3], 0.5);
        assert_eq!(f32_px(&dst, 1, 0)[3], 0.25);

        let src8 = rgba8_frame(1, 1, &[[64, 128, 200, 77]], ColorInfo::default());
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        kernel.process(&src8, &mut dst8).unwrap();
        assert_eq!(dst8.plane(0).unwrap().data()[3], 77);
    }

    /// `dst.color` records the honest unspecified description —
    /// `{Unspecified, Unspecified, Identity, Full, hdr: None}` — and
    /// `pts`/`duration` propagate; a src `hdr` tag is not carried.
    #[test]
    fn icc_transform_records_unspecified_color_and_timestamps() {
        let mut src = rgba8_frame(1, 1, &[[10, 20, 30, 255]], ColorInfo::default());
        src.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Pq,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: Some(HdrMetadata {
                mastering_display: None,
                content_light_level: Some(ContentLightLevel {
                    max_cll: 1000,
                    max_fall: 400,
                }),
            }),
        });
        src.set_duration(Some(MediaDuration::new(1000, TimeBase::COMMON_90K)));
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        IccTransform::new(SRGB, SRGB, IccIntent::Perceptual)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(dst.color(), icc_tag());
        assert_eq!(dst.pts(), src.pts());
        assert_eq!(dst.duration(), src.duration());
    }

    /// Intent mapping: both contract intents build and run; on a
    /// matrix-shaper sRGB pair the baked math is intent-independent —
    /// identical bytes is the observable check that the mapping was
    /// accepted and baked deterministically.
    #[test]
    fn icc_transform_intents_build_and_run() {
        let src = rgba8_frame(1, 1, &[[77, 150, 220, 255]], ColorInfo::default());
        let mut outs = Vec::new();
        for intent in [IccIntent::Perceptual, IccIntent::RelativeColorimetric] {
            let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
            IccTransform::new(SRGB, SRGB, intent)
                .unwrap()
                .process(&src, &mut dst)
                .unwrap();
            outs.push(dst.plane(0).unwrap().data().to_vec());
        }
        // `outs[0] == outs[1]` proves both intents were accepted and
        // baked deterministically — NOT that the intent changes the
        // mapping: matrix-shaper sRGB fixtures carry no
        // intent-distinguishing LUT, so identical output bytes are
        // expected here. The intent-distinguishing proof lives on the
        // LUT fixtures: `icc_transform_intent_selects_distinct_a2b_luts`
        // and `icc_transform_intent_selects_distinct_b2a_luts`.
        assert_eq!(outs[0], outs[1]);
    }

    /// Shared rejections: non-target formats, dimension mismatch and a
    /// shared `dst` backing all fail before a single byte is written.
    #[test]
    fn icc_transform_invalid_inputs_are_rejected() {
        let mut kernel = IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap();
        let src = rgba8_frame(1, 1, &[[128, 128, 128, 255]], ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();

        // Planar YUV / RgbaF16 sources.
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_yuv, &mut dst),
            Err(Error::InvalidValue("src format"))
        );
        let src_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_f16, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Non-target destinations (RgbaF16 is not an ICC endpoint).
        let mut dst_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_f16),
            Err(Error::InvalidValue("dst format"))
        );
        let mut dst_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_yuv),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let src_f32 = rgba_f32_frame(1, 1, &[[0.5, 0.5, 0.5, 1.0]], ColorInfo::default());
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_f32, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // A shared `dst` (clone still holding the `Arc` backing) —
        // rejected before any write.
        let mut dst_shared = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let sharing_clone = dst_shared.clone();
        assert!(dst_shared.plane_mut(0).is_none());
        assert_eq!(
            kernel.process(&src, &mut dst_shared),
            Err(Error::InvalidValue("dst plane"))
        );
        // Rejection wrote nothing — pixels, `ColorInfo`, pts intact.
        assert_eq!(dst_shared.plane(0).unwrap().data(), &[0; 4]);
        assert_eq!(dst_shared.color(), ColorInfo::default());
        drop(sharing_clone);
        kernel.process(&src, &mut dst_shared).unwrap();

        assert_eq!(dst.plane(0).unwrap().data(), &[0; 4]);
    }

    /// A `stride > row_bytes` destination: conversion writes rows
    /// only — padding bytes keep their caller-chosen contents.
    // The row asserts compare literal engine output — pinned to the
    // x86_64 non-FMA baseline like every literal golden (moxcms
    // `mlaf` selects fused FMA on aarch64/`fma`-enabled x86).
    #[cfg(all(target_arch = "x86_64", not(target_feature = "fma")))]
    #[test]
    fn icc_transform_padded_dst_stride_is_never_written() {
        // 1×2 `Rgba8` dst: row_bytes 4, padded stride 8 → usable
        // 8 + 4 = 12 bytes — row 0 at 0..4, inter-row padding at
        // 4..8, row 1 at 8..12.
        let src = rgba8_frame(
            1,
            2,
            &[[255, 0, 0, 255], [0, 0, 255, 255]],
            ColorInfo::default(),
        );
        let mut dst = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    vec![0_u8, 0, 0, 0, 0xEE, 0xEE, 0xEE, 0xEE, 0, 0, 0, 0].into_boxed_slice(),
                ),
                stride: 8,
            }],
        )
        .unwrap();
        assert_eq!(dst.plane(0).unwrap().stride(), 8);
        IccTransform::new(SRGB, SRGB, IccIntent::Perceptual)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let data = dst.plane(0).unwrap().data();
        assert_eq!(&data[0..4], &[255, 0, 0, 255]);
        assert_eq!(&data[4..8], &[0xEE; 4]);
        assert_eq!(&data[8..12], &[0, 0, 255, 255]);
    }

    /// A `stride > row_bytes` source: conversion reads the row
    /// region only — the padding bytes never enter the pipeline.
    // The row asserts compare literal engine output — pinned to the
    // x86_64 non-FMA baseline like every literal golden (moxcms
    // `mlaf` selects fused FMA on aarch64/`fma`-enabled x86).
    #[cfg(all(target_arch = "x86_64", not(target_feature = "fma")))]
    #[test]
    fn icc_transform_padded_src_stride_reads_rows_only() {
        // 1×2 `Rgba8` src: row_bytes 4, padded stride 8 → usable
        // 8 + 4 = 12 bytes — row 0 at 0..4, padding at 4..8, row 1
        // at 8..12. 0xEE is not pixel data.
        let mut src = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    vec![255_u8, 0, 0, 255, 0xEE, 0xEE, 0xEE, 0xEE, 0, 0, 255, 255]
                        .into_boxed_slice(),
                ),
                stride: 8,
            }],
        )
        .unwrap();
        assert_eq!(src.plane(0).unwrap().stride(), 8);
        src.set_color(ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 2, pts()).unwrap();
        IccTransform::new(SRGB, SRGB, IccIntent::Perceptual)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let data = dst.plane(0).unwrap().data();
        assert_eq!(&data[0..4], &[255, 0, 0, 255]);
        assert_eq!(&data[4..8], &[0, 0, 255, 255]);
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_icc_is_repeatable_and_deterministic() {
        let src = rgba8_frame(1, 1, &[[128, 200, 50, 255]], ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let mut first = [0_u8; 4];
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
    fn rt_process_icc_never_mutates_source() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.25, 0.5, 0.75, 1.0], [1.5, 2.0, 2.5, 0.5]],
            ColorInfo::default(),
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }

    /// Runs `kernel` over a one-row `Rgba8` frame holding `pixels`
    /// and returns the destination row bytes.
    fn run_rgba8(kernel: &mut IccTransform, pixels: &[[u8; 4]]) -> Vec<u8> {
        let width = u32::try_from(pixels.len()).unwrap();
        let src = rgba8_frame(width, 1, pixels, ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, width, 1, pts()).unwrap();
        kernel.process(&src, &mut dst).unwrap();
        dst.plane(0).unwrap().data().to_vec()
    }

    /// The bit-exact oracle for `run_rgba8`: the raw engine executor
    /// (same profiles/options/`intent`) followed by the same
    /// `quantize8` the kernel applies.
    fn oracle_rgba8(
        src_profile: &[u8],
        dst_profile: &[u8],
        intent: RenderingIntent,
        pixels: &[[u8; 4]],
    ) -> Vec<u8> {
        let executor = raw_executor(src_profile, dst_profile, intent);
        let mut out = Vec::with_capacity(pixels.len() * 4);
        for px in pixels {
            let mut dst = [0.0_f32; 4];
            executor.transform(&rgba8_signal(*px), &mut dst).unwrap();
            out.extend(dst.map(quantize8));
        }
        out
    }

    /// A mid-gamut pixel set where every LUT fixture's mapping is
    /// byte-observable (nonzero, non-saturated channels).
    const LUT_PIXELS: &[[u8; 4]] = &[
        [200, 100, 50, 255],
        [128, 200, 50, 255],
        [60, 90, 200, 128],
        [17, 34, 51, 64],
    ];

    /// Structure: every synthetic LUT fixture parses back as an RGB
    /// profile carrying the intended `LutWarehouse` tags — mft1
    /// `Lut8`/`Store8` with the format-fixed 256-entry in/out tables,
    /// mft2 `Lut16`/`Store16` with 2-entry tables, mAB as
    /// `Multidimensional` with a 2³ `Store16` CLUT — in the right
    /// intent/direction slots.
    #[test]
    fn lut_fixtures_parse_with_intended_tag_shapes() {
        // `lut_mft1`: all four tags are mft1 `Lut8` warehouses with
        // 256-entry `Store8` in/out tables and a 2³ `Store8` CLUT.
        let profile = ColorProfile::new_from_slice(LUT_MFT1).unwrap();
        assert_eq!(profile.color_space, DataColorSpace::Rgb);
        for wh in [
            &profile.lut_a_to_b_perceptual,
            &profile.lut_a_to_b_colorimetric,
            &profile.lut_b_to_a_perceptual,
            &profile.lut_b_to_a_colorimetric,
        ] {
            let Some(LutWarehouse::Lut(lut)) = wh else {
                panic!("mft1 tag is not a `Lut` warehouse");
            };
            assert_eq!(lut.lut_type, LutType::Lut8);
            assert_eq!(lut.num_input_table_entries, 256);
            assert_eq!(lut.num_clut_grid_points, 2);
            assert!(matches!(lut.input_table, LutStore::Store8(_)));
            assert!(matches!(&lut.clut_table, LutStore::Store8(v) if v.len() == 24));
            assert!(matches!(lut.output_table, LutStore::Store8(_)));
        }

        // `lut_mft2`: same shape, `Lut16`/`Store16`, 2-entry tables.
        let profile = ColorProfile::new_from_slice(LUT_MFT2).unwrap();
        for wh in [
            &profile.lut_a_to_b_perceptual,
            &profile.lut_a_to_b_colorimetric,
            &profile.lut_b_to_a_perceptual,
            &profile.lut_b_to_a_colorimetric,
        ] {
            let Some(LutWarehouse::Lut(lut)) = wh else {
                panic!("mft2 tag is not a `Lut` warehouse");
            };
            assert_eq!(lut.lut_type, LutType::Lut16);
            assert_eq!(lut.num_input_table_entries, 2);
            assert_eq!(lut.num_output_table_entries, 2);
            assert!(matches!(lut.input_table, LutStore::Store16(_)));
            assert!(matches!(&lut.clut_table, LutStore::Store16(v) if v.len() == 24));
            assert!(matches!(lut.output_table, LutStore::Store16(_)));
        }

        // `lut_mab`: all four tags are `Multidimensional` warehouses —
        // mAB under A2B, mBA under B2A — with a 2³ `Store16` CLUT and
        // three b/m curves (a curves are optional in the format).
        let profile = ColorProfile::new_from_slice(LUT_MAB).unwrap();
        for wh in [
            &profile.lut_a_to_b_perceptual,
            &profile.lut_a_to_b_colorimetric,
            &profile.lut_b_to_a_perceptual,
            &profile.lut_b_to_a_colorimetric,
        ] {
            let Some(LutWarehouse::Multidimensional(lut)) = wh else {
                panic!("mAB tag is not a `Multidimensional` warehouse");
            };
            assert_eq!(&lut.grid_points[..3], &[2, 2, 2]);
            assert!(matches!(&lut.clut, Some(LutStore::Store16(v)) if v.len() == 24));
            assert_eq!(lut.b_curves.len(), 3);
            assert_eq!(lut.m_curves.len(), 3);
        }

        // `lut_intent_pair`: the two A2B tags differ (×0.5 vs
        // identity), as do the two B2A tags — this is what makes the
        // intent observably change the mapping.
        let profile = ColorProfile::new_from_slice(LUT_INTENT_PAIR).unwrap();
        let (Some(LutWarehouse::Lut(a2b0)), Some(LutWarehouse::Lut(a2b1))) = (
            &profile.lut_a_to_b_perceptual,
            &profile.lut_a_to_b_colorimetric,
        ) else {
            panic!("intent-pair A2B tags are not `Lut` warehouses");
        };
        assert_ne!(a2b0, a2b1);
        let (Some(LutWarehouse::Lut(b2a0)), Some(LutWarehouse::Lut(b2a1))) = (
            &profile.lut_b_to_a_perceptual,
            &profile.lut_b_to_a_colorimetric,
        ) else {
            panic!("intent-pair B2A tags are not `Lut` warehouses");
        };
        assert_ne!(b2a0, b2a1);

        // `lut_a2b0_only`: only the A2B0/Perceptual tag is present.
        let profile = ColorProfile::new_from_slice(LUT_A2B0_ONLY).unwrap();
        assert!(matches!(
            profile.lut_a_to_b_perceptual,
            Some(LutWarehouse::Lut(_))
        ));
        assert!(profile.lut_a_to_b_colorimetric.is_none());
        assert!(profile.lut_b_to_a_perceptual.is_none());
        assert!(profile.lut_b_to_a_colorimetric.is_none());
    }

    /// Intent selection (A2B direction): `lut_intent_pair → sRGB`
    /// picks A2B0 (×0.5) for Perceptual and A2B1 (identity) for
    /// `RelativeColorimetric` — the baked mapping is observably
    /// different, each side bit-exact against the raw executor baked
    /// with the same intent.
    #[test]
    fn icc_transform_intent_selects_distinct_a2b_luts() {
        let perceptual = run_rgba8(
            &mut IccTransform::new(LUT_INTENT_PAIR, SRGB, IccIntent::Perceptual).unwrap(),
            LUT_PIXELS,
        );
        let colorimetric = run_rgba8(
            &mut IccTransform::new(LUT_INTENT_PAIR, SRGB, IccIntent::RelativeColorimetric).unwrap(),
            LUT_PIXELS,
        );
        // ×0.5 PCS vs identity PCS: every pixel comes out
        // channel-wise darker (or equal where a channel clamps)
        // under Perceptual, and observably different bytes overall.
        assert_ne!(perceptual, colorimetric);
        for (p, c) in perceptual.chunks(4).zip(colorimetric.chunks(4)) {
            assert!(
                p[0] <= c[0] && p[1] <= c[1] && p[2] <= c[2],
                "{p:?} vs {c:?}"
            );
            assert!(p[..3] != c[..3], "{p:?} vs {c:?}");
        }
        assert_eq!(
            perceptual,
            oracle_rgba8(
                LUT_INTENT_PAIR,
                SRGB,
                RenderingIntent::Perceptual,
                LUT_PIXELS
            )
        );
        assert_eq!(
            colorimetric,
            oracle_rgba8(
                LUT_INTENT_PAIR,
                SRGB,
                RenderingIntent::RelativeColorimetric,
                LUT_PIXELS
            )
        );
    }

    /// Intent selection (B2A direction): `sRGB → lut_intent_pair`
    /// picks B2A0 (×0.5) for Perceptual and B2A1 (identity) for
    /// `RelativeColorimetric` — the mirror of the A2B case.
    #[test]
    fn icc_transform_intent_selects_distinct_b2a_luts() {
        let perceptual = run_rgba8(
            &mut IccTransform::new(SRGB, LUT_INTENT_PAIR, IccIntent::Perceptual).unwrap(),
            LUT_PIXELS,
        );
        let colorimetric = run_rgba8(
            &mut IccTransform::new(SRGB, LUT_INTENT_PAIR, IccIntent::RelativeColorimetric).unwrap(),
            LUT_PIXELS,
        );
        assert_ne!(perceptual, colorimetric);
        for (p, c) in perceptual.chunks(4).zip(colorimetric.chunks(4)) {
            assert!(p[0] < c[0] && p[1] < c[1] && p[2] < c[2], "{p:?} vs {c:?}");
        }
        assert_eq!(
            perceptual,
            oracle_rgba8(
                SRGB,
                LUT_INTENT_PAIR,
                RenderingIntent::Perceptual,
                LUT_PIXELS
            )
        );
        assert_eq!(
            colorimetric,
            oracle_rgba8(
                SRGB,
                LUT_INTENT_PAIR,
                RenderingIntent::RelativeColorimetric,
                LUT_PIXELS
            )
        );
    }

    /// mft1 path: the `Lut8` R↔B swap runs in both directions — each
    /// output is bit-exact against the raw executor, and the swap is
    /// observable two ways: pure red exits blue-dominant in each
    /// direction, and the self-pipeline `lut_mft1 → lut_mft1`
    /// (swap ∘ swap = identity) roundtrips the input within
    /// quantization.
    #[test]
    fn icc_transform_mft1_lut_path_swaps_channels() {
        // Forward (A2B mft1 → sRGB B2A) and reverse (sRGB A2B → B2A
        // mft1) both build and match the engine.
        for (src, dst) in [(LUT_MFT1, SRGB), (SRGB, LUT_MFT1)] {
            let out = run_rgba8(
                &mut IccTransform::new(src, dst, IccIntent::Perceptual).unwrap(),
                LUT_PIXELS,
            );
            assert_eq!(
                out,
                oracle_rgba8(src, dst, RenderingIntent::Perceptual, LUT_PIXELS)
            );
            // Behavioral pin on the channel swap — pure red comes out
            // blue-dominant, so the CLUT genuinely permutes channels.
            let out = run_rgba8(
                &mut IccTransform::new(src, dst, IccIntent::Perceptual).unwrap(),
                &[[255, 0, 0, 255]],
            );
            assert!(
                out[2] > out[0] && out[2] > out[1],
                "swapped red came out {out:?}"
            );
        }
        // Swap ∘ swap = identity: the self-pipeline returns each
        // pixel near-verbatim (±2 covers resample/store
        // quantization).
        let mut kernel = IccTransform::new(LUT_MFT1, LUT_MFT1, IccIntent::Perceptual).unwrap();
        let out = run_rgba8(&mut kernel, LUT_PIXELS);
        for (px, ch) in LUT_PIXELS.iter().zip(out.chunks(4)) {
            for i in 0..3 {
                assert!(
                    px[i].abs_diff(ch[i]) <= 2,
                    "swap∘swap pixel {px:?} came out {ch:?}"
                );
            }
            assert_eq!(px[3], ch[3], "alpha passthrough");
        }
    }

    /// mft2 path: the `Lut16` scales run in both directions — output
    /// is bit-exact against the raw executor, and the self-pipeline
    /// `lut_mft2 → lut_mft2` composes A2B ×0.5 with B2A ×0.75 into an
    /// observable ×0.375 darkening.
    #[test]
    fn icc_transform_mft2_lut_path_scales_channels() {
        for (src, dst) in [(LUT_MFT2, SRGB), (SRGB, LUT_MFT2)] {
            let out = run_rgba8(
                &mut IccTransform::new(src, dst, IccIntent::Perceptual).unwrap(),
                LUT_PIXELS,
            );
            assert_eq!(
                out,
                oracle_rgba8(src, dst, RenderingIntent::Perceptual, LUT_PIXELS)
            );
        }
        let mut kernel = IccTransform::new(LUT_MFT2, LUT_MFT2, IccIntent::Perceptual).unwrap();
        let out = run_rgba8(&mut kernel, LUT_PIXELS);
        for (px, ch) in LUT_PIXELS.iter().zip(out.chunks(4)) {
            for i in 0..3 {
                // ×0.5 × 0.75 = ×0.375; ±3 covers store quantization.
                let expected = f32::from(px[i]) * 0.375;
                assert!(
                    (f32::from(ch[i]) - expected).abs() <= 3.0,
                    "×0.375 pixel {px:?} came out {ch:?}"
                );
            }
            assert_eq!(px[3], ch[3], "alpha passthrough");
        }
    }

    /// mAB/mBA path: the `LutMultidimensionalType` tags run in both
    /// directions — output is bit-exact against the raw executor, and
    /// the CLUT channel rotation is observable: pure green rotates to
    /// a blue-dominant PCS, so `lut_mab → srgb` emits a blue pixel.
    #[test]
    fn icc_transform_mab_lut_path_executes_clut() {
        for (src, dst) in [(LUT_MAB, SRGB), (SRGB, LUT_MAB)] {
            let out = run_rgba8(
                &mut IccTransform::new(src, dst, IccIntent::Perceptual).unwrap(),
                LUT_PIXELS,
            );
            assert_eq!(
                out,
                oracle_rgba8(src, dst, RenderingIntent::Perceptual, LUT_PIXELS)
            );
        }
        // Behavioral pin on the rotation: device green → A2B rotate →
        // blue-ish PCS → sRGB B2A → blue-dominant output.
        let out = run_rgba8(
            &mut IccTransform::new(LUT_MAB, SRGB, IccIntent::Perceptual).unwrap(),
            &[[0, 255, 0, 255]],
        );
        assert!(
            out[2] > 128 && out[2] > out[0] && out[2] > out[1],
            "rotated green came out {out:?}"
        );
        // The same input through the no-op sRGB→sRGB path stays
        // green — the difference is the mAB LUT, not the pipeline.
        let baseline = run_rgba8(
            &mut IccTransform::new(SRGB, SRGB, IccIntent::Perceptual).unwrap(),
            &[[0, 255, 0, 255]],
        );
        assert!(baseline[1] > baseline[2]);
    }

    /// Missing-tag rejection: `lut_a2b0_only` carries only A2B0.
    /// Forward + Perceptual selects it and runs; forward +
    /// `RelativeColorimetric` needs the absent A2B1 tag — `new` rejects
    /// with `"icc transform"`. The reverse direction has no B2A
    /// tables at all — both intents reject.
    #[test]
    fn icc_transform_missing_intent_lut_tag_rejects() {
        assert!(
            IccTransform::new(LUT_A2B0_ONLY, SRGB, IccIntent::Perceptual).is_ok(),
            "A2B0 + Perceptual must build"
        );
        assert_eq!(
            IccTransform::new(LUT_A2B0_ONLY, SRGB, IccIntent::RelativeColorimetric).map(|_| ()),
            Err(Error::InvalidValue("icc transform")),
            "missing A2B1 must reject"
        );
        for intent in [IccIntent::Perceptual, IccIntent::RelativeColorimetric] {
            assert_eq!(
                IccTransform::new(SRGB, LUT_A2B0_ONLY, intent).map(|_| ()),
                Err(Error::InvalidValue("icc transform")),
                "missing B2A tables must reject ({intent:?})"
            );
        }
    }

    /// RT contract on the LUT path: repeated `process` on an
    /// mAB-tag kernel stays deterministic and never mutates `src` —
    /// the per-pixel executor is stateless.
    #[test]
    fn rt_process_icc_lut_path_is_deterministic_and_pure() {
        let src = rgba8_frame(
            2,
            1,
            &[[200, 100, 50, 255], [60, 90, 200, 128]],
            ColorInfo::default(),
        );
        let before = src.plane(0).unwrap().data().to_vec();
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        let mut kernel = IccTransform::new(LUT_MAB, SRGB, IccIntent::Perceptual).unwrap();
        let mut first = [0_u8; 8];
        for i in 0..64 {
            kernel.process(&src, &mut dst).unwrap();
            let row = dst.plane(0).unwrap().row(0).unwrap();
            if i == 0 {
                first.copy_from_slice(row);
            } else {
                assert_eq!(row, &first);
            }
        }
        assert_eq!(src.plane(0).unwrap().data(), &before[..]);
    }
}
