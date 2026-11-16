---
id: "PLAN-034YnCJRmR1GQYwKErRZOn"
type: "plan"
status: "approved"
title: "Issue #532 dcomp-occlusion 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-dcomp-occlusion.md"
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
    - "docs/specs/issue-532-dcomp-occlusion.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/plans/issue-532-dcomp-pixel.md"
    - "evidence/rf631/verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-dcomp-occlusion.md"
  derived-by:
    - "docs/tasks/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  verifies: []
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

# Issue #532 dcomp-occlusion 実装計画 Implementation Plan

## Goal

`real-dcomp-pixel` case に query-only の遮蔽層診断（root exstyle・
root/parent の sibling z-order・`WindowFromPoint` 代表点 probe）を
note 級 step row として追加し、baseline/env-on の interactive
実走で遮蔽が HWND 層か合成器層かを証拠化する。

## Scope

単一 implementation stream。`crates/ruviform-plugin-dock-test` と
docs/evidence に閉じる（consumer・adapter・`mise.toml`・GPUI
patch は不変）。

## Upstream Documents

- `docs/specs/issue-532-dcomp-occlusion.md`

## Design Inputs

- `docs/designs/issue-532-dcomp-occlusion.md`

## File Map

- `crates/ruviform-plugin-dock-test/src/osoccl.rs` — 遮蔽層診断
  module（`GetAncestor(GA_ROOT)`・`GetWindowLongPtrW(GWL_EXSTYLE)`・
  `GetWindow(GW_CHILD/GW_HWNDNEXT)` z-order 走査・`GetParent`・
  `GetClassNameW`・`GetWindowRect`・`IsWindow`/`IsWindowVisible`・
  `WindowFromPoint`、query-only、`#[cfg(windows)]` + stub）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `observe_one_child_pixels` への `record_occlusion_notes` 配線、
  role/`above_*` 分類、3 系の note step row。
- `crates/ruviform-plugin-dock-test/src/lib.rs` — `osoccl`
  module 宣言。
- `crates/ruviform-plugin-dock-test/Cargo.toml` — 変更なし
  （`Win32_Foundation` の `POINT` は既存 feature 内。差分が出た
  場合のみ再生成）。
- `crates/ruviform-plugin-dock-test/README.md` — case 診断記述の
  更新が必要な場合のみ。
- `evidence/rf631/verification.md` — 実走結果・二分解釈の追記。
- `evidence/rf631/plugin-dock-real-scenario.jsonl`（baseline）・
  `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
  （env-on、canonical JSONL は上書きしない）— 診断 step row を含む
  実走証拠。
- `docs/impl/ir/issue-532-dcomp-occlusion.md`・
  `docs/impl/exp/issue-532-dcomp-occlusion-exp.jsonl` — 実装記録・
  実験ログ。

## Implementation Sequence

- [ ] `osoccl.rs` 実装（`#[cfg(windows)]` + `#[cfg(not(windows))]`
  stub、`oschild`/`ospix` の query-only 規約踏襲）。
- [ ] `real_dock.rs`: `observe_one_child_pixels` から
  `record_occlusion_notes` 呼出 + 3 系 note row（`verdict` 不変、
  `PixelScan`/`PixelObservation`/case verdict に非干渉）。
- [ ] `lib.rs` module 宣言・README 整合。
- [ ] 検証 lane 実行（fmt-check、check/lint/test default+
  real-dock、headless lane、boundary guard、prek）。
- [ ] 実機 lane baseline/env-on 実走 → evidence 記録（exit code・
  stderr・JSONL・H1-H5 分類）。
- [ ] impl/ir・impl/exp + companion jsonl 作成、doc chain 関係
  整合、doc index 再生成。

## Dependencies and Risks

- `Cargo.lock` 差分が出た場合は `cargo update -p` 系で必要依存
  のみ再生成（今回は feature 追加なしの見込み）。
- `WindowFromPoint` の GPU 合成下での決定性は未検証 — limitation
  として記録し、観測値と解釈を分離する。
- env-on run の exit-127 は観測結果 — 失敗を取り繕わない。
- headless 化していたら real lane は全 case `NotRun`+
  `environment_blocked`（exit 0）— 実走部が実機 gate になる
  ことを明記。

## Task Breakdown

- `docs/tasks/issue-532-dcomp-occlusion.md`（単一 task）。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`
  （default feature）
- [ ] `mise run check|lint|test ruviform-plugin-dock-test
  --features real-dock`
- [ ] `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console`
  / `test-ui-scenario-contract` / `check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock-real`（baseline）と
  `GPUI_DISABLE_DIRECT_COMPOSITION=1` 前置の実走（env-on、
  `--out` で別 JSONL に出力）— exit code・stderr・JSONL を記録。
  headless 化していたら `NotRun`+`environment_blocked` を正直に
  記録。
- [ ] `mise exec -- prek run --all-files`
- [ ] doc index 再生成（`doc-maintenance` skill の管理スクリプト）。

## Open Questions / Gaps

- `WindowFromPoint` が DComp/flip-model 合成下で child を返すか
  は実機依存 — 観測結果をそのまま記録し、二分不能なら
  「unresolved」と正直に記述する。
- env-on で `WS_EX_NOREDIRECTIONBITMAP` が外れるかは GPUI の
  env var 受理値に依存（`"1"`/`"true"`）— 観測で確認。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 stream・診断 module
  1 本 + 配線）。
- Independent work streams: none。
- User approval required before dispatch: yes。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- 前提: `osoccl` は `ospix::window_screen_rect` を再利用し
  Win32 型を module 内に閉じる。role 分類は `real_dock.rs` 側。
- reviewer focus: note row が verdict/`PixelScan` に非干渉で
  あること、query-only 境界（変異系 API 不使用）、env-on run
  が canonical JSONL を上書きしないこと。
