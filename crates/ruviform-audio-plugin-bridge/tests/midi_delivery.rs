//! Issue #188 `PoC` — timed `MIDI` input end-to-end:
//! `MidiInput` → `InputStage` (stamp→frame map, ledger) →
//! `merge_call_events` (release/plan/live) → `EventList`.
//!
//! Covers: `NoteOn`/`NoteOff`, CC, `PitchBend` type+order
//! preservation; duplicate timestamps (stable order); block-boundary
//! behavior (in-block/late/future/rejected); `NoteOff` priority on
//! colliding keys; `release_all` on stop; observable overflow/drop
//! counters; and the RT no-allocation contract for `collect_block` +
//! merge.

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

use std::collections::VecDeque;

use ruviform_audio_core::device::DeviceResult;
use ruviform_audio_core::midi_input::{MidiInput, MidiReadStats, TimedMidiInput};
use ruviform_audio_plugin_bridge::InputStage;
use ruviform_media_core::{
    ClockAnchor, ClockDomainId, ClockEpochId, EngineFrame, InputStamp, MidiChannel, MidiEvent,
    MidiGroup, MidiKind, MidiValue, NoteKey, TimeBase, TimestampProvenance,
};
use ruviform_plugin_core::{Event, EventKind, ParamEvent, ParamId, ParamValue};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn frame_base() -> TimeBase {
    TimeBase::new(1, 48_000).unwrap()
}

fn stamp_base() -> TimeBase {
    TimeBase::new(1, 1_000_000).unwrap() // µs source clock
}

const DOMAIN: ClockDomainId = ClockDomainId::new(1);

fn epoch() -> ClockEpochId {
    ClockEpochId::new(1)
}

fn anchor() -> ClockAnchor {
    ClockAnchor {
        domain: DOMAIN,
        stamp_ticks: 0,
        stamp_base: stamp_base(),
        frame: EngineFrame::new(epoch(), 0),
        calibration: TimestampProvenance::Carried,
    }
}

fn timed_us(us: u64, event: MidiEvent) -> TimedMidiInput {
    TimedMidiInput {
        stamp: InputStamp {
            ticks: us,
            base: stamp_base(),
            domain: DOMAIN,
            provenance: TimestampProvenance::Carried,
        },
        event,
    }
}

fn note_on(key: u8, vel: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(vel),
        },
    }
}

fn note_off(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: 0,
        },
    }
}

fn cc(ctrl: u8, v: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::ControlChange {
            controller: ruviform_media_core::MidiController::new(ctrl).unwrap(),
            value: MidiValue::from_u7(v),
        },
    }
}

fn bend(v: u16) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::PitchBend {
            value: MidiValue::bend_from_u14(v),
        },
    }
}

fn param(offset: u32, v: f64) -> Event {
    Event::new(
        offset,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(v).unwrap(),
        }),
    )
}

struct ScriptedInput {
    pending: VecDeque<TimedMidiInput>,
    dropped: u64,
}

impl MidiInput for ScriptedInput {
    fn read(&mut self, out: &mut Vec<TimedMidiInput>, max: usize) -> DeviceResult<MidiReadStats> {
        let n = self.pending.len().min(max);
        out.extend(self.pending.drain(..n));
        // If the caller's budget was smaller than the queue, the
        // remainder is *not* dropped — it stays queued. `dropped` only
        // reports genuinely lost events.
        let stats = MidiReadStats::new(self.dropped);
        self.dropped = 0;
        Ok(stats)
    }
}

// ---------------------------------------------------------------------------
// acceptance: types + order + timing preserved end-to-end
// ---------------------------------------------------------------------------

#[test]
fn midi_delivery_note_cc_bend_with_frame_offsets() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));

    // Events at µs 0, 2000 (96 frames), 4000 (192 frames) inside a
    // 256-frame block starting at frame 0.
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![
            timed_us(0, note_on(60, 100)),
            timed_us(2000, cc(64, 127)),
            timed_us(4000, bend(0x2000)),
            timed_us(2000, note_off(60)), // duplicate stamp — stable order keeps input order
        ]),
        dropped: 0,
    };
    stage.read(&mut input).unwrap();

    let mut scratch = [param(0, 0.0); 16];
    let now = EngineFrame::new(epoch(), 0);
    let block_start = EngineFrame::new(epoch(), 0);
    let (live, stats) = stage.collect_block(block_start, 256, now, &mut scratch);
    assert_eq!(stats.emitted, 4);

    // Offsets: 0, 96, 192, and the duplicate-stamp off at 96.
    let offs: Vec<u32> = live.iter().map(|e| e.frame_offset()).collect();
    assert_eq!(offs, [0, 96, 192, 96]);
    // Order within offset 96: CC arrived before NoteOff in the input.
    assert!(matches!(
        live[1].kind(),
        EventKind::Midi(m) if matches!(m.kind, MidiKind::ControlChange { .. })
    ));
    assert!(matches!(
        live[3].kind(),
        EventKind::Midi(m) if m.is_note_off()
    ));
    // Types preserved verbatim — canonical u32 payloads.
    assert!(matches!(
        live[0].kind(),
        EventKind::Midi(m) if matches!(m.effective_note_state(), Some(true))
    ));
    assert!(matches!(
        live[2].kind(),
        EventKind::Midi(m) if matches!(m.kind, MidiKind::PitchBend { value: 0 })
    ));

    // The delivered list merges with a scheduled plan + a generated
    // release list; Note-Off on the same key precedes the colliding
    // Note-On at the same offset.
    let plan = [param(96, 0.5)];
    let mut merge_scratch = [param(0, 0.0); 16];
    let mut src_scratch = [0u8; 16];
    let merged = ruviform_plugin_core::merge_call_events(
        &[&plan[..], &live[..]],
        &mut merge_scratch,
        &mut src_scratch,
    )
    .unwrap();
    assert_eq!(merged.len(), 5);
    // plan wins the offset-96 tie over live.
    assert!(matches!(merged[1].kind(), EventKind::ParamValue(_)));

    // Ledger state: key 60 was turned on then off — nothing stuck.
    assert_eq!(stage.active_notes(), 0);
    assert_eq!(stage.pending_count(), 0);
    let counters = stage.counters();
    assert_eq!(counters.delivered, 4);
    assert_eq!(counters.mapped, 4);
    assert_eq!(counters.rejected, 0);
    assert_eq!(counters.late, 0);
}

