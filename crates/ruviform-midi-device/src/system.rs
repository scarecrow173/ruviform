//! The OS MIDI backend entry point.
//!
//! [`SystemBackend`] is the single public adapter type on every
//! platform: on Windows it wraps the `WinMM` input and output
//! implementations, on unsupported platforms every call returns
//! [`DeviceError::Unsupported`]. Keeping one type means consumers never
//! carry `cfg` for MIDI.

use ruviform_audio_core::device::DeviceResult;
use ruviform_audio_core::midi_input::{MidiInputEnumerator, MidiInputPort};
use ruviform_audio_core::midi_output::{MidiOutputEnumerator, MidiOutputPort};

/// The host OS MIDI backend.
///
/// A unit struct — MIDI has exactly one system backend per platform in
/// this slice (`WinMM` on Windows; `Unsupported` elsewhere). Construct
/// via [`backend`] or directly.
///
/// **RT contract:** non-RT object — methods call OS APIs and may
/// allocate.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemBackend;

impl MidiInputEnumerator for SystemBackend {
    /// Input ports currently present — an empty `Vec` when none, or
    /// [`DeviceError::Unsupported`] on platforms without a backend.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    fn input_ports(&self) -> DeviceResult<Vec<Box<dyn MidiInputPort>>> {
        // Windows enumeration is infallible (vanished devices are
        // skipped inside the backend), so the contract's `DeviceResult`
        // is satisfied by wrapping the `Vec` here; other platforms
        // return `Err(DeviceError::Unsupported)` from `enumerate_ports`.
        #[cfg(windows)]
        return Ok(crate::os::enumerate_ports());
        #[cfg(not(windows))]
        return crate::os::enumerate_ports();
    }
}

impl MidiOutputEnumerator for SystemBackend {
    /// Output ports currently present — an empty `Vec` when none, or
    /// [`DeviceError::Unsupported`] on platforms without a backend.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    fn output_ports(&self) -> DeviceResult<Vec<Box<dyn MidiOutputPort>>> {
        // Same shape as `input_ports`: Windows enumeration is
        // infallible, other platforms return
        // `Err(DeviceError::Unsupported)` from `enumerate_output_ports`.
        #[cfg(windows)]
        return Ok(crate::os::enumerate_output_ports());
        #[cfg(not(windows))]
        return crate::os::enumerate_output_ports();
    }
}

/// Returns the system MIDI backend.
///
/// **RT contract:** non-RT — trivially pure, listed for symmetry with
/// the port contract's control-path APIs.
#[must_use]
pub fn backend() -> SystemBackend {
    SystemBackend
}
