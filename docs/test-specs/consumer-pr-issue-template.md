---
id: "TSPEC-034QdTKuBv9EDEBUuGxUT9"
type: "test-spec"
status: "draft"
title: "Consumer 影響判定・証拠テンプレート必須欄（PR/Issue template）"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Consumer 影響判定・証拠テンプレート必須欄（PR/Issue template）

## Purpose

`impacted`/`no impact`/`unknown` の Consumer 影響分類と検証証拠形式（R1/R4）が PR/Issue 運用で必須記入されることを保証する。

## Feature

PR 本文・Issue form の Consumer 影響欄。

## Rules

- PR template は両 Consumer 行（Synvelis・AudioLens）の分類・根拠・リンク欄を持つ
- 検証証拠表は lane/test・OS・feature・ruviform revision・結果(pass/fail/not run)・許容例外・Rollback の列を持つ
- `unknown` は未検証理由と解消先の記述を必須とする
- Issue template は Ruviform↔Consumer 相互リンク欄を持つ
- `unknown` の merge 時点確認はレビュアーの human gate として運用責務が設計に明記される

## Examples

- 共通 API 変更 PR → AudioLens `impacted` + #127 系リンク、Synvelis `unknown` + 解消 dispatch 予定
- CI 基盤のみの変更 → 両 Consumer `no impact` + 根拠のみ

## Guarantees

- 片 Consumer だけ緑で両方合格と読み替えられない
- `not run` と `fail` が区別される

## Non-goals

- 分類そのものの正しさ（人間の判断を機械保証しない）

## Risk

- 欄が無いと `unknown` が記録されず未検証滞留が可視化されない
