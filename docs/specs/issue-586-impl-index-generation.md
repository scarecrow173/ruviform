---
id: "SPEC-Ede97f0t7vgWfr5wdww2Ib"
type: "spec"
status: "approved"
title: "Issue #586 docs/impl/ir・exp の管理 index を doc-maintenance で生成可能にする"
created: "2026-11-13"
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
  implements: []
  implemented-by:
    - "docs/plans/issue-586-impl-index-generation-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-586-impl-index-generation-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  references:
    - "https://github.com/scarecrow173/apm-packages/issues/208"
  defers: []
  deferred-by: []
---

# Issue #586 docs/impl/ir・exp の管理 index を doc-maintenance で生成可能にする

## Problem

`docs/impl/ir` と `docs/impl/exp` の管理 index(README)が `doc-maintenance` の `index plan` / `index apply` で生成できず(`rejected`)、文書の追加・status 変更のたびに index 行を手動で追記している。他の docs dir(specs/designs/plans/tasks/test-specs/adr/ideas/discovery)は生成可能であり、impl だけが手作業の例外になっている。

## Diagnosis(変更前に診断する)

- `index plan --dir docs/impl/ir` は `rejected`(index sources failed validation)。ir 文書 240 件のうち 30 件の front matter が `implementationRecordSchema` に違反する。内訳: `metadata` 欠落 15 件、`metadata.experiments` 欠落 2 件、`relations.changes.*` がオブジェクトでなく文字列 9 件、`type` が `"impl"` でない 4 件、`id` 欠落 2 件。
- `index plan --dir docs/impl/exp` は `rejected`(hand-curated or ambiguous index: missing generated-index marker)。exp の README は手書きキュレーションで generated-index marker を持たない。source は `*.jsonl` 15 件。
- `docs/impl/ir/README.md` も marker を持たない手書き index であり、source 検証を通過させても同じく `rejected` になる。
- audit(`audit_impl_record.js`)は schema 不正のファイルで section 検査をスキップするため、front matter を適合させると既存文書の `missing-section` 不足が新たに可視化される(事前計測: invalid-front-matter 104 件が解消される一方 missing-section が約 147 件出現し、error 合計は 187 → 約 230)。これは検査の空白が埋まっただけであり、新規の品質低下ではない。

## Requirements

1. `docs/impl/ir` の 30 件の front matter を schema に適合させる。是正は意味を変えない形式修正に限る(`metadata` / `metadata.experiments` の構造追加、`relations.changes` エントリの `{type, path}` オブジェクト化、`type` / `status` / `id` / `created` / `updated` / `owners` の形式修正)。
2. `docs/impl/exp/README.md` と `docs/impl/ir/README.md` に generated-index marker を追加し、手書き部分を保持したまま owner の sync engine(`sync_impl_index` / `doc_maintenance index`)で再生成する。
3. 既存文書の本文・ファイル名・id は変更しない。`id` を持たない 2 件(issue-259/260)への id 付与は schema 適合に必要な新規採番であり、既存 id の振り直しではない。
4. `index plan --dir docs/impl/ir` / `--dir docs/impl/exp` が `in-sync` になり、引数なしの `index apply` が exit 0 になる。
5. `audit_impl_record.js` / `audit_experiment_log.js` で、これまで schema 適合だったファイルに新規 error を作らない。可視化される既存 `missing-section` 不足は本 Issue の対象外として記録する。

## Acceptance Criteria

- AC1: `index plan --dir docs/impl/ir` が `rejected` でなくなり、`index apply` 後に `index plan` が `in-sync`。
- AC2: `docs/impl/exp` も同様に `in-sync`。
- AC3: 引数なしの `index apply` が exit 0。
- AC4: ir / exp の既存文書の本文・ファイル名・id が変わっていない(front matter の形式修正のみ)。
- AC5: `audit_impl_record.js` / `audit_experiment_log.js` の error が、schema 適合済みファイルについて増えていない。

## Non-goals

- 既存文書本文への `##` セクション補完(missing-section の解消は別 Issue で扱う)。
- `migrate_ids` による id / ファイル名の振り直し。
- 他の docs dir の index drift 解消。
- 製品コード(`crates/`、`Cargo.toml`、`mise.toml`、`.github/` 等)の変更。
