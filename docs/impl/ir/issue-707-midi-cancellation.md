---
id: "IMPL-034YJnaTZDWSZddB4rqGfu"
type: "impl"
status: "completed"
title: "Issue #707 MIDI input cancellation fault slice"
created: "2026-11-16"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/707"
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/tasks/issue-707-midi-cancellation.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-test/tests/midi_faults.rs"
      - type: "file"
        path: "docs/impl/ir/issue-707-midi-cancellation.md"
      - type: "file"
        path: "docs/impl/exp/issue-707-midi-cancellation.jsonl"
      - type: "file"
        path: "evidence/rf707/verification.md"
      - type: "file"
        path: "evidence/rf707/ac-inventory.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input/mock.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-707-midi-cancellation.md"
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
    - "docs/plans/issue-707-midi-cancellation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-707-midi-cancellation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-707-midi-cancellation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-707-midi-cancellation.jsonl"
    rejected: []
---

# Issue #707 MIDI input cancellation fault slice

## Summary

`MockPort`/`MockInput`（`crates/ruviform-audio-core/src/midi_input/mock.rs`）
に read-level fault injector を追加し、audio 側 `StreamFault::Cancellation`
（#628・merge `0b510cf8`）の MIDI 写像を deterministic OS-L1 で再現した。
`fail_next_read`（非終端 1-shot error）と `inject_cancelled`（終端
`Backend("cancelled")` → 後続 `StreamInvalidated`）を FIFO キューで
`read` が消費する。`tests/midi_faults.rs` で `MidiLeg` 経由の正負 6
test を追加。production trait・`DeviceError`・pipeline 配線は不変。

## Context

`docs/specs/issue-707-midi-cancellation.md`・
`docs/designs/issue-707-midi-cancellation.md`・
`docs/plans/issue-707-midi-cancellation.md`・
`docs/tasks/issue-707-midi-cancellation.md`・
`docs/test-specs/issue-707-midi-cancellation.md` に従う。
rf628 `ac-inventory.md` Deferred 表の「MIDI 側 fault への
cancellation 拡張 → `scarecrow173/ruviform#707`」行の回収 slice。

## Implementation

### Changes

- `PortState` に `terminated: bool`・`read_faults:
  VecDeque<ReadFault>` を追加。`ReadFault` は private enum
  （`Fail(DeviceError)`/`Cancel`）。
- `MockPort::fail_next_read(error)`・`inject_cancelled()`・
  `is_terminated()` を追加（`fail_next_open`/`inject_dropped`/
  `inject_unsupported` と同じ fixture handle 面）。
- `MockInput::read`: `closed || terminated` を先に検査 →
  `read_faults.pop_front()` → `Fail` は `Err(error)`、
  `Cancel` は `terminated=true` + `Err(Backend("cancelled"))`。
  いずれも drain 前に return するため `pending`/counters/`out`
  を消費しない。dead subscription は armed fault を消費しない。
- `mock.rs` unit test 4 件、`tests/midi_faults.rs` 6 件追加。

### Decisions During Implementation

- cancelled read は `close` と区別: `is_closed()==false`・
  `is_terminated()==true`。consumer が `close` していなくても
  backend 側 abort で subscription が死ぬ observable を分離。
- `Backend("cancelled")` の error 値と cancel semantics を分離:
  `fail_next_read(Backend("cancelled"))` は非終端（負例 test で固定）。
- docs index（`docs/**/README.md`）は Worker 範囲外 — 未再生成の
  まま。Coordinator が統合時に再生成。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-707-midi-cancellation.jsonl` — (a) MockPort
  injector 案の採用根拠と観測分解（Err vs stats）の記録。

### Rejected

- (b) `MidiLeg`/`MidiFeed` leg-level fault script — subscription
  状態の複製と observable 乖離リスクにより不採用（exp log seq 1）。

## Validation

`evidence/rf707/verification.md` に全記録。canonical（mise）:
fmt-check・check/lint/test × ruviform-audio-core・
ruviform-audio-test・test-rt・prek 全て exit 0。
`git merge origin/main` は Already up to date。

## Risks

- `MidiInput::read` が `Err` と stats を同時に返せない契約上、
  audio 側「error callback + data callback」併存観測は失敗 read
  と stats 付き成功 read に分解される — Issue close 判定で
  分解の妥当性が問われる可能性（test-spec `Risk` に記録済み）。
- 実 MIDI device/endpoint の fault 実 timing は EXT-L3 で NotRun
  （`scarecrow173/ruviform#629`/`#630` 領域）。

## Follow-ups

- Coordinator: docs index 再生成・merge 後 SHA の CI 統合検証。
- `scarecrow173/ruviform#629`/`#630`: 実 endpoint/device 上の
  cancellation 実 timing（EXT-L3）。
