//! Stateful sequential offline render (Issue #198, milestone M6).
//!
//! A generic, non-RT render path that processes a
//! [`PreparedPlan`](crate::typed::PreparedPlan) over a caller-described
//! transport program in **monotonically increasing engine frames** and
//! lands the result in a staged media file — preserving Graph / Time /
//! Event / stateful-processor semantics end to end.
//!
//! ## Contract summary
//!
//! - **Prepared instance** — [`PreparedRender`] compiles a
//!   [`ValidatedGraph`] into its own [`GraphRuntime`] + [`RtRuntime`].
//!   The processor set belongs to this instance alone; nothing is shared
//!   with any Live runtime (independence by construction, not by copy).
//! - **Capability gate** — every installed processor must report
//!   [`OfflineRenderSupport::Verified`] through
//!   [`GraphProcessor::offline_support`](crate::typed::GraphProcessor::offline_support);
//!   `Unknown`/`Declared` reject at preflight. There is no automatic
//!   degraded mode.
//! - **Sequential drive** — the transport program is staged into a
//!   [`TransportScheduler`]; each `run_callback` chunk emits ordered
//!   [`TransportSpan`]s which are cut at event engine-frames and
//!   partitioned into `process` calls under the [`BlockContract`]. No
//!   temporal parallelization is ever applied.
//! - **Preroll / PDC / tail** — emitted output frame `j` of a block at
//!   position `p` maps to `p + j - plan.total_latency`; emission is
//!   limited to `[range_start, range_end + tail)`. Warm-up frames are
//!   processed but never emitted. [`TailSpec`] classifies the tail;
//!   an undefined tail is a caller error, not a silent truncation.
//! - **Output lifecycle** — emitted runs feed
//!   `Encoder → Packet → Muxer` inside a [`StagedOutput`]: drain,
//!   checked `finish`, optional reopen/verify, then `publish`. Partial
//!   output can never reach the destination: any failure cleans the
//!   temp file and leaves an existing destination untouched.
//! - **Progress / cancel** — [`Progress`] counts processed engine
//!   frames; [`CancellationToken`] is honored at every physical call
//!   boundary and before drain/finalize/verify/publish.
//!
//! **RT contract:** every API in this module is **non-RT only** — it
//! allocates, performs filesystem I/O, and drives the RT-side runtime
//! from the calling thread (the same role a device callback plays,
//! without the realtime deadline).

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
use ruviform_exec::{CancellationToken, Progress};
use ruviform_graph::{NodeId, PortId, ValidatedGraph};
use ruviform_media_core::{
    ApplyStatus, BlockContract, EngineFrame, FaultKind, MediaDuration, MediaTime, MediaTimestamp,
    RejectReason, ScheduledEvent, SpanSink, TransportCommand, TransportPlan, TransportScheduler,
    TransportSpan, TransportStatus, event_engine_frame,
};
use ruviform_media_io::{
    AudioFrame, Encoder, Error as MediaError, ErrorKind, Frame, Muxer, PublishOptions, StagedOutput,
};
use ruviform_rt::RtContext;

use crate::typed::{
    Compiler, CompilerConfig, GraphRuntime, NodeEvent, NodeFactory, OfflineRenderSupport,
    PreparedPlan, RtRuntime, Slot,
};

/// Tail classification for a render range. The tail extends the emitted
/// frame window past `range_end` so processor tails (reverb rings,
/// delay-line flush, filter decay) are captured rather than cut.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy. There is deliberately no `Unknown` variant: a render that
/// cannot bound its tail must say so through the *caller* (estimated
/// length + [`TailEvidence::Estimated`] on the receipt), never by
/// silently truncating inside the driver.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TailSpec {
    /// No tail — emit exactly `[range_start, range_end)`.
    None,
    /// Provably bounded tail of `frames` (e.g. `total_latency` flush).
    Known(u64),
    /// Estimated tail of `frames` — rendered fully, but the receipt
    /// classifies the evidence as estimated, not known.
    Estimated(u64),
}

impl TailSpec {
    /// Extra emitted frames past `range_end`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frames(self) -> u64 {
        match self {
            Self::None => 0,
            Self::Known(n) | Self::Estimated(n) => n,
        }
    }
}

/// Tail evidence recorded on the receipt — mirrors the [`TailSpec`]
/// the render was driven with.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TailEvidence {
    /// `TailSpec::None`.
    None,
    /// `TailSpec::Known`.
    Known,
    /// `TailSpec::Estimated`.
    Estimated,
}

