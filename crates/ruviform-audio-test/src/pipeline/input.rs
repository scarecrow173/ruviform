//! Input leg — scripted input stream → bounded capture → production
//! typed runtime (`input_writer` → `process` → `output_reader`)
//! (Issue #628, 3rd slice).
//!
//! The production "Recording" component does not exist yet (deferred
//! to the consumer side — see the slice docs), so the device-input-tap
//! equivalent boundary is the typed runtime's graph-input slot: the
//! scripted `InputDataCallback` cannot call `input_writer` itself
//! (the callback is `FnMut` on borrowed views, not the runtime owner),
//! so PCM crosses through [`InputCapture`] — a bounded, preallocated
//! SPSC handoff in the same style as
//! `ruviform-audio-device/tests/mock_input_backend.rs` — and the test
//! driver feeds each captured block through `RtRuntime` exactly once
//! per callback, preserving the production per-block contract.
//!
//! ## Graph under test
//!
//! `Source(out) → GainNode(in/out) → Sink(in)` compiled by the
//! production [`Compiler`] into a [`PreparedPlan`](ruviform_audio::typed::PreparedPlan)
//! published through [`GraphRuntime::publish`]. The boundary ports are
//! the processor-less Source output (`input_writer` target) and the
//! processor-less Sink input (`output_reader` target) — the same
//! surface a real device-tap consumer would use.
//!
//! ## Honest limits
//!
//! `InputCallbackInfo.timestamp`/`clock_domain`/`frame_position` are
//! captured as [`CaptureBlock`] metadata — the typed runtime's PCM
//! boundary does not carry timestamp provenance, so capture/callback
//! instants are *observed*, not *consumed*. That gap is recorded in
//! the slice evidence; nothing substitutes callback time for a
//! missing capture instant.
//!
//! **RT contract:** [`InputCapture`]'s offer path (inside the device
//! data callback), [`GainNode::process`], and the
//! writer→process→reader section of [`InputPipeline::pump_block`]
//! are allocation-free — proven by `rt_pipeline_*` tests. Driver-side
//! drain/bookkeeping is non-RT.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_audio::typed::{
    Compiler, CompilerConfig, DEFAULT_ALIGNMENT, FactoryError, GraphProcessor, GraphRuntime,
    InstalledInfo, NodeFactory, NodeIo, PrepareOutcome, RtRuntime,
};
use ruviform_audio_core::AudioBufferView;
use ruviform_audio_core::AudioFormat;
use ruviform_audio_core::device::{
    DEVICE_CLOCK_TIMEBASE, Device, DeviceError, InputCallbackInfo, InputDataCallback,
    InputErrorCallback, InputStream, InputStreamConfig,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, GraphSpec, Multiplicity,
    NodeDef, NodeId, NodeKind, NodeSpec, PortDef, PortFormat, PortId, PortRef, PortRole,
    ValidatedGraph, Validator,
};
use ruviform_media_core::{ClockDomainId, MediaTime, MediaTimestamp};
use ruviform_rt::{Consumer, Producer, RtContext, channel};

use crate::backend::{PumpReport, ScriptedDeviceHandle};
use crate::pipeline::PipelineError;

/// Callback-side handoff block: one input callback's PCM plus the
/// full [`InputCallbackInfo`] metadata, preallocated to
/// `channels × capacity_frames` planar `f32`.
///
/// `data` stride is `capacity_frames` per channel; only the first
/// `frames` samples of each channel plane are valid for this block.
pub struct CaptureBlock {
    /// The callback's timestamp/domain/frame-position container —
    /// observed verbatim, never synthesized.
    pub info: InputCallbackInfo,
    /// Frames delivered this callback (`<= capacity_frames`).
    pub frames: u32,
    /// Channels delivered this callback.
    pub channels: u16,
    /// Planar `channels × capacity_frames` samples.
    pub data: Vec<f32>,
}

impl CaptureBlock {
    fn new(channels: usize, capacity_frames: usize) -> Self {
        Self {
            info: InputCallbackInfo {
                timestamp: ruviform_audio_core::device::InputTimestamp {
                    callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
                    capture: None,
                },
                clock_domain: ClockDomainId::new(0),
                frame_position: None,
            },
            frames: 0,
            channels: u16::try_from(channels).unwrap_or(u16::MAX),
            data: vec![0.0; channels * capacity_frames],
        }
    }

