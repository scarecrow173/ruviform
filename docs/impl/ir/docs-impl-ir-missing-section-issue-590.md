---
id: "IMPL-034WuWdvMVZx1zeCjy7MoV"
type: "impl"
status: "completed"
title: "docs/impl/ir missing-section 本文補完（Issue #590）"
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
  implements:
    - "docs/tasks/issue-590-ir-missing-sections.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
    - "docs/specs/issue-590-ir-missing-sections.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-590-ir-missing-sections-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/586"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# docs/impl/ir missing-section 本文補完（Issue #590）

## Summary

`audit_impl_record.js` が `missing-section` を報告していた `docs/impl/ir/` の 50 ファイル・230 件に対し、canonical 必須セクション(Summary / Context / Implementation / Related Experiments / Validation / Risks / Follow-ups)の本文を補完した。既存の非 canonical 見出し(`## Changes` / `## Verification` / `## What Was Done` / `## 実装内容` / `## 検証結果` 等)は内容を失わないよう canonical セクションへ再配置(rename・格下げ)し、front matter・ファイル名は変更していない。

## Context

- Issue: <https://github.com/scarecrow173/ruviform/issues/590>
- task: [../../tasks/issue-590-ir-missing-sections.md](../../tasks/issue-590-ir-missing-sections.md)
- plan: [../../plans/issue-590-ir-missing-sections-plan.md](../../plans/issue-590-ir-missing-sections-plan.md)
- spec: [../../specs/issue-590-ir-missing-sections.md](../../specs/issue-590-ir-missing-sections.md)
- design: [../../designs/issue-590-ir-missing-sections-design.md](../../designs/issue-590-ir-missing-sections-design.md)
- test-spec: [../../test-specs/issue-590-ir-missing-sections.md](../../test-specs/issue-590-ir-missing-sections.md)
- 前提: #586(front matter 是正・index 生成可能化)/ PR #588 が main `a97833dd` に統合済み。

## Implementation

### Changes

- doc-chain(spec/design/plan/task/test-spec)を新規作成。
- `docs/impl/ir/` の missing-section 対象 50 ファイルの本文を補完。ファイル単位で独立した作業のため、内部の並列作業(7 系統)に分割して実施。
- `docs/impl/ir/README.md`・`docs/{specs,designs,plans,tasks,test-specs}/README.md` を `doc-maintenance` `index apply` で再生成。

### Decisions During Implementation

- 既存の非 canonical 見出しは削除せず canonical セクションへ再配置する方針を統一(`## Verification`→`## Validation`、`## Changes`/`## What Was Done`/`## 実装内容`→`## Implementation`+`### Changes`、`## Decisions...`→`### Decisions During Implementation`、`## Known Gaps`/`## NotRun`/`## 未検証項目`→`## Risks`/`## Follow-ups`/`## Validation` へ内容を振り分け)。
- `## Related Experiments` は `metadata.experiments.adopted`/`rejected` と本文中の `docs/impl/exp/` 参照に一致させ、実験のない文書は `### Adopted`/`### Rejected` ともに「なし。」とする既存慣行に従った。実際に exp 参照があった文書(issue-157, issue-329, issue-306, t1-check-step-dedup)は採用/却下の実内容を記載。
- 実態が文書から読み取れないセクションは、文書・リンク先 task/spec/test-spec・Issue/PR 記録に根拠を限定した簡潔な記述に留め、推測による捏造を避けた。

## Related Experiments

### Adopted

なし(本タスク自体は実験的検証を伴わない文書メンテナンス)。

### Rejected

なし。

## Validation

Windows 11 / Git Bash / Node.js v26.12.1 で実行。

- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json`: `missing-section` 230 件 → 0 件。`broken-relation-link` 5 / `missing-experiment-link` 6 は baseline と同数で非増加。index 系 findings(`index-missing-entry`/`index-ordering`)は新規 ir 追加による drift で、`index apply` 後に解消。
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir`: `in-sync`(apply 後)。
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply --dir docs/{specs,designs,plans,tasks,test-specs}`: 各 dir の README を再生成(blocked なし)。
- front matter 不変性: 変更した 50 ファイル全件で、front matter ブロック(先頭 2 つの `---` 間)が HEAD と byte 一致することを script で確認。
- canonical 見出しの重複なし: 全 `docs/impl/ir/*.md` で `## Summary|Context|Implementation|Related Experiments|Validation|Risks|Follow-ups` の出現回数が各 1 以下であることを確認。
- `git fetch origin main && git merge origin/main`: Already up to date(main `a97833dd`)。
- review-protocol: Reviewer A(協調)/Reviewer B(敵対)の独立レビュー → Validator 検証 → Judge 判定で `ACCEPT WITH NON-BLOCKING NOTES`。CONFIRMED は P3 の簿記未完了のみで修正済み。P2/P3 の既存負債(後述 Follow-ups)は本タスク範囲外として PR 本文に開示。

## Risks

- 50 ファイル一括の本文再配置のため、稀に既存内容の欠落・不自然な構造が混入するリスク → review-protocol の独立レビューと audit/index 検証で確認。指摘は CONFIRMED のみ修正。
- 補完した Risks/Follow-ups/Validation の記述が各文書の実態とずれるリスク → 根拠を文書・リンク先・Issue/PR 記録に限定するルールで抑制。レビューで抜き取り検証。

## Follow-ups

- `missing-section` 以外の audit findings(`broken-relation-link` 5 / `missing-experiment-link` 6)と `audit_experiment_log.js` の findings(invalid-event-* 7 件)の解消は Issue #590 の対象外であり、別 Issue 候補。
- review で確認した既存負債(本タスクの変更対象外・別 Issue 候補): bare prose の `## Related Experiments` 6 件(issue-306 系 3 件・issue-195・issue-191・io-06-gate-run-ir。issue-306-io-e2e-slice-2 は本文「なし」と front matter `metadata.experiments.rejected` 3 件が不整合)、`### Adopted`/`### Rejected` が HTML コメントのみの既存 IR 19 件(うち 2 件は英語テンプレートプレースホルダ残存: issue-299-io-structured-xml・fix-time-arithmetic-saturation)、issue-156-gui-foundation-verification の `../exp/data/issue-156-gate-*.log` 参照先が `.gitignore` の `*.log` で未コミットのため検証不能。
- `docs/impl/exp` 文書の canonical 化が必要になった場合も別 Issue で扱う。
