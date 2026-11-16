//! Loopback round-trip verifier — Issue #629 third slice.
//!
//! `#[ignore]`-gated manual real-device lane: the canonical lane is
//! `mise run test-midi-virtual-endpoint` (`--test loopback --
//! --ignored`); `test-midi-real-device` also collects it as part of
//! its all-`#[ignore]` sweep. Exercises the production
//! `midi_input`/`midi_output` port traits end to end over the real
//! OS backend: enumerate → name-match a loopback candidate pair →
//! open both → send a fixed channel-voice sequence → drain `read` →
//! verify payload, order, and timestamp invariants against an
//! independent expected list.
//!
//! Fail-closed by contract: with no provisioned loopback pair (the
//! common case — `WinMM` reports `MidiPortOrigin::Unknown` and cannot
//! prove a port is virtual), the test records `verdict: NotRun` and
//! returns without asserting. A bidirectional name match is a
//! *candidate*, not proof of a virtual endpoint — a Pass here is
//! OS-L1 "real OS API round-trip" evidence, never
//! `environment_kind=virtual-os-endpoint`.

use std::time::{Duration, Instant};

use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputPort, TimedMidiInput,
};
use ruviform_audio_core::midi_output::{MidiOutput, MidiOutputEnumerator, MidiOutputPort};
use ruviform_media_core::{
    ClockDomainId, MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiProgram,
    MidiValue, NoteKey, TimestampProvenance,
};
use ruviform_midi_device::backend;

/// The fixed channel-voice sequence sent through the loopback and
/// verified on the receive side. Channel 3 and a non-center pitch
/// bend keep the values distinctive; every value is built from its
/// wire width (`from_u7`/`bend_from_u14`) so encode→decode is the
/// identity the codec contract guarantees. This list is an
/// independent literal oracle — the adapter under test never
/// generates expected values.
fn expected_channel_voice_sequence() -> [MidiEvent; 4] {
    let channel = MidiChannel::new(3).unwrap();
    let key = NoteKey::new(60).unwrap();
    [
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::NoteOn {
                key,
                velocity: MidiValue::from_u7(100),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::ControlChange {
                controller: MidiController::new(10).unwrap(),
                value: MidiValue::from_u7(90),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::PitchBend {
                value: MidiValue::bend_from_u14(0x2A00),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::NoteOff {
                key,
                velocity: MidiValue::from_u7(64),
            },
        },
    ]
}

/// Pairs each output port with the first still-unclaimed input port
/// of the same `info().name` — the same bidirectional-name signature
/// the capability probe reports as a loopback-style candidate. Port
/// ids differ per direction (`winmm:{i}` vs `winmm-out:{i}`), so the
/// name is the only honest cross-direction key `WinMM` exposes.
fn loopback_pair_candidates(
    mut inputs: Vec<Box<dyn MidiInputPort>>,
    outputs: Vec<Box<dyn MidiOutputPort>>,
) -> Vec<(Box<dyn MidiInputPort>, Box<dyn MidiOutputPort>)> {
    let mut pairs = Vec::new();
    for out_port in outputs {
        let out_name = out_port.info().name.clone();
        if let Some(index) = inputs.iter().position(|p| p.info().name == out_name) {
            let in_port = inputs.swap_remove(index);
            pairs.push((in_port, out_port));
        }
    }
    pairs
}

/// Order-preserving subsequence check: every expected event must
/// appear in `observed` in order, but extra traffic between matches
/// is tolerated (a loopback may carry unrelated messages).
fn is_ordered_subsequence(observed: &[TimedMidiInput], expected: &[MidiEvent]) -> bool {
    let mut cursor = observed.iter();
    expected
        .iter()
        .all(|want| cursor.any(|got| &got.event == want))
}

/// Drains the open input until the expected sequence is observed as
/// an ordered subsequence or the deadline expires. Returns the
/// accumulated events plus the observable loss counters.
fn drain_until_match(
    input: &mut dyn MidiInput,
    expected: &[MidiEvent],
    deadline: Duration,
) -> (Vec<TimedMidiInput>, u64, u64) {
    let started = Instant::now();
    let mut observed = Vec::new();
    let mut buf = Vec::new();
    let (mut dropped, mut unsupported) = (0u64, 0u64);
    while started.elapsed() < deadline {
        buf.clear();
        match input.read(&mut buf, 64) {
            Ok(stats) => {
                dropped += stats.dropped;
                unsupported += stats.unsupported;
                observed.append(&mut buf);
            }
            Err(error) => panic!("verdict: Fail — read on open input failed: {error:?}"),
        }
        if is_ordered_subsequence(&observed, expected) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    (observed, dropped, unsupported)
}

/// Enumerates both directions through the production enumerators and
/// selects the first loopback pair candidate. Every `NotRun` verdict
/// is printed here; `None` means the environment cannot run the
/// verifier (honest skip — never a failure).
fn select_loopback_pair() -> Option<(Box<dyn MidiInputPort>, Box<dyn MidiOutputPort>)> {
    let backend_in: Box<dyn MidiInputEnumerator> = Box::new(backend());
    let backend_out: Box<dyn MidiOutputEnumerator> = Box::new(backend());
    let inputs = match backend_in.input_ports() {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("verdict: NotRun — input enumeration unavailable ({error:?})");
            return None;
        }
    };
    let outputs = match backend_out.output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("verdict: NotRun — output enumeration unavailable ({error:?})");
            return None;
        }
    };
    eprintln!(
        "enumerated: {} input ports, {} output ports",
        inputs.len(),
        outputs.len()
    );

    let mut pairs = loopback_pair_candidates(inputs, outputs);
    if pairs.is_empty() {
        eprintln!(
            "verdict: NotRun — NOT_PROVISIONED (no bidirectional name match; \
             provision a loopMIDI/Windows MIDI Services loopback pair first)"
        );
        return None;
    }
    for (index, (in_port, out_port)) in pairs.iter().enumerate() {
        let in_info = in_port.info();
        let out_info = out_port.info();
        eprintln!(
            "  candidate[{index}]: in id={} name={:?} | out id={} name={:?}",
            in_info.id, in_info.name, out_info.id, out_info.name
        );
    }
    Some(pairs.swap_remove(0))
}

