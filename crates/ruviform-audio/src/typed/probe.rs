//! Generic observation probe for the typed runtime (Issue #185).
//!
//! A probe observes one `(node, port)` point non-destructively: it
//! never mutates slot data, never touches routing, and is entirely
//! separate from processor Bypass. Modes are [`ProbeMode::Off`]
//! (no tap work), [`ProbeMode::Monitor`] (per-channel Peak/RMS via
//! [`ChannelLevel::measure`]), and [`ProbeMode::Capture`] (bounded
//! PCM copy into a preallocated chunk pool).
//!
//! Ownership split mirrors the rest of `typed`: the non-RT
//! [`ProbeBroker`] resolves points against the published plan and
//! sends generation-checked [`ProbeCommand`]s; the RT-side
//! [`ProbeDeck`] applies them inside [`RtRuntime::process`](super::RtRuntime::process)
//! and hands records to the non-RT [`ProbeCollector`] through two
//! bounded SPSC channels — `chunks_out` (boxed PCM, capacity equal
//! to the pool so pushes are infallible by construction) and
//! `telemetry` (POD metric/status records, drop-newest on overflow
//! with a shared loss counter). The RT side never allocates, never
//! blocks, and never runs destructors.
//!
//! Probe capture may drop under pressure (`LostBlocks`/`LostRecords`)
//! — that tolerance is scoped to observation and is *not* the
//! lossless Recording Writer contract (Issue #187).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_audio_analysis::ChannelLevel;
use ruviform_graph::{Direction, GraphSpec, NodeId, PortCapability, PortFormat, PortId};
use ruviform_media_core::{ClockDomainId, ClockEpochId, TimestampProvenance};
use ruviform_observability::{Counter, CounterReader, counter};
use ruviform_rt::{Consumer, Producer};

use super::buffer::Slot;
use super::compile::PreparedPlan;
use super::runtime::{GraphRuntime, SharedStatus};

/// Probe time context, clock marks and observation alignment
/// (Issue #309) — non-RT side only.
pub mod time;

pub use time::{
    AlignPolicy, Alignment, AlignmentBasis, AlignmentCondition, CoverageClass, CoverageEvent,
    CoverageInterval, FrameInterval, GenerationRange, IncomparableReason, LatencyKnowledge,
    ObservationCoverage, ObservationFormat, ObservationSpan, ProbeClockMark, ProbePrecision,
    ProbeTimeContext, align_observation_points,
};

/// Maximum channels a [`MetricRecord`] carries inline (POD bound —
/// records travel the telemetry channel without allocation).
pub const METRIC_MAX_CHANNELS: usize = 16;

/// Broker-issued probe identifier — stable for the broker's lifetime.
/// `0` is never issued.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProbeId(u64);

impl ProbeId {
    /// Raw issued value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Which schedule position a binding taps — resolved at bind time,
/// never re-derived on the RT side.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TapKind {
    /// Processor-less `Source` output — caller-fed graph input slot.
    GraphInput,
    /// Processor node input — post-materialization (merged/scratch
    /// or direct producer slot).
    NodeInput,
    /// Processor node output — post-process, after the slot is
    /// stamped for this callback.
    NodeOutput,
    /// Processor-less `Sink` input — graph output drain point.
    GraphOutput,
}

/// Per-probe observation state — independent of processor Bypass.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProbeMode {
    /// No tap work at all — the tap site skips slot borrows.
    Off,
    /// Per-channel Peak/RMS per block, delivered as [`MetricRecord`].
    Monitor,
    /// Bounded PCM copy per block into the chunk pool.
    Capture,
}

/// Capture budget — the maximum frames a single capture may record
/// before it auto-stops and returns to `Monitor`.
#[derive(Copy, Clone, Debug)]
pub struct CaptureConfig {
    /// Frame budget (`0` rejected at `capture`).
    pub max_frames: u64,
}

/// A logical observation point: one port of one node.
#[derive(Copy, Clone, Debug)]
pub struct ProbePoint {
    /// Owning node.
    pub node: NodeId,
    /// Observed port.
    pub port: PortId,
}

/// Resolution outcome of a point against spec + published plan.
/// The physical slot resolution leads — a point is never reported
/// observable without a real backing slot.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PointAvailability {
    /// `ProbePoint` capability declared and a slot is bound.
    Configured,
    /// No declaration, but a slot is bound — observable anyway.
    Inferred,
    /// Audio port with no backing slot (unconnected input, node not
    /// in the published schedule).
    Unavailable,
    /// Non-audio port — TimedEvent/Control observation is deferred.
    Unsupported,
    /// Node or port not present in the spec.
    Unknown,
}

/// One resolved point of a capability report.
#[derive(Copy, Clone, Debug)]
pub struct ObservationPoint {
    /// The point.
    pub point: ProbePoint,
    /// Resolution classification.
    pub availability: PointAvailability,
    /// Tap position when a slot is bound.
    pub kind: Option<TapKind>,
    /// Whether `ProbePoint` was declared on the port.
    pub declared: bool,
}

/// Deterministic capability enumeration over one spec + plan pair.
#[derive(Clone, Debug)]
pub struct ObservationReport {
    /// Plan generation the report resolved against.
    pub generation: u64,
    /// Every port of every spec node, in declaration order.
    pub points: Vec<ObservationPoint>,
}

/// Metadata envelope carried by every emitted record — the alignment
/// contract (`generation` + `sample_position` place two points on the
/// same processing span).
#[derive(Copy, Clone, Debug)]
pub struct RecordMeta {
    /// Emitting probe.
    pub probe: ProbeId,
    /// Observed node/port.
    pub node: NodeId,
    /// Observed port.
    pub port: PortId,
    /// Plan generation the binding resolved against.
    pub generation: u64,
    /// Absolute sample position of the block's first frame
    /// (`RtRuntime` frame cursor).
    pub sample_position: u64,
    /// Frames observed this block.
    pub frames: u32,
    /// Logical channel count of the observed slot.
    pub channels: u16,
    /// Declared sample rate of the observed port's audio format.
    pub sample_rate: u32,
    /// Path latency to this tap point in frames.
    pub effective_latency: u32,
    /// Clock identity of the frame-cursor series this record belongs
    /// to (Issue #309) — `UNKNOWN` until the deck is wired via
    /// `SetClockMark`; records self-describe their `(domain, epoch)`
    /// so interpretation never depends on registry state.
    pub clock: ProbeClockMark,
}

/// Per-block Peak/RMS — POD, telemetry-channel payload.
#[derive(Copy, Clone, Debug)]
pub struct MetricRecord {
    /// Envelope.
    pub meta: RecordMeta,
    /// Valid entries of `levels` (`<= METRIC_MAX_CHANNELS`).
    pub n: u16,
    /// Per-channel levels for the block.
    pub levels: [ChannelLevel; METRIC_MAX_CHANNELS],
}

