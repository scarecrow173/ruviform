//! The collector — the authoritative non-RT observation path for
//! [`Counter`]/[`Gauge`]/[`Histogram`] sources registered through the
//! OBS-02 [`ObservationRegistrar`] port.
//!
//! # Ownership structure
//!
//! ```text
//! Arc<Shared> ──── Mutex<State> + AtomicU64 id-mint + AtomicU8 state
//!   ├── Collector          poll(&mut self) — the single poll owner
//!   ├── CollectorRegistrar ObservationRegistrar port — Clone/Arc
//!   └── CollectorHandle    snapshot/latest_batch/history/health — Clone
//! ```
//!
//! **Single poll owner, guaranteed by ownership**: `poll` takes
//! `&mut self`, [`Collector`] is deliberately not `Clone`, and the only
//! `Shared → Collector` construction is [`Collector::new`]. No
//! handle type exposes a poll call — concurrent polling is impossible
//! by type, not by convention.
//!
//! # Lock discipline
//!
//! Every `poll` runs its enumerate → read → commit inside **one**
//! `Mutex<State>` acquisition. There is no subscriber/exporter/user
//! callback or file I/O anywhere in this design — the lock only ever
//! covers atomic reads, `Vec` operations and state commits, so holding
//! it for the whole pass both satisfies the Issue #289 rule and avoids
//! mid-poll source disappearance.
//!
//! `lease.retire()`/lease `Drop` are lock-free CAS on the shared
//! [`SourceState`] — a retire can land *inside* a poll. The commit
//! therefore re-reads `entry.state.lifecycle()` per source and drains
//! a mid-poll-retired source in the same poll.
//!
//! Mutex poisoning is recovered via `PoisonError::into_inner` and
//! counted (`poison_recoveries_total`): lock sections are short
//! coherent updates, and one panic must not brick observability.
//!
//! **Non-RT only** — every operation here allocates and/or locks. RT
//! writers never hold a registrar handle and never touch this module;
//! the writer path (`Counter::increment` etc.) is untouched.

use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ruviform_media_core::ClockDomainId;

use crate::metrics::{Measurement, MeasurementIssue, MeasurementValidity, MetricValue};
use crate::source::{
    LifecycleError, MetricKind, MetricReader, MetricSource, ObservationRegistrar,
    RegistrationError, RegistrationLease, SourceId, SourceIdentity, SourceLifecycle,
};

use super::config::CollectorConfig;
use super::health::{CollectorHealth, CollectorState, HealthCounters};
use super::history::History;
use super::registry::{Registry, SeriesState, SourceEntry};
use super::{MetricsSnapshot, ObservationBatch, ObservationWindow};

const STATE_RUNNING: u8 = 0;
const STATE_DRAINING: u8 = 1;
const STATE_STOPPED: u8 = 2;

/// Everything the three public handle types share.
#[derive(Debug)]
struct Shared {
    /// Registry table + history + health + latest results — the only
    /// lock-protected state.
    state: Mutex<State>,
    /// Monotonic `SourceId` mint — never reused while the collector
    /// lives.
    next_source_id: AtomicU64,
    /// `Running`/`Draining`/`Stopped` — outside the mutex so `Draining`
    /// is observable while a drain pass holds the lock.
    collector_state: AtomicU8,
    /// Validated at construction; immutable for the collector's life.
    config: CollectorConfig,
}

