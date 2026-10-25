//! Working-space entry/exit kernels: [`ToWorking`] decodes a
//! `ColorInfo`-tagged frame into the pipeline's `RgbaF32` linear
//! working space — the transfer decode and the source-primaries →
//! `BT.2020` gamut transform fused into one pass — and [`FromWorking`]
//! encodes working-space frames back out to a constructor-fixed
//! primaries/transfer pair.
//!
//! ## Working space
//!
//! Per `docs/designs/color-conversion-kernels.md` the working space is
//! `RgbaF32` tagged `{primaries: Bt2020, transfer: Linear, matrix:
//! Identity, range: Full}` — channel values are linear light on an
//! absolute-luminance scale where `1.0` = 100 cd/m² (the SDR reference
//! white). The value range is not limited to `[0, 1]`: scene-referred
//! values above `1.0` and negative wide-gamut remnants carry through
//! losslessly. Alpha is straight, normalized to `[0, 1]`, and passes
//! through every kernel unconverted — never transfer-decoded/encoded
//! and never matrixed.
//!
//! The transfer decode/encode formulas (signal ↔ absolute luminance on
//! the `1.0` = 100 nits scale), their clamp rules, the per-variant
//! combined 3×3 primaries matrices and the `Unspecified` → `Bt709`
//! slot resolution live in `crate::transfer` — all `f64` math, the
//! reference evaluation whose `binary32` rounding is the output.
//!
//! ## `ColorInfo` interpretation
//!
//! [`ToWorking`] reads `src.color()` on every call (the
//! [`YuvToRgb`](crate::YuvToRgb) convention): `primaries`/`transfer`
//! resolve under the shared default rule (`Unspecified` → `Bt709`) and
//! unknown future `#[non_exhaustive]` variants are rejected;
//! `matrix`/`range` do not apply to RGB input and are not validated.
//! [`FromWorking`] resolves the destination `primaries`/`transfer` at
//! construction instead — same default rule and rejection — because
//! the destination interpretation is a kernel parameter, not a
//! property readable from the input frame (the
//! [`RgbToYuv`](crate::RgbToYuv) precedent).
//!
//! ## Equivalence contract
//!
//! These scalar implementations are the reference per the baseline
//! DSP/SIMD decision: each channel's output is the `f64` evaluation of
//! the documented formulas rounded to `binary32`. Any optimized
//! backend must match within **≤ 4 ulp (`binary32`)** per channel —
//! `Rgba8` quantization outputs are additionally allowed the ±1 code
//! of the quantization boundary. `Rgba8`/`RgbaF16` decode inputs are
//! exact. Non-finite channels propagate per `f32`/`f64` semantics
//! (including zero-coefficient `0·∞` → `NaN` mixing through the
//! primaries matrix); the contract bounds finite outputs only.
//!
//! Contract source: `docs/designs/color-conversion-kernels.md` and
//! `docs/test-specs/color-working-space-kernel-tests.md`.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{
    ColorInfo, ColorPrimaries, ColorRange, MatrixCoefficients, PixelFormat,
    TransferCharacteristics, VideoFrame,
};

use crate::rgba_f16::f16_bits_to_f64;
use crate::transfer::{
    self, FROM_BT2020, Mat3, RESOLVED_PRIMARIES, RESOLVED_TRANSFER, TO_BT2020, TRANSFER_DECODE,
    TRANSFER_ENCODE,
};

/// Reads one `Rgba8` pixel's normalized signal channels (`v / 255`).
fn rgba8_signal(px: [u8; 4]) -> [f64; 4] {
    std::array::from_fn(|i| f64::from(px[i]) / 255.0)
}

/// Reads one `RgbaF16` pixel's signal channels — little-endian
/// `binary16` pairs decoded exactly via `f16_bits_to_f64`.
fn rgba_f16_signal(px: [u8; 8]) -> [f64; 4] {
    std::array::from_fn(|i| f16_bits_to_f64(u16::from_le_bytes([px[2 * i], px[2 * i + 1]])))
}

/// Reads one `RgbaF32` pixel's signal channels — raw little-endian
/// `binary32` quads widened to `f64` (every `f32` is exact in `f64`;
/// `NaN`/`±∞` propagate).
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

/// The fused working-space entry transform on one normalized signal
/// quad: transfer-decode the RGB channels, apply the `src primaries →
/// BT.2020` matrix, pass alpha through as the normalized signal —
/// `signal[3]` deliberately skips `decode`.
fn to_working_px(decode: fn(f64) -> f64, matrix: &Mat3, signal: [f64; 4]) -> [f64; 4] {
    let [r, g, b] = transfer::apply(
        matrix,
        [decode(signal[0]), decode(signal[1]), decode(signal[2])],
    );
    [r, g, b, signal[3]]
}

/// The fused working-space exit transform on one working-space quad:
/// the `BT.2020 → dst primaries` matrix on RGB, then the transfer
/// encode; alpha passes through un-encoded (`signal[3]` deliberately
/// skips `encode`).
fn from_working_px(matrix: &Mat3, encode: fn(f64) -> f64, signal: [f64; 4]) -> [f64; 4] {
    let [r, g, b] = transfer::apply(matrix, [signal[0], signal[1], signal[2]]);
    [encode(r), encode(g), encode(b), signal[3]]
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

/// `round(clamp(v, 0, 1) · 255)` quantization (round-half-up) for
/// `Rgba8` output channels — applied to RGB channels after the
/// transfer encode and to alpha as the passthrough normalized value.
/// Negative and `> 1` signals saturate; `NaN` folds to `0` (explicit —
/// the `as u8` cast would land on `0` anyway).
// `normalized * 255` is in `[0, 255]` and `.round()` makes it integral
// and non-negative — the cast can neither truncate nor lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize8(v: f64) -> u8 {
    let normalized = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    (normalized * 255.0).round() as u8
}