/// Fixed render geometry: what to render, where the output boundary is,
/// and which engine-frame window to drive. All values are frozen at
/// [`render`] entry — the spec *is* the render snapshot alongside the
/// compiled plan (Live edits cannot reach it).
///
/// **RT contract:** non-RT only — plain data.
#[derive(Clone, Debug)]
pub struct RenderSpec {
    /// Output sample rate — must equal the transport scheduler's
    /// `frame_base` rate (one tick == one frame).
    pub sample_rate: SampleRate,
    /// Output channel layout — must match the boundary output slot.
    pub layout: ChannelLayout,
    /// Boundary output port to drain (a `plan.outputs` port).
    pub output_port: PortId,
    /// First emitted position (inclusive), in sample frames.
    pub range_start: u64,
    /// Last emitted position (exclusive), in sample frames.
    pub range_end: u64,
    /// Tail classification — extends the emitted window past
    /// `range_end`.
    pub tail: TailSpec,
    /// First engine frame the driver processes.
    pub job_start: u64,
    /// Engine frame the driver stops at (exclusive). Must cover the
    /// full emitted window plus `total_latency` of warm-up.
    pub job_end: u64,
    /// `run_callback` granularity in engine frames. Smaller chunks bound
    /// per-callback span counts; the emitted stream is identical for
    /// any legal value. `0` selects `min(job span, 4096)`.
    pub callback_chunk: u32,
}

/// A transport command staged for application at `due`.
///
/// **RT contract:** non-RT only — plain data.
#[derive(Clone, Debug)]
pub struct StagedCommand {
    /// Command to apply.
    pub command: TransportCommand,
    /// Engine frame it applies at.
    pub due: EngineFrame,
}

/// The transport program a render is driven over: a caller-built
/// [`TransportScheduler`] (frame base, [`BlockContract`],
/// `DeliveryPolicy`, budgets — all fixed by the caller) plus the
/// commands and event plans to stage into it.
///
/// Commands are [`TransportScheduler::preflight`]ed before staging:
/// an unrepresentable boundary under `DeliveryPolicy::Exact` is a
/// preflight rejection, never a mid-render surprise.
///
/// **RT contract:** non-RT only.
pub struct RenderTransport<P> {
    /// Scheduler instance — its epoch fixes the staged `EngineFrame`s.
    pub scheduler: TransportScheduler<P>,
    /// Commands staged in due order (any order accepted; staged sorted).
    pub commands: Vec<StagedCommand>,
    /// `(plan, due)` installs — each applies as a plan-generation
    /// boundary at `due`.
    pub plans: Vec<(Arc<TransportPlan<P>>, EngineFrame)>,
}

/// Per-call feed context handed to [`InputFeeder::feed`]: where the
/// block sits on both clocks.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug)]
pub struct FeedContext {
    /// Engine frame this physical call starts at.
    pub engine_frame: u64,
    /// Transport position (sample frames) at the call start — the
    /// input-domain position, *before* PDC. Constant on non-Playing
    /// spans (position does not advance).
    pub position_ticks: i64,
    /// Frames this call will process.
    pub frames: usize,
}

/// Caller-owned boundary-input source. The driver calls [`feed`] once
/// per boundary input port per physical call, *before* `process` —
/// exactly like a device input tap.
///
/// **RT contract:** RT-suitable implementations only — the driver is
/// non-RT but calls `feed` inside the hot sequential loop; allocating
/// implementations are legal yet discouraged.
pub trait InputFeeder {
    /// Fills `slot` for the block described by `ctx`.
    fn feed(&mut self, port: PortId, ctx: &FeedContext, slot: &mut Slot);
}

/// Zero-fill feeder — the correct default for graphs whose boundary
/// inputs are silence or unused.
pub struct SilentFeeder;

impl InputFeeder for SilentFeeder {
    fn feed(&mut self, _port: PortId, ctx: &FeedContext, slot: &mut Slot) {
        for ch in 0..usize::from(slot.channels) {
            if let Some(plane) = slot.data.channel_mut(ch) {
                plane[..ctx.frames].fill(0.0);
            }
        }
    }
}

/// Facts about the rendered output, handed to the verify hook before
/// publish — everything an independent reader needs to check the file
/// against the request.
///
/// **RT contract:** non-RT only — plain data.
#[derive(Clone, Debug)]
pub struct OutputFacts {
    /// Declared sample rate.
    pub sample_rate: SampleRate,
    /// Declared channel layout.
    pub layout: ChannelLayout,
    /// Emitted frames actually sent to the encoder.
    pub frames_emitted: u64,
    /// First emitted position.
    pub range_start: u64,
    /// Emitted-window end (`range_end + tail frames`).
    pub range_end_with_tail: u64,
}

/// The staged output lifecycle bundle handed to [`render`].
///
/// `make_muxer` receives an owned [`File::try_clone`] of the staged
/// temp file — a second handle on the same open file description, so
/// bytes the muxer writes land in the staged file and remain visible
/// to `publish`'s sync/rename. Owning the handle keeps the muxer
/// `'static` (boxed) while the original handle stays with
/// [`StagedOutput`].
///
/// `publish` normally delegates to [`StagedOutput::publish`]; it is a
/// closure so the publish stage (sync/close/rename) participates in the
/// same failure-injection contract as the rest of the pipeline.
///
/// **RT contract:** non-RT only.
pub struct RenderOutput<Mk, V, Pb> {
    /// Staged temp/destination pair (created by the caller).
    pub staged: StagedOutput,
    /// Muxer constructor over the cloned staged handle.
    pub make_muxer: Mk,
    /// Canonical planar-f32 encoder.
    pub encoder: Box<dyn Encoder>,
    /// Publish policy handed to `publish`.
    pub publish_options: PublishOptions,
    /// Optional reopen/verify hook run on `staged.temp_path()` after
    /// the muxer is finished and closed, before publish.
    pub verify: Option<V>,
    /// Publish step — normally `StagedOutput::publish`.
    pub publish: Pb,
}

