//! Builder entry points — [`build_logging`] / [`build_logging_layers`].
//!
//! Both functions share [`build_sinks`]: per-sink filter parsing,
//! destination initialization, bounded-writer spawn, and `fmt` layer
//! construction. Neither installs anything globally — see the
//! module-level ownership contract.

use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::sync::Arc;

use tracing::Dispatch;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::{Registry, fmt, registry};

use super::config::{
    DEFAULT_STDERR_QUEUE_CAPACITY, LogFormat, LoggingConfig, RotationPeriod, SinkConfig,
    SinkDestination, SinkFailureMode, SpanEvents,
};
use super::error::{LoggingBuildError, SinkFailure};
use super::health::{HealthBuilder, LoggingHealth, SinkKind, SinkMetrics};
use super::writer::{self, BoundedSender, WriterGuard};

/// Composable layer set returned by [`build_logging_layers`].
///
/// Feeding these into `registry().with(result.layers)` produces the same
/// subscriber as [`build_logging`]'s `dispatch`, but lets the host add its
/// own layers before (or after) in the same registry.
pub type LoggingLayers = Vec<Box<dyn Layer<Registry> + Send + Sync>>;

/// A `BestEffort` sink that failed to initialize: build continued without
/// it and this record is the explicit notification — never silent.
///
/// `Clone` is derived so hosts can retain snapshot copies of the degraded
/// record (issue #336); the health snapshot types in `health` are `Clone`
/// for the same reason.
#[derive(Debug, Clone)]
pub struct SinkDegraded {
    /// Index of the failed sink in the `sinks` list of the supplied
    /// `LoggingConfig`.
    pub sink_index: usize,
    /// Which destination kind failed.
    pub kind: SinkKind,
    /// Human-readable initialization failure detail.
    pub reason: String,
}

/// Result of [`build_logging`]: a ready-to-use [`Dispatch`] plus the
/// ownership pieces the host must keep.
///
/// `dispatch` is **not** installed anywhere — call
/// `tracing::dispatcher::set_global_default(dispatch)` once per process to
/// make it global, or use `tracing::dispatcher::with_default(&dispatch, f)`
/// for scoped use (e.g. tests, or when a global default already exists).
///
/// Keep `guards` alive for the dispatch's lifetime; dropping a guard
/// shuts down its sink's writer (see [`WriterGuard`]).
#[derive(Debug)]
#[must_use]
pub struct LoggingBuildResult {
    /// The composed subscriber as a `Dispatch`, not yet installed.
    pub dispatch: Dispatch,
    /// Per-sink writer shutdown handles, in `sinks` order.
    pub guards: Vec<WriterGuard>,
    /// `BestEffort` sinks that failed to initialize (explicit, not silent).
    pub degraded: Vec<SinkDegraded>,
    /// Health observation handle — cloneable, non-recursive.
    pub health: LoggingHealth,
}

/// Result of [`build_logging_layers`]: composable layers plus the same
/// ownership pieces as [`LoggingBuildResult`].
#[must_use]
pub struct LoggingLayersResult {
    /// Per-sink layers for `registry().with(..)` composition.
    pub layers: LoggingLayers,
    /// Per-sink writer shutdown handles, in `sinks` order.
    pub guards: Vec<WriterGuard>,
    /// `BestEffort` sinks that failed to initialize (explicit, not silent).
    pub degraded: Vec<SinkDegraded>,
    /// Health observation handle — cloneable, non-recursive.
    pub health: LoggingHealth,
}

impl std::fmt::Debug for LoggingLayersResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingLayersResult")
            .field("layers", &format_args!("{} layer(s)", self.layers.len()))
            .field("guards", &self.guards)
            .field("degraded", &self.degraded)
            .field("health", &self.health)
            .finish()
    }
}

