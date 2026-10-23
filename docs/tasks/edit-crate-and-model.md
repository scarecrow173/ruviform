---
id: "TASK-034KPgPqF3NHzPtZPvvCHY"
type: "task"
status: "done"
title: "edit クレート骨格 + データモデル型・mutation"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-edit/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-edit/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-edit/src/error.rs"
      - type: "file"
        path: "crates/ruviform-edit/src/model.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
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

- [x] 型が design の形状で実装される（格納モデル private+accessor・
  `ClipSpec` は pub フィールド plain data・単調 ID・採番再利用なし）
- [x] 全 mutation（`add_track`/`remove_track`/`insert_clip`/`remove_clip`/
  `move_clip`/`trim_clip`/`enabled`）が完全検証ルールセットと `range` の
  base 正規化を実施
- [x] test-spec の mutation・検証拒否・accessor ケースをパス

## Verification

- [x] `mise run check`/`lint`/`test` パス

## Completion Notes

- `crates/ruviform-edit` 新設 — workspace `members` 登録・`publish.workspace = true` 継承・依存は `ruviform-media-core` path のみ
- `error.rs`: `#[non_exhaustive]` `Copy` `Error`（`TrackNotFound`/`ClipNotFound`/`InvalidRange`/`KindMismatch`/`InvalidValue`/`TimeOverflow`）+ `From<media_core::Error>` 正規化
- `model.rs`: ID newtype・`TrackKind`/`ClipAttributes`（enum レベル `#[non_exhaustive]`）・`Clip`/`ClipSpec`/`Track`/`Timeline` + 全 mutation。`normalize_range`（`NearestEven` で start/duration を timeline base へ）と `validate_clip`（完全検証ルールセット）を mutation 境界に集約
- 格納モデルは全フィールド private + accessor、`ClipSpec` は pub フィールド plain data。ID は単調採番・再利用なし
- mutation・検証拒否・accessor の test-spec ケースを `model.rs`/`error.rs` のテストモジュールでパス
