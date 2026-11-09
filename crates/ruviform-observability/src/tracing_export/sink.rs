//! [`TracingFindingSink`] — adapts [`DiagnosticsEvent`]s into `tracing`
//! events on a host-injected [`Dispatch`].
//!
//! This is the OBS-03 (#278) implementation deferred from
//! [`crate::diagnostics::sink`]: a host plugs it into
//! [`crate::diagnostics::Diagnostics::set_sink`] and health
//! transitions flow into the same `tracing` pipeline as measurement
//! events — with Logging OFF the sink is simply never installed and
//! diagnostics keeps working unchanged.
//!
//! # Failure isolation
//!
//! Emission is wrapped in `catch_unwind`: a panicking subscriber path
//! is counted in [`Self::panics`] and swallowed — the diagnostics
//! engine is never taken down by its own observability sink, and the
//! sink never recursively logs about its own failure.
//!
//! Non-RT only: [`FindingSink::emit`] allocates (`String`, scoped
//! dispatch) — the diagnostics engine already drives sinks from
//! non-RT evaluation code.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use tracing::{Dispatch, dispatcher};

use crate::diagnostics::{DiagnosticsEvent, FindingSink};

use super::schema::{
    EVENT_CODE_HEALTH_TRANSITION, EVENT_VERSION, TARGET, health_state_str, health_transition_level,
    mono_us_since,
};

/// A [`FindingSink`] that forwards [`DiagnosticsEvent`]s to `tracing`.
///
/// The host owns subscriber/dispatch selection — typically the same
/// `Dispatch` handed to [`super::TracingExporter`]. Install via
/// [`crate::diagnostics::Diagnostics::set_sink`]:
///
/// ```
/// use ruviform_observability::diagnostics::Diagnostics;
/// use ruviform_observability::tracing_export::TracingFindingSink;
///
/// fn install(diagnostics: &mut Diagnostics, dispatch: tracing::Dispatch) {
///     diagnostics.set_sink(Box::new(TracingFindingSink::new(dispatch)));
/// }
/// ```
pub struct TracingFindingSink {
    dispatch: Dispatch,
    /// First event instant — origin of `at_mono_us`, matching the
    /// exporter's relative-time convention.
    t0: Option<Instant>,
    /// Emissions that panicked inside `catch_unwind`.
    panics: u64,
}

impl TracingFindingSink {
    /// Creates a sink emitting on `dispatch`.
    #[must_use]
    pub fn new(dispatch: Dispatch) -> Self {
        Self {
            dispatch,
            t0: None,
            panics: 0,
        }
    }

    /// Number of emissions isolated by `catch_unwind` since creation.
    #[must_use]
    pub fn panics(&self) -> u64 {
        self.panics
    }

    fn emit_inner(&mut self, event: DiagnosticsEvent) {
        match event {
            DiagnosticsEvent::HealthTransition {
                domain,
                from,
                to,
                at,
            } => {
                let t0 = *self.t0.get_or_insert(at);
                let level = health_transition_level(to);
                // `tracing::event!` callsites carry a static level —
                // dispatch through one callsite per level.
                macro_rules! go {
                    ($lvl:expr) => {
                        dispatcher::with_default(&self.dispatch, || {
                            tracing::event!(
                                target: TARGET,
                                $lvl,
                                event_code = EVENT_CODE_HEALTH_TRANSITION,
                                event_version = EVENT_VERSION,
                                domain = domain,
                                from_state = health_state_str(from),
                                to_state = health_state_str(to),
                                at_mono_us = mono_us_since(at, t0),
                            );
                        })
                    };
                }
                match level {
                    tracing::Level::INFO => go!(tracing::Level::INFO),
                    tracing::Level::WARN => go!(tracing::Level::WARN),
                    // `health_transition_level` only yields
                    // INFO/WARN/ERROR; anything else fails safe upward.
                    _ => go!(tracing::Level::ERROR),
                }
            }
        }
    }
}

impl FindingSink for TracingFindingSink {
    /// Forwards one [`DiagnosticsEvent`] as a `tracing` event.
    ///
    /// Panics raised while dispatching are caught and counted — the
    /// diagnostics engine is unaffected and no recursive logging of
    /// the failure is attempted.
    fn emit(&mut self, event: DiagnosticsEvent) {
        if catch_unwind(AssertUnwindSafe(|| self.emit_inner(event))).is_err() {
            self.panics += 1;
        }
    }
}
