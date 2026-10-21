---
id: "TASK-034JZDNRML000000000006"
type: "task"
status: "done"
title: "DenormalFlush ノード実装 + テスト"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "docs/plans/implement-issue-54-denormal-policy.md"

  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-54-denormal-policy.md"

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

# DenormalFlush ノード実装

## Done When

- [x] `crates/ruviform-audio/src/denormal.rs` に `DenormalFlush` 実装・lib.rs export
- [x] test-spec の4ケースがグリーン

## Verification

- `cargo test -p ruviform-audio`
- `mise run lint`
