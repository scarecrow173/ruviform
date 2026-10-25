//! Compositing and transition kernels on packed RGBA frames:
//! [`CompositeOver`] composites `src` over `dst` in place — Porter-Duff
//! source-over on straight alpha with a per-call uniform `opacity` —
//! and [`Crossfade`] dissolves between two source frames on all four
//! channels (`dst = from·(1−t) + to·t`).
//!
//! ## Formats
//!
//! Both kernels accept [`PixelFormat::Rgba8`],
//! [`PixelFormat::RgbaF16`] and [`PixelFormat::RgbaF32`] only, and
//! every frame in one call must share the format. Planar YUV formats
//! are out of scope: they carry no alpha, and compositing on a chroma
//! lattice is a separate design concern.
//!
//! ## Value rules
//!
//! Alpha is straight (unassociated) — the FFmpeg-style packed-RGBA
//! convention — and all math happens in the code space as-is: no
//! transfer linearization and no gamut mapping. `RgbaF32` frames from
//! the `BT.2020`/`Linear` working space are what close the #97
//! deferred item — compositing them *is* linear-light compositing,
//! while the kernels stay code-space math throughout.
//!
//! The per-call mixing parameters — `opacity` for [`CompositeOver`],
//! `t` for [`Crossfade`] — are validated (`is_finite()` and `[0, 1]`)
//! and quantized once per `process` call to Q8 weights
//! (`w = round(opacity·255)`, `tq = round(t·255)`); every pixel path
//! shares the quantized weight.
//!
//! ### `CompositeOver` (`src over dst`, in place)
//!
//! - `Rgba8` — integer only, round-half-up: effective source alpha
//!   `sa = round(a_s·w/255)`, backdrop term
//!   `b = round(a_b·(255−sa)/255)` computed once per pixel and reused
//!   by both output alpha `oa = sa + b` and color
//!   `oc = round((c_s·sa + c_b·b)/oa)` — the self-consistent Q8
//!   expansion of `out = (c_s·âs + c_b·âb·(1−âs))/ôa`. `c_s`, `c_b` ≤
//!   255 bound the numerator by `255·oa`, so `oc ≤ 255` is structural
//!   and never needs a clamp. `oa == 0` → `(0, 0, 0, 0)`.
//! - `RgbaF16` — the same formulas evaluated in `f32` and rounded to
//!   `binary16` (round-to-nearest-even). Only `w` is quantized:
//!   `sa = a_s·(w/255)`, `b = a_b·(1−sa)`, `oa = sa + b`,
//!   `oc = (c_s·sa + c_b·b)/oa` on the decoded continuous channel
//!   values — no 257-step re-quantization, which would lose HDR pixel
//!   precision. Channels are not clamped (HDR code values above 1.0 are
//!   preserved). `oa < 2⁻¹⁴` — zero and the `binary16` subnormal
//!   region, where the division is ill-conditioned — is cut from the
//!   equivalence contract; the defined output is `(0, 0, 0, 0)`.
//!   Non-finite channel values (NaN/±∞) propagate per `f32` semantics.
//! - `RgbaF32` — the same `f32` evaluation stored to `binary32` with
//!   no `binary16` rounding (no re-quantization at all beyond the Q8
//!   weight — the point of the linear working-space arm). `oa <
//!   2⁻¹²⁶` — zero and the `binary32` subnormal region, where the
//!   division is ill-conditioned — is cut from the equivalence
//!   contract the same way; the defined output is `(0, 0, 0, 0)`.
//!   Channels are not clamped (HDR and negative wide-gamut values
//!   are preserved) and non-finite values propagate per `f32`
//!   semantics.
//! - `w == 0` returns early — the quantized weight is what triggers:
//!   `opacity = 0`, or any `opacity < 0.5/255 ≈ 0.002` quantizing to
//!   `w = 0` — after all validation (shared `dst` included), before
//!   any write — leaving `dst` byte-identical: even transparent
//!   pixels keep their stored RGB.
//!
//! ### `Crossfade` (`from → to` dissolve)
//!
//! - `Rgba8` — `oc = round((a_c·(255−tq) + b_c·tq)/255)` on all four
//!   channels, alpha included: a straight-alpha lerp, not a
//!   premultiplied one (the two agree for opaque sources).
//! - `RgbaF16` — `oc = a_c·(1 − tq/255) + b_c·(tq/255)` evaluated in
//!   `f32` and rounded to `binary16` — same single-quantization rule:
//!   only `tq` is quantized, channel values stay continuous.
//! - `RgbaF32` — the same `f32` evaluation stored to `binary32` with
//!   no `binary16` rounding — again only `tq` is quantized.
//! - `tq = 0` reproduces `from` and `tq = 255` reproduces `to` —
//!   bit-exactly for `Rgba8`; for `RgbaF16`/`RgbaF32` finite channels
//!   are reproduced numerically: the `f32` add can canonicalize `−0.0`
//!   to `+0.0`, and NaN payloads may normalize (to `0x7E00` through
//!   the `binary16` write on `RgbaF16`). `from` and `to` may
//!   reference the same frame — both are immutable borrows — while
//!   `dst` must not share its backing.
//!
//! ## `ColorInfo` and timestamps
//!
//! [`CompositeOver`] is an in-place canvas update: `dst`'s
//! `ColorInfo`/`pts`/`duration` keep their pre-call values.
//! [`Crossfade`] propagates the first source's identity — `dst`
//! receives `from`'s `ColorInfo`/`pts`/`duration` (a consumer needing
//! `to`'s timing overwrites explicitly). `ColorInfo` agreement between
//! source frames is not checked — the math is code-space, so
//! interpretation consistency stays the consumer's responsibility.
//!
//! ## Equivalence contract
//!
//! These scalar implementations are the reference implementations per
//! the baseline DSP/SIMD decision:
//!
//! - `Rgba8` paths are integer-only — **bit-exact**: every conforming
//!   backend must emit identical bytes.
//! - `RgbaF16` paths are the documented `f32` pipeline + RNE `binary16`
//!   rounding — an optimized backend must stay within **2 ulp
//!   (`binary16` domain)** per channel. The `oa < 2⁻¹⁴` region and
//!   non-finite channel values are outside the contract.
//! - `RgbaF32` paths are the documented `f32` pipeline stored to
//!   `binary32` with no `binary16` rounding — the `f64` evaluation of
//!   the formulas rounded to `binary32` is the reference an optimized
//!   backend must match within **2 ulp (`binary32` domain)** per
//!   channel. The `oa < 2⁻¹²⁶` (`binary32` subnormal) region and
//!   non-finite channel values are outside the contract.
//!
//! Rows are processed through `Plane::row`/`PlaneMut::row_mut` only, so
//! `stride > row_bytes` padding is never read or written. `RgbaF16`
//! pixels are four little-endian `binary16` channels in R, G, B, A
//! order (the `rgba_f16.rs` byte-layout contract); `RgbaF32` pixels
//! are four little-endian `binary32` channels in the same order.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{PixelFormat, VideoFrame};

use crate::rgba_f16::{f16_bits_to_f64, f64_to_f16_bits};

/// Smallest normal `binary16` magnitude (`2⁻¹⁴` = `1/16384` exactly):
/// below this the source-over division is ill-conditioned, so the
/// defined output is transparent black — the same guard as
/// `oa == 0.0`. NaN fails the comparison and propagates per `f32`
/// semantics.
const F16_MIN_NORMAL: f32 = 1.0 / 16384.0;

/// Quantizes a validated `[0, 1]` mixing parameter to the Q8 weight
/// `round(v·255)` — run exactly once per `process` call so every
/// pixel path shares the weight.
// `v·255` is in `[0, 255]` after the callers' `is_finite`/`[0, 1]`
// validation and `.round()` makes it integral and non-negative — the
// cast can neither truncate nor lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn q8_weight(v: f32) -> u32 {
    (v * 255.0).round() as u32
}

/// The `f32`-domain weight `w/255` matching a quantized Q8 weight —
/// `w ≤ 255` is exactly representable in `f32`, so the cast loses
/// nothing.
#[allow(clippy::cast_precision_loss)]
fn q8_scale(w: u32) -> f32 {
    w as f32 / 255.0
}

/// Decodes one little-endian `binary16` channel pair (2 bytes of a
/// pixel chunk) to `f32` — every `binary16` value is exactly
/// representable in `f32`, so the `binary16 → f64 → f32` chain is
/// lossless.
// The `f64 → f32` narrowing cannot truncate here: the value came from
// a `binary16` bit pattern, all of which fit in `f32`.
#[allow(clippy::cast_possible_truncation)]
fn f16_channel(pair: &[u8]) -> f32 {
    f16_bits_to_f64(u16::from_le_bytes([pair[0], pair[1]])) as f32
}

/// Encodes one `f32` channel to a little-endian `binary16` pair — the
/// `f32 → f64` widening is exact, then [`f64_to_f16_bits`] applies IEEE
/// round-to-nearest-even.
fn f16_bytes(v: f32) -> [u8; 2] {
    f64_to_f16_bits(f64::from(v)).to_le_bytes()
}

