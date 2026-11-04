//! Health observation for the logging adapter.
//!
//! [`LoggingHealth`] is a cloneable handle over per-sink atomic counters.
//! `snapshot()` performs plain `Relaxed` loads — it never emits `tracing`
//! events and never touches the sinks, so the logger's own state is
//! measurable without re-entering the logging pipeline (non-recursive
//! measurement). The logger's failures are observable **only** through this
//! handle; they are never written to the sinks themselves.

use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Coarse lifecycle state of a built sink, as seen by health observation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkState {
    /// The sink initialized and its writer channel is live.
    Active,
    /// The sink's writer channel disconnected — the worker thread exited
    /// (panicked or completed shutdown) or was abandoned after a shutdown
    /// timeout. Events for this sink are dropped and counted in
    /// `channel_disconnected`.
    Failed,
}

impl fmt::Display for SinkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

/// Which destination kind a sink was built with — kept free of path/prefix
/// details so snapshots stay cheap and don't echo filesystem layout.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkKind {
    /// [`SinkDestination::Stderr`](crate::logging::SinkDestination::Stderr).
    Stderr,
    /// [`SinkDestination::RollingFile`](crate::logging::SinkDestination::RollingFile).
    RollingFile,
}

impl fmt::Display for SinkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stderr => f.write_str("stderr"),
            Self::RollingFile => f.write_str("rolling-file"),
        }
    }
}

/// Point-in-time view of one sink's counters — all counters are
/// monotonically non-decreasing while the sink exists:
///
/// - `events_written`: messages the worker wrote to the sink without error.
/// - `dropped_queue_full`: formatted events dropped because the bounded
///   queue was full (the emitting thread is never blocked).
/// - `partial_writes`: writes where the sink reported `WriteZero`
///   (`Ok(0)` progress) — partial-write failure.
/// - `write_failures`: other `io::Error`s from `write_all`, including
///   rotation-related failures surfaced as write errors.
/// - `flush_failures`: `io::Error`s from the final `flush` during worker
///   shutdown.
/// - `channel_disconnected`: events dropped because the sink's channel was
///   already disconnected (worker dead or shut down) — each such event is
///   counted here, not in `dropped_queue_full`.
/// - `shutdown_abandoned`: the [`WriterGuard`](crate::logging::WriterGuard)
///   shutdown wait timed out and the worker was abandoned rather than
///   joined.
#[derive(Debug, Clone)]
pub struct SinkHealthSnapshot {
    /// Index of the sink in the `sinks` list of the supplied
    /// `LoggingConfig`.
    pub sink_index: usize,
    /// Destination kind.
    pub kind: SinkKind,
    /// Coarse lifecycle state.
    pub state: SinkState,
    /// Messages the worker wrote to the sink without error.
    pub events_written: u64,
    /// Events dropped because the bounded queue was full.
    pub dropped_queue_full: u64,
    /// `WriteZero` partial-write failures.
    pub partial_writes: u64,
    /// Other write/rotation failures.
    pub write_failures: u64,
    /// Final-flush failures during shutdown.
    pub flush_failures: u64,
    /// Events dropped after the channel disconnected.
    pub channel_disconnected: u64,
    /// `true` once a shutdown wait timed out and the worker was abandoned.
    pub shutdown_abandoned: bool,
}

/// Whole-adapter point-in-time view returned by
/// [`LoggingHealth::snapshot`].
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    /// One entry per successfully built sink, in `sinks` order.
    /// Build-time `BestEffort` failures do **not** produce entries — they
    /// live in the build result's `degraded` list.
    pub sinks: Vec<SinkHealthSnapshot>,
}

/// Cloneable handle to the adapter's per-sink counters.
///
/// Obtained from [`LoggingBuildResult::health`](crate::logging::LoggingBuildResult::health)
/// / [`LoggingLayersResult::health`](crate::logging::LoggingLayersResult::health).
/// Clones share the same counters.
#[derive(Debug, Clone)]
pub struct LoggingHealth {
    sinks: std::sync::Arc<[std::sync::Arc<SinkMetrics>]>,
}

impl LoggingHealth {
    /// Takes a non-recursive snapshot: atomic `Relaxed` loads only. Never
    /// emits `tracing` events, never writes to any sink, and is safe to
    /// call repeatedly from a monitoring path.
    #[must_use]
    pub fn snapshot(&self) -> HealthSnapshot {
        HealthSnapshot {
            sinks: self
                .sinks
                .iter()
                .map(|metrics| metrics.snapshot())
                .collect(),
        }
    }
}

/// Per-sink shared counters — the writer/worker side increments, the
/// health side reads. Hot-path counters are plain atomics; the rarely
/// touched `state`/`shutdown_abandoned` flags sit behind a `Mutex`.
#[derive(Debug)]
pub(crate) struct SinkMetrics {
    pub(crate) sink_index: usize,
    pub(crate) kind: SinkKind,
    state: Mutex<SinkState>,
    pub(crate) events_written: AtomicU64,
    pub(crate) dropped_queue_full: AtomicU64,
    pub(crate) partial_writes: AtomicU64,
    pub(crate) write_failures: AtomicU64,
    pub(crate) flush_failures: AtomicU64,
    pub(crate) channel_disconnected: AtomicU64,
    shutdown_abandoned: Mutex<bool>,
}

impl SinkMetrics {
    pub(crate) fn new(sink_index: usize, kind: SinkKind) -> Self {
        Self {
            sink_index,
            kind,
            state: Mutex::new(SinkState::Active),
            events_written: AtomicU64::new(0),
            dropped_queue_full: AtomicU64::new(0),
            partial_writes: AtomicU64::new(0),
            write_failures: AtomicU64::new(0),
            flush_failures: AtomicU64::new(0),
            channel_disconnected: AtomicU64::new(0),
            shutdown_abandoned: Mutex::new(false),
        }
    }

    pub(crate) fn set_state(&self, state: SinkState) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = state;
        }
    }

    pub(crate) fn mark_shutdown_abandoned(&self) {
        if let Ok(mut guard) = self.shutdown_abandoned.lock() {
            *guard = true;
        }
    }

    pub(crate) fn snapshot(&self) -> SinkHealthSnapshot {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        SinkHealthSnapshot {
            sink_index: self.sink_index,
            kind: self.kind,
            state: self.state.lock().map_or(SinkState::Failed, |s| *s),
            events_written: load(&self.events_written),
            dropped_queue_full: load(&self.dropped_queue_full),
            partial_writes: load(&self.partial_writes),
            write_failures: load(&self.write_failures),
            flush_failures: load(&self.flush_failures),
            channel_disconnected: load(&self.channel_disconnected),
            shutdown_abandoned: self.shutdown_abandoned.lock().is_ok_and(|flag| *flag),
        }
    }
}

/// Collects per-sink metrics during a build; `finish` produces the shared
/// [`LoggingHealth`] handle.
#[derive(Debug, Default)]
pub(crate) struct HealthBuilder {
    sinks: Vec<std::sync::Arc<SinkMetrics>>,
}

impl HealthBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, metrics: std::sync::Arc<SinkMetrics>) {
        self.sinks.push(metrics);
    }

    pub(crate) fn finish(self) -> LoggingHealth {
        LoggingHealth {
            sinks: self.sinks.into(),
        }
    }
}