/// Why a capture stopped.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// Explicit `stop_capture`.
    Requested,
    /// `CaptureConfig::max_frames` exhausted — mode returns to
    /// `Monitor`.
    BudgetExhausted,
    /// A generation rebind committed while the probe was capturing
    /// and its staged mode is no longer `Capture` — the unspent
    /// budget is discarded, never carried into the new generation
    /// (Issue #308).
    GenerationBoundary,
}

/// Why a point produced no data record for a block.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnobservedReason {
    /// Slot stamp predates the current callback — producer missing
    /// or not yet run (the same freshness audit consumers apply).
    Stale,
    /// The slot was mutably borrowed at tap time.
    BorrowConflict,
}

/// Non-data events on the telemetry channel.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StatusKind {
    /// Binding applied on the RT side.
    Installed,
    /// `set_mode` applied.
    ModeChanged {
        /// New mode.
        to: ProbeMode,
    },
    /// Capture began with a frame budget.
    CaptureStarted {
        /// Remaining frame budget.
        budget_frames: u64,
    },
    /// Capture ended.
    CaptureStopped {
        /// Why.
        reason: StopReason,
    },
    /// Binding's `bound_generation` no longer matches the active
    /// plan — the point is skipped until `on_publish` rebinds, so a
    /// stale slot is never observed as valid data.
    Unbound,
    /// Freshness/borrow audit rejected the slot this block.
    Unobserved {
        /// Why.
        reason: UnobservedReason,
    },
    /// Chunk pool exhausted — this block's PCM was dropped
    /// (`lost_blocks` counter holds the cumulative total).
    LostBlocks,
    /// Telemetry channel full — a record was dropped
    /// (`lost_records` counter holds the cumulative total).
    LostRecords,
    /// Probe removed on the RT side.
    Removed,
    /// Binding dropped by a committed generation rebind — the
    /// probe's point no longer resolves against the new plan
    /// (Issue #308). The broker keeps the probe installed; a later
    /// publish may re-bind it.
    RebindDropped,
}

/// Status event — POD, telemetry-channel payload.
#[derive(Copy, Clone, Debug)]
pub struct StatusRecord {
    /// Emitting probe.
    pub probe: ProbeId,
    /// Point identity.
    pub node: NodeId,
    /// Point identity.
    pub port: PortId,
    /// Plan generation at emit time.
    pub generation: u64,
    /// Absolute block position at emit time.
    pub sample_position: u64,
    /// Clock identity of the cursor series at emit time (Issue #309).
    pub clock: ProbeClockMark,
    /// What happened.
    pub kind: StatusKind,
}

/// Telemetry-channel payload (POD — metrics and status only).
#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum TelemetryRecord {
    /// One block's Peak/RMS.
    Metric(MetricRecord),
    /// Lifecycle/fault/loss event.
    Status(StatusRecord),
}

/// One block of captured PCM — preallocated pool object, planar
/// layout (`data[ch * chunk_frames + frame]`). Owns no per-block
/// allocation: the box itself is the pooled object that travels
/// `free → chunks_out → collector → free`.
#[derive(Debug)]
pub struct CaptureChunk {
    /// Envelope — `None` until the first capture writes it.
    pub meta: Option<RecordMeta>,
    /// Frames valid this record (`<= chunk capacity`).
    pub frames: usize,
    /// Channels valid this record.
    pub channels: usize,
    /// Planar sample storage (`chunk_channels × chunk_frames`).
    pub data: Box<[f32]>,
}

/// Event handed to [`ProbeCollector::drain`]'s callback.
#[non_exhaustive]
pub enum ProbeEvent<'a> {
    /// Per-block Peak/RMS.
    Metric(MetricRecord),
    /// Captured PCM — borrow ends when the callback returns; the
    /// chunk is then recycled to the free pool automatically.
    Captured(&'a CaptureChunk),
    /// Lifecycle/fault/loss event.
    Status(StatusRecord),
}

/// Defined non-RT control-plane failures — never a panic or a
/// silent no-op.
#[non_exhaustive]
#[derive(Debug)]
pub enum ProbeError {
    /// `ProbeId` not installed.
    UnknownProbe,
    /// The point resolved to a non-installable class.
    PointUnavailable {
        /// The classification.
        availability: PointAvailability,
    },
    /// Probe table is full (`ProbeConfig::max_probes`).
    CapacityExceeded {
        /// Bound.
        max: usize,
    },
    /// `CaptureConfig::max_frames` was zero (`max` reports the pool
    /// budget for context).
    BudgetExceeded {
        /// Requested.
        requested: u64,
        /// Pool total.
        max: u64,
    },
    /// Command channel could not accept the command.
    CommandChannelFull,
}

impl core::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownProbe => f.write_str("probe not installed"),
            Self::PointUnavailable { availability } => {
                write!(f, "point not installable: {availability:?}")
            }
            Self::CapacityExceeded { max } => write!(f, "probe table full (max {max})"),
            Self::BudgetExceeded { requested, max } => {
                write!(f, "capture budget {requested} exceeds pool {max}")
            }
            Self::CommandChannelFull => f.write_str("probe command channel full"),
        }
    }
}

impl core::error::Error for ProbeError {}

/// RT-side binding resolved by the broker — POD, copied by value.
#[derive(Copy, Clone, Debug)]
pub struct ProbeBinding {
    /// Probe identity.
    pub probe: ProbeId,
    /// Observed node.
    pub node: NodeId,
    /// Observed port.
    pub port: PortId,
    /// Tap position.
    pub kind: TapKind,
    /// Arena slot to observe.
    pub slot: usize,
    /// Plan generation this binding resolved against — the RT side
    /// skips (`Unbound`) and never observes a slot under a newer
    /// generation than the one the slot was resolved for.
    pub bound_generation: u64,
    /// Current mode.
    pub mode: ProbeMode,
    /// Remaining capture budget in frames.
    pub capture_remaining: u64,
    /// Logical channels of the observed slot.
    pub channels: u16,
    /// Declared sample rate.
    pub sample_rate: u32,
    /// Path latency to the tap point.
    pub effective_latency: u32,
}

