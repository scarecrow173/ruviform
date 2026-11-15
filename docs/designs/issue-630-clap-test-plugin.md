---
id: "DESIGN-034XTB8x8FnnWOE3AoR7Q1"
type: "design"
status: "approved"
title: "Issue #630 CLAP slice — test-only crate ruviform-plugin-test-clap 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "dir"
        path: "crates/ruviform-plugin-test-clap"
      - type: "file"
        path: "docs/designs/issue-630-clap-test-plugin.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-630-clap-test-plugin.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-test-plugin.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  blocks:
    - "docs/plans/issue-630-clap-test-plugin.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-clap-test-plugin.md"
    - "crates/ruviform-plugin-clap/src/host.rs"
    - "crates/ruviform-plugin-clap/src/plugin.rs"
    - "crates/ruviform-plugin-test-vst3/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-plugin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP slice — test-only crate ruviform-plugin-test-clap 設計

## Context

Issue #630's approved suite design
(`docs/designs/issue-630-test-plugin-suite.md`) fixes the test-plugin
suite shape: one test-only crate per plugin format, each an independent
implementation on the real ABI plus the neutral contracts, verified
through the production host path. The VST3 slice
(`crates/ruviform-plugin-test-vst3`) landed that pattern for VST3; this
design fixes the same width for CLAP.

The production CLAP host bridge (`ruviform-plugin-clap` feature `host`)
already provides `ClapLibrary::load` → `descriptors()`/`factories()` →
`ErasedPlugin` lifecycle and `process_host` with `EventKind::ParamValue`
(normalized) → `ParamValueEvent` (plain via `ParamInfo::denormalize`)
translation. The fixture under test must therefore speak the CLAP
`params` extension plain domain on the wire while keeping the neutral
event path normalized — the same oracle shape the VST3 slice uses.

## Scope

### In Scope

- Crate `crates/ruviform-plugin-test-clap`, `crate-type =
  ["cdylib", "rlib"]`, workspace member.
- `clack_export_entry!(SinglePluginEntry<TestGainClapPlugin>)` in
  `src/lib.rs`; descriptor id `org.ruviform-test.clap-test-gain`,
  vendor `"Ruviform Test"`, name `test_gain`, version `0.0.0`,
  features `audio-effect` + `stereo` — distinct from the adapter
  fixture's `org.ruviform.clap-gain-poc` / `"Ruviform"` so both modules
  coexist in one host scan.
- `src/gain.rs`: independent `TestGainProcessor` implementing the
  neutral `PluginProcessor` — `out[c][n] = in[c][n] * gain(n)` with
  plain `[0.0, 2.0]` default `1.0` gain, sample-accurate event offsets,
  same-offset last-wins, zero-frame flush, per-activation reset.
- `src/block.rs`: CLAP→neutral translation helpers — `InputEvents`
  `ParamValueEvent` (plain) → normalized `ParamValue` events;
  `PlanarBufferView`/`PlanarBufferViewMut` construction over host
  channel buffers with raw-pointer validation (null, alignment,
  in-place overlap classification); atomic lifecycle + `AtomicU64`
  param mirror; per-instance `BlockScratch`/`EventBuffer` sized at
  `activate`.
- `src/plugin.rs`: clack `Plugin`/`PluginShared`/`PluginMainThread`/
  `PluginAudioProcessor` impls plus `PluginAudioPorts` and
  `PluginParams`/`PluginAudioProcessorParams` extension impls; lifecycle
  mapped onto neutral `LifecycleState` with honest rejection of
  protocol violations; `process` self-loop gate.
- Tests: `#[cfg(test)]` unit/lifecycle/process/ABI tests driven through
  `clack-host` fixtures (in-process, no mocks of the plugin side), an
  `rt_` `assert_no_alloc` process-path test, and
  `tests/host_path.rs` driving the built cdylib through the production
  `ClapLibrary` bridge.
- Deps: `clack-plugin`/`clack-common`/`clack-extensions` `=0.2.0`
  (SDK-equivalent binding layer, same pin as the adapter — recorded in
  `Cargo.toml` comments and this doc), `ruviform-plugin-core`,
  `ruviform-audio-core`; dev-deps `assert_no_alloc = "1.1"`,
  `clack-host = "=0.2.0"` (features `clack-plugin`), and
  `ruviform-plugin-clap = { path, features = ["host"] }` — dev-dep edge
  only, adapter sources untouched.

### Out of Scope

- Reusing `ruviform-plugin-clap` internals for plugin-side logic —
  independence is the anti-self-verification requirement.
- State extension, MIDI/transport, editor, fault injection, L1 runner,
  external validators, other OS lanes, consumer-oracle cross-checks.
