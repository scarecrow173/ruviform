//! Concrete [`ProbeSource`](ruviform_observability::debug::ProbeSource)
//! adapter for the typed-runtime probe pair (Issue #293, OBS-11).
//!
//! `ruviform-observability` cannot depend on `ruviform-audio`
//! (the audio crate already depends on observability — the reverse
//! would be a cycle), so the port's concrete implementation lives
//! here, on the audio side, exactly as
//! `ruviform_observability::debug::probe` prescribes.
//!
//! [`ProbeSourceAdapter`] wraps the non-RT halves of
//! [`pair`](super::pair) — the [`ProbeBroker`] control plane and the
//! [`ProbeCollector`] drain side — behind one `Arc<Mutex<..>>`
//! shared state, mirroring the collector handle pattern used by
//! OBS-07. The RT-side [`ProbeDeck`](super::ProbeDeck) is untouched:
//! install it on [`RtRuntime::set_probe_deck`](crate::typed::RtRuntime::set_probe_deck)
//! exactly as before; this adapter adds no RT-path code and never
//! runs on the callback thread.
//!
//! # Reading semantics
//!
//! - `decks()` drains the **telemetry channel only** (metrics +
//!   statuses) into per-probe accumulators, then reports one
//!   [`ProbeDeckSnapshot`]. Capture chunks are *never* consumed by
//!   inspection — they stay queued for
//!   [`drain_events`](ProbeSourceAdapter::drain_events), which is the
//!   host's record-stream API (the same events
//!   [`ProbeCollector::drain`](super::ProbeCollector::drain) would
//!   deliver, with identical telemetry-then-chunks ordering).
//!   Whichever side drains first consumes that window's records;
//!   `decks()` therefore reflects everything drained so far plus
//!   whatever telemetry is pending at call time.
//! - `recorded` counts drained telemetry records (metrics and
//!   statuses) per probe; capture chunks are not telemetry and are
//!   not counted.
//! - `dropped`/`lost_blocks` report the **deck-global** cumulative
//!   counters ([`ProbeCounts`]) on every reading: channel-full and
//!   pool-exhausted drops are accounted deck-wide by the RT side and
//!   cannot be attributed to a single probe. The same totals are
//!   available with `unobserved`/`unbound` via
//!   [`counts`](ProbeSourceAdapter::counts). Do not sum the per-probe
//!   fields — they are identical mirrors, not partitions.
//! - `generation` is the generation the broker last staged a binding
//!   for (`install`/`on_publish`); a probe whose point stopped
//!   resolving keeps its previous generation so the inspector's
//!   generation-mismatch check surfaces the skew instead of hiding it.
//! - `mode` is the last **RT-confirmed** mode, reconciled from the
//!   status stream (`Installed` resolves to the broker's commanded
//!   mode — the binding's staged mode is not carried by the status
//!   itself). A commanded-but-unconfirmed transition is not reported
//!   as applied.
//! - `clock` is the mark most recently observed on the drained stream
//!   (telemetry records, and capture-chunk metas seen by
//!   `drain_events`). [`ProbeClockMark::UNKNOWN`](obs) is reported
//!   until the deck emits a record after wiring — an unwired or silent
//!   deck cannot masquerade as aligned.
//! - `confidentiality` is declared [`Confidentiality::Internal`]:
//!   readings carry point identity, modes, generations, clocks and
//!   counters — never PCM, paths, or credentials.
//!
//! All methods are non-RT: they allocate and lock. Never call from an
//! RT writer path. The [`ProbeSource`] impl is `Send` and safe to
//! register via `DebugInspector::add_probe_source` or the runner's
//! `debug_mut()`; the inspector isolates panics via `catch_unwind`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use ruviform_graph::GraphSpec;
use ruviform_observability::debug::{
    ProbeAvailability as ObsAvailability, ProbeClockMark as ObsClockMark, ProbeDeckSnapshot,
    ProbeMode as ObsMode, ProbeReading, ProbeSource, ProbeStatusKind as ObsStatusKind,
    ProbeStopReason as ObsStopReason, ProbeUnobservedReason as ObsUnobservedReason,
};
use ruviform_observability::source::Confidentiality;

