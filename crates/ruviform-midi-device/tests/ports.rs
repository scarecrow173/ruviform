//! Public-surface wiring tests: the `Box<dyn …>` trait-object path a
//! consumer (e.g. `InputTx::pump`) actually takes.

use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputPort, MidiPortOrigin, MidiReadStats,
};
use ruviform_midi_device::backend;

#[cfg(windows)]
#[test]
fn enumerator_produces_ports_with_honest_declarations() {
    let backend: Box<dyn MidiInputEnumerator> = Box::new(backend());
    let ports = backend
        .input_ports()
        .expect("WinMM enumeration must succeed (empty is fine)");

    let mut domains = std::collections::HashSet::new();
    for port in &ports {
        let port: &dyn MidiInputPort = port.as_ref();
        let info = port.info();
        assert!(!info.id.is_empty(), "port id must be non-empty");
        // WinMM cannot distinguish external/virtual/internal.
        assert_eq!(info.origin, MidiPortOrigin::Unknown);
        // MIDI 1.0 only, timestamps carried, per-port domain.
        assert!(!info.capabilities.supports_ump);
        assert!(info.capabilities.provides_timestamps);
        let domain = info
            .capabilities
            .clock_domain
            .expect("stamped port declares a domain");
        assert!(domains.insert(domain), "domains must be unique per port");
    }
}

#[cfg(windows)]
#[test]
fn read_contract_shape_holds_for_trait_objects() {
    // Compile-level + behavioral check of the `MidiInput` contract the
    // bridge pump relies on: `read` appends to an engine-owned buffer
    // and returns observable stats through the `dyn` trait object.
    // With no devices there is nothing to open, so this verifies the
    // type path only — it is never called.
    fn read_via_trait_object(
        input: &mut dyn MidiInput,
    ) -> ruviform_audio_core::device::DeviceResult<MidiReadStats> {
        let mut out = Vec::new();
        input.read(&mut out, 64)
    }
    std::hint::black_box(read_via_trait_object);
}

#[cfg(not(windows))]
#[test]
fn unsupported_platforms_return_explicit_unsupported() {
    let backend: Box<dyn MidiInputEnumerator> = Box::new(ruviform_midi_device::SystemBackend);
    assert_eq!(backend.input_ports().unwrap_err(), DeviceError::Unsupported);
}

#[cfg(windows)]
#[test]
#[ignore = "requires a MIDI input device; manual opt-in check"]
fn real_device_open_read_close() {
    let ports = backend().input_ports().unwrap();
    let Some(port) = ports.into_iter().next() else {
        eprintln!("no MIDI input ports — nothing to verify");
        return;
    };
    let mut input: Box<dyn MidiInput> = port.open().unwrap();
    let mut out = Vec::new();
    let stats = input.read(&mut out, 64).unwrap();
    eprintln!("read {} events, stats {stats:?}", out.len());
    input.close();
    // After close the subscription reports invalidation.
    assert!(matches!(
        input.read(&mut out, 64),
        Err(DeviceError::StreamInvalidated)
    ));
}
