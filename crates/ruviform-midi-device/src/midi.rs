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

/// Encodes one canonical [`MidiEvent`] into the packed
/// `status + data1 + data2` triple [`decode_message`] consumes — the
/// exact inverse mapping, for the `WinMM` `midiOutShortMsg` send path
/// (Issue #629).
///
/// Returns `None` for events the MIDI 1.0 channel-voice vocabulary
/// cannot represent on this port: non-zero UMP groups (a
/// `supports_ump = false` port has no group field) and future
/// [`MidiKind`] variants the encoder does not know. Callers must
/// treat `None` as an honest refusal, never a silent drop.
///
/// Values narrow through [`MidiValue`] canonical scale-to-max
/// conversions — the same math the decode direction widens with, so
/// `decode_message(encode_message(event))` round-trips every
/// representable MIDI 1.0 message.
///
/// **RT contract:** RT-suitable — pure, allocation-free, no panics.
#[must_use]
pub(crate) fn encode_message(event: MidiEvent) -> Option<(u8, u8, u8)> {
    // MIDI 1.0 byte-stream output has no group field — group ZERO is
    // the only group a `midiOut*` port can carry.
    if event.group != MidiGroup::ZERO {
        return None;
    }
    let channel = event.channel.get();
    let (status, data1, data2) = match event.kind {
        MidiKind::NoteOff { key, velocity } => (0x80, key.get(), MidiValue::as_u7(velocity)),
        MidiKind::NoteOn { key, velocity } => (0x90, key.get(), MidiValue::as_u7(velocity)),
        MidiKind::PolyPressure { key, pressure } => (0xA0, key.get(), MidiValue::as_u7(pressure)),
        MidiKind::ControlChange { controller, value } => {
            (0xB0, controller.get(), MidiValue::as_u7(value))
        }
        MidiKind::ProgramChange { program } => (0xC0, program.get(), 0),
        MidiKind::ChannelPressure { pressure } => (0xD0, MidiValue::as_u7(pressure), 0),
        MidiKind::PitchBend { value } => {
            // data1 = LSB, data2 = MSB of the raw 14-bit bend
            // (0x2000 = center) — the exact inverse of the decode
            // branch. `bend_as_u14` yields <= 0x3FFF by construction,
            // so both byte extractions stay 7-bit.
            let raw14 = MidiValue::bend_as_u14(value);
            let lo = u8::try_from(raw14 & 0x7F).ok()?;
            let hi = u8::try_from(raw14 >> 7).ok()?;
            (0xE0, lo, hi)
        }
        // Future `MidiKind` variants (the enum is `#[non_exhaustive]`)
        // are refused rather than guessed at.
        _ => return None,
    };
    Some((status | channel, data1, data2))
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

    fn ev(group: u8, channel: u8, kind: MidiKind) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::new(group).unwrap(),
            channel: MidiChannel::new(channel).unwrap(),
            kind,
        }
    }

    #[test]
    fn encode_inverts_decode_for_channel_voice() {
        // Every channel-voice status nibble, wire-representative data.
        let cases: [(u8, u8, u8); 7] = [
            (0x80, 60, 64),
            (0x90, 60, 100),
            (0xA0, 48, 33),
            (0xB0, 7, 100),
            (0xC0, 5, 0),
            (0xD0, 90, 0),
            (0xE0, 0x34, 0x12),
        ];
        for (status, d1, d2) in cases {
            let decoded = event(status, d1, d2);
            let (s, a, b) = encode_message(decoded).expect("channel-voice must encode");
            assert_eq!((s, a, b), (status, d1, d2), "triple for {status:#04x}");
            // And the full decode∘encode round-trip is the identity.
            assert_eq!(decode_message(s, a, b), Some(decoded));
        }
    }

    #[test]
    fn encode_channel_voice_kinds() {
        let note_on = ev(
            0,
            3,
            MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        );
        assert_eq!(encode_message(note_on), Some((0x93, 60, 100)));

        let pc = ev(
            0,
            5,
            MidiKind::ProgramChange {
                program: MidiProgram::new(9).unwrap(),
            },
        );
        // 2-byte message: data2 is zero on the wire.
        assert_eq!(encode_message(pc), Some((0xC5, 9, 0)));

        let chp = ev(
            0,
            2,
            MidiKind::ChannelPressure {
                pressure: MidiValue::from_u7(90),
            },
        );
        assert_eq!(encode_message(chp), Some((0xD2, 90, 0)));
    }

    #[test]
    fn encode_pitch_bend_min_center_max() {
        for (value, d1, d2) in [
            (i32::MIN, 0x00, 0x00),
            (0, 0x00, 0x40),
            (i32::MAX, 0x7F, 0x7F),
        ] {
            let bend = ev(0, 0, MidiKind::PitchBend { value });
            assert_eq!(encode_message(bend), Some((0xE0, d1, d2)));
            // decode∘encode is identity for wire-representable bends.
            let decoded = decode_message(0xE0, d1, d2).unwrap();
            assert_eq!(encode_message(decoded), Some((0xE0, d1, d2)));
        }
    }

    #[test]
    fn encode_rejects_nonzero_group() {
        let note_on = ev(
            1,
            0,
            MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        );
        // A MIDI 1.0 byte-stream port cannot carry group != 0 —
        // the encoder refuses rather than dropping the group field.
        assert_eq!(encode_message(note_on), None);
    }

    #[test]
    fn encode_narrows_canonical_values_to_7bit() {
        // Canonical u32 values wider than 7-bit narrow through
        // `MidiValue::as_u7` round-to-nearest — never a raw truncation.
        let hot = ev(
            0,
            0,
            MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: u32::MAX,
            },
        );
        assert_eq!(encode_message(hot), Some((0x90, 60, 0x7F)));
    }
}
