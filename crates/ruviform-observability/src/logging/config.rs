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
///         span_events: SpanEvents::NONE,
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
    /// format, span events, filter, and failure mode.
    pub sinks: Vec<SinkConfig>,
}

/// One sink: destination + format + span events + filter + failure mode.
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
    /// Which span lifecycle records this sink emits.
    ///
    /// [`SpanEvents::NONE`] (the [`Default`]) emits no span lifecycle
    /// records — the pre-#335 behaviour. [`SpanEvents::CLOSE`] makes a span
    /// with no inner events still leave a `close` record carrying its
    /// fields plus `time.busy`/`time.idle`, which is what makes leaf-span
    /// observation possible (the `AudioLens` AL-OBS-01 requirement). The
    /// records are synthesized by this sink's layer at the *span's* level
    /// and target, so they pass through `filter` like ordinary events.
    pub span_events: SpanEvents,
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
    /// `myapp.2026-11-04` for daily rotation), or
    /// `{filename_prefix}.{rotation-date}.{filename_suffix}` when
    /// `filename_suffix` is `Some` (e.g. `myapp.2026-11-04.log`). With
    /// [`RotationPeriod::Never`] there is no date component, so the names
    /// are `{filename_prefix}` / `{filename_prefix}.{filename_suffix}`.
    ///
    /// Retention is delegated to the appender's built-in `max_log_files`
    /// mechanism: when the appender is constructed and at each rotation,
    /// files matching the prefix *and* the suffix (non-directory,
    /// non-symlink) are deleted oldest-first until at most `max_files`
    /// remain. Note the upstream caveat that the retained count can
    /// occasionally dip *below* the maximum; `max_files` is a cap, not a
    /// guaranteed minimum.
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
        /// Optional extension-like suffix appended after the rotation
        /// date, producing `{prefix}.{rotation-date}.{suffix}` file names
        /// (e.g. `Some("log")` → `app.2026-11-04.log`). `None` keeps the
        /// original `{prefix}.{rotation-date}` naming. The value is passed
        /// verbatim to `tracing_appender`'s `filename_suffix` — supply the
        /// bare extension without a leading dot.
        filename_suffix: Option<String>,
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

/// Which span lifecycle records a sink emits — the adapter-owned
/// equivalent of tracing-subscriber's `FmtSpan`, kept crate-specific so
/// no upstream flag type appears in the public API (the same policy that
/// wraps `Rotation` as [`RotationPeriod`] and the formatter choice as
/// [`LogFormat`]).
///
/// Combine flags with `|`: `SpanEvents::NEW | SpanEvents::CLOSE`. The
/// composite presets [`SpanEvents::ACTIVE`] (`ENTER | EXIT`) and
/// [`SpanEvents::FULL`] (every lifecycle point) cover the common cases.
///
/// [`SpanEvents::CLOSE`] records carry the closing span's fields plus
/// `time.busy`/`time.idle` durations — the leaf-span observation path.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct SpanEvents(u8);

impl SpanEvents {
    /// One record when a span is created.
    pub const NEW: Self = Self(1 << 0);
    /// One record per span enter.
    pub const ENTER: Self = Self(1 << 1);
    /// One record per span exit.
    pub const EXIT: Self = Self(1 << 2);
    /// One record when a span closes — the flag that makes a leaf span
    /// with no inner events observable.
    pub const CLOSE: Self = Self(1 << 3);
    /// No span lifecycle records — the default, matching upstream
    /// `FmtSpan::NONE`.
    pub const NONE: Self = Self(0);
    /// `ENTER | EXIT` — one record per span enter/exit.
    pub const ACTIVE: Self = Self(Self::ENTER.0 | Self::EXIT.0);
    /// `NEW | ENTER | EXIT | CLOSE` — a record at every lifecycle point.
    pub const FULL: Self = Self(Self::NEW.0 | Self::ENTER.0 | Self::EXIT.0 | Self::CLOSE.0);

    /// Whether all of `other`'s flags are set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for SpanEvents {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for SpanEvents {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0);
    }
}

impl std::fmt::Debug for SpanEvents {
    // Same flag-name rendering as upstream `FmtSpan`'s `Debug`, so config
    // dumps read `SpanEvents::NEW | SpanEvents::CLOSE` rather than a raw
    // bit pattern.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut wrote = false;
        for (flag, name) in [
            (Self::NEW, "SpanEvents::NEW"),
            (Self::ENTER, "SpanEvents::ENTER"),
            (Self::EXIT, "SpanEvents::EXIT"),
            (Self::CLOSE, "SpanEvents::CLOSE"),
        ] {
            if self.contains(flag) {
                if wrote {
                    f.write_str(" | ")?;
                }
                f.write_str(name)?;
                wrote = true;
            }
        }
        if !wrote {
            f.write_str("SpanEvents::NONE")?;
        }
        Ok(())
    }
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
