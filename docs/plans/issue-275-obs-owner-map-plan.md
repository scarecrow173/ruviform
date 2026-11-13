---
id: "PLAN-034WZJtam1qsAaVqT64TYw"
type: "plan"
status: "approved"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-275-obs-owner-map.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/designs/issue-275-obs-owner-map-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/plans/issue-275-obs-owner-map-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-275-obs-owner-map-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/impl/exp/issue-275-obs-owner-map.md"
      - type: "file"
        path: "evidence/rf275/obs-owner-map-audit.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-275-obs-owner-map.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-275-obs-owner-map.md"
    - "docs/designs/issue-275-obs-owner-map-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-275-observability-integration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-275-obs-owner-map-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-275-obs-owner-map.md"
  references:
    - "evidence/rf275/obs-owner-map-audit.md"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  defers: []
  deferred-by: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け計画 Implementation Plan

## Goal

統合設計正本 `docs/designs/issue-275-observability-integration-
design.md` に OBS-01〜11 の Owner/API/Feature/CI Case 対応表と
Deferred 範囲の Owner/Trigger 表を追記し、`issue-275-obs-owner-
map` 文書チェーンと監査証拠を完成させる。

## Scope

単一の文書スライス。コード・CI・manifest の変更を伴わないため
分割しない。全作業は 1 つの PR（worker branch
`docs/275-obs-owner-map`）に収める。

## Upstream Documents

- `docs/specs/issue-275-obs-owner-map.md`（approved）
- `docs/specs/issue-275-observability-integration-design.md`
  （親 spec）

## Design Inputs

- `docs/designs/issue-275-obs-owner-map-design.md`（approved）
- `docs/designs/issue-275-observability-integration-design.md`
  （更新対象の正本）

## File Map

- `docs/designs/issue-275-observability-integration-design.md` -
  対応表・Deferred 表の追記（「Workerへの共通契約」節は不変）
- `evidence/rf275/obs-owner-map-audit.md` - Issue 状態・crate/
  feature・CI lane の実測 snapshot（作成済み）
- `docs/specs|designs|test-specs|plans|tasks/issue-275-obs-
  owner-map*.md` - 本チェーン（作成済み）
- `docs/impl/ir/issue-275-obs-owner-map.md`・
  `docs/impl/exp/issue-275-obs-owner-map.md` - 実施記録
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/{ir,exp}/README.md` - 生成 index（doc-maintenance
  /sync_impl_index で再生成）

## Implementation Sequence

- [x] Issue 状態（`gh issue view`）・crate/feature/CI lane の
      実測棚卸し → `evidence/rf275/obs-owner-map-audit.md`
- [x] spec → design → test-spec の文書チェーン作成
- [x] 統合設計正本への対応表・Deferred 表・AL/SV 引き渡し節の
      追記
- [x] plan → task → impl(ir/exp) の文書チェーン完成
- [x] index 再生成（doc-maintenance `index apply` で
      specs/designs/plans/tasks/test-specs を再生成、
      `docs/impl/ir/README.md` は `sync_impl_index.js
      --kind ir --apply` で再生成。`docs/impl/exp` の
      `.md` exp 記録は管理 index（`*.jsonl` のみ対象）の
      対象外のため変更なし）
- [x] `mise run fmt-check` + `prek run --files` による軽量検証
- [x] `doc-status` audit と review-protocol レビュー
- [ ] `git fetch origin main && git merge origin/main` → index
      再生成 → commit → push → PR

## Dependencies and Risks

- **依存**: 親 spec/design（issue-275-observability-integration-
  design）の approved 状態 — 既に approved。
- **リスク**: Issue 状態の変動 → snapshot 日付明記で緩和。
  lane/feature 構成の将来変更 → gate 段階で再照合。
- **リスク**: 対応表の記述が per-issue 正本と drift → 代表型
  のみ列挙し詳細は正本参照とする設計で緩和。

## Task Breakdown

- T1: 監査証拠の固定（完了）
- T2: 文書チェーン作成（spec/design/test-spec/plan/task）
- T3: 統合設計正本への対応表・Deferred 節追記
- T4: impl ir/exp 記録と index 再生成
- T5: 検証（fmt-check・prek・doc-status audit・review-protocol）
- T6: merge main・commit・push・PR

## Verification

- [ ] `mise run fmt-check` が成功する
- [ ] `mise exec -- prek run --files <変更ファイル>` が成功する
- [ ] `doc_maintenance.js index plan|apply` で触った dir の index
      が in-sync になる
- [ ] `audit_docs.js`/`audit_impl_record.js`/`audit_experiment_log.js`
      で新規文書の finding がない（既存の範囲外 finding は分離報告）
- [ ] 対応表の全行が `evidence/rf275/obs-owner-map-audit.md` の
      実測と一致する
- [ ] review-protocol で blocker が残っていない

## Open Questions / Gaps

- なし。Issue 状態・crate 構成・CI lane は 2026-11-14 時点で
  実測済み。残る不確実性は将来の状態変動のみで、snapshot
  日付で扱う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
      consistent.

## Implementation Handoff

- Recommended mode: inline execution（文書のみ・単一スライス）
- Independent work streams: none
- User approval required before dispatch: no（review-protocol
  レビューを承認の代替とする — worker 共通契約）

## Review Handoff

- reviewer は対応表の各行が実在する Issue/module/feature/lane に
  基づくこと、`NotRun`→`Pass` の置換がないこと、「Workerへの
  共通契約」節に差分がないことを重点確認する。
