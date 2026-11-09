//! Composed runner health — Issue #293 (OBS-11).
//!
//! [`RunnerHealth`] aggregates the existing per-layer health snapshots
//! into one fixed-size, value-only view. It is **logging-independent**
//! and **non-recursive**: the runner never registers itself as a
//! metric source and never routes health reads through `tracing` or a
//! logging sink — every field is a clone of an already-computed
//! snapshot or a plain counter.
//!
//! Loss stays typed per layer instead of being flattened into one
//! number: collector discontinuities/clock anomalies live in
//! [`CollectorHealth`], export suppression in [`ExportTotals`], RT
//! record loss in the debug [`SessionView`] drain accounting, writer
//! queue/sink failures in the logging [`HealthSnapshot`], and report
//! failures in the diagnostics/debug health counters.

use crate::collector::CollectorHealth;
use crate::debug::{DebugHealth, SessionView};
use crate::diagnostics::DiagnosticsHealth;

use super::runner::RunnerState;

/// Cumulative tracing-export accounting across the runner's lifetime.
///
/// Mirrors [`EmitStats`](crate::tracing_export::EmitStats) field-for-
/// field but accumulates every `emit` call the runner performed —
/// `reconfigure` does **not** reset it (health is the runner's
/// lifetime total; per-configuration values stay on `PollOutcome`).
#[cfg(feature = "tracing-export")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExportTotals {
    /// `TracingExporter::emit` calls completed.
    pub emits_total: u64,
    /// Measurement events actually emitted, total.
    pub emitted_total: u64,
    /// Snapshot-policy suppressions, total.
    pub snapshot_suppressed_total: u64,
    /// `OnIncrease` policy-gate suppressions, total.
    pub policy_suppressed_total: u64,
    /// Valueless `OnIncrease` skips, total.
    pub valueless_skipped_total: u64,
    /// Per-poll-cap drops, total.
    pub cap_dropped_total: u64,
}

#[cfg(feature = "tracing-export")]
impl ExportTotals {
    /// Accumulates one `emit` call's [`EmitStats`].
    pub(crate) fn add(&mut self, stats: crate::tracing_export::EmitStats) {
        self.emits_total += 1;
        self.emitted_total += stats.emitted;
        self.snapshot_suppressed_total += stats.snapshot_suppressed;
        self.policy_suppressed_total += stats.policy_suppressed;
        self.valueless_skipped_total += stats.valueless_skipped;
        self.cap_dropped_total += stats.cap_dropped;
    }
}

/// Point-in-time health of an [`ObservationRunner`].
///
/// Produced on demand by
/// [`ObservationRunner::health`](crate::integration::ObservationRunner::health);
/// every subsystem field is `None` when that subsystem is disabled or
/// the runner is `Unconfigured`. Cheap to produce — no logging, no
/// tracing events, no source registration.
#[derive(Debug, Clone)]
pub struct RunnerHealth {
    /// Runner lifecycle state at read time.
    pub state: RunnerState,
    /// Successful `poll_once` calls across the runner's lifetime.
    pub polls_completed: u64,
    /// `poll_once` calls that failed inside the collector.
    pub poll_errors_total: u64,
    /// Successful `reconfigure` calls.
    pub reconfigures_total: u64,
    /// Completed `finish_shutdown`/`shutdown` calls.
    pub shutdowns_total: u64,
    /// Collector health — poll delay, source/series budgets, loss
    /// counters. `None` while `Unconfigured`.
    pub collector: Option<CollectorHealth>,
    /// Diagnostics health — evaluations, sink panics/suppression,
    /// report rejections. `None` when diagnostics is disabled.
    pub diagnostics: Option<DiagnosticsHealth>,
    /// Debug health — inspections, invariant outcomes, session drain
    /// accounting, report rejections. `None` when debug is disabled.
    pub debug: Option<DebugHealth>,
    /// Debug session view — session state, retained/overflowed/
    /// pending record counts (session capacity). `None` when debug is
    /// disabled.
    pub debug_session: Option<SessionView>,
    /// Cumulative tracing-export accounting. `None` when tracing
    /// export is disabled or the `tracing-export` feature is off.
    #[cfg(feature = "tracing-export")]
    pub export: Option<ExportTotals>,
    /// Logging sink health — queue-full drops, write/flush failures,
    /// channel disconnects. `None` when logging is not attached.
    #[cfg(feature = "logging")]
    pub logging: Option<crate::logging::HealthSnapshot>,
    /// The most recent shutdown/reconfigure-retire drain report.
    pub last_shutdown: Option<crate::integration::ShutdownReport>,
}
