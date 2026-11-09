---
id: "TASK-034UCVp0kqovy1IEDp0LWy"
type: "task"
status: "done"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/specs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-migration-execution-plan.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-193-migration-execution-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/23"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本

## Work

Issue #193（Migration Execution Plan v1.0 — M0〜M7・依存
DAG・受入 Gate・互換移行、実行計画として承認済み）の合意
済み計画を `docs/` チェーンに正本化する docs のみの
スライス。spec → design → test-spec → plan → task を作成し、
umbrella 設計文書を設計権威・#193 本文を実行権威とする
Execution Plan canonical として、M0〜M7 実行順・依存
DAG・G0〜G7 受入 Gate・Cross-Consumer 受入追加条件・
前提判断 6 項目・RB0〜RB6 Rollback・Synvelis E2E-L/R/X
接続・GUI lane Start/Integration/Acceptance・WebView 撤去
lane（#465）・横断依存正規化 v1.1・Signal Flow 所有境界・
実装ステータス同期・進捗/Close 条件・traceability を記述
する。コード・テスト・CI・依存設定・Consumer repo・
umbrella 設計文書・#193 本文は変更しない。M0〜M7 の実装・
gate 受入・Consumer 実測は対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-193-migration-execution-plan.md](../plans/issue-193-migration-execution-plan.md)
- [docs/specs/issue-193-migration-execution-plan.md](../specs/issue-193-migration-execution-plan.md)
- [docs/designs/issue-193-migration-execution-plan.md](../designs/issue-193-migration-execution-plan.md)
- [docs/test-specs/issue-193-migration-execution-plan.md](../test-specs/issue-193-migration-execution-plan.md)
- Issue #193（受入正本: Migration Execution Plan v1.0・
  M0〜M7・G0〜G7・2026-11-09 GUI lane/WebView 撤去 lane
  補足）
- Issue #192（umbrella migration tracking。設計権威は
  umbrella 設計文書）

## Dependencies

- 参照正本（読取のみ）: umbrella 設計文書
  `docs/designs/synvelis-driven-target-architecture-and-
  migration.md`（approved）、`docs/discovery/
  issue-194-m0-baseline-audit.md`、`docs/specs/
  issue-219-consumer-compat-contract.md`（proposed）、
  `docs/specs/issue-384-driver-neutral-gui-architecture.md`、
  `docs/designs/issue-384-driver-neutral-gui-contract.md`、
  `docs/designs/issue-257-driver-neutral-ui-contract.md`、
  領域別 `docs/*/issue-18x〜19x/38x〜46x-*.md`、関連 ADR
  各件。
- 別 Worker が並行作業中の #192 設計チェーン
  （`issue-192-*` 文書、未 merge）は参照しない。

## Affected Files

- `docs/specs/issue-193-migration-execution-plan.md`（新規）
- `docs/designs/issue-193-migration-execution-plan.md`（新規）
- `docs/test-specs/issue-193-migration-execution-plan.md`（新規）
- `docs/plans/issue-193-migration-execution-plan.md`（新規）
- `docs/tasks/issue-193-migration-execution-plan.md`（新規・本書）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  （生成 index の再生成）

## Done When

- [x] spec / design / plan / task / test-spec の
  `issue-193-migration-execution-plan.md` 5 文書が
  `docs/` チェーンに存在する。
- [x] Issue #193 本文の受入条件 4 項目と GUI lane DoD が
  設計文書に対応行として存在する。
- [x] M0〜M7 表・依存の意味・G0〜G7・Cross-Consumer 受入
  表・前提判断 6 項目・RB0〜RB6・E2E-L/R/X・GUI lane・
  WebView 撤去 lane・横断正規化・Signal Flow 境界・
  実装ステータス同期・進捗/Close 条件・traceability が
  設計文書に記録されている。
- [x] 参照が merge 済み正本のみに限定され、未 merge の
  `issue-192-*` 文書を参照していない。
- [x] docs index が再生成され、audit_docs で新規文書の
  error が 0、`mise run fmt-check` が pass する。
- [x] `docs/` 以外に差分がない。

## Verification

- [x] `node .agents/skills/doc-maintenance/scripts/
  doc_maintenance.js apply` で docs index を再生成。
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all` で新規文書の error が 0（既存の範囲外
  finding は分離して記録）。
- [x] `mise run fmt-check` pass。
- [x] `git status`/`git diff` で変更が `docs/` の新規
  `issue-193-*` 文書と生成 index のみであることを確認。
- [x] review-protocol によるレビュー（参照実在性・権威
  関係・受入条件対応・状態分離）を実施し blocker なし。
