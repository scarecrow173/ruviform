//! Binary transfer encoding and the [`PagedBytes`] port.
//!
//! `viz` encodes view results into bounded little-endian byte buffers
//! a frontend can page through — the "binary transfer" side of the
//! Issue #157 contract. [`PagedBytes`] deliberately matches the shape
//! of `ruviform_ui::DataSource` (`len` + `read(offset, buf)`), so an
//! application registers encoded payloads behind `rvf://data` with a
//! ~5-line adapter while `viz` keeps zero dependency on `ruviform-ui`
//! or any GUI backend.
//!
//! ## `RVBW` — waveform view bins
//!
//! | offset | field | type |
//! |---|---|---|
//! | 0 | magic `"RVBW"` | 4 B |
//! | 4 | version = 1 | `u16` |
//! | 6 | reserved | `u16` |
//! | 8 | bin count | `u32` |
//! | 12 | reserved | `u32` |
//! | 16 + 44·i | per-bin record: `start u64, end u64, min f64, max f64, rms f64, count u64` | 44 B |
//!
//! ## `RVST` — spectrogram tile
//!
//! | offset | field | type |
//! |---|---|---|
//! | 0 | magic `"RVST"` | 4 B |
//! | 4 | version = 1 | `u16` |
//! | 6 | dtype: 0=f64, 1=f32, 2=u8 | `u16` |
//! | 8 | cols | `u32` |
//! | 12 | rows | `u32` |
//! | 16 | frame start | `u64` |
//! | 24 | values, row-major `rows × cols`, row 0 = lowest Hz | dtype·rows·cols |
//!
//! For [`Quantization::U8`], values are already unit-normalized by the
//! caller (e.g. through [`crate::display::DbRange`]) and `NaN` encodes
//! as `255`; `f32`/`f64` keep IEEE `NaN` verbatim.

use crate::display::Quantization;
use crate::spectrogram::Tile;
use crate::waveform::ViewBin;

/// Paged read port for encoded payloads — same shape as
/// `ruviform_ui::DataSource`, so a trivial adapter bridges to
/// `rvf://data` registration.
///
/// **RT contract:** non-RT only.
pub trait PagedBytes: Send + Sync {
    /// Total payload length in bytes.
    fn len(&self) -> u64;

    /// Whether the payload is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads up to `buf.len()` bytes at `offset`; returns bytes read
    /// (`0` at end). Errors report a human-readable message, mirroring
    /// `DataSource::read`.
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String>;
}

/// An immutable encoded payload.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Debug, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    /// Wraps an encoded buffer.
    #[must_use]
    pub fn new(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PagedBytes for Bytes {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String> {
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(self.0.len());
        let avail = self.0.len().saturating_sub(start);
        let n = avail.min(buf.len());
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

/// An encoded waveform view plus its covered range — the metadata
/// callers key caches by.
#[derive(Clone, Debug, PartialEq)]
pub struct LodBytes {
    /// Encoded `RVBW` payload.
    pub bytes: Bytes,
    /// First sample covered.
    pub start: u64,
    /// Last sample covered (exclusive).
    pub end: u64,
    /// Bin count encoded.
    pub bins: usize,
}

/// Encodes view bins as an `RVBW` payload (layout in module docs).
///
/// `NaN` statistics stay `NaN` — the frontend renders gaps, not zeros.
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn encode_buckets(bins: &[ViewBin]) -> LodBytes {
    let mut out = Vec::with_capacity(16 + bins.len() * 44);
    out.extend_from_slice(b"RVBW");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(bins.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let (mut s, mut e) = (u64::MAX, 0u64);
    for b in bins {
        s = s.min(b.start);
        e = e.max(b.end);
        out.extend_from_slice(&b.start.to_le_bytes());
        out.extend_from_slice(&b.end.to_le_bytes());
        out.extend_from_slice(&b.bucket.min.to_le_bytes());
        out.extend_from_slice(&b.bucket.max.to_le_bytes());
        out.extend_from_slice(&b.bucket.rms.to_le_bytes());
        out.extend_from_slice(&b.bucket.count.to_le_bytes());
    }
    LodBytes {
        bytes: Bytes(out),
        start: if bins.is_empty() { 0 } else { s },
        end: e,
        bins: bins.len(),
    }
}

/// Encodes a tile as an `RVST` payload at the chosen
/// [`Quantization`]. `q` is applied per value — the explicit display
/// rounding step; the source tile is untouched.
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn encode_tile(tile: &Tile, q: Quantization) -> Bytes {
    let dtype = match q {
        Quantization::F64 => 0u16,
        Quantization::F32 => 1u16,
        Quantization::U8 => 2u16,
    };
    let mut out = Vec::with_capacity(24 + tile.values.len() * q.bytes());
    out.extend_from_slice(b"RVST");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&dtype.to_le_bytes());
    out.extend_from_slice(&(tile.cols as u32).to_le_bytes());
    out.extend_from_slice(&(tile.rows as u32).to_le_bytes());
    out.extend_from_slice(&tile.frames.start.to_le_bytes());
    match q {
        Quantization::F64 => {
            for &v in &tile.values {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        Quantization::F32 => {
            for &v in &tile.values {
                out.extend_from_slice(&(v as f32).to_le_bytes());
            }
        }
        Quantization::U8 => {
            for &v in &tile.values {
                // Unit input assumed (caller normalized); NaN → 255.
                let b = if v.is_finite() {
                    (v.clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    255
                };
                out.push(b);
            }
        }
    }
    Bytes(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::LodBucket;

    #[test]
    fn paged_bytes_reads_bounded_ranges() {
        let b = Bytes::new((0u8..10).collect());
        assert_eq!(b.len(), 10);
        let mut buf = [0u8; 4];
        assert_eq!(b.read(2, &mut buf).unwrap(), 4);
        assert_eq!(buf, [2, 3, 4, 5]);
        assert_eq!(b.read(8, &mut buf).unwrap(), 2);
        assert_eq!(b.read(20, &mut buf).unwrap(), 0);
    }

    #[test]
    fn encode_buckets_layout() {
        let bins = vec![ViewBin {
            start: 8,
            end: 16,
            bucket: LodBucket {
                min: -1.0,
                max: 2.0,
                rms: 1.5,
                count: 8,
            },
        }];
        let lb = encode_buckets(&bins);
        assert_eq!(&lb.bytes.0[..4], b"RVBW");
        assert_eq!(lb.bins, 1);
        assert_eq!((lb.start, lb.end), (8, 16));
        let rec = &lb.bytes.0[16..];
        assert_eq!(f64::from_le_bytes(rec[16..24].try_into().unwrap()), -1.0);
        assert_eq!(f64::from_le_bytes(rec[24..32].try_into().unwrap()), 2.0);
        assert_eq!(u64::from_le_bytes(rec[40..48].try_into().unwrap()), 8);
    }

    #[test]
    fn encode_tile_dtypes_and_nan() {
        let tile = Tile {
            frames: 0..2,
            cols: 2,
            rows: 1,
            values: vec![0.5, f64::NAN],
        };
        let b64 = encode_tile(&tile, Quantization::F64);
        assert_eq!(b64.0.len(), 24 + 16);
        let b8 = encode_tile(&tile, Quantization::U8);
        assert_eq!(b8.0.len(), 24 + 2);
        assert_eq!(b8.0[24], 128); // 0.5 → 128
        assert_eq!(b8.0[25], 255); // NaN → 255
        let b32 = encode_tile(&tile, Quantization::F32);
        assert_eq!(b32.0.len(), 24 + 8);
    }
}
