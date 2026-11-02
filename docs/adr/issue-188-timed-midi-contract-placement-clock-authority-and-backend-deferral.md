---
id: "ADR-NWm4RRQv0BypmTnFRlChlN"
type: "adr"
status: "proposed"
title: "Timed MIDI contract placement, clock authority, and deferral of OS backend selection"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/synvelis/issues/12"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-188-timed-midi-inventory.md"
    - "docs/specs/issue-188-timed-midi.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-188-timed-midi-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/188"
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: ["synvelis", "audio-lens"]
---

# Timed MIDI contract placement, clock authority, and deferral of OS backend selection

## Context and Problem Statement

Issue #188 requires a generic timed-MIDI foundation: native timestamp preservation, Audio/Plugin block delivery, clock-domain mapping, bounded RT handoff, note safety, and honest adapter capability. The inventory (`docs/discovery/issue-188-timed-midi-inventory.md`) shows the *scheduled* delivery skeleton already exists — `TransportPlan<EventKind>` + `TransportScheduler` + `PluginNode` + `EventList` (#190/#196) — while the MIDI vocabulary, input boundary, clock-domain vocabulary, live-input merge, note ledger, and adapter honesty do not. Six placement and policy questions must be decided:

1. Where the `MidiEvent`/`MidiKind`/`MidiChannel`/`MidiGroup`/`NoteKey` vocabulary lives across the layer boundary.
2. Where the MIDI input port trait (enumerate/open/close/backend capability) lives.
3. Which representation carries native input timestamps and who owns stamp→`EngineFrame` mapping.
4. How live input reaches the per-block `EventList` relative to the existing plan path.
5. Where note-safety (ledger/reset/chase) is owned.
6. Whether to select and implement an OS MIDI backend now or defer after comparison.

Additionally: whether a new crate is needed, and how unsupported adapter capabilities are handled.

## Decision Drivers

