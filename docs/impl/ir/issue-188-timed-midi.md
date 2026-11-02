---
id: "IMPL-fGB1DM0VeWj7PjZgDkJ4th"
type: "impl"
status: "completed"
title: "Issue #188 timed MIDI input PoC — vocabulary, stamps, delivery merge, input port, InputStage, adapter honesty"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/188"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timed_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs"
      - type: "file"
        path: "docs/discovery/issue-188-timed-midi-inventory.md"
      - type: "file"
        path: "docs/specs/issue-188-timed-midi.md"
      - type: "file"
        path: "docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
      - type: "file"
        path: "docs/designs/issue-188-timed-midi.md"
      - type: "file"
        path: "docs/designs/issue-188-timed-midi-detailed.md"
      - type: "file"
        path: "docs/plans/issue-188-timed-midi.md"
      - type: "file"
        path: "docs/test-specs/issue-188-timed-midi.md"
      - type: "file"
        path: "docs/tasks/issue-188-01-media-core-midi-vocabulary.md"
      - type: "file"
        path: "docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md"
      - type: "file"
        path: "docs/tasks/issue-188-03-audio-core-midi-input-port.md"
      - type: "file"
        path: "docs/tasks/issue-188-04-bridge-input-stage-merge.md"
      - type: "file"
        path: "docs/tasks/issue-188-05-adapter-capability-honesty.md"
      - type: "file"
        path: "docs/tasks/issue-188-06-midi-delivery-tests.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "../../tasks/issue-188-01-media-core-midi-vocabulary.md"
    - "../../tasks/issue-188-02-plugin-core-eventkind-midi-merge.md"
    - "../../tasks/issue-188-03-audio-core-midi-input-port.md"
    - "../../tasks/issue-188-04-bridge-input-stage-merge.md"
    - "../../tasks/issue-188-05-adapter-capability-honesty.md"
    - "../../tasks/issue-188-06-midi-delivery-tests.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-188-timed-midi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-188-timed-midi.md"
    - "../../adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
    - "../../designs/issue-188-timed-midi-detailed.md"
    - "../../test-specs/issue-188-timed-midi.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #188 timed MIDI PoC — implementation record

## What was built

ADR Option A, in existing foundation crates (no new crate):

- **media-core `midi`** — channel-voice vocabulary: `MidiGroup`/`MidiChannel`/`NoteKey`/`MidiController`/`MidiProgram` newtypes, `MidiKind` (7 variants, `#[non_exhaustive]`), `MidiEvent`, canonical scale-to-max `MidiValue` widening/narrowing (7/14/16/32-bit + signed bend), `LedgerKey`, `NoteLedger` (`NOTE_LEDGER_CAPACITY=128`, overflow force-release observable, `release_all` for seek/stop).
- **media-core `timed_input`** — `ClockDomainId`, `InputStamp` (ticks+`TimeBase`+domain+`TimestampProvenance`), `ClockAnchor`, `MappedStamp`, `MapRejection`, `map_stamp_to_frame` (integer/rational math via `MediaDuration::convert`, `NearestEven`), `weaker_provenance`. Raw stamps are preserved for consumers — no PPQN quantization anywhere in the chain.
- **plugin-core `event`** — `EventKind::Midi(MidiEvent)` (additive, `#[non_exhaustive]` kept), `MAX_MERGE_SOURCES=8`, `MergeError{Overflow,TooManySources}`, `merge_call_events` — bounded stable k-way merge over caller scratch; equal offsets prefer earlier source index; same-offset colliding `LedgerKey` Note-Off precedes Note-On; source-local causal order preserved. RT `rt_merge_call_events_no_alloc` proves no-alloc.
- **plugin-core `wire`** — kind tag `1` for MIDI behind `encode_event`/`decode_event`: routing byte `group<<4|channel`, sub-tags 0–6, canonical payloads, two reserved `u64`s must be zero; unknown tags/sub-tags and out-of-range fields are `WireError::Malformed` — never silently encoded/dropped. `EventList` revalidation on decode unchanged.
- **audio-core `midi_input`** — port boundary mirroring `device.rs`: `MidiInputEnumerator` → `MidiInputPort` (`info`/`open`) → `MidiInput` (`read`/`close`), `MidiInputInfo`, `MidiPortOrigin{External,Virtual,Internal}`, `MidiPortCapabilities` (ump/timestamps/clock_domain — honest declarations), `MidiReadStats{dropped}` overflow observability, `TimedMidiInput` alias. All non-RT except the bounded drain contract.
- **bridge `input`** — `InputStage` (`INPUT_STAGE_CAPACITY=1024`): non-RT `read` into bounded pending + backend drop counters; block-boundary `collect_block` maps stamps→offsets (late→offset 0 counted, future stays pending, rejections counted); `release_all` emits ledger NoteOffs; `InputStageCounters`/`BlockInputStats`/`active_notes`/`pending_count` observability. `collect_block`/`release_all` RT-suitable (bounded, no alloc).
- **bridge `cap`** — four new capability fields: `midi_channel_voice`, `midi_ump`, `note_expression`, `midi_sysex`. L0/L1 `Verified`; CLAP `Declared`/`Declared`; VST3 `Unsupported` for all four with explicit defined-skip notes.
- **plugin-clap `adapt`** — `translate_midi_event`: `CoreEventSpace::Midi` (MIDI 1.0 bytes) and `Midi2` (UMP mt2/mt4) → `EventKind::Midi` channel voice; system/SysEx/NoteExpression/NoteChoke/NoteEnd defined-skipped; note-attribute bytes a defined narrowing. MIDI translation is wired into `translate_events` behind the param path.

## Evidence

- `tests/midi_delivery.rs` (bridge): 5 tests — Note On/Off, CC, Pitch Bend with µs-stamp→frame-offset mapping incl. duplicate stamps (stable order), block-boundary late/future/zero-frame, rejection + backend-drop observability, `release_all` on stop, `rt_midi_delivery_collect_and_merge_no_alloc`.
- CLAP `translate_events_translates_midi_channel_voice` — Note On / CC / bend center delivered as `EventKind::Midi` at correct offsets (representative-adapter Note/CC delivery).
- media-core: `rt_ledger_no_alloc`, `rt_map_stamp_no_alloc`; plugin-core: `rt_merge_call_events_no_alloc`; wire round-trip + malformed-input tests for all 7 sub-tags.

## Consumer impact

- **synvelis**: impacted (new capability surface; `#188` is the foundation for Synvelis #12 recording — no breaking changes; `EventKind` non-exhaustive addition is source-compatible).
- **audio-lens**: no impact (does not consume plugin-event surface) — *unverified*, repo not inspected in this worktree.

## Remaining work (follow-up issues to split)

- Bridge `PluginNode`/`CallSink` wiring of `InputStage` into the span call path (stage exists standalone; node integration deferred).
- VST3 `Vst::Event` note/legacy-CC translation (currently `Unsupported`, defined skip).
- OS MIDI backend adapter (e.g. midir/coremidi) implementing `MidiInputEnumerator`.
- `ScheduledEvent` replay path through `TransportScheduler` for recorded-MIDI playback (plan-level integration), loop/seek chase beyond `release_all`.
- `note_expression`/`midi_sysex` neutral vocabulary if needed — currently declared `Unsupported` honestly.

## Verification

`mise run fmt-check`, `check`, `lint`, `test`, `test-rt` — all green at record time; `verify-ci` run at finalize.
