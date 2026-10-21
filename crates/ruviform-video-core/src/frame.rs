//! Video frame container: [`VideoFrame`] bundles a [`PixelFormat`], a
//! frame size, a presentation timestamp ([`MediaTime`] in the container's
//! time base), an optional [`MediaDuration`], a [`ColorInfo`] and its
//! plane backing; [`VideoFormat`] is the standalone stream-level
//! description of pixel layout plus color interpretation.
//!
//! Timestamps use the container-level `MediaTime`/`MediaDuration` of
//! `ruviform-media-core` — frame stores `duration` as an `Option` only;
//! durations may be carried explicitly or derived downstream. A frame's
//! `Clone` is cheap: it shares the `Arc`'d plane backing (a new
//! reference, not a copy), so mutable plane access through
//! [`VideoFrame::plane_mut`] is withheld while any clone shares the
//! backing — lifetime management of shared frames is the caller's
//! responsibility and a non-RT concern. Dropping the last `Arc`
//! reference deallocates the backing, so an owned frame leaves the RT
//! path for the non-RT side before its final drop (drop retreat).

use std::fmt;
use std::sync::Arc;

use ruviform_media_core::{Error, MediaDuration, MediaTime, Result};

use crate::color::ColorInfo;
use crate::pixel::PixelFormat;
use crate::plane::{Plane, PlaneMut, SharedPlane, required_len};

/// Standalone description of a video stream's pixel layout and color
/// interpretation — the stream-level counterpart of the per-frame data in
/// [`VideoFrame`].
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_video_core::{ColorInfo, PixelFormat, VideoFormat};
///
/// let format = VideoFormat {
///     width: 1_920,
///     height: 1_080,
///     format: PixelFormat::Yuv420p,
///     color: ColorInfo::default(),
/// };
/// assert_eq!(format.format.plane_count(), 3);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VideoFormat {
    /// Frame width in samples on the full-resolution (luma) grid.
    pub width: u32,
    /// Frame height in samples on the full-resolution grid.
    pub height: u32,
    /// Pixel format — determines the plane layout.
    pub format: PixelFormat,
    /// Color space and HDR description.
    pub color: ColorInfo,
}

/// Plane backing owned (or shared) by a [`VideoFrame`].
///
/// The `Arc` makes `VideoFrame`'s `Clone` a cheap share; `width`/`height`
/// are this plane's own grid dimensions (subsampling applied) and
/// `stride` its own row pitch, both derived from the frame's
/// [`PixelFormat`] at construction.
#[derive(Clone)]
struct OwnedPlane {
    /// Plane pixels including row padding, reference-counted.
    data: Arc<[u8]>,
    /// Bytes between consecutive row starts (≥ `width * sample_bytes`).
    stride: usize,
    /// Samples per row on this plane's grid.
    width: u32,
    /// Rows on this plane.
    height: u32,
    /// Bytes per sample on this plane.
    sample_bytes: usize,
}

/// Grid dimensions `(width, height)` of plane `plane` of `format` at
/// frame size `width`×`height`.
///
/// Plane 0 is always the full-resolution grid; further planes are the
/// format's chroma-subsampled grids. Returns `None` when `plane` is out
/// of range for `format`.
fn plane_dimensions(
    format: PixelFormat,
    width: u32,
    height: u32,
    plane: usize,
) -> Option<(u32, u32)> {
    if plane == 0 {
        return Some((width, height));
    }
    if plane >= format.plane_count() {
        return None;
    }
    let (sub_w, sub_h) = format.chroma_subsampling()?;
    Some((width.div_ceil(sub_w), height.div_ceil(sub_h)))
}

/// Checked `width * sample_bytes` for a plane grid.
fn row_bytes_of(width: u32, sample_bytes: usize) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(sample_bytes))
        .ok_or(Error::InvalidValue("dimensions"))
}