/// Timestamp contract for a stamped port — the same invariants
/// `ports.rs` fixes: declared clock domain, `Carried` provenance,
/// non-decreasing driver ticks. Unstamped ports skip the check.
fn assert_stamp_contract(
    observed: &[TimedMidiInput],
    provides_timestamps: bool,
    clock_domain: Option<ClockDomainId>,
) {
    if !provides_timestamps {
        return;
    }
    let declared_domain = clock_domain.expect("stamped port declares a domain");
    let mut prev_ticks = None;
    for event in observed {
        assert_eq!(
            event.stamp.domain, declared_domain,
            "events must be stamped in the port's declared domain"
        );
        assert_eq!(event.stamp.provenance, TimestampProvenance::Carried);
        if let Some(prev) = prev_ticks {
            assert!(
                event.stamp.ticks >= prev,
                "driver ticks must be non-decreasing"
            );
        }
        prev_ticks = Some(event.stamp.ticks);
    }
}

/// Loopback round-trip verifier — the Issue #629 AC2 evidence path.
///
/// Manual opt-in: `cargo test -p ruviform-midi-device --test loopback
/// -- --ignored` (canonical lane: `mise run
/// test-midi-virtual-endpoint`).
/// Honest `NotRun` when the backend is unavailable, a direction has
/// no ports, or no bidirectional name match exists — a name match
/// alone is a candidate, and a matched pair that fails to round-trip
/// is a real `Fail` (panic with the observed sequence dumped).
#[test]
#[ignore = "requires a provisioned MIDI loopback pair; manual opt-in check"]
fn loopback_round_trip() {
    let Some((in_port, out_port)) = select_loopback_pair() else {
        return;
    };
    let in_info = in_port.info();
    let out_info = out_port.info();
    let mut input: Box<dyn MidiInput> = in_port.open().unwrap_or_else(|error| {
        panic!("verdict: Fail — matched input port failed to open: {error:?}")
    });
    // Input opens first so nothing sent below can arrive before the
    // subscription exists.
    let mut output: Box<dyn MidiOutput> = out_port.open().unwrap_or_else(|error| {
        panic!("verdict: Fail — matched output port failed to open: {error:?}")
    });

    let expected = expected_channel_voice_sequence();
    let started = Instant::now();
    for event in &expected {
        output
            .send(*event)
            .expect("send loopback event through matched output port");
        // WinMM short messages are queued driver-side; a small gap
        // keeps the loopback's forwarding observable per event.
        std::thread::sleep(Duration::from_millis(30));
    }

    let (observed, dropped, unsupported) =
        drain_until_match(&mut *input, &expected, Duration::from_secs(3));

    if !is_ordered_subsequence(&observed, &expected) {
        eprintln!("verdict: Fail — expected sequence did not round-trip");
        eprintln!("  expected: {expected:?}");
        eprintln!(
            "  observed ({} events): {:?}",
            observed.len(),
            observed.iter().map(|e| &e.event).collect::<Vec<_>>()
        );
        panic!("loopback round-trip did not reproduce the expected sequence");
    }

    // The tick span is a driver-domain observation only; the input
    // clock domain is not comparable to send wall-clock time.
    assert_stamp_contract(
        &observed,
        in_info.capabilities.provides_timestamps,
        in_info.capabilities.clock_domain,
    );
    let tick_span = match (observed.first(), observed.last()) {
        (Some(first), Some(last)) => Some(last.stamp.ticks - first.stamp.ticks),
        _ => None,
    };
    eprintln!("verdict: Pass — round-trip observed through real OS ports");
    eprintln!(
        "  pair: in id={} name={:?} | out id={} name={:?}",
        in_info.id, in_info.name, out_info.id, out_info.name
    );
    eprintln!(
        "  sent={} observed={} matched={} dropped={} unsupported={} elapsed={:?} tick_span={tick_span:?}",
        expected.len(),
        observed.len(),
        expected.len(),
        dropped,
        unsupported,
        started.elapsed(),
    );
    eprintln!(
        "  evidence note: name match is a candidate, not proof of a virtual \
         endpoint (`MidiPortOrigin::Unknown`) — this is OS-L1 round-trip \
         evidence, not `virtual-os-endpoint`"
    );

    input.close();
    assert!(matches!(
        input.read(&mut Vec::new(), 64),
        Err(DeviceError::StreamInvalidated)
    ));
    output.close();
    assert!(matches!(
        output.send(expected[0]),
        Err(DeviceError::StreamInvalidated)
    ));
}

