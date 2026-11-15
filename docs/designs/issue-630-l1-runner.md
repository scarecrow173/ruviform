---
id: "DESIGN-034XVrHUBVOxUT8uBVZNso"
type: "design"
status: "approved"
title: "Issue #630 L1 runner slice — test plugin L1 out-of-process path 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
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
        path: "docs/designs/issue-630-l1-runner.md"
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
    - "docs/specs/issue-630-l1-runner.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  blocks:
    - "docs/plans/issue-630-l1-runner.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-l1-runner.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs"
    - "crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs"
    - "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
    - "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 L1 runner slice — test plugin L1 out-of-process path 設計

## Context

Issue #630's approved suite design
(`docs/designs/issue-630-test-plugin-suite.md`) splits the test-plugin
suite into one test-only crate per format; both fixtures
(`ruviform-plugin-test-vst3`, `ruviform-plugin-test-clap`) landed with
L0 in-process production-host-path verification only. The Issue #114
chain (`docs/designs/issue-114-real-adapter-l1-e2e.md`) built the L1
out-of-process substrate — `ruviform-plugin-export::run` child runner +
`ruviform-plugin-host` `PluginHost`/`OutOfProcessSpec` parent — and
proved it against the *adapter's own* fixture cdylib via the
format-specific runner bins and `tests/l1_real_adapter_e2e.rs`.

Two facts fix this slice's shape:

1. The adapter runners are compile-time fixed to the adapter fixture
   (`fixture_lib_name()` returns `ruviform_plugin_{vst3,clap}`) and
   `OutOfProcessSpec` carries no env/argv channel to redirect them at
   the test fixture. Extending either would touch production adapter
   code — outside this slice's boundary.
2. The test crates already carry the exact same dependency edges the
   runners need: the adapter crate (currently dev-dep with `host`) for
   the production `*Library` bridge, plus neutral contracts. Only
   `ruviform-plugin-export` and a bin target are missing.

The cleanest boundary is therefore a per-test-crate runner bin +
feature-gated e2e suite inside each fixture crate — production adapter
sources stay byte-identical.

## Scope

### In Scope

- `crates/ruviform-plugin-test-vst3/Cargo.toml` and
  `crates/ruviform-plugin-test-clap/Cargo.toml`:
  - `[features] l1-runner = ["dep:<adapter>", "dep:ruviform-plugin-export"]`
    where `<adapter>` is `ruviform-plugin-vst3` /
    `ruviform-plugin-clap` with `features = ["host"]`, moved to an
    optional regular dependency while the existing dev-dependency
    entry is kept (the `clack-host` precedent in
    `ruviform-plugin-clap` — features union, DAG stays acyclic);
  - `ruviform-plugin-export` optional regular dep;
  - `[[bin]]` `ruviform-plugin-test-{vst3,clap}-l1-runner` with
    `required-features = ["l1-runner"]`;
  - dev-dep `ruviform-plugin-host` (same dev-dep direction the adapter
    crates use for their e2e suites — `plugin-host` depends on no
    adapter or test crate).
- `src/bin/ruviform-plugin-test-{vst3,clap}-l1-runner.rs` (~130 lines
  each): structural mirror of the adapter runner — resolve fixture
  path (`RUVIFORM_L1_REAL_PLUGIN_DIR` override → exe-dir),
  `*Library::load`, `factories()` → `Box<dyn PluginFactory>` →
  `ruviform_plugin_export::run`. The only semantic difference is the
  fixture lib name, which is **local** (`ruviform_plugin_test_{vst3,clap}`)
  because the adapter's `fixture_lib_name()` is compile-time fixed to
  the adapter fixture. The bin's `#[cfg(test)]` fixture-resolution
  tests mirror the adapter's.
