//! `ObservationRunner` — the host-facing non-RT lifecycle and poll
//! driver — Issue #293 (OBS-11).
//!
//! The runner composes the existing observability subsystems through
//! their public APIs only: it owns one [`Collector`], optionally one
//! [`Diagnostics`] engine, one [`DebugInspector`], and (with the
//! `tracing-export` feature) one [`TracingExporter`]. It adds the
//! missing integration concerns — a six-state lifecycle, ordered
//! shutdown drain, dynamic reconfiguration, and a composed
//! [`RunnerHealth`] — without reimplementing any subsystem.
//!
//! # Lifecycle
//!
//! `Unconfigured → Prepared → Ready → Running → Draining → Stopped`
//! is a one-way state machine; every transition is validated and
//! illegal transitions return [`RunnerError::InvalidState`]. `Degraded`
//! is a health condition reported through [`RunnerHealth`], never a
//! lifecycle state.
//!
//! # Poll model
//!
//! The runner owns **no thread and no timer**. The host calls
//! [`poll_once`](ObservationRunner::poll_once) from non-RT code at the
//! configured [`poll_interval`](ObservationRunner::poll_interval)
//! cadence; each call runs `Collector::poll(now)` →
//! `Diagnostics::evaluate` → `TracingExporter::emit` in order and
//! returns a [`PollOutcome`]. All runner APIs are non-RT only —
//! allocation, locking, and thread-adjacent work throughout.
//!
//! # Shutdown order
//!
//! [`begin_shutdown`](ObservationRunner::begin_shutdown) declares
//! `Draining` so the host can stop domain writers and confirm
//! quiescence; [`finish_shutdown`](ObservationRunner::finish_shutdown)
//! (or the one-shot [`shutdown`](ObservationRunner::shutdown)) then
//! runs, in order: a final collector poll → `Collector::shutdown`
//! (terminal drain batch) → terminal-batch export → logging
//! `WriterGuard` drop (flush/close). Sources still `Active` at drain
//! are force-drained by the collector as `Partial` +
//! `WriterNotQuiesced` and reported as such — the runner never claims
//! complete preservation it cannot prove.
//!
//! # Dynamic configuration
//!
//! [`reconfigure`](ObservationRunner::reconfigure) performs
//! Validate → Prepare → Activate → Retire for a new
//! [`ObservabilityConfig`]: validation failure or build failure
//! leaves the running configuration untouched; on success the new
//! components swap in and the old collector is drained (terminal
//! batch emitted through the old exporter) before being dropped.
//! Source registrations, diagnostics retention, debug sessions, and
//! exporter baselines do **not** migrate — the
//! [`ReconfigureReport`] says so explicitly.
//!
//! # Dispatch ownership
//!
//! The library never calls `set_global_default`/`init`. A
//! `tracing::Dispatch` is injected by the host through
//! [`ExportPolicy::dispatch`](crate::integration::ExportPolicy);
//! the exporter scopes every emit to it. Global setup, scoped guards,
//! and guard lifetimes are entirely the host's concern.
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "integration")] {
//! use std::time::Duration;
//! use ruviform_observability::collector::CollectorConfig;
//! use ruviform_observability::integration::{ObservabilityConfig, ObservationRunner};
//!
//! let mut runner = ObservationRunner::new();
//! runner.prepare(&ObservabilityConfig {
//!     collection: CollectorConfig {
//!         poll_interval: Duration::from_millis(100),
//!         ..CollectorConfig::default()
//!     },
//!     diagnostics: None,
//!     debug: None,
//!     #[cfg(feature = "tracing-export")]
//!     export: Default::default(),
//! })?;
//! runner.activate()?;
//!
//! // Host loop (non-RT thread):
//! // while running { runner.poll_once(Instant::now())?; sleep(interval); }
//!
//! // Normal stop — one-shot (or begin_shutdown + domain quiescence +
//! // finish_shutdown for the staged path):
//! let report = runner.shutdown(std::time::Instant::now())?;
//! assert_eq!(report.completeness, ruviform_observability::integration::DrainCompleteness::NoSources);
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::time::{Duration, Instant};

use crate::collector::{
    Collector, CollectorHandle, CollectorRegistrar, MetricsSnapshot, ObservationBatch, PollError,
};
use crate::debug::{DebugInspector, DebugSessionState};
use crate::diagnostics::{DiagnosticSnapshot, Diagnostics, DiagnosticsConfigError};
use crate::metrics::{Measurement, MeasurementIssue};
use crate::source::SourceIdentity;

#[cfg(feature = "tracing-export")]
use crate::tracing_export::{EmitStats, TracingExporter, TracingFindingSink};

#[cfg(feature = "logging")]
use crate::logging::{LoggingHealth, WriterGuard};

use super::config::{ConfigError, ObservabilityConfig};
#[cfg(feature = "tracing-export")]
use super::health::ExportTotals;
use super::health::RunnerHealth;

/// Runner lifecycle state — one-way `Unconfigured → Prepared →
/// Ready → Running → Draining → Stopped`.
///
/// `Degraded` is deliberately absent: degradation is a health
/// condition surfaced through [`RunnerHealth`], not a lifecycle
/// state.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunnerState {
    /// No configuration has been prepared — `ObservationRunner::new`.
    #[default]
    Unconfigured,
    /// Config validated and components built; not yet activated.
    Prepared,
    /// Activated and awaiting the first poll.
    Ready,
    /// At least one `poll_once` has committed.
    Running,
    /// `begin_shutdown` called — domain quiescence window; polls and
    /// reconfiguration are rejected until `finish_shutdown`.
    Draining,
    /// Terminal — drain report available via `shutdown_report()`.
    Stopped,
}

