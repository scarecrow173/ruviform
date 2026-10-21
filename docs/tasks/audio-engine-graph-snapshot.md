---
id: "TASK-034JRQBhUGz8OWIgGAnEgl"
type: "task"
status: "todo"
title: "ruviform-audio クレート新設 + pull グラフ/ミキサー/プランスナップショット"
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

# ruviform-audio クレート新設 + pull グラフ/ミキサー/プランスナップショット

## Work

- `crates/ruviform-audio` 新設 + Cargo.toml 依存
- `graph.rs`/`mixer.rs`/`plan.rs` 実装
- lib.rs: `pub mod` + crate doc

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [ ] 実装がコンパイルし公開 API が doc 済み
- [ ] 対応テストがグリーン

## Verification

- [ ] `mise run check`/`lint`/`fmt-check`/`test` パス
