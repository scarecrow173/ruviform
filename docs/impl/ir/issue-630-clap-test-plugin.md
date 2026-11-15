---
id: "IMPL-034XTBVQ7ft6fUvrNI0tXJ"
type: "impl"
status: "completed"
title: "Issue #630 CLAP test_gain — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/tasks/issue-630-clap-test-plugin.md"
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-plugin-test-clap"
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
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/tasks/issue-630-clap-test-plugin.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-clap-test-plugin.md"
    - "docs/plans/issue-630-clap-test-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-630-test-plugin-suite.md"
    - "docs/impl/exp/issue-630-clap-test-plugin.jsonl"
    - "docs/test-specs/issue-630-clap-test-plugin.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-clap-test-plugin.jsonl"
    rejected: []
---

# Issue #630 CLAP test_gain — 実装記録

## Summary

Implemented `crates/ruviform-plugin-test-clap`, the test-only CLAP
`test_gain` fixture for Issue #630's CLAP slice: a real cdylib exporting
`clap_entry` (`SinglePluginEntry`), an independent gain DSP core on the
neutral `PluginProcessor` contract, a clack wiring layer covering
lifecycle/params/audio-ports, unit + ABI + RT tests, and a production
host-path integration test through `ClapLibrary`.

## Context

Follows `docs/tasks/issue-630-clap-test-plugin.md` →
`docs/plans/issue-630-clap-test-plugin.md` →
`docs/specs|designs/issue-630-clap-test-plugin.md`, derived from the
approved suite design `docs/designs/issue-630-test-plugin-suite.md` and
the merged VST3 first slice
(`docs/impl/ir/issue-630-test-plugin-suite.md`).

## Implementation

### Changes

- `Cargo.toml` — `[lib] crate-type = ["cdylib", "rlib"]`; deps
  `clack-plugin`/`clack-common`/`clack-extensions` `=0.2.0` (the
  SDK-equivalent binding layer, same pin as the adapter),
  `ruviform-plugin-core`, `ruviform-audio-core`; dev-deps
  `assert_no_alloc = "1.1"`, `clack-host = "=0.2.0"` (feature
  `clack-plugin`), `ruviform-plugin-clap` path dep with `features =
  ["host"]` (dev-dep edge only — adapter sources untouched).
- `src/lib.rs` — test-only crate docs + independence boundary +
  `clack_export_entry!(SinglePluginEntry<TestGainClapPlugin>)`.
- `src/gain.rs` — `TestGainProcessor`: `out = in × gain`, plain
  `[0.0, 2.0]` default `1.0`, sample-accurate offsets, last-wins,
  zero-frame flush, per-activation reset; 11 unit tests.
- `src/block.rs` — `InputEvents` `ParamValueEvent` (plain) → normalized
  `ParamValue` translation; zero-copy `PlanarBufferView` construction
  with raw-pointer validation and in-place classification; `BlockScratch`,
  `EventBuffer` pools, `AtomicLifecycle`, `ParamMirror` (`AtomicU64`).
- `src/plugin.rs` — clack `Plugin`/`PluginShared`/`PluginMainThread`/
  `PluginAudioProcessor` + `PluginAudioPorts` + `PluginParams`/
  `PluginAudioProcessorParams` impls; lifecycle → neutral
  `LifecycleState`; 10 in-crate tests incl. `rt_` `assert_no_alloc`
  process-path test and a `clack-host`-driven end-to-end ABI test.
- `tests/host_path.rs` — 3 production-path tests via `ClapLibrary::load`
  on the built cdylib (fixture resolved beside `current_exe()` under
  `deps/`, missing binary assert-fails).
- `README.md`, workspace `members` line, `Cargo.lock`, `crates/README.md`
  index row, doc chain, `evidence/rf630/clap-test-gain-verification.md`.

### Decisions During Implementation

- Descriptor version pinned to literal `"0.0.0"` (suite fixture
  convention, matching the VST3 slice) rather than
  `CARGO_PKG_VERSION` — the oracle asserts the fixture identity.
- `clack-extensions` features limited to the `clack-plugin` glue plus
  `params` and `audio-ports` — only what the plugin side declares/uses;
  `thread-check` and host-side extension features stay off this edge
  (the dev-dep `clack-host` carries its own).
- No `unsafe` in crate code — `clack_export_entry!` emits its own
  scoped allow (adapter precedent).
- DSP-law commonization across format crates explicitly not done —
  independent implementations per suite design.

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-clap-test-plugin.jsonl` — hypothesis →
  implementation → validation → decision → summary chain for this
  slice.

### Rejected

- (none)

## Validation

- `mise run fmt-check` — pass.
- `mise run check ruviform-plugin-test-clap` — pass.
- `mise run lint ruviform-plugin-test-clap` — pass (`-D warnings`).
- `mise run test ruviform-plugin-test-clap` — 21 unit + 3 host-path +
  0 doctests, all pass.
- `mise run build ruviform-plugin-test-clap` — cdylib+rlib build pass.
- `mise run test-rt` — `rt_process_drives_paired_stereo_block_allocation_free`
  collected by `cargo test --workspace rt_` and passes under
  `assert_no_alloc`.
- `mise exec -- prek run --all-files` — see evidence record.

## Risks

- L0 in-process verification only; L1 out-of-process runner, external
  SDK validator, non-Windows lanes remain NotRun (subsequent slices).
- `test_stateful` blocked on production host bridges lacking
  `save_state`/`restore_state` — adapter feature gap recorded for a
  separate Issue.

## Follow-ups

- Subsequent Issue #630 slices: `test_stateful` (after adapter state
  support), `test_instrument`, `test_fault`, `test_editor`, L1 runner,
  external validator, consumer-oracle cross-checks.