impl std::fmt::Display for RunnerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unconfigured => f.write_str("unconfigured"),
            Self::Prepared => f.write_str("prepared"),
            Self::Ready => f.write_str("ready"),
            Self::Running => f.write_str("running"),
            Self::Draining => f.write_str("draining"),
            Self::Stopped => f.write_str("stopped"),
        }
    }
}

/// Result of one [`ObservationRunner::poll_once`] — the committed
/// batch plus the per-subsystem outputs of that same poll.
#[derive(Debug)]
pub struct PollOutcome {
    /// The committed [`ObservationBatch`] (generation, window,
    /// measurements, removals, snapshot).
    pub batch: ObservationBatch,
    /// The diagnostics evaluation against this poll's snapshot —
    /// `None` when diagnostics is disabled.
    pub diagnostics: Option<DiagnosticSnapshot>,
    /// The exporter's per-call accounting — `None` when tracing
    /// export is disabled (`tracing-export` feature only).
    #[cfg(feature = "tracing-export")]
    pub emit: Option<EmitStats>,
}

/// How completely the drained sources' final state was recovered.
///
/// The runner reports what it measured — it never upgrades an
/// unconfirmed drain into a "complete" claim.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainCompleteness {
    /// Every registered source had completed quiescence (`retire`)
    /// before the drain — final values are authoritative.
    AllQuiesced,
    /// Sources were force-drained without confirmed writer
    /// quiescence — `count` measurements reported `Partial` +
    /// `WriterNotQuiesced`.
    PartialDrains {
        /// Measurements reported with `WriterNotQuiesced`.
        count: u64,
    },
    /// No source was ever registered — nothing to recover.
    NoSources,
}

/// Measured result of one shutdown drain (or one reconfigure retire).
///
/// All fields are counts/generations observed at drain time; the
/// report never asserts persistence the collector could not confirm.
#[derive(Debug, Clone)]
pub struct ShutdownReport {
    /// Polls the collector completed for this configuration
    /// (including the final pre-shutdown poll).
    pub polls_completed: u64,
    /// `poll_once` calls that failed inside the collector for this
    /// configuration.
    pub poll_errors: u64,
    /// Generation of the extra final poll issued immediately before
    /// `Collector::shutdown` — `None` when that poll failed.
    pub final_poll_generation: Option<u64>,
    /// Generation of the terminal drain batch committed by
    /// `Collector::shutdown`.
    pub terminal_generation: Option<u64>,
    /// Measurements carried by the terminal batch.
    pub measurements_drained: u64,
    /// Sources listed in the terminal batch's `removed` set.
    pub removed_total: u64,
    /// Measurements drained as `Partial` + `WriterNotQuiesced`.
    pub partial_drains: u64,
    /// `Active` sources still un-quiesced when the drain began.
    pub sources_active_at_drain: usize,
    /// `Retiring`/`FinalDrain` sources still awaiting drain completion.
    pub sources_retiring_at_drain: usize,
    /// Debug session records (retained + in-flight) discarded with
    /// the inspector — `0` when debug is disabled.
    pub debug_records_lost: u64,
    /// The terminal batch's export accounting — `None` when tracing
    /// export is disabled.
    #[cfg(feature = "tracing-export")]
    pub emit: Option<EmitStats>,
    /// Whether attached logging writers were flushed/closed during
    /// this drain (`false` for a reconfigure retire — logging is
    /// runner-level and stays open).
    pub writers_closed: bool,
    /// A debug session was in a non-`Disabled` state when the
    /// inspector was dropped — its retained records are gone.
    pub debug_session_abandoned: bool,
    /// Diagnostics evidence entries discarded with the engine.
    pub diagnostics_evidence_lost: usize,
    /// Diagnostics findings discarded with the engine.
    pub diagnostics_findings_lost: usize,
    /// Recovery completeness — see [`DrainCompleteness`].
    pub completeness: DrainCompleteness,
}

