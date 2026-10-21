//! Plane views and shared plane backing: [`Plane`] and [`PlaneMut`] are
//! borrowed, allocation-free views over one frame plane;
//! [`SharedPlane`] is the `Arc`-backed input type of
//! [`VideoFrame::from_shared_planes`](crate::VideoFrame::from_shared_planes) —
//! together they are the sole zero-copy supply path for externally
//! allocated buffers.
//!
//! A plane is a rectangular grid of `width`×`height` samples of
//! `sample_bytes` bytes each. Rows are `row_bytes` long; `stride` bytes
//! separate consecutive row starts. `stride` is always ≥ `row_bytes` —
//! any difference is explicit padding, so stride is independent of
//! `width`×`sample_bytes`.

use std::sync::Arc;

/// Bytes of backing storage a plane needs: `stride` bytes per row for
/// `height - 1` rows plus `row_bytes` for the last row (no padding is
/// required after the final row).
///
/// Returns `None` on arithmetic overflow, or when the result exceeds
/// `isize::MAX` — the largest byte size a single allocation or slice may
/// have, so no backing could ever satisfy the described plane.
pub(crate) fn required_len(stride: usize, height: u32, row_bytes: usize) -> Option<usize> {
    stride
        .checked_mul(usize::try_from(height.checked_sub(1)?).ok()?)
        .and_then(|rows| rows.checked_add(row_bytes))
        .filter(|len| isize::try_from(*len).is_ok())
}

/// Borrowed read-only view of one frame plane.
///
/// The view exposes the plane's *usable* region:
/// `stride * (height - 1) + row_bytes` bytes — inter-row padding
/// included, any backing bytes beyond the last row excluded.
///
/// Obtained from [`VideoFrame::plane`](crate::VideoFrame::plane).
///
/// **RT contract:** RT-suitable — `Copy` borrowed view; every method is a
/// pure, allocation-free borrow returning `Option`/`&[u8]` instead of
/// panicking.
#[derive(Copy, Clone)]
pub struct Plane<'a> {
    /// Usable plane data: `stride * (height - 1) + row_bytes` bytes.
    data: &'a [u8],
    /// Bytes between consecutive row starts (≥ `row_bytes`).
    stride: usize,
    /// Samples per row on this plane's grid.
    width: u32,
    /// Rows on this plane.
    height: u32,
    /// Bytes per sample on this plane.
    sample_bytes: usize,
}

impl<'a> Plane<'a> {
    /// Builds a view over `data`, trimmed to the usable region.
    ///
    /// Returns `None` when `stride < row_bytes` (rows would overlap) or
    /// `data` is too short for the described plane.
    pub(crate) fn new(
        data: &'a [u8],
        stride: usize,
        width: u32,
        height: u32,
        sample_bytes: usize,
    ) -> Option<Self> {
        let row_bytes = usize::try_from(width).ok()?.checked_mul(sample_bytes)?;
        if stride < row_bytes {
            return None;
        }
        let needed = required_len(stride, height, row_bytes)?;
        if data.len() < needed {
            return None;
        }
        Some(Self {
            data: &data[..needed],
            stride,
            width,
            height,
            sample_bytes,
        })
    }

    /// The plane's usable bytes: `stride * (height - 1) + row_bytes`,
    /// including inter-row padding.
    ///
    /// **RT contract:** RT-suitable — allocation-free borrow.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Bytes between the start of two consecutive rows; ≥
    /// [`row_bytes`](Self::row_bytes), with any difference being explicit
    /// padding.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Samples per row on this plane's grid (frame width scaled by the
    /// plane's subsampling).
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Rows on this plane.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Bytes per sample on this plane.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn sample_bytes(&self) -> usize {
        self.sample_bytes
    }

    /// Bytes per row of actual samples: `width * sample_bytes` — padding
    /// excluded.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn row_bytes(&self) -> usize {
        self.width as usize * self.sample_bytes
    }

    /// Row `y` as a slice of exactly [`row_bytes`](Self::row_bytes)
    /// bytes — padding excluded.
    ///
    /// Returns `None` when `y >= height`.
    ///
    /// **RT contract:** RT-suitable — allocation-free borrow; `None`
    /// instead of panic on out-of-range input.
    #[must_use]
    pub fn row(&self, y: u32) -> Option<&'a [u8]> {
        if y >= self.height {
            return None;
        }
        let start = y as usize * self.stride;
        self.data.get(start..start + self.row_bytes())
    }
}

