---
id: "TASK-034Yf0GO1eY84kAmbq6BBv"
type: "task"
status: "in-progress"
title: "Issue #532 dcomp-pixel slice — real-dcomp-pixel case + A' 実機観測"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-pixel.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-532-dcomp-pixel.md"
  implemented-by:
    - "docs/impl/ir/issue-532-dcomp-pixel.md"
  depends-on:
    - "docs/plans/issue-532-dcomp-pixel.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-dcomp-pixel.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  verified-by:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-pixel slice — real-dcomp-pixel case + A' 実機観測

## Work

`crates/ruviform-plugin-dock-test` に `real-dcomp-pixel` case を追加
（docked `test_editor` child HWND の screen `BitBlt` 画素を主 oracle、
`PrintWindow(child)` を補助診断、`EDITOR_COLOR` 緑判定）、
`GPUI_DISABLE_DIRECT_COMPOSITION` の env 記録と baseline/env-on
実機実走の証拠化、docs/evidence チェーンの作成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-532-dcomp-pixel.md`
- `docs/specs/issue-532-dcomp-pixel.md`
- `docs/designs/issue-532-dcomp-pixel.md`
- `docs/test-specs/issue-532-dcomp-pixel.md`

## Dependencies

- merge 済み前提: `test_editor` fixture（`editor_view.rs` の実
  `IPlugView`・`EDITOR_COLOR`）、real-window lane、`oschild`
  query-only 観測、`capture_frame` 経路、`REAL_CATALOG`。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/ospix.rs`（新規）
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs`
- `crates/ruviform-plugin-dock-test/src/lib.rs`
- `crates/ruviform-plugin-dock-test/Cargo.toml`
- `crates/ruviform-plugin-dock-test/README.md`
- `evidence/rf631/`（別名 JSONL・PNG artifact・verification.md）
- `docs/{specs,designs,plans,tasks,test-specs,discovery,impl/ir,impl/exp}/issue-532-dcomp-pixel.md`

## Done When

- [ ] `real-dcomp-pixel` が `rf631.plugin-dock-real` に追加され
  catalog↔entries 一致 test が green。
- [ ] baseline/env-on の実機実走が記録され、env var 有無・
  interactive session・観測手法・repo SHA・OS/GPU/DPI・
  limitation が evidence に明記される。
- [ ] exit-127 観測の有無と H1-H5 分類が記録される。
- [ ] headless lane・contract・boundary guard が非回帰。
- [ ] impl/ir・impl/exp（+ companion jsonl）が作成される。

## Verification

- [ ] `docs/plans/issue-532-dcomp-pixel.md` の Verification 節の
  全コマンド。