use super::{
    CaptureConfig, CapturePolicy, PointAvailability, ProbeBroker, ProbeClockMark, ProbeCollector,
    ProbeCounts, ProbeError, ProbeEvent, ProbeId, ProbeMode, ProbePoint, ProbeTimeContext,
    RebindReceipt, StatusKind, StopReason, TelemetryRecord, UnobservedReason,
};
use crate::typed::GraphRuntime;

/// Shared non-RT probe state — the broker control plane, the
/// collector drain side, and the clock mark most recently observed
/// on the drained stream.
struct Shared {
    state: Mutex<SharedState>,
    /// Mutex poison recoveries — counted, never silent (the
    /// collector's own discipline).
    lock_recoveries: AtomicU64,
}

struct SharedState {
    broker: ProbeBroker,
    collector: ProbeCollector,
    /// Clock mark stamped on the most recently drained record —
    /// `None` until the deck emits anything after wiring.
    observed_mark: Option<ProbeClockMark>,
}

impl SharedState {
    /// Folds one drained telemetry record into the per-probe
    /// accumulators and the series clock observation.
    fn observe_telemetry(&mut self, rec: TelemetryRecord) {
        let (probe, clock, status) = match rec {
            TelemetryRecord::Metric(m) => (m.meta.probe, m.meta.clock, None),
            TelemetryRecord::Status(s) => (s.probe, s.clock, Some(s.kind)),
        };
        self.observed_mark = Some(clock);
        let Some(entry) = self.broker.probes.get_mut(&probe) else {
            return;
        };
        entry.recorded = entry.recorded.saturating_add(1);
        if let Some(kind) = status {
            entry.last_status = Some(kind);
            entry.applied_mode = applied_mode_for(kind, entry.mode, entry.applied_mode);
        }
    }
}

/// The mode a status event proves the RT side applied — `Installed`
/// resolves to the broker's commanded mode because the status does
/// not carry the binding's mode payload.
fn applied_mode_for(kind: StatusKind, commanded: ProbeMode, current: ProbeMode) -> ProbeMode {
    match kind {
        StatusKind::ModeChanged { to } => to,
        StatusKind::CaptureStarted { .. } => ProbeMode::Capture,
        StatusKind::CaptureStopped { .. } => ProbeMode::Monitor,
        StatusKind::Installed => commanded,
        _ => current,
    }
}

fn map_mode(mode: ProbeMode) -> ObsMode {
    match mode {
        ProbeMode::Off => ObsMode::Off,
        ProbeMode::Monitor => ObsMode::Monitor,
        ProbeMode::Capture => ObsMode::Capture,
    }
}

fn map_availability(availability: PointAvailability) -> ObsAvailability {
    match availability {
        PointAvailability::Configured => ObsAvailability::Configured,
        PointAvailability::Inferred => ObsAvailability::Inferred,
        PointAvailability::Unavailable => ObsAvailability::Unavailable,
        PointAvailability::Unsupported => ObsAvailability::Unsupported,
        PointAvailability::Unknown => ObsAvailability::Unknown,
    }
}

fn map_stop_reason(reason: StopReason) -> ObsStopReason {
    match reason {
        StopReason::Requested => ObsStopReason::Requested,
        StopReason::BudgetExhausted => ObsStopReason::BudgetExhausted,
        StopReason::GenerationBoundary => ObsStopReason::GenerationBoundary,
    }
}

fn map_unobserved_reason(reason: UnobservedReason) -> ObsUnobservedReason {
    match reason {
        UnobservedReason::Stale => ObsUnobservedReason::Stale,
        UnobservedReason::BorrowConflict => ObsUnobservedReason::BorrowConflict,
    }
}

