---
id: "TASK-K54oAzr1E0TFOYRxcMd30t"
type: "task"
status: "done"
title: "Issue 198 T2: sequential driver + staged output lifecycle"
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
    - "docs/tasks/task-198-t1-render-core.md"
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

# Issue 198 T2: sequential driver + output lifecycle

## Work

`render()` driver: `RenderTransport` command staging with `preflight`, chunked `run_callback`, span→segment→physical partitioning (event cuts + `BlockContract`), `InputFeeder`, emit algebra (`pos − total_latency` vs range+tail), `RenderOutput` (staged sink → muxer → drain → finalize → verify → publish), cancel points, typed `RenderError`/`RenderReceipt`.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-198-sequential-offline-render.md`
- `docs/designs/issue-198-sequential-offline-render.md`

## Dependencies

- `task-198-t1-render-core.md`

## Affected Files

- `crates/ruviform-audio/src/render.rs`

## Tests

- Drive-loop unit coverage via T3 integration tests.

## Done

- Driver renders a full transport program into a staged WAV; all failure paths typed; temp cleanup on every error.
