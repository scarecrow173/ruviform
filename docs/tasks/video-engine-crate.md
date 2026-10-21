---
id: "TASK-034JZVID00000000000060"
type: "task"
status: "done"
title: "ruviform-video クレート実装 + テスト"
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
    - "docs/plans/implement-issue-27-video-engine.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-27-video-engine.md"
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

# ruviform-video 実装

## Done When

- [x] クレート新設 + pipeline/sync/session 実装
- [x] test-spec 5 ケースグリーン

## Verification

- `cargo test -p ruviform-video`・`mise run lint`