/// Prepared working-space entry kernel — `Rgba8`/`RgbaF16`/`RgbaF32`
/// → `RgbaF32` fused transfer-decode + gamut transform. The two-phase
/// API shape of ADR D1-b: [`new`](Self::new) is the non-RT `prepare`
/// step (this kernel bakes nothing — the frame's own `ColorInfo`
/// drives each call), and [`process`](Self::process) is the
/// allocation-free conversion step.
///
/// ## Conversion rules
///
/// `src`'s pixel format selects the signal read: `Rgba8` channels
/// normalize by `v/255`, `RgbaF16` channels decode through
/// `f16_bits_to_f64`, `RgbaF32` channels read as raw little-endian
/// `binary32`. Every call re-reads `src.color()`: `primaries` and
/// `transfer` resolve under the default rule (`Unspecified` →
/// `Bt709`); unknown future variants reject with
/// [`Error::InvalidValue`]. `matrix`/`range` do not apply to RGB
/// input and are not validated.
///
/// Per pixel: the RGB signal channels decode through the resolved
/// transfer (the `crate::transfer` formulas — signal → absolute
/// luminance on the `1.0` = 100 nits scale), then transform RGB-only
/// by the `src primaries → BT.2020` combined 3×3; alpha passes through
/// as the normalized signal. An `RgbaF32` input already tagged
/// `Bt2020`/`Linear` is the mathematical identity — the pixels still
/// flow through the full path (recording the working-space `ColorInfo`
/// is this kernel's responsibility; there is no no-op early return).
/// `NaN`/`±∞` channels propagate per `f32`/`f64` semantics.
///
/// ## Output frame
///
/// `dst` must be `RgbaF32` at the same `width`×`height` and
/// exclusively owned — a shared `dst` (`plane_mut(0)` → `None`) is
/// rejected before a single byte is written. Rows are processed
/// through `Plane::row`/`PlaneMut::row_mut` only, so `stride >
/// row_bytes` padding is never read or written.
///
/// `dst`'s `ColorInfo` records the working space literally:
/// `{primaries: Bt2020, transfer: Linear, matrix: Identity, range:
/// Full, hdr: src.hdr}` — `hdr` and `pts`/`duration` propagate from
/// `src`; a color conversion never retimes the frame.
///
/// ## Equivalence contract
///
/// The module doc's contract applies: per-channel output is the `f64`
/// reference evaluation rounded to `binary32`; optimized backends must
/// match within ≤ 4 ulp (`binary32`). `Rgba8`/`RgbaF16` decode inputs
/// are exact.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{
///     ColorPrimaries, PixelFormat, TransferCharacteristics, VideoFrame,
/// };
/// use ruviform_video_proc::ToWorking;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// src.plane_mut(0)
///     .unwrap()
///     .data_mut()
///     .copy_from_slice(&[255, 255, 255, 255]);
/// let mut color = src.color();
/// color.primaries = ColorPrimaries::Bt709;
/// color.transfer = TransferCharacteristics::Bt709;
/// src.set_color(color);
/// let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts).unwrap();
///
/// ToWorking::new().process(&src, &mut dst).unwrap();
///
/// // SDR white (1.0 signal) decodes to working-space 1.0 = 100 cd/m².
/// let row = dst.plane(0).unwrap().row(0).unwrap();
/// let r = f32::from_le_bytes(row[0..4].try_into().unwrap());
/// assert!((r - 1.0).abs() < 1e-6);
/// assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
/// assert_eq!(dst.color().transfer, TransferCharacteristics::Linear);
/// ```
#[derive(Debug)]
pub struct ToWorking;

impl ToWorking {
    /// Builds the kernel — nothing to bake (the frame's `ColorInfo`
    /// drives each call), so construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::ToWorking::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Converts `src` into `dst` per the rules on [`ToWorking`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `Rgba8`/`RgbaF16`/
    /// `RgbaF32` (`Yuv*p` or an unknown format variant), `dst` is not
    /// `RgbaF32`, the frames disagree on dimensions, `src`'s
    /// `primaries`/`transfer` is an unknown future variant, or `dst`'s
    /// backing is still shared (`dst.plane_mut(0)` → `None`). All
    /// validation runs before any write; the in-loop row borrows are
    /// infallible under the `Plane` contract — a failed call never
    /// partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking
    /// and bounded (exactly `width`×`height` pixel iterations over
    /// borrowed rows; the only scratch is stack channel quads).
    /// Dropping `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        match src.format() {
            PixelFormat::Rgba8 | PixelFormat::RgbaF16 | PixelFormat::RgbaF32 => {}
            // Planar `Yuv*p` and every future `#[non_exhaustive]`
            // variant — RGB preparation belongs to `YuvToRgb` & co.
            _ => return Err(Error::InvalidValue("src format")),
        }
        if dst.format() != PixelFormat::RgbaF32 {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        // Resolve this call's interpretation ahead of the plane borrows
        // — every failure path above writes nothing.
        let decode = TRANSFER_DECODE[transfer::transfer_slot(src_color.transfer)?];
        let matrix = &TO_BT2020[transfer::primaries_slot(src_color.primaries)?];
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
                match src.format() {
                    PixelFormat::Rgba8 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px =
                                f32_px_bytes(to_working_px(decode, matrix, rgba8_signal(*src_px)));
                        }
                    }
                    PixelFormat::RgbaF16 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px = f32_px_bytes(to_working_px(
                                decode,
                                matrix,
                                rgba_f16_signal(*src_px),
                            ));
                        }
                    }
                    PixelFormat::RgbaF32 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px = f32_px_bytes(to_working_px(
                                decode,
                                matrix,
                                rgba_f32_signal(src_px),
                            ));
                        }
                    }
                    // Validated above — this arm only keeps the match
                    // panic-free on future `PixelFormat` variants.
                    _ => return Err(Error::InvalidValue("src format")),
                }
            }
        }
        // Record the working space literally; `hdr` and the timestamps
        // propagate — a color conversion never retimes the frame.
        dst.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Linear,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: src_color.hdr,
        });
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for ToWorking {
    /// Equivalent to [`ToWorking::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

/// Prepared working-space exit kernel — `RgbaF32` → `Rgba8`/`RgbaF32`
/// inverse transform: the `BT.2020 → dst primaries` 3×3 and the
/// destination transfer encode are baked by the constructor
/// ([`new`](Self::new) — the non-RT `prepare` step), and
/// [`process`](Self::process) is the allocation-free conversion step.
///
/// ## Conversion rules
///
/// Per pixel: the RGB working-space values transform by the baked
/// `BT.2020 → dst primaries` matrix, then encode through the baked
/// transfer (working value → signal, the `crate::transfer` formulas);
/// alpha passes through as the normalized signal (never encoded). The
/// encode-side clamp rules are per transfer: `Pq`/`Hlg` saturate at
/// the signal domain `[0, 1]`; `Bt709`/`Smpte170m`/`Linear` are not
/// clamped — negative and `> 1` signals are preserved on `RgbaF32`
/// output.
///
/// `Rgba8` output quantizes each channel with
/// `round(clamp(v, 0, 1)·255)` (round-half-up) — negative and `> 1`
/// encoded signals saturate at the quantization step. Feeding HDR
/// working-space content (values `> 1.0`) straight into an SDR
/// primaries/transfer pair without a [`ToneMapSdr`](crate::ToneMapSdr)-stage
/// tone map is consumer misuse — the highlights
/// saturate by design.
///
/// The identity construction (`Bt2020` + `Linear`) applies the
/// identity matrix and the identity encode — combined with an `Rgba8`
/// destination it expresses `RgbaF32`'s pure quantization, so no
/// dedicated shuffle kernel exists.
///
/// ## Output frame
///
/// `src` must be `RgbaF32`; `dst` must be `Rgba8` or `RgbaF32` at the
/// same `width`×`height` and exclusively owned — a shared `dst`
/// (`plane_mut(0)` → `None`) is rejected before a single byte is
/// written. Rows are processed through `Plane::row`/`PlaneMut::
/// row_mut` only, so `stride > row_bytes` padding is never read or
/// written.
///
/// `dst`'s `ColorInfo` records the constructor-resolved
/// interpretation: `{primaries: resolved dst_primaries, transfer:
/// resolved dst_transfer, matrix: Identity, range: Full, hdr:
/// src.hdr}` — `pts`/`duration` propagate from `src`.
///
/// ## Equivalence contract
///
/// The module doc's contract applies: per-channel output is the `f64`
/// reference evaluation rounded to `binary32`; optimized backends must
/// match within ≤ 4 ulp (`binary32`), with `Rgba8` outputs allowed
/// the quantization boundary's ±1 code.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{
///     ColorPrimaries, PixelFormat, TransferCharacteristics, VideoFrame,
/// };
/// use ruviform_video_proc::FromWorking;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts).unwrap();
/// // Working-space SDR white: 1.0 = 100 cd/m², little-endian f32s.
/// src.plane_mut(0)
///     .unwrap()
///     .data_mut()
///     .copy_from_slice(&[1.0_f32.to_le_bytes(); 4].concat());
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
///
/// let mut kernel =
///     FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709).unwrap();
/// kernel.process(&src, &mut dst).unwrap();
///
/// // 1.0 luminance re-encodes to the 1.0 signal → code 255.
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[255; 4]);
/// assert_eq!(dst.color().primaries, ColorPrimaries::Bt709);
/// ```
#[derive(Debug)]
pub struct FromWorking {
    /// Baked `BT.2020 → dst primaries` combined matrix for the
    /// constructor-resolved slot.
    matrix: Mat3,
    /// Baked transfer encode (`working value → signal`) for the
    /// constructor-resolved slot.
    encode: fn(f64) -> f64,
    /// The resolved primaries actually applied — recorded on `dst`.
    primaries: ColorPrimaries,
    /// The resolved transfer actually applied — recorded on `dst`.
    transfer: TransferCharacteristics,
}

