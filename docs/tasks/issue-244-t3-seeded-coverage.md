---
id: "TASK-034Qxfiiwcs7VUi4SIArot"
type: "task"
status: "done"
title: "T3 seeded fault fixture + discard_pending coverage (Issue #244)"
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

# T3 seeded fault fixture + discard_pending coverage (Issue #244)

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

- `SeededSource` fault-injection fixture (flag-gated `Read`/`Seek`) and the `discard_pending` matching-gen `Fault` arm + `End`-then-`Boundary` drain coverage.

## Implementation

- `tests/prefetch.rs`: `SeededSource` fixture (armed after construction; probe runs unarmed); decode-failure drain test asserting the counted `Fault`, `ended` on `End`, then `boundary: Some` clearing.

## Acceptance

- `discard_pending` fault arm + `End`-then-`Boundary` covered deterministically; no `stale_drops` artifacts.
