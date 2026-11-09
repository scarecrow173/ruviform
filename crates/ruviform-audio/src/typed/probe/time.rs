//! Probe time context, clock marks and observation alignment
//! (Issue #309 — PROBE-TIME).
//!
//! This module is the non-RT half of probe time identity:
//!
//! - [`ProbeClockMark`] is stamped on every [`RecordMeta`] /
//!   [`StatusRecord`] (additive `clock` field) so records self-describe
//!   which `(ClockDomainId, ClockEpochId)` their `sample_position`
//!   cursor belongs to. `UNKNOWN` is never a usable identity — two
//!   unknown domains never align.
//! - [`ProbeTimeContext`] is the per-series non-RT registry entry
//!   (domain/epoch/runtime-instance/generation window/anchors) owned
//!   by the [`ProbeBroker`](super::ProbeBroker). The RT side only ever
//!   sees a copyable [`ProbeClockMark`] delivered through the lifetime
//!   [`ProbeCommand::SetClockMark`](super::ProbeCommand::SetClockMark)
//!   command — minting and epoch bumps stay off the RT path.
//! - [`align_observation_points`] is the explicit three-valued
//!   contract (`Aligned` / `Conditional` / `Incomparable`) comparing
//!   two [`ObservationSpan`]s. It never resamples, never correlates
//!   PCM, and never promotes an unknown clock, latency, epoch,
//!   generation crossing or coverage gap to unconditional `Aligned`.
//! - [`ObservationCoverage`] reconstructs per-probe classified
//!   intervals (valid / unobserved / lost / boundary / off / unknown)
//!   from the drained record stream — measured silence stays `Valid`,
//!   it is never collapsed into "missing".
//!
//! Clock vocabulary reuses `ruviform-media-core`
//! ([`ClockDomainId`], [`ClockEpochId`], [`ClockAnchor`],
//! [`EngineFrame`], [`InputStamp`], [`TimestampProvenance`],
//! [`map_stamp_to_frame`]) — this module adds no second clock
//! authority.

use ruviform_audio_core::ChannelLayout;
use ruviform_media_core::{
    ClockAnchor, ClockDomainId, ClockEpochId, EngineFrame, InputStamp, MapRejection, MediaDuration,
    Rounding, TimeBase, TimestampProvenance, map_stamp_to_frame,
};

use super::{
    ProbeId, ProbePoint, RecordMeta, StatusKind, StatusRecord, StopReason, UnobservedReason,
};

/// Clock identity of the frame-cursor series a record belongs to.
///
/// Stamped on every [`RecordMeta`] and [`StatusRecord`] as `clock`.
/// The mark describes the *series* (one `RtRuntime` frame cursor),
/// not the individual probe — every tap on the same cursor shares
/// the same `(domain, epoch)`.
///
/// [`ProbeClockMark::UNKNOWN`] is the fail-closed default: a deck
/// that was never wired via `SetClockMark` stamps `UNKNOWN`, and
/// `UNKNOWN` never compares equal to anything — including another
/// `UNKNOWN`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProbeClockMark {
    /// Clock domain of the cursor series (`ClockDomainId::UNKNOWN`
    /// until the series is wired).
    pub domain: ClockDomainId,
    /// Epoch of the cursor series — bumped by the non-RT side on a
    /// declared discontinuity. Meaningless while `domain` is
    /// `UNKNOWN`.
    pub epoch: ClockEpochId,
}

impl ProbeClockMark {
    /// The unwired mark — stamped before `SetClockMark` arrives and
    /// on decks that were never wired. `domain` is
    /// [`ClockDomainId::UNKNOWN`]; `epoch` carries no meaning.
    pub const UNKNOWN: Self = Self {
        domain: ClockDomainId::UNKNOWN,
        epoch: ClockEpochId::new(0),
    };

    /// Whether this mark carries no usable clock identity.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.domain.get() == ClockDomainId::UNKNOWN.get()
    }
}

/// How much is known about an observation's effective latency.
///
/// `Unknown` is never treated as a known zero — alignment either
/// refuses it ([`IncomparableReason::LatencyUnknown`]) or bounds it
/// explicitly via [`AlignPolicy::unknown_latency_bound_frames`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LatencyKnowledge {
    /// Effective latency resolved, in the span's declared frame
    /// units.
    Known {
        /// Latency in frames.
        frames: u64,
    },
    /// Latency exists but its value is not known.
    Unknown,
    /// Latency does not apply to this observation — contributes a
    /// known zero to alignment.
    NotApplicable,
}

impl LatencyKnowledge {
    /// The latency in frames when known (`NotApplicable` counts as a
    /// known zero), else `None`.
    #[must_use]
    pub const fn frames(self) -> Option<u64> {
        match self {
            Self::Known { frames } => Some(frames),
            Self::Unknown => None,
            Self::NotApplicable => Some(0),
        }
    }
}

/// Inclusive range of plan generations an observation (or a whole
/// time series) covered.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenerationRange {
    /// First (oldest) generation observed.
    pub first: u64,
    /// Last (newest) generation observed.
    pub last: u64,
}

impl GenerationRange {
    /// A range covering exactly `generation`.
    #[must_use]
    pub const fn single(generation: u64) -> Self {
        Self {
            first: generation,
            last: generation,
        }
    }

    /// Whether the range internally crosses a generation boundary
    /// (`first != last`).
    #[must_use]
    pub const fn crosses(&self) -> bool {
        self.first != self.last
    }
}

/// Non-RT metadata for one probe time series — the registry entry
/// owned by [`ProbeBroker`](super::ProbeBroker) for its `pair()`.
///
/// Identity (`domain`, `epoch`) is what records carry; everything
/// else is lookup context for consumers building [`ObservationSpan`]s
/// — it is never stamped per-record. Series-level only: format and
/// latency are per-observation and live on [`ObservationSpan`].
#[derive(Clone, Debug)]
pub struct ProbeTimeContext {
    /// Clock domain minted for this cursor series — unique per
    /// `pair()` within the probe domain namespace (never a media-core
    /// well-known id).
    pub domain: ClockDomainId,
    /// Current epoch — starts at `ClockEpochId::new(0)` and is bumped
    /// by [`ProbeBroker::bump_epoch`](super::ProbeBroker::bump_epoch)
    /// on a declared discontinuity.
    pub epoch: ClockEpochId,
    /// Non-RT-minted runtime instance identity — distinguishes
    /// rebuilt runtimes even if a domain were ever reused.
    pub runtime_instance: u64,
    /// Declared engine sample rate backing the frame cursor
    /// (`0` = undeclared). [`Self::frame_base`] derives the
    /// [`TimeBase`] used for cursor→[`EngineFrame`] anchor mapping.
    pub sample_rate: u32,
    /// Provenance of cursor-derived stamps — `Derived` by
    /// construction (the runtime counts processed frames),
    /// overridable by the consumer.
    pub provenance: TimestampProvenance,
    /// `stamp ↔ EngineFrame` anchors for this series. A cursor→engine
    /// anchor uses `domain == self.domain`, `stamp_base ==
    /// self.frame_base()`, and `stamp_ticks ==` cursor position.
    /// Empty until the consumer wires one (device-side anchor
    /// generation is deferred — see the Issue #309 plan).
    pub anchors: Vec<ClockAnchor>,
    /// Plan generations this series has observed — `None` until the
    /// first install/publish is recorded.
    pub generation_window: Option<GenerationRange>,
}

