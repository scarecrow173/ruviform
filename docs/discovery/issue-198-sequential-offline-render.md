---
id: "DISC-ubO5TxPGQDZSaLVHth3jpg"
type: "discovery"
status: "resolved"
title: "Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt — inventory, placement, contract shape"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/198"
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
    - "docs/designs/audio-engine.md"
    - "docs/adr/offline-determinism-scope.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt — discovery

## Summary

Issue #198 (M6) needs a **stateful sequential offline render path**: a prepared graph instance independent of Live playback, processing `[start,end)` in monotonically increasing Engine Frames with preroll/PDC/tail accounted for, and a fully typed output lifecycle (encode → mux finalize → close/sync → reopen/verify → publish receipt) that never reports partial output as complete. The inventory shows all primitives already exist but are split across three crates with a layer-rule constraint deciding placement:

- `ruviform-audio` (engine tier) owns the stateful `RtRuntime`/`GraphProcessor`/`PreparedPlan` machinery and already depends on `ruviform-media-core` (transport scheduler). The layer baseline **forbids `ruviform-exec` from depending on `ruviform-media-core`**, so the sequential driver must live in `ruviform-audio` (or a new engine-tier crate) — not in `ruviform-exec` where the legacy `OfflineDriver` sits.
- `ruviform-media-io` owns `StagedOutput`/`PublishOptions`/`MediaSink`/`Encoder`/`Muxer` — the checked output lifecycle. `WavMuxer::finalize` is the only checked completion path; `Drop` is explicitly not a guarantee.
- `ruviform-exec` owns `OfflineJob`/`CancellationToken`/`Progress` (non-RT job lifecycle) and the legacy `OfflineDriver` (independent-chunk semantics only — its docs already disclaim state-across-chunk determinism).

Tentative conclusion (confidence: high): add a `ruviform_audio::render` module containing `PreparedRender` (compile-once graph instance) + `render()` sequential driver + `RenderOutput` lifecycle bundle + typed `RenderError`/`RenderReceipt`, with an additive `GraphProcessor::offline_support()` capability hook. `ruviform-exec` keeps its current API unchanged; the new driver uses `OfflineJob`/`CancellationToken`/`Progress` via a new dependency edge `ruviform-audio → ruviform-exec` (exec is a primitives-tier crate; engine → primitives edges are permitted and already exist, e.g. audio → rt/exec-adjacent foundations).

## Exploration Goal

Confirm where the sequential render driver lives, which contracts already exist vs must be built, what the frame/position algebra is, and what capability-gating looks like — so the spec/ADR pair fixes scope before implementation.

## Key Findings

### Layer rules decide placement (decisive constraint)

`docs/designs/ruviform-architecture-design-baseline.md` dependency table:
- `ruviform-exec` allowed deps: **Primitives only** — `ruviform-media-core` is **not** allowed.
- `ruviform-audio` (engine tier) allowed deps: domain foundations + primitives + platform adapters — already deps `ruviform-media-core`, `ruviform-graph`, `ruviform-rt`, `ruviform-audio-core`.

The transport scheduler (`ruviform_media_core::transport_schedule`) is required for Time/Event semantics (tempo maps, loops, spans, timed events with `EngineFrame` delivery). Therefore the driver **cannot** live in `ruviform-exec`. `ruviform-audio` is the natural owner (it owns `RtRuntime` and the `GraphProcessor` contract).

### Existing foundations (reused, not rebuilt)

