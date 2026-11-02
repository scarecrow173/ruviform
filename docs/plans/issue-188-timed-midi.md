---
id: "PLAN-ayJ9gag4RycXPJ01POnAkp"
type: "plan"
status: "approved"
title: "Issue #188 Timed MIDI — implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timed_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/control.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/status.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/attach.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/tasks/issue-188-01-media-core-midi-vocabulary.md"
    - "docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md"
    - "docs/tasks/issue-188-03-audio-core-midi-input-port.md"
    - "docs/tasks/issue-188-04-bridge-input-stage-merge.md"
    - "docs/tasks/issue-188-05-adapter-capability-honesty.md"
    - "docs/tasks/issue-188-06-midi-delivery-tests.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-188-timed-midi.md"
  references:
    - "docs/discovery/issue-188-timed-midi-inventory.md"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 188
---

# Issue #188 Timed MIDI — implementation plan

## Goal

ADR の Option A を実装する: media-core に MIDI 語彙（`midi`）+ stamp/anchor/mapper/NoteLedger（`timed_input`）、plugin-core に `EventKind::Midi` + wire tag `1` + `merge_call_events`、audio-core に MIDI input port trait + Mock、bridge に `InputStage` + merge/ledger 配線 + control/status/attach 拡張、adapter に cap honesty + CLAP translation + VST3 defined-drop。L0 mock E2E と `rt_` lane で検証する。

## Scope

単一実装ストリーム。境界: 中立語彙/権威関数は media-core、wire/merge は plugin-core、input port は audio-core、consumer 機構は bridge、規格固有差分は adapter。OS backend・graph 配線・VST3 `IEventList`・CLAP NoteExpression/SysEx・L1 verb・録音 tap は分割 Issue。

## Upstream Documents

- `docs/specs/issue-188-timed-midi.md`
- `docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md`

## Design Inputs

- `docs/designs/issue-188-timed-midi.md`（overview・D1〜D8）
- `docs/designs/issue-188-timed-midi-detailed.md`（API 形・merge 規約・cap 行列・fault 分類）

## File Map

- `crates/ruviform-media-core/src/midi.rs` — `MidiChannel`/`MidiGroup`/`NoteKey`/`MidiKind`/`MidiEvent`/`MidiValue`/`NoteLedger`
- `crates/ruviform-media-core/src/timed_input.rs` — `ClockDomainId`/`InputStamp`/`TimedInputEvent`/`ClockAnchor`/`MapRejection`/`MappedInput`/`map_stamp_to_frame`
- `crates/ruviform-media-core/src/lib.rs` — `pub mod midi; pub mod timed_input;`
- `crates/ruviform-plugin-core/src/event.rs` — `EventKind::Midi` + merge/NoteOff-priority doc
- `crates/ruviform-plugin-core/src/wire.rs` — `EVENT_KIND_MIDI` tag + encode/decode
- `crates/ruviform-plugin-core/src/transport_bridge.rs` — `merge_call_events` + `MergeOverflow`
- `crates/ruviform-plugin-core/src/lib.rs` — re-export
- `crates/ruviform-audio-core/src/midi.rs` — port trait + `MidiPortKind`/`MidiSupport`/`MidiInputCaps`/`MidiInputError` + `midi::mock` Mock 実装
- `crates/ruviform-audio-core/src/lib.rs` — `pub mod midi;`
- `crates/ruviform-audio-plugin-bridge/src/input.rs` — `InputStage`/`InputConfig`/`InputTx`/`INPUT_DRAIN_MAX`/`INPUT_RING_CAPACITY`
- `crates/ruviform-audio-plugin-bridge/src/control.rs` — `NodeControl::SetClockAnchor`
- `crates/ruviform-audio-plugin-bridge/src/node.rs` — `CallSink` merge + `NoteLedger` + discontinuity release + drain wiring
- `crates/ruviform-audio-plugin-bridge/src/status.rs` — input counters
- `crates/ruviform-audio-plugin-bridge/src/attach.rs` — `AttachConfig::input`/`Attached::input_tx`
- `crates/ruviform-audio-plugin-bridge/src/cap.rs` — note/channel-event capability fields
- `crates/ruviform-audio-plugin-bridge/src/lib.rs` — `pub mod input;`
- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs` — Mock input→PluginNode→L0 E2E
- `crates/ruviform-plugin-clap/src/host.rs` — `EventKind::Midi` → `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` + counted drop
- `crates/ruviform-plugin-vst3/src/host.rs` — `EventKind::Midi` counted defined-drop

## Implementation Sequence

- [x] task-01: media-core `midi`+`timed_input`（語彙・canonical 値・stamp/anchor/mapper・NoteLedger、unit+`rt_` test、doctest）
- [x] task-02: plugin-core `EventKind::Midi`・wire tag `1`・`merge_call_events`（roundtrip・merge・NoteOff priority・overflow test）
- [x] task-03: audio-core `midi` port trait + `midi::mock`（enumerator/port/connection/caps/error）
- [x] task-04: bridge `input` module + control/status/attach/`CallSink` 配線（drain→map→merge→ledger→release→fault）
- [x] task-05: adapter cap 拡張 + CLAP translation + VST3 honesty + counted defined-drop
- [x] task-06: `midi_delivery.rs` E2E + `rt_` no-alloc + 全検証緑化

## Dependencies and Risks

- 依存: `ruviform-media-core`（`EngineFrame`/`MediaTimeBase`/`TimestampProvenance`/`convert`/`TransportPlan`）、`ruviform-plugin-core`（`Event`/`EventList`/`EventKind`/`PluginEndpoint`）、`ruviform-rt`（`spsc`/`SnapshotReader`/`assert_no_alloc`）、`ruviform-audio-core`（`device` パターン）、clack 0.2.0（`NoteOnEvent`/`NoteOffEvent`/`MidiEvent`）。
- リスク: `PluginNode`/`CallSink` 内部構造が merge 配線を受け入れるか — `build_call_event_list` の scratch と `EventScratchOverflow` パターンをそのまま反復する。`NodeControl`/`NodeReport` は `#[non_exhaustive]` で additive。
- リスク: `map_stamp_to_frame` の `convert` が media-core 公開 API で構成可能か — `media_time_ticks`/`event_engine_frame` と同じ `convert` 権威を使うため公開済みのはず。内部参照のみでなければ `pub(crate)`→`pub` 昇格は IR に記録。
- リスク: `MidiEvent` を `Event` wire 形（u32 offset + kind）に収める — `MidiEvent{group,channel,kind}` は `ParamEvent{ParamId(u32),ParamValue(f64)}` より小さいため `reserved` を使わず収まる。

