---
id: "TASK-034KPgPqF3NHzPtZPvvCHY"
type: "task"
status: "todo"
title: "edit クレート骨格 + データモデル型・mutation"
created: "2026-10-23"
updated: "2026-10-23"
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
    - "docs/plans/implement-issue-96-edit-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-96-edit-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# edit クレート骨格 + データモデル型・mutation

## Work

`crates/ruviform-edit` 新設 + データモデル型と mutation の実装（plan 手順
1–2・design の型表と完全検証ルールセット）。評価射影は後続 task
（edit-evaluation-and-closeout）で実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-96-edit-model.md
- docs/designs/edit-data-model.md
- docs/specs/edit-data-model.md

## Dependencies

- `ruviform-media-core`（`MediaTime`/`MediaDuration`/`MediaRange`/`TimeBase`/
  `Rounding` — `convert`/`checked_end`/`checked_add`・cross-base 比較）

## Affected Files

- Cargo.toml（workspace `members` 追加）
- crates/ruviform-edit/Cargo.toml（workspace 継承・media-core path 依存のみ）
- crates/ruviform-edit/src/lib.rs（crate doc: 責務・許可依存層・RT 契約 —
  全 API 非 RT + re-export）
- crates/ruviform-edit/src/error.rs（`#[non_exhaustive]` `Copy` `Error`/`Result`）
- crates/ruviform-edit/src/model.rs（`Timeline`/`Track`/`Clip`/`ClipSpec`/
  `TrackKind`/`ClipAttributes`/`TrackId`/`ClipId`/`ClipSource` + mutation）

## Done When

- [ ] 型が design の形状で実装される（格納モデル private+accessor・
  `ClipSpec` は pub フィールド plain data・単調 ID・採番再利用なし）
- [ ] 全 mutation（`add_track`/`remove_track`/`insert_clip`/`remove_clip`/
  `move_clip`/`trim_clip`/`enabled`）が完全検証ルールセットと `range` の
  base 正規化を実施
- [ ] test-spec の mutation・検証拒否・accessor ケースをパス

## Verification

- [ ] `mise run check`/`lint`/`test` パス
