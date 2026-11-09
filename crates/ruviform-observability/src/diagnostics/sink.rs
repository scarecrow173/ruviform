//! Finding sink — the bounded, subscriber-independent port for
//! important diagnostics events.
//!
//! [`FindingSink`] is an object-safe trait a host plugs in to receive
//! [`DiagnosticsEvent`]s — currently only confirmed
//! [`HealthState`](crate::diagnostics::HealthState) transitions at
//! domain granularity. The diagnostics engine itself never depends on
//! a tracing subscriber or logging backend: with Logging OFF the sink
//! is simply absent and every other diagnostics feature keeps working.
//!
//! The real `tracing` adapter is deferred to OBS-03 (#278); this
//! module provides only the port and the suppression policy. A sink
//! implementation may forward events into `tracing` itself — that
//! dependency lives on the adapter side, never inside this crate's
//! default build.
//!
//! # Bounds
//!
//! [`SinkPolicy`] caps emissions per `window`: events beyond
//! `max_events` inside a window are dropped and counted in
//! [`DiagnosticsHealth::sink_suppressed_total`](crate::diagnostics::DiagnosticsHealth).
//! A panicking sink is isolated via `catch_unwind` and counted in
//! `sink_panics_total` — an observability sink must never take the
//! diagnostics evaluation down with it (and diagnostics never log
//! about its own failure recursively).

use std::time::{Duration, Instant};

use super::health::HealthState;

/// A diagnostics event delivered to a [`FindingSink`].
///
/// Only *important* state changes are delivered — currently domain
/// rollup health transitions. Routine per-rule chatter stays inside
/// the [`DiagnosticSnapshot`](crate::diagnostics::DiagnosticSnapshot).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsEvent {
    /// A domain's rolled-up [`HealthState`] changed between
    /// evaluations.
    HealthTransition {
        /// The domain label.
        domain: &'static str,
        /// Previous state (`Unknown` before the first evaluation).
        from: HealthState,
        /// New state.
        to: HealthState,
        /// Evaluation instant the transition was observed at.
        at: Instant,
    },
}

/// Object-safe sink for [`DiagnosticsEvent`]s.
///
/// `Send` is required so a sink may live on the engine's owning
/// thread while being handed off during setup; `Sync` is deliberately
/// not required — the engine drives `emit` serially from non-RT code.
pub trait FindingSink: Send {
    /// Handles one event. Implementations must not panic-dependent
    /// for correctness — the engine isolates panics — and must keep
    /// `emit` bounded (it runs inside evaluation).
    fn emit(&mut self, event: DiagnosticsEvent);
}

/// Suppression policy for sink emissions — a leaky bucket over a
/// fixed window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SinkPolicy {
    /// Maximum events delivered per `window`.
    pub max_events: u32,
    /// Length of the suppression window. Must be non-zero —
    /// [`DiagnosticsConfig::validate`](crate::diagnostics::DiagnosticsConfig::validate)
    /// rejects zero.
    pub window: Duration,
}

impl Default for SinkPolicy {
    fn default() -> Self {
        Self {
            max_events: 8,
            window: Duration::from_secs(1),
        }
    }
}
