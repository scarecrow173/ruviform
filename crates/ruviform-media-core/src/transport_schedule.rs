//! Generic transport scheduling contract (Issue #190): epoch/generation
//! identity, [`TransportCommand`] → [`AppliedReceipt`] pipeline, bounded
//! half-open logical span scheduling and plugin block-contract legality.
//!
//! The canonical position authority is [`EngineFrame`] inside a
//! [`ClockEpochId`] frame series. All intervals are half-open
//! `[start, end)` — an event at a span's end boundary belongs to the next
//! span exactly once.
//!
//! Layering: the canonical contract and scheduler live in media-core so
//! generic consumers (engines, clock/device, composition, graph) can use
//! them without depending on the plugin extension layer. Plugin wire
//! conversion (`EventList`/`TransportInfo`) lives in `ruviform-plugin-core`.
//! Decisions: `docs/adr/transport-span-scheduler-applied-receipt.md`.
//!
//! ## RT contract
//!
//! [`TransportScheduler::run_callback`] is RT-suitable: allocation-free
//! (the pending/receipt/fault buffers are pre-allocated at construction or
//! `request` time and only pushed within capacity), non-blocking, and
//! every boundary/legality scan is bounded by [`ScheduleBudget`]. All
//! `request`/`preflight`/`install_plan`/`device_restart`/drain methods are
//! non-RT and may allocate. Value types are `Copy` where the type allows.

use std::sync::Arc;

use crate::{MediaRange, MediaTime, TempoMap, TimeBase, TransportStatus};

/// Identity of a device-clock epoch: a frame series restarted on device
/// restart. `EngineFrame`s from different epochs are never equal and must
/// not be subtracted.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClockEpochId(u64);

impl ClockEpochId {
    /// Creates an epoch id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Generation of the transport position lineage: bumped by
/// Play/Pause/Stop/Seek (position-epoch changes). Loop wraps and
/// SetLoop/ClearLoop do not bump it — absolute-position events remain
/// valid across loop changes.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransportGeneration(u64);

impl TransportGeneration {
    /// Creates a generation id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Generation of the installed execution plan (tempo map + scheduled
/// events). Bumped when a new [`TransportPlan`] is applied.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlanGeneration(u64);

impl PlanGeneration {
    /// Creates a generation id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Revision of the [`TempoMap`] basis carried by a plan.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TempoMapRevision(u64);

impl TempoMapRevision {
    /// Creates a revision id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Loop occurrence counter: incremented at each loop wrap. Distinguishes
/// the Nth pass through a loop region — a wrap is neither a clock-epoch
/// discontinuity nor a new transport generation.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LoopOccurrence(u64);

impl LoopOccurrence {
    /// Creates an occurrence id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identifier of a submitted transport command; links a command's
/// `Pending` and `Applied`/`Rejected` receipts.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(u64);

impl CommandId {
    /// The raw id.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Canonical engine frame inside a [`ClockEpochId`]: the position
/// authority from which callback-relative, span-relative and
/// process-relative offsets are derived.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EngineFrame {
    epoch: ClockEpochId,
    frame: u64,
}

impl EngineFrame {
    /// Creates a frame in `epoch`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(epoch: ClockEpochId, frame: u64) -> Self {
        Self { epoch, frame }
    }

    /// The clock epoch this frame belongs to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn epoch(self) -> ClockEpochId {
        self.epoch
    }

    /// The raw frame index inside the epoch.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frame(self) -> u64 {
        self.frame
    }

    /// Whether both frames belong to the same epoch — frames of
    /// different epochs must not be compared or subtracted.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn same_epoch(self, other: Self) -> bool {
        self.epoch.get() == other.epoch.get()
    }

    /// `self - earlier` in frames when both share an epoch and
    /// `self >= earlier`, else `None`. This is the single authority for
    /// frame→offset conversion.
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn checked_offset(self, earlier: Self) -> Option<u64> {
        self.same_epoch(earlier)
            .then(|| self.frame.checked_sub(earlier.frame))
            .flatten()
    }

    /// The frame `frames` later in the same epoch.
    ///
    /// **RT contract:** RT-suitable — saturating pure integer math.
    #[must_use]
    pub fn advance(self, frames: u64) -> Self {
        Self::new(self.epoch, self.frame.saturating_add(frames))
    }
}

/// Generic transport command submitted to the scheduler. `Seek` carries a
/// media-time target; `SetLoop` carries a non-empty half-open region.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportCommand {
    /// Begin or resume position advance.
    Play,
    /// Freeze position.
    Pause,
    /// Stop and reset position to zero.
    Stop,
    /// Jump position.
    Seek(MediaTime),
    /// Activate a loop region.
    SetLoop(MediaRange),
    /// Deactivate the loop region.
    ClearLoop,
}

/// How a discontinuity that cannot be represented as a legal physical
/// call boundary is handled.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeliveryPolicy {
    /// Exact transport: unrepresentable boundaries are rejected — never
    /// silently quantized. Required for offline/exact rendering.
    Exact,
    /// Live-degraded transport: the caller has explicitly opted into
    /// deferring a discontinuity to the next legal boundary within
    /// [`ScheduleBudget::max_deferral`]. The receipt records the actual
    /// applied frame and [`ApplyDegradation`].
    Degraded,
}

/// Lifecycle status of a transport command.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ApplyStatus {
    /// Submitted, not yet staged.
    Requested,
    /// Validated and staged; will be applied at its due frame.
    Pending,
    /// Applied at a frame boundary inside a callback.
    Applied,
    /// Rejected before or instead of application; `reason` is set.
    Rejected,
}

/// Why a command was rejected.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RejectReason {
    /// The command is semantically invalid (negative position, empty
    /// loop, wrong epoch, …).
    InvalidCommand,
    /// A position cannot be represented in the scheduler's frame base.
    PositionUnrepresentable,
    /// The discontinuity cannot be placed on a legal physical-call
    /// boundary under the delivery contract (`Exact` policy).
    UnsupportedExactTransport,
    /// The command/event's generation tag does not match the live
    /// generation (stale).
    StaleGeneration,
    /// A bounded capacity (pending applies, spans, events) is full.
    BudgetExceeded,
    /// A degraded deferral found no legal boundary within
    /// `max_deferral`.
    DeferralExceeded,
}

/// Accuracy loss actually applied under [`DeliveryPolicy::Degraded`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ApplyDegradation {
    /// The discontinuity was applied later than requested.
    DeferredApply {
        /// Requested boundary frame.
        requested: EngineFrame,
        /// Actual applied boundary frame.
        applied: EngineFrame,
    },
}

/// Outcome of a command: the receipt a host observes. Accepted ≠ Applied
/// — the receipt reports the *actual* applied frame, the relevant
/// generations/epoch, an optional degradation and an optional reject
/// reason.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppliedReceipt {
    /// The command this receipt belongs to.
    pub id: CommandId,
    /// Terminal/intermediate lifecycle status.
    pub status: ApplyStatus,
    /// The requested apply frame.
    pub requested_frame: EngineFrame,
    /// The frame the command was actually applied at (`Applied` only).
    pub applied_frame: Option<EngineFrame>,
    /// Transport generation after this receipt was produced.
    pub transport_generation: TransportGeneration,
    /// Plan generation after this receipt was produced.
    pub plan_generation: PlanGeneration,
    /// Clock epoch after this receipt was produced.
    pub clock_epoch: ClockEpochId,
    /// Applied degradation, if any.
    pub degradation: Option<ApplyDegradation>,
    /// Reject reason, if `status == Rejected`.
    pub reason: Option<RejectReason>,
}

/// Classification of a bounded, observable RT fault.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FaultKind {
    /// A command apply boundary could not be represented legally.
    UnrepresentableBoundary,
    /// A loop wrap could not be represented legally (transport keeps
    /// running; the span is marked undeliverable).
    UnrepresentableLoopWrap,
    /// An event whose generation tag mismatched the live span was
    /// skipped.
    StaleEventGeneration,
    /// More span boundaries than `max_spans_per_callback` arose; the
    /// callback tail was emitted as a single best-effort span.
    SpanBudgetExceeded,
    /// More matching events than `max_events_per_span`; the tail was
    /// dropped.
    EventBudgetExceeded,
    /// The fault ledger itself overflowed; further faults increment
    /// `TransportScheduler::fault_overflow` only.
    FaultLedgerOverflow,
}

/// A bounded observable fault from the RT path. RT work never panics or
/// silently succeeds on a contract violation — it records a fault.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScheduleFault {
    /// What went wrong.
    pub kind: FaultKind,
    /// Callback-relative context: the frame at which it happened.
    pub at: EngineFrame,
    /// Kind-specific detail (span length, event count, …).
    pub detail: u64,
}

/// Bounded-work budgets evaluated non-RT and enforced on the RT path.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScheduleBudget {
    /// Maximum logical spans emitted per callback.
    pub max_spans_per_callback: u32,
    /// Maximum events delivered per span.
    pub max_events_per_span: u32,
    /// Maximum staged pending applies.
    pub max_pending_applies: u32,
    /// Maximum deferral distance (frames) for
    /// [`DeliveryPolicy::Degraded`].
    pub max_deferral: u32,
}

