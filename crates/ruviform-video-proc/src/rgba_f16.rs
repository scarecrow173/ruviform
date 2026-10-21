//! `Rgba8 ↔ RgbaF16` numeric-conversion kernels: [`Rgba8ToRgbaF16`] and
//! [`RgbaF16ToRgba8`] remap the channel code space between 8-bit integer
//! and IEEE 754 half-precision float (`binary16`), normalized to
//! `[0, 1]`.
//!
//! ## Byte layout
//!
//! `RgbaF16` pixels are 8 bytes: four little-endian `binary16` channels
//! in R, G, B, A order. The endianness is part of the format contract.
//!
//! ## Codec implementation
//!
//! The `binary16` codec is implemented in software on integer/`f64`
//! math: the primitive `f16` type is still `unstable` on
//! `*-pc-windows-msvc` targets (rust-lang/rust#116909), so it cannot be
//! used by a stable-MSRV crate that must compile there. The byte-level
//! contract is unaffected — [`f64_to_f16_bits`] reproduces IEEE
//! round-to-nearest-even encoding bit-for-bit (verified against the
//! platform `f16` conversion), and [`f16_bits_to_f64`] is exact.
//!
//! ## Value rules (plan open question Q3 defaults)
//!
//! - **u8 → binary16:** `v ↦ round_to_nearest_even(f64::from(v) / 255.0)`
//!   — the channel value divided by 255, computed in `f64`, rounded to
//!   the nearest `binary16` with ties to even — bit-identical to what
//!   `f16::from_f64(f64::from(v) / 255.0)` would produce. `0` → `0.0`
//!   (0x0000), `255` → `1.0` (0x3C00), `128` → `0.501953125` (0x3804).
//! - **binary16 → u8:** `v ↦ round(clamp(v, 0.0, 1.0) · 255)` with
//!   round-half-up (`f64::round` on the decoded value): `1.0` → `255`,
//!   `0.5` → `128`. Out-of-range inputs saturate: negative → `0`,
//!   `> 1.0` → `255`, `±∞` → `0`/`255`, and `NaN` → `0` (the NaN fold
//!   is explicit rather than relying on `f64::clamp` propagating NaN
//!   into the `as u8` cast, which would also land on `0`).
//!
//! Alpha is a channel like any other — no special-casing.
//!
//! ## `ColorInfo` and timestamps
//!
//! This is a pure numeric conversion: `dst`'s `ColorInfo` is set to
//! `src`'s unchanged — `matrix`/`range`/`primaries`/`transfer`/`hdr`
//! all carry through (ADR D3-a non-destructive carriage). `pts`/
//! `duration` propagate from `src` — a conversion never retimes the
//! frame (plan open question Q4).
//!
//! ## Equivalence contract
//!
//! Floating-point math is involved, so the scalar implementation is the
//! reference with an **ulp-bounded** contract per the baseline DSP/SIMD
//! decision:
//!
//! - Forward (`u8 → binary16`): this implementation emits the
//!   `binary16` nearest to `v/255` — the canonical, deterministic
//!   result. An optimized backend must produce values within **1 ulp
//!   (binary16 domain)** of it (a different-but-defensible rounding of
//!   `v/255` may legitimately land one step away).
//! - Inverse (`binary16 → u8`): **bit-exact** — after the documented
//!   clamp + round-half-up rule the output byte is determined by the
//!   `binary16` input bit pattern, and every conforming backend must
//!   emit the identical byte.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{PixelFormat, VideoFrame};

/// Canonical `u8 → binary16` map: the `binary16` nearest to `v/255`,
/// ties to even — bit-identical to `f16::from_f64(f64::from(v)/255.0)`
/// on `f16`-capable targets.
fn u8_to_f16_bits(v: u8) -> u16 {
    f64_to_f16_bits(f64::from(v) / 255.0)
}

/// Canonical `binary16 → u8` map: decode the bit pattern, clamp to
/// `[0, 1]` (`NaN` → `0`, `±∞` and out-of-range inputs saturate), scale
/// by 255, round half-up.
// `normalized * 255` is in `[0, 255]` and `.round()` makes it integral
// and non-negative — the cast can neither truncate nor lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f16_bits_to_u8(bits: u16) -> u8 {
    let decoded = f16_bits_to_f64(bits);
    // NaN folds to 0 before the `clamp` — `clamp` would propagate it
    // into the cast (still 0), but the explicit branch keeps the
    // documented rule visible rather than incidental.
    let normalized = if decoded.is_nan() {
        0.0
    } else {
        decoded.clamp(0.0, 1.0)
    };
    (normalized * 255.0).round() as u8
}

