---
id: "TASK-034XVtnIJFkGrODb3uToEC"
type: "task"
status: "done"
title: "Issue #630 — test plugin L1 out-of-process runner slice"
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
        path: "crates/ruviform-plugin-test-vst3/src/bin/ruviform-plugin-test-vst3-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/bin/ruviform-plugin-test-clap-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "docs/tasks/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-l1-runner.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-630-l1-runner.md"
  implemented-by:
    - "docs/impl/ir/issue-630-l1-runner.md"
  depends-on:
    - "docs/plans/issue-630-l1-runner.md"
    - "docs/tasks/issue-630-test-plugin-suite.md"
    - "docs/tasks/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-l1-runner.md"
    - "docs/tasks/real-adapter-runner-bins.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-l1-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 — test plugin L1 out-of-process runner slice

## Work

Add the L1 out-of-process execution path to both Issue #630 test-plugin
crates: `l1-runner` feature + `[[bin]]` runner +
`tests/l1_real_test_plugin_e2e.rs` in `ruviform-plugin-test-vst3` and
`ruviform-plugin-test-clap`, mirroring the adapter runner/e2e shape
while resolving each crate's own cdylib; extend the
`test-l1-real-plugin` / `check-l1-real-plugin` /
`lint-l1-real-plugin` mise lanes to cover the test crates.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-l1-runner.md`
- `docs/specs/issue-630-l1-runner.md`
- `docs/designs/issue-630-l1-runner.md`
- `docs/test-specs/issue-630-l1-runner.md`

## Dependencies

- `docs/tasks/issue-630-test-plugin-suite.md` (VST3 fixture — merged,
  PR #641) and `docs/tasks/issue-630-clap-test-plugin.md` (CLAP fixture
  — merged, PR #657): both fixtures exist on main; no blocking
  dependency remains.
- Issue #114 adapter chain (`docs/tasks/real-adapter-runner-bins.md`,
  `docs/tasks/real-adapter-e2e-and-lane.md`) supplies the runner/e2e
  precedent being mirrored.

## Affected Files

- `crates/ruviform-plugin-test-vst3/{Cargo.toml,src/bin/ruviform-plugin-test-vst3-l1-runner.rs,tests/l1_real_test_plugin_e2e.rs}`
- `crates/ruviform-plugin-test-clap/{Cargo.toml,src/bin/ruviform-plugin-test-clap-l1-runner.rs,tests/l1_real_test_plugin_e2e.rs}`
- `mise.toml` (`test-l1-real-plugin`, `check-l1-real-plugin`,
  `lint-l1-real-plugin`)
- `Cargo.lock` (regenerated)
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-l1-runner.*`
- `evidence/rf630/l1-runner-verification.md`

## Done When

- [ ] Both test crates expose `l1-runner` feature + runner bin
      (`required-features`) + feature-gated e2e suite; default builds
      and default `cargo test` unaffected.
- [ ] `mise run test-l1-real-plugin` builds test cdylibs + runner bins
      and passes both `#[ignore]`d e2e suites on Windows (real spawn).
- [ ] `mise run check`/`lint`/`test`/`fmt-check`/`test-rt`/`prek` all
      pass for the touched scope.
- [ ] Zero diff under `crates/ruviform-plugin-{vst3,clap}/src/**`,
      `.github/**`, `POLICIES.md`, `AGENTS.md`; dependency DAG acyclic.
- [ ] Doc chain + evidence records written; review-protocol clean.

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check ruviform-plugin-test-vst3` /
      `check ruviform-plugin-test-clap`
- [ ] `mise run lint ruviform-plugin-test-vst3` /
      `lint ruviform-plugin-test-clap`
- [ ] `mise run test ruviform-plugin-test-vst3` /
      `test ruviform-plugin-test-clap`
- [ ] `mise run test-rt`
- [ ] `mise run test-l1-real-plugin`
- [ ] `mise exec -- prek run --all-files`
