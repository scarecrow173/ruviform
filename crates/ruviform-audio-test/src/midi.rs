//! Deterministic MIDI scenario → trace (Issue #628).
//!
//! A [`MidiScenario`] compiles timestamped [`MidiCue`]s into a
//! production [`TransportPlan<MidiEvent>`] via
//! [`TransportPlanBuilder`], then [`run_midi_trace`] drives a
//! production [`TransportScheduler`] block-by-block. Delivery is
//! captured through the production [`SpanSink`] contract — the same
//! boundary a real host adapter would observe. There is no production
//! MIDI *output* port in the repo today, so the sink capture is the
//! outgoing-trace record point (recorded in the design doc).
//!
//! **RT contract:** [`TraceSink`] implements [`SpanSink`] and is
//! allocation-free once its entry vec is preallocated — proven by
//! `rt_`-prefixed `assert_no_alloc` tests. `run_midi_trace` itself is
//! non-RT (it owns the scheduler driver loop).

use std::sync::Arc;

use ruviform_media_core::{
    AppliedReceipt, BlockContract, ClockEpochId, DeliveryPolicy, EngineFrame, Error,
    LoopOccurrence, MediaTime, MidiEvent, PlanGeneration, Result, ScheduleBudget, ScheduleFault,
    ScheduledEvent, SpanSink, TimeBase, TransportCommand, TransportPlan, TransportPlanBuilder,
    TransportScheduler, TransportSpan, event_engine_frame,
};

/// One timestamped MIDI event in a scenario: canonical absolute
/// `position`, explicit causal `source_sequence` ordering key, and the
/// production [`MidiEvent`] payload.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MidiCue {
    /// Canonical absolute position (converted to plan frame-base ticks
    /// at compile; unrepresentable positions are rejected).
    pub position: MediaTime,
    /// Explicit causal ordering key — events at the same position are
    /// delivered in `source_sequence` order (no implicit
    /// note-off-before-note-on rule).
    pub source_sequence: u64,
    /// The MIDI message payload (production type, used as-is).
    pub event: MidiEvent,
}

/// A deterministic MIDI scenario: a frame base plus an unordered cue
/// list. Compile sorts by `(position_ticks, source_sequence)` through
/// the production [`TransportPlanBuilder`].
#[derive(Clone, Debug)]
pub struct MidiScenario {
    /// Plan frame base — one tick equals one engine frame.
    pub frame_base: TimeBase,
    /// Scenario cues (any order; the builder sorts them).
    pub cues: Vec<MidiCue>,
}

impl MidiScenario {
    /// Creates a scenario. `frame_base` is already a validated
    /// [`TimeBase`]; cue positions are validated at [`Self::compile`]
    /// (exact representability in the frame base).
    #[must_use]
    pub fn new(frame_base: TimeBase, cues: Vec<MidiCue>) -> Self {
        Self { frame_base, cues }
    }

    /// Compiles the cues into an immutable [`TransportPlan`] under
    /// plan generation `1` (single-plan scenarios).
    ///
    /// **RT contract:** non-RT — allocates the plan.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when a cue position cannot be represented
    /// exactly in the frame base (the production builder's reject path).
    pub fn compile(&self) -> Result<Arc<TransportPlan<MidiEvent>>> {
        let mut builder =
            TransportPlanBuilder::<MidiEvent>::new(self.frame_base, PlanGeneration::new(1));
        for cue in &self.cues {
            builder.event(cue.position, cue.source_sequence, cue.event)?;
        }
        Ok(Arc::new(builder.build()))
    }
}

/// One delivered MIDI event observed at the [`SpanSink`] boundary.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MidiTraceEntry {
    /// Engine frame the event was delivered at (epoch + frame).
    pub frame: EngineFrame,
    /// The event's canonical position in plan frame-base ticks.
    pub position_ticks: i64,
    /// Loop occurrence of the delivering span.
    pub occurrence: LoopOccurrence,
    /// The cue's causal ordering key.
    pub source_sequence: u64,
    /// The delivered MIDI payload.
    pub event: MidiEvent,
}