- **Typed runtime** (`ruviform_audio::typed`, Issue #195): `Compiler::compile(&ValidatedGraph, &mut dyn NodeFactory, CompilerConfig) -> CompiledPlan { plan: Arc<PreparedPlan>, installs }`; `GraphRuntime::publish(plan, installs, ctx)` queues generation-tagged installs and publishes the immutable plan; `RtRuntime` processes bounded blocks (`rt.process(frames, &RtContext)`) with preallocated slots and PDC `DelayLine`s; `input_writer(port, frames)`/`output_reader(port)` bound the boundary slots. `PreparedPlan.total_latency` = accumulated path latency incl. PDC alignment.
- **Processor contract**: `GraphProcessor::{activate, event(&NodeEvent), process(&mut NodeIo, &RtContext), deactivate, latency()}`. `PrepareOutcome::{New, Reuse, Boundary}` — a fresh compile with `New` outcomes produces a fully independent processor set (Live independence = compile a separate `GraphRuntime`, never share processors).
- **Transport** (`ruviform_media_core::transport_schedule`, Issue #190): `TransportScheduler` stages `TransportCommand`s (Play/Seek/Stop/SetTempo/SetTimeSignature/SetLoop/…), `run_callback(EngineFrame, frames, sink)` emits ordered `TransportSpan`s each with `SpanContext { position_at_offset, plan_generation, status, … }`; `preflight` simulates command boundaries; per-node events live in `TransportPlan<P>` (installed via `install_plan`), with `events_in_window`/frame-resolution helpers; `drain_receipts`/`drain_faults` surface rejected/degraded applies and span-budget overruns; `Span.deliverable` flags physically undeliverable segments (min-frames violations).
- **Output lifecycle** (`ruviform-media-io`, Issue #191): `Encoder` (send_frame/receive_packet/drain), `Muxer` (write_packet/finalize→`Receipt`), `StagedOutput { temp_path, sink() -> &mut File, publish(PublishOptions) }`, `PublishOptions { sync_before_publish, overwrite }`. `MediaSink = Write + Seek + Send` — `&mut dyn MediaSink` satisfies it, enabling fault-injection wrappers (`FailSink`).
- **WAV backend** (`ruviform-io-wav`): `WavEncoder` planar-f32 → interleaved PCM packets; `WavMuxer` header-at-construction, `finalize` patches sizes and returns `WavReceipt`; second finalize → `InvalidData`.
- **Reopen/verify** (`ruviform-io-symphonia::open_demuxer` + `Demuxer::make_decoder`, Issue #197 boundary): decode the produced file back to planar f32 and compare frames/channels/rate/values.
- **Job/cancel** (`ruviform-exec`): `CancellationToken`/`Progress` are standalone non-RT types usable without `OfflineJob`'s thread; `OfflineDriver`'s independent-chunk API stays untouched (additive requirement).

### Contract shape decided here

- `GraphProcessor::offline_support() -> OfflineRenderSupport` — additive defaulted trait method returning `Unknown`; `Declared`/`Verified` are opt-in. Exact Render preflight rejects any processor that is not `Verified` (mock/plugin separation: mocks declare `Verified` in tests; real-plugin gate lands with Issue #196 transport capability).
- `PreparedRender` = `GraphRuntime` + `RtRuntime` + `Arc<PreparedPlan>` + captured per-node `OfflineRenderSupport` map. Compiled from `&ValidatedGraph` + `&mut dyn NodeFactory` + `CompilerConfig`; the caller's separate factory instance guarantees processor independence.
- `RenderSpec` fixes: sample rate, channel layout, output port, `[range_start, range_end)` position range, `TailSpec`, `BlockContract`, job engine-frame window. Fixed at prepare; Live edits cannot affect it (snapshot semantics = the `ValidatedGraph` is `Clone`-immutable input).
- `TailSpec::{None, Known(frames), Estimated(frames)}` — `Estimated` renders the extra frames but the receipt marks `tail_evidence: Estimated`; `Unknown` is rejected at preflight (never silently truncate).
- Emit rule: output frame j of a block at engine frame f (span position `p0`) maps to position `p0 + j - plan.total_latency`; emitted iff in `[range_start, range_end + tail)`; preroll = the `-total_latency` region plus range-start-warmup processed but not emitted.
- `render(prepared, transport_program, spec, feeder, output_plan, progress, cancel) -> Result<RenderReceipt, RenderError>` — non-RT, single-threaded sequential loop; `run_callback` chunked (bounded spans), spans→segments cut at event engine-frames→physical `process` calls partitioned by `BlockContract`; events sent as `NodeEvent` before the covering call.
- `RenderOutput` bundle: `StagedOutput` + muxer factory `FnOnce(&mut dyn MediaSink) -> Result<M>` + `Box<dyn Encoder>` + `PublishOptions` + optional verify hook `Fn(&Path, &OutputFacts) -> Result<()>` + publish closure (default `StagedOutput::publish`; injectable for failure-class testing).
- `RenderError` typed variants: `Preflight`/`Unsupported`/`Cancelled`/`Output`/`Finalize`/`Publish`/`Verify`/`DestinationExists`/`TransportFault`/`EventChannelFull`/`Segmentation` — every failure cleans the temp file via `StagedOutput` drop; destination untouched unless `publish` succeeded.

## Alternatives Considered

1. **Driver in `ruviform-exec` next to `OfflineDriver`** — rejected: layer rules forbid `exec → media-core`; transport semantics are required, and genericizing the driver over "any time source" would re-create the scheduler contract (DRY violation).
2. **New crate `ruviform-offline`** — rejected for now: the driver is an audio-engine concern (RT path owner), the new-crate overhead (layer table, index regeneration) outweighs the benefit; can be extracted later without API change.
3. **Reuse `OfflineDriver` with a "stateful" flag** — rejected: its contract is chunk-independent; bolting sequencing onto it blurs the guarantee boundary the issue demands stay explicit.

## Open Risks

- `audio → exec` dependency edge is new; validated against the layer table (exec is primitives-tier) — ADR records it.
- Plugin `ProcessConfig` capability (Issue #196) is still in flight → `offline_support` is the seam; real-plugin verification gate is deferred and documented.
- Event→engine-frame mapping inside non-Playing spans: scheduler only resolves event frames while Playing; non-Playing span output is not emitted (position does not advance) — documented.