- `tests/l1_real_test_plugin_e2e.rs` in each crate:
  `#![cfg(feature = "l1-runner")]` + three `#[ignore]`d tests mirroring
  `l1_real_adapter_e2e.rs` — `full_series` (scan/instantiate/
  initialize/activate/start → 32 stereo shm blocks with a
  `ParamValue` event → defined-`Unsupported` state+GUI verbs → clean
  teardown), `kill_reports_failed_once`, `missing_runner_is_spawn_failed`.
  Descriptor pins: VST3 derives `expected_id()` from
  `vst3::uid(0x6D4F_8A21, 0x3B7C_9E05, 0xF1A2_4C68, 0xB9D3_7E51)` (the
  fixture's `pub(crate)` CID constants, same literal-derivation
  precedent as the adapter e2e) with platform literal regression
  pins; CLAP pins the literal descriptor
  (`org.ruviform-test.clap-test-gain` / `test_gain` / `"Ruviform Test"`
  / `0.0.0` / `["audio-effect", "stereo"]`). `EXPECTED_GAIN` is `0.5`
  for both — `ParamValue::new(0.25)` normalized → plain `0.5` through
  the fixtures' non-identity `[0.0, 2.0]` gain law.
- `mise.toml`: `test-l1-real-plugin` gains a build line
  (`cargo build -p <test crates> --features l1-runner`), a `--lib
  --bins` unit-test line, and a `--test l1_real_test_plugin_e2e
  -- --ignored` e2e line; `check-l1-real-plugin` /
  `lint-l1-real-plugin` gain a second cargo invocation for the test
  crates under `--features l1-runner` (feature names differ per
  package, so a single `--features` cannot cover both groups).
- `Cargo.lock` regeneration (new path-dep edges only — no new external
  crates).
- Doc chain + `evidence/rf630/` records.

### Out of Scope

- Production adapter sources (`crates/ruviform-plugin-{vst3,clap}/src/**`),
  `.github/**` (the existing CI lanes already run the same mise tasks
  on windows+ubuntu+macos — `ci.yml:1396-1405`/`1472-1476`),
  `POLICIES.md`, `AGENTS.md`, fixture plugin-side sources.
- State round-trip, editor, MIDI/instrument classes, external
  validators, non-Windows lanes, consumer-oracle cross-checks —
  recorded NotRun/Unsupported/Deferred in evidence.

## Components and Boundaries

- Test-crate runner bins — own only fixture resolution + factory handoff;
  all wire/lifecycle/process semantics live in
  `ruviform-plugin-export`/`ruviform-plugin-host`/the adapter `host`
  bridges, exactly as for the adapter runners.
- `tests/l1_real_test_plugin_e2e.rs` — parent-side oracle only; it
  never dlopens the fixture itself (the child does, through the
  production bridge — the independence boundary is preserved: parent
  pins literals, child scans the real binary).
- `mise.toml` tasks — the single entry point; `cargo test -p` default
  runs never see the `#[ignore]`d tests (`#![cfg]` + `required-features`
  + `#[ignore]` triple gate, same as the adapter lane).

## Data and Control Flow

