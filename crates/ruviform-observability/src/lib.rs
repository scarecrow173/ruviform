//! Shared observability primitives and the observation-source
//! registration contract — `core` lightweight primitives plus the
//! `source` descriptor/identity/registration/lifecycle vocabulary
//! fixed by Issue #277 (OBS-02) as the foundation for the Issue #289
//! (OBS-07) collector.
//!
//! # Modules
//!
//! - [`core`]: writer/reader primitive pairs sharing `Arc<Atomic*>`
//!   cells — [`counter`], [`core::gauge`], [`core::histogram`].
//! - [`source`]: metric descriptors, scope, [`source::SourceId`] /
//!   [`source::SourceGeneration`], the [`source::ObservationRegistrar`]
//!   port, [`source::RegistrationLease`] and the
//!   `Active → Retiring → FinalDrain → Removed` lifecycle contract.
//!
//! `counter`, `Counter` and `CounterReader` are re-exported at the
//! crate root for compatibility with existing consumers; new code may
//! use either path.
//!
//! # RT contract
//!
//! - [`Counter::increment`], [`Counter::read`], [`Counter::reader`],
//!   [`CounterReader::read`], [`core::Gauge::set`], [`core::Gauge::read`],
//!   [`core::Gauge::reader`], [`core::GaugeReader::read`]: wait-free,
//!   no-alloc, bounded — `Relaxed` atomic ops / an `Arc` refcount bump
//!   only. These are the methods an RT writer may call.
//! - [`core::Histogram::observe`]: demonstrated no-alloc and bounded
//!   under the `assert_no_alloc` test lane but **not yet declared
//!   RT-safe** — the formal declaration is deferred.
//! - [`counter`], [`core::gauge`], [`core::histogram`] (construction),
//!   `Clone`, `Drop`, [`core::HistogramReader::snapshot`], and every
//!   [`source`] operation (registration, `retire`, lifecycle
//!   transitions, lease drop): **non-RT only** — allocation and/or
//!   registry control flow. RT writers never hold a registrar handle
//!   or registry lock; required readers/metadata are prepared before
//!   RT operation begins.
//!
//! # Read semantics
//!
//! Each `read` is an independent `Relaxed` load. Reading several
//! counters (or several readers) sequentially is **not** a
//! transactionally consistent snapshot — values observed together may
//! come from different instants. Callers needing correlated values
//! must define that invariant themselves; this crate only guarantees
//! each individual counter is monotonically non-decreasing.
//!
//! `u64` wraps modulo 2^64 — at ~5.8×10⁸ increments/second that is
//! unreachable in any realistic configuration, so no saturation policy
//! is provided.
//!
//! # Dependency policy
//!
//! The default build has **no dependencies** (`std` only): no
//! `tracing`, no `tracing-subscriber`, no external I/O or GUI types.
//! The core/source contract must remain usable without any subscriber
//! or logging backend — feature separation alone does not make code
//! RT-safe, so RT suitability is declared per operation, not per
//! module.
//!
//! The opt-in `logging` feature (Issue #276) adds `tracing` /
//! `tracing-subscriber` / `tracing-appender` for host-owned
//! subscriber construction only; it is off by default and never
//! installs a global dispatcher itself.

// Domain vocabulary intentionally repeats module names at the crate
// root re-export surface (`Counter`, `MetricDescriptor`,
// `SourceState`, …), and this `publish = false` workspace-internal
// contract crate relies on literal construction for descriptor
// structs. Every public enum carries enum-level `#[non_exhaustive]`
// per `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

/// Generic host-owned logging adapter on the standard `tracing` stack —
/// Issue #276 (OBS-01). Only available with the `logging` feature, which is
/// off by default so the core/source contract keeps zero external
/// dependencies.
#[cfg(feature = "logging")]
pub mod logging;

pub mod core;
pub mod source;

pub use crate::core::{Counter, CounterReader, counter};

#[cfg(test)]
mod tests {
    /// Issue #130 RT-verification lane: `AllocDisabler` swaps the lib
    /// test binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window. One static per
    /// crate test binary — the `rt_*` tests in the module test
    /// submodules measure against this same allocator; the crate has
    /// no `tests/*.rs` integration binary.
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
}