    /// Frames-per-channel stride of `data`.
    #[must_use]
    pub fn stride(&self) -> usize {
        let ch = usize::from(self.channels).max(1);
        self.data.len() / ch
    }

    /// Channel `ch`'s valid `frames` samples.
    #[must_use]
    pub fn channel(&self, ch: usize) -> Option<&[f32]> {
        let stride = self.stride();
        let frames = self.frames as usize;
        let start = ch.checked_mul(stride)?;
        self.data.get(start..start + frames)
    }
}

/// Observable handoff counters — every loss path is counted, none is
/// silent.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CaptureCounters {
    /// Callbacks that found no free block (pool exhausted).
    pub starved: u64,
    /// Deferred blocks dropped after a second failed channel push.
    pub overflow: u64,
    /// Callbacks rejected for frames/channel-count outside the
    /// preallocated block shape.
    pub rejected_shape: u64,
    /// Blocks successfully handed to the driver.
    pub delivered: u64,
    /// Blocks the driver failed to recycle (free channel full).
    pub recycle_dropped: u64,
}

/// Driver side of the input capture: owns the free-block producer,
/// the filled-block consumer, and the shared counters. Created
/// together with the callback half by [`Self::pair`].
///
/// **RT contract:** non-RT driver object — `drain_into`/`recycle` may
/// be called from the RT-adjacent driver loop but perform no
/// allocation when the caller's `Vec` is pre-reserved.
pub struct InputCapture {
    free_tx: Producer<CaptureBlock>,
    filled_rx: Consumer<CaptureBlock>,
    counters: Arc<SharedCounters>,
    channels: usize,
    capacity_frames: usize,
    pool_blocks: usize,
}

#[derive(Default)]
struct SharedCounters {
    starved: AtomicU64,
    overflow: AtomicU64,
    rejected_shape: AtomicU64,
    delivered: AtomicU64,
    recycle_dropped: AtomicU64,
}

/// Callback side of the capture — moved into the `InputDataCallback`
/// closure. Holds the free-block consumer, the filled-block producer,
/// one deferred `working` slot, and a bounded `spare` recycle list.
struct CaptureState {
    free_rx: Consumer<CaptureBlock>,
    filled_tx: Producer<CaptureBlock>,
    spare: Vec<CaptureBlock>,
    working: Option<CaptureBlock>,
    counters: Arc<SharedCounters>,
    channels: usize,
    capacity_frames: usize,
}

impl InputCapture {
    /// Creates the capture pair: `pool_blocks` preallocated
    /// `channels × capacity_frames` blocks circulating through the
    /// free/filled SPSC channels.
    ///
    /// The returned [`InputDataCallback`] is what
    /// `Device::open_input_stream` registers; the returned
    /// [`InputCapture`] is the driver-side drain/recycle handle.
    ///
    /// **RT contract:** non-RT — allocates the pool once.
    pub fn pair(
        channels: usize,
        capacity_frames: usize,
        pool_blocks: usize,
    ) -> (Self, InputDataCallback) {
        let (mut free_tx, free_rx) = channel::<CaptureBlock>(pool_blocks.max(1));
        let (filled_tx, filled_rx) = channel::<CaptureBlock>(pool_blocks.max(1));
        let counters = Arc::new(SharedCounters::default());
        for _ in 0..pool_blocks {
            if free_tx
                .try_push(CaptureBlock::new(channels, capacity_frames))
                .is_err()
            {
                break;
            }
        }
        let mut state = CaptureState {
            free_rx,
            filled_tx,
            spare: Vec::with_capacity(pool_blocks),
            working: None,
            counters: Arc::clone(&counters),
            channels,
            capacity_frames,
        };
        let callback: InputDataCallback = Box::new(move |view, info| state.offer(&view, info));
        (
            Self {
                free_tx,
                filled_rx,
                counters,
                channels,
                capacity_frames,
                pool_blocks,
            },
            callback,
        )
    }

    /// Moves every currently-filled block into `out` (driver side).
    /// `out` should be pre-reserved for the expected in-flight count —
    /// pushes beyond capacity allocate (non-RT bookkeeping).
    pub fn drain_into(&mut self, out: &mut Vec<CaptureBlock>) {
        while let Ok(block) = self.filled_rx.try_pop() {
            out.push(block);
        }
    }

