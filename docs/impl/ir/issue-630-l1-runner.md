---
id: "IMPL-034XVFtnDImIcYkvdrAeSl"
type: "impl"
status: "completed"
title: "Issue #630 test-plugin L1 runner — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/tasks/issue-630-l1-runner.md"
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
        path: "docs/impl/ir/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-l1-runner.jsonl"
      - type: "file"
        path: "evidence/rf630/l1-runner-verification.md"
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
    - "docs/tasks/issue-630-l1-runner.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-l1-runner.md"
    - "docs/plans/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-630-l1-runner.jsonl"
    - "docs/impl/ir/issue-630-test-plugin-suite.md"
    - "docs/impl/ir/issue-630-clap-test-plugin.md"
    - "docs/test-specs/issue-630-l1-runner.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-l1-runner.jsonl"
    rejected: []
---

# Issue #630 test-plugin L1 runner — 実装記録

## Summary

Extended both Issue #630 test-plugin fixtures
(`crates/ruviform-plugin-test-vst3`,
`crates/ruviform-plugin-test-clap`) with the L1 out-of-process
execution path: an `l1-runner` cargo feature, a per-crate runner
binary, and a feature-gated `#[ignore]`d E2E suite that drives the
production `PluginHost`/`OutOfProcessSpec` wire path against each
crate's own real cdylib. The `test-l1-real-plugin` mise lane now
builds both fixture cdylibs + runner bins and runs both new E2E
suites alongside the existing adapter suites. Production adapter
sources are zero-diff.

## Context