impl ScheduleBudget {
    /// Creates a budget.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(
        max_spans_per_callback: u32,
        max_events_per_span: u32,
        max_pending_applies: u32,
        max_deferral: u32,
    ) -> Self {
        Self {
            max_spans_per_callback,
            max_events_per_span,
            max_pending_applies,
            max_deferral,
        }
    }
}

impl Default for ScheduleBudget {
    fn default() -> Self {
        Self::new(64, 256, 16, 4096)
    }
}

/// Whether the target can accept in-block transport updates (e.g. CLAP
/// `CLAP_EVENT_TRANSPORT`). `Unknown`/`Declared` are *not* treated as
/// supported — only `Verified` may enable the in-block path.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InBlockTransportSupport {
    /// Support not established — never treated as supported.
    Unknown,
    /// Declared by the plugin but not yet verified by the adapter —
    /// never treated as supported.
    Declared,
    /// Verified end-to-end by the adapter for this plugin.
    Verified,
}

/// Activated plugin block contract: the physical `process` call must
/// satisfy `min_frames <= frames <= max_frames` (an L1 contract may
/// delegate `min_frames` to a child contract — that is the caller's
/// `min_frames` input, see `transport_bridge` in plugin-core).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockContract {
    /// Minimum legal frames per physical call (0 = unconstrained).
    pub min_frames: u32,
    /// Maximum legal frames per physical call.
    pub max_frames: u32,
    /// Verified in-block transport-update capability.
    pub in_block: InBlockTransportSupport,
}

impl BlockContract {
    /// Creates a contract.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(min_frames: u32, max_frames: u32, in_block: InBlockTransportSupport) -> Self {
        Self {
            min_frames,
            max_frames,
            in_block,
        }
    }

    /// Whether a physical call of exactly `len` frames is legal.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn legal_call_len(&self, len: u32) -> bool {
        len >= self.min_frames && len <= self.max_frames
    }

    /// Whether a span of `len` frames can be delivered as one or more
    /// legal physical calls: either one call (`min <= len <= max`) or an
    /// even split into `ceil(len/max)` chunks each `>= min`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure integer math.
    #[must_use]
    pub const fn legal_span_len(&self, len: u32) -> bool {
        if len == 0 {
            return false;
        }
        if len <= self.max_frames {
            return len >= self.min_frames;
        }
        let chunks = len.div_ceil(self.max_frames);
        len >= chunks * self.min_frames
    }

    /// The legal call partition for `len`, or `None` when
    /// [`Self::legal_span_len`] is false.
    ///
    /// **RT contract:** RT-suitable — `const`, pure integer math.
    #[must_use]
    pub const fn partition(&self, len: u32) -> Option<CallPartition> {
        if !self.legal_span_len(len) {
            return None;
        }
        let chunks = len.div_ceil(self.max_frames);
        Some(CallPartition {
            len,
            chunks,
            base: len / chunks,
            rem: len % chunks,
        })
    }
}

/// An even split of a span into `chunks` physical calls each in
/// `[min_frames, max_frames]`: the first `rem` chunks carry `base + 1`
/// frames, the rest `base`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CallPartition {
    len: u32,
    chunks: u32,
    base: u32,
    rem: u32,
}

impl CallPartition {
    /// Number of physical calls.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn chunks(&self) -> u32 {
        self.chunks
    }

    /// The `(offset, frames)` of chunk `i` — `i < chunks`.
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn chunk(&self, i: u32) -> (u32, u32) {
        debug_assert!(i < self.chunks);
        let offset = if i <= self.rem {
            i * (self.base + 1)
        } else {
            self.rem * (self.base + 1) + (i - self.rem) * self.base
        };
        let frames = self.base + u32::from(i < self.rem);
        debug_assert!(offset + frames <= self.len);
        (offset, frames)
    }
}

/// A plan-scheduled event: canonical absolute `position`, causal
/// `source_sequence` ordering key, generation tags and an opaque payload.
///
/// `transport_generation: Some(tg)` binds the event to one position epoch
/// (e.g. a live-injected one-shot); `None` marks it absolute — it fires
/// whenever the transport position passes `position` under the same plan.
///
/// **RT contract:** RT-suitable — `Copy` when `P` is `Copy`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ScheduledEvent<P> {
    /// Canonical absolute position (converted to scheduler ticks at
    /// plan build).
    pub position: MediaTime,
    /// Explicit causal ordering key for same-position events — stable
    /// source order is preserved, no universal note-off-before-note-on
    /// rule is imposed.
    pub source_sequence: u64,
    /// Plan the event was compiled under.
    pub plan_generation: PlanGeneration,
    /// Optional transport-generation binding (`None` = absolute).
    pub transport_generation: Option<TransportGeneration>,
    /// Opaque payload.
    pub payload: P,
    /// `position` in the plan's frame base, stamped by the builder.
    pub(crate) position_ticks: i64,
}

impl<P> ScheduledEvent<P> {
    /// Creates an absolute-position event.
    ///
    /// **RT contract:** non-RT — construction happens at plan build.
    #[must_use]
    pub const fn new(position: MediaTime, source_sequence: u64, payload: P) -> Self {
        Self {
            position,
            source_sequence,
            plan_generation: PlanGeneration::new(0),
            transport_generation: None,
            payload,
            position_ticks: 0,
        }
    }

    /// Returns a copy bound to transport generation `tg`.
    ///
    /// **RT contract:** non-RT — construction happens at plan build.
    #[must_use]
    pub fn bound_to(self, tg: TransportGeneration) -> Self {
        Self {
            transport_generation: Some(tg),
            ..self
        }
    }
}

/// An immutable compiled plan: tempo map + scheduled events, installed
/// atomically under a new [`PlanGeneration`].
///
/// Built non-RT by [`TransportPlanBuilder`]; the RT path only reads it
/// through `Arc`.
#[derive(Debug)]
pub struct TransportPlan<P> {
    /// The plan's generation tag (matched against event tags).
    pub generation: PlanGeneration,
    /// Tempo-map revision this plan was compiled from.
    pub tempo_revision: TempoMapRevision,
    /// Tempo map used for `musical` transport info (optional).
    pub tempo_map: Option<Arc<TempoMap>>,
    /// Events sorted by `(position_ticks, source_sequence)`.
    pub events: Box<[ScheduledEvent<P>]>,
    /// Time base the event `position_ticks` are expressed in.
    pub frame_base: TimeBase,
}

/// Non-RT builder for [`TransportPlan`]: validates event positions,
/// stamps generation tags and enforces sorted order.
#[derive(Debug)]
pub struct TransportPlanBuilder<P> {
    frame_base: TimeBase,
    generation: PlanGeneration,
    tempo_revision: TempoMapRevision,
    tempo_map: Option<Arc<TempoMap>>,
    events: Vec<ScheduledEvent<P>>,
}

impl<P> TransportPlanBuilder<P> {
    /// Creates a builder for plan `generation` in `frame_base` (one tick
    /// == one engine frame in this `PoC`).
    ///
    /// **RT contract:** non-RT — allocates.
    #[must_use]
    pub fn new(frame_base: TimeBase, generation: PlanGeneration) -> Self {
        Self {
            frame_base,
            generation,
            tempo_revision: TempoMapRevision::new(0),
            tempo_map: None,
            events: Vec::new(),
        }
    }

    /// Attaches a tempo map + revision for `musical` transport info.
    ///
    /// **RT contract:** non-RT.
    #[must_use]
    pub fn with_tempo_map(mut self, map: Arc<TempoMap>, revision: TempoMapRevision) -> Self {
        self.tempo_map = Some(map);
        self.tempo_revision = revision;
        self
    }

    /// Adds an event. Positions must convert exactly to the frame base;
    /// `Err(())` (via [`RejectReason::PositionUnrepresentable`] callers
    /// observe) is returned otherwise.
    ///
    /// **RT contract:** non-RT — allocates.
    ///
    /// # Errors
    ///
    /// [`crate::Error::TimeOverflow`] when the position cannot be
    /// represented in the plan frame base or overflows `i64` ticks.
    pub fn event(
        &mut self,
        position: MediaTime,
        source_sequence: u64,
        payload: P,
    ) -> crate::Result<&mut Self> {
        let ticks =
            media_time_ticks(position, self.frame_base).ok_or(crate::Error::TimeOverflow)?;
        self.events.push(ScheduledEvent {
            position_ticks: ticks,
            ..ScheduledEvent::new(position, source_sequence, payload)
        });
        Ok(self)
    }

