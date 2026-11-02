---
id: "TASK-034QkFB3oVWTfZxG7hGYMc"
type: "task"
status: "done"
title: "rt lane: bounded dispatch no-alloc + plan vault keep/retire + verification"
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
    - "docs/tasks/issue-186-02-bridge-target-dispatch.md"
    - "docs/tasks/issue-186-03-delivery-observability-tests.md"
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

# rt lane: bounded dispatch no-alloc + plan vault keep/retire + verification

## Work

RT lane テストを追加する: param 配送経路（`PluginNode::process` の EventList 構築〜endpoint 呼出）が `assert_no_alloc` 測定窓で allocation-free であること、`PlanVault` が旧 plan を callback 中 keep-alive し非RT で retire できること、event scratch/budget 超過が defined fault（panic/UB ではなく report+counter）になることを検証する。全 mise 検証を緑化する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-186-param-control-events.md`
- `docs/test-specs/issue-186-param-control-events.md`

## Dependencies

- task-02, task-03

## Affected Files

- `crates/ruviform-audio-plugin-bridge/tests/`（rt_ テスト）

## Done When

- [ ] `rt_` テストが param 配送・vault keep/retire・有界 fault を検証する
- [ ] `mise run test-rt` / `verify-ci` green

## Verification

- [ ] `mise run test-rt` / `test` / `lint` / `check` / `fmt-check` / `verify-ci` 全 green