    /// Returns a consumed block to the free pool. A full free channel
    /// means the callback side is not consuming — the block is
    /// dropped and counted (`recycle_dropped`), never silently lost.
    pub fn recycle(&mut self, block: CaptureBlock) {
        if self.free_tx.try_push(block).is_err() {
            self.counters
                .recycle_dropped
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Shared counter snapshot.
    #[must_use]
    pub fn counters(&self) -> CaptureCounters {
        CaptureCounters {
            starved: self.counters.starved.load(Ordering::Relaxed),
            overflow: self.counters.overflow.load(Ordering::Relaxed),
            rejected_shape: self.counters.rejected_shape.load(Ordering::Relaxed),
            delivered: self.counters.delivered.load(Ordering::Relaxed),
            recycle_dropped: self.counters.recycle_dropped.load(Ordering::Relaxed),
        }
    }

    /// Configured block shape (for assertions).
    #[must_use]
    pub const fn shape(&self) -> (usize, usize, usize) {
        (self.channels, self.capacity_frames, self.pool_blocks)
    }
}

impl CaptureState {
    /// The RT-adjacent offer path — bounded copy into a pooled block,
    /// deferred/overflow counted. No allocation, no lock, no blocking.
    fn offer(&mut self, view: &AudioBufferView<'_, f32>, info: InputCallbackInfo) {
        // Flush the deferred block first — FIFO order is preserved:
        // the older block must reach the driver before this one.
        if let Some(working) = self.working.take() {
            match self.filled_tx.try_push(working) {
                Ok(()) => {
                    self.counters.delivered.fetch_add(1, Ordering::Relaxed);
                }
                Err(full) => {
                    self.counters.overflow.fetch_add(1, Ordering::Relaxed);
                    self.spare.push(full.into_inner());
                }
            }
        }
        let frames = view.frames();
        let channels = view.channels();
        if frames > self.capacity_frames || channels != self.channels {
            self.counters.rejected_shape.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let Some(mut block) = self.spare.pop().or_else(|| self.free_rx.try_pop().ok()) else {
            self.counters.starved.fetch_add(1, Ordering::Relaxed);
            return;
        };
        block.info = info;
        block.frames = u32::try_from(frames).unwrap_or(u32::MAX);
        block.channels = u16::try_from(channels).unwrap_or(u16::MAX);
        let stride = block.stride();
        for ch in 0..channels {
            let Some(src) = view.channel(ch) else {
                continue;
            };
            let start = ch * stride;
            if let Some(dst) = block.data.get_mut(start..start + frames) {
                dst.copy_from_slice(src);
            }
        }
        match self.filled_tx.try_push(block) {
            Ok(()) => {
                self.counters.delivered.fetch_add(1, Ordering::Relaxed);
            }
            Err(full) => {
                // Channel full — defer to the next offer; not yet
                // counted as loss (it may still be delivered).
                self.working = Some(full.into_inner());
            }
        }
    }
}

/// Per-block observed metadata — the PCM itself lives contiguously
/// in [`InputPipeline::observed_pcm`] at `pcm_offset`.
#[derive(Copy, Clone, Debug)]
pub struct ObservedBlock {
    /// `frame_position` reported by the callback (`None` = backend
    /// does not report — never synthesized).
    pub start_frame: Option<u64>,
    /// Frames processed for this block.
    pub frames: u32,
    /// Channels observed.
    pub channels: u16,
    /// `timestamp.callback` in `DEVICE_CLOCK_TIMEBASE` nanoseconds.
    pub callback_nanos: i64,
    /// `timestamp.capture` verbatim (`None` stays `None`).
    pub capture: Option<MediaTimestamp>,
    /// `clock_domain` reported with the callback.
    pub clock_domain: ClockDomainId,
    /// Offset into `observed_pcm` (planar `channels × frames`).
    pub pcm_offset: usize,
}

/// A live input pipeline: scripted input stream → [`InputCapture`] →
/// production typed runtime.
pub struct InputPipeline {
    handle: ScriptedDeviceHandle,
    stream: Box<dyn InputStream>,
    runtime: GraphRuntime,
    rt: RtRuntime,
    in_port: PortId,
    out_port: PortId,
    capture: InputCapture,
    error_rx: Consumer<DeviceError>,
    error_overflow: Arc<AtomicU64>,
    errors: Vec<DeviceError>,
    reports: Vec<PumpReport>,
    pending: Vec<CaptureBlock>,
    observed_blocks: Vec<ObservedBlock>,
    observed_pcm: Vec<f32>,
    gain: f32,
}

/// The canonical test processor: `out = in * gain`, zero latency.
///
/// **RT contract:** `process` is RT-suitable — no allocation.
pub struct GainNode {
    gain: f32,
}

impl GraphProcessor for GainNode {
    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let input = io.input(0);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for ch in 0..out.channels() {
                let Some(plane) = out.channel_mut(ch) else {
                    continue;
                };
                match input.as_ref().and_then(|v| v.channel(ch)) {
                    Some(src) => {
                        for (d, s) in plane.iter_mut().zip(src.iter()) {
                            *d = s * self.gain;
                        }
                    }
                    None => plane.fill(0.0),
                }
            }
        }
    }
}

/// `NodeFactory` that installs [`GainNode`] on the processor node and
/// declares every other node a boundary.
struct GainFactory {
    node: NodeId,
    gain: f32,
}

impl NodeFactory for GainFactory {
    fn prepare(
        &mut self,
        node: &NodeSpec,
        _existing: Option<&InstalledInfo>,
    ) -> Result<PrepareOutcome, FactoryError> {
        if node.id() == self.node && node.kind() == NodeKind::Processor {
            Ok(PrepareOutcome::New(Box::new(GainNode { gain: self.gain })))
        } else {
            Ok(PrepareOutcome::Boundary)
        }
    }
}

fn port(direction: Direction, format: PortFormat) -> PortDef {
    PortDef {
        direction,
        role: PortRole::Main,
        format,
        multiplicity: Multiplicity::unbounded(0),
        capabilities: Vec::new(),
        availability: Availability::Always,
        activation: Activation::Enabled,
    }
}

fn conn(from: PortRef, to: PortRef) -> ConnDef {
    ConnDef {
        from,
        to,
        delay: EdgeDelay::Zero,
        channel_map: None,
    }
}

/// `Source(out fmt) → Processor(in/out fmt) → Sink(in fmt)` — the
/// minimal typed-runtime chain the input leg drives.
fn chain_spec(format: AudioFormat) -> (GraphSpec, NodeId, PortId, PortId) {
    let pf = PortFormat::Audio(format);
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&NodeDef {
        kind: NodeKind::Source,
        ports: vec![port(Direction::Output, pf)],
    });
    let (proc, pp) = b.add_node(&NodeDef {
        kind: NodeKind::Processor,
        ports: vec![port(Direction::Input, pf), port(Direction::Output, pf)],
    });
    let (sink, kp) = b.add_node(&NodeDef {
        kind: NodeKind::Sink,
        ports: vec![port(Direction::Input, pf)],
    });
    b.connect(&conn(
        PortRef {
            node: src,
            port: sp[0],
        },
        PortRef {
            node: proc,
            port: pp[0],
        },
    ));
    b.connect(&conn(
        PortRef {
            node: proc,
            port: pp[1],
        },
        PortRef {
            node: sink,
            port: kp[0],
        },
    ));
    (b.build(), proc, sp[0], kp[0])
}

