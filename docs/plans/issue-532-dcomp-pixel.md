---
id: "PLAN-034Yf0GO1eY84kAmbq6BBv"
type: "plan"
status: "approved"
title: "Issue #532 dcomp-pixel 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-dcomp-pixel.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/specs/issue-532-dcomp-pixel.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-dcomp-pixel.md"
  derived-by:
    - "docs/tasks/issue-532-dcomp-pixel.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-pixel 実装計画 Implementation Plan

## Goal

`rf631.plugin-dock-real` scenario に `real-dcomp-pixel` case を追加し、
docked `test_editor` child HWND の画面合成画素で `EDITOR_COLOR` 緑の
出現を正直判定する regression を固定し、`GPUI_DISABLE_DIRECT_COMPOSITION`
（方式 A'）の実機観測を証拠化する。

## Scope

単一 implementation stream。`crates/ruviform-plugin-dock-test` と
docs/evidence に閉じる（consumer・adapter・`mise.toml` は不変）。

## Upstream Documents

- `docs/specs/issue-532-dcomp-pixel.md`

## Design Inputs

- `docs/designs/issue-532-dcomp-pixel.md`

## File Map

- `crates/ruviform-plugin-dock-test/src/ospix.rs` — Win32 画素採取
  module（screen `BitBlt`・`PrintWindow`・`GetWindowRect`・
  `GetAncestor`/`IsIconic`/`IsWindowVisible`、query/capture のみ、
  非 Windows stub）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `real-dcomp-pixel` case、`observe_editor_pixel_lifecycle`、
  `conclude_editor_session` 抽出、`attempt_real_attach` の
  `observe_open` 注入化、`dcomp_env_extra`/`finalize_real`、
  `run_all_with_pixel_artifacts`、case 表・module doc 更新。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` —
  `REAL_CATALOG` 5 行目 + doc 更新。
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs`
  — `--pixel-artifacts <dir>` flag。
- `crates/ruviform-plugin-dock-test/src/lib.rs` — `ospix` module 宣言。
- `crates/ruviform-plugin-dock-test/Cargo.toml` — `windows-sys`
  features に `Win32_Graphics_Gdi`。
- `crates/ruviform-plugin-dock-test/README.md` — lane/case 記述更新。
- `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`・
  `evidence/rf631/dcomp-pixel/`（PNG artifact）・
  `evidence/rf631/verification.md` — 実走証拠。

## Implementation Sequence

- [ ] `ospix.rs` 実装（`#[cfg(windows)]` + stub、DIB 転写・
  green 計数 helper）。
- [ ] `real_dock.rs`: `conclude_editor_session` 抽出 +
  `attempt_real_attach` 一般化（attach case の verdict 不変を確認）。
- [ ] `real_dcomp-pixel` case + `REAL_CATALOG`/`real_entries`/test
  更新（4→5）。
- [ ] `dcomp_env_extra` を real lane 全 report の `environment` に
  merge。`--pixel-artifacts` flag + PNG 保存。
- [ ] doc/README 整合更新。
- [ ] 検証 lane 実行（check/lint/test default+real-dock、headless
  lane、boundary guard、prek）。
- [ ] 実機 lane baseline/env-on 実走 → evidence 記録。
- [ ] impl/ir・impl/exp + companion jsonl 作成。

## Dependencies and Risks

- `windows-sys` `Win32_Graphics_Gdi` feature 追加は既存 optional
  edge 内 — `Cargo.lock` 差分が出たら再生成のみ。
- `PrintWindow` は `Win32_Storage_Xps` 限定のため local extern
  宣言（`gpui_driver/capture.rs` 先例）。
- env-on run の exit-127 は観測結果 — 失敗を取り繕わない。

## Task Breakdown

- `docs/tasks/issue-532-dcomp-pixel.md`（単一 task）。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`
  （default feature）
- [ ] `mise run check|lint|test ruviform-plugin-dock-test
  --features real-dock`
- [ ] `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console`
  / `test-ui-scenario-contract` / `check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock-real`（baseline）と
  `GPUI_DISABLE_DIRECT_COMPOSITION=1 mise run test-ui-plugin-dock-real`
  （env-on、env 前置で canonical task 呼出）— exit code・stderr・
  JSONL を記録。headless 化していたら `NotRun`+`environment_blocked`
  を正直に記録。
- [ ] `mise exec -- prek run --all-files`

## Open Questions / Gaps

- exit-127 が env-on run で再現するかは実機依存 — 観測結果を
  H1-H5 に正直分類（未確定は未確定のまま）。
