//! `ruviform-exec` — non-RT execution infrastructure for the Ruviform media
//! foundation: thread pool, job parallelism, progress / cancellation tokens,
//! and batch driving.
//!
//! Holds no RT primitives. As part of its "hide OS thread details"
//! responsibility it owns the thread-priority mechanism —
//! [`promote_current_thread_to_realtime`],
//! [`demote_current_thread_from_realtime`], [`ThreadPriorityParams`],
//! [`ThreadPriorityGuard`] and [`PriorityError`] — exposing OS-provided
//! real-time scheduling (Windows MMCSS, macOS time-constraint policy,
//! Linux `SCHED_FIFO`) as a `non-RT only` self-promotion API.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). No `path` dependencies are
//! declared yet; they are added by later implementation issues once the APIs
//! are fixed. `audio_thread_priority` is a private crates.io implementation
//! detail — no type, error or feature name of it appears in the public API.
//! `ruviform-exec` and `ruviform-media-core` must not depend on each other
//! (same-tier dependency is prohibited).
//!
//! ## RT constraints
//!
//! Non-RT only. Calling this crate from an RT path is a contract violation.
//! In particular `promote_*` / `demote_*` and `ThreadPriorityGuard`
//! creation / destruction perform OS syscalls: promotion must run before the
//! RT contract interval begins (before `RtContext` is minted and before the
//! first `process` call). The non-RT driver that mints `RtContext` tokens to
//! batch- and parallel-drive `Processor` kernels offline arrives with later
//! implementation issues.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "RT／非 RT の分離方式") and
//! `docs/adr/rt-foundation-primitives-implementation.md`.

// The public names below are the fixed API shape of the thread-priority
// mechanism: `ThreadPriority*` deliberately echoes its module
// (`thread_priority::ThreadPriorityParams`) while being re-exported flat at
// the crate root, mirroring the `ruviform-rt` pattern. Every public enum
// carries enum-level `#[non_exhaustive]` per
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]

mod thread_priority;

pub use thread_priority::{
    PriorityError, ThreadPriorityGuard, ThreadPriorityParams, demote_current_thread_from_realtime,
    promote_current_thread_to_realtime,
};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
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