/// Compiler budget for the pipeline graph — generous bounds; the
/// arena is preallocated once, so oversizing costs setup memory only.
#[must_use]
pub fn pipeline_compiler_config(max_frames: usize) -> CompilerConfig {
    CompilerConfig {
        max_frames,
        alignment: DEFAULT_ALIGNMENT,
        max_slots: 16,
        max_channels: 8,
        max_delay_lines: 8,
        max_delay_frames: 4096,
        max_processors: 8,
    }
}

impl InputPipeline {
    /// Opens `handle`'s input stream through the production
    /// `Device::open_input_stream` contract, compiles the
    /// Source→Gain→Sink chain, and starts delivery.
    ///
    /// `gain` is the [`GainNode`] coefficient; `compiler` is the
    /// [`CompilerConfig`] budget (`max_frames` must cover the largest
    /// callback `frames` the stream script can deliver).
    /// `capture_pool` sizes the handoff block pool (recommended: 4–8).
    ///
    /// # Errors
    ///
    /// [`PipelineError::Graph`] on spec validation failure,
    /// [`PipelineError::Compile`]/[`PipelineError::Activation`] on
    /// typed-runtime rejection, [`PipelineError::Device`] when the
    /// device refuses open/start (scripted open faults surface here).
    pub fn open(
        handle: &ScriptedDeviceHandle,
        config: InputStreamConfig,
        gain: f32,
        compiler: CompilerConfig,
        capture_pool: usize,
    ) -> Result<Self, PipelineError> {
        let channels = usize::try_from(config.format.layout.channels())
            .map_err(|_| PipelineError::Config("channel count"))?;
        if channels == 0 {
            return Err(PipelineError::Config("zero-channel layout"));
        }
        let (spec, proc_node, in_port, out_port) = chain_spec(config.format);
        let graph: ValidatedGraph = Validator::validate(&spec).map_err(PipelineError::Graph)?;

        let max_frames = compiler.max_frames;
        let (mut runtime, rt) = GraphRuntime::new(&compiler, 16, 16);
        let mut factory = GainFactory {
            node: proc_node,
            gain,
        };
        let plan = Compiler::new(compiler).compile(
            &graph,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            runtime.generation() + 1,
        )?;
        runtime.publish(plan)?;

        let (capture, data_cb) = InputCapture::pair(channels, max_frames, capture_pool);
        let (error_tx, error_rx) = channel::<DeviceError>(256);
        let error_overflow = Arc::new(AtomicU64::new(0));
        let error_overflow_cb = Arc::clone(&error_overflow);
        let mut error_tx_cb = error_tx;
        let error_cb: InputErrorCallback = Box::new(move |e| {
            if error_tx_cb.try_push(e).is_err() {
                error_overflow_cb.fetch_add(1, Ordering::Relaxed);
            }
        });

        let stream = handle
            .open_input_stream(&config, data_cb, error_cb)
            .map_err(PipelineError::Device)?;
        stream.start().map_err(PipelineError::Device)?;

        Ok(Self {
            handle: handle.clone(),
            stream,
            runtime,
            rt,
            in_port,
            out_port,
            capture,
            error_rx,
            error_overflow,
            errors: Vec::with_capacity(256),
            // Diagnostic buffers preallocated (64 reports / 64 observed
            // blocks) so steady-state `pump_block` stays allocation-free
            // within those bounds; beyond them the driver path allocates
            // — non-RT bookkeeping, counted honestly by the tests.
            reports: Vec::with_capacity(64),
            pending: Vec::with_capacity(capture_pool + 2),
            observed_blocks: Vec::with_capacity(64),
            observed_pcm: Vec::with_capacity(channels * max_frames * 64),
            gain,
        })
    }