/// Terminal receipt of a successful render: published path plus the
/// observable render facts. There is no receipt for a failed render —
/// failure is a typed [`RenderError`], never a partial receipt.
///
/// Backend-specific muxer receipts (e.g. `WavReceipt`) are captured by
/// the caller's own [`Muxer`] implementation when needed — the driver
/// sees only the trait.
///
/// **RT contract:** non-RT only — plain data.
#[derive(Clone, Debug)]
pub struct RenderReceipt {
    /// Published destination path.
    pub path: PathBuf,
    /// Audio frames emitted to the encoder (== `range` + tail).
    pub frames_emitted: u64,
    /// Physical `process` calls executed.
    pub blocks_processed: u64,
    /// Audio plan generation the render ran under.
    pub plan_generation: u64,
    /// First emitted position.
    pub range_start: u64,
    /// Emitted-window end including tail.
    pub range_end_with_tail: u64,
    /// Tail classification actually rendered.
    pub tail_evidence: TailEvidence,
    /// Generation-stale events the scheduler recorded skipping
    /// (defined-skip semantics; all other faults fail the render).
    pub stale_events_skipped: u64,
}

/// Why a render was refused before any `process` call ran.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Debug)]
pub enum PreflightError {
    /// A processor does not report `OfflineRenderSupport::Verified`.
    OfflineUnsupported {
        /// Rejecting node.
        node: NodeId,
        /// Reported support level.
        support: OfflineRenderSupport,
    },
    /// A staged command failed `TransportScheduler::preflight`.
    Command {
        /// Index into `RenderTransport::commands`.
        index: usize,
        /// Scheduler rejection.
        reason: RejectReason,
    },
    /// A command could not be staged (immediate `request` rejection).
    Stage {
        /// Index into `RenderTransport::commands` (or `plans` when
        /// `is_plan`).
        index: usize,
        /// Whether the rejection came from a plan install.
        is_plan: bool,
        /// Scheduler rejection.
        reason: RejectReason,
    },
    /// Render geometry is inconsistent.
    Range {
        /// Human-readable detail.
        detail: &'static str,
    },
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OfflineUnsupported { node, support } => {
                write!(
                    f,
                    "node {node:?} offline support {support:?} is not Verified"
                )
            }
            Self::Command { index, reason } => {
                write!(
                    f,
                    "staged command {index} rejected at preflight: {reason:?}"
                )
            }
            Self::Stage {
                index,
                is_plan,
                reason,
            } => {
                let what = if *is_plan { "plan install" } else { "command" };
                write!(f, "{what} {index} rejected at staging: {reason:?}")
            }
            Self::Range { detail } => write!(f, "invalid render range: {detail}"),
        }
    }
}

impl std::error::Error for PreflightError {}

