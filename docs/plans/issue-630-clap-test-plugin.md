---
id: "PLAN-034XTBbXfYIHXWJ12juqo7"
type: "plan"
status: "approved"
title: "Issue #630 CLAP test_gain 実装計画"
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
        path: "docs/plans/issue-630-clap-test-plugin.md"
      - type: "file"
        path: "docs/tasks/issue-630-clap-test-plugin.md"
      - type: "file"
        path: "docs/test-specs/issue-630-clap-test-plugin.md"
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
    - "docs/specs/issue-630-clap-test-plugin.md"
  implemented-by:
    - "docs/tasks/issue-630-clap-test-plugin.md"
  depends-on:
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/designs/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-clap-test-plugin.md"
    - "docs/plans/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/designs/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP test_gain 実装計画 Implementation Plan

## Goal

Deliver the test-only CLAP `test_gain` fixture crate
`ruviform-plugin-test-clap`, verified through the production
`ClapLibrary` host path, per the CLAP-slice spec and design.

## Scope

Single implementation stream — one crate, one plugin, one host-path
integration test. No split needed.

## Upstream Documents

- `docs/specs/issue-630-clap-test-plugin.md`
- `docs/designs/issue-630-clap-test-plugin.md`
- `docs/designs/issue-630-test-plugin-suite.md` (suite conventions)

## Design Inputs

- `docs/designs/issue-630-clap-test-plugin.md`

## File Map

- `crates/ruviform-plugin-test-clap/Cargo.toml` — crate manifest:
  `cdylib`+`rlib`, `clack-* =0.2.0`, neutral deps, dev-deps
  (`assert_no_alloc`, `clack-host`, `ruviform-plugin-clap` feature
  `host`).
- `crates/ruviform-plugin-test-clap/src/lib.rs` — test-only docs +
  `clack_export_entry!(SinglePluginEntry)`.
- `crates/ruviform-plugin-test-clap/src/gain.rs` — independent
  `TestGainProcessor` (neutral `PluginProcessor`, DSP + param law).
- `crates/ruviform-plugin-test-clap/src/block.rs` — CLAP↔neutral
  translation, zero-copy views, scratch, atomic lifecycle/param mirror.
- `crates/ruviform-plugin-test-clap/src/plugin.rs` — clack trait impls +
  `#[cfg(test)]` unit/lifecycle/process/ABI/RT tests.
- `crates/ruviform-plugin-test-clap/tests/host_path.rs` — production
  `ClapLibrary` path integration tests.
- `crates/ruviform-plugin-test-clap/README.md` — test-only declaration,
  ABI shape, semantics, independence, RT contract, scope.
- `Cargo.toml` — one `members` line; `Cargo.lock` regenerated;
  `crates/README.md` index row.
- `docs/**` chain + `evidence/rf630/` records for this slice.

## Dependency Graph

`Cargo.toml`/`lib.rs` → `gain.rs`/`block.rs` → `plugin.rs` →
`tests/host_path.rs` → workspace integration → doc chain → evidence →
verification (`fmt-check`, `check`, `lint`, `test`, `build`,
`test-rt`, `prek`) → review-protocol → PR.

## Risk Register

- clack API surface mismatch → compile errors surface immediately;
  precedent is the adapter's `plugin.rs`/`host.rs` on the same pin.
- Host-path fixture resolution → `current_exe`→`deps/` precedent from
  `host.rs`/`test-vst3`; missing binary assert-fails.
- RT allocation regressions → `assert_no_alloc` gate via `rt_` name.

## Verification Matrix

| Check | Command | Gate |
| --- | --- | --- |
| fmt | `mise run fmt-check` | required |
| check | `mise run check ruviform-plugin-test-clap` | required |
| lint | `mise run lint ruviform-plugin-test-clap` (`-D warnings`) | required |
| unit+integration | `mise run test ruviform-plugin-test-clap` | required |
| cdylib build | `mise run build ruviform-plugin-test-clap` | required |
| RT lane | `mise run test-rt` (`rt_` collected) | required |
| hooks | `mise exec -- prek run --all-files` | required |

## Rollback

Single-commit revert of the crate + workspace line + docs; no shared
state mutated.
