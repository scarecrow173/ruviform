//! VST3 ↔ canonical MIDI event translation (Issue #253).
//!
//! Two directions over the same [`MidiEvent`] vocabulary
//! (`ruviform_media_core`, the Issue #188 canonical type):
//!
//! - **Host → plugin (write path):** [`outgoing_event`] maps a
//!   canonical [`EventKind::Midi`] payload to a VST3 [`Event`] that
//!   the host bridge stores in its `IEventList` and hands to
//!   `ProcessData.inputEvents`.
//! - **Plugin ← host (read path):** [`collect_incoming`] drains a
//!   host-supplied `IEventList` into canonical [`Event`]s appended to
//!   the component's bounded [`EventScratch`].
//!
//! ## Mapping policy
//!
//! | Canonical `MidiKind` | VST3 `Event::type` | Notes |
//! | --- | --- | --- |
//! | `NoteOn` | `kNoteOnEvent` | velocity `u32` → `f32` normalized `[0,1]` |
//! | `NoteOff` | `kNoteOffEvent` | same velocity narrowing |
//! | `PolyPressure` | `kPolyPressureEvent` | pressure `u32` → `f32` |
//! | `ControlChange` | `kLegacyMIDICCOutEvent` | `controlNumber` = controller |
//! | `ChannelPressure` | `kLegacyMIDICCOutEvent` | `controlNumber = kAfterTouch` |
//! | `PitchBend` | `kLegacyMIDICCOutEvent` | `controlNumber = kPitchBend`, 14-bit in `value`(LSB)/`value2`(MSB) |
//! | `ProgramChange` | `kLegacyMIDICCOutEvent` | `controlNumber = kCtrlProgramChange` |
//!
//! `kLegacyMIDICCOutEvent` is *reserved for plugin output* by the VST3
//! spec (3.6.12 change note), but de-facto hosts emit it on input —
//! JUCE's `createVstEvent` does exactly this — and plugins that do
//! not understand it ignore unknown event types per the VST3 contract.
//! Emitting it is the only way to carry channel pressure, pitch bend
//! and program change into `inputEvents`; `IMidiMapping`-routed
//! parameter delivery is a separate, unimplemented gate.
//!
//! ## Defined skips (counted, never silent)
//!
//! Both directions count [`MidiBridgeStats::skipped`] for:
//! `group != MidiGroup::ZERO` (VST3 has no group routing), non-zero
//! `busIndex`, out-of-block `sampleOffset`, unsupported event types
//! (`kDataEvent`/note-expression/chord/scale), invalid field ranges
//! (channel > 15, pitch > 127, non-finite normalized values),
//! `tuning != 0.0` (canonical has no tuning field), `noteId` /
//! `length` (note-expression linkage — canonical has none), and a
//! full bounded scratch/event list.
//!
//! `velocity`/`pressure` narrow `u32` → `f32` on the way out
//! (24-bit mantissa — documented precision loss; the round trip is
//! semantically exact on the 7-bit lattice but not bit-exact on the
//! full `u32` range) and widen `f32` → `u32` scale-to-max on the way
//! in.
//!
//! **RT contract:** the translation functions are pure and
//! allocation-free; `collect_incoming` is `O(min(count,
//! MAX_BLOCK_EVENTS))` bounded FFI reads with no allocation.

// The generated bindings declare flag/type constants as
// `DefaultEnumType` (`c_int` on Windows, `c_uint` elsewhere); the
// conversions below go through `i64` — same rationale as
// `crate::sdk_enum_i32`.
#![allow(clippy::unnecessary_cast)]

#[cfg(feature = "host")]
use std::cell::RefCell;

#[cfg(feature = "host")]
use ruviform_media_core::MusicalTimeInfo;
use ruviform_media_core::{
    MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiProgram, MidiValue, NoteKey,
};
#[cfg(feature = "host")]
use ruviform_plugin_core::EventList;
use ruviform_plugin_core::{Event, EventKind};
use vst3::ComRef;
use vst3::Steinberg::Vst::{
    ControllerNumbers_, Event as VstEvent, Event_::EventTypes_, IEventList, IEventListTrait,
    LegacyMIDICCOutEvent,
};
#[cfg(feature = "host")]
use vst3::Steinberg::Vst::{Event__type0, NoteOffEvent, NoteOnEvent, PolyPressureEvent};
use vst3::Steinberg::kResultTrue;
#[cfg(feature = "host")]
use vst3::Steinberg::{kResultFalse, kResultOk, tresult};
#[cfg(feature = "host")]
use vst3::{Class, ComWrapper};

use crate::block::{self, EventScratch, MAX_BLOCK_EVENTS};

/// Per-direction translation counters — the observable side of the
/// defined-skip policy above. `translated` counts canonical events
/// successfully emitted/consumed; `skipped` counts every defined
/// drop. Cumulative across blocks (the caller owns reset policy).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MidiBridgeStats {
    /// Events successfully translated.
    pub(crate) translated: u64,
    /// Events dropped by a defined skip (see module docs).
    pub(crate) skipped: u64,
}

impl MidiBridgeStats {
    /// Zeroed counters.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            translated: 0,
            skipped: 0,
        }
    }
}