/// Typed render failure. Every variant maps to one pipeline stage and
/// carries that stage's evidence; on any `Err` the staged temp file is
/// removed and the destination is untouched.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API policy.
#[non_exhaustive]
#[derive(Debug)]
pub enum RenderError {
    /// Refused before processing — capability, range, or transport
    /// preflight.
    Preflight(PreflightError),
    /// The requested shape is not supported by this pipeline stage.
    Unsupported {
        /// What was requested.
        what: &'static str,
    },
    /// Cooperative cancellation observed at a phase boundary.
    Cancelled {
        /// Phase where cancel landed (`process`/`drain`/`finalize`/
        /// `verify`/`publish`).
        phase: &'static str,
    },
    /// `process`/feed/boundary plumbing inside the drive loop failed.
    Process {
        /// Failure detail.
        detail: String,
    },
    /// A span had no legal physical-call partition (`deliverable ==
    /// false`) or a non-benign schedule fault was recorded.
    TransportFault {
        /// Engine frame the fault/span was observed at.
        at: EngineFrame,
        /// Fault kind (`UnrepresentableBoundary` for undeliverable
        /// spans).
        kind: FaultKind,
        /// Kind-specific detail (span length, event count).
        detail: u64,
    },
    /// A staged command/plan receipt reported `Rejected` status or a
    /// degradation.
    CommandRejected {
        /// Lifecycle status observed.
        status: ApplyStatus,
        /// Reject reason or degradation detail.
        detail: String,
    },
    /// Segmenting around events left no legal physical-call partition.
    Segmentation {
        /// Human-readable detail.
        detail: String,
    },
    /// Event channel could not accept a mapped `NodeEvent`.
    EventChannelFull {
        /// Events that could not be delivered.
        pending: usize,
    },
    /// Encoder `send_frame`/`receive_packet` or muxer `write_packet`
    /// failed — the write/encode stage.
    Output {
        /// Backend error (kind classifies disk-full vs corrupt vs …).
        source: MediaError,
    },
    /// `encoder.drain()` failed.
    Drain {
        /// Backend error.
        source: MediaError,
    },
    /// Muxer `finish` failed — trailer/index/flush stage.
    Finalize {
        /// Backend error.
        source: MediaError,
    },
    /// The caller's reopen/verify hook rejected the staged file.
    Verify {
        /// Hook error.
        source: MediaError,
    },
    /// Destination exists and the publish policy forbids overwrite —
    /// detected before any byte was rendered, or surfaced from a
    /// `replace_existing == false` publish.
    DestinationExists {
        /// Occupied destination.
        path: PathBuf,
    },
    /// Publish (sync/close/rename) failed.
    Publish {
        /// Backend error.
        source: MediaError,
    },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preflight(e) => write!(f, "preflight failed: {e}"),
            Self::Unsupported { what } => write!(f, "unsupported: {what}"),
            Self::Cancelled { phase } => write!(f, "render cancelled during {phase}"),
            Self::Process { detail } => write!(f, "process failed: {detail}"),
            Self::TransportFault { at, kind, detail } => {
                write!(
                    f,
                    "transport fault {kind:?} at frame {}: {detail}",
                    at.frame()
                )
            }
            Self::CommandRejected { status, detail } => {
                write!(f, "transport command {status:?}: {detail}")
            }
            Self::Segmentation { detail } => write!(f, "segmentation failed: {detail}"),
            Self::EventChannelFull { pending } => {
                write!(f, "event channel full with {pending} events pending")
            }
            Self::Output { source } => write!(f, "output stage failed: {source}"),
            Self::Drain { source } => write!(f, "encoder drain failed: {source}"),
            Self::Finalize { source } => write!(f, "muxer finalize failed: {source}"),
            Self::Verify { source } => write!(f, "reopen verify failed: {source}"),
            Self::DestinationExists { path } => {
                write!(f, "destination exists: {}", path.display())
            }
            Self::Publish { source } => write!(f, "publish failed: {source}"),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(e) => Some(e),
            Self::Output { source: e }
            | Self::Drain { source: e }
            | Self::Finalize { source: e }
            | Self::Verify { source: e }
            | Self::Publish { source: e } => Some(e),
            _ => None,
        }
    }
}

/// An isolated compiled graph for sequential offline rendering: its own
/// [`GraphRuntime`] + [`RtRuntime`] pair, the published
/// [`PreparedPlan`], and the per-node offline capability map captured
/// at install time.
///
/// Construction is the *snapshot* — compile from a `&ValidatedGraph`
/// and a caller factory whose `New` outcomes produce fresh instances.
/// Nothing inside is shared with any Live runtime.
///
/// **RT contract:** non-RT only.
pub struct PreparedRender {
    graph: GraphRuntime,
    rt: RtRuntime,
    plan: Arc<PreparedPlan>,
    capabilities: BTreeMap<NodeId, OfflineRenderSupport>,
}

impl PreparedRender {
    /// Compiles `graph` into a fresh isolated runtime pair and
    /// publishes it as generation `1`.
    ///
    /// `command_capacity`/`event_capacity` bound the runtime's
    /// generation-tagged channels (see [`GraphRuntime::new`]).
    ///
    /// # Errors
    ///
    /// [`RenderError::Process`] wrapping the compile/activate failure —
    /// the plan is never partially published.
    pub fn compile(
        graph: &ValidatedGraph,
        factory: &mut dyn NodeFactory,
        config: CompilerConfig,
        command_capacity: usize,
        event_capacity: usize,
    ) -> Result<Self, RenderError> {
        let (mut graph_rt, rt) = GraphRuntime::new(&config, command_capacity, event_capacity);
        let compiled = Compiler::new(config)
            .compile(
                graph,
                factory,
                graph_rt.registry(),
                graph_rt.allocator(),
                graph_rt.generation() + 1,
            )
            .map_err(|e| RenderError::Process {
                detail: format!("compile failed: {e:?}"),
            })?;
        let capabilities = compiled
            .installs
            .iter()
            .map(|install| (install.node, install.processor.offline_support()))
            .collect();
        graph_rt
            .publish(compiled)
            .map_err(|e| RenderError::Process {
                detail: format!("publish failed: {e:?}"),
            })?;
        let plan = Arc::clone(graph_rt.published_plan());
        Ok(Self {
            graph: graph_rt,
            rt,
            plan,
            capabilities,
        })
    }

    /// The published plan (boundary ports, `total_latency`, generation).
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub fn plan(&self) -> &Arc<PreparedPlan> {
        &self.plan
    }

    /// End-to-end accumulated latency in frames — the PDC offset
    /// subtracted from block positions to obtain emitted positions.
    ///
    /// **RT contract:** non-RT only — `Copy` value.
    #[must_use]
    pub fn total_latency(&self) -> u64 {
        u64::from(self.plan.total_latency)
    }

    /// Per-node offline capability captured at install time.
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub fn capabilities(&self) -> &BTreeMap<NodeId, OfflineRenderSupport> {
        &self.capabilities
    }
}