impl ProbeTimeContext {
    /// The mark currently in force for this series.
    #[must_use]
    pub const fn mark(&self) -> ProbeClockMark {
        ProbeClockMark {
            domain: self.domain,
            epoch: self.epoch,
        }
    }

    /// The [`TimeBase`] of one cursor frame (`1/sample_rate`), or
    /// `None` while `sample_rate` is undeclared/invalid.
    #[must_use]
    pub fn frame_base(&self) -> Option<TimeBase> {
        TimeBase::sample_rate(self.sample_rate).ok()
    }

    /// The first anchor calibrating this series' domain, if any.
    fn domain_anchor(&self) -> Option<&ClockAnchor> {
        self.anchors.iter().find(|a| a.domain == self.domain)
    }

    /// Maps a cursor `position` to an [`EngineFrame`] through this
    /// series' anchor — `None`-equivalent failures surface as
    /// [`MapRejection`] (`UnmappedDomain` when no anchor exists).
    ///
    /// `now` is taken as the anchor's own frame so the epoch check is
    /// self-consistent: this maps *historical* positions for offline
    /// alignment, it does not police live anchor staleness.
    ///
    /// # Errors
    /// [`MapRejection`] when no usable anchor/base exists or the
    /// position precedes the anchor / overflows.
    pub fn cursor_to_engine_frame(&self, position: u64) -> Result<EngineFrame, MapRejection> {
        let base = self.frame_base().ok_or(MapRejection::UnmappedDomain)?;
        let anchor = self.domain_anchor().ok_or(MapRejection::UnmappedDomain)?;
        let stamp = InputStamp {
            ticks: position,
            base,
            domain: self.domain,
            provenance: self.provenance,
        };
        map_stamp_to_frame(&stamp, anchor, base, anchor.frame).map(|m| m.frame)
    }
}

/// What the observed values of a span are — declared so consumers
/// never mistake f32 RT probe values for a bit-exact source claim
/// (f64 native analysis comparison is Issue #294's separate gate).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProbePrecision {
    /// f32 samples observed on the RT probe path.
    F32Rt,
    /// f64 native/reference values (e.g. an analysis-side source).
    F64Native,
}

/// Declared format of one observation span.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationFormat {
    /// Logical channel count.
    pub channels: u16,
    /// Declared sample rate of the observed port's frame unit.
    pub sample_rate: u32,
    /// Declared channel layout.
    pub channel_layout: ChannelLayout,
}

/// Half-open frame interval `[start, end)` in cursor coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameInterval {
    /// First frame (inclusive).
    pub start: u64,
    /// One past the last frame (exclusive).
    pub end: u64,
}

impl FrameInterval {
    /// Creates an interval; `start >= end` denotes an empty interval.
    #[must_use]
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// Whether the interval covers no frames.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Intersection with `other`, `None` when disjoint.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(Self { start, end })
    }
}

/// Classification of one reconstructed coverage interval.
///
/// `Valid` means *measured* data was observed — it includes measured
/// silence; it is never a claim about signal content.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoverageClass {
    /// Observed data (metric or capture chunk) covered this interval.
    Valid,
    /// The freshness/borrow audit rejected the block.
    Unobserved {
        /// Why the block was skipped.
        reason: UnobservedReason,
    },
    /// Capture PCM dropped — chunk pool exhausted.
    LostBlocks,
    /// A telemetry record was dropped — the loss counter carries no
    /// position, so reconstruction downgrades windows to `Unknown`
    /// instead of guessing where.
    LostRecords,
    /// A plan-generation rebind boundary falls inside this interval.
    GenerationBoundary,
    /// The binding was dropped by a committed rebind.
    RebindDropped,
    /// The probe was `Off` (or removed) — no observation by design.
    Off,
    /// The point's capability is unsupported for observation.
    Unsupported,
    /// The point has no backing slot.
    Unavailable,
    /// State cannot be determined — unexplained gaps and telemetry
    /// loss land here rather than being claimed valid.
    Unknown,
}

/// One classified half-open interval `[start, end)` in cursor frame
/// coordinates, tagged with the clock mark in effect there.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CoverageInterval {
    /// First frame (inclusive).
    pub start: u64,
    /// One past the last frame (exclusive).
    pub end: u64,
    /// Interval classification.
    pub class: CoverageClass,
    /// Clock mark the records in this interval carried.
    pub mark: ProbeClockMark,
}

/// Owned coverage input — built while draining
/// [`ProbeCollector::drain`](super::ProbeCollector::drain).
#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum CoverageEvent {
    /// Metric or capture chunk — carries the record's [`RecordMeta`]
    /// (position + frames + clock mark).
    Data(RecordMeta),
    /// Lifecycle/fault/loss status.
    Status(StatusRecord),
}

/// Per-probe observation coverage reconstructed from the drained
/// record stream — sorted, disjoint, classified intervals in cursor
/// coordinates.
///
/// Reconstruction is best-effort and conservative: unexplained gaps
/// become [`CoverageClass::Unknown`], statuses overwrite data they
/// cover, and a nonzero `lost_records` delta downgrades every
/// interval to `Unknown` (the counter cannot localize the loss, so
/// no window may claim validity it cannot prove).
#[derive(Clone, Debug, Default)]
pub struct ObservationCoverage {
    /// Classified intervals sorted by `start`, disjoint.
    pub intervals: Vec<CoverageInterval>,
}