    /// Fires one input callback, drains the capture, and drives each
    /// captured block through `input_writer → process → output_reader`.
    ///
    /// # Errors
    ///
    /// [`PipelineError::Device`] on pump failure (invalidated/vanished/
    /// not-started); [`PipelineError::Process`] when `frames` exceeds
    /// the compiler budget; [`PipelineError::Config`] on an unbound
    /// boundary port (a compile/publish inconsistency).
    pub fn pump_block(&mut self) -> Result<PumpReport, PipelineError> {
        let report = self.handle.pump_input_next()?;
        self.reports.push(report);
        self.drain_and_process()?;
        Ok(report)
    }

    /// Pumps `n` callbacks.
    ///
    /// # Errors
    ///
    /// First [`PipelineError`] encountered.
    pub fn pump_blocks(&mut self, n: usize) -> Result<Vec<PumpReport>, PipelineError> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.pump_block()?);
        }
        Ok(out)
    }

    fn drain_and_process(&mut self) -> Result<(), PipelineError> {
        self.capture.drain_into(&mut self.pending);
        self.drain_errors();
        let mut pending = std::mem::take(&mut self.pending);
        for block in pending.drain(..) {
            self.process_block(&block)?;
            self.capture.recycle(block);
        }
        self.pending = pending;
        Ok(())
    }

    /// Feeds one captured block through the typed runtime and records
    /// the observed output. One `input_writer` is always paired with
    /// exactly one `process` before the next writer call — the slot
    /// stamp contract forbids stacking writers.
    fn process_block(&mut self, block: &CaptureBlock) -> Result<(), PipelineError> {
        let frames = block.frames as usize;
        {
            let mut slot = self
                .rt
                .input_writer(self.in_port, frames)
                .ok_or(PipelineError::Config("graph input port unbound"))?;
            for ch in 0..usize::from(block.channels) {
                let Some(src) = block.channel(ch) else {
                    return Err(PipelineError::Config("capture channel short"));
                };
                let plane = slot
                    .data
                    .channel_mut(ch)
                    .ok_or(PipelineError::Config("slot channel missing"))?;
                plane[..frames].copy_from_slice(src);
            }
        }
        let ctx = RtContext::mint();
        self.rt.process(frames, &ctx)?;
        let view = self
            .rt
            .output_reader(self.out_port)
            .ok_or(PipelineError::Config("graph output port unbound"))?;
        let channels = usize::from(view.channels);
        let valid = usize::try_from(view.frames_valid).unwrap_or(0);
        let pcm_offset = self.observed_pcm.len();
        for ch in 0..channels {
            let plane = view
                .data
                .channel(ch)
                .ok_or(PipelineError::Config("output channel missing"))?;
            self.observed_pcm
                .extend_from_slice(&plane[..valid.min(plane.len())]);
        }
        self.observed_blocks.push(ObservedBlock {
            start_frame: block.info.frame_position,
            frames: u32::try_from(valid).unwrap_or(u32::MAX),
            channels: u16::try_from(channels).unwrap_or(u16::MAX),
            callback_nanos: block.info.timestamp.callback.value(),
            capture: block.info.timestamp.capture,
            clock_domain: block.info.clock_domain,
            pcm_offset,
        });
        Ok(())
    }

    fn drain_errors(&mut self) {
        while let Ok(e) = self.error_rx.try_pop() {
            self.errors.push(e);
        }
    }

    /// Errors observed on the stream's error callback, in order.
    /// Drained once per [`Self::pump_block`]; call [`Self::drain_errors_now`]
    /// for an explicit mid-pump check.
    #[must_use]
    pub fn errors(&self) -> &[DeviceError] {
        &self.errors
    }

    /// Drains any pending error-callback values outside a pump.
    pub fn drain_errors_now(&mut self) {
        self.drain_errors();
    }

    /// Error-callback channel overflow count (values dropped because
    /// the 256-deep channel was full).
    #[must_use]
    pub fn error_overflow(&self) -> u64 {
        self.error_overflow.load(Ordering::Relaxed)
    }

    /// Per-pump reports accumulated so far.
    #[must_use]
    pub fn reports(&self) -> &[PumpReport] {
        &self.reports
    }

    /// Observed output blocks (metadata) in pump order.
    #[must_use]
    pub fn observed_blocks(&self) -> &[ObservedBlock] {
        &self.observed_blocks
    }

    /// Observed output PCM — planar `channels × frames` per
    /// [`ObservedBlock`], concatenated in block order.
    #[must_use]
    pub fn observed_pcm(&self) -> &[f32] {
        &self.observed_pcm
    }

    /// The observed PCM slice of `observed_blocks()[i]`.
    #[must_use]
    pub fn observed_block_data(&self, i: usize) -> Option<&[f32]> {
        let block = self.observed_blocks.get(i)?;
        let len = usize::from(block.channels) * block.frames as usize;
        self.observed_pcm
            .get(block.pcm_offset..block.pcm_offset + len)
    }

    /// Capture handoff counters.
    #[must_use]
    pub fn capture_counters(&self) -> CaptureCounters {
        self.capture.counters()
    }

    /// The non-RT runtime owner (status, published plan, retire drain).
    #[must_use]
    pub fn runtime(&self) -> &GraphRuntime {
        &self.runtime
    }

    /// Shared runtime status snapshot (faults/counters).
    #[must_use]
    pub fn status_snapshot(&self) -> ruviform_audio::typed::SharedStatusSnapshot {
        self.runtime.status().snapshot()
    }

    /// The scripted device handle (fault injection, pump access).
    #[must_use]
    pub fn handle(&self) -> &ScriptedDeviceHandle {
        &self.handle
    }

    /// The gain applied by [`GainNode`].
    #[must_use]
    pub const fn gain(&self) -> f32 {
        self.gain
    }

    /// Stops the input stream (dropping also stops it).
    ///
    /// # Errors
    ///
    /// [`PipelineError::Device`] when the backend refuses.
    pub fn stop(&self) -> Result<(), PipelineError> {
        self.stream.stop().map_err(PipelineError::Device)
    }
}

impl core::fmt::Debug for InputPipeline {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InputPipeline")
            .field("gain", &self.gain)
            .field("reports", &self.reports.len())
            .field("observed_blocks", &self.observed_blocks.len())
            .field("errors", &self.errors.len())
            .field("capture", &self.capture.counters())
            .finish_non_exhaustive()
    }
}