/// Sink capturing delivered MIDI events — a [`SpanSink`]
/// implementation usable both inside [`run_midi_trace`] and directly
/// with a consumer-owned [`TransportScheduler`].
///
/// `span()` is allocation-free once `entries` has sufficient capacity;
/// `trace_capacity` in [`MidiTraceConfig`] preallocates for the
/// expected entry count.
#[derive(Debug)]
pub struct TraceSink {
    /// Delivered entries in span order.
    pub entries: Vec<MidiTraceEntry>,
    /// Spans emitted with `deliverable == false` (no legal physical-call
    /// partition — events on them are not delivered).
    pub undeliverable_spans: u32,
}

impl TraceSink {
    /// Creates an empty sink with `capacity` entry preallocation.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            undeliverable_spans: 0,
        }
    }
}

impl SpanSink<MidiEvent> for TraceSink {
    fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<MidiEvent>]) {
        if !span.deliverable {
            self.undeliverable_spans += 1;
            return;
        }
        for event in events {
            // Per-event deliverability is decided by the production
            // authority `event_engine_frame` (generation tags +
            // Playing status + position window).
            if let Some(frame) = event_engine_frame(span, event) {
                self.entries.push(MidiTraceEntry {
                    frame,
                    position_ticks: event.position_ticks(),
                    occurrence: span.ctx.occurrence,
                    source_sequence: event.source_sequence,
                    event: event.payload,
                });
            }
        }
    }
}

/// Driver configuration for [`run_midi_trace`].
#[derive(Clone, Debug)]
pub struct MidiTraceConfig {
    /// Frames per scheduler callback.
    pub block_frames: u32,
    /// Total engine frames to run.
    pub total_frames: u64,
    /// Plugin block contract the scheduler delivers under.
    pub contract: BlockContract,
    /// Exact or degraded delivery policy.
    pub policy: DeliveryPolicy,
    /// Bounded-work budgets.
    pub budget: ScheduleBudget,
    /// Additional transport commands `(due_frame, command)` issued
    /// before the callback covering their due frame — supports
    /// Seek/SetLoop/ClearLoop/Pause/Stop scenarios. Stable-sorted by
    /// due frame internally; commands with `due >= total_frames` are
    /// never issued.
    pub commands: Vec<(u64, TransportCommand)>,
    /// Preallocation hint for [`TraceSink::entries`]; must cover the
    /// expected entry count for the `rt_` no-alloc path.
    pub trace_capacity: usize,
}

/// The recorded result of a MIDI trace run.
///
/// # Examples
///
/// ```
/// use ruviform_audio_test::midi::{MidiCue, MidiScenario, MidiTraceConfig, run_midi_trace};
/// use ruviform_media_core::{
///     BlockContract, DeliveryPolicy, InBlockTransportSupport, MediaTime, MidiChannel,
///     MidiEvent, MidiGroup, MidiKind, NoteKey, ScheduleBudget, TimeBase,
/// };
///
/// let base = TimeBase::sample_rate(48_000).unwrap();
/// let event = MidiEvent {
///     group: MidiGroup::ZERO,
///     channel: MidiChannel::new(0).unwrap(),
///     kind: MidiKind::NoteOn {
///         key: NoteKey::new(60).unwrap(),
///         velocity: 0x8000_0000,
///     },
/// };
/// let scenario = MidiScenario::new(base, vec![MidiCue {
///     position: MediaTime::new(100, base),
///     source_sequence: 0,
///     event,
/// }]);
/// let trace = run_midi_trace(&scenario, &MidiTraceConfig {
///     block_frames: 64,
///     total_frames: 256,
///     contract: BlockContract::new(1, 4_096, InBlockTransportSupport::Unknown),
///     policy: DeliveryPolicy::Exact,
///     budget: ScheduleBudget::new(64, 1_024, 16, 0),
///     commands: Vec::new(),
///     trace_capacity: 16,
/// }).unwrap();
/// assert_eq!(trace.entries().len(), 1);
/// assert_eq!(trace.entries()[0].frame.frame(), 100);
/// ```
#[derive(Debug)]
pub struct MidiTrace {
    entries: Vec<MidiTraceEntry>,
    receipts: Vec<AppliedReceipt>,
    faults: Vec<ScheduleFault>,
    fault_overflow: u64,
    blocks_run: u32,
    undeliverable_spans: u32,
}

