---
id: "PLAN-7dchOzXC5tOlj8UNXYoJZV"
type: "plan"
status: "approved"
title: "Issue #192 Virtual-first 受入 Owner remap 計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/designs/issue-192-virtual-first-owner-remap-design.md"
      - type: "file"
        path: "docs/test-specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
      - type: "file"
        path: "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
      - type: "file"
        path: "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "file"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  implemented-by:
    - "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
  depends-on:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
    - "docs/plans/issue-275-obs-owner-map-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
  derived-by:
    - "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
---

# Issue #192 Virtual-first 受入 Owner remap 実装計画 Implementation Plan

## Goal

tracking canonical
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
末尾と対応表
`docs/discovery/issue-192-ta-invariant-map.md` の
「Headless Gap → Open Gate 接続一覧」節へ、TA DAG L2/L3
受入 Owner 欄の Virtual-first remap を dated addendum
として追記し、`issue-192-virtual-first-owner-remap`
文書チェーンと検証記録を完成させる。

## Scope

単一の文書スライス。コード・CI・manifest の変更を伴わ
ないため分割しない。全作業は 1 つの PR（worker branch
`scarecrow173/rf192-virtualfirst`）に収める。

## Upstream Documents

- `docs/specs/issue-192-virtual-first-owner-remap.md`
  （approved）
- `docs/specs/issue-192-synvelis-driven-target-architecture-migration.md`
  （親 spec）
- `docs/specs/issue-192-ta-invariant-map.md`（先行 slice
  の spec）

## Design Inputs

- `docs/designs/issue-192-virtual-first-owner-remap-design.md`
  （approved）
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  （追記対象の tracking canonical）
- `docs/discovery/issue-192-ta-invariant-map.md`
  （追記対象の対応表）
- `docs/designs/issue-275-observability-integration-design.md`
  §7（表現スタイルの先例）

## File Map

- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  - 末尾への dated addendum 追記（`updated` 更新・既存
    本文不変）
- `docs/discovery/issue-192-ta-invariant-map.md`
  - 「Headless Gap → Open Gate 接続一覧」節への dated
    addendum 追記（`updated` 更新・既存表・指定外行不変）
- `docs/specs|designs|test-specs|plans|tasks/issue-192-
  virtual-first-owner-remap*.md` - 本チェーン
- `docs/impl/ir/issue-192-virtual-first-owner-remap.md`・
  `docs/impl/exp/issue-192-virtual-first-owner-remap.md`
  - 実施記録と検証記録
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/ir/README.md` - 生成 index（doc-maintenance
  `index apply` で再生成）

## Implementation Sequence

- [x] 参照 Issue の `gh issue view` 実在照合（ruviform
      #627〜#631・#294・#275・#251/#253/#388/#389・
      #243/#313、AL #127、SV #28 — 2026-11-15）
- [x] spec → design → test-spec の文書チェーン作成
- [x] tracking canonical 末尾への dated addendum 追記
- [x] 対応表「Headless Gap → Open Gate 接続一覧」節への
      dated addendum 追記
- [x] plan → task → impl(ir/exp) の文書チェーン完成
- [x] index 再生成（doc-maintenance `index plan` で
      in-sync 確認後 `index apply`。`docs/impl/ir` は
      impl-doc safe sync API 経由。`docs/impl/exp` の
      `.md` exp 記録は管理 index 対象外のため変更なし）
- [x] `mise run fmt-check` + `mise exec -- prek run
      --all-files` + doc-status 監査 + type 別 lint +
      impl 監査による軽量検証
- [x] review-protocol レビュー（docs-only 小 slice —
      proportionate self-review）
- [x] `git fetch origin main && git merge origin/main` →
      index 再生成 → commit → push → PR

## Dependencies and Risks

- **依存**: 親 spec/design（issue-192-synvelis-driven-
  target-architecture-migration）の approved 状態 —
  既に approved。対応表（issue-192-ta-invariant-map）は
  active で追記可能。
- **リスク**: Issue 状態の変動 → 照合日（2026-11-15）を
  明記し snapshot として扱う。
- **リスク**: 既存 snapshot 表の誤改変 → append-only を
  設計制約とし diff で確認する。
- **リスク**: Owner/実装 Owner の混同 → 列分離と原則
  明記で緩和。

## Task Breakdown

- T1: 参照 Issue 実在照合（完了）
- T2: 文書チェーン作成（spec/design/test-spec/plan/task）
- T3: tracking canonical・対応表への dated addendum 追記
- T4: impl ir/exp 記録と index 再生成
- T5: 検証（fmt-check・prek・index・doc-status・lint・
      impl 監査・review-protocol）
- T6: merge main・commit・push・PR

## Verification

- [ ] `mise run fmt-check` が成功する
- [ ] `mise exec -- prek run --all-files` が成功する
- [ ] `doc_maintenance.js index plan` で触った全 dir が
      in-sync、`index apply` で再生成される
- [ ] `doc_status.js audit --type all --profile artifact`
      と新規 slice の type 別 lint、`audit_impl_record.js`・
      `audit_experiment_log.js` で新規文書の finding が
      ない（既存の範囲外 finding は分離報告）
- [ ] addendum の全参照が `gh issue view` 実測と一致する
- [ ] review-protocol で blocker が残っていない

## Open Questions / Gaps

- なし。Issue 状態・追記位置・表語彙は 2026-11-15 時点で
  確定済み。残る不確実性は将来の状態変動のみで、
  snapshot 日付で扱う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
      are consistent.

## Implementation Handoff

- Recommended mode: inline execution（文書のみ・単一
  スライス）
- Independent work streams: none
- User approval required before dispatch: no
  （review-protocol レビューを承認の代替とする — worker
  共通契約）

## Review Handoff

- reviewer は両 addendum が append-only で既存 snapshot
  表に差分がないこと、remap 表の各行が実在する
  Issue/状態に基づくこと、`NotRun`→`Pass` 置換や
  Required→Deferred 降格がないこと、変更範囲が
  `docs/` と生成 index のみであることを重点確認する。
