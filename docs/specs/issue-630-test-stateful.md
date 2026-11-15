---
id: "SPEC-034Y4czy8Gzr9POTbUQdKh"
type: "spec"
status: "approved"
title: "Issue #630 test_stateful slice — real state blob plugin class + L1 save/mutate/restore/save round-trip E2E"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/specs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs"
      - type: "file"
        path: "docs/specs/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/designs/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/plans/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/tasks/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/test-specs/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-test-stateful.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-test-stateful.jsonl"
      - type: "file"
        path: "evidence/rf630/test-stateful-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/gain.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "mise.toml"
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
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/specs/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_stateful slice — real state blob plugin class + L1 save/mutate/restore/save round-trip E2E

## What Should Be Built

A `test_stateful` plugin class inside each existing Issue #630 test
fixture cdylib (`ruviform-plugin-test-vst3`,
`ruviform-plugin-test-clap`) carrying **real plugin-defined state
blobs**, plus the L1 out-of-process E2E oracle that exercises the
production host bridges' `save_state`/`restore_state` verbs through
the real plugin ABI:

- **`test_stateful` class per format** — same stereo bus shape,
  gain parameter (plain `[0.0, 2.0]`, default `1.0`) and DSP law
  (`out = in * gain`) as `test_gain`; the class exists to carry
  real state, not new DSP. VST3 gains a second component/controller
  CID pair (factory 6→8 classes) implementing `IComponent::
  setState`/`getState` over `IBStream` and the controller-side
  `setComponentState`/`setState`/`getState` verbs; CLAP registers
  `clap.ext.state` (`PluginStateImpl::save`/`load` over the
  host-provided `InputStream`/`OutputStream`).
- **State blob law (both formats)** — a deterministic 16-byte
  payload `[u32 LE "RFST"][u32 LE version=1][f64 LE]`:
  VST3 component payload stores the **plain** gain, controller
  payload stores the **normalized** gain, and `setComponentState`
  converts between domains; CLAP's single stream stores the plain
  gain. Wrong length, magic, version or non-finite values are
  defined rejections; finite out-of-range values clamp to the
  parameter range. Serialization is byte-exact deterministic.
- **Persistent-state law** — unlike `test_gain` (whose activation
  deterministically resets the parameter to default),
  `test_stateful` activation **preserves** the persisted gain, so a
  restored state survives re-activation.
- **`tests/l1_stateful_test_plugin_e2e.rs` per crate**
  (`#![cfg(feature = "l1-runner")]` + `#[ignore]`) covering the
  save → mutate → restore → save round-trip: save(initial) →
  `ParamValue` mutation → save(mutated) → second mutation →
  save → restore(initial) → save == initial byte-exact → delivered
  audio proves the restored gain drives the DSP; plus a
  malformed-blob defined-rejection case. Every state verb is gated
  on *delivered audio* (distinct gain outputs self-identify
  processed blocks through the L1 lookahead pipeline).
- **`mise.toml`** — one added run line in `test-l1-real-plugin`
  selecting `--test l1_stateful_test_plugin_e2e -- --ignored`.
- **Doc/evidence chain** for this slice.

## Why Now

Issue #630's `test_stateful` slice is the state counterpart of the
adapter-state-bridge slice (`issue-630-adapter-state-bridge`, merged
as the production `save_state`/`restore_state` plumbing): the host
bridges now speak VST3 `IBStream` state and CLAP `clap.ext.state`,
but no fixture exercised them end-to-end. Until now every fixture
declined state honestly (`kNotImplemented` / missing extension →
`Unsupported`), so the real-ABI state path had no independent
oracle. This slice adds the fixture that makes save/restore a
verifiable contract rather than an untested code path.

## Acceptance Criteria (this slice)

- [ ] `test_stateful` discoverable + instantiable through the real
  VST3 class scan (factory 8 classes, 4 audio descriptors) and CLAP
  factory scan (`plugin_count() == 3`), with correct gain parameter
  metadata and sample-accurate processing identical to `test_gain`.
- [ ] In-process host-path tests (real cdylib through the
  production `Vst3Library`/`ClapLibrary` bridges): save → mutate →
  restore → save is byte-exact; malformed blobs are defined
  `PluginFailed` rejections and the instance survives; stateless
  classes keep `Unsupported`/`kNotImplemented` declines.
- [ ] L1 E2E (ignored, `l1-runner` feature, real cdylib + runner):
  both round-trip and malformed-rejection cases pass per format on
  Windows OS-L1 via `mise run test-l1-real-plugin`.
- [ ] `rt_`-safe: the `process` path stays alloc/lock/wait-free —
  state verbs live on the non-RT component/controller (VST3) and
  main-thread (CLAP) surfaces only.
- [ ] Honest boundaries recorded: plugin-side state is exercised
  through the production bridges but the fixtures never import
  production adapter internals; `test_instrument`, CLAP
  `test_editor`, external SDK validators, non-Windows OS runs, and
  `*-msrv`/`*-full` lanes are out of scope (deferred/NotRun).

## Non-Goals

- No production crate changes (`ruviform-plugin-{vst3,clap,host,
  export,core}` stay zero-diff); no new `unsafe` beyond the
  clack-macro-generated entry statics.
- `test_gain`/`test_fault`/`test_editor` class, descriptor, CID and
  DSP contracts unchanged; their state declines stay honest.
- No preset/bank management, no GUI state, no migration of foreign
  plugin state formats — the blob is the fixture's own contract.
- PR uses `Refs #630` — the Issue stays open for later slices.