/// Status effects used by coverage reconstruction.
#[derive(Copy, Clone)]
enum StatusEffect {
    /// Per-block classification covering `[pos, next event pos)`.
    Block(CoverageClass),
    /// Persistent state until a resume signal.
    State(CoverageClass),
    /// Closes any open state (probe observing again).
    Resume,
}

/// One probe event normalized for reconstruction.
#[derive(Copy, Clone)]
struct CovEvent {
    pos: u64,
    /// `(end, mark)` when this event is observed data.
    data: Option<(u64, ProbeClockMark)>,
    effect: Option<StatusEffect>,
    mark: ProbeClockMark,
}

fn status_effect(kind: StatusKind) -> StatusEffect {
    match kind {
        StatusKind::Unobserved { reason } => {
            StatusEffect::Block(CoverageClass::Unobserved { reason })
        }
        StatusKind::LostBlocks => StatusEffect::Block(CoverageClass::LostBlocks),
        StatusKind::LostRecords => StatusEffect::Block(CoverageClass::LostRecords),
        StatusKind::Unbound => StatusEffect::Block(CoverageClass::Unknown),
        StatusKind::ModeChanged { to } => {
            if to == super::ProbeMode::Off {
                StatusEffect::State(CoverageClass::Off)
            } else {
                StatusEffect::Resume
            }
        }
        StatusKind::CaptureStarted { .. }
        | StatusKind::Installed
        | StatusKind::CaptureStopped {
            reason: StopReason::Requested | StopReason::BudgetExhausted,
        } => StatusEffect::Resume,
        StatusKind::CaptureStopped {
            reason: StopReason::GenerationBoundary,
        } => StatusEffect::Block(CoverageClass::GenerationBoundary),
        StatusKind::Removed => StatusEffect::State(CoverageClass::Off),
        StatusKind::RebindDropped => StatusEffect::State(CoverageClass::RebindDropped),
    }
}

/// A coverage candidate before the priority sweep.
#[derive(Copy, Clone)]
struct Cand {
    start: u64,
    end: u64,
    class: CoverageClass,
    mark: ProbeClockMark,
    /// Status-derived intervals outrank observed data on overlap.
    status: bool,
}

impl ObservationCoverage {
    /// Reconstructs coverage for `probe` from drained events.
    ///
    /// `lost_records` is the telemetry-loss counter *delta* over the
    /// drained window ([`ProbeCounts::lost_records`](super::ProbeCounts::lost_records)
    /// before/after `drain`). Any positive value downgrades every
    /// interval to [`CoverageClass::Unknown`] — the counter carries
    /// no position, so no window can prove it was not affected.
    ///
    /// Non-RT only — allocates.
    #[must_use]
    pub fn reconstruct(
        probe: ProbeId,
        lost_records: u64,
        events: impl IntoIterator<Item = CoverageEvent>,
    ) -> Self {
        let mut evs: Vec<CovEvent> = Vec::new();
        for ev in events {
            match ev {
                CoverageEvent::Data(meta) if meta.probe == probe => evs.push(CovEvent {
                    pos: meta.sample_position,
                    data: Some((
                        meta.sample_position.saturating_add(u64::from(meta.frames)),
                        meta.clock,
                    )),
                    effect: None,
                    mark: meta.clock,
                }),
                CoverageEvent::Status(s) if s.probe == probe => evs.push(CovEvent {
                    pos: s.sample_position,
                    data: None,
                    effect: Some(status_effect(s.kind)),
                    mark: s.clock,
                }),
                _ => {}
            }
        }
        if evs.is_empty() {
            return Self::default();
        }
        evs.sort_by_key(|e| e.pos);

        // Per-block status intervals end at the next event position;
        // the block size is inferred from observed data when the
        // status is the last event.
        let block_hint = block_hint(&evs);
        let intervals = sweep(&candidates(&evs, block_hint));
        let mut cov = Self { intervals };
        if lost_records > 0 {
            for iv in &mut cov.intervals {
                iv.class = CoverageClass::Unknown;
            }
        }
        cov
    }
}

/// Builds the coverage candidates for sorted events: observed data
/// intervals plus per-block status intervals and persistent states
/// resolved in one forward pass.
fn candidates(evs: &[CovEvent], block_hint: u64) -> Vec<Cand> {
    let mut cands: Vec<Cand> = Vec::new();
    // Data candidates.
    for e in evs {
        if let Some((end, mark)) = e.data
            && end > e.pos
        {
            cands.push(Cand {
                start: e.pos,
                end,
                class: CoverageClass::Valid,
                mark,
                status: false,
            });
        }
    }
    // Status candidates.
    let mut open_state: Option<(CoverageClass, u64, ProbeClockMark)> = None;
    for (i, e) in evs.iter().enumerate() {
        let next_pos = evs[i + 1..]
            .iter()
            .map(|n| n.pos)
            .find(|&p| p > e.pos)
            .unwrap_or_else(|| e.pos.saturating_add(block_hint));
        if let Some(effect) = e.effect {
            match effect {
                StatusEffect::Block(class) => {
                    if next_pos > e.pos {
                        cands.push(Cand {
                            start: e.pos,
                            end: next_pos,
                            class,
                            mark: e.mark,
                            status: true,
                        });
                    }
                }
                StatusEffect::State(class) => {
                    if let Some((oc, os, om)) = open_state.take()
                        && e.pos > os
                    {
                        cands.push(Cand {
                            start: os,
                            end: e.pos,
                            class: oc,
                            mark: om,
                            status: true,
                        });
                    }
                    open_state = Some((class, e.pos, e.mark));
                }
                StatusEffect::Resume => {
                    if let Some((oc, os, om)) = open_state.take()
                        && e.pos > os
                    {
                        cands.push(Cand {
                            start: os,
                            end: e.pos,
                            class: oc,
                            mark: om,
                            status: true,
                        });
                    }
                }
            }
        }
    }
    if let Some((oc, os, om)) = open_state {
        let end = evs
            .last()
            .map_or(os, |e| e.pos)
            .max(os.saturating_add(block_hint));
        if end > os {
            cands.push(Cand {
                start: os,
                end,
                class: oc,
                mark: om,
                status: true,
            });
        }
    }
    cands
}

/// Infers the per-block frame count for trailing status intervals:
/// the modal `frames` of data records, else the smallest positive
/// delta between consecutive event positions, else 1.
fn block_hint(evs: &[CovEvent]) -> u64 {
    let mut counts: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    for e in evs {
        if let Some((end, _)) = e.data {
            *counts.entry(end.saturating_sub(e.pos)).or_insert(0) += 1;
        }
    }
    if let Some((&frames, _)) = counts.iter().max_by_key(|(_, n)| *n) {
        return frames.max(1);
    }
    evs.windows(2)
        .filter_map(|w| w[1].pos.checked_sub(w[0].pos))
        .filter(|&d| d > 0)
        .min()
        .unwrap_or(1)
}

