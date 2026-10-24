//! Pixel format description: [`PixelFormat`] — planar YUV and packed RGBA
//! layouts with their plane counts, per-plane sample sizes and chroma
//! subsampling factors.
//!
//! The enum is `#[non_exhaustive]`: the design keeps the pixel-format range
//! extensible (other subsampling schemes, packed YUV, higher bit depths,
//! GPU-side formats) and refines the initial set in later issues.

/// Pixel format of a [`VideoFrame`](crate::VideoFrame).
///
/// The format alone determines the plane layout of a frame: the number of
/// planes ([`plane_count`](Self::plane_count)), the bytes per sample on each
/// plane ([`bytes_per_sample`](Self::bytes_per_sample)) and the chroma grid
/// relative to the luma/full-resolution grid
/// ([`chroma_subsampling`](Self::chroma_subsampling)).
///
/// For the planar YUV formats the plane order is fixed: plane 0 is luma
/// (`Y`), plane 1 is `Cb` and plane 2 is `Cr` — producers supplying
/// planes (e.g. via
/// [`VideoFrame::from_shared_planes`](crate::VideoFrame::from_shared_planes))
/// must use this order; consumers may rely on it.
///
/// **RT contract:** RT-suitable — `Copy` value type; every method is a pure
/// `const fn` returning a small value.
///
/// # Examples
///
/// ```
/// use ruviform_video_core::PixelFormat;
///
/// let format = PixelFormat::Yuv420p;
/// assert!(format.is_planar());
/// assert_eq!(format.plane_count(), 3);
/// assert_eq!(format.chroma_subsampling(), Some((2, 2)));
/// assert_eq!(PixelFormat::Rgba8.plane_count(), 1);
/// assert_eq!(PixelFormat::Rgba8.bytes_per_sample(0), 4);
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Planar YUV 4:2:0 — one full-resolution luma plane plus two chroma
    /// planes subsampled by 2 horizontally and vertically, 8 bits per
    /// sample.
    Yuv420p,
    /// Planar YUV 4:2:2 — one full-resolution luma plane plus two chroma
    /// planes subsampled by 2 horizontally only, 8 bits per sample.
    Yuv422p,
    /// Planar YUV 4:4:4 — one full-resolution luma plane plus two
    /// unsubsampled chroma planes, 8 bits per sample.
    Yuv444p,
    /// Packed RGBA, 8 bits per channel — one plane, 4 bytes per pixel.
    Rgba8,
    /// Packed RGBA, IEEE 754 half-precision float per channel — one plane,
    /// 8 bytes per pixel.
    RgbaF16,
    /// Packed RGBA, IEEE 754 single-precision (`binary32`)
    /// little-endian float per channel — one plane, 16 bytes per pixel.
    ///
    /// This is also the storage format of the color-management working
    /// space: tagged `{primaries: Bt2020, transfer: Linear,
    /// matrix: Identity, range: Full}`, a frame holds linear-light,
    /// absolute-luminance pixel values (`1.0` = 100 cd/m²), not limited
    /// to `[0, 1]`.
    RgbaF32,
}

impl PixelFormat {
    /// Number of image planes this format uses.
    ///
    /// Planar YUV formats use 3 planes (luma + 2 chroma); packed formats
    /// use a single plane.
    ///
    /// **RT contract:** RT-suitable — pure `const fn`.
    #[must_use]
    pub const fn plane_count(self) -> usize {
        match self {
            Self::Yuv420p | Self::Yuv422p | Self::Yuv444p => 3,
            Self::Rgba8 | Self::RgbaF16 | Self::RgbaF32 => 1,
        }
    }

    /// Whether this format stores components on separate planes (`true`)
    /// or interleaved on a single packed plane (`false`).
    ///
    /// **RT contract:** RT-suitable — pure `const fn`.
    #[must_use]
    pub const fn is_planar(self) -> bool {
        match self {
            Self::Yuv420p | Self::Yuv422p | Self::Yuv444p => true,
            Self::Rgba8 | Self::RgbaF16 | Self::RgbaF32 => false,
        }
    }