/// A video frame: format, dimensions, presentation timestamp and its
/// plane backing.
///
/// Plane grid dimensions are derived from the [`PixelFormat`]: plane 0
/// covers the full `width`×`height` grid; for planar YUV the chroma
/// planes are scaled by the format's subsampling factors (`ceil` division
/// — e.g. 4:2:0 chroma is `ceil(w/2)`×`ceil(h/2)`). Strides are
/// per-plane and independent of `width`×`sample_bytes` — padding is
/// explicit. [`VideoFrame::new_owned`] produces tightly packed planes;
/// [`VideoFrame::from_shared_planes`] is the sole zero-copy supply path
/// for externally allocated (e.g. decoder) buffers and may carry padded
/// strides — the decoder mechanism itself is `要PoC`, deferred to the
/// media-io issue.
///
/// # Ownership
///
/// `Clone` is cheap: it shares the `Arc`'d plane backing — a new
/// reference, not a copy. Mutation requires exclusive ownership: while
/// any clone shares a plane's backing, [`plane_mut`](Self::plane_mut)
/// returns `None` for it (the `Arc::get_mut` write gate), so a shared
/// frame is read-only and `plane_mut() == None` is the observable
/// sharing signal — there is no other sharing-introspection API.
/// Dropping every other clone (a non-RT lifetime-management concern of
/// the caller) restores mutable access.
///
/// # Drop and frame pools
///
/// Dropping the last `Arc` reference deallocates the plane backing —
/// deallocation on the media path is an RT violation, so a `VideoFrame`
/// must not reach its final drop on an RT path. Retired frames are
/// handed to the non-RT side for dropping (the drop-retreat pattern of
/// the architecture baseline). A future frame pool will be owned by the
/// non-RT layer and its return/ownership-transfer protocol will be
/// explicit APIs fixed in later design; the contract this type fixes
/// today is shared-backing reference counting plus mutation only under
/// exclusive ownership.
///
/// PTS uses the container's [`MediaTime`]; `duration` is stored as an
/// `Option` only — supplied explicitly or derived downstream.
///
/// **RT contract:** value access (`format`/`width`/`height`/`pts`/
/// `duration`/`color`/`plane_count`), the `set_*` writers and
/// [`plane`](Self::plane) are RT-suitable borrows. [`plane_mut`](
/// Self::plane_mut) is RT-suitable but yields `None` while the backing
/// is shared. `Clone` (an `Arc` refcount bump) and `Drop` (deallocates
/// the backing when the last `Arc` is released) are non-RT — do not
/// clone a `VideoFrame` on an RT path, and let its last owner leave the
/// RT path before dropping it.
#[derive(Clone)]
pub struct VideoFrame {
    /// Pixel format — determines the plane layout.
    format: PixelFormat,
    /// Frame width on the full-resolution grid.
    width: u32,
    /// Frame height on the full-resolution grid.
    height: u32,
    /// Presentation timestamp in the container's time base.
    pts: MediaTime,
    /// Optional frame duration, explicit or derived downstream.
    duration: Option<MediaDuration>,
    /// Color space and HDR description.
    color: ColorInfo,
    /// One [`OwnedPlane`] per [`PixelFormat::plane_count`].
    planes: Vec<OwnedPlane>,
}