/// Priority sweep: sorts candidates, walks boundary points, and for
/// each elementary segment keeps the highest-priority covering
/// candidate (status-derived beats data; later start wins ties).
/// Adjacent segments with equal `(class, mark)` merge.
fn sweep(cands: &[Cand]) -> Vec<CoverageInterval> {
    if cands.is_empty() {
        return Vec::new();
    }
    let mut bounds: Vec<u64> = Vec::with_capacity(cands.len() * 2);
    for c in cands {
        bounds.push(c.start);
        bounds.push(c.end);
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut out: Vec<CoverageInterval> = Vec::new();
    for w in bounds.windows(2) {
        let (s, e) = (w[0], w[1]);
        if s >= e {
            continue;
        }
        // Highest priority covering candidate: status first, then
        // latest start, then widest.
        let best = cands
            .iter()
            .filter(|c| c.start <= s && c.end >= e)
            .max_by_key(|c| (c.status, c.start, c.end));
        let Some(c) = best else { continue };
        if let Some(last) = out.last_mut()
            && last.end == s
            && last.class == c.class
            && last.mark == c.mark
        {
            last.end = e;
            continue;
        }
        out.push(CoverageInterval {
            start: s,
            end: e,
            class: c.class,
            mark: c.mark,
        });
    }
    // Fill unexplained gaps inside the observed range with Unknown.
    if out.len() >= 2 {
        let mut filled: Vec<CoverageInterval> = Vec::with_capacity(out.len() * 2);
        filled.push(out[0]);
        for iv in out.iter().skip(1) {
            let prev = filled.last().unwrap();
            if iv.start > prev.end {
                filled.push(CoverageInterval {
                    start: prev.end,
                    end: iv.start,
                    class: CoverageClass::Unknown,
                    mark: iv.mark,
                });
            }
            filled.push(*iv);
        }
        return filled;
    }
    out
}

/// Alignment basis actually used — `SharedCursor` for same
/// `(domain, epoch)`, `EngineFrame` when both spans map through
/// anchors into one engine epoch.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlignmentBasis {
    /// Same cursor series and epoch — offsets are cursor frame
    /// differences (latency-compensated).
    SharedCursor {
        /// Shared clock domain.
        domain: ClockDomainId,
        /// Shared clock epoch.
        epoch: ClockEpochId,
    },
    /// Both spans mapped to [`EngineFrame`]s through their contexts'
    /// anchors — offsets are engine frame differences.
    EngineFrame {
        /// Shared engine epoch the frames live in.
        epoch: ClockEpochId,
    },
}

/// A condition that keeps an alignment `Conditional` rather than
/// `Aligned` — evidence the consumer must weigh, never hidden.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlignmentCondition {
    /// One side's latency is `Unknown`; the declared bound frames the
    /// worst-case misalignment.
    LatencyBounded {
        /// Worst-case latency uncertainty in frames.
        bound_frames: u64,
    },
    /// Anchor calibration or stamp provenance is `Estimated`.
    ProvenanceEstimated,
    /// Part of the compared window lacks valid coverage on at least
    /// one side — `excluded_intervals` names it.
    PartialCoverage,
    /// The spans' generation ranges differ or cross a rebind.
    CrossGeneration,
    /// The spans declare different precisions (e.g. f32 RT vs f64
    /// native) — comparison is meaningful only with tolerance.
    MixedPrecision,
}

/// Why two spans cannot be aligned at all.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IncomparableReason {
    /// Either span's clock mark is `UNKNOWN` — unknown domains never
    /// align, not even with each other.
    UnknownClock,
    /// Different clock domains and no shared anchor evidence.
    DomainMismatch,
    /// Same domain but different epochs and no anchor evidence.
    EpochMismatch,
    /// Anchor mapping placed the spans in different engine epochs —
    /// different frame series.
    SeriesMismatch,
    /// Declared sample rates differ and no conversion was permitted
    /// (or the rate is unusable).
    RateMismatch {
        /// First span's declared rate.
        a: u32,
        /// Second span's declared rate.
        b: u32,
    },
    /// Channel count or layout differ — not the same signal shape.
    FormatMismatch,
    /// Latency is `Unknown` and the policy allows no bound.
    LatencyUnknown,
    /// No jointly valid coverage inside the compared window.
    CoverageEmpty,
    /// A generation/epoch discontinuity or mixed clock marks fall
    /// inside the compared window.
    DiscontinuityInWindow,
    /// The spans' generation ranges cross a plan boundary while the
    /// policy requires a single generation.
    GenerationSpan,
    /// No anchor exists for a span's domain (or its context/frame
    /// base is undeclared).
    AnchorMissing,
    /// Anchor mapping was attempted and rejected.
    AnchorRejected {
        /// The media-core rejection.
        rejection: MapRejection,
    },
    /// Frame arithmetic overflowed.
    Overflow,
    /// Computed `error_budget` exceeds `AlignPolicy::max_error_frames`.
    ErrorBudgetExceeded,
    /// Provenance is `Estimated` and conditional results are
    /// disallowed.
    EstimatedProvenance,
}

/// Explicit consumer choices for [`align_observation_points`]. No
/// implicit promotion: stricter policies return `Incomparable`
/// where permissive ones return `Conditional`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "flat policy bag — each flag is an independent consumer choice"
)]
#[derive(Clone, Debug)]
pub struct AlignPolicy {
    /// Both spans must share one plan generation (no crossing and
    /// equal ranges) — else `Incomparable{GenerationSpan}`.
    pub require_same_generation: bool,
    /// Both spans must declare known latency — else
    /// `Incomparable{LatencyUnknown}` regardless of bounds.
    pub require_known_latency: bool,
    /// Permit rational [`TimeBase`] conversion when declared sample
    /// rates differ (residue enters `error_budget`).
    pub allow_rate_conversion: bool,
    /// When latency is `Unknown`, the frame bound that permits
    /// `Conditional{LatencyBounded}`; `None` refuses.
    pub unknown_latency_bound_frames: Option<u64>,
    /// Maximum tolerated `error_budget` in frames — exceeding it is
    /// `Incomparable{ErrorBudgetExceeded}`.
    pub max_error_frames: u64,
    /// Whether `Conditional` results may be returned; `false` turns
    /// every condition into the matching `Incomparable` reason.
    pub allow_conditional: bool,
    /// Try the `EngineFrame` anchor path before `SharedCursor`.
    pub prefer_engine_frame: bool,
}