1. Lane: `cargo build -p <test crates> --features l1-runner` emits the
   test cdylib (`target/<profile>/`) + runner exe; `cargo test --lib
   --bins` runs the bins' fixture-resolution unit tests; `cargo test
   --test l1_real_test_plugin_e2e -- --ignored` compiles the e2e with
   `CARGO_BIN_EXE_*` defined and selects the ignored tests.
2. Test: `PluginHost::new()` → `register_out_of_process(OutOfProcessSpec::new(
   expected_descriptor, runner_program()))` → `Scan` (parent-side
   replay) → `Instantiate` spawns the runner → runner resolves +
   dlopens the test cdylib via the production `*Library` bridge →
   `factories()` → `run()` → wire `Hello`/`Scanned` handshake →
   `Descriptor::matches` field-equality against the parent pin.
3. `Initialize`/`Activate`/`StartProcessing` → 32 stereo blocks through
   shm with a normalized `ParamValue(0.25)` event; delivered blocks must
   equal `0.5 ×` an earlier input (non-identity gain law), first
   `lookahead_blocks` calls are defined silence, underruns are defined
   silence — every call `Ok(CONTINUE)`.
4. `SaveState`/`RestoreState` → defined `Err(Unsupported)` wire
   replies; `OpenGui`/`CloseGui` → host-side `Err(Unsupported)`
   short-circuit (L1 GUI deferred); `StopProcessing`/`Deactivate`/
   `Destroy` → clean child exit 0, no notification.
5. Kill path: `kill_child` → `Failed` convergence → exactly one
   `InstanceFailed` notification → defined `Err(Failed)` rejections +
   silence → `Destroy` frees the slot.
6. Missing-runner path: nonexistent `spec.program` →
   `Err(SpawnFailed{cause: Spawn})` pre-`Hello`, no instance, no
   notification, no hang.

## Non-Functional Constraints

- **Test-only boundary**: no production crate gains a dependency on
  the test crates; the test crates' new edges point only at adapter
  crates (optional), `ruviform-plugin-export` (optional), and
  `ruviform-plugin-host` (dev-dep) — all already in the workspace DAG.
- **Feature hygiene**: default feature set stays minimal; `l1-runner`
  is opt-in; the e2e file is empty without it.
- **No new external deps**: `Cargo.lock` diff is limited to the two
  test crates' dependency lists.
- **Determinism**: descriptor pins are literals/`vst3::uid`
  derivations — platform byte order handled by `#[cfg]` pins verified
  on Windows (this lane's OS) with the non-Windows literal derived by
  the same verified transform.

## Risks and Trade-offs

- **Feature-unification drift**: enabling `l1-runner` activates the
  adapter's `host` feature (and for CLAP, `clack-extensions/clack-host`
  + `gui`) inside the test crates' build graph. Harmless — features
  only add API surface the fixture does not call, and the same
  unification already occurs for `cargo test` via the existing dev-dep.
- **`--features` across heterogeneous packages**: `host` and
  `l1-runner` are different feature names, so check/lint lanes run two
  cargo invocations rather than one — slightly slower, zero ambiguity.
- **`env!("CARGO_BIN_EXE_*")` availability**: guaranteed because the
  bin's `required-features` are satisfied in the same invocation that
  compiles the test; the `#![cfg]` keeps the macro unexpanded without
  the feature (adapter precedent).
- **Self-verification bias**: unchanged from prior slices — the
  fixture implementations stay independent; this slice adds only the
  execution path, and the parent oracle pins literals computed
  independently of the child's scan.

## Deferred Design Concerns

- State round-trip (`test_stateful`) — production bridges keep
  `save_state`/`restore_state` at `Plugin` defaults
  (`Err(HostError::Unsupported)`); adapter-side state extension is a
  separate Issue.
- External SDK validator runs — Deferred per suite design.
- `test_instrument`/`test_fault`/`test_editor` classes — later slices;
  this slice provides the shared spawn/lifecycle substrate they reuse.
- Per-OS (macOS/Linux) lane evidence — the mise tasks are
  platform-neutral; OS-L1 recording beyond Windows is follow-up.

## Verification Notes

- [ ] `mise run fmt-check` clean.
- [ ] `mise run check ruviform-plugin-test-vst3` /
  `check ruviform-plugin-test-clap` pass (default features).
- [ ] `mise run lint ruviform-plugin-test-vst3` /
  `lint ruviform-plugin-test-clap` pass (`-D warnings`).
- [ ] `mise run test ruviform-plugin-test-vst3` /
  `test ruviform-plugin-test-clap` pass (default features — e2e file
  compiles empty).
- [ ] `mise run test-l1-real-plugin` — full lane incl. both new
  `#[ignore]`d suites passes on Windows.
- [ ] `mise run test-rt` — `rt_` tests still collected/pass.
- [ ] `mise exec -- prek run --all-files` clean.
- [ ] `git diff` shows zero changes under
  `crates/ruviform-plugin-{vst3,clap}/src/**` and `.github/**`.
