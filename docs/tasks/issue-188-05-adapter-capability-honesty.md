---
id: "TASK-RjMEQhLpvbKbkrKJQvqsz8"
type: "task"
status: "done"
title: "adapters: cap.rs note/channel-event fields + CLAP translation + VST3 honesty"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-188-timed-midi.md"
  implemented-by:
    - "../impl/ir/issue-188-timed-midi.md"
  depends-on:
    - "docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-188-timed-midi.md"
  references: []
  defers: []
  deferred-by: []
---

# adapters: cap.rs note/channel-event fields + CLAP translation + VST3 honesty

## Work

adapter 能力行列とイベント変換を拡張する。未対応 kind/精度の暗黙消失を defined drop へ置き換える。

- `crates/ruviform-audio-plugin-bridge/src/cap.rs`: `AdapterCapability` に `note_events`/`channel_events`/`note_expression`/`sysex`（全 `MidiSupport` 三値）を追加。行列値: L0=Supported/Declared、L1=Declared、CLAP=Supported（PoC で translation 実装）、VST3=Unavailable。
- `crates/ruviform-plugin-clap/src/host.rs`: `push_events` の `let EventKind::ParamValue = .. else continue` を `match` 化。`Midi` → `MidiKind` ごとに clack `NoteOnEvent`/`NoteOffEvent`（velocity f64、Pckn{port,channel,key,note_id=-1}）または `MidiEvent`（CC/Bend/Pressure/PC が 7bit 表現可能な場合）へ変換。表現不能精度 → counted drop。
- `crates/ruviform-plugin-vst3/src/host.rs`/`block.rs`: `EventKind::Midi` → counted defined-drop。`IEventList` 実装は分割 Issue（cap で Unavailable 宣言）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/designs/issue-188-timed-midi-detailed.md`（§6）

## Dependencies

- `docs/tasks/issue-188-02-plugin-core-eventkind-midi-merge.md`（`EventKind::Midi`）

## Affected Files

- `crates/ruviform-audio-plugin-bridge/src/cap.rs`
- `crates/ruviform-plugin-clap/src/host.rs`
- `crates/ruviform-plugin-vst3/src/host.rs`

## Done When

- [ ] `AdapterCapability` に note/channel-event フィールドが追加され行列値が正直に宣言される
- [ ] CLAP `push_events` が `Midi` → `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` を翻訳し、表現不能精度は counted drop
- [ ] VST3 `fill_param_queues` が `Midi` を counted defined-drop する
- [ ] non-ParamValue の暗黙 `let-else` skip が全て `match` + count へ置き換えられる
- [ ] CLAP translation の単体テストが存在する

## Verification

- [ ] `mise run check` / `lint` / `test` green
- [ ] `mise run test-l1-real-plugin` / `lint-l1-real-plugin` green（CLAP/VST3 変更）
