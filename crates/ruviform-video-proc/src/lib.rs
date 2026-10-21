//! `ruviform-video-proc` — base video processing kernels for the Ruviform
//! media foundation: color conversion, scaling and compositing kernels
//! operating on [`VideoFrame`](ruviform_video_core::VideoFrame) and the
//! video-core pixel/color types, plus the owner of the GPU compute-kernel
//! boundary port.
//!
//! Holds no codec, demux/mux, or I/O. Frame, plane, pixel-format and
//! color/HDR metadata types live in `ruviform-video-core`; the media I/O
//! boundary belongs to `ruviform-media-io`.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`) and Domain foundations
//! (`ruviform-video-core`). Used today: `ruviform-video-core` (frame and
//! color types) and `ruviform-media-core` ([`Error`](ruviform_media_core::Error)/
//! [`Result`](ruviform_media_core::Result) — ADR D7). There is no
//! `ruviform-rt` dependency while kernels take no `RtContext` argument
//! (ADR D6), and this crate must not depend on same-tier crates
//! (`ruviform-dsp`, `ruviform-media-io`) — the baseline layer order
//! prohibits same-tier dependencies among the processing/boundary
//! crates.
//!
//! ## Kernel API shape
//!
//! Kernels follow the two-phase processing shape of the architecture
//! baseline: a non-RT `prepare`/constructor fixes coefficients and work
//! areas and produces a prepared processing object, whose
//! `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame)` is
//! allocation-free. No kernel takes an `RtContext` argument or implements
//! a `Processor` trait — when `ruviform-rt` defines `Processor`, the
//! kernel types implement it as an additive, backward-compatible
//! extension (ADR D1).
//!
//! ## Kernels
//!
//! - [`YuvToRgb`] — planar `Yuv420p`/`Yuv422p`/`Yuv444p` → packed `Rgba8`
//!   (matrix/range aware, chroma upsampling; the module doc carries the
//!   siting assumption, `ColorInfo` update rules and the bit-exact
//!   equivalence contract).
//! - [`RgbToYuv`] — packed `Rgba8` → planar `Yuv420p`/`Yuv422p`/`Yuv444p`
//!   (matrix/range fixed by constructor arguments, chroma downsampling
//!   by cell averaging in the Q16 domain; bit-exact equivalence
//!   contract).
//! - [`Rgba8ToRgbaF16`] / [`RgbaF16ToRgba8`] — `Rgba8` ↔ `RgbaF16`
//!   channel normalization (`v/255` ↔ `round(clamp(v)·255)`, `ColorInfo`
//!   carried unchanged; ulp-bounded forward, bit-exact inverse
//!   equivalence contract).
//!
//! Kernels are scalar reference implementations per the baseline
//! DSP/SIMD decision — each kernel documents whether a future optimized
//! backend must be bit-exact or within a ulp bound. Scaling and
//! compositing kernels stay reserved for later issues.
//!
//! ## RT constraints
//!
//! `process` APIs are RT-suitable by design — allocation-free,
//! non-blocking and bounded — but per the baseline a guarantee is only
//! declared per API via `**RT contract:**` labels; an API without the
//! label is not RT-guaranteed. Initialization/`prepare` APIs are non-RT.
//!
//! ## GPU compute-kernel boundary port
//!
//! This crate owns the GPU compute-kernel boundary port (the user-side
//! port in the baseline's contract/boundary-ownership table). The port
//! trait itself is deferred to the `ruviform-gpu` implementation issue —
//! this crate declares ownership only (ADR D4).
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2 `ruviform-video-proc` row, the GPU boundary row of
//! "契約型・境界ポートの所有", and the Domain foundations layer order) and
//! `docs/adr/video-proc-kernel-api-and-color-hdr-scope.md` (D1/D4/D6/D7).

// Kernel and kernel-option names deliberately echo their module
// (`yuv_to_rgb::YuvToRgb`, …) while being re-exported at the crate root;
// as a `publish = false` workspace-internal crate this API relies on
// literal construction for structs. Every public enum carries enum-level
// `#[non_exhaustive]`: exhaustive matching is reserved for crate-internal
// code while external users (sibling crates included) get forward
// compatibility — see
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod coef;
mod rgb_to_yuv;
mod rgba_f16;
mod yuv_to_rgb;

pub use rgb_to_yuv::RgbToYuv;
pub use rgba_f16::{Rgba8ToRgbaF16, RgbaF16ToRgba8};
pub use yuv_to_rgb::YuvToRgb;

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_video_proc::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// Dependency wiring smoke check: the `path` dependencies
    /// (`ruviform-video-core`, `ruviform-media-core`) resolve and the
    /// frame/pixel API surface is reachable — the kernel input/output
    /// types of later issues.
    #[test]
    fn video_core_dependency_is_reachable() {
        assert_eq!(ruviform_video_core::PixelFormat::Rgba8.plane_count(), 1);
        assert!(ruviform_video_core::PixelFormat::Yuv420p.is_planar());
    }
}
