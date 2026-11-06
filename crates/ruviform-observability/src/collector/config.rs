//! Collector configuration — bounded limits validated at
//! [`Collector::new`](crate::collector::Collector::new) so a
//! misconfigured collector fails typed instead of silently
//! starving or unbounding.

use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Limits and behaviour of a [`crate::collector::Collector`].
///
/// All limits are cardinality controls: `max_sources` bounds live
/// registrations, `max_series` bounds live *plus retained-removed*
/// series, `max_batch_size` bounds how many `Active` sources one poll
/// may observe (round-robin keeps the rest fair), and the
/// `history_*` pair bounds the process-local measurement ring.
///
/// A `history_*` value of `0` is **not an error** — it explicitly
/// disables history retention.
///
/// Use [`CollectorConfig::validate`] (called by
/// [`Collector::new`](crate::collector::Collector::new)) to reject
/// zero/invalid combinations up front.
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// The intended interval between polls — recorded in each
    /// [`ObservationWindow`](crate::collector::ObservationWindow) as
    /// `expected` and used by
    /// [`PollRunner`](crate::collector::PollRunner) for its sleep
    /// cycle. Must be non-zero.
    pub poll_interval: Duration,
    /// Maximum simultaneously live sources
    /// (`Active`/`Retiring`/`FinalDrain`). Must be non-zero.
    pub max_sources: usize,
    /// Maximum tracked series — live sources plus retained-removed
    /// tombstones. Must be `>= max_sources` and non-zero.
    pub max_series: usize,
    /// Maximum `Active` sources observed in one poll — excess is
    /// deferred fairly by a round-robin cursor (retiring sources are
    /// always drained and do not consume this budget). Must be
    /// non-zero.
    pub max_batch_size: usize,
    /// Maximum measurements kept in the history ring. `0` disables
    /// history.
    pub history_max_samples: usize,
    /// Approximate byte budget of the history ring (measurement fixed
    /// part plus histogram bucket payloads). `0` disables history.
    pub history_budget_bytes: usize,
    /// When `Some(d)`, an `Active` source whose value has not changed
    /// for longer than `d` is reported `Stale`. `None` (default)
    /// disables automatic staleness. `Some(Duration::ZERO)` is
    /// rejected.
    pub stale_after: Option<Duration>,
}

impl CollectorConfig {
    /// Validates the configured limits.
    ///
    /// # Errors
    ///
    /// Returns the typed [`CollectorConfigError`] variant matching the
    /// first invalid field.
    pub fn validate(&self) -> Result<(), CollectorConfigError> {
        if self.poll_interval.is_zero() {
            return Err(CollectorConfigError::ZeroPollInterval);
        }
        if self.max_sources == 0 {
            return Err(CollectorConfigError::ZeroMaxSources);
        }
        if self.max_series == 0 {
            return Err(CollectorConfigError::ZeroMaxSeries);
        }
        if self.max_batch_size == 0 {
            return Err(CollectorConfigError::ZeroMaxBatchSize);
        }
        if self.stale_after.is_some_and(|d| d.is_zero()) {
            return Err(CollectorConfigError::ZeroStaleThreshold);
        }
        if self.max_series < self.max_sources {
            return Err(CollectorConfigError::SeriesBelowSources {
                max_series: self.max_series,
                max_sources: self.max_sources,
            });
        }
        Ok(())
    }
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(1000),
            max_sources: 256,
            max_series: 1024,
            max_batch_size: 256,
            history_max_samples: 1024,
            history_budget_bytes: 256 * 1024,
            stale_after: None,
        }
    }
}

/// Error returned by [`Collector::new`](crate::collector::Collector::new)
/// when a [`CollectorConfig`] fails validation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorConfigError {
    /// `poll_interval` was zero — a polling collector cannot run on a
    /// zero-length cycle (the runner would busy-spin).
    ZeroPollInterval,
    /// `max_sources` was zero — no source could ever register.
    ZeroMaxSources,
    /// `max_series` was zero — no series could ever be tracked.
    ZeroMaxSeries,
    /// `max_batch_size` was zero — every `Active` source would starve
    /// forever.
    ZeroMaxBatchSize,
    /// `stale_after` was `Some(0)` — every source would flap stale.
    ZeroStaleThreshold,
    /// `max_series < max_sources` — the retained budget must cover
    /// every live source.
    SeriesBelowSources {
        /// The configured series limit.
        max_series: usize,
        /// The configured live-source limit.
        max_sources: usize,
    },
}

impl fmt::Display for CollectorConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ZeroPollInterval => f.write_str("poll_interval must be non-zero"),
            Self::ZeroMaxSources => f.write_str("max_sources must be non-zero"),
            Self::ZeroMaxSeries => f.write_str("max_series must be non-zero"),
            Self::ZeroMaxBatchSize => f.write_str("max_batch_size must be non-zero"),
            Self::ZeroStaleThreshold => f.write_str("stale_after must be non-zero when set"),
            Self::SeriesBelowSources {
                max_series,
                max_sources,
            } => write!(
                f,
                "max_series ({max_series}) must be >= max_sources ({max_sources})"
            ),
        }
    }
}

impl Error for CollectorConfigError {}