impl MidiTrace {
    /// Delivered entries in engine-frame order.
    #[must_use]
    pub fn entries(&self) -> &[MidiTraceEntry] {
        &self.entries
    }

    /// Command receipts recorded by the scheduler (Requested/Pending/
    /// Applied/Rejected lifecycle).
    #[must_use]
    pub fn receipts(&self) -> &[AppliedReceipt] {
        &self.receipts
    }

    /// Bounded faults recorded by the scheduler.
    #[must_use]
    pub fn faults(&self) -> &[ScheduleFault] {
        &self.faults
    }

    /// Faults dropped because the scheduler's fault ledger overflowed.
    #[must_use]
    pub fn fault_overflow(&self) -> u64 {
        self.fault_overflow
    }

    /// Callbacks executed.
    #[must_use]
    pub fn blocks_run(&self) -> u32 {
        self.blocks_run
    }

    /// Spans the scheduler emitted as undeliverable.
    #[must_use]
    pub fn undeliverable_spans(&self) -> u32 {
        self.undeliverable_spans
    }
}

/// Compiles `scenario` and runs it through a production
/// [`TransportScheduler`] for `config.total_frames`, capturing the
/// delivered trace via [`TraceSink`].
///
/// The plan is installed and `Play` is requested at frame `0` of the
/// scheduler epoch; `config.commands` add further transport commands
/// (seek/loop/pause/…) at their due frames.
///
/// **RT contract:** non-RT — allocates the plan, trace vectors and
/// scheduler ledgers.
///
/// # Errors
///
/// [`Error::TimeOverflow`] when a cue position is unrepresentable, or
/// [`Error::InvalidValue`] when a scheduler request is rejected.
pub fn run_midi_trace(scenario: &MidiScenario, config: &MidiTraceConfig) -> Result<MidiTrace> {
    let plan = scenario.compile()?;
    if config.block_frames == 0 {
        return Err(Error::InvalidValue("block_frames"));
    }
    let mut scheduler = TransportScheduler::<MidiEvent>::new(
        scenario.frame_base,
        config.contract,
        config.policy,
        config.budget,
    );
    let epoch = scheduler.epoch();
    let install_due = EngineFrame::new(epoch, 0);
    scheduler
        .install_plan(Arc::clone(&plan), install_due)
        .map_err(|_| Error::InvalidValue("install_plan"))?;
    scheduler
        .request(TransportCommand::Play, install_due)
        .map_err(|_| Error::InvalidValue("play"))?;

    let mut commands: Vec<(u64, TransportCommand)> = config.commands.clone();
    commands.sort_by_key(|(due, _)| *due);

    let mut sink = TraceSink::with_capacity(config.trace_capacity);
    let mut next_cmd = 0usize;
    let mut start = 0u64;
    let mut blocks_run = 0u32;
    while start < config.total_frames {
        let frames = config
            .block_frames
            .min(u32::try_from(config.total_frames - start).unwrap_or(u32::MAX));
        let window_end = start.saturating_add(u64::from(frames));
        while next_cmd < commands.len() && commands[next_cmd].0 < window_end {
            let (due, cmd) = commands[next_cmd];
            scheduler
                .request(cmd, EngineFrame::new(epoch, due))
                .map_err(|_| Error::InvalidValue("command"))?;
            next_cmd += 1;
        }
        scheduler.run_callback(EngineFrame::new(epoch, start), frames, &mut sink);
        start = window_end;
        blocks_run = blocks_run.saturating_add(1);
    }

    let fault_overflow = scheduler.fault_overflow;
    Ok(MidiTrace {
        entries: sink.entries,
        receipts: scheduler.drain_receipts(),
        faults: scheduler.drain_faults(),
        fault_overflow,
        blocks_run,
        undeliverable_spans: sink.undeliverable_spans,
    })
}

/// The scheduler epoch used by [`run_midi_trace`] — exposed for callers
/// building [`EngineFrame`]s against the same epoch.
#[must_use]
pub fn trace_epoch() -> ClockEpochId {
    ClockEpochId::new(0)
}
