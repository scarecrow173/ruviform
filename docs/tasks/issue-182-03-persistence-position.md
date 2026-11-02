---
id: "TASK-yUcTer6Jz05I9eBkeM4mQX"
type: "task"
status: "todo"
title: "ruviform-ui: layout persistence schema + geometry correction + WindowConfig.position plumbing"
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
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/session.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
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
    - "docs/tasks/issue-182-02-workspace-move.md"
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

# ruviform-ui: layout persistence + WindowConfig.position

## Work

`dock.rs` に永続化層を追加し、復元幾何の適用経路を adapter へ通す（TDD — ユニットテスト先行）。

- `dock.rs` persist 節: `LAYOUT_SCHEMA_VERSION=1`、`LayoutSnapshot{schema,windows,detached}`（`capture(ws,&[(WindowId,WindowGeometry)])`・`to_json`/`from_json`・`restore(monitors)->Result<RestoredLayout>`・`restore_or_default`）、`WindowSnapshot{x,y,width,height,monitor:Option<MonitorHint>,tree:Option<DockNode>}`、`MonitorHint{name,width,height,scale_factor}`（signature のみ・display ID なし）、`WindowGeometry`、`RestoredLayout{placements,detached}`、`WindowPlacement{x,y,width,height,monitor:Option<usize>,tree}`、`DockWorkspace::apply_restored_tree`。
- restore 検証: schema 一致・各 tree の normalize+validate・全 window+detached 横断の View 重複拒否・w/h>0。幾何補正: rect center が現モニター内 → 採用（`monitor=Some(i)`）、外 → signature 一致 → 先頭モニターへ中央配置（サイズクランプ）、monitors 空 → rect 保持。
- `window.rs`: `WindowConfig.position: Option<(i32,i32)>`（物理 px・Default None）。`session.rs::create_entry` と `host.rs::run` で `with_position(PhysicalPosition::new(x,y))` を `Some` 時のみ適用。`SessionConfig::single` に `position: None` 反映。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-182-dock-tree.md`
- `docs/designs/issue-182-dock-tree.md`（Persistence・WindowConfig.position・Data and Control Flow 節）
- `docs/test-specs/issue-182-dock-tree.md`（Feature: 永続化と復元）

## Dependencies

- `docs/tasks/issue-182-01-dock-model.md`
- `docs/tasks/issue-182-02-workspace-move.md`

## Affected Files

- `crates/ruviform-ui/src/dock.rs`
- `crates/ruviform-ui/src/window.rs`
- `crates/ruviform-ui/src/lib.rs`
- `crates/ruviform-ui-webview/src/session.rs`
- `crates/ruviform-ui-webview/src/host.rs`

## Done When

- [ ] snapshot round-trip・schema 不一致 `UnsupportedSchema`・壊れた JSON/tree 無効/重複 View/w=0 の `InvalidSnapshot`・`restore_or_default` 空フォールバック・画面外 center 補正・signature 照合・monitors 空経路がユニットテスト green。
- [ ] `position` が `run`/`run_multi` 生成経路へ条件付き適用される（`webview` feature の check/lint green）。

## Verification

- `mise run fmt-check`・`check`・`lint`・`test` + `check-ui-webview`・`lint-ui-webview`・`test-ui-webview`。
