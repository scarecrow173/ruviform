---
id: "SPEC-034XVqPg6BgUwKye5dwTPA"
type: "spec"
status: "approved"
title: "Issue #630 L1 runner slice — test plugin L1 out-of-process path (ruviform-plugin-test-{vst3,clap})"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-clap-test-plugin.md"
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
        path: "docs/specs/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/designs/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/plans/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/tasks/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/test-specs/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-l1-runner.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-l1-runner.md"
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
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "crates/ruviform-plugin-vst3/README.md"
    - "crates/ruviform-plugin-clap/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 L1 runner slice — test plugin L1 out-of-process path (ruviform-plugin-test-{vst3,clap})

## What Should Be Built

The L1 out-of-process execution path for both Issue #630 test-plugin
fixtures. Each test crate (`ruviform-plugin-test-vst3`,
`ruviform-plugin-test-clap`) gains:

- an `l1-runner` Cargo feature wiring optional regular deps
  `ruviform-plugin-vst3`/`ruviform-plugin-clap` (with `host`) and
  `ruviform-plugin-export`, plus a `[[bin]]` (feature-gated by
  `required-features`) — a per-crate runner binary mirroring the
  production adapter runners (`ruviform-plugin-{vst3,clap}-l1-runner`)
  that dlopens **its own crate's** cdylib through the production host
  bridge and serves the scanned factories to
  `ruviform_plugin_export::run`;
- a `tests/l1_real_test_plugin_e2e.rs` integration suite
  (`#![cfg(feature = "l1-runner")]` + `#[ignore]`-gated) mirroring the
  adapter `tests/l1_real_adapter_e2e.rs` oracle: spawn + handshake
  descriptor field-match, lifecycle to `Processing`, shm `process`
  blocks with a `ParamValue` event (delivered-block membership —
  matched calls only, substituted `last_output` replays classified
  out via the `passthrough_submits` delta; underrun = defined
  silence), defined `Err(Unsupported)` for
  `SaveState`/`RestoreState`/`OpenGui`/`CloseGui`, clean teardown,
  `kill_child` → `Failed` + exactly-once notification, and missing
  runner → `SpawnFailed`;
- `mise.toml` lane wiring so `test-l1-real-plugin` /
  `check-l1-real-plugin` / `lint-l1-real-plugin` cover the test crates
  under `--features l1-runner` (CI picks the same tasks up unchanged —
  `.github/**` is not touched);
- the doc/evidence chain for this slice.

## Why Now

Issue #630's suite design defers the L1 out-of-process path to a later
slice; the fixtures' first slices proved the in-process production host
path (L0) only. `test_fault`'s Crash/Hang/Timeout/Isolation coverage is
blocked on this runner path, and every remaining fixture class
(`test_stateful`, `test_instrument`, `test_editor`) needs the same
spawn/handshake/lifecycle substrate. Landing the runner inside the test
crates — rather than widening the adapter runners, whose fixture lib
name is compile-time fixed and whose `OutOfProcessSpec` carries no
env/argv extension — keeps production adapter code untouched (zero
product-crate diff) while giving each format's fixture its own
out-of-process evidence.

## Users and Value

Consumers are the Ruviform test infrastructure and downstream consumer
oracle checks. Value: the test fixtures — independent implementations
on the real ABI — now execute through the production `PluginHost` L1
wire path end to end, so host↔fixture verification no longer stops at
the in-process bridge; crash/kill/isolation semantics get a real
binary under test.

## Scope

### In Scope

- `crates/ruviform-plugin-test-vst3/` and
  `crates/ruviform-plugin-test-clap/`: `Cargo.toml` feature/deps/bin
  wiring, `src/bin/*-l1-runner.rs`, `tests/l1_real_test_plugin_e2e.rs`.
- `mise.toml`: extend `test-l1-real-plugin`, `check-l1-real-plugin`,
  `lint-l1-real-plugin` run lists for the test crates under
  `--features l1-runner`.
- `Cargo.lock` regeneration for the new dependency edges.
- Doc chain (`docs/{specs,designs,plans,tasks,test-specs,impl}`) and
  `evidence/rf630/` records for this slice.

### Out of Scope

- Any change to production adapter sources
  (`crates/ruviform-plugin-vst3/src/**`,
  `crates/ruviform-plugin-clap/src/**`), `.github/**`, `POLICIES.md`,
  `AGENTS.md`, or the fixtures' plugin-side implementation sources.
- `test_stateful` state round-trip evidence — the production bridges
  keep `save_state`/`restore_state` at the `Plugin` defaults
  (`Err(Unsupported)`); recorded as an adapter-side feature gap for a
  separate Issue.
