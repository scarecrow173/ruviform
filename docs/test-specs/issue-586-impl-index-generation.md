---
id: "TSPEC-qGTnI0yoN6l104dWZX2Oqx"
type: "test-spec"
status: "approved"
title: "Issue #586 テスト仕様 — index 生成可能性・front matter 不変性・audit 非回帰"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-586-impl-index-generation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #586 テスト仕様 — index 生成可能性・front matter 不変性・audit 非回帰

## Scope

`docs/impl/ir` の front matter 形式修正、`docs/impl/ir/README.md` / `docs/impl/exp/README.md` の generated-index 化、およびそれに伴う各 docs dir index の再生成を対象とする。製品コード・他 docs dir の本文は対象外。

## Invariants

- INV-1(意味不変): 既存文書の本文・ファイル名・既存 id は不変。差分は front matter ブロック内の形式修正のみ(issue-259/260 の新規 FM 追加を除く — id は新規採番であり既存 id の変更ではない)。
- INV-2(index 生成可能): `index plan --dir docs/impl/ir` / `--dir docs/impl/exp` が `rejected` でなく、apply 後に `in-sync`。引数なし `index apply` が exit 0。
- INV-3(audit 非回帰): `audit_impl_record.js` / `audit_experiment_log.js` で、変更前に schema 適合だったファイルに新規 error を作らない。front matter 修正により invalid-front-matter が減少する。
- INV-4(生成物の一貫性): 生成された README は marker を 1 つだけ含み、managed region が sync engine の出力と一致する。手書き preamble/tail は保持される。

## Cases

| case | 検証方法 | 期待 |
| --- | --- | --- |
| ir index plan | `doc_maintenance.js index plan --dir docs/impl/ir` | apply 前: `out-of-sync` または plan 生成可、apply 後: `in-sync` |
| exp index plan | 同上 `--dir docs/impl/exp` | `in-sync` |
| bare index apply | `doc_maintenance.js index apply` | exit 0(全 managed index が最新) |
| FM schema 適合 | `index plan` の source 検証 / `audit_impl_record.js` | 30 件の invalid-front-matter が解消 |
| audit 回帰 | `audit_impl_record.js --json` / `audit_experiment_log.js --json` の before/after 比較 | schema 適合済みファイルの error 非増加。可視化される missing-section は pre-existing として分離記録 |
| 本文不変 | `git diff` で FM ブロック外の差分がないこと | 差分なし |
| 再 plan | apply 後に `index plan` を再実行 | `in-sync`(冪等) |
