---
id: "TASK-lzn8pZrsr6ThLMc0iKw8bN"
type: "task"
status: "done"
title: "tests: midi_delivery E2E + rt_ no-alloc + full verification"
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
    - "docs/tasks/issue-188-04-bridge-input-stage-merge.md"
    - "docs/tasks/issue-188-05-adapter-capability-honesty.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-188-timed-midi.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# tests: midi_delivery E2E + rt_ no-alloc + full verification

## Work

Mock input→`PluginNode`→L0 mock plugin の E2E テストと `rt_` 契約テストを追加し、全検証を緑化する。

- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs`: `param_delivery.rs` の mock fixture を反復。Mock `MidiInputEnumerator`→`PluginNode`→L0 plugin で `MidiEvent` の型・順序・frame offset・native stamp 保持を記録・検証する。`tests/param_delivery.rs` と同じ RecPlugin パターン（kind/frame_offset/値 を記録）を使う。
- 検証ケース（spec AC-3〜AC-8 に対応）:
  - Note On/Off・CC・PitchBend の型・順序・frame offset 配送（複数時刻・複数対象）。
  - stamp→frame 写像（実 anchor 経由の block frame offset 決定）、block 境界（境界上は次ブロック先頭）、0-frame flush。
  - Loop・tempo change（plan 経由で `EventKind::Midi` の再発火・generation 切断）。
  - 重複 timestamp（同時刻安定順）・NoteOff priority（同一 frame・同一 channel+key で NoteOff 先）。
  - input ring overflow（`InputQueueFull` counter+report）、unmapped domain（`InputUnmapped`）、before anchor（`InputBeforeAnchor`）、ledger overflow（`InputLedgerOverflow`+force-release）、merge scratch overflow（`InputMergeOverflow`）。
  - transport 不連続（tg 変化・Stop・`device_restart`）→ `NoteLedger::release_all` offset 0、stuck-note 予防。
  - `rt_` no-alloc（drain・map・merge・ledger・EventList 構築）。
  - `EventKind`/`TransportInfo`/`EventList`/`ScheduledEvent` の互換維持（`ParamValue` 配送が引き続き動作）。
- `crates/ruviform-media-core/src/midi.rs`/`timed_input.rs`、`crates/ruviform-plugin-core/src/event.rs`/`wire.rs`/`transport_bridge.rs` の単体+`rt_` テスト。
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` と `test-l1-real-plugin`/`lint-l1-real-plugin` を全緑化。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/test-specs/issue-188-timed-midi.md`

## Dependencies

- `docs/tasks/issue-188-04-bridge-input-stage-merge.md`
- `docs/tasks/issue-188-05-adapter-capability-honesty.md`

## Affected Files

- `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs`（新設）
- `crates/ruviform-media-core/src/midi.rs`（unit+rt_ test 内包）
- `crates/ruviform-media-core/src/timed_input.rs`（unit+rt_ test 内包）
- `crates/ruviform-plugin-core/src/event.rs`/`wire.rs`/`transport_bridge.rs`（unit test 内包）

## Done When

- [ ] `midi_delivery.rs` E2E が型・順序・offset・stamp 保持・block 境界・loop/tempo・重複 stamp・NoteOff priority・0-frame・overflow visibility・ledger release・不連続 release を検証
- [ ] `rt_` no-alloc テストが drain/map/merge/ledger/EventList 構築を測定
- [ ] `ParamValue` 配送の回帰（`param_delivery.rs` 既存 test 緑）
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` 全緑
- [ ] `test-l1-real-plugin`/`lint-l1-real-plugin` 緑

## Verification

- [ ] 上記 Done When 全項目