/// Command channel payload — generation-scoped variants
/// (`RebindBegin`/`RebindStage`/`RebindCommit`, and `Bind` from
/// `install`) are applied only when their generation equals the
/// *published* plan generation (stale ones are dropped and
/// counted); the rest apply every callback.
///
/// Rebind is staged, not incremental: `RebindBegin` opens a staging
/// area, `RebindStage` fills it, and only `RebindCommit` swaps it
/// into the live bindings — so a rebind is all-or-nothing on the RT
/// side (Issue #308).
#[non_exhaustive]
#[derive(Debug)]
pub enum ProbeCommand {
    /// Install/replace one binding for `generation`.
    Bind {
        /// Target generation.
        generation: u64,
        /// Resolved binding.
        binding: ProbeBinding,
    },
    /// Open a rebind staging area for `generation` — clears any
    /// staged content and marks the attempt's epoch.
    RebindBegin {
        /// Target generation.
        generation: u64,
    },
    /// Stage one binding into the open rebind attempt — upserts by
    /// probe id; applied only while `staging` matches `generation`.
    RebindStage {
        /// Target generation.
        generation: u64,
        /// Resolved binding.
        binding: ProbeBinding,
    },
    /// Commit the staged set for `generation` — atomically swaps it
    /// into the live bindings and emits per-probe statuses.
    RebindCommit {
        /// Target generation.
        generation: u64,
    },
    /// Switch mode (capture state preserved except via the capture
    /// commands below).
    SetMode {
        /// Target probe.
        probe: ProbeId,
        /// New mode.
        mode: ProbeMode,
    },
    /// Begin capture with a frame budget.
    StartCapture {
        /// Target probe.
        probe: ProbeId,
        /// Frame budget.
        budget_frames: u64,
    },
    /// End capture, returning to `Monitor`.
    StopCapture {
        /// Target probe.
        probe: ProbeId,
    },
    /// Drop the binding entirely.
    Remove {
        /// Target probe.
        probe: ProbeId,
    },
    /// Replace the deck's clock mark — series/deck-scoped (not
    /// per-probe): every tap shares the same frame cursor, so one
    /// mark covers all bindings. A lifetime command applied every
    /// callback like `SetMode`; the non-RT side mints marks and bumps
    /// epochs, the RT side only stores the copy (Issue #309).
    SetClockMark {
        /// New mark for the cursor series.
        mark: ProbeClockMark,
    },
}

/// Probe construction bounds — all allocation happens at [`pair`].
#[derive(Clone, Debug)]
pub struct ProbeConfig {
    /// Maximum simultaneously installed probes (binding arena size).
    pub max_probes: usize,
    /// Capture chunk pool size — also the `chunks_out` capacity, so
    /// a filled chunk always fits the output channel by construction.
    pub pool_chunks: usize,
    /// Frames per chunk. Must be `>= CompilerConfig::max_frames` so
    /// one callback always fits one chunk.
    pub chunk_frames: usize,
    /// Channel planes per chunk (`>= CompilerConfig::max_channels`).
    pub chunk_channels: usize,
    /// Metric/status channel capacity (drop-newest on overflow).
    pub telemetry_capacity: usize,
    /// Probe command channel capacity.
    pub command_capacity: usize,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            max_probes: 32,
            pool_chunks: 8,
            chunk_frames: 512,
            chunk_channels: 8,
            telemetry_capacity: 256,
            command_capacity: 64,
        }
    }
}

/// Capture continuation policy across a generation rebind
/// (Issue #308) — set per probe via
/// [`ProbeBroker::set_capture_policy`](ProbeBroker::set_capture_policy)
/// and consumed by
/// [`ProbeBroker::on_publish`](ProbeBroker::on_publish) when it
/// stages bindings. An in-flight capture is never silently carried
/// into the new generation: the unspent budget is either discarded
/// or restarted as an explicit new segment.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapturePolicy {
    /// Cut the capture at the boundary: the probe is staged as
    /// `Monitor`, the unspent budget is discarded, and the deck
    /// emits `CaptureStopped { reason: GenerationBoundary }` when
    /// the rebind commits. This is the default — matching the
    /// consumer expectation that a graph change ends the capture
    /// segment and never auto-restarts it.
    StopAtBoundary,
    /// Restart capture under the new generation with the full
    /// configured budget — an explicit new segment, not a
    /// continuation: the deck emits `CaptureStarted` at commit and
    /// `RecordMeta.generation` separates its chunks from the
    /// previous segment's.
    Restart,
}

/// GUI-readable receipt for one [`ProbeBroker::on_publish`] rebind
/// attempt (Issue #308). It covers the synchronous stages —
/// requested and enqueued; RT-installed confirmation arrives as
/// [`StatusKind`] records on the telemetry channel (matched by
/// `generation`), and observed data as [`MetricRecord`] /
/// [`CaptureChunk`] records under the same generation.
pub struct RebindReceipt {
    /// Plan generation the rebind targets.
    pub generation: u64,
    /// Probes resolved against the new plan — each becomes an
    /// `Installed` status when the rebind commits.
    pub bound: Vec<ProbeId>,
    /// Installed probes whose point no longer resolves — their
    /// bindings are dropped at commit (`RebindDropped` status). The
    /// broker keeps them installed; a later publish may re-bind.
    pub unbound: Vec<ProbeId>,
    /// Bound probes whose in-flight capture is cut at the boundary
    /// (`CaptureStopped { reason: GenerationBoundary }` at commit).
    pub capture_stopped: Vec<ProbeId>,
    /// Bound probes whose capture restarts with a fresh full budget
    /// under the new generation (`CaptureStarted` at commit — a new
    /// segment).
    pub capture_restarted: Vec<ProbeId>,
}

/// Monotonic probe counters readable on the non-RT side.
#[derive(Copy, Clone, Debug, Default)]
pub struct ProbeCounts {
    /// Telemetry records dropped (channel full).
    pub lost_records: u64,
    /// Capture blocks dropped (pool exhausted).
    pub lost_blocks: u64,
    /// Blocks skipped by the freshness/borrow audit.
    pub unobserved: u64,
    /// Blocks skipped by the generation guard.
    pub unbound: u64,
}

/// RT half — lives inside [`RtRuntime`], driven by `process`.
///
/// **RT contract:** every method is RT-suitable — no allocation, no
/// blocking, no destructors. `bindings` is a fixed-capacity arena
/// mutated only by `Vec` ops that cannot reallocate (`clear`,
/// in-place replace, length-checked `push`).
pub struct ProbeDeck {
    commands: Consumer<ProbeCommand>,
    free: Consumer<Box<CaptureChunk>>,
    chunks_out: Producer<Box<CaptureChunk>>,
    telemetry: Producer<TelemetryRecord>,
    bindings: Vec<ProbeBinding>,
    /// Rebind staging area — never observed until `RebindCommit`
    /// swaps it into `bindings` (Issue #308). Same `max_probes`
    /// capacity as `bindings`; mutated only by capacity-safe `Vec`
    /// ops.
    staged: Vec<ProbeBinding>,
    /// Generation of the open rebind attempt (`None` = no open
    /// attempt — `RebindStage`/`RebindCommit` are rejected).
    staging: Option<u64>,
    /// Clock mark currently stamped on records — `UNKNOWN` until a
    /// `SetClockMark` command lands (Issue #309). Series/deck-scoped:
    /// every binding shares the same frame cursor.
    current_mark: ProbeClockMark,
    max_probes: usize,
    chunk_frames: usize,
    lost_records: Counter,
    lost_blocks: Counter,
    unobserved: Counter,
    unbound: Counter,
}