impl ShutdownReport {
    /// An empty report for the (defensive) no-components drain path.
    fn empty() -> Self {
        Self {
            polls_completed: 0,
            poll_errors: 0,
            final_poll_generation: None,
            terminal_generation: None,
            measurements_drained: 0,
            removed_total: 0,
            partial_drains: 0,
            sources_active_at_drain: 0,
            sources_retiring_at_drain: 0,
            debug_records_lost: 0,
            #[cfg(feature = "tracing-export")]
            emit: None,
            writers_closed: false,
            debug_session_abandoned: false,
            diagnostics_evidence_lost: 0,
            diagnostics_findings_lost: 0,
            completeness: DrainCompleteness::NoSources,
        }
    }
}

/// State that does **not** migrate across `reconfigure`.
///
/// Reported explicitly so hosts never assume a configuration swap is
/// a transparent hot-reload.
#[derive(Debug, Clone)]
pub struct NonMigrated {
    /// Sources that were live at retire time — domains must
    /// re-register against the new `registration_port()`.
    pub sources_requiring_reregistration: usize,
    /// Diagnostics evidence entries discarded with the old engine.
    pub diagnostics_evidence_lost: usize,
    /// Diagnostics findings discarded with the old engine.
    pub diagnostics_findings_lost: usize,
    /// A debug session was abandoned with the old inspector.
    pub debug_session_abandoned: bool,
    /// Debug session records (retained + in-flight) lost with it.
    pub debug_records_lost: u64,
    /// `true` when both the retired and the new configuration had a
    /// tracing exporter — the new exporter re-baselines on its first
    /// poll, so the first post-reconfigure `OnIncrease` observations
    /// are suppressed as baselines rather than emitted.
    pub exporter_baselines_reset: bool,
}

/// Result of [`ObservationRunner::reconfigure`].
#[derive(Debug)]
pub struct ReconfigureReport {
    /// Drain outcome of the retired configuration.
    pub retired: ShutdownReport,
    /// What did not carry over into the new configuration.
    pub non_migrated: NonMigrated,
}

/// Integration-layer failure — state violations, config validation,
/// and collector poll errors.
#[non_exhaustive]
#[derive(Debug)]
pub enum RunnerError {
    /// The operation is not legal in the runner's current
    /// [`RunnerState`]. `operation` names the rejected call.
    InvalidState {
        /// State observed when the call was rejected.
        state: RunnerState,
        /// The rejected operation name.
        operation: &'static str,
    },
    /// [`ObservabilityConfig::validate`] or a component constructor
    /// rejected the supplied configuration.
    InvalidConfig(ConfigError),
    /// The collector rejected `poll` (only
    /// [`PollError::CollectorStopped`] exists today).
    Collector(PollError),
    /// `attach_logging` was called while logging was already
    /// attached. (`logging` feature only.)
    #[cfg(feature = "logging")]
    AlreadyAttached,
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidState { state, operation } => {
                write!(f, "{operation} is not allowed while runner is {state}")
            }
            Self::InvalidConfig(err) => write!(f, "{err}"),
            Self::Collector(err) => write!(f, "collector poll failed: {err}"),
            #[cfg(feature = "logging")]
            Self::AlreadyAttached => f.write_str("logging is already attached"),
        }
    }
}

impl std::error::Error for RunnerError {}

/// The composed subsystem set owned by the runner for one
/// configuration.
struct Components {
    collector: Collector,
    diagnostics: Option<Diagnostics>,
    debug: Option<DebugInspector>,
    #[cfg(feature = "tracing-export")]
    exporter: Option<TracingExporter>,
    latest_diagnostics: Option<DiagnosticSnapshot>,
    poll_interval: Duration,
    poll_errors: u64,
}

/// Pre-drop accounting the drain collects beyond the public report —
/// used by `reconfigure` for [`NonMigrated`].
struct DrainExtras {
    /// Whether the retired set had a tracing exporter (its baselines
    /// are gone after the swap).
    #[cfg(feature = "tracing-export")]
    had_exporter: bool,
}

