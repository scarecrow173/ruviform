---
id: "PLAN-034YJjLMWS9oGq5V9aJPaf"
type: "plan"
status: "approved"
title: "Issue #707 — MIDI input cancellation fault implementation plan"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-707-midi-cancellation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-707-midi-cancellation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-707-midi-cancellation.md"
    - "docs/test-specs/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-707-midi-cancellation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-707-midi-cancellation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #707 — MIDI input cancellation fault implementation plan

## Goal

`MockPort`/`MockInput` fixture に read-level fault injector
（`fail_next_read`/`inject_cancelled`）を追加し、MIDI input path で
`Backend("cancelled")` + subscription 終了（後続
`StreamInvalidated`）を deterministic に再現する正負 test と
docs chain・evidence を揃える。

## Scope

Single plan — fixture 変更・unit/integration test・docs/evidence は
同一 slice の受入単位。

## Upstream Documents

- `docs/specs/issue-707-midi-cancellation.md`

## Design Inputs

- `docs/designs/issue-707-midi-cancellation.md`（approved）

## File Map

| File | Change |
| --- | --- |
| `crates/ruviform-audio-core/src/midi_input/mock.rs` | `PortState` に `terminated`/`read_faults` 追加、private `ReadFault` enum、`MockPort::{fail_next_read,inject_cancelled,is_terminated}`、`MockInput::read` の fault 消費経路、module doc 更新、unit test 追加 |
| `crates/ruviform-audio-test/tests/midi_faults.rs` | 新規 — `MidiLeg` 経由の正負 test（pre-armed/mid-stream inject/負例/ordered fault） |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-707-midi-cancellation.md` | 本 chain |
| `docs/impl/ir/issue-707-midi-cancellation.md`・`docs/impl/exp/issue-707-midi-cancellation.{md,jsonl}` | impl records |
| `evidence/rf707/` | 検証記録・AC inventory |

変更しないことを確認済みの file: `src/backend/midi.rs`、
`src/pipeline/midi.rs`（既存 `DeviceError` 伝播で完結）、
audio 側 `src/backend.rs`/`src/backend/stream.rs`/
`tests/backend_faults.rs`（#628 先例・read-only）、
`mise.toml`・`docs/development-environment.md`（enumerated lane
不要 — `cargo test -p` が新規 `tests/*.rs` を自動収集）、
`Cargo.toml`/`Cargo.lock`（依存追加なし）。

## Implementation Sequence

- [ ] T1: `mock.rs` — `PortState`/`ReadFault`/injector/read 経路の
      実装 + module doc 更新
- [ ] T2: `mock.rs` unit test 追加（fail_next_read 非終端・
      inject_cancelled 終了・FIFO 順序・dead subscription 不発火）
- [ ] T3: `tests/midi_faults.rs` 新規 — pipeline 観測の正負 test
- [ ] T4: 検証実行（fmt-check/check/lint/test × audio-core・
      audio-test、test-rt、prek）
- [ ] T5: `evidence/rf707/` 検証記録 + AC inventory
- [ ] T6: impl/ir + impl/exp 記録、review-protocol レビュー
- [ ] T7: origin/main merge・commit/push/PR（`Refs #707`）

## Dependencies and Risks

- `MockPort` は `#[derive(Clone)]` + `Arc<Mutex<PortState>>` —
  field 追加は内部表現のみで API 互換。既存 consumer
  （plugin-bridge `midi_wiring`、plugin-test e2e）への影響なし。
- `read` の fault 消費順序は `closed || terminated` 判定を
  fault pop より先に置く — dead subscription で armed fault が
  残らないことを unit test で pin。
- `fail_next_read` の error は drain 前に return — `out`・
  `pending`・`dropped`/`unsupported` counter を消費しない
  semantics を doc/test で固定。

## Task Breakdown

`docs/tasks/issue-707-midi-cancellation.md` を参照。

## Verification

| Check | Command |
| --- | --- |
| fmt | `mise run fmt-check` |
| check | `mise run check ruviform-audio-core`・`mise run check ruviform-audio-test` |
| lint | `mise run lint ruviform-audio-core`・`mise run lint ruviform-audio-test` |
| test | `mise run test ruviform-audio-core`・`mise run test ruviform-audio-test` |
| rt | `mise run test-rt`（既存 `rt_` 非回帰 — 新規 `rt_` test なし） |
| hooks | `mise exec -- prek run --all-files` |
| boundary | `git diff --name-only` が許可範囲内のみ |

## Open Questions / Gaps

- なし（写像・API・test 配置は design で確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 内で完結）。
- Independent work streams: none（T1–T7 は逐次）。
- User approval required before dispatch: no — `review-protocol`
  Skill のレビューで承認を代替（blocker が残っていない状態を承認
  とする）。

## Review Handoff

- `Backend("cancelled")` + 終了の写像が #628 audio 側と整合するか。
- `fail_next_read`/`inject_cancelled` の semantics（1-shot・
  非消費・FIFO・dead 不発火）が fixture 契約として妥当か。
- 負例（非終端 `Backend("cancelled")`）が termination との区別を
  十分に pin しているか。