impl ProbeDeck {
    /// Drains the command channel: generation-scoped variants apply
    /// only when `generation` matches the published plan (stale are
    /// dropped + counted — a broker `Rebind` always follows a
    /// publish, so a dropped stale bind is never the last word).
    /// Rebind commands additionally require the staging epoch opened
    /// by `RebindBegin`, so a stage/commit without its begin — or
    /// from an invalidated attempt — is rejected rather than
    /// half-applied (Issue #308).
    pub(crate) fn drain_commands(&mut self, generation: u64, base: u64, status: &SharedStatus) {
        while let Ok(cmd) = self.commands.try_pop() {
            match cmd {
                ProbeCommand::RebindBegin { generation: g } => {
                    self.rebind_begin(g, generation, status);
                }
                ProbeCommand::RebindStage {
                    generation: g,
                    binding,
                } => {
                    self.rebind_stage(g, binding, generation, status);
                }
                ProbeCommand::RebindCommit { generation: g } => {
                    self.rebind_commit(g, generation, base, status);
                }
                ProbeCommand::Bind {
                    generation: g,
                    binding,
                } => {
                    if g != generation {
                        status.record_stale_commands();
                    } else if let Some(existing) =
                        self.bindings.iter_mut().find(|b| b.probe == binding.probe)
                    {
                        *existing = binding;
                        self.push_status(&binding, generation, base, StatusKind::Installed);
                    } else if self.bindings.len() < self.max_probes {
                        self.bindings.push(binding);
                        self.push_status(&binding, generation, base, StatusKind::Installed);
                    } else {
                        self.lost_records.increment();
                    }
                }
                ProbeCommand::SetMode { probe, mode } => {
                    if let Some(b) = self.bindings.iter_mut().find(|b| b.probe == probe) {
                        b.mode = mode;
                        let b = *b;
                        self.push_status(
                            &b,
                            generation,
                            base,
                            StatusKind::ModeChanged { to: mode },
                        );
                    }
                }
                ProbeCommand::StartCapture {
                    probe,
                    budget_frames,
                } => {
                    if let Some(b) = self.bindings.iter_mut().find(|b| b.probe == probe) {
                        b.mode = ProbeMode::Capture;
                        b.capture_remaining = budget_frames;
                        let b = *b;
                        self.push_status(
                            &b,
                            generation,
                            base,
                            StatusKind::CaptureStarted { budget_frames },
                        );
                    }
                }
                ProbeCommand::StopCapture { probe } => {
                    if let Some(b) = self.bindings.iter_mut().find(|b| b.probe == probe)
                        && b.mode == ProbeMode::Capture
                    {
                        b.mode = ProbeMode::Monitor;
                        let b = *b;
                        self.push_status(
                            &b,
                            generation,
                            base,
                            StatusKind::CaptureStopped {
                                reason: StopReason::Requested,
                            },
                        );
                    }
                }
                ProbeCommand::Remove { probe } => {
                    if let Some(pos) = self.bindings.iter().position(|b| b.probe == probe) {
                        let b = self.bindings[pos];
                        self.bindings.remove(pos);
                        self.push_status(&b, generation, base, StatusKind::Removed);
                    }
                }
                ProbeCommand::SetClockMark { mark } => {
                    self.current_mark = mark;
                }
            }
        }
    }

    /// The clock mark currently stamped on records.
    pub(crate) fn current_mark(&self) -> ProbeClockMark {
        self.current_mark
    }

    /// `RebindBegin`: opens the staging epoch for `g`, discarding
    /// any leftover staged content from an incomplete attempt.
    fn rebind_begin(&mut self, g: u64, generation: u64, status: &SharedStatus) {
        if g == generation {
            self.staged.clear();
            self.staging = Some(g);
        } else {
            status.record_stale_commands();
        }
    }

    /// `RebindStage`: upserts one binding into the open staging area.
    /// Rejected (counted stale) unless the command's generation both
    /// matches the published plan and the open staging epoch.
    fn rebind_stage(
        &mut self,
        g: u64,
        binding: ProbeBinding,
        generation: u64,
        status: &SharedStatus,
    ) {
        if g != generation || self.staging != Some(g) {
            status.record_stale_commands();
            return;
        }
        if let Some(existing) = self.staged.iter_mut().find(|b| b.probe == binding.probe) {
            *existing = binding;
        } else if self.staged.len() < self.max_probes {
            self.staged.push(binding);
        } else {
            // Staged area full — the attempt can never commit
            // correctly; invalidate it so the Commit is rejected too.
            self.staging = None;
            self.lost_records.increment();
        }
    }

    /// `RebindCommit`: swaps the staged set live only when the
    /// staging epoch still matches — anything else is stale.
    fn rebind_commit(&mut self, g: u64, generation: u64, base: u64, status: &SharedStatus) {
        if g == generation && self.staging == Some(g) {
            self.staging = None;
            self.commit_rebind(generation, base);
        } else {
            status.record_stale_commands();
        }
    }

    /// Atomically swaps the staged rebind set into the live
    /// bindings, then reports the transition per probe (Issue #308).
    /// Both Vecs keep their `max_probes` capacity — `mem::swap` and
    /// `clear` never reallocate, so this stays inside the RT
    /// contract. Runs inside `drain_commands`, before any `run_site`
    /// of the same callback — observers see either the whole old set
    /// or the whole new set, never a mix.
    fn commit_rebind(&mut self, generation: u64, base: u64) {
        core::mem::swap(&mut self.bindings, &mut self.staged);
        // `bindings` is now the staged set; `staged` holds the
        // previous live set for diffing, cleared before return.
        for i in 0..self.bindings.len() {
            let b = self.bindings[i];
            self.push_status(&b, generation, base, StatusKind::Installed);
        }
        for i in 0..self.staged.len() {
            let old = self.staged[i];
            let Some(new) = self.bindings.iter().find(|b| b.probe == old.probe).copied() else {
                self.push_status(&old, generation, base, StatusKind::RebindDropped);
                continue;
            };
            match (old.mode, new.mode) {
                // Restart policy: explicit new capture segment.
                (_, ProbeMode::Capture) => {
                    self.push_status(
                        &new,
                        generation,
                        base,
                        StatusKind::CaptureStarted {
                            budget_frames: new.capture_remaining,
                        },
                    );
                }
                (ProbeMode::Capture, _) => {
                    self.push_status(
                        &old,
                        generation,
                        base,
                        StatusKind::CaptureStopped {
                            reason: StopReason::GenerationBoundary,
                        },
                    );
                }
                _ => {}
            }
        }
        self.staged.clear();
    }

    /// Emits one status record; drops + counts on a full channel.
    fn push_status(&mut self, b: &ProbeBinding, generation: u64, base: u64, kind: StatusKind) {
        let rec = TelemetryRecord::Status(StatusRecord {
            probe: b.probe,
            node: b.node,
            port: b.port,
            generation,
            sample_position: base,
            clock: self.current_mark,
            kind,
        });
        if self.telemetry.try_push(rec).is_err() {
            self.lost_records.increment();
        }
    }

