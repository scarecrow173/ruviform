---
id: "IMPL-034XCLHSacLbmd5EOUazlw"
type: "impl"
status: "completed"
title: "VST3 test_gain plugin fixture (issue-630 first slice)"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-plugin-test-vst3"
      - type: "file"
        path: "docs/specs/issue-630-test-plugin-suite.md"
      - type: "file"
        path: "docs/designs/issue-630-test-plugin-suite.md"
      - type: "file"
        path: "docs/plans/issue-630-test-plugin-suite.md"
      - type: "file"
        path: "docs/tasks/issue-630-test-plugin-suite.md"
      - type: "file"
        path: "docs/test-specs/issue-630-test-plugin-suite.md"
      - type: "file"
        path: "evidence/rf630/verification.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-630-test-plugin-suite.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-630-test-plugin-suite.md"
    - "../../designs/issue-630-test-plugin-suite.md"
    - "../../plans/issue-630-test-plugin-suite.md"
    - "../../test-specs/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# VST3 test_gain plugin fixture (issue-630 first slice)

## Summary

New test-only crate `crates/ruviform-plugin-test-vst3` (`cdylib` +
`rlib`) implementing a real VST3 `test_gain` module: factory +
component + distributed edit controller over the `vst3` binding
crate and the neutral `ruviform-plugin-core` / `ruviform-audio-core`
contracts. The DSP law is `out = in * gain` with sample-accurate
parameter application at event frame offsets and a deliberately
non-identity normalized↔plain mapping (`plain = normalized * 2`).

## Context

Implements the first slice of `scarecrow173/ruviform#630` per
`docs/tasks/issue-630-test-plugin-suite.md`, derived from
`docs/specs/` → `docs/designs/` → `docs/plans/` →
`docs/test-specs/issue-630-test-plugin-suite.md`. Independent of the
`scarecrow173/ruviform#628` runtime (`crates/ruviform-audio-test` is
unmerged — zero dependency edges). The `ruviform-plugin-vst3`
adapter appears only as a dev-dependency (`host` feature) so
`tests/host_path.rs` can drive the real binary through the
production `Vst3Library` bridge.

## Implementation

### Changes

- `src/lib.rs` — crate docs (test-only declaration, independence
  boundary, RT contract), module wiring, `sdk_enum_i32` helper, and
  the VST3 module entry points: `GetPluginFactory` plus trivially-true
  platform hooks (`InitDll`/`ExitDll`, `ModuleEntry`/`ModuleExit`,
  `bundleEntry`/`bundleExit`).
- `src/factory.rs` — `TestGainFactory` implementing
  `IPluginFactory`/`2`/`3`; vendor `"Ruviform Test"`, two classes
  (`test_gain` component + `test_gain Controller`), fresh `uid`s
  distinct from the adapter fixture's `Gain`/`Gain Controller`.
- `src/component.rs` — `TestGainComponent` implementing `IComponent`
  + `IAudioProcessor` + `IProcessContextRequirements` over
  `ComWrapper`; `RefCell<ComponentState>` interior mutability with
  defined `kResultFalse` rejections on protocol violations;
  lifecycle mapped through the shared `Lifecycle` machine;
  `setupProcessing` prepares the neutral processor and sizes
  per-instance scratch; `process` validates `ProcessData`, collects
  bounded `IParameterChanges` points into `EventScratch`, builds
  `EventList`, and drives `TestGainProcessor` through zero-copy
  `PlanarBufferView` views.
- `src/controller.rs` — `TestGainController` implementing
  `IEditController`; one parameter (id 0, `Gain`), plain↔normalized
  mapping via `ParamInfo`, string parse/format in the plain domain,
  normalized value in `AtomicU64`; state streams report
  `kNotImplemented` honestly.
- `src/block.rs` — `ChannelScratch`/`EventScratch`/`PreparedBlock`:
  pointer null/alignment validation, exact + partial alias detection,
  bounded bounce buffers with deterministic writeback, bounded event
  queue/point/total-scan budgets.
- `src/gain.rs` — `GAIN_PARAM` descriptor and `TestGainProcessor`
  implementing `PluginProcessor`: sample-accurate gain law, persisted
  gain across blocks, zero-frame flush semantics.