Follows `docs/tasks/issue-630-l1-runner.md` →
`docs/plans/issue-630-l1-runner.md` →
`docs/specs|designs/issue-630-l1-runner.md`, verified against
`docs/test-specs/issue-630-l1-runner.md`. Precedent:
`docs/tasks/real-adapter-runner-bins.md` /
`docs/tasks/real-adapter-e2e-and-lane.md` (Issue #114 adapter chain).
Prior slices: `docs/impl/ir/issue-630-test-plugin-suite.md` (VST3
fixture) and `docs/impl/ir/issue-630-clap-test-plugin.md` (CLAP
fixture).

## Implementation

### Changes

- `crates/ruviform-plugin-test-vst3/Cargo.toml` +
  `crates/ruviform-plugin-test-clap/Cargo.toml` — `[features]
  l1-runner = ["dep:<adapter>", "dep:ruviform-plugin-export"]`;
  the adapter crate moved to an optional regular dep (`features =
  ["host"]`, unioned with the existing dev-dep edge — the
  `clack-host` precedent); `ruviform-plugin-export` added optional;
  `ruviform-plugin-host` added as dev-dep for the E2E;
  `[[bin]] required-features = ["l1-runner"]`.
- `src/bin/ruviform-plugin-test-{vst3,clap}-l1-runner.rs` — mirror of
  the adapter runners: resolve
  `{DLL_PREFIX}ruviform_plugin_test_{vst3,clap}{DLL_SUFFIX}` via
  `RUVIFORM_L1_REAL_PLUGIN_DIR` → exe-dir; `Vst3Library::load` /
  `ClapLibrary::load` → `factories()` →
  `ruviform_plugin_export::run`; resolution/load failure = stderr +
  exit 1 pre-`Hello`. Local `fixture_lib_name()` (the adapter's is
  compile-time fixed to the adapter fixture). Two `#[cfg(test)]`
  fixture-resolution unit tests per bin.
- `tests/l1_real_test_plugin_e2e.rs` (both crates) —
  `#![cfg(feature = "l1-runner")]` + three `#[ignore]`d tests:
  `real_test_plugin_l1_full_series` (Scan → Instantiate handshake →
  lifecycle → 32 stereo shm blocks with `ParamValue(0.25)` →
  delivered-block `0.5×` membership + defined silence/underrun →
  `Err(Unsupported)` SaveState/RestoreState/OpenGui/CloseGui →
  clean teardown), `real_test_plugin_l1_kill_reports_failed_once`
  (kill → Failed + exactly one `InstanceFailed` + `Err(Failed)` +
  silence + no restart + Destroy), and
  `real_test_plugin_l1_missing_runner_is_spawn_failed`
  (`SpawnFailed{cause: Spawn}`, registration survives). Descriptor
  oracle: VST3 derives `expected_id()` at test time from the literal
  `vst3::uid(0x6D4F_8A21, 0x3B7C_9E05, 0xF1A2_4C68, 0xB9D3_7E51)`
  construction (`TestGainComponent::CID` is `pub(crate)`), with
  platform byte-order literal pins; CLAP pins the literal
  `org.ruviform-test.clap-test-gain` descriptor.
- `mise.toml` — `test-l1-real-plugin` gains test-crate build +
  `--lib --bins` + `--test l1_real_test_plugin_e2e -- --ignored`
  steps; `check-l1-real-plugin`/`lint-l1-real-plugin` cover all four
  crates with package-qualified features
  (`ruviform-plugin-{clap,vst3}/host`,
  `ruviform-plugin-test-{vst3,clap}/l1-runner`).
- `Cargo.lock` regenerated by cargo during `cargo check`.
- Doc chain (`docs/{specs,designs,plans,tasks,test-specs}/
  issue-630-l1-runner.md`), this record,
  `docs/impl/exp/issue-630-l1-runner.jsonl`,
  `evidence/rf630/l1-runner-verification.md`, managed doc indexes.

### Decisions During Implementation

- Runner lives inside each test crate rather than redirecting the
  adapter runners: `fixture_lib_name()` is compile-time fixed to the
  adapter fixture and `OutOfProcessSpec` carries no env/argv
  extension, so per-crate runners are the clean boundary — production
  adapter sources stay zero-diff.
- `l1-runner` as a single feature covering both the runner bin and
  the e2e surface keeps default builds/tests untouched; the e2e
  file's `#![cfg]` means `env!("CARGO_BIN_EXE_*")` is never expanded
  without the feature (verified: default `cargo test` runs the e2e
  target with 0 tests).
- `check`/`lint` lane commands use package-qualified
  `--features pkg/feat` syntax because the four selected crates do
  not share one feature name (`host` vs `l1-runner`).
- VST3 `expected_id()` derives from `vst3::uid(...)` at test time —
  the same vendored deterministic `const fn` the fixture CID uses —
  keeping the oracle independent of the child's real-binary scan;
  the platform byte-order literal pins guard the hex mapping.
- `EXPECTED_GAIN = 0.5` (normalized `0.25` × plain `[0.0, 2.0]`)
  exercises the non-identity domain law; identity `1.0` would hide a
  normalized↔plain confusion.

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-l1-runner.jsonl` — hypothesis → change →
  decision → validation → summary chain for this slice.

### Rejected

- (none)

## Validation

All commands actually run on Windows (Git Bash + mise toolchain),
exit 0:

- `mise run fmt-check`
- `mise run check ruviform-plugin-test-vst3` /
  `check ruviform-plugin-test-clap`
- `mise run lint ruviform-plugin-test-vst3` /
  `lint ruviform-plugin-test-clap`
- `mise run test ruviform-plugin-test-vst3` /
  `test ruviform-plugin-test-clap` — e2e target compiles to 0 tests
  without the feature
- `mise run test-rt` — `cargo test --workspace rt_` pass
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin`
- `mise run test-l1-real-plugin` — adapter e2e 3+3 and test-fixture
  e2e 3+3 pass with real child-process spawn
- `mise exec -- prek run --all-files`

## Risks

- L1 evidence is Windows OS-L1 only; macOS/Linux lane runs are
  follow-up (tasks are platform-neutral).
- State Save/Reload, native editor, MIDI-over-L1, and external SDK
  validator coverage remain NotRun/Unsupported per the test-spec
  non-goals — adapter-side feature gaps recorded for separate
  Issues.

## Follow-ups

- Subsequent Issue #630 slices: `test_stateful` (blocked on adapter
  `save_state`/`restore_state`), `test_instrument`, `test_fault`
  expansion, `test_editor` (Issue #631 lane), external validator,
  consumer-oracle cross-checks, non-Windows lane evidence.
