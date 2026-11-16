//! VST3 input-event and process-context translation for
//! `test_instrument` — the fixture's own implementation of the VST3
//! event ABI → neutral [`Event`]/[`TransportInfo`] contracts.
//!
//! This module is deliberately independent of the production
//! `ruviform-plugin-vst3` adapter's `midi.rs`/`component.rs`
//! translators: both sides implement the same published VST3 ABI
//! semantics (the SDK contract), so a host↔plugin round trip through
//! this fixture is not a self-verification loop of one adapter.
//!
//! ## Event translation law
//!
//! Only bus `0` events are meaningful (the fixture declares no event
//! bus topology). Per event type:
//!
//! | VST3 `Event::type` | Neutral kind | Field mapping |
//! | --- | --- | --- |
//! | `kNoteOnEvent` | `MidiKind::NoteOn` | `pitch`→key, `velocity` f32→canonical `u32`; `tuning != 0` → skip |
//! | `kNoteOffEvent` | `MidiKind::NoteOff` | same; `tuning != 0` → skip |
//! | `kPolyPressureEvent` | `MidiKind::PolyPressure` | `pitch`→key, `pressure`→canonical `u32` |
//! | `kLegacyMIDICCOutEvent` | CC / channel pressure / pitch bend / program change | `controlNumber` selects; `value`/`value2` are 7-bit data bytes |
//! | everything else | — | defined skip (no canonical payload) |
//!
//! Channel `int16` must be `0..=15`, pitch `0..=127`, normalized
//! `f32` must be finite — violations are defined skips. Out-of-block
//! `sampleOffset`s are dropped (never clamped), matching the shared
//! `block_offset` policy. All MIDI-1.0 traffic maps to
//! `MidiGroup::ZERO` — VST3 has no group concept.
//!
//! ## Transport translation law
//!
//! `kPlaying` → [`TransportStatus::Playing`], anything else →
//! `Stopped`. `projectTimeSamples` → [`MediaTime`] at the negotiated
//! sample rate. Musical fields map to [`MusicalTimeInfo`] only when
//! `kProjectTimeMusicValid` **and** `kTempoValid` are both set with a
//! finite positive tempo — no implicit 4/4 or tempo is fabricated.
//! Quarter-note (`TQuarterNotes`) positions convert to PPQN-960
//! ticks by `round(ppq * 960)`; the signature maps only with
//! `kTimeSigValid`, and `bar_start_tick` only when
//! `kBarPositionValid` accompanies a valid signature. Loop/cycle
//! fields are not consumed by this fixture (the click oracle derives
//! entirely from position + tempo + ppqn).

use ruviform_media_core::{
    MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiProgram, MidiValue,
    MusicalTimeInfo, NoteKey, PPQN, TimeBase, TransportStatus,
};
use ruviform_plugin_core::{Event, EventKind, TransportInfo};
use vst3::ComRef;
use vst3::Steinberg::Vst::{
    ControllerNumbers_, Event as VstEvent, Event_::EventTypes_, IEventList, IEventListTrait,
    LegacyMIDICCOutEvent, ProcessContext as VstProcessContext, ProcessContext_,
};
use vst3::Steinberg::kResultTrue;

use crate::block::{self, EventScratch, MAX_BLOCK_EVENTS};

/// VST3 normalized `f32` `[0.0, 1.0]` → canonical `u32`
/// scale-to-max. `None` for non-finite input; out-of-range finite
/// values clamp — the defined narrowing for a host sending e.g.
/// `1.0000001`.
///
/// **RT contract:** RT-suitable — pure.
fn norm_f32_to_u32(value: f32) -> Option<u32> {
    if !value.is_finite() {
        return None;
    }
    let scaled = (f64::from(value.clamp(0.0, 1.0)) * f64::from(u32::MAX)).round();
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(scaled as u32)
}

/// `DefaultEnumType` → `u16` for `Event::type` — out-of-range clamps
/// to `u16::MAX` (the SDK constants all fit; the fallback only guards
/// a hypothetical future constant).
fn enum_to_u16(value: impl Into<i64>) -> u16 {
    u16::try_from(value.into()).unwrap_or(u16::MAX)
}

