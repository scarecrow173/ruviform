---
id: "TASK-eM0HvOCV03pTzfSqyt3jgm"
type: "task"
status: "done"
title: "Issue 198 T1: render core — OfflineRenderSupport, PreparedRender, Progress::bump_by, deps"
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
    - "docs/plans/implement-issue-198-sequential-offline-render.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-198-sequential-offline-render.md"
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
    - "docs/test-specs/issue-198-sequential-offline-render.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 198 T1: render core

## Work

Add `ruviform_audio::render` module skeleton: `OfflineRenderSupport` tri-state, additive defaulted `GraphProcessor::offline_support()`, `PreparedRender` (compile → `GraphRuntime`+`RtRuntime`+`Arc<PreparedPlan>`+capability map), `Progress::bump_by(u64)` in `ruviform-exec`, and the `ruviform-audio → ruviform-media-io`/`ruviform-exec` dependency edges (+ dev-deps).

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-198-sequential-offline-render.md`
- `docs/designs/issue-198-sequential-offline-render.md`
- `docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md`

## Dependencies

- none (first in chain)

## Affected Files

- `crates/ruviform-audio/src/render.rs` (new)
- `crates/ruviform-audio/src/lib.rs`, `crates/ruviform-audio/src/typed/runtime.rs`
- `crates/ruviform-audio/Cargo.toml`, `crates/ruviform-exec/src/offline.rs` (or progress site)
- `Cargo.lock` (generated)

## Tests

- Smoke compile test; acceptance in T3.

## Done

- `mise run check` + `lint` green; `PreparedRender::compile` produces an isolated runtime.