## Task Breakdown

- `docs/tasks/issue-188-01-media-core-midi-vocabulary.md`
- `docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md`（depends-on: 01）
- `docs/tasks/issue-188-03-audio-core-midi-input-port.md`（depends-on: 01）
- `docs/tasks/issue-188-04-bridge-input-stage-merge.md`（depends-on: 02, 03）
- `docs/tasks/issue-188-05-adapter-capability-honesty.md`（depends-on: 02）
- `docs/tasks/issue-188-06-midi-delivery-tests.md`（depends-on: 04, 05）

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `verify-ci`
- [ ] `mise run test-l1-real-plugin` / `lint-l1-real-plugin`（CLAP/VST3 変更）
- [ ] AC-3〜AC-8 の各テストが存在し緑
- [ ] doctest が `check`/`test` に含まれる

## Open Questions / Gaps

- `MockMidiEnumerator`/`MockMidiInput` の公開方法（`midi::mock` pub module か feature 隔離か）は実装時に確定 — PoC テストから使える必要がある。
- `map_stamp_to_frame` の `convert` が media-core の既存公開 API で十分かどうか — 不足なら `pub(crate)` 関数の `pub` 昇格を IR に記録。

## Self-Review

- [x] Every spec requirement maps to a task（R1/R3/R4/R8/R9: task-01+04、R2/R6/R7: task-02+04、R5/R10/R12: task-04、R11: task-05、R13/R14/R15: 全 task）。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が直接実装）。
- Independent work streams: task-01 が完了すれば task-02 と task-03 は並行可能（crate 別）。task-04 は 02+03 依存、task-05 は 02 依存、task-06 は 04+05 依存。
- User approval required before dispatch: yes（worker は coordinator 委任済みとして進行）。
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: `map_stamp_to_frame` の defined rejection（UnmappedDomain/EpochMismatch/BeforeAnchor）が anchor/epoch 規約と一致しているか、`merge_call_events` の plan-before-live + NoteOff priority が spec R6 と一致するか、`NoteLedger` の force-release が安全側か、adapter の counted defined-drop が capability honesty を満たすか。