impl FromWorking {
    /// Builds the prepared kernel, resolving `dst_primaries` ×
    /// `dst_transfer` under the documented default rules
    /// (`Unspecified` → `Bt709`) and baking the combined matrix +
    /// encode pair; an unknown future `#[non_exhaustive]` variant is
    /// rejected instead of silently mis-converting.
    ///
    /// **RT contract:** non-RT only by convention (the resolved values
    /// are stored inline — even here no allocation happens — but
    /// constructors are setup-path APIs).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `dst_primaries` or `dst_transfer` is
    /// an unknown future variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_video_core::{ColorPrimaries, TransferCharacteristics};
    /// use ruviform_video_proc::FromWorking;
    ///
    /// let _kernel =
    ///     FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Pq).unwrap();
    /// // `Unspecified` fields resolve to the BT.709 defaults.
    /// let _kernel =
    ///     FromWorking::new(ColorPrimaries::Unspecified, TransferCharacteristics::Unspecified)
    ///         .unwrap();
    /// ```
    pub fn new(
        dst_primaries: ColorPrimaries,
        dst_transfer: TransferCharacteristics,
    ) -> Result<Self> {
        let primaries = transfer::primaries_slot(dst_primaries)?;
        let transfer = transfer::transfer_slot(dst_transfer)?;
        Ok(Self {
            matrix: FROM_BT2020[primaries],
            encode: TRANSFER_ENCODE[transfer],
            primaries: RESOLVED_PRIMARIES[primaries],
            transfer: RESOLVED_TRANSFER[transfer],
        })
    }