impl Shared {
    /// Locks the state, recovering from poisoning — the critical
    /// sections are short in-place updates, so the recovered state is
    /// coherent; every recovery is counted in health.
    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| {
            let mut guard = poisoned.into_inner();
            guard.health.poison_recoveries_total += 1;
            guard
        })
    }

    /// The collector state — lock-free.
    fn collector_state(&self) -> CollectorState {
        match self.collector_state.load(Ordering::Relaxed) {
            STATE_RUNNING => CollectorState::Running,
            STATE_DRAINING => CollectorState::Draining,
            _ => CollectorState::Stopped,
        }
    }

    /// `Running → Draining` CAS, then under the state lock drains
    /// every live source and stores `Stopped`. Whoever holds the lock
    /// while the state is `Draining` performs the drain; once
    /// `Stopped` is stored every caller returns — idempotent and
    /// concurrent-safe.
    fn shutdown(&self) {
        let _ = self.collector_state.compare_exchange(
            STATE_RUNNING,
            STATE_DRAINING,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        let now = Instant::now();
        let mut st = self.lock_state();
        self.drain_all(&mut st, now);
    }

    /// Drains every live source and stores `Stopped` — the caller must
    /// hold the state lock. Idempotent; returns immediately when the
    /// drain already happened.
    fn drain_all(&self, guard: &mut MutexGuard<'_, State>, now: Instant) {
        if self.collector_state() == CollectorState::Stopped {
            return;
        }
        let ids: Vec<SourceId> = guard.registry.entries.keys().copied().collect();
        for id in ids {
            if let Some(m) = guard.commit_drain(id, now, &self.config) {
                let outcome = guard.history.push(&m);
                guard.health.history_evicted_total += outcome.evicted as u64;
                if outcome.oversized {
                    guard.health.history_oversized_dropped_total += 1;
                }
            }
        }
        let generation = guard.generation;
        guard.latest_snapshot = rebuild_snapshot(&guard.registry, generation, now);
        self.collector_state.store(STATE_STOPPED, Ordering::Release);
    }
}

/// The lock-protected collector state.
#[derive(Debug)]
struct State {
    registry: Registry,
    history: History,
    health: HealthCounters,
    latest_snapshot: MetricsSnapshot,
    latest_batch: Option<ObservationBatch>,
    generation: u64,
    /// Round-robin position into the sorted `Active` id list — the
    /// batch-limit fairness cursor.
    active_cursor: usize,
    /// `ended` of the previous window — the next window's `started`
    /// (the collector's creation instant until the first poll).
    last_window_end: Instant,
}

impl State {
    /// Observes one `Active` source and commits the measurement.
    fn commit_sample(
        &mut self,
        id: SourceId,
        now: Instant,
        config: &CollectorConfig,
    ) -> Option<Measurement> {
        let entry = self.registry.entries.get_mut(&id)?;
        let mut observed = observe_entry(entry, now, &mut self.health);

        if matches!(observed.validity, MeasurementValidity::Valid)
            && let Some(threshold) = config.stale_after
            && now
                .checked_duration_since(entry.last_change_at)
                .is_some_and(|elapsed| elapsed > threshold)
        {
            observed.validity = MeasurementValidity::Stale;
        }
        entry.last_observed_at = now;

        let m = Measurement {
            identity: entry.identity,
            descriptor: entry.descriptor.clone(),
            scope: entry.scope,
            lifecycle: SourceLifecycle::Active,
            observed_at: now,
            last_change_at: entry.last_change_at,
            validity: observed.validity,
            issue: observed.issue,
            loss: None,
            value: Some(observed.value),
        };
        entry.last = Some(m.clone());
        self.health.samples_total += 1;
        Some(m)
    }

