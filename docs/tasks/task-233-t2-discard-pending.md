---
id: "TASK-034QoozSINn40AO2e1Ke1k"
type: "task"
status: "todo"
title: "Issue 233 T2: discard surface — PrefetchConsumer::discard_pending + DiscardReport + seek_discard_pending pending purge"
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
    - "docs/plans/issue-233-media-reader-prefetch-improvements.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-233-media-reader-prefetch-improvements.md"
    - "docs/tasks/task-233-t1-contract-docs.md"
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
    - "docs/test-specs/issue-233-media-reader-prefetch-improvements.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 233 T2: discard surface — PrefetchConsumer::discard_pending + DiscardReport + seek_discard_pending pending purge

## Work

Feature candidate 2 (TDD — failing contract tests first). In `crates/ruviform-media-reader/src/prefetch.rs`: (a) `PrefetchCmd::Seek` gains private `discard_pending: bool`; `exec_cmd` runs `self.pending.clear()` before enqueueing `Boundary` when set; (b) `PrefetchHandle::seek_discard_pending(first)` — same contract as `seek` (`Fact::Unsupported` → immediate `ErrorKind::Unsupported`, no generation consumed) sending `discard_pending: true`; `seek` sends `false` through a shared send path; (c) `PrefetchConsumer::discard_pending(&RtContext) -> DiscardReport` — bounded drain: return the in-flight `current` block to the free ring, pop the data ring applying `step()`'s marker rules (Boundary adopts generation + `next_frame` + clears `fault_remaining`/`ended`, Fault/End state transitions, blocks → `stale()` + `return_block`), stop at the first `Boundary` or ring-empty, report `{items, blocks, boundary: Option<u64>}`; (d) new public `#[non_exhaustive]` `DiscardReport`; (e) `lib.rs` re-export; (f) README API entry + contract paragraph. Tests: drain adopts Boundary and serves no pre-boundary content; `boundary: None` when none arrived; stale-marker suppression under `seek_discard_pending`; default `seek` keeps play-through; `rt_`-prefixed `assert_no_alloc` bounded test in `tests/rt_fill.rs`.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-233-media-reader-prefetch-improvements.md`
- `docs/specs/issue-233-media-reader-prefetch-contract-improvements.md` (AC2, AC3, AC4)
- `docs/designs/issue-233-media-reader-prefetch-improvements.md` (drain semantics, command field, naming)

## Dependencies

- `docs/tasks/task-233-t1-contract-docs.md`

## Affected Files

- `crates/ruviform-media-reader/src/prefetch.rs`
- `crates/ruviform-media-reader/src/lib.rs`
- `crates/ruviform-media-reader/README.md`
- `crates/ruviform-media-reader/tests/prefetch.rs`
- `crates/ruviform-media-reader/tests/rt_fill.rs`

## Done When

- [ ] `discard_pending` drain test: backlog + Boundary → `boundary: Some(landed)`, position re-anchored, blocks returned to free ring, no pre-boundary content served by `fill`.
- [ ] `boundary: None` path and stop-at-Boundary semantics tested.
- [ ] `seek_discard_pending` purges producer `pending`; stale Fault/End markers never emit; `seek` default unchanged.
- [ ] `rt_` test proves zero allocation and bounded pops under `assert_no_alloc`.
- [ ] Existing tests unmodified and green.

## Verification

- [ ] `mise run test`
- [ ] `mise run lint`
- [ ] `mise run check`
- [ ] `mise run fmt-check`
- [ ] `mise run test-rt`