/// Borrowed mutable view of one frame plane.
///
/// Same layout semantics as [`Plane`]. Obtained from
/// [`VideoFrame::plane_mut`](crate::VideoFrame::plane_mut), which yields
/// `None` while the plane's backing is shared by a
/// [`VideoFrame`](crate::VideoFrame) clone — the mutable view therefore
/// always sees exclusively-owned backing.
///
/// **RT contract:** RT-suitable — borrowed view; every method is a pure,
/// allocation-free (re)borrow returning `Option` instead of panicking.
pub struct PlaneMut<'a> {
    /// Usable plane data: `stride * (height - 1) + row_bytes` bytes.
    data: &'a mut [u8],
    /// Bytes between consecutive row starts (≥ `row_bytes`).
    stride: usize,
    /// Samples per row on this plane's grid.
    width: u32,
    /// Rows on this plane.
    height: u32,
    /// Bytes per sample on this plane.
    sample_bytes: usize,
}

impl<'a> PlaneMut<'a> {
    /// Builds a view over `data`, trimmed to the usable region.
    ///
    /// Returns `None` when `stride < row_bytes` (rows would overlap) or
    /// `data` is too short for the described plane.
    pub(crate) fn new(
        data: &'a mut [u8],
        stride: usize,
        width: u32,
        height: u32,
        sample_bytes: usize,
    ) -> Option<Self> {
        let row_bytes = usize::try_from(width).ok()?.checked_mul(sample_bytes)?;
        if stride < row_bytes {
            return None;
        }
        let needed = required_len(stride, height, row_bytes)?;
        if data.len() < needed {
            return None;
        }
        Some(Self {
            data: &mut data[..needed],
            stride,
            width,
            height,
            sample_bytes,
        })
    }

    /// The plane's usable bytes: `stride * (height - 1) + row_bytes`,
    /// including inter-row padding.
    ///
    /// **RT contract:** RT-suitable — allocation-free borrow.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Mutable access to the plane's usable bytes.
    ///
    /// The returned slice borrows this view (not the frame's whole
    /// backing beyond the last row).
    ///
    /// **RT contract:** RT-suitable — allocation-free reborrow.
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Bytes between the start of two consecutive rows; ≥
    /// [`row_bytes`](Self::row_bytes), with any difference being explicit
    /// padding.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Samples per row on this plane's grid (frame width scaled by the
    /// plane's subsampling).
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Rows on this plane.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Bytes per sample on this plane.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn sample_bytes(&self) -> usize {
        self.sample_bytes
    }

    /// Bytes per row of actual samples: `width * sample_bytes` — padding
    /// excluded.
    ///
    /// **RT contract:** RT-suitable — pure value access.
    #[must_use]
    pub fn row_bytes(&self) -> usize {
        self.width as usize * self.sample_bytes
    }

    /// Row `y` as a slice of exactly [`row_bytes`](Self::row_bytes)
    /// bytes — padding excluded.
    ///
    /// Returns `None` when `y >= height`.
    ///
    /// **RT contract:** RT-suitable — allocation-free borrow; `None`
    /// instead of panic on out-of-range input.
    #[must_use]
    pub fn row(&self, y: u32) -> Option<&[u8]> {
        if y >= self.height {
            return None;
        }
        let start = y as usize * self.stride;
        self.data.get(start..start + self.row_bytes())
    }

    /// Mutable access to row `y` — exactly
    /// [`row_bytes`](Self::row_bytes) bytes, padding excluded.
    ///
    /// Returns `None` when `y >= height`.
    ///
    /// **RT contract:** RT-suitable — allocation-free reborrow; `None`
    /// instead of panic on out-of-range input.
    pub fn row_mut(&mut self, y: u32) -> Option<&mut [u8]> {
        if y >= self.height {
            return None;
        }
        let start = y as usize * self.stride;
        self.data.get_mut(start..start + self.row_bytes())
    }
}