/// Smallest normal `binary32` magnitude (`2⁻¹²⁶`): the `binary32`
/// counterpart of [`F16_MIN_NORMAL`] — below this the source-over
/// division is ill-conditioned, so the defined output is transparent
/// black. NaN fails the comparison and propagates per `f32` semantics.
const F32_MIN_NORMAL: f32 = f32::MIN_POSITIVE;

/// Decodes one little-endian `binary32` channel (4 bytes of a pixel
/// chunk) to `f32` — a bit-exact read of the `RgbaF32` byte layout.
fn f32_channel(quad: &[u8]) -> f32 {
    f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]])
}

/// Encodes one `f32` channel to a little-endian `binary32` quad — a
/// bit-exact write (no `binary16` rounding on the `RgbaF32` path).
fn f32_bytes(v: f32) -> [u8; 4] {
    v.to_le_bytes()
}

/// `Rgba8` source-over on one pixel pair — the self-consistent Q8
/// expansion documented on the module doc: `sa` and the backdrop term
/// `b` are computed once per pixel and reused by `oa` and every `oc`.
// `num ≤ 255·oa` structurally keeps `oc ≤ 255`, and `oa ≤ 255` —
// both `as u8` casts are lossless.
#[allow(clippy::cast_possible_truncation)]
fn composite_over_u8(src_px: &[u8], dst_px: &mut [u8], w: u32) {
    let a_s = u32::from(src_px[3]);
    let a_b = u32::from(dst_px[3]);
    // `round(n/255)` — denominator 255 is odd, so `.5` ties cannot
    // occur and `+127` is exact round-half-up.
    let sa = (a_s * w + 127) / 255;
    // Backdrop term `b = round(a_b·(255−sa)/255)` — once per pixel,
    // reused by `oa` and all three `oc` channels.
    let b = (a_b * (255 - sa) + 127) / 255;
    let oa = sa + b;
    if oa == 0 {
        dst_px.copy_from_slice(&[0_u8; 4]);
        return;
    }
    for ch in 0..3 {
        let num = u32::from(src_px[ch]) * sa + u32::from(dst_px[ch]) * b;
        // `oc = round(num/oa)` — `+oa/2` is exact round-half-up (for
        // odd `oa` a `.5` tie cannot occur, for even `oa` it is the
        // literal half divisor).
        dst_px[ch] = ((num + oa / 2) / oa) as u8;
    }
    dst_px[3] = oa as u8;
}

/// `RgbaF16` source-over on one pixel pair — the same formulas as
/// [`composite_over_u8`] evaluated in `f32`, then RNE-rounded to
/// `binary16`. `sa_scale` is the per-call quantized `w/255`.
// The reference formula is `c_s·sa + c_b·b` evaluated as written — a
// fused `mul_add` would change the rounding of the normative pipeline.
#[allow(clippy::suboptimal_flops)]
fn composite_over_f16(src_px: &[u8], dst_px: &mut [u8], sa_scale: f32) {
    let a_s = f16_channel(&src_px[6..8]);
    let a_b = f16_channel(&dst_px[6..8]);
    let sa = a_s * sa_scale;
    let b = a_b * (1.0 - sa);
    let oa = sa + b;
    // `oa == 0.0` and the `binary16` subnormal region: the division is
    // ill-conditioned there, so the defined output is transparent
    // black. NaN fails the comparison and propagates.
    if oa < F16_MIN_NORMAL {
        dst_px.copy_from_slice(&[0_u8; 8]);
        return;
    }
    for ch in 0..3 {
        let c_s = f16_channel(&src_px[ch * 2..ch * 2 + 2]);
        let c_b = f16_channel(&dst_px[ch * 2..ch * 2 + 2]);
        let oc = (c_s * sa + c_b * b) / oa;
        dst_px[ch * 2..ch * 2 + 2].copy_from_slice(&f16_bytes(oc));
    }
    dst_px[6..8].copy_from_slice(&f16_bytes(oa));
}

/// `RgbaF32` source-over on one pixel pair — the same formulas as
/// [`composite_over_f16`] evaluated in `f32` and stored to `binary32`
/// with no `binary16` rounding. `sa_scale` is the per-call quantized
/// `w/255`.
// The reference formula is `c_s·sa + c_b·b` evaluated as written — a
// fused `mul_add` would change the rounding of the normative pipeline.
#[allow(clippy::suboptimal_flops)]
fn composite_over_f32(src_px: &[u8], dst_px: &mut [u8], sa_scale: f32) {
    let a_s = f32_channel(&src_px[12..16]);
    let a_b = f32_channel(&dst_px[12..16]);
    let sa = a_s * sa_scale;
    let b = a_b * (1.0 - sa);
    let oa = sa + b;
    // `oa == 0.0` and the `binary32` subnormal region: the division is
    // ill-conditioned there, so the defined output is transparent
    // black — the `binary32` counterpart of the `oa < 2⁻¹⁴` cut on the
    // `RgbaF16` path. NaN fails the comparison and propagates.
    if oa < F32_MIN_NORMAL {
        dst_px.copy_from_slice(&[0_u8; 16]);
        return;
    }
    for ch in 0..3 {
        let c_s = f32_channel(&src_px[ch * 4..ch * 4 + 4]);
        let c_b = f32_channel(&dst_px[ch * 4..ch * 4 + 4]);
        let oc = (c_s * sa + c_b * b) / oa;
        dst_px[ch * 4..ch * 4 + 4].copy_from_slice(&f32_bytes(oc));
    }
    dst_px[12..16].copy_from_slice(&f32_bytes(oa));
}

/// `Rgba8` dissolve on one pixel triple — `oc = round((a_c·(255−tq) +
/// b_c·tq)/255)` on all four channels, integer round-half-up
/// (denominator 255 is odd — no `.5` ties can occur).
// `a·(255−tq) + b·tq ≤ 255·255` — `oc` is structurally in `0..=255`,
// so the `as u8` cast is lossless.
#[allow(clippy::cast_possible_truncation)]
fn crossfade_u8(a_px: &[u8], b_px: &[u8], dst_px: &mut [u8], tq: u32) {
    for ch in 0..4 {
        let num = u32::from(a_px[ch]) * (255 - tq) + u32::from(b_px[ch]) * tq;
        dst_px[ch] = ((num + 127) / 255) as u8;
    }
}

/// `RgbaF16` dissolve on one pixel triple — `a_c·(1 − t) + b_c·t` in
/// `f32` on all four channels, RNE-rounded to `binary16`. `t` is the
/// per-call quantized `tq/255`.
// Same normative-formula rule as `composite_over_f16` — no `mul_add`.
#[allow(clippy::suboptimal_flops)]
fn crossfade_f16(a_px: &[u8], b_px: &[u8], dst_px: &mut [u8], t: f32) {
    for ch in 0..4 {
        let a = f16_channel(&a_px[ch * 2..ch * 2 + 2]);
        let b = f16_channel(&b_px[ch * 2..ch * 2 + 2]);
        dst_px[ch * 2..ch * 2 + 2].copy_from_slice(&f16_bytes(a * (1.0 - t) + b * t));
    }
}

/// `RgbaF32` dissolve on one pixel triple — `a_c·(1 − t) + b_c·t` in
/// `f32` on all four channels, stored to `binary32` with no `binary16`
/// rounding. `t` is the per-call quantized `tq/255`.
// Same normative-formula rule as `crossfade_f16` — no `mul_add`.
#[allow(clippy::suboptimal_flops)]
fn crossfade_f32(a_px: &[u8], b_px: &[u8], dst_px: &mut [u8], t: f32) {
    for ch in 0..4 {
        let a = f32_channel(&a_px[ch * 4..ch * 4 + 4]);
        let b = f32_channel(&b_px[ch * 4..ch * 4 + 4]);
        dst_px[ch * 4..ch * 4 + 4].copy_from_slice(&f32_bytes(a * (1.0 - t) + b * t));
    }
}

/// Prepared `src over dst` compositing kernel — the two-phase API
/// shape of ADR D1-b extended to the composite family: [`new`](Self::new)
/// is the trivial non-RT `prepare` step (no coefficients to bake — the
/// same unit shape as [`Rgba8ToRgbaF16`](crate::Rgba8ToRgbaF16)), and
/// [`process`](Self::process) is the allocation-free compositing step
/// carrying the per-call animation parameter `opacity`. See the module
/// doc for the value, propagation and equivalence contracts.
///
/// `process` reads `dst` as the backdrop and rewrites it in place —
/// the natural shape for stacking NLE layer tracks bottom-up.
/// `opacity` is a uniform multiplier on the effective source alpha
/// (the consumption shape of the edit model's clip `opacity`).
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::CompositeOver;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// src.plane_mut(0).unwrap().data_mut().copy_from_slice(&[200, 100, 50, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// dst.plane_mut(0).unwrap().data_mut().copy_from_slice(&[10, 20, 30, 40]);
///
/// let mut kernel = CompositeOver::new();
/// kernel.process(&src, &mut dst, 1.0).unwrap();
///
/// // Opaque source (`a_s = 255`) overwrites the backdrop completely.
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[200, 100, 50, 255]);
/// ```
#[derive(Debug)]
pub struct CompositeOver;

