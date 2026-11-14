//! `midi_endpoint_probe` — Issue #629 first-slice capability probe
//! for Windows virtual MIDI endpoints.
//!
//! Manual diagnostic — never in CI. Run with
//! `cargo run -p ruviform-midi-device --example midi_endpoint_probe`
//! (equivalently `mise exec -- cargo run -p ruviform-midi-device
//! --example midi_endpoint_probe`). The captured output is survey
//! evidence for `docs/discovery/issue-629-virtual-endpoint-survey.md`
//! and `evidence/rf629/`.
//!
//! What it does: reports OS facts, enumerates `WinMM` `midiIn*` and
//! `midiOut*` port capabilities (name, ids, interface id, driver
//! version, support flags), detects *loopback-style* pairs — the same
//! port name visible in both directions — and cross-checks the ports
//! against the production surface (`ruviform_midi_device::backend()`
//! input enumeration).
//!
//! What it never does: claim `virtual-os-endpoint` evidence. `WinMM`
//! cannot distinguish a virtual port from a physical one
//! (`MidiPortOrigin::Unknown` is the honest answer), so every
//! virtual-endpoint case this probe feeds stays `NotRun` unless a
//! human confirms a provisioned loopback on the dedicated runner —
//! and even then, streaming proof is the next slice.

use std::process::ExitCode;

fn main() -> ExitCode {
    println!("midi_endpoint_probe — Issue #629 virtual-endpoint capability probe (WinMM)");
    println!("manual diagnostic; the output is survey evidence, not a test verdict");
    println!();
    print_environment();
    #[cfg(windows)]
    win::run();
    #[cfg(not(windows))]
    {
        println!();
        println!("== verdict ==");
        println!("  winmm enumeration: NotRun — this probe requires Windows");
        println!("  virtual midi-endpoint cases: NotRun — no Windows WinMM on this host");
    }
    ExitCode::SUCCESS
}

