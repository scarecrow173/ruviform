//! Honest per-backend capability declarations (M4-01/M4-05).
//!
//! `Unknown`/`Declared`/`Unavailable` are *never* treated as supported;
//! only `Verified` is. The matrix is code — the doc table derives from
//! these values, not the other way around.

/// How well a capability is established for a backend.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CapabilityLevel {
    /// Proven end-to-end by executed tests on this layer/backend.
    Verified,
    /// Declared by the layer/backend but not proven — never treated
    /// as supported.
    Declared,
    /// Not established — never treated as supported.
    Unknown,
    /// Not available at this layer (e.g. L1 wire has no bus verb).
    Unavailable,
    /// Not supported by the plugin standard.
    Unsupported,
}

/// The hosting layer + plugin standard combination a capability row
/// describes.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    /// L0 in-process host, standard-neutral.
    L0,
    /// L1 out-of-process host, standard-neutral.
    L1,
    /// The CLAP adapter (`ruviform-plugin-clap`).
    Clap,
    /// The VST3 adapter (`ruviform-plugin-vst3`).
    Vst3,
}

/// One row of the capability matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterCapability {
    /// Bus enumeration via `Plugin::buses`/`PluginHost::buses`.
    pub bus_enumeration: CapabilityLevel,
    /// More than one audio bus presented to processing.
    pub multi_bus: CapabilityLevel,
    /// Aux/sidechain bus roles.
    pub aux_sidechain: CapabilityLevel,
    /// Process-start-relative event delivery (`EventList`).
    pub events: CapabilityLevel,
    /// In-block transport updates (e.g. CLAP `CLAP_EVENT_TRANSPORT`).
    /// `Verified` is required to adopt the in-block path — no adapter
    /// currently qualifies.
    pub in_block_transport: CapabilityLevel,
    /// Activated `min_frames`/`max_frames` enforced on the process path.
    pub min_frames_enforced: CapabilityLevel,
    /// Save/restore state verbs.
    pub state_save_restore: CapabilityLevel,
    /// Crash isolation (panic/failure converges to `Failed`, host and
    /// other instances continue).
    pub crash_isolation: CapabilityLevel,
    /// Timed `ParamEvent` delivery precision (`frame_offset` inside
    /// `EventList`). `Verified` = sample-accurate path proven on this
    /// layer; `Declared` = offset preserved but block capacity caps
    /// apply (CLAP 256/block, VST3 1024/queue).
    pub param_event_precision: CapabilityLevel,
    /// Per-note/per-voice parameter targeting — `Unsupported` on every
    /// backend: `EventKind` carries no note identity and adapter
    /// note-targeting (e.g. CLAP `pckn`) collapses to global.
    pub per_note_param: CapabilityLevel,
    /// Plugin→host parameter value reporting (gestures, rescan,
    /// output parameter queues). `Declared` = the adapter supplies
    /// and collects the standard's output channel into canonical
    /// reports (`PluginProcessor::drain_output_reports`) but no
    /// host-graph consumer is wired yet; `Unavailable` = the
    /// adapter acks and drops the channel (CLAP output events) or
    /// never surfaces it.
    pub param_reporting: CapabilityLevel,
    /// Channel-voice MIDI delivery (`EventKind::Midi` → adapter event
    /// vocabulary). `Verified` = proven on this layer; `Declared` =
    /// translated with documented narrowing; `Unsupported` = the
    /// adapter drops or never receives them — never silently.
    pub midi_channel_voice: CapabilityLevel,
    /// UMP/MIDI 2.0 channel-voice input (16-bit velocity, 32-bit bend,
    /// group routing). `Declared` = accepted with explicit narrowing
    /// into the canonical representation; `Unsupported` = rejected.
    pub midi_ump: CapabilityLevel,
    /// Per-note expression vocabulary (CLAP `NoteExpression`/`NoteChoke`
    /// /`NoteEnd`, VST3 note-expression/legacy CC events). The neutral
    /// `MidiKind` has no note-expression payload yet — `Unsupported`
    /// is the honest level: adoption is a defined skip, not a collapse.
    pub note_expression: CapabilityLevel,
    /// System/SysEx transport. `Unsupported` on every backend — the
    /// wire vocabulary has no variant and events are defined-skipped.
    pub midi_sysex: CapabilityLevel,
    /// Free-text caveat attached to the row.
    pub notes: &'static str,
}

