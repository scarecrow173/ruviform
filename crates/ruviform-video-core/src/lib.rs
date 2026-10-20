//! `ruviform-video-core` — video data types for the Ruviform media
//! foundation: video frame / plane / pixel formats, strides, color space and
//! HDR metadata, and frame time.
//!
//! Holds no conversion, compositing, or I/O.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). No `path` dependencies are
//! declared yet; they are added by later implementation issues once the APIs
//! are fixed.
//!
//! ## RT constraints
//!
//! Frame references are RT-suitable; allocation and frame pools are non-RT
//! APIs.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "動画データモデル"). Phase 1 skeleton — the public API
//! is not yet defined and is fixed by later implementation issues.

/// Returns the crate version string.
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

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }
}
