---
id: "IMPL-issue198-sequential-offline-render"
type: "impl"
status: "completed"
title: "Issue #198 — stateful sequential offline render: prepared instance, PDC/preroll/tail, safe output receipt"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/198"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/render.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "docs/discovery/issue-198-sequential-offline-render.md"
      - type: "file"
        path: "docs/specs/issue-198-sequential-offline-render.md"
      - type: "file"
        path: "docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md"
      - type: "file"
        path: "docs/designs/issue-198-sequential-offline-render.md"
      - type: "file"
        path: "docs/plans/implement-issue-198-sequential-offline-render.md"
      - type: "file"
        path: "docs/test-specs/issue-198-sequential-offline-render.md"
      - type: "file"
        path: "docs/tasks/task-198-t1-render-core.md"
      - type: "file"
        path: "docs/tasks/task-198-t2-driver-output.md"
      - type: "file"
        path: "docs/tasks/task-198-t3-tests-verification.md"
      - type: "file"
        path: "docs/impl/ir/issue-198-sequential-offline-render.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/mod.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/runtime.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_runtime.rs"
      - type: "file"
        path: "crates/ruviform-exec/src/offline.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/task-198-t1-render-core.md"
    - "../../tasks/task-198-t2-driver-output.md"
    - "../../tasks/task-198-t3-tests-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-issue-198-sequential-offline-render.md"
    - "../../designs/issue-198-sequential-offline-render.md"
    - "../../specs/issue-198-sequential-offline-render.md"
    - "../../adr/issue-198-sequential-offline-render-placement-and-contracts.md"
    - "../../discovery/issue-198-sequential-offline-render.md"
    - "../../test-specs/issue-198-sequential-offline-render.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #198 implementation record — stateful sequential offline render

## Summary

Implemented Issue #198: a stateful sequential offline render driver in
the new `ruviform-audio::render` module — a fresh prepared instance per
render, preroll/PDC/tail handling with typed tail evidence, ordered
event/transport delivery, staged output with independent reopen
verification, and a `RenderReceipt`. Also fixed a root-cause channel
configuration bug in `RtRuntime::input_writer` found by the render
tests. Verified by 7 render tests (OF-01..OF-07) plus workspace lanes.

## Context

- Source: <https://github.com/scarecrow173/ruviform/issues/198>
- Tasks: [../../tasks/task-198-t1-render-core.md](../../tasks/task-198-t1-render-core.md), [../../tasks/task-198-t2-driver-output.md](../../tasks/task-198-t2-driver-output.md), [../../tasks/task-198-t3-tests-verification.md](../../tasks/task-198-t3-tests-verification.md)
- Upstream: [plan](../../plans/implement-issue-198-sequential-offline-render.md), [design](../../designs/issue-198-sequential-offline-render.md), [spec](../../specs/issue-198-sequential-offline-render.md), [ADR](../../adr/issue-198-sequential-offline-render-placement-and-contracts.md), [discovery](../../discovery/issue-198-sequential-offline-render.md), [test-spec](../../test-specs/issue-198-sequential-offline-render.md)

## Implementation

### Changes

