//! Unit tests for the `integration` module — lifecycle, drain
//! ordering, dynamic reconfigure, and health composition. Feature-
//! gated contract coverage (dispatch, logging) lives in
//! `tests/integration.rs`.

use std::time::{Duration, Instant};

use crate::collector::CollectorConfig;
use crate::core::counter;
use crate::debug::DebugConfig;
use crate::diagnostics::DiagnosticsConfig;
use crate::source::{
    CounterDescriptor, CounterSource, MetricSource, ObservationRegistrar, ObservationScope,
    ObservationUnit, ReportingHint,
};

#[cfg(feature = "tracing-export")]
use super::config::ExportPolicy;
use super::config::{DebugSettings, ObservabilityConfig};
use super::{DrainCompleteness, ObservationRunner, RunnerError, RunnerState};

fn now() -> Instant {
    Instant::now()
}

fn collector_config() -> CollectorConfig {
    CollectorConfig {
        poll_interval: Duration::from_millis(10),
        ..CollectorConfig::default()
    }
}

fn config() -> ObservabilityConfig {
    ObservabilityConfig {
        collection: collector_config(),
        diagnostics: None,
        debug: None,
        #[cfg(feature = "tracing-export")]
        export: ExportPolicy::default(),
    }
}

fn full_config() -> ObservabilityConfig {
    ObservabilityConfig {
        diagnostics: Some(DiagnosticsConfig::default()),
        debug: Some(DebugSettings::new(DebugConfig::default())),
        ..config()
    }
}

fn counter_source(key: &'static str) -> MetricSource {
    let (_counter, reader) = counter();
    MetricSource::from(CounterSource {
        descriptor: CounterDescriptor {
            key,
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
        },
        scope: ObservationScope::Process,
        reader,
    })
}

fn prepared_runner() -> ObservationRunner {
    let mut runner = ObservationRunner::new();
    runner.prepare(&config()).unwrap();
    runner
}

fn running_runner() -> ObservationRunner {
    let mut runner = prepared_runner();
    runner.activate().unwrap();
    runner
}

// --- lifecycle ---------------------------------------------------------------

#[test]
fn lifecycle_happy_path_reaches_stopped() {
    let mut runner = ObservationRunner::new();
    assert_eq!(runner.state(), RunnerState::Unconfigured);
    assert!(runner.snapshot().is_none());
    assert!(runner.collector_handle().is_none());

    runner.prepare(&config()).unwrap();
    assert_eq!(runner.state(), RunnerState::Prepared);

    runner.activate().unwrap();
    assert_eq!(runner.state(), RunnerState::Ready);

    runner.poll_once(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Running);
    runner.poll_once(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Running);

    runner.begin_shutdown().unwrap();
    assert_eq!(runner.state(), RunnerState::Draining);

    let report = runner.finish_shutdown(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Stopped);
    assert_eq!(report.completeness, DrainCompleteness::NoSources);
    assert!(runner.shutdown_report().is_some());
}

#[test]
fn invalid_transitions_are_rejected_without_state_change() {
    let mut runner = ObservationRunner::new();
    for result in [
        runner.activate().err(),
        runner.poll_once(now()).err(),
        runner.begin_shutdown().err(),
        runner.finish_shutdown(now()).err(),
        runner.shutdown(now()).err(),
        runner.reconfigure(&config()).err(),
    ] {
        assert!(matches!(result, Some(RunnerError::InvalidState { .. })));
    }
    assert_eq!(runner.state(), RunnerState::Unconfigured);

    // Prepared still rejects poll/shutdown/reconfigure.
    runner.prepare(&config()).unwrap();
    assert!(matches!(
        runner.poll_once(now()),
        Err(RunnerError::InvalidState { .. })
    ));
    assert!(matches!(
        runner.reconfigure(&config()),
        Err(RunnerError::InvalidState { .. })
    ));
    assert!(matches!(
        runner.prepare(&config()),
        Err(RunnerError::InvalidState { .. })
    ));

    // activate is not idempotent — a second call is rejected.
    runner.activate().unwrap();
    assert!(matches!(
        runner.activate(),
        Err(RunnerError::InvalidState { .. })
    ));

    // Draining rejects poll/reconfigure/activate/prepare.
    runner.begin_shutdown().unwrap();
    for result in [
        runner.poll_once(now()).err(),
        runner.reconfigure(&config()).err(),
        runner.activate().err(),
        runner.prepare(&config()).err(),
        runner.begin_shutdown().err(),
    ] {
        assert!(matches!(result, Some(RunnerError::InvalidState { .. })));
    }

    // Stopped rejects everything except prepare (restart).
    runner.finish_shutdown(now()).unwrap();
    for result in [
        runner.poll_once(now()).err(),
        runner.reconfigure(&config()).err(),
        runner.activate().err(),
        runner.begin_shutdown().err(),
        runner.finish_shutdown(now()).err(),
        runner.shutdown(now()).err(),
    ] {
        assert!(matches!(result, Some(RunnerError::InvalidState { .. })));
    }
    runner.prepare(&config()).unwrap();
    assert_eq!(runner.state(), RunnerState::Prepared);
}

