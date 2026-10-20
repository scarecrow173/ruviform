//! `ruviform-audio-core` — audio data types for the Ruviform media
//! foundation: audio buffer / frame / sample formats, interleaved and planar
//! layouts, sample rates, channel layouts, the basic contract for audio
//! time, and the audio device boundary traits (ports).
//!
//! Holds no processing, I/O, or device implementations. `ruviform-audio`
//! (engine) and `ruviform-audio-device` (adapter) connect through the device
//! boundary defined here, structurally eliminating a direct engine ↔
//! adapter dependency.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). No `path` dependencies are
//! declared yet; they are added by later implementation issues once the APIs
//! are fixed.
//!
//! ## RT constraints
//!
//! Buffer types and references are RT-suitable; allocation and growth are
//! non-RT APIs (allocation builders are explicitly labelled for non-RT use).
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "音声データモデル"). Phase 1 skeleton — the public API
//! is not yet defined and is fixed by later implementation issues.

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_audio_core::version(), "");
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