- Any modification outside the leased paths (adapter sources,
  `mise.toml`, `.github/**`, VST3 fixture crate, policies).

## Components and Boundaries

- `lib.rs` — crate docs (test-only declaration, independence boundary,
  ABI entry) + `clack_export_entry!` (macro emits its own
  `#[allow(unsafe_code)]`; the crate writes no `unsafe`).
- `gain.rs` — pure DSP/param core on neutral contracts; no clack types.
- `block.rs` — the only clack↔neutral boundary helpers; owns scratch,
  event translation, view construction, atomic state.
- `plugin.rs` — clack trait impls (thin shell over `block`/`gain`) and
  all tests.
- `tests/host_path.rs` — production-path integration test; resolves the
  cdylib beside `current_exe()` under `deps/` (assert-fail if missing,
  never silent-skip).

## Data and Control Flow

1. Host `dlopen`s the cdylib → `clap_entry` → `SinglePluginEntry` →
   `PluginDescriptor` (`test_gain`).
2. `plugin.init` → `new_shared` runs `LifecycleEvent::Initialize`;
   `new_main_thread` failure maps to `InitFailed`.
3. `plugin.activate` validates `PluginAudioConfiguration` (sample rate
   > 0, `min_frames ≤ frames ≤ max_frames`, stereo main ports), sizes
   `BlockScratch`, maps `Activate`; failure maps `ActivateFailed` and
   leaves the machine at `Initialized`.
4. `plugin.start_processing` → `StartProcessing`; `clap_process`
   requires `Processing` (self-loop gate rejects otherwise).
5. `clap_process`: sync processor gain from the atomic mirror →
   translate `InputEvents` `ParamValueEvent`s (plain → normalized,
   frame-offset sorted, last-wins per offset) → build borrowed planar
   views over the host's channel buffers (in-place pairs scale the
   shared region without aliased slices) → `PluginProcessor::process`
   → store post-block gain back to the mirror → map
   `ProcessStatus` → `ProcessStatus` (`Sleep` when no audio ports).
6. `params` extension: `get_info`/`get_info_by_id` declare `Gain`
   (plain `[0.0, 2.0]`, default `1.0`, automatable, dimensionless);
   main-thread `get_value`/`flush` read/fold the mirror.
7. `stop_processing` → `StopProcessing`; `deactivate` → `Deactivate`;
   `Shared::drop` (`plugin.destroy`) → `Destroy`.

## Parameter Domain Contract

CLAP `params` speaks the **plain** domain on the wire
(`ParamValueEvent.value` is plain; `ParamInfo` declares min/max/default
in plain). The neutral `EventKind::ParamValue`/`ParamValue` path is
**normalized** `[0,1]`. `block::translate_events` converts plain →
normalized via the declared `[0.0, 2.0]` range — deliberately
non-identity (`plain = normalized * 2`), so a host or plugin that
confuses the domains produces measurably wrong golden output. The
production host bridge performs the mirror-image `denormalize` on its
side (`host.rs` `push_events`), so the round trip exercises both
mappings.

## RT Contract

`clap_process` performs no allocation, blocking, waiting, or I/O:
channel/event scratch and `EventBuffer`s are allocated once at
`activate`; audio uses zero-copy `PlanarBufferView` views over host
buffers; the only synchronization is three `AtomicU64` ops per block.
Hostile inputs are bounded at the boundary: frame count is checked
against the negotiated `max_frames_count`, channel counts must match
the declared stereo ports, sample-format pairs must be consistent, and
in-place classification is validated per channel pair before any slice
is formed. Verified by `rt_process_drives_paired_stereo_block_allocation_free`
under `assert_no_alloc` (collected by `cargo test --workspace rt_`).

## Risks and Mitigations

- **clack API drift / misuse** — pinned `=0.2.0` identical to the
  adapter; compile + host-path tests exercise the real ABI surface.
- **Domain-mapping bugs** — non-identity range plus golden vectors at
  offset 8 catch both host-side and plugin-side mapping errors.
- **Self-verification bias** — plugin-side logic is independently
  written; only the `clack-*` binding layer and neutral contracts are
  shared with the adapter.
- **Silent RT regressions** — `assert_no_alloc` aborts the test binary
  on any allocation inside the process window.

## Deferred Design Concerns

- DSP-law commonization across format test crates — deferred per suite
  design until a third format or shared requirement appears.
- `test_stateful` — blocked on production host bridges lacking
  `save_state`/`restore_state` (adapter feature gap; separate Issue).
- External CLAP validator and L1 runner coverage — subsequent slices.
