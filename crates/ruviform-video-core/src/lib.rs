//! `ruviform-video-core` — video data types for the Ruviform media
//! foundation: pixel formats ([`PixelFormat`]), color space and HDR
//! descriptors ([`ColorInfo`], [`HdrMetadata`], [`MasteringDisplayInfo`],
//! [`ContentLightLevel`], [`Chromaticity`] and the labeling
//! enumerations), plane views ([`Plane`], [`PlaneMut`], [`SharedPlane`])
//! and the frame container ([`VideoFrame`], [`VideoFormat`]).
//!
//! Holds no conversion, compositing, or I/O. Color-conversion and HDR
//! processing belong to `ruviform-video-proc`; frame pools, decoder
//! mechanics and stream ports are later-crate concerns.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`) and `ruviform-media-core`.
//! Only media-core is used today: [`MediaTime`], [`MediaDuration`] and
//! the media-wide [`Error`]/[`Result`] types.
//!
//! ## RT contract
//!
//! Frame and plane accessors are RT-suitable: borrowing a [`Plane`] /
//! [`PlaneMut`] and slicing rows is allocation-free and never panics.
//! Allocating constructors ([`VideoFrame::new_owned`],
//! [`VideoFrame::from_shared_planes`]) and `VideoFrame`'s `Clone`
//! (reference-counted lifetime management) are non-RT APIs. Per-API
//! guarantees are documented as `**RT contract:**` lines on each item,
//! mirroring the RT ADR rule that the guaranteed scope is defined per
//! API.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "動画データモデル") and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names below deliberately echo their module
// (`pixel::PixelFormat`, `plane::Plane`, …) while being re-exported at the
// crate root; as a `publish = false` workspace-internal data-model crate
// this API relies on exhaustive matching and literal construction rather
// than `#[non_exhaustive]` forward-compat guards — `PixelFormat` itself
// is the one decided exception (the pixel-format range stays extensible).
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_enums)]
#![allow(clippy::exhaustive_structs)]

mod color;
mod frame;
mod pixel;
mod plane;

pub use color::{
    Chromaticity, ColorInfo, ColorPrimaries, ColorRange, ContentLightLevel, HdrMetadata,
    MasteringDisplayInfo, MatrixCoefficients, TransferCharacteristics,
};
pub use frame::{VideoFormat, VideoFrame};
pub use pixel::PixelFormat;
pub use plane::{Plane, PlaneMut, SharedPlane};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_video_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{MediaTime, TimeBase};

    fn pts() -> MediaTime {
        MediaTime::new(0, TimeBase::COMMON_90K)
    }

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: `plane()`/`row()` are allocation-free borrows over a
    /// built frame — pure slicing, `None` out of range, never a panic.
    /// (Frame construction is the non-RT setup; only the accessors are
    /// exercised here.)
    #[test]
    fn rt_plane_access_and_row_slicing() {
        let frame = VideoFrame::new_owned(PixelFormat::Yuv420p, 5, 3, pts()).unwrap();

        let luma = frame.plane(0).unwrap();
        assert_eq!((luma.width(), luma.height()), (5, 3));
        assert_eq!(luma.data().len(), 15);
        for y in 0..luma.height() {
            assert_eq!(luma.row(y).unwrap().len(), luma.row_bytes());
        }
        assert!(luma.row(3).is_none());
        assert!(frame.plane(3).is_none());

        // Chroma rows are exactly the subsampled row width — no padding
        // is invented for tightly packed owned planes.
        let chroma = frame.plane(1).unwrap();
        assert_eq!(chroma.row(0).unwrap().len(), 3);
    }

    /// RT contract: `plane_mut()`/`row_mut()` are allocation-free writes
    /// on exclusively-owned backing; mutability is withheld (`None`)
    /// while a clone shares the `Arc` backing.
    #[test]
    fn rt_plane_mut_row_write() {
        let mut frame = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 2, pts()).unwrap();

        frame
            .plane_mut(0)
            .unwrap()
            .row_mut(1)
            .unwrap()
            .copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(frame.plane(0).unwrap().row(1).unwrap()[7], 8);

        let clone = frame.clone();
        assert!(frame.plane_mut(0).is_none());
        drop(clone);
        assert!(frame.plane_mut(0).is_some());
    }

    /// RT contract: `set_pts`/`set_duration`/`pts`/`duration` are `Copy`
    /// value access — the timestamping path allocates nothing.
    #[test]
    fn rt_timestamp_access() {
        let mut frame = VideoFrame::new_owned(PixelFormat::Yuv444p, 2, 2, pts()).unwrap();
        frame.set_pts(MediaTime::new(90_000, TimeBase::COMMON_90K));
        assert_eq!(frame.pts().value(), 90_000);
    }
}