- `ruviform-audio::render` (new module, engine layer — `ruviform-exec`
  cannot see media types per the layer baseline): the sequential
  offline render driver.
  - `PreparedRender::compile` — fresh `GraphRuntime`/`RtRuntime` +
    compiled `PreparedPlan` per render; processor instances are
    independent of any live runtime; plan, capabilities, geometry,
    sample rate, and range are frozen at preparation.
  - `GraphProcessor::offline_support` → `OfflineRenderSupport`
    (`Unknown`/`Declared`/`Verified`); exact rendering requires
    `Verified` for every installed processor — `Unknown`/`Declared`
    reject at preflight (`PreflightError::OfflineUnsupported`).
  - `RenderTransport` — `TransportScheduler` (Exact delivery policy)
    plus staged `TransportPlan`s and due commands; plans install
    before commands; rejected/degraded receipts are typed preflight
    errors, never silently deferred.
  - `RenderSink` (`SpanSink`) — monotonically increasing Engine Frames;
    events become segment cut points and are delivered before the
    physical call covering their frame; each segment partitions under
    `BlockContract` ∩ plan `max_frames`; no temporal parallelization.
  - Emit math `position − total_latency + sample_offset`; warm-up
    frames processed but never emitted; emission window
    `[range_start, range_end + tail)`; `TailSpec::{None,Known,Estimated}`
    → receipt `TailEvidence`; undefined tails cannot be silently
    truncated (caller must pick a variant).
  - Output lifecycle: feeder → `RtRuntime::process` → boundary-slot
    capture → `Encoder::send_frame`/`drain` → `Muxer::write_packet`/
    `finish` → independent reopen `verify(temp, facts)` → `publish`.
    `StagedOutput` temp sibling; destination untouched on every
    failure; `PublishOptions::{no_overwrite,replace}` expose the
    caller policy. The muxer receives an owned `try_clone`d `File`
    (shared file description), so `publish`'s `sync_all`/`rename` sees
    the finalized bytes while the constructor bound stays
    lifetime-free (`FnOnce(File) -> Box<dyn Muxer>`).
  - `Progress` bumps per processed engine frame; `CancellationToken`
    checked at every physical call and at drain/finalize/verify/publish
    boundaries — `Cancelled{phase}` is typed.
  - `RenderReceipt`: path, frames_emitted, blocks_processed,
    plan_generation, range, range_end_with_tail, tail_evidence,
    stale_events_skipped.
  - `RenderError`: Preflight (incl. `OfflineUnsupported`, `Stage`),
    `Cancelled{phase}`, `Process`, `TransportUndeliverable`,
    `CommandRejected`, `Segmentation`, `EventOverflow`, `Output`,
    `Drain`, `Finalize`, `Verify`, `DestinationExists`, `Publish`.
- `typed::RtRuntime::input_writer` now configures the slot's logical
  `channels` from the pending plan when handing out the writer —
  previously `channels == 0` until `activate_generation` ran inside
  the first `process`, so channels-gated feeds on the first callback
  were silently dropped (root cause found by the render tests; fixed
  at the owning layer).
- `tests/common::MockDsp` gained `with_support`/`offline_support`.
- `crates/ruviform-exec::offline` unchanged in guarantee scope — the
  independent-chunk `OfflineDriver` keeps its existing contract; the
  new driver does not broaden it.

### Decisions During Implementation

- The driver lives in `ruviform-audio` (engine layer) because
  `ruviform-exec` cannot see media types per the layer baseline.
- Exact rendering requires `OfflineRenderSupport::Verified` for every
  installed processor; `Unknown`/`Declared` reject at preflight
  (`PreflightError::OfflineUnsupported`) rather than approximating.
- `DeliveryPolicy::Exact` is required; degraded/approximate policies
  are rejected rather than emulated.
- `TailSpec::{None,Known,Estimated}` keeps undefined tails explicit —
  the caller must pick a variant and nothing is silently truncated;
  `TailEvidence` reports which variant was used.
- Output goes through `StagedOutput` temp sibling + independent reopen
  `verify` + `publish`; the destination is untouched on every failure.
