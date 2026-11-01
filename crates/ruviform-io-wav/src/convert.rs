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
