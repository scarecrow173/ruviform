---
id: "IMPL-034QqISfp1gxWFA4cmeeA0"
type: "impl"
status: "completed"
title: "Dock model layer"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
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
    - "docs/tasks/issue-182-01-dock-model.md"
  implemented-by: []
  depends-on: []
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

# Dock model layer

## Summary

Issue #182 Task 1 の model 層を `crates/ruviform-ui/src/dock.rs` に実装:
`ViewInstanceId`(consumer 発行の論理 view 識別子)、`Axis`、
`DockNode`(Split/Tabs、`#[non_exhaustive]` + 内部タグ serde)、
`DockPath`、`ViewLocation`、`DockTree`(normalize→validate で
構築時不変条件を強制)、`DockError`。Window 毎に ≤1 tree、
tree 内の view 重複は validate が拒否する。

## Context

docs/tasks/issue-182-01-dock-model.md — spec/ADR/design/plan は
docs/specs・docs/designs・docs/adr・docs/plans の issue-182 系
ドキュメントに従う。

## Implementation

### Changes

- `ViewInstanceId(NonZeroU64)` — `new`/`get`/Display/serde
  transparent。`0` 拒否。
- `DockNode::tabs`/`DockNode::split` が不変条件を構築時に強制
  (empty Tabs / <2 children / 非有限・範囲外 ratio を拒否、
  `active` は clamp)。
- `DockNode::normalize`(私的)が post-edit の修復を担う:
  空 Tabs 除去、1-child Split collapse、active clamp。
  ratio は修復せず常に拒否。
- `DockNode::validate` が構造検査(有限 `(0,1)` ratio・非空 leaf・
  in-range active・≥2 children・tree 内 view 一意)。
- `DockTree::new` = normalize→validate。root が view を
  持たない場合 `EmptyTabs`。
- `locate`/`node`/`node_mut` が DockPath ベースの
  アドレッシングを提供。
- `DockError` — `#[non_exhaustive]`、Display+Error 実装。
- `lib.rs` へ `pub mod dock` と re-export を追加。

### Decisions During Implementation

- `ViewsIter` は stack+cursor の反復子 — Tabs の view 列を
  そのまま `slice::Iter` で流し、Split は children を逆順
  push で DFS 順序を維持。
- `DuplicateView{view, owner: Option<WindowId>}` —
  tree 内重複は `owner: None`、window 横断は `Some(w)`。
- `is_empty`/`is_root` を両方用意(clippy の
  len-without-is-empty 要求と意味的 API の両立)。

## Related Experiments

### Adopted

なし — design/plan で確定済みの直線的実装。

### Rejected

なし。

## Validation

`mise exec -- cargo test -p ruviform-ui dock` — unit 9 件 green
(T1 完了時点)。最終は `mise run test` workspace 全体 green。

## Risks

`DockPath` は構造変更で無効化されるため、edit 後は
`DockTree::locate` で取り直す契約(doc comment 記載済み)。

## Follow-ups

Task 2 の `DockWorkspace` が本 model を消費する。
