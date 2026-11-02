---
id: "TASK-ubO5TxPGQDZSaLVHth3aaa"
type: "task"
status: "done"
title: "Issue 198 T3: OF-01..OF-07 acceptance tests + verification + merge + impl record"
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
    - "docs/tasks/task-198-t2-driver-output.md"
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

# Issue 198 T3: tests + verification

## Work

OF-01..OF-06 integration tests in `crates/ruviform-audio/tests/render.rs` (new fixtures: `OnePole`, `SignalFeeder`, `Captured`/`CaptureEncoder`/`NullMuxer`, `FailSink`, `verify_wav`; `MockDsp` gained `with_support`). OF-07: run `cargo test -p ruviform-audio --locked` / `-p ruviform-exec --locked` / `cargo test --workspace --locked` + `mise run fmt-check/check/lint/test/test-rt/verify-ci`; merge `origin/main`; regenerate `Cargo.lock`; commit; write implementation record.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-198-sequential-offline-render.md`
- `docs/test-specs/issue-198-sequential-offline-render.md`

## Dependencies

- `task-198-t2-driver-output.md`

## Affected Files

- `crates/ruviform-audio/tests/typed_render.rs`, `crates/ruviform-audio/tests/common/mod.rs`
- `docs/impl/ir/issue-198-sequential-offline-render-implementation-record.md`
- `Cargo.lock` (regenerated post-merge)

## Tests

- The acceptance suite itself.

## Done

- All gates green, merge clean, commits made, impl record written, exit-audit passes.
