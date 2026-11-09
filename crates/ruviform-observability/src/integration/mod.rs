//! Host-facing non-RT integration layer — Issue #293 (OBS-11).
//!
//! This module is the single composition boundary a consumer Host
//! (AudioLens, Synvelis, …) uses to build and operate the observability
//! stack: [`ObservabilityConfig`] for per-subsystem settings,
//! [`ObservationRunner`] for the `Unconfigured → Prepared → Ready →
//! Running → Draining → Stopped` lifecycle, ordered drain, dynamic
//! reconfiguration, and a composed [`RunnerHealth`].
//!
//! # What it composes
//!
//! - [`collector::Collector`] — the authoritative non-RT poll owner
//!   (single `poll(now)` per cycle).
//! - [`diagnostics::Diagnostics`] — optional evidence/finding engine,
//!   evaluated once per poll against the committed snapshot.
//! - [`debug::DebugInspector`] — optional bounded inspection/debug
//!   sessions (host drives sessions via `debug_mut()` and permits
//!   minted from the configured [`DebugAuthority`](crate::debug::DebugAuthority)).
//! - [`tracing_export::TracingExporter`] — optional, `tracing-export`
//!   feature only; emits each batch on a **host-injected**
//!   `tracing::Dispatch`.
//! - logging [`WriterGuard`](crate::logging::WriterGuard)s —
//!   optional, `logging` feature only, attached via
//!   [`ObservationRunner::attach_logging`]; the host keeps dispatch
//!   installation and `LoggingConfig` ownership (OBS-01).
//!
//! Nothing here is reimplemented: every subsystem keeps its own
//! invariants and the runner only sequences their public APIs.
//!
//! # Contracts
//!
//! - **Non-RT only.** Every runner API allocates/locks; no RT thread
//!   may call them. The runner itself spawns no thread — the host
//!   drives [`ObservationRunner::poll_once`] at
//!   [`poll_interval`](ObservationRunner::poll_interval) cadence.
//! - **No implicit global subscriber.** The library never calls
//!   `set_global_default`/`init`; tracing emission is scoped to the
//!   injected `Dispatch` and logging dispatch installation is
//!   host-owned.
//! - **Honest drain.** Shutdown order is final poll → collector
//!   terminal drain → exporter drain → writer flush/close. Unconfirmed
//!   quiescence surfaces as `Partial`/`WriterNotQuiesced` in the
//!   [`ShutdownReport`] — complete preservation is never claimed.
//! - **Independent subsystems.** `diagnostics: None`, `debug: None`,
//!   no export dispatch, and no logging attachment all yield a
//!   working Collector-only runner.
//! - **Logger-independent health.** [`RunnerHealth`] composes existing
//!   snapshots and plain counters; it never registers itself as a
//!   source and never routes through logging/tracing (no recursion).
//!
//! # Feature composition
//!
//! `integration` adds this module with **no new dependencies** and
//! does not force `tracing-export` or `logging`:
//!
//! | features | runner capability |
//! | --- | --- |
//! | `integration` | Collector (+ optional diagnostics/debug) poll, drain, health |
//! | `+ tracing-export` | per-poll batch export + optional diagnostics finding sink on the injected dispatch |
//! | `+ logging` | `attach_logging` writer guards + sink health in `RunnerHealth` |
//!
//! Feature unification is **not** an isolation guarantee — enabling
//! `logging` still leaves dispatch installation to the host.

mod config;
mod health;
mod runner;
#[cfg(test)]
mod tests;

#[cfg(feature = "tracing-export")]
pub use config::ExportPolicy;
pub use config::{ConfigError, DebugSettings, ObservabilityConfig};
#[cfg(feature = "tracing-export")]
pub use health::ExportTotals;
pub use health::RunnerHealth;
pub use runner::{
    DrainCompleteness, NonMigrated, ObservationRunner, PollOutcome, ReconfigureReport, RunnerError,
    RunnerState, ShutdownReport,
};
