//! `ruviform-math` — numeric operations, vectors, SIMD kernels, CPU feature
//! detection, and numeric utilities for the Ruviform media foundation.
//!
//! This crate holds no media-specific concepts, I/O, or threads.
//!
//! ## Allowed dependency layers
//!
//! None (std only). Safe-SIMD backend crates and similar external
//! dependencies are implementation details; their types must not appear in
//! the public API.
//!
//! ## RT constraints
//!
//! All public functions are designed to be RT-suitable: allocation-free and
//! bounded (this crate is a target of the RT contract). The scalar reference
//! implementation is mandatory; SIMD paths are validated against it per
//! kernel.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "DSP／SIMD 設計"). Phase 1 skeleton — the public API is
//! not yet defined and is fixed by later implementation issues.

// Issue #19 DSP/SIMD PoC experiment bridge — see the module docs. Not part
// of the public API contract; may be promoted or removed by the PoC ADR.
#[doc(hidden)]
pub mod simd_poc;

/// SIMD-dispatched DSP kernels — Issue #53 production dispatch layer.
/// See the module docs for the precision contract and RT guarantees.
pub mod dsp;

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_math::version(), "");
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