* Layering rule: neutral contracts in foundations (media-core/plugin-core), platform device boundaries in audio-core (`device` precedent), consumer-facing machinery in bridge, adapter-specific details in adapter crates (`docs/adr/synvelis-driven-generic-execution-boundaries.md`).
* `EngineFrame{epoch,frame}` is the sole authority for delivery position (#190). `DEVICE_CLOCK_TIMEBASE` (ns) and `OutputTimestamp` already exist; a separate "MIDI clock" must not fork the timeline.
* PPQN960 forced quantization is explicitly prohibited (issue constraint + #189 rational-beat policy); the native stamp must be preserved and handed to consumers.
* RT constraint: no-alloc/no-block/no-OS-I/O with bounded queues and defined overflow; preparation/publish/retire is non-RT.
* Capability honesty: `Unknown`/`Declared` must not be treated as supported; unsupported kinds/precision require defined rejection or counted drop — no silent conversion or loss.
* No new crate without demonstrated need (issue constraint + architecture baseline).
* `EventKind` is `#[non_exhaustive]` with `reserved` — additive variants are non-breaking; internal exhaustive matches surface at compile time.
* Synvelis is the only consumer; generic Control Graph / MIDI routing DAGs are YAGNI.

## Considered Options

* Option A: `MidiEvent` vocabulary in `ruviform-media-core`; `EventKind::Midi(MidiEvent)` in plugin-core; MIDI input port traits in `ruviform-audio-core::midi`; `TimedInputEvent`/`ClockAnchor`/mapper/`NoteLedger` in media-core; live-input stage + merge + ledger plumbing in bridge — **no new crate**.
* Option B: New `ruviform-midi-core` domain-foundation crate owning all MIDI vocabulary + port traits.
* Option C: `MidiEvent` in plugin-core only; bridge invents a second input event type — vocabulary forks.
* Option D: Route live input through `TransportScheduler` as `bound_event` injects (plan path) — scheduler gains a second source.
* Option E: Implement a real OS backend now (midir crate or direct OS API) — PoC on real hardware.

## Decision Outcome

Chosen option: **Option A** — media-core vocabulary + audio-core port traits + bridge machinery, no new crate, with a bounded live-input stage merged into the existing `EventList` delivery path.

### Placement reasoning

* `MidiEvent`/`MidiKind`/`MidiChannel`/`MidiGroup`/`NoteKey` in `ruviform-media-core::midi`: the vocabulary is a standards-neutral data model consumed by audio-core (input port), plugin-core (`EventKind::Midi`), bridge, and adapters. media-core is the lowest layer that all of these already depend on. Putting it in plugin-core (Option C) would force audio-core input ports to depend on a plugin-layer type or fork the vocabulary.
* `TimedInputEvent<P>`/`InputStamp`/`ClockDomainId`/`ClockAnchor`/stamp→frame mapper/`NoteLedger` in `ruviform-media-core::{timed_input,midi}`: these are the *authority* types for clock-domain identity and note state — pure value/logic types with no device knowledge, same pattern as `media_time_ticks`/`event_engine_frame`/`TempoMap`. The mapper is a pure integer function consuming a `ClockAnchor` snapshot; anchor calibration itself is non-RT policy owned by bridge.
* MIDI input port traits in `ruviform-audio-core::midi`: `audio-core::device` already establishes the Enumerator→Device→Stream port-trait pattern (non-RT `Box<dyn>`, `DeviceError` normalized errors, capability queries). MIDI input is the same platform-device boundary pattern applied to a new direction; `MidiPortKind{External,Virtual,Loopback,Unknown}` and `MidiSupport{Unknown,Unsupported,Supported}` honesty tri-state are defined there.
* Live-input stage + `merge_call_events` + `NoteLedger` plumbing in `ruviform-audio-plugin-bridge::input`: the bridge owns `NodeControl`/`NodeReport`/`PlanVault`/status counters and the `CallSink` that builds `EventList`s — it is the consumer-layer owner of bounded handoff + merge + fault visibility.

### Clock authority reasoning

* `EngineFrame{epoch,frame}` remains the sole delivery-position authority. Input stamps carry `domain: ClockDomainId` + `ticks:u64` + `base:MediaTimeBase` + `provenance`; mapping to frames happens exactly once via `ClockAnchor{domain, stamp_ticks, frame, calibration}` + the `frame_base` tick conversion. `ClockAnchor` is a *non-RT-calibrated snapshot* (the audio callback thread publishes it via `SetClockAnchor` control), so RT mapping is integer arithmetic only. Unknown domain → `UnmappedDomain` defined handling; `ClockEpochId` mismatch → fault, never subtraction across epochs (#190 epoch rule).
* Provenance inheritance: mapped-event reliability = min(stamp.provenance, anchor.calibration) — `Carried`+`Verified` → verified, any `Estimated`/`Unknown` → estimated/unknown. `TimestampProvenance` vocabulary is reused from media-core `observe`.

### Pipeline reasoning

* Live input does NOT go through `TransportScheduler` (Option D rejected): live events are a per-callback drain source, not a plan; folding them into the scheduler would complicate span/occurrence/ordering contracts. Instead `PluginNode`'s `CallSink` merges `build_call_event_list` (plan) output with `InputStage` (live) output via a new `merge_call_events` helper into one `EventList`. Merge convention: same-offset events keep per-source causal order; cross-source tie-break is plan-before-live; Note-Off precedes Note-On at same frame for the same channel+key (Note-Off priority, #188 requirement). This is a *deterministic documented convention*, not a scheduler policy change.
* `merge_call_events` lives in `ruviform-plugin-core::event` (or `transport_bridge`) as a pure function — same authoritative-function pattern as `sort_events_by_frame_offset`.

### Note-safety reasoning

* `NoteLedger` is a bounded channel+key→state map in media-core (pure logic, RT-safe, no-alloc). The bridge `CallSink` feeds it from merged `EventList`s and, on transport discontinuity (`TransportGeneration` change, `Stop`, `device_restart`), emits release events at offset 0 for active notes — same `bound_event`/generation-cut pattern #190 established for stale delivery. MIDI1 duplicate-pitch voice ambiguity is documented (per channel+key count, not per-voice identity).
* Ledger overflow → force-release oldest + fault counter (defined behavior, never silent drop).

### Adapter honesty reasoning

* `cap.rs` gains note/channel-event capability fields; CLAP implements `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` translation (clack 0.2.0 provides the types), VST3 gets `Declared`/`Unavailable` honesty + counted defined-drop in `fill_param_queues` (no `IEventList` impl this Issue). L1 wire adds kind tag `1` additively; old codec rejects unknown tags as `Malformed`. Non-`ParamValue` `let-else` skips in adapters become counted drops — never silent.

### Backend reasoning

* OS backend selection is **deferred after comparison** (Option E rejected for this Issue): midir's timestamp quality is OS-dependent (CoreMIDI host ticks / ALSA seq ticks / WinMM host-clock estimation), direct OS APIs are platform-specific and need real-device validation. PoC uses `MockMidiEnumerator`/`MockMidiInput` — a controlled `TimedInputEvent` source that exercises enumerate/open/close/stamp/domain/capability paths without hardware. The `audio-core::midi` port trait is designed so a real backend (midir or direct) can implement it as a follow-up adapter crate (e.g. `ruviform-audio-midi`) without touching foundations.

### New-crate decision

* **No new crate.** The vocabulary is small and fits media-core's primitives role; port traits fit audio-core's device-boundary role; bridge owns machinery. A `ruviform-midi-core` crate may be revisited if a real backend adapter plus UMP/serialization vocabulary outgrows media-core — recorded as a deferred concern.

## Consequences

* Good, because vocabulary/stamp/anchor/ledger are neutral and reusable by any consumer; `EventKind::Midi` rides the proven `TransportPlan`→`EventList`→`PluginNode` path for replay, and the live stage adds no scheduler complexity.
* Good, because stamp preservation is first-class (`TimedInputEvent` is a `Copy` value type; PPQN quantization never happens at the source) and clock-domain honesty is explicit (`domain` + `provenance` + anchor `calibration`).
* Good, because note safety is a single authoritative mechanism (`NoteLedger` + discontinuity release) rather than per-caller compensation.
* Good, because no new crate/workspace churn; `EventKind::Midi` is non-breaking.
* Bad, because `EventKind` now carries two semantically different payloads (param vs MIDI) — mitigated by `#[non_exhaustive]` and documented merge/ordering convention.
* Bad, because live-input delivery bypasses `TransportScheduler`'s receipt/dispatch-ledger observability — live events are observable via status counters/faults but not via plan receipts (deferred concern: recording tap).
* Bad, because VST3 MIDI delivery is still unimplemented (honest `Unavailable`) — capability matrix records the gap.

## Confirmation

Confirmed by: spec AC-3..AC-8 tests (Mock input→PluginNode→L0 E2E, stamp→offset mapping, block boundary/loop/tempo/duplicate-stamp/Note-Off-priority/0-frame, overflow visibility, note-ledger release, adapter translation, `rt_` no-alloc), `verify-ci` green, cap matrix honesty in `cap.rs`.

## Implementation Plan

* `crates/ruviform-media-core/src/midi.rs` (new): `MidiChannel`, `MidiGroup`, `NoteKey`, `MidiKind`, `MidiEvent`, canonical MSB-justified value helpers (`u7`/`u14`/`u16`/`u32` conversions), `NoteLedger` (bounded channel+key→state, release emission, force-release+overflow flag).
* `crates/ruviform-media-core/src/timed_input.rs` (new): `ClockDomainId`, `InputStamp`, `TimedInputEvent<P>`, `ClockAnchor`, `ClockCalibration`, `MapRejection`, `map_stamp_to_frame` (pure integer mapper).
* `crates/ruviform-plugin-core/src/event.rs`: `EventKind::Midi(MidiEvent)` variant + merge/ordering convention docs.
* `crates/ruviform-plugin-core/src/wire.rs`: kind tag `1` = Midi.
* `crates/ruviform-plugin-core/src/transport_bridge.rs` (or `event.rs`): `merge_call_events` pure merge helper.
* `crates/ruviform-audio-core/src/midi.rs` (new): `MidiPortKind`, `MidiSupport`, `MidiInputCaps`, `MidiPort`, `MidiInputConnection`, `MidiInputEnumerator`, `MidiInputError`, `MockMidiEnumerator`/`MockMidiInput` (test/PoC source).
* `crates/ruviform-audio-plugin-bridge/src/input.rs` (new): `InputStage` — bounded ring + drain budget + anchor snapshot + stamp mapping + mapped hold + counters.
* `crates/ruviform-audio-plugin-bridge/src/control.rs`: `NodeControl::SetClockAnchor` + input attach variant.
* `crates/ruviform-audio-plugin-bridge/src/node.rs`: `CallSink` merge wiring, `NoteLedger` feed + discontinuity release, `INPUT_DRAIN_MAX` budget.
* `crates/ruviform-audio-plugin-bridge/src/cap.rs`: note/channel-event capability fields.
* `crates/ruviform-plugin-clap/src/host.rs`: `EventKind::Midi` → clack `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` translation + counted defined-drop for non-representable.
* `crates/ruviform-plugin-vst3/src/host.rs`/`block.rs`: counted defined-drop for `Midi` (no `IEventList` impl); `cap.rs` honesty.
* Tests: `crates/ruviform-media-core/src/midi.rs`/`timed_input.rs` unit + `rt_` tests; `crates/ruviform-plugin-core/src/event.rs`/`wire.rs`/`transport_bridge.rs` merge + roundtrip; `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs` E2E; CLAP translation tests.

## More Information

* Follow-up Issues (split proposals): real OS backend adapter crate; typed `PortFormat::TimedEvent` payload + graph wiring; VST3 `IEventList`/note-expression; CLAP `NoteExpressionEvent`/`Midi2Event`/SysEx + plugin→host events; L1 wire MIDI verb + version negotiation; engine-layer recording tap; `NoteLedger` controller-chase extension; SysEx/System vocabulary + wire.
* Evidence: `docs/discovery/issue-188-timed-midi-inventory.md` (E1–E10).
