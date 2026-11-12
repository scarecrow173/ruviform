---
id: "TASK-034WuW3rqyBgYH8WlRZH2D"
type: "task"
status: "done"
title: "Issue #590 T1 — ir missing-section 本文補完・audit/index 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/590"
  changes:
    added:
      - type: "directory"
        path: "docs/impl/ir/"
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
  implements:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
    - "docs/specs/issue-590-ir-missing-sections.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-590-ir-missing-sections-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #590 T1 — ir missing-section 本文補完・audit/index 検証

## Work

`audit_impl_record.js` が `missing-section` を報告する `docs/impl/ir/*.md` 50 ファイル・230 件に対し、canonical 必須セクション(Summary / Context / Implementation / Related Experiments / Validation / Risks / Follow-ups)の本文を、各文書の実装・検証の実態に沿って補完する。既存の非 canonical 見出しは内容を失わないよう canonical セクションへ再配置し、front matter・ファイル名は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/issue-590-ir-missing-sections-plan.md
- docs/specs/issue-590-ir-missing-sections.md
- docs/designs/issue-590-ir-missing-sections-design.md
- docs/test-specs/issue-590-ir-missing-sections.md

## Dependencies

- なし(前提 Issue #586 / PR #588 は main `a97833dd` に統合済み)。

## Affected Files

- `docs/impl/ir/*.md` — missing-section 対象 50 ファイルの本文(見出し以降)のみ。
- `docs/impl/ir/docs-impl-ir-missing-section-issue-590.md` — 本タスクの ir(新規)。
- `docs/impl/ir/README.md`、`docs/{specs,designs,plans,tasks,test-specs}/README.md` — index apply による再生成。

## Done When

- [x] `audit_impl_record.js --json` の `missing-section` が 0 件。
- [x] `missing-section` 以外の error code 件数が baseline(broken-relation-link 5 / missing-experiment-link 6)を超えない。
- [x] 追加セクションが対象文書の実装・検証内容に基づく具体的な本文である(空見出し・TODO のみ・定型文でない)。
- [x] 各文書の front matter(id / status / relations を含む)とファイル名が変わっていない。
- [x] `doc_maintenance.js index plan --dir docs/impl/ir` が `in-sync`。

## Verification

- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json`
- [x] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir`
- [x] `git diff` で front matter・ファイル名の差分がないこと。