#[test]
fn invalid_config_is_rejected_and_state_preserved() {
    let mut runner = ObservationRunner::new();
    let mut bad = config();
    bad.collection.poll_interval = Duration::ZERO;
    assert!(matches!(
        runner.prepare(&bad),
        Err(RunnerError::InvalidConfig(_))
    ));
    assert_eq!(runner.state(), RunnerState::Unconfigured);

    // Same guarantee while Running: reconfigure validates first.
    let mut runner = running_runner();
    runner.poll_once(now()).unwrap();
    let mut bad = config();
    bad.collection.max_series = 0;
    assert!(matches!(
        runner.reconfigure(&bad),
        Err(RunnerError::InvalidConfig(_))
    ));
    assert_eq!(runner.state(), RunnerState::Running);
    assert_eq!(runner.poll_interval(), Some(Duration::from_millis(10)));
}

// --- poll / snapshot ---------------------------------------------------------

#[test]
fn poll_drives_collector_and_exposes_snapshots() {
    let mut runner = running_runner();
    // The snapshot exists from construction (generation 0, empty);
    // a batch exists only after the first committed poll.
    assert_eq!(runner.snapshot().map(|s| s.generation()), Some(0));
    assert!(runner.latest_batch().is_none());
    assert_eq!(runner.poll_interval(), Some(Duration::from_millis(10)));
    assert!(runner.collector_handle().is_some());
    assert!(runner.registration_port().is_some());

    let outcome = runner.poll_once(now()).unwrap();
    assert_eq!(outcome.batch.generation(), 1);
    assert!(outcome.diagnostics.is_none());
    // Default `ExportPolicy` has `dispatch: None` — no exporter is
    // built, so `emit` is `None` even with `tracing-export` on.
    #[cfg(feature = "tracing-export")]
    assert!(outcome.emit.is_none());

    let outcome = runner.poll_once(now()).unwrap();
    assert_eq!(outcome.batch.generation(), 2);

    assert!(runner.snapshot().is_some());
    assert!(runner.latest_batch().is_some());
    assert_eq!(runner.health().polls_completed, 2);
}

#[test]
fn diagnostics_and_debug_subsystems_compose_when_enabled() {
    let mut runner = ObservationRunner::new();
    runner.prepare(&full_config()).unwrap();
    runner.activate().unwrap();
    assert!(runner.diagnostics().is_some());
    assert!(runner.debug().is_some());
    assert!(runner.debug_mut().is_some());
    assert!(runner.diagnostics_mut().is_some());

    let outcome = runner.poll_once(now()).unwrap();
    assert!(outcome.diagnostics.is_some());
    assert!(runner.diagnostics_snapshot().is_some());

    let health = runner.health();
    assert!(health.diagnostics.is_some());
    assert!(health.debug.is_some());
    assert!(health.debug_session.is_some());
    assert_eq!(
        health.debug_session.as_ref().map(|v| v.state),
        Some(crate::debug::DebugSessionState::Disabled)
    );
}

#[test]
fn collector_only_operation_needs_no_optional_subsystems() {
    // diagnostics: None, debug: None, no export dispatch, no logging —
    // the full lifecycle still works.
    let mut runner = running_runner();
    runner.poll_once(now()).unwrap();
    let report = runner.shutdown(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Stopped);
    assert_eq!(report.completeness, DrainCompleteness::NoSources);
    assert!(report.terminal_generation.is_some());
    assert!(report.writers_closed);
}

// --- drain -------------------------------------------------------------------

#[test]
fn unquiesced_source_is_reported_partial_not_complete() {
    let mut runner = running_runner();
    let port = runner.registration_port().unwrap();
    let source = counter_source("test.integration.unquiesced");
    // Lease deliberately kept alive and never retired — the source
    // stays `Active` through the drain.
    let _lease = port.register(source).unwrap();
    runner.poll_once(now()).unwrap();

    let report = runner.shutdown(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Stopped);
    assert_eq!(report.sources_active_at_drain, 1);
    assert!(report.partial_drains >= 1);
    assert!(matches!(
        report.completeness,
        DrainCompleteness::PartialDrains { .. }
    ));
}

