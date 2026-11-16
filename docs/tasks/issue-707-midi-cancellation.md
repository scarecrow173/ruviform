---
id: "TASK-034YJlja5SyGAbJySCQZfM"
type: "task"
status: "done"
title: "Issue #707 — MIDI input cancellation fault タスク"
created: "2026-11-16"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/707"
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/plans/issue-707-midi-cancellation.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-707-midi-cancellation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
    - "docs/plans/issue-707-midi-cancellation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-707-midi-cancellation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-707-midi-cancellation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #707 — MIDI input cancellation fault タスク

## Work

`crates/ruviform-audio-core/src/midi_input/mock.rs` の
`MockPort`/`MockInput` に read-level fault injector
（`fail_next_read`/`inject_cancelled`/`is_terminated`）を追加し、
`crates/ruviform-audio-test/tests/midi_faults.rs` で `MidiLeg`
経由の正負 test を追加する。production trait・pipeline 配線・
audio 側 fault 実装は無変更が境界条件。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-707-midi-cancellation.md`
- `docs/designs/issue-707-midi-cancellation.md`
- `docs/plans/issue-707-midi-cancellation.md`
- `docs/test-specs/issue-707-midi-cancellation.md`

## Dependencies

- #628 close-out の audio 側 `StreamFault::Cancellation` 実装
  （merge `0b510cf8`）が main 着地済みであること — 本 slice の
  写像規約の先例。

## Affected Files

- `crates/ruviform-audio-core/src/midi_input/mock.rs` —
  `PortState`/`ReadFault`/injector/`read` 経路 + unit test
- `crates/ruviform-audio-test/tests/midi_faults.rs` — 新規 test file
- `docs/{specs,designs,plans,tasks,test-specs}/
  issue-707-midi-cancellation.md` — 本 chain
- `docs/impl/{ir,exp}/issue-707-midi-cancellation.*` — impl records
- `evidence/rf707/` — 検証記録

## Task List

- [x] T1: `mock.rs` — `PortState`/`ReadFault`/injector/read 経路の
      実装 + module doc 更新
- [x] T2: `mock.rs` unit test 追加
- [x] T3: `tests/midi_faults.rs` 新規 — pipeline 観測の正負 test
- [x] T4: 検証実行（fmt-check/check/lint/test × audio-core・
      audio-test、test-rt、prek）
- [x] T5: `evidence/rf707/` 検証記録 + AC inventory
- [x] T6: impl/ir + impl/exp 記録、review-protocol レビュー
- [x] T7: origin/main merge・commit/push/PR（`Refs #707`）

## Done When

- [x] `fail_next_read`/`inject_cancelled` が `MockPort` の test
      handle として実装され、`MockInput::read` が FIFO で消費する。
- [x] cancelled read が `Backend("cancelled")` を返し subscription
      を終了（後続 `StreamInvalidated`）する正負 test が
      `ruviform-audio-test` に存在し pass する。
- [x] `Backend("cancelled")` の非終端 read error・
      `inject_dropped`/`inject_unsupported` が終了しない負例が
      pass する。
- [x] `evidence/rf707/` に検証記録と AC 棚卸しがある。
- [x] 指定検証コマンド全て pass、docs index は未再生成のまま
      （Coordinator が統合時に再生成）。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-audio-core`
- [x] `mise run check ruviform-audio-test`
- [x] `mise run lint ruviform-audio-core`
- [x] `mise run lint ruviform-audio-test`
- [x] `mise run test ruviform-audio-core`
- [x] `mise run test ruviform-audio-test`
- [x] `mise run test-rt`（既存 `rt_` 非回帰確認）
- [x] `mise exec -- prek run --all-files`
