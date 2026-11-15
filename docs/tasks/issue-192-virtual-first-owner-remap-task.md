---
id: "TASK-0AkBucIA3R2J63ZuOHF6p1"
type: "task"
status: "done"
title: "Issue #192 Virtual-first 受入 Owner remap タスク"
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
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
  depends-on:
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
    - "docs/tasks/issue-275-obs-owner-map-task.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  derived-by:
    - "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #192 Virtual-first 受入 Owner remap タスク

## Work

Issue #192 の残件（headless 可能な文書スライス）を実施する。

1. 参照 Issue の `gh issue view` 実在照合（ruviform
   #627〜#631・#294・#275・#251/#253/#388/#389・
   #243/#313、AL #127、SV #28）を行い、結果を
   `docs/impl/exp/` の記録に残す。
2. `docs/designs/issue-192-synvelis-driven-target-
   architecture-migration.md` 末尾へ dated addendum
   （受入 Owner remap 記録）を追記し、`updated` を
   当日日付へ更新する。既存 snapshot 表は不変。
3. `docs/discovery/issue-192-ta-invariant-map.md`
   「Headless Gap → Open Gate 接続一覧」節へ dated
   addendum（領域別 受入 Owner/実装 Owner 再指向）を
   追記し、`updated` を当日日付へ更新する。既存表・
   指定外行は不変。
4. `issue-192-virtual-first-owner-remap` 文書チェーン
   （spec/design/test-spec/plan/task/impl ir・exp）を
   完成させる。
5. 触った dir の生成 index を `doc-maintenance`
   `index apply` で再生成し、軽量検証
   （`mise run fmt-check`・`mise exec -- prek run
   --all-files`・doc-status 監査・type 別 lint・impl
   監査）と review-protocol レビューを行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-192-virtual-first-owner-remap-plan.md`
- `docs/specs/issue-192-virtual-first-owner-remap.md`
- `docs/designs/issue-192-virtual-first-owner-remap-design.md`
- `docs/test-specs/issue-192-virtual-first-owner-remap.md`
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  （追記対象の tracking canonical）
- `docs/discovery/issue-192-ta-invariant-map.md`
  （追記対象の対応表）

## Dependencies

- 親 spec/design（issue-192-synvelis-driven-target-
  architecture-migration）は approved。
- 対応表（issue-192-ta-invariant-map）は active。
- #628/#629/#630/#631・#294・AL #127・SV #28 は OPEN
  （受入 Owner 候補 — 本タスクの完了条件ではない）。
  #627・#243・#313 は CLOSED。#251/#253/#388/#389 は
  OPEN（実装 Owner — 維持）。

## Affected Files

- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
- `docs/discovery/issue-192-ta-invariant-map.md`
- `docs/specs|designs|test-specs|plans|tasks/issue-192-
  virtual-first-owner-remap*.md`（新規チェーン）
- `docs/impl/ir/issue-192-virtual-first-owner-remap.md`・
  `docs/impl/exp/issue-192-virtual-first-owner-remap.md`
  （新規）
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/ir/README.md`（生成 index の再生成）

## Done When

- [x] tracking canonical 末尾に dated addendum が追記
      され、remap 対応と各 Issue の番号・役割・状態が
      明記されている
- [x] 対応表「Headless Gap → Open Gate 接続一覧」節に
      dated addendum が追記され、受入/実装 Owner の
      分離・#243/#313 CLOSED・指定外行の不変が確認
      できる
- [x] Required→Deferred 降格・Pass 付与・NotRun の
      Pass 化がなく、維持原則が dated note に記されて
      いる
- [x] 文書チェーン 7 ファイルが作成され、検証記録が
      `docs/impl/exp/` に残っている
- [x] 生成 index が再生成され、軽量検証と
      review-protocol レビューが完了している

## Verification

- [x] `mise run fmt-check` が成功する
- [x] `mise exec -- prek run --all-files` が成功する
- [x] `doc_maintenance.js index plan` で触った全 dir が
      in-sync、`index apply` で再生成される
- [x] `doc_status.js audit --type all --profile artifact`・
      type 別 lint・`audit_impl_record.js`・
      `audit_experiment_log.js` で新規文書の finding が
      ない（既存の範囲外 finding は分離報告）
- [x] addendum の全参照が `gh issue view` 実測と一致する
- [x] review-protocol で blocker が残っていない