/// `DefaultEnumType` → `u8` for `LegacyMIDICCOutEvent.controlNumber`.
fn enum_to_u8(value: impl Into<i64>) -> u8 {
    u8::try_from(value.into()).unwrap_or(u8::MAX)
}

/// `StatesAndFlags` bit → `u32` for `ctx.state` masking — the SDK
/// typedef is `c_int` on Windows and `c_uint` elsewhere, so the
/// conversion widens through `i64` (both platform types convert
/// losslessly) instead of `as`, which is a no-op cast off-Windows.
/// Out-of-range clamps to `u32::MAX` — unreachable for the shipped
/// flag constants, matching the `enum_to_*` fallback.
fn flag_to_u32(value: impl Into<i64>) -> u32 {
    u32::try_from(value.into()).unwrap_or(u32::MAX)
}

/// Validates an `int16` VST3 channel field (`0..=15`) into the
/// canonical [`MidiChannel`].
fn midi_channel(raw: i16) -> Option<MidiChannel> {
    u8::try_from(raw)
        .ok()
        .and_then(|v| MidiChannel::new(v).ok())
}

/// Validates an `int16` VST3 pitch field (`0..=127`) into the
/// canonical [`NoteKey`].
fn note_key(raw: i16) -> Option<NoteKey> {
    u8::try_from(raw).ok().and_then(|v| NoteKey::new(v).ok())
}

/// The two 7-bit data bytes of a `LegacyMIDICCOutEvent` — `None` when
/// either byte is outside `0..=0x7F`.
fn legacy_cc_bytes(cc: LegacyMIDICCOutEvent) -> Option<(u8, u8)> {
    let value = u8::try_from(cc.value).ok()?;
    let value2 = u8::try_from(cc.value2).ok()?;
    (value <= 0x7F && value2 <= 0x7F).then_some((value, value2))
}

/// Wraps a translated payload with the canonical MIDI-1.0 routing —
/// `group = ZERO` always (VST3 has no group concept).
fn wrap_midi(kind: MidiKind, channel: MidiChannel) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel,
        kind,
    }
}

