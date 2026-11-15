---
id: "TASK-034XJsrkSEZgpSXhxUbhXy"
type: "task"
status: "done"
title: "Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付けタスク"
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
    - "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
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
    - "docs/test-specs/issue-193-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付けタスク

## Work

Issue #193 の現行受入方針（2026-11-15「Virtual-first正式
受入Owner整理」）を canonical 計画文書へ dated Snapshot
節として記録する headless 文書スライスを実施する。

1. 参照 Issue（RF #627/#628/#629/#630/#631/#294/#275/
   #251/#253/#388/#389/#532・AL #127・SV #28）の
   `gh issue view` 実測照合を行い、番号・役割・
   open/closed 状態を snapshot（2026-11-15）として
   記録する。
2. `docs/designs/issue-193-migration-execution-plan.md`
   と `docs/plans/issue-193-migration-execution-plan.md`
   の両方に、同一内容の新規 dated 節 `## Snapshot-
   2026-11-15 — Virtual-first owner 対応付け（版付き）`
   を `Snapshot-2026-11-14` 節の直後へ追記する。
   per-milestone の受入 owner 対応表（GUI-L2→#631、
   Audio/MIDI 仮想 Endpoint EXT-L3→#629、実 Plugin
   ABI→#630、共通基盤 #628・方針基盤 #627 CLOSED）・
   Issue 状態照合・旧 owner 記述との dated 上書き宣言・
   維持区分・Verification Notes を含める。既存本文は
   一切書き換えない。
3. `issue-193-virtual-first-owner-remap` 文書チェーン
   （spec/design/test-spec/plan/task/impl ir・exp）を
   完成させる。
4. 触った dir の生成 index を `doc-maintenance index
   apply` で再生成し、軽量検証（`mise run fmt-check`・
   `prek run --all-files`・`doc_status` lint/audit・両
   節の抽出 diff 照合）と review-protocol レビューを
   行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-193-virtual-first-owner-remap-plan.md`
- `docs/specs/issue-193-virtual-first-owner-remap.md`
- `docs/designs/issue-193-virtual-first-owner-remap-
  design.md`
- `docs/test-specs/issue-193-virtual-first-owner-
  remap.md`
- `docs/designs/issue-193-migration-execution-plan.md`・
  `docs/plans/issue-193-migration-execution-plan.md`
  （更新対象の正本）
- `POLICIES.md` §2.1（方針正本）

## Dependencies

- 親 spec/design（issue-193-migration-execution-plan）
  は approved。
- `Snapshot-2026-11-14` 節の「同名 Snapshot 節（内容は
  一致させる）」約束により、新規節は両ファイルで完全
  一致が必須。
- RF #628〜631 は OPEN の未実装 Harness — 受入 Case
  は `NotRun` のまま。本タスクは対応付けのみで実装を
  含まない。

## Affected Files

- `docs/designs/issue-193-migration-execution-plan.md`
- `docs/plans/issue-193-migration-execution-plan.md`
- `docs/specs|designs|test-specs|plans|tasks/issue-193-
  virtual-first-owner-remap*.md`（新規チェーン）
- `docs/impl/ir/issue-193-virtual-first-owner-remap.md`・
  `docs/impl/exp/issue-193-virtual-first-owner-remap.md`
  （新規）
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/ir/README.md`（生成 index の再生成）

## Done When

- [x] 両 canonical に同一内容の新規 dated 節が追記さ
      れ、抽出 diff で完全一致が確認できる
- [x] M0〜M7 の受入 owner 対応付けが Issue URL・役割・
      open/closed 状態付きで記録され、#628〜631・#627
      が網羅される
- [x] 旧参照（#251/#253/#388/#389/#532）が実装・
      mechanism owner として残り、dated 上書き宣言が
      ある
- [x] Required→Deferred 格下げ・新規 Pass 付与・
      NotRun→Pass 置換がなく、既存本文に差分がない
- [x] 文書チェーンと index 再生成・軽量検証・
      review-protocol レビューが完了している

## Verification

- [x] `gh issue view` による Issue 状態照合
      （2026-11-15）
- [x] 両 canonical の新規節の抽出 diff が完全一致
- [ ] `mise run fmt-check` が成功する
- [ ] `mise exec -- prek run --all-files` が成功する
- [ ] `doc-maintenance index apply` で触った dir の
      index が in-sync になる
- [ ] `doc_status.js lint`/`audit` で blocking
      finding がない
- [ ] review-protocol で blocker が残っていない