    /// Runs all bindings of `kind` (optionally scoped to `node`)
    /// against `arena` for this callback.
    pub(crate) fn run_site(
        &mut self,
        kind: TapKind,
        node: Option<NodeId>,
        arena: &[core::cell::RefCell<Slot>],
        c: &TapCtx,
    ) {
        // Cheap OFF gate: scan without touching the arena at all.
        if !self
            .bindings
            .iter()
            .any(|b| b.kind == kind && b.mode != ProbeMode::Off)
        {
            return;
        }
        for i in 0..self.bindings.len() {
            let b = self.bindings[i];
            if b.kind != kind || b.mode == ProbeMode::Off {
                continue;
            }
            if node.is_some_and(|n| b.node != n) {
                continue;
            }
            if b.bound_generation != c.generation {
                self.unbound.increment();
                self.push_status(&b, c.generation, c.base, StatusKind::Unbound);
                continue;
            }
            let Some(cell) = arena.get(b.slot) else {
                continue;
            };
            let Ok(slot) = cell.try_borrow() else {
                self.unobserved.increment();
                self.push_status(
                    &b,
                    c.generation,
                    c.base,
                    StatusKind::Unobserved {
                        reason: UnobservedReason::BorrowConflict,
                    },
                );
                continue;
            };
            if slot.stamp() != c.seq {
                drop(slot);
                self.unobserved.increment();
                self.push_status(
                    &b,
                    c.generation,
                    c.base,
                    StatusKind::Unobserved {
                        reason: UnobservedReason::Stale,
                    },
                );
                continue;
            }
            let meta = RecordMeta {
                probe: b.probe,
                node: b.node,
                port: b.port,
                generation: c.generation,
                sample_position: c.base,
                frames: u32::try_from(c.frames).unwrap_or(u32::MAX),
                channels: b.channels,
                sample_rate: b.sample_rate,
                effective_latency: b.effective_latency,
                clock: c.mark,
            };
            match b.mode {
                ProbeMode::Monitor => self.emit_metric(&slot, meta),
                ProbeMode::Capture => self.emit_capture(i, b, &slot, meta, c),
                ProbeMode::Off => {}
            }
        }
    }

    /// One block's Peak/RMS onto the telemetry channel.
    fn emit_metric(&mut self, slot: &Slot, meta: RecordMeta) {
        let n = usize::from(meta.channels).min(METRIC_MAX_CHANNELS);
        let mut rec = MetricRecord {
            meta,
            n: u16::try_from(n).unwrap_or(u16::MAX),
            levels: [ChannelLevel::default(); METRIC_MAX_CHANNELS],
        };
        for (ch, level) in rec.levels.iter_mut().enumerate().take(n) {
            if let Some(plane) = slot.data.channel(ch) {
                *level = ChannelLevel::measure(&plane[..meta.frames as usize]);
            }
        }
        if self
            .telemetry
            .try_push(TelemetryRecord::Metric(rec))
            .is_err()
        {
            self.lost_records.increment();
        }
    }

    /// One block's PCM copy into a pooled chunk → `chunks_out`.
    fn emit_capture(
        &mut self,
        i: usize,
        mut b: ProbeBinding,
        slot: &Slot,
        meta: RecordMeta,
        c: &TapCtx,
    ) {
        let n = c.frames.min(self.chunk_frames);
        let Ok(mut chunk) = self.free.try_pop() else {
            self.lost_blocks.increment();
            self.push_status(&b, c.generation, c.base, StatusKind::LostBlocks);
            return;
        };
        let mut meta = meta;
        meta.frames = u32::try_from(n).unwrap_or(u32::MAX);
        chunk.meta = Some(meta);
        chunk.frames = n;
        chunk.channels = usize::from(b.channels);
        for ch in 0..usize::from(b.channels) {
            if let (Some(plane), Some(dst)) = (
                slot.data.channel(ch),
                chunk
                    .data
                    .get_mut(ch * self.chunk_frames..ch * self.chunk_frames + n),
            ) {
                dst.copy_from_slice(&plane[..n]);
            }
        }
        debug_assert!(self.chunks_out.try_push(chunk).is_ok());
        b.capture_remaining = b.capture_remaining.saturating_sub(n as u64);
        if b.capture_remaining == 0 {
            b.mode = ProbeMode::Monitor;
            self.push_status(
                &b,
                c.generation,
                c.base,
                StatusKind::CaptureStopped {
                    reason: StopReason::BudgetExhausted,
                },
            );
        }
        self.bindings[i] = b;
    }
}

/// Per-callback constants shared by every tap site — bundles the
/// schedule position the runtime passes to [`ProbeDeck::run_site`].
pub(crate) struct TapCtx {
    /// Slot write-stamp sequence of this callback.
    pub seq: u64,
    /// Frames this callback.
    pub frames: usize,
    /// Published plan generation.
    pub generation: u64,
    /// Absolute position of the block's first frame (`frame_cursor`).
    pub base: u64,
    /// Clock mark of the cursor series — refreshed from the deck
    /// after `drain_commands` each callback (Issue #309).
    pub mark: ProbeClockMark,
}

/// Non-RT control plane — resolves points against the published
/// plan, tracks installed probes, and sends commands.
///
/// **RT contract:** non-RT only — allocates and sends.
pub struct ProbeBroker {
    commands: Producer<ProbeCommand>,
    probes: BTreeMap<ProbeId, ProbeEntry>,
    next: u64,
    max_probes: usize,
    pool_budget: u64,
    /// Non-RT time-series registry entry for this pair (Issue #309) —
    /// one cursor series = one domain; records carry its mark.
    time: ProbeTimeContext,
}

#[derive(Copy, Clone, Debug)]
struct ProbeEntry {
    point: ProbePoint,
    mode: ProbeMode,
    budget_frames: u64,
    /// Capture continuation policy consumed by `on_publish`
    /// (Issue #308) — never sent to the RT side directly.
    capture_policy: CapturePolicy,
}

/// Resolved physical binding for one point.
#[derive(Copy, Clone, Debug)]
struct Resolved {
    slot: usize,
    kind: TapKind,
    channels: u16,
    sample_rate: u32,
    effective_latency: u32,
}