    /// Adds a generation-bound event.
    ///
    /// **RT contract:** non-RT — allocates.
    ///
    /// # Errors
    ///
    /// Same as [`Self::event`].
    pub fn bound_event(
        &mut self,
        position: MediaTime,
        source_sequence: u64,
        tg: TransportGeneration,
        payload: P,
    ) -> crate::Result<&mut Self> {
        let ticks =
            media_time_ticks(position, self.frame_base).ok_or(crate::Error::TimeOverflow)?;
        self.events.push(
            ScheduledEvent::new(position, source_sequence, payload)
                .bound_to(tg)
                .with_ticks(ticks),
        );
        Ok(self)
    }

    /// Sorts events by `(position, source_sequence)`, stamps the plan
    /// generation and freezes the plan.
    ///
    /// **RT contract:** non-RT — allocates.
    #[must_use]
    pub fn build(mut self) -> TransportPlan<P> {
        self.events
            .sort_by_key(|e| (e.position_ticks, e.source_sequence));
        for e in &mut self.events {
            e.plan_generation = self.generation;
        }
        TransportPlan {
            generation: self.generation,
            tempo_revision: self.tempo_revision,
            tempo_map: self.tempo_map,
            events: self.events.into_boxed_slice(),
            frame_base: self.frame_base,
        }
    }
}

impl<P> ScheduledEvent<P> {
    pub(crate) const fn with_ticks(mut self, ticks: i64) -> Self {
        self.position_ticks = ticks;
        self
    }
}

/// Transport context carried by one logical span. All values are the
/// context *at the span start*; during `Playing`, position advances by
/// one frame-base tick per frame.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpanContext {
    /// Playback status for the span.
    pub status: TransportStatus,
    /// Position at the span start, in `position_base` ticks.
    pub position_ticks: i64,
    /// Base of `position_ticks` (one tick == one frame in this `PoC`).
    pub position_base: TimeBase,
    /// Active loop region `(start, end)` in `position_base` ticks.
    pub loop_ticks: Option<(i64, i64)>,
    /// Which pass through the loop this span belongs to.
    pub occurrence: LoopOccurrence,
    /// Clock epoch of the span's frame series.
    pub clock_epoch: ClockEpochId,
    /// Live transport generation for event tag matching.
    pub transport_generation: TransportGeneration,
    /// Live plan generation for event tag matching.
    pub plan_generation: PlanGeneration,
    /// Tempo-map revision the plan was compiled from.
    pub tempo_revision: TempoMapRevision,
}

impl SpanContext {
    /// Position as a [`MediaTime`] `offset` frames after the span start
    /// (position advances only while `Playing`).
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn position_at_offset(&self, offset: u32) -> MediaTime {
        let advance = i64::from(offset) * i64::from(self.status == TransportStatus::Playing);
        MediaTime::new(self.position_ticks + advance, self.position_base)
    }

    /// Position at the span start.
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn position(&self) -> MediaTime {
        MediaTime::new(self.position_ticks, self.position_base)
    }

    /// The active loop region as a [`MediaRange`].
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn loop_range(&self) -> Option<MediaRange> {
        self.loop_ticks.map(|(s, e)| {
            MediaRange::new(
                MediaTime::new(s, self.position_base),
                crate::MediaDuration::new(e - s, self.position_base),
            )
        })
    }
}

/// One half-open logical transport span `[start, end)` inside a
/// callback. `deliverable == false` means no legal physical-call
/// partition exists under the delivery contract — the span is still
/// emitted (time keeps running) with a recorded fault; the sink must not
/// deliver it.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TransportSpan {
    /// Span start frame.
    pub start: EngineFrame,
    /// Span end frame (exclusive).
    pub end: EngineFrame,
    /// Transport context for the whole span.
    pub ctx: SpanContext,
    /// Whether a legal physical-call partition exists.
    pub deliverable: bool,
}

impl TransportSpan {
    /// Span length in frames.
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn frames(&self) -> u32 {
        u32::try_from(self.end.frame.saturating_sub(self.start.frame)).unwrap_or(u32::MAX)
    }
}

/// The single authority mapping a scheduled event to its engine frame
/// inside a span: `Some(frame)` when the event's generation tags match
/// the span's live generations and its position falls inside the span's
/// `[pos_start, pos_end)` range; `None` = stale or out of range (do not
/// deliver).
///
/// **RT contract:** RT-suitable — pure integer math.
#[must_use]
pub fn event_engine_frame<P>(
    span: &TransportSpan,
    event: &ScheduledEvent<P>,
) -> Option<EngineFrame> {
    if event.plan_generation != span.ctx.plan_generation {
        return None;
    }
    if let Some(tg) = event.transport_generation
        && tg != span.ctx.transport_generation
    {
        return None;
    }
    if span.ctx.status != TransportStatus::Playing {
        return None;
    }
    let offset = event.position_ticks.checked_sub(span.ctx.position_ticks)?;
    if offset < 0 || offset >= i64::from(span.frames()) {
        return None;
    }
    Some(span.start.advance(u64::try_from(offset).ok()?))
}

/// The sub-slice of `plan.events` whose absolute positions fall inside
/// the span's `[pos_start, pos_start + frames)` position range, in
/// stable `(position, source_sequence)` order. Empty when the span is
/// not `Playing` (position does not advance, so nothing fires).
///
/// Because positions are absolute and the scheduler re-walks the plan
/// per span, the same in-loop event matches exactly once per loop
/// occurrence — never duplicated inside one occurrence, never dropped.
///
/// **RT contract:** RT-suitable — binary search, no allocation.
#[must_use]
pub fn span_event_window<'a, P>(
    plan: &'a TransportPlan<P>,
    span: &TransportSpan,
) -> &'a [ScheduledEvent<P>] {
    if span.ctx.status != TransportStatus::Playing {
        return &[];
    }
    let lo = span.ctx.position_ticks;
    let hi = lo.saturating_add(i64::from(span.frames()));
    let start = plan.events.partition_point(|e| e.position_ticks < lo);
    let end = plan.events.partition_point(|e| e.position_ticks < hi);
    &plan.events[start..end.max(start)]
}

/// Sink receiving each emitted logical span plus its scheduled events
/// (the position-windowed slice of the active plan; per-event
/// deliverability is decided by [`event_engine_frame`]).
pub trait SpanSink<P> {
    /// Called once per emitted span, in frame order.
    ///
    /// **RT contract:** RT-suitable — the implementation must not
    /// allocate, block or do I/O.
    fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<P>]);
}

/// Command payload after validation, with positions converted to
/// frame-base ticks.
#[derive(Copy, Clone, Debug)]
enum Validated {
    Simple,
    ClearLoop,
    Seek(i64),
    Loop(i64, i64),
}

#[derive(Debug)]
enum PendingKind<P> {
    Command(TransportCommand),
    InstallPlan(Arc<TransportPlan<P>>),
}

#[derive(Debug)]
struct PendingApply<P> {
    id: CommandId,
    due: u64,
    kind: PendingKind<P>,
    policy: DeliveryPolicy,
}

/// Generic transport scheduler (`PoC`, Issue #190): a non-RT request side
/// ([`Self::request`], [`Self::preflight`], [`Self::install_plan`],
/// [`Self::device_restart`], drain methods) and the RT
/// [`Self::run_callback`] that splits each callback into bounded half-open
/// logical spans at transport-context discontinuities.
///
/// One scheduler instance models one downstream delivery contract; a host
/// with several plugins evaluates each plugin's [`BlockContract`]
/// independently (per-plugin delivery is issue #196's adapter scope).
#[derive(Debug)]
pub struct TransportScheduler<P> {
    frame_base: TimeBase,
    contract: BlockContract,
    policy: DeliveryPolicy,
    budget: ScheduleBudget,
    epoch: ClockEpochId,
    tg: TransportGeneration,
    pg: PlanGeneration,
    tempo_rev: TempoMapRevision,
    status: TransportStatus,
    anchor_frame: u64,
    anchor_pos: i64,
    loop_ticks: Option<(i64, i64)>,
    occurrence: LoopOccurrence,
    plan: Option<Arc<TransportPlan<P>>>,
    pending: Vec<PendingApply<P>>,
    receipts: Vec<AppliedReceipt>,
    faults: Vec<ScheduleFault>,
    /// Faults dropped because the ledger was full.
    pub fault_overflow: u64,
    next_cmd: u64,
    next_pg: u64,
}

impl<P> TransportScheduler<P> {
    /// Creates a scheduler. `frame_base` is the media-time base where one
    /// tick equals one engine frame (`PoC`: no varispeed).
    ///
    /// **RT contract:** non-RT — allocates the bounded ledgers.
    #[must_use]
    pub fn new(
        frame_base: TimeBase,
        contract: BlockContract,
        policy: DeliveryPolicy,
        budget: ScheduleBudget,
    ) -> Self {
        let cap = (budget.max_pending_applies as usize) * 3 + 8;
        Self {
            frame_base,
            contract,
            policy,
            budget,
            epoch: ClockEpochId::new(0),
            tg: TransportGeneration::new(0),
            pg: PlanGeneration::new(0),
            tempo_rev: TempoMapRevision::new(0),
            status: TransportStatus::Stopped,
            anchor_frame: 0,
            anchor_pos: 0,
            loop_ticks: None,
            occurrence: LoopOccurrence::new(0),
            plan: None,
            pending: Vec::with_capacity(budget.max_pending_applies as usize),
            receipts: Vec::with_capacity(cap),
            faults: Vec::with_capacity(64),
            fault_overflow: 0,
            next_cmd: 1,
            next_pg: 1,
        }
    }

