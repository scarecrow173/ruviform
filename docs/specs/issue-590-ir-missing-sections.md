---
id: "SPEC-034WuVytWK4glbu9CinZwq"
type: "spec"
status: "approved"
title: "Issue #590 docs/impl/ir の missing-section 本文補完"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/590"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-590-ir-missing-sections.md"
      - type: "file"
        path: "docs/designs/issue-590-ir-missing-sections-design.md"
      - type: "file"
        path: "docs/plans/issue-590-ir-missing-sections-plan.md"
      - type: "file"
        path: "docs/tasks/issue-590-ir-missing-sections.md"
      - type: "file"
        path: "docs/test-specs/issue-590-ir-missing-sections.md"
      - type: "file"
        path: "docs/impl/ir/docs-impl-ir-missing-section-issue-590.md"
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
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
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-590-ir-missing-sections-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "https://github.com/scarecrow173/apm-packages/issues/208"
  defers: []
  deferred-by: []
---

# Issue #590 docs/impl/ir の missing-section 本文補完

## What Should Be Built

`docs/impl/ir/` の ir 文書のうち、`audit_impl_record.js` が `missing-section` を報告する文書に対し、canonical な必須セクション(Summary / Context / Implementation / Related Experiments / Validation / Risks / Follow-ups)の本文を、各文書が記録する実装・検証の実態に沿って補完する。対象は audit の `missing-section` findings で列挙されるファイルに限る。

## Why Now

#586 で ir 文書の front matter を schema 適合させ index 生成を可能にした結果(PR #588 で統合)、それまで検査を通らず不可視だった本文の必須セクション不足が `missing-section` error として可視化された。#586 では受入条件の矛盾(本文変更禁止 vs audit error 増加)を避けるため本文は触らず、補完を本 Issue に分離した。missing-section を 0 にしない限り `audit_impl_record.js` の error 件数は減らず、ir 文書の audit が実質的な品質ゲートとして機能しない。

## Users and Value

- ir 文書を読む開発者・レビュアー: canonical セクション構成により、実装内容・検証・リスク・後続課題を一貫した位置で参照できる。
- doc-driven-dev の監査利用者: `missing-section` が 0 になり、audit の残存 error が本来対処すべきものだけになる。

## Scope

### In Scope

- `docs/impl/ir/*.md` の本文への canonical 必須セクションの補完(見出し以降のセクションのみ)。
- 補完に伴う `docs/impl/ir/README.md` の drift が出た場合の `doc-maintenance` `index apply` による再生成。
- 本 Issue の doc-chain(spec/design/plan/task/test-spec)と Issue #590 自身の ir。

### Out of Scope

- 各文書の front matter(id / status / relations を含む)とファイル名の変更。
- `missing-section` 以外の audit findings(`broken-relation-link` 5 / `missing-experiment-link` 6)と `audit_experiment_log.js` の findings の解消。
- `docs/impl/exp/`、`docs/impl/ir` 以外の docs dir、製品コードの変更。

## Requirements

1. `audit_impl_record.js --json` の `missing-section` が 0 件になる。
2. 追加する本文は対象文書の実装・検証内容に基づく具体的な記述とし、空見出し・TODO のみ・定型文の羅列にしない。実態が文書から分からない場合は、文書から分かる範囲の簡潔な記述に留め、推測で捏造しない。
3. 各文書の front matter(id / status / relations を含む)とファイル名を変更しない。
4. `missing-section` 以外の error code の件数が現状値を超えない。
5. `doc-maintenance` の `index plan --dir docs/impl/ir` が `in-sync` になる。

## Acceptance Criteria

- AC1: `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` の `missing-section` が 0 件。
- AC2: 追加した全セクションに対象文書の実装・検証内容に基づく具体的な本文がある。
- AC3: 各文書の front matter とファイル名が変わっていない。
- AC4: `missing-section` 以外の error code の件数が現状値(broken-relation-link 5 / missing-experiment-link 6)を超えない。
- AC5: `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir` が `in-sync`。

## Deferred Design Concerns

- `missing-section` 以外の audit findings(broken-relation-link / missing-experiment-link / experiment log の invalid-event-*)の解消は別 Issue 候補として対象外。

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/590
- 前提 Issue: #586(front matter 是正・index 生成可能化)/ 統合 PR #588。
- 上流: scarecrow173/apm-packages#208。