/// Resolves a point to `(availability, physical binding)` — the
/// physical slot resolution leads; declaration only distinguishes
/// `Configured` from `Inferred`.
fn resolve_point(
    spec: &GraphSpec,
    plan: &PreparedPlan,
    point: ProbePoint,
) -> (PointAvailability, Option<Resolved>) {
    let Some(node_spec) = spec.node(point.node) else {
        return (PointAvailability::Unknown, None);
    };
    let Some(port_spec) = node_spec.ports().iter().find(|p| p.id() == point.port) else {
        return (PointAvailability::Unknown, None);
    };
    let PortFormat::Audio(format) = port_spec.format() else {
        return (PointAvailability::Unsupported, None);
    };
    let declared = port_spec
        .capabilities()
        .contains(&PortCapability::ProbePoint);
    let snode = plan.schedule.iter().find(|s| s.node == point.node);
    let resolved =
        if let Some(&(_, slot)) = plan.inputs.iter().find(|(port, _)| *port == point.port) {
            Some((slot, TapKind::GraphInput, 0))
        } else if let Some(&(_, slot)) = plan.outputs.iter().find(|(port, _)| *port == point.port) {
            Some((slot, TapKind::GraphOutput, plan.total_latency))
        } else {
            match port_spec.direction() {
                Direction::Output => snode.and_then(|s| {
                    s.outputs
                        .iter()
                        .find(|(port, _)| *port == point.port)
                        .map(|&(_, slot)| (slot, TapKind::NodeOutput, s.latency_accum))
                }),
                Direction::Input => snode.and_then(|s| {
                    s.inputs
                        .iter()
                        .find(|b| b.port == point.port)
                        .and_then(|binding| {
                            binding.effective_slot().map(|slot| {
                                // Path latency to this input: max over
                                // edges of (producer accum + PDC delay).
                                let latency = binding
                                    .edges
                                    .iter()
                                    .map(|e| {
                                        let prod = plan
                                            .schedule
                                            .iter()
                                            .find(|s| s.outputs.iter().any(|(_, sl)| *sl == e.slot))
                                            .map_or(0, |s| s.latency_accum);
                                        prod.saturating_add(e.delay_frames)
                                    })
                                    .max()
                                    .unwrap_or(0);
                                (slot, TapKind::NodeInput, latency)
                            })
                        })
                }),
                _ => None,
            }
        };
    let Some((slot, kind, effective_latency)) = resolved else {
        return (PointAvailability::Unavailable, None);
    };
    let channels = plan.slots.get(slot).map_or(0, |s| s.channels);
    (
        if declared {
            PointAvailability::Configured
        } else {
            PointAvailability::Inferred
        },
        Some(Resolved {
            slot,
            kind,
            channels,
            sample_rate: format.sample_rate.get(),
            effective_latency,
        }),
    )
}

/// Enumerates every port of every spec node against `plan` —
/// deterministic for a given `(spec, plan)` pair.
#[must_use]
pub fn capabilities(spec: &GraphSpec, plan: &PreparedPlan) -> ObservationReport {
    let mut points = Vec::new();
    for node in spec.nodes() {
        for port in node.ports() {
            let (availability, resolved) = resolve_point(
                spec,
                plan,
                ProbePoint {
                    node: node.id(),
                    port: port.id(),
                },
            );
            points.push(ObservationPoint {
                point: ProbePoint {
                    node: node.id(),
                    port: port.id(),
                },
                availability,
                kind: resolved.map(|r| r.kind),
                declared: port.capabilities().contains(&PortCapability::ProbePoint),
            });
        }
    }
    ObservationReport {
        generation: plan.generation,
        points,
    }
}

impl ProbeBroker {
    /// Installs `point` in `mode`. The point must resolve to
    /// `Configured` or `Inferred` against the *currently published*
    /// plan — binding is stamped with that generation, so a publish
    /// between install and apply makes the command stale (dropped +
    /// counted) until [`on_publish`](Self::on_publish) rebinds.
    ///
    /// # Errors
    /// [`ProbeError::PointUnavailable`], [`ProbeError::CapacityExceeded`],
    /// [`ProbeError::CommandChannelFull`].
    pub fn install(
        &mut self,
        runtime: &GraphRuntime,
        spec: &GraphSpec,
        point: ProbePoint,
        mode: ProbeMode,
    ) -> Result<ProbeId, ProbeError> {
        let plan = runtime.published_plan();
        self.note_generation(plan.generation);
        let (availability, resolved) = resolve_point(spec, plan, point);
        let Some(r) = resolved else {
            return Err(ProbeError::PointUnavailable { availability });
        };
        if self.probes.len() >= self.max_probes {
            return Err(ProbeError::CapacityExceeded {
                max: self.max_probes,
            });
        }
        self.next += 1;
        let probe = ProbeId(self.next);
        let binding = ProbeBinding {
            probe,
            node: point.node,
            port: point.port,
            kind: r.kind,
            slot: r.slot,
            bound_generation: plan.generation,
            mode,
            capture_remaining: 0,
            channels: r.channels,
            sample_rate: r.sample_rate,
            effective_latency: r.effective_latency,
        };
        self.commands
            .try_push(ProbeCommand::Bind {
                generation: plan.generation,
                binding,
            })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        self.probes.insert(
            probe,
            ProbeEntry {
                point,
                mode,
                budget_frames: 0,
                capture_policy: CapturePolicy::StopAtBoundary,
            },
        );
        Ok(probe)
    }

    /// Switches `probe`'s mode.
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn set_mode(&mut self, probe: ProbeId, mode: ProbeMode) -> Result<(), ProbeError> {
        let entry = self
            .probes
            .get_mut(&probe)
            .ok_or(ProbeError::UnknownProbe)?;
        self.commands
            .try_push(ProbeCommand::SetMode { probe, mode })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        entry.mode = mode;
        Ok(())
    }