    /// The active clock epoch.
    #[must_use]
    pub const fn epoch(&self) -> ClockEpochId {
        self.epoch
    }

    /// Live transport generation.
    #[must_use]
    pub const fn transport_generation(&self) -> TransportGeneration {
        self.tg
    }

    /// Live plan generation.
    #[must_use]
    pub const fn plan_generation(&self) -> PlanGeneration {
        self.pg
    }

    /// Live tempo-map revision.
    #[must_use]
    pub const fn tempo_revision(&self) -> TempoMapRevision {
        self.tempo_rev
    }

    /// Current status.
    #[must_use]
    pub const fn status(&self) -> TransportStatus {
        self.status
    }

    /// Current loop occurrence.
    #[must_use]
    pub const fn occurrence(&self) -> LoopOccurrence {
        self.occurrence
    }

    /// The delivery contract.
    #[must_use]
    pub const fn contract(&self) -> BlockContract {
        self.contract
    }

    /// The frame base.
    #[must_use]
    pub const fn frame_base(&self) -> TimeBase {
        self.frame_base
    }

    /// Mints the next plan generation for a [`TransportPlanBuilder`].
    ///
    /// **RT contract:** non-RT.
    #[must_use]
    pub fn next_plan_generation(&self) -> PlanGeneration {
        PlanGeneration::new(self.next_pg)
    }

    /// A [`TransportPlanBuilder`] bound to this scheduler's frame base and
    /// next plan generation.
    ///
    /// **RT contract:** non-RT.
    #[must_use]
    pub fn plan_builder(&self) -> TransportPlanBuilder<P> {
        TransportPlanBuilder::new(self.frame_base, self.next_plan_generation())
    }

    /// Converts a media time to frame ticks; `None` when not exactly
    /// representable.
    ///
    /// **RT contract:** RT-suitable — pure integer math.
    #[must_use]
    pub fn ticks(&self, t: MediaTime) -> Option<i64> {
        media_time_ticks(t, self.frame_base)
    }

    /// Position (in frame ticks) at absolute `frame` under the current
    /// anchor.
    fn pos_at(&self, frame: u64) -> i64 {
        let d = i64::try_from(frame.saturating_sub(self.anchor_frame)).unwrap_or(i64::MAX);
        if self.status == TransportStatus::Playing {
            self.anchor_pos.saturating_add(d)
        } else {
            self.anchor_pos
        }
    }

    /// The frame at which the playing position next reaches the loop end
    /// (`> after` required — a wrap due at or before `after` is handled
    /// by the caller's start-of-span logic).
    fn next_wrap_after(&self, after: u64) -> Option<u64> {
        if self.status != TransportStatus::Playing {
            return None;
        }
        let (_, le) = self.loop_ticks?;
        let pos = self.pos_at(after);
        let delta = le.saturating_sub(pos);
        Some(after.saturating_add(u64::try_from(delta.max(0)).unwrap_or(u64::MAX)))
    }

    fn context_at(&self, frame: u64) -> SpanContext {
        SpanContext {
            status: self.status,
            position_ticks: self.pos_at(frame),
            position_base: self.frame_base,
            loop_ticks: self.loop_ticks,
            occurrence: self.occurrence,
            clock_epoch: self.epoch,
            transport_generation: self.tg,
            plan_generation: self.pg,
            tempo_revision: self.tempo_rev,
        }
    }

    fn push_receipt(&mut self, r: AppliedReceipt) {
        if self.receipts.len() < self.receipts.capacity() {
            self.receipts.push(r);
        } else {
            self.fault_overflow = self.fault_overflow.saturating_add(1);
        }
    }

    fn push_fault(&mut self, kind: FaultKind, at: EngineFrame, detail: u64) {
        if self.faults.len() < self.faults.capacity() {
            self.faults.push(ScheduleFault { kind, at, detail });
        } else {
            self.fault_overflow = self.fault_overflow.saturating_add(1);
        }
    }

    fn receipt(
        &self,
        id: CommandId,
        status: ApplyStatus,
        requested: EngineFrame,
    ) -> AppliedReceipt {
        AppliedReceipt {
            id,
            status,
            requested_frame: requested,
            applied_frame: None,
            transport_generation: self.tg,
            plan_generation: self.pg,
            clock_epoch: self.epoch,
            degradation: None,
            reason: None,
        }
    }

    fn reject(&mut self, id: CommandId, requested: EngineFrame, reason: RejectReason) {
        let mut r = self.receipt(id, ApplyStatus::Rejected, requested);
        r.reason = Some(reason);
        self.push_receipt(r);
    }

    /// Validates a command's shape and converts media positions to ticks.
    fn validate(&self, cmd: &TransportCommand) -> Result<Validated, RejectReason> {
        match *cmd {
            TransportCommand::Play | TransportCommand::Pause | TransportCommand::Stop => {
                Ok(Validated::Simple)
            }
            TransportCommand::ClearLoop => Ok(Validated::ClearLoop),
            TransportCommand::Seek(pos) => {
                let t = self
                    .ticks(pos)
                    .ok_or(RejectReason::PositionUnrepresentable)?;
                if t < 0 {
                    return Err(RejectReason::InvalidCommand);
                }
                Ok(Validated::Seek(t))
            }
            TransportCommand::SetLoop(range) => {
                let s = self
                    .ticks(range.start())
                    .ok_or(RejectReason::PositionUnrepresentable)?;
                let e = self
                    .ticks(range.end())
                    .ok_or(RejectReason::PositionUnrepresentable)?;
                if s < 0 || e <= s {
                    return Err(RejectReason::InvalidCommand);
                }
                Ok(Validated::Loop(s, e))
            }
        }
    }

    /// Stages a command for RT application at `due` — non-RT
    /// validate/prepare. The command enters `Pending`; the apply itself
    /// happens inside [`Self::run_callback`] (accepted ≠ applied). For a
    /// legality check against a declared callback window first, call
    /// [`Self::preflight`].
    ///
    /// **RT contract:** non-RT — may allocate on first growth of the
    /// pending buffer; capacity is bounded by
    /// `budget.max_pending_applies`.
    ///
    /// # Errors
    ///
    /// Returns a `Rejected` [`AppliedReceipt`] when validation or the
    /// pending budget fails (the receipt is also recorded).
    pub fn request(
        &mut self,
        cmd: TransportCommand,
        due: EngineFrame,
    ) -> Result<CommandId, AppliedReceipt> {
        self.stage(PendingKind::Command(cmd), due, self.policy)
    }

    fn stage(
        &mut self,
        kind: PendingKind<P>,
        due: EngineFrame,
        policy: DeliveryPolicy,
    ) -> Result<CommandId, AppliedReceipt> {
        let id = CommandId(self.next_cmd);
        if due.epoch() != self.epoch {
            let mut r = self.receipt(id, ApplyStatus::Rejected, due);
            r.reason = Some(RejectReason::StaleGeneration);
            self.push_receipt(r);
            return Err(r);
        }
        if let PendingKind::Command(cmd) = &kind
            && let Err(reason) = self.validate(cmd)
        {
            let mut r = self.receipt(id, ApplyStatus::Rejected, due);
            r.reason = Some(reason);
            self.push_receipt(r);
            return Err(r);
        }
        if self.pending.len() >= self.budget.max_pending_applies as usize {
            let mut r = self.receipt(id, ApplyStatus::Rejected, due);
            r.reason = Some(RejectReason::BudgetExceeded);
            self.push_receipt(r);
            return Err(r);
        }
        self.next_cmd += 1;
        self.push_receipt(self.receipt(id, ApplyStatus::Requested, due));
        self.push_receipt(self.receipt(id, ApplyStatus::Pending, due));
        let idx = self
            .pending
            .iter()
            .position(|p| p.due > due.frame())
            .unwrap_or(self.pending.len());
        self.pending.insert(
            idx,
            PendingApply {
                id,
                due: due.frame(),
                kind,
                policy,
            },
        );
        Ok(id)
    }

    /// Installs a compiled plan at `due`: applies as a
    /// [`PlanGeneration`]/[`TempoMapRevision`] boundary inside a callback.
    ///
    /// **RT contract:** non-RT — allocates.
    ///
    /// # Errors
    ///
    /// Same contract as [`Self::request`].
    pub fn install_plan(
        &mut self,
        plan: Arc<TransportPlan<P>>,
        due: EngineFrame,
    ) -> Result<CommandId, AppliedReceipt> {
        self.stage(PendingKind::InstallPlan(plan), due, self.policy)
    }