    /// Converts `src` into `dst` per the rules on [`FromWorking`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `RgbaF32`, `dst` is not
    /// `Rgba8`/`RgbaF32`, the frames disagree on dimensions, or
    /// `dst`'s backing is still shared (`dst.plane_mut(0)` → `None`).
    /// All validation runs before any write; the in-loop row borrows
    /// are infallible under the `Plane` contract — a failed call
    /// never partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking
    /// and bounded (exactly `width`×`height` pixel iterations over
    /// borrowed rows; the only scratch is stack channel quads; matrix
    /// and encode were baked by [`new`](Self::new)). Dropping
    /// `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        if src.format() != PixelFormat::RgbaF32 {
            return Err(Error::InvalidValue("src format"));
        }
        let dst_format = dst.format();
        match dst_format {
            PixelFormat::Rgba8 | PixelFormat::RgbaF32 => {}
            // Every other format — planar `Yuv*p` and future
            // `#[non_exhaustive]` variants share the `_` arm.
            _ => return Err(Error::InvalidValue("dst format")),
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
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
                // `dst_format` was captured before the `plane_mut`
                // borrow — `dst.format()` cannot be called here.
                match dst_format {
                    PixelFormat::Rgba8 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                        {
                            let [r, g, b, a] =
                                from_working_px(&self.matrix, self.encode, rgba_f32_signal(src_px));
                            *dst_px = [quantize8(r), quantize8(g), quantize8(b), quantize8(a)];
                        }
                    }
                    PixelFormat::RgbaF32 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            *dst_px = f32_px_bytes(from_working_px(
                                &self.matrix,
                                self.encode,
                                rgba_f32_signal(src_px),
                            ));
                        }
                    }
                    // Validated above — this arm only keeps the match
                    // panic-free on future `PixelFormat` variants.
                    _ => return Err(Error::InvalidValue("dst format")),
                }
            }
        }
        // Record the constructor-resolved interpretation; `hdr` and
        // the timestamps propagate — a conversion never retimes.
        dst.set_color(ColorInfo {
            primaries: self.primaries,
            transfer: self.transfer,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: src_color.hdr,
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
    use ruviform_video_core::{
        ColorInfo, ContentLightLevel, HdrMetadata, PixelFormat, SharedPlane,
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

    /// Builds an `RgbaF16` frame of one row holding `pixels` as
    /// `binary16` bit patterns, tagged `color`.
    fn rgba_f16_frame(
        width: u32,
        height: u32,
        pixels: &[[u16; 4]],
        color: ColorInfo,
    ) -> VideoFrame {
        let mut frame = VideoFrame::new_owned(PixelFormat::RgbaF16, width, height, pts()).unwrap();
        frame
            .plane_mut(0)
            .unwrap()
            .data_mut()
            .as_chunks_mut::<8>()
            .0
            .iter_mut()
            .zip(pixels.iter())
            .for_each(|(dst, px)| {
                for (bits, dst_ch) in px.iter().zip(dst.as_chunks_mut::<2>().0.iter_mut()) {
                    *dst_ch = bits.to_le_bytes();
                }
            });
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

    /// Reads the first pixel of an `RgbaF32` frame's plane as four
    /// `f32` channels.
    fn f32_px(frame: &VideoFrame, x: usize, y: u32) -> [f32; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let px: &[u8; 16] = row[16 * x..16 * x + 16].try_into().unwrap();
        std::array::from_fn(|i| {
            f32::from_le_bytes([px[4 * i], px[4 * i + 1], px[4 * i + 2], px[4 * i + 3]])
        })
    }

    /// A `ColorInfo` with `primaries`/`transfer` set and everything
    /// else unspecified (`matrix`/`range`/`hdr`).
    fn color_tag(primaries: ColorPrimaries, transfer: TransferCharacteristics) -> ColorInfo {
        ColorInfo {
            primaries,
            transfer,
            ..ColorInfo::default()
        }
    }

    /// The working-space `ColorInfo` every `ToWorking` output records.
    fn working_tag() -> ColorInfo {
        ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Linear,
            matrix: MatrixCoefficients::Identity,
            range: ColorRange::Full,
            hdr: None,
        }
    }

    /// Golden: `Rgba8` `[255, 128, 0, 255]` tagged `Bt709`/`Bt709`
    /// decodes through the H.273 inverse OETF and the `Bt709 →
    /// BT.2020` matrix — expected values are the `f64` reference
    /// evaluation (`decode(1.0)` = 1.0, `decode(128/255)` ≈
    /// 0.2615 through the matrix columns).
    #[test]
    fn to_working_decodes_bt709_tagged_rgba8() {
        let src = rgba8_frame(
            1,
            1,
            &[[255, 128, 0, 255]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 0.713_505_321_017_274_3, ULP_BOUND);
        assert_ulp(g, 0.309_540_097_548_466_45, ULP_BOUND);
        assert_ulp(b, 0.039_405_291_249_058_45, ULP_BOUND);
        // Alpha is the passthrough normalized signal — never decoded.
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// Golden: an `Rgba8` `Pq`-tagged signal decodes through the
    /// ST 2084 EOTF to absolute luminance — `128/255` ≈ 94.1 nits ≈
    /// 0.9407 on the `1.0` = 100 nits scale (`Bt2020` primaries keep
    /// the matrix the identity).
    #[test]
    fn to_working_decodes_pq_tagged_signal() {
        let src = rgba8_frame(
            1,
            1,
            &[[128, 0, 0, 255]],
            color_tag(ColorPrimaries::Bt2020, TransferCharacteristics::Pq),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 0.940_745_991_723_271_8, ULP_BOUND);
        assert_ulp(g, 0.0, ULP_BOUND);
        assert_ulp(b, 0.0, ULP_BOUND);
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// Golden: an `Hlg`-tagged `RgbaF32` signal of `1.0` decodes to
    /// the BT.2100 reference OOTF output `S^1.2` at the nominal
    /// 1000-nit peak — `display_rel` 1.0 ↔ working `10.0`, the same
    /// absolute scale as PQ's 1000 nits.
    #[test]
    fn to_working_decodes_hlg_to_the_absolute_scale() {
        let src = rgba_f32_frame(
            1,
            1,
            &[[1.0, 1.0, 1.0, 1.0]],
            color_tag(ColorPrimaries::Bt2020, TransferCharacteristics::Hlg),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        for (i, ch) in f32_px(&dst, 0, 0).iter().enumerate() {
            if i == 3 {
                assert_ulp(*ch, 1.0, ULP_BOUND); // alpha passthrough
            } else {
                assert_ulp(*ch, 10.0, ULP_BOUND);
            }
        }
    }

    /// `RgbaF16` channels decode through `f16_bits_to_f64` — an exact
    /// read, so `Linear`/`Bt2020` input reproduces the stored values
    /// exactly.
    #[test]
    fn to_working_reads_rgba_f16_signals() {
        let src = rgba_f16_frame(
            1,
            1,
            // [1.0, 0.5, 0.0, 1.0] in binary16.
            &[[0x3C00, 0x3800, 0x0000, 0x3C00]],
            color_tag(ColorPrimaries::Bt2020, TransferCharacteristics::Linear),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        // Exact decode + identity transform → bit-exact output for
        // finite non-`−0.0` values (`−0.0` and NaN payloads may
        // canonicalize through the `f64` round trip).
        assert_eq!(
            f32_px(&dst, 0, 0).map(f32::to_bits),
            [1.0_f32, 0.5, 0.0, 1.0].map(f32::to_bits)
        );
    }

    /// Golden: an `RgbaF16` `Bt709`-tagged signal takes the full
    /// non-identity path — `binary16` read, the H.273 inverse OETF
    /// (`decode_bt709(0.5)` ≈ 0.2596 exercises the power branch) and
    /// the `Bt709 → BT.2020` matrix mixing columns 0 + 1.
    #[test]
    fn to_working_decodes_bt709_tagged_rgba_f16() {
        let src = rgba_f16_frame(
            1,
            1,
            // [1.0, 0.5, 0.0, 1.0] in binary16 — a saturated orange.
            &[[0x3C00, 0x3800, 0x0000, 0x3C00]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        // `f64` reference: `apply(&TO_BT2020[Bt709], [1.0,
        // 0.259_589_400_506_285_76, 0.0])`.
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 0.712_882_282_464_102_3, ULP_BOUND);
        assert_ulp(g, 0.307_800_229_257_183_5, ULP_BOUND);
        assert_ulp(b, 0.039_238_760_703_574_42, ULP_BOUND);
        // Alpha is the passthrough normalized signal — never decoded.
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// Gamut: a `P3D65`-tagged unit-red pixel maps into `BT.2020`
    /// coordinates through the combined matrix — including the
    /// legitimately negative blue component, kept unclamped.
    #[test]
    fn to_working_maps_p3d65_primaries_to_bt2020() {
        let src = rgba_f32_frame(
            1,
            1,
            &[[1.0, 0.0, 0.0, 1.0]],
            color_tag(ColorPrimaries::P3D65, TransferCharacteristics::Linear),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 0.753_833_034_361_721_7, ULP_BOUND);
        assert_ulp(g, 0.045_743_848_965_358_27, ULP_BOUND);
        assert_ulp(b, -0.001_210_340_354_518_392_1, ULP_BOUND);
        assert!(b < 0.0);
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// `Unspecified` `primaries`/`transfer` resolve to the `Bt709`
    /// defaults — identical output to an explicit `Bt709`/`Bt709` tag.
    #[test]
    fn to_working_unspecified_resolves_to_bt709() {
        let unspecified = rgba8_frame(
            2,
            1,
            &[[255, 128, 64, 255], [10, 20, 30, 200]],
            ColorInfo::default(),
        );
        let tagged = rgba8_frame(
            2,
            1,
            &[[255, 128, 64, 255], [10, 20, 30, 200]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        let mut dst_default = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        let mut dst_tagged = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToWorking::new()
            .process(&unspecified, &mut dst_default)
            .unwrap();
        ToWorking::new().process(&tagged, &mut dst_tagged).unwrap();
        assert_eq!(
            dst_default.plane(0).unwrap().data(),
            dst_tagged.plane(0).unwrap().data()
        );
    }

    /// `matrix`/`range` do not apply to RGB input: every value —
    /// `Unspecified`, `Identity`, even a YCbCr `Bt601`/`Limited` tag —
    /// is accepted without validation and produces identical output.
    #[test]
    fn to_working_does_not_validate_matrix_or_range() {
        let mut tagged = rgba8_frame(
            1,
            1,
            &[[200, 100, 50, 255]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        tagged.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Bt709,
            matrix: MatrixCoefficients::Bt601,
            range: ColorRange::Limited,
            hdr: None,
        });
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        ToWorking::new().process(&tagged, &mut dst).unwrap();
        // The matrix/range tag changed nothing — the `f64` reference
        // for `Rgba8` [200, 100, 50] through `Bt709`/`Bt709`.
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 0.443_418_863_487_060_33, ULP_BOUND);
        assert_ulp(g, 0.196_696_593_121_842_5, ULP_BOUND);
        assert_ulp(b, 0.072_990_001_067_306_69, ULP_BOUND);
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// Every known `primaries`/`transfer` variant resolves — the
    /// `_` rejection arms exist only for future `#[non_exhaustive]`
    /// variants (structurally reachable, review-verified per the test
    /// spec since no rejectable variant can be constructed).
    #[test]
    fn to_working_accepts_every_known_color_variant() {
        for primaries in [
            ColorPrimaries::Unspecified,
            ColorPrimaries::Bt709,
            ColorPrimaries::Smpte170m,
            ColorPrimaries::Bt470Bg,
            ColorPrimaries::Bt2020,
            ColorPrimaries::P3D65,
        ] {
            for transfer in [
                TransferCharacteristics::Unspecified,
                TransferCharacteristics::Bt709,
                TransferCharacteristics::Smpte170m,
                TransferCharacteristics::Linear,
                TransferCharacteristics::Pq,
                TransferCharacteristics::Hlg,
            ] {
                let src = rgba8_frame(
                    1,
                    1,
                    &[[128, 128, 128, 255]],
                    color_tag(primaries, transfer),
                );
                let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
                ToWorking::new()
                    .process(&src, &mut dst)
                    .unwrap_or_else(|e| panic!("{primaries:?} × {transfer:?} rejected: {e}"));
            }
        }
    }

    /// An `RgbaF32` input already tagged `Bt2020`/`Linear` is the
    /// mathematical identity — the pixels still flow through the full
    /// decode+matrix path (no early return) and land bit-identical
    /// for finite non-`−0.0` values (`−0.0` and NaN payloads may
    /// canonicalize), while `dst.color` records the working space.
    #[test]
    fn to_working_identity_path_copies_pixels_and_records() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.25, 0.5, 2.5, 0.75], [0.0, 1.0, 0.125, 1.0]],
            working_tag(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        // `f32 → f64 → identity → f32` is bit-exact for finite
        // non-`−0.0` values (`−0.0` and NaN payloads may
        // canonicalize).
        assert_eq!(
            f32_px(&dst, 0, 0).map(f32::to_bits),
            [0.25_f32, 0.5, 2.5, 0.75].map(f32::to_bits)
        );
        assert_eq!(
            f32_px(&dst, 1, 0).map(f32::to_bits),
            [0.0_f32, 1.0, 0.125, 1.0].map(f32::to_bits)
        );
        assert_eq!(dst.color(), working_tag());
    }

    /// Non-finite `RgbaF32` channels propagate per `f32`/`f64`
    /// semantics: `NaN` stays `NaN`, `+∞` survives on its own channel
    /// and — through the matrix's zero coefficients (`0·∞`) — mixes
    /// `NaN` into the remaining outputs.
    #[test]
    fn to_working_non_finite_channels_propagate() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[f32::INFINITY, 0.0, 0.0, 1.0], [f32::NAN, 0.5, 0.5, 1.0]],
            working_tag(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        // `+∞` lands on R; `0·∞` in the matrix's zero terms turns
        // G/B `NaN` — the defined f64 mixing, not a failure.
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert!(r.is_infinite() && r.is_sign_positive());
        assert!(g.is_nan() && b.is_nan());
        assert_eq!(a.to_bits(), 1.0_f32.to_bits());
        let [r2, ..] = f32_px(&dst, 1, 0);
        assert!(r2.is_nan());
    }

    /// `dst` records the working space literally and inherits `hdr`/
    /// `pts`/`duration` from `src`.
    #[test]
    fn to_working_records_working_space_metadata() {
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let hdr = HdrMetadata {
            mastering_display: None,
            content_light_level: Some(ContentLightLevel {
                max_cll: 1_000,
                max_fall: 400,
            }),
        };
        let mut src = rgba8_frame(
            1,
            1,
            &[[255, 255, 255, 255]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        src.set_color(ColorInfo {
            hdr: Some(hdr),
            ..src.color()
        });
        src.set_duration(Some(duration));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        // Scrub `dst`'s timestamps so the propagation below is
        // observable — `dst` would otherwise already hold `src`'s pts.
        dst.set_pts(MediaTime::ZERO);
        ToWorking::new().process(&src, &mut dst).unwrap();
        assert_eq!(
            dst.color(),
            ColorInfo {
                hdr: Some(hdr),
                ..working_tag()
            }
        );
        assert_eq!(dst.pts(), src.pts());
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Format/dimension/shared-backing rejections all happen before
    /// any write — a failed call leaves `dst` untouched.
    #[test]
    fn to_working_invalid_inputs_are_rejected() {
        let src = rgba8_frame(
            1,
            1,
            &[[128, 128, 128, 255]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();

        // Planar YUV sources — RGB preparation is `YuvToRgb`'s job.
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        assert_eq!(
            ToWorking::new().process(&src_yuv, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Non-`RgbaF32` destinations.
        let mut dst_u8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            ToWorking::new().process(&src, &mut dst_u8),
            Err(Error::InvalidValue("dst format"))
        );
        // The rejected `dst` is untouched — pixels, `ColorInfo`, pts.
        assert_eq!(dst_u8.plane(0).unwrap().data(), &[0; 4]);
        assert_eq!(dst_u8.color(), ColorInfo::default());
        assert_eq!(dst_u8.pts(), pts());
        let mut dst_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            ToWorking::new().process(&src, &mut dst_f16),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        assert_eq!(
            ToWorking::new().process(&src, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // A shared `dst` (clone still holding the `Arc` backing) —
        // rejected before any write.
        let mut dst_shared = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let sharing_clone = dst_shared.clone();
        assert!(dst_shared.plane_mut(0).is_none());
        assert_eq!(
            ToWorking::new().process(&src, &mut dst_shared),
            Err(Error::InvalidValue("dst plane"))
        );
        // Rejection wrote nothing.
        assert_eq!(dst_shared.plane(0).unwrap().data(), &[0; 16]);
        drop(sharing_clone);
        assert!(ToWorking::new().process(&src, &mut dst_shared).is_ok());

        assert_eq!(dst.plane(0).unwrap().data(), &[0; 16]);
    }

    /// A `stride > row_bytes` destination: conversion writes rows
    /// only — padding bytes keep their caller-chosen contents.
    #[test]
    fn to_working_padded_dst_stride_is_never_written() {
        // 1×2 `RgbaF32` dst: row_bytes 16, padded stride 20 → usable
        // 20 + 16 = 36 bytes — row 0 at 0..16, inter-row padding at
        // 16..20, row 1 at 20..36.
        let src = rgba8_frame(
            1,
            2,
            &[[255, 0, 0, 255], [255, 0, 0, 255]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Linear),
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
        ToWorking::new().process(&src, &mut dst).unwrap();
        let data = dst.plane(0).unwrap().data();
        // Both rows written (`Linear` decode of 1.0 → `Bt709 →
        // BT.2020` column 0 ≈ 0.6274); the inter-row padding keeps 0xEE.
        assert_ulp(f32_px(&dst, 0, 0)[0], 0.627_403_895_934_699_1, ULP_BOUND);
        assert_eq!(&data[16..20], &[0xEE; 4]);
        assert_ulp(f32_px(&dst, 0, 1)[0], 0.627_403_895_934_699_1, ULP_BOUND);
    }

    /// A `stride > row_bytes` source: conversion reads the row
    /// region only — the padding bytes never enter the pipeline.
    #[test]
    fn to_working_padded_src_stride_reads_rows_only() {
        // 1×2 `Rgba8` src: row_bytes 4, padded stride 8 → usable
        // 8 + 4 = 12 bytes — row 0 at 0..4, padding at 4..8, row 1
        // at 8..12. 0xEE is not pixel data: a read straying into
        // padding would corrupt the outputs asserted below.
        let mut src = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    vec![255_u8, 0, 0, 255, 0xEE, 0xEE, 0xEE, 0xEE, 0, 255, 0, 255]
                        .into_boxed_slice(),
                ),
                stride: 8,
            }],
        )
        .unwrap();
        assert_eq!(src.plane(0).unwrap().stride(), 8);
        src.set_color(color_tag(
            ColorPrimaries::Bt709,
            TransferCharacteristics::Linear,
        ));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 2, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        // Row 0 = unit red → `Bt709 → BT.2020` column 0; row 1 =
        // unit green → column 1 — exactly the row contents.
        let [r0, g0, b0, a0] = f32_px(&dst, 0, 0);
        assert_ulp(r0, 0.627_403_895_934_699_1, ULP_BOUND);
        assert_ulp(g0, 0.069_097_289_358_231_99, ULP_BOUND);
        assert_ulp(b0, 0.016_391_438_875_150_224, ULP_BOUND);
        assert_ulp(a0, 1.0, ULP_BOUND);
        let [r1, g1, b1, a1] = f32_px(&dst, 0, 1);
        assert_ulp(r1, 0.329_283_038_377_883_8, ULP_BOUND);
        assert_ulp(g1, 0.919_540_395_075_458_7, ULP_BOUND);
        assert_ulp(b1, 0.088_013_307_877_225_78, ULP_BOUND);
        assert_ulp(a1, 1.0, ULP_BOUND);
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_to_working_is_repeatable_and_deterministic() {
        let src = rgba8_frame(
            1,
            1,
            &[[128, 200, 50, 255]],
            color_tag(ColorPrimaries::P3D65, TransferCharacteristics::Pq),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        let mut kernel = ToWorking::new();
        let mut first = [0_u8; 16];
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
    fn rt_process_to_working_never_mutates_source() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.25, 0.5, 0.75, 1.0], [1.5, 2.0, 2.5, 0.5]],
            color_tag(ColorPrimaries::Bt709, TransferCharacteristics::Bt709),
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        ToWorking::new().process(&src, &mut dst).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }

    /// Golden: ctor `Bt709`/`Bt709` — working-space white `[1, 1, 1]`
    /// (1.0 = 100 nits SDR white per channel) re-encodes to the
    /// ≈1.0 signal → code 255; a working-space unit R goes through
    /// the `BT.2020 → Bt709` column then saturates on G/B.
    #[test]
    fn from_working_encodes_bt709_to_rgba8() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[1.0, 1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]],
            working_tag(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        // White: matrix row sums ≈ 1.0 → encode ≈ 1.0 → 255.
        assert_eq!(&row[0..4], &[255, 255, 255, 255]);
        // Unit R: encode(1.6605) ≈ 1.2817 saturates to 255; the
        // −0.5605 / −0.0817 encode outputs saturate to 0.
        assert_eq!(&row[4..8], &[255, 0, 0, 255]);
    }

    /// Golden: ctor `Bt2020`/`Pq` (identity matrix) — working `1.0`
    /// = 100 nits inverse-EOTFs to the ST 2084 signal ≈ 0.5081 →
    /// `round(0.5081·255)` ≈ code 130 (the ±1 code quantization
    /// bound is allowed).
    #[test]
    fn from_working_encodes_pq_to_rgba8() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 1.0, 1.0, 1.0]], working_tag());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt2020, TransferCharacteristics::Pq)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        // `pq_eotf_inv(100)` = 0.508_078_421_517_399 → ·255 ≈ 129.56.
        for &code in &row[0..3] {
            assert!(
                (129..=131).contains(&code),
                "expected 130 ± 1 code, got {code}"
            );
        }
        assert_eq!(row[3], 255);
    }

    /// `RgbaF32` output writes the encode result un-clamped per the
    /// `Bt709` rule — a unit-R pixel produces the `> 1` and negative
    /// `BT.2020 → Bt709` components verbatim.
    #[test]
    fn from_working_rgba_f32_writes_the_unclamped_encode() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 0.0, 0.0, 1.0]], working_tag());
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let [r, g, b, a] = f32_px(&dst, 0, 0);
        assert_ulp(r, 1.281_715_274_558_044_2, ULP_BOUND);
        assert_ulp(g, -0.560_477_135_347_157_8, ULP_BOUND);
        assert_ulp(b, -0.081_678_435_097_073_43, ULP_BOUND);
        assert_ulp(a, 1.0, ULP_BOUND);
    }

    /// The identity construction (`Bt2020`/`Linear`) + `Rgba8` dst is
    /// the pure quantization path — `round(clamp(v, 0, 1)·255)` with
    /// HDR residuals (`2.0`) and negatives saturating.
    #[test]
    fn from_working_identity_ctor_is_pure_quantization() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.5, 0.25, 2.0, 1.0], [-0.5, 0.0, 1.0, 0.75]],
            working_tag(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt2020, TransferCharacteristics::Linear)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        // 0.5·255 = 127.5 → 128 (RHU); 0.25·255 = 63.75 → 64;
        // 2.0/−0.5 saturate; alpha quantizes like any channel
        // (0.75·255 = 191.25 → 191).
        let row = dst.plane(0).unwrap().row(0).unwrap();
        assert_eq!(&row[0..4], &[128, 64, 255, 255]);
        assert_eq!(&row[4..8], &[0, 0, 255, 191]);
    }

    /// `NaN` working-space channels reach `quantize8`, which folds
    /// `NaN` → code `0`: a `NaN` RGB channel also mixes `NaN` into
    /// the sibling outputs through the matrix's `0·NaN` terms (the
    /// documented non-finite mixing), while `NaN` alpha folds on its
    /// own channel only.
    #[test]
    fn from_working_nan_channels_quantize_to_zero() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[f32::NAN, 0.5, 0.25, 1.0], [0.5, 0.25, 0.125, f32::NAN]],
            working_tag(),
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt2020, TransferCharacteristics::Linear)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        // px0: `0·NaN` terms turn every RGB output `NaN` → all fold
        // to 0; finite alpha still quantizes normally → 255.
        assert_eq!(&row[0..4], &[0, 0, 0, 255]);
        // px1: RGB quantizes normally (128/64/32 RHU); `NaN` alpha
        // folds to 0.
        assert_eq!(&row[4..8], &[128, 64, 32, 0]);
    }

    /// The same identity construction + `RgbaF32` dst is a bit-exact
    /// passthrough for finite non-`−0.0` values (`−0.0` and NaN
    /// payloads may canonicalize) — identity matrix and identity
    /// encode preserve those `f32` bit patterns (also above `1.0`).
    #[test]
    fn from_working_identity_ctor_f32_passthrough() {
        let pixels = [[0.5, 2.0, -0.5, 0.75], [1.0, 0.0, 0.125, 1.0]];
        let src = rgba_f32_frame(2, 1, &pixels, working_tag());
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt2020, TransferCharacteristics::Linear)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(
            f32_px(&dst, 0, 0).map(f32::to_bits),
            pixels[0].map(f32::to_bits)
        );
        assert_eq!(
            f32_px(&dst, 1, 0).map(f32::to_bits),
            pixels[1].map(f32::to_bits)
        );
    }

    /// `Unspecified` constructor arguments resolve to `Bt709` under
    /// the default rule — and `dst.color` records the resolved values,
    /// never `Unspecified`.
    #[test]
    fn from_working_ctor_unspecified_resolves_to_bt709() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 1.0, 1.0, 1.0]], working_tag());
        let mut dst_default = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        FromWorking::new(
            ColorPrimaries::Unspecified,
            TransferCharacteristics::Unspecified,
        )
        .unwrap()
        .process(&src, &mut dst_default)
        .unwrap();
        let mut dst_explicit = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709)
            .unwrap()
            .process(&src, &mut dst_explicit)
            .unwrap();
        assert_eq!(
            dst_default.plane(0).unwrap().data(),
            dst_explicit.plane(0).unwrap().data()
        );
        assert_eq!(dst_default.color().primaries, ColorPrimaries::Bt709);
        assert_eq!(dst_default.color().transfer, TransferCharacteristics::Bt709);
    }

    /// Every known `primaries`/`transfer` variant constructs — the
    /// `_` rejection arms exist only for future `#[non_exhaustive]`
    /// variants (structurally reachable, review-verified per the test
    /// spec).
    #[test]
    fn from_working_ctor_accepts_every_known_variant() {
        for primaries in [
            ColorPrimaries::Unspecified,
            ColorPrimaries::Bt709,
            ColorPrimaries::Smpte170m,
            ColorPrimaries::Bt470Bg,
            ColorPrimaries::Bt2020,
            ColorPrimaries::P3D65,
        ] {
            for transfer in [
                TransferCharacteristics::Unspecified,
                TransferCharacteristics::Bt709,
                TransferCharacteristics::Smpte170m,
                TransferCharacteristics::Linear,
                TransferCharacteristics::Pq,
                TransferCharacteristics::Hlg,
            ] {
                FromWorking::new(primaries, transfer)
                    .unwrap_or_else(|e| panic!("{primaries:?} × {transfer:?} rejected: {e}"));
            }
        }
    }

    /// `dst` records the constructor-resolved interpretation and
    /// inherits `hdr`/`pts`/`duration` from `src`.
    #[test]
    fn from_working_records_metadata() {
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let hdr = HdrMetadata {
            mastering_display: None,
            content_light_level: Some(ContentLightLevel {
                max_cll: 1_000,
                max_fall: 400,
            }),
        };
        let mut src = rgba_f32_frame(1, 1, &[[0.5, 0.5, 0.5, 1.0]], working_tag());
        src.set_color(ColorInfo {
            hdr: Some(hdr),
            ..src.color()
        });
        src.set_duration(Some(duration));
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 1, 1, pts()).unwrap();
        // Scrub `dst`'s timestamps so the propagation below is
        // observable — `dst` would otherwise already hold `src`'s pts.
        dst.set_pts(MediaTime::ZERO);
        FromWorking::new(ColorPrimaries::P3D65, TransferCharacteristics::Hlg)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(
            dst.color(),
            ColorInfo {
                primaries: ColorPrimaries::P3D65,
                transfer: TransferCharacteristics::Hlg,
                matrix: MatrixCoefficients::Identity,
                range: ColorRange::Full,
                hdr: Some(hdr),
            }
        );
        assert_eq!(dst.pts(), src.pts());
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Format/dimension/shared-backing rejections all happen before
    /// any write — a failed call leaves `dst` untouched.
    #[test]
    fn from_working_invalid_inputs_are_rejected() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 1.0, 1.0, 1.0]], working_tag());
        let mut kernel =
            FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709).unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();

        // Non-`RgbaF32` sources.
        let src_u8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_u8, &mut dst),
            Err(Error::InvalidValue("src format"))
        );
        let src_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_f16, &mut dst),
            Err(Error::InvalidValue("src format"))
        );

        // Non-`Rgba8`/`RgbaF32` destinations.
        let mut dst_f16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_f16),
            Err(Error::InvalidValue("dst format"))
        );
        // The rejected `dst` is untouched — pixels, `ColorInfo`, pts.
        assert_eq!(dst_f16.plane(0).unwrap().data(), &[0; 8]);
        assert_eq!(dst_f16.color(), ColorInfo::default());
        assert_eq!(dst_f16.pts(), pts());
        let mut dst_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 1, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_yuv),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 1, pts()).unwrap();
        assert_eq!(
            kernel.process(&src, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );

        // A shared `dst` (clone still holding the `Arc` backing).
        let mut dst_shared = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let sharing_clone = dst_shared.clone();
        assert_eq!(
            kernel.process(&src, &mut dst_shared),
            Err(Error::InvalidValue("dst plane"))
        );
        assert_eq!(dst_shared.plane(0).unwrap().row(0).unwrap(), &[0; 4]);
        drop(sharing_clone);
        assert!(kernel.process(&src, &mut dst_shared).is_ok());

        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0; 4]);
    }

    /// A `stride > row_bytes` destination: conversion writes rows
    /// only — padding bytes keep their caller-chosen contents.
    #[test]
    fn from_working_padded_dst_stride_is_never_written() {
        // 2×2 `Rgba8` dst: row_bytes 8, padded stride 12 → usable
        // 12 + 8 = 20 bytes — row 0 at 0..8, inter-row padding at
        // 8..12, row 1 at 12..20.
        let src = rgba_f32_frame(
            2,
            2,
            &[
                [0.5, 0.5, 0.5, 1.0],
                [0.25, 0.25, 0.25, 1.0],
                [0.5, 0.5, 0.5, 1.0],
                [0.25, 0.25, 0.25, 1.0],
            ],
            working_tag(),
        );
        let mut dst = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            2,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(
                    vec![
                        0_u8, 0, 0, 0, 0, 0, 0, 0, 0xEE, 0xEE, 0xEE, 0xEE, 0, 0, 0, 0, 0, 0, 0, 0,
                    ]
                    .into_boxed_slice(),
                ),
                stride: 12,
            }],
        )
        .unwrap();
        FromWorking::new(ColorPrimaries::Bt2020, TransferCharacteristics::Linear)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        let data = dst.plane(0).unwrap().data();
        // 0.5·255 → 128, 0.25·255 → 64; padding bytes keep 0xEE.
        assert_eq!(&data[0..8], &[128, 128, 128, 255, 64, 64, 64, 255]);
        assert_eq!(&data[8..12], &[0xEE; 4]);
        assert_eq!(&data[12..20], &[128, 128, 128, 255, 64, 64, 64, 255]);
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_from_working_is_repeatable_and_deterministic() {
        let src = rgba_f32_frame(1, 1, &[[1.0, 0.5, 0.25, 0.75]], working_tag());
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut kernel =
            FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Pq).unwrap();
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
    fn rt_process_from_working_never_mutates_source() {
        let src = rgba_f32_frame(
            2,
            1,
            &[[0.25, 0.5, 0.75, 1.0], [1.5, 2.0, 2.5, 0.5]],
            working_tag(),
        );
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF32, 2, 1, pts()).unwrap();
        FromWorking::new(ColorPrimaries::Bt709, TransferCharacteristics::Bt709)
            .unwrap()
            .process(&src, &mut dst)
            .unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }
}
