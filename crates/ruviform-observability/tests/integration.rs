//! `integration` contract tests — Issue #293 (OBS-11).
//!
//! Covers the host-facing lifecycle/drain/reconfigure/health
//! contracts end-to-end through the public API, plus the feature-
//! gated dispatch and logging sections. The library itself never
//! installs a global dispatcher — every dispatch test injects
//! explicitly.

#[cfg(feature = "logging")]
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use ruviform_observability::collector::CollectorConfig;
use ruviform_observability::core::{Counter, counter};
use ruviform_observability::integration::{
    DebugSettings, DrainCompleteness, ObservabilityConfig, ObservationRunner, RunnerError,
    RunnerState,
};
#[cfg(feature = "tracing-export")]
use ruviform_observability::source::ReportingImpact;
use ruviform_observability::source::{
    CounterDescriptor, CounterSource, MetricSource, ObservationRegistrar, ObservationScope,
    ObservationUnit, ReportingHint,
};

fn now() -> Instant {
    Instant::now()
}

#[cfg(feature = "logging")]
fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("non-zero")
}

fn config() -> ObservabilityConfig {
    ObservabilityConfig {
        collection: CollectorConfig {
            poll_interval: Duration::from_millis(10),
            ..CollectorConfig::default()
        },
        diagnostics: None,
        debug: None,
        #[cfg(feature = "tracing-export")]
        export: ruviform_observability::integration::ExportPolicy::default(),
    }
}

fn counter_source(key: &'static str) -> (Counter, MetricSource) {
    counter_source_with(key, ReportingHint::SnapshotOnly)
}

fn counter_source_with(key: &'static str, reporting: ReportingHint) -> (Counter, MetricSource) {
    let (counter, reader) = counter();
    let source = MetricSource::from(CounterSource {
        descriptor: CounterDescriptor {
            key,
            unit: ObservationUnit::Count,
            reporting,
        },
        scope: ObservationScope::Process,
        reader,
    });
    (counter, source)
}

fn running_runner() -> ObservationRunner {
    let mut runner = ObservationRunner::new();
    runner.prepare(&config()).unwrap();
    runner.activate().unwrap();
    runner
}

// --- lifecycle / drain ---------------------------------------------------------

#[test]
fn staged_shutdown_reports_quiesced_drain_and_terminal_batch() {
    let mut runner = running_runner();
    let port = runner.registration_port().unwrap();
    let (counter, source) = counter_source("test.integration.e2e");
    let lease = port.register(source).unwrap();
    counter.increment();
    runner.poll_once(now()).unwrap();
    counter.increment();
    runner.poll_once(now()).unwrap();

    // Staged path: declare Draining, retire the writer, finish.
    runner.begin_shutdown().unwrap();
    assert_eq!(runner.state(), RunnerState::Draining);
    drop(lease.retire());
    let report = runner.finish_shutdown(now()).unwrap();

    assert_eq!(runner.state(), RunnerState::Stopped);
    assert_eq!(report.completeness, DrainCompleteness::AllQuiesced);
    assert_eq!(report.partial_drains, 0);
    assert!(report.final_poll_generation.is_some());
    assert!(report.terminal_generation.is_some());
    assert!(report.terminal_generation.unwrap() > report.final_poll_generation.unwrap());
    assert!(report.writers_closed);
    // The terminal batch is still inspectable after Stopped.
    let terminal = runner.latest_batch().unwrap();
    assert_eq!(terminal.generation(), report.terminal_generation.unwrap());
}

#[test]
fn unquiesced_source_is_partial_not_complete() {
    let mut runner = running_runner();
    let port = runner.registration_port().unwrap();
    let (_counter, source) = counter_source("test.integration.partial");
    let _lease = port.register(source).unwrap(); // never retired
    runner.poll_once(now()).unwrap();

    let report = runner.shutdown(now()).unwrap();
    assert!(matches!(
        report.completeness,
        DrainCompleteness::PartialDrains { count } if count >= 1
    ));
    assert!(report.partial_drains >= 1);
}

#[test]
fn reconfigure_validates_prepares_activates_and_retires() {
    let mut runner = running_runner();
    runner.poll_once(now()).unwrap();

    // Invalid config — rejected before any swap.
    let mut bad = config();
    bad.collection.poll_interval = Duration::ZERO;
    assert!(matches!(
        runner.reconfigure(&bad),
        Err(RunnerError::InvalidConfig(_))
    ));
    assert_eq!(runner.poll_interval(), Some(Duration::from_millis(10)));

    // Valid config — poll interval changes, diagnostics enabled.
    let mut next = config();
    next.collection.poll_interval = Duration::from_millis(25);
    #[cfg(feature = "tracing-export")]
    {
        next.export.max_emissions_per_poll = 7;
    }
    let report = runner.reconfigure(&next).unwrap();
    assert_eq!(runner.state(), RunnerState::Running);
    assert_eq!(runner.poll_interval(), Some(Duration::from_millis(25)));
    assert!(report.retired.terminal_generation.is_some());
    assert_eq!(report.non_migrated.sources_requiring_reregistration, 0);

    // New pipeline polls independently.
    let outcome = runner.poll_once(now()).unwrap();
    assert_eq!(outcome.batch.generation(), 1);
}

