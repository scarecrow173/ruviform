//! MIDI trace tests through the production `TransportScheduler`
//! (Issue #628).

use ruviform_audio_test::midi::{MidiCue, MidiScenario, MidiTraceConfig, run_midi_trace};
use ruviform_audio_test::oracle::{
    midi_expected_playthrough, midi_trace_eq, midi_trace_frames_monotonic,
};
use ruviform_media_core::{
    ApplyStatus, BlockContract, DeliveryPolicy, InBlockTransportSupport, MediaDuration, MediaRange,
    MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey,
    ScheduleBudget, TimeBase, TransportCommand,
};

fn base() -> TimeBase {
    TimeBase::sample_rate(48_000).unwrap()
}

fn ch() -> MidiChannel {
    MidiChannel::new(0).unwrap()
}

fn note_on(key: u8, vel: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(vel),
        },
    }
}

fn note_off(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: 0,
        },
    }
}

fn cc(controller: u8, value: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::ControlChange {
            controller: MidiController::new(controller).unwrap(),
            value: MidiValue::from_u7(value),
        },
    }
}

fn cue(pos: i64, seq: u64, event: MidiEvent) -> MidiCue {
    MidiCue {
        position: MediaTime::new(pos, base()),
        source_sequence: seq,
        event,
    }
}

fn config(total_frames: u64) -> MidiTraceConfig {
    MidiTraceConfig {
        block_frames: 64,
        total_frames,
        contract: BlockContract::new(1, 4_096, InBlockTransportSupport::Unknown),
        policy: DeliveryPolicy::Exact,
        budget: ScheduleBudget::new(64, 1_024, 16, 0),
        commands: Vec::new(),
        trace_capacity: 256,
    }
}

#[test]
fn playthrough_trace_matches_independent_oracle() {
    let scenario = MidiScenario::new(
        base(),
        vec![
            cue(300, 3, note_on(60, 100)),
            cue(100, 1, note_on(62, 90)),
            cue(200, 2, cc(7, 64)),
            cue(400, 4, note_off(60)),
            cue(100, 0, note_on(64, 80)), // same position, lowest seq first
        ],
    );
    let trace = run_midi_trace(&scenario, &config(512)).unwrap();

    assert_eq!(trace.blocks_run(), 8);
    assert_eq!(trace.undeliverable_spans(), 0);
    assert!(trace.faults().is_empty(), "faults: {:?}", trace.faults());
    assert!(midi_trace_frames_monotonic(trace.entries()));

    // Oracle: independently sorted + framed, no scheduler involvement.
    let expected = midi_expected_playthrough(base(), &scenario.cues, 512);
    assert!(midi_trace_eq(trace.entries(), &expected));

    // Spot-check concrete frames/values.
    let e = trace.entries();
    assert_eq!(e[0].frame.frame(), 100);
    assert_eq!(e[0].source_sequence, 0);
    assert_eq!(e[1].frame.frame(), 100);
    assert_eq!(e[1].source_sequence, 1);
    assert_eq!(e[2].frame.frame(), 200);
    assert_eq!(e[4].frame.frame(), 400);
    // velocity went through the production canonical scale
    match e[0].event.kind {
        MidiKind::NoteOn { velocity, .. } => {
            assert_eq!(velocity, MidiValue::from_u7(80));
        }
        _ => panic!("expected NoteOn"),
    }
}

#[test]
fn events_beyond_total_frames_do_not_fire() {
    let scenario = MidiScenario::new(
        base(),
        vec![
            cue(100, 0, note_on(60, 100)),
            cue(9_999, 1, note_on(62, 100)),
        ],
    );
    let trace = run_midi_trace(&scenario, &config(512)).unwrap();
    assert_eq!(trace.entries().len(), 1);
    assert_eq!(trace.entries()[0].frame.frame(), 100);
}

