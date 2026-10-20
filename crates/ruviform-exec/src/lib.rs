//! `ruviform-exec` — non-RT execution infrastructure for the Ruviform media
//! foundation: thread pool, job parallelism, progress / cancellation tokens,
//! and batch driving.
//!
//! Holds no RT primitives.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). No `path` dependencies are
//! declared yet; they are added by later implementation issues once the APIs
//! are fixed. `ruviform-exec` and `ruviform-media-core` must not depend on
//! each other (same-tier dependency is prohibited).
//!
//! ## RT constraints
//!
//! Non-RT only. Calling this crate from an RT path is a contract violation.
//! The non-RT driver mints `RtContext` tokens to batch- and
//! parallel-drive `Processor` kernels offline.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "RT／非 RT の分離方式"). Phase 1 skeleton — the public
//! API is not yet defined and is fixed by later implementation issues.

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_exec::version(), "");
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
