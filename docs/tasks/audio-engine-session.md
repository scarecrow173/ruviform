---
id: "TASK-034JRQBPxKgu0yLVIB6Xm7"
type: "task"
status: "done"
title: "セッション統合（OutputStream callback → RT グラフ駆動）"
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
    - "docs/plans/implement-issue-25-audio-engine.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-25-audio-engine.md"
    - "docs/tasks/audio-engine-graph-snapshot.md"
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
    - "docs/test-specs/audio-engine-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# セッション統合（OutputStream callback → RT グラフ駆動）

## Work

- `session.rs`: `Session::open_output` + callback 配線
- `RtContext::mint` 呼出し・`PlanHandle` 参照・RT 契約テスト

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [x] 実装がコンパイルし公開 API が doc 済み
- [x] 対応テストがグリーン

## Verification

- [x] `mise run check`/`lint`/`fmt-check`/`test` パス