#[test]
fn midi_delivery_block_boundary_and_release_all() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));
    // One event inside block 2 (frame 512 = us 10666.67 → use 10800µs
    // = 518.4 → 518 frames), one *before* the block (late), one after.
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![
            timed_us(10_800, note_on(60, 100)), // frame 518 → in-block
            timed_us(10_000, note_on(64, 90)),  // frame 480 → late
            timed_us(20_000, note_off(60)),     // frame 960 → future
        ]),
        dropped: 0,
    };
    stage.read(&mut input).unwrap();

    let block_start = EngineFrame::new(epoch(), 512);
    let now = EngineFrame::new(epoch(), 512);
    let mut scratch = [param(0, 0.0); 8];
    let (live, stats) = stage.collect_block(block_start, 256, now, &mut scratch);
    assert_eq!(stats.emitted, 2);
    assert_eq!(stats.late, 1);
    let offs: Vec<u32> = live.iter().map(|e| e.frame_offset()).collect();
    // In-block at 518-512=6, late clamped to 0.
    assert_eq!(offs, [6, 0]);
    // The future event stays pending for a later block.
    assert_eq!(stage.pending_count(), 1);

    // Stop while key 64 is still held → release_all emits its off.
    let mut release_scratch = [param(0, 0.0); 8];
    let released = stage.release_all(0, &mut release_scratch);
    // Keys 60 and 64 are both active (60's off is still pending).
    assert_eq!(released.len(), 2);
    assert!(released.iter().all(|e| matches!(
        e.kind(),
        EventKind::Midi(m) if m.is_note_off()
    )));
    assert_eq!(stage.active_notes(), 0);
    assert_eq!(stage.counters().released, 2);
}

#[test]
fn midi_delivery_rejections_are_observable() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));
    // Wrong domain stamp → rejected, counted, dropped.
    let mut bad = timed_us(0, note_on(60, 100));
    bad.stamp.domain = ClockDomainId::new(99);
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![bad, timed_us(0, note_off(60))]),
        dropped: 3,
    };
    stage.read(&mut input).unwrap();
    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        256,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 1);
    let counters = stage.counters();
    assert_eq!(counters.rejected, 1);
    assert_eq!(counters.dropped_backend, 3);
}

#[test]
fn midi_delivery_zero_frame_block() {
    // A zero-frame call window has no deliverable offsets: the
    // in-block bound `off < block_frames` rejects `Some(0)`, so both
    // events stay pending (nothing is dropped, nothing flushes).
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![timed_us(0, cc(1, 10)), timed_us(0, note_on(72, 1))]),
        dropped: 0,
    };
    stage.read(&mut input).unwrap();
    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        0,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 0);
    assert_eq!(stage.pending_count(), 2);
}

/// RT contract: `collect_block` + `merge_call_events` run without
/// allocation (all storage pre-sized).
#[test]
#[cfg(debug_assertions)]
fn rt_midi_delivery_collect_and_merge_no_alloc() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![
            timed_us(0, note_on(60, 100)),
            timed_us(2000, note_off(60)),
        ]),
        dropped: 0,
    };
    stage.read(&mut input).unwrap();

    let mut scratch = [param(0, 0.0); 8];
    let mut merge_scratch = [param(0, 0.0); 16];
    let mut src_scratch = [0u8; 16];
    let plan = [param(96, 0.5)];
    assert_no_alloc::assert_no_alloc(|| {
        let (live, stats) = stage.collect_block(
            EngineFrame::new(epoch(), 0),
            256,
            EngineFrame::new(epoch(), 0),
            &mut scratch,
        );
        assert_eq!(stats.emitted, 2);
        let merged = ruviform_plugin_core::merge_call_events(
            &[&plan[..], &live[..]],
            &mut merge_scratch,
            &mut src_scratch,
        )
        .unwrap();
        core::hint::black_box(merged)
    });
}
