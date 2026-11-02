---
id: "TASK-341XTt270miIp3zJjTK07P"
type: "task"
status: "todo"
title: "ruviform-ui: DockWorkspace + two-phase view move + close policy + ops + DockEvent"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
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
    - "docs/plans/issue-182-dock-tree.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-182-dock-tree.md"
    - "docs/tasks/issue-182-01-dock-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-182-dock-tree.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-ui: DockWorkspace + two-phase view move

## Work

`dock.rs` に workspace 層を追加する（TDD — ユニットテスト先行）。

- 型: `DockWorkspace`（`windows: BTreeMap<WindowId,Option<DockTree>>`・`detached: BTreeSet<ViewInstanceId>`・`pending`・`next_move`・`events`）、`DockTarget{Window(w),Zone{window,anchor,zone}}`、`DropZone{Left,Right,Top,Bottom,Tab}`、`MoveToken`（private フィールド）、`MoveReport{view,from,to}`、`WindowCloseDisposition{DetachViews,RelocateTo{target},ReleaseViews}`、`DockEvent`（`#[non_exhaustive]`）。
- Window 管理: `register_window`（冪等）・`remove_window(w,policy)->Vec<ViewInstanceId>`（pending の自動 abort → policy 適用 → `WindowRemoved`）・`windows`/`tree`/`is_registered`。
- View 管理: `attach_tree`・`attach_view(view,target)`・`detach_view`・`remove_view`・`owner_of`・`is_detached`・`detached_views`・`all_views` — 所有一意性を全挿入経路で拒否。
- op: `set_ratio`・`move_tab`・`activate_view`。
- move: `begin_move`（検証+pending 登録、構造不変）・`commit_move`（target 再検証 → 消失なら `MoveAborted`+`Err(TargetUnavailable)` で自動 rollback、成功は detach→attach→normalize → `MoveCommitted`）・`abort_move`・`move_view`（one-shot = menu-move 経路）・`is_transferring`・`drain_events`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-182-dock-tree.md`
- `docs/designs/issue-182-dock-tree.md`（Workspace・Move 契約節）
- `docs/test-specs/issue-182-dock-tree.md`（Feature: DockWorkspace / Two-phase move）

## Dependencies

- `docs/tasks/issue-182-01-dock-model.md`（モデル型を利用）。

## Affected Files

- `crates/ruviform-ui/src/dock.rs`
- `crates/ruviform-ui/src/lib.rs`（re-export 追加）

## Done When

- [ ] 上記 API が design どおり存在し、ユニットテストが green: 所有一意性拒否・Zone 4 方向+Tab・Window root attach・close 3 ポリシー返り値・`ViewBusy`・commit 時消失 rollback・abort 完全復元・pending 中 remove_window 自動 abort・one-shot `move_view`・event 排出列。
- [ ] `remove_window` は policy 必須引数（既定なし）・Job cancel/Project 語彙を持たない。

## Verification

- `mise run fmt-check`・`check`・`lint`・`test`。