fn map_status(kind: StatusKind) -> ObsStatusKind {
    match kind {
        StatusKind::Installed => ObsStatusKind::Installed,
        StatusKind::ModeChanged { to } => ObsStatusKind::ModeChanged { to: map_mode(to) },
        StatusKind::CaptureStarted { budget_frames } => {
            ObsStatusKind::CaptureStarted { budget_frames }
        }
        StatusKind::CaptureStopped { reason } => ObsStatusKind::CaptureStopped {
            reason: map_stop_reason(reason),
        },
        StatusKind::Unbound => ObsStatusKind::Unbound,
        StatusKind::Unobserved { reason } => ObsStatusKind::Unobserved {
            reason: map_unobserved_reason(reason),
        },
        StatusKind::LostBlocks => ObsStatusKind::LostBlocks,
        StatusKind::LostRecords => ObsStatusKind::LostRecords,
        StatusKind::Removed => ObsStatusKind::Removed,
        StatusKind::RebindDropped => ObsStatusKind::RebindDropped,
    }
}

fn map_mark(mark: ProbeClockMark) -> ObsClockMark {
    ObsClockMark {
        domain: mark.domain,
        epoch: mark.epoch,
    }
}

/// The concrete [`ProbeSource`] adapter plus the non-RT probe control
/// surface — one cloneable handle owning the broker/collector halves
/// of [`pair`](super::pair).
///
/// Construction takes the `broker` and `collector` halves directly;
/// the `deck` half goes to [`RtRuntime::set_probe_deck`](crate::typed::RtRuntime::set_probe_deck)
/// unchanged. Wire it for inspection by cloning the adapter into
/// `DebugInspector::add_probe_source` (or the runner's
/// `debug_mut().add_probe_source`) and keeping the original for
/// control/drain calls:
///
/// ```no_run
/// # use ruviform_audio::typed::probe::{
/// #     pair, ProbeConfig, ProbeMode, ProbePoint, ProbeSourceAdapter,
/// # };
/// # use ruviform_audio::typed::GraphRuntime;
/// # use ruviform_graph::GraphSpec;
/// # use ruviform_observability::debug::DebugInspector;
/// # fn example(
/// #     runtime: &GraphRuntime,
/// #     spec: &GraphSpec,
/// #     point: ProbePoint,
/// #     inspector: &mut DebugInspector,
/// # ) {
/// let parts = pair(&ProbeConfig::default());
/// // `parts.deck` → `rt.set_probe_deck(parts.deck)` as usual.
/// let probe = ProbeSourceAdapter::new(parts.broker, parts.collector);
/// inspector.add_probe_source(Box::new(probe.clone()));
/// let _id = probe.install(runtime, spec, point, ProbeMode::Monitor).unwrap();
/// # }
/// ```
///
/// `Clone` shares the same underlying state — registering two clones
/// is safe (`decks()` is idempotent over accumulated state) but
/// pointless; register once.
#[derive(Clone)]
pub struct ProbeSourceAdapter {
    shared: Arc<Shared>,
}

