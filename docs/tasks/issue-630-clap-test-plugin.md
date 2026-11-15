---
id: "TASK-034XTBfPlK12MmvsP1OkhW"
type: "task"
status: "completed"
title: "Issue #630 T2 — CLAP test_gain plugin（ruviform-plugin-test-clap）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-clap-test-plugin.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-clap-test-plugin.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-clap-test-plugin.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-test-plugin.jsonl"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-630-clap-test-plugin.md"
  implemented-by:
    - "docs/impl/ir/issue-630-clap-test-plugin.md"
  depends-on:
    - "docs/plans/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-clap-test-plugin.md"
    - "docs/tasks/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-clap-test-plugin.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 T2 — CLAP test_gain plugin（ruviform-plugin-test-clap）

## Work

Create the test-only crate `crates/ruviform-plugin-test-clap`: a real
CLAP cdylib fixture `test_gain` on `clack-* =0.2.0`, independently
implemented against the neutral contracts, verified through the
production `ClapLibrary` host path (L0 in-process), with unit,
lifecycle, process-law, ABI, host-path, and `rt_` no-allocation tests.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-clap-test-plugin.md`
- `docs/specs/issue-630-clap-test-plugin.md`
- `docs/designs/issue-630-clap-test-plugin.md`

## Dependencies

- `docs/tasks/issue-630-test-plugin-suite.md` (T1 VST3 slice — merged
  precedent; no blocking dependency remains).

## Affected Files

- `crates/ruviform-plugin-test-clap/**` (new)
- `Cargo.toml` (`members` +1 line), `Cargo.lock` (regenerated),
  `crates/README.md` (index row)
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-clap-test-plugin.*`
- `evidence/rf630/clap-test-gain-verification.md`

## Done When

- [x] Crate builds `cdylib`+`rlib`; `clap_entry` exports one
      `test_gain` descriptor (`org.ruviform-test.clap-test-gain`,
      vendor `"Ruviform Test"`).
- [x] `mise run test ruviform-plugin-test-clap` passes (unit +
      host-path through production `ClapLibrary`).
- [x] `mise run test-rt` collects and passes
      `rt_process_drives_paired_stereo_block_allocation_free`.
- [x] fmt/check/lint/build/prek all pass for the crate.
- [x] Doc chain + evidence records written; review-protocol clean.
