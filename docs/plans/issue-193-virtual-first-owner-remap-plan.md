---
id: "PLAN-034XJsqELLLLcAakLIA6YG"
type: "plan"
status: "approved"
title: "Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/designs/issue-193-virtual-first-owner-remap-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/impl/exp/issue-193-virtual-first-owner-remap.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
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
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
    - "docs/designs/issue-193-virtual-first-owner-remap-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-193-virtual-first-owner-remap-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
---

# Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け計画 Implementation Plan

## Goal

両 canonical（`docs/designs/issue-193-migration-
execution-plan.md`・`docs/plans/issue-193-migration-
execution-plan.md`）に M0〜M7 の Virtual-first 受入
owner 対応付けを記録した同一内容の dated Snapshot 節
`## Snapshot-2026-11-15 — Virtual-first owner 対応付け
（版付き）` を追記し、`issue-193-virtual-first-owner-
remap` 文書チェーンを完成させる。

## Scope

単一の文書スライス。コード・CI・manifest の変更を伴わ
ないため分割しない。全作業は 1 つの PR（worker
branch `scarecrow173/rf193-virtualfirst`）に収める。

## Upstream Documents

- `docs/specs/issue-193-virtual-first-owner-remap.md`
  （approved）
- `docs/specs/issue-193-migration-execution-plan.md`
  （親 spec）

## Design Inputs

- `docs/designs/issue-193-virtual-first-owner-remap-
  design.md`（approved）
- `docs/designs/issue-193-migration-execution-plan.md`・
  `docs/plans/issue-193-migration-execution-plan.md`
  （更新対象の正本 — `Snapshot-2026-11-14` 節の
  「同名 Snapshot 節（内容は一致させる）」約束あり）
- `POLICIES.md` §2.1（2026-11-15 決定「仮想優先の受入
  と共通テスト基盤」— 方針正本）

## File Map

- `docs/designs/issue-193-migration-execution-plan.md` -
  新規 dated 節の追記（`## Deferred Design Concerns`
  直前。既存本文は不変）
- `docs/plans/issue-193-migration-execution-plan.md` -
  同一内容の新規 dated 節の追記（`## Implementation
  Handoff` 直前。既存本文は不変）
- `docs/specs|designs|test-specs|plans|tasks/issue-193-
  virtual-first-owner-remap*.md`・`docs/impl/{ir,exp}/
  issue-193-virtual-first-owner-remap*` - 本チェーン
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/ir/README.md` - 生成 index（doc-maintenance
  `index apply` で再生成。`docs/impl/exp` の管理 index
  は `*.jsonl` のみ対象のため `.md` exp 記録は対象外）

## Implementation Sequence

- [x] 参照 Issue の `gh issue view` 実測照合（RF
      #627/#628/#629/#630/#631/#294/#275/#251/#253/
      #388/#389/#532・AL #127・SV #28 — 2026-11-15）
- [x] spec → design → test-spec の文書チェーン作成
- [x] 両 canonical への同一 dated 節 `## Snapshot-
      2026-11-15 — Virtual-first owner 対応付け（版付き）`
      の追記（per-milestone 対応表・Issue 状態照合・
      dated 上書き宣言・維持区分・Verification Notes）
- [x] plan → task → impl(ir/exp) の文書チェーン完成
- [x] index 再生成（doc-maintenance `index apply`）
- [x] `mise run fmt-check` + `prek run --all-files` +
      `doc_status` lint/audit + 両節の抽出 diff 照合
- [ ] `git fetch origin main && git merge origin/main`
      → index 再生成 → commit → push → PR

## Dependencies and Risks

- **依存**: 親 spec/design（issue-193-migration-
  execution-plan）の approved 状態 — 既に approved。
  `Snapshot-2026-11-14` 節の「同名 Snapshot 節（内容は
  一致させる）」約束により、新規節は両ファイルで完全
  一致が必須。
- **リスク**: Issue 状態の変動 → 実測日付（2026-11-15）
  明記で緩和。次回 dated Snapshot で再照合。
- **リスク**: 旧 owner 記述との二重正本化 → dated 上書
  き宣言で「2026-11-15 以降の受入 owner 正本は本節」
  と明示し、旧 owner は mechanism/製品 Owner として
  残存すると役割分離。
- **リスク**: 相対リンクによるファイル間 drift → Issue
  は GitHub URL、文書は repo 相対パスの inline code で
  記述し完全一致を保証。

## Task Breakdown

- T1: Issue 状態照合と対応表語彙の確定（完了）
- T2: 文書チェーン作成（spec/design/test-spec/plan/
      task）
- T3: 両 canonical への同一 dated 節追記
- T4: impl ir/exp 記録と index 再生成
- T5: 検証（fmt-check・prek・doc_status・diff 照合・
      review-protocol）
- T6: merge main・commit・push・PR

## Verification

- [x] `gh issue view` による Issue 状態照合
      （2026-11-15）: #627 CLOSED、#628/#629/#630/#631
      OPEN、#294/#275/#251/#253/#388/#389/#532 OPEN、
      AL #127・SV #28 OPEN
- [x] 両 canonical の新規節が抽出 diff で完全一致
- [ ] `mise run fmt-check` が成功する
- [ ] `mise exec -- prek run --all-files` が成功する
- [ ] `doc_maintenance.js index apply` で触った dir の
      index が in-sync になる
- [ ] `doc_status.js lint`/`audit` で新規文書の
      blocking finding がない
- [ ] review-protocol で blocker が残っていない

## Open Questions / Gaps

- なし。Issue 状態・Snapshot 節構成・挿入位置は
  2026-11-15 時点で実測済み。残る不確実性は将来の
  状態変動のみで、snapshot 日付で扱う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and
      terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（文書のみ・単一
  スライス）
- Independent work streams: none
- User approval required before dispatch: no
  （review-protocol レビューを承認の代替とする —
  worker 共通契約）

## Review Handoff

- reviewer は新規節が両ファイルで完全一致であること、
  `Snapshot-2026-11-14` 節を含む既存本文に差分がない
  こと、Issue 参照の番号・状態が `gh` 実測と一致する
  こと、`NotRun`→`Pass` 置換や Required→Deferred 格
  下げがないことを重点確認する。
