//! Pipeline MIDI-leg tests (Issue #628, 3rd slice): `MidiFeed` →
//! `MockPort`/`MockBackend` → `InputPump` (production `InputTx`
//! contract shape) → production `InputStage::pump`/`collect_block`
//! (`ClockAnchor` + `map_stamp_to_frame`) → observed block-relative
//! events vs independent expected offsets.
//!
//! OS-L1 deterministic evidence only — `InputStage::attach()` and
//! real endpoint wiring are out of scope (runtime-integration slice).

mod backend_support;

use backend_support::*;
use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_input::mock::MockBackend;
use ruviform_audio_core::midi_input::{
    MidiInputInfo, MidiPortCapabilities, MidiPortOrigin, TimedMidiInput,
};
use ruviform_audio_test::backend::MidiFeed;
use ruviform_audio_test::midi::MidiCue;
use ruviform_audio_test::pipeline::{MidiLeg, PipelineError};
use ruviform_media_core::{
    ClockDomainId, ClockEpochId, InputStamp, MediaTime, MidiChannel, MidiEvent, MidiGroup,
    MidiKind, MidiValue, NoteKey, TimeBase, TimestampProvenance,
};

const BLOCK: u32 = 64;
const EPOCH: ClockEpochId = ClockEpochId::new(1);

fn note_on(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(100),
        },
    }
}

fn note_off(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(64),
        },
    }
}

fn scenario(cues: Vec<MidiCue>) -> ruviform_audio_test::midi::MidiScenario {
    ruviform_audio_test::midi::MidiScenario::new(TimeBase::sample_rate(48_000).unwrap(), cues)
}

fn cue(ticks: i64, seq: u64, event: MidiEvent) -> MidiCue {
    MidiCue {
        position: MediaTime::new(ticks, TimeBase::sample_rate(48_000).unwrap()),
        source_sequence: seq,
        event,
    }
}

fn open_leg(cues: Vec<MidiCue>) -> MidiLeg {
    MidiLeg::open(&scenario(cues), test_domain(), EPOCH, 64).expect("midi leg open")
}

/// A `TimedMidiInput` stamped directly — for late/rejected/fault
/// injections the scenario feed cannot express.
fn stamped(ticks: u64, domain: ClockDomainId, event: MidiEvent) -> TimedMidiInput {
    TimedMidiInput {
        stamp: InputStamp {
            ticks,
            base: TimeBase::sample_rate(48_000).unwrap(),
            domain,
            provenance: TimestampProvenance::Carried,
        },
        event,
    }
}

#[test]
fn cues_arrive_at_independent_expected_block_offsets() {
    let leg_cues = vec![
        cue(10, 0, note_on(60)),
        cue(10, 1, note_on(64)), // same tick — source_sequence order
        cue(100, 0, note_off(60)),
        cue(200, 0, note_on(67)),
    ];
    let mut leg = open_leg(leg_cues);

    // Block [0,64): cues at ticks 10,10.
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(b0.block_start.frame(), 0);
    assert_eq!(b0.events, vec![(10, note_on(60)), (10, note_on(64))]);
    assert_eq!(b0.stats.emitted, 2);
    assert_eq!(b0.stats.late, 0);
    assert_eq!(b0.fed, 2);
    assert_eq!(b0.pump.pushed, 2);
    assert_eq!(b0.moved, 2);

    // Block [64,128): cue at tick 100 → offset 36.
    let b1 = leg.pump_next(BLOCK).expect("block 1");
    assert_eq!(b1.block_start.frame(), 64);
    assert_eq!(b1.events, vec![(36, note_off(60))]);

    // Block [128,192): nothing due.
    let b2 = leg.pump_next(BLOCK).expect("block 2");
    assert_eq!(b2.events.len(), 0);
    assert_eq!(b2.stats.emitted, 0);

    // Block [192,256): cue at tick 200 → offset 8.
    let b3 = leg.pump_next(BLOCK).expect("block 3");
    assert_eq!(b3.events, vec![(8, note_on(67))]);
    assert_eq!(leg.feed_remaining(), 0);

    let counters = leg.counters();
    assert_eq!(counters.mapped, 4);
    assert_eq!(counters.delivered, 4);
    assert_eq!(counters.late, 0);
    assert_eq!(counters.rejected, 0);
    assert_eq!(counters.anchor_rejected, 0);
    assert_eq!(leg.pending_count(), 0);
}

#[test]
fn out_of_order_cue_list_still_delivers_in_stamp_order() {
    // Cues listed backwards — the feed sorts by (ticks, source_sequence)
    // exactly like the production TransportPlanBuilder.
    let mut leg = open_leg(vec![
        cue(50, 5, note_on(62)),
        cue(10, 9, note_on(60)),
        cue(10, 2, note_on(64)),
    ]);
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(
        b0.events,
        vec![(10, note_on(64)), (10, note_on(60)), (50, note_on(62))]
    );
}

#[test]
fn late_stamp_is_clamped_to_offset_zero_and_counted() {
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.pump_next(BLOCK).expect("block 0"); // cursor → 64
    // A stamp for frame 5 arrives after block 0 already ran — the
    // production late-event policy delivers it at offset 0, counted.
    leg.port().push(stamped(5, test_domain(), note_on(72)));
    let b1 = leg.pump_next(BLOCK).expect("block 1");
    assert_eq!(b1.events, vec![(0, note_on(72))]);
    assert_eq!(b1.stats.late, 1);
    assert_eq!(leg.counters().late, 1);
}