impl Components {
    /// Builds the component set for `config`. Assumes
    /// `config.validate()` already ran — constructor errors are still
    /// mapped honestly.
    fn build(config: &ObservabilityConfig) -> Result<Self, RunnerError> {
        let collector = Collector::new(config.collection.clone())
            .map_err(|err| RunnerError::InvalidConfig(err.into()))?;
        let diagnostics = config
            .diagnostics
            .clone()
            .map(Diagnostics::new)
            .transpose()
            .map_err(|err: DiagnosticsConfigError| {
                RunnerError::InvalidConfig(ConfigError::Diagnostics(err))
            })?;

        #[cfg(feature = "tracing-export")]
        let exporter = config.export.dispatch.as_ref().map(|dispatch| {
            TracingExporter::with_max_emissions_per_poll(
                dispatch.clone(),
                config.export.max_emissions_per_poll,
            )
        });

        // Wire diagnostics findings into the same injected dispatch —
        // an opt-in default the host can still override via
        // `diagnostics_mut().set_sink(..)`.
        #[cfg(feature = "tracing-export")]
        let diagnostics = {
            let mut diagnostics = diagnostics;
            if config.export.diagnostics_events
                && let (Some(engine), Some(dispatch)) =
                    (diagnostics.as_mut(), config.export.dispatch.as_ref())
            {
                engine.set_sink(Box::new(TracingFindingSink::new(dispatch.clone())));
            }
            diagnostics
        };

        Ok(Self {
            collector,
            diagnostics,
            debug: config.build_debug(),
            #[cfg(feature = "tracing-export")]
            exporter,
            latest_diagnostics: None,
            poll_interval: config.collection.poll_interval,
            poll_errors: 0,
        })
    }

    /// The measured drain of this component set — final collector
    /// poll, terminal shutdown batch, terminal export, and pre-drop
    /// accounting. Does **not** touch runner-level logging writers
    /// (those belong to `ObservationRunner`).
    fn drain(&mut self, now: Instant) -> (ShutdownReport, DrainExtras) {
        // Step 1 — final poll while sources may still be Active.
        let final_poll_generation = if let Ok(batch) = self.collector.poll(now) {
            Some(batch.generation())
        } else {
            self.poll_errors += 1;
            None
        };

        // Step 2 — pre-drain counts, then terminal shutdown.
        let pre_health = self.collector.health();
        let sources_active_at_drain = pre_health.sources_active;
        let sources_retiring_at_drain = pre_health.sources_retiring;
        self.collector.shutdown();

        // Step 3 — terminal batch accounting.
        let terminal = self.collector.latest_batch();
        let terminal_generation = terminal.as_ref().map(ObservationBatch::generation);
        let measurements_drained = terminal
            .as_ref()
            .map_or(0, |batch| batch.measurements().len() as u64);
        let removed_total = terminal
            .as_ref()
            .map_or(0, |batch| batch.removed().len() as u64);
        let partial_drains = terminal.as_ref().map_or(0, |batch| {
            batch
                .measurements()
                .iter()
                .filter(|m| m.issue == Some(MeasurementIssue::WriterNotQuiesced))
                .count() as u64
        });

        // Step 4 — export the terminal batch ("exporter drain").
        #[cfg(feature = "tracing-export")]
        let emit = terminal
            .as_ref()
            .and_then(|batch| self.exporter.as_mut().map(|exporter| exporter.emit(batch)));

        // Step 5 — pre-drop accounting for lost/non-migrated state.
        let diagnostics_evidence_lost = self
            .diagnostics
            .as_ref()
            .map_or(0, |d| d.health().evidence_retained);
        let diagnostics_findings_lost = self
            .diagnostics
            .as_ref()
            .map_or(0, |d| d.health().findings_retained);
        let (debug_session_abandoned, debug_records_lost) =
            self.debug.as_ref().map_or((false, 0), |debug| {
                let view = debug.session_view();
                (
                    view.state != DebugSessionState::Disabled,
                    u64::try_from(view.records_retained).unwrap_or(u64::MAX) + view.pending_records,
                )
            });

        let completeness = if partial_drains > 0 {
            DrainCompleteness::PartialDrains {
                count: partial_drains,
            }
        } else if pre_health.registered_total == 0 {
            DrainCompleteness::NoSources
        } else if sources_active_at_drain == 0 {
            DrainCompleteness::AllQuiesced
        } else {
            // Defensive: active sources existed but no measurement was
            // marked unquiesced — report conservatively as partial.
            DrainCompleteness::PartialDrains {
                count: u64::try_from(sources_active_at_drain).unwrap_or(u64::MAX),
            }
        };

        let collector_health = self.collector.health();
        let report = ShutdownReport {
            polls_completed: collector_health.polls_total,
            poll_errors: self.poll_errors,
            final_poll_generation,
            terminal_generation,
            measurements_drained,
            removed_total,
            partial_drains,
            sources_active_at_drain,
            sources_retiring_at_drain,
            debug_records_lost,
            #[cfg(feature = "tracing-export")]
            emit,
            writers_closed: false,
            debug_session_abandoned,
            diagnostics_evidence_lost,
            diagnostics_findings_lost,
            completeness,
        };
        #[cfg(feature = "tracing-export")]
        let extras = DrainExtras {
            had_exporter: self.exporter.is_some(),
        };
        #[cfg(not(feature = "tracing-export"))]
        let extras = DrainExtras {};
        (report, extras)
    }
}

