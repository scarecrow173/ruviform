---
id: "TASK-034QxfhGvckBvhC0QQYCgA"
type: "task"
status: "done"
title: "T2 prefetch End commit-once under backpressure (Issue #244)"
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

# T2 prefetch End commit-once under backpressure (Issue #244)

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

- `end_emitted` commit-once: set when the `End` marker is committed at synthesis, so a ring-full requeued `End` is never synthesized twice.

## Implementation

- `src/prefetch.rs` `pump` step 2: set `self.end_emitted = true` before `try_push`.
- `tests/prefetch.rs`: TDD — `block_count:1` ring saturated by four failed reopens; after drain the next `pump` returns `Idle` (no duplicate `End`).

## Acceptance

- Exactly one `End` per generation under backpressure; `Pending`→`Idle` pump sequence verified.
