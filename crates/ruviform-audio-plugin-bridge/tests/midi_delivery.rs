//! Issue #188 `PoC` — timed `MIDI` input end-to-end:
//! `MidiInput` → `InputStage` (stamp→frame map) →
//! `merge_call_events` (evict/release/chase/plan/live) → `EventList`.
//!
//! Covers: `NoteOn`/`NoteOff`, CC, `PitchBend` type+order
//! preservation; duplicate timestamps (stable order); block-boundary
//! behavior (in-block/late/future/rejected); `NoteOff` priority on
//! colliding keys; anchor republish pending preservation and
//! `anchor_rejected` classification (Issue #253 V12); observable
//! overflow/drop counters; and the RT no-allocation contract for
//! `collect_block` + merge. Note-state tracking/release authority
//! lives in `PluginNode`'s delivered-note ledger — covered by
//! `midi_wiring.rs` (Issue #253 V11).

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
    unsupported: u64,
}

impl MidiInput for ScriptedInput {
    fn read(&mut self, out: &mut Vec<TimedMidiInput>, max: usize) -> DeviceResult<MidiReadStats> {
        let n = self.pending.len().min(max);
        out.extend(self.pending.drain(..n));
        // If the caller's budget was smaller than the queue, the
        // remainder is *not* dropped — it stays queued. `dropped` only
        // reports genuinely lost events.
        let stats = MidiReadStats::with_unsupported(self.dropped, self.unsupported);
        self.dropped = 0;
        self.unsupported = 0;
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
        unsupported: 0,
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
    .unwrap()
    .events;
    assert_eq!(merged.len(), 5);
    // plan wins the offset-96 tie over live.
    assert!(matches!(merged[1].kind(), EventKind::ParamValue(_)));

    // The stage holds no note state (Issue #253 V11 — the node
    // ledger is the sole authority); nothing stays pending.
    assert_eq!(stage.pending_count(), 0);
    let counters = stage.counters();
    assert_eq!(counters.delivered, 4);
    assert_eq!(counters.mapped, 4);
    assert_eq!(counters.rejected, 0);
    assert_eq!(counters.anchor_rejected, 0);
    assert_eq!(counters.late, 0);
}

#[test]
fn midi_delivery_block_boundary_and_pending() {
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
        unsupported: 0,
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
        unsupported: 0,
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
    // Wrong domain is an anchor-coverage rejection (V12).
    assert_eq!(counters.anchor_rejected, 1);
    assert_eq!(counters.dropped_backend, 3);
}

/// Issue #253 V12 — with no anchor published, `pending` waits
/// unmapped (no rejection, no consumption); publishing an anchor maps
/// the queued events under it.
#[test]
fn midi_delivery_anchor_none_pending_waits_then_maps() {
    let mut stage = InputStage::new(frame_base());
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![
            timed_us(0, note_on(60, 100)),
            timed_us(2_000, note_off(60)),
        ]),
        dropped: 0,
        unsupported: 0,
    };
    stage.read(&mut input).unwrap();

    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        256,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 0);
    assert_eq!(stage.pending_count(), 2);
    let counters = stage.counters();
    assert_eq!(counters.mapped, 0);
    assert_eq!(counters.rejected, 0);
    assert_eq!(counters.anchor_rejected, 0);

    // The anchor arrives — the same pending events now map and deliver.
    stage.publish_anchor(Some(anchor()));
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        256,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 2);
    let offs: Vec<u32> = live.iter().map(|e| e.frame_offset()).collect();
    assert_eq!(offs, [0, 96]);
    assert_eq!(stage.pending_count(), 0);
}

/// Issue #253 V12 — republishing a *recalibrated* anchor re-maps the
/// pending queue under the new anchor: mapping authority is always
/// the anchor current at `collect_block`, never the one published
/// when the event was pushed.
#[test]
fn midi_delivery_anchor_republish_remaps_pending() {
    let mut stage = InputStage::new(frame_base());
    // Anchor A: stamp 0µs ↔ frame 0 — a 1000µs stamp maps to frame 48.
    stage.publish_anchor(Some(anchor()));
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![timed_us(1_000, note_on(60, 100))]),
        dropped: 0,
        unsupported: 0,
    };
    stage.read(&mut input).unwrap();

    // Republish anchor B *before* any collect: stamp 1000µs ↔ frame 0,
    // so the same stamp now maps to frame 0 — not 48.
    stage.publish_anchor(Some(ClockAnchor {
        stamp_ticks: 1_000,
        ..anchor()
    }));
    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        256,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].frame_offset(), 0);
    assert_eq!(stage.pending_count(), 0);
}

