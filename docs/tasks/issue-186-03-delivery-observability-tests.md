---
id: "TASK-034QkFAipKgHjxJRSBAYXt"
type: "task"
status: "done"
title: "tests: delivery E2E + rejection + time conversion + observability"
created: "2026-11-02"
updated: "2026-11-02"
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
    - "docs/plans/issue-186-param-control-events.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-186-param-control-events.md"
    - "docs/tasks/issue-186-01-plugin-core-param-write.md"
    - "docs/tasks/issue-186-02-bridge-target-dispatch.md"
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
    - "docs/test-specs/issue-186-param-control-events.md"
  references: []
  defers: []
  deferred-by: []
---

# tests: delivery E2E + rejection + time conversion + observability

## Work

`crates/ruviform-audio-plugin-bridge/tests/param_delivery.rs` を新設し、(a) 複数時刻・複数対象の ParamEvent が期待順序・frame offset・値で L0 in-process recording plugin に届く E2E（`PluginNode`+`install_plan`+`process`）、(b) defined rejection（READONLY/PerNote/Immediate/未解決 Target/非 Processing）、(c) beat→frame 変換・Loop occurrence・tempo 変更・block 境界分割・非同期 install_plan（control channel 経由）、(d) dispatch ledger の Scheduled/Dispatched/Dropped/Unavailable 区別の各テストを実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-186-param-control-events.md`
- `docs/test-specs/issue-186-param-control-events.md`

## Dependencies

- task-01, task-02

## Affected Files

- `crates/ruviform-audio-plugin-bridge/tests/param_delivery.rs`（新設）

## Done When

- [ ] AC-3/AC-4/AC-5/AC-7 のテストが全て存在し緑
- [ ] 複数時刻×複数 param の順序/位置/値が厳密に検証される

## Verification

- [ ] `mise run test` green