impl Default for AlignPolicy {
    fn default() -> Self {
        Self {
            require_same_generation: true,
            require_known_latency: true,
            allow_rate_conversion: false,
            unknown_latency_bound_frames: None,
            max_error_frames: 0,
            allow_conditional: true,
            prefer_engine_frame: false,
        }
    }
}

/// A self-describing view of one probe's observation over a frame
/// interval — the input to [`align_observation_points`].
///
/// `interval` and `coverage` are in the emitting series' cursor frame
/// coordinates; `clock_mark` is the mark those records carried.
pub struct ObservationSpan<'a> {
    /// Emitting probe.
    pub probe: ProbeId,
    /// Observed point.
    pub point: ProbePoint,
    /// Clock mark of the span's records.
    pub clock_mark: ProbeClockMark,
    /// Covered cursor interval `[start, end)`.
    pub interval: FrameInterval,
    /// Plan generations the span's records covered.
    pub generation_range: GenerationRange,
    /// Declared format.
    pub format: ObservationFormat,
    /// Latency knowledge.
    pub latency: LatencyKnowledge,
    /// Reconstructed coverage for this probe.
    pub coverage: &'a ObservationCoverage,
    /// Series context for anchor/provenance lookup — `None` when the
    /// consumer has no registry entry (anchor paths then fail closed).
    pub context: Option<&'a ProbeTimeContext>,
    /// Value precision.
    pub precision: ProbePrecision,
}

/// The three-valued alignment result.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum Alignment {
    /// Unconditionally aligned — basis, offset, error budget and the
    /// jointly observed overlap are all explicit.
    Aligned {
        /// Coordinate basis used.
        basis: AlignmentBasis,
        /// A→B offset in basis frame units for the same signal
        /// instant (latency-compensated on `SharedCursor`; `0` on
        /// `EngineFrame` where the basis is already signal-aligned).
        offset: i64,
        /// Worst-case alignment error in frames, with its basis
        /// stated by construction (rounding residue, latency bound).
        error_budget: u64,
        /// Jointly observed overlap in B-side basis coordinates.
        overlap: Option<FrameInterval>,
    },
    /// Alignable only under the stated conditions — never silently
    /// promoted.
    Conditional {
        /// Coordinate basis used.
        basis: AlignmentBasis,
        /// A→B offset (same convention as [`Alignment::Aligned`]).
        offset: i64,
        /// Worst-case alignment error in frames.
        error_budget: u64,
        /// Conditions the consumer must weigh.
        conditions: Vec<AlignmentCondition>,
        /// Sub-intervals of the overlap excluded from valid
        /// comparison, in B-side basis coordinates.
        excluded_intervals: Vec<FrameInterval>,
    },
    /// Cannot be aligned — explicit reason.
    Incomparable {
        /// Why.
        reason: IncomparableReason,
    },
}

impl Alignment {
    /// Whether this is [`Alignment::Aligned`].
    #[must_use]
    pub fn is_aligned(&self) -> bool {
        matches!(self, Self::Aligned { .. })
    }
}

fn inc(reason: IncomparableReason) -> Alignment {
    Alignment::Incomparable { reason }
}

/// Internal basis resolution carrying the engine mapping results.
enum Basis {
    SharedCursor {
        domain: ClockDomainId,
        epoch: ClockEpochId,
    },
    Engine {
        a: EngineSide,
        b: EngineSide,
    },
}

/// One span's engine-epoch result plus mapping provenance.
struct EngineSide {
    epoch: ClockEpochId,
    provenance: TimestampProvenance,
}

/// Maps a span's cursor interval to engine frames via its context's
/// anchor. `Err` carries the `IncomparableReason` directly.
fn map_span_to_engine(span: &ObservationSpan<'_>) -> Result<EngineSide, IncomparableReason> {
    let ctx = span.context.ok_or(IncomparableReason::AnchorMissing)?;
    let base = ctx.frame_base().ok_or(IncomparableReason::AnchorMissing)?;
    let anchor = ctx
        .anchors
        .iter()
        .find(|a| a.domain == span.clock_mark.domain)
        .ok_or(IncomparableReason::AnchorMissing)?;
    let stamp = |ticks: u64| InputStamp {
        ticks,
        base,
        domain: span.clock_mark.domain,
        provenance: ctx.provenance,
    };
    let m_start = map_stamp_to_frame(&stamp(span.interval.start), anchor, base, anchor.frame)
        .map_err(|rejection| IncomparableReason::AnchorRejected { rejection })?;
    let m_end = map_stamp_to_frame(&stamp(span.interval.end), anchor, base, anchor.frame)
        .map_err(|rejection| IncomparableReason::AnchorRejected { rejection })?;
    if m_start.frame.epoch() != m_end.frame.epoch() {
        return Err(IncomparableReason::SeriesMismatch);
    }
    Ok(EngineSide {
        epoch: m_start.frame.epoch(),
        provenance: ruviform_media_core::weaker_provenance(m_start.provenance, m_end.provenance),
    })
}

/// Resolves the comparison basis. Direct `(domain, epoch)` equality
/// wins unless `prefer_engine_frame` is set; otherwise both sides
/// must map through anchors into one engine epoch.
fn resolve_basis(
    a: &ObservationSpan<'_>,
    b: &ObservationSpan<'_>,
    policy: &AlignPolicy,
) -> Result<Basis, IncomparableReason> {
    let same_domain = a.clock_mark.domain == b.clock_mark.domain;
    let same_epoch = a.clock_mark.epoch == b.clock_mark.epoch;
    let direct = same_domain && same_epoch;

    if policy.prefer_engine_frame || !direct {
        match (map_span_to_engine(a), map_span_to_engine(b)) {
            (Ok(ea), Ok(eb)) => {
                if ea.epoch == eb.epoch {
                    return Ok(Basis::Engine { a: ea, b: eb });
                }
                // Anchor evidence placed the spans in different
                // engine epochs — only fatal when no direct cursor
                // basis exists to fall back to.
                if !direct {
                    return Err(IncomparableReason::SeriesMismatch);
                }
            }
            // One-sided anchor evidence that failed is more
            // informative than the bare domain/epoch mismatch — but
            // only when no direct basis exists.
            (Err(r), Ok(_)) | (Ok(_), Err(r)) if !direct => return Err(r),
            _ => {}
        }
    }
    if direct {
        return Ok(Basis::SharedCursor {
            domain: a.clock_mark.domain,
            epoch: a.clock_mark.epoch,
        });
    }
    Err(if same_domain {
        IncomparableReason::EpochMismatch
    } else {
        IncomparableReason::DomainMismatch
    })
}

