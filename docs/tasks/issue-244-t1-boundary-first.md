---
id: "TASK-034Qxfh3HNkNZecEqGSpM1"
type: "task"
status: "done"
title: "T1 prefetch boundary-first fault + dead-generation sequence (Issue #244 P2 amendment)"
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
    - "docs/plans/issue-244-media-reader-prefetch-warts.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-244-media-reader-prefetch-warts.md"
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
    - "docs/test-specs/issue-244-media-reader-prefetch-warts.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 prefetch boundary-first fault + dead-generation sequence (Issue #244 P2 amendment)

## Work

<!-- Describe the implementation slice. -->

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->

- Boundary-first emission on rejected `seek`/`seek_discard_pending` and post-`reopen_at` seek failure; `Boundary` → `Fault` → `End` on `open`/`reader.reopen` failure; contract text updates (module Contract, rustdoc for `seek`/`seek_discard_pending`/`reopen`/`reopen_at`, README).

## Implementation

- `src/prefetch.rs` `exec_cmd`: swap `Fault`/`Boundary` push order in the `Seek` rejection arm and the `Reopen` `Some(first)` seek-failure arm; push `Boundary{gen, 0}` before `Fault` in the `Reopen` `Err` arm.
- `tests/prefetch.rs`: TDD — seeded seek-failure `faults.unseekable`/`fault_events` test; updated `rd02_reopen_at_seek_failure_faults_then_reanchors`; reopen-failure adopt+`eof` test.

## Acceptance

- Fault counted + surfaced (never stale-dropped); reopen failure adopts the generation and reaches `eof`; doc text matches amended semantics.
