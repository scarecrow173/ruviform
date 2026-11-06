---
id: "TASK-BkA2wlhpSoTJR2UkZvWTJX"
type: "task"
status: "done"
title: "T5 mise task 追加と正規検証 (Issue #306)"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/plans/issue-306-io-composition-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-306-t2-file-structured-boundary.md"
    - "docs/tasks/issue-306-t3-archive-compression-boundary.md"
    - "docs/tasks/issue-306-t4-wav-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# T5 mise task 追加と正規検証 (Issue #306)

## Work

`mise.toml` に `[tasks.test-io-e2e]`（`cargo test -p
ruviform-io-e2e`）を追加し、`mise run test-io-e2e`・
`mise run fmt-check`・`mise run lint`・`mise run test` を実走する。
AGENTS.md のクレート数表記を更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