    /// Non-RT preflight of a command apply at `due` inside the declared
    /// callback window `[window_start, +frames)`: simulates the pending
    /// set plus this command's discontinuity and post-command loop wraps.
    ///
    /// Returns `Ok(applied_frame)` — `due` under `Exact`, or the legal
    /// boundary it would defer to under `Degraded` — and
    /// `Err(RejectReason)` when the discontinuity cannot be represented.
    ///
    /// **RT contract:** non-RT — allocates.
    ///
    /// # Errors
    ///
    /// [`RejectReason::UnsupportedExactTransport`] (Exact policy),
    /// [`RejectReason::DeferralExceeded`] (Degraded with no legal
    /// boundary inside `max_deferral`) or the validation reason.
    #[allow(clippy::too_many_lines)]
    pub fn preflight(
        &self,
        cmd: TransportCommand,
        due: EngineFrame,
        window_start: EngineFrame,
        frames: u32,
    ) -> Result<EngineFrame, RejectReason> {
        self.validate(&cmd)?;
        if due.epoch() != self.epoch || window_start.epoch() != self.epoch {
            return Err(RejectReason::StaleGeneration);
        }
        let ws = window_start.frame;
        let we = ws + u64::from(frames);
        // Simulate the window: boundaries are pending dues (applying
        // their commands to the sim state), the candidate `due`, and
        // loop wraps — all walked in frame order on a scalar state copy.
        let mut sim = SimState {
            status: self.status,
            anchor_frame: self.anchor_frame,
            anchor_pos: self.anchor_pos,
            loop_ticks: self.loop_ticks,
        };
        // Applies due at or before the window start fire first.
        for p in &self.pending {
            if p.due <= ws
                && let PendingKind::Command(c) = &p.kind
            {
                sim.apply_command(c, p.due, self);
            }
        }
        let mut bounds: Vec<(u64, Option<&TransportCommand>)> = Vec::new();
        for p in &self.pending {
            if p.due > ws && p.due < we {
                let c = match &p.kind {
                    PendingKind::Command(c) => Some(c),
                    PendingKind::InstallPlan(_) => None,
                };
                bounds.push((p.due, c));
            }
        }
        let due_in_window = due.frame() > ws && due.frame() < we;
        if due_in_window {
            bounds.push((due.frame(), Some(&cmd)));
        }
        bounds.sort_by_key(|b| b.0);
        let mut applied = due;
        let mut bi = 0usize;
        let mut prev = ws;
        loop {
            let mut b = we;
            let mut wrap = false;
            if bi < bounds.len() && bounds[bi].0 < b {
                b = bounds[bi].0;
            }
            if let Some(w) = sim.next_wrap(prev)
                && w < b
            {
                b = w;
                wrap = true;
            }
            if b >= we {
                break;
            }
            if b <= prev {
                if wrap {
                    sim.wrap(b);
                }
                if bi < bounds.len() && bounds[bi].0 <= prev {
                    if let Some(c) = bounds[bi].1 {
                        sim.apply_command(c, b, self);
                    }
                    bi += 1;
                }
                continue;
            }
            let len = u32::try_from(b - prev).unwrap_or(u32::MAX);
            let is_due = due_in_window && b == due.frame();
            if !self.contract.legal_span_len(len) {
                match self.policy {
                    DeliveryPolicy::Exact => {
                        return Err(RejectReason::UnsupportedExactTransport);
                    }
                    DeliveryPolicy::Degraded => match self.find_legal_boundary(prev, b, we) {
                        Some(nb) => {
                            prev = nb;
                            if wrap {
                                sim.wrap(nb);
                            } else {
                                while bi < bounds.len() && bounds[bi].0 == b {
                                    if let Some(c) = bounds[bi].1 {
                                        sim.apply_command(c, nb, self);
                                    }
                                    bi += 1;
                                }
                                if is_due {
                                    applied = EngineFrame::new(self.epoch, nb);
                                }
                            }
                            continue;
                        }
                        None => return Err(RejectReason::DeferralExceeded),
                    },
                }
            }
            prev = b;
            if wrap {
                sim.wrap(b);
            }
            while bi < bounds.len() && bounds[bi].0 <= b {
                if let Some(c) = bounds[bi].1 {
                    sim.apply_command(c, b, self);
                }
                bi += 1;
            }
        }
        let tail = u32::try_from(we - prev).unwrap_or(u32::MAX);
        if !self.contract.legal_span_len(tail) {
            return Err(match self.policy {
                DeliveryPolicy::Exact => RejectReason::UnsupportedExactTransport,
                DeliveryPolicy::Degraded => RejectReason::DeferralExceeded,
            });
        }
        Ok(applied)
    }

    /// Finds the next boundary `> prev` at which both adjacent lengths
    /// are legal, scanning at most `max_deferral` frames from `b`.
    fn find_legal_boundary(&self, prev: u64, b: u64, we: u64) -> Option<u64> {
        let limit = (b + u64::from(self.budget.max_deferral)).min(we);
        let mut cand = b;
        while cand < limit {
            let left = u32::try_from(cand - prev).unwrap_or(u32::MAX);
            let right = u32::try_from(we - cand).unwrap_or(u32::MAX);
            if self.contract.legal_span_len(left) && self.contract.legal_span_len(right) {
                return Some(cand);
            }
            cand += 1;
        }
        None
    }

    /// Device restart: bumps the clock epoch, resets the frame base and
    /// rejects every pending apply as [`RejectReason::StaleGeneration`].
    /// Distinct from a seek (which bumps transport generation inside the
    /// same epoch).
    ///
    /// **RT contract:** non-RT.
    pub fn device_restart(&mut self, base_frame: u64) -> ClockEpochId {
        self.epoch = ClockEpochId::new(self.epoch.get() + 1);
        let at = EngineFrame::new(self.epoch, base_frame);
        let drained: Vec<PendingApply<P>> = std::mem::take(&mut self.pending);
        for p in drained {
            self.reject(
                p.id,
                EngineFrame::new(self.epoch, p.due),
                RejectReason::StaleGeneration,
            );
        }
        self.status = TransportStatus::Stopped;
        self.anchor_frame = base_frame;
        self.anchor_pos = 0;
        self.occurrence = LoopOccurrence::new(0);
        let _ = at;
        self.epoch
    }

    /// Drains all recorded receipts (Requested/Pending/Applied/Rejected
    /// lifecycle entries, in order).
    ///
    /// **RT contract:** non-RT — allocates the returned vec.
    pub fn drain_receipts(&mut self) -> Vec<AppliedReceipt> {
        std::mem::take(&mut self.receipts)
    }

    /// Drains all recorded faults.
    ///
    /// **RT contract:** non-RT — allocates the returned vec.
    pub fn drain_faults(&mut self) -> Vec<ScheduleFault> {
        std::mem::take(&mut self.faults)
    }

    /// Removes and returns the oldest recorded receipt, or `None`.
    ///
    /// **RT contract:** RT-suitable — `receipts` has a fixed capacity
    /// derived from [`ScheduleBudget::max_pending_applies`]; a
    /// `remove(0)` shift stays within that bound and performs no
    /// allocation.
    pub fn pop_receipt(&mut self) -> Option<AppliedReceipt> {
        if self.receipts.is_empty() {
            None
        } else {
            Some(self.receipts.remove(0))
        }
    }

    /// Removes and returns the oldest recorded fault, or `None`.
    ///
    /// **RT contract:** RT-suitable — `faults` has a fixed capacity of
    /// 64; a `remove(0)` shift stays within that bound and performs no
    /// allocation.
    pub fn pop_fault(&mut self) -> Option<ScheduleFault> {
        if self.faults.is_empty() {
            None
        } else {
            Some(self.faults.remove(0))
        }
    }

    fn apply(&mut self, p: PendingApply<P>, frame: u64, degradation: Option<ApplyDegradation>) {
        match p.kind {
            PendingKind::Command(cmd) => {
                let pos = self.pos_at(frame);
                let valid = self.validate(&cmd);
                match (cmd, valid) {
                    (TransportCommand::Play, _) => {
                        self.status = TransportStatus::Playing;
                        self.anchor_frame = frame;
                        self.anchor_pos = pos;
                        self.tg = TransportGeneration::new(self.tg.get() + 1);
                    }
                    (TransportCommand::Pause, _) => {
                        self.status = TransportStatus::Paused;
                        self.anchor_frame = frame;
                        self.anchor_pos = pos;
                        self.tg = TransportGeneration::new(self.tg.get() + 1);
                    }
                    (TransportCommand::Stop, _) => {
                        self.status = TransportStatus::Stopped;
                        self.anchor_frame = frame;
                        self.anchor_pos = 0;
                        self.tg = TransportGeneration::new(self.tg.get() + 1);
                    }
                    (TransportCommand::Seek(_), Ok(Validated::Seek(t))) => {
                        self.anchor_frame = frame;
                        self.anchor_pos = t;
                        self.tg = TransportGeneration::new(self.tg.get() + 1);
                    }
                    (TransportCommand::SetLoop(_), Ok(Validated::Loop(s, e))) => {
                        self.loop_ticks = Some((s, e));
                        self.occurrence = LoopOccurrence::new(0);
                    }
                    (TransportCommand::ClearLoop, _) => {
                        self.loop_ticks = None;
                    }
                    _ => {
                        self.reject(
                            p.id,
                            EngineFrame::new(self.epoch, p.due),
                            RejectReason::InvalidCommand,
                        );
                        return;
                    }
                }
            }
            PendingKind::InstallPlan(plan) => {
                self.pg = plan.generation;
                self.tempo_rev = plan.tempo_revision;
                self.plan = Some(plan);
                self.next_pg = self.next_pg.max(self.pg.get() + 1);
            }
        }
        let mut r = self.receipt(
            p.id,
            ApplyStatus::Applied,
            EngineFrame::new(self.epoch, p.due),
        );
        r.applied_frame = Some(EngineFrame::new(self.epoch, frame));
        r.degradation = degradation;
        self.push_receipt(r);
    }

