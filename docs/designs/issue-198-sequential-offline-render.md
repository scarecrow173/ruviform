---
id: "DESIGN-aRsJILdLBlEulHcYpfH4eb"
type: "design"
status: "approved"
title: "Issue #198 stateful sequential offline render — PreparedRender, sequential driver, staged output lifecycle"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-198-sequential-offline-render.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/audio-engine.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/adr/issue-198-sequential-offline-render-placement-and-contracts.md"
  derived-by:
    - "docs/plans/implement-issue-198-sequential-offline-render.md"
    - "docs/test-specs/issue-198-sequential-offline-render.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Design — Issue #198 sequential offline render

## Overview

`ruviform_audio::render` adds three things to the engine crate:

1. `PreparedRender` — an isolated compiled graph (own `GraphRuntime` + `RtRuntime` + `Arc<PreparedPlan>`) plus the captured per-node offline-capability map.
2. `render()` — a sequential non-RT driver that walks a `TransportScheduler` over the job window, feeds boundary inputs, delivers events, calls `RtRuntime::process` in contract-legal block sizes, and maps output frames through the PDC/range/tail algebra into an `Encoder`/`Muxer`/`StagedOutput` pipeline.
3. Typed `RenderError`/`RenderReceipt` — the honest-status surface.

## Types

```rust
pub enum OfflineRenderSupport { Unknown, Declared, Verified }   // non_exhaustive

// GraphProcessor (additive, defaulted):
fn offline_support(&self) -> OfflineRenderSupport { OfflineRenderSupport::Unknown }

pub struct PreparedRender { /* GraphRuntime, RtRuntime, Arc<PreparedPlan>, caps: BTreeMap<NodeId, OfflineRenderSupport> */ }
impl PreparedRender {
    pub fn compile(graph: &ValidatedGraph, factory: &mut dyn NodeFactory,
                   config: CompilerConfig, cmd_cap: usize, ev_cap: usize) -> Result<Self, RenderError>;
    pub fn plan(&self) -> &Arc<PreparedPlan>;
    pub fn total_latency(&self) -> u64;
    pub fn capabilities(&self) -> &BTreeMap<NodeId, OfflineRenderSupport>;
}

pub enum TailSpec { None, Known(u64), Estimated(u64) }          // non_exhaustive; unknown tails rejected at call site
pub enum TailEvidence { None, Known, Estimated }

pub struct RenderSpec {
    pub sample_rate: SampleRate,
    pub layout: ChannelLayout,
    pub output_port: PortId,            // boundary output (sink input port)
    pub range_start: u64,               // position ticks (frames at sample_rate)
    pub range_end: u64,                 // exclusive
    pub tail: TailSpec,                 // extra emitted frames after range_end
    pub job_start: u64,                 // engine-frame window to drive
    pub job_end: u64,                   // exclusive
    pub callback_chunk: u32,            // run_callback granularity (bounded span count)
}

pub struct StagedCommand { pub command: TransportCommand, pub due: EngineFrame }
pub struct RenderTransport<P> {
    pub scheduler: TransportScheduler<P>,   // caller-built (tempo map, contract, budgets, epoch)
    pub commands: Vec<StagedCommand>,       // each is scheduler.preflight'ed then request()'ed by render()
    pub plans: Vec<(Arc<TransportPlan<P>>, EngineFrame)>,  // install_plan due-frames
}

pub trait InputFeeder {
    /// Fill a boundary input slot for the block starting at `ctx.engine_frame`
    /// (position `ctx.position_ticks`). Called before each `process`.
    fn feed(&mut self, port: PortId, ctx: &FeedContext, slot: &mut Slot<'_>);
}
pub struct FeedContext { pub engine_frame: u64, pub position_ticks: i64, pub frames: usize }
pub struct SilentFeeder;                    // zero-fill (default for input-less graphs)

pub struct OutputFacts { pub sample_rate: SampleRate, pub layout: ChannelLayout,
                         pub frames_emitted: u64, pub range_start: u64, pub range_end_with_tail: u64 }

pub struct RenderOutput<Mk, V, Pb> {
    pub staged: StagedOutput,
    pub make_muxer: Mk,                     // FnOnce(&mut dyn MediaSink) -> Result<M, Error>
    pub encoder: Box<dyn Encoder>,
    pub publish_options: PublishOptions,
    pub verify: Option<V>,                  // Fn(&Path, &OutputFacts) -> Result<(), Error>
    pub publish: Pb,                        // FnOnce(StagedOutput, PublishOptions) -> Result<(), Error>
}

pub struct RenderReceipt { pub path: PathBuf, pub muxer_receipt: Box<dyn Any + Send>,
    pub frames_emitted: u64, pub blocks_processed: u64, pub plan_generation: u64,
    pub range_start: u64, pub range_end: u64, pub tail_evidence: TailEvidence }

pub enum RenderError {
    Preflight(PreflightError),
    Unsupported { what: &'static str },
    Cancelled { phase: &'static str },
    Output { source: ruviform_media_io::Error },
    Finalize { source: ruviform_media_io::Error },
    Publish { source: ruviform_media_io::Error },
    Verify { source: ruviform_media_io::Error },
    DestinationExists { path: PathBuf },
    TransportUndeliverable { span_start: u64, span_frames: u64 },
    ScheduleFault { at: EngineFrame, detail: String },
    CommandRejected { command: String, at: EngineFrame },
    EventChannelFull { pending: usize },
    Segmentation { detail: String },
}
pub enum PreflightError { OfflineUnsupported { node: NodeId, support: OfflineRenderSupport },
                          Command { index: usize, source: PreflightRejection },
                          Range { detail: &'static str } }
```