/// IEEE 754 binary16 (half-precision) encoder: converts a finite `f64`
/// to the nearest `binary16` bit pattern, rounding to nearest with ties
/// to even; overflows round to `±infinity`, sub-binary16-subnormal
/// magnitudes to `±0`, `NaN` to the canonical quiet NaN `0x7E00`.
/// Pure integer math after the `f64` bit decompose — deterministic.
fn f64_to_f16_bits(value: f64) -> u16 {
    if value.is_nan() {
        return 0x7E00;
    }
    let bits = value.to_bits();
    // u16: sign occupies bit 15 — the top bit shifted right by 48.
    let sign = u16::try_from((bits >> 48) & 0x8000).unwrap_or(0);
    let raw_exp = (bits >> 52) & 0x7FF;
    let frac = bits & 0x000F_FFFF_FFFF_FFFF;
    if raw_exp == 0x7FF {
        return sign | 0x7C00; // ±infinity
    }
    if raw_exp == 0 {
        return sign; // f64 subnormal: |v| < 2⁻¹⁰²² ≪ half of the
        // smallest binary16 subnormal → ±0
    }
    // Rebias the f64 exponent (bias 1023) to binary16 (bias 15).
    // `raw_exp` is 11 bits (≤ 0x7FF), so the `try_from` cannot fail.
    let e = i32::try_from(raw_exp).unwrap_or(0) - 1023 + 15;
    if e > 30 {
        return sign | 0x7C00; // |v| ≥ 2¹⁶ rounds past max normal → ±inf
    }
    if e <= 0 {
        // Binary16 subnormal: |v| = mant63 · 2^(e−43), i.e. keep
        // `43 − e` shifted-out bits rounded nearest-even. For
        // `e ≤ −11` the magnitude is below half the smallest
        // subnormal and the shift overflows — ±0 directly.
        let shift = 43 - e;
        if shift > 62 {
            return sign;
        }
        let mant63 = frac | 0x10_0000_0000_0000;
        // `shift ∈ [43, 62]` here; the cast is a widening of a bounded
        // value (mant63 ≤ 2⁵³ kept 10 bits ≪ u16::MAX after the shift).
        #[allow(clippy::cast_possible_truncation)]
        return sign | round_shift_even(mant63, u32::try_from(shift).unwrap_or(64)) as u16;
    }
    // Normal binary16: keep the top 10 mantissa bits, round the
    // remaining 42 nearest-even; a carry propagates into the exponent
    // (and from e = 30 into ±inf, matching IEEE rounding at 65520).
    let mut out = u64::try_from(e).unwrap_or(0) << 10 | frac >> 42;
    let rem = frac & 0x3FF_FFFF_FFFF;
    let halfway = 1_u64 << 41;
    if rem > halfway || (rem == halfway && (frac >> 42) & 1 == 1) {
        out += 1;
    }
    #[allow(clippy::cast_possible_truncation)]
    return sign | out as u16;
}

/// Round-to-nearest-even on the bits a right shift would drop:
/// `v >> shift` plus one when the dropped part exceeds `2^(shift−1)`,
/// or equals it with odd `kept`. `shift` must be in `1..=62`.
fn round_shift_even(v: u64, shift: u32) -> u64 {
    let kept = v >> shift;
    let rem = v & ((1_u64 << shift) - 1);
    let halfway = 1_u64 << (shift - 1);
    kept + u64::from(rem > halfway || (rem == halfway && kept & 1 == 1))
}

/// Exact `binary16 → f64` decode of a bit pattern (every `binary16`
/// value is exactly representable in `f64`; NaN stays NaN, `±inf` stay
/// infinite).
fn f16_bits_to_f64(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exp = i32::from((bits >> 10) & 0x1F);
    let mant = f64::from(bits & 0x3FF);
    match exp {
        // Subnormal: mant · 2⁻²⁴ — the power of two is exact in f64.
        0 => sign * mant * f64::powi(2.0, -24),
        31 => {
            if mant == 0.0 {
                sign * f64::INFINITY
            } else {
                f64::NAN
            }
        }
        // Normal: (1 + mant/1024) · 2^(e−15) = (1024 + mant) · 2^(e−25).
        e => sign * (1024.0 + mant) * f64::powi(2.0, e - 25),
    }
}

