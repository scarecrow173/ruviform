---
id: "TASK-034QkFzCiPKInh3jrWFr9b"
type: "task"
status: "done"
title: "bridge: TargetId binding + dispatch ledger + cap matrix param fields"
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
    - "docs/plans/issue-186-param-control-events.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-186-param-control-events.md"
    - "docs/tasks/issue-186-01-plugin-core-param-write.md"
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
    - "docs/test-specs/issue-186-param-control-events.md"
  references: []
  defers: []
  deferred-by: []
---

# bridge: TargetId binding + dispatch ledger + cap matrix param fields

## Work

`ruviform-audio-plugin-bridge` に `target` モジュール（`TargetId`/`BindingGeneration`/`ParamBinding`/`TargetResolution::{Resolved,Unresolved,Defunct}`/`ParamTargetTable`）と `dispatch` モジュール（`DispatchStage`/`DropReason`/`ParamValueObservation`/`ParamDispatchLedger`）を追加する。`cap.rs` の `AdapterCapability` に `param_event_precision`/`per_note_param`/`param_reporting` を追加し L0/L1/CLAP/VST3 の現行値を正直に宣言する。`lib.rs` で公開。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-186-param-control-events.md`
- `docs/designs/issue-186-param-control-events-detailed.md`

## Dependencies

- task-01（`ParamWriteRejection` 等の型を消費）

## Affected Files

- `crates/ruviform-audio-plugin-bridge/src/target.rs`（新設）
- `crates/ruviform-audio-plugin-bridge/src/dispatch.rs`（新設）
- `crates/ruviform-audio-plugin-bridge/src/cap.rs`（行列拡張）
- `crates/ruviform-audio-plugin-bridge/src/lib.rs`（公開）

## Done When

- [ ] binding resolve/unresolved/defunct/generation 増分が正しく動く
- [ ] ledger が Scheduled→Dispatched（投影）→Dropped/Reported/Unavailable の stage を照会できる
- [ ] cap 行列が4 backend 全てで param 精度・per-note・reporting を正直に宣言する

## Verification

- [ ] `mise run check` / `lint` / `test` green