/// Real-device smoke for the output path on a *known* Windows synth —
/// independent of any loopback pair. Sends only non-audible messages
/// (`ProgramChange` / `ControlChange` / `NoteOff` — no `NoteOn`, so
/// nothing sounds) through `midiOut*` to prove the send path reaches a
/// real `WinMM` device. `GS Wavetable Synth` ships in-box on Windows;
/// hosts without it record `NotRun`.
#[test]
#[ignore = "requires a MIDI output device (GS Wavetable Synth); manual opt-in check"]
fn real_device_silent_output_smoke() {
    let backend: Box<dyn MidiOutputEnumerator> = Box::new(backend());
    let ports = match backend.output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("verdict: NotRun — output enumeration unavailable ({error:?})");
            return;
        }
    };
    let Some(port) = ports
        .into_iter()
        .find(|p| p.info().name.to_lowercase().contains("gs wavetable"))
    else {
        eprintln!("verdict: NotRun — no 'GS Wavetable' output port on this host");
        return;
    };

    let info = port.info();
    eprintln!(
        "port: id={} name={:?} origin={:?}",
        info.id, info.name, info.origin
    );
    let mut output: Box<dyn MidiOutput> = port.open().unwrap_or_else(|error| {
        panic!("verdict: Fail — 'GS Wavetable' output port failed to open: {error:?}")
    });

    let channel = MidiChannel::new(0).unwrap();
    let silent_messages = [
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::ProgramChange {
                program: MidiProgram::new(5).unwrap(),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::ControlChange {
                controller: MidiController::new(7).unwrap(),
                value: MidiValue::from_u7(64),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::NoteOff {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(0),
            },
        },
    ];
    for event in &silent_messages {
        output
            .send(*event)
            .expect("send silent message through real WinMM output");
    }

    output.close();
    assert!(matches!(
        output.send(silent_messages[0]),
        Err(DeviceError::StreamInvalidated)
    ));
    eprintln!("verdict: Pass — silent messages accepted by real WinMM output port");
}
