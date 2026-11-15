//! MIDI leg — scripted `MidiFeed`/`MockPort` → production
//! `InputStage` staging/pump/`collect_block` path (Issue #628,
//! 3rd slice).
//!
//! The production non-RT→RT handoff owner is `InputTx`, but it is
//! `pub(crate)` — unreachable from this crate. [`InputPump`] mirrors
//! its exact contract (bounded staging `Vec` + flush→read→flush into
//! an SPSC channel, deferred-on-full backpressure, cumulative
//! backend drop/unsupported counters) so the pipeline exercises the
//! same *shape* of handoff; [`InputStage::pump`] then moves events
//! into `pending` and [`InputStage::collect_block`] performs the
//! production `ClockAnchor`/`map_stamp_to_frame` mapping — none of
//! that mapping/staging logic is reimplemented here.
//!
//! `InputStage::attach()` / real plugin endpoint wiring is
//! deliberately out of scope (the runtime-integration slice owns it);
//! this leg stops at `collect_block`, the last production surface
//! before `merge_call_events`.
//!
//! ## Anchor
//!
//! [`MidiLeg::open`] publishes a [`ClockAnchor`] calibrated as
//! "stamp 0 (in `frame_base`) on `domain` == engine frame 0 of
//! `epoch`". `MidiFeed` stamps each cue's frame position directly in
//! `frame_base` ticks, so `map_stamp_to_frame` reduces to identity —
//! the *test's* expected offsets are computed independently from the
//! scenario (see `tests/pipeline_midi.rs`), not read back from the
//! mapping output.
//!
//! **RT contract:** [`InputPump::pump`] is non-RT (may lock the mock
//! port's queue); [`InputStage::pump`]/`collect_block` are the
//! production RT-suitable surface — `rt_pipeline_midi_*` tests prove
//! the combined path allocation-free.

use ruviform_audio_core::device::DeviceResult;
use ruviform_audio_core::midi_input::mock::{MockBackend, MockPort};
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputInfo, MidiInputPort, MidiPortCapabilities,
    MidiPortOrigin, TimedMidiInput,
};
use ruviform_audio_plugin_bridge::{BlockInputStats, INPUT_STAGE_CAPACITY, InputStage};
use ruviform_media_core::{
    ClockAnchor, ClockDomainId, ClockEpochId, EngineFrame, MidiChannel, MidiEvent, MidiGroup,
    MidiKind, NoteKey, TimeBase, TimestampProvenance,
};
use ruviform_plugin_core::{Event, EventKind};
use ruviform_rt::{Consumer, Producer, channel};

use crate::backend::MidiFeed;
use crate::midi::MidiScenario;
use crate::pipeline::PipelineError;

/// Non-RT producer half mirroring the `pub(crate)` production
/// `InputTx`: drains a [`MidiInput`] into a bounded staging buffer and
/// pushes staged events into the RT channel, preserving arrival order.
/// A full channel defers (backpressure — never a silent drop); a full
/// staging buffer stops `read` early. Loss is observable only inside
/// the backend's own queue via `backend_dropped`/`backend_unsupported`.
///
/// **RT contract:** non-RT only — owns the `Producer` endpoint and a
/// staging `Vec` preallocated to [`INPUT_STAGE_CAPACITY`].
pub struct InputPump {
    tx: Producer<TimedMidiInput>,
    staging: Vec<TimedMidiInput>,
    dropped_backend: u64,
    unsupported_backend: u64,
}

impl InputPump {
    /// Pairs with the consumer half of `channel`.
    #[must_use]
    pub fn new(tx: Producer<TimedMidiInput>) -> Self {
        Self {
            tx,
            staging: Vec::with_capacity(INPUT_STAGE_CAPACITY),
            dropped_backend: 0,
            unsupported_backend: 0,
        }
    }

    /// Flush staged events, top up from `input`, flush again —
    /// identical algorithm to production `InputTx::pump`.
    ///
    /// # Errors
    /// [`ruviform_audio_core::device::DeviceError`] from `input.read`.
    pub fn pump(&mut self, input: &mut dyn MidiInput) -> DeviceResult<InputPumpStats> {
        let mut pushed = 0usize;
        let mut kept = self.flush(&mut pushed);
        let spare = INPUT_STAGE_CAPACITY - kept;
        let stats = input.read(&mut self.staging, spare)?;
        let received = self.staging.len() - kept;
        self.dropped_backend += stats.dropped;
        self.unsupported_backend += stats.unsupported;
        kept = self.flush(&mut pushed);
        Ok(InputPumpStats {
            received,
            pushed,
            deferred: kept,
            backend_dropped: stats.dropped,
            backend_unsupported: stats.unsupported,
        })
    }

