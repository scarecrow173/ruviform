---
id: "TASK-034L6yib4dmV9rd1MUZxbw"
type: "task"
status: "todo"
title: "OFX host final verification"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
    - "docs/tasks/ofx-render-implementation.md"
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
    - "docs/test-specs/video-plugin-plain-contract-tests.md"
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# OFX host final verification

## Work

**Full Gate（1 回のみ・plan 最終段）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise exec -- prek run --all-files`・doc audit — 全 log 保存。`test-rt` は実一致件数で証明（0 件スキップを成功と偽らない）。Windows ローカル + Rust 1.89 実証のみ・mac/Linux/CI は実施時のみ記録

## Verification

- [ ] `mise run fmt-check` — exit 0
- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0
- [ ] `mise run test-rt` — exit 0 かつ **実一致件数 >0** の log（0 件スキップを成功と偽らない）
- [ ] `mise run bench-smoke` — exit 0
- [ ] `mise exec -- prek run --all-files` — exit 0
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — exit 0・0 findings
- **expected**: 8 コマンドすべて実実行・log 保存。full gate は plan 最終段で 1 回のみ。