/// Translates one VST3 [`VstEvent`] into a canonical plugin-core
/// [`Event`] (frame offset + [`EventKind::Midi`]). `None` is a
/// defined skip — see the module docs for the full list.
///
/// Union access is `unsafe`: each arm reads `__field0` only after
/// `r#type` selects the member — the VST3 layout contract.
#[expect(unsafe_code)]
fn translate_event(raw: &VstEvent, frames: usize) -> Option<Event> {
    // Only bus 0 is meaningful — the fixture declares no event bus
    // topology, so any other index is an unexpected host input.
    if raw.busIndex != 0 {
        return None;
    }
    let offset = block::block_offset(raw.sampleOffset, frames)?;
    let r#type = raw.r#type;
    let midi = if r#type == enum_to_u16(EventTypes_::kNoteOnEvent) {
        // SAFETY: `r#type == kNoteOnEvent` selects the `noteOn` union
        // member — the only layout valid for this event.
        let note = unsafe { raw.__field0.noteOn };
        if note.tuning != 0.0 {
            return None;
        }
        wrap_midi(
            MidiKind::NoteOn {
                key: note_key(note.pitch)?,
                velocity: norm_f32_to_u32(note.velocity)?,
            },
            midi_channel(note.channel)?,
        )
    } else if r#type == enum_to_u16(EventTypes_::kNoteOffEvent) {
        // SAFETY: `r#type == kNoteOffEvent` selects `noteOff`.
        let note = unsafe { raw.__field0.noteOff };
        if note.tuning != 0.0 {
            return None;
        }
        wrap_midi(
            MidiKind::NoteOff {
                key: note_key(note.pitch)?,
                velocity: norm_f32_to_u32(note.velocity)?,
            },
            midi_channel(note.channel)?,
        )
    } else if r#type == enum_to_u16(EventTypes_::kPolyPressureEvent) {
        // SAFETY: `r#type == kPolyPressureEvent` selects
        // `polyPressure`.
        let note = unsafe { raw.__field0.polyPressure };
        wrap_midi(
            MidiKind::PolyPressure {
                key: note_key(note.pitch)?,
                pressure: norm_f32_to_u32(note.pressure)?,
            },
            midi_channel(note.channel)?,
        )
    } else if r#type == enum_to_u16(EventTypes_::kLegacyMIDICCOutEvent) {
        // SAFETY: `r#type == kLegacyMIDICCOutEvent` selects
        // `midiCCOut`.
        let cc = unsafe { raw.__field0.midiCCOut };
        let channel = midi_channel(i16::from(cc.channel))?;
        let control = cc.controlNumber;
        let (value, value2) = legacy_cc_bytes(cc)?;
        match control {
            // Plain CC — `controlNumber` in `0..=127`.
            0..=0x7F => wrap_midi(
                MidiKind::ControlChange {
                    controller: MidiController::new(control).ok()?,
                    value: MidiValue::from_u7(value),
                },
                channel,
            ),
            c if c == enum_to_u8(ControllerNumbers_::kAfterTouch) => wrap_midi(
                MidiKind::ChannelPressure {
                    pressure: MidiValue::from_u7(value),
                },
                channel,
            ),
            // Pitch bend — `value` = LSB, `value2` = MSB.
            c if c == enum_to_u8(ControllerNumbers_::kPitchBend) => wrap_midi(
                MidiKind::PitchBend {
                    value: MidiValue::bend_from_u14(u16::from(value) | (u16::from(value2) << 7)),
                },
                channel,
            ),
            c if c == enum_to_u8(ControllerNumbers_::kCtrlProgramChange) => wrap_midi(
                MidiKind::ProgramChange {
                    program: MidiProgram::new(value).ok()?,
                },
                channel,
            ),
            // `kCtrlPolyPressure` cannot reconstruct its key (the
            // legacy event has no pitch field); anything else is
            // unknown.
            _ => return None,
        }
    } else {
        // `kDataEvent` (SysEx), note-expression, chord/scale and
        // unknown types — no canonical representation.
        return None;
    };

    Some(Event::new(offset, EventKind::Midi(midi)))
}

/// Drains a host-supplied `IEventList` into canonical [`Event`]s
/// appended to `scratch` — the `test_instrument` input path. The
/// caller runs the parameter collector first (it owns `clear()`);
/// this collector only appends, bounded by the shared
/// [`MAX_BLOCK_EVENTS`] scratch capacity. Out-of-block offsets,
/// unsupported types, invalid fields and scratch overflow are all
/// defined skips.
///
/// # Safety
///
/// `event_list` may be null; otherwise it must point to a valid
/// `IEventList` for the duration of the call (the VST3 `process`
/// contract guarantees it for `data.inputEvents`).
#[expect(unsafe_code)]
pub(crate) unsafe fn collect_midi_events(
    event_list: *mut IEventList,
    frames: usize,
    events: &mut EventScratch,
) {
    // SAFETY: caller guarantees `event_list` is null or a valid
    // `IEventList` for this call — `ComRef` borrows without touching
    // the refcount.
    let Some(list) = (unsafe { ComRef::from_raw(event_list) }) else {
        return;
    };
    // SAFETY: `list` is a live `IEventList` for the call.
    let count = usize::try_from(unsafe { list.getEventCount() }.max(0)).unwrap_or(0);
    for index in 0..count.min(MAX_BLOCK_EVENTS) {
        let mut raw = core::mem::MaybeUninit::<VstEvent>::uninit();
        // SAFETY: `index < count` within this live list; `raw` is a
        // valid out-param the callee initializes on `kResultTrue`.
        if unsafe { list.getEvent(i32::try_from(index).unwrap_or(i32::MAX), raw.as_mut_ptr()) }
            != kResultTrue
        {
            continue;
        }
        // SAFETY: `getEvent` returned `kResultTrue`, so `raw` is
        // initialized.
        let Some(event) = (unsafe { translate_event(&*raw.as_ptr(), frames) }) else {
            continue;
        };
        // Scratch full → every remaining host event drops anyway;
        // stop scanning (bounded-work guarantee).
        if !events.push(event) {
            break;
        }
    }
}