`MuxReceipt` is generic: `RenderReceipt::muxer_receipt` is boxed `Any` because `Muxer::Receipt` is associated — consumers downcast to e.g. `WavReceipt`. (Chosen over a second generic parameter to keep `render()` object-callable.)

## Drive algorithm

```
prepared = PreparedRender::compile(graph, factory, cfg, cmd_cap, ev_cap)?;
preflight: every capability must be Verified; every staged command scheduler.preflight'ed;
           range/job sanity; output_port must be a boundary output of plan.
stage:    for each plan → scheduler.install_plan(plan, due); for each command → scheduler.request(cmd, due)
drive:    cursor = job_start
          while cursor < job_end:
              chunk = min(callback_chunk, job_end - cursor)
              scheduler.run_callback(EngineFrame(epoch, cursor), chunk, &mut collector)
              drain faults → error; drain receipts → rejected/degraded → error
              for span in collector.spans:
                  if !span.deliverable → TransportUndeliverable
                  plan = plans[span.ctx.plan_generation] (0 → none)
                  cuts  = engine frames of in-span events (> start, < end), sorted dedup
                  for segment in [span cut into pieces at cuts]:
                      for (phys_off, frames) in contract.partition_or_split(segment_len, max_frames):
                          cancel? → Cancelled{phase:"process"}
                          deliver events whose frame == segment_start (before first call only)
                          feed boundary inputs (position = ctx.position_at_offset(seg_off))
                          rt.process(frames, ctx)
                          for each maximal contiguous emitted run in output:
                              frame = AudioFrame(buf, pts=run_start_pos); encoder.send_frame
                              pump packets → muxer.write_packet
                          progress.bump_by(frames)
              cursor += chunk
finish:   cancel? → Cancelled{phase:"drain"}; encoder.drain(); pump
          cancel? → Cancelled{phase:"finalize"}; muxer.finalize() → receipt; drop(muxer)
          cancel? → Cancelled{phase:"verify"}; verify?(temp_path, facts)
          cancel? → Cancelled{phase:"publish"}; staged.exists && !overwrite → DestinationExists
          publish(staged, options) → RenderReceipt
```

Key invariants:

- **Monotonic engine frames**: physical calls always advance; block boundaries are exactly the union of span boundaries, event frames, and contract/legal partitions — the same stream is produced for any legal partition (statefulness is inside processors, unaffected by where calls split).
- **Emit algebra**: `emit_pos = span_position_at_block_start + j − total_latency`, Playing-only. Warm-up (preroll) = positions `< range_start` — processed, not emitted. Tail extends emit window to `range_end + tail`.
- **Consistent failure**: any `RenderError` → `staged` dropped → temp removed → destination untouched unless `publish` already returned `Ok` (which only happens on the success path).
- **Cancel coherence**: cancel is observed at phase boundaries; no receipt is produced and the staged temp is cleaned — status and file state can never contradict.

## Event mapping

`map_event: &mut dyn FnMut(&ScheduledEvent<P>, audio_generation: u64) -> Option<NodeEvent>` — caller-owned (events are payload-typed `P`); `None` skips. `NodeEvent.generation` is set to the audio plan's generation so runtime delivery matches `Issue #195` semantics.

## Consumer impact

- Synvelis: new API only — impacted (gains the render path; drives `PreparedRender` + supplies feeder/encoder/muxer/publish).
- AudioLens: no impact (no existing API changed; `OfflineDriver` untouched).
- `Progress::bump_by` addition: additive — no impact.
- Deferred/unverified: real-plugin `Verified` attestation (#196 in flight); `cargo test -p ruviform-exec` still passes (regression check).
