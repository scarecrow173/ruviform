---
id: "TASK-034UjQLrFyv9HY5QSH9HmW"
type: "task"
status: "done"
title: "Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/specs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/plans/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/tasks/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
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
    - "docs/plans/issue-257-driver-neutral-ui-contract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-257-driver-neutral-ui-contract.md"
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
    - "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合

## Work

Issue #257 の横断設計正本として、蓄積済み driver-neutral UI 契約
（#384 方針・#385 Theme・#386 Window/Host・#387 Component/View・
#390 Lab/Guard・#464 Scenario・#451/#478/#479 補遷契約）を
`docs/designs/issue-257-driver-neutral-ui-contract.md` へ参照統合し、
受入項目ごとの証拠対応表・歴史的参照の区別・GPUI 採否未確定の
明記を含む文書チェーン（spec/design/test-spec/plan/task）を作成
する。docs のみでコード変更なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- [plan](../plans/issue-257-driver-neutral-ui-contract.md)
- [spec](../specs/issue-257-driver-neutral-ui-contract.md)
- [design](../designs/issue-257-driver-neutral-ui-contract.md)
- [test-spec](../test-specs/issue-257-driver-neutral-ui-contract.md)

## Dependencies

- 参照先の契約文書・実装型が main に merge 済みであること
  （#385/#386/#387/#390 CLOSED、#451/#478/#479 CLOSED、#464 merge 済み・
  Issue Open、#465 P0/P1 merge 済み）。
- GPUI 採否未確定（#262 Open / #419 Open / #388・#389 Open）を
  前提として扱うこと。

## Affected Files

- `docs/designs/issue-257-driver-neutral-ui-contract.md`（新規・正本）
- `docs/specs/issue-257-driver-neutral-ui-contract.md`（新規）
- `docs/plans/issue-257-driver-neutral-ui-contract.md`（新規）
- `docs/tasks/issue-257-driver-neutral-ui-contract.md`（新規）
- `docs/test-specs/issue-257-driver-neutral-ui-contract.md`（新規）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [ ] 統合正本が領域別参照索引・証拠対応表・歴史的参照区別・
  未確定事項を含む。
- [ ] 証拠対応表の各エントリが実在する merge 済み PR / 文書 / 型を
  指す。
- [ ] GPUI 最終採用未確定（#262 verdict pending）が明記され、
  本書が採用確定の根拠にならない。
- [ ] 既存他 Issue 文書に変更がない。
- [ ] `mise run fmt-check` と doc-status 系 lint が pass
  （本 PR 追加分に新規 finding なし）。

## Verification

- [ ] `mise run fmt-check`
- [ ] doc-status lint/audit（`doc_status.js` 系。既存の範囲外
  finding は分離して記録）
- [ ] PR diff が `docs/` の新規 `issue-257-*` 文書と生成 index のみ
  であることを `git status`/`git diff` で確認。