/// Converts a VST3 PPQ (`TQuarterNotes`) position to PPQN-960 ticks:
/// `ppq × 960`, rounded to nearest. Saturates to the `i64` range.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
fn ppq_to_ticks(ppq: f64) -> i64 {
    let ticks = (ppq * f64::from(PPQN.ticks_per_quarter())).round();
    // `as` on a finite f64 saturates at i64 bounds.
    #[expect(clippy::cast_possible_truncation)]
    {
        ticks as i64
    }
}

/// Maps a VST3 `ProcessContext`'s musical fields to a neutral
/// [`MusicalTimeInfo`]: `Some` only when both
/// `kProjectTimeMusicValid` and `kTempoValid` are set and `tempo` is
/// a finite positive BPM — no implicit 4/4 or tempo is fabricated.
/// Signature maps only with `kTimeSigValid` (else `0`/`0`), and
/// `bar_start_tick` stays `0` unless `kBarPositionValid`
/// accompanies a valid signature.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
fn musical_from_context(ctx: &VstProcessContext) -> Option<MusicalTimeInfo> {
    use ProcessContext_::StatesAndFlags_ as F;
    let state = ctx.state;
    if state & flag_to_u32(F::kProjectTimeMusicValid) == 0
        || state & flag_to_u32(F::kTempoValid) == 0
        || !ctx.tempo.is_finite()
        || ctx.tempo <= 0.0
    {
        return None;
    }
    let bpm_milli = (ctx.tempo * 1_000.0).round().clamp(1.0, 999_999_999.0);
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bpm_milli = bpm_milli as u32;
    let (sig_num, sig_den) = if state & flag_to_u32(F::kTimeSigValid) != 0 {
        match (
            u8::try_from(ctx.timeSigNumerator),
            u8::try_from(ctx.timeSigDenominator),
        ) {
            (Ok(n @ 1..), Ok(d @ 1..)) => (n, d),
            _ => (0, 0),
        }
    } else {
        (0, 0)
    };
    Some(MusicalTimeInfo {
        position_ticks: ppq_to_ticks(ctx.projectTimeMusic),
        bar_start_tick: if sig_num != 0 && state & flag_to_u32(F::kBarPositionValid) != 0 {
            ppq_to_ticks(ctx.barPositionMusic)
        } else {
            0
        },
        bpm_milli,
        ppqn: PPQN.ticks_per_quarter(),
        sig_num,
        sig_den,
    })
}

/// Maps the optional VST3 `ProcessContext` to the neutral
/// [`TransportInfo`]: `kPlaying` → [`TransportStatus::Playing`],
/// otherwise `Stopped`; `projectTimeSamples` → [`MediaTime`] at the
/// negotiated rate; musical fields → [`MusicalTimeInfo`] via
/// [`musical_from_context`]. Returns `None` when the context is
/// absent or the rate unset — matching the neutral
/// `Option<TransportInfo>` contract.
///
/// # Safety
///
/// `process_context` must be null or point to a valid
/// `ProcessContext` for this call (VST3 `process` contract).
#[expect(unsafe_code)]
pub(crate) unsafe fn transport_from_context(
    process_context: *const VstProcessContext,
    sample_rate: u32,
) -> Option<TransportInfo> {
    use ProcessContext_::StatesAndFlags_ as F;
    if process_context.is_null() || sample_rate == 0 {
        return None;
    }
    // SAFETY: caller guarantees the pointer is valid for the call.
    let ctx = unsafe { &*process_context };
    let status = if ctx.state & flag_to_u32(F::kPlaying) != 0 {
        TransportStatus::Playing
    } else {
        TransportStatus::Stopped
    };
    let musical = musical_from_context(ctx);
    TimeBase::sample_rate(sample_rate).ok().map(|base| {
        let info = TransportInfo::new(status, MediaTime::new(ctx.projectTimeSamples, base));
        match musical {
            Some(m) => info.with_musical(m),
            None => info,
        }
    })
}
