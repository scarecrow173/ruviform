---
id: "PLAN-034WuW2Nawrc0pCehmfMcn"
type: "plan"
status: "completed"
title: "Issue #590 実施計画 — ir missing-section 本文補完と検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/590"
    - "docs/specs/issue-590-ir-missing-sections.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-590-ir-missing-sections-plan.md"
      - type: "file"
        path: "docs/tasks/issue-590-ir-missing-sections.md"
      - type: "file"
        path: "docs/test-specs/issue-590-ir-missing-sections.md"
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
    - "docs/specs/issue-590-ir-missing-sections.md"
  implemented-by:
    - "docs/tasks/issue-590-ir-missing-sections.md"
  depends-on:
    - "docs/specs/issue-590-ir-missing-sections.md"
    - "docs/designs/issue-590-ir-missing-sections-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-590-ir-missing-sections.md"
    - "docs/designs/issue-590-ir-missing-sections-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #590 実施計画 — ir missing-section 本文補完と検証

## Goal

`audit_impl_record.js` が `missing-section` を報告する `docs/impl/ir/*.md` 50 ファイル・230 件に canonical 必須セクションの本文を補完し、missing-section を 0 にする。

## Scope

単一の実施ストリーム。文書のみの変更であり、コード実装・分割計画は不要。

## Upstream Documents

- docs/specs/issue-590-ir-missing-sections.md

## Design Inputs

- docs/designs/issue-590-ir-missing-sections-design.md

## File Map

- `docs/impl/ir/*.md`(missing-section 対象 50 ファイル) — canonical セクション本文の補完・既存非 canonical 見出しの再配置。対象一覧は audit findings と design 文書の内訳を参照。
- `docs/impl/ir/docs-impl-ir-missing-section-issue-590.md` — 本 Issue の ir(新規)。
- `docs/impl/ir/README.md` ほか触った docs dir の `README.md` — `doc-maintenance` `index apply` による再生成のみ。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-590-ir-missing-sections*.md` — doc-chain(新規 5 件)。

## Implementation Sequence

- [x] doc-chain(spec/design/plan/task/test-spec)を作成する。
- [x] `new_impl_record.js` で Issue #590 の in-progress IR を作成する。
- [x] audit の `missing-section` findings をファイル単位でグループ化し、ファイルごとに本文を読んで canonical セクションを補完する(既存内容の再配置優先・推測による捏造禁止)。
- [x] `audit_impl_record.js --json` で missing-section 0・他 code 件数非増加を確認する。
- [x] `doc-maintenance` `index plan --dir docs/impl/ir` で drift を確認し、必要なら `index apply` で再生成する。触った docs dir の index も同様に再生成する。
- [x] `git fetch origin main && git merge origin/main` で最新 main を取り込み、生成物を再生成してコミットに含める。
- [x] commit → push → PR。

## Dependencies and Risks

- 補完作業はファイル単位で独立しており、並列化可能。
- リスク: 既存内容の欠落 → 編集前後の突合で確認。実態不明の推測記述 → 文書・リンク先・Issue/PR 記録に根拠を限定。
- リスク: audit 他 code の増加 → 本文のみの変更で front matter/relations に触れないため原理上非増加。最終 audit で baseline 比較。

## Task Breakdown

- T1(docs/tasks/issue-590-ir-missing-sections.md): doc-chain・ir 作成、missing-section 対象ファイルの本文補完、audit/index 検証、index 再生成、commit/push/PR。

## Verification

- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` → `missing-section` 0、broken-relation-link 5 / missing-experiment-link 6 を超えない。
- [x] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir` → `in-sync`。
- [x] `git diff` で front matter・ファイル名の差分がないこと。

## Open Questions / Gaps

- なし(Issue #590 の受入条件・対象範囲は確定済み)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(文書のみの機械的に近い補完作業。ファイル単位の独立性から内部の並列作業は許容)。
- Independent work streams: なし(単一 task T1)。
- User approval required before dispatch: no(受入条件確定済みの是正タスク。レビューは review-protocol で実施)。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー観点: 追加本文が各文書の実態に基づくか(捏造・定型文でないか)、front matter/ファイル名が不変か、既存内容が欠落していないか。
