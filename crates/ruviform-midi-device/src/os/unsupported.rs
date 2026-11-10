//! Non-Windows boundary: no MIDI backend exists in this slice, so the
//! port contract is satisfied by an explicit `Unsupported` error rather
//! than a missing implementation. macOS (`CoreMIDI`) and Linux
//! (`ALSA`/`JACK`/`PipeWire`) backends are deferred to later Issue #253
//! slices — see `docs/designs/issue-253-os-midi-backend.md`.

use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_input::MidiInputPort;

/// Always fails with [`DeviceError::Unsupported`] — there is no port
/// enumeration on unsupported platforms.
///
/// **RT contract:** non-RT — pure error construction.
pub(crate) fn enumerate_ports() -> DeviceResult<Vec<Box<dyn MidiInputPort>>> {
    Err(DeviceError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_is_explicitly_unsupported() {
        assert!(matches!(enumerate_ports(), Err(DeviceError::Unsupported)));
    }
}