#[test]
fn unmapped_domain_stamp_is_rejected_and_counted() {
    let mut leg = open_leg(vec![]);
    leg.port()
        .push(stamped(10, ClockDomainId::new(99), note_on(60)));
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(b0.events.len(), 0);
    let counters = leg.counters();
    assert_eq!(counters.rejected, 1);
    assert_eq!(counters.anchor_rejected, 1);
    assert_eq!(counters.mapped, 0);
    assert_eq!(leg.pending_count(), 0); // rejected events are consumed
}

#[test]
fn backend_drops_and_unsupported_are_observable_per_pump_and_cumulative() {
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.port().inject_dropped(2);
    leg.port().inject_unsupported(3);
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    // The scripted backend-side counters surface through the pump —
    // the event itself still delivers.
    assert_eq!(b0.pump.backend_dropped, 2);
    assert_eq!(b0.pump.backend_unsupported, 3);
    assert_eq!(b0.events.len(), 1);
    assert_eq!(leg.pump_counters(), (2, 3));

    // Counters are per-read: a second pump reports zero new loss.
    let b1 = leg.pump_next(BLOCK).expect("block 1");
    assert_eq!(b1.pump.backend_dropped, 0);
    assert_eq!(b1.pump.backend_unsupported, 0);
    assert_eq!(leg.pump_counters(), (2, 3));
}

#[test]
fn open_failure_surfaces_device_error() {
    let mut backend = MockBackend::new();
    let port = backend.add_port(MidiInputInfo::new(
        "fail-port".to_owned(),
        "Fail Port".to_owned(),
        MidiPortOrigin::Internal,
        MidiPortCapabilities::stamped(false, test_domain()),
    ));
    port.fail_next_open(DeviceError::DeviceBusy);
    let err = MidiLeg::open_port(&scenario(vec![]), test_domain(), EPOCH, 64, port)
        .expect_err("open must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::DeviceBusy)),
        "expected DeviceBusy, got {err:?}"
    );
}

#[test]
fn invalid_channel_capacity_is_config_error() {
    let mut backend = MockBackend::new();
    let port = backend.add_port(MidiInputInfo::new(
        "cap-port".to_owned(),
        "Cap Port".to_owned(),
        MidiPortOrigin::Internal,
        MidiPortCapabilities::stamped(false, test_domain()),
    ));
    for bad in [0usize, 1025] {
        let err = MidiLeg::open_port(&scenario(vec![]), test_domain(), EPOCH, bad, port.clone())
            .expect_err("capacity must be rejected");
        assert!(matches!(err, PipelineError::Config(_)), "got {err:?}");
    }
}

#[test]
fn close_makes_next_pump_fail_with_stream_invalidated() {
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.close();
    assert!(leg.port().is_closed());
    let err = leg
        .pump_inputs(u64::MAX)
        .expect_err("closed subscription must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "got {err:?}"
    );
}

#[test]
fn unrepresentable_cue_positions_are_counted_not_silent() {
    // Negative position → `convert_exact` fails → feed.skipped,
    // never emitted into the port.
    let mut leg = open_leg(vec![cue(-5, 0, note_on(60)), cue(10, 0, note_on(64))]);
    assert_eq!(leg.feed_skipped(), 1);
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(b0.events, vec![(10, note_on(64))]);
    assert_eq!(leg.feed_remaining(), 0);
}

#[test]
fn channel_capacity_backpressure_defers_then_delivers_in_order() {
    // Queue-full at the non-RT→RT handoff: `channel_capacity = 4`
    // against ten pushed events forces `InputPump` staging to defer —
    // counted per pump, flushed by later pumps, never silently lost.
    // Events stamped inside already-passed blocks then surface through
    // the production late-event policy (clamp to offset 0, counted).
    let mut backend = MockBackend::new();
    let port = backend.add_port(MidiInputInfo::new(
        "defer-port".to_owned(),
        "Defer Port".to_owned(),
        MidiPortOrigin::Internal,
        MidiPortCapabilities::stamped(false, test_domain()),
    ));
    let mut leg =
        MidiLeg::open_port(&scenario(vec![]), test_domain(), EPOCH, 4, port).expect("leg open");
    for t in 1..=10u64 {
        leg.port().push(stamped(
            t,
            test_domain(),
            note_on(u8::try_from(t).expect("key in range")),
        ));
    }

    let mut deferred = Vec::new();
    let mut keys = Vec::new();
    for _ in 0..3 {
        let obs = leg.pump_block(u64::MAX, BLOCK).expect("block");
        deferred.push(obs.pump.deferred);
        for (_offset, event) in &obs.events {
            match event.kind {
                MidiKind::NoteOn { key, .. } => keys.push(key.get()),
                ref other => panic!("unexpected event kind {other:?}"),
            }
        }
    }
    assert_eq!(deferred, vec![6, 2, 0]);
    assert_eq!(keys, (1..=10).collect::<Vec<_>>());
    // Six events arrived after their stamped block had passed —
    // production late policy delivered them clamped, counted.
    assert_eq!(leg.counters().late, 6);
    assert_eq!(leg.pending_count(), 0);
}

#[test]
fn feed_stamp_domain_matches_leg_anchor_domain() {
    // The feed stamps cues with the caller-chosen domain; the leg
    // publishes its anchor on the same domain — the identity mapping
    // the expected offsets rely on. A stamp on any other domain is
    // rejected (covered by `unmapped_domain_stamp_is_rejected`).
    let feed = MidiFeed::new(&scenario(vec![cue(7, 0, note_on(60))]), test_domain());
    assert_eq!(feed.domain(), test_domain());
    let mut leg = open_leg(vec![]);
    leg.port().push(stamped(7, feed.domain(), note_on(60)));
    let b = leg.pump_next(BLOCK).expect("block");
    assert_eq!(b.events, vec![(7, note_on(60))]);
}
