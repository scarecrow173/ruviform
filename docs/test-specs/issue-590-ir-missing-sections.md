---
id: "TSPEC-034WuVVSQeKKcnzXki6miZ"
type: "test-spec"
status: "approved"
title: "Issue #590 テスト仕様 — missing-section 0・本文品質・front matter/filename 不変・audit 非回帰"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/590"
  changes:
    added: []
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
    - "docs/specs/issue-590-ir-missing-sections.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-590-ir-missing-sections.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #590 テスト仕様 — missing-section 0・本文品質・front matter/filename 不変・audit 非回帰

## Purpose

`docs/impl/ir` の missing-section 補完が、audit 上の形式要件だけでなく「実態に基づく本文」という質的要件を満たすことを保証する。ir 文書は実装の記録であり、canonical セクションが形だけ揃って内容を欠く状態は記録としての価値を損なうため、形式検証(audit)と内容検証(diff レビュー)の両方で担保する。

## Feature

ir 文書の canonical セクション補完 — `audit_impl_record.js` の `missing-section` findings が指す各文書に、実装・検証の実態に沿った必須セクション本文を持たせる。

## Rules

- R-1(網羅): `audit_impl_record.js --json` の `missing-section` findings が 0 件。
- R-2(非回帰): `missing-section` 以外の error/warning code の件数が baseline(broken-relation-link 5 / missing-experiment-link 6)を超えない。
- R-3(不変性): 変更は `docs/impl/ir/*.md` の本文(front matter 以降)のみ。front matter(id / status / relations を含む)・ファイル名・他ファイルは不変。
- R-4(内容の実態性): 追加・再配置した各セクションは対象文書の記録内容に基づく。空見出し・TODO のみ・定型文の羅列・推測による捏造を含まない。
- R-5(既存内容の保存): 既存本文の情報は canonical セクションへの再配置によって保持され、欠落しない。
- R-6(index 整合): `doc_maintenance.js index plan --dir docs/impl/ir` が `in-sync`。drift 時は `index apply` のみで再生成し手編集しない。

## Examples

- E-1(Given baseline で missing-section 230 件 / 50 ファイル、When 補完後に audit を実行、Then missing-section 0 件かつ他 code 件数が baseline 以下)。
- E-2(Given 補完済み ir 文書、When `git diff` で front matter ブロックとファイル名を確認、Then 差分なし)。
- E-3(Given `## Verification` 等の非 canonical 見出しを持つ文書、When 補完、Then 内容が `## Validation` 等の canonical セクションへ移り、情報が重複・欠落しない)。
- E-4(Given 実験ログを持たない文書、When `## Related Experiments` を補完、Then `### Adopted` / `### Rejected` が「なし。」の既存慣行に従う)。

## Guarantees

- audit の missing-section が 0 になり、ir 文書が canonical セクション構成を満たす。
- 各文書の記録内容(実装・検証・リスク・後続)が canonical セクションから追跡可能になる。
- front matter・ファイル名・他 docs dir・製品コードへの副作用がない。

## Non-goals

- `broken-relation-link` / `missing-experiment-link` / experiment log findings の解消。
- ir 文書の内容そのものの正確性の再監査(実装当時の記録をそのまま canonical 化する作業であり、記録内容の訂正は行わない)。
- `docs/impl/exp/` の変更。

## Risk

- 形式だけの空セクションや定型文で受入条件をすり抜けるリスク → R-4 を diff レビューで確認する。
- 大規模一括編集による既存内容の欠落・front matter の誤編集リスク → R-3/R-5 を git diff と audit で確認する。
