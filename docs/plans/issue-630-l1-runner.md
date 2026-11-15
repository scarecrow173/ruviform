---
id: "PLAN-034XVsD1tGn1Q9zdYY4pHW"
type: "plan"
status: "approved"
title: "Issue #630 L1 runner slice 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/tasks/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/test-specs/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-l1-runner.jsonl"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-630-l1-runner.md"
  implemented-by:
    - "docs/tasks/issue-630-l1-runner.md"
  depends-on:
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/designs/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-l1-runner.md"
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "docs/plans/issue-630-clap-test-plugin.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/designs/issue-630-l1-runner.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-l1-runner.md"
  verified-by:
    - "docs/test-specs/issue-630-l1-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 L1 runner slice 実装計画 Implementation Plan

## Goal

Deliver the L1 out-of-process path for both Issue #630 test-plugin
fixtures: per-crate `l1-runner` feature + runner bin +
`#[ignore]`d e2e suite in `ruviform-plugin-test-vst3` and
`ruviform-plugin-test-clap`, wired into the existing
`test-l1-real-plugin` / `check-l1-real-plugin` /
`lint-l1-real-plugin` mise lanes — zero production-crate changes.

## Scope

Single implementation stream — two symmetric crate touch-points plus
one shared `mise.toml` lane edit. The VST3 and CLAP halves are
mechanically identical modulo format names, descriptor pins, and the
gain coefficient; they land in one commit series.

## Upstream Documents

- `docs/specs/issue-630-l1-runner.md`
- `docs/designs/issue-630-l1-runner.md`
- `docs/designs/issue-630-test-plugin-suite.md` (suite conventions)
- `docs/designs/issue-114-real-adapter-l1-e2e.md` (L1 substrate)
- `docs/test-specs/real-adapter-l1-e2e-tests.md` (adapter e2e oracle)

## Design Inputs

- `docs/designs/issue-630-l1-runner.md`

## File Map

- `crates/ruviform-plugin-test-vst3/Cargo.toml` — `l1-runner` feature,
  optional `ruviform-plugin-vst3`(host)+`ruviform-plugin-export` deps,
  `[[bin]]`, dev-dep `ruviform-plugin-host`.
- `crates/ruviform-plugin-test-vst3/src/bin/ruviform-plugin-test-vst3-l1-runner.rs`
  — runner bin mirroring the adapter runner; local
  `fixture_lib_name()` → `ruviform_plugin_test_vst3`.
- `crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs`
  — `#![cfg(feature = "l1-runner")]` + 3 `#[ignore]`d tests; VST3
  descriptor pin via `vst3::uid` CID derivation.
- `crates/ruviform-plugin-test-clap/Cargo.toml` — same shape with
  `ruviform-plugin-clap`.
- `crates/ruviform-plugin-test-clap/src/bin/ruviform-plugin-test-clap-l1-runner.rs`
  — runner bin; local `fixture_lib_name()` →
  `ruviform_plugin_test_clap`.
- `crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs`
  — same suite; literal CLAP descriptor pin.
- `mise.toml` — extend `test-l1-real-plugin` (build + `--lib --bins` +
  e2e lines), `check-l1-real-plugin`, `lint-l1-real-plugin` (second
  cargo invocation per feature-name group).
- `Cargo.lock` — regenerated for the new path-dep edges.
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-l1-runner.*`
  — this chain.
- `evidence/rf630/l1-runner-verification.md` — run evidence +
  NotRun/Unsupported ledger.

## Implementation Sequence

- [ ] `crates/ruviform-plugin-test-vst3/Cargo.toml` + runner bin +
  e2e test (VST3 half).
- [ ] `crates/ruviform-plugin-test-clap/Cargo.toml` + runner bin +
  e2e test (CLAP half).
- [ ] `mise.toml` lane extension; `Cargo.lock` regeneration.
- [ ] `mise run fmt-check` → `check`/`lint`/`test` both crates →
  `test-l1-real-plugin` (real spawn) → `test-rt` → `prek`.
- [ ] Doc chain completion (task/test-spec/ir/exp) + evidence +
  docs index regeneration.
- [ ] `git fetch origin main && git merge origin/main`, regenerate
  generated files, final verification, commit, push, PR (`Refs #630`).

## Dependencies and Risks

- Depends on merged PRs #641 (VST3 fixture) and #657 (CLAP fixture) —
  both on main (`77070a0`).
- Risk: `--features` cannot span `host`/`l1-runner` in one cargo call →
  check/lint lanes use two invocations (design decision recorded).
- Risk: cdylib uplift location differs per profile → lane always runs
  `cargo build` (dev profile) before `cargo test`; e2e resolves the
  runner exe's own directory, same as the adapter lane.
- Risk: `env!("CARGO_BIN_EXE_*")` undefined → `#![cfg(feature)]`
  guarantees the macro never expands without `l1-runner`.

## Task Breakdown

- T1: VST3 half — manifest, runner bin, e2e suite.
- T2: CLAP half — manifest, runner bin, e2e suite.
- T3: Lane wiring (`mise.toml`) + `Cargo.lock` + verification +
  evidence/docs.

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-test-vst3` /
  `mise run check ruviform-plugin-test-clap`
- [ ] `mise run lint ruviform-plugin-test-vst3` /
  `mise run lint ruviform-plugin-test-clap`
- [ ] `mise run test ruviform-plugin-test-vst3` /
  `mise run test ruviform-plugin-test-clap`
- [ ] `mise run test-rt`
- [ ] `mise run test-l1-real-plugin` (new lines actually execute —
  real binary spawn)
- [ ] `mise exec -- prek run --all-files`
- [ ] `git diff -- crates/ruviform-plugin-vst3/src crates/ruviform-plugin-clap/src .github` empty

## Open Questions / Gaps

- None blocking. Non-Windows literal pin for the VST3 CID is derived
  by the same verified `uid()` transform; Windows literal is
  runtime-verified by this lane.

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution (dispatched worker slice).
- Independent work streams: T1 and T2 are symmetric but share
  `mise.toml`; executed sequentially in one worktree.
- User approval required before dispatch: yes (worker contract —
  review-protocol substitutes for approval gates).

## Review Handoff

- Reviewer focus: Cargo feature/dep wiring correctness (optional dep +
  dev-dep union, `required-features` gating), descriptor pin
  independence (literal oracle vs child scan), lane coverage parity
  with the adapter lane, zero-diff guarantee on production sources.