/// Prepared `Rgba8 → RgbaF16` conversion kernel — the two-phase API
/// shape of ADR D1-b: [`new`](Self::new) is the non-RT `prepare` step
/// (this conversion has no coefficients to bake — the object exists for
/// API-shape uniformity), and [`process`](Self::process) is the
/// allocation-free conversion step. See the module doc for the value,
/// byte-order, propagation and equivalence contracts.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::Rgba8ToRgbaF16;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
/// src.plane_mut(0).unwrap().data_mut().copy_from_slice(&[0, 128, 255, 255]);
/// let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts).unwrap();
///
/// let mut kernel = Rgba8ToRgbaF16::new();
/// kernel.process(&src, &mut dst).unwrap();
///
/// // 0 → 0.0 (0x0000), 128 → 0.501953125 (0x3804), 255 → 1.0 (0x3C00),
/// // little-endian byte order.
/// let row = dst.plane(0).unwrap().row(0).unwrap();
/// assert_eq!(&row[0..2], &[0x00, 0x00]);
/// assert_eq!(&row[2..4], &[0x04, 0x38]);
/// assert_eq!(&row[4..6], &[0x00, 0x3C]);
/// ```
#[derive(Debug)]
pub struct Rgba8ToRgbaF16;

impl Rgba8ToRgbaF16 {
    /// Builds the prepared kernel. No coefficients are needed — the
    /// value rule is fixed — so construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::Rgba8ToRgbaF16::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Converts `src` (`Rgba8`) into `dst` (`RgbaF16`) per the rules on
    /// the module doc.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `Rgba8`, `dst` is not
    /// `RgbaF16`, the frames disagree on dimensions, or `dst`'s backing
    /// is still shared (`dst.plane_mut(0)` → `None`). Every check runs
    /// before any write — a failed call never partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (exactly `width`×`height` pixel iterations over borrowed
    /// rows). Dropping `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        if src.format() != PixelFormat::Rgba8 {
            return Err(Error::InvalidValue("src format"));
        }
        if dst.format() != PixelFormat::RgbaF16 {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        let src_plane = src.plane(0).ok_or(Error::InvalidValue("src planes"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..src.height() {
                // These row borrows cannot fail: `y < height` —
                // `ok_or` keeps the contract panic-free regardless.
                let src_row = src_plane.row(y).ok_or(Error::InvalidValue("src planes"))?;
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                // `row_bytes` is `width * 4` (src) / `width * 8` (dst),
                // so exactly `width` pixel chunks pair up — padding
                // never included.
                for (src_px, dst_px) in src_row
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(dst_row.as_chunks_mut::<8>().0.iter_mut())
                {
                    for (channel, dst_ch) in
                        src_px.iter().zip(dst_px.as_chunks_mut::<2>().0.iter_mut())
                    {
                        dst_ch.copy_from_slice(&u8_to_f16_bits(*channel).to_le_bytes());
                    }
                }
            }
        }
        // Pure numeric conversion: `ColorInfo` propagates unchanged,
        // and a conversion never retimes the frame.
        dst.set_color(src_color);
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for Rgba8ToRgbaF16 {
    /// Equivalent to [`Rgba8ToRgbaF16::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

/// Prepared `RgbaF16 → Rgba8` conversion kernel — the inverse of
/// [`Rgba8ToRgbaF16`], same two-phase API shape. See the module doc for
/// the value, byte-order, propagation and equivalence contracts.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_video_core::{PixelFormat, VideoFrame};
/// use ruviform_video_proc::RgbaF16ToRgba8;
///
/// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
/// let mut src = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts).unwrap();
/// // [1.0, 0.5, 0.0, 1.0] as little-endian binary16 (0x3C00/0x3800/…).
/// let px: Vec<u8> = [0x3C00_u16, 0x3800, 0x0000, 0x3C00]
///     .iter()
///     .flat_map(|v| v.to_le_bytes())
///     .collect();
/// src.plane_mut(0).unwrap().data_mut().copy_from_slice(&px);
/// let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts).unwrap();
///
/// let mut kernel = RgbaF16ToRgba8::new();
/// kernel.process(&src, &mut dst).unwrap();
///
/// // 1.0 → 255; 0.5 → round(127.5) = 128 (round-half-up).
/// assert_eq!(dst.plane(0).unwrap().row(0).unwrap(), &[255, 128, 0, 255]);
/// ```
#[derive(Debug)]
pub struct RgbaF16ToRgba8;

impl RgbaF16ToRgba8 {
    /// Builds the prepared kernel. No coefficients are needed — the
    /// value rule is fixed — so construction cannot fail.
    ///
    /// **RT contract:** non-RT only by convention (no allocation or
    /// computation actually happens, but constructors are setup-path
    /// APIs).
    ///
    /// # Examples
    ///
    /// ```
    /// let kernel = ruviform_video_proc::RgbaF16ToRgba8::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Converts `src` (`RgbaF16`) into `dst` (`Rgba8`) per the rules on
    /// the module doc.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] — `src` is not `RgbaF16`, `dst` is not
    /// `Rgba8`, the frames disagree on dimensions, or `dst`'s backing
    /// is still shared (`dst.plane_mut(0)` → `None`). Every check runs
    /// before any write — a failed call never partially fills `dst`.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking and
    /// bounded (exactly `width`×`height` pixel iterations over borrowed
    /// rows). Dropping `src`/`dst` stays the caller's non-RT concern.
    pub fn process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()> {
        if src.format() != PixelFormat::RgbaF16 {
            return Err(Error::InvalidValue("src format"));
        }
        if dst.format() != PixelFormat::Rgba8 {
            return Err(Error::InvalidValue("dst format"));
        }
        if src.width() != dst.width() || src.height() != dst.height() {
            return Err(Error::InvalidValue("dimensions"));
        }
        let src_color = src.color();
        let src_plane = src.plane(0).ok_or(Error::InvalidValue("src planes"))?;
        {
            let mut dst_plane = dst.plane_mut(0).ok_or(Error::InvalidValue("dst plane"))?;
            for y in 0..src.height() {
                // These row borrows cannot fail: `y < height` —
                // `ok_or` keeps the contract panic-free regardless.
                let src_row = src_plane.row(y).ok_or(Error::InvalidValue("src planes"))?;
                let dst_row = dst_plane
                    .row_mut(y)
                    .ok_or(Error::InvalidValue("dst plane"))?;
                // `row_bytes` is `width * 8` (src) / `width * 4` (dst),
                // so exactly `width` pixel chunks pair up — padding
                // never included.
                for (src_px, dst_px) in src_row
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
                {
                    for (src_ch, dst_ch) in src_px.as_chunks::<2>().0.iter().zip(dst_px.iter_mut())
                    {
                        *dst_ch = f16_bits_to_u8(u16::from_le_bytes([src_ch[0], src_ch[1]]));
                    }
                }
            }
        }
        // Pure numeric conversion: `ColorInfo` propagates unchanged,
        // and a conversion never retimes the frame.
        dst.set_color(src_color);
        dst.set_pts(src.pts());
        dst.set_duration(src.duration());
        Ok(())
    }
}

impl Default for RgbaF16ToRgba8 {
    /// Equivalent to [`RgbaF16ToRgba8::new`] — non-RT.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
    use ruviform_video_core::{
        ColorInfo, ColorPrimaries, ColorRange, ContentLightLevel, HdrMetadata, MatrixCoefficients,
        PixelFormat, SharedPlane, TransferCharacteristics, VideoFrame,
    };

    use super::*;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    /// Builds an `Rgba8` frame of one row holding `pixels`.
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

    // binary16 patterns used below (little-endian when stored):
    // 0x0000 = 0.0, 0x3800 = 0.5, 0x3804 = 0.501953125, 0x3C00 = 1.0,
    // 0x3400 = 0.25, 0x3A00 = 0.75, 0x4000 = 2.0, 0x3BFF = 0.99951…
    // (largest binary16 below 1), 0x7C00 = +inf, 0xFC00 = −inf,
    // 0x7E00 = canonical quiet NaN, 0xB800 = −0.5, 0x8000 = −0.0,
    // 0x1419 = 0.001000404… (normal just above 1e-3), 0x0001 = 2⁻²⁴
    // (smallest subnormal), 0x03FF = largest subnormal, 0x7BFF = 65504
    // (largest normal).

    /// Edge values through the forward map: `0 → 0x0000`, `255 →
    /// 0x3C00` (1.0), `128 → 0x3804` — the nearest binary16 to
    /// `128/255` (0.501953125).
    #[test]
    fn edge_values_normalize_to_documented_f16() {
        let src = rgba8_frame(1, 1, &[[0, 128, 255, 255]]);
        let mut dst = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        Rgba8ToRgbaF16::new().process(&src, &mut dst).unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        assert_eq!(&row[0..2], &[0x00, 0x00]); // 0.0
        assert_eq!(&row[2..4], &[0x04, 0x38]); // 0.501953125
        assert_eq!(&row[4..6], &[0x00, 0x3C]); // 1.0
        assert_eq!(&row[6..8], &[0x00, 0x3C]);
    }

    /// The inverse map's documented rule: decode, clamp to [0,1], then
    /// round-half-up — `0.5 → 128` (127.5 rounds up); NaN, infinities
    /// and out-of-range inputs saturate instead of panicking.
    #[test]
    fn f16_edge_values_clamp_and_round() {
        let src = f16_frame(
            6,
            1,
            &[
                [0x3800, 0x0000, 0x3C00, 0x3C00], // 0.5 → 128, 0 → 0, 1 → 255
                [0x7E00, 0x0000, 0x0000, 0x3C00], // NaN → 0
                [0xB800, 0xFC00, 0x0000, 0x3C00], // −0.5 / −∞ → 0
                [0x4000, 0x7C00, 0x0000, 0x3C00], // 2.0 / +∞ → 255
                [0x8000, 0x1419, 0x3BFF, 0x3C00], // −0.0 → 0, 0.001·255 → 0, 0.99951·255 → 255
                [0x0001, 0x03FF, 0x7BFF, 0x3C00], // tiny subnormals → 0, 65504 → 255
            ],
        );
        let mut dst = VideoFrame::new_owned(PixelFormat::Rgba8, 6, 1, pts()).unwrap();
        RgbaF16ToRgba8::new().process(&src, &mut dst).unwrap();
        let row = dst.plane(0).unwrap().row(0).unwrap();
        assert_eq!(&row[0..4], &[128, 0, 255, 255]);
        assert_eq!(&row[4..8], &[0, 0, 0, 255]);
        assert_eq!(&row[8..12], &[0, 0, 0, 255]);
        assert_eq!(&row[12..16], &[255, 255, 0, 255]);
        assert_eq!(&row[16..20], &[0, 0, 255, 255]);
        assert_eq!(&row[20..24], &[0, 0, 255, 255]);
    }

    /// Round-trip through both kernels returns every u8 code value
    /// exactly — the forward map lands within 1 ulp of `v/255`, whose
    /// neighborhoods always round back to `v`.
    #[test]
    fn roundtrips_all_u8_codes_exactly() {
        let pixels: Vec<[u8; 4]> = (u8::MIN..=u8::MAX).map(|v| [v; 4]).collect();
        let src = rgba8_frame(256, 1, &pixels);
        let mut mid = VideoFrame::new_owned(PixelFormat::RgbaF16, 256, 1, pts()).unwrap();
        Rgba8ToRgbaF16::new().process(&src, &mut mid).unwrap();
        let mut back = VideoFrame::new_owned(PixelFormat::Rgba8, 256, 1, pts()).unwrap();
        RgbaF16ToRgba8::new().process(&mid, &mut back).unwrap();
        let row = back.plane(0).unwrap().row(0).unwrap();
        for (x, want) in pixels.iter().enumerate() {
            assert_eq!(&row[x * 4..x * 4 + 4], want, "pixel {x}");
        }
    }

    /// Format/dimension mismatches are all rejected before any write —
    /// for both kernels, in both directions.
    #[test]
    fn invalid_inputs_are_rejected() {
        let src8 = rgba8_frame(1, 1, &[[0, 0, 0, 255]]);
        let src16 = f16_frame(1, 1, &[[0x0000; 4]]);
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut dst16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        let mut forward = Rgba8ToRgbaF16::new();
        let mut backward = RgbaF16ToRgba8::new();

        // Swapped directions.
        assert_eq!(
            forward.process(&src16, &mut dst16),
            Err(Error::InvalidValue("src format"))
        );
        assert_eq!(
            backward.process(&src8, &mut dst8),
            Err(Error::InvalidValue("src format"))
        );

        // Same-format or planar destinations.
        assert_eq!(
            forward.process(&src8, &mut dst8),
            Err(Error::InvalidValue("dst format"))
        );
        assert_eq!(
            backward.process(&src16, &mut dst16),
            Err(Error::InvalidValue("dst format"))
        );
        let mut dst_yuv = VideoFrame::new_owned(PixelFormat::Yuv444p, 1, 1, pts()).unwrap();
        assert_eq!(
            forward.process(&src8, &mut dst_yuv),
            Err(Error::InvalidValue("dst format"))
        );

        // Dimension mismatch.
        let mut dst_wide = VideoFrame::new_owned(PixelFormat::RgbaF16, 2, 1, pts()).unwrap();
        assert_eq!(
            forward.process(&src8, &mut dst_wide),
            Err(Error::InvalidValue("dimensions"))
        );
    }

    /// A shared `dst` (clone still holding the `Arc` backing) must be
    /// rejected with no partial write — for both kernels.
    #[test]
    fn shared_dst_backing_is_rejected() {
        let src8 = rgba8_frame(1, 1, &[[255, 0, 0, 255]]);
        let mut dst16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        let sharing16 = dst16.clone();
        assert_eq!(
            Rgba8ToRgbaF16::new().process(&src8, &mut dst16),
            Err(Error::InvalidValue("dst plane"))
        );
        assert_eq!(dst16.plane(0).unwrap().data(), &[0_u8; 8]);
        drop(sharing16);

        let src16 = f16_frame(1, 1, &[[0x3C00; 4]]);
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let sharing8 = dst8.clone();
        assert_eq!(
            RgbaF16ToRgba8::new().process(&src16, &mut dst8),
            Err(Error::InvalidValue("dst plane"))
        );
        assert_eq!(dst8.plane(0).unwrap().data(), &[0_u8; 4]);
        drop(sharing8);
    }

    /// A `stride > row_bytes` destination: both kernels write rows
    /// only — padding bytes keep their caller-chosen contents.
    #[test]
    fn padded_dst_stride_is_never_written() {
        // Forward: 1×2 `RgbaF16` dst — row_bytes 8, padded stride 10 →
        // usable `10·(2−1) + 8` = 18 bytes; offsets 8–9 are inter-row
        // padding carrying sentinels.
        let src8 = rgba8_frame(1, 2, &[[128, 128, 128, 255], [128, 128, 128, 255]]);
        let mut backing16 = vec![0_u8; 18];
        backing16[8..10].copy_from_slice(&[0xEE, 0xEE]);
        let mut dst16 = VideoFrame::from_shared_planes(
            PixelFormat::RgbaF16,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(backing16.into_boxed_slice()),
                stride: 10,
            }],
        )
        .unwrap();
        assert_eq!(dst16.plane(0).unwrap().stride(), 10);
        Rgba8ToRgbaF16::new().process(&src8, &mut dst16).unwrap();
        // 128 → 0x3804 (LE `04 38`), 255 → 0x3C00 (LE `00 3C`): both
        // rows written, padding intact.
        assert_eq!(
            dst16.plane(0).unwrap().data(),
            &[
                0x04, 0x38, 0x04, 0x38, 0x04, 0x38, 0x00, 0x3C, // row 0
                0xEE, 0xEE, // inter-row padding
                0x04, 0x38, 0x04, 0x38, 0x04, 0x38, 0x00, 0x3C, // row 1
            ]
        );

        // Inverse: 1×2 `Rgba8` dst — row_bytes 4, padded stride 6 →
        // usable `6·(2−1) + 4` = 10 bytes; offsets 4–5 are padding.
        let src16 = f16_frame(1, 2, &[[0x3C00; 4], [0x3C00; 4]]);
        let mut backing8 = vec![0_u8; 10];
        backing8[4..6].copy_from_slice(&[0xEE, 0xEE]);
        let mut dst8 = VideoFrame::from_shared_planes(
            PixelFormat::Rgba8,
            1,
            2,
            pts(),
            vec![SharedPlane {
                data: Arc::from(backing8.into_boxed_slice()),
                stride: 6,
            }],
        )
        .unwrap();
        assert_eq!(dst8.plane(0).unwrap().stride(), 6);
        RgbaF16ToRgba8::new().process(&src16, &mut dst8).unwrap();
        // 1.0 → 255 on every channel: both rows written, padding intact.
        assert_eq!(
            dst8.plane(0).unwrap().data(),
            &[255, 255, 255, 255, 0xEE, 0xEE, 255, 255, 255, 255]
        );
    }

    /// Pure numeric conversion: the output `ColorInfo` is `src`'s
    /// unchanged (matrix/range and HDR fields all carry through), and
    /// the timestamps propagate.
    #[test]
    fn color_and_time_pass_through_unchanged() {
        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        let mut src = rgba8_frame(1, 1, &[[10, 20, 30, 40]]);
        src.set_color(ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Pq,
            matrix: MatrixCoefficients::Bt2020,
            range: ColorRange::Limited,
            hdr: Some(HdrMetadata {
                content_light_level: Some(ContentLightLevel {
                    max_cll: 1_000,
                    max_fall: 400,
                }),
                ..HdrMetadata::default()
            }),
        });
        src.set_duration(Some(duration));
        let mut mid = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        mid.set_pts(MediaTime::ZERO);
        Rgba8ToRgbaF16::new().process(&src, &mut mid).unwrap();
        assert_eq!(mid.color(), src.color());
        assert_eq!(mid.pts(), pts());
        assert_eq!(mid.duration(), Some(duration));

        let mut back = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        RgbaF16ToRgba8::new().process(&mid, &mut back).unwrap();
        assert_eq!(back.color(), src.color());
        assert_eq!(back.pts(), pts());
        assert_eq!(back.duration(), Some(duration));
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_to_f16_is_repeatable_and_deterministic() {
        let src8 = rgba8_frame(1, 1, &[[0, 128, 255, 7]]);
        let mut dst16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 1, 1, pts()).unwrap();
        let mut forward = Rgba8ToRgbaF16::new();
        for _ in 0..64 {
            forward.process(&src8, &mut dst16).unwrap();
            let row = dst16.plane(0).unwrap().row(0).unwrap();
            assert_eq!(&row[..2], &[0x00, 0x00]); // 0 → 0.0
            assert_eq!(&row[2..4], &[0x04, 0x38]); // 128 → 0.501953125
        }
    }

    /// RT contract: `process` is allocation-free, non-blocking and
    /// bounded — observably: repeated calls on the same kernel succeed
    /// and stay deterministic, never panic.
    #[test]
    fn rt_process_to_u8_is_repeatable_and_deterministic() {
        // [0.25, 0.5, 0.75, 1.0].
        let src16 = f16_frame(1, 1, &[[0x3400, 0x3800, 0x3A00, 0x3C00]]);
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 1, 1, pts()).unwrap();
        let mut backward = RgbaF16ToRgba8::new();
        for _ in 0..64 {
            backward.process(&src16, &mut dst8).unwrap();
            // 0.25·255 = 63.75 → 64; 0.5·255 = 127.5 → 128;
            // 0.75·255 = 191.25 → 191.
            assert_eq!(dst8.plane(0).unwrap().row(0).unwrap(), &[64, 128, 191, 255]);
        }
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// never allocates a new frame, and `src` stays byte-identical
    /// across the call (a borrow, not a drain).
    #[test]
    fn rt_process_to_f16_never_mutates_source() {
        let src8 = rgba8_frame(
            2,
            2,
            &[
                [1, 2, 3, 4],
                [5, 6, 7, 8],
                [9, 10, 11, 12],
                [13, 14, 15, 16],
            ],
        );
        let before: [u8; 16] = src8.plane(0).unwrap().data().try_into().unwrap();
        let mut dst16 = VideoFrame::new_owned(PixelFormat::RgbaF16, 2, 2, pts()).unwrap();
        Rgba8ToRgbaF16::new().process(&src8, &mut dst16).unwrap();
        assert_eq!(src8.plane(0).unwrap().data(), &before);
    }

    /// RT contract: `process` writes only the caller-provided `dst` —
    /// never allocates a new frame, and `src` stays byte-identical
    /// across the call (a borrow, not a drain).
    #[test]
    fn rt_process_to_u8_never_mutates_source() {
        let src16 = f16_frame(2, 2, &[[0x2C66; 4], [0x3000; 4], [0x3400; 4], [0x3800; 4]]);
        let before: [u8; 32] = src16.plane(0).unwrap().data().try_into().unwrap();
        let mut dst8 = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 2, pts()).unwrap();
        RgbaF16ToRgba8::new().process(&src16, &mut dst8).unwrap();
        assert_eq!(src16.plane(0).unwrap().data(), &before);
    }
}