/// Converts `value` frames from rate `from` to rate `to` via
/// rational [`TimeBase`] math with [`Rounding::NearestEven`].
/// Returns the converted value and a conservative residue bound
/// (one frame per conversion — `NearestEven` residue is < 0.5 frame).
fn convert_frames(value: u64, from: u32, to: u32) -> Result<(u64, u64), ()> {
    let from_base = TimeBase::sample_rate(from).map_err(|_| ())?;
    let to_base = TimeBase::sample_rate(to).map_err(|_| ())?;
    let dur = MediaDuration::new(i64::try_from(value).map_err(|_| ())?, from_base);
    let conv = dur
        .convert(to_base, Rounding::NearestEven)
        .map_err(|_| ())?;
    let v = u64::try_from(conv.value()).map_err(|_| ())?;
    Ok((v, 1))
}

/// Interval in i128 coordinates — offsets can shift ranges negative
/// before clamping to the reported u64 basis coordinates.
#[derive(Copy, Clone)]
struct IVal {
    start: i128,
    end: i128,
}

impl IVal {
    fn intersection(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(Self { start, end })
    }
}

/// Sorted disjoint i128 intervals covering `range` minus `cuts`.
fn subtract(range: IVal, cuts: &[IVal]) -> Vec<IVal> {
    let mut out = vec![range];
    for cut in cuts {
        let mut next = Vec::with_capacity(out.len() + 1);
        for seg in out {
            match seg.intersection(cut) {
                None => next.push(seg),
                Some(over) => {
                    if seg.start < over.start {
                        next.push(IVal {
                            start: seg.start,
                            end: over.start,
                        });
                    }
                    if over.end < seg.end {
                        next.push(IVal {
                            start: over.end,
                            end: seg.end,
                        });
                    }
                }
            }
        }
        out = next;
    }
    out
}

/// Union of sorted interval lists into sorted disjoint intervals.
fn union(mut segs: Vec<IVal>) -> Vec<IVal> {
    segs.sort_by_key(|s| s.start);
    let mut out: Vec<IVal> = Vec::with_capacity(segs.len());
    for s in segs {
        if let Some(last) = out.last_mut()
            && s.start <= last.end
        {
            last.end = last.end.max(s.end);
        } else {
            out.push(s);
        }
    }
    out
}

/// Per-side evidence collected in common (B-side) coordinates.
struct SideView {
    /// Span interval mapped to common coords.
    interval: IVal,
    /// Valid coverage segments mapped likewise.
    valid: Vec<IVal>,
    /// Coverage intervals proving a hard discontinuity inside the
    /// span's own window (generation boundary / rebind drop / mixed
    /// clock marks) — mapped to common coords.
    hard: Vec<IVal>,
}

/// Aligns two observation spans under `policy` — the Issue #309
/// three-valued contract.
///
/// `Aligned` requires: known clock identity on both sides, a shared
/// `(domain, epoch)` cursor or anchor-mapped `EngineFrame` basis,
/// compatible declared formats, policy-satisfying latency and
/// generation knowledge, continuous valid coverage over the overlap,
/// and an `error_budget` within `max_error_frames`.
/// Everything weaker is `Conditional` (with explicit conditions and
/// excluded intervals) or `Incomparable` (with an explicit reason).
/// Never resamples, never correlates PCM, never treats `UNKNOWN` as
/// a usable identity.
///
/// Non-RT only — allocates for interval sets.
#[must_use]
pub fn align_observation_points(
    a: &ObservationSpan<'_>,
    b: &ObservationSpan<'_>,
    policy: &AlignPolicy,
) -> Alignment {
    // Fail closed on unknown clocks before anything else — two
    // UNKNOWN domains never align, not even with each other.
    if a.clock_mark.is_unknown() || b.clock_mark.is_unknown() {
        return inc(IncomparableReason::UnknownClock);
    }
    if a.interval.is_empty() || b.interval.is_empty() {
        return inc(IncomparableReason::CoverageEmpty);
    }

    let basis = match resolve_basis(a, b, policy) {
        Ok(b) => b,
        Err(r) => return inc(r),
    };

    // Format compatibility: channel count and layout must match.
    if a.format.channels != b.format.channels || a.format.channel_layout != b.format.channel_layout
    {
        return inc(IncomparableReason::FormatMismatch);
    }

    // Rate compatibility: differing declared rates need explicit
    // conversion permission; unusable rates always fail.
    let (rate_a, rate_b) = match &basis {
        Basis::SharedCursor { .. } => (a.format.sample_rate, b.format.sample_rate),
        Basis::Engine { .. } => (
            a.context.map_or(0, |c| c.sample_rate),
            b.context.map_or(0, |c| c.sample_rate),
        ),
    };
    if rate_a != rate_b && (rate_a == 0 || rate_b == 0 || !policy.allow_rate_conversion) {
        return inc(IncomparableReason::RateMismatch {
            a: rate_a,
            b: rate_b,
        });
    }

    // Fail-closed phases, each returning its demotion conditions or
    // the incomparable reason — nothing promotes silently.
    let mut eval = AlignEval::default();
    if let Err(r) = latency_term(
        a.latency,
        b.latency,
        &basis,
        (rate_a, rate_b),
        policy,
        &mut eval,
    ) {
        return inc(r);
    }
    if let Err(r) = evidence_conditions(a, b, &basis, policy, &mut eval.conditions) {
        return inc(r);
    }
    let phase = match coverage_phase(
        a,
        b,
        &basis,
        eval.offset,
        (rate_a, rate_b),
        policy,
        &mut eval,
    ) {
        Ok(p) => p,
        Err(r) => return inc(r),
    };

    if eval.error_budget > policy.max_error_frames {
        return inc(IncomparableReason::ErrorBudgetExceeded);
    }

    let out_basis = match &basis {
        Basis::SharedCursor { domain, epoch } => AlignmentBasis::SharedCursor {
            domain: *domain,
            epoch: *epoch,
        },
        Basis::Engine { b: eb, .. } => AlignmentBasis::EngineFrame { epoch: eb.epoch },
    };
    let overlap_out = Some(FrameInterval::new(
        u64::try_from(phase.overlap.start.max(0)).unwrap_or(0),
        u64::try_from(phase.overlap.end.max(0)).unwrap_or(0),
    ));
    if eval.conditions.is_empty() {
        Alignment::Aligned {
            basis: out_basis,
            offset: eval.offset,
            error_budget: eval.error_budget,
            overlap: overlap_out,
        }
    } else {
        Alignment::Conditional {
            basis: out_basis,
            offset: eval.offset,
            error_budget: eval.error_budget,
            conditions: eval.conditions,
            excluded_intervals: phase
                .excluded
                .iter()
                .map(|v| {
                    FrameInterval::new(
                        u64::try_from(v.start.max(0)).unwrap_or(0),
                        u64::try_from(v.end.max(0)).unwrap_or(0),
                    )
                })
                .collect(),
        }
    }
}