/// Builds a [`Dispatch`] from the given configuration.
///
/// The returned dispatch is **not** installed globally — the host decides
/// how to use it (once-per-process `set_global_default`, or scoped
/// `dispatch::with_default`). See the module-level ownership contract.
///
/// # Errors
///
/// - [`LoggingBuildError::NoSinks`] if `config.sinks` is empty.
/// - [`LoggingBuildError::InvalidFilter`] if any sink's `filter` fails to
///   parse as an `EnvFilter` directive.
/// - [`LoggingBuildError::RequiredSinkFailed`] if a `Required` sink fails
///   to initialize (already-built sinks are shut down first).
/// - [`LoggingBuildError::AllSinksFailed`] if every sink failed — an
///   all-dead pipeline is never reported as success.
///
/// # Examples
///
/// ```
/// use ruviform_observability::logging::*;
///
/// let config = LoggingConfig {
///     sinks: vec![SinkConfig {
///         destination: SinkDestination::Stderr,
///         format: LogFormat::Text,
///         span_events: SpanEvents::NONE,
///         filter: "info".to_owned(),
///         failure_mode: SinkFailureMode::Required,
///     }],
/// };
/// let result = build_logging(&config).unwrap();
/// // Scoped use — the library never installs a global subscriber:
/// tracing::dispatcher::with_default(&result.dispatch, || {
///     tracing::info!("hello from the host");
/// });
/// ```
pub fn build_logging(config: &LoggingConfig) -> Result<LoggingBuildResult, LoggingBuildError> {
    let built = build_sinks(config)?;
    let dispatch = Dispatch::new(registry().with(built.layers));
    Ok(LoggingBuildResult {
        dispatch,
        guards: built.guards,
        degraded: built.degraded,
        health: built.health,
    })
}

/// Builds composable [`tracing_subscriber::Layer`]s instead of a finished
/// `Dispatch`, for hosts that need to add their own layers/subscriber.
///
/// Usage: `registry().with(result.layers).with(host_layer)` — then install
/// or scope that registry as the host sees fit. Failure semantics are
/// identical to [`build_logging`].
///
/// # Errors
///
/// Same as [`build_logging`].
pub fn build_logging_layers(
    config: &LoggingConfig,
) -> Result<LoggingLayersResult, LoggingBuildError> {
    let built = build_sinks(config)?;
    Ok(LoggingLayersResult {
        layers: built.layers,
        guards: built.guards,
        degraded: built.degraded,
        health: built.health,
    })
}

/// Internal shared build: returns the layers plus the ownership pieces
/// both public entry points hand back.
struct BuiltSinks {
    layers: LoggingLayers,
    guards: Vec<WriterGuard>,
    degraded: Vec<SinkDegraded>,
    health: LoggingHealth,
}

fn build_sinks(config: &LoggingConfig) -> Result<BuiltSinks, LoggingBuildError> {
    if config.sinks.is_empty() {
        return Err(LoggingBuildError::NoSinks);
    }
    let mut layers = Vec::with_capacity(config.sinks.len());
    let mut guards = Vec::with_capacity(config.sinks.len());
    let mut degraded = Vec::new();
    let mut failures = Vec::new();
    let mut health = HealthBuilder::new();
    for (sink_index, sink) in config.sinks.iter().enumerate() {
        // Filter strings are validated for every sink — a malformed filter
        // is a host configuration bug, not a sink failure, so it always
        // aborts the build regardless of `failure_mode`.
        let filter =
            EnvFilter::try_new(&sink.filter).map_err(|e| LoggingBuildError::InvalidFilter {
                sink_index,
                filter: sink.filter.clone(),
                reason: e.to_string(),
            })?;
        match init_sink(sink_index, sink, filter) {
            Ok(built) => {
                layers.push(built.layer);
                guards.push(built.guard);
                health.register(built.metrics);
            }
            Err(source) => {
                let kind = sink_kind(&sink.destination);
                match sink.failure_mode {
                    SinkFailureMode::Required => {
                        // `layers`/`guards` built so far drop here — each
                        // guard performs its normal drain+join shutdown.
                        return Err(LoggingBuildError::RequiredSinkFailed {
                            failure: SinkFailure {
                                sink_index,
                                kind,
                                source,
                            },
                        });
                    }
                    SinkFailureMode::BestEffort => {
                        degraded.push(SinkDegraded {
                            sink_index,
                            kind,
                            reason: source.to_string(),
                        });
                        failures.push(SinkFailure {
                            sink_index,
                            kind,
                            source,
                        });
                    }
                }
            }
        }
    }
    if layers.is_empty() {
        // Every sink was BestEffort and every one failed — an all-dead
        // pipeline must not be reported as success.
        return Err(LoggingBuildError::AllSinksFailed { failures });
    }
    Ok(BuiltSinks {
        layers,
        guards,
        degraded,
        health: health.finish(),
    })
}

fn sink_kind(destination: &SinkDestination) -> SinkKind {
    // No wildcard arm: `#[non_exhaustive]` only relaxes matching for
    // downstream crates, so a newly added variant must fail compilation
    // here until it is deliberately mapped.
    match destination {
        SinkDestination::Stderr => SinkKind::Stderr,
        SinkDestination::RollingFile { .. } => SinkKind::RollingFile,
    }
}

