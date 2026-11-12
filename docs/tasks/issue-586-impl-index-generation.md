---
id: "TASK-rQRQ2bkMVhq40SWTKL1zXd"
type: "task"
status: "done"
title: "Issue #586 T1 — ir front matter 是正・ir/exp index 再生成・audit 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/586"
  changes:
    added: []
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
    generated: []
  implements:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
    - "docs/specs/issue-586-impl-index-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #586 T1 — ir front matter 是正・ir/exp index 再生成・audit 検証

## Objective

`docs/impl/ir` 30 件の front matter を schema 適合させ、`docs/impl/ir/README.md` と `docs/impl/exp/README.md` に generated-index marker を追加して `doc-maintenance` の index apply で管理 index を生成可能にする。

## Checklist

- [x] doc-chain(spec/design/plan/task/test-spec)作成
- [x] `docs/impl/ir` 30 件の front matter を design の分類 A–E で機械修正(本文・ファイル名・既存 id 不変)
- [x] `docs/impl/ir/README.md` に generated-index marker 追加
- [x] `docs/impl/exp/README.md` 末尾に managed region(marker + generated table)追加
- [x] `index apply --dir docs/impl/ir` / `--dir docs/impl/exp` 実行後 `index plan` が `in-sync`
- [x] 引数なし `index apply` が exit 0
- [x] 触った docs dir の管理 index を再生成
- [x] audit_impl_record.js / audit_experiment_log.js で baseline 比較(schema 適合済みファイルの error 非増加)
- [x] `git merge origin/main` → index 再生成 → commit → push → PR
