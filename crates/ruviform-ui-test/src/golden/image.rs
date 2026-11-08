//! `RgbaImage` — RGBA8 画素バッファと PNG / `Screenshot` decode
//! （Issue #314 slice 1）。
//!
//! 内部表現は `width * height * 4` byte の RGBA8。`from_png` は
//! `png` crate の `normalize_to_color8`（EXPAND|STRIP_16）で
//! 8bit Gray/RGB/GA/RGBA を受理して RGBA8 に正規化する。
//! `from_screenshot` は `proto::Screenshot` の `png` /
//! `rgba-raw` format のみを受理し、それ以外の format は
//! 「画像を得られなかった」ことを成功扱いせずエラーにする。
//!
//! **RT contract:** non-RT only.

use std::io::{BufReader, Cursor};

use base64::Engine as _;

use crate::proto::Screenshot;

/// PNG decode の次元上限（1 辺）。8K screenshot（7680×4320）を
/// 十分に収めつつ、異常な巨大画像を拒否する。
const MAX_DIMENSION: u32 = 16384;
/// PNG decode の総画素数上限（`1 << 27` ≈ 512MiB RGBA）。
const MAX_PIXELS: u64 = 1 << 27;

/// 画像 decode/encode の失敗。
#[non_exhaustive]
#[derive(Debug)]
pub enum GoldenImageError {
    /// 画素バッファ長が `width * height * 4` と一致しない、または
    /// 次元が 0。
    InvalidBuffer {
        /// 宣言された幅。
        width: u32,
        /// 宣言された高さ。
        height: u32,
        /// 実際のバッファ長。
        len: usize,
    },
    /// PNG decode の失敗。
    Png(png::DecodingError),
    /// PNG encode の失敗。
    Encode(png::EncodingError),
    /// PNG の色形式/bit depth が RGBA8 正規化の対象外。
    UnsupportedColorType {
        /// `png::ColorType` の debug 表現。
        color_type: String,
        /// `png::BitDepth` の debug 表現。
        bit_depth: String,
    },
    /// PNG の次元が上限を超過。
    DimensionLimit {
        /// 宣言された幅。
        width: u32,
        /// 宣言された高さ。
        height: u32,
    },
    /// `Screenshot.format` が未対応（`png`/`rgba-raw` 以外）。
    UnsupportedFormat {
        /// wire 上の format 文字列。
        format: String,
    },
    /// base64 payload の decode 失敗。
    Base64(base64::DecodeError),
    /// `rgba-raw` のバッファ長が `width * height * 4` と不一致。
    RawLength {
        /// 宣言された幅。
        width: u32,
        /// 宣言された高さ。
        height: u32,
        /// 実際のバッファ長。
        len: usize,
    },
}

impl std::fmt::Display for GoldenImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBuffer {
                width,
                height,
                len,
            } => write!(
                f,
                "invalid rgba buffer: {width}x{height} needs {} bytes, got {len}",
                u64::from(*width) * u64::from(*height) * 4
            ),
            Self::Png(e) => write!(f, "png decode: {e}"),
            Self::Encode(e) => write!(f, "png encode: {e}"),
            Self::UnsupportedColorType {
                color_type,
                bit_depth,
            } => write!(f, "unsupported png format: {color_type}/{bit_depth}"),
            Self::DimensionLimit { width, height } => {
                write!(f, "png dimensions {width}x{height} exceed limit")
            }
            Self::UnsupportedFormat { format } => {
                write!(f, "unsupported screenshot format `{format}`")
            }
            Self::Base64(e) => write!(f, "base64: {e}"),
            Self::RawLength {
                width,
                height,
                len,
            } => write!(
                f,
                "rgba-raw length mismatch: {width}x{height} needs {} bytes, got {len}",
                u64::from(*width) * u64::from(*height) * 4
            ),
        }
    }
}

impl std::error::Error for GoldenImageError {}

impl From<png::DecodingError> for GoldenImageError {
    fn from(e: png::DecodingError) -> Self {
        Self::Png(e)
    }
}

impl From<png::EncodingError> for GoldenImageError {
    fn from(e: png::EncodingError) -> Self {
        Self::Encode(e)
    }
}

impl From<base64::DecodeError> for GoldenImageError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Base64(e)
    }
}

