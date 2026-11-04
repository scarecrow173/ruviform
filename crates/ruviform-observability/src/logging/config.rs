//! Configuration types for the logging adapter — the host-supplied inputs
//! to [`build_logging`](crate::logging::build_logging) /
//! [`build_logging_layers`](crate::logging::build_logging_layers).

use std::num::NonZeroUsize;
use std::path::PathBuf;

/// Queue capacity used for the [`SinkDestination::Stderr`] sink.
///
/// `Stderr` has no configurable `queue_capacity` field in the Issue #276 API
/// sketch, so its bounded writer uses this fixed default instead of growing
/// the public config surface. The sink still drops (and counts) events when
/// the bound is exceeded — it never blocks the emitting thread.
pub const DEFAULT_STDERR_QUEUE_CAPACITY: usize = 4096;

/// Top-level logging configuration: an ordered list of sinks.
///
/// An empty `sinks` list is rejected with
/// [`LoggingBuildError::NoSinks`](crate::logging::LoggingBuildError::NoSinks)
/// at build time rather than producing a silently dead pipeline.
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
///         filter: "info".to_owned(),
///         failure_mode: SinkFailureMode::Required,
///     }],
/// };
/// let built = build_logging(&config).unwrap();
/// # built.guards.len();
/// ```
#[derive(Debug)]
pub struct LoggingConfig {
    /// The sinks to build. Each entry is independent: own destination,
    /// format, filter, and failure mode.
    pub sinks: Vec<SinkConfig>,
}

/// One sink: destination + format + filter + failure mode.
///
/// `filter` uses [`tracing_subscriber::filter::EnvFilter`] directive syntax
/// (e.g. `"info"`, `"my_crate=debug,warn"`, `"off"`). It is applied
/// per-sink via `Layer::with_filter`, so sinks route the same events
/// independently. An empty string follows `EnvFilter`'s default behaviour
/// (equivalent to enabling `ERROR` and above). An unparsable value fails
/// the build with
/// [`LoggingBuildError::InvalidFilter`](crate::logging::LoggingBuildError::InvalidFilter).
#[derive(Debug)]
pub struct SinkConfig {
    /// Where the formatted records go.
    pub destination: SinkDestination,
    /// How records are rendered before reaching the destination.
    pub format: LogFormat,
    /// `EnvFilter` directive string applied to this sink only.
    pub filter: String,
    /// Whether a build-time failure of this sink aborts the whole build
    /// (`Required`) or is recorded as degraded (`BestEffort`).
    pub failure_mode: SinkFailureMode,
}

/// Sink destination — where formatted records are written.
///
/// There is deliberately **no stdout destination**: nothing built by this
/// adapter can write to stdout.
#[non_exhaustive]
#[derive(Debug)]
pub enum SinkDestination {
    /// Write to the process standard error stream, through the shared
    /// bounded writer path with [`DEFAULT_STDERR_QUEUE_CAPACITY`].
    Stderr,
    /// Write to rolling files under `directory`, rotated on `rotation` and
    /// kept to at most `max_files` matching files.
    ///
    /// File names are `{filename_prefix}.{rotation-date}` (e.g.
    /// `myapp.2026-11-04` for daily rotation).
    ///
    /// Retention is delegated to the appender's built-in `max_log_files`
    /// mechanism: when the appender is constructed and at each rotation,
    /// files matching the prefix (non-directory, non-symlink) are deleted
    /// oldest-first until at most `max_files` remain. Note the upstream
    /// caveat that the retained count can occasionally dip *below* the
    /// maximum; `max_files` is a cap, not a guaranteed minimum.
    ///
    /// `queue_capacity` bounds how many formatted events may wait in memory
    /// for the sink's background worker; events arriving beyond that bound
    /// are dropped and counted as `dropped_queue_full` in the health
    /// snapshot.
    RollingFile {
        /// Directory the rolling files live in; created recursively if
        /// missing. A path that exists as a non-directory or cannot be
        /// created fails sink initialization.
        directory: PathBuf,
        /// Prefix prepended to the rotation date in the file name.
        filename_prefix: String,
        /// How often a new file is started (daily / hourly / minutely /
        /// never).
        rotation: RotationPeriod,
        /// Maximum number of matching files retained (oldest deleted
        /// first). Must be non-zero.
        max_files: NonZeroUsize,
        /// Bounded in-memory queue of formatted events awaiting the sink's
        /// background writer. Must be non-zero.
        queue_capacity: NonZeroUsize,
    },
}

/// Record rendering format for a sink.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable single-line text (tracing-subscriber default
    /// formatter, ANSI codes disabled for deterministic output).
    Text,
    /// One JSON object per line (JSON Lines / JSONL).
    JsonLines,
}

/// Rolling-file rotation period, mapped onto `tracing_appender`'s
/// `Rotation`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationPeriod {
    /// Rotate once per day (`{prefix}.YYYY-MM-DD`).
    Daily,
    /// Rotate once per hour (`{prefix}.YYYY-MM-DD-HH`).
    Hourly,
    /// Rotate once per minute (`{prefix}.YYYY-MM-DD-HH-MM`). Mostly useful
    /// for verifying rotation behaviour without waiting.
    Minutely,
    /// Never rotate — a single `{prefix}` file (no date suffix) accumulates
    /// all records.
    Never,
}

/// What a build-time sink failure means for the overall build.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkFailureMode {
    /// The sink may fail: record it in the build result's `degraded` list
    /// and keep the remaining sinks running.
    BestEffort,
    /// The sink must initialize: any failure aborts the whole build with
    /// [`LoggingBuildError::RequiredSinkFailed`](crate::logging::LoggingBuildError::RequiredSinkFailed).
    Required,
}