/// Host-owned non-RT observation runner — see module docs.
///
/// Construct with [`ObservationRunner::new`], configure with
/// [`prepare`](Self::prepare), activate with
/// [`activate`](Self::activate), then drive [`poll_once`](Self::poll_once)
/// at the configured interval from non-RT code. All operations are
/// `&mut self` — the runner is a single-owner object; share reads
/// through [`collector_handle`](Self::collector_handle) /
/// [`registration_port`](Self::registration_port) clones.
pub struct ObservationRunner {
    state: RunnerState,
    components: Option<Components>,
    polls_completed: u64,
    poll_errors_total: u64,
    reconfigures_total: u64,
    shutdowns_total: u64,
    last_shutdown: Option<ShutdownReport>,
    #[cfg(feature = "tracing-export")]
    export_totals: ExportTotals,
    #[cfg(feature = "logging")]
    logging: Option<LoggingAttachment>,
}

/// Logging pieces the runner holds between `attach_logging` and the
/// final drain step — `logging` feature only.
#[cfg(feature = "logging")]
struct LoggingAttachment {
    guards: Vec<WriterGuard>,
    health: LoggingHealth,
}

impl ObservationRunner {
    /// An `Unconfigured` runner — no components allocated.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: RunnerState::Unconfigured,
            components: None,
            polls_completed: 0,
            poll_errors_total: 0,
            reconfigures_total: 0,
            shutdowns_total: 0,
            last_shutdown: None,
            #[cfg(feature = "tracing-export")]
            export_totals: ExportTotals::default(),
            #[cfg(feature = "logging")]
            logging: None,
        }
    }

    /// Validates `config` and builds all enabled components —
    /// `Unconfigured`/`Stopped → Prepared`.
    ///
    /// From `Stopped` this is the restart path: every component is
    /// rebuilt fresh (new collector, new diagnostics retention, new
    /// debug inspector, new exporter baselines).
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] outside `Unconfigured`/`Stopped`;
    /// [`RunnerError::InvalidConfig`] when validation or construction
    /// fails (the runner is left `Unconfigured`/`Stopped` with no
    /// partial components).
    pub fn prepare(&mut self, config: &ObservabilityConfig) -> Result<(), RunnerError> {
        if !matches!(self.state, RunnerState::Unconfigured | RunnerState::Stopped) {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "prepare",
            });
        }
        config.validate().map_err(RunnerError::InvalidConfig)?;
        // Build into a local first — a construction failure must not
        // leave half-installed components behind.
        let components = Components::build(config)?;
        // Restart (`Stopped → Prepared`) begins a fresh incarnation:
        // the previous attachment's guards were already drained at
        // shutdown — drop the zombie (health handle included) so
        // `attach_logging` can take fresh writers (`logging` feature).
        #[cfg(feature = "logging")]
        if self.state == RunnerState::Stopped {
            self.logging = None;
        }
        self.components = Some(components);
        self.state = RunnerState::Prepared;
        Ok(())
    }

    /// `Prepared → Ready` — declares setup complete (sources may be
    /// registered any time after `prepare`; `activate` is the explicit
    /// host checkpoint before polling begins).
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Prepared`.
    pub fn activate(&mut self) -> Result<(), RunnerError> {
        if self.state != RunnerState::Prepared {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "activate",
            });
        }
        self.state = RunnerState::Ready;
        Ok(())
    }

    /// Drives one observation cycle: `Collector::poll(now)` →
    /// `Diagnostics::evaluate` → `TracingExporter::emit`.
    ///
    /// `now` is host-supplied — pass a non-RT timestamp (the poll
    /// owner's clock). The first successful call transitions
    /// `Ready → Running`; later calls keep `Running`.
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Ready`/`Running`;
    /// [`RunnerError::Collector`] when the collector rejects the poll
    /// (counted in `poll_errors_total`).
    pub fn poll_once(&mut self, now: Instant) -> Result<PollOutcome, RunnerError> {
        if !matches!(self.state, RunnerState::Ready | RunnerState::Running) {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "poll_once",
            });
        }
        let components = self.components.as_mut().ok_or(RunnerError::InvalidState {
            state: self.state,
            operation: "poll_once",
        })?;

        let batch = match components.collector.poll(now) {
            Ok(batch) => batch,
            Err(err) => {
                self.poll_errors_total += 1;
                components.poll_errors += 1;
                return Err(RunnerError::Collector(err));
            }
        };
        self.polls_completed += 1;
        if self.state == RunnerState::Ready {
            self.state = RunnerState::Running;
        }

        let snapshot = batch.snapshot();
        let diagnostics = components
            .diagnostics
            .as_mut()
            .map(|diagnostics| diagnostics.evaluate(&snapshot, now));
        components.latest_diagnostics.clone_from(&diagnostics);

        #[cfg(feature = "tracing-export")]
        let emit = components
            .exporter
            .as_mut()
            .map(|exporter| exporter.emit(&batch));
        #[cfg(feature = "tracing-export")]
        if let Some(stats) = emit {
            self.export_totals.add(stats);
        }

        Ok(PollOutcome {
            batch,
            diagnostics,
            #[cfg(feature = "tracing-export")]
            emit,
        })
    }

    /// `Ready`/`Running → Draining` — declares shutdown intent.
    ///
    /// Polls, `reconfigure`, and `activate` are rejected from here on.
    /// The host uses this window to stop domain writers and confirm
    /// quiescence (lease `retire`) before calling
    /// [`finish_shutdown`](Self::finish_shutdown).
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Ready`/`Running`.
    pub fn begin_shutdown(&mut self) -> Result<(), RunnerError> {
        if !matches!(self.state, RunnerState::Ready | RunnerState::Running) {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "begin_shutdown",
            });
        }
        self.state = RunnerState::Draining;
        Ok(())
    }

    /// `Draining → Stopped` — runs the ordered drain and returns the
    /// measured [`ShutdownReport`].
    ///
    /// Order: final collector poll → `Collector::shutdown` terminal
    /// batch → terminal-batch export → logging writer flush/close.
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Draining`.
    pub fn finish_shutdown(&mut self, now: Instant) -> Result<ShutdownReport, RunnerError> {
        if self.state != RunnerState::Draining {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "finish_shutdown",
            });
        }
        Ok(self.force_shutdown(now))
    }

    /// One-shot `Ready`/`Running → Stopped` — `begin_shutdown` +
    /// `finish_shutdown` without the domain-quiescence window. Hosts
    /// that need to stop writers first should call the two phases
    /// separately.
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Ready`/`Running`.
    pub fn shutdown(&mut self, now: Instant) -> Result<ShutdownReport, RunnerError> {
        self.begin_shutdown()?;
        self.finish_shutdown(now)
    }

    /// Dynamic reconfiguration — Validate → Prepare → Activate →
    /// Retire, non-RT, from `Ready`/`Running`.
    ///
    /// On success the runner swaps to a freshly built component set
    /// (new collector `SourceId` space, new diagnostics retention,
    /// new debug inspector, new exporter baselines) and drains the
    /// retired set — its terminal batch is emitted through the
    /// **old** exporter. Validation or construction failure leaves
    /// the running configuration completely untouched.
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] unless `Ready`/`Running`;
    /// [`RunnerError::InvalidConfig`] on validation/build failure.
    pub fn reconfigure(
        &mut self,
        config: &ObservabilityConfig,
    ) -> Result<ReconfigureReport, RunnerError> {
        if !matches!(self.state, RunnerState::Ready | RunnerState::Running) {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "reconfigure",
            });
        }
        // Validate → Prepare: build fully before touching live state.
        config.validate().map_err(RunnerError::InvalidConfig)?;
        let new_components = Components::build(config)?;
        #[cfg(feature = "tracing-export")]
        let new_has_exporter = new_components.exporter.is_some();

        // Activate: swap component sets.
        let old = self.components.replace(new_components);
        self.reconfigures_total += 1;

        // Retire: drain the old set (no writer close — logging is
        // runner-level and stays attached).
        let (mut retired, extras) = old.map_or_else(
            || (ShutdownReport::empty(), DrainExtras::empty()),
            |mut components| components.drain(Instant::now()),
        );
        retired.writers_closed = false;

        // Account the terminal emit in lifetime export totals.
        #[cfg(feature = "tracing-export")]
        if let Some(stats) = retired.emit {
            self.export_totals.add(stats);
        }

        #[cfg(feature = "tracing-export")]
        let baselines_reset = extras.had_exporter && new_has_exporter;
        #[cfg(not(feature = "tracing-export"))]
        let baselines_reset = {
            let _ = extras;
            false
        };

        let non_migrated = NonMigrated {
            sources_requiring_reregistration: retired
                .sources_active_at_drain
                .saturating_add(retired.sources_retiring_at_drain),
            diagnostics_evidence_lost: retired.diagnostics_evidence_lost,
            diagnostics_findings_lost: retired.diagnostics_findings_lost,
            debug_session_abandoned: retired.debug_session_abandoned,
            debug_records_lost: retired.debug_records_lost,
            exporter_baselines_reset: baselines_reset,
        };

        self.last_shutdown = Some(retired.clone());
        Ok(ReconfigureReport {
            retired,
            non_migrated,
        })
    }

    /// Attaches host-built logging writers + health — `logging`
    /// feature only, `Prepared`/`Ready`/`Running` only.
    ///
    /// The host keeps ownership of dispatch installation (global or
    /// scoped) and of `LoggingBuildResult::dispatch`; this call takes
    /// only the writer guards (dropped in order at the final drain
    /// step — flush/close) and the cloneable health handle.
    ///
    /// # Errors
    ///
    /// [`RunnerError::InvalidState`] outside `Prepared`/`Ready`/
    /// `Running`; [`RunnerError::AlreadyAttached`] on a second call.
    #[cfg(feature = "logging")]
    pub fn attach_logging(
        &mut self,
        guards: Vec<WriterGuard>,
        health: LoggingHealth,
    ) -> Result<(), RunnerError> {
        if !matches!(
            self.state,
            RunnerState::Prepared | RunnerState::Ready | RunnerState::Running
        ) {
            return Err(RunnerError::InvalidState {
                state: self.state,
                operation: "attach_logging",
            });
        }
        if self.logging.is_some() {
            return Err(RunnerError::AlreadyAttached);
        }
        self.logging = Some(LoggingAttachment { guards, health });
        Ok(())
    }

    /// Current lifecycle state.
    #[must_use]
    pub fn state(&self) -> RunnerState {
        self.state
    }

    /// The configured collection poll interval — `None` while
    /// `Unconfigured`.
    #[must_use]
    pub fn poll_interval(&self) -> Option<Duration> {
        self.components.as_ref().map(|c| c.poll_interval)
    }

    /// Shareable collector handle for GUI/other-thread pull reads —
    /// `None` while `Unconfigured`.
    #[must_use]
    pub fn collector_handle(&self) -> Option<CollectorHandle> {
        self.components.as_ref().map(|c| c.collector.handle())
    }

    /// The live registration port for runtime source registration —
    /// `None` while `Unconfigured`.
    #[must_use]
    pub fn registration_port(&self) -> Option<CollectorRegistrar> {
        self.components
            .as_ref()
            .map(|c| c.collector.registration_port())
    }

    /// Latest committed [`MetricsSnapshot`] — `None` only while
    /// `Unconfigured`. Before the first poll this is the empty
    /// generation-0 snapshot, matching `Collector::snapshot`.
    #[must_use]
    pub fn snapshot(&self) -> Option<MetricsSnapshot> {
        self.components.as_ref().map(|c| c.collector.snapshot())
    }

    /// Latest committed [`ObservationBatch`] — `None` while
    /// `Unconfigured` or before the first poll.
    #[must_use]
    pub fn latest_batch(&self) -> Option<ObservationBatch> {
        self.components
            .as_ref()
            .and_then(|c| c.collector.latest_batch())
    }

    /// History-ring measurements — `None` while `Unconfigured`.
    #[must_use]
    pub fn history(&self) -> Option<Vec<Measurement>> {
        self.components.as_ref().map(|c| c.collector.history())
    }

    /// Per-source measurement series — `None` while `Unconfigured`.
    #[must_use]
    pub fn series(&self, identity: SourceIdentity) -> Option<Vec<Measurement>> {
        self.components
            .as_ref()
            .map(|c| c.collector.series(identity))
    }

    /// The diagnostics engine — `None` when diagnostics is disabled or
    /// the runner is `Unconfigured`.
    #[must_use]
    pub fn diagnostics(&self) -> Option<&Diagnostics> {
        self.components
            .as_ref()
            .and_then(|c| c.diagnostics.as_ref())
    }

    /// Mutable diagnostics engine — add rules, replace the sink.
    #[must_use]
    pub fn diagnostics_mut(&mut self) -> Option<&mut Diagnostics> {
        self.components
            .as_mut()
            .and_then(|c| c.diagnostics.as_mut())
    }

    /// The latest diagnostics evaluation snapshot — `None` when
    /// diagnostics is disabled or no poll has run yet.
    #[must_use]
    pub fn diagnostics_snapshot(&self) -> Option<&DiagnosticSnapshot> {
        self.components
            .as_ref()
            .and_then(|c| c.latest_diagnostics.as_ref())
    }

    /// The debug inspector — `None` when debug is disabled or the
    /// runner is `Unconfigured`.
    #[must_use]
    pub fn debug(&self) -> Option<&DebugInspector> {
        self.components.as_ref().and_then(|c| c.debug.as_ref())
    }

    /// Mutable debug inspector — register probe sources, invariant
    /// checks, and drive sessions (with a host-minted permit).
    #[must_use]
    pub fn debug_mut(&mut self) -> Option<&mut DebugInspector> {
        self.components.as_mut().and_then(|c| c.debug.as_mut())
    }

    /// Composed, logging-independent health snapshot — see
    /// [`RunnerHealth`]. Never emits logging/tracing itself.
    #[must_use]
    pub fn health(&self) -> RunnerHealth {
        let components = self.components.as_ref();
        RunnerHealth {
            state: self.state,
            polls_completed: self.polls_completed,
            poll_errors_total: self.poll_errors_total,
            reconfigures_total: self.reconfigures_total,
            shutdowns_total: self.shutdowns_total,
            collector: components.map(|c| c.collector.health()),
            diagnostics: components.and_then(|c| c.diagnostics.as_ref().map(Diagnostics::health)),
            debug: components.and_then(|c| c.debug.as_ref().map(DebugInspector::health)),
            debug_session: components
                .and_then(|c| c.debug.as_ref().map(DebugInspector::session_view)),
            #[cfg(feature = "tracing-export")]
            export: (self.export_totals.emits_total > 0
                || self.export_totals.emitted_total > 0
                || components.is_some_and(|c| c.exporter.is_some()))
            .then_some(self.export_totals),
            #[cfg(feature = "logging")]
            logging: self.logging.as_ref().map(|l| l.health.snapshot()),
            last_shutdown: self.last_shutdown.clone(),
        }
    }

    /// The most recent drain report — shutdown or reconfigure
    /// retire.
    #[must_use]
    pub fn shutdown_report(&self) -> Option<&ShutdownReport> {
        self.last_shutdown.as_ref()
    }

    /// Runs the drain sequence and closes attached writers — shared
    /// by `finish_shutdown`, `shutdown`, `Drop`, and tests. Assumes
    /// the caller already validated the lifecycle transition.
    fn force_shutdown(&mut self, now: Instant) -> ShutdownReport {
        let (mut report, _extras) = self.components.as_mut().map_or_else(
            || (ShutdownReport::empty(), DrainExtras::empty()),
            |components| components.drain(now),
        );

        // Account the terminal emit in lifetime export totals.
        #[cfg(feature = "tracing-export")]
        if let Some(stats) = report.emit {
            self.export_totals.add(stats);
        }

        // Final step — writer flush/close (logging only). The health
        // handle stays attached so post-shutdown sink counters
        // (flush failures, abandoned workers) remain readable.
        #[cfg(feature = "logging")]
        if let Some(attachment) = self.logging.as_mut() {
            // Dropping each guard drains + flushes + joins its
            // worker (bounded wait, then abandon).
            for guard in std::mem::take(&mut attachment.guards) {
                drop(guard);
            }
        }
        report.writers_closed = true;

        self.state = RunnerState::Stopped;
        self.shutdowns_total += 1;
        self.last_shutdown = Some(report.clone());
        report
    }
}

impl DrainExtras {
    fn empty() -> Self {
        Self {
            #[cfg(feature = "tracing-export")]
            had_exporter: false,
        }
    }
}

impl Default for ObservationRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ObservationRunner {
    fn drop(&mut self) {
        // Best-effort drain for any non-terminal state with live
        // components — `Prepared`/`Ready`/`Running`/`Draining`. The
        // collector's own `Drop` would drain anyway; running the full
        // sequence here also emits the terminal batch and closes
        // writers in the documented order. `Unconfigured` has no
        // components; `Stopped` already drained (components are kept
        // only for post-mortem reads).
        if !matches!(self.state, RunnerState::Unconfigured | RunnerState::Stopped)
            && self.components.is_some()
        {
            let _ = self.force_shutdown(Instant::now());
        }
    }
}