impl CompositeOver {
    /// Builds the prepared kernel. No coefficients are needed — the
    /// value rule is fixed and `opacity` arrives per `process` call —
    /// so construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::CompositeOver::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Composites `src` over `dst` in place per the rules on the module
    /// doc — `dst` is both the backdrop and the output.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `opacity` is not finite or outside
    /// `[0, 1]` (`"opacity"`), `src` is not `Rgba8`/`RgbaF16`/`RgbaF32`
    /// (`"src format"`), `dst` has a different format (`"dst format"`),
    /// the frames disagree on dimensions (`"dimensions"`), or `dst`'s
    /// backing is still shared (`dst.plane_mut(0)` → `None`,
    /// `"dst plane"`). Every check runs before any write — a failed
    /// call never partially fills `dst`, and `w == 0` (`opacity = 0`
    /// or any `opacity < 0.5/255 ≈ 0.002` quantizing to `w = 0`)
    /// additionally returns early leaving `dst` byte-identical.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (exactly `width`×`height` pixel iterations over borrowed
    /// rows). Dropping `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame, opacity: f32) -> Result<()> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(Error::InvalidValue("opacity"));
        }
        // Quantize once per call — every pixel path shares `w`/`w/255`.
        let w = q8_weight(opacity);
        let sa_scale = q8_scale(w);
        match src.format() {
            PixelFormat::Rgba8 | PixelFormat::RgbaF16 | PixelFormat::RgbaF32 => {}
            _ => return Err(Error::InvalidValue("src format")),
        }
        if dst.format() != src.format() {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_plane = src.plane(0).ok_or(Error::InvalidValue("src plane"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            // `w == 0` early return — the quantized weight triggers:
            // `opacity = 0`, or any `opacity < 0.5/255 ≈ 0.002` —
            // after *all* validation (shared `dst` included), before
            // any write: `dst` keeps every byte, transparent pixels
            // included.
            if w == 0 {
                return Ok(());
            }
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
                            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                        {
                            composite_over_u8(src_px, dst_px, w);
                        }
                    }
                    PixelFormat::RgbaF16 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<8>().0.iter_mut())
                        {
                            composite_over_f16(src_px, dst_px, sa_scale);
                        }
                    }
                    PixelFormat::RgbaF32 => {
                        for (src_px, dst_px) in src_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            composite_over_f32(src_px, dst_px, sa_scale);
                        }
                    }
                    // Validated above — this arm only keeps the match
                    // panic-free on future `PixelFormat` variants.
                    _ => return Err(Error::InvalidValue("src format")),
                }
            }
        }
        // In-place canvas: `dst`'s `ColorInfo`/`pts`/`duration` are
        // deliberately untouched.
        Ok(())
    }
}

impl Default for CompositeOver {
    /// Equivalent to [`CompositeOver::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

/// Prepared two-source dissolve kernel — `dst = from·(1−t) + to·t` on
/// all four channels, the transition A→B at progress `t`. Same
/// composite-family two-phase shape as [`CompositeOver`]: trivial
/// non-RT [`new`](Self::new), allocation-free [`process`](Self::process)
/// carrying the per-call `t`. See the module doc for the value,
/// propagation and equivalence contracts.
///
/// `from` and `to` may reference the same frame (both are immutable
/// borrows); `dst` must be exclusively owned. `dst` receives `from`'s
/// `ColorInfo`/`pts`/`duration` — the first source owns the output's
/// identity.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::Crossfade;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut from = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// from.plane_mut(0).unwrap().data_mut().copy_from_slice(&[0, 0, 0, 255]);
/// let mut to = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// to.plane_mut(0).unwrap().data_mut().copy_from_slice(&[100, 150, 200, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
///
/// let mut kernel = Crossfade::new();
/// kernel.process(&from, &to, &mut dst, 0.5).unwrap();
///
/// // `tq = round(0.5·255) = 128`: `oc = round((a·127 + b·128)/255)`.
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[50, 75, 100, 255]);
/// ```
#[derive(Debug)]
pub struct Crossfade;

impl Crossfade {
    /// Builds the prepared kernel. No coefficients are needed — the
    /// value rule is fixed and `t` arrives per `process` call — so
    /// construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::Crossfade::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Dissolves `from` into `to` at progress `t` into `dst` per the
    /// rules on the module doc.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `t` is not finite or outside `[0, 1]`
    /// (`"t"`), `from` is not `Rgba8`/`RgbaF16`/`RgbaF32`
    /// (`"src format"`), `to` or `dst` disagree on the format
    /// (`"to format"`/`"dst format"`),
    /// the three frames disagree on dimensions (`"dimensions"`), or
    /// `dst`'s backing is still shared (`dst.plane_mut(0)` → `None`,
    /// `"dst plane"`). Every check runs before any write — a failed
    /// call never partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (exactly `width`×`height` pixel iterations over borrowed
    /// rows). Dropping `from`/`to`/`dst` stays the caller's non-RT
    /// concern.
    pub fn process(
        &mut self,
        from: &VideoFrame,
        to: &VideoFrame,
        dst: &mut VideoFrame,
        t: f32,
    ) -> Result<()> {
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err(Error::InvalidValue("t"));
        }
        // Quantize once per call — every pixel path shares `tq`/`tq/255`.
        let tq = q8_weight(t);
        let t_scale = q8_scale(tq);
        match from.format() {
            PixelFormat::Rgba8 | PixelFormat::RgbaF16 | PixelFormat::RgbaF32 => {}
            _ => return Err(Error::InvalidValue("src format")),
        }
        if to.format() != from.format() {
            return Err(Error::InvalidValue("to format"));
        }
        if dst.format() != from.format() {
            return Err(Error::InvalidValue("dst format"));
        }
        if from.width() != to.width()
            || from.height() != to.height()
            || from.width() != dst.width()
            || from.height() != dst.height()
        {
            return Err(Error::InvalidValue("dimensions"));
        }
        let from_color = from.color();
        let from_plane = from.plane(0).ok_or(Error::InvalidValue("src plane"))?;
        let to_plane = to.plane(0).ok_or(Error::InvalidValue("to plane"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..from.height() {
                // These row borrows cannot fail: `y < height` —
                // `ok_or` keeps the contract panic-free regardless.
                let from_row = from_plane.row(y).ok_or(Error::InvalidValue("src plane"))?;
                let to_row = to_plane.row(y).ok_or(Error::InvalidValue("to plane"))?;
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                match from.format() {
                    PixelFormat::Rgba8 => {
                        for ((a_px, b_px), dst_px) in from_row
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(to_row.as_chunks::<4>().0.iter())
                            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                        {
                            crossfade_u8(a_px, b_px, dst_px, tq);
                        }
                    }
                    PixelFormat::RgbaF16 => {
                        for ((a_px, b_px), dst_px) in from_row
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .zip(to_row.as_chunks::<8>().0.iter())
                            .zip(dst_row.as_chunks_mut::<8>().0.iter_mut())
                        {
                            crossfade_f16(a_px, b_px, dst_px, t_scale);
                        }
                    }
                    PixelFormat::RgbaF32 => {
                        for ((a_px, b_px), dst_px) in from_row
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .zip(to_row.as_chunks::<16>().0.iter())
                            .zip(dst_row.as_chunks_mut::<16>().0.iter_mut())
                        {
                            crossfade_f32(a_px, b_px, dst_px, t_scale);
                        }
                    }
                    // Validated above — this arm only keeps the match
                    // panic-free on future `PixelFormat` variants.
                    _ => return Err(Error::InvalidValue("src format")),
                }
            }
        }
        // Metadata: `dst` takes `from`'s identity — the first source
        // owns color interpretation and timing.
        dst.set_color(from_color);
        dst.set_pts(from.pts());
        dst.set_duration(from.duration());
        Ok(())
    }
}

impl Default for Crossfade {
    /// Equivalent to [`Crossfade::new`] — non-RT.
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

