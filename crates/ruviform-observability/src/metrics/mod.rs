//! Typed measurement vocabulary — the value types every collector
//! output ([`ObservationBatch`](crate::collector::ObservationBatch),
//! [`MetricsSnapshot`](crate::collector::MetricsSnapshot), history)
//! is built from.
//!
//! This module is deliberately the *shared vocabulary* for metrics:
//! the [`crate::collector`] poll engine produces [`Measurement`]s and
//! downstream consumers (tracing export, diagnostics, debug UI —
//! Issues #278/#291/#292) read them without re-implementing
//! baseline/delta/rate/validity logic.
//!
//! # Semantics and precision
//!
//! - **None vs zero.** [`Measurement::value`] is `Option`: `None` means
//!   the metric has no reportable value (`Unavailable`/`Unsupported`),
//!   which is never represented as a numeric `0`. A gauge observed at
//!   zero reports `Some(Gauge { value: 0, .. })` — an *observed* zero.
//! - **Delta/rate are optional.** A counter whose raw value decreased
//!   (wrap/reset suspicion) or whose elapsed anchor failed yields
//!   `delta: None` / `rate_per_sec: None` rather than a fabricated
//!   huge delta — loss of confidence is typed, not guessed.
//! - **Rate precision.** `rate_per_sec` is `f64` derived from a `u64`
//!   delta and a measured [`Duration`]. It is a display/aggregation
//!   grade figure: deltas above 2^53 lose integer precision, which is
//!   acceptable for a rate but callers must not round-trip it into a
//!   count.
//! - **Histogram consistency.** Bucket reads are per-bucket `Relaxed`
//!   loads — *not* a transactional snapshot. `total_approx` is the sum
//!   of those loads and is approximate by construction.
//! - **Clock.** All instants come from `std::time::Instant` (a host
//!   monotonic clock); no wall-clock time is fabricated and no
//!   stamp↔frame conversion is performed here (that is
//!   [`ruviform_media_core`]'s `ClockAnchor` responsibility).
//!
//! # Provenance
//!
//! A [`Measurement`] records *when it was observed* (`observed_at` —
//! the collector's poll instant) and *when the value last changed*
//! (`last_change_at` — the latest poll at which the raw value differed
//! from its predecessor). An abandoned or collector-forced final drain
//! is observable through `validity: Partial` +
//! `issue: WriterNotQuiesced`, never silently `Valid`.

use std::time::Instant;

use crate::source::{MetricDescriptor, ObservationScope, SourceIdentity, SourceLifecycle};

/// The validity classification of a single observation.
///
/// This is the typed answer to "how much can I trust this value" —
/// orthogonal to [`MeasurementIssue`], which carries *why*.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementValidity {
    /// Observed normally — the value is as reported.
    Valid,
    /// The value is reported but the source has not changed past its
    /// configured `stale_after` threshold — freshness is suspect,
    /// not the value itself.
    Stale,
    /// Only part of the intended observation could be committed —
    /// e.g. a final drain whose writer quiescence was never confirmed
    /// (lease dropped without `retire`, or a collector-forced drain at
    /// shutdown).
    Partial,
    /// A discontinuity was detected (counter decreased, reset/wrap
    /// suspicion, re-registration boundary). The raw value is current,
    /// but delta/rate against the previous baseline are suppressed.
    Discontinuous,
    /// No value exists — the source was removed (retained tombstone)
    /// or was never observed. `value` is `None`.
    Unavailable,
    /// Reserved for future metric kinds the collector cannot
    /// aggregate. No current trigger exists — the variant keeps the
    /// contract forward-compatible.
    Unsupported,
}

/// The reason a measurement carries reduced confidence — always paired
/// with a non-`Valid` (or anomaly-flagged `Valid`) validity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementIssue {
    /// The counter's raw value decreased — wrap, reset or a new cell;
    /// the cases cannot be distinguished reliably so none is invented.
    CounterDecreased,
    /// A timestamp ordering anomaly made the elapsed measurement
    /// impossible (caller `now` earlier than the observation anchor,
    /// or a zero-length interval). Delta/rate were suppressed.
    ClockAnomaly,
    /// The final drain ran without a confirmed writer-quiescence
    /// declaration — the domain abandoned its lease or the collector
    /// forced the transition (shutdown).
    WriterNotQuiesced,
    /// A histogram's observed bucket layout failed to match its
    /// descriptor — defensive; registration validates boundaries, so
    /// reaching this indicates an internal inconsistency.
    HistogramInconsistent,
}

/// The typed value of one observation.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// A cumulative counter observation.
    Counter {
        /// Raw cell value at observation time (registration baseline
        /// included — the cell's absolute count).
        raw: u64,
        /// Sum of all deltas reported since registration — a
        /// monotonic accumulation over *this* registration's life,
        /// which deliberately is **not** reset by re-baselining after
        /// a discontinuity.
        cumulative_since_registration: u64,
        /// Increase since the previous observation — `None` when the
        /// counter decreased or the elapsed measurement failed.
        delta: Option<u64>,
        /// `delta / measured elapsed seconds` — `None` when no delta
        /// exists or the elapsed measurement failed. Display-grade
        /// precision (see module docs).
        rate_per_sec: Option<f64>,
    },
    /// A latest-value gauge observation.
    Gauge {
        /// Current raw cell value.
        value: u64,
        /// Previously observed value — `None` on the first
        /// observation of this registration.
        previous: Option<u64>,
    },
    /// A fixed-bucket histogram observation.
    Histogram {
        /// Per-bucket counts — `boundaries.len() + 1` entries in the
        /// descriptor's boundary order. Read per bucket (non-atomic
        /// across buckets).
        buckets: Box<[u64]>,
        /// Sum over the per-bucket loads — *approximate* since the
        /// buckets are not read atomically.
        total_approx: u64,
    },
}

/// One typed observation of a registered metric source.
///
/// Every collector output is a `Vec<Measurement>` (per-batch
/// observations) or the latest-per-series form (snapshots, history).
#[derive(Debug, Clone)]
pub struct Measurement {
    /// The registration-unique identity of the observed source.
    pub identity: SourceIdentity,
    /// The immutable descriptor declared at registration.
    pub descriptor: MetricDescriptor,
    /// The declared observation scope.
    pub scope: ObservationScope,
    /// The lifecycle state the observation was committed under —
    /// `Removed` for a final drain / tombstone.
    pub lifecycle: SourceLifecycle,
    /// When this observation was committed (the poll's `now`).
    pub observed_at: Instant,
    /// When the raw value last changed as far as the collector could
    /// observe — the newest poll whose raw value differed from the
    /// previous one (registration time for never-changed sources).
    pub last_change_at: Instant,
    /// Typed validity of this observation.
    pub validity: MeasurementValidity,
    /// The reason for reduced confidence, when any.
    pub issue: Option<MeasurementIssue>,
    /// Quantified loss when the collector can bound it (e.g. skipped
    /// polls); `None` when loss is unquantified — the `issue` field
    /// still carries the reason.
    pub loss: Option<u64>,
    /// The observed value — `None` for `Unavailable`/`Unsupported`,
    /// never a disguised zero.
    pub value: Option<MetricValue>,
}