    fn flush(&mut self, pushed: &mut usize) -> usize {
        let mut kept = 0usize;
        for r in 0..self.staging.len() {
            match self.tx.try_push(self.staging[r]) {
                Ok(()) => *pushed += 1,
                Err(full) => {
                    self.staging[kept] = full.into_inner();
                    kept += 1;
                }
            }
        }
        self.staging.truncate(kept);
        kept
    }

    /// Events still staged (channel full — delivered on a later pump).
    #[must_use]
    pub fn deferred(&self) -> usize {
        self.staging.len()
    }

    /// Cumulative backend-reported drops.
    #[must_use]
    pub fn dropped_backend(&self) -> u64 {
        self.dropped_backend
    }

    /// Cumulative backend-reported untranslatable messages.
    #[must_use]
    pub fn unsupported_backend(&self) -> u64 {
        self.unsupported_backend
    }
}

/// Per-`pump` stats — same field semantics as the production
/// (non-exhaustive, crate-private-constructed) `PumpStats`, exposed
/// under a test-owned name so the pipeline can report them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputPumpStats {
    /// Events `read` appended to the staging buffer this call.
    pub received: usize,
    /// Events pushed into the RT channel this call.
    pub pushed: usize,
    /// Events still staged afterwards (channel full — delivered on a
    /// later `pump`; not lost).
    pub deferred: usize,
    /// Backend-reported drops inside `read` this call.
    pub backend_dropped: u64,
    /// Backend-reported untranslatable messages this call.
    pub backend_unsupported: u64,
}

/// The RT-suitable half of one block: `InputStage::pump` (channel →
/// `pending`) plus `collect_block` (mapping + emission into the
/// leg's preallocated scratch). Returned by [`MidiLeg::collect_block`].
///
/// **RT contract:** allocation-free — `pending`, the SPSC channel and
/// `scratch` are all fixed at construction (`rt_pipeline_midi_*`
/// tests prove it). The returned slice borrows the leg's scratch.
pub struct CollectedBlock<'a> {
    /// Absolute engine frame the block started at.
    pub block_start: EngineFrame,
    /// Block length in frames.
    pub block_frames: u32,
    /// Events `InputStage::pump` moved into `pending` this call.
    pub moved: u32,
    /// Emitted `Event`s (call-relative offsets), borrowed from the
    /// leg's scratch — valid until the next `collect_block`.
    pub events: &'a mut [Event],
    /// Per-block `InputStage` stats (emitted/late/consumed).
    pub stats: BlockInputStats,
}

/// One block's observed output: the `Event`s `collect_block`
/// emitted plus the production stats/counters around them.
#[derive(Clone, Debug)]
pub struct BlockObservation {
    /// Absolute engine frame the block started at.
    pub block_start: EngineFrame,
    /// Block length in frames.
    pub block_frames: u32,
    /// `(frame_offset, MidiEvent)` pairs in emission order.
    pub events: Vec<(u32, MidiEvent)>,
    /// Per-block `InputStage` stats (emitted/late/consumed).
    pub stats: BlockInputStats,
    /// Events the pump moved from the SPSC channel into `pending`.
    pub moved: u32,
    /// Events `feed.pump_into_port` pushed this call.
    pub fed: usize,
    /// `InputPump::pump` stats for this call.
    pub pump: InputPumpStats,
}

/// A live MIDI leg: `MockBackend`/`MockPort` + `MidiFeed` →
/// `InputPump` → `InputStage`.
pub struct MidiLeg {
    port: MockPort,
    input: Box<dyn MidiInput>,
    stage: InputStage,
    pump: InputPump,
    rx: Consumer<TimedMidiInput>,
    feed: MidiFeed,
    epoch: ClockEpochId,
    frame_base: TimeBase,
    scratch: Vec<Event>,
    block_cursor: u64,
}

/// Scratch-seed event for preallocating the `collect_block` scratch —
/// contents are always overwritten before use.
fn scratch_seed() -> Event {
    Event::new(
        0,
        EventKind::Midi(MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).expect("channel 0 in range"),
            kind: MidiKind::NoteOff {
                key: NoteKey::new(0).expect("key 0 in range"),
                velocity: 0,
            },
        }),
    )
}