/// One successfully initialized sink.
struct BuiltSink {
    layer: Box<dyn Layer<Registry> + Send + Sync>,
    guard: WriterGuard,
    metrics: Arc<SinkMetrics>,
}

fn init_sink(sink_index: usize, sink: &SinkConfig, filter: EnvFilter) -> io::Result<BuiltSink> {
    let (kind, inner, queue_capacity) = init_destination(&sink.destination)?;
    let metrics = Arc::new(SinkMetrics::new(sink_index, kind));
    let (sender, guard) = writer::spawn(inner, queue_capacity, Arc::clone(&metrics));
    Ok(BuiltSink {
        layer: make_layer(sink.format, sink.span_events, sender, filter),
        guard,
        metrics,
    })
}

fn init_destination(
    destination: &SinkDestination,
) -> io::Result<(SinkKind, Box<dyn Write + Send>, NonZeroUsize)> {
    match destination {
        SinkDestination::Stderr => Ok((
            SinkKind::Stderr,
            Box::new(io::stderr()),
            // `DEFAULT_STDERR_QUEUE_CAPACITY` is a nonzero literal — this
            // cannot fail.
            NonZeroUsize::new(DEFAULT_STDERR_QUEUE_CAPACITY).expect("nonzero"),
        )),
        SinkDestination::RollingFile {
            directory,
            filename_prefix,
            filename_suffix,
            rotation,
            max_files,
            queue_capacity,
        } => {
            // Create the directory up front ourselves: `build()` would
            // eventually do this inside `create_writer`, but it runs
            // `prune_old_logs` *first* — which `eprintln!`s a spurious
            // read error on a missing directory. Pre-creating keeps
            // stderr clean and surfaces a plain io error for invalid or
            // readonly directories.
            std::fs::create_dir_all(directory)?;
            let mut builder = RollingFileAppender::builder()
                .rotation(rotation_period(*rotation))
                .filename_prefix(filename_prefix)
                .max_log_files(max_files.get());
            if let Some(suffix) = filename_suffix {
                builder = builder.filename_suffix(suffix);
            }
            let appender = builder.build(directory).map_err(io::Error::other)?;
            Ok((SinkKind::RollingFile, Box::new(appender), *queue_capacity))
        }
    }
}

fn rotation_period(period: RotationPeriod) -> Rotation {
    match period {
        RotationPeriod::Daily => Rotation::DAILY,
        RotationPeriod::Hourly => Rotation::HOURLY,
        RotationPeriod::Minutely => Rotation::MINUTELY,
        RotationPeriod::Never => Rotation::NEVER,
    }
}

/// Maps the crate-owned [`SpanEvents`] flags onto `FmtSpan`. The mapping
/// is explicit rather than bit-punning so the public API never depends on
/// the upstream flag layout.
fn fmt_span(events: SpanEvents) -> fmt::format::FmtSpan {
    let mut kind = fmt::format::FmtSpan::NONE;
    for (flag, upstream) in [
        (SpanEvents::NEW, fmt::format::FmtSpan::NEW),
        (SpanEvents::ENTER, fmt::format::FmtSpan::ENTER),
        (SpanEvents::EXIT, fmt::format::FmtSpan::EXIT),
        (SpanEvents::CLOSE, fmt::format::FmtSpan::CLOSE),
    ] {
        if events.contains(flag) {
            kind |= upstream;
        }
    }
    kind
}

/// Builds one sink's `fmt` layer: per-sink format + per-sink span lifecycle
/// records + per-sink `EnvFilter` applied via `Layer::with_filter`, writing
/// through the bounded writer. ANSI is disabled so text output stays
/// deterministic.
fn make_layer(
    format: LogFormat,
    span_events: SpanEvents,
    sender: BoundedSender,
    filter: EnvFilter,
) -> Box<dyn Layer<Registry> + Send + Sync> {
    let span_events = fmt_span(span_events);
    match format {
        LogFormat::Text => Box::new(
            fmt::layer::<Registry>()
                .with_ansi(false)
                .with_span_events(span_events)
                .with_writer(move || sender.event_writer())
                .with_filter(filter),
        ),
        LogFormat::JsonLines => Box::new(
            fmt::layer::<Registry>()
                .json()
                .with_ansi(false)
                .with_span_events(span_events)
                .with_writer(move || sender.event_writer())
                .with_filter(filter),
        ),
    }
}
