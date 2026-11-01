---
id: "TASK-034QdVbbaa7jKsLfhGKepC"
type: "task"
status: "done"
title: "T4 検証・IR 記録（actionlint・doc audit・実装記録）"
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
  implements:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T4 検証・IR 記録（actionlint・doc audit・実装記録）

## Work

T1–T3 完了後の検証と実装記録（IR）作成。
- `actionlint`（consumer-verify.yml 対象）
- `audit_docs.js` で本 Issue 新規文書（discovery/spec/adr/design/plan/task/test-spec）の blocking finding 0 確認
- `docs/impl/ir/` に IR を作成（impl-doc の convention に従う）: 実装内容・実行した検証・未検証項目（credential 未設定のため初回実走は別運用）を区別して記録

## Classification

- Type: implementation
- Size: S

## Upstream Documents

- `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`

## Dependencies

T1・T2・T3 完了後

## Affected Files

- `docs/impl/ir/<ir-file>.md`

## Done When

- actionlint pass・doc audit の blocking finding なし
- IR が実装/検証/未検証を分離して記録

## Verification

- [ ] 上記コマンド実行結果を IR に記録
