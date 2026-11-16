---
id: "TASK-034YnFdRrLrRSoA5rYgwr3"
type: "task"
status: "done"
title: "Issue #532 dcomp-occlusion slice — query-only 遮蔽層診断 + 根因二分実走"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-occlusion.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-532-dcomp-occlusion.md"
  implemented-by:
    - "docs/impl/ir/issue-532-dcomp-occlusion.md"
  depends-on:
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/tasks/issue-532-dcomp-pixel.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-dcomp-occlusion.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  verified-by:
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-occlusion slice — query-only 遮蔽層診断 + 根因二分実走

## Work

`crates/ruviform-plugin-dock-test` に query-only 遮蔽層診断 module
`osoccl` を追加し、`real-dcomp-pixel` case の docked child ごとに
GPUI root HWND の `GWL_EXSTYLE`（`WS_EX_NOREDIRECTIONBITMAP`
含む）、root/parent の direct-child z-order 走査、`WindowFromPoint`
代表点 probe を `verdict: "note"` の step row として記録する。
baseline/env-on の interactive 実走で遮蔽が HWND 層か合成器層かを
証拠化し、docs/evidence チェーンを作成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-532-dcomp-occlusion.md`
- `docs/specs/issue-532-dcomp-occlusion.md`
- `docs/designs/issue-532-dcomp-occlusion.md`
- `docs/test-specs/issue-532-dcomp-occlusion.md`

## Dependencies

- merge 済み前提: `real-dcomp-pixel` case（`ospix` screen/PrintWindow/
  windowdc capture・`EDITOR_COLOR` 緑判定・`observe_editor_pixel_
  lifecycle`）、`oschild` query-only 観測、real-window lane、
  `REAL_CATALOG`。
- 前提 slice: `docs/tasks/issue-532-dcomp-pixel.md`（本 slice の
  診断はその `observe_one_child_pixels` に載る）。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/osoccl.rs`（新規）
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/lib.rs`
- `crates/ruviform-plugin-dock-test/README.md`（必要な場合）
- `evidence/rf631/verification.md`
- `evidence/rf631/plugin-dock-real-scenario.jsonl`（baseline 上書き）
- `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
  （env-on、canonical とは別名）
- `docs/{discovery,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-532-dcomp-occlusion.md`
  + `docs/impl/exp/issue-532-dcomp-occlusion.jsonl`

## Done When

- [x] `osoccl` module が `real-dock` feature で compile し、
  `probe(child)` が root/parent/z-order/hit-test を plain 値で
  返す（非 Windows stub 含む）。
- [x] `real-dcomp-pixel` が 3 系の `verdict: "note"` step row を
  child ごとに出力し、`PixelScan`/`PixelObservation`/case verdict
  に非干渉。
- [x] baseline/env-on の実機実走が記録され、exstyle 差分・
  z-order・`WindowFromPoint` 結果・二分解釈・limitation が
  evidence/docs に明記される。
- [x] exit-127 観測の有無と H1-H5 分類が記録される。
- [x] headless lane・contract・boundary guard が非回帰。
- [x] impl/ir・impl/exp（+ companion jsonl）が作成される。
- [x] Issue #532 は open のまま（close しない）。

## Verification

- [x] `docs/plans/issue-532-dcomp-occlusion.md` の Verification 節の
  全コマンド。