/// Issue #253 V12 — republishing an anchor for a *different domain*
/// preserves the pending queue: stamps the new anchor covers are
/// delivered, stamps it cannot cover are defined rejections counted
/// as `anchor_rejected` — nothing is silently consumed by the
/// republish itself.
#[test]
fn midi_delivery_anchor_republish_domain_switch() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor())); // domain 1
    let mut foreign = timed_us(0, cc(1, 10));
    foreign.stamp.domain = ClockDomainId::new(99);
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![timed_us(0, note_on(60, 100)), foreign]),
        dropped: 0,
        unsupported: 0,
    };
    stage.read(&mut input).unwrap();

    // Switch the anchor's domain before the first collect.
    stage.publish_anchor(Some(ClockAnchor {
        domain: ClockDomainId::new(99),
        ..anchor()
    }));
    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        256,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    // The domain-99 CC delivered; the domain-1 note-on is a defined
    // anchor-coverage rejection — both survived the republish itself.
    assert_eq!(live.len(), 1);
    assert!(matches!(
        live[0].kind(),
        EventKind::Midi(m) if matches!(m.kind, MidiKind::ControlChange { .. })
    ));
    let counters = stage.counters();
    assert_eq!(counters.rejected, 1);
    assert_eq!(counters.anchor_rejected, 1);
    assert_eq!(counters.delivered, 1);
    assert_eq!(stage.pending_count(), 0);
}

/// Issue #253 V12 — rejection classification: anchor domain/base/
/// epoch coverage failures count `anchor_rejected`; `BeforeAnchor`
/// counts `rejected` only.
#[test]
fn midi_delivery_anchor_rejection_classification() {
    let mut stage = InputStage::new(frame_base());
    // Anchor claims stamp 5000µs ↔ frame 240.
    stage.publish_anchor(Some(ClockAnchor {
        stamp_ticks: 5_000,
        frame: EngineFrame::new(epoch(), 240),
        ..anchor()
    }));
    let mut foreign = timed_us(6_000, note_on(64, 90));
    foreign.stamp.domain = ClockDomainId::new(99);
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![
            timed_us(1_000, note_on(60, 100)), // BeforeAnchor — not anchor coverage
            foreign,                           // UnmappedDomain — anchor coverage
            timed_us(6_000, note_off(60)),     // maps to frame 288 — in-block
        ]),
        dropped: 0,
        unsupported: 0,
    };
    stage.read(&mut input).unwrap();

    let mut scratch = [param(0, 0.0); 8];
    let (live, _) = stage.collect_block(
        EngineFrame::new(epoch(), 0),
        512,
        EngineFrame::new(epoch(), 0),
        &mut scratch,
    );
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].frame_offset(), 288);
    let counters = stage.counters();
    assert_eq!(counters.rejected, 2);
    assert_eq!(counters.anchor_rejected, 1);
    assert_eq!(counters.mapped, 1);
    assert_eq!(counters.delivered, 1);
}

/// Issue #253 — backend-reported untranslatable traffic must not die
/// at `InputStage::read`: it accumulates on
/// `InputStageCounters::unsupported_backend`, separately from
/// `dropped_backend`, and never perturbs event delivery.
#[test]
fn midi_delivery_unsupported_is_observable() {
    let mut stage = InputStage::new(frame_base());
    stage.publish_anchor(Some(anchor()));
    let mut input = ScriptedInput {
        pending: VecDeque::from(vec![timed_us(0, note_on(60, 100))]),
        dropped: 3,
        unsupported: 4,
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
    assert_eq!(counters.unsupported_backend, 4);
    assert_eq!(counters.dropped_backend, 3);
    assert_eq!(counters.delivered, 1);

    // Cumulative across reads — the second batch adds, not replaces.
    let mut input = ScriptedInput {
        pending: VecDeque::new(),
        dropped: 0,
        unsupported: 3,
    };
    stage.read(&mut input).unwrap();
    assert_eq!(stage.counters().unsupported_backend, 7);
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
        unsupported: 0,
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
        unsupported: 0,
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
        .unwrap()
        .events;
        core::hint::black_box(merged)
    });
}