/// The `SpanSink` the drive loop pushes spans through — owns the
/// per-render mutable state so `run_callback` borrows stay disjoint.
/// Errors are captured in `error` (the sink contract cannot fail); the
/// loop aborts on the next boundary.
struct RenderSink<'a, P, F> {
    prepared: &'a mut PreparedRender,
    spec: &'a RenderSpec,
    feeder: &'a mut F,
    encoder: &'a mut dyn Encoder,
    muxer: &'a mut dyn Muxer,
    map_event: &'a mut dyn FnMut(&ScheduledEvent<P>, u64) -> Option<NodeEvent>,
    progress: &'a Progress,
    cancel: &'a CancellationToken,
    effective_contract: BlockContract,
    frames_emitted: u64,
    blocks_processed: u64,
    stale_events_skipped: u64,
    error: Option<RenderError>,
}

impl<P, F: InputFeeder> RenderSink<'_, P, F> {
    /// Delivers every event whose engine frame is exactly `at` — the
    /// ordering point is *before* the physical call covering `at`.
    fn deliver_events(&mut self, span: &TransportSpan, events: &[ScheduledEvent<P>], at: u64) {
        let mut pending = 0usize;
        for ev in events {
            let Some(frame) = event_engine_frame(span, ev) else {
                continue;
            };
            if frame.frame() != at {
                continue;
            }
            if let Some(node_ev) = (self.map_event)(ev, self.prepared.plan.generation)
                && self.prepared.graph.send_event(node_ev).is_err()
            {
                pending += 1;
            }
        }
        if pending > 0 && self.error.is_none() {
            self.error = Some(RenderError::EventChannelFull { pending });
        }
    }

    /// One physical `process` call at absolute engine frame `at`:
    /// feed inputs → process → drain emitted runs → encode → mux.
    fn physical_call(&mut self, span: &TransportSpan, at: u64, frames: u32) {
        if self.cancel.is_cancelled() {
            self.error = Some(RenderError::Cancelled { phase: "process" });
            return;
        }
        let offset = u32::try_from(at - span.start.frame()).unwrap_or(u32::MAX);
        let position = span.ctx.position_at_offset(offset);
        let feed_ctx = FeedContext {
            engine_frame: at,
            position_ticks: position.value(),
            frames: frames as usize,
        };
        {
            // Borrow scope: immutable on `prepared` while `feeder` is
            // touched mutably — disjoint fields.
            let inputs = &self.prepared.plan.inputs;
            for &(port, _) in inputs {
                let Some(mut slot) = self.prepared.rt.input_writer(port, frames as usize) else {
                    self.error = Some(RenderError::Process {
                        detail: format!("boundary input port {port:?} unbound"),
                    });
                    return;
                };
                self.feeder.feed(port, &feed_ctx, &mut slot);
            }
        }
        if let Err(e) = self
            .prepared
            .rt
            .process(frames as usize, &RtContext::mint())
        {
            self.error = Some(RenderError::Process {
                detail: format!("{e:?}"),
            });
            return;
        }
        self.blocks_processed += 1;
        self.progress.bump_by(u64::from(frames));

        if span.ctx.status != TransportStatus::Playing {
            // Position does not advance — processed for warm-up/state
            // continuity only, never emitted.
            return;
        }
        let pdc = i64::try_from(self.prepared.total_latency()).unwrap_or(i64::MAX);
        let emit_lo = i64::try_from(self.spec.range_start).unwrap_or(i64::MAX);
        let emit_hi = i64::try_from(self.spec.range_end.saturating_add(self.spec.tail.frames()))
            .unwrap_or(i64::MAX);
        let base = self.spec.sample_rate.time_base();
        let pos0 = position.value() - pdc;
        let n = frames as usize;
        let mut j = 0usize;
        while j < n {
            let j64 = i64::try_from(j).unwrap_or(i64::MAX);
            if pos0 + j64 < emit_lo || pos0 + j64 >= emit_hi {
                j += 1;
                continue;
            }
            let j0 = j;
            while j < n && {
                let p = pos0 + i64::try_from(j).unwrap_or(i64::MAX);
                p >= emit_lo && p < emit_hi
            } {
                j += 1;
            }
            if let Err(e) = self.emit_run(j0, j - j0, pos0, base) {
                self.error = Some(e);
                return;
            }
        }
    }

    /// Copies one emitted run `[j0, j0+run)` out of the boundary slot,
    /// wraps it in an `AudioFrame` and pushes it through the encoder
    /// into the muxer.
    fn emit_run(
        &mut self,
        j0: usize,
        run: usize,
        pos0: i64,
        base: ruviform_media_core::TimeBase,
    ) -> Result<(), RenderError> {
        let mut buffer = AudioBuffer::new(BufferSpec {
            layout: self.spec.layout,
            frames: run,
            sample_rate: self.spec.sample_rate,
        });
        {
            // Scoped so the slot `Ref` drops before the encoder/muxer
            // mutable borrows below.
            let Some(view) = self.prepared.rt.output_reader(self.spec.output_port) else {
                return Err(RenderError::Process {
                    detail: "boundary output port unbound".into(),
                });
            };
            for ch in 0..buffer.channels() {
                let Some(dst) = buffer.channel_mut(ch) else {
                    continue;
                };
                let Some(src) = view.data.channel(ch) else {
                    return Err(RenderError::Process {
                        detail: "output slot channel missing for spec layout".into(),
                    });
                };
                dst.copy_from_slice(&src[j0..j0 + run]);
            }
        }
        let mut frame = AudioFrame::new(
            buffer,
            MediaTimestamp::derived(MediaTime::new(
                pos0 + i64::try_from(j0).unwrap_or(i64::MAX),
                base,
            )),
        );
        frame.set_duration(Some(MediaDuration::new(
            i64::try_from(run).unwrap_or(i64::MAX),
            base,
        )));
        self.frames_emitted += run as u64;
        self.encoder
            .send_frame(Frame::Audio(frame))
            .map_err(|e| RenderError::Output { source: e })?;
        self.pump_encoder()
    }

    /// Drains pending encoder packets into the muxer.
    fn pump_encoder(&mut self) -> Result<(), RenderError> {
        while let Some(packet) = self
            .encoder
            .receive_packet()
            .map_err(|e| RenderError::Output { source: e })?
        {
            self.muxer
                .write_packet(packet)
                .map_err(|e| RenderError::Output { source: e })?;
        }
        Ok(())
    }

    /// The callback loop: chunked `run_callback`s over the job window,
    /// fault classification, and receipt draining. Returns the first
    /// captured sink error, if any.
    fn drive(&mut self, scheduler: &mut TransportScheduler<P>) {
        let spec = self.spec;
        let chunk = if spec.callback_chunk == 0 {
            u32::try_from(spec.job_end - spec.job_start)
                .unwrap_or(4096)
                .min(4096)
        } else {
            spec.callback_chunk
        };
        let mut cursor = spec.job_start;
        while cursor < spec.job_end && self.error.is_none() {
            let frames = u32::try_from(spec.job_end - cursor)
                .unwrap_or(u32::MAX)
                .min(chunk);
            scheduler.run_callback(EngineFrame::new(scheduler.epoch(), cursor), frames, self);
            for fault in scheduler.drain_faults() {
                if fault.kind == FaultKind::StaleEventGeneration {
                    // Defined-skip semantics — counted, not fatal.
                    self.stale_events_skipped = self.stale_events_skipped.saturating_add(1);
                } else if self.error.is_none() {
                    self.error = Some(RenderError::TransportFault {
                        at: fault.at,
                        kind: fault.kind,
                        detail: fault.detail,
                    });
                }
            }
            if let Err(e) = check_receipts(scheduler)
                && self.error.is_none()
            {
                self.error = Some(e);
            }
            cursor += u64::from(frames);
        }
    }
}

