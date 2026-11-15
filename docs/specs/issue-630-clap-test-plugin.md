---
id: "SPEC-034XTAfC3ThbkSW16xLQ7k"
type: "spec"
status: "approved"
title: "Issue #630 CLAP slice — test-only crate ruviform-plugin-test-clap (CLAP test_gain)"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-plugin-test-clap"
      - type: "file"
        path: "docs/specs/issue-630-clap-test-plugin.md"
      - type: "file"
        path: "docs/designs/issue-630-clap-test-plugin.md"
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
      - type: "file"
        path: "evidence/rf630/clap-test-gain-verification.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
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
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "crates/ruviform-plugin-test-vst3/README.md"
    - "crates/ruviform-plugin-clap/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP slice — test-only crate ruviform-plugin-test-clap (CLAP test_gain)

## What Should Be Built

A test-only Rust crate `crates/ruviform-plugin-test-clap` that builds a
real CLAP shared object (`cdylib` + `rlib`) exporting `clap_entry` via
`clack_export_entry!(SinglePluginEntry)`, exposing exactly one plugin
`test_gain` (CLAP id `org.ruviform-test.clap-test-gain`, vendor
`"Ruviform Test"`, features `audio-effect` + `stereo`). The fixture
implements a deterministic gain processor against the real CLAP ABI and
the `clack-*` `=0.2.0` binding surface, independently of the production
`ruviform-plugin-clap` adapter internals, and is verified end to end
through the production host path (`ClapLibrary::load` →
`descriptors()`/`factories()` → `ErasedPlugin`
init→prepare→start_processing→process→teardown).

## Why Now

Issue #630's suite design pins format-width crates: the VST3
`test_gain` slice (PR #641, merged) proved the fixture pattern on VST3;
this slice fixes the same width on CLAP so the suite covers both
formats the production host bridges already support, with the same
oracle shape (golden PCM, param domain, sample-accurate event offsets,
RT non-allocation).

## Users and Value

Consumers are the Ruviform test infrastructure and downstream consumer
oracle checks (`scarecrow173/synvelis#101`/`#127`/`#128`/`#176`,
`scarecrow173/audio-lens#127` in later slices). Value: a real,
independently-implemented CLAP binary exercises the production host
without mocks and without self-verification bias.

## Scope

### In Scope

- New test-only crate with `crate-type = ["cdylib", "rlib"]`.
- `test_gain` semantics identical in law to the VST3 slice: stereo
  in/out, one automatable `Gain` param (plain `[0.0, 2.0]`, default
  `1.0`, non-identity normalized mapping), sample-accurate param events,
  zero-frame flush, gain persistence, deterministic per-activation reset.
- CLAP `params` + `audio-ports` extensions; lifecycle mapped onto the
  neutral `LifecycleState` machine with honest protocol-violation
  handling.
- Unit/in-process tests (clack-host fixtures), an `rt_`
  `assert_no_alloc` test, and a production host-path integration test
  that loads the built cdylib through `ruviform-plugin-clap`'s `host`
  feature (dev-dependency only).
- Workspace integration (`members` entry, `Cargo.lock`, crates index)
  and the doc/evidence chain for this slice.

### Out of Scope

- `test_stateful` (State Save/Reload/Preset) — the production VST3 and
  CLAP host bridges both lack `save_state`/`restore_state`; recorded as
  an adapter-side feature gap for a separate Issue.
- `test_instrument` (Tempo/Loop/MIDI), `test_fault`
  (Crash/Hang/Timeout/Isolation — L1-dependent), `test_editor`
  (GUI-L2, `scarecrow173/ruviform#631` linkage).
- L1 out-of-process runner path, external SDK validators
  (clap-validator etc.), non-Windows OS runs, GUI-L2/EXT-L3, CI lane
  integration, consumer-oracle cross-checks, DSP-law commonization
  across format crates.
- Any change to `ruviform-plugin-clap` sources, `mise.toml`,
  `.github/**`, `POLICIES.md`, `AGENTS.md`, or the VST3 fixture crate.

## Requirements

- The cdylib must be a real CLAP module loadable by the production
  `ClapLibrary` host bridge — no fake plugin ports anywhere in the
  verification path.
- The plugin-side implementation must not reuse `ruviform-plugin-clap`
  internals; the only shared layer is the `clack-*` binding crates
  (SDK-equivalent) plus neutral contracts.
- `process` must be allocation-free, bounded, and non-blocking; RT
  scratch is sized at `activate`.
- The crate is test-only: no production crate may depend on it, and it
  must not force new dependencies on consumers such as AudioLens.

## Acceptance Criteria

- [x] Real `.clap`/cdylib artifact is discovered, instantiated,
      prepared, processed, and torn down through the production
      `ClapLibrary` host path (L0 in-process), with no fake plugin port.
- [x] Fixed golden PCM vectors and a normalized `ParamValue` event at
      frame offset 8 verify sample-accurate gain application
      (frames 0–7 unity, 8–63 ×1.5) and persistence into the next block.
- [x] Parameter surface reports plain `[0.0, 2.0]` default `1.0` through
      the production `params()` path.
- [x] `rt_`-prefixed test verifies the process path performs no
      allocation under `assert_no_alloc`.
- [x] Independence boundary documented; test-only status declared in
      crate docs and README.

## Deferred Design Concerns

- State/preset plugin class (`test_stateful`) — blocked on the
  production host bridges' unimplemented `save_state`/`restore_state`;
  re-engage when an adapter-side state extension lands (separate Issue).
- External CLAP SDK validator run — deferred; no external authority is
  claimed for ABI conformance in this slice.
- L1 out-of-process execution of this fixture — deferred to the runner
  slice of Issue #630.
- DSP-law commonization across `test_gain` crates — deferred until a
  third format or shared requirement appears (suite design decision).

## Sources and Relations

- Issue #630 suite acceptance criteria and slice scoping (Coordinator
  task spec); approved suite design
  `docs/designs/issue-630-test-plugin-suite.md` pins crate name,
  format-per-crate split, independent implementations, and suite
  conventions (vendor `"Ruviform Test"`, `test_<role>` naming).
- VST3 first-slice chain
  (`docs/specs|designs|plans|tasks|test-specs|impl/*issue-630-test-plugin-suite*`)
  is the direct precedent for oracle shape and doc structure.
- `crates/ruviform-plugin-clap` (`src/host.rs`, `src/plugin.rs`,
  `src/gain.rs`) is the production host path under test and the
  clack-API precedent; `crates/ruviform-plugin-test-vst3` is the
  fixture precedent.
