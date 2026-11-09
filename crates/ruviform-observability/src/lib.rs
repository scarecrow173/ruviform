//! Shared observability primitives and the observation-source
//! registration contract — `core` lightweight primitives plus the
//! `source` descriptor/identity/registration/lifecycle vocabulary
//! fixed by Issue #277 (OBS-02) as the foundation for the Issue #289
//! (OBS-07) collector, the fixed-size record vocabulary of Issue #280
//! (OBS-05), and the bounded RT → non-RT record bridge that carries it.
//!
//! # Modules
//!
//! - [`core`]: writer/reader primitive pairs sharing `Arc<Atomic*>`
//!   cells — [`counter`], [`core::gauge`], [`core::histogram`].
//! - [`source`]: metric descriptors, scope, [`source::SourceId`] /
//!   [`source::SourceGeneration`], the [`source::ObservationRegistrar`]
//!   port, [`source::RegistrationLease`] and the
//!   `Active → Retiring → FinalDrain → Removed` lifecycle contract.
//! - [`record`]: the fixed-size record vocabulary — [`record::ClockContext`]
//!   (clock-domain stamps), [`record::CorrelationBook`] / link/token
//!   (bounded correlation), and [`record::EventRecord`].
//! - [`rt_bridge`]: [`rt_bridge::bridge`] — a bounded SPSC record
//!   bridge over [`ruviform_rt::channel`] with drop-on-full loss
//!   accounting (`recorded`/`dropped` counters and per-record
//!   `dropped_before`).
//! - [`rt_timing`]: measured-interval observation for real-time code —
//!   [`rt_timing::timing_bridge`] prepares a [`TimingRecorder`](rt_timing::TimingRecorder)
//!   (RT side: alloc/lock/panic-free `record`) and a
//!   [`TimingSink`](rt_timing::TimingSink) (non-RT side: per-poll
//!   [`TimingWindow`](rt_timing::TimingWindow), cumulative
//!   [`TimingSnapshot`](rt_timing::TimingSnapshot), fixed-bucket
//!   duration histogram for OBS-07, clock-anomaly/budget/loss
//!   accounting) — Issue #290 (OBS-08) RT Timing slice.
//! - [`metrics`]: the shared measurement vocabulary — typed values
//!   (`Counter`/`Gauge`/`Histogram`), [`metrics::Measurement`] and its
//!   typed [`metrics::MeasurementValidity`]/issue/loss semantics.
//! - [`collector`]: the authoritative non-RT observation path —
//!   [`collector::Collector`] (single poll owner),
//!   [`collector::CollectorRegistrar`] (the OBS-02 registrar port),
//!   [`collector::CollectorHandle`] (shareable snapshot/history/
//!   health reads), [`collector::ObservationBatch`],
//!   [`collector::MetricsSnapshot`] and [`collector::CollectorHealth`].
//! - `tracing_export` (feature `tracing-export` only): the opt-in non-RT
//!   `tracing` conversion layer — `TracingExporter` turns
//!   [`collector::ObservationBatch`] measurements into `tracing` events
//!   under a fixed schema, and `TracingFindingSink` adapts
//!   [`diagnostics::FindingSink`] events (Issue #278, OBS-03). The host
//!   injects the `tracing::Dispatch`.
//! - [`diagnostics`]: the logging-independent diagnostic layer —
//!   classified [`diagnostics::Evidence`], [`diagnostics::DiagnosticFinding`],
//!   [`diagnostics::HealthState`]/[`diagnostics::HealthEvaluator`],
//!   bounded [`diagnostics::DiagnosticSnapshot`]/
//!   [`diagnostics::DiagnosticReport`], and the
//!   [`diagnostics::FindingSink`] port (Issue #291, OBS-09).
//! - `integration` *(feature `integration` only)*: the host-facing
//!   non-RT composition layer — [`integration::ObservabilityConfig`]
//!   per-subsystem settings and [`integration::ObservationRunner`]
//!   lifecycle (`Unconfigured → Prepared → Ready → Running →
//!   Draining → Stopped`), ordered drain, dynamic reconfigure, and
//!   composed [`integration::RunnerHealth`] (Issue #293, OBS-11).
//! - [`debug`]: the bounded, non-RT, disclosure-authorized debug
//!   layer — [`debug::InspectionBudget`] limits,
//!   [`debug::InspectionSnapshot`] sections with typed
//!   [`debug::SectionStatus`]es, three-valued
//!   [`debug::InvariantOutcome`] checks published into diagnostics,
//!   the [`debug::ProbeSource`] port over Generic Probe vocabulary,
//!   the [`debug::DebugSessionState`] lifecycle with bounded drain,
//!   and [`debug::DebugReport`] exports (Issue #292, OBS-10).
//! - `profiling` *(feature `allocation-probe` only)*: development/test
//!   allocation probe — `CountingAllocator` and the thread-local
//!   `AllocationScope` measurement windows.
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
//! - [`rt_bridge::RtRecorder::try_record`], [`rt_bridge::RtRecorder::dropped`],
//!   [`rt_bridge::RtRecorder::recorded`]: bounded, non-blocking, no-alloc —
//!   one `try_push` plus `Relaxed` counter ops. These are the bridge
//!   operations an RT writer may call.
//! - [`rt_timing::TimingRecorder::record`], [`rt_timing::TimingRecorder::dropped`],
//!   [`rt_timing::TimingRecorder::recorded`]: RT-suitable — a `const`
//!   header build plus the inner bridge's bounded `try_push`; no alloc,
//!   no lock, no panic path, no tracing/subscriber call, no I/O.
//!   [`rt_timing::timing_bridge`] construction and every
//!   [`rt_timing::TimingSink`] operation (`poll`, `snapshot`, reader
//!   accessors) are **non-RT only**.
//! - [`record`] value types ([`record::ClockContext`],
//!   [`record::CorrelationLink`], [`record::EventRecord`],
//!   [`record::RecordHeader`]): `Copy` value types with `const` accessors
//!   — construction and reads are RT-suitable.
//! - [`counter`], [`core::gauge`], [`core::histogram`] (construction),
//!   `Clone`, `Drop`, [`core::HistogramReader::snapshot`],
//!   [`rt_bridge::bridge`], every [`rt_bridge::RecordSink`] operation
//!   (`drain`, `drain_final`, counter readers — including the sink `Drop`
//!   that frees the ring), [`record::CorrelationBook`] operations, and
//!   every [`source`] operation (registration, `retire`, lifecycle
//!   transitions, lease drop): **non-RT only** — allocation and/or
//!   registry control flow. RT writers never hold a registrar handle
//!   or registry lock; required readers/metadata are prepared before
//!   RT operation begins. Endpoint `Drop`s are likewise non-RT — never
//!   drop a recorder or sink on an RT thread.
//! - Every [`collector`] operation — `poll`, `register`, `snapshot`,
//!   `latest_batch`, `series`, `history`, `health`, `shutdown`,
//!   [`collector::PollRunner`]: **non-RT only** — locking and
//!   allocation throughout. The collector's registry lock is never
//!   held on an RT writer path.
//! - Every [`diagnostics`] operation — evidence/finding recording,
//!   rule declaration, `evaluate`, `report`, `health`: **non-RT only** —
//!   allocation throughout (vectors, maps, strings, `catch_unwind`
//!   around sink calls).
//! - Every [`debug`] operation — `inspect`, `report`, session
//!   control, invariant evaluation, probe-source reads: **non-RT
//!   only** — allocation throughout. Debug exposes no RT-suitable
//!   operations; only its `Copy` value types are RT-adjacent.
//! - `profiling` *(feature `allocation-probe` only)* declares its
//!   own per-operation RT contract in the module docs: the
//!   `GlobalAlloc` methods and scope enter/exit/stats/reset add no
//!   allocation, lock, or panic beyond the wrapped allocator `A`;
//!   scope construction is non-RT.
//! - Every `integration` operation — `prepare`, `activate`,
//!   `poll_once`, `begin_shutdown`/`finish_shutdown`/`shutdown`,
//!   `reconfigure`, `attach_logging`, `health`, and every accessor:
//!   **non-RT only** — allocation and locking throughout, and the
//!   runner spawns no thread of its own.
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
//! The default build has **no external (crates.io) dependencies**: no
//! `tracing`, no `tracing-subscriber`, no external I/O or GUI types.
//! Since Issue #280 the crate depends on two leaf primitives —
//! `ruviform-rt` (the bounded SPSC ring the bridge reuses) and
//! `ruviform-media-core` (the single clock-domain vocabulary
//! `ClockContext` composes) — strictly one-directional and downward;
//! the reverse direction must never exist
//! (`docs/adr/obs-05-observability-leaf-primitive-deps.md`). The
//! core/source contract must remain usable without any subscriber
//! or logging backend — feature separation alone does not make code
//! RT-safe, so RT suitability is declared per operation, not per
//! module.
//!
//! The opt-in `logging` feature (Issue #276) adds `tracing` /
//! `tracing-subscriber` / `tracing-appender` for host-owned
//! subscriber construction only; it is off by default and never
//! installs a global dispatcher itself. The opt-in
//! `allocation-probe` feature (Issue #290) adds the `profiling`
//! module and **no dependencies** — it is std-only.

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

/// Opt-in `tracing` export layer — Issue #278 (OBS-03). Only available
/// with the `tracing-export` feature, which enables `dep:tracing` alone;
/// subscriber construction stays host-owned (typically via the `logging`
/// feature's `Dispatch`).
#[cfg(feature = "tracing-export")]
pub mod tracing_export;

pub mod collector;
pub mod core;
pub mod debug;
pub mod diagnostics;
/// Host-facing non-RT integration layer — Issue #293 (OBS-11).
/// Only available with the `integration` feature, which is off by
/// default so the lightweight core stays minimal; it adds no
/// dependencies and leaves `tracing-export`/`logging` optional.
#[cfg(feature = "integration")]
pub mod integration;
pub mod metrics;
/// Development/test allocation probe — Issue #290 (OBS-08).
/// Only available with the `allocation-probe` feature, which is off by
/// default so shipped builds carry no instrumentation.
#[cfg(feature = "allocation-probe")]
pub mod profiling;
pub mod record;
pub mod rt_bridge;
pub mod rt_timing;
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
