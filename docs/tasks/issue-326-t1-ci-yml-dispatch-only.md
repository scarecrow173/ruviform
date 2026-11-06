---
id: "TASK-LSK3NBKCdjJ7fablK2DEFK"
type: "task"
status: "done"
title: "T1 ci.yml dispatch 専用化 + concurrency 簡素化 + コメント同期（Issue #326）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "docs/plans/issue-326-ci-manual-only.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-326-ci-manual-only.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-326-ci-manual-only.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-326-t2-trigger-contract-test.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-326-ci-manual-only.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1 ci.yml dispatch 専用化 + concurrency 簡素化 + コメント同期（Issue #326）

## Work

`.github/workflows/ci.yml` の `on:` を `workflow_dispatch:`（`inputs.expected_sha` 宣言は name/type/required/description 一切不変 — Daily dispatch 契約）のみとし、`pull_request`（opened/synchronize/reopened/ready_for_review/edited）と `push`（branches: main）を除去。`concurrency` を `group: ${{ github.workflow }}-${{ github.run_id }}`・`cancel-in-progress: false` に簡素化（ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） の非 PR 不変条件と同一解）。PR 前提のコメント（header・`expected_sha` の "Empty disables (manual dispatch, pull_request, push)"・各 lane の "no-op for pull_request/push"・`ci-summary` transition note）を実態に同期 — lane/job/steps/permissions/env の実行構造は一切変更しない。

## Classification

- `normal-plan-task`

## Acceptance

- [x] `on:` が `workflow_dispatch` のみ、`pull_request:`/`push:` が 0 件。
- [x] `expected_sha` input 宣言が不変。
- [x] concurrency が `ci-<run_id>` 一意・`cancel-in-progress: false`。
- [x] `actionlint` clean・`git diff` で lane/job/permissions 不変を確認。
- [x] `ci-issue-tracker.yml`/`reconcile`/`daily-ci`/`consumer-verify` は byte-identical。