impl<P, F: InputFeeder> SpanSink<P> for RenderSink<'_, P, F> {
    fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<P>]) {
        if self.error.is_some() {
            return;
        }
        if !span.deliverable {
            self.error = Some(RenderError::TransportFault {
                at: span.start,
                kind: FaultKind::UnrepresentableBoundary,
                detail: u64::from(span.frames()),
            });
            return;
        }
        // Event engine frames strictly inside the span become physical
        // cut points — events at the start are delivered before the
        // first call; events at the end belong to the next span.
        let mut cuts: Vec<u64> = events
            .iter()
            .filter_map(|ev| event_engine_frame(span, ev))
            .map(EngineFrame::frame)
            .filter(|&f| f > span.start.frame() && f < span.end.frame())
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts.push(span.end.frame());

        let mut seg_start = span.start.frame();
        for seg_end in cuts {
            self.deliver_events(span, events, seg_start);
            if self.error.is_some() {
                return;
            }
            let Ok(seg_len) = u32::try_from(seg_end - seg_start) else {
                self.error = Some(RenderError::Segmentation {
                    detail: "segment exceeds u32".into(),
                });
                return;
            };
            let Some(partition) = self.effective_contract.partition(seg_len) else {
                self.error = Some(RenderError::Segmentation {
                    detail: format!(
                        "no legal partition for {seg_len} frames under {:?}",
                        self.effective_contract
                    ),
                });
                return;
            };
            for i in 0..partition.chunks() {
                let (off, frames) = partition.chunk(i);
                self.physical_call(span, seg_start + u64::from(off), frames);
                if self.error.is_some() {
                    return;
                }
            }
            seg_start = seg_end;
        }
    }
}