/// VST3 `ppqPosition` (quarter notes, `f64`) for a frame offset,
/// computed from the block's [`MusicalTimeInfo`] snapshot when the
/// host supplies one. `0.0` when musical info is absent or
/// degenerate — an honest "unknown", never a fabricated position.
///
/// `position_ticks` is normalized by `ppqn` to quarter notes; the
/// frame delta converts at the block's `bpm_milli` tempo (a per-block
/// snapshot — mid-block tempo changes are not modeled, the same
/// approximation `ProcessContext` fields already carry).
///
/// **RT contract:** RT-suitable — pure float math.
#[cfg(feature = "host")]
#[must_use]
pub(crate) fn ppq_position(
    musical: Option<MusicalTimeInfo>,
    frame_offset: u32,
    sample_rate: u32,
) -> f64 {
    let Some(musical) = musical else {
        return 0.0;
    };
    if musical.ppqn == 0 || musical.bpm_milli == 0 || sample_rate == 0 {
        return 0.0;
    }
    // `i64 → f64` keeps the full tick range approximately — the
    // defined precision policy of the `ProcessContext` mapping.
    #[expect(clippy::cast_precision_loss)]
    let base = musical.position_ticks as f64 / f64::from(musical.ppqn);
    // quarter-notes per frame = (bpm_milli / 1000 beats-per-minute)
    // / 60 s/min / sample_rate frames-per-second.
    let per_frame = f64::from(musical.bpm_milli) / (60_000.0 * f64::from(sample_rate));
    base + f64::from(frame_offset) * per_frame
}

/// Canonical `u32` (scale-to-max) → VST3 normalized `f32` `[0.0, 1.0]`.
///
/// **RT contract:** RT-suitable — pure.
#[cfg(feature = "host")]
#[must_use]
fn u32_to_norm_f32(value: u32) -> f32 {
    // `f64 → f32` is the defined 24-bit-mantissa narrowing — the
    // module docs record the precision policy.
    #[expect(clippy::cast_possible_truncation)]
    {
        (f64::from(value) / f64::from(u32::MAX)) as f32
    }
}

/// VST3 normalized `f32` `[0.0, 1.0]` → canonical `u32` scale-to-max.
/// `None` for non-finite input (NaN would saturate to `0` on the `as`
/// cast and silently turn a `NoteOn` into a note-off — rejected
/// instead). Out-of-range finite values clamp — the defined
/// narrowing for a host sending e.g. `1.0000001`.
///
/// **RT contract:** RT-suitable — pure.
#[must_use]
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
/// a hypothetical future constant, same policy as `sdk_enum_i32`).
fn enum_to_u16(value: impl Into<i64>) -> u16 {
    u16::try_from(value.into()).unwrap_or(u16::MAX)
}

/// `DefaultEnumType` → `u8` for `LegacyMIDICCOutEvent.controlNumber`.
fn enum_to_u8(value: impl Into<i64>) -> u8 {
    u8::try_from(value.into()).unwrap_or(u8::MAX)
}

/// `u8` 7-bit value → `i8` for `LegacyMIDICCOutEvent` fields — the
/// inputs are `<= 0x7F` by construction (`MidiValue::as_u7`,
/// `MidiChannel`, `MidiProgram`); the `unwrap_or` arm is unreachable
/// but keeps the conversion total.
#[cfg(feature = "host")]
fn u7_to_i8(value: u8) -> i8 {
    i8::try_from(value).unwrap_or(i8::MAX)
}

/// Wraps a translated payload with the canonical MIDI-1.0 routing —
/// `group = ZERO` always (VST3 has no group concept) plus the
/// validated channel.
fn wrap_midi(kind: MidiKind, channel: MidiChannel) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel,
        kind,
    }
}

