//! `ruviform-rt` — real-time contract definitions and RT-safe primitives for
//! the Ruviform media foundation.
//!
//! Owns the `Processor` trait (the `prepare` / `process` separated processing
//! contract), the [`RtContext`] capability, the `MasterClock` time-sharing
//! primitive, and RT-safe primitives such as SPSC rings, drop-offload
//! channels, immutable snapshot swapping, and pre-allocated handoff
//! structures. Holds no scheduling or I/O.
//!
//! ## Allowed dependency layers
//!
//! None. External synchronization crates (e.g. `rtrb`) are implementation
//! details; their types must not be exposed in the public API.
//!
//! ## RT constraints
//!
//! This crate is the contract definition site. Every public API is either
//! RT-suitable (no-alloc, non-blocking) or explicitly labelled non-RT-only.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "RT／非 RT の分離方式", "RT 契約"). Phase 1 skeleton —
//! only the [`RtContext`] token exists so far; `Processor`, `MasterClock`,
//! and the RT primitives are defined by later implementation issues.

/// Capability token marking that the caller is inside an RT contract
/// section.
///
/// `RtContext` is the `prepare` / `process` boundary of the `Processor`
/// contract: `prepare(setup)` runs in the non-RT section (allocation
/// allowed) and produces the processing object whose `process(&mut self,
/// io, ctx)` API requires this token.
///
/// Issuance rule: `RtContext` can only be minted by a driver that has
/// started an RT contract section — the RT driver (`ruviform-audio`, inside
/// the `ruviform-audio-device` device callback) or the offline driver
/// (`ruviform-exec`'s non-RT driver, where no-alloc constraints do not
/// apply). Application code must not be able to construct it, so this type
/// intentionally has no public constructor; mint APIs are exposed only at
/// the driver entry points.
///
/// Placeholder for Issue #17: the fields and mint API arrive with the RT
/// contract implementation issue.
#[derive(Debug)]
pub struct RtContext {
    _private: (),
}

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_rt::version(), "");
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
