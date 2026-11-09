---
id: "TASK-DBfsqONthDwUwmD4GpGXzq"
type: "task"
status: "done"
title: "Issue #490 T1 oracle addQuery epoch 整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/490"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-490-oracle-addquery-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
    - "https://github.com/scarecrow173/ruviform/issues/490"
  defers: []
  deferred-by: []
---

# Issue #490 T1 oracle addQuery epoch 整合

## Work

`ui/guitest/src/oracle.ts` の `ContractModel.addQuery` を、修復済み SDK `ViewLink.registerQuery`（`ui/binding/src/link.ts`、Issue #377 / PR #489）と同じ epoch 採用契約へ整合させる: `QueryModel` 生成後、`linkAttached` 時に `attach(this.linkGeneration)` で live view epoch を採用させる。併せてファイルヘッダの `QueryBinding` 契約記述（late-create query の epoch adopt を含まない旧記述）を現行契約へ更新する。

恒久テストとして `ui/guitest/test/oracle.test.ts` へ late-create query ケースを追加し、`ContractModel` を直接駆動して「初回 attach 後 / Dock（別 window attach）後 / detach→reattach 後に `addQuery`」の 3 ケースで query model が link generation を採用することを検証する（`addBinding` の 3 ケース構成・`ui/binding/test/link.test.ts` の late-created queries を oracle の観測面へ移植）。併せて attach 前登録の対照ケースを置く。scenario schema / op 語彙・`ui/binding/`・`replay.ts`・fixture/golden には触れない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-490-oracle-addquery-epoch.md`
- `docs/specs/issue-490-oracle-addquery-epoch.md`
- `docs/designs/issue-490-oracle-addquery-epoch.md`
- `docs/test-specs/issue-490-oracle-addquery-epoch.md`

## Dependencies

- SDK 側の前提修復: Issue #377 / PR #489（merge 済み）、Issue #378 / PR #488（`addBinding` 同型先例、merge 済み）。

## Affected Files

- `ui/guitest/src/oracle.ts` - `ContractModel.addQuery` とヘッダコメント。
- `ui/guitest/test/oracle.test.ts` - late-create query ケースの追加。
- `docs/specs|designs|plans|test-specs|tasks/issue-490-*.md` と各 `README.md` index - 文書チェーン。

## Done When

- [ ] `addQuery` が `linkAttached` 時に `attach(linkGeneration)` で live epoch を採用し、ヘッダ記述が整合している。
- [ ] oracle.test.ts の late-create query 3 ケースが RED → GREEN を記録している。
- [ ] `addBinding`、`ui/binding/`、`replay.ts`、scenario 語彙、fixture/golden を変更していない。

## Verification

- [ ] `mise run test-ui-guitest` exits 0（新規テストを含む全件 green。修正前は late-create query ケースが FAIL することを RED として記録済み）
- [ ] `mise run check-ui-guitest` exits 0（tsc --noEmit）
- [ ] `mise run fmt-check` exits 0
- [ ] `git fetch origin main && git merge origin/main` 後に上記を再実行して green
