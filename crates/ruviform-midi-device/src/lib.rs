//! `ruviform-midi-device` — the OS MIDI device adapter: an
//! implementation of the MIDI boundary ports
//! ([`ruviform_audio_core::midi_input`] and
//! [`ruviform_audio_core::midi_output`]) on the host OS's native MIDI
//! API.
//!
//! The crate is the *only* place OS MIDI APIs are linked: their types,
//! errors and feature names never appear on this crate's public
//! surface — [`SystemBackend`] reaches the caller only as the ports'
//! `Box<dyn …>` trait objects, and every backend error is normalized
//! onto the port's `DeviceError`.
//!
//! ## Platform boundary
//!
//! - **Windows**: `WinMM` (`winmm.dll`) via `windows-sys`. Input
//!   enumerates `midiIn*` devices, opens a `CALLBACK_FUNCTION |
//!   MIDI_IO_STATUS` subscription, decodes channel-voice messages,
//!   preserves the driver timestamp as an
//!   [`InputStamp`](ruviform_media_core::InputStamp) with `Carried`
//!   provenance, and reports queue overflow (`dropped`) and
//!   untranslatable traffic (`unsupported`) through
//!   [`MidiReadStats`](ruviform_audio_core::midi_input::MidiReadStats).
//!   Output enumerates `midiOut*` devices, opens a
//!   `CALLBACK_FUNCTION | MIDI_IO_STATUS` subscription, encodes
//!   channel-voice events through `midiOutShortMsg` and `SysEx` through
//!   `midiOutLongMsg` with `MM_MOM_DONE` completion.
//! - **macOS / Linux**: not implemented in this slice — the same
//!   [`SystemBackend`] type returns `DeviceError::Unsupported` so
//!   consumers compile and fail honestly on every target.
//!
//! ## Responsibilities
//!
//! - Port enumeration and open: [`backend`] →
//!   `MidiInputEnumerator::input_ports` /
//!   `MidiOutputEnumerator::output_ports` → `…Port::open`.
//! - Bounded callback→reader queue with observable loss accounting
//!   (input); synchronous driver send with `MM_MOM_DONE` completion
//!   for `SysEx` (output).
//! - Honest capability declaration: MIDI 1.0 bytes only
//!   (`supports_ump = false`); input preserves driver timestamps
//!   (`provides_timestamps = true`), output declares none
//!   (`provides_timestamps = false`, `clock_domain = None`); origin
//!   reported as `MidiPortOrigin::Unknown` because `WinMM` exposes no
//!   external/virtual distinction.
//!
//! ## Allowed dependency layers
//!
//! `ruviform-audio-core` (the ports) and `ruviform-media-core` (event
//! and stamp types), plus the OS binding (`windows-sys`, Windows only,
//! confined). Adapters never depend on the engine crate
//! (`ruviform-audio`) nor on other adapters.
//!
//! ## RT contract
//!
//! Enumeration, `info`, `open`, `read`, `send`, `send_sysex`, `close`
//! are control-path APIs — non-RT (they call OS APIs and may
//! allocate; `send_sysex` additionally waits on the driver completion
//! callback). The `WinMM` driver callbacks run on system threads:
//! input only locks the adapter's internal queue (a pre-reserved
//! bounded `VecDeque`), output only flags a completion — no
//! allocation, no OS calls beyond the allowed synchronization
//! primitives — per the `WinMM` callback contract. Per-API guarantees
//! are documented as `**RT contract:**` lines. CI success is not a
//! complete proof of RT safety (realtime ADR).
//!
//! Contract source:
//! `docs/specs/issue-253-os-midi-backend.md`,
//! `docs/designs/issue-253-os-midi-backend.md`,
//! `docs/adr/select-midi-input-backend.md`,
//! `docs/specs/issue-629-midi-output.md`,
//! `docs/designs/issue-629-midi-output.md`.
//!
//! # Examples
//!
//! ```no_run
//! use ruviform_audio_core::midi_input::MidiInputEnumerator;
//! use ruviform_audio_core::midi_output::MidiOutputEnumerator;
//!
//! // Enumerate OS MIDI ports — control path. On unsupported
//! // platforms these return `Err(DeviceError::Unsupported)`.
//! let backend = ruviform_midi_device::backend();
//! for port in backend.input_ports().unwrap_or_default() {
//!     let info = port.info();
//!     println!("in: {}: {}", info.id, info.name);
//! }
//! for port in backend.output_ports().unwrap_or_default() {
//!     let info = port.info();
//!     println!("out: {}: {}", info.id, info.name);
//! }
//! ```

// The decoder/encoder and bounded queue exist only to serve the WinMM
// backend in this slice; they are ungated when the macOS/Linux
// backends land.
#[cfg(windows)]
mod midi;
mod os;
#[cfg(windows)]
mod queue;
mod system;

pub use system::{SystemBackend, backend};
