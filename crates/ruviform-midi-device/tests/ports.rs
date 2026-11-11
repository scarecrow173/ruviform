//! Public-surface wiring tests: the `Box<dyn …>` trait-object path a
//! consumer (e.g. `InputTx::pump`) actually takes.

use ruviform_audio_core::device::DeviceError;
use ruviform_audio_core::midi_input::{MidiInput, MidiInputEnumerator};
#[cfg(windows)]
use ruviform_audio_core::midi_input::{MidiInputPort, MidiPortOrigin, MidiReadStats};
use ruviform_media_core::TimestampProvenance;
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
    assert!(matches!(
        backend.input_ports(),
        Err(DeviceError::Unsupported)
    ));
}

/// Issue #253 real-device lane — the full port lifecycle on the actual
/// OS backend: enumerate → open → repeated `read` (timestamp domain,
/// provenance, `dropped`/`unsupported` observation) → close →
/// `StreamInvalidated`.
///
/// Manual opt-in: `cargo test -p ruviform-midi-device -- --ignored`
/// (canonical mise lane registration is a recorded follow-up — this
/// crate's Cargo.toml and `mise.toml` are outside this slice's edit
/// scope). Honest exit on platforms without a backend
/// (`DeviceError::Unsupported`) or with no input ports — hardware
/// traffic (nonzero counters, actual events) is recorded, never
/// asserted, because nothing in the lane can physically generate it.
#[test]
#[ignore = "requires a MIDI input device; manual opt-in check"]
fn real_device_open_read_observe_close() {
    let backend: Box<dyn MidiInputEnumerator> = Box::new(backend());
    let ports = match backend.input_ports() {
        Ok(ports) => ports,
        Err(err) => {
            eprintln!("OS MIDI backend unavailable ({err:?}) — nothing to verify");
            return;
        }
    };
    let Some(port) = ports.into_iter().next() else {
        eprintln!("no MIDI input ports — nothing to verify");
        return;
    };

    let info = port.info();
    eprintln!(
        "port: id={} name={:?} origin={:?} ump={} timestamps={} domain={:?}",
        info.id,
        info.name,
        info.origin,
        info.capabilities.supports_ump,
        info.capabilities.provides_timestamps,
        info.capabilities.clock_domain,
    );

    let mut input: Box<dyn MidiInput> = port.open().expect("open declared port");

    // Drain for a short window so real traffic has a chance to arrive;
    // every observable counter and stamp field is recorded.
    let mut out = Vec::new();
    let (mut total_dropped, mut total_unsupported) = (0u64, 0u64);
    for _ in 0..10 {
        let stats = input.read(&mut out, 64).expect("read on open subscription");
        total_dropped += stats.dropped;
        total_unsupported += stats.unsupported;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Contract-level stamp invariants (hold without traffic): a
    // stamped port's events must carry the *declared* clock domain and
    // `Carried` provenance, with non-decreasing driver ticks.
    if info.capabilities.provides_timestamps {
        let declared_domain = info
            .capabilities
            .clock_domain
            .expect("stamped port declares a domain");
        let mut prev_ticks = None;
        for event in &out {
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

    eprintln!(
        "observed: {} events, dropped={total_dropped}, unsupported={total_unsupported}",
        out.len()
    );

    input.close();
    // After close the subscription reports invalidation.
    assert!(matches!(
        input.read(&mut out, 64),
        Err(DeviceError::StreamInvalidated)
    ));
}