#[test]
fn health_snapshot_is_value_only_and_non_recursive() {
    let mut runner = ObservationRunner::new();
    let before = runner.health();
    assert_eq!(before.state, RunnerState::Unconfigured);
    assert!(before.collector.is_none());

    runner.prepare(&config()).unwrap();
    runner.activate().unwrap();
    runner.poll_once(now()).unwrap();
    let health = runner.health();
    assert_eq!(health.state, RunnerState::Running);
    assert_eq!(health.polls_completed, 1);
    let collector = health.collector.expect("collector health");
    assert_eq!(collector.polls_total, 1);
    // Reading health twice must not perturb counters or emit anything.
    let again = runner.health();
    assert_eq!(again.polls_completed, health.polls_completed);
    assert_eq!(again.collector.unwrap().polls_total, 1);
}

// --- tracing-export -------------------------------------------------------------

#[cfg(feature = "tracing-export")]
mod dispatch {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use serde_json::Value;
    use tracing::{Dispatch, dispatcher};
    use tracing_subscriber::Registry;
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;
    use ruviform_observability::integration::ExportPolicy;

    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl SharedBuf {
        fn take(&self) -> Vec<Value> {
            let mut data = self.0.lock().unwrap();
            let text = String::from_utf8_lossy(&data).into_owned();
            data.clear();
            text.lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str(line).expect("captured line is JSON"))
                .collect()
        }
    }

    fn capture_dispatch() -> (Dispatch, SharedBuf) {
        let buf = SharedBuf::default();
        let layer = fmt::layer().json().with_writer({
            let b = buf.clone();
            move || b.clone()
        });
        (Dispatch::new(Registry::default().with(layer)), buf)
    }

    fn export_config(dispatch: Dispatch) -> ObservabilityConfig {
        let mut cfg = config();
        cfg.export = ExportPolicy {
            dispatch: Some(dispatch),
            max_emissions_per_poll: 64,
            diagnostics_events: false,
        };
        cfg
    }

    #[test]
    fn injected_dispatch_gets_events_scoped_over_ambient() {
        let (injected, injected_buf) = capture_dispatch();
        let (ambient, ambient_buf) = capture_dispatch();

        let mut runner = ObservationRunner::new();
        runner.prepare(&export_config(injected)).unwrap();
        runner.activate().unwrap();
        let port = runner.registration_port().unwrap();
        // OnIncrease so the exporter actually emits (SnapshotOnly is
        // suppressed by policy). First poll only baselines.
        let (counter, source) = counter_source_with(
            "test.integration.dispatch",
            ReportingHint::OnIncrease {
                min_delta: 1,
                min_interval: Duration::ZERO,
                impact: ReportingImpact::Normal,
            },
        );
        let _lease = port.register(source).unwrap();

        // Poll under a *different* scoped ambient dispatch — the
        // injected one must still be the emission target.
        dispatcher::with_default(&ambient, || {
            counter.increment();
            runner.poll_once(now()).unwrap();
        });
        counter.increment();
        runner.poll_once(now()).unwrap();

        let events = injected_buf.take();
        assert_ne!(events.len(), 0, "injected dispatch saw no events");
        let ambient = ambient_buf.take();
        assert_eq!(ambient.len(), 0, "ambient dispatch leaked events");

        runner.shutdown(now()).unwrap();
        // Terminal drain batch is also exported through the injection.
        let terminal = injected_buf.take();
        assert_ne!(terminal.len(), 0);
    }

    #[test]
    fn global_default_never_captures_injected_emissions() {
        let (injected, injected_buf) = capture_dispatch();
        let (global, global_buf) = capture_dispatch();
        // One-time per process — the library itself never calls this.
        let _ = dispatcher::set_global_default(global);

        let mut runner = ObservationRunner::new();
        runner.prepare(&export_config(injected)).unwrap();
        runner.activate().unwrap();
        let port = runner.registration_port().unwrap();
        let (counter, source) = counter_source_with(
            "test.integration.global",
            ReportingHint::OnIncrease {
                min_delta: 1,
                min_interval: Duration::ZERO,
                impact: ReportingImpact::Normal,
            },
        );
        let _lease = port.register(source).unwrap();
        runner.poll_once(now()).unwrap();
        counter.increment();
        runner.poll_once(now()).unwrap();

        let injected = injected_buf.take();
        assert_ne!(injected.len(), 0);
        let global = global_buf.take();
        assert_eq!(global.len(), 0);
        runner.shutdown(now()).unwrap();
    }

    #[test]
    fn dispatch_none_is_a_valid_noop_export() {
        let mut cfg = config();
        cfg.export.dispatch = None;
        let mut runner = ObservationRunner::new();
        runner.prepare(&cfg).unwrap();
        runner.activate().unwrap();
        let outcome = runner.poll_once(now()).unwrap();
        assert!(outcome.emit.is_none());
        assert!(runner.health().export.is_none());
        runner.shutdown(now()).unwrap();
    }
}

