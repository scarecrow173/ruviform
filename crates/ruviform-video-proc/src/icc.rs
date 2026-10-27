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
    /// constructor pre-built). Allocation-free is verified
    /// mechanically: the `rt_process_icc_*` tests run `process`
    /// inside an `assert_no_alloc` window, so an executor-internal
    /// (de)allocation aborts the test binary (Issue #126). Dropping
    /// `src`/`dst` stays the caller's non-RT concern.
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

    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{ContentLightLevel, HdrMetadata, SharedPlane};

    use super::*;

    /// The committed ICC fixtures — moxcms `encode()` output of the
    /// pinned version's built-in profiles (see `generate_fixtures`).
    const SRGB: &[u8] = include_bytes!("../tests/fixture/srgb.icc");
    const DISPLAY_P3: &[u8] = include_bytes!("../tests/fixture/display_p3.icc");
    const GRAY_GAMMA22: &[u8] = include_bytes!("../tests/fixture/gray_gamma22.icc");

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// One-shot fixture (re)generator — rewrites the committed `.icc`
    /// files under `tests/fixture/` from the pinned engine's built-in
    /// profiles. Run explicitly:
    /// `cargo test -p ruviform-video-proc --features icc generate_fixtures -- --ignored`.
    #[test]
    #[ignore = "one-shot fixture regeneration — run explicitly when the pinned engine changes"]
    fn generate_fixtures() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture");
        std::fs::create_dir_all(dir).unwrap();
        for (name, profile) in [
            ("srgb.icc", ColorProfile::new_srgb()),
            ("display_p3.icc", ColorProfile::new_display_p3()),
            ("gray_gamma22.icc", ColorProfile::new_gray_with_gamma(2.2)),
        ] {
            std::fs::write(format!("{dir}/{name}"), profile.encode().unwrap()).unwrap();
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
        // expected here. An intent-distinguishing LUT fixture pair is
        // a recorded follow-up.
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
    /// bounded — the `assert_no_alloc` window aborts the test binary
    /// on any (de)allocation inside it, **including allocations inside
    /// the baked moxcms executor** (the kernel wraps the engine's
    /// `transform` call inside the window; the allocating
    /// `IccTransform::new` prepare step and fixtures stay outside) —
    /// and observably repeated calls on the same kernel succeed and
    /// stay deterministic, never panic.
    #[test]
    fn rt_process_icc_is_repeatable_and_deterministic() {
        let src = rgba8_frame(1, 1, &[[128, 200, 50, 255]], ColorInfo::default());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        let mut first = [0_u8; 4];
        for i in 0..64 {
            assert_no_alloc::assert_no_alloc(|| kernel.process(&src, &mut dst)).unwrap();
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
    /// drain), and the `assert_no_alloc` window aborts on any
    /// (de)allocation inside it, engine-internal allocations included.
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
        let mut kernel = IccTransform::new(SRGB, DISPLAY_P3, IccIntent::Perceptual).unwrap();
        assert_no_alloc::assert_no_alloc(|| kernel.process(&src, &mut dst)).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }
}