impl MidiLeg {
    /// Opens the scripted MIDI path: registers one `MockPort` on a
    /// fresh [`MockBackend`], enumerates + opens it through the
    /// production [`MidiInputEnumerator`]/[`MidiInputPort`] traits,
    /// builds the [`InputPump`]→[`InputStage`] handoff, and publishes
    /// the identity [`ClockAnchor`].
    ///
    /// `channel_capacity` is the non-RT→RT handoff depth — `0` or
    /// `> INPUT_STAGE_CAPACITY` is rejected (mirrors
    /// [`ruviform_audio_plugin_bridge::InputConfig`]'s defined bounds).
    ///
    /// # Errors
    /// [`PipelineError::Device`] on enumeration/open failure
    /// (`MockPort::fail_next_open` exercises this),
    /// [`PipelineError::Config`] on an invalid channel capacity.
    pub fn open(
        scenario: &MidiScenario,
        clock_domain: ClockDomainId,
        epoch: ClockEpochId,
        channel_capacity: usize,
    ) -> Result<Self, PipelineError> {
        let mut backend = MockBackend::new();
        let port = backend.add_port(MidiInputInfo::new(
            "pipeline-midi-0".to_owned(),
            "Pipeline MIDI In".to_owned(),
            MidiPortOrigin::Internal,
            MidiPortCapabilities::stamped(false, clock_domain),
        ));
        // Exercise the production enumerator surface; the yielded
        // trait objects share the same `Arc` state as `port`, so the
        // subscription `open_port` opens is the enumerated one.
        let ports = backend.input_ports().map_err(PipelineError::Device)?;
        if ports.len() != 1 {
            return Err(PipelineError::Config("mock backend port count"));
        }
        Self::open_port(scenario, clock_domain, epoch, channel_capacity, port)
    }

    /// Opens the leg on a caller-supplied [`MockPort`] — same path as
    /// [`Self::open`] minus backend registration, so tests can script
    /// `fail_next_open`/`inject_dropped`/`inject_unsupported` before
    /// the subscription opens. Still goes through the production
    /// [`MidiInputPort::open`]/[`MidiInput`] traits.
    ///
    /// # Errors
    /// Same as [`Self::open`].
    pub fn open_port(
        scenario: &MidiScenario,
        clock_domain: ClockDomainId,
        epoch: ClockEpochId,
        channel_capacity: usize,
        port: MockPort,
    ) -> Result<Self, PipelineError> {
        if channel_capacity == 0 || channel_capacity > INPUT_STAGE_CAPACITY {
            return Err(PipelineError::Config("midi channel capacity"));
        }
        let input = port.open().map_err(PipelineError::Device)?;

        let (tx, rx) = channel::<TimedMidiInput>(channel_capacity);
        let pump = InputPump::new(tx);
        let mut stage = InputStage::new(scenario.frame_base);
        stage.publish_anchor(Some(ClockAnchor {
            domain: clock_domain,
            stamp_ticks: 0,
            stamp_base: scenario.frame_base,
            frame: EngineFrame::new(epoch, 0),
            calibration: TimestampProvenance::Carried,
        }));
        let feed = MidiFeed::new(scenario, clock_domain);
        let scratch = vec![scratch_seed(); INPUT_STAGE_CAPACITY];
        Ok(Self {
            port,
            input,
            stage,
            pump,
            rx,
            feed,
            epoch,
            frame_base: scenario.frame_base,
            scratch,
            block_cursor: 0,
        })
    }

    /// Non-RT feed half of one block: pushes due cues into the port,
    /// then drains the subscription through [`InputPump`] into the
    /// SPSC channel. Returns `(fed, pump_stats)`.
    ///
    /// `upto_frame` is the driver's current virtual position — cues
    /// with `stamp.ticks <= upto_frame` are pushed to the port first
    /// (same contract as `MidiFeed::pump_into_port`).
    ///
    /// **RT contract:** non-RT — `MockPort`/`MidiInput::read` may lock
    /// and allocate.
    ///
    /// # Errors
    /// [`PipelineError::Device`] when the subscription's `read` fails.
    pub fn pump_inputs(
        &mut self,
        upto_frame: u64,
    ) -> Result<(usize, InputPumpStats), PipelineError> {
        let fed = self.feed.pump_into_port(&self.port, upto_frame);
        let stats = self
            .pump
            .pump(&mut *self.input)
            .map_err(PipelineError::Device)?;
        Ok((fed, stats))
    }