/// Mutable evaluation state threaded through the align phases —
/// accumulated error budget, demotion conditions, and the computed
/// A→B offset.
#[derive(Default)]
struct AlignEval {
    /// Worst-case alignment error in basis frames.
    error_budget: u64,
    /// Conditions keeping the result `Conditional`.
    conditions: Vec<AlignmentCondition>,
    /// A→B offset in B-side basis units (`SharedCursor` only).
    offset: i64,
}

/// Latency phase — computes the A→B offset (`SharedCursor` only) and
/// records bounded-unknown conditions. `Err` is the fail-closed
/// reason.
fn latency_term(
    lat_a: LatencyKnowledge,
    lat_b: LatencyKnowledge,
    basis: &Basis,
    rates: (u32, u32),
    policy: &AlignPolicy,
    eval: &mut AlignEval,
) -> Result<(), IncomparableReason> {
    let (rate_a, rate_b) = rates;
    if let (Some(la), Some(lb)) = (lat_a.frames(), lat_b.frames()) {
        if matches!(basis, Basis::SharedCursor { .. }) {
            // Convert A's latency into B's declared frame units when
            // rates differ, then take the difference.
            let lat_a_in_b = if rate_a == rate_b {
                la
            } else {
                match convert_frames(la, rate_a, rate_b) {
                    Ok((v, r)) => {
                        eval.error_budget += r;
                        v
                    }
                    Err(()) => {
                        return Err(IncomparableReason::RateMismatch {
                            a: rate_a,
                            b: rate_b,
                        });
                    }
                }
            };
            eval.offset = i64::try_from(lb)
                .unwrap_or(i64::MAX)
                .saturating_sub(i64::try_from(lat_a_in_b).unwrap_or(i64::MAX));
        }
        return Ok(());
    }
    if policy.require_known_latency {
        return Err(IncomparableReason::LatencyUnknown);
    }
    match policy.unknown_latency_bound_frames {
        Some(bound) => {
            eval.conditions.push(AlignmentCondition::LatencyBounded {
                bound_frames: bound,
            });
            eval.error_budget = eval.error_budget.saturating_add(bound);
            Ok(())
        }
        None => Err(IncomparableReason::LatencyUnknown),
    }
}

/// Generation/precision/provenance phase — pushes demotion
/// conditions or fails closed.
fn evidence_conditions(
    a: &ObservationSpan<'_>,
    b: &ObservationSpan<'_>,
    basis: &Basis,
    policy: &AlignPolicy,
    conditions: &mut Vec<AlignmentCondition>,
) -> Result<(), IncomparableReason> {
    // Generation knowledge — a generation crossing is an observation
    // discontinuity, not coordinate continuity.
    let gen_cross = a.generation_range != b.generation_range
        || a.generation_range.crosses()
        || b.generation_range.crosses();
    if gen_cross {
        if policy.require_same_generation {
            return Err(IncomparableReason::GenerationSpan);
        }
        if policy.allow_conditional {
            conditions.push(AlignmentCondition::CrossGeneration);
        } else {
            return Err(IncomparableReason::GenerationSpan);
        }
    }

    // Precision.
    if a.precision != b.precision {
        if policy.allow_conditional {
            conditions.push(AlignmentCondition::MixedPrecision);
        } else {
            return Err(IncomparableReason::FormatMismatch);
        }
    }

    // Provenance: Estimated anywhere along the evidence chain
    // demotes to Conditional.
    let estimated = match basis {
        Basis::Engine { a: ea, b: eb } => {
            ea.provenance == TimestampProvenance::Estimated
                || eb.provenance == TimestampProvenance::Estimated
        }
        Basis::SharedCursor { .. } => {
            a.context
                .is_some_and(|c| c.provenance == TimestampProvenance::Estimated)
                || b.context
                    .is_some_and(|c| c.provenance == TimestampProvenance::Estimated)
        }
    };
    if estimated {
        if policy.allow_conditional {
            conditions.push(AlignmentCondition::ProvenanceEstimated);
        } else {
            return Err(IncomparableReason::EstimatedProvenance);
        }
    }
    Ok(())
}

/// Mapped overlap and excluded (not jointly valid) intervals in
/// common B-side i128 coordinates.
struct CoveragePhase {
    /// Intersection of the two mapped span windows.
    overlap: IVal,
    /// Parts of the overlap lacking joint valid coverage.
    excluded: Vec<IVal>,
}