impl ProbeSourceAdapter {
    /// Wraps the non-RT halves of [`pair`](super::pair). All probe
    /// control and drain traffic then flows through this adapter so
    /// inspection sees a consistent, accumulated view.
    #[must_use]
    pub fn new(broker: ProbeBroker, collector: ProbeCollector) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(SharedState {
                    broker,
                    collector,
                    observed_mark: None,
                }),
                lock_recoveries: AtomicU64::new(0),
            }),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, SharedState> {
        self.shared.state.lock().unwrap_or_else(|poisoned| {
            self.shared.lock_recoveries.fetch_add(1, Ordering::Relaxed);
            poisoned.into_inner()
        })
    }

    /// Cumulative count of mutex poison recoveries — nonzero means a
    /// panic was isolated mid-update and the shared state was
    /// recovered rather than propagated.
    #[must_use]
    pub fn lock_recoveries_total(&self) -> u64 {
        self.shared.lock_recoveries.load(Ordering::Relaxed)
    }

    /// Installs `point` in `mode` — same contract as
    /// [`ProbeBroker::install`](super::ProbeBroker::install).
    ///
    /// # Errors
    /// [`ProbeError::PointUnavailable`], [`ProbeError::CapacityExceeded`],
    /// [`ProbeError::CommandChannelFull`].
    pub fn install(
        &self,
        runtime: &GraphRuntime,
        spec: &GraphSpec,
        point: ProbePoint,
        mode: ProbeMode,
    ) -> Result<ProbeId, ProbeError> {
        self.lock_state().broker.install(runtime, spec, point, mode)
    }

    /// Switches `probe`'s commanded mode — see
    /// [`ProbeBroker::set_mode`](super::ProbeBroker::set_mode). The
    /// reading's `mode` reflects the RT-confirmed value once the
    /// `ModeChanged` status drains.
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn set_mode(&self, probe: ProbeId, mode: ProbeMode) -> Result<(), ProbeError> {
        self.lock_state().broker.set_mode(probe, mode)
    }

    /// Starts a bounded capture — see
    /// [`ProbeBroker::capture`](super::ProbeBroker::capture).
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::BudgetExceeded`],
    /// [`ProbeError::CommandChannelFull`].
    pub fn capture(&self, probe: ProbeId, config: CaptureConfig) -> Result<(), ProbeError> {
        self.lock_state().broker.capture(probe, config)
    }

    /// Stops an in-flight capture — see
    /// [`ProbeBroker::stop_capture`](super::ProbeBroker::stop_capture).
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn stop_capture(&self, probe: ProbeId) -> Result<(), ProbeError> {
        self.lock_state().broker.stop_capture(probe)
    }

    /// Removes `probe` — see [`ProbeBroker::remove`](super::ProbeBroker::remove).
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn remove(&self, probe: ProbeId) -> Result<(), ProbeError> {
        self.lock_state().broker.remove(probe)
    }

    /// Sets the capture continuation policy — see
    /// [`ProbeBroker::set_capture_policy`](super::ProbeBroker::set_capture_policy).
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`].
    pub fn set_capture_policy(
        &self,
        probe: ProbeId,
        policy: CapturePolicy,
    ) -> Result<(), ProbeError> {
        self.lock_state().broker.set_capture_policy(probe, policy)
    }

    /// Rebinds every installed probe against `runtime`'s newly
    /// published plan — see
    /// [`ProbeBroker::on_publish`](super::ProbeBroker::on_publish).
    /// Reading `generation`/`availability` update only after the
    /// whole rebind sequence is enqueued.
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`].
    pub fn on_publish(
        &self,
        runtime: &GraphRuntime,
        spec: &GraphSpec,
    ) -> Result<RebindReceipt, ProbeError> {
        self.lock_state().broker.on_publish(runtime, spec)
    }

    /// Point of an installed probe.
    #[must_use]
    pub fn point(&self, probe: ProbeId) -> Option<ProbePoint> {
        self.lock_state().broker.point(probe)
    }

    /// Installed probe count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock_state().broker.len()
    }

    /// Whether no probes are installed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock_state().broker.is_empty()
    }

    /// Read access to the series time context (Issue #309) —
    /// domain/epoch mark, runtime instance, anchors, generation
    /// window. The closure runs under the shared lock; keep it short.
    #[must_use]
    pub fn with_time_context<R>(&self, f: impl FnOnce(&ProbeTimeContext) -> R) -> R {
        let state = self.lock_state();
        f(&state.broker.time)
    }

    /// Mutable access for consumer wiring — declare `sample_rate`,
    /// install `anchors`, adjust `provenance`. The closure runs under
    /// the shared lock; keep it short and non-blocking.
    pub fn with_time_context_mut<R>(&self, f: impl FnOnce(&mut ProbeTimeContext) -> R) -> R {
        let mut state = self.lock_state();
        f(&mut state.broker.time)
    }

    /// Delivers the series' current clock mark to the deck — see
    /// [`ProbeBroker::wire_clock_mark`](super::ProbeBroker::wire_clock_mark).
    /// Until this runs, records carry `UNKNOWN` and `decks()` reports
    /// an unknown clock.
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`].
    pub fn wire_clock_mark(&self) -> Result<(), ProbeError> {
        self.lock_state().broker.wire_clock_mark()
    }

    /// Declares a cursor-series discontinuity — see
    /// [`ProbeBroker::bump_epoch`](super::ProbeBroker::bump_epoch).
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`].
    pub fn bump_epoch(&self) -> Result<ruviform_media_core::ClockEpochId, ProbeError> {
        self.lock_state().broker.bump_epoch()
    }

    /// Drains both channels like
    /// [`ProbeCollector::drain`](super::ProbeCollector::drain) —
    /// all queued telemetry first, then queued chunks — while folding
    /// every record into the inspection accumulators. The callback
    /// runs **outside** the shared lock, so it may call back into
    /// this adapter without deadlocking; it must still be non-RT.
    ///
    /// `Captured` borrows end inside the callback; chunks then
    /// recycle to the free pool automatically. Returns the number of
    /// events delivered.
    pub fn drain_events(&self, mut f: impl FnMut(ProbeEvent<'_>)) -> usize {
        let telemetry = {
            let mut state = self.lock_state();
            let mut telemetry = Vec::new();
            while let Ok(rec) = state.collector.telemetry.try_pop() {
                state.observe_telemetry(rec);
                telemetry.push(rec);
            }
            telemetry
        };
        let mut delivered = 0usize;
        for rec in telemetry {
            delivered += 1;
            match rec {
                TelemetryRecord::Metric(m) => f(ProbeEvent::Metric(m)),
                TelemetryRecord::Status(s) => f(ProbeEvent::Status(s)),
            }
        }
        // Chunks are checked out one at a time and recycled to the
        // free pool immediately after each callback — the same
        // progressive refill `ProbeCollector::drain` performs, so a
        // long drain never starves the RT capture pool.
        loop {
            let chunk = {
                let mut state = self.lock_state();
                match state.collector.chunks_out.try_pop() {
                    Ok(chunk) => {
                        if let Some(meta) = chunk.meta {
                            state.observed_mark = Some(meta.clock);
                        }
                        chunk
                    }
                    Err(_) => break,
                }
            };
            delivered += 1;
            f(ProbeEvent::Captured(&chunk));
            let mut state = self.lock_state();
            let _ = state.collector.free.try_push(chunk);
        }
        delivered
    }

    /// Deck-global cumulative counters — see
    /// [`ProbeCollector::counts`](super::ProbeCollector::counts).
    /// The per-reading `dropped`/`lost_blocks` mirror `lost_records`/
    /// `lost_blocks` from here.
    #[must_use]
    pub fn counts(&self) -> ProbeCounts {
        self.lock_state().collector.counts()
    }
}

impl ProbeSource for ProbeSourceAdapter {
    /// Drains pending telemetry into the accumulators and returns one
    /// deck snapshot. Never touches `chunks_out` — capture PCM stays
    /// queued for [`drain_events`](Self::drain_events).
    fn decks(&mut self) -> Vec<ProbeDeckSnapshot> {
        let state = &mut *self.lock_state();
        while let Ok(rec) = state.collector.telemetry.try_pop() {
            state.observe_telemetry(rec);
        }
        let generation = state.broker.time.generation_window.map_or(0, |w| w.last);
        let clock = state.observed_mark.map_or(ObsClockMark::UNKNOWN, map_mark);
        let counts = state.collector.counts();
        let probes = state
            .broker
            .probes
            .iter()
            .map(|(id, entry)| ProbeReading {
                probe: id.get(),
                node: entry.point.node.get(),
                port: entry.point.port.get(),
                mode: map_mode(entry.applied_mode),
                availability: map_availability(entry.availability),
                generation: entry.bound_generation,
                clock,
                last_status: entry.last_status.map(map_status),
                recorded: entry.recorded,
                dropped: counts.lost_records,
                lost_blocks: counts.lost_blocks,
            })
            .collect();
        vec![ProbeDeckSnapshot {
            generation,
            clock,
            confidentiality: Confidentiality::Internal,
            probes,
        }]
    }
}