impl VideoFrame {
    /// Allocates a zeroed frame of `format` at `width`×`height` with
    /// tightly packed planes (`stride == row_bytes`).
    ///
    /// Fails with [`Error::InvalidValue`] on zero dimensions or a plane
    /// size exceeding `isize::MAX` bytes — the maximum single-allocation
    /// size.
    ///
    /// **RT contract:** non-RT only — allocates one `Vec` per plane.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TimeBase};
    /// use ruviform_video_core::{PixelFormat, VideoFrame};
    ///
    /// let pts = MediaTime::new(3_000, TimeBase::COMMON_90K);
    /// let frame = VideoFrame::new_owned(PixelFormat::Yuv420p, 5, 3, pts).unwrap();
    /// assert_eq!(frame.plane_count(), 3);
    /// // 4:2:0 chroma grid is ceil(5/2) × ceil(3/2).
    /// assert_eq!(frame.plane(1).unwrap().width(), 3);
    /// assert_eq!(frame.plane(1).unwrap().height(), 2);
    /// ```
    pub fn new_owned(format: PixelFormat, width: u32, height: u32, pts: MediaTime) -> Result<Self> {
        if width == 0 {
            return Err(Error::InvalidValue("width"));
        }
        if height == 0 {
            return Err(Error::InvalidValue("height"));
        }
        let planes = (0..format.plane_count())
            .map(|plane| {
                let (plane_width, plane_height) = plane_dimensions(format, width, height, plane)
                    .ok_or(Error::InvalidValue("dimensions"))?;
                let sample_bytes = format.bytes_per_sample(plane);
                let row_bytes = row_bytes_of(plane_width, sample_bytes)?;
                let len = required_len(row_bytes, plane_height, row_bytes)
                    .ok_or(Error::InvalidValue("dimensions"))?;
                Ok(OwnedPlane {
                    data: vec![0_u8; len].into(),
                    stride: row_bytes,
                    width: plane_width,
                    height: plane_height,
                    sample_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            format,
            width,
            height,
            pts,
            duration: None,
            color: ColorInfo::default(),
            planes,
        })
    }

    /// Adopts caller-supplied `Arc`-backed planes — the sole zero-copy
    /// supply path for externally allocated buffers.
    ///
    /// This is the only way external (e.g. decoder) buffers enter a
    /// frame: one [`SharedPlane`] per image plane, each carrying its own
    /// `Arc<[u8]>` backing. `SharedPlane` has no plane-offset field — a
    /// plane starting partway into a larger shared buffer cannot be
    /// expressed, so producers holding such layouts must repackage them
    /// into per-plane `Arc`s (a known limitation, re-evaluated in the
    /// media-io issue).
    ///
    /// Validates `width`/`height` (non-zero), `planes.len() ==
    /// format.plane_count()`, per-plane `stride >= row_bytes` and
    /// `data.len() >= stride * (plane_height - 1) + row_bytes` (longer
    /// backings are accepted; views trim to the usable region);
    /// [`Error::InvalidValue`] otherwise. Plane grid dimensions are
    /// derived from `format` exactly as in [`new_owned`](Self::new_owned).
    ///
    /// **RT contract:** non-RT only — moves `Arc` references and
    /// allocates the plane vector; runs on the frame supply path, not
    /// on the media path.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use ruviform_media_core::{MediaTime, TimeBase};
    /// use ruviform_video_core::{PixelFormat, SharedPlane, VideoFrame};
    ///
    /// let pts = MediaTime::new(0, TimeBase::COMMON_90K);
    /// // 4×2 RGBA8: row_bytes 16, padded stride 20 → 20 + 16 = 36 bytes.
    /// let planes = vec![SharedPlane {
    ///     data: Arc::from(vec![0_u8; 36].into_boxed_slice()),
    ///     stride: 20,
    /// }];
    /// let frame = VideoFrame::from_shared_planes(PixelFormat::Rgba8, 4, 2, pts, planes).unwrap();
    /// assert_eq!(frame.plane(0).unwrap().stride(), 20);
    /// assert_eq!(frame.plane(0).unwrap().row_bytes(), 16);
    /// ```
    pub fn from_shared_planes(
        format: PixelFormat,
        width: u32,
        height: u32,
        pts: MediaTime,
        planes: Vec<SharedPlane>,
    ) -> Result<Self> {
        if width == 0 {
            return Err(Error::InvalidValue("width"));
        }
        if height == 0 {
            return Err(Error::InvalidValue("height"));
        }
        if planes.len() != format.plane_count() {
            return Err(Error::InvalidValue("plane count"));
        }
        let planes = planes
            .into_iter()
            .enumerate()
            .map(|(i, shared)| {
                let (plane_width, plane_height) = plane_dimensions(format, width, height, i)
                    .ok_or(Error::InvalidValue("plane count"))?;
                let sample_bytes = format.bytes_per_sample(i);
                let row_bytes = row_bytes_of(plane_width, sample_bytes)?;
                if shared.stride < row_bytes {
                    return Err(Error::InvalidValue("stride"));
                }
                let needed = required_len(shared.stride, plane_height, row_bytes)
                    .ok_or(Error::InvalidValue("dimensions"))?;
                if shared.data.len() < needed {
                    return Err(Error::InvalidValue("data"));
                }
                Ok(OwnedPlane {
                    data: shared.data,
                    stride: shared.stride,
                    width: plane_width,
                    height: plane_height,
                    sample_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            format,
            width,
            height,
            pts,
            duration: None,
            color: ColorInfo::default(),
            planes,
        })
    }

    /// The frame's [`PixelFormat`].
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn format(&self) -> PixelFormat {
        self.format
    }

    /// Frame width on the full-resolution grid.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Frame height on the full-resolution grid.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Presentation timestamp in the container's time base.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn pts(&self) -> MediaTime {
        self.pts
    }

    /// Sets the presentation timestamp.
    ///
    /// **RT contract:** RT-suitable — `Copy` write, no allocation.
    pub const fn set_pts(&mut self, pts: MediaTime) {
        self.pts = pts;
    }

    /// Frame duration, when one is carried — stored as an `Option` only
    /// (explicit or derived downstream).
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn duration(&self) -> Option<MediaDuration> {
        self.duration
    }

    /// Sets or clears the frame duration.
    ///
    /// **RT contract:** RT-suitable — `Copy` write, no allocation.
    pub const fn set_duration(&mut self, duration: Option<MediaDuration>) {
        self.duration = duration;
    }

    /// Color space and HDR description of the frame.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub const fn color(&self) -> ColorInfo {
        self.color
    }

    /// Sets the color space and HDR description.
    ///
    /// **RT contract:** RT-suitable — `Copy` write, no allocation.
    pub const fn set_color(&mut self, color: ColorInfo) {
        self.color = color;
    }

    /// Number of image planes — always `format.plane_count()`.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn plane_count(&self) -> usize {
        self.planes.len()
    }

    /// Read-only view of plane `i`, or `None` when `i` is out of range.
    ///
    /// **RT contract:** RT-suitable — allocation-free borrow.
    #[must_use]
    pub fn plane(&self, i: usize) -> Option<Plane<'_>> {
        let plane = self.planes.get(i)?;
        Plane::new(
            &plane.data,
            plane.stride,
            plane.width,
            plane.height,
            plane.sample_bytes,
        )
    }