    /// Starts a bounded capture. `max_frames` must be nonzero — the
    /// pool bounds *outstanding* PCM, not total budget: chunks return
    /// to `free` as the collector drains, so a capture may exceed the
    /// pool size (blocks dropped while the pool is exhausted are
    /// counted `lost_blocks`, never blocking).
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::BudgetExceeded`],
    /// [`ProbeError::CommandChannelFull`].
    pub fn capture(&mut self, probe: ProbeId, config: CaptureConfig) -> Result<(), ProbeError> {
        let entry = self
            .probes
            .get_mut(&probe)
            .ok_or(ProbeError::UnknownProbe)?;
        if config.max_frames == 0 {
            return Err(ProbeError::BudgetExceeded {
                requested: 0,
                max: self.pool_budget,
            });
        }
        self.commands
            .try_push(ProbeCommand::StartCapture {
                probe,
                budget_frames: config.max_frames,
            })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        entry.mode = ProbeMode::Capture;
        entry.budget_frames = config.max_frames;
        Ok(())
    }

    /// Stops an in-flight capture, returning the probe to `Monitor`.
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn stop_capture(&mut self, probe: ProbeId) -> Result<(), ProbeError> {
        let entry = self
            .probes
            .get_mut(&probe)
            .ok_or(ProbeError::UnknownProbe)?;
        self.commands
            .try_push(ProbeCommand::StopCapture { probe })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        entry.mode = ProbeMode::Monitor;
        Ok(())
    }

    /// Removes `probe` entirely.
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`], [`ProbeError::CommandChannelFull`].
    pub fn remove(&mut self, probe: ProbeId) -> Result<(), ProbeError> {
        if !self.probes.contains_key(&probe) {
            return Err(ProbeError::UnknownProbe);
        }
        self.commands
            .try_push(ProbeCommand::Remove { probe })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        self.probes.remove(&probe);
        Ok(())
    }

    /// Sets what happens to `probe`'s in-flight capture when the next
    /// generation rebind commits (Issue #308). The policy is consumed
    /// by [`on_publish`](Self::on_publish) at resolve time — it sends
    /// no command by itself, so changing it without a following
    /// publish has no observable effect.
    ///
    /// # Errors
    /// [`ProbeError::UnknownProbe`].
    pub fn set_capture_policy(
        &mut self,
        probe: ProbeId,
        policy: CapturePolicy,
    ) -> Result<(), ProbeError> {
        let entry = self
            .probes
            .get_mut(&probe)
            .ok_or(ProbeError::UnknownProbe)?;
        entry.capture_policy = policy;
        Ok(())
    }

    /// Rebinds every installed probe against `runtime`'s newly
    /// published plan — call after every successful `publish`.
    ///
    /// The rebind is staged and committed atomically on the RT side
    /// (Issue #308): `RebindBegin` opens a staging area,
    /// `RebindStage` fills it, and only `RebindCommit` swaps it into
    /// the live bindings — observers see either the whole old set or
    /// the whole new set, never a partial apply. Points that no
    /// longer resolve are *not* re-bound: the probe stays installed
    /// in the broker, its binding is dropped at commit with a
    /// `RebindDropped` status, and a later publish may re-bind it.
    ///
    /// An in-flight capture is never silently carried into the new
    /// generation: [`CapturePolicy::StopAtBoundary`] (default) cuts
    /// it at the boundary — the probe is staged as `Monitor`, the
    /// unspent budget is discarded, and the deck emits
    /// `CaptureStopped { reason: GenerationBoundary }`;
    /// [`CapturePolicy::Restart`] starts a fresh full-budget capture
    /// as an explicit new segment (`CaptureStarted` at commit,
    /// `RecordMeta.generation` separates the chunks).
    ///
    /// Returns a [`RebindReceipt`] covering the requested/enqueued
    /// stages; RT-installed confirmation arrives as [`StatusKind`]
    /// records on the telemetry channel and observed data as records
    /// under `receipt.generation`.
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`] — `PendingRetry`: the
    /// bounded command channel could not hold the whole
    /// Begin/Stage/Commit sequence, so nothing was committed. The
    /// staged prefix left in the queue is inert (no `RebindCommit`
    /// follows it) and the live bindings keep their previous
    /// generation-guarded state — safe to retry after the RT side
    /// drains.
    pub fn on_publish(
        &mut self,
        runtime: &GraphRuntime,
        spec: &GraphSpec,
    ) -> Result<RebindReceipt, ProbeError> {
        let plan = runtime.published_plan();
        let generation = plan.generation;
        self.note_generation(generation);
        // Resolve everything before pushing: the staged set only
        // reaches the RT side inside RebindCommit, so a mid-sequence
        // channel-full failure leaves the live bindings untouched.
        let mut staged = Vec::with_capacity(self.probes.len());
        let mut receipt = RebindReceipt {
            generation,
            bound: Vec::with_capacity(self.probes.len()),
            unbound: Vec::new(),
            capture_stopped: Vec::new(),
            capture_restarted: Vec::new(),
        };
        for (&probe, entry) in &mut self.probes {
            let Some(r) = resolve_point(spec, plan, entry.point).1 else {
                receipt.unbound.push(probe);
                continue;
            };
            let (mode, capture_remaining) = match entry.mode {
                ProbeMode::Capture => match entry.capture_policy {
                    CapturePolicy::StopAtBoundary => {
                        receipt.capture_stopped.push(probe);
                        entry.mode = ProbeMode::Monitor;
                        (ProbeMode::Monitor, 0)
                    }
                    CapturePolicy::Restart => {
                        receipt.capture_restarted.push(probe);
                        (ProbeMode::Capture, entry.budget_frames)
                    }
                },
                mode => (mode, 0),
            };
            receipt.bound.push(probe);
            staged.push(ProbeBinding {
                probe,
                node: entry.point.node,
                port: entry.point.port,
                kind: r.kind,
                slot: r.slot,
                bound_generation: generation,
                mode,
                capture_remaining,
                channels: r.channels,
                sample_rate: r.sample_rate,
                effective_latency: r.effective_latency,
            });
        }
        self.commands
            .try_push(ProbeCommand::RebindBegin { generation })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        for binding in staged {
            self.commands
                .try_push(ProbeCommand::RebindStage {
                    generation,
                    binding,
                })
                .map_err(|_| ProbeError::CommandChannelFull)?;
        }
        self.commands
            .try_push(ProbeCommand::RebindCommit { generation })
            .map_err(|_| ProbeError::CommandChannelFull)?;
        Ok(receipt)
    }

    /// Point of an installed probe (non-RT bookkeeping).
    #[must_use]
    pub fn point(&self, probe: ProbeId) -> Option<ProbePoint> {
        self.probes.get(&probe).map(|e| e.point)
    }

    /// Installed probe count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.probes.len()
    }

    /// Whether no probes are installed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    /// The non-RT time-series context for this pair (Issue #309):
    /// `(domain, epoch)` identity plus runtime-instance, anchors and
    /// generation window used to build [`ObservationSpan`]s. The
    /// record stream stays interpretable without it — records carry
    /// their own [`ProbeClockMark`].
    #[must_use]
    pub fn time_context(&self) -> &ProbeTimeContext {
        &self.time
    }

    /// Mutable access for consumer wiring — declare `sample_rate`,
    /// install `anchors`, adjust `provenance`. Non-RT only.
    pub fn time_context_mut(&mut self) -> &mut ProbeTimeContext {
        &mut self.time
    }

    /// Delivers the series' current `(domain, epoch)` mark to the
    /// deck via `SetClockMark`. Until this runs, records carry
    /// [`ProbeClockMark::UNKNOWN`]. Idempotent — re-sending the same
    /// mark is harmless.
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`].
    pub fn wire_clock_mark(&mut self) -> Result<(), ProbeError> {
        self.commands
            .try_push(ProbeCommand::SetClockMark {
                mark: self.time.mark(),
            })
            .map_err(|_| ProbeError::CommandChannelFull)
    }

    /// Declares a cursor-series discontinuity: bumps the epoch and
    /// delivers the new mark through the same command path. Records
    /// after the bump carry the new epoch, so spans built across the
    /// boundary classify as `EpochMismatch`/`DiscontinuityInWindow`
    /// instead of silently aligning (Issue #309).
    ///
    /// # Errors
    /// [`ProbeError::CommandChannelFull`] — the epoch is bumped
    /// before the push; on failure the context and deck disagree.
    /// Retry [`wire_clock_mark`](Self::wire_clock_mark) after the
    /// channel drains.
    pub fn bump_epoch(&mut self) -> Result<ClockEpochId, ProbeError> {
        self.time.epoch = ClockEpochId::new(self.time.epoch.get().wrapping_add(1));
        self.wire_clock_mark()?;
        Ok(self.time.epoch)
    }

    /// Records a plan generation the series has observed, widening
    /// `generation_window`.
    fn note_generation(&mut self, generation: u64) {
        self.time.generation_window = Some(match self.time.generation_window {
            None => GenerationRange::single(generation),
            Some(w) => GenerationRange {
                first: w.first.min(generation),
                last: w.last.max(generation),
            },
        });
    }
}

