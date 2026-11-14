---
id: "PLAN-034XGryAYYlLjlEdp2DGUn"
type: "plan"
status: "approved"
title: "Issue #389 dock-test session-lifecycle 実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-session-lifecycle.md"
    - "docs/designs/issue-389-session-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 dock-test session-lifecycle 実装計画 Implementation Plan

## Goal

`ruviform-plugin-dock-test` の scenario に、実 `test_gain` VST3 バイナリ経路で `PluginEditorSessions` の session lifecycle・host 終了・open-or-degrade 契約を観測する case を追加し、`docs/designs/issue-389-plugin-editor-session.md` の stale な verification-status 記述を現行コードの事実に reconcile する。

## Scope

単一の実装ストリーム。`crates/ruviform-plugin-dock-test/`（test-only consumer composition root）と Issue #389 設計書 1 ファイル、および本 slice の doc chain・evidence に限定する。plugin-host/plugin-core/plugin-vst3/plugin-clap/ui-test のソースは読み取り参照のみで変更しない。

## Upstream Documents

- `docs/specs/issue-389-session-lifecycle.md`
- `docs/designs/issue-389-session-lifecycle.md`
- `docs/test-specs/issue-389-session-lifecycle.md`
- `docs/designs/issue-389-plugin-editor-session.md`（reconcile 対象）
- `docs/plans/issue-631-plugin-dock-scenario.md`（先行 slice）

## Design Inputs

- `docs/designs/issue-389-session-lifecycle.md`
- `crates/ruviform-plugin-host/src/editor_session.rs`（session API — 読み取りのみ）
- `crates/ruviform-plugin-dock-test/src/scenario.rs`（既存 harness — `attempt_editor_attach` の fake-slot redeemer seam）

## File Map

- `crates/ruviform-plugin-dock-test/src/scenario.rs` - `session-lifecycle`/`host-terminate`/`open-or-degrade` case、共有 helper（`SessionCaseState`・`editor_ops`/`rejecting_editor_ops`・`CountingEditorOps`・`teardown_editor_case`）追加、module doc case 表更新
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` - case 数 5→8、fail-closed NotRun 一覧、新 case の実 verdict 断言
- `crates/ruviform-plugin-dock-test/README.md` - verdict 表に新 case 行追加
- `docs/designs/issue-389-plugin-editor-session.md` - R5 matrix・D8 移行表・D5/R2/R4/R6 の `GuiParentToken`/`WIRE_VERSION` stale 記述を現行事実へ reconcile
- `evidence/rf389-dock/` - lane 実走 JSONL と verification report（新規）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-389-session-lifecycle.md` + `docs/impl/exp/issue-389-session-lifecycle-exp.jsonl` - 本 slice の doc chain
- `docs/*/README.md` index - doc-maintenance ツールで再生成

## Implementation Sequence

- [ ] T1: `scenario.rs` に session-lifecycle 系 3 case と共有 helper を実装（fake-slot redeemer seam 再利用・fail-closed NotRun 規律維持）
- [ ] T2: `tests/plugin_dock.rs`・`README.md` を新 case 数・verdict に追従
- [ ] T3: `mise run build ruviform-plugin-test-vst3` → `mise run test-ui-plugin-dock` 実走で全 case verdict 確認、`evidence/rf389-dock/` に証拠記録
- [ ] T4: `docs/designs/issue-389-plugin-editor-session.md` の verification-status reconcile
- [ ] T5: doc index 再生成・canonical 検証 lane・review-protocol・commit/push/PR

## Dependencies and Risks

- `test_gain` は `createView` null で editor surface を持たない — open は必ず `Ops(Unsupported)` に落ちる。`Failed{Unsupported}` + `editor_live=false` の live session を作るのが Terminated event を得る honest 経路（born-`Closed` session には event が出ない）。
- `editor-attach`=`unsupported`・`dock-pixels`=`not_run` を回帰させない — 本 slice は実 attach 成功経路を持たない。
- lane `test-ui-plugin-dock` は固定 `--out evidence/rf631/plugin-dock-scenario.jsonl` — `evidence/rf631/` は forbidden につき lane 生成物は commit せず `git restore` で戻し、証拠は `evidence/rf389-dock/` に別途記録する。

## Task Breakdown

- T1（code+test 更新）→ T2（docs verdict 表）→ T3（実走 evidence）→ T4（設計書 reconcile）→ T5（検証・提出）。T4 はコード非依存で並行可能だが同一コミット系列で提出する。

## Verification

- [ ] `mise run fmt-check` / `mise run check|lint|test ruviform-plugin-dock-test`
- [ ] `mise run build ruviform-plugin-test-vst3` → `mise run test-ui-plugin-dock`（全 8 case verdict 記録）
- [ ] `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver`（session 契約非回帰）
- [ ] `mise run check-ui-driver-boundary` / `mise exec -- prek run --all-files`
- [ ] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply`（触れた dir）+ `sync_impl_index.js --kind ir|exp --apply`
- [ ] review-protocol 1 cycle 以上・blocker なし

## Open Questions / Gaps

- 実 editor attach・Native Dock 画素・実 OS GUI L2/L3 は後続 slice（`test_editor` fixture・`scarecrow173/ruviform#630` 残存 scope）。本 slice では `Unsupported`/`NotRun` を維持する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol の blocker なしを承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: verdict の正直性（実走のみ Pass・推測 Pass 禁止）、既存 5 case 非回帰、`environment_kind` の `real-abi-test-plugin` 主張範囲、設計書 reconcile が現行コード事実と一致するか。
