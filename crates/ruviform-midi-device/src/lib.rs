//! `ruviform-midi-device` — the OS MIDI input device adapter: an
//! implementation of the MIDI input boundary ports
//! ([`ruviform_audio_core::midi_input`]) on the host OS's native MIDI
//! API.
//!
//! The crate is the *only* place OS MIDI APIs are linked: their types,
//! errors and feature names never appear on this crate's public
//! surface — [`SystemBackend`] reaches the caller only as the port's
//! `Box<dyn MidiInputEnumerator>` / `Box<dyn MidiInputPort>` /
//! `Box<dyn MidiInput>` trait objects, and every backend error is
//! normalized onto the port's `DeviceError`.
//!
//! ## Platform boundary
//!
//! - **Windows**: `WinMM` (`winmm.dll`) via `windows-sys`. Enumerates
//!   `midiIn*` devices, opens a `CALLBACK_FUNCTION | MIDI_IO_STATUS`
//!   subscription, decodes channel-voice messages, preserves the
//!   driver timestamp as an [`InputStamp`](ruviform_media_core::InputStamp)
//!   with `Carried` provenance, and reports queue overflow
//!   (`dropped`) and untranslatable traffic (`unsupported`) through
//!   [`MidiReadStats`](ruviform_audio_core::midi_input::MidiReadStats).
//! - **macOS / Linux**: not implemented in this slice — the same
//!   [`SystemBackend`] type returns `DeviceError::Unsupported` so
//!   consumers compile and fail honestly on every target.
//!
//! ## Responsibilities
//!
//! - Port enumeration and open: [`backend`] →
//!   `MidiInputEnumerator::input_ports` → `MidiInputPort::open`.
//! - Bounded callback→reader queue with observable loss accounting.
//! - Honest capability declaration: MIDI 1.0 bytes only
//!   (`supports_ump = false`), driver timestamps preserved
//!   (`provides_timestamps = true`), origin reported as
//!   `MidiPortOrigin::Unknown` because `WinMM` exposes no
//!   external/virtual distinction.
//!
//! ## Allowed dependency layers
//!
//! `ruviform-audio-core` (the port) and `ruviform-media-core` (event
//! and stamp types), plus the OS binding (`windows-sys`, Windows only,
//! confined). Adapters never depend on the engine crate
//! (`ruviform-audio`) nor on other adapters.
//!
//! ## RT contract
//!
//! Enumeration, `info`, `open`, `read`, `close` are control-path APIs —
//! non-RT (they call OS APIs and may allocate). The `WinMM` driver
//! callback runs on a system thread: it only locks the adapter's
//! internal queue (a pre-reserved bounded `VecDeque`) — no allocation,
//! no OS calls beyond the allowed synchronization primitives — per
//! the `WinMM` callback contract. Per-API guarantees are documented as
//! `**RT contract:**` lines. CI success is not a complete proof of RT
//! safety (realtime ADR).
//!
//! Contract source:
//! `docs/specs/issue-253-os-midi-backend.md`,
//! `docs/designs/issue-253-os-midi-backend.md`,
//! `docs/adr/select-midi-input-backend.md`.
//!
//! # Examples
//!
//! ```no_run
//! use ruviform_audio_core::midi_input::MidiInputEnumerator;
//!
//! // Enumerate OS MIDI input ports — control path. On unsupported
//! // platforms this returns `Err(DeviceError::Unsupported)`.
//! let backend = ruviform_midi_device::backend();
//! for port in backend.input_ports().unwrap_or_default() {
//!     let info = port.info();
//!     println!("{}: {}", info.id, info.name);
//! }
//! ```

mod midi;
mod os;
mod queue;
mod system;

pub use system::{SystemBackend, backend};
