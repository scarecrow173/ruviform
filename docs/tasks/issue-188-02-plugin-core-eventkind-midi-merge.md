---
id: "TASK-CKJ1IaOWUc0aUcraqOPD3q"
type: "task"
status: "done"
title: "plugin-core: EventKind::Midi + wire tag + merge_call_events"
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
    - "docs/tasks/issue-188-01-media-core-midi-vocabulary.md"
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

# plugin-core: EventKind::Midi + wire tag + merge_call_events

## Work

`ruviform-plugin-core` に `EventKind::Midi(MidiEvent)` を additive 追加し、wire codec と merge 権威を実装する。

- `event.rs`: `EventKind::Midi(MidiEvent)` variant（`#[non_exhaustive]` 維持、`Copy`、wire 形・`reserved` ポリシー不変）。merge/ordering 規約（causal order・plan-before-live・NoteOff priority）を doc で明文化。
- `wire.rs`: `EVENT_KIND_MIDI: u16 = 1` 追加、encode/decode 実装。未知 tag は既存 `Malformed` のまま。
- `transport_bridge.rs`（または `event.rs`）: `merge_call_events(plan:&[Event], live:&[Event], scratch:&mut [Event]) -> Result<&mut [Event], MergeOverflow>` を pure function として実装。規約: 非減少 offset 安定 merge・同一 offset cross-source plan-before-live・同一 `{group,channel,key}` NoteOff-before-NoteOn・scratch 不足は `MergeOverflow`。
- `lib.rs` re-export。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/designs/issue-188-timed-midi-detailed.md`（§3）

## Dependencies

- `docs/tasks/issue-188-01-media-core-midi-vocabulary.md`（`MidiEvent`/`MidiKind` 語彙）

## Affected Files

- `crates/ruviform-plugin-core/src/event.rs`
- `crates/ruviform-plugin-core/src/wire.rs`
- `crates/ruviform-plugin-core/src/transport_bridge.rs`
- `crates/ruviform-plugin-core/src/lib.rs`

## Done When

- [ ] `EventKind::Midi` が additive に追加され、`Event`/`EventList`/`reserved`/`ParamValue` 意味が不変
- [ ] wire codec が tag `0`/`1` で往復同一値を返し、未知 tag は `Malformed`
- [ ] `merge_call_events` が安定 merge・plan-before-live・NoteOff priority・`MergeOverflow` を実装
- [ ] merge/ordering 規約が event.rs doc に記述される
- [ ] `rt_` no-alloc テストが存在する

## Verification

- [ ] `mise run check` / `lint` / `test` / `test-rt` green（wire roundtrip・merge test 含む）