- `test_instrument` (MIDI/Tempo/Loop classes do not exist yet) and
  `test_editor` (class absent; GUI-L2 owned by Issue #631 linkage).
- External SDK validators (VST3/CLAP validator runs stay Deferred);
  macOS/Linux lanes (Windows OS-L1 only in this slice); GUI-L2/EXT-L3;
  consumer-oracle cross-checks; MSRV lane (user-specified exclusion);
  AudioLens-facing dependency changes (none introduced — test-only
  boundary preserved).

## Requirements

- The runner binaries must be feature-gated (`l1-runner`) so default
  builds, `cargo test` default runs, and dependent crates are
  unaffected; the e2e file must compile to nothing without the
  feature (`#![cfg]`) so `env!("CARGO_BIN_EXE_*")` stays undefined.
- The runner must resolve its own crate's cdylib
  (`{DLL_PREFIX}ruviform_plugin_test_{vst3,clap}{DLL_SUFFIX}`) via
  `RUVIFORM_L1_REAL_PLUGIN_DIR` override → exe-dir default, mirroring
  the adapter runner contract (defined exit-1 rejection before `Hello`
  on resolution/load failure).
- The e2e oracle must field-match the child's scanned descriptor
  against a parent-side pin computed independently (literal CID hex
  derivation for VST3; literal descriptor for CLAP), reuse the
  adapter e2e's matched-only delivered-block membership (substituted
  replays classified out via `passthrough_submits` deltas;
  `ruviform#768` oracle audit) / underrun-silence /
  exactly-once-notification semantics, and assert-fail (never
  silent-skip) on missing artifacts.
- Dependency direction must stay acyclic: test crates may depend on
  the adapter crates and `ruviform-plugin-export` (optional) and
  `ruviform-plugin-host` (dev-dep); nothing may depend back on the
  test crates.
- The lane must remain the only entry point for the `#[ignore]`d tests;
  `cargo test -p` default runs must not see them.

## Acceptance Criteria

- [ ] `mise run test-l1-real-plugin` builds both test-crate cdylibs +
  runner bins and passes the `#[ignore]`d e2e suites on Windows:
  spawn+handshake descriptor field-match, lifecycle, param-event shm
  process with matched-only delivered-block membership and underrun
  silence, defined `Err(Unsupported)` state/editor verbs, clean
  teardown, kill→`Failed`+notification exactly once, missing
  runner→`SpawnFailed`.
- [ ] `mise run check`/`lint`/`test`/`fmt-check`/`test-rt` pass for
  both test crates (default features) and the `l1-runner` feature
  targets via the extended lane tasks; `prek run --all-files` clean.
- [ ] Production adapter sources, `.github/**`, `POLICIES.md`,
  `AGENTS.md` unchanged; dependency DAG stays acyclic; no new external
  dependencies (path deps only).
- [ ] Honest boundaries recorded: state round-trip / editor /
  MIDI-over-L1 / external validator remain NotRun or Unsupported with
  reasons in evidence.

## Deferred Design Concerns

- State round-trip (`test_stateful`) — blocked on production bridges'
  unimplemented `save_state`/`restore_state`; re-engage when an
  adapter-side state extension lands (separate Issue).
- External VST3/CLAP SDK validator runs — deferred; no external
  authority is claimed for ABI conformance in this slice.
- `test_instrument` / `test_editor` classes — later Issue #630 slices.
- macOS/Linux OS-L1 lanes for this path — the lane tasks are
  OS-agnostic cargo invocations and run in existing CI where the
  artifacts build; per-OS evidence recording is a follow-up.

## Sources and Relations

- Issue #630 suite acceptance criteria and slice scoping (Coordinator
  task spec); approved suite design
  `docs/designs/issue-630-test-plugin-suite.md` pins the
  fixture-per-format crates and defers L1 to this slice.
- Merged precedents: VST3 first slice
  (`docs/*/issue-630-test-plugin-suite.*`), CLAP slice
  (`docs/*/issue-630-clap-test-plugin.*`), and the Issue #114 real
  adapter L1 chain (`docs/designs/issue-114-real-adapter-l1-e2e.md`,
  `docs/test-specs/real-adapter-l1-e2e-tests.md`,
  `crates/ruviform-plugin-{vst3,clap}/tests/l1_real_adapter_e2e.rs`,
  `src/bin/*-l1-runner.rs`) whose runner/e2e shapes this slice mirrors.
- `POLICIES.md` §2/§2.1 — OS-L1 vs GUI-L2/EXT-L3 separation and the
  real-ABI-test-plugin (not mock) verification rule this slice
  satisfies for the L1 process boundary.