    /// Runs one callback `[start, +frames)` in the scheduler's epoch:
    /// applies due commands/installs at their boundary frames, splits the
    /// window into half-open logical spans at discontinuities (applies,
    /// loop wraps, plan-generation changes) and calls `sink` per span.
    ///
    /// **RT contract:** RT-suitable — allocation-free, bounded by
    /// `budget`: boundary scans are `O(pending + wraps)` with
    /// `max_spans_per_callback` as the hard cap, deferral scans are
    /// capped by `max_deferral`, event windows by `max_events_per_span`.
    /// Contract violations record bounded [`ScheduleFault`]s and produce
    /// `Rejected` receipts or undeliverable spans — never unbounded work
    /// or silent success.
    pub fn run_callback<S: SpanSink<P>>(&mut self, start: EngineFrame, frames: u32, sink: &mut S) {
        if start.epoch() != self.epoch {
            self.push_fault(FaultKind::UnrepresentableBoundary, start, 0);
            return;
        }
        let end = start.frame + u64::from(frames);
        // Apply everything due at or before the callback start.
        while self.pending.first().is_some_and(|p| p.due <= start.frame) {
            let p = self.pending.remove(0);
            self.apply(p, start.frame, None);
        }
        let mut prev = start.frame;
        let mut spans: u32 = 0;
        loop {
            // Next boundary: earliest pending apply or loop wrap. On a
            // tie the apply wins; the wrap is re-evaluated under the new
            // state on the next iteration.
            let mut b = end;
            let mut wrap = false;
            if let Some(p) = self.pending.first()
                && p.due < b
            {
                b = p.due;
            }
            if let Some(w) = self.next_wrap_after(prev)
                && w < b
            {
                b = w;
                wrap = true;
            }
            if b >= end {
                break;
            }
            if b <= prev {
                // Boundary at the span start: applies due now plus a
                // wrap due now take effect without emitting a span.
                while self.pending.first().is_some_and(|p| p.due <= prev) {
                    let p = self.pending.remove(0);
                    self.apply(p, prev, None);
                }
                if wrap {
                    self.do_wrap(prev);
                }
                if spans >= self.budget.max_spans_per_callback {
                    self.push_fault(
                        FaultKind::SpanBudgetExceeded,
                        EngineFrame::new(self.epoch, prev),
                        u64::from(spans),
                    );
                    break;
                }
                continue;
            }
            let len32 = u32::try_from(b - prev).unwrap_or(u32::MAX);
            if self.contract.legal_span_len(len32) {
                self.emit(prev, b, true, sink);
                prev = b;
                if wrap {
                    self.do_wrap(b);
                } else {
                    while self.pending.first().is_some_and(|p| p.due == b) {
                        let p = self.pending.remove(0);
                        self.apply(p, b, None);
                    }
                }
            } else {
                prev = self.resolve_illegal_boundary(prev, b, wrap, end, sink);
            }
            spans += 1;
            if spans >= self.budget.max_spans_per_callback {
                self.push_fault(
                    FaultKind::SpanBudgetExceeded,
                    EngineFrame::new(self.epoch, prev),
                    u64::from(spans),
                );
                // Emit the remainder as one span and stop splitting.
                if prev < end {
                    self.emit(
                        prev,
                        end,
                        self.contract
                            .legal_span_len(u32::try_from(end - prev).unwrap_or(u32::MAX)),
                        sink,
                    );
                }
                // Drop no state: applies/wraps beyond the cap still occur
                // next callback via `pending`/wrap computation.
                return;
            }
        }
        if prev < end {
            self.emit(
                prev,
                end,
                self.contract
                    .legal_span_len(u32::try_from(end - prev).unwrap_or(u32::MAX)),
                sink,
            );
        }
    }

    fn effective_policy_at(&self, b: u64) -> DeliveryPolicy {
        self.pending
            .iter()
            .find(|p| p.due == b)
            .map_or(self.policy, |p| p.policy)
    }

    /// Handles a boundary `b` whose closing span `[prev, b)` has no legal
    /// physical-call partition. Returns the new `prev` for the next
    /// boundary scan.
    ///
    /// - `Exact` + wrap: time cannot stop — emit the span undeliverable
    ///   with `UnrepresentableLoopWrap` and wrap anyway.
    /// - `Exact` + apply: keep last-good state, reject the applies at
    ///   `b` (`UnsupportedExactTransport`), drop the boundary.
    /// - `Degraded`: defer the boundary to the next legal frame within
    ///   `max_deferral` (bounded scan); applies record
    ///   [`ApplyDegradation::DeferredApply`]. No legal frame → wrap is
    ///   emitted undeliverable / applies rejected (`DeferralExceeded`).
    fn resolve_illegal_boundary<S: SpanSink<P>>(
        &mut self,
        prev: u64,
        b: u64,
        wrap: bool,
        end: u64,
        sink: &mut S,
    ) -> u64 {
        let len32 = u32::try_from(b - prev).unwrap_or(u32::MAX);
        let at = EngineFrame::new(self.epoch, b);
        if self.effective_policy_at(b) == DeliveryPolicy::Degraded {
            let next_bound = self
                .pending
                .iter()
                .map(|p| p.due)
                .filter(|d| *d > b)
                .min()
                .unwrap_or(end)
                .min(end);
            if let Some(nb) = self.find_legal_boundary(prev, b, next_bound) {
                self.emit(prev, nb, true, sink);
                if wrap {
                    self.do_wrap(nb);
                } else {
                    let applied = EngineFrame::new(self.epoch, nb);
                    let deg = ApplyDegradation::DeferredApply {
                        requested: at,
                        applied,
                    };
                    while self.pending.first().is_some_and(|p| p.due == b) {
                        let p = self.pending.remove(0);
                        self.apply(p, nb, Some(deg));
                    }
                }
                return nb;
            }
        }
        let reason = if self.effective_policy_at(b) == DeliveryPolicy::Degraded {
            RejectReason::DeferralExceeded
        } else {
            RejectReason::UnsupportedExactTransport
        };
        if wrap {
            self.push_fault(FaultKind::UnrepresentableLoopWrap, at, u64::from(len32));
            self.emit(prev, b, false, sink);
            self.do_wrap(b);
            b
        } else {
            // Reject all applies at b; keep last-good state (boundary
            // dropped — the span continues under the current context).
            self.push_fault(FaultKind::UnrepresentableBoundary, at, u64::from(len32));
            while self.pending.first().is_some_and(|p| p.due == b) {
                let p = self.pending.remove(0);
                self.reject(p.id, at, reason);
            }
            prev
        }
    }

    fn do_wrap(&mut self, frame: u64) {
        if let Some((ls, _)) = self.loop_ticks {
            let pos = self.pos_at(frame);
            // Degraded deferral: position may have passed the loop end;
            // the post-wrap anchor keeps the overflow so position stays
            // continuous.
            let overflow = self
                .loop_ticks
                .map_or(0, |(_, le)| pos.saturating_sub(le).max(0));
            self.anchor_frame = frame;
            self.anchor_pos = ls.saturating_add(overflow);
            self.occurrence = LoopOccurrence::new(self.occurrence.get() + 1);
        }
    }

    fn emit<S: SpanSink<P>>(&mut self, from: u64, to: u64, deliverable: bool, sink: &mut S) {
        if to <= from {
            return;
        }
        let span = TransportSpan {
            start: EngineFrame::new(self.epoch, from),
            end: EngineFrame::new(self.epoch, to),
            ctx: self.context_at(from),
            deliverable,
        };
        let plan = self.plan.clone();
        let events: &[ScheduledEvent<P>] = plan
            .as_ref()
            .map_or(&[], |p| span_event_window(p.as_ref(), &span));
        // Bounded observable faults: stale generation tags and the
        // per-span event budget. The sink receives the position-matched
        // window; per-event deliverability is `event_engine_frame`.
        let mut delivered = 0u32;
        for e in events {
            if event_engine_frame(&span, e).is_none() {
                self.push_fault(
                    FaultKind::StaleEventGeneration,
                    span.start,
                    e.source_sequence,
                );
            } else {
                delivered += 1;
                if delivered > self.budget.max_events_per_span {
                    self.push_fault(
                        FaultKind::EventBudgetExceeded,
                        span.start,
                        u64::from(delivered),
                    );
                    break;
                }
            }
        }
        sink.span(&span, events);
    }
}

