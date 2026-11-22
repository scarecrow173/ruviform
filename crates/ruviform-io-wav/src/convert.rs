//! Canonical `f32` → wire sample conversion — private to the backend.
//!
//! The declared deterministic policy (ADR
//! `issue-191-pcm-wav-output-backend-*`):
//!
//! - `NaN` → `0` (silence); checked before any scaling.
//! - `±Inf` and `|x| > 1.0` → saturates to the wire format's full
//!   scale (`x.clamp(-1.0, 1.0)` before scaling).
//! - Integer formats scale to full range and round half away from
//!   zero (`f32::round`): `S16` = `x * 32768`, `S24` = `x * 8388608`,
//!   `S32` = `x * 2147483648`, `U8` = `x * 128 + 128`. `-1.0` maps to
//!   the exact negative full scale; `+1.0` clamps to positive full
//!   scale minus one LSB.
//! - `F32` writes IEEE-754 little-endian verbatim (no clipping — a
//!   float WAV can carry out-of-range values).
//! - **No dither** — noise shaping is an upstream processing concern,
//!   deliberately out of backend scope.

use ruviform_audio_core::SampleFormat;

/// Converts one canonical `f32` sample to its wire representation and
/// appends the little-endian bytes to `out`.
///
/// `format` must be one of the formats accepted by
/// [`crate::wav::WIRE_FORMATS`]; anything else is a caller bug at the
/// `Wire` validation layer and writes nothing.
// The `f64` → int casts below are provably range-safe: `clipped` is
// clamped to `[-1.0, 1.0]` first, so every scaled+rounded value lands
// inside the destination type's range before the cast (Rust's `as`
// also saturates float→int). The lints can't see the clamp.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn write_sample(out: &mut Vec<u8>, format: SampleFormat, value: f32) {
    let clipped = if value.is_nan() {
        0.0_f64
    } else {
        f64::from(value).clamp(-1.0, 1.0)
    };
    match format {
        SampleFormat::U8 => {
            let sample = (clipped * 128.0 + 128.0).round().clamp(0.0, 255.0) as u8;
            out.push(sample);
        }
        SampleFormat::S16 => {
            let sample = (clipped * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16;
            out.extend_from_slice(&sample.to_le_bytes());
        }
        SampleFormat::S24 => {
            let sample = (clipped * 8_388_608.0)
                .round()
                .clamp(-8_388_608.0, 8_388_607.0) as i32;
            out.extend_from_slice(&sample.to_le_bytes()[..3]);
        }
        SampleFormat::S32 => {
            let sample = (clipped * 2_147_483_648.0)
                .round()
                .clamp(-2_147_483_648.0, 2_147_483_647.0) as i64;
            out.extend_from_slice(&(sample as i32).to_le_bytes());
        }
        SampleFormat::F32 => {
            // Float wire carries the value verbatim — no clipping.
            out.extend_from_slice(&value.to_le_bytes());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(format: SampleFormat, value: f32) -> Vec<u8> {
        let mut out = Vec::new();
        write_sample(&mut out, format, value);
        out
    }

    #[test]
    fn s16_full_scale_and_zero() {
        assert_eq!(bytes(SampleFormat::S16, 0.0), [0, 0]);
        assert_eq!(bytes(SampleFormat::S16, -1.0), (-32_768_i16).to_le_bytes());
        assert_eq!(bytes(SampleFormat::S16, 1.0), (32_767_i16).to_le_bytes());
        assert_eq!(bytes(SampleFormat::S16, 0.5), (16_384_i16).to_le_bytes());
    }

    #[test]
    fn s16_clips_and_nan() {
        assert_eq!(bytes(SampleFormat::S16, 2.0), (32_767_i16).to_le_bytes());
        assert_eq!(bytes(SampleFormat::S16, -2.0), (-32_768_i16).to_le_bytes());
        assert_eq!(
            bytes(SampleFormat::S16, f32::INFINITY),
            (32_767_i16).to_le_bytes()
        );
        assert_eq!(
            bytes(SampleFormat::S16, f32::NEG_INFINITY),
            (-32_768_i16).to_le_bytes()
        );
        assert_eq!(bytes(SampleFormat::S16, f32::NAN), [0, 0]);
    }

    #[test]
    fn s24_three_le_bytes() {
        assert_eq!(bytes(SampleFormat::S24, -1.0), [0x00, 0x00, 0x80]);
        assert_eq!(bytes(SampleFormat::S24, 1.0), [0xFF, 0xFF, 0x7F]);
        assert_eq!(bytes(SampleFormat::S24, 0.0), [0, 0, 0]);
    }

    #[test]
    fn u8_midpoint_and_ends() {
        assert_eq!(bytes(SampleFormat::U8, 0.0), [128]);
        assert_eq!(bytes(SampleFormat::U8, -1.0), [0]);
        assert_eq!(bytes(SampleFormat::U8, 1.0), [255]);
    }

    #[test]
    fn f32_passthrough_unclipped() {
        assert_eq!(bytes(SampleFormat::F32, 1.5), 1.5_f32.to_le_bytes());
        assert_eq!(bytes(SampleFormat::F32, f32::NAN), f32::NAN.to_le_bytes());
    }
}

/// Kani proof of the documented conversion policy — `write_sample` is
/// `pub(crate)`, so the harness lives in-source; `cargo kani --tests`
/// covers lib-target proofs (run via `mise run test-formal-kani`).
///
/// Slice-1 lesson applied: symbolic scalars (`value`, a format
/// selector) on a concrete empty sink — no symbolic containers, no
/// `format!` on any path. Verified on the slice-4 WSL2 box at
/// `unwind(16)` — see `evidence/rf806/slice4/`.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// For a symbolic `f32` and every `SampleFormat`: wire formats
    /// emit exactly `bits_per_sample / 8` little-endian bytes that
    /// satisfy the documented policy (`NaN` → silence code point,
    /// `>= 1.0` → positive full scale, `<= -1.0` → negative full
    /// scale, `F32` bit-verbatim); formats outside the wire set emit
    /// nothing.
    #[kani::proof]
    #[kani::unwind(16)]
    fn write_sample_documented_policy() {
        let sel: u8 = kani::any();
        let format = match sel % 10 {
            0 => SampleFormat::F32,
            1 => SampleFormat::F64,
            2 => SampleFormat::U8,
            3 => SampleFormat::S8,
            4 => SampleFormat::U16,
            5 => SampleFormat::S16,
            6 => SampleFormat::U24,
            7 => SampleFormat::S24,
            8 => SampleFormat::U32,
            _ => SampleFormat::S32,
        };
        let value: f32 = kani::any();
        let mut out = Vec::with_capacity(4);
        write_sample(&mut out, format, value);
        match format {
            SampleFormat::U8 => {
                assert!(out.len() == 1);
                if value.is_nan() {
                    assert!(out[0] == 128);
                }
                if value >= 1.0 {
                    assert!(out[0] == 255);
                }
                if value <= -1.0 {
                    assert!(out[0] == 0);
                }
            }
            SampleFormat::S16 => {
                assert!(out.len() == 2);
                let s = i16::from_le_bytes([out[0], out[1]]);
                if value.is_nan() {
                    assert!(s == 0);
                }
                if value >= 1.0 {
                    assert!(s == i16::MAX);
                }
                if value <= -1.0 {
                    assert!(s == i16::MIN);
                }
            }
            SampleFormat::S24 => {
                assert!(out.len() == 3);
                let sign = if out[2] & 0x80 == 0 { 0 } else { 0xFF };
                let s = i32::from_le_bytes([out[0], out[1], out[2], sign]);
                if value.is_nan() {
                    assert!(s == 0);
                }
                if value >= 1.0 {
                    assert!(s == 8_388_607);
                }
                if value <= -1.0 {
                    assert!(s == -8_388_608);
                }
            }
            SampleFormat::S32 => {
                assert!(out.len() == 4);
                let s = i32::from_le_bytes([out[0], out[1], out[2], out[3]]);
                if value.is_nan() {
                    assert!(s == 0);
                }
                if value >= 1.0 {
                    assert!(s == i32::MAX);
                }
                if value <= -1.0 {
                    assert!(s == i32::MIN);
                }
            }
            SampleFormat::F32 => {
                assert!(out.len() == 4);
                assert!(out[..] == value.to_le_bytes()[..]);
            }
            _ => {
                assert!(out.is_empty());
            }
        }
    }
}