// --- logging ---------------------------------------------------------------------

#[cfg(feature = "logging")]
mod logging {
    use std::path::Path;

    use ruviform_observability::logging::{
        LogFormat, LoggingBuildResult, LoggingConfig, RotationPeriod, SinkConfig, SinkDestination,
        SinkFailureMode, SpanEvents, build_logging,
    };

    use super::*;

    fn file_sink(dir: &Path, prefix: &str) -> SinkConfig {
        SinkConfig {
            destination: SinkDestination::RollingFile {
                directory: dir.to_path_buf(),
                filename_prefix: prefix.to_owned(),
                filename_suffix: None,
                rotation: RotationPeriod::Never,
                max_files: nz(4),
                queue_capacity: nz(64),
            },
            format: LogFormat::JsonLines,
            span_events: SpanEvents::NONE,
            filter: "info".to_owned(),
            failure_mode: SinkFailureMode::Required,
        }
    }

    #[test]
    fn attach_logging_reports_sink_health_and_closes_on_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let built = build_logging(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "obs11")],
        })
        .unwrap();

        let mut runner = ObservationRunner::new();
        runner.prepare(&config()).unwrap();
        let LoggingBuildResult { guards, health, .. } = built;
        runner.attach_logging(guards, health.clone()).unwrap();
        // Second attach is rejected.
        assert!(matches!(
            runner.attach_logging(Vec::new(), health),
            Err(RunnerError::AlreadyAttached)
        ));
        runner.activate().unwrap();
        runner.poll_once(now()).unwrap();

        let health = runner.health();
        let logging = health.logging.expect("logging health attached");
        assert_eq!(logging.sinks.len(), 1);
        assert_eq!(
            logging.sinks[0].state,
            ruviform_observability::logging::SinkState::Active
        );

        let report = runner.shutdown(now()).unwrap();
        assert!(report.writers_closed);
        // Post-shutdown the sink health remains readable through the
        // retained handle semantics — guards are closed, counters stay.
        let health = runner.health();
        assert!(health.logging.is_some());
    }

    #[test]
    fn attach_logging_rejected_outside_active_states() {
        let dir = tempfile::tempdir().unwrap();
        let built = build_logging(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "obs11c")],
        })
        .unwrap();
        let LoggingBuildResult { guards, health, .. } = built;

        let mut runner = ObservationRunner::new();
        // Unconfigured — rejected.
        assert!(matches!(
            runner.attach_logging(guards, health),
            Err(RunnerError::InvalidState { .. })
        ));
        // Prepared — accepted.
        runner.prepare(&config()).unwrap();
        let built = build_logging(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "obs11d")],
        })
        .unwrap();
        let LoggingBuildResult { guards, health, .. } = built;
        runner.attach_logging(guards, health).unwrap();
    }

    #[test]
    fn restart_reattaches_fresh_logging() {
        let dir = tempfile::tempdir().unwrap();
        let mut runner = ObservationRunner::new();
        runner.prepare(&config()).unwrap();
        let built = build_logging(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "obs11r")],
        })
        .unwrap();
        let LoggingBuildResult { guards, health, .. } = built;
        runner.attach_logging(guards, health).unwrap();
        runner.activate().unwrap();
        runner.shutdown(now()).unwrap();

        // Restart: the drained attachment must not block re-attach.
        runner.prepare(&config()).unwrap();
        assert!(runner.health().logging.is_none());
        let built = build_logging(&LoggingConfig {
            sinks: vec![file_sink(dir.path(), "obs11r2")],
        })
        .unwrap();
        let LoggingBuildResult { guards, health, .. } = built;
        runner.attach_logging(guards, health).unwrap();
        runner.activate().unwrap();
        runner.shutdown(now()).unwrap();
    }
}

// --- debug session ---------------------------------------------------------------

#[cfg(feature = "integration")]
mod debug_session {
    use std::sync::Arc;

    use ruviform_observability::debug::{
        DebugAuthority, DebugConfig, DebugSessionState, SessionRequest,
    };

    use super::*;

    #[test]
    fn debug_session_state_feeds_health_and_drain_report() {
        let authority = Arc::new(DebugAuthority::new());
        let mut cfg = config();
        cfg.debug = Some(DebugSettings::with_authority(
            DebugConfig::default(),
            Arc::clone(&authority),
        ));

        let mut runner = ObservationRunner::new();
        runner.prepare(&cfg).unwrap();
        runner.activate().unwrap();

        // No authority -> no session; with authority -> prepare works.
        let permit = authority.permit().expect("authority mints permit");
        runner
            .debug_mut()
            .unwrap()
            .prepare_session(
                permit,
                SessionRequest {
                    label: "test.session",
                    max_records: 8,
                },
                now(),
            )
            .unwrap();

        let health = runner.health();
        let session = health.debug_session.expect("session view");
        assert_eq!(session.state, DebugSessionState::Prepared);

        // Abandoning mid-session is reported, never hidden.
        let report = runner.shutdown(now()).unwrap();
        assert!(report.debug_session_abandoned);
        assert_eq!(report.debug_records_lost, 0);
    }
}
