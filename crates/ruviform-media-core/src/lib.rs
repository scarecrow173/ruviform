//! `ruviform-media-core` — shared media value types for the Ruviform media
//! foundation: rational time, time bases and PTS; stream / format
//! descriptors; media-wide error types; timecode and other display
//! conversions; transport state value types (playback state and position
//! snapshots).
//!
//! Holds no processing or I/O.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). No `path` dependencies are
//! declared yet; they are added by later implementation issues once the APIs
//! are fixed. `ruviform-media-core` and `ruviform-exec` must not depend on
//! each other (same-tier dependency is prohibited).
//!
//! ## RT constraints
//!
//! Value types only, RT-suitable: types are copyable, allocation-free, and
//! conversions are pure functions.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "時間モデル"). Phase 1 skeleton — the public API is not
//! yet defined and is fixed by later implementation issues.

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_media_core::version(), "");
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