/// Translates one canonical [`MidiEvent`] into a VST3 [`Event`] for
/// the host→plugin direction. `None` is a defined skip — currently
/// only `group != MidiGroup::ZERO` (VST3 channel voice has no group
/// routing; MIDI 2.0/UMP groups are Issue #317 scope).
///
/// `length`/`noteId` are emitted as "unknown" (`0`/`-1`) — the
/// canonical event carries no duration or note-expression linkage.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
#[cfg(feature = "host")]
#[must_use]
pub(crate) fn outgoing_event(
    frame_offset: u32,
    midi: &MidiEvent,
    ppq_position: f64,
) -> Option<VstEvent> {
    if midi.group != MidiGroup::ZERO {
        return None;
    }
    let channel = i16::from(midi.channel.get());
    let cc_channel = u7_to_i8(midi.channel.get());
    let sample_offset = i32::try_from(frame_offset).unwrap_or(i32::MAX);
    let event = |r#type: u16, field: Event__type0| VstEvent {
        busIndex: 0,
        sampleOffset: sample_offset,
        ppqPosition: ppq_position,
        flags: 0,
        r#type,
        __field0: field,
    };
    let legacy_cc = |control_number: u8, value: u8, value2: u8| {
        event(
            enum_to_u16(EventTypes_::kLegacyMIDICCOutEvent),
            Event__type0 {
                midiCCOut: LegacyMIDICCOutEvent {
                    controlNumber: control_number,
                    channel: cc_channel,
                    value: u7_to_i8(value),
                    value2: u7_to_i8(value2),
                },
            },
        )
    };
    match midi.kind {
        MidiKind::NoteOn { key, velocity } => Some(event(
            enum_to_u16(EventTypes_::kNoteOnEvent),
            Event__type0 {
                noteOn: NoteOnEvent {
                    channel,
                    pitch: i16::from(key.get()),
                    tuning: 0.0,
                    velocity: u32_to_norm_f32(velocity),
                    length: 0,
                    noteId: -1,
                },
            },
        )),
        MidiKind::NoteOff { key, velocity } => Some(event(
            enum_to_u16(EventTypes_::kNoteOffEvent),
            Event__type0 {
                noteOff: NoteOffEvent {
                    channel,
                    pitch: i16::from(key.get()),
                    velocity: u32_to_norm_f32(velocity),
                    noteId: -1,
                    tuning: 0.0,
                },
            },
        )),
        MidiKind::PolyPressure { key, pressure } => Some(event(
            enum_to_u16(EventTypes_::kPolyPressureEvent),
            Event__type0 {
                polyPressure: PolyPressureEvent {
                    channel,
                    pitch: i16::from(key.get()),
                    pressure: u32_to_norm_f32(pressure),
                    noteId: -1,
                },
            },
        )),
        // `LegacyMIDICCOutEvent` layout: `value` carries the first
        // data byte (7-bit value / pitch-bend LSB), `value2` the
        // second (pitch-bend MSB only — `vsteventshelper` semantics).
        MidiKind::ControlChange { controller, value } => {
            Some(legacy_cc(controller.get(), MidiValue::as_u7(value), 0))
        }
        MidiKind::ChannelPressure { pressure } => Some(legacy_cc(
            enum_to_u8(ControllerNumbers_::kAfterTouch),
            MidiValue::as_u7(pressure),
            0,
        )),
        MidiKind::PitchBend { value } => {
            let bend = MidiValue::bend_as_u14(value);
            Some(legacy_cc(
                enum_to_u8(ControllerNumbers_::kPitchBend),
                u8::try_from(bend & 0x7F).unwrap_or(u8::MAX),
                u8::try_from(bend >> 7).unwrap_or(u8::MAX),
            ))
        }
        MidiKind::ProgramChange { program } => Some(legacy_cc(
            enum_to_u8(ControllerNumbers_::kCtrlProgramChange),
            program.get(),
            0,
        )),
        // `MidiKind` is `#[non_exhaustive]` — a future variant (MIDI
        // 2.0/UMP payloads are Issue #317 scope) has no VST3
        // channel-voice mapping yet: defined skip.
        _ => None,
    }
}

/// Reads one `LegacyMIDICCOutEvent`'s 7-bit `value`/`value2` fields —
/// `None` when either is outside `0..=127` (spec range; a hostile or
/// buggy host must not feed negative bytes into `from_u7`).
fn legacy_cc_bytes(cc: LegacyMIDICCOutEvent) -> Option<(u8, u8)> {
    let value = u8::try_from(cc.value).ok()?;
    let value2 = u8::try_from(cc.value2).ok()?;
    (value <= 0x7F && value2 <= 0x7F).then_some((value, value2))
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

/// Translates one VST3 [`Event`] into a canonical plugin-core
/// [`Event`] (frame offset + [`EventKind::Midi`]). `None` is a
/// defined skip — see the module docs for the full list.
///
/// Union access is `unsafe`: each arm reads `__field0` only after
/// `r#type` selects the member — the VST3 layout contract.
#[expect(unsafe_code)]
fn incoming_event(raw: &VstEvent, frames: usize) -> Option<Event> {
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
            // legacy event has no pitch field); system messages have
            // no canonical payload; anything else is unknown.
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
/// appended to `scratch` — the plugin-side read path.
///
/// Bounded: at most `MAX_BLOCK_EVENTS` `getEvent` inspections; the
/// loop stops early once the scratch is full (every later event
/// would drop anyway). Out-of-block offsets, unsupported types,
/// invalid fields and scratch overflow all count as
/// [`MidiBridgeStats::skipped`].
///
/// # Safety
///
/// `event_list` may be null; otherwise it must point to a valid
/// `IEventList` for the duration of the call (the VST3 `process`
/// contract guarantees it for `data.inputEvents`).
#[expect(unsafe_code)]
pub(crate) unsafe fn collect_incoming(
    event_list: *mut IEventList,
    frames: usize,
    scratch: &mut EventScratch,
    stats: &mut MidiBridgeStats,
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
            stats.skipped += 1;
            continue;
        }
        // SAFETY: `getEvent` returned `kResultTrue`, so `raw` was
        // initialized with a complete `Event` value.
        let event = unsafe { raw.assume_init() };
        match incoming_event(&event, frames) {
            Some(event) => {
                if scratch.push(event) {
                    stats.translated += 1;
                } else {
                    // Scratch full — every remaining event would
                    // drop anyway; stop the bounded scan.
                    stats.skipped += 1;
                    break;
                }
            }
            None => stats.skipped += 1,
        }
    }
}

/// Host-side `IEventList` implementation — a bounded `Vec<VstEvent>`
/// the bridge fills per block and hands to `ProcessData.inputEvents`.
/// The same object also serves as the fixture's fake *incoming* list
/// in plugin-side tests.
///
/// `RefCell` interior mutability mirrors `BridgeParamQueue`: the COM
/// trait methods take `&self` while the VST3 contract serializes
/// access per object.
#[cfg(feature = "host")]
pub(crate) struct BridgeEventList {
    events: RefCell<Vec<VstEvent>>,
}