/// A light clone of the mutable scalar transport state used by non-RT
/// [`TransportScheduler::preflight`] simulation.
#[derive(Copy, Clone)]
struct SimState {
    status: TransportStatus,
    anchor_frame: u64,
    anchor_pos: i64,
    loop_ticks: Option<(i64, i64)>,
}

impl SimState {
    fn pos_at(&self, frame: u64) -> i64 {
        let d = i64::try_from(frame.saturating_sub(self.anchor_frame)).unwrap_or(i64::MAX);
        if self.status == TransportStatus::Playing {
            self.anchor_pos.saturating_add(d)
        } else {
            self.anchor_pos
        }
    }

    fn next_wrap(&self, after: u64) -> Option<u64> {
        if self.status != TransportStatus::Playing {
            return None;
        }
        let (_, le) = self.loop_ticks?;
        let delta = le.saturating_sub(self.pos_at(after)).max(0);
        Some(after.saturating_add(u64::try_from(delta).unwrap_or(u64::MAX)))
    }

    fn wrap(&mut self, frame: u64) {
        if let Some((ls, le)) = self.loop_ticks {
            let overflow = self.pos_at(frame).saturating_sub(le).max(0);
            self.anchor_frame = frame;
            self.anchor_pos = ls + overflow;
        }
    }

    fn apply_command<Q>(
        &mut self,
        cmd: &TransportCommand,
        frame: u64,
        sched: &TransportScheduler<Q>,
    ) {
        let pos = self.pos_at(frame);
        match sched.validate(cmd) {
            Ok(Validated::Seek(t)) => {
                self.anchor_frame = frame;
                self.anchor_pos = t;
            }
            Ok(Validated::Loop(s, e)) => {
                self.loop_ticks = Some((s, e));
            }
            Ok(Validated::ClearLoop) => self.loop_ticks = None,
            Ok(Validated::Simple) => match cmd {
                TransportCommand::Play => {
                    self.status = TransportStatus::Playing;
                    self.anchor_frame = frame;
                    self.anchor_pos = pos;
                }
                TransportCommand::Pause => {
                    self.status = TransportStatus::Paused;
                    self.anchor_frame = frame;
                    self.anchor_pos = pos;
                }
                _ => {
                    self.status = TransportStatus::Stopped;
                    self.anchor_frame = frame;
                    self.anchor_pos = 0;
                }
            },
            Err(_) => {}
        }
    }
}

