---
id: "TASK-034Uhnq0kOMsOzPLSOL8Bz"
type: "task"
status: "done"
title: "Issue #378 T1 oracle addBinding epoch 整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/378"
  changes:
    added:
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    modified:
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-378-oracle-late-create-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #378 T1 oracle addBinding epoch 整合

## Work

`ui/guitest/src/oracle.ts` の `ContractModel.addBinding` を、修復済み SDK `ViewLink.registerBinding`（`ui/binding/src/link.ts`）と同じ epoch 採用順序へ整合させる: `setContext` を無条件に行い、`linkAttached` 時に `b.attach(this.linkGeneration)` → `subscribe` の順で処理する。併せてファイルヘッダの旧契約記述（late-create binding は view epoch を採用しない旨）を現行契約へ更新する。

恒久テストとして `ui/guitest/test/oracle.test.ts` を新設し、`ContractModel` を直接駆動して「初回 attach 後 / Dock（別 window attach）後 / detach→reattach 後に `addBinding`」の 3 ケースで late-create binding が link generation を採用することを検証する（`ui/binding/test/link.test.ts` の late-created 3 ケース構成を oracle の観測面へ移植）。scenario schema / op 語彙・`addQuery`・`ui/binding/`・fixture/golden には触れない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-378-oracle-late-create-epoch.md`
- `docs/specs/issue-378-oracle-late-create-epoch.md`
- `docs/designs/issue-378-oracle-late-create-epoch.md`
- `docs/test-specs/issue-378-oracle-late-create-epoch.md`

## Dependencies

- SDK 側の前提修復: Issue #376 / PR #379（merge 済み）。Issue #377（`QueryBinding` late-create）は未 merge のため `addQuery` は対象外。

## Affected Files

- `ui/guitest/src/oracle.ts` - `ContractModel.addBinding` とヘッダコメント。
- `ui/guitest/test/oracle.test.ts` - 新規 unit test。
- `docs/specs|designs|plans|test-specs|tasks/issue-378-*.md` と各 `README.md` index - 文書チェーン。

## Done When

- [ ] `addBinding` が `setContext` 無条件 → `linkAttached` 時 `attach(linkGeneration)` → `subscribe` の順で処理し、ヘッダ記述が整合している。
- [ ] oracle.test.ts の late-create 3 ケースが RED → GREEN を記録している。
- [ ] `addQuery` / `QueryModel`、`ui/binding/`、`replay.ts`、scenario 語彙、fixture/golden を変更していない。
- [ ] #377 merge 後に `addQuery` + ヘッダの query 契約記述の整合が必要である旨を PR 本文に記録している。

## Verification

- [ ] `mise run test-ui-guitest` exits 0（新規テストを含む全件 green。修正前は late-create 3 ケースが FAIL することを RED として記録済み）
- [ ] `mise run check-ui-guitest` exits 0（tsc --noEmit）
- [ ] `mise run fmt-check` exits 0
- [ ] `git fetch origin main && git merge origin/main` 後に上記を再実行して green
