---
id: "TASK-laPtj98NpzxI9OnVwLg5y4"
type: "task"
status: "done"
title: "T2 File・Structured 境界試験 (Issue #306)"
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
    - "docs/tasks/issue-306-t1-crate-scaffold.md"
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

# T2 File・Structured 境界試験 (Issue #306)

## Work

`tests/staged_lifecycle.rs`（R1/R2: create→write→seal→readback→
publish→再 Open、CreateNew 競合→NotPublished+DestinationExists、
Replace 差替え、sealed abort、未 seal drop の temp 回収）と
`tests/structured_staged.rs`（R3: json/csv write_file→read_file、
Encode 失敗→不 publish、Publish 失敗→PublishFailure 保持、
末尾混入→TrailingData）を実装。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
- `docs/test-specs/issue-306-io-composition-e2e.md`（R1-R3）
