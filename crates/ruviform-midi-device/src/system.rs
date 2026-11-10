//! The OS MIDI input backend entry point.
//!
//! [`SystemBackend`] is the single public adapter type on every
//! platform: on Windows it wraps the `WinMM` implementation, on
//! unsupported platforms every call returns
//! [`DeviceError::Unsupported`]. Keeping one type means consumers never
//! carry `cfg` for MIDI input.

use ruviform_audio_core::device::DeviceResult;
use ruviform_audio_core::midi_input::{MidiInputEnumerator, MidiInputPort};

/// The host OS MIDI input backend.
///
/// A unit struct — MIDI input has exactly one system backend per
/// platform in this slice (`WinMM` on Windows; `Unsupported` elsewhere).
/// Construct via [`backend`] or directly.
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

/// Returns the system MIDI input backend.
///
/// **RT contract:** non-RT — trivially pure, listed for symmetry with
/// the port contract's control-path APIs.
#[must_use]
pub fn backend() -> SystemBackend {
    SystemBackend
}