    /// Performs the final drain of a retiring source:
    /// `request_retire` (no-op unless still `Active`) →
    /// `begin_final_drain` → **final read** → `mark_removed` →
    /// tombstone. The last read happens *after* the drain
    /// declaration, matching the OBS-02 ordering contract.
    fn commit_drain(
        &mut self,
        id: SourceId,
        now: Instant,
        config: &CollectorConfig,
    ) -> Option<Measurement> {
        let state = {
            let entry = self.registry.entries.get(&id)?;
            Arc::clone(&entry.state)
        };
        // `true` means *this call* performed Active→Retiring — a
        // collector-forced drain with no writer-quiescence
        // declaration, which must surface as `Partial`.
        let forced = state.request_retire();
        let abandoned = state.is_abandoned();
        if abandoned {
            self.health.abandoned_sources_total += 1;
        }
        match state.begin_final_drain() {
            // Defensive re-entry — the drain pass already claimed
            // FinalDrain for this source.
            Ok(())
            | Err(LifecycleError::InvalidTransition {
                actual: SourceLifecycle::FinalDrain,
                ..
            }) => {}
            Err(_) => self.health.lifecycle_errors_total += 1,
        }

        let entry = self.registry.entries.get_mut(&id)?;
        let observed = observe_entry(entry, now, &mut self.health);

        // Validity precedence: Discontinuous > Partial > Valid — a
        // data-integrity signal beats a confidence signal.
        let (validity, issue) = match observed.validity {
            MeasurementValidity::Valid if forced || abandoned => (
                MeasurementValidity::Partial,
                Some(MeasurementIssue::WriterNotQuiesced),
            ),
            v => (v, observed.issue),
        };

        if state.mark_removed().is_err() {
            self.health.lifecycle_errors_total += 1;
        }
        let m = Measurement {
            identity: entry.identity,
            descriptor: entry.descriptor.clone(),
            scope: entry.scope,
            lifecycle: SourceLifecycle::Removed,
            observed_at: now,
            last_change_at: entry.last_change_at,
            validity,
            issue,
            loss: None,
            value: Some(observed.value),
        };
        self.health.samples_total += 1;
        self.health.sources_removed_total += 1;

        let entry = self.registry.remove_entry(id)?;
        let tombstone = Measurement {
            identity: entry.identity,
            descriptor: entry.descriptor.clone(),
            scope: entry.scope,
            lifecycle: SourceLifecycle::Removed,
            observed_at: now,
            last_change_at: entry.last_change_at,
            validity: MeasurementValidity::Unavailable,
            issue: None,
            loss: None,
            value: None,
        };
        let evicted = self.registry.retain_tombstone(id, tombstone, config);
        self.health.removed_series_evicted_total += evicted as u64;
        Some(m)
    }

    /// Enumerates registrations into `(drains, actives)` — lifecycle
    /// is re-read per source at commit, so a lock-free retire landing
    /// mid-poll still drains this poll.
    fn enumerate(&self) -> (Vec<SourceId>, Vec<SourceId>) {
        let mut drains = Vec::new();
        let mut actives = Vec::new();
        for (id, entry) in &self.registry.entries {
            match entry.state.lifecycle() {
                SourceLifecycle::Active => actives.push(*id),
                SourceLifecycle::Retiring | SourceLifecycle::FinalDrain => drains.push(*id),
                _ => {}
            }
        }
        (drains, actives)
    }

    /// Round-robin select over the sorted `Active` ids — every
    /// `Active` source is observed within
    /// `ceil(len / max_batch_size)` polls.
    fn select_batch(&mut self, actives: &[SourceId], max_batch: usize) -> Vec<SourceId> {
        let selected = actives.len().min(max_batch);
        let mut picked = Vec::with_capacity(selected);
        if !actives.is_empty() {
            let start = self.active_cursor % actives.len();
            for i in 0..selected {
                picked.push(actives[(start + i) % actives.len()]);
            }
            self.active_cursor = (start + selected) % actives.len();
        }
        picked
    }

    /// Re-reads `id`'s lifecycle and commits a sample or final drain,
    /// then offers the measurement to the bounded history. Returns
    /// the measurement and the number of history entries it evicted.
    fn observe_and_commit(
        &mut self,
        id: SourceId,
        now: Instant,
        config: &CollectorConfig,
    ) -> Option<(Measurement, usize)> {
        let lifecycle = self.registry.entries.get(&id).map(|e| e.state.lifecycle());
        let m = match lifecycle {
            Some(SourceLifecycle::Retiring | SourceLifecycle::FinalDrain) => {
                self.commit_drain(id, now, config)?
            }
            Some(SourceLifecycle::Active) => self.commit_sample(id, now, config)?,
            _ => return None,
        };
        let outcome = self.history.push(&m);
        self.health.history_evicted_total += outcome.evicted as u64;
        if outcome.oversized {
            self.health.history_oversized_dropped_total += 1;
        }
        Some((m, outcome.evicted))
    }
}

/// The raw observation of one source — before validity decoration
/// (staleness/quiescence) is applied.
struct Observed {
    value: MetricValue,
    validity: MeasurementValidity,
    issue: Option<MeasurementIssue>,
}

/// Reads the reader for its kind and folds it into the series'
/// aggregation state — shared by both sample and final-drain paths so
/// the "final value" follows exactly the same delta/rate rules.
fn observe_entry(entry: &mut SourceEntry, now: Instant, health: &mut HealthCounters) -> Observed {
    match entry.descriptor.kind {
        MetricKind::Counter => observe_counter(entry, now, health),
        MetricKind::Gauge => observe_gauge(entry, now),
        MetricKind::Histogram => observe_histogram(entry, now, health),
    }
}

