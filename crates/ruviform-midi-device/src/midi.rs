//! Pure MIDI 1.0 byte-stream decoding — no OS types, no allocation.
//!
//! `WinMM` expands running status before delivery, so every `MIM_DATA`
//! message arrives as a complete `status + data1 + data2` triple. This
//! module maps that triple onto the canonical [`MidiEvent`] vocabulary
//! (`ruviform-media-core`), widening 7-/14-bit fields through
//! [`MidiValue`]. Anything outside the channel-voice vocabulary —
//! system common/realtime messages, `SysEx` fragments, malformed bytes —
//! returns `None` so the caller can count it as *unsupported* traffic
//! rather than dropping it silently.
//!
//! **RT contract:** RT-suitable — `const`-compatible pure integer math,
//! allocation-free, no panics on untrusted input.

use ruviform_media_core::{
    MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiProgram, MidiValue, NoteKey,
};

/// Decodes one packed `WinMM` `MIM_DATA` message into a canonical
/// [`MidiEvent`].
///
/// `status`/`data1`/`data2` are the three meaningful bytes of
/// `dwParam1` (low byte = status, next = data1, next = data2; the high
/// byte is unused by `WinMM`). Returns `None` for anything that is not a
/// well-formed channel-voice message — system messages, `SysEx` status
/// bytes, non-7-bit data bytes, or out-of-range field values. Callers
/// must treat `None` as countable unsupported traffic, never a silent
/// drop.
///
/// **RT contract:** RT-suitable — pure, allocation-free, no panics.
#[must_use]
pub(crate) fn decode_message(status: u8, data1: u8, data2: u8) -> Option<MidiEvent> {
    // Channel voice occupies 0x80..=0xEF; everything else (0xF0..=0xFF
    // system/SysEx, <0x80 malformed) is outside the vocabulary.
    if !(0x80..=0xEF).contains(&status) {
        return None;
    }
    // MIDI 1.0 data bytes are seven-bit; a driver that hands us a
    // malformed byte is reported as unsupported, not reinterpreted.
    if data1 > 0x7F || data2 > 0x7F {
        return None;
    }

    let channel = MidiChannel::new(status & 0x0F).ok()?;
    let kind = match status & 0xF0 {
        0x80 => MidiKind::NoteOff {
            key: NoteKey::new(data1).ok()?,
            velocity: MidiValue::from_u7(data2),
        },
        0x90 => MidiKind::NoteOn {
            key: NoteKey::new(data1).ok()?,
            velocity: MidiValue::from_u7(data2),
        },
        0xA0 => MidiKind::PolyPressure {
            key: NoteKey::new(data1).ok()?,
            pressure: MidiValue::from_u7(data2),
        },
        0xB0 => MidiKind::ControlChange {
            controller: MidiController::new(data1).ok()?,
            value: MidiValue::from_u7(data2),
        },
        0xC0 => MidiKind::ProgramChange {
            program: MidiProgram::new(data1).ok()?,
        },
        0xD0 => MidiKind::ChannelPressure {
            pressure: MidiValue::from_u7(data1),
        },
        0xE0 => {
            // Pitch bend: data1 = LSB, data2 = MSB of the 14-bit value
            // (0x2000 = center). Each byte is already validated <= 0x7F.
            let raw14 = u16::from(data1) | (u16::from(data2) << 7);
            MidiKind::PitchBend {
                value: MidiValue::bend_from_u14(raw14),
            }
        }
        _ => return None,
    };

    Some(MidiEvent {
        group: MidiGroup::ZERO,
        channel,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(status: u8, d1: u8, d2: u8) -> MidiEvent {
        decode_message(status, d1, d2).expect("channel-voice message must decode")
    }

    #[test]
    fn decodes_note_on_with_channel_and_velocity() {
        let ev = event(0x93, 60, 100);
        assert_eq!(ev.group, MidiGroup::ZERO);
        assert_eq!(ev.channel.get(), 3);
        match ev.kind {
            MidiKind::NoteOn { key, velocity } => {
                assert_eq!(key.get(), 60);
                assert_eq!(velocity, MidiValue::from_u7(100));
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
    }

    #[test]
    fn decodes_note_off() {
        let ev = event(0x81, 64, 0);
        assert_eq!(ev.channel.get(), 1);
        assert!(matches!(
            ev.kind,
            MidiKind::NoteOff { key, .. } if key.get() == 64
        ));
    }

    #[test]
    fn note_on_velocity_zero_stays_note_on() {
        // MIDI 1.0 convention treats NoteOn vel=0 as NoteOff, but the
        // wire said NoteOn — the adapter preserves it losslessly and
        // leaves the interpretation to `effective_note_state` consumers.
        let ev = event(0x90, 60, 0);
        assert!(matches!(ev.kind, MidiKind::NoteOn { .. }));
    }

    #[test]
    fn decodes_control_change_program_and_pressures() {
        let cc = event(0xB4, 7, 100);
        assert!(matches!(
            cc.kind,
            MidiKind::ControlChange { controller, .. } if controller.get() == 7
        ));

        let pc = event(0xC0, 5, 0);
        assert!(matches!(
            pc.kind,
            MidiKind::ProgramChange { program } if program.get() == 5
        ));

        let chp = event(0xD2, 90, 0);
        assert!(matches!(
            chp.kind,
            MidiKind::ChannelPressure { pressure } if pressure == MidiValue::from_u7(90)
        ));

        let pp = event(0xA6, 48, 33);
        assert!(matches!(
            pp.kind,
            MidiKind::PolyPressure { key, pressure }
                if key.get() == 48 && pressure == MidiValue::from_u7(33)
        ));
    }

    #[test]
    fn decodes_pitch_bend_14bit() {
        // LSB=0x00, MSB=0x40 → raw 0x2000 = center → canonical 0.
        let center = event(0xE0, 0x00, 0x40);
        assert!(matches!(center.kind, MidiKind::PitchBend { value: 0 }));

        // LSB=0x7F, MSB=0x7F → raw 0x3FFF = max.
        let max = event(0xE0, 0x7F, 0x7F);
        match max.kind {
            MidiKind::PitchBend { value } => assert_eq!(value, i32::MAX),
            other => panic!("expected PitchBend, got {other:?}"),
        }

        // LSB=0x00, MSB=0x00 → raw 0 = min.
        let min = event(0xE0, 0x00, 0x00);
        match min.kind {
            MidiKind::PitchBend { value } => assert_eq!(value, i32::MIN),
            other => panic!("expected PitchBend, got {other:?}"),
        }
    }

    #[test]
    fn rejects_system_and_sysex_statuses() {
        for status in [0xF0u8, 0xF1, 0xF2, 0xF3, 0xF6, 0xF8, 0xFA, 0xFF] {
            assert_eq!(decode_message(status, 0, 0), None, "status {status:#04x}");
        }
    }

    #[test]
    fn rejects_malformed_status_and_data_bytes() {
        // Status without the high bit set is not a status byte.
        assert_eq!(decode_message(0x40, 0, 0), None);
        // Data bytes must be 7-bit.
        assert_eq!(decode_message(0x90, 0x80, 0), None);
        assert_eq!(decode_message(0x90, 0, 0xFF), None);
        assert_eq!(decode_message(0xE0, 0x7F, 0x80), None);
    }

    #[test]
    fn rejects_out_of_range_fields_without_panicking() {
        // d1 > 0x7F is already rejected above; these exercise the
        // newtype validation path with in-range bytes that still map
        // to every valid value — the constructors cannot fail for
        // 7-bit inputs, so this asserts the decode stays total.
        for key in 0..=0x7Fu8 {
            assert!(decode_message(0x90, key, 0).is_some());
        }
        for cc in 0..=0x7Fu8 {
            assert!(decode_message(0xB0, cc, 0).is_some());
        }
    }
}