/// Prints the OS facts every Case record needs (`os`/`arch`/`family`);
/// the Windows build string is added by the platform module.
fn print_environment() {
    println!("== environment ==");
    println!(
        "  os={} arch={} family={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    );
    #[cfg(windows)]
    win::print_os_build();
}

#[cfg(windows)]
mod win {
    //! Windows/`WinMM` probe body — the only place Win32 FFI appears in
    //! this example. Every `unsafe` block is individually justified at
    //! its site (workspace policy: local allowance + SAFETY comments —
    //! precedent `src/os/windows.rs`).

    #![expect(unsafe_code)]

    use std::collections::BTreeSet;
    use std::process::Command;

    use ruviform_audio_core::midi_input::MidiInputEnumerator;
    use ruviform_midi_device::backend;
    use windows_sys::Win32::Media::Audio::{self, HMIDIIN, MIDIINCAPSW, MIDIOUTCAPSW};
    use windows_sys::Win32::Media::MMSYSERR_NOERROR;
    use windows_sys::Win32::Media::Multimedia::{
        DRV_QUERYDEVICEINTERFACE, DRV_QUERYDEVICEINTERFACESIZE,
    };

    /// One enumerated port in either direction.
    struct PortRow {
        direction: &'static str,
        index: u32,
        name: String,
        mid: u16,
        pid: u16,
        driver_version: u32,
        /// `dwSupport` — only populated for `midiOut*` ports: the
        /// `MIDIINCAPSW` layout in `windows-sys` ends at `szPname`, so
        /// reading its trailing field would report misaligned bytes.
        support: Option<u32>,
        technology: Option<u16>,
        voices: Option<u16>,
        notes: Option<u16>,
        channel_mask: Option<u16>,
        interface_id: Option<String>,
        probe_notes: Vec<String>,
        is_loopback_candidate: bool,
    }

    /// Runs the whole probe and prints the report.
    pub(super) fn run() {
        println!();
        println!("== WinMM port enumeration ==");
        let (inputs, outputs) = enumerate_all();
        for row in inputs.iter().chain(outputs.iter()) {
            print_port(row);
        }
        println!();
        println!("== production port view (ruviform_midi_device) ==");
        print_port_view();
        println!();
        print_verdict(&inputs, &outputs);
    }

    /// Prints the Windows version string via `cmd /c ver` — the raw
    /// output is recorded verbatim for the evidence file. Same
    /// feature-gap rationale as the audio probe: no new `windows`
    /// features are added for this slice.
    pub(super) fn print_os_build() {
        match Command::new("cmd").args(["/c", "ver"]).output() {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout);
                println!("  os_version: {}", text.trim());
            }
            Err(error) => println!("  os_version: NotRun — `cmd /c ver` failed: {error}"),
        }
    }

    /// Enumerates `midiIn*` and `midiOut*` devices. `WinMM` port
    /// enumeration does not open devices — `*GetDevCapsW`/`*Message`
    /// queries are metadata-only and safe to run without a provisioned
    /// endpoint.
    fn enumerate_all() -> (Vec<PortRow>, Vec<PortRow>) {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // SAFETY: `midiInGetNumDevs` is infallible by contract.
        let in_count = unsafe { Audio::midiInGetNumDevs() };
        for index in 0..in_count {
            match input_caps(index) {
                Ok(caps) => inputs.push(PortRow {
                    direction: "input",
                    index,
                    name: pname(caps.szPname),
                    mid: caps.wMid,
                    pid: caps.wPid,
                    driver_version: caps.vDriverVersion,
                    support: None,
                    technology: None,
                    voices: None,
                    notes: None,
                    channel_mask: None,
                    interface_id: interface_id_in(index),
                    probe_notes: Vec::new(),
                    is_loopback_candidate: false,
                }),
                Err(error) => println!("  warning: midiIn caps[{index}] failed: {error}"),
            }
        }

        // SAFETY: `midiOutGetNumDevs` is infallible by contract.
        let out_count = unsafe { Audio::midiOutGetNumDevs() };
        for index in 0..out_count {
            match output_caps(index) {
                Ok(caps) => outputs.push(PortRow {
                    direction: "output",
                    index,
                    name: pname(caps.szPname),
                    mid: caps.wMid,
                    pid: caps.wPid,
                    driver_version: caps.vDriverVersion,
                    support: Some(caps.dwSupport),
                    technology: Some(caps.wTechnology),
                    voices: Some(caps.wVoices),
                    notes: Some(caps.wNotes),
                    channel_mask: Some(caps.wChannelMask),
                    // `midiOutMessage`'s `DRV_QUERYDEVICEINTERFACE*`
                    // handling is not reliable on unopened output
                    // devices — observed on this host: the size query
                    // returned `MMSYSERR_NOERROR` with size=2 and the
                    // follow-up fetch crashed inside winmm. The input
                    // path (the adapter's proven mechanism) is queried;
                    // output interface ids stay unqueried and are
                    // recorded as a probe limitation.
                    interface_id: None,
                    probe_notes: vec![
                        "interface_id: NotRun — midiOutMessage DRV_QUERYDEVICEINTERFACE crashed on this host's driver; input-path query only".to_owned(),
                    ],
                    is_loopback_candidate: false,
                }),
                Err(error) => println!("  warning: midiOut caps[{index}] failed: {error}"),
            }
        }

        // Loopback-style pair detection: a port name visible in BOTH
        // directions is the signature of software loopbacks (loopMIDI,
        // loopBE, Windows MIDI Services loop endpoints all register a
        // paired in/out under one name). It is a candidate signal, not
        // proof — names are user-chosen. Name sets are owned so the
        // rows can be mutated freely below.
        let output_names: BTreeSet<String> = outputs.iter().map(|row| row.name.clone()).collect();
        let input_names: BTreeSet<String> = inputs.iter().map(|row| row.name.clone()).collect();
        for row in &mut inputs {
            row.is_loopback_candidate = output_names.contains(&row.name);
        }
        for row in &mut outputs {
            row.is_loopback_candidate = input_names.contains(&row.name);
        }

        (inputs, outputs)
    }

    /// `midiInGetDevCapsW` for one device index.
    #[expect(clippy::cast_possible_truncation)]
    fn input_caps(index: u32) -> Result<MIDIINCAPSW, String> {
        let mut caps = MIDIINCAPSW::default();
        // SAFETY: `caps` is a valid out-struct of the declared size.
        let rc = unsafe {
            Audio::midiInGetDevCapsW(
                index as usize,
                &raw mut caps,
                size_of::<MIDIINCAPSW>() as u32,
            )
        };
        if rc == MMSYSERR_NOERROR {
            Ok(caps)
        } else {
            Err(format!("midiInGetDevCapsW rc={rc}"))
        }
    }

    /// `midiOutGetDevCapsW` for one device index.
    #[expect(clippy::cast_possible_truncation)]
    fn output_caps(index: u32) -> Result<MIDIOUTCAPSW, String> {
        let mut caps = MIDIOUTCAPSW::default();
        // SAFETY: `caps` is a valid out-struct of the declared size.
        let rc = unsafe {
            Audio::midiOutGetDevCapsW(
                index as usize,
                &raw mut caps,
                size_of::<MIDIOUTCAPSW>() as u32,
            )
        };
        if rc == MMSYSERR_NOERROR {
            Ok(caps)
        } else {
            Err(format!("midiOutGetDevCapsW rc={rc}"))
        }
    }

    /// Decodes the fixed-width UTF-16 `szPname` field (NUL-terminated,
    /// max 32 units). The caps structs are `repr(packed)` — the array
    /// is copied out before any slice is taken (same rule as the
    /// adapter's `caps_name`).
    fn pname(sz_pname: [u16; 32]) -> String {
        let end = sz_pname
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(sz_pname.len());
        String::from_utf16_lossy(&sz_pname[..end])
    }

    /// Queries the stable device-interface id via `midiInMessage`
    /// (`DRV_QUERYDEVICEINTERFACE`) — the same mechanism the adapter's
    /// `interface_id` uses. Only the input direction is queried: see
    /// the call-site note for why `midiOutMessage` is not used.
    /// Returns `None` for legacy devices that do not answer.
    fn interface_id_in(index: u32) -> Option<String> {
        // SAFETY: `size`/`buf` are valid out-parameters for the
        // duration of each call; `index` is passed as the device id
        // handle exactly as the WinMM docs specify for unopened
        // devices (precedent: adapter `interface_id`).
        unsafe {
            let mut size: u32 = 0;
            let rc = Audio::midiInMessage(
                index as usize as HMIDIIN,
                DRV_QUERYDEVICEINTERFACESIZE,
                (&raw mut size) as usize,
                0,
            );
            if rc != MMSYSERR_NOERROR || size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let rc = Audio::midiInMessage(
                index as usize as HMIDIIN,
                DRV_QUERYDEVICEINTERFACE,
                size as usize,
                buf.as_mut_ptr() as usize,
            );
            if rc != MMSYSERR_NOERROR {
                return None;
            }
            // The interface id is a NUL-terminated UTF-16LE string.
            let units: Vec<u16> = buf
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_ne_bytes(*c))
                .take_while(|&c| c != 0)
                .collect();
            String::from_utf16(&units).ok()
        }
    }

    /// Cross-checks the `WinMM` enumeration against the production port
    /// surface (`ruviform_midi_device::backend()` → `WinMM` input).
    fn print_port_view() {
        match backend().input_ports() {
            Ok(ports) => {
                println!("  production input ports: {}", ports.len());
                for port in ports {
                    let info = port.info();
                    println!(
                        "    - {}  id={}  origin={:?}  ump={} timestamps={} domain={:?}",
                        info.name,
                        info.id,
                        info.origin,
                        info.capabilities.supports_ump,
                        info.capabilities.provides_timestamps,
                        info.capabilities.clock_domain,
                    );
                }
            }
            Err(error) => println!("  production input enumeration failed: {error}"),
        }
    }

    fn print_port(row: &PortRow) {
        let candidate = if row.is_loopback_candidate {
            "  [loopback-candidate: name appears in both directions, unconfirmed]"
        } else {
            ""
        };
        println!(
            "  [{}] {} index={} mid=0x{:04x} pid=0x{:04x} driver_ver={}{}",
            row.direction, row.name, row.index, row.mid, row.pid, row.driver_version, candidate
        );
        if let Some(id) = &row.interface_id {
            println!("      interface_id: {id}");
        }
        if let Some(support) = row.support {
            println!(
                "      support=0x{support:08x} tech={} voices={} notes={} ch_mask=0x{:04x}",
                row.technology.unwrap_or(0),
                row.voices.unwrap_or(0),
                row.notes.unwrap_or(0),
                row.channel_mask.unwrap_or(0)
            );
        }
        for note in &row.probe_notes {
            println!("      note: {note}");
        }
    }

    /// The fail-closed summary.
    fn print_verdict(inputs: &[PortRow], outputs: &[PortRow]) {
        let candidates: usize = inputs
            .iter()
            .chain(outputs.iter())
            .filter(|row| row.is_loopback_candidate)
            .count();
        println!("== verdict ==");
        println!(
            "  winmm input ports: {}, output ports: {}",
            inputs.len(),
            outputs.len()
        );
        println!(
            "  loopback-style candidates (bidirectional name match, unconfirmed): {candidates}"
        );
        println!(
            "  driver signature check: NotRun — driver/package signature query needs SetupAPI surfaces outside the declared `windows-sys` features (discovery doc gap)"
        );
        println!(
            "  provisioning: {}",
            if candidates == 0 {
                "NOT_PROVISIONED"
            } else {
                "CANDIDATES_OBSERVED"
            }
        );
        println!("  capability notes:");
        println!("    winmm in/out capability + interface-id enumeration: observed");
        println!(
            "    port open + Note/CC/PitchBend/timestamp/order round-trip: NotRun — needs a provisioned loopback plus the `midiOut*` send path the crate does not yet have (discovery doc gap)"
        );
        println!("    virtual hotplug / device-loss / permission-denial: NotRun — same reason");
        println!(
            "  midi virtual-endpoint cases: NotRun — WinMM cannot prove a port is virtual (`MidiPortOrigin::Unknown`); open/stream proof is the next slice"
        );
    }
}
