//! Public-surface wiring tests for the MIDI output boundary: the
//! `Box<dyn …>` trait-object path a consumer takes, the honest `WinMM`
//! capability declaration, and the non-Windows `Unsupported` boundary
//! (Issue #629, 2nd slice — discovery gap #1 `midiOut*` send path).

use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_output::{MidiOutput, MidiOutputEnumerator};
#[cfg(windows)]
use ruviform_audio_core::midi_output::{MidiOutputPort, MidiPortOrigin};
use ruviform_media_core::{MidiChannel, MidiGroup, MidiKind, MidiValue, NoteKey};
use ruviform_midi_device::backend;

#[cfg(windows)]
#[test]
fn output_enumerator_produces_ports_with_honest_declarations() {
    let backend: Box<dyn MidiOutputEnumerator> = Box::new(backend());
    let ports = backend
        .output_ports()
        .expect("WinMM output enumeration must succeed (empty is fine)");

    for port in &ports {
        let port: &dyn MidiOutputPort = port.as_ref();
        let info = port.info();
        assert!(!info.id.is_empty(), "port id must be non-empty");
        // WinMM cannot distinguish external/virtual/internal.
        assert_eq!(info.origin, MidiPortOrigin::Unknown);
        // MIDI 1.0 only; the send direction carries no timestamps, so
        // no clock domain is declared either.
        assert!(!info.capabilities.supports_ump);
        assert!(!info.capabilities.provides_timestamps);
        assert_eq!(info.capabilities.clock_domain, None);
    }
}

#[cfg(windows)]
#[test]
fn send_contract_shape_holds_for_trait_objects() {
    // Compile-level + behavioral check of the `MidiOutput` contract
    // the engine send path relies on: `send`/`send_sysex`/`close`
    // compose through the `dyn` trait object. With no devices there is
    // nothing to open, so this verifies the type path only — the
    // function is never called.
    fn send_via_trait_object(
        output: &mut dyn MidiOutput,
    ) -> ruviform_audio_core::device::DeviceResult<()> {
        let event = ruviform_media_core::MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        };
        output.send(event)?;
        output.send_sysex(&[0xF0, 0x7E, 0x00, 0xF7])
    }
    std::hint::black_box(send_via_trait_object);
}

#[cfg(not(windows))]
#[test]
fn unsupported_platforms_return_explicit_unsupported() {
    let backend: Box<dyn MidiOutputEnumerator> = Box::new(ruviform_midi_device::SystemBackend);
    assert!(matches!(
        backend.output_ports(),
        Err(DeviceError::Unsupported)
    ));
}

/// Issue #629 real-device lane — the output port lifecycle on the
/// actual OS backend: enumerate → open → send channel-voice + `SysEx` →
/// close → `StreamInvalidated`.
///
/// Manual opt-in: `cargo test -p ruviform-midi-device -- --ignored`
/// (the `test-midi-real-device` lane collects `#[ignore]` tests; this
/// slice does not register new mise lanes). Honest exit on platforms
/// without a backend (`DeviceError::Unsupported`) or with no output
/// ports — whether the device actually emitted is hardware-observable
/// only, so send acceptance is asserted and physical delivery is
/// recorded, never claimed.
#[test]
#[ignore = "requires a MIDI output device; manual opt-in check"]
fn real_device_send_smoke() {
    let backend: Box<dyn MidiOutputEnumerator> = Box::new(backend());
    let ports = match backend.output_ports() {
        Ok(ports) => ports,
        Err(err) => {
            eprintln!("OS MIDI output backend unavailable ({err:?}) — nothing to verify");
            return;
        }
    };
    let Some(port) = ports.into_iter().next() else {
        eprintln!("no MIDI output ports — nothing to verify");
        return;
    };

    let info = port.info();
    eprintln!(
        "port: id={} name={:?} origin={:?} ump={} timestamps={}",
        info.id,
        info.name,
        info.origin,
        info.capabilities.supports_ump,
        info.capabilities.provides_timestamps,
    );

    let mut output: Box<dyn MidiOutput> = port.open().expect("open declared port");

    let note_on = ruviform_media_core::MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(60).unwrap(),
            velocity: MidiValue::from_u7(100),
        },
    };
    let note_off = ruviform_media_core::MidiEvent {
        kind: MidiKind::NoteOff {
            key: NoteKey::new(60).unwrap(),
            velocity: MidiValue::from_u7(0),
        },
        ..note_on
    };
    let bend = ruviform_media_core::MidiEvent {
        kind: MidiKind::PitchBend { value: 0 },
        ..note_on
    };

    // Send acceptance is the contract under test; physical audibility
    // is not assertable headlessly.
    output.send(note_on).expect("send note on");
    output.send(bend).expect("send pitch bend center");
    output.send(note_off).expect("send note off");
    // `send_sysex` waits on `MM_MOM_DONE`, which requires the driver
    // to have accepted `MIDI_IO_STATUS` at open. Drivers that reject
    // the flag (e.g. GS Wavetable Synth) degrade to the documented
    // `Unsupported` — both outcomes are contract-honest.
    match output.send_sysex(&[0xF0, 0x7E, 0x00, 0xF7]) {
        Ok(()) => eprintln!("send_sysex: accepted (MM_MOM_DONE tracked)"),
        Err(DeviceError::Unsupported) => {
            eprintln!(
                "send_sysex: Unsupported — driver rejected MIDI_IO_STATUS (documented degrade)"
            );
        }
        Err(err) => panic!("send sysex failed unexpectedly: {err:?}"),
    }

    output.close();
    // After close the subscription reports invalidation.
    assert!(matches!(
        output.send(note_on),
        Err(DeviceError::StreamInvalidated)
    ));
}