/// Reference-counted plane backing supplied to
/// [`VideoFrame::from_shared_planes`](crate::VideoFrame::from_shared_planes).
///
/// The caller keeps ownership of the allocation while the frame shares it
/// via `Arc` — together with `from_shared_planes` this is the sole
/// zero-copy supply path for externally allocated buffers (the decoder
/// mechanism producing these buffers is itself `要PoC`, deferred to the
/// media-io issue).
///
/// One `SharedPlane` is supplied per image plane, in the format's fixed
/// plane order — for the planar YUV formats: luma (`Y`) first, then
/// `Cb`, then `Cr` (see [`PixelFormat`](crate::PixelFormat)).
///
/// `data` addresses the plane from byte 0: there is no plane-offset
/// field, so a plane starting partway into a larger shared buffer cannot
/// be expressed — producers must repackage such regions into their own
/// `Arc<[u8]>` first. This offset-less shape is a known limitation, to
/// be re-evaluated in the media-io issue (ADR
/// `docs/adr/video-proc-kernel-api-and-color-hdr-scope.md`, D5-i).
///
/// `data` must cover at least `stride * (plane_height - 1) + row_bytes`
/// bytes for the plane the frame derives it as; `stride` is per-plane and
/// independent of `width`×`sample_bytes` (padding explicit). Longer
/// backings are accepted — frame views trim to the usable region and
/// never expose the tail.
///
/// **RT contract:** non-RT only — constructing, cloning and dropping it
/// manages `Arc` refcounts (and deallocates the backing on the last
/// drop) on the frame supply path, not on the media path.
#[derive(Clone, Debug)]
pub struct SharedPlane {
    /// Plane pixels including row padding, reference-counted.
    pub data: Arc<[u8]>,
    /// Bytes between consecutive row starts; must be ≥ `row_bytes` of the
    /// derived plane.
    pub stride: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shared view type must be constructible without a frame for these
    /// tests; `Plane::new` is `pub(crate)` so unit tests can exercise it
    /// directly.
    #[test]
    fn view_trims_to_usable_region() {
        // 2×2 plane of 1-byte samples at stride 4 → 4 + 2 = 6 usable
        // bytes; backing carries 4 tail bytes.
        let backing = [0u8; 10];
        let plane = Plane::new(&backing, 4, 2, 2, 1).unwrap();
        assert_eq!(plane.data().len(), 6);
        assert_eq!(plane.row_bytes(), 2);
        assert_eq!(plane.row(0).unwrap().len(), 2);
        assert_eq!(plane.row(1).unwrap().len(), 2);
        assert!(plane.row(2).is_none());
    }

    #[test]
    fn view_rejects_undersized_backing() {
        let backing = [0u8; 5];
        assert!(Plane::new(&backing, 4, 2, 2, 1).is_none());

        let mut backing = [0u8; 5];
        assert!(PlaneMut::new(&mut backing, 4, 2, 2, 1).is_none());
    }

    #[test]
    fn view_rejects_overlapping_rows() {
        // stride < row_bytes (2 > 1) would overlap consecutive rows even
        // though the backing is long enough; stride == row_bytes is the
        // tightest legal packing.
        let backing = [0u8; 10];
        assert!(Plane::new(&backing, 1, 2, 2, 1).is_none());
        assert!(Plane::new(&backing, 2, 2, 2, 1).is_some());

        let mut backing = [0u8; 10];
        assert!(PlaneMut::new(&mut backing, 1, 2, 2, 1).is_none());
    }

    #[test]
    fn mutable_view_exposes_plane_geometry() {
        // 3×2 plane of 2-byte samples: row_bytes 6, padded stride 8 →
        // usable 8 + 6 = 14 bytes.
        let mut backing = [0u8; 14];
        let mut plane = PlaneMut::new(&mut backing, 8, 3, 2, 2).unwrap();
        assert_eq!(plane.width(), 3);
        assert_eq!(plane.height(), 2);
        assert_eq!(plane.sample_bytes(), 2);
        assert_eq!(plane.row_bytes(), 6);
        assert_eq!(plane.stride(), 8);
        assert_eq!(plane.data().len(), 14);
        // `data_mut` writes through the view into the backing.
        plane.data_mut()[8] = 0xAB; // first byte of row 1
        assert_eq!(plane.data()[8], 0xAB);
        assert_eq!(plane.row(1).unwrap()[0], 0xAB);
    }

    #[test]
    fn mutable_view_writes_through() {
        let mut backing = [0u8; 10];
        let mut plane = PlaneMut::new(&mut backing, 4, 2, 2, 1).unwrap();
        plane.row_mut(1).unwrap().copy_from_slice(&[7, 8]);
        assert_eq!(plane.row(1), Some(&[7, 8][..]));
        assert!(plane.row_mut(2).is_none());
    }

    #[test]
    fn required_len_accounts_for_last_row() {
        assert_eq!(required_len(4, 1, 2), Some(2));
        assert_eq!(required_len(4, 3, 2), Some(10));
        assert_eq!(required_len(usize::MAX, 2, 1), None);
        assert_eq!(required_len(4, 0, 2), None);
        // Past `isize::MAX` bytes (usize::MAX / 2 + 2) — rejected even
        // without `usize` overflow: no backing could ever be that large.
        assert_eq!(required_len(usize::MAX / 2, 2, 2), None);
    }
}