    /// Mutable view of plane `i`.
    ///
    /// Mutation requires exclusive ownership: returns `None` when `i` is
    /// out of range *or* while the plane's backing is still shared —
    /// the `Arc::get_mut` write gate fails as long as another
    /// [`VideoFrame`] clone holds a reference. `plane_mut()` returning
    /// `None` on an in-range index is the observable signal that a clone
    /// still shares the backing; dropping every other clone restores
    /// mutable access.
    ///
    /// **RT contract:** RT-suitable — allocation-free; mutability is
    /// gated on exclusive `Arc` ownership, never on copying.
    pub fn plane_mut(&mut self, i: usize) -> Option<PlaneMut<'_>> {
        let plane = self.planes.get_mut(i)?;
        let data = Arc::get_mut(&mut plane.data)?;
        PlaneMut::new(
            data,
            plane.stride,
            plane.width,
            plane.height,
            plane.sample_bytes,
        )
    }
}

impl fmt::Debug for VideoFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoFrame")
            .field("format", &self.format)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pts", &self.pts)
            .field("duration", &self.duration)
            .field("color", &self.color)
            .field("plane_count", &self.planes.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorPrimaries;
    use ruviform_media_core::TimeBase;

    fn pts() -> MediaTime {
        MediaTime::new(42, TimeBase::COMMON_90K)
    }

    #[test]
    fn owned_planes_derive_dims_per_format() {
        // Odd sizes exercise ceil division of the chroma grids.
        let cases = [
            (PixelFormat::Yuv420p, [(5, 3), (3, 2), (3, 2)]),
            (PixelFormat::Yuv422p, [(5, 3), (3, 3), (3, 3)]),
            (PixelFormat::Yuv444p, [(5, 3), (5, 3), (5, 3)]),
        ];
        for (format, expected) in cases {
            let frame = VideoFrame::new_owned(format, 5, 3, pts()).unwrap();
            assert_eq!(frame.plane_count(), 3);
            for (i, (w, h)) in expected.iter().enumerate() {
                let plane = frame.plane(i).unwrap();
                assert_eq!((plane.width(), plane.height()), (*w, *h));
                assert_eq!(plane.sample_bytes(), 1);
                assert_eq!(plane.stride(), plane.row_bytes());
            }
        }

        for (format, sample_bytes) in [(PixelFormat::Rgba8, 4), (PixelFormat::RgbaF16, 8)] {
            let frame = VideoFrame::new_owned(format, 5, 3, pts()).unwrap();
            assert_eq!(frame.plane_count(), 1);
            let plane = frame.plane(0).unwrap();
            assert_eq!((plane.width(), plane.height()), (5, 3));
            assert_eq!(plane.sample_bytes(), sample_bytes);
            assert_eq!(plane.stride(), 5 * sample_bytes);
            assert!(frame.plane(1).is_none());
        }
    }

    #[test]
    fn owned_frames_are_zeroed_and_tightly_packed() {
        let frame = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts()).unwrap();
        let luma = frame.plane(0).unwrap();
        // 4×2 luma, stride 4 → 8 bytes, no padding.
        assert_eq!(luma.data(), &[0_u8; 8]);
        assert_eq!(luma.stride(), luma.row_bytes());
    }

    #[test]
    fn owned_frame_rejects_oversized_planes() {
        // ~1.2e19 bytes fits `usize` on 64-bit targets but exceeds the
        // maximum single-allocation size — `Err`, never a panic or an
        // allocation attempt.
        assert_eq!(
            VideoFrame::new_owned(PixelFormat::Rgba8, 3_000_000_000, 1_000_000_000, pts())
                .unwrap_err(),
            Error::InvalidValue("dimensions")
        );
    }

    #[test]
    fn shared_planes_keep_padded_stride() {
        // 2×2 RGBA8: row_bytes 8, padded stride 10 → usable 10 + 8 = 18
        // bytes. Padding bytes sit between rows and must not leak into
        // row() slices.
        let mut backing = vec![0_u8; 18];
        backing[8] = 0xEE; // padding after row 0
        backing[9] = 0xEE;
        backing[10] = 0x11; // first byte of row 1
        let planes = vec![SharedPlane {
            data: Arc::from(backing.into_boxed_slice()),
            stride: 10,
        }];
        let frame =
            VideoFrame::from_shared_planes(PixelFormat::Rgba8, 2, 2, pts(), planes).unwrap();
        let plane = frame.plane(0).unwrap();
        assert_eq!(plane.stride(), 10);
        assert_eq!(plane.row_bytes(), 8);
        assert_eq!(plane.data().len(), 18);
        assert_eq!(plane.row(0).unwrap(), &[0_u8; 8]);
        assert_eq!(plane.row(1).unwrap()[0], 0x11);
    }

    #[test]
    fn shared_plane_views_trim_to_usable_region() {
        // 2×2 RGBA8: row_bytes 8, stride 10 → usable 10 + 8 = 18 bytes;
        // backing is 32 bytes — views must not expose the tail.
        let planes = vec![SharedPlane {
            data: Arc::from(vec![0_u8; 32].into_boxed_slice()),
            stride: 10,
        }];
        // 2×2 RGBA8: row_bytes 8, stride 10 → usable 18 of 32 bytes.
        let frame =
            VideoFrame::from_shared_planes(PixelFormat::Rgba8, 2, 2, pts(), planes).unwrap();
        let plane = frame.plane(0).unwrap();
        assert_eq!(plane.data().len(), 18);
        assert_eq!(plane.row(1).unwrap().len(), 8);
        assert!(plane.row(2).is_none());
    }

    #[test]
    fn shared_plane_validation_errors() {
        // Wrong plane count.
        let planes = vec![];
        assert_eq!(
            VideoFrame::from_shared_planes(PixelFormat::Yuv420p, 4, 2, pts(), planes).unwrap_err(),
            Error::InvalidValue("plane count")
        );

        // Stride below row_bytes (4×2 luma needs ≥ 4).
        let planes = vec![
            SharedPlane {
                data: Arc::from(vec![0_u8; 64].into_boxed_slice()),
                stride: 3,
            },
            SharedPlane {
                data: Arc::from(vec![0_u8; 8].into_boxed_slice()),
                stride: 2,
            },
            SharedPlane {
                data: Arc::from(vec![0_u8; 8].into_boxed_slice()),
                stride: 2,
            },
        ];
        assert_eq!(
            VideoFrame::from_shared_planes(PixelFormat::Yuv420p, 4, 2, pts(), planes).unwrap_err(),
            Error::InvalidValue("stride")
        );

        // Backing too short: chroma is 2×1, stride 2 → needs 2 bytes.
        let planes = vec![
            SharedPlane {
                data: Arc::from(vec![0_u8; 8].into_boxed_slice()),
                stride: 4,
            },
            SharedPlane {
                data: Arc::from(vec![0_u8; 1].into_boxed_slice()),
                stride: 2,
            },
            SharedPlane {
                data: Arc::from(vec![0_u8; 2].into_boxed_slice()),
                stride: 2,
            },
        ];
        assert_eq!(
            VideoFrame::from_shared_planes(PixelFormat::Yuv420p, 4, 2, pts(), planes).unwrap_err(),
            Error::InvalidValue("data")
        );

        // Zero dimensions.
        let planes = vec![SharedPlane {
            data: Arc::from(vec![0_u8; 16].into_boxed_slice()),
            stride: 16,
        }];
        assert_eq!(
            VideoFrame::from_shared_planes(PixelFormat::Rgba8, 0, 2, pts(), planes).unwrap_err(),
            Error::InvalidValue("width")
        );
        assert_eq!(
            VideoFrame::new_owned(PixelFormat::Rgba8, 4, 0, pts()).unwrap_err(),
            Error::InvalidValue("height")
        );
    }

    #[test]
    fn clone_shares_backing_until_dropped() {
        let mut frame = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts()).unwrap();
        let mut clone = frame.clone();

        // While a clone shares the backing, mutable access is withheld.
        assert!(frame.plane_mut(0).is_none());
        assert!(clone.plane_mut(0).is_none());

        drop(clone);
        frame.plane_mut(0).unwrap().row_mut(0).unwrap()[0] = 0xAB;
        assert_eq!(frame.plane(0).unwrap().row(0).unwrap()[0], 0xAB);
    }

    #[test]
    fn timestamp_duration_and_color_accessors() {
        let mut frame = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 2, pts()).unwrap();
        assert_eq!(frame.pts(), pts());
        assert_eq!(frame.duration(), None);
        assert_eq!(frame.color(), ColorInfo::default());
        assert_eq!(frame.format(), PixelFormat::Rgba8);
        assert_eq!((frame.width(), frame.height()), (2, 2));

        let duration = MediaDuration::new(3_000, TimeBase::COMMON_90K);
        frame.set_pts(MediaTime::ZERO);
        frame.set_duration(Some(duration));
        let color = ColorInfo {
            primaries: ColorPrimaries::Bt2020,
            ..ColorInfo::default()
        };
        frame.set_color(color);

        assert_eq!(frame.pts(), MediaTime::ZERO);
        assert_eq!(frame.duration(), Some(duration));
        assert_eq!(frame.color().primaries, ColorPrimaries::Bt2020);
    }
}
