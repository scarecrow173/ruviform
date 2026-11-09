//! Probe inspection port — the observability-side vocabulary for
//! reading #185 Generic Probe state without depending on
//! `ruviform-audio` (Issue #292, OBS-10).
//!
//! `ruviform-observability → ruviform-audio` would be a dependency
//! cycle (`ruviform-audio` already depends on this crate), so the
//! concrete probe adapter — mapping `ProbeDeck`/`ProbeCollector`
//! state onto these types — is **#293 / `ruviform-audio`'s
//! responsibility** and does not exist in this crate. What lives
//! here is only the *read* vocabulary plus the [`ProbeSource`] port:
//!
//! - [`ProbeMode`], [`ProbeAvailability`], [`ProbeStatusKind`] (with
//!   [`ProbeStopReason`]/[`ProbeUnobservedReason`]), [`ProbeClockMark`],
//!   [`ProbeReading`], [`ProbeDeckSnapshot`] mirror the audio-side
//!   `typed::probe` contract — `Off`/`Monitor`/`Capture` modes,
//!   `Configured`/`Inferred`/`Unavailable`/`Unsupported`/`Unknown`
//!   availability, generation + clock-mark alignment, and cumulative
//!   loss counters. This module **never re-implements** probe
//!   OFF/Monitor/Capture behavior, capture pools, or rebind logic —
//!   it only reads what a domain adapter reports.
//! - [`ProbeClockMark::UNKNOWN`] mirrors the audio-side fail-closed
//!   default: `ClockDomainId::UNKNOWN` never compares equal to a
//!   known domain, so an unwired deck cannot masquerade as aligned.
//!
//! All value types are `Copy` (except [`ProbeDeckSnapshot`], which
//! owns its readings `Vec`) and carry no payloads — point identity
//! stays an opaque `(node, port)` pair because `NodeId`/`PortId` are
//! domain types this crate cannot name.

use ruviform_media_core::{ClockDomainId, ClockEpochId};

use crate::source::Confidentiality;

/// Mirror of the audio-side `ProbeMode` — the declared observation
/// mode of one probe point.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeMode {
    /// No observation — the probe binding is inert.
    Off,
    /// Per-block metric observation without capture.
    Monitor,
    /// Metric observation plus bounded PCM capture.
    Capture,
}

/// Mirror of the audio-side `PointAvailability` — whether a probe
/// point can actually be observed.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeAvailability {
    /// The point is configured and installable.
    Configured,
    /// The point was inferred (e.g. resolved through a passthrough).
    Inferred,
    /// The point exists but is currently unavailable.
    Unavailable,
    /// The point class cannot be probed at all.
    Unsupported,
    /// Availability could not be determined — fail-closed, never
    /// treated as known.
    Unknown,
}

/// Mirror of the audio-side `StopReason` — why a capture ended.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeStopReason {
    /// Explicit stop request.
    Requested,
    /// The declared frame budget was exhausted.
    BudgetExhausted,
    /// A generation rebind ended the capture; the unspent budget is
    /// discarded, never carried into the new generation.
    GenerationBoundary,
}

/// Mirror of the audio-side `UnobservedReason` — why a point
/// produced no data record for a block.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeUnobservedReason {
    /// The slot stamp predates the current callback — producer
    /// missing or not yet run.
    Stale,
    /// The slot was mutably borrowed at tap time.
    BorrowConflict,
}

/// Mirror of the audio-side `StatusKind` — the latest non-data
/// lifecycle/loss event a probe reported. `Copy`, no payloads
/// beyond the typed fields shown.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStatusKind {
    /// Binding applied.
    Installed,
    /// Observation mode changed.
    ModeChanged {
        /// The new mode.
        to: ProbeMode,
    },
    /// Capture began.
    CaptureStarted {
        /// Remaining frame budget.
        budget_frames: u64,
    },
    /// Capture ended.
    CaptureStopped {
        /// Why it ended.
        reason: ProbeStopReason,
    },
    /// Binding's generation no longer matches the active plan — the
    /// point is skipped until rebind.
    Unbound,
    /// Freshness/borrow audit rejected the slot.
    Unobserved {
        /// Why.
        reason: ProbeUnobservedReason,
    },
    /// PCM blocks were dropped (pool exhausted).
    LostBlocks,
    /// Telemetry records were dropped (channel full).
    LostRecords,
    /// Probe removed.
    Removed,
    /// Binding dropped by a committed generation rebind.
    RebindDropped,
}