    /// Bytes per sample on `plane` — the size of one grid element on that
    /// plane (one 8-bit component on a YUV plane; one whole pixel on a
    /// packed plane: 4 bytes for [`Rgba8`](Self::Rgba8), 8 bytes for
    /// [`RgbaF16`](Self::RgbaF16), 16 bytes for
    /// [`RgbaF32`](Self::RgbaF32)).
    ///
    /// Returns 0 when `plane` is out of range for this format.
    ///
    /// **RT contract:** RT-suitable — pure `const fn`.
    #[must_use]
    pub const fn bytes_per_sample(self, plane: usize) -> usize {
        if plane >= self.plane_count() {
            return 0;
        }
        match self {
            Self::Yuv420p | Self::Yuv422p | Self::Yuv444p => 1,
            Self::Rgba8 => 4,
            Self::RgbaF16 => 8,
            Self::RgbaF32 => 16,
        }
    }

    /// Chroma subsampling factors `(horizontal, vertical)` relative to the
    /// full-resolution grid.
    ///
    /// `Some((2, 2))` for 4:2:0, `Some((2, 1))` for 4:2:2 and
    /// `Some((1, 1))` for unsubsampled 4:4:4. `None` for packed formats,
    /// where the luma/chroma plane split does not apply.
    ///
    /// **RT contract:** RT-suitable — pure `const fn`.
    #[must_use]
    pub const fn chroma_subsampling(self) -> Option<(u32, u32)> {
        match self {
            Self::Yuv420p => Some((2, 2)),
            Self::Yuv422p => Some((2, 1)),
            Self::Yuv444p => Some((1, 1)),
            Self::Rgba8 | Self::RgbaF16 | Self::RgbaF32 => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_layout_per_format() {
        for (format, planes, planar) in [
            (PixelFormat::Yuv420p, 3, true),
            (PixelFormat::Yuv422p, 3, true),
            (PixelFormat::Yuv444p, 3, true),
            (PixelFormat::Rgba8, 1, false),
            (PixelFormat::RgbaF16, 1, false),
            (PixelFormat::RgbaF32, 1, false),
        ] {
            assert_eq!(format.plane_count(), planes);
            assert_eq!(format.is_planar(), planar);
        }
    }

    #[test]
    fn bytes_per_sample_per_plane() {
        for format in [
            PixelFormat::Yuv420p,
            PixelFormat::Yuv422p,
            PixelFormat::Yuv444p,
        ] {
            for plane in 0..3 {
                assert_eq!(format.bytes_per_sample(plane), 1);
            }
            assert_eq!(format.bytes_per_sample(3), 0);
        }
        assert_eq!(PixelFormat::Rgba8.bytes_per_sample(0), 4);
        assert_eq!(PixelFormat::RgbaF16.bytes_per_sample(0), 8);
        assert_eq!(PixelFormat::RgbaF32.bytes_per_sample(0), 16);
        assert_eq!(PixelFormat::Rgba8.bytes_per_sample(1), 0);
        assert_eq!(PixelFormat::RgbaF32.bytes_per_sample(1), 0);
    }

    #[test]
    fn subsampling_per_format() {
        assert_eq!(PixelFormat::Yuv420p.chroma_subsampling(), Some((2, 2)));
        assert_eq!(PixelFormat::Yuv422p.chroma_subsampling(), Some((2, 1)));
        assert_eq!(PixelFormat::Yuv444p.chroma_subsampling(), Some((1, 1)));
        assert_eq!(PixelFormat::Rgba8.chroma_subsampling(), None);
        assert_eq!(PixelFormat::RgbaF16.chroma_subsampling(), None);
        assert_eq!(PixelFormat::RgbaF32.chroma_subsampling(), None);
    }
}