/// Renders `prepared` over `transport` into `output`, honoring `spec`,
/// `progress` and `cancel`.
///
/// `map_event` translates each in-window [`ScheduledEvent`] into a
/// [`NodeEvent`] for the audio plan (argument: the event and the audio
/// plan generation); `None` skips the event. Events are delivered
/// before the `process` call that covers their engine frame, in
/// `(position, source_sequence)` order.
///
/// On `Ok` the destination holds a finished, published file. On `Err`
/// nothing was published: the staged temp is cleaned and an existing
/// destination is untouched.
///
/// **RT contract:** non-RT only — blocks on I/O.
///
/// # Errors
///
/// [`RenderError`] — one typed variant per failure stage; see the enum.
#[expect(clippy::too_many_arguments, reason = "top-level driver parameters")]
pub fn render<P, F, Mk, V, Pb>(
    prepared: &mut PreparedRender,
    transport: &mut RenderTransport<P>,
    spec: &RenderSpec,
    feeder: &mut F,
    output: RenderOutput<Mk, V, Pb>,
    progress: &Progress,
    cancel: &CancellationToken,
    map_event: &mut dyn FnMut(&ScheduledEvent<P>, u64) -> Option<NodeEvent>,
) -> Result<RenderReceipt, RenderError>
where
    F: InputFeeder,
    Mk: FnOnce(File) -> Result<Box<dyn Muxer>, MediaError>,
    V: FnOnce(&Path, &OutputFacts) -> Result<(), MediaError>,
    Pb: FnOnce(StagedOutput, PublishOptions) -> Result<(), MediaError>,
{
    let dest_path = output.staged.dest_path().to_path_buf();
    preflight(prepared, transport, spec)?;
    if output.staged.dest_existed() && !output.publish_options.replace_existing {
        return Err(RenderError::DestinationExists { path: dest_path });
    }
    stage_transport(transport)?;
    run_pipeline(
        prepared, transport, spec, feeder, output, progress, cancel, map_event, dest_path,
    )
}

/// Everything after preflight/staging: muxer construction over the
/// staged sink, the sequential drive, and the checked output tail.
/// `staged` must live in this frame — the muxer borrows its `&mut
/// File` sink until `finish` completes.
#[expect(
    clippy::too_many_arguments,
    reason = "pipeline stages share one context"
)]
#[expect(
    clippy::too_many_lines,
    reason = "single borrow scope for staged/muxer"
)]
fn run_pipeline<P, F, Mk, V, Pb>(
    prepared: &mut PreparedRender,
    transport: &mut RenderTransport<P>,
    spec: &RenderSpec,
    feeder: &mut F,
    output: RenderOutput<Mk, V, Pb>,
    progress: &Progress,
    cancel: &CancellationToken,
    map_event: &mut dyn FnMut(&ScheduledEvent<P>, u64) -> Option<NodeEvent>,
    dest_path: PathBuf,
) -> Result<RenderReceipt, RenderError>
where
    F: InputFeeder,
    Mk: FnOnce(File) -> Result<Box<dyn Muxer>, MediaError>,
    V: FnOnce(&Path, &OutputFacts) -> Result<(), MediaError>,
    Pb: FnOnce(StagedOutput, PublishOptions) -> Result<(), MediaError>,
{
    let RenderOutput {
        mut staged,
        make_muxer,
        mut encoder,
        publish_options,
        verify,
        publish,
    } = output;

    let mut muxer: Box<dyn Muxer> = {
        let handle = staged
            .sink()
            .and_then(|f| {
                f.try_clone()
                    .map_err(|e| MediaError::with_source(ErrorKind::Io, e))
            })
            .map_err(|e| RenderError::Output { source: e })?;
        make_muxer(handle).map_err(|e| RenderError::Output { source: e })?
    };

    let max = u32::try_from(prepared.plan.max_frames).unwrap_or(u32::MAX);
    let contract = transport.scheduler.contract();
    let effective_contract = BlockContract {
        min_frames: contract.min_frames,
        max_frames: contract.max_frames.min(max),
        in_block: contract.in_block,
    };
    if effective_contract.min_frames > effective_contract.max_frames {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "block contract min exceeds the plan's max_frames",
        }));
    }

    let (frames_emitted, blocks_processed, stale_events_skipped) = {
        let mut sink = RenderSink {
            prepared,
            spec,
            feeder,
            encoder: &mut *encoder,
            muxer: &mut *muxer,
            map_event,
            progress,
            cancel,
            effective_contract,
            frames_emitted: 0,
            blocks_processed: 0,
            stale_events_skipped: 0,
            error: None,
        };
        sink.drive(&mut transport.scheduler);
        match sink.error {
            Some(e) => return Err(e),
            None => (
                sink.frames_emitted,
                sink.blocks_processed,
                sink.stale_events_skipped,
            ),
        }
    };

    // ---- drain / finalize / verify / publish -------------------------
    if cancel.is_cancelled() {
        return Err(RenderError::Cancelled { phase: "drain" });
    }
    encoder
        .drain()
        .map_err(|e| RenderError::Drain { source: e })?;
    while let Some(packet) = encoder
        .receive_packet()
        .map_err(|e| RenderError::Output { source: e })?
    {
        muxer
            .write_packet(packet)
            .map_err(|e| RenderError::Output { source: e })?;
    }

    if cancel.is_cancelled() {
        return Err(RenderError::Cancelled { phase: "finalize" });
    }
    muxer
        .finish()
        .map_err(|e| RenderError::Finalize { source: e })?;
    drop(muxer);

    let facts = OutputFacts {
        sample_rate: spec.sample_rate,
        layout: spec.layout,
        frames_emitted,
        range_start: spec.range_start,
        range_end_with_tail: spec.range_end.saturating_add(spec.tail.frames()),
    };

    if cancel.is_cancelled() {
        return Err(RenderError::Cancelled { phase: "verify" });
    }
    if let Some(verify) = verify {
        verify(staged.temp_path(), &facts).map_err(|e| RenderError::Verify { source: e })?;
    }

    if cancel.is_cancelled() {
        return Err(RenderError::Cancelled { phase: "publish" });
    }
    publish(staged, publish_options).map_err(|e| match e.kind() {
        ErrorKind::InvalidValue => RenderError::DestinationExists {
            path: dest_path.clone(),
        },
        _ => RenderError::Publish { source: e },
    })?;

    Ok(RenderReceipt {
        path: dest_path,
        frames_emitted,
        blocks_processed,
        plan_generation: prepared.plan.generation,
        range_start: spec.range_start,
        range_end_with_tail: spec.range_end.saturating_add(spec.tail.frames()),
        tail_evidence: match spec.tail {
            TailSpec::None => TailEvidence::None,
            TailSpec::Known(_) => TailEvidence::Known,
            TailSpec::Estimated(_) => TailEvidence::Estimated,
        },
        stale_events_skipped,
    })
}

