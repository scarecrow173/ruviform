//! MIDI-leg cancellation fault tests (Issue #707): the MIDI-side
//! mapping of the Issue #628 audio `StreamFault::Cancellation`
//! observable onto the `MidiInput::read → DeviceResult` contract —
//! `Err(DeviceError::Backend("cancelled"))` on the cancelled read,
//! subscription termination, then `StreamInvalidated` on every later
//! read. Negative controls prove the `"cancelled"` error *value*
//! alone and the loss counters do not terminate.
//!
//! OS-L1 deterministic evidence only — real MIDI device/endpoint
//! fault timing stays EXT-L3 (`scarecrow173/ruviform#629`/`#630`).

mod backend_support;

use backend_support::*;
use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_input::mock::MockBackend;
use ruviform_audio_core::midi_input::{MidiInputInfo, MidiPortCapabilities, MidiPortOrigin};
use ruviform_audio_test::midi::{MidiCue, MidiScenario};
use ruviform_audio_test::pipeline::{MidiLeg, PipelineError};
use ruviform_media_core::{
    MediaTime, MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey, TimeBase,
};

const BLOCK: u32 = 64;
const EPOCH: ruviform_media_core::ClockEpochId = ruviform_media_core::ClockEpochId::new(1);

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

fn scenario(cues: Vec<MidiCue>) -> MidiScenario {
    MidiScenario::new(TimeBase::sample_rate(48_000).unwrap(), cues)
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

/// A leg on a caller-supplied port so faults can be armed before the
/// subscription opens (same pattern as `open_failure_surfaces_device_error`).
fn open_leg_on_port(
    cues: Vec<MidiCue>,
    port: ruviform_audio_core::midi_input::mock::MockPort,
) -> MidiLeg {
    MidiLeg::open_port(&scenario(cues), test_domain(), EPOCH, 64, port).expect("midi leg open")
}

fn scripted_port(id: &str) -> ruviform_audio_core::midi_input::mock::MockPort {
    let mut backend = MockBackend::new();
    backend.add_port(MidiInputInfo::new(
        id.to_owned(),
        format!("Fault Port {id}"),
        MidiPortOrigin::Internal,
        MidiPortCapabilities::stamped(false, test_domain()),
    ))
}

#[test]
fn pre_armed_cancel_reports_backend_error_then_stream_invalidates() {
    let port = scripted_port("cancel-pre");
    port.inject_cancelled();
    let mut leg = open_leg_on_port(vec![cue(10, 0, note_on(60))], port.clone());

    // The cancelled read surfaces `Backend("cancelled")` through the
    // production `DeviceError → PipelineError::Device` mapping — the
    // same classification the audio-side cancellation reports.
    let err = leg.pump_next(BLOCK).expect_err("cancelled read must fail");
    assert!(
        matches!(
            err,
            PipelineError::Device(DeviceError::Backend("cancelled"))
        ),
        "got {err:?}"
    );

    // Aborted delivery: the due cue was fed to the port but the
    // cancelled read delivered nothing — it stays queued, never
    // reaches the stage, and the feed is already drained.
    assert_eq!(leg.feed_remaining(), 0);
    assert_eq!(port.pending_len(), 1);
    assert!(port.is_terminated());
    assert!(!port.is_closed());
    assert_eq!(leg.counters().mapped, 0);

    // Every later read — through `pump_next` or `pump_inputs` — is
    // `StreamInvalidated`; the queued cue is never delivered.
    for err in [
        leg.pump_next(BLOCK).expect_err("terminated leg must fail"),
        leg.pump_inputs(u64::MAX)
            .expect_err("terminated leg must fail"),
    ] {
        assert!(
            matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
            "got {err:?}"
        );
    }
    assert_eq!(leg.counters().mapped, 0);
}

#[test]
fn cancelled_leg_reports_stream_invalidated_on_every_pump() {
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.port().inject_cancelled();
    leg.pump_next(BLOCK).expect_err("cancelled read must fail");
    // `pump_inputs` alone observes the same terminal error.
    let err = leg
        .pump_inputs(u64::MAX)
        .expect_err("terminated leg must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "got {err:?}"
    );
}

#[test]
fn mid_stream_cancel_aborts_in_flight_block_then_terminates() {
    let mut leg = open_leg(vec![cue(10, 0, note_on(60)), cue(100, 0, note_on(64))]);

    // Block 0 delivers normally.
    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(b0.events, vec![(10, note_on(60))]);
    assert_eq!(leg.counters().mapped, 1);

    // Arm cancellation, then pump block 1: the cue at tick 100 is due
    // inside [64,128) so `pump_inputs` pushes it to the port, but the
    // cancelled read aborts before delivery.
    leg.port().inject_cancelled();
    let err = leg.pump_next(BLOCK).expect_err("cancelled read must fail");
    assert!(
        matches!(
            err,
            PipelineError::Device(DeviceError::Backend("cancelled"))
        ),
        "got {err:?}"
    );
    assert_eq!(leg.feed_remaining(), 0);
    assert_eq!(leg.port().pending_len(), 1);
    assert!(leg.port().is_terminated());
    assert_eq!(leg.counters().mapped, 1); // block 1 mapped nothing

    // The failed block did not advance the cursor — but the
    // subscription is dead, so re-pumping reports StreamInvalidated
    // rather than redelivering.
    assert_eq!(leg.block_cursor(), u64::from(BLOCK));
    let err = leg.pump_next(BLOCK).expect_err("terminated leg must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "got {err:?}"
    );
    assert_eq!(leg.counters().mapped, 1);
}

#[test]
fn backend_cancelled_error_value_alone_does_not_terminate() {
    // Negative control: `Backend("cancelled")` scripted as a plain
    // one-shot read error carries the same classification as a real
    // cancellation but must NOT kill the subscription — termination
    // is the cancel fault's semantics, not the error value's.
    let mut leg = open_leg(vec![cue(10, 0, note_on(60)), cue(100, 0, note_on(64))]);
    leg.port().fail_next_read(DeviceError::Backend("cancelled"));

    let err = leg
        .pump_next(BLOCK)
        .expect_err("armed read error must fail");
    assert!(
        matches!(
            err,
            PipelineError::Device(DeviceError::Backend("cancelled"))
        ),
        "got {err:?}"
    );
    assert!(!leg.port().is_terminated());
    assert!(!leg.port().is_closed());
    // The failed read consumed nothing: the due cue is still queued
    // on the port and the block cursor did not advance.
    assert_eq!(leg.port().pending_len(), 1);
    assert_eq!(leg.block_cursor(), 0);

    // The next pump succeeds and delivers both cues in order — the
    // subscription survived the "cancelled"-valued error.
    let b0 = leg.pump_next(BLOCK).expect("block 0 retry");
    assert_eq!(b0.events, vec![(10, note_on(60))]);
    let b1 = leg.pump_next(BLOCK).expect("block 1");
    assert_eq!(b1.events, vec![(36, note_on(64))]);
    assert_eq!(leg.counters().mapped, 2);
    assert!(!leg.port().is_terminated());
}

#[test]
fn read_faults_report_in_arm_order_then_invalidate() {
    // FIFO ordering through the pipeline: an ordinary error, then
    // the terminal cancel — mirroring the audio-side ordered
    // error/cancelled/invalidated series.
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.port().fail_next_read(DeviceError::DeviceBusy);
    leg.port().inject_cancelled();

    let err = leg
        .pump_next(BLOCK)
        .expect_err("first armed fault must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::DeviceBusy)),
        "got {err:?}"
    );
    assert!(!leg.port().is_terminated());

    let err = leg.pump_next(BLOCK).expect_err("cancel must fail");
    assert!(
        matches!(
            err,
            PipelineError::Device(DeviceError::Backend("cancelled"))
        ),
        "got {err:?}"
    );
    assert!(leg.port().is_terminated());

    let err = leg.pump_next(BLOCK).expect_err("terminated leg must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "got {err:?}"
    );
}

#[test]
fn loss_counters_do_not_terminate_the_subscription() {
    // Negative control: `inject_dropped`/`inject_unsupported` surface
    // as `MidiReadStats` on an otherwise successful read — they are
    // loss accounting, not stream termination.
    let mut leg = open_leg(vec![cue(10, 0, note_on(60))]);
    leg.port().inject_dropped(2);
    leg.port().inject_unsupported(3);

    let b0 = leg.pump_next(BLOCK).expect("block 0");
    assert_eq!(b0.pump.backend_dropped, 2);
    assert_eq!(b0.pump.backend_unsupported, 3);
    assert_eq!(b0.events, vec![(10, note_on(60))]);
    assert!(!leg.port().is_terminated());
    assert!(!leg.port().is_closed());

    let b1 = leg.pump_next(BLOCK).expect("block 1");
    assert_eq!(b1.events.len(), 0);
    assert!(!leg.port().is_terminated());
}