/// Coverage phase — maps both spans into common coordinates, checks
/// hard discontinuities, and computes the jointly valid overlap.
/// Pushes `PartialCoverage` into `eval.conditions` (or fails closed
/// when conditional results are disallowed) and adds the documented
/// rate-conversion residue to `eval.error_budget`.
fn coverage_phase(
    a: &ObservationSpan<'_>,
    b: &ObservationSpan<'_>,
    basis: &Basis,
    offset: i64,
    rates: (u32, u32),
    policy: &AlignPolicy,
    eval: &mut AlignEval,
) -> Result<CoveragePhase, IncomparableReason> {
    let (rate_a, rate_b) = rates;
    let (a_view, b_view) = side_views(a, b, basis, offset, rates)?;
    // Rate-conversion residue bound for interval/coverage mapping:
    // each mapped boundary rounds < 0.5 frame; two boundaries bound
    // the interval error (coverage boundaries share the same
    // systematic factor). Documented, not unexplained zero.
    if rate_a != rate_b {
        eval.error_budget = eval.error_budget.saturating_add(2);
    }

    // Hard discontinuities inside either side's window that reach
    // the overlap are incomparable.
    let overlap = a_view
        .interval
        .intersection(&b_view.interval)
        .ok_or(IncomparableReason::CoverageEmpty)?;
    if a_view
        .hard
        .iter()
        .any(|h| h.intersection(&overlap).is_some())
        || b_view
            .hard
            .iter()
            .any(|h| h.intersection(&overlap).is_some())
    {
        return Err(IncomparableReason::DiscontinuityInWindow);
    }

    // Joint valid coverage inside the overlap: a_cov ∩ b_cov.
    let a_cov: Vec<IVal> = a_view
        .valid
        .iter()
        .filter_map(|v| v.intersection(&overlap))
        .collect();
    let b_cov: Vec<IVal> = b_view
        .valid
        .iter()
        .filter_map(|v| v.intersection(&overlap))
        .collect();
    let mut valid_joint = Vec::new();
    for av in &a_cov {
        for bv in &b_cov {
            if let Some(i) = av.intersection(bv) {
                valid_joint.push(i);
            }
        }
    }
    let valid_joint = union(valid_joint);
    if valid_joint.is_empty() {
        return Err(IncomparableReason::CoverageEmpty);
    }
    let excluded = subtract(overlap, &valid_joint);
    if !excluded.is_empty() {
        if !policy.allow_conditional {
            return Err(IncomparableReason::DiscontinuityInWindow);
        }
        eval.conditions.push(AlignmentCondition::PartialCoverage);
    }
    Ok(CoveragePhase { overlap, excluded })
}

/// Maps both spans into common (B-side) coordinates.
/// `SharedCursor`: A-side positions convert rate when needed and
/// shift by `+offset`; B-side is native. `EngineFrame`: each side's
/// cursor positions map through its own anchor.
fn side_views(
    a: &ObservationSpan<'_>,
    b: &ObservationSpan<'_>,
    basis: &Basis,
    offset: i64,
    rates: (u32, u32),
) -> Result<(SideView, SideView), IncomparableReason> {
    let (rate_a, rate_b) = rates;
    let rate_differs = rate_a != rate_b;
    let a_view = match basis {
        Basis::SharedCursor { .. } => {
            let convert = |pos: u64| -> Option<u64> {
                if rate_differs {
                    convert_frames(pos, rate_a, rate_b).ok().map(|(v, _)| v)
                } else {
                    Some(pos)
                }
            };
            side_view_shifted(a, offset, &convert)?
        }
        Basis::Engine { .. } => {
            let ctx_a = a.context.ok_or(IncomparableReason::AnchorMissing)?;
            let base_a = ctx_a
                .frame_base()
                .ok_or(IncomparableReason::AnchorMissing)?;
            let anchor_a = ctx_a
                .anchors
                .iter()
                .find(|an| an.domain == a.clock_mark.domain)
                .ok_or(IncomparableReason::AnchorMissing)?;
            side_view_engine(a, ctx_a, anchor_a, base_a)?
        }
    };
    let b_view = match basis {
        Basis::SharedCursor { .. } => {
            let identity = |pos: u64| Some(pos);
            side_view_shifted(b, 0, &identity)?
        }
        Basis::Engine { .. } => {
            let ctx_b = b.context.ok_or(IncomparableReason::AnchorMissing)?;
            let base_b = ctx_b
                .frame_base()
                .ok_or(IncomparableReason::AnchorMissing)?;
            let anchor_b = ctx_b
                .anchors
                .iter()
                .find(|an| an.domain == b.clock_mark.domain)
                .ok_or(IncomparableReason::AnchorMissing)?;
            side_view_engine(b, ctx_b, anchor_b, base_b)?
        }
    };
    Ok((a_view, b_view))
}

/// `SharedCursor` side view: native cursor positions converted via
/// `convert` (identity for the B side) and shifted by `+offset` into
/// B coordinates.
fn side_view_shifted(
    span: &ObservationSpan<'_>,
    offset: i64,
    convert: &dyn Fn(u64) -> Option<u64>,
) -> Result<SideView, IncomparableReason> {
    let map_pos = |pos: u64| -> Result<i128, IncomparableReason> {
        let conv = convert(pos).ok_or(IncomparableReason::Overflow)?;
        Ok(i128::from(conv) + i128::from(offset))
    };
    build_side_view(span, map_pos)
}

/// `EngineFrame` side view: native cursor positions mapped through
/// the side's anchor into engine-frame coordinates.
fn side_view_engine(
    span: &ObservationSpan<'_>,
    ctx: &ProbeTimeContext,
    anchor: &ClockAnchor,
    base: TimeBase,
) -> Result<SideView, IncomparableReason> {
    let map_pos = |pos: u64| -> Result<i128, IncomparableReason> {
        let stamp = InputStamp {
            ticks: pos,
            base,
            domain: span.clock_mark.domain,
            provenance: ctx.provenance,
        };
        let m = map_stamp_to_frame(&stamp, anchor, base, anchor.frame)
            .map_err(|rejection| IncomparableReason::AnchorRejected { rejection })?;
        Ok(i128::from(m.frame.frame()))
    };
    build_side_view(span, map_pos)
}

/// Shared tail of side-view construction: clips coverage to the
/// span's window, maps positions, and separates valid from hard
/// (discontinuity) evidence.
fn build_side_view(
    span: &ObservationSpan<'_>,
    map_pos: impl Fn(u64) -> Result<i128, IncomparableReason>,
) -> Result<SideView, IncomparableReason> {
    let start = map_pos(span.interval.start)?;
    let end = map_pos(span.interval.end)?;
    let interval = IVal { start, end };

    let mut valid = Vec::new();
    let mut hard = Vec::new();
    let mut discontinuity = false;
    for iv in &span.coverage.intervals {
        let overlaps_span = iv.start < span.interval.end && iv.end > span.interval.start;
        if overlaps_span && iv.mark != span.clock_mark {
            discontinuity = true;
        }
        let clipped_start = iv.start.max(span.interval.start);
        let clipped_end = iv.end.min(span.interval.end);
        if clipped_start >= clipped_end {
            continue;
        }
        let s = map_pos(clipped_start)?;
        let e = map_pos(clipped_end)?;
        let mapped = IVal { start: s, end: e };
        match iv.class {
            CoverageClass::Valid => valid.push(mapped),
            CoverageClass::GenerationBoundary | CoverageClass::RebindDropped if overlaps_span => {
                hard.push(mapped);
            }
            _ => {}
        }
    }
    if discontinuity {
        hard.push(interval);
    }
    Ok(SideView {
        interval,
        valid: union(valid),
        hard: union(hard),
    })
}
