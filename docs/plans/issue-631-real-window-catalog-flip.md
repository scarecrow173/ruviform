---
id: "PLAN-eRJIJo4Ufvn0Zb1s92nk4I"
type: "plan"
status: "approved"
title: "Issue #631 real-window-close/dpi catalog flip 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/specs/issue-631-real-window-catalog-flip.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-window-catalog-flip.md"
    - "docs/designs/issue-631-real-window-catalog-flip.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-window-close/dpi catalog flip 実装計画 Implementation Plan

## Goal

`rf631.plugin-dock-real` の `real-window-close`/`real-window-dpi` を
`Unsupported` stub から `RealOsGuiTest` 観測 API による実判定へ
flip し、catalog 宣言・evidence schema・実走 evidence を整合させる。

## Scope

単一 slice（#631 catalog-flip slice + AC4 evidence stretch）。
`crates/ruviform-plugin-dock-test` 内に閉じる。

## Upstream Documents

- `docs/specs/issue-631-real-window-catalog-flip.md`（approved）
- `docs/designs/issue-631-real-window-catalog-flip.md`（approved）
- `docs/discovery/issue-631-real-window-catalog-flip.md`

## Design Inputs

- `docs/designs/issue-631-real-window-catalog-flip.md` — session
  3-window 構成・close 駆動 sequence・DPI 整合判定・AC4 field の
  設計正本。
- `crates/ruviform-ui-gpui/tests/real_window_lifecycle.rs` —
  veto/defer/default-approve/sentinel の実走済み先例パターン。

## File Map

- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — 2 case の
  実判定化・window C mount・`RealDockContext` field 追加・
  `finalize_case` wrapper・module doc 表更新。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` — close
  row の required を `MultiWindow` へ + doc コメント整合。
- `crates/ruviform-plugin-dock-test/src/jsonl.rs` — `receipts`/
  `case_seed` field 追加 + unit test。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` — 実走で再生成。
- `evidence/rf631/plugin-dock-real-headless.jsonl` — blocked-probe
  で再生成。
- `evidence/rf631/verification.md` — slice 記録追記。
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-real-window-catalog-flip.*` — doc chain。

## Implementation Sequence

- [ ] T1: `real_dock.rs` session 構成 — window C mount
  （`ViewInstanceId` 633・`ViewHost`・`WindowSpec` position
  (2900,80)）+ `RealDockContext.window_c`/`session_scale_factor`
  追加 + `run_all_with_pixel_artifacts` での scale factor 実測。
- [ ] T2: `real_dock.rs` case 実装 — `close_body`/`dpi_body` +
  lifecycle 集計 helper + `finalize_case` wrapper + 全 case の
  `finalize_real`→`finalize_case` 機械的置換 + module doc 表更新。
- [ ] T3: `real_catalog.rs` — close row required を `MultiWindow`
  へ + `real_entries` の close entry を協調更新 + doc コメント。
- [ ] T4: `jsonl.rs` — `receipts`/`case_seed` 追加 + unit test。
- [ ] T5: 検証 lane 実行（fmt/check/lint/test 全 feature 変種 +
  console/scenario-contract/driver-boundary + headless lane）。
- [ ] T6: interactive 実走（`test-ui-plugin-dock-real`）で
  evidence JSONL 再生成 + `verification.md` 追記。
- [ ] T7: review-protocol → merge `origin/main` → commit/push/PR。

## Dependencies and Risks

- `RealOsGuiTest` の観測 API は実装済み（window-surface slice）—
  新規の adapter 変更は不要。
- `MultiWindow` capability は `supported_capabilities()` が無条件
  claim — required 変更で preflight が弾くことはない。
- close case の window 破壊は case 12/21 の位置で安全（後続 case は
  window A のみ使用）— catalog 順を変えないことが前提。
- 実走での OS 挙動差（geometry 値・イベント timing）は honest
  Fail/NotRun で記録 — 判定を緩めて辻褄を合わせない。

## Task Breakdown

- T1+T2+T3 は `real_dock.rs`/`real_catalog.rs` の協調変更として
  1 commit 単位が自然（catalog 整合 test が両者を検査するため
  分割すると中間状態で test が落ちる）。
- T4 は独立 — `jsonl.rs` + test のみ。
- T5-T7 は検証・統合。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-dock-test`（default）
- [ ] `mise run check ruviform-plugin-dock-test --features real-dock`
- [ ] `mise run check ruviform-plugin-dock-test --features lab-gui`
- [ ] `mise run lint ruviform-plugin-dock-test`（default + real-dock）
- [ ] `mise run test ruviform-plugin-dock-test`（default + real-dock）
- [ ] `mise run test-ui-plugin-dock-console`
- [ ] `mise run test-ui-plugin-dock-real-headless`（exit 0・全行
  NotRun+environment_blocked）
- [ ] `mise run test-ui-plugin-dock-real`（interactive — flip 後
  verdict を実測記録）
- [ ] `mise run test-ui-scenario-contract`・`check-ui-driver-boundary`
- [ ] `git fetch origin && git merge origin/main`（最終検証前）

## Open Questions / Gaps

- なし — session 構成・case sequence・capability・evidence field
  は設計で確定済み。実走値は検証時に実測記録する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker — 単一
  slice、依存は直列）。
- Independent work streams: none。
- User approval required before dispatch: yes（worker contract に
  より review-protocol の blocker 無しを承認として扱う）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- spec `docs/specs/issue-631-real-window-catalog-flip.md`、design
  `docs/designs/issue-631-real-window-catalog-flip.md`、discovery
  `docs/discovery/issue-631-real-window-catalog-flip.md` を参照。
- reviewer focus: close case の sentinel/window 破壊順序と
  `REAL_CATALOG`/`real_entries` の協調更新、AC4 field が
  lockfile/manifest を変えないこと、fail-closed 形状の維持。
