---
id: "PLAN-Fd93oTzaMRA6jFk8sVw4Ne"
type: "plan"
status: "approved"
title: "Issue #631 real-command-receipt case 実装計画"
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
    - "docs/specs/issue-631-command-receipt.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-command-receipt.md"
    - "docs/designs/issue-631-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-command-receipt case 実装計画 Implementation Plan

## Goal

`rf631.plugin-dock-real` catalog に `real-command-receipt` case を
追加し、実 Window の gesture→intent→`post_event` receipt/
projection→`CommandPhase` 遷移→decode 済み `CommandReceipt`
populate の AC1 link + AC4 evidence を実走で記録する。

## Scope

単一 slice（#631 command-receipt slice）。
`crates/ruviform-plugin-dock-test` 内に閉じる（fixture は
`ruviform-ui-test` の既存 `sv_fader_dock_fixture` を再利用・
変更なし）。

## Upstream Documents

- `docs/specs/issue-631-command-receipt.md`（approved）
- `docs/designs/issue-631-command-receipt.md`（approved）
- `docs/discovery/issue-631-command-receipt.md`

## Design Inputs

- `docs/designs/issue-631-command-receipt.md` — session 4-window
  構成・gesture/receipt 駆動 sequence・close 終端 assert
  一般化の設計正本。
- `crates/ruviform-ui-test/tests/scenario_contract.rs` —
  `post_event_routes_receipt_and_projection_to_binding` の
  先例パターン。
- `crates/ruviform-ui-test/src/lab_story.rs` —
  `sv_fader_dock_fixture` の spec（`fader-ch1` control）。

## File Map

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` —
  `real-command-receipt` row 追加 + doc コメント。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — window D
  mount・`RealDockContext` field・`command_receipt_body`・
  close 終端 assert 一般化・module doc 表・件数更新。
- `crates/ruviform-plugin-dock-test/tests/console.rs` —
  `--list` 30 行・real-dock assertion 22 行へ更新。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` — 実走で再生成。
- `evidence/rf631/plugin-dock-real-headless.jsonl` —
  blocked-probe で再生成。
- `evidence/rf631/verification.md` — slice 記録追記。
- `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl/ir,impl/exp}/issue-631-command-receipt.*` — doc chain。

## Implementation Sequence

- [x] T1: `real_dock.rs` session 構成 — window D mount
  （`ViewInstanceId` 634・`sv_fader_dock_fixture`・
  `WindowSpec` position (4350,80)）+ `RealDockContext`
  `window_d`/`view_d` 追加。
- [x] T2: `real_dock.rs` case 実装 — `command_receipt_body`
  （gesture→intent 採取→applied/rejected `post_event`→
  settle/observe→`from_result` decode→`receipts` populate）+
  `real_entries` entry + module doc 表更新。
- [x] T3: `real_catalog.rs` + `real_entries` — catalog row と
  entry の協調追加（`required` = `SemanticAction`）。
- [x] T4: `tests/console.rs` — 件数 assertion 29→30・real 21→22。
- [x] T5: `real-window-close` 終端 assert 一般化 — 「非対象
  window 全生存」へ（window D 追加の実走 fail を観測して修正）。
- [x] T6: 検証 lane 実行（fmt/check/lint/test 全 feature 変種 +
  console/scenario/headless/driver-boundary）。
- [x] T7: interactive 実走（`test-ui-plugin-dock-real`）で
  evidence JSONL 再生成 + `verification.md` 追記。
- [ ] T8: review-protocol → merge `origin/main` → commit/push/PR。

## Dependencies and Risks

- `SemanticGuiTest`/`MintedIntent`/`CommandReceipt::from_result`/
  `ScenarioOutcome.receipts`→`jsonl.rs` 配線は全て実装済み —
  新規 API・依存・`Cargo.lock` 変更は不要。
- `sv_fader_dock_fixture` は `ruviform-ui-test` の既存 fixture —
  `real-dock` feature が既に同 crate へ依存しているため feature
  追加も不要。
- gesture の実 OS timing で intent mint が遅延する可能性 —
  bounded `settle_until` + honest Fail で対応（実走で安定確認）。
- window D 追加は close case の終端 assert に影響 — T5 で
  survivor 一般化として対処済み。

## Task Breakdown

- T1-T4 は `real_dock.rs`/`real_catalog.rs`/`console.rs` の協調
  変更として 1 commit 単位が自然（catalog 整合 test が両者を
  検査するため分割すると中間状態で test が落ちる）。
- T5 は同じ `real_dock.rs` 内の修正 — T2 と同 commit でよい。
- T6-T8 は検証・統合。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check/lint/test ruviform-plugin-dock-test`
  （default/`real-dock`/`lab-gui`）
- [x] `mise run test-ui-plugin-dock-console`
- [x] `mise run test-ui-plugin-dock`（headless scenario lane）
- [x] `mise run test-ui-plugin-dock-real-headless`（exit 0・全行
  NotRun+environment_blocked・22 case 行）
- [x] `mise run test-ui-plugin-dock-real`（interactive —
  `real-command-receipt` pass + receipts 2 件、
  `real-window-close` pass、`real-dcomp-pixel` 既知 Fail のみで
  exit 1 は設計どおり）
- [x] `mise run test-ui-scenario-contract`・
  `check-ui-driver-boundary`
- [x] `mise exec -- prek run --all-files`
- [ ] `git fetch origin && git merge origin/main`（最終検証前）

## Open Questions / Gaps

- `test-ui-plugin-dock-lab`（interactive GPUI lab）は manual/
  opt-in lane で operator の Run all→Quit 操作が必要 — 本
  worktree では computer-use/UIA いずれも不可のため window 起動
  →clean exit(0) の確認まで。catalog 投影は
  `test-ui-plugin-dock-console --list --include-real`（30 行）と
  `catalog_rows_cover_headless_and_real` unit test で検証済み。
  実走 JSONL 追記は operator 実行時の残件として PR に記録。

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

## Review Handoff

- spec `docs/specs/issue-631-command-receipt.md`、design
  `docs/designs/issue-631-command-receipt.md`、discovery
  `docs/discovery/issue-631-command-receipt.md` を参照。
- reviewer focus: `command_id` 相関が minted intent 値であること・
  receipts が実 event decode のみであること・close 終端 assert
  の一般化が invariant を弱めないこと・fail-closed 形状の維持。