#[cfg(feature = "host")]
impl BridgeEventList {
    /// Capacity for `MAX_BLOCK_EVENTS` events — reserved once at
    /// setup; `push_event` beyond it is a defined `false`.
    pub(crate) fn with_capacity() -> Self {
        Self {
            events: RefCell::new(Vec::with_capacity(MAX_BLOCK_EVENTS)),
        }
    }

    /// Clears the list — called once per `process` before refill.
    ///
    /// **RT contract:** RT-suitable — `O(1)` length reset.
    pub(crate) fn clear(&self) {
        if let Ok(mut events) = self.events.try_borrow_mut() {
            events.clear();
        }
    }

    /// Appends a translated event; `false` when the bounded capacity
    /// is full — the defined truncation the stats count.
    ///
    /// **RT contract:** RT-suitable — bounded, allocation-free while
    /// `len() < MAX_BLOCK_EVENTS`.
    pub(crate) fn push_event(&self, event: VstEvent) -> bool {
        let Ok(mut events) = self.events.try_borrow_mut() else {
            return false;
        };
        if events.len() >= MAX_BLOCK_EVENTS {
            return false;
        }
        events.push(event);
        true
    }
}

#[cfg(feature = "host")]
impl Class for BridgeEventList {
    type Interfaces = (IEventList,);
}

#[cfg(feature = "host")]
impl IEventListTrait for BridgeEventList {
    /// Number of events collected for this block.
    #[expect(unsafe_code)]
    unsafe fn getEventCount(&self) -> i32 {
        self.events
            .try_borrow()
            .map_or(0, |events| i32::try_from(events.len()).unwrap_or(i32::MAX))
    }

    /// Copies event `index` into the caller's out-param. Out-of-range
    /// or a null `e` is `kResultFalse` — the VST3 error contract.
    #[expect(unsafe_code)]
    unsafe fn getEvent(&self, index: i32, e: *mut VstEvent) -> tresult {
        if e.is_null() {
            return kResultFalse;
        }
        let Ok(events) = self.events.try_borrow() else {
            return kResultFalse;
        };
        let Some(event) = usize::try_from(index).ok().and_then(|i| events.get(i)) else {
            return kResultFalse;
        };
        // SAFETY: `e` is a valid out-param per the VST3 contract
        // (caller-allocated `Event`); `event` is a `Copy` value.
        unsafe { e.write(*event) };
        kResultOk
    }

    /// Appends an event — used by plugins writing *output* events;
    /// bounded at `MAX_BLOCK_EVENTS`.
    #[expect(unsafe_code)]
    unsafe fn addEvent(&self, e: *mut VstEvent) -> tresult {
        if e.is_null() {
            return kResultFalse;
        }
        // SAFETY: `e` is a valid `Event` pointer per the VST3 contract
        // for the duration of the call; the value is `Copy`.
        let event = unsafe { *e };
        if self.push_event(event) {
            kResultTrue
        } else {
            kResultFalse
        }
    }
}