/// RGBA8 画素バッファ。`pixels` は row-major RGBA8 で
/// `len == width * height * 4`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
    /// RGBA8 バッファから構築する。長さ・次元を検証する。
    ///
    /// # Errors
    /// [`GoldenImageError::InvalidBuffer`] when `pixels.len()` does not
    /// equal `width * height * 4` or a dimension is 0.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, GoldenImageError> {
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|p| p.checked_mul(4))
            .ok_or(GoldenImageError::InvalidBuffer {
                width,
                height,
                len: pixels.len(),
            })?;
        if width == 0 || height == 0 || expected != pixels.len() as u64 {
            return Err(GoldenImageError::InvalidBuffer {
                width,
                height,
                len: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// 単色画像を構築する（test/fixture 用途）。
    #[must_use]
    pub fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Self {
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..u64::from(width) * u64::from(height) {
            pixels.extend_from_slice(&rgba);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// 幅。
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// 高さ。
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// 画素数。
    #[must_use]
    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// RGBA8 バイト列（row-major）。
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// `(x, y)` の RGBA8 画素。範囲外は `None`。
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels.get(i..i + 4).map(|s| [s[0], s[1], s[2], s[3]])
    }

    /// `(x, y)` の画素を上書きする。範囲外は無視する。
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        if let Some(dst) = self.pixels.get_mut(i..i + 4) {
            dst.copy_from_slice(&rgba);
        }
    }

    /// PNG バイト列から decode する。
    ///
    /// `normalize_to_color8`（EXPAND|STRIP_16）を適用し、8bit の
    /// Gray/RGB/GA/RGBA 出力を RGBA8 に正規化する。次元は
    /// `MAX_DIMENSION`/`MAX_PIXELS` で制限する。
    ///
    /// # Errors
    /// [`GoldenImageError::Png`] on decode failure,
    /// [`GoldenImageError::DimensionLimit`] when dimensions exceed the
    /// cap, [`GoldenImageError::UnsupportedColorType`] for color types
    /// that cannot be normalized.
    pub fn from_png(bytes: &[u8]) -> Result<Self, GoldenImageError> {
        let mut decoder = png::Decoder::new(BufReader::new(Cursor::new(bytes)));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        check_dimensions(info.width, info.height)?;
        let buf_size = reader
            .output_buffer_size()
            .ok_or(GoldenImageError::DimensionLimit {
                width: info.width,
                height: info.height,
            })?;
        let mut buf = vec![0u8; buf_size];
        let output = reader.next_frame(&mut buf)?;
        check_dimensions(output.width, output.height)?;
        let rgba = expand_to_rgba(&buf[..output.buffer_size()], &output)?;
        Self::new(output.width, output.height, rgba)
    }

    /// RGBA8 PNG バイト列を生成する（encoder 設定は固定 —
    /// color=Rgba, depth=Eight）。
    ///
    /// # Errors
    /// [`GoldenImageError::Png`] on encode failure.
    pub fn to_png(&self) -> Result<Vec<u8>, GoldenImageError> {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&self.pixels)?;
            writer.finish()?;
        }
        Ok(buf)
    }

    /// `proto::Screenshot` wire 型から画像を構築する。
    ///
    /// `format == "png"` は base64 PNG、`format == "rgba-raw"` は
    /// base64 生 RGBA8（`width`/`height` は wire フィールド）を
    /// 受理する。それ以外の format は画像を得られなかったことを
    /// 成功扱いせず [`GoldenImageError::UnsupportedFormat`] を返す。
    ///
    /// # Errors
    /// [`GoldenImageError`] on unsupported format, base64 failure,
    /// decode failure, or buffer length mismatch.
    pub fn from_screenshot(s: &Screenshot) -> Result<Self, GoldenImageError> {
        let raw = base64::engine::general_purpose::STANDARD.decode(&s.data_base64)?;
        match s.format.as_str() {
            "png" => Self::from_png(&raw),
            "rgba-raw" => {
                let expected = u64::from(s.width)
                    .checked_mul(u64::from(s.height))
                    .and_then(|p| p.checked_mul(4));
                if expected != Some(raw.len() as u64) {
                    return Err(GoldenImageError::RawLength {
                        width: s.width,
                        height: s.height,
                        len: raw.len(),
                    });
                }
                Self::new(s.width, s.height, raw)
            }
            other => Err(GoldenImageError::UnsupportedFormat {
                format: other.to_string(),
            }),
        }
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), GoldenImageError> {
    let pixels = u64::from(width) * u64::from(height);
    if width > MAX_DIMENSION || height > MAX_DIMENSION || pixels > MAX_PIXELS {
        return Err(GoldenImageError::DimensionLimit { width, height });
    }
    Ok(())
}

/// `normalize_to_color8` 後の 8bit 出力を RGBA8 に展開する。
/// 受理するのは Grayscale/Rgb/GrayscaleAlpha/Rgba の 8bit のみ
/// （Indexed は EXPAND で展開済み、16bit は STRIP_16 で 8bit 化済み）。
fn expand_to_rgba(
    buf: &[u8],
    info: &png::OutputInfo,
) -> Result<Vec<u8>, GoldenImageError> {
    use png::{BitDepth, ColorType};
    if info.bit_depth != BitDepth::Eight {
        return Err(GoldenImageError::UnsupportedColorType {
            color_type: format!("{:?}", info.color_type),
            bit_depth: format!("{:?}", info.bit_depth),
        });
    }
    let (w, h) = (info.width as usize, info.height as usize);
    let mut out = Vec::with_capacity(w * h * 4);
    match info.color_type {
        ColorType::Rgba => out.extend_from_slice(buf),
        ColorType::Rgb => {
            for px in buf.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        ColorType::Grayscale => {
            for &g in buf {
                out.extend_from_slice(&[g, g, g, 255]);
            }
        }
        ColorType::GrayscaleAlpha => {
            for px in buf.chunks_exact(2) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        other => {
            return Err(GoldenImageError::UnsupportedColorType {
                color_type: format!("{other:?}"),
                bit_depth: format!("{:?}", info.bit_depth),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::Screenshot;

    #[test]
    fn rgba_image_new_validates_buffer() {
        assert!(RgbaImage::new(2, 2, vec![0; 16]).is_ok());
        assert!(matches!(
            RgbaImage::new(2, 2, vec![0; 15]),
            Err(GoldenImageError::InvalidBuffer { .. })
        ));
        assert!(matches!(
            RgbaImage::new(0, 2, vec![]),
            Err(GoldenImageError::InvalidBuffer { .. })
        ));
        // w*h*4 の overflow も拒否。
        assert!(matches!(
            RgbaImage::new(u32::MAX, u32::MAX, vec![]),
            Err(GoldenImageError::InvalidBuffer { .. })
        ));
    }

    #[test]
    fn rgba_image_pixel_access() {
        let mut img = RgbaImage::solid(2, 2, [10, 20, 30, 40]);
        assert_eq!(img.pixel(0, 0), Some([10, 20, 30, 40]));
        assert_eq!(img.pixel(2, 0), None);
        img.set_pixel(1, 1, [1, 2, 3, 4]);
        assert_eq!(img.pixel(1, 1), Some([1, 2, 3, 4]));
        img.set_pixel(9, 9, [9, 9, 9, 9]); // 範囲外は無視
        assert_eq!(img.pixel_count(), 4);
    }

    #[test]
    fn png_round_trip_preserves_rgba8() {
        let mut img = RgbaImage::solid(3, 2, [0, 0, 0, 0]);
        img.set_pixel(1, 0, [255, 0, 0, 255]);
        img.set_pixel(2, 1, [0, 255, 0, 128]);
        let png = img.to_png().unwrap();
        let back = RgbaImage::from_png(&png).unwrap();
        assert_eq!(img, back);
    }

    #[test]
    fn png_decode_rejects_garbage() {
        assert!(RgbaImage::from_png(b"not a png").is_err());
        assert!(RgbaImage::from_png(&[]).is_err());
    }

    #[test]
    fn from_screenshot_decodes_png() {
        let img = RgbaImage::solid(2, 2, [9, 8, 7, 6]);
        let png = img.to_png().unwrap();
        let shot = Screenshot {
            format: "png".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&png),
            width: 2,
            height: 2,
        };
        assert_eq!(RgbaImage::from_screenshot(&shot).unwrap(), img);
    }

    #[test]
    fn from_screenshot_decodes_rgba_raw() {
        let img = RgbaImage::solid(2, 1, [1, 2, 3, 4]);
        let shot = Screenshot {
            format: "rgba-raw".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD
                .encode(img.pixels()),
            width: 2,
            height: 1,
        };
        assert_eq!(RgbaImage::from_screenshot(&shot).unwrap(), img);
    }

    #[test]
    fn from_screenshot_rejects_unknown_format_and_bad_input() {
        let shot = Screenshot {
            format: "bmp".to_string(),
            data_base64: "AAAA".to_string(),
            width: 1,
            height: 1,
        };
        assert!(matches!(
            RgbaImage::from_screenshot(&shot),
            Err(GoldenImageError::UnsupportedFormat { .. })
        ));
        let bad_b64 = Screenshot {
            format: "png".to_string(),
            data_base64: "!!!not-base64!!!".to_string(),
            width: 1,
            height: 1,
        };
        assert!(matches!(
            RgbaImage::from_screenshot(&bad_b64),
            Err(GoldenImageError::Base64(_))
        ));
        // width*height*4 = 16 needed, only 4 given.
        let short_raw = Screenshot {
            format: "rgba-raw".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode([0u8; 4]),
            width: 2,
            height: 2,
        };
        assert!(matches!(
            RgbaImage::from_screenshot(&short_raw),
            Err(GoldenImageError::RawLength { .. })
        ));
    }
}