/// Observes a counter: raw read, delta/rate derivation, decrease
/// detection and re-baselining.
fn observe_counter(entry: &mut SourceEntry, now: Instant, health: &mut HealthCounters) -> Observed {
    let SeriesState::Counter {
        prev_raw,
        cumulative,
    } = &mut entry.series
    else {
        unreachable!("counter kind always pairs counter series state")
    };
    let MetricReader::Counter(r) = &entry.reader else {
        unreachable!("counter series always pairs a counter reader")
    };
    let raw = r.read();
    let mut validity = MeasurementValidity::Valid;
    let mut issue = None;
    let mut delta = None;
    let mut rate = None;
    if raw < *prev_raw {
        // Counter decreased — reset/wrap suspicion. The cases are
        // indistinguishable, so none is invented: no delta, no rate,
        // re-baseline.
        validity = MeasurementValidity::Discontinuous;
        issue = Some(MeasurementIssue::CounterDecreased);
        health.discontinuities_total += 1;
    } else {
        match now.checked_duration_since(entry.last_observed_at) {
            Some(elapsed) if !elapsed.is_zero() => {
                let d = raw - *prev_raw;
                delta = Some(d);
                rate = Some(rate_per_sec(d, elapsed));
                *cumulative = cumulative.saturating_add(d);
            }
            _ => {
                // A zero or backwards elapsed measurement cannot
                // produce a rate — suppress both delta and rate
                // rather than fabricate.
                issue = Some(MeasurementIssue::ClockAnomaly);
                health.clock_anomalies_total += 1;
            }
        }
    }
    if raw != *prev_raw {
        entry.last_change_at = now;
    }
    *prev_raw = raw;
    Observed {
        value: MetricValue::Counter {
            raw,
            cumulative_since_registration: *cumulative,
            delta,
            rate_per_sec: rate,
        },
        validity,
        issue,
    }
}

/// Observes a gauge: latest value plus the previously committed one.
fn observe_gauge(entry: &mut SourceEntry, now: Instant) -> Observed {
    let SeriesState::Gauge {
        last_raw,
        observed_once,
    } = &mut entry.series
    else {
        unreachable!("gauge kind always pairs gauge series state")
    };
    let MetricReader::Gauge(r) = &entry.reader else {
        unreachable!("gauge series always pairs a gauge reader")
    };
    let raw = r.read();
    let previous = if *observed_once {
        Some(*last_raw)
    } else {
        None
    };
    *observed_once = true;
    if raw != *last_raw {
        entry.last_change_at = now;
    }
    *last_raw = raw;
    Observed {
        value: MetricValue::Gauge {
            value: raw,
            previous,
        },
        validity: MeasurementValidity::Valid,
        issue: None,
    }
}

/// Observes a histogram: per-bucket reads plus the defensive
/// descriptor-consistency check.
fn observe_histogram(
    entry: &mut SourceEntry,
    now: Instant,
    health: &mut HealthCounters,
) -> Observed {
    let SeriesState::Histogram { prev_buckets } = &mut entry.series else {
        unreachable!("histogram kind always pairs histogram series state")
    };
    let MetricReader::Histogram(r) = &entry.reader else {
        unreachable!("histogram series always pairs a histogram reader")
    };
    let buckets = r.snapshot();
    let mut validity = MeasurementValidity::Valid;
    let mut issue = None;
    // Defensive: registration validates the descriptor boundaries
    // against the reader's, so reaching here means an internal
    // inconsistency — report it typed.
    if entry
        .descriptor
        .buckets
        .is_some_and(|declared| r.boundaries() != declared)
        || buckets.len() != prev_buckets.len()
    {
        validity = MeasurementValidity::Partial;
        issue = Some(MeasurementIssue::HistogramInconsistent);
        health.histogram_inconsistencies_total += 1;
    }
    if buckets != *prev_buckets {
        entry.last_change_at = now;
    }
    prev_buckets.clone_from(&buckets);
    Observed {
        value: MetricValue::Histogram {
            total_approx: buckets.iter().sum(),
            buckets,
        },
        validity,
        issue,
    }
}

