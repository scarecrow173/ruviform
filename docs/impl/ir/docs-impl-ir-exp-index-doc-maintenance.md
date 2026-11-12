---
id: "IMPL-034WsHV56f7WLl8ACG16mb"
type: "impl"
status: "completed"
title: "docs/impl/ir・exp の管理 index を doc-maintenance で生成可能にする"
created: "2026-11-12"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/586"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-586-impl-index-generation.md"
      - type: "file"
        path: "docs/designs/issue-586-impl-index-generation-design.md"
      - type: "file"
        path: "docs/plans/issue-586-impl-index-generation-plan.md"
      - type: "file"
        path: "docs/tasks/issue-586-impl-index-generation.md"
      - type: "file"
        path: "docs/test-specs/issue-586-impl-index-generation.md"
      - type: "file"
        path: "docs/impl/ir/docs-impl-ir-exp-index-doc-maintenance.md"
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
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
  implements:
    - "docs/tasks/issue-586-impl-index-generation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
    - "docs/specs/issue-586-impl-index-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-586-impl-index-generation-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# docs/impl/ir・exp の管理 index を doc-maintenance で生成可能にする

## Summary

`docs/impl/ir` の 30 件の front matter を `implementationRecordSchema` に適合させ、`docs/impl/ir/README.md` と `docs/impl/exp/README.md` に generated-index marker を追加して `doc-maintenance` の index apply で管理 index を生成可能にした。

## Context

Issue #586 / `docs/specs/issue-586-impl-index-generation.md` / `docs/designs/issue-586-impl-index-generation-design.md` / `docs/plans/issue-586-impl-index-generation-plan.md` / `docs/tasks/issue-586-impl-index-generation.md` に従う。

## Implementation

### Changes

- `docs/impl/ir` の 30 件の front matter を機械修正(本文・ファイル名・既存 id は不変)。分類と判断は design 文書に記録。
- `docs/impl/ir/README.md` に generated-index marker を挿入し sync engine で再生成。
- `docs/impl/exp/README.md` 末尾に managed region を追加し sync engine で再生成。
- doc-chain(spec/design/plan/task/test-spec)を新規作成。

### Decisions During Implementation

- front matter 是正は形式のみ(意味を変えない)。`type: "impl-record"` → `"impl"`、`status: "done"`/`"delivered (...)"` → `"completed"`。issue-259/260 は front matter が皆無のため新規追加し、id は `IMPL-<22 Base62>` を新規採番、`created`/`updated` は git 初回コミット日、owners はコミット author。
- `relations.changes` の文字列エントリは `{type, path}` オブジェクトに変換し、ディレクトリ(trailing `/` または repo 内実在ディレクトリ)は `type: "directory"`。
- 本文の missing-section 不足は本タスク範囲外(別 Issue で対応予定)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

Windows 11 / Git Bash / Node.js v26.12.1 で実行。

- `doc_maintenance.js index plan --dir docs/impl/ir` → `sync`(source validation 通過・marker 追加前は `rejected: missing generated-index marker`)、`index apply --expect <planHash>` で `docs/impl/ir/README.md` を書き込み、再 plan で `in-sync`。
- `doc_maintenance.js index plan --dir docs/impl/exp` → 同様に `sync` → apply → `in-sync`。
- `doc_maintenance.js index apply`(引数なし)→ exit 0、5 つの shared-suite index(specs/designs/plans/tasks/test-specs)を再生成、blocked なし。全 10 target が `in-sync`。
- `audit_impl_record.js --json`: error 187 → 230。`invalid-front-matter` 104 → 0(全件解消)、`index-not-managed` warning 1 → 0。`missing-section` 83 → 230(うち +147 は front matter 適合化で新たに検査対象となった既存文書の事前存在する不足分)。`broken-relation-link` 5・`missing-experiment-link` 6 は変化なし。
- `audit_experiment_log.js --json`: error 7 → 7(変化なし)、`index-not-managed` warning 1 → 0。
- 30 ファイルの本文が HEAD と byte 一致すること、既存 id が不変であることを script で確認(issue-259/260 は FM 新規追加のため本文のみ比較・完全一致)。

## Risks

- `missing-section` error が 230 件残る。いずれも本文の `##` セクション構成の既存不足であり、本タスクの front matter 是正で検査対象に復帰したことで可視化されたもの。別 Issue での本文補完が必要。
- `broken-relation-link` 5 件・`missing-experiment-link` 6 件の warning は本タスク前から存在し未対応。

## Follow-ups

- `docs/impl/ir` の 50 ファイル・230 件の `missing-section`(不足セクションの一覧は PR 本文に記録)を canonical セクション構成へ補完する後続 Issue。Coordinator が起票・依存管理を行う。
