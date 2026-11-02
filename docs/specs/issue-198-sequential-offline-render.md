---
id: "SPEC-Yr26o4c8Q0cCi56S9vjvsF"
type: "spec"
status: "approved"
title: "Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "docs/discovery/issue-198-sequential-offline-render.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/offline-determinism-scope.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-198-sequential-offline-render.md"
  derived-by:
    - "docs/designs/issue-198-sequential-offline-render.md"
    - "docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md"
    - "docs/test-specs/issue-198-sequential-offline-render.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-198-sequential-offline-render.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #198 — Stateful Sequential Offline Render spec

## Purpose

Define a generic, non-RT sequential render path that renders a `PreparedPlan` over a transport program into a staged media file, preserving Graph/Time/Event/Plugin state semantics, and reports a typed receipt — never exposing incomplete output as complete.

## Scope boundaries (from Issue #198)

1. **Prepared Offline Instance** — same Graph/Time/Event semantics as Live; independent stateful processor instance; fixed Snapshot/Capability/Initial State/Channel Layout/Sample Rate/Range.
2. **Sequential Driver** — `[start,end)` processed in monotonically increasing Engine Frames; arbitrary block segmentation must produce bit-comparable results within a defined precision; no temporal parallelization over stateful/IR/PDC/plugin state.
3. **Time/State** — Tempo/Loop/Span/Timed-Event/Plan-Generation and plugin `ProcessConfig` capability validation; unsupported Exact Render rejected at preflight, never silently degraded.
4. **Preroll/PDC/Tail** — warm-up frames distinct from output frames; latency alignment; known/estimated/unknown tail classification; undefined tails never silently cut.
5. **Output lifecycle** — PCM convert/SRC/dither (when applicable) → Encoder → Drain → Muxer Finalize → Close/Sync → Reopen/Verify → Publish Receipt via the #191 WAV backend; typed results for unsupported/error/cancel/output failure.
6. **Safe staging** — partial output never reported complete; explicit destination-overwrite policy; no blanket atomic-rename/fsync claim.
7. **Progress/Cancel** — non-RT job ownership, explicit cancel points; cancel during drain/finalize/publish never produces contradictory receipt/file state.

## Functional requirements

- `FR-1 PreparedRender`: compile a `ValidatedGraph` into an isolated `GraphRuntime`+`RtRuntime` with per-node `OfflineRenderSupport` capture; separate from any Live runtime (caller's factory produces fresh instances).
- `FR-2 Capability gate`: `GraphProcessor::offline_support()` (defaulted → `Unknown`); preflight rejects non-`Verified` processors as `RenderError::Preflight(PreflightError::OfflineUnsupported{node})`.
- `FR-3 Transport program`: `RenderTransport<P>` = `TransportScheduler` + pre-staged `(TransportCommand, due EngineFrame)` list + `(Arc<TransportPlan<P>>, due EngineFrame)` installs; each command `preflight`ed before staging; applies observed via receipts/faults.
- `FR-4 Sequential drive`: chunked `run_callback` over the job window; `deliverable=false` spans → `RenderError::TransportUndeliverable`; spans partitioned at event engine-frames and by `BlockContract`; events mapped to `NodeEvent` and delivered before the covering `process` call.
- `FR-5 Frame algebra`: boundary input fed at span position; emitted position `= span_position + j - total_latency`; emit iff `∈ [range_start, range_end + tail_frames)`; non-Playing spans processed but not emitted.
- `FR-6 Output lifecycle`: emitted runs → `AudioFrame` → `Encoder::send_frame` → pump `receive_packet` → `Muxer::write_packet`; after drive: `encoder.drain()` + pump, `muxer.finalize()`, drop muxer (close), optional `verify(temp_path, facts)`, then `publish(PublishOptions)`.
- `FR-7 Typed errors`: `Preflight` / `Unsupported` / `Cancelled` / `Output` / `Finalize` / `Publish` / `Verify` / `DestinationExists` / `TransportUndeliverable` / `ScheduleFault` / `CommandRejected` / `EventChannelFull` / `Segmentation`; every failure path leaves `StagedOutput` to clean its temp file on drop.
- `FR-8 Cancel points`: before each `process` call, before drain, before finalize, before publish → `RenderError::Cancelled`; receipt/status/file state stay consistent (no receipt, dest untouched unless published).
- `FR-9 Progress`: `Progress::reset(job_frames)` + `bump_by(frames)` per processed block (new additive `Progress::bump_by`).
- `FR-10 Receipt`: `RenderReceipt { path, muxer_receipt, frames_emitted, range, tail_evidence, blocks_processed, plan_generation }`.

## Non-goals

- Synvelis Export Recipe / Stem / Freeze / Bounce / Project Commit — consumer-side policy.
- SRC/dither implementations — caller supplies encoder/muxer; the pipeline only sequences them.
- Resume/retry of failed renders — staging is safely discardable; retry = new `render` call.
- Temporal parallelism — explicitly prohibited over stateful regions; independent-chunk `OfflineDriver` remains the only parallel path.

## Acceptance Criteria

- OF-01: deterministic stateful fixture renders bit-exact PCM across continuous and 64/128/256-frame blockings.
- OF-02: mid-stream range start, preroll, PDC alignment, and a known tail select exactly the specified output frames.
- OF-03: tempo/loop boundaries and same-frame events are applied sequentially in transport order.
- OF-04: render state is independent of any Live instance; unsupported capabilities reject at preflight before any output exists.
- OF-05: published WAV re-opens and decodes with matching FrameCount/Channels/Rate/PCM via an independent reader.
- OF-06: injected cancel/disk-full/write/close/sync/destination-exists/verify failures produce typed errors with no partial or overwritten output.
- OF-07: recorded `cargo test --locked` package + workspace runs and `mise run` gates pass.

## Acceptance mapping (OF-01..OF-07 → test spec)

- OF-01: stateful fixture, continuous vs 64/128/256-block equality → `of01` tests.
- OF-02: mid-stream range start + preroll + PDC + known tail → `of02` tests.
- OF-03: tempo/loop boundary + same-frame events → `of03` tests.
- OF-04: capability gate + Live independence → `of04` tests (mock-only; real-plugin gate deferred to #196).
- OF-05: WAV reopen + decode verify → `of05` tests.
- OF-06: cancel/disk-full/write/close/sync/dest-exists/verify-fail injection → `of06` tests.
- OF-07: `cargo test -p ruviform-audio --locked` + `cargo test --workspace --locked` records (note: driver lives in `ruviform-audio` per layer rules — the `-p ruviform-exec` run is also recorded for the legacy-API regression check).