/// `delta / elapsed` — the measured-rate conversion.
#[expect(
    clippy::cast_precision_loss,
    reason = "rate_per_sec is a display/aggregation-grade figure; u64 deltas above 2^53 acceptably lose integer precision and must not round-trip into counts"
)]
fn rate_per_sec(delta: u64, elapsed: Duration) -> f64 {
    delta as f64 / elapsed.as_secs_f64()
}

/// The post-commit snapshot — latest measurement per live series plus
/// all retained-removed tombstones, sorted by `SourceId`.
fn rebuild_snapshot(registry: &Registry, generation: u64, now: Instant) -> MetricsSnapshot {
    let mut measurements = Vec::with_capacity(registry.entries.len() + registry.retained.len());
    for entry in registry.entries.values() {
        if let Some(m) = &entry.last {
            measurements.push(m.clone());
        }
    }
    measurements.extend(registry.retained.values().cloned());
    MetricsSnapshot::new(generation, now, measurements)
}

/// Error returned by [`Collector::poll`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollError {
    /// The collector is `Draining` or `Stopped` — no further polls
    /// are accepted.
    CollectorStopped,
}

impl fmt::Display for PollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CollectorStopped => f.write_str("collector is stopped"),
        }
    }
}

impl Error for PollError {}

/// The collector — single poll owner over the shared registry.
///
/// **Deliberately not `Clone`/`Copy`** — the type-level single-poll
/// guarantee rests on there being exactly one `Collector` per
/// `Shared`; do not add a `Shared → Collector` constructor besides
/// [`Collector::new`]. Dropping the collector performs
/// [`Collector::shutdown`] (idempotent) so registrar clones left
/// behind see `RegistrarStopped` and pending sources are drained.
///
/// **Non-RT only** — every method allocates and/or locks.
#[derive(Debug)]
pub struct Collector {
    shared: Arc<Shared>,
}