#[test]
fn retired_source_drains_all_quiesced() {
    let mut runner = running_runner();
    let port = runner.registration_port().unwrap();
    let source = counter_source("test.integration.quiesced");
    let lease = port.register(source).unwrap();
    runner.poll_once(now()).unwrap();
    let _retired = lease.retire();
    runner.poll_once(now()).unwrap();

    let report = runner.shutdown(now()).unwrap();
    assert_eq!(report.sources_active_at_drain, 0);
    assert_eq!(report.partial_drains, 0);
    assert_eq!(report.completeness, DrainCompleteness::AllQuiesced);
}

#[test]
fn staged_shutdown_rejects_polls_during_draining() {
    let mut runner = running_runner();
    runner.poll_once(now()).unwrap();
    runner.begin_shutdown().unwrap();
    assert!(matches!(
        runner.poll_once(now()),
        Err(RunnerError::InvalidState { .. })
    ));
    let report = runner.finish_shutdown(now()).unwrap();
    assert_eq!(report.completeness, DrainCompleteness::NoSources);
}

// --- dynamic reconfigure -----------------------------------------------------

#[test]
fn reconfigure_swaps_config_and_reports_non_migrated_state() {
    let mut runner = running_runner();
    let port = runner.registration_port().unwrap();
    let source = counter_source("test.integration.reconfigure");
    let _lease = port.register(source).unwrap();
    runner.poll_once(now()).unwrap();

    let mut next = config();
    next.collection.poll_interval = Duration::from_millis(50);
    next.diagnostics = Some(DiagnosticsConfig::default());
    let report = runner.reconfigure(&next).unwrap();

    assert_eq!(runner.state(), RunnerState::Running);
    assert_eq!(runner.poll_interval(), Some(Duration::from_millis(50)));
    assert!(runner.diagnostics().is_some());
    // The live source did not migrate — re-registration is required.
    assert_eq!(report.non_migrated.sources_requiring_reregistration, 1);
    // The retired collector drained the still-active source honestly.
    assert!(matches!(
        report.retired.completeness,
        DrainCompleteness::PartialDrains { .. }
    ));
    assert!(!report.retired.writers_closed);

    // New collector accepts fresh registrations and polls.
    let new_source = counter_source("test.integration.reconfigure.new");
    runner
        .registration_port()
        .unwrap()
        .register(new_source)
        .unwrap();
    runner.poll_once(now()).unwrap();
}

#[test]
fn reconfigure_restarts_after_stop_via_prepare() {
    let mut runner = running_runner();
    runner.shutdown(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Stopped);
    runner.prepare(&config()).unwrap();
    assert_eq!(runner.state(), RunnerState::Prepared);
    runner.activate().unwrap();
    runner.poll_once(now()).unwrap();
    assert_eq!(runner.state(), RunnerState::Running);
}

// --- health ------------------------------------------------------------------

#[test]
fn health_is_composed_and_logging_independent() {
    let mut runner = ObservationRunner::new();
    let health = runner.health();
    assert_eq!(health.state, RunnerState::Unconfigured);
    assert!(health.collector.is_none());
    assert!(health.diagnostics.is_none());
    assert!(health.debug.is_none());
    assert!(health.debug_session.is_none());
    assert!(health.last_shutdown.is_none());
    #[cfg(feature = "tracing-export")]
    assert!(health.export.is_none());
    #[cfg(feature = "logging")]
    assert!(health.logging.is_none());

    runner.prepare(&full_config()).unwrap();
    let health = runner.health();
    assert_eq!(health.state, RunnerState::Prepared);
    assert!(health.collector.is_some());
    assert!(health.diagnostics.is_some());
    assert!(health.debug.is_some());
    assert!(health.debug_session.is_some());

    // Prepared still rejects the full shutdown path.
    assert!(matches!(
        runner.shutdown(now()),
        Err(RunnerError::InvalidState { .. })
    ));

    let mut runner = running_runner();
    runner.poll_once(now()).unwrap();
    let report = runner.shutdown(now()).unwrap();
    let health = runner.health();
    assert_eq!(health.state, RunnerState::Stopped);
    assert_eq!(health.shutdowns_total, 1);
    assert!(health.last_shutdown.is_some());
    assert_eq!(
        health.last_shutdown.as_ref().map(|r| r.terminal_generation),
        Some(report.terminal_generation)
    );
}
