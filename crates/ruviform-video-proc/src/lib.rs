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
//! allocation-free. The composite family keeps this shape while taking
//! animating edit parameters as per-call `process` arguments —
//! `process(src, dst, opacity)` for [`CompositeOver`],
//! `process(from, to, dst, t)` for [`Crossfade`] and
//! `process(src, dst, map)` for [`AffineTransform`] — because
//! constructor-baked animation inputs would force a non-RT re-`prepare`
//! per frame. Per-call validation/quantization stays a bounded scalar
//! step, so `process` remains allocation-free. No kernel takes an
//! `RtContext` argument or implements a `Processor` trait — when
//! `ruviform-rt` defines `Processor`, the kernel types implement it as
//! an additive, backward-compatible extension (ADR D1).
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
//! - [`ToWorking`] — `Rgba8`/`RgbaF16`/`RgbaF32` → `RgbaF32` entry
//!   into the f32 linear working space (fused transfer decode +
//!   `src primaries → BT.2020` gamut transform driven by the frame's
//!   `ColorInfo`; ≤ 4 ulp `binary32` equivalence contract).
//! - [`FromWorking`] — `RgbaF32` → `Rgba8`/`RgbaF32` working-space
//!   exit (`BT.2020 → dst primaries` matrix + transfer encode baked
//!   by the constructor; ≤ 4 ulp contract, `Rgba8` ±1 code
//!   quantization bound).
//! - [`ToneMapSdr`] — `RgbaF32` → `RgbaF32` HDR→SDR tone map
//!   (BT.2390-8 §5.4 EETF on the BT.2020 luma with ratio-preserving
//!   RGB scaling; `L_HS`/`L_min` derived from `src`'s `HdrMetadata`
//!   per call, `hdr` consumed — output records `hdr: None`; ≤ 4 ulp
//!   `binary32` equivalence contract).
//! - [`CompositeOver`] — `src over dst` source-over alpha compositing
//!   on straight alpha, in place on `dst`, with a per-call `opacity`
//!   (self-consistent Q8 integer expansion on `Rgba8`, `f32` pipeline +
//!   RNE `binary16` rounding on `RgbaF16`, unrounded `f32` pipeline on
//!   `RgbaF32` — the linear working-space arm; bit-exact / ≤ 2 ulp
//!   equivalence contract).
//! - [`Crossfade`] — `dst = from·(1−t) + to·t` two-source dissolve on
//!   all four channels with a per-call `t` (same formats, quantization
//!   and equivalence rules as [`CompositeOver`]).
//! - [`AffineTransform`] — integer-rectangle translate+scale via
//!   [`Rect`]/[`RectMap`]: cell-center nearest sampling, off-canvas and
//!   uncovered pixels filled with transparent black (pure pixel
//!   mapping/copy — bit-exact on every accepted format).
//! - `IccTransform` — `Rgba8`/`RgbaF32` → `Rgba8`/`RgbaF32` conversion
//!   through a caller-supplied ICC profile pair (`IccIntent` selects
//!   the rendering intent; bit-exact vs the pinned engine's executor
//!   output). **Cargo feature `icc` only** — see "Cargo features".
//!
//! Kernels are scalar reference implementations per the baseline
//! DSP/SIMD decision — each kernel documents whether a future optimized
//! backend must be bit-exact or within a ulp bound. Higher-quality
//! scaling filters (bilinear and above), full affine (rotation/shear),
//! additional blend modes and planar-YUV compositing stay reserved for
//! later issues.
//!
//! ## Cargo features
//!
//! - `icc` (default off — `default = []`): exposes `IccTransform` and
//!   `IccIntent`, the ICC profile transform path driven by an
//!   optional ICC engine dependency kept as a private implementation
//!   detail (no engine types, errors or feature names appear in the
//!   public API). With the feature off the crate keeps zero external
//!   dependencies — `ruviform-video-core`/`ruviform-media-core` `path`
//!   deps only. `cargo test --workspace` does not cover the feature;
//!   run `mise run test-video-proc-icc` /
//!   `mise run lint-video-proc-icc` for the feature lane.
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

mod affine;
mod coef;
mod composite;
#[cfg(feature = "icc")]
mod icc;
mod rgb_to_yuv;
mod rgba_f16;
mod tone_map;
mod transfer;
mod working_space;
mod yuv_to_rgb;

pub use affine::{AffineTransform, Rect, RectMap};
pub use composite::{CompositeOver, Crossfade};
#[cfg(feature = "icc")]
pub use icc::{IccIntent, IccTransform};
pub use rgb_to_yuv::RgbToYuv;
pub use rgba_f16::{Rgba8ToRgbaF16, RgbaF16ToRgba8};
pub use tone_map::ToneMapSdr;
pub use working_space::{FromWorking, ToWorking};
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

    /// Issue #126 RT-verification lane: `AllocDisabler` swaps the lib
    /// test binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window. One static per
    /// crate test binary — every module's `rt_process_*` tests measure
    /// against this same allocator; the crate has no `tests/*.rs`
    /// integration binary.
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

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