- The `input_writer` channel fix was made at the owning layer
  (`typed/runtime.rs`) rather than worked around in the driver.

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- `cargo test -p ruviform-audio --test render` — 7 tests green:
  - `of01_block_size_invariance`: `OnePole` stateful render at
    continuous (768) vs 64/128/256 blocks — emitted PCM identical to
    the continuous reference on every variant (OF-01).
  - `of02_midstream_preroll_pdc_tail`: (a) range [200,600) over job
    [0,600) — warm-up processed, never emitted; emitted PCM = reference
    [200,600); first pts = 200. (b) declared latency 96 →
    `position − 96` emit alignment; `TailSpec::Known(64)` → emitted
    576 frames, `tail_evidence=Known`, `range_end_with_tail=576`
    (OF-02 preroll/PDC/tail).
  - `of03_events_loop_transport_order`: tempo plan install, loop
    boundary, same-frame events — events delivered before the
    covering physical call, in transport order; receipt
    `stale_events_skipped` counted (OF-03).
  - `of04_live_independence_and_capability_gate`: render instance
    disjoint from a live runtime on the same graph (live log
    untouched, still processes after); `Unknown`/`Declared` reject at
    preflight with no destination and no temp left (OF-04).
  - `of05_wav_reopen_verify`: real `WavEncoder`/`WavMuxer` into
    `StagedOutput`; verify hook reopens the temp through
    `ruviform-io-symphonia` demuxer→decoder — 1 stream, Stereo/48 kHz,
    decoded frames == emitted, PCM == analytic one-pole prediction
    (OF-05).
  - `of06_failure_injection` + `of06_staging_policy`: pre-cancelled →
    `Cancelled{process}`; `FailSink` byte budget → `Output`/`Finalize`;
    `FailSink` flush → `Finalize`/`Output`; publish failure →
    `Publish`; pre-existing destination → `DestinationExists` with
    content byte-for-byte unchanged; verify rejection → `Verify`. In
    every case: typed error, no destination created (or untouched),
    zero `.rvf-tmp-*` leftovers (OF-06).
- `g2_06_channels_gated_feed_before_first_process_lands`
  (`tests/typed_runtime.rs`): regression for the `input_writer`
  channel-configuration fix.
- OF-07 command record (Windows 11 x64):
  - `cargo test -p ruviform-audio --locked` — 8 test targets, all
    `test result: ok` (incl. the 7 render tests above).
  - `cargo test -p ruviform-exec --locked` — all targets ok (offline
    driver suite unchanged).
  - `cargo test --workspace --locked` — 109 test targets, 0 failures.
  - `mise run fmt-check` / `check` / `lint` / `test` (106 targets) /
    `test-rt` (73 targets) / `verify-ci` (CI profile: fmt-check,
    check-ci, lint-ci, test-ci — 109 targets) — all green.
- `origin/main` merged (PR #232, Issue #196) before final
  verification; `Cargo.lock` regenerated via `cargo
  generate-lockfile`.

## Risks

- `InputFeeder` covers only the boundary-input contract; file/source
  decode feeding (Issue #197 `SourceReader` integration) is not part
  of this driver — renders needing decoded sources must integrate it
  downstream.
- Plugin `ProcessConfig` capability (Issue #196, in parallel) is not
  wired into the render gate; plugin-specific exact-render capabilities
  arrive with the plugin adapter's own fixtures.
- `DeliveryPolicy::Exact` is required; degraded/approximate policies
  are rejected rather than emulated, so callers needing approximation
  cannot use this driver.
- `tail=Estimated` emits the caller's estimate and reports it as
  `TailEvidence::Estimated` — it does not model plugin decay curves.
- Evidence is Windows 11 x64 only; other platforms are unverified in
  this record (the CI matrix covers them on merge).

## Follow-ups

- Issue #197 `SourceReader` integration for file/source decode
  feeding.
- Plugin-adapter fixtures for plugin-specific exact-render
  capabilities (with the #196 `ProcessConfig` wiring).
- Non-Windows platform verification via the CI matrix on merge.

## Consumer-impact classification

- synvelis: no impact — additive module in `ruviform-audio`; no
  existing public signature changed. `GraphProcessor::offline_support`
  is a defaulted trait method — existing impls keep `Unknown` and are
  unaffected unless they opt into the render gate.
- audio-lens: no impact — same reasoning; the existing
  `ruviform-exec` chunk APIs are untouched.