/// The capability row for `backend` — the matrix's single authority.
///
/// **RT contract:** non-RT only — declarative metadata.
#[must_use]
pub const fn capability_for(backend: Backend) -> AdapterCapability {
    use CapabilityLevel::{Declared, Unavailable, Unsupported, Verified};
    match backend {
        Backend::L0 => AdapterCapability {
            bus_enumeration: Verified,
            multi_bus: Verified,
            aux_sidechain: Verified,
            events: Verified,
            in_block_transport: Declared,
            min_frames_enforced: Verified,
            state_save_restore: Verified,
            crash_isolation: Verified,
            param_event_precision: Verified,
            per_note_param: Unsupported,
            param_reporting: Unavailable,
            midi_channel_voice: Verified,
            midi_ump: Verified,
            note_expression: Unsupported,
            midi_sysex: Unsupported,
            notes: "in-block transport gated on adapter evidence; none exists",
        },
        Backend::L1 => AdapterCapability {
            bus_enumeration: Unavailable,
            multi_bus: Verified,
            aux_sidechain: Verified,
            events: Verified,
            in_block_transport: Declared,
            min_frames_enforced: Declared,
            state_save_restore: Verified,
            crash_isolation: Verified,
            param_event_precision: Verified,
            per_note_param: Unsupported,
            param_reporting: Unavailable,
            midi_channel_voice: Verified,
            midi_ump: Verified,
            note_expression: Unsupported,
            midi_sysex: Unsupported,
            notes: "bus enumeration has no wire verb yet; min_frames is the child contract's responsibility",
        },
        Backend::Clap => AdapterCapability {
            bus_enumeration: Unavailable,
            multi_bus: Unavailable,
            aux_sidechain: Unavailable,
            events: Declared,
            in_block_transport: Declared,
            min_frames_enforced: Declared,
            state_save_restore: Verified,
            crash_isolation: Verified,
            param_event_precision: Declared,
            per_note_param: Unsupported,
            param_reporting: Unavailable,
            midi_channel_voice: Declared,
            midi_ump: Declared,
            note_expression: Unsupported,
            midi_sysex: Unsupported,
            notes: "adapter exposes a single stereo main bus; audio-ports scanning is generalized but not exposed via Plugin::buses; CLAP_EVENT_TRANSPORT is discarded; param events capped at 256/block; pckn collapses to global; output events discarded; NoteOn/NoteOff/NoteExpression/NoteChoke/NoteEnd and MidiSysEx are defined-skipped (no neutral payload) — CLAP Midi/Midi2 channel-voice bytes narrow into EventKind::Midi",
        },
        Backend::Vst3 => AdapterCapability {
            bus_enumeration: Unavailable,
            multi_bus: Unavailable,
            aux_sidechain: Unavailable,
            events: Declared,
            in_block_transport: Unsupported,
            min_frames_enforced: Declared,
            state_save_restore: Verified,
            crash_isolation: Verified,
            param_event_precision: Declared,
            per_note_param: Unsupported,
            param_reporting: Declared,
            midi_channel_voice: Declared,
            midi_ump: Unsupported,
            note_expression: Unsupported,
            midi_sysex: Unsupported,
            notes: "single stereo main bus; no in-block transport event vocabulary adopted — CLAP behavior is not generalized; param queues capped at 1024 points with a scan bound; outputParameterChanges/outputEvents are collected into canonical drain_output_reports (fixture-proven in-process; no graph consumer wired yet); EventKind::Midi translates both directions — note on/off/poly-pressure → Vst note events, CC/channel-pressure/pitch-bend/program-change → kLegacyMIDICCOutEvent (SDK output-reserved, used on input per de-facto host practice) — with counted defined skips (group!=0, tuning!=0, out-of-range fields, unsupported types); IMidiMapping CC→param routing is snapshot-routed at prepare; real-plugin verification remains a gate",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix_is_honest() {
        let l0 = capability_for(Backend::L0);
        assert_eq!(l0.bus_enumeration, CapabilityLevel::Verified);
        assert_eq!(l0.multi_bus, CapabilityLevel::Verified);
        let l1 = capability_for(Backend::L1);
        assert_eq!(l1.bus_enumeration, CapabilityLevel::Unavailable);
        for backend in [Backend::L0, Backend::L1, Backend::Clap, Backend::Vst3] {
            assert_ne!(
                capability_for(backend).in_block_transport,
                CapabilityLevel::Verified,
                "no adapter may claim verified in-block transport"
            );
        }
        assert_eq!(
            capability_for(Backend::Vst3).in_block_transport,
            CapabilityLevel::Unsupported
        );
        assert_eq!(
            capability_for(Backend::L0).crash_isolation,
            CapabilityLevel::Verified
        );
        let _ = CapabilityLevel::Unknown;
        let _ = CapabilityLevel::Declared;
        for backend in [Backend::L0, Backend::L1, Backend::Clap, Backend::Vst3] {
            let row = capability_for(backend);
            assert_eq!(
                row.per_note_param,
                CapabilityLevel::Unsupported,
                "no backend may claim per-note param targeting yet"
            );
        }
        for backend in [Backend::L0, Backend::L1, Backend::Clap] {
            assert_eq!(
                capability_for(backend).param_reporting,
                CapabilityLevel::Unavailable,
                "only the VST3 adapter surfaces plugin→host reports so far"
            );
        }
        assert_eq!(
            capability_for(Backend::Vst3).param_reporting,
            CapabilityLevel::Declared,
            "Issue #253: VST3 output reporting is adapter-proven, not consumer-wired"
        );
        assert_eq!(
            capability_for(Backend::L0).param_event_precision,
            CapabilityLevel::Verified
        );
        assert_eq!(
            capability_for(Backend::Clap).param_event_precision,
            CapabilityLevel::Declared
        );
        assert_eq!(
            capability_for(Backend::Vst3).midi_channel_voice,
            CapabilityLevel::Declared,
            "Issue #253: VST3 MIDI translation is headless-proven, not real-plugin verified"
        );
    }
}