/// Fills a [`BridgeEventList`] from a canonical [`EventList`]: every
/// [`EventKind::Midi`] event translates through [`outgoing_event`];
/// `ParamValue` and unknown kinds are not MIDI and are not counted.
/// Returns the per-call stats (translated/skipped MIDI events).
///
/// **RT contract:** RT-suitable — bounded iteration, no allocation.
#[cfg(feature = "host")]
pub(crate) fn fill_event_list(
    event_list: &ComWrapper<BridgeEventList>,
    events: EventList<'_>,
    musical: Option<MusicalTimeInfo>,
    sample_rate: u32,
) -> MidiBridgeStats {
    let mut stats = MidiBridgeStats::new();
    event_list.clear();
    for event in events.iter().take(MAX_BLOCK_EVENTS) {
        let EventKind::Midi(midi) = event.kind() else {
            continue;
        };
        let ppq = ppq_position(musical, event.frame_offset(), sample_rate);
        match outgoing_event(event.frame_offset(), &midi, ppq) {
            Some(vst) => {
                if event_list.push_event(vst) {
                    stats.translated += 1;
                } else {
                    stats.skipped += 1;
                    break;
                }
            }
            None => stats.skipped += 1,
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use vst3::Steinberg::Vst::{
        Event_::EventTypes_::*, Event__type0, NoteOffEvent, NoteOnEvent, PolyPressureEvent,
    };
    use vst3::Steinberg::{kResultFalse, kResultOk, tresult};
    use vst3::{Class, ComWrapper};

    fn midi_event(kind: MidiKind) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(3).unwrap(),
            kind,
        }
    }

    fn note_on() -> MidiEvent {
        midi_event(MidiKind::NoteOn {
            key: NoteKey::new(60).unwrap(),
            velocity: MidiValue::from_u7(100),
        })
    }

    /// VST3 normalized `f32` for a canonical 7-bit value — the same
    /// normalization the host-gated outgoing path applies, replicated
    /// literally so the always-on incoming fixtures do not depend on
    /// the `host` feature.
    fn norm_f32_of_u7(v: u8) -> f32 {
        #[expect(clippy::cast_possible_truncation)]
        {
            (f64::from(MidiValue::from_u7(v)) / f64::from(u32::MAX)) as f32
        }
    }

    /// Literal VST3 event shell — union *construction* is safe; only
    /// union reads need `unsafe` (inside `incoming_event`).
    fn vst_shell(r#type: u16, offset: i32, field: Event__type0) -> VstEvent {
        VstEvent {
            busIndex: 0,
            sampleOffset: offset,
            ppqPosition: 0.0,
            flags: 0,
            r#type,
            __field0: field,
        }
    }

    fn vst_note_on(offset: i32, channel: i16, pitch: i16, velocity: f32) -> VstEvent {
        vst_shell(
            enum_to_u16(kNoteOnEvent),
            offset,
            Event__type0 {
                noteOn: NoteOnEvent {
                    channel,
                    pitch,
                    tuning: 0.0,
                    velocity,
                    length: 0,
                    noteId: -1,
                },
            },
        )
    }

    fn vst_note_off(offset: i32, channel: i16, pitch: i16, velocity: f32) -> VstEvent {
        vst_shell(
            enum_to_u16(kNoteOffEvent),
            offset,
            Event__type0 {
                noteOff: NoteOffEvent {
                    channel,
                    pitch,
                    velocity,
                    noteId: -1,
                    tuning: 0.0,
                },
            },
        )
    }

    fn vst_poly_pressure(offset: i32, channel: i16, pitch: i16, pressure: f32) -> VstEvent {
        vst_shell(
            enum_to_u16(kPolyPressureEvent),
            offset,
            Event__type0 {
                polyPressure: PolyPressureEvent {
                    channel,
                    pitch,
                    pressure,
                    noteId: -1,
                },
            },
        )
    }

    fn vst_legacy_cc(offset: i32, control: u8, channel: i8, value: i8, value2: i8) -> VstEvent {
        vst_shell(
            enum_to_u16(kLegacyMIDICCOutEvent),
            offset,
            Event__type0 {
                midiCCOut: LegacyMIDICCOutEvent {
                    controlNumber: control,
                    channel,
                    value,
                    value2,
                },
            },
        )
    }

    /// Minimal read-only `IEventList` for the plugin-side drain tests —
    /// mirrors the `TestQueue`/`TestChanges` fixtures in `component.rs`.
    struct TestEventList {
        events: Vec<VstEvent>,
    }

    impl Class for TestEventList {
        type Interfaces = (IEventList,);
    }

    impl IEventListTrait for TestEventList {
        #[expect(unsafe_code)]
        unsafe fn getEventCount(&self) -> i32 {
            i32::try_from(self.events.len()).unwrap_or(i32::MAX)
        }

        #[expect(unsafe_code)]
        unsafe fn getEvent(&self, index: i32, e: *mut VstEvent) -> tresult {
            if e.is_null() {
                return kResultFalse;
            }
            let Some(event) = usize::try_from(index).ok().and_then(|i| self.events.get(i)) else {
                return kResultFalse;
            };
            // SAFETY: `e` is a valid out-param per the VST3 contract;
            // `event` is a `Copy` value.
            unsafe { e.write(*event) };
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn addEvent(&self, _e: *mut VstEvent) -> tresult {
            kResultFalse
        }
    }

    /// Semantic equality for round-trip assertions — the
    /// `u32 → f32 → u32` velocity/pressure path is documented
    /// narrowing (exact on the 7-bit lattice, not bit-exact on the
    /// full range), so values compare through their source-width
    /// projection while routing and discrete fields compare exactly.
    fn assert_midi_semantic_eq(got: &MidiEvent, want: &MidiEvent) {
        assert_eq!(got.group, want.group);
        assert_eq!(got.channel, want.channel);
        // Discriminant equality first — the merged or-patterns below
        // would otherwise allow NoteOn↔NoteOff cross-matching.
        assert_eq!(
            core::mem::discriminant(&got.kind),
            core::mem::discriminant(&want.kind),
            "kind mismatch: {got:?} vs {want:?}"
        );
        match (got.kind, want.kind) {
            (
                MidiKind::NoteOn {
                    key: gk,
                    velocity: gv,
                }
                | MidiKind::NoteOff {
                    key: gk,
                    velocity: gv,
                },
                MidiKind::NoteOn {
                    key: wk,
                    velocity: wv,
                }
                | MidiKind::NoteOff {
                    key: wk,
                    velocity: wv,
                },
            ) => {
                assert_eq!(gk, wk);
                assert_eq!(MidiValue::as_u7(gv), MidiValue::as_u7(wv));
            }
            (
                MidiKind::ControlChange {
                    controller: gc,
                    value: gv,
                },
                MidiKind::ControlChange {
                    controller: wc,
                    value: wv,
                },
            ) => {
                assert_eq!(gc, wc);
                assert_eq!(MidiValue::as_u7(gv), MidiValue::as_u7(wv));
            }
            (MidiKind::PitchBend { value: gv }, MidiKind::PitchBend { value: wv }) => {
                assert_eq!(MidiValue::bend_as_u14(gv), MidiValue::bend_as_u14(wv));
            }
            (
                MidiKind::PolyPressure {
                    key: gk,
                    pressure: gp,
                },
                MidiKind::PolyPressure {
                    key: wk,
                    pressure: wp,
                },
            ) => {
                assert_eq!(gk, wk);
                assert_eq!(MidiValue::as_u7(gp), MidiValue::as_u7(wp));
            }
            (
                MidiKind::ChannelPressure { pressure: gp },
                MidiKind::ChannelPressure { pressure: wp },
            ) => {
                assert_eq!(MidiValue::as_u7(gp), MidiValue::as_u7(wp));
            }
            (MidiKind::ProgramChange { program: gp }, MidiKind::ProgramChange { program: wp }) => {
                assert_eq!(gp, wp);
            }
            _ => unreachable!("discriminant matched but pair did not: {got:?} vs {want:?}"),
        }
    }

    #[cfg(feature = "host")]
    #[test]
    #[expect(unsafe_code)]
    fn outgoing_note_on_maps_fields() {
        let vst = outgoing_event(42, &note_on(), 1.5).unwrap();
        assert_eq!(vst.busIndex, 0);
        assert_eq!(vst.sampleOffset, 42);
        assert_eq!(vst.ppqPosition, 1.5);
        assert_eq!(vst.r#type, enum_to_u16(kNoteOnEvent));
        // SAFETY: type selects `noteOn`.
        let note = unsafe { vst.__field0.noteOn };
        assert_eq!(note.channel, 3);
        assert_eq!(note.pitch, 60);
        assert_eq!(note.velocity, u32_to_norm_f32(MidiValue::from_u7(100)));
        assert_eq!(note.noteId, -1);
        assert_eq!(note.tuning, 0.0);
    }

    #[cfg(feature = "host")]
    #[test]
    fn outgoing_group_nonzero_is_defined_skip() {
        let mut midi = note_on();
        midi.group = MidiGroup::new(1).unwrap();
        assert!(outgoing_event(0, &midi, 0.0).is_none());
    }

    #[cfg(feature = "host")]
    #[test]
    #[expect(unsafe_code)]
    fn outgoing_pitch_bend_uses_lsb_msb_pair() {
        let midi = midi_event(MidiKind::PitchBend {
            value: MidiValue::bend_from_u14(0x3ABC),
        });
        let vst = outgoing_event(0, &midi, 0.0).unwrap();
        assert_eq!(vst.r#type, enum_to_u16(kLegacyMIDICCOutEvent));
        // SAFETY: type selects `midiCCOut`.
        let cc = unsafe { vst.__field0.midiCCOut };
        assert_eq!(cc.controlNumber, enum_to_u8(ControllerNumbers_::kPitchBend));
        assert_eq!(cc.channel, 3);
        assert_eq!(cc.value, i8::try_from(0x3ABC & 0x7F).unwrap());
        assert_eq!(cc.value2, i8::try_from(0x3ABC >> 7).unwrap());
    }

    #[cfg(feature = "host")]
    #[test]
    #[expect(unsafe_code)]
    fn outgoing_program_change_channel_pressure_and_cc() {
        let pc = midi_event(MidiKind::ProgramChange {
            program: MidiProgram::new(42).unwrap(),
        });
        // SAFETY: type selects `midiCCOut`.
        let vst = outgoing_event(0, &pc, 0.0).unwrap();
        let cc = unsafe { vst.__field0.midiCCOut };
        assert_eq!(
            cc.controlNumber,
            enum_to_u8(ControllerNumbers_::kCtrlProgramChange)
        );
        assert_eq!(cc.value, 42);

        let cp = midi_event(MidiKind::ChannelPressure {
            pressure: MidiValue::from_u7(64),
        });
        let vst = outgoing_event(0, &cp, 0.0).unwrap();
        // SAFETY: type selects `midiCCOut`.
        let cc = unsafe { vst.__field0.midiCCOut };
        assert_eq!(
            cc.controlNumber,
            enum_to_u8(ControllerNumbers_::kAfterTouch)
        );
        assert_eq!(cc.value, 64);

        let cc_midi = midi_event(MidiKind::ControlChange {
            controller: MidiController::new(7).unwrap(),
            value: MidiValue::from_u7(100),
        });
        let vst = outgoing_event(0, &cc_midi, 0.0).unwrap();
        // SAFETY: type selects `midiCCOut`.
        let cc = unsafe { vst.__field0.midiCCOut };
        assert_eq!(cc.controlNumber, 7);
        assert_eq!(cc.value, 100);
        assert_eq!(cc.value2, 0);
    }

    #[test]
    fn incoming_note_on_maps_canonical_fields() {
        let vst = vst_note_on(7, 3, 60, norm_f32_of_u7(100));
        let event = incoming_event(&vst, 64).unwrap();
        assert_eq!(event.frame_offset(), 7);
        let EventKind::Midi(got) = event.kind() else {
            panic!("expected Midi kind");
        };
        assert_midi_semantic_eq(&got, &note_on());
    }

    #[test]
    fn incoming_all_kinds_map_semantically() {
        // `value` = bend LSB, `value2` = MSB — `0x1234` → `0x34`/`0x24`.
        let cases: [(VstEvent, MidiEvent); 6] = [
            (
                vst_note_off(5, 3, 62, norm_f32_of_u7(64)),
                midi_event(MidiKind::NoteOff {
                    key: NoteKey::new(62).unwrap(),
                    velocity: MidiValue::from_u7(64),
                }),
            ),
            (
                vst_legacy_cc(5, 11, 3, 90, 0),
                midi_event(MidiKind::ControlChange {
                    controller: MidiController::new(11).unwrap(),
                    value: MidiValue::from_u7(90),
                }),
            ),
            (
                vst_legacy_cc(5, enum_to_u8(ControllerNumbers_::kPitchBend), 3, 0x34, 0x24),
                midi_event(MidiKind::PitchBend {
                    value: MidiValue::bend_from_u14(0x1234),
                }),
            ),
            (
                vst_poly_pressure(5, 3, 60, norm_f32_of_u7(33)),
                midi_event(MidiKind::PolyPressure {
                    key: NoteKey::new(60).unwrap(),
                    pressure: MidiValue::from_u7(33),
                }),
            ),
            (
                vst_legacy_cc(5, enum_to_u8(ControllerNumbers_::kAfterTouch), 3, 80, 0),
                midi_event(MidiKind::ChannelPressure {
                    pressure: MidiValue::from_u7(80),
                }),
            ),
            (
                vst_legacy_cc(
                    5,
                    enum_to_u8(ControllerNumbers_::kCtrlProgramChange),
                    3,
                    17,
                    0,
                ),
                midi_event(MidiKind::ProgramChange {
                    program: MidiProgram::new(17).unwrap(),
                }),
            ),
        ];
        for (vst, want) in cases {
            let event = incoming_event(&vst, 64).unwrap();
            assert_eq!(event.frame_offset(), 5);
            let EventKind::Midi(got) = event.kind() else {
                panic!("expected Midi kind for {want:?}");
            };
            assert_midi_semantic_eq(&got, &want);
        }
    }

    /// The normalized `f32` endpoints map to the canonical `u32`
    /// scale-to-max extremes exactly.
    #[test]
    fn incoming_velocity_extremes_map_exactly() {
        for (velocity, want) in [(0.0_f32, 0_u32), (1.0, u32::MAX)] {
            let vst = vst_note_on(0, 3, 60, velocity);
            let event = incoming_event(&vst, 64).unwrap();
            let EventKind::Midi(got) = event.kind() else {
                panic!("expected Midi kind");
            };
            let MidiKind::NoteOn {
                velocity: got_v, ..
            } = got.kind
            else {
                panic!("expected NoteOn");
            };
            assert_eq!(got_v, want);
        }
    }

    #[test]
    fn incoming_skips_unsupported_and_invalid() {
        // SysEx (`kDataEvent`) — no canonical payload. Union fields
        // are never read for skipped types, so a zeroed payload is a
        // valid hostile-input stand-in.
        let data = vst_shell(
            enum_to_u16(kDataEvent),
            0,
            Event__type0 {
                data: vst3::Steinberg::Vst::DataEvent {
                    size: 0,
                    r#type: 0,
                    bytes: core::ptr::null(),
                },
            },
        );
        assert!(incoming_event(&data, 64).is_none());

        // Out-of-block offset drops regardless of type.
        let vst = vst_note_on(64, 3, 60, norm_f32_of_u7(100));
        assert!(incoming_event(&vst, 64).is_none());

        // Non-zero bus index drops.
        let mut vst = vst_note_on(0, 3, 60, norm_f32_of_u7(100));
        vst.busIndex = 1;
        assert!(incoming_event(&vst, 64).is_none());

        // Non-zero tuning is unrepresentable → defined skip. Union
        // writes target the `noteOn` member selected by `r#type`.
        vst.__field0.noteOn.tuning = 12.5;
        assert!(incoming_event(&vst, 64).is_none());

        // NaN velocity is rejected, not saturated to a note-off.
        vst.__field0.noteOn.tuning = 0.0;
        vst.__field0.noteOn.velocity = f32::NAN;
        assert!(incoming_event(&vst, 64).is_none());

        // `kCtrlPolyPressure` legacy CC cannot reconstruct its key —
        // defined skip, not a guessed channel-pressure.
        let cc = vst_legacy_cc(
            0,
            enum_to_u8(ControllerNumbers_::kCtrlPolyPressure),
            3,
            1,
            0,
        );
        assert!(incoming_event(&cc, 64).is_none());
    }

    #[test]
    fn incoming_rejects_out_of_range_fields() {
        let mut vst = vst_note_on(0, 3, 60, norm_f32_of_u7(100));
        vst.__field0.noteOn.pitch = 200;
        assert!(incoming_event(&vst, 64).is_none());
        vst.__field0.noteOn.pitch = 60;
        vst.__field0.noteOn.channel = 16;
        assert!(incoming_event(&vst, 64).is_none());
    }

    #[cfg(feature = "host")]
    #[test]
    fn ppq_position_uses_musical_snapshot() {
        let musical = MusicalTimeInfo {
            position_ticks: 960 * 4,
            bar_start_tick: 0,
            bpm_milli: 120_000,
            ppqn: 960,
            sig_num: 4,
            sig_den: 4,
        };
        // 4 quarter notes base + 96000 frames @48k, 120bpm → 2s → 4
        // quarter notes → 8.0 total.
        let ppq = ppq_position(Some(musical), 96_000, 48_000);
        assert!((ppq - 8.0).abs() < 1e-9);
        assert_eq!(ppq_position(None, 96_000, 48_000), 0.0);
    }

    /// Host-side fill: canonical `EventList` → `IEventList`. Only
    /// `EventKind::Midi` counts — `ParamValue` passes through
    /// untouched (the param-queue path owns it); a `group != 0`
    /// event is a defined skip; `sampleOffset` survives verbatim.
    #[cfg(feature = "host")]
    #[test]
    #[expect(unsafe_code)]
    fn fill_event_list_translates_counts_and_preserves_offsets() {
        let group1 = {
            let mut midi = note_on();
            midi.group = MidiGroup::new(1).unwrap();
            midi
        };
        let param = Event::new(
            20,
            EventKind::ParamValue(ruviform_plugin_core::ParamEvent {
                param_id: ruviform_plugin_core::ParamId(0),
                value: ruviform_plugin_core::ParamValue::new(0.5).unwrap(),
            }),
        );
        let events = [
            Event::new(3, EventKind::Midi(note_on())),
            Event::new(9, EventKind::Midi(group1)),
            param,
        ];
        let list_events = EventList::new(&events, 64).unwrap();
        let list = ComWrapper::new(BridgeEventList::with_capacity());
        let stats = fill_event_list(&list, list_events, None, 48_000);
        assert_eq!(stats.translated, 1);
        assert_eq!(stats.skipped, 1);

        let com = list.to_com_ptr::<IEventList>().unwrap();
        // SAFETY: `com` is a live IEventList for this test.
        assert_eq!(unsafe { com.getEventCount() }, 1);
        let mut raw = core::mem::MaybeUninit::<VstEvent>::uninit();
        // SAFETY: index 0 is in range; `raw` is a valid out-param.
        assert_eq!(unsafe { com.getEvent(0, raw.as_mut_ptr()) }, kResultOk);
        // SAFETY: `getEvent` returned `kResultOk` — `raw` initialized.
        let got = unsafe { raw.assume_init() };
        assert_eq!(got.sampleOffset, 3);
        assert_eq!(got.r#type, enum_to_u16(kNoteOnEvent));
    }

    /// Plugin-side read: a host `IEventList` drains into canonical
    /// `Event`s in the bounded scratch — unsupported types count as
    /// skips, valid events keep their frame offsets and payloads.
    #[test]
    #[expect(unsafe_code)]
    fn collect_incoming_drains_to_scratch_with_stats() {
        let bend = midi_event(MidiKind::PitchBend {
            value: MidiValue::bend_from_u14(0x1234),
        });
        let list = ComWrapper::new(TestEventList {
            events: vec![
                vst_note_on(5, 3, 60, norm_f32_of_u7(100)),
                // `value` = bend LSB, `value2` = MSB — `0x1234`.
                vst_legacy_cc(9, enum_to_u8(ControllerNumbers_::kPitchBend), 3, 0x34, 0x24),
                // SysEx — no canonical payload, defined skip.
                vst_shell(
                    enum_to_u16(kDataEvent),
                    12,
                    Event__type0 {
                        data: vst3::Steinberg::Vst::DataEvent {
                            size: 0,
                            r#type: 0,
                            bytes: core::ptr::null(),
                        },
                    },
                ),
            ],
        });

        let com = list.to_com_ptr::<IEventList>().unwrap();
        let mut scratch = EventScratch::empty();
        scratch.configure();
        let mut stats = MidiBridgeStats::new();
        // SAFETY: `com` is a live IEventList for this call.
        unsafe { collect_incoming(com.as_ptr(), 64, &mut scratch, &mut stats) };
        scratch.sort();
        assert_eq!(stats.translated, 2);
        assert_eq!(stats.skipped, 1);

        let events = scratch.as_slice();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].frame_offset(), 5);
        assert_eq!(events[1].frame_offset(), 9);
        let EventKind::Midi(first) = events[0].kind() else {
            panic!("expected Midi kind");
        };
        assert_midi_semantic_eq(&first, &note_on());
        let EventKind::Midi(second) = events[1].kind() else {
            panic!("expected Midi kind");
        };
        assert_midi_semantic_eq(&second, &bend);
    }

    #[cfg(feature = "host")]
    #[test]
    #[expect(unsafe_code)]
    fn bridge_event_list_com_round_trip() {
        let list = ComWrapper::new(BridgeEventList::with_capacity());
        let com = list.to_com_ptr::<IEventList>().unwrap();
        // SAFETY: `com` is a live IEventList for this test.
        assert_eq!(unsafe { com.getEventCount() }, 0);
        let vst = outgoing_event(3, &note_on(), 0.0).unwrap();
        assert!(list.push_event(vst));
        // SAFETY: same live object.
        assert_eq!(unsafe { com.getEventCount() }, 1);
        // Read back through the COM trait — the same path a plugin
        // takes on `inputEvents`.
        let mut raw = core::mem::MaybeUninit::<VstEvent>::uninit();
        // SAFETY: `raw` is a valid out-param; index 0 is in range.
        assert_eq!(unsafe { com.getEvent(0, raw.as_mut_ptr()) }, kResultOk);
        // SAFETY: getEvent initialized `raw`.
        let got = unsafe { raw.assume_init() };
        assert_eq!(got.r#type, vst.r#type);
        assert_eq!(got.sampleOffset, 3);
        // Out-of-range and null are defined refusals.
        let mut raw2 = core::mem::MaybeUninit::<VstEvent>::uninit();
        // SAFETY: out-param valid; index out of range → kResultFalse.
        assert_eq!(unsafe { com.getEvent(5, raw2.as_mut_ptr()) }, kResultFalse);
        // SAFETY: null out-param → kResultFalse, never a write.
        assert_eq!(
            unsafe { com.getEvent(0, core::ptr::null_mut()) },
            kResultFalse
        );
    }
}
