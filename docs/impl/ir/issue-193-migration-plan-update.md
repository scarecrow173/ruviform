---
id: "IMPL-034WZ8jEVUJPCfjc4NtvqF"
type: "impl"
status: "in-progress"
title: "Issue 193 migration plan update snapshot"
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
    generated: []
  implements:
    - "docs/tasks/issue-193-migration-plan-update-task.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/designs/issue-193-migration-plan-update-design.md"
    - "docs/test-specs/issue-193-migration-plan-update.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-193-migration-plan-update-task.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf193/issue-state-census-2026-11-14.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-193-migration-plan-update.md"
    rejected: []
---

# Issue 193 migration plan update snapshot

## Summary

Issue #193 残件の版付き Snapshot（`Snapshot-2026-11-14`）を
計画正本 `docs/designs|plans/
issue-193-migration-execution-plan.md` へ追記し、
M0〜M7 の開始条件・着手可能 Headless Slice・完了定義・
Integration Gate・責務・Blocked by・独立 Owner Issue を
Issue URL 付きで確定した。Issue 状態は 2026-11-14 の
`gh` 実測に基づき、`evidence/rf193/` に census 証拠を
記録した。文書のみの変更で、コード・CI・Issue 本文・
GitHub 依存関係は変更していない。

## Context

`docs/tasks/issue-193-migration-plan-update-task.md` に
従う。計画正本は PR #526 で統合済みであり、本タスクは
追記型の状態記録であって計画の再定義ではない。
POLICIES.md の OS-L1/GUI-L2/EXT-L3 と
Required/Deferred/NotApplicable・Pass/Fail/NotRun/
Unsupported の分離、および 2026-11-13 のゲート正規化
（着手必須依存と統合・受入依存の分離）を Snapshot へ
接続した。

## Implementation

### Changes

- `docs/designs/issue-193-migration-execution-plan.md`:
  `## Snapshot-2026-11-14` 節を `Deferred Design
  Concerns` の前に追記。`updated` を 2026-11-14 へ更新。
- `docs/plans/issue-193-migration-execution-plan.md`:
  同名節を `## Implementation Handoff` の前に追記。
  `updated` を 2026-11-14 へ更新。
- 新規文書チェーン:
  `docs/specs|designs|tasks|test-specs/
  issue-193-migration-plan-update*.md`、本 IR、
  `docs/impl/exp/issue-193-migration-plan-update.md`。
- 証拠: `evidence/rf193/
  issue-state-census-2026-11-14.md`（3 repo の Issue
  状態 census・Blocked by 照合結果）。

### Decisions During Implementation

- Snapshot の版識別子は基準日 `Snapshot-2026-11-14` と
  し、Issue 状態の `gh` 実測日を版とした（Issue 本文の
  版番号とは別軸）。
- 計画正本 2 文書へ同一内容の Snapshot 節を追記し、
  設計正本側を詳細正本・実行正本側を同内容の実行記録
  として相互参照させた（内容の一致を保証条件とした）。
- 全 Open Issue の `着手条件（Blocked by）` が「なし」
  であることを census で確認し、実 OS GUI/Device/
  Plugin Gate は Integration Gate/独立 Owner Issue 側に
  のみ記述した（Blocked by への転記なし）。
- Script（RF #553/#558〜#560、SV #259/#264/#266/
  #271〜#274）・Creative Branch（SV #277〜#296、
  RF #562）・Future capability（RF #315〜#318、
  SV #119/#124/#130/#138）は並行 Lane として M0〜M7 の
  Blocked by にしない構成とした。
- `docs/impl/ir`・`docs/impl/exp` の README index は
  doc-maintenance の index 生成対象外のため手動追記と
  した（タスク表のユーザー指定どおり）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-193-migration-plan-update.md` —
  Issue 状態 census の収集方法・Blocked by 照合・
  Snapshot 記述規則の決定記録。

### Rejected

- なし。

## Validation

- `gh issue list -R scarecrow173/{ruviform,synvelis,
  audio-lens} --state all` による Issue 状態 census
  （2026-11-14、補助証拠）。
- `gh issue view` による Blocked by 節の照合（計画関連
  Open Issue 全件、`着手条件（Blocked by）: なし` を
  確認）。
- `mise run fmt-check`・`mise exec -- prek run --files`
  ・doc-maintenance index・review-protocol・
  `git status`/`git diff` による編集範囲確認
  （PR 前に実施、結果は PR 本文に記録）。

## Risks

- Snapshot は 2026-11-14 時点の状態記録であり、Issue
  状態の変化で陳腐化する。版識別子と census 証拠で
  基準日を固定し、Issue #193 本文更新時の同期義務を
  維持する。
- GitHub 正式 blockedBy と Issue 本文の齟齬は本タスク
  の範囲外（Coordinator 責務）。今回の照合では本文側
  の一律 Blocked by 記述は検出されなかったが、
  GitHub 側依存の確認は未実施。

## Follow-ups

- Issue #193 本文の受入項目チェックと、実装・実 OS・
  Consumer 受入は各 Owner Issue の責務として継続。
- Snapshot の次版更新は Issue 状態変化の検知時に実施。
