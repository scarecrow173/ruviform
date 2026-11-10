//! Manual verification helper: lists OS MIDI input ports.
//!
//! Not part of CI — run with `cargo run -p ruviform-midi-device
//! --example list_midi_ports`. On platforms without a backend it prints
//! the `Unsupported` error honestly.

use ruviform_audio_core::midi_input::MidiInputEnumerator;
use ruviform_midi_device::backend;

fn main() {
    match backend().input_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                println!("no MIDI input ports");
            }
            for port in ports {
                let info = port.info();
                println!(
                    "id={} name={:?} origin={:?} ump={} timestamps={} domain={:?}",
                    info.id,
                    info.name,
                    info.origin,
                    info.capabilities.supports_ump,
                    info.capabilities.provides_timestamps,
                    info.capabilities.clock_domain,
                );
            }
        }
        Err(err) => eprintln!("enumeration failed: {err}"),
    }
}
