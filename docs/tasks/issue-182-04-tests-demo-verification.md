---
id: "TASK-AirSKtHS1TtcoQZnYr9s9U"
type: "task"
status: "done"
title: "ruviform-ui: dock integration tests + demo_dock example + docs/index consistency + verification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/examples/demo_dock.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
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
    - "docs/tasks/issue-182-03-persistence-position.md"
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

# ruviform-ui: dock integration tests + demo + verification

## Work

spec AC-4/AC-5/AC-7/AC-8 を headless で立証し、doc/索引/README を整合させる。

- `tests/dock.rs`: fake host（`WindowHandle` 実装 — multi_window.rs の FakeHost パターン再利用）で end-to-end シナリオ: detach（menu-move one-shot と begin+webview-ready+commit の両経路）→reattach で論理 View ID と `MoveCommitted` handoff（consumer 側 view state 実演）保持・create_window 失敗/Ready 未到達 abort・commit 時 dest Destroyed rollback・close 3 ポリシー・menu-move が drag 機構なしで動くこと・snapshot save/restore end-to-end。
- `examples/demo_dock.rs`: headless script デモ — 2 window で detach→reattach を print 実行。
- `crates/ruviform-ui/README.md`（`dock` モジュール節: 責務・不変条件・move 契約・persist・menu-move）、`crates/ruviform-ui-webview/README.md`（`WindowConfig.position`）、`crates/README.md`（必要な節のみ）、`AGENTS.md`（ワークスペース節への #182 反映が必要なら最小限に）。
- docs 索引: `doc_maintenance.js apply` で `index-missing-entry` を解消（本 Issue の新規 doc 群 + 既存欠落行の機械修復）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-182-dock-tree.md`
- `docs/designs/issue-182-dock-tree.md`
- `docs/test-specs/issue-182-dock-tree.md`（全 Feature）
- `docs/specs/issue-182-dock-tree.md`（AC-4..AC-8）

## Dependencies

- `docs/tasks/issue-182-01-dock-model.md`
- `docs/tasks/issue-182-02-workspace-move.md`
- `docs/tasks/issue-182-03-persistence-position.md`

## Affected Files

- `crates/ruviform-ui/tests/dock.rs`（新規）
- `crates/ruviform-ui/examples/demo_dock.rs`（新規）
- `crates/ruviform-ui/README.md`、`crates/ruviform-ui-webview/README.md`、`crates/README.md`、`AGENTS.md`
- `docs/*/README.md`（索引再生成）

## Done When

- [ ] `tests/dock.rs` が spec の必須シナリオ一式を検証し green。detach→reattach が論理 View ID/状態を保持（fake-host 実演）。
- [ ] `cargo run -p ruviform-ui --example demo_dock` が script デモを完走。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` + `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` green。
- [ ] README/索引整合・実機未検証範囲は impl record に capability 記録。

## Verification

- `mise run verify-ci` 相当（fmt-check→check-ci→lint-ci→test-ci または dev lane 一式）+ UI webview lane。