/// Non-RT receive side — drains `chunks_out` and `telemetry`,
/// recycles chunks to `free`, and mirrors the RT counters.
///
/// **RT contract:** non-RT only.
pub struct ProbeCollector {
    chunks_out: Consumer<Box<CaptureChunk>>,
    telemetry: Consumer<TelemetryRecord>,
    free: Producer<Box<CaptureChunk>>,
    lost_records: CounterReader,
    lost_blocks: CounterReader,
    unobserved: CounterReader,
    unbound: CounterReader,
}

impl ProbeCollector {
    /// Drains both channels, invoking `f` per record. `Captured`
    /// borrows end inside the callback — chunks recycle to the free
    /// pool automatically (a full `free` channel drops the box here,
    /// which is legal non-RT deallocation).
    ///
    /// Ordering: all queued telemetry first, then queued chunks —
    /// cross-channel interleave is not a span guarantee; align by
    /// `RecordMeta` instead.
    pub fn drain(&mut self, mut f: impl FnMut(ProbeEvent<'_>)) -> usize {
        let mut n = 0;
        while let Ok(rec) = self.telemetry.try_pop() {
            n += 1;
            match rec {
                TelemetryRecord::Metric(m) => f(ProbeEvent::Metric(m)),
                TelemetryRecord::Status(s) => f(ProbeEvent::Status(s)),
            }
        }
        while let Ok(chunk) = self.chunks_out.try_pop() {
            n += 1;
            f(ProbeEvent::Captured(&chunk));
            let _ = self.free.try_push(chunk);
        }
        n
    }

    /// Current counter mirror — each field is independently
    /// monotonic (no cross-counter snapshot guarantee).
    #[must_use]
    pub fn counts(&self) -> ProbeCounts {
        ProbeCounts {
            lost_records: self.lost_records.read(),
            lost_blocks: self.lost_blocks.read(),
            unobserved: self.unobserved.read(),
            unbound: self.unbound.read(),
        }
    }
}

/// The three probe halves returned by [`pair`]: install the `deck`
/// via [`RtRuntime::set_probe_deck`](super::RtRuntime::set_probe_deck)
/// before handing the runtime to the RT side.
pub struct ProbeParts {
    /// Control plane.
    pub broker: ProbeBroker,
    /// RT half.
    pub deck: ProbeDeck,
    /// Receive/drain side.
    pub collector: ProbeCollector,
}

/// Probe-series clock domains live in a dedicated high namespace —
/// far above media-core's well-known ids (`0`=UNKNOWN, `1`=
/// `SAMPLE_CLOCK`, `2`=MONOTONIC) — so a probe cursor domain can never
/// collide with a device/backend domain id (Issue #309).
const PROBE_DOMAIN_NAMESPACE: u64 = 0x5256_4650_0000_0000;

/// Non-RT mint counters — `pair()` is non-RT, so plain atomics are
/// fine; they only need process-wide uniqueness, not RT speed.
static NEXT_PROBE_DOMAIN: AtomicU64 = AtomicU64::new(0);
static NEXT_RUNTIME_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Builds the broker/deck/collector triple and preallocates the
/// entire capture pool — all allocation happens here.
///
/// **RT contract:** non-RT only — allocates.
#[must_use]
pub fn pair(config: &ProbeConfig) -> ProbeParts {
    let (cmd_prod, cmd_cons) = ruviform_rt::channel(config.command_capacity.max(1));
    let (free_prod, free_cons) =
        ruviform_rt::channel::<Box<CaptureChunk>>(config.pool_chunks.max(1));
    let (chunks_prod, chunks_cons) =
        ruviform_rt::channel::<Box<CaptureChunk>>(config.pool_chunks.max(1));
    let (tele_prod, tele_cons) =
        ruviform_rt::channel::<TelemetryRecord>(config.telemetry_capacity.max(1));
    let (lost_records_w, lost_records_r) = counter();
    let (lost_blocks_w, lost_blocks_r) = counter();
    let (unobserved_w, unobserved_r) = counter();
    let (unbound_w, unbound_r) = counter();
    // Mint the cursor-series identity (Issue #309): one domain per
    // pair, epoch starts at 0, runtime instance is a non-RT counter.
    // The deck stays `UNKNOWN`-marked until the consumer wires the
    // mark via `ProbeBroker::wire_clock_mark`.
    let domain = ClockDomainId::new(
        PROBE_DOMAIN_NAMESPACE.wrapping_add(NEXT_PROBE_DOMAIN.fetch_add(1, Ordering::Relaxed)),
    );
    let runtime_instance = NEXT_RUNTIME_INSTANCE.fetch_add(1, Ordering::Relaxed);
    let mut free_prod = free_prod;
    for _ in 0..config.pool_chunks {
        let _ = free_prod.try_push(Box::new(CaptureChunk {
            meta: None,
            frames: 0,
            channels: 0,
            data: vec![0.0_f32; config.chunk_channels * config.chunk_frames].into_boxed_slice(),
        }));
    }
    ProbeParts {
        broker: ProbeBroker {
            commands: cmd_prod,
            probes: BTreeMap::new(),
            next: 0,
            max_probes: config.max_probes,
            pool_budget: (config.pool_chunks as u64) * (config.chunk_frames as u64),
            time: ProbeTimeContext {
                domain,
                epoch: ClockEpochId::new(0),
                runtime_instance,
                sample_rate: 0,
                provenance: TimestampProvenance::Derived,
                anchors: Vec::new(),
                generation_window: None,
            },
        },
        deck: ProbeDeck {
            commands: cmd_cons,
            free: free_cons,
            chunks_out: chunks_prod,
            telemetry: tele_prod,
            bindings: Vec::with_capacity(config.max_probes),
            staged: Vec::with_capacity(config.max_probes),
            staging: None,
            current_mark: ProbeClockMark::UNKNOWN,
            max_probes: config.max_probes,
            chunk_frames: config.chunk_frames,
            lost_records: lost_records_w,
            lost_blocks: lost_blocks_w,
            unobserved: unobserved_w,
            unbound: unbound_w,
        },
        collector: ProbeCollector {
            chunks_out: chunks_cons,
            telemetry: tele_cons,
            free: free_prod,
            lost_records: lost_records_r,
            lost_blocks: lost_blocks_r,
            unobserved: unobserved_r,
            unbound: unbound_r,
        },
    }
}
