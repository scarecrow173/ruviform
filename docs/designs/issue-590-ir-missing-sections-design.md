---
id: "DESIGN-034WuVTmYfKSkp3sJ8biu2"
type: "design"
status: "approved"
title: "Issue #590 設計 — ir missing-section 補完方針・セクション記述規約・検証手順"
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
        path: "docs/designs/issue-590-ir-missing-sections-design.md"
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-590-ir-missing-sections.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-590-ir-missing-sections.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-590-ir-missing-sections.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-590-ir-missing-sections.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #590 設計 — ir missing-section 補完方針・セクション記述規約・検証手順

## Overview

`audit_impl_record.js` が `missing-section` を報告する `docs/impl/ir/*.md`(実測 50 ファイル・230 件)に対し、canonical 必須セクションの本文を補完する。変更は本文(見出し以降)に限定し、front matter・ファイル名・他 docs dir・製品コードは触らない。

canonical セクションと監査規則(audit_impl_record.js / impl-conventions.md より):

- 必須 `##` 見出し: `Summary` / `Context` / `Implementation` / `Related Experiments` / `Validation` / `Risks` / `Follow-ups`(大文字小文字不問、fenced code block 内は除外、日本語 alias なし)。
- 慣例構造: `## Implementation` は `### Changes` と `### Decisions During Implementation` を持ち、`## Related Experiments` は `### Adopted` / `### Rejected` を持つ。
- 完成例: `docs/impl/ir/implement-audio-core-data-model.md`、`docs/impl/ir/docs-impl-ir-exp-index-doc-maintenance.md`。

## Context

- #586 / PR #588 で ir front matter が `implementationRecordSchema` に適合し、schema 不正で検査をスキップされていた文書の本文不足が `missing-section` として可視化された。
- baseline(main `a97833dd` 実測): `missing-section` 230 件 / 50 ファイル。不足内訳: Related Experiments 44 / Risks 43 / Follow-ups 40 / Validation 37 / Implementation 27 / Context 23 / Summary 16。
- その他の findings(本 Issue 対象外): `broken-relation-link` 5 / `missing-experiment-link` 6(いずれも warning)。

## Scope

### In Scope

- `missing-section` findings の対象ファイルへの canonical セクション補完。
- 補完に伴う `docs/impl/ir/README.md` の drift 解消(`doc-maintenance` `index apply` のみ。手編集しない)。
- 本 Issue の doc-chain と Issue #590 自身の ir(`docs/impl/ir/docs-impl-ir-missing-section-issue-590.md`)。

### Out of Scope

- front matter(id / status / relations)・ファイル名の変更。
- `missing-section` 以外の findings の解消。
- `docs/impl/exp/`、他 docs dir、製品コード。

## Components and Boundaries

- 補完単位はファイル単位。各ファイルの本文に既に存在する非 canonical 見出し(`## Changes` / `## Verification` / `## What Was Done` / `## 実装内容` / `## 検証結果` / `## Decisions...` / `## Known Gaps` / `## NotRun` / `## 未検証項目` 等)は、内容を失わないよう canonical セクション配下へ再配置(見出し rename・格下げ)することを優先し、内容の複製は避ける。
- セクション記述の根拠は対象文書自身の本文と、front matter relations が指す task/spec/design/test-spec、source の GitHub Issue、対応 PR の記録。文書から実態が読み取れない場合は、読み取れる範囲の簡潔な記述に留め、推測で捏造しない。
- `Related Experiments` は `metadata.experiments.adopted` / `rejected` と本文中の `docs/impl/exp/` 参照に一致させる。実験がない場合は `### Adopted` / `### Rejected` ともに「なし。」とする既存慣行に従う。

## Data and Control Flow

- 入力: audit の `missing-section` findings(ファイル × 不足セクションの一覧)。
- 処理: ファイルごとに本文を読み、既存内容を canonical セクションへ再配置または新規記述を追加。
- 出力: 補完済み `docs/impl/ir/*.md`。検証は `audit_impl_record.js --json`(missing-section 0・他 code 非増加)と `doc_maintenance.js index plan --dir docs/impl/ir`(`in-sync`)。

## Non-Functional Constraints

- front matter・ファイル名・既存本文の情報量を保持する(セクション再配置は許容、内容削除は不可)。
- 追加本文は日本語を基本とし、コマンド・パス・識別子は原文のまま記す。文書ごとの既存言語に合わせる。
- 製品コード・CI・ビルド系検証は対象外(文書のみの変更)。

## Risks and Trade-offs

- 再配置時に既存内容を欠落させるリスク → 各ファイルで編集前後の本文を突合し、情報が canonical セクションへ移っていることを確認する。
- 実態不明のセクションを推測で埋めるリスク → 根拠は文書・リンク先文書・Issue/PR 記録に限定し、不明な場合は簡潔な実態記述に留める。全セクションが空見出しや定型文にならないことを受入条件で担保する。
- audit の他 error code の増加 → 補完は本文のみで front matter・relations に触れないため、broken-relation-link / missing-experiment-link は原理上増えない。最終 audit で件数を baseline と比較して確認する。

## Verification Notes

- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` で `missing-section` 0 件・他 code 件数が baseline(broken-relation-link 5 / missing-experiment-link 6)以下であること。
- [x] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir` が `in-sync` であること(drift 時は `index apply` で再生成)。
- [x] `git diff` で front matter・ファイル名に差分がないこと、追加セクションが実態に基づく本文であることを確認。