#[test]
fn seek_skips_intermediate_positions() {
    // Events at positions 100 (fires), 350 (skipped by seek 300→400),
    // 450 (fires at frame 350 = 300 + (450-400)).
    let scenario = MidiScenario::new(
        base(),
        vec![
            cue(100, 0, note_on(60, 100)),
            cue(350, 1, note_on(62, 100)),
            cue(450, 2, note_on(64, 100)),
        ],
    );
    let mut cfg = config(512);
    cfg.commands
        .push((300, TransportCommand::Seek(MediaTime::new(400, base()))));
    let trace = run_midi_trace(&scenario, &cfg).unwrap();

    let frames: Vec<u64> = trace.entries().iter().map(|e| e.frame.frame()).collect();
    assert_eq!(frames, [100, 350]);
    // position 350 event never delivered; position 450 delivered at 350.
    let e450 = trace
        .entries()
        .iter()
        .find(|e| e.source_sequence == 2)
        .unwrap();
    assert_eq!(e450.frame.frame(), 350);
    assert!(trace.entries().iter().all(|e| e.source_sequence != 1));

    // The Seek receipt reached Applied.
    assert!(
        trace
            .receipts()
            .iter()
            .any(|r| { r.status == ApplyStatus::Applied && r.requested_frame.frame() == 300 })
    );
}

#[test]
fn loop_region_repeats_in_loop_events() {
    // Loop over position ticks [100, 200); event at 150 fires once per
    // occurrence while the transport loops.
    let scenario = MidiScenario::new(base(), vec![cue(150, 0, note_on(60, 100))]);
    let mut cfg = config(1_024);
    cfg.commands.push((
        0,
        TransportCommand::SetLoop(MediaRange::new(
            MediaTime::new(100, base()),
            MediaDuration::new(100, base()),
        )),
    ));
    let trace = run_midi_trace(&scenario, &cfg).unwrap();

    let in_loop: Vec<_> = trace
        .entries()
        .iter()
        .filter(|e| e.event == note_on(60, 100))
        .collect();
    // Must fire at least twice (initial pass + ≥1 loop wrap) with
    // distinct occurrences.
    assert!(in_loop.len() >= 2, "entries: {}", in_loop.len());
    assert_ne!(in_loop[0].occurrence, in_loop[1].occurrence);
    // Loop wraps are legal boundaries here → no undeliverable spans.
    assert_eq!(trace.undeliverable_spans(), 0);
}

#[test]
fn unrepresentable_position_is_rejected_at_compile() {
    // A position in a different, non-convertible timebase fails the
    // production builder's exact-representation check.
    let weird = TimeBase::new(1, 7).unwrap(); // 1/7 s — not frame-aligned
    let scenario = MidiScenario::new(
        base(),
        vec![MidiCue {
            position: MediaTime::new(1, weird),
            source_sequence: 0,
            event: note_on(60, 100),
        }],
    );
    assert!(scenario.compile().is_err());
}

#[test]
fn zero_block_frames_is_rejected() {
    let scenario = MidiScenario::new(base(), vec![]);
    let mut cfg = config(64);
    cfg.block_frames = 0;
    assert!(run_midi_trace(&scenario, &cfg).is_err());
}

#[test]
fn pause_stops_event_delivery() {
    let scenario = MidiScenario::new(
        base(),
        vec![cue(100, 0, note_on(60, 100)), cue(400, 1, note_on(62, 100))],
    );
    let mut cfg = config(512);
    cfg.commands.push((200, TransportCommand::Pause));
    cfg.commands.push((300, TransportCommand::Play));
    let trace = run_midi_trace(&scenario, &cfg).unwrap();
    // Position freezes at 200 from frame 200→300: event at 400 fires
    // 100 frames later at frame 400 still? Position advance pauses —
    // event fires when position reaches 400, i.e. frame 500.
    let e = trace
        .entries()
        .iter()
        .find(|e| e.source_sequence == 1)
        .unwrap();
    assert_eq!(e.frame.frame(), 500);
}
