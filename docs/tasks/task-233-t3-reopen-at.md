---
id: "TASK-034QooAu3EbKEEhgcUSJnv"
type: "task"
status: "done"
title: "Issue 233 T3: reopen_at fused reopen-to-seek-target"
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
    - "docs/tasks/task-233-t2-discard-pending.md"
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

# Issue 233 T3: reopen_at fused reopen-to-seek-target

## Work

Feature candidate 4 (TDD — failing contract tests first). In `crates/ruviform-media-reader/src/prefetch.rs`: (a) `PrefetchCmd::Reopen` gains private `first: Option<u64>`; `PrefetchHandle::reopen` delegates through the shared send path with `first: None` (byte-identical behavior); (b) new `PrefetchHandle::reopen_at(source, open, first: SourceFrame)` sends `first: Some(first.0)`; (c) `exec_cmd` `Reopen` success + `Some(f)` → `reader.seek_source_frame(SourceFrame(f))` → `Boundary{first_frame: reader.position().0}` (honest landed); post-reopen seek failure → `Fault{Unseekable, f, 0}` + `Boundary{first_frame: reader.position().0}` (reopened position); reopen failure keeps `Fault + stream_ended`, no Boundary. README API entry. Tests: `reopen_at` lands at post-seek position through one Boundary; seek-failure path; `reopen` regression (frame-0 anchor unchanged).

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-233-media-reader-prefetch-improvements.md`
- `docs/specs/issue-233-media-reader-prefetch-contract-improvements.md` (AC6)
- `docs/designs/issue-233-media-reader-prefetch-improvements.md` (command field, failure shape)

## Dependencies

- `docs/tasks/task-233-t2-discard-pending.md`

## Affected Files

- `crates/ruviform-media-reader/src/prefetch.rs`
- `crates/ruviform-media-reader/README.md`
- `crates/ruviform-media-reader/tests/prefetch.rs`

## Done When

- [x] `reopen_at` emits exactly one `Boundary` reporting `reader.position()` after the fused seek.
- [x] Post-reopen seek failure → `Fault{Unseekable}` + `Boundary` at reopened position.
- [x] `reopen` keeps frame-0 anchor and signature; regression test unmodified.
- [x] All tests green.

## Verification

- [x] `mise run test`
- [x] `mise run lint`
- [x] `mise run check`
- [x] `mise run fmt-check`
