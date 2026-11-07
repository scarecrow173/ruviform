---
id: "TASK-MzTtn00zwhx1kzvEGUUep1"
type: "task"
status: "done"
title: "T3 Archive・Compression 境界試験 (Issue #306)"
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

# T3 Archive・Compression 境界試験 (Issue #306)

## Work

`tests/archive_compression_staged.rs`（R4/R5: TAR→gzip→StagedFile・
TAR→zstd→StagedFile の 3 層合成と finish 直列化、切り詰め gzip の
Decoder::finish 失敗→abort→不 publish、未 finish entry の
poisoned archive）と `tests/extraction.rs`（R6:
publish 済み zip/tar.gz→再 Open→verify→extract→tree 照合、
`../` entry・casefold 衝突・budget 超過の否定経路で staging
非残存）を実装。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
- `docs/test-specs/issue-306-io-composition-e2e.md`（R4-R6）