impl Collector {
    /// Creates a collector — validates `config` first.
    ///
    /// # Errors
    ///
    /// Returns the typed [`crate::collector::CollectorConfigError`]
    /// for zero/invalid/inconsistent limits.
    pub fn new(config: CollectorConfig) -> Result<Self, super::CollectorConfigError> {
        config.validate()?;
        let created_at = Instant::now();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                registry: Registry::default(),
                history: History::new(config.history_max_samples, config.history_budget_bytes),
                health: HealthCounters::default(),
                latest_snapshot: MetricsSnapshot::new(0, created_at, Vec::new()),
                latest_batch: None,
                generation: 0,
                active_cursor: 0,
                last_window_end: created_at,
            }),
            next_source_id: AtomicU64::new(0),
            collector_state: AtomicU8::new(STATE_RUNNING),
            config,
        });
        Ok(Self { shared })
    }

    /// The registration port — cloneable, `Arc`-able, handable to
    /// domains as `Arc<dyn ObservationRegistrar>`. Registrations are
    /// accepted while the collector is `Running`.
    #[must_use]
    pub fn registration_port(&self) -> CollectorRegistrar {
        CollectorRegistrar {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The read-only shareable view — snapshots, batches, history,
    /// series, health, shutdown. No poll access by construction.
    #[must_use]
    pub fn handle(&self) -> CollectorHandle {
        CollectorHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The current collector state — lock-free.
    #[must_use]
    pub fn state(&self) -> CollectorState {
        self.shared.collector_state()
    }

    /// The latest committed snapshot — generation `0` and empty
    /// before the first poll; the last committed generation after
    /// `Stopped`.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.shared.lock_state().latest_snapshot.clone()
    }

    /// The latest committed batch — `None` until the first poll.
    /// Readable after `Stopped`.
    #[must_use]
    pub fn latest_batch(&self) -> Option<ObservationBatch> {
        self.shared.lock_state().latest_batch.clone()
    }

    /// The current health counters — logging-independent.
    #[must_use]
    pub fn health(&self) -> CollectorHealth {
        let st = self.shared.lock_state();
        health_snapshot(&self.shared, &st)
    }

    /// Chronological clone of the whole retained history.
    #[must_use]
    pub fn history(&self) -> Vec<Measurement> {
        self.shared.lock_state().history.all()
    }

    /// Chronological clone of one registration's retained history.
    #[must_use]
    pub fn series(&self, identity: SourceIdentity) -> Vec<Measurement> {
        self.shared.lock_state().history.series(identity)
    }

    /// Runs one poll — enumerate, read and commit inside a single
    /// registry-lock acquisition.
    ///
    /// `now` must be a fresh [`Instant::now`] on the caller's side;
    /// passing a stale or reused instant lands in `ClockAnomaly`
    /// measurements rather than a panic.
    ///
    /// **Non-RT only** — allocates and locks throughout.
    ///
    /// # Errors
    ///
    /// [`PollError::CollectorStopped`] when the collector is
    /// `Draining` or `Stopped`.
    pub fn poll(&mut self, now: Instant) -> Result<ObservationBatch, PollError> {
        if self.shared.collector_state() != CollectorState::Running {
            return Err(PollError::CollectorStopped);
        }
        let poll_started = Instant::now();
        let mut st = self.shared.lock_state();
        // Re-check under the lock — a shutdown CAS could have landed
        // between the first check and the lock acquisition.
        if self.shared.collector_state() != CollectorState::Running {
            return Err(PollError::CollectorStopped);
        }

        let generation = st.generation + 1;
        let window_started = st.last_window_end;
        let mut window_anomaly = false;
        let measured = if let Some(d) = now.checked_duration_since(window_started) {
            d
        } else {
            window_anomaly = true;
            Duration::ZERO
        };

        let (drains, actives) = st.enumerate();
        let planned = actives.len() + drains.len();
        let to_sample = st.select_batch(&actives, self.shared.config.max_batch_size);
        let skipped = actives.len() - to_sample.len();

        let mut measurements = Vec::new();
        let mut removed = Vec::new();
        let mut ring_evicted = 0usize;
        for id in drains.iter().chain(to_sample.iter()) {
            if let Some((m, evicted)) = st.observe_and_commit(*id, now, &self.shared.config) {
                ring_evicted += evicted;
                if matches!(m.issue, Some(MeasurementIssue::ClockAnomaly)) {
                    window_anomaly = true;
                }
                if m.lifecycle == SourceLifecycle::Removed {
                    removed.push(m.identity);
                }
                measurements.push(m);
            }
        }

        let window = ObservationWindow {
            generation,
            started: window_started,
            ended: now,
            expected: self.shared.config.poll_interval,
            measured,
            clock: ClockDomainId::MONOTONIC,
            planned,
            observed: measurements.len(),
            skipped,
            drained: removed.len(),
            history_evicted: ring_evicted,
            clock_anomaly: window_anomaly,
        };

        let snapshot = rebuild_snapshot(&st.registry, generation, now);
        let batch =
            ObservationBatch::new(generation, window, measurements, removed, snapshot.clone());

        st.health.polls_total += 1;
        if measured > self.shared.config.poll_interval {
            st.health.overdue_polls_total += 1;
        }
        st.health.last_poll_at = Some(now);
        st.health.last_poll_duration = Some(poll_started.elapsed());
        st.health.samples_skipped_total += skipped as u64;
        st.generation = generation;
        st.last_window_end = now;
        st.latest_snapshot = snapshot;
        st.latest_batch = Some(batch.clone());
        Ok(batch)
    }

    /// Drains every live source and stops the collector.
    ///
    /// Idempotent — the first caller performs `Running → Draining →
    /// (single lock pass) → Stopped`; concurrent callers complete the
    /// drain themselves or return once `Stopped` is stored. While
    /// `Draining`, registrations fail `FinalDrainInProgress` and polls
    /// fail `CollectorStopped`.
    ///
    /// Sources still `Active` at shutdown are drained by the
    /// collector itself — their final values commit as `Partial` +
    /// `WriterNotQuiesced` since writer quiescence was never
    /// confirmed.
    ///
    /// **Non-RT only** — locks and allocates.
    pub fn shutdown(&self) {
        self.shared.shutdown();
    }

    /// Moves the collector onto a dedicated thread that polls every
    /// `poll_interval` — the generic non-RT periodic helper (no
    /// tracing/logging is emitted).
    ///
    /// The thread exits when [`PollRunner::stop`] is called or the
    /// collector stops; the in-thread `Collector` drop then runs the
    /// final drain itself.
    ///
    /// **Non-RT only** — spawns a thread.
    #[must_use]
    pub fn into_poll_runner(self) -> PollRunner {
        let handle = self.handle();
        let registrar = self.registration_port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let interval = self.shared.config.poll_interval;
        let join = thread::spawn(move || {
            let mut collector = self;
            while !flag.load(Ordering::Relaxed) {
                let t0 = Instant::now();
                if collector.poll(t0).is_err() {
                    break;
                }
                // Sleep the remainder in small slices so `stop` is
                // observed promptly.
                let mut remaining = interval.saturating_sub(t0.elapsed());
                while !remaining.is_zero() && !flag.load(Ordering::Relaxed) {
                    let slice = remaining.min(Duration::from_millis(1));
                    thread::sleep(slice);
                    remaining = remaining.saturating_sub(slice);
                }
            }
            // `collector` drops here — Drop runs shutdown(), which
            // drains every remaining source before Stopped.
        });
        PollRunner {
            stop,
            join: Some(join),
            handle,
            registrar,
        }
    }
}

impl Drop for Collector {
    /// Dropping the poll owner ends collection — equivalent to
    /// [`Collector::shutdown`] (idempotent). Registrar clones left
    /// behind observe `RegistrarStopped` and live sources are drained
    /// as `Partial` (quiescence unconfirmed).
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
impl Collector {
    /// Panics while holding the state lock — poisons the mutex so
    /// tests can exercise the `PoisonError::into_inner` recovery path
    /// (impossible to reach through the normal API: no code inside a
    /// lock section can panic).
    pub(crate) fn poison_state_for_test(&self) {
        let _guard = self.shared.state.lock().unwrap();
        panic!("deliberate poison for recovery test");
    }
}

/// The shareable registration port — implements
/// [`ObservationRegistrar`] over the shared state. `Clone`/`Arc` it
/// freely and hand it to domains as `Arc<dyn ObservationRegistrar>`.
///
/// **Non-RT only** — `register` locks and allocates.
#[derive(Debug, Clone)]
pub struct CollectorRegistrar {
    shared: Arc<Shared>,
}

impl ObservationRegistrar for CollectorRegistrar {
    fn register(&self, source: MetricSource) -> Result<RegistrationLease, RegistrationError> {
        // Fast lock-free rejection — widens the Draining window
        // without waiting on the mutex.
        match self.shared.collector_state() {
            CollectorState::Draining => return Err(RegistrationError::FinalDrainInProgress),
            CollectorState::Stopped => return Err(RegistrationError::RegistrarStopped),
            _ => {}
        }
        let mut st = self.shared.lock_state();
        // Re-check under the lock — the correctness check; a shutdown
        // CAS could have landed while we waited.
        match self.shared.collector_state() {
            CollectorState::Draining => return Err(RegistrationError::FinalDrainInProgress),
            CollectorState::Stopped => return Err(RegistrationError::RegistrarStopped),
            _ => {}
        }
        match st
            .registry
            .register(&source, &self.shared.config, &self.shared.next_source_id)
        {
            Ok((lease, tombstones_evicted)) => {
                st.health.registered_total += 1;
                st.health.removed_series_evicted_total += tombstones_evicted as u64;
                Ok(lease)
            }
            Err(e) => {
                st.health.registration_rejected_total += 1;
                Err(e)
            }
        }
    }
}

/// The shareable read side — snapshots, batches, history, series,
/// health, shutdown. Exposes **no** poll call: concurrent polling is
/// impossible by construction.
///
/// `Clone + Send + Sync` — hand it to GUI/diagnostics/diagnostics
/// consumers freely.
///
/// **Non-RT only** — every method locks and allocates.
#[derive(Debug, Clone)]
pub struct CollectorHandle {
    shared: Arc<Shared>,
}

impl CollectorHandle {
    /// The registration port — kept reachable after
    /// `into_poll_runner` consumed the `Collector`.
    #[must_use]
    pub fn registration_port(&self) -> CollectorRegistrar {
        CollectorRegistrar {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The current collector state — lock-free.
    #[must_use]
    pub fn state(&self) -> CollectorState {
        self.shared.collector_state()
    }

    /// The latest committed snapshot (see [`Collector::snapshot`]).
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.shared.lock_state().latest_snapshot.clone()
    }

    /// The latest committed batch — `None` until the first poll;
    /// readable after `Stopped`.
    #[must_use]
    pub fn latest_batch(&self) -> Option<ObservationBatch> {
        self.shared.lock_state().latest_batch.clone()
    }

    /// The current health counters — logging-independent.
    #[must_use]
    pub fn health(&self) -> CollectorHealth {
        let st = self.shared.lock_state();
        health_snapshot(&self.shared, &st)
    }

    /// Chronological clone of the whole retained history.
    #[must_use]
    pub fn history(&self) -> Vec<Measurement> {
        self.shared.lock_state().history.all()
    }

    /// Chronological clone of one registration's retained history.
    #[must_use]
    pub fn series(&self, identity: SourceIdentity) -> Vec<Measurement> {
        self.shared.lock_state().history.series(identity)
    }

    /// Drains every live source and stops the collector — same
    /// contract as [`Collector::shutdown`].
    ///
    /// **Non-RT only** — locks and allocates.
    pub fn shutdown(&self) {
        self.shared.shutdown();
    }
}

/// Builds the public [`CollectorHealth`] from the locked counters.
fn health_snapshot(shared: &Shared, st: &State) -> CollectorHealth {
    let (active, retiring) = st.registry.live_counts();
    CollectorHealth {
        state: shared.collector_state(),
        polls_total: st.health.polls_total,
        overdue_polls_total: st.health.overdue_polls_total,
        last_poll_at: st.health.last_poll_at,
        last_poll_duration: st.health.last_poll_duration,
        registered_total: st.health.registered_total,
        registration_rejected_total: st.health.registration_rejected_total,
        sources_active: active,
        sources_retiring: retiring,
        sources_removed_total: st.health.sources_removed_total,
        abandoned_sources_total: st.health.abandoned_sources_total,
        samples_total: st.health.samples_total,
        samples_skipped_total: st.health.samples_skipped_total,
        discontinuities_total: st.health.discontinuities_total,
        clock_anomalies_total: st.health.clock_anomalies_total,
        histogram_inconsistencies_total: st.health.histogram_inconsistencies_total,
        lifecycle_errors_total: st.health.lifecycle_errors_total,
        history_len: st.history.len(),
        history_bytes: st.history.bytes(),
        history_evicted_total: st.health.history_evicted_total,
        history_oversized_dropped_total: st.health.history_oversized_dropped_total,
        removed_series_evicted_total: st.health.removed_series_evicted_total,
        poison_recoveries_total: st.health.poison_recoveries_total,
    }
}

/// The periodic-poll helper — owns the collector on a dedicated
/// thread. Dropping or [`PollRunner::stop`]ping ends the loop and lets
/// the in-thread `Collector` drop run the final drain.
///
/// **Non-RT only** — owns a thread; `stop` joins it.
#[derive(Debug)]
pub struct PollRunner {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    handle: CollectorHandle,
    registrar: CollectorRegistrar,
}

impl PollRunner {
    /// The shareable read side.
    #[must_use]
    pub fn handle(&self) -> CollectorHandle {
        self.handle.clone()
    }

    /// The registration port — registrations stay possible while the
    /// runner is `Running`.
    #[must_use]
    pub fn registration_port(&self) -> CollectorRegistrar {
        self.registrar.clone()
    }

    /// Signals the loop to exit and joins the thread — the in-thread
    /// collector drop drains all remaining sources before it returns.
    ///
    /// # Errors
    ///
    /// Propagates the poll thread's panic via `JoinHandle::join`.
    pub fn stop(mut self) -> thread::Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        match self.join.take() {
            Some(join) => join.join(),
            None => Ok(()),
        }
    }
}

impl Drop for PollRunner {
    /// Same as [`PollRunner::stop`] — dropping the runner without
    /// stopping still ends the loop and drains.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