- `src/strings.rs` — fixed-size `char8`/`TChar` copy helpers.
- `tests/host_path.rs` — production host-path integration tests:
  `Vst3Library::load` on the crate's own cdylib → `descriptors()` →
  `factories()` → `create()` → `init`/`prepare`/`start_processing` →
  `process_host` → drop teardown. No `Plugin` port fake.
- `Cargo.toml` (root) — one `members` line; `Cargo.lock` regenerated.

### Decisions During Implementation

- **Independent implementation**: adapter internals are `pub(crate)`
  and unreachable; per the issue's no-self-verification-loop rule the
  component/controller/DSP were written directly on the `vst3`
  binding + neutral contracts. The shared `vst3` crate is recorded as
  the SDK-equivalent layer in the design doc.
- **`RefCell<ComponentState>`**: the generated trait methods take
  `&self`; per-instance state lives behind one `RefCell` with
  `try_borrow_mut` → defined `kResultFalse` on re-entrant calls
  (never a panic across `extern "system"`). `getTailSamples` stays an
  `AtomicU32` for any-thread access.
- **Bounded hostile-input handling**: `maxSamplesPerBlock` capped at
  1 MiB frames; `IParameterChanges` scan capped at
  `MAX_BLOCK_EVENTS` queues/points plus a total
  `MAX_PARAM_SCAN_OPERATIONS` budget; out-of-block offsets and
  non-finite values dropped per the shared policy.
- **Aliasing**: exact in-place channel aliases bounce through
  per-channel scratch and write back; partial overlaps are rejected —
  never reinterpreted.
- **State surface**: `setState`/`getState`/`setComponentState` report
  `kNotImplemented` — honest rejection instead of silent state loss
  (`test_stateful` is a later class).
- **`kSample64`**: declined at `canProcessSampleSize` and re-checked
  at `setupProcessing`/`process` — never reinterpreted as f32.

## Related Experiments

### Adopted

None — the approach was determined by the spec/design chain; no
exploratory experiment logs were needed.

### Rejected

None.

## Validation

All run on Windows (mbx-managed target dir), toolchain pinned by
`mise.toml` (rust 1.99.0 line):

- `mise run fmt-check` — exit 0.
- `mise run check ruviform-plugin-test-vst3` — exit 0.
- `mise run lint ruviform-plugin-test-vst3` — exit 0
  (`clippy -- -D warnings` clean).
- `mise run test ruviform-plugin-test-vst3` — exit 0: 44 unit tests +
  3 `tests/host_path.rs` integration tests pass.
- `mise run build ruviform-plugin-test-vst3` — exit 0 (cdylib + rlib).
- `mise run test-rt` (`cargo test --workspace rt_`) — exit 0; this
  crate's `rt_` tests collected and passed:
  `block::tests::rt_prepare_finish_block_does_not_allocate`,
  `component::tests::rt_process_copies_gain_zero_copy`,
  `component::tests::rt_param_scan_stays_bounded_when_host_counts_lie`.

Production host-path evidence: `tests/host_path.rs` dlopens the real
built cdylib via `Vst3Library::load`, discovers `test_gain`
(vendor `Ruviform Test`), instantiates component + controller,
verifies the plain-domain parameter surface `[0.0, 2.0]` default
`1.0`, and processes golden PCM with a normalized `0.75` event at
frame offset 8 (→ plain `1.5` from frame 8), including gain
persistence into the next block. Recorded in `evidence/rf630/`.

## Risks

- `RefCell` interior mutability relies on the VST3 threading contract
  (no concurrent method calls); a cross-thread host violation is
  outside the defended boundary — documented in `component.rs`.
- The non-identity normalized↔plain mapping is intentional; a host
  bug that confuses domains will surface as wrong gain — that is the
  fixture's purpose.
- macOS/Linux module-hook paths compile only on those targets; this
  slice verified Windows only (OS-L1). Other OS lanes are follow-up.

## Follow-ups

- `test_instrument` / `test_stateful` / `test_fault` / `test_editor`
  classes, CLAP crate, `ruviform-audio-test` runtime integration,
  L1 out-of-process path, external SDK validator, CI lane integration
  — all deferred to later slices of issue-630.