    /// Builds an `RgbaF32` frame of `pixels` — channels given as `f32`
    /// values (or `f32::from_bits` patterns), written little-endian.
    fn f32_frame(width: u32, height: u32, pixels: &[[f32; 4]]) -> VideoFrame {
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
                for (dst_ch, value) in dst_px.as_chunks_mut::<4>().0.iter_mut().zip(src_px.iter()) {
                    dst_ch.copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        frame
    }

    /// Reads the `x`-th `RgbaF32` pixel of row `y` back as `f32`
    /// values.
    fn f32_px(frame: &VideoFrame, x: usize, y: u32) -> [f32; 4] {
        let row = frame.plane(0).unwrap().row(y).unwrap();
        let mut out = [0.0_f32; 4];
        for (dst, ch) in out
            .iter_mut()
            .zip(row[x * 16..x * 16 + 16].as_chunks::<4>().0.iter())
        {
            *dst = f32::from_le_bytes(*ch);
        }
        out
    }

    /// Asserts `actual` is within `ulp` `binary32` steps of `expected`
    /// (the `f64` reference value rounded to `binary32`) — the
    /// `RgbaF32` arms' ≤ 2 ulp equivalence bound. Sign-magnitude bit
    /// distance is the ulp distance for the same-sign comparisons
    /// used here.
    // The `f64 → f32` narrowing is the contract's own rounding step.
    #[allow(clippy::cast_possible_truncation)]
    fn assert_ulp(actual: f32, expected: f64, ulp: u32) {
        let expected = expected as f32;
        assert!(
            actual.to_bits().abs_diff(expected.to_bits()) <= ulp,
            "expected {expected} ± {ulp} ulp, got {actual}",
        );
    }

    /// `w = 255` (`opacity = 1`): an opaque source overwrites `dst`
    /// completely (`sa = 255`, `b = 0`, `oc = c_s`); a transparent
    /// source keeps the backdrop (`sa = 0`, `b = a_b`, `oc = c_b`).
    #[test]
    fn composite_over_full_and_zero_source_alpha() {
        let src = rgba8_frame(2, 1, &[[200, 100, 50, 255], [200, 100, 50, 0]]);
        let mut dst = rgba8_frame(2, 1, &[[10, 20, 30, 40], [10, 20, 30, 40]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        assert_eq!(&row[0..4], &[200, 100, 50, 255]);
        assert_eq!(&row[4..8], &[10, 20, 30, 40]);
    }

    /// Trap vectors for the structural `oc ≤ 255` bound — both must
    /// emit exactly 255, never overflow: `a_s = 254`/`a_b = 127` gives
    /// `sa = 254`/`b = 0`/`oa = 254` (`oc = round(255·254/254)`), and
    /// `a_s = 1`/`a_b = 128` gives `sa = 1`/`b = 127`/`oa = 128` where
    /// `c_s = c_b = 255` reaches the `255·oa` numerator bound exactly.
    #[test]
    fn composite_over_oc_boundary_stays_in_range() {
        let src = rgba8_frame(2, 1, &[[255, 0, 0, 254], [255, 255, 255, 1]]);
        let mut dst = rgba8_frame(2, 1, &[[0, 0, 0, 127], [255, 255, 255, 128]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        // `oc = round(64770/254) = 255` — the u8 ceiling, not overflow.
        assert_eq!(&row[0..4], &[255, 0, 0, 254]);
        // `oc = round(32640/128) = 255` — numerator bound `255·oa`.
        assert_eq!(&row[4..8], &[255, 255, 255, 128]);
    }

    /// `oa == 0` guard in both formats: source and backdrop fully
    /// transparent → defined output `(0, 0, 0, 0)`, no division.
    #[test]
    fn composite_over_zero_alpha_writes_transparent_black() {
        let src = rgba8_frame(1, 1, &[[1, 2, 3, 0]]);
        let mut dst = rgba8_frame(1, 1, &[[4, 5, 6, 0]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[0, 0, 0, 0]);

        // `RgbaF16`: `a_s = a_b = 0.0` → `oa = 0.0` — same guard.
        let src16 = f16_frame(1, 1, &[[0x3C00, 0x3C00, 0x3C00, 0x0000]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00, 0x3C00, 0x3C00, 0x0000]]);
        CompositeOver::new()
            .process(&src16, &mut dst16, 1.0)
            .unwrap();
        assert_eq!(f16_px(&dst16, 0, 0), [0x0000; 4]);
    }

    /// `w == 0` (`opacity = 0`) early return — after validation, before
    /// any write: `dst` stays byte-identical, including a transparent
    /// pixel whose RGB the formula would canonicalize to `(0,0,0,0)`.
    /// The quantized weight is the trigger: a tiny positive `opacity`
    /// below `0.5/255 ≈ 0.002` also yields `w = 0`.
    #[test]
    fn composite_over_zero_opacity_leaves_dst_byte_identical() {
        let src = rgba8_frame(1, 2, &[[255, 255, 255, 255], [255, 255, 255, 255]]);
        let mut dst = rgba8_frame(1, 2, &[[9, 8, 7, 0], [10, 20, 30, 40]]);
        let before = dst.plane(0).unwrap().data().to_vec();
        CompositeOver::new().process(&src, &mut dst, 0.0).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), &before[..]);
        // `0.001·255 ≈ 0.26` quantizes to `w = 0` — same early return.
        CompositeOver::new().process(&src, &mut dst, 0.001).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), &before[..]);
    }

    /// `opacity` non-finite or outside `[0, 1]` →
    /// `InvalidValue("opacity")`, `dst` untouched.
    #[test]
    fn composite_over_rejects_out_of_range_opacity() {
        let src = rgba8_frame(1, 1, &[[255, 0, 0, 255]]);
        let mut dst = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let mut kernel = CompositeOver::new();
        for bad in [f32::NAN, -0.5, 1.5, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                kernel.process(&src, &mut dst, bad),
                Err(Error::InvalidValue("opacity")),
                "opacity {bad}"
            );
        }
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[10, 20, 30, 40]);
    }

    /// Validation precedes the `w == 0` early return: a shared `dst`
    /// backing is rejected even when `opacity = 0` would write nothing.
    #[test]
    fn composite_over_shared_dst_rejected_before_opacity_early_return() {
        let src = rgba8_frame(1, 1, &[[255, 0, 0, 255]]);
        let mut dst = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let sharing = dst.clone();
        assert_eq!(
            CompositeOver::new().process(&src, &mut dst, 0.0),
            Err(Error::InvalidValue("dst plane"))
        );
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[10, 20, 30, 40]);
        drop(sharing);
    }

    /// Format and dimension mismatches and shared backings are all
    /// rejected before any write.
    #[test]
    fn composite_over_rejects_invalid_frames() {
        let src = rgba8_frame(1, 1, &[[255, 0, 0, 255]]);
        let src16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let mut kernel = CompositeOver::new();

        // Non-RGBA source.
        let src_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, pts()).unwrap();
        assert_eq!(
            kernel.process(&src_yuv, &mut dst, 1.0),
            Err(Error::InvalidValue("src format"))
        );

        // src↔dst format disagreement, in either direction.
        let mut dst16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        assert_eq!(
            kernel.process(&src, &mut dst16, 1.0),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&src16, &mut dst, 1.0),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = rgba8_frame(2, 1, &[[0; 4], [0; 4]]);
        assert_eq!(
            kernel.process(&src, &mut dst_wide, 1.0),
            Err(Error::InvalidValue("dimensions"))
        );

        // `dst` sharing its backing — with an unrelated clone or with
        // `src` itself — is rejected.
        let sharing = dst.clone();
        assert_eq!(
            kernel.process(&src, &mut dst, 1.0),
            Err(Error::InvalidValue("dst plane"))
        );
        drop(sharing);
        let mut dst_aliasing_src = src.clone();
        assert_eq!(
            kernel.process(&src, &mut dst_aliasing_src, 1.0),
            Err(Error::InvalidValue("dst plane"))
        );

        // No rejection wrote anything.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[10, 20, 30, 40]);
    }

    /// Mid-opacity golden — `opacity = 0.5` quantizes to `w = 128`:
    /// `sa = round(128·128/255) = 64`, `b = round(200·191/255) = 150`,
    /// `oa = 214`, then `oc = round((c_s·64 + c_b·150)/214)` per
    /// channel.
    #[test]
    fn composite_over_mid_opacity_golden() {
        let src = rgba8_frame(1, 1, &[[100, 200, 50, 128]]);
        let mut dst = rgba8_frame(1, 1, &[[40, 80, 160, 200]]);
        CompositeOver::new().process(&src, &mut dst, 0.5).unwrap();
        // r: round(12 400/214) = 58, g: round(24 800/214) = 116,
        // b: round(27 200/214) = 127, a: 214.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[58, 116, 127, 214]);
    }

    /// `RgbaF16` golden — `opacity = 1` (`w/255 = 1.0`): `sa = 0.5`,
    /// `b = 0.25`, `oa = 0.75`; every channel lands exactly on a
    /// `binary16` value: `oc_r = (0.25·0.5 + 1.0·0.25)/0.75 = 0.5`,
    /// `oc_g = (0.5·0.5 + 0.5·0.25)/0.75 = 0.5`,
    /// `oc_b = (1.0·0.5 + 0.25·0.25)/0.75 = 0.75`.
    #[test]
    fn composite_over_f16_golden() {
        let src = f16_frame(1, 1, &[[0x3400, 0x3800, 0x3C00, 0x3800]]);
        let mut dst = f16_frame(1, 1, &[[0x3C00, 0x3800, 0x3400, 0x3800]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x3800, 0x3800, 0x3A00, 0x3A00]);
    }

    /// Channels are not clamped — HDR code values above 1.0 pass
    /// through the compositing math.
    #[test]
    fn composite_over_f16_preserves_hdr_channels() {
        // Opaque source with `c_r = 2.0` → `oc_r = 2.0` (0x4000).
        let src = f16_frame(1, 1, &[[0x4000, 0x3C00, 0x3C00, 0x3C00]]);
        let mut dst = f16_frame(1, 1, &[[0x0000; 4]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x4000, 0x3C00, 0x3C00, 0x3C00]);
    }

    /// `oa` in the `binary16` subnormal region (`< 2⁻¹⁴`) is
    /// ill-conditioned — the defined output is `(0, 0, 0, 0)`, the same
    /// guard as `oa == 0.0` (a negative `oa` takes the same branch).
    #[test]
    fn composite_over_f16_denormal_alpha_writes_transparent_black() {
        // `a_s = 2⁻²⁴` (smallest `binary16` subnormal) → `oa ≈ 6e-8`;
        // `a_s = −0.5` with `a_b = 0` → `oa = −0.5`.
        let src = f16_frame(
            1,
            2,
            &[
                [0x3C00, 0x3C00, 0x3C00, 0x0001],
                [0x3C00, 0x3C00, 0x3C00, 0xB800],
            ],
        );
        let mut dst = f16_frame(
            1,
            2,
            &[
                [0x3C00, 0x3C00, 0x3C00, 0x0000],
                [0x3C00, 0x3C00, 0x3C00, 0x0000],
            ],
        );
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x0000; 4]);
        assert_eq!(f16_px(&dst, 0, 1), [0x0000; 4]);
    }

    /// Non-finite channel values propagate per `f32` semantics — they
    /// are input data, not validated parameters.
    #[test]
    fn composite_over_f16_propagates_non_finite_channels() {
        // `a_s = NaN` → `sa`/`oa` NaN → every channel NaN (0x7E00).
        // `c_r = +∞` with `a_s = 1.0` → `oc_r = +∞` (0x7C00).
        let src = f16_frame(
            2,
            1,
            &[
                [0x3C00, 0x3C00, 0x3C00, 0x7E00],
                [0x7C00, 0x0000, 0x0000, 0x3C00],
            ],
        );
        let mut dst = f16_frame(2, 1, &[[0x0000; 4], [0x0000; 4]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x7E00; 4]);
        assert_eq!(f16_px(&dst, 1, 0), [0x7C00, 0x0000, 0x0000, 0x3C00]);
    }

    /// `RgbaF32` acceptance + golden — `opacity = 1` (`w/255 = 1.0`):
    /// the same `f32` formulas as `RgbaF16` with no `binary16`
    /// rounding. `a_s = 1.0` overwrites completely; `a_s = 0.5`/
    /// `a_b = 0.5` gives `sa = 0.5`, `b = 0.25`, `oa = 0.75` with
    /// every channel landing exactly (`oc = (0.375, 0.375, 0.5625)/
    /// 0.75`); `a_s = 0.0` keeps the backdrop.
    #[test]
    fn composite_over_f32_golden() {
        let src = f32_frame(
            1,
            3,
            &[
                [1.0, 0.0, 0.5, 1.0],
                [0.25, 0.5, 1.0, 0.5],
                [0.7, 0.7, 0.7, 0.0],
            ],
        );
        let mut dst = f32_frame(
            1,
            3,
            &[
                [0.0, 0.0, 0.0, 0.0],
                [1.0, 0.5, 0.25, 0.5],
                [0.1, 0.2, 0.3, 1.0],
            ],
        );
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f32_px(&dst, 0, 0), [1.0, 0.0, 0.5, 1.0]);
        assert_eq!(f32_px(&dst, 0, 1), [0.5, 0.5, 0.75, 0.75]);
        // `sa = 0`, `b = a_b = 1.0`, `oa = 1.0` → `oc = c_b` exactly.
        assert_eq!(f32_px(&dst, 0, 2), [0.1, 0.2, 0.3, 1.0]);
    }

    /// Equivalence bound — `opacity = 0.5` quantizes to `w = 128`
    /// (`sa_scale = 128/255` in `f32`): every channel must match the
    /// `f64` evaluation of `sa = a_s·(w/255)`, `b = a_b·(1−sa)`,
    /// `oc = (c_s·sa + c_b·b)/oa` rounded to `binary32` within
    /// ≤ 2 ulp.
    #[test]
    fn composite_over_f32_mid_opacity_within_ulp_bound() {
        let src = f32_frame(1, 1, &[[0.2, 0.9, 1.5, 0.6]]);
        let mut dst = f32_frame(1, 1, &[[0.8, 0.1, 0.3, 0.4]]);
        CompositeOver::new().process(&src, &mut dst, 0.5).unwrap();
        // `f64` reference evaluation of the documented formula — the
        // stored `f32` channels widened exactly.
        let w = 128.0_f64 / 255.0;
        let a_s = f64::from(0.6_f32);
        let a_b = f64::from(0.4_f32);
        let sa = a_s * w;
        let b = a_b * (1.0 - sa);
        let oa = sa + b;
        let got = f32_px(&dst, 0, 0);
        for (ch, (c_s, c_b)) in got[..3].iter().zip([
            (f64::from(0.2_f32), f64::from(0.8_f32)),
            (f64::from(0.9_f32), f64::from(0.1_f32)),
            (f64::from(1.5_f32), f64::from(0.3_f32)),
        ]) {
            assert_ulp(*ch, (c_s * sa + c_b * b) / oa, 2);
        }
        assert_ulp(got[3], oa, 2);
    }

    /// `oa` outside the `binary32` normal range — zero, subnormal
    /// (`< 2⁻¹²⁶`) or negative — takes the defined transparent-black
    /// output, the `binary32` counterpart of the `oa < 2⁻¹⁴` cut on
    /// the `RgbaF16` path.
    #[test]
    fn composite_over_f32_out_of_range_alpha_writes_transparent_black() {
        let src = f32_frame(
            1,
            3,
            &[
                [1.0, 1.0, 1.0, 0.0],
                // Smallest `binary32` subnormal `a_s` ≈ 1.4e-45.
                [1.0, 1.0, 1.0, f32::from_bits(1)],
                [1.0, 1.0, 1.0, -0.5],
            ],
        );
        let mut dst = f32_frame(1, 3, &[[0.5, 0.5, 0.5, 0.0]; 3]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        for y in 0..3 {
            assert_eq!(f32_px(&dst, 0, y).map(f32::to_bits), [0; 4], "row {y}");
        }
    }

    /// The `oa < 2⁻¹⁴` cut is `RgbaF16`-specific: `a_s = 2⁻²⁰` sits in
    /// the divergence window `[2⁻¹²⁶, 2⁻¹⁴)` — a `binary16` subnormal
    /// but a normal `binary32` — so the same input writes transparent
    /// black on `RgbaF16` while `RgbaF32` composites it normally.
    #[test]
    fn composite_over_f32_faint_alpha_diverges_from_f16_cut() {
        // `2⁻²⁰` as a `binary16` subnormal (0x0010) → `oa < 2⁻¹⁴` →
        // the defined transparent black.
        let src16 = f16_frame(1, 1, &[[0x3C00, 0x3C00, 0x3C00, 0x0010]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00, 0x3C00, 0x3C00, 0x0000]]);
        CompositeOver::new()
            .process(&src16, &mut dst16, 1.0)
            .unwrap();
        assert_eq!(f16_px(&dst16, 0, 0), [0x0000; 4]);

        // The same `a_s` as a normal `binary32` (0x3580_0000) →
        // `oa ≥ 2⁻¹²⁶` → composites: `oc = c_s`, `oa = 2⁻²⁰`.
        let src32 = f32_frame(1, 1, &[[1.0, 1.0, 1.0, f32::from_bits(0x3580_0000)]]);
        let mut dst32 = f32_frame(1, 1, &[[1.0, 1.0, 1.0, 0.0]]);
        CompositeOver::new()
            .process(&src32, &mut dst32, 1.0)
            .unwrap();
        assert_eq!(
            f32_px(&dst32, 0, 0),
            [1.0, 1.0, 1.0, f32::from_bits(0x3580_0000)]
        );
    }

    /// Non-finite channel values propagate per `f32` semantics — they
    /// are input data, not validated parameters.
    #[test]
    fn composite_over_f32_propagates_non_finite_channels() {
        // `a_s = NaN` → `sa`/`oa` NaN → every channel NaN.
        // `c_r = +∞` with `a_s = 1.0` → `oc_r = +∞`.
        let src = f32_frame(
            2,
            1,
            &[[1.0, 1.0, 1.0, f32::NAN], [f32::INFINITY, 0.0, 0.0, 1.0]],
        );
        let mut dst = f32_frame(2, 1, &[[0.0; 4]; 2]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert!(f32_px(&dst, 0, 0).iter().all(|c| c.is_nan()));
        assert_eq!(f32_px(&dst, 1, 0), [f32::INFINITY, 0.0, 0.0, 1.0]);
    }

    /// Channels are not clamped — HDR code values above 1.0 and
    /// negative wide-gamut remnants pass through the compositing math
    /// (the point of the unclamped `f32` arm).
    #[test]
    fn composite_over_f32_preserves_unclamped_channels() {
        // Opaque source with `c_r = 2.0`, `c_g = −0.25` → passthrough.
        let src = f32_frame(1, 1, &[[2.0, -0.25, 0.5, 1.0]]);
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f32_px(&dst, 0, 0), [2.0, -0.25, 0.5, 1.0]);
    }

    /// `w == 0` early return on `RgbaF32` — `dst` stays byte-identical
    /// down to `−0.0`/subnormal/NaN-payload bits, and a shared backing
    /// is still rejected before the early return.
    #[test]
    fn composite_over_f32_zero_opacity_leaves_dst_byte_identical() {
        let src = f32_frame(1, 1, &[[1.0; 4]]);
        let mut dst = f32_frame(
            1,
            1,
            &[[
                f32::from_bits(0x8000_0000), // −0.0
                f32::from_bits(0x0000_0001), // smallest subnormal
                f32::from_bits(0x7FC0_0001), // NaN payload
                f32::from_bits(0x0000_0000),
            ]],
        );
        let before = dst.plane(0).unwrap().data().to_vec();
        CompositeOver::new().process(&src, &mut dst, 0.0).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), &before[..]);

        // Validation precedes the early return: a shared `dst` is
        // rejected even when `opacity = 0` would write nothing.
        let sharing = dst.clone();
        assert_eq!(
            CompositeOver::new().process(&src, &mut dst, 0.0),
            Err(Error::InvalidValue("dst plane"))
        );
        assert_eq!(dst.plane(0).unwrap().data(), &before[..]);
        drop(sharing);
    }

    /// `RgbaF32` sources are accepted, but every frame in the call
    /// must share the format — mixing with `Rgba8`/`RgbaF16` rejects
    /// `"dst format"` in either direction, and dimension mismatch on
    /// `RgbaF32` frames rejects `"dimensions"`, all before any write.
    #[test]
    fn composite_over_f32_rejects_mismatched_frames() {
        let src32 = f32_frame(1, 1, &[[1.0; 4]]);
        let src8 = rgba8_frame(1, 1, &[[0; 4]]);
        let mut dst32 = f32_frame(1, 1, &[[0.0; 4]]);
        let mut dst8 = rgba8_frame(1, 1, &[[0; 4]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst32_wide = f32_frame(2, 1, &[[0.0; 4]; 2]);
        let mut kernel = CompositeOver::new();

        assert_eq!(
            kernel.process(&src32, &mut dst8, 1.0),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&src32, &mut dst16, 1.0),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&src8, &mut dst32, 1.0),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&src32, &mut dst32_wide, 1.0),
            Err(Error::InvalidValue("dimensions"))
        );
        // No rejection wrote anything.
        assert_eq!(f32_px(&dst32, 0, 0), [0.0; 4]);
        assert_eq!(f32_px(&dst32_wide, 0, 0), [0.0; 4]);
    }

    /// In-place canvas semantics on `RgbaF32`: `dst`'s
    /// `ColorInfo`/`pts`/`duration` keep their pre-call values.
    #[test]
    fn composite_over_f32_keeps_dst_metadata() {
        let src = f32_frame(1, 1, &[[0.5; 4]]);
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        dst.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        dst.set_pts(MediaTime::ZERO);
        dst.set_duration(Some(duration));
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(f32_px(&dst, 0, 0), [0.5; 4]);
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), MediaTime::ZERO);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// In-place canvas semantics: `dst`'s `ColorInfo`/`pts`/`duration`
    /// keep their pre-call values — compositing never re-identifies
    /// the frame.
    #[test]
    fn composite_over_keeps_dst_metadata() {
        let src = rgba8_frame(1, 1, &[[200, 100, 50, 255]]);
        let mut dst = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        dst.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        dst.set_pts(MediaTime::ZERO);
        dst.set_duration(Some(duration));
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        // Pixels were composited; identity was not touched.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[200, 100, 50, 255]);
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), MediaTime::ZERO);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// Endpoint reproduction — `tq = 0` gives `dst` bit-identical to
    /// `from`, `tq = 255` to `to` (all four channels, alpha included).
    #[test]
    fn crossfade_endpoints_reproduce_sources() {
        let from = rgba8_frame(2, 1, &[[10, 20, 30, 40], [50, 60, 70, 80]]);
        let to = rgba8_frame(2, 1, &[[90, 100, 110, 120], [130, 140, 150, 160]]);
        let mut dst = rgba8_frame(2, 1, &[[0; 4], [0; 4]]);
        let mut kernel = Crossfade::new();
        kernel.process(&from, &to, &mut dst, 0.0).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), from.plane(0).unwrap().data());
        kernel.process(&from, &to, &mut dst, 1.0).unwrap();
        assert_eq!(dst.plane(0).unwrap().data(), to.plane(0).unwrap().data());
    }

    /// Mid-`t` golden vectors through the integer pipeline: `t = 0.5`
    /// → `tq = 128`, `t = 0.25` → `tq = 64`.
    #[test]
    fn crossfade_golden_quantized_weights() {
        let mut kernel = Crossfade::new();
        // `tq = 128`: `oc = round((a·127 + b·128)/255)`.
        let from = rgba8_frame(1, 1, &[[0, 0, 0, 255]]);
        let to = rgba8_frame(1, 1, &[[100, 150, 200, 255]]);
        let mut dst = rgba8_frame(1, 1, &[[0; 4]]);
        kernel.process(&from, &to, &mut dst, 0.5).unwrap();
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[50, 75, 100, 255]);

        // `tq = 64`: `oc = round((a·191 + b·64)/255)` — alpha mixes
        // like any channel: round((255·191 + 128·64)/255) = 223.
        let from = rgba8_frame(1, 1, &[[10, 100, 200, 255]]);
        let to = rgba8_frame(1, 1, &[[200, 50, 0, 128]]);
        kernel.process(&from, &to, &mut dst, 0.25).unwrap();
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[58, 87, 150, 223]);
    }

    /// `from` and `to` may be the same frame — both are immutable
    /// borrows; mixing a frame with itself reproduces it
    /// (`round(a·255/255) = a`). Only `dst`'s shared backing is
    /// rejected.
    #[test]
    fn crossfade_allows_same_frame_as_both_sources() {
        let frame = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let mut dst = rgba8_frame(1, 1, &[[0; 4]]);
        Crossfade::new()
            .process(&frame, &frame, &mut dst, 0.5)
            .unwrap();
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[10, 20, 30, 40]);

        // `dst` sharing its backing with a source is still rejected.
        let mut aliasing_dst = frame.clone();
        assert_eq!(
            Crossfade::new().process(&frame, &frame, &mut aliasing_dst, 0.5),
            Err(Error::InvalidValue("dst plane"))
        );
    }

    /// `t` non-finite or outside `[0, 1]` → `InvalidValue("t")`, `dst`
    /// untouched.
    #[test]
    fn crossfade_rejects_out_of_range_t() {
        let from = rgba8_frame(1, 1, &[[0; 4]]);
        let to = rgba8_frame(1, 1, &[[255; 4]]);
        let mut dst = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        let mut kernel = Crossfade::new();
        for bad in [f32::NAN, -0.5, 1.5, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                kernel.process(&from, &to, &mut dst, bad),
                Err(Error::InvalidValue("t")),
                "t {bad}"
            );
        }
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[10, 20, 30, 40]);
    }

    /// Format disagreements among `from`/`to`/`dst`, dimension
    /// mismatches and shared `dst` backings are rejected before any
    /// write.
    #[test]
    fn crossfade_rejects_invalid_frames() {
        let from = rgba8_frame(1, 1, &[[0; 4]]);
        let to = rgba8_frame(1, 1, &[[255; 4]]);
        let to16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst = rgba8_frame(1, 1, &[[9; 4]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut kernel = Crossfade::new();

        // Non-RGBA `from`.
        let from_yuv = VideoFrame::new_owned(PixelFormat::Yuv420p, 2, 2, pts()).unwrap();
        assert_eq!(
            kernel.process(&from_yuv, &to, &mut dst, 0.5),
            Err(Error::InvalidValue("src format"))
        );

        // `to` disagrees on format.
        assert_eq!(
            kernel.process(&from, &to16, &mut dst, 0.5),
            Err(Error::InvalidValue("to format"))
        );

        // `dst` disagrees on format.
        assert_eq!(
            kernel.process(&from, &to, &mut dst16, 0.5),
            Err(Error::InvalidValue("dst format"))
        );

        // `to`/`dst` dimension mismatches.
        let to_wide = rgba8_frame(2, 1, &[[0; 4], [0; 4]]);
        assert_eq!(
            kernel.process(&from, &to_wide, &mut dst, 0.5),
            Err(Error::InvalidValue("dimensions"))
        );
        let mut dst_wide = rgba8_frame(2, 1, &[[0; 4], [0; 4]]);
        assert_eq!(
            kernel.process(&from, &to, &mut dst_wide, 0.5),
            Err(Error::InvalidValue("dimensions"))
        );

        // Shared `dst` backing.
        let sharing = dst.clone();
        assert_eq!(
            kernel.process(&from, &to, &mut dst, 0.5),
            Err(Error::InvalidValue("dst plane"))
        );
        drop(sharing);

        // No rejection wrote anything.
        assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[9; 4]);
    }

    /// `RgbaF16` golden — `tq = 128`, `t = 128/255` evaluated in `f32`
    /// on continuous channel values: `oc_r = 1−t = 0.498…` → `0x37F8`,
    /// `oc_g = t = 0.50196…` → `0x3804` (the `binary16` nearest to
    /// `128/255`), equal `0.5` channels stay exact.
    #[test]
    fn crossfade_f16_golden() {
        let from = f16_frame(1, 1, &[[0x3C00, 0x0000, 0x3800, 0x3800]]);
        let to = f16_frame(1, 1, &[[0x0000, 0x3C00, 0x3800, 0x3800]]);
        let mut dst = f16_frame(1, 1, &[[0x0000; 4]]);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x37F8, 0x3804, 0x3800, 0x3800]);
    }

    /// `RgbaF16` endpoint reproduction is numeric, not bitwise: `tq = 0`
    /// reproduces finite `from` channels and `tq = 255` reproduces
    /// finite `to` channels, but `a·1.0 + b·0.0` canonicalizes `−0.0`
    /// to `+0.0` in `f32` (`−0.0 + +0.0 = +0.0`), and the NaN payload
    /// `0x7E01` folds to the canonical `0x7E00` on the way through
    /// `f64_to_f16_bits`.
    #[test]
    fn crossfade_f16_endpoints_normalize_zero_sign_and_nan_payload() {
        let mut kernel = Crossfade::new();
        let mut dst = f16_frame(1, 1, &[[0x0000; 4]]);

        // `tq = 0`: `1.0`/`2.0` reproduce bit-exactly; `−0.0` (0x8000)
        // becomes `+0.0` and the NaN payload `0x7E01` becomes `0x7E00`.
        let from = f16_frame(1, 1, &[[0x3C00, 0x8000, 0x7E01, 0x4000]]);
        let to = f16_frame(1, 1, &[[0x3800; 4]]);
        kernel.process(&from, &to, &mut dst, 0.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x3C00, 0x0000, 0x7E00, 0x4000]);

        // `tq = 255` is symmetric on `to`.
        let from = f16_frame(1, 1, &[[0x3400; 4]]);
        let to = f16_frame(1, 1, &[[0x3800, 0x8000, 0x7E01, 0x3C00]]);
        kernel.process(&from, &to, &mut dst, 1.0).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x3800, 0x0000, 0x7E00, 0x3C00]);
    }

    /// Non-finite channel values propagate per `f32` semantics — NaN in
    /// `from` and +∞ in `to` appear as non-finite results in `dst`.
    #[test]
    fn crossfade_f16_propagates_non_finite_channels() {
        let from = f16_frame(2, 1, &[[0x7E00, 0x0000, 0x0000, 0x3C00], [0x3C00; 4]]);
        let to = f16_frame(
            2,
            1,
            &[
                [0x0000, 0x0000, 0x0000, 0x3C00],
                [0x7C00, 0x0000, 0x0000, 0x3C00],
            ],
        );
        let mut dst = f16_frame(2, 1, &[[0x0000; 4], [0x0000; 4]]);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        assert_eq!(f16_px(&dst, 0, 0), [0x7E00, 0x0000, 0x0000, 0x3C00]);
        // +∞ in `to`'s R: `oc_r = 1·(1−t) + ∞·t = +∞`; `from`'s 1.0
        // G/B mix toward `to`'s 0.0 → `1−t = 0.498…` (0x37F8).
        assert_eq!(f16_px(&dst, 1, 0), [0x7C00, 0x37F8, 0x37F8, 0x3C00]);
    }

    /// `RgbaF32` acceptance + golden — `tq = 128`, `t = 128/255`
    /// evaluated in `f32` with no `binary16` rounding: each channel
    /// is the `f64` reference `a·(1 − 128/255) + b·(128/255)` rounded
    /// to `binary32` within ≤ 2 ulp.
    #[test]
    fn crossfade_f32_golden_within_ulp_bound() {
        let from = f32_frame(1, 1, &[[1.0, 0.25, -0.5, 0.75]]);
        let to = f32_frame(1, 1, &[[0.5, 0.75, 0.125, 0.25]]);
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        let t = 128.0_f64 / 255.0;
        let got = f32_px(&dst, 0, 0);
        for (ch, (a, b)) in got.iter().zip([
            (f64::from(1.0_f32), f64::from(0.5_f32)),
            (f64::from(0.25_f32), f64::from(0.75_f32)),
            (f64::from(-0.5_f32), f64::from(0.125_f32)),
            (f64::from(0.75_f32), f64::from(0.25_f32)),
        ]) {
            assert_ulp(*ch, a * (1.0 - t) + b * t, 2);
        }
    }

    /// `RgbaF32` endpoint reproduction is numeric, like `RgbaF16`:
    /// `tq = 0` reproduces finite `from` channels and `tq = 255`
    /// reproduces finite `to` channels, but `a·1.0 + b·0.0`
    /// canonicalizes `−0.0` to `+0.0` and NaN channels propagate with
    /// possibly-normalized payloads.
    #[test]
    fn crossfade_f32_endpoints_reproduce_finite_channels() {
        let mut kernel = Crossfade::new();
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);

        // `tq = 0`: `1.0`/`2.0`/`0.5` reproduce bit-exactly; `−0.0`
        // becomes `+0.0`; a NaN channel stays NaN.
        let from = f32_frame(1, 1, &[[1.0, -0.0, f32::from_bits(0x7FC0_0001), 2.0]]);
        let to = f32_frame(1, 1, &[[0.5; 4]]);
        kernel.process(&from, &to, &mut dst, 0.0).unwrap();
        let got = f32_px(&dst, 0, 0);
        assert_eq!(got[0].to_bits(), 1.0_f32.to_bits());
        assert_eq!(got[1].to_bits(), 0.0_f32.to_bits());
        assert!(got[2].is_nan());
        assert_eq!(got[3].to_bits(), 2.0_f32.to_bits());

        // `tq = 255` is symmetric on `to`.
        let from = f32_frame(1, 1, &[[0.25; 4]]);
        let to = f32_frame(1, 1, &[[0.5, -0.0, f32::from_bits(0x7FC0_0001), 1.0]]);
        kernel.process(&from, &to, &mut dst, 1.0).unwrap();
        let got = f32_px(&dst, 0, 0);
        assert_eq!(got[0].to_bits(), 0.5_f32.to_bits());
        assert_eq!(got[1].to_bits(), 0.0_f32.to_bits());
        assert!(got[2].is_nan());
        assert_eq!(got[3].to_bits(), 1.0_f32.to_bits());
    }

    /// Non-finite channel values propagate per `f32` semantics — NaN
    /// in `from` and +∞ in `to` appear as non-finite results in `dst`.
    /// Channels are not clamped: HDR and negative values mix through.
    #[test]
    fn crossfade_f32_propagates_non_finite_channels() {
        let from = f32_frame(2, 1, &[[f32::NAN, 0.0, 0.0, 1.0], [1.0; 4]]);
        let to = f32_frame(
            2,
            1,
            &[[0.0, 0.0, 0.0, 1.0], [f32::INFINITY, 0.0, 0.0, 1.0]],
        );
        let mut dst = f32_frame(2, 1, &[[0.0; 4]; 2]);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        assert!(f32_px(&dst, 0, 0)[0].is_nan());
        // +∞ in `to`'s R: `oc_r = 1·(1−t) + ∞·t = +∞`.
        assert_eq!(f32_px(&dst, 1, 0)[0], f32::INFINITY);
        // Alpha mixes like any channel — `1.0·(1−t) + 1.0·t = 1.0`.
        assert_eq!(f32_px(&dst, 1, 0)[3], 1.0);
    }

    /// `RgbaF32` sources are accepted, but `to`/`dst` must share the
    /// format — mixing with `Rgba8`/`RgbaF16` rejects before any
    /// write.
    #[test]
    fn crossfade_f32_rejects_mismatched_formats() {
        let from32 = f32_frame(1, 1, &[[0.0; 4]]);
        let to32 = f32_frame(1, 1, &[[1.0; 4]]);
        let to8 = rgba8_frame(1, 1, &[[255; 4]]);
        let mut dst8 = rgba8_frame(1, 1, &[[9; 4]]);
        let mut dst16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst32 = f32_frame(1, 1, &[[0.0; 4]]);
        let mut kernel = Crossfade::new();

        // `to` disagrees on format.
        assert_eq!(
            kernel.process(&from32, &to8, &mut dst32, 0.5),
            Err(Error::InvalidValue("to format"))
        );
        // `dst` disagrees on format.
        assert_eq!(
            kernel.process(&from32, &to32, &mut dst8, 0.5),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            kernel.process(&from32, &to32, &mut dst16, 0.5),
            Err(Error::InvalidValue("dst format"))
        );
        // No rejection wrote anything.
        assert_eq!(f32_px(&dst32, 0, 0), [0.0; 4]);
    }

    /// `dst` takes `from`'s `ColorInfo`/`pts`/`duration` on `RgbaF32`
    /// too — the first source owns the output identity, not `to`.
    #[test]
    fn crossfade_f32_propagates_from_metadata() {
        let mut from = f32_frame(1, 1, &[[0.0; 4]]);
        let from_pts = MediaTime::new(7, TimeBase::COMMON_90K);
        let duration = MediaDuration::new(9_000, TimeBase::COMMON_90K);
        from.set_pts(from_pts);
        from.set_duration(Some(duration));
        from.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        let to = f32_frame(1, 1, &[[1.0; 4]]);
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);
        dst.set_pts(MediaTime::ZERO);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), from_pts);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// `dst` takes `from`'s `ColorInfo`/`pts`/`duration` — the first
    /// source owns the output identity, not `to`.
    #[test]
    fn crossfade_propagates_from_metadata() {
        let mut from = rgba8_frame(1, 1, &[[0; 4]]);
        let from_pts = MediaTime::new(7, TimeBase::COMMON_90K);
        let duration = MediaDuration::new(9_000, TimeBase::COMMON_90K);
        from.set_pts(from_pts);
        from.set_duration(Some(duration));
        from.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        });
        // `to` carries different metadata (default color, `pts()` = 42)
        // — none of it may leak into `dst`.
        let to = rgba8_frame(1, 1, &[[255; 4]]);
        let mut dst = rgba8_frame(1, 1, &[[0; 4]]);
        dst.set_pts(MediaTime::ZERO);
        Crossfade::new().process(&from, &to, &mut dst, 0.5).unwrap();
        assert_eq!(dst.color().primaries, ColorPrimaries::Bt2020);
        assert_eq!(dst.pts(), from_pts);
        assert_eq!(dst.duration(), Some(duration));
    }

    /// `stride > row_bytes` destinations for both kernels: rows are
    /// written through `row_mut` only — inter-row padding keeps its
    /// caller-chosen contents.
    #[test]
    fn padded_dst_stride_is_never_written() {
        // 1×2 `Rgba8`: row_bytes 4, padded stride 6 → usable
        // `6·(2−1) + 4` = 10 bytes; offsets 4–5 are sentinels.
        let padded = || {
            VideoFrame::from_shared_planes(
                PixelFormat::Rgba8,
                1,
                2,
                pts(),
                vec![SharedPlane {
                    data: Arc::from(vec![0_u8, 0, 0, 0, 0xEE, 0xEE, 0, 0, 0, 0].into_boxed_slice()),
                    stride: 6,
                }],
            )
            .unwrap()
        };

        let src = rgba8_frame(1, 2, &[[9, 9, 9, 255], [9, 9, 9, 255]]);
        let mut dst = padded();
        assert_eq!(dst.plane(0).unwrap().stride(), 6);
        CompositeOver::new().process(&src, &mut dst, 1.0).unwrap();
        assert_eq!(
            dst.plane(0).unwrap().data(),
            &[9, 9, 9, 255, 0xEE, 0xEE, 9, 9, 9, 255]
        );

        let from = rgba8_frame(1, 2, &[[10, 10, 10, 255], [10, 10, 10, 255]]);
        let to = rgba8_frame(1, 2, &[[20, 20, 20, 255], [20, 20, 20, 255]]);
        let mut dst = padded();
        Crossfade::new().process(&from, &to, &mut dst, 1.0).unwrap();
        assert_eq!(
            dst.plane(0).unwrap().data(),
            &[20, 20, 20, 255, 0xEE, 0xEE, 20, 20, 20, 255]
        );
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic. (`CompositeOver` is
    /// in-place, so `dst` is restored between iterations.)
    #[test]
    fn rt_process_composite_over_is_repeatable_and_deterministic() {
        let src = rgba8_frame(1, 1, &[[100, 200, 50, 128]]);
        let mut dst = rgba8_frame(1, 1, &[[40, 80, 160, 200]]);
        let mut kernel = CompositeOver::new();
        for _ in 0..64 {
            dst.plane_mut(0)
                .unwrap()
                .data_mut()
                .copy_from_slice(&[40, 80, 160, 200]);
            kernel.process(&src, &mut dst, 0.5).unwrap();
            assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[58, 116, 127, 214]);
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// `src` stays byte-identical across the call (a borrow, not a
    /// drain).
    #[test]
    fn rt_process_composite_over_never_mutates_source() {
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
        CompositeOver::new().process(&src, &mut dst, 0.75).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_crossfade_is_repeatable_and_deterministic() {
        let from = rgba8_frame(1, 1, &[[0, 0, 0, 255]]);
        let to = rgba8_frame(1, 1, &[[100, 150, 200, 255]]);
        let mut dst = rgba8_frame(1, 1, &[[0; 4]]);
        let mut kernel = Crossfade::new();
        for _ in 0..64 {
            kernel.process(&from, &to, &mut dst, 0.5).unwrap();
            assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[50, 75, 100, 255]);
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// `from` and `to` both stay byte-identical across the call.
    #[test]
    fn rt_process_crossfade_never_mutates_sources() {
        let from = rgba8_frame(2, 1, &[[1, 2, 3, 4], [5, 6, 7, 8]]);
        let to = rgba8_frame(2, 1, &[[9, 10, 11, 12], [13, 14, 15, 16]]);
        let from_before: [u8; 8] = from.plane(0).unwrap().data().try_into().unwrap();
        let to_before: [u8; 8] = to.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = rgba8_frame(2, 1, &[[0; 4], [0; 4]]);
        Crossfade::new()
            .process(&from, &to, &mut dst, 0.25)
            .unwrap();
        assert_eq!(from.plane(0).unwrap().data(), &from_before);
        assert_eq!(to.plane(0).unwrap().data(), &to_before);
    }

    /// RT contract on the `RgbaF32` arm: `process` is allocation-free,
    /// non-blocking and bounded — observably: repeated calls on the
    /// same kernel succeed and stay deterministic, never panic.
    /// (`CompositeOver` is in-place, so `dst` is restored between
    /// iterations.)
    #[test]
    fn rt_process_composite_over_f32_is_repeatable_and_deterministic() {
        let src = f32_frame(1, 1, &[[0.2, 0.9, 1.5, 0.6]]);
        let mut dst = f32_frame(1, 1, &[[0.8, 0.1, 0.3, 0.4]]);
        let mut kernel = CompositeOver::new();
        kernel.process(&src, &mut dst, 0.5).unwrap();
        let expected = f32_px(&dst, 0, 0);
        for _ in 0..64 {
            dst.plane_mut(0)
                .unwrap()
                .data_mut()
                .as_chunks_mut::<16>()
                .0
                .iter_mut()
                .for_each(|px| *px = f32_px_bytes([0.8, 0.1, 0.3, 0.4]));
            kernel.process(&src, &mut dst, 0.5).unwrap();
            assert_eq!(f32_px(&dst, 0, 0), expected);
        }
    }

    /// Writes one `RgbaF32` pixel (four little-endian `binary32`
    /// channels) — the test-side writer for in-place restoration.
    fn f32_px_bytes(px: [f32; 4]) -> [u8; 16] {
        let mut out = [0_u8; 16];
        for (i, ch) in px.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&ch.to_le_bytes());
        }
        out
    }

    /// RT contract on the `RgbaF32` arm: `process` writes only the
    /// caller-provided `dst` — `src` stays byte-identical across the
    /// call.
    #[test]
    fn rt_process_composite_over_f32_never_mutates_source() {
        let src = f32_frame(2, 1, &[[1.0, -0.0, 0.25, 0.5], [0.125, 2.0, 0.0, 1.0]]);
        let before: [u8; 32] = src.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = f32_frame(2, 1, &[[0.0; 4]; 2]);
        CompositeOver::new().process(&src, &mut dst, 0.75).unwrap();
        assert_eq!(src.plane(0).unwrap().data(), &before);
    }

    /// RT contract on the `RgbaF32` arm: `process` is allocation-free,
    /// non-blocking and bounded — observably: repeated calls on the
    /// same kernel succeed and stay deterministic, never panic.
    #[test]
    fn rt_process_crossfade_f32_is_repeatable_and_deterministic() {
        let from = f32_frame(1, 1, &[[1.0, 0.25, -0.5, 0.75]]);
        let to = f32_frame(1, 1, &[[0.5, 0.75, 0.125, 0.25]]);
        let mut dst = f32_frame(1, 1, &[[0.0; 4]]);
        let mut kernel = Crossfade::new();
        kernel.process(&from, &to, &mut dst, 0.5).unwrap();
        let expected = f32_px(&dst, 0, 0);
        for _ in 0..64 {
            kernel.process(&from, &to, &mut dst, 0.5).unwrap();
            assert_eq!(f32_px(&dst, 0, 0), expected);
        }
    }

    /// RT contract on the `RgbaF32` arm: `process` writes only the
    /// caller-provided `dst` — `from` and `to` both stay
    /// byte-identical across the call.
    #[test]
    fn rt_process_crossfade_f32_never_mutates_sources() {
        let from = f32_frame(2, 1, &[[1.0, -0.0, 0.25, 0.5], [0.125, 2.0, 0.0, 1.0]]);
        let to = f32_frame(
            2,
            1,
            &[[0.0, 0.5, 1.5, 0.75], [f32::INFINITY, 0.0, 0.25, 0.5]],
        );
        let from_before: [u8; 32] = from.plane(0).unwrap().data().try_into().unwrap();
        let to_before: [u8; 32] = to.plane(0).unwrap().data().try_into().unwrap();
        let mut dst = f32_frame(2, 1, &[[0.0; 4]; 2]);
        Crossfade::new()
            .process(&from, &to, &mut dst, 0.25)
            .unwrap();
        assert_eq!(from.plane(0).unwrap().data(), &from_before);
        assert_eq!(to.plane(0).unwrap().data(), &to_before);
    }
}
