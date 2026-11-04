//! Generic host-owned logging adapter — Issue #276 (OBS-01), on the standard
//! `tracing` / `tracing-subscriber` / `tracing-appender` stack.
//!
//! Non-RT Ruviform code and consumers emit ordinary `tracing` events/spans
//! directly; this module does **not** introduce a Ruviform-specific logger or
//! span-emit API. It only builds the receiving side: per-sink `stderr` /
//! rolling-file destinations, per-sink [`EnvFilter`]-syntax filters and
//! text/JSON-lines formats, opt-in per-sink span lifecycle records
//! ([`SpanEvents`]), a bounded non-blocking writer path with
//! distinguished failure counters, and a health handle for observing them.
//!
//! # Ownership contract
//!
//! - **The host owns the subscriber.** [`build_logging`] returns a
//!   [`tracing::Dispatch`]; [`build_logging_layers`] returns composable
//!   [`tracing_subscriber::Layer`]s for hosts that already have (or want)
//!   their own subscriber stack. This library **never** calls
//!   `set_global_default`, `init`, or reads any environment variable.
//!   Installing the returned [`tracing::Dispatch`] globally is the host's
//!   decision — `tracing::dispatcher::set_global_default` returns an error if
//!   a global default already exists; scoped use via
//!   `tracing::dispatcher::with_default` always works. Global registration is
//!   a per-process, once-only host operation (no re-registration from
//!   multi-window contexts).
//! - **No stdout.** [`SinkDestination`] only offers `Stderr` and
//!   `RollingFile`. Nothing in this module writes to stdout, so structured
//!   output channels on stdout (e.g. an `inspect` JSON stream) are protected
//!   structurally.
//! - **No product-specific configuration.** Filter strings use `EnvFilter`
//!   directive syntax but are supplied explicitly through [`SinkConfig`];
//!   mapping env vars / CLI flags / GUI settings into a [`LoggingConfig`] is
//!   the host's job.
//!
//! # RT contract
//!
//! **Never emit ordinary `tracing` events from real-time (RT) contexts.**
//! Subscriber layers run synchronously on the emitting thread, and this
//! adapter's writer path allocates (each event is buffered into a `Vec<u8>`
//! before entering the bounded channel — and so are the synthesized span
//! lifecycle records enabled via [`SpanEvents`]). RT-side observation must
//! use dedicated RT-safe mechanisms instead.
//!
//! # Failure model
//!
//! - A `Required` sink that fails to initialize fails the whole build with
//!   [`LoggingBuildError::RequiredSinkFailed`]. A `BestEffort` sink that
//!   fails is recorded in [`LoggingBuildResult::degraded`] /
//!   [`LoggingLayersResult::degraded`] and the remaining sinks still work.
//!   If *every* configured sink fails, the build returns
//!   [`LoggingBuildError::AllSinksFailed`] — an all-dead configuration is
//!   never silently reported as success.
//! - At runtime, distinguished counters are kept per sink: `queue-full`
//!   drops, `WriteZero` partial writes, other write/rotation failures, flush
//!   failures, and channel disconnects. The logger's own failures are
//!   **never** fed back through the logging pipeline itself (no recursive
//!   logging); they are only observable through [`LoggingHealth`].
//!   One upstream caveat: `tracing-appender` reports a *rotation-time*
//!   file-creation failure to stderr itself and keeps writing to the
//!   previous file — that path cannot reach our counters. Write errors
//!   that do surface through `Write` are counted as `write_failures`.
//! - Each sink is drained by a dedicated background worker thread. Dropping
//!   the returned [`WriterGuard`]s signals shutdown; the worker drains and
//!   flushes before exiting and the guard joins it with a bounded timeout
//!   ([`SHUTDOWN_TIMEOUT`]). On timeout the worker is abandoned (never
//!   force-killed) and `shutdown_abandoned` is recorded. Buffered messages
//!   that never reach the sink — including on process crash/abort — are
//!   lost; see [`WriterGuard`] for the exact contract.
//!
//! # Sensitive data
//!
//! The adapter passes event fields through to the sinks verbatim. It does
//! not inspect, redact, or rate-limit event content — emitting raw PCM,
//! prompts, filesystem paths, or credentials is the emitting side's
//! responsibility, and this module never generates such events itself.

mod build;
mod config;
mod error;
mod health;
mod writer;

pub use build::{
    LoggingBuildResult, LoggingLayersResult, SinkDegraded, build_logging, build_logging_layers,
};
pub use config::{
    DEFAULT_STDERR_QUEUE_CAPACITY, LogFormat, LoggingConfig, RotationPeriod, SinkConfig,
    SinkDestination, SinkFailureMode, SpanEvents,
};
pub use error::{LoggingBuildError, SinkFailure};
pub use health::{HealthSnapshot, LoggingHealth, SinkHealthSnapshot, SinkKind, SinkState};
pub use writer::{SHUTDOWN_TIMEOUT, WriterGuard};
