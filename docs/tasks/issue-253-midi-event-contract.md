---
id: "TASK-034XCTdx2H01Fga6nmdMzZ"
type: "task"
status: "done"
title: "Issue #253 MIDI event contract fixes — V9/V11/V12 実装"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-253-midi-event-contract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-253-midi-event-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-midi-event-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 MIDI event contract fixes — V9/V11/V12 実装

## Work

Issue #188 実装記録の non-blocking follow-up V9/V11/V12 を
実装する:

- V9: `merge_call_events`（`ruviform-plugin-core`）を defined
  truncation 化 — over-budget 時に stable merge 順の先頭
  `min(scratch.len(), src_scratch.len())` 件を配送し、
  `truncated` 件数を `MergeOutcome` で返す。`MergeError::
  Overflow` は到達不能のため除去。
- V11: `InputStage` の `NoteLedger`/`release_all`/
  `active_notes`/`released`/`ledger_overflows` を削除し、
  note state 権威を node ledger に一本化。
- V12: `publish_anchor` は pending を保全し、写像は常に
  現行 anchor のみで行う契約を確定。anchor coverage 起因の
  rejection を `anchor_rejected` counter で識別可能にする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-253-midi-event-contract.md`（approved）
- `docs/designs/issue-253-midi-event-contract.md`（approved）
- `docs/specs/issue-253-midi-event-contract.md`（approved）
- `docs/test-specs/issue-253-midi-event-contract.md`（approved）

## Dependencies

- `docs/impl/ir/issue-188-midi-node-wiring-and-chase.md` の
  V9/V11/V12 follow-up（本 task が解消する対象）

## Affected Files

- `crates/ruviform-plugin-core/src/event.rs`
- `crates/ruviform-audio-plugin-bridge/src/input.rs`
- `crates/ruviform-audio-plugin-bridge/src/node.rs`
- `crates/ruviform-audio-plugin-bridge/src/status.rs`
- `crates/ruviform-audio-plugin-bridge/tests/`（midi_delivery.rs
  他）
- `docs/impl/ir/issue-253-midi-event-contract.md`（新規）
- `docs/impl/exp/issue-253-midi-event-contract.md`（新規）
- `evidence/rf253/`（検証証跡）

## Done When

- [x] `merge_call_events` が defined truncation を返し、
  `MergeOutcome { events, truncated }` で truncate 量を報告
- [x] `NodeStatusSnapshot::event_merge_truncated` が truncate
  量を観測可能
- [x] `InputStage` の ledger API/counter が削除され、note
  state 権威が node ledger に一本化
- [x] `publish_anchor` が pending を保全し、`anchor_rejected`
  counter が anchor coverage 起因 rejection を識別
- [x] test-spec の Rule V9/V11/V12 全てに対応する test が
  存在し Pass
- [x] 検証 lane（fmt-check/check/lint/test/test-rt/
  test-plugin-core/lint-plugin-core/test-l1-real-plugin）が
  Pass
- [x] impl/ir・impl/exp・evidence が記録され、docs index が
  再生成されている

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-plugin-core` /
  `mise run check ruviform-audio-plugin-bridge`
- [x] `mise run lint ruviform-plugin-core` /
  `mise run lint ruviform-audio-plugin-bridge`
- [x] `mise run test ruviform-plugin-core` /
  `mise run test ruviform-audio-plugin-bridge`
- [x] `mise run test-plugin-core` / `mise run lint-plugin-core`
- [x] `mise run test-rt`
- [x] `mise run test-l1-real-plugin`
- [x] `docs/test-specs/issue-253-midi-event-contract.md` の
  Rule V9/V11/V12 対応 test が Pass
