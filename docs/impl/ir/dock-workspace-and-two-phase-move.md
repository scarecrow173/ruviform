---
id: "IMPL-034QqISOcHeC4rS84Z3V0d"
type: "impl"
status: "completed"
title: "Dock workspace and two-phase move"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-182-02-workspace-move.md"
  implemented-by: []
  depends-on:
    - "dock-model-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-182-dock-tree.md"
    - "../../designs/issue-182-dock-tree.md"
    - "../../specs/issue-182-dock-tree.md"
    - "../../adr/issue-182-dock-tree-placement-move-contract-and-persistence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-182-dock-tree.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Dock workspace and two-phase move

## Summary

Issue #182 Task 2 の workspace+move 層を `dock.rs` に実装:
`DockWorkspace`(全 window の dock 状態の唯一正本 — `BTreeMap<
WindowId, Option<DockTree>>` + detached pool + pending move +
event queue)、`DropZone`/`DockTarget`/`WindowCloseDisposition`/
`MoveToken`/`MoveReport`/`DockEvent`。

## Context

docs/tasks/issue-182-02-workspace-move.md — ADR の move 契約
(destination 準備→owner 切替、失敗は元 dock へ rollback、
transfer 中 dual-edit 禁止)を model 層の上に実装。

## Implementation

### Changes

- `DockWorkspace` API: `register_window`/`remove_window(disposition)`/
  `attach_tree`/`attach_view`/`detach_view`/`remove_view`/
  `add_detached`/`owner_of`/`all_views`/`tree`/`set_ratio`/`move_tab`/
  `activate_view`/`begin_move`/`commit_move`/`abort_move`/`move_view`/
  `is_transferring`/`drain_events`。
- 2 相 move: `begin_move` が view を in-transfer 化(以後の
  edit は `ViewBusy` 拒否)。`commit_move` は target 再検証 →
  source からの detach + normalize → target へ insert を atomic
  に行い `MoveCommitted` を emit。`abort_move` は何も変更せず
  lock 解除のみ。
- `remove_window` は consumer 選択の disposition
  (`DetachViews`/`RelocateTo`/`ReleaseViews`)を要求 — view を
  暗黙破棄しない。window 内の view に pending があれば先に abort
  し、宛先として参照される pending は温存(commit 時に
  `TargetUnavailable` で rollback)。
- `DockEvent`: `TreeChanged`/`MovePrepared`/`MoveCommitted`/
  `MoveAborted`/`WindowRemoved` を `drain_events` で排出。
- operation vocabulary: `set_ratio`(splitter commit)・`move_tab`
  (reorder, active 追跡)・`activate_view`・Zone attach
  (Left/Right/Top/Bottom→2-child Split 生成、Tab→leaf 追加)。

### Decisions During Implementation

- `attach_view` は busy→owner→target の順に検証してから
  ownership を触る — insert 前失敗で detached view が
  消失しないよう insert-boundary rollback を徹底(テストで
  `UnknownWindow` 到達を確認し当初の欠陥を修正)。
- `attach_view` の重複検査は docked のみを拒否し、detached
  pool 内の view は re-attach 対象として許容(初版は
  `assert_free` を共用して detached を誤って拒否していた —
  テストが捕捉)。
- `commit_move` の target 消失は `check_target` 再検証で捕捉し、
  `UnknownWindow` → `TargetUnavailable` に写像して pending を
  畳む(自動 rollback + `MoveAborted`)。
- `move_view` = `begin`+`commit` の 1-shot — menu-move fallback
  と in-window drop の経路で drag 機構不要。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

`mise exec -- cargo test -p ruviform-ui dock` — unit 19 件 green
(T2 完了時点)。workspace テストが 3 件の実装欠陥を捕捉
(上記 Decisions)。最終は `mise run test` 全体 green。

## Risks

pending move の lingering は consumer が token を必ず commit/
abort で解消する前提 — `remove_window` が view-owner pending
を強制 abort するので dangling は限定的。

## Follow-ups

Task 3 が persistence/geometry をこの workspace の上に乗せる。