/// Clock identity of a probe cursor series — the observability-side
/// mirror of `ruviform_audio::typed::probe::ProbeClockMark`
/// (Issue #309 vocabulary, re-declared here because the audio crate
/// cannot be a dependency).
///
/// [`ProbeClockMark::UNKNOWN`] is the fail-closed default: `domain`
/// is [`ClockDomainId::UNKNOWN`] and the mark never compares equal
/// to a known domain — an unwired deck cannot masquerade as aligned.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProbeClockMark {
    /// Clock domain of the cursor series.
    pub domain: ClockDomainId,
    /// Epoch of the cursor series — bumped on a declared
    /// discontinuity; meaningless while `domain` is `UNKNOWN`.
    pub epoch: ClockEpochId,
}

impl ProbeClockMark {
    /// The unwired mark — stamped before a clock mark is declared.
    pub const UNKNOWN: Self = Self {
        domain: ClockDomainId::UNKNOWN,
        epoch: ClockEpochId::new(0),
    };

    /// Whether this mark carries no usable clock identity —
    /// `domain == ClockDomainId::UNKNOWN`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.domain.get() == ClockDomainId::UNKNOWN.get()
    }
}

/// One probe's non-RT state as of a [`ProbeSource::decks`] call —
/// `Copy`, bounded, carrying generation + clock mark + cumulative
/// loss counters. Point identity stays opaque (`node`/`port` as
/// `u64` — the adapter maps its domain `NodeId`/`PortId` onto them).
///
/// **RT contract:** RT-suitable — `Copy` value type; construction
/// and field reads allocate nothing.
#[derive(Clone, Copy, Debug)]
pub struct ProbeReading {
    /// Domain probe id (opaque — the `ProbeId` equivalent).
    pub probe: u64,
    /// Observed node id (opaque).
    pub node: u64,
    /// Observed port id (opaque).
    pub port: u64,
    /// Declared observation mode.
    pub mode: ProbeMode,
    /// Point availability classification.
    pub availability: ProbeAvailability,
    /// Plan generation the binding resolved against.
    pub generation: u64,
    /// Clock identity of the cursor series at read time.
    pub clock: ProbeClockMark,
    /// Latest non-data status event, if any.
    pub last_status: Option<ProbeStatusKind>,
    /// Cumulative telemetry records accepted.
    pub recorded: u64,
    /// Cumulative telemetry records dropped (channel full).
    pub dropped: u64,
    /// Cumulative PCM blocks lost (pool exhausted).
    pub lost_blocks: u64,
}

/// One probe deck (broker pair) snapshot — the plan generation the
/// deck binds against, its shared clock mark, the adapter-declared
/// confidentiality of the readings, and the per-probe readings.
///
/// The adapter bounds `probes` itself; the inspector applies
/// [`InspectionBudget::max_items`](crate::debug::InspectionBudget)
/// on top.
#[derive(Clone, Debug)]
pub struct ProbeDeckSnapshot {
    /// Plan generation the deck currently binds against.
    pub generation: u64,
    /// Deck-level clock mark — every probe on one cursor shares it.
    pub clock: ProbeClockMark,
    /// Confidentiality the adapter declares for this deck's
    /// readings — drives inspection/report disclosure filtering.
    pub confidentiality: Confidentiality,
    /// Per-probe readings — bounded by the adapter.
    pub probes: Vec<ProbeReading>,
}

/// Non-RT probe read port — implemented by a domain adapter (e.g.
/// an `ruviform-audio` adapter mapping `ProbeDeck` state; that
/// adapter is Issue #293's responsibility) and injected via
/// [`DebugInspector::add_probe_source`](crate::debug::DebugInspector::add_probe_source).
///
/// Called at most once per `inspect` that includes
/// [`InspectionTarget::Probes`](crate::debug::InspectionTarget)
/// (and once more when `Invariants` is also requested, so checks can
/// cite readings — the decks are collected once and shared).
/// Implementations must be non-RT, bounded, and must not block;
/// panics are isolated by the caller and reported as
/// [`SectionStatus::Unavailable`](crate::debug::SectionStatus) with
/// [`UnavailableReason::PortFailure`](crate::debug::UnavailableReason).
pub trait ProbeSource: Send {
    /// Returns the current deck snapshots.
    fn decks(&mut self) -> Vec<ProbeDeckSnapshot>;
}
