---
id: "IMPL-issue196-m4-adapter"
type: "impl"
status: "completed"
title: "Issue #196 — M4 typed graph plugin adapter (multi-bus / lifecycle / offset precision)"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/e2e.rs"
      - type: "file"
        path: "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
      - type: "file"
        path: "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
      - type: "file"
        path: "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
      - type: "file"
        path: "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
      - type: "file"
        path: "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
      - type: "file"
        path: "docs/test-specs/issue-196-typed-graph-plugin-adapter-tests.md"
      - type: "file"
        path: "docs/tasks/issue-196-t-1-plugin-core-bus-enumeration.md"
      - type: "file"
        path: "docs/tasks/issue-196-t-2-host-media-bus-enumeration.md"
      - type: "file"
        path: "docs/tasks/issue-196-t-3-bridge-crate.md"
      - type: "file"
        path: "docs/tasks/issue-196-t-4-verification-evidence.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/bus.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-196-t-1-plugin-core-bus-enumeration.md"
    - "../../tasks/issue-196-t-2-host-media-bus-enumeration.md"
    - "../../tasks/issue-196-t-3-bridge-crate.md"
    - "../../tasks/issue-196-t-4-verification-evidence.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-issue-196-typed-graph-plugin-adapter.md"
    - "../../designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "../../specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "../../adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
    - "../../discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "../../test-specs/issue-196-typed-graph-plugin-adapter-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #196 implementation record — M4 typed graph plugin adapter

## What was built

- `ruviform-plugin-core`: `BusRole` (Main/Aux/Sidechain/Send/Return)
  replaces `main: bool` on `BusDescriptor`; `Plugin::buses()` +
  `ErasedPlugin::buses()` enumerate the declared bus table (non-RT).
- `ruviform-plugin-host`: `BusReport::{Reported, Unavailable}` +
  `PluginHost::buses(handle)` — L0 reports, L1 stays `Unavailable`
  (no wire verb; honest, never inferred).
- `ruviform-media-core`: bounded `pop_receipt`/`pop_fault` on
  `TransportScheduler` for RT-adjacent report draining.
- `ruviform-audio-plugin-bridge` (new, consumer layer): `attach`,
  `bus_to_port_def`/`validate_buses`, `PluginNode` (scheduler +
  endpoint, `SpanSink` → legal `BlockContract` partitions,
  process-start-relative `EventList`, silent slabs for
  unconnected/disabled buses, no implicit downmix), bounded
  `PluginNodeControl`/`NodeReport` SPSC, `PlanVault`, `PluginNodeFactory`,
  honest `AdapterCapability` matrix per backend.

## Measured evidence (M4 gates)

- `cargo test -p ruviform-audio-plugin-bridge` — 5 E2E tests green:
  - `multi_bus_audio_routes_and_processes`: Main stereo + Sidechain
    mono in, Main stereo + Aux stereo out — gain verified per channel
    (M4-02, multi-bus・mono/stereo).
  - `loop_100_min_128_zero_violating_calls`: 256-frame callback,
    Loop=100, min=128 → zero calls outside [128,256], no 100-frame
    call, `UndeliverableSpan` audited (M4-03 rejection case).
  - `loop_200_partial_delivery_no_violations`: 200-frame lap delivered
    as one legal call; 56-frame remainder audited, never called.
  - `event_offsets_are_process_start_relative`: max=128 splits
    256 → 2 calls; event at position 200 observed at offset 72 in
    call 2 (process-start-relative, not engine-relative) (M4-04).
  - `capability_matrix_is_honest`: L0 `Verified`, L1
    `bus_enumeration=Unavailable` (M4-01).
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` — all
  green (workspace-wide, no regressions).
- Legacy single-bus path preserved: `PluginEndpoint::process_audio`
  untouched; bridge is additive.

## Known gaps (named, not hidden)

- L1 bus enumeration needs a wire verb — declared `Unavailable`.
- CLAP in-block transport events (`in_block_transport`) stay
  `Declared` until real-plugin evidence exists (not adopted).
- Graph Swap / Retire-overflow / crash convergence are covered by the
  host's existing fault suite (`PluginHost` lifecycle convergence);
  the bridge adds `VaultExceeded`/`EndpointError` reports but does not
  re-verify the full L1 crash matrix end-to-end.
- `verify-ci` / `test-l1-real-plugin` / `lint-l1-real-plugin` lanes
  not run in this worktree pass (CI-profile lanes; dev lanes green).
