---
id: "TASK-034WZ7YMSR5UKu8AhtpOgk"
type: "task"
status: "todo"
title: "Issue #193 Migration Plan Update Snapshot 文書化"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-193-migration-plan-update.md"
      - type: "doc"
        path: "docs/designs/issue-193-migration-plan-update-design.md"
      - type: "doc"
        path: "docs/tasks/issue-193-migration-plan-update-task.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-migration-plan-update.md"
      - type: "doc"
        path: "docs/impl/ir/issue-193-migration-plan-update.md"
      - type: "doc"
        path: "docs/impl/exp/issue-193-migration-plan-update.md"
      - type: "doc"
        path: "evidence/rf193/issue-state-census-2026-11-14.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/designs/issue-193-migration-plan-update-design.md"
  implemented-by:
    - "docs/impl/ir/issue-193-migration-plan-update.md"
  depends-on:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/designs/issue-193-migration-plan-update-design.md"
    - "docs/test-specs/issue-193-migration-plan-update.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-193-migration-execution-plan.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/designs/issue-193-migration-plan-update-design.md"
  derived-by:
    - "docs/impl/ir/issue-193-migration-plan-update.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-migration-plan-update.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "evidence/rf193/issue-state-census-2026-11-14.md"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Plan Update Snapshot 文書化

## Work

Issue #193 の残件「各 Milestone M0〜M7 の着手条件・
着手可能 Headless Slice・完了定義・Integration Gate・
責務・Blocked by・独立 Owner Issue の確定と、現行
GPUI/IO/OBS/Script/Creative Branch 新要件との DAG 矛盾を
解消した版付き Snapshot」を、計画正本
`docs/designs|plans/issue-193-migration-execution-plan.md`
へ追記し、本更新スライスの文書チェーン
（spec/design/task/test-spec/ir/exp）と Issue 状態
census 証拠を作成する。文書のみの変更であり、コード・
テスト・CI・依存設定・Issue 本文・GitHub 依存関係は
変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-193-migration-plan-update.md`
- `docs/designs/issue-193-migration-plan-update-design.md`
- `docs/specs/issue-193-migration-execution-plan.md`
- `docs/designs/issue-193-migration-execution-plan.md`
- `docs/plans/issue-193-migration-execution-plan.md`
- `docs/test-specs/issue-193-migration-execution-plan.md`
- `docs/tasks/issue-193-migration-execution-plan.md`

## Dependencies

- 計画正本（PR #526 merge 済み）と umbrella 設計文書
  （approved）が前提。本タスクは追記型の状態記録であり、
  M0〜M7/G0〜G7/RB0〜RB6 の定義変更を伴わない。
- Issue 状態の根拠は 2026-11-14 の `gh` 実測
  （`evidence/rf193/issue-state-census-2026-11-14.md`）。

## Affected Files

- `docs/designs/issue-193-migration-execution-plan.md`（Snapshot 節追記・updated 更新）
- `docs/plans/issue-193-migration-execution-plan.md`（Snapshot 節追記・updated 更新）
- `docs/specs/issue-193-migration-plan-update.md`（新規）
- `docs/designs/issue-193-migration-plan-update-design.md`（新規）
- `docs/tasks/issue-193-migration-plan-update-task.md`（新規）
- `docs/test-specs/issue-193-migration-plan-update.md`（新規）
- `docs/impl/ir/issue-193-migration-plan-update.md`（新規）
- `docs/impl/exp/issue-193-migration-plan-update.md`（新規）
- `evidence/rf193/issue-state-census-2026-11-14.md`（新規）
- `docs/specs/README.md`、`docs/designs/README.md`、
  `docs/tasks/README.md`、`docs/test-specs/README.md`
  （doc-maintenance による再生成）
- `docs/impl/ir/README.md`、`docs/impl/exp/README.md`
  （手動追記 — doc-maintenance の index 生成対象外のため）

## Done When

- [x] 計画正本 2 文書に `Snapshot-2026-11-14` 節が追記
  され、M0〜M7 全 Milestone に開始条件・着手可能
  Headless Slice・完了定義・Integration Gate・責務・
  Blocked by・独立 Owner Issue が Issue URL 付きで
  揃っている。
- [x] Issue 状態照合の検証記録（Open/Closed/Deferred
  分類と Blocked by 誤記なしの確認）が Snapshot と
  `evidence/rf193/` に存在する。
- [x] AL/SV 別 Oracle・候補 SHA・既存 CI・OS 別実走証拠
  の M0〜M7 受入への接続が記録されている。
- [x] L2/L3 の製品利用条件が Deferred/Activation とし
  て記録され、計画 Snapshot 完了と将来実機受入が
  分離されている。
- [x] 文書チェーン（spec/design/task/test-spec/ir/exp）
  が作成され、docs index が整合している。
- [x] 差分が編集許可範囲内の文書・証拠・生成 index のみ
  であることを `git status`/`git diff` で確認する
  （PR 前に実施）。

## Verification

- [x] `gh issue list/view` による Issue 状態 census
  （2026-11-14 実測、補助証拠）。
- [ ] `mise run fmt-check`（文書のみ変更のため必須軽量
  チェック）。
- [ ] `mise exec -- prek run --files <変更ファイル>`。
- [ ] `doc-maintenance` の index plan/apply（specs/
  designs/tasks/test-specs）と rescan。
- [ ] `docs/impl/ir`・`docs/impl/exp` の README 手動追記。
- [ ] review-protocol によるレビュー（blocker なし）。
- [ ] `git status`/`git diff` で編集範囲の確認。