/// Stages the transport program: plan installs then commands, in
/// caller order. Immediate `Rejected` receipts become preflight errors;
/// the staging receipts are then drained through [`check_receipts`].
fn stage_transport<P>(transport: &mut RenderTransport<P>) -> Result<(), RenderError> {
    for (i, (plan, due)) in transport.plans.iter().enumerate() {
        transport
            .scheduler
            .install_plan(Arc::clone(plan), *due)
            .map_err(|receipt| {
                RenderError::Preflight(PreflightError::Stage {
                    index: i,
                    is_plan: true,
                    reason: receipt.reason.unwrap_or(RejectReason::InvalidCommand),
                })
            })?;
    }
    for (i, staged_cmd) in transport.commands.iter().enumerate() {
        transport
            .scheduler
            .request(staged_cmd.command, staged_cmd.due)
            .map_err(|receipt| {
                RenderError::Preflight(PreflightError::Stage {
                    index: i,
                    is_plan: false,
                    reason: receipt.reason.unwrap_or(RejectReason::InvalidCommand),
                })
            })?;
    }
    check_receipts(&mut transport.scheduler)
}

/// Static preflight: capabilities, geometry, and transport command
/// simulation — all before the first byte is staged.
fn preflight<P>(
    prepared: &PreparedRender,
    transport: &RenderTransport<P>,
    spec: &RenderSpec,
) -> Result<(), RenderError> {
    for (&node, &support) in prepared.capabilities() {
        if support != OfflineRenderSupport::Verified {
            return Err(RenderError::Preflight(PreflightError::OfflineUnsupported {
                node,
                support,
            }));
        }
    }
    if spec.range_start >= spec.range_end {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "range_start must precede range_end",
        }));
    }
    if spec.job_start >= spec.job_end {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "job_start must precede job_end",
        }));
    }
    let Some(slot_index) = prepared.plan.output_slot(spec.output_port) else {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "output_port is not a boundary output of the plan",
        }));
    };
    let expected_channels = prepared
        .plan
        .slots
        .get(slot_index)
        .map_or(0, |slot| slot.channels);
    if u32::from(expected_channels) != spec.layout.channels() {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "output port channel count differs from spec.layout",
        }));
    }
    if transport.scheduler.frame_base() != spec.sample_rate.time_base() {
        return Err(RenderError::Preflight(PreflightError::Range {
            detail: "scheduler frame_base differs from spec.sample_rate",
        }));
    }
    let window_frames = u32::try_from(spec.job_end - spec.job_start).unwrap_or(u32::MAX);
    let window_start = EngineFrame::new(transport.scheduler.epoch(), spec.job_start);
    for (i, staged) in transport.commands.iter().enumerate() {
        transport
            .scheduler
            .preflight(staged.command, staged.due, window_start, window_frames)
            .map_err(|reason| {
                RenderError::Preflight(PreflightError::Command { index: i, reason })
            })?;
    }
    Ok(())
}

/// Fails when any drained receipt is `Rejected` or carried a
/// degradation — accepted ≠ applied honesty for the transport program.
fn check_receipts<P>(scheduler: &mut TransportScheduler<P>) -> Result<(), RenderError> {
    for receipt in scheduler.drain_receipts() {
        if receipt.status == ApplyStatus::Rejected {
            return Err(RenderError::CommandRejected {
                status: receipt.status,
                detail: format!("{:?}", receipt.reason),
            });
        }
        if receipt.degradation.is_some() {
            return Err(RenderError::CommandRejected {
                status: receipt.status,
                detail: format!("degradation {:?}", receipt.degradation),
            });
        }
    }
    Ok(())
}
