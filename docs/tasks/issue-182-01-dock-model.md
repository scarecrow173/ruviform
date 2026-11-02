---
id: "TASK-J9cxFOsGKSoiMZKmpXRMOp"
type: "task"
status: "done"
title: "ruviform-ui: dock model layer (ViewInstanceId/Axis/DockNode/DockTree/DockPath/DockError)"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/dock.rs"
    modified:
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

# ruviform-ui: dock model layer

## Work

`ruviform-ui` に `dock` モジュールを追加し、モデル層の型を実装する（TDD — ユニットテスト先行）。

- `dock.rs`（モデル節）: `ViewInstanceId`（NonZeroU64・`new(u64)->Option<Self>`・`get()`・Display `v{n}`・Serialize/Deserialize transparent — consumer 採番）、`Axis{Horizontal,Vertical}`、`DockNode`（`#[non_exhaustive]`・`Split{axis,ratio,children}`/`Tabs{views,active}`・`tabs`/`split` コンストラクタで不変条件拒否・`views()`・`is_leaf`）、`DockTree`（`new`→normalize+validate・`root`/`views`/`locate`/`node`/`validate`/`normalize` — `normalize` は空 Tabs 除去・<2 子 Split 畳み込み・active clamp のボトムアップ再帰、`validate` は ratio 有限 (0,1)・葉 views≥1・split children≥2・tree 内重複 View なし）、`DockPath`（`Vec<usize>` ラッパー・root/child/push/pop）、`ViewLocation{leaf,tab}`、`DockError`（`#[non_exhaustive]`・Display+Error — `UiError` 流儀）。serde: `#[serde(tag="type",rename_all="lowercase")]`。全公開 API doc + RT 非該当明記。
- `lib.rs`: `pub mod dock;` + re-export（`ViewInstanceId`/`Axis`/`DockNode`/`DockTree`/`DockPath`/`ViewLocation`/`DockError`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-182-dock-tree.md`
- `docs/designs/issue-182-dock-tree.md`（Components and Boundaries — Identity/Model 節）
- `docs/adr/issue-182-dock-tree-placement-move-contract-and-persistence.md`
- `docs/test-specs/issue-182-dock-tree.md`（Feature: Dock Tree モデル）

## Dependencies

- なし（最初のタスク）。

## Affected Files

- `crates/ruviform-ui/src/dock.rs`（新規 — モデル節）
- `crates/ruviform-ui/src/lib.rs`

## Done When

- [ ] 上記型が design どおりのシグネチャで存在し、ユニットテストが green: 構築拒否（EmptyTabs/InvalidChildren/InvalidRatio）・normalize（空葉除去・単一子畳み込み・active clamp・ルート空→None 相当）・tree 内重複 View 拒否・locate/node・serde round-trip・`ViewInstanceId` の ID 規約。
- [ ] 全公開 enum に `#[non_exhaustive]`、doc 付き、unsafe/新規依存なし。

## Verification

- `mise run fmt-check`・`check`・`lint`・`test`（dock ユニットテスト green + 既存非回帰）。