    /// RT-suitable half of one block: `InputStage::pump` moves queued
    /// channel events into `pending`, then `collect_block` maps and
    /// emits this block's events into the leg's preallocated scratch.
    /// Advances the block cursor by `block_frames`.
    ///
    /// **RT contract:** allocation-free — proven by
    /// `rt_pipeline_midi_collect_does_not_alloc`.
    pub fn collect_block(&mut self, block_frames: u32) -> CollectedBlock<'_> {
        let moved = self.stage.pump(&mut self.rx);
        let block_start = EngineFrame::new(self.epoch, self.block_cursor);
        let block_end = self.block_cursor.saturating_add(u64::from(block_frames));
        let now = EngineFrame::new(self.epoch, block_end);
        let (events, stats) =
            self.stage
                .collect_block(block_start, block_frames, now, self.scratch.as_mut_slice());
        self.block_cursor = block_end;
        CollectedBlock {
            block_start,
            block_frames,
            moved,
            events,
            stats,
        }
    }

    /// Pumps one whole block: [`Self::pump_inputs`] then
    /// [`Self::collect_block`], returning the observed events as
    /// `(frame_offset, MidiEvent)` pairs in emission order.
    ///
    /// `upto_frame` feeds cues with `stamp.ticks <= upto_frame`;
    /// `block_frames` is the block length — call blocks in order
    /// (the cursor advances per call). [`Self::pump_next`] computes
    /// `upto_frame` as the upcoming block's last frame.
    ///
    /// # Errors
    /// [`PipelineError::Device`] when the subscription's `read` fails.
    pub fn pump_block(
        &mut self,
        upto_frame: u64,
        block_frames: u32,
    ) -> Result<BlockObservation, PipelineError> {
        let (fed, pump_stats) = self.pump_inputs(upto_frame)?;
        let collected = self.collect_block(block_frames);
        let observed = collected
            .events
            .iter()
            .filter_map(|e| match e.kind() {
                // `collect_block` only ever emits `EventKind::Midi`;
                // anything else is filtered rather than fabricated.
                EventKind::Midi(m) => Some((e.frame_offset(), m)),
                _ => None,
            })
            .collect();
        Ok(BlockObservation {
            block_start: collected.block_start,
            block_frames: collected.block_frames,
            events: observed,
            stats: collected.stats,
            moved: collected.moved,
            fed,
            pump: pump_stats,
        })
    }

    /// Convenience: pumps the block `[cursor, cursor + block_frames)`
    /// and feeds every cue due inside it (`upto = end - 1`).
    ///
    /// # Errors
    /// Same as [`Self::pump_block`].
    pub fn pump_next(&mut self, block_frames: u32) -> Result<BlockObservation, PipelineError> {
        let end = self.block_cursor.saturating_add(u64::from(block_frames));
        let upto = end.saturating_sub(1).max(self.block_cursor);
        self.pump_block(upto, block_frames)
    }

    /// The mock port handle — for `inject_dropped`/`inject_unsupported`/
    /// direct `push` fault injection in tests.
    #[must_use]
    pub fn port(&self) -> &MockPort {
        &self.port
    }

    /// The scenario's frame base (anchor `stamp_base`).
    #[must_use]
    pub const fn frame_base(&self) -> TimeBase {
        self.frame_base
    }

    /// The published anchor's epoch.
    #[must_use]
    pub const fn epoch(&self) -> ClockEpochId {
        self.epoch
    }

    /// Next block-start frame (leg cursor).
    #[must_use]
    pub fn block_cursor(&self) -> u64 {
        self.block_cursor
    }

    /// Cues not yet fed to the port.
    #[must_use]
    pub fn feed_remaining(&self) -> usize {
        self.feed.remaining()
    }

    /// Cues skipped at `MidiFeed` conversion (unrepresentable positions).
    #[must_use]
    pub fn feed_skipped(&self) -> u64 {
        self.feed.skipped()
    }

    /// Events still pending inside `InputStage`.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.stage.pending_count()
    }

    /// Cumulative `InputStage` counters (`mapped`/`delivered`/`late`/
    /// `rejected`/`anchor_rejected`/`dropped_*`/`unsupported_*`).
    #[must_use]
    pub fn counters(&self) -> ruviform_audio_plugin_bridge::InputStageCounters {
        self.stage.counters()
    }

    /// `InputPump` cumulative backend drop/unsupported totals.
    #[must_use]
    pub fn pump_counters(&self) -> (u64, u64) {
        (self.pump.dropped_backend(), self.pump.unsupported_backend())
    }

    /// Closes the subscription (non-RT).
    pub fn close(&mut self) {
        self.input.close();
    }
}

impl core::fmt::Debug for MidiLeg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MidiLeg")
            .field("block_cursor", &self.block_cursor)
            .field("feed_remaining", &self.feed.remaining())
            .field("pending", &self.stage.pending_count())
            .field("counters", &self.stage.counters())
            .finish_non_exhaustive()
    }
}