/// Converts `t` to ticks in `base` exactly; `None` when not
/// representable. The single position→tick authority shared by the
/// controller and plan builder.
///
/// **RT contract:** RT-suitable — pure integer math.
#[must_use]
pub fn media_time_ticks(t: MediaTime, base: TimeBase) -> Option<i64> {
    t.convert_exact(base).map(MediaTime::value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MediaDuration;
    use std::sync::Arc;

    fn base() -> TimeBase {
        TimeBase::sample_rate(48_000).unwrap()
    }

    fn t(v: i64) -> MediaTime {
        MediaTime::new(v, base())
    }

    fn range(s: i64, e: i64) -> MediaRange {
        MediaRange::new(t(s), MediaDuration::new(e - s, base()))
    }

    fn contract(min: u32, max: u32) -> BlockContract {
        BlockContract::new(min, max, InBlockTransportSupport::Unknown)
    }

    fn scheduler<P>(min: u32, max: u32, policy: DeliveryPolicy) -> TransportScheduler<P> {
        TransportScheduler::new(
            base(),
            contract(min, max),
            policy,
            ScheduleBudget::default(),
        )
    }

    struct Collect {
        spans: Vec<TransportSpan>,
        events: Vec<Vec<u64>>,
    }

    impl SpanSink<u32> for Collect {
        fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<u32>]) {
            self.spans.push(*span);
            self.events.push(
                events
                    .iter()
                    .filter_map(|e| event_engine_frame(span, e))
                    .map(EngineFrame::frame)
                    .collect(),
            );
        }
    }

    fn collect() -> Collect {
        Collect {
            spans: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Non-allocating sink for the `rt_` no-alloc contract test.
    struct CountSink {
        spans: u32,
        events: u32,
    }

    impl SpanSink<u32> for CountSink {
        fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<u32>]) {
            self.spans += 1;
            self.events += u32::try_from(events.len()).unwrap_or(u32::MAX);
            let _ = span;
        }
    }

    fn receipts_for(r: &[AppliedReceipt], id: CommandId) -> Vec<ApplyStatus> {
        r.iter().filter(|x| x.id == id).map(|x| x.status).collect()
    }

    #[test]
    fn legal_span_len_partitions() {
        let c = contract(128, 256);
        assert!(!c.legal_span_len(0));
        assert!(!c.legal_span_len(100)); // below min, no split possible
        assert!(c.legal_span_len(128));
        assert!(c.legal_span_len(156)); // single call <= max
        assert!(c.legal_span_len(256));
        assert!(c.legal_span_len(300)); // 150+150
        assert!(c.legal_span_len(257)); // 129+128
        let c2 = contract(128, 128);
        assert!(!c2.legal_span_len(156)); // chunks would be 128+28 < min
        assert!(c2.legal_span_len(256)); // 128+128
        let c1 = contract(1, 256);
        assert!(c1.legal_span_len(100));
    }

    #[test]
    fn partition_chunks_cover_len() {
        let c = contract(1, 128);
        let p = c.partition(300).unwrap();
        assert_eq!(p.chunks(), 3);
        let mut sum = 0;
        let mut off = 0;
        for i in 0..p.chunks() {
            let (o, f) = p.chunk(i);
            assert_eq!(o, off);
            assert!((1..=128).contains(&f));
            off += f;
            sum += f;
        }
        assert_eq!(sum, 300);
        let p = c.partition(100).unwrap();
        assert_eq!(p.chunks(), 1);
        assert_eq!(p.chunk(0), (0, 100));
    }

    #[test]
    fn engine_frame_offsets_are_epoch_scoped() {
        let e0 = ClockEpochId::new(0);
        let e1 = ClockEpochId::new(1);
        let a = EngineFrame::new(e0, 100);
        let b = EngineFrame::new(e0, 156);
        assert_eq!(b.checked_offset(a), Some(56));
        assert_eq!(a.checked_offset(b), None);
        assert_eq!(a.checked_offset(EngineFrame::new(e1, 100)), None);
        assert_eq!(a.advance(156), EngineFrame::new(e0, 256));
    }

    #[test]
    fn play_then_seek_receipt_lifecycle() {
        let mut s: TransportScheduler<u32> = scheduler(1, 256, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        let id = s
            .request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let seek = s
            .request(TransportCommand::Seek(t(1000)), EngineFrame::new(e0, 200))
            .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        // Play applied at 0, seek boundary at 200 → spans [0,200)+[200,256).
        assert_eq!(sink.spans.len(), 2);
        assert_eq!(sink.spans[0].start.frame(), 0);
        assert_eq!(sink.spans[1].start.frame(), 200);
        assert_eq!(sink.spans[1].ctx.status, TransportStatus::Playing);
        assert_eq!(sink.spans[1].ctx.position_ticks, 1000);
        let r = s.drain_receipts();
        assert_eq!(
            receipts_for(&r, id),
            vec![
                ApplyStatus::Requested,
                ApplyStatus::Pending,
                ApplyStatus::Applied
            ]
        );
        let applied_seek = r
            .iter()
            .find(|x| x.id == seek && x.status == ApplyStatus::Applied)
            .unwrap();
        assert_eq!(applied_seek.applied_frame, Some(EngineFrame::new(e0, 200)));
        assert_ne!(s.transport_generation(), TransportGeneration::new(0));
    }

    #[test]
    fn loop_wrap_splits_at_end() {
        let mut s: TransportScheduler<u32> = scheduler(1, 256, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(
            TransportCommand::SetLoop(range(64, 164)),
            EngineFrame::new(e0, 0),
        )
        .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        // Position reaches 164 at frame 164 → spans [0,164)+[164,256).
        assert_eq!(sink.spans.len(), 2);
        assert_eq!(sink.spans[0].end.frame(), 164);
        assert_eq!(sink.spans[1].ctx.position_ticks, 64);
        assert_eq!(sink.spans[1].ctx.occurrence, LoopOccurrence::new(1));
        // Next callback: loop len is 100 → wraps at 264, 364, 464.
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 256), 256, &mut sink);
        assert_eq!(sink.spans.len(), 4);
        assert_eq!(sink.spans[0].frames(), 8);
        assert_eq!(sink.spans[0].ctx.occurrence, LoopOccurrence::new(1));
        assert_eq!(sink.spans[1].ctx.occurrence, LoopOccurrence::new(2));
        assert_eq!(sink.spans[3].ctx.occurrence, LoopOccurrence::new(4));
    }

    #[test]
    fn loop_events_fire_once_per_occurrence() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(
            TransportCommand::SetLoop(range(100, 356)),
            EngineFrame::new(e0, 0),
        )
        .unwrap();
        let mut b = s.plan_builder();
        b.event(t(120), 1, 7).unwrap();
        b.event(t(50), 2, 9).unwrap();
        let plan = Arc::new(b.build());
        s.install_plan(plan, EngineFrame::new(e0, 0)).unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 512, &mut sink);
        let all: Vec<u64> = sink.events.iter().flatten().copied().collect();
        // Event @50 fires once at frame 50 (pre-loop, first pass).
        // Event @120 fires at 120 and again at 376 (occurrence 1).
        assert_eq!(all, vec![50, 120, 376]);
    }

    #[test]
    fn same_frame_events_keep_source_order() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let mut b = s.plan_builder();
        b.event(t(100), 3, 30).unwrap();
        b.event(t(100), 1, 10).unwrap();
        b.event(t(100), 2, 20).unwrap();
        let plan = Arc::new(b.build());
        s.install_plan(plan, EngineFrame::new(e0, 0)).unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        assert_eq!(sink.events[0], vec![100, 100, 100]);
        let seqs: Vec<u64> = s
            .plan
            .as_ref()
            .unwrap()
            .events
            .iter()
            .map(|e| e.source_sequence)
            .collect();
        assert_eq!(seqs, vec![1, 2, 3]);
    }

    #[test]
    fn stale_transport_generation_event_is_faulted_not_delivered() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let tg_now = s.transport_generation();
        let mut b = s.plan_builder();
        // Bound to the current tg — the seek at 100 bumps tg → stale.
        b.bound_event(t(300), 1, tg_now, 5).unwrap();
        let plan = Arc::new(b.build());
        s.install_plan(plan, EngineFrame::new(e0, 0)).unwrap();
        s.request(TransportCommand::Seek(t(0)), EngineFrame::new(e0, 100))
            .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 512, &mut sink);
        let all: Vec<u64> = sink.events.iter().flatten().copied().collect();
        assert_eq!(all, Vec::<u64>::new());
        let faults = s.drain_faults();
        assert!(
            faults
                .iter()
                .any(|f| f.kind == FaultKind::StaleEventGeneration)
        );
    }

    #[test]
    fn min_frames_violation_exact_wrap_undeliverable() {
        // Loop wrap at 100 in a 256-frame window under min_frames=128:
        // span [0,100) is unrepresentable under Exact.
        let mut s: TransportScheduler<u32> = scheduler(128, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(
            TransportCommand::SetLoop(range(0, 100)),
            EngineFrame::new(e0, 0),
        )
        .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        assert!(sink.spans.iter().any(|sp| !sp.deliverable));
        let faults = s.drain_faults();
        assert!(
            faults
                .iter()
                .any(|f| f.kind == FaultKind::UnrepresentableLoopWrap)
        );
    }

    #[test]
    fn min_frames_violation_degraded_defers_first_wrap_then_faults() {
        // Loop [0,100) under min=128 is unrepresentable at *every* wrap —
        // Degraded defers the first wrap to the next legal boundary (128)
        // and records defined faults for the later impossible wraps.
        let mut s: TransportScheduler<u32> = scheduler(128, 512, DeliveryPolicy::Degraded);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(
            TransportCommand::SetLoop(range(0, 100)),
            EngineFrame::new(e0, 0),
        )
        .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        // First wrap deferred to 128 → [0,128) legal; the deferred
        // post-wrap stretch [128,200) is also unrepresentable (72 < 128)
        // so the second wrap is emitted undeliverable with a fault.
        assert_eq!(sink.spans[0].frames(), 128);
        assert!(sink.spans[0].deliverable);
        assert_eq!(sink.spans[1].ctx.occurrence, LoopOccurrence::new(1));
        assert_eq!(sink.spans[1].ctx.position_ticks, 28); // ls + overflow
        let faults = s.drain_faults();
        assert!(
            faults
                .iter()
                .any(|f| f.kind == FaultKind::UnrepresentableLoopWrap)
        );
    }

    #[test]
    fn degraded_apply_records_deferred_receipt() {
        // Seek at frame 100 inside a 256 window under min=128: Exact
        // preflight rejects; Degraded defers the apply to 128 and the
        // receipt records requested vs actual frame.
        let mut s: TransportScheduler<u32> = scheduler(128, 512, DeliveryPolicy::Degraded);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let seek = s
            .request(TransportCommand::Seek(t(7)), EngineFrame::new(e0, 100))
            .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e0, 0), 256, &mut sink);
        assert_eq!(sink.spans.len(), 2);
        assert!(sink.spans.iter().all(|sp| sp.deliverable));
        assert_eq!(sink.spans[0].end.frame(), 128);
        assert_eq!(sink.spans[1].ctx.position_ticks, 7);
        let r = s.drain_receipts();
        let applied = r
            .iter()
            .find(|x| x.id == seek && x.status == ApplyStatus::Applied)
            .unwrap();
        assert_eq!(applied.applied_frame, Some(EngineFrame::new(e0, 128)));
        assert_eq!(
            applied.degradation,
            Some(ApplyDegradation::DeferredApply {
                requested: EngineFrame::new(e0, 100),
                applied: EngineFrame::new(e0, 128),
            })
        );
    }

    #[test]
    fn preflight_rejects_illegal_apply_under_exact() {
        let mut s: TransportScheduler<u32> = scheduler(128, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let r = s.preflight(
            TransportCommand::Seek(t(500)),
            EngineFrame::new(e0, 100),
            EngineFrame::new(e0, 0),
            256,
        );
        assert_eq!(r, Err(RejectReason::UnsupportedExactTransport));
        let mut s2: TransportScheduler<u32> = scheduler(128, 512, DeliveryPolicy::Degraded);
        s2.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let r = s2.preflight(
            TransportCommand::Seek(t(500)),
            EngineFrame::new(e0, 100),
            EngineFrame::new(e0, 0),
            256,
        );
        assert_eq!(r, Ok(EngineFrame::new(e0, 128)));
    }

    #[test]
    fn device_restart_vs_seek_distinct_epoch_generation() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        let tg0 = s.transport_generation();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        let pending = s
            .request(TransportCommand::Seek(t(10)), EngineFrame::new(e0, 500))
            .unwrap();
        let e1 = s.device_restart(0);
        assert_ne!(e0, e1);
        let r = s.drain_receipts();
        assert_eq!(
            receipts_for(&r, pending).last(),
            Some(&ApplyStatus::Rejected)
        );
        assert_eq!(
            r.iter()
                .find(|x| x.id == pending && x.status == ApplyStatus::Rejected)
                .unwrap()
                .reason,
            Some(RejectReason::StaleGeneration)
        );
        s.request(TransportCommand::Seek(t(5)), EngineFrame::new(e1, 10))
            .unwrap();
        let mut sink = collect();
        s.run_callback(EngineFrame::new(e1, 0), 256, &mut sink);
        assert_eq!(s.epoch(), e1);
        assert_ne!(s.transport_generation(), tg0);
        assert_eq!(s.transport_generation(), TransportGeneration::new(1));
    }

    #[test]
    fn pending_budget_rejects_overflow() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        s.budget = ScheduleBudget::new(64, 256, 2, 4096);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(TransportCommand::Pause, EngineFrame::new(e0, 100))
            .unwrap();
        let r = s.request(TransportCommand::Stop, EngineFrame::new(e0, 200));
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().reason, Some(RejectReason::BudgetExceeded));
    }

    /// RT contract: `run_callback` must not allocate — pending/receipt/
    /// fault buffers are pre-allocated and only pushed within capacity,
    /// plan is behind `Arc`, boundary scans are budget-bounded.
    #[test]
    fn rt_run_callback_no_alloc() {
        let mut s: TransportScheduler<u32> = scheduler(1, 512, DeliveryPolicy::Exact);
        let e0 = s.epoch();
        s.request(TransportCommand::Play, EngineFrame::new(e0, 0))
            .unwrap();
        s.request(
            TransportCommand::SetLoop(range(64, 192)),
            EngineFrame::new(e0, 0),
        )
        .unwrap();
        s.request(TransportCommand::Seek(t(0)), EngineFrame::new(e0, 300))
            .unwrap();
        let mut b = s.plan_builder();
        for i in 0..8i64 {
            b.event(
                t(50 + i * 10),
                u64::try_from(i).unwrap(),
                u32::try_from(i).unwrap(),
            )
            .unwrap();
        }
        let plan = Arc::new(b.build());
        s.install_plan(plan, EngineFrame::new(e0, 0)).unwrap();
        let mut sink = CountSink {
            spans: 0,
            events: 0,
        };
        s.drain_receipts();
        s.drain_faults();
        assert_no_alloc::assert_no_alloc(|| {
            for i in 0..4u64 {
                s.run_callback(EngineFrame::new(e0, i * 256), 256, &mut sink);
            }
        });
        assert!(sink.spans > 0);
        let _ = s.drain_receipts();
    }
}
