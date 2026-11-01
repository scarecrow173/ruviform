//! Stable-slot RT runtime and its non-RT owner.
//!
//! [`GraphRuntime`] (non-RT) compiles, installs/removes processors
//! through a generation-tagged SPSC command channel, and publishes
//! immutable [`PreparedPlan`]s through [`ruviform_rt::slot`].
//! [`RtRuntime`] (single RT owner) reads the last-good plan through a
//! [`SnapshotGuard`] — whose drop already retires the `Arc` through
//! the reader's own [`Retirer`] — applies install/remove commands on
//! generation change, delivers generation-matching events, runs the
//! schedule, and never runs a destructor: displaced processors go to
//! the processor [`Retirer`] drained by [`GraphRuntime::collect`].

use core::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_graph::{NodeId, PortId};
use ruviform_rt::{
    Consumer, DropCollector, Producer, Retirer, RtContext, SnapshotReader, SnapshotWriter,
    retire_channel,
};

use super::buffer::{DelayLine, Slot};
use super::compile::{
    CompiledPlan, CompilerConfig, Install, InstalledInfo, PreparedPlan, ScheduledNode,
    SlotAllocator,
};
use super::io::NodeIo;

/// RT-suitable per-node processor — the object-safe counterpart of
/// `ruviform_rt::RtProcessor` for borrowed multi-port I/O (its `type
/// Io` associated type cannot express `&mut NodeIo<'_>` on `dyn`).
/// The `&RtContext` capability gating contract is preserved: only a
/// driver inside an RT contract section can call `process`/`event`.
pub trait GraphProcessor: Send {
    /// Processing latency in frames, consulted by the compiler's PDC
    /// pass. Non-RT only (called at compile, cached in the registry).
    fn latency(&self) -> u32 {
        0
    }

    /// Called once after install, before the first `process`.
    /// RT-suitable — must be bounded, non-allocating.
    fn activate(&mut self, _ctx: &RtContext) {}

    /// Generation-checked event delivery. RT-suitable.
    fn event(&mut self, _ev: &NodeEvent, _ctx: &RtContext) {}

    /// One processing block. RT contract: no allocation, no heavy
    /// clone, no blocking lock, no blocking, no OS I/O.
    fn process(&mut self, io: &mut NodeIo<'_>, ctx: &RtContext);

    /// Called once before removal. RT-suitable — bounded.
    fn deactivate(&mut self, _ctx: &RtContext) {}
}

impl<F> GraphProcessor for F
where
    F: FnMut(&mut NodeIo<'_>, &RtContext) + Send,
{
    fn process(&mut self, io: &mut NodeIo<'_>, ctx: &RtContext) {
        self(io, ctx);
    }
}

/// Generation-tagged processor registry command — the only path by
/// which instances move to or leave the RT side.
#[non_exhaustive]
pub enum RuntimeCommand {
    /// Install `processor` at `slot` for `generation` (replacing any
    /// occupant, which is deactivated and retired).
    Install {
        /// Target generation.
        generation: u64,
        /// Registry slot.
        slot: usize,
        /// Instance (deactivated+retired on replace).
        processor: Box<dyn GraphProcessor>,
    },
    /// Remove the occupant of `slot` for `generation`.
    Remove {
        /// Target generation.
        generation: u64,
        /// Registry slot.
        slot: usize,
    },
}

/// Control event routed to a node's processor — only delivered when
/// its `generation` matches the runtime's active plan generation.
pub struct NodeEvent {
    /// Plan generation this event targets.
    pub generation: u64,
    /// Destination node.
    pub node: NodeId,
    /// Caller-defined opcode (parameter kind etc.).
    pub opcode: u32,
    /// Caller-defined payload.
    pub value: f32,
}

/// Fault kinds surfaced to non-RT as shared-status bit flags.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum RuntimeFault {
    /// A consumer read a producer slot not written this callback.
    UninitializedRead = 1 << 0,
    /// A scheduled node's registry slot held no processor.
    MissingProcessor = 1 << 1,
    /// A stale-generation command/event was discarded.
    StaleDiscarded = 1 << 2,
    /// The processor retire channel overflowed (leak-on-full).
    RetireOverflow = 1 << 3,
    /// A retired plan `Arc` overflowed the snapshot channel
    /// (leak-on-full, observed via `writer.retire_overflows()`).
    PlanRetireOverflow = 1 << 4,
}

/// Counters shared with the non-RT side — atomics only, safe to load
/// from either side (non-blocking by construction).
#[derive(Debug, Default)]
pub struct SharedStatus {
    /// Events dropped for generation mismatch.
    pub stale_events: AtomicU64,
    /// Commands dropped for generation mismatch.
    pub stale_commands: AtomicU64,
    /// Uninitialized-read rejections.
    pub uninit_reads: AtomicU64,
    /// Processor retire overflows (leak-on-full).
    pub retire_overflows: AtomicU64,
    /// Plan `Arc` retire overflows — mirrored from
    /// `SnapshotWriter::retire_overflows` by [`GraphRuntime::collect`].
    pub plan_retire_overflows: AtomicU64,
    /// `RuntimeFault` bit set.
    pub fault_flags: AtomicU64,
    /// Active plan generation as observed by the RT side.
    pub active_generation: AtomicU64,
}

/// Point-in-time readout of [`SharedStatus`].
#[derive(Copy, Clone, Debug, Default)]
pub struct SharedStatusSnapshot {
    /// See [`SharedStatus::stale_events`].
    pub stale_events: u64,
    /// See [`SharedStatus::stale_commands`].
    pub stale_commands: u64,
    /// See [`SharedStatus::uninit_reads`].
    pub uninit_reads: u64,
    /// See [`SharedStatus::retire_overflows`].
    pub retire_overflows: u64,
    /// See [`SharedStatus::plan_retire_overflows`].
    pub plan_retire_overflows: u64,
    /// See [`SharedStatus::fault_flags`].
    pub fault_flags: u64,
    /// See [`SharedStatus::active_generation`].
    pub active_generation: u64,
}

impl SharedStatus {
    /// Records an uninitialized-read rejection (`UninitializedRead`
    /// flag + counter).
    pub(crate) fn record_uninit_read(&self) {
        self.uninit_reads.fetch_add(1, Ordering::Relaxed);
        self.fault_flags
            .fetch_or(RuntimeFault::UninitializedRead as u64, Ordering::Relaxed);
    }

    fn flag(&self, fault: RuntimeFault) {
        self.fault_flags.fetch_or(fault as u64, Ordering::Relaxed);
    }

    fn record_stale_commands(&self) {
        self.stale_commands.fetch_add(1, Ordering::Relaxed);
        self.flag(RuntimeFault::StaleDiscarded);
    }

    fn record_stale_events(&self) {
        self.stale_events.fetch_add(1, Ordering::Relaxed);
        self.flag(RuntimeFault::StaleDiscarded);
    }

    fn record_missing_processor(&self) {
        self.flag(RuntimeFault::MissingProcessor);
    }

    fn record_proc_retire_overflow(&self) {
        self.retire_overflows.fetch_add(1, Ordering::Relaxed);
        self.flag(RuntimeFault::RetireOverflow);
    }

    /// Consistent-enough point-in-time copy (relaxed loads; counters
    /// are monotonic so the snapshot is never inconsistent).
    #[must_use]
    pub fn snapshot(&self) -> SharedStatusSnapshot {
        SharedStatusSnapshot {
            stale_events: self.stale_events.load(Ordering::Relaxed),
            stale_commands: self.stale_commands.load(Ordering::Relaxed),
            uninit_reads: self.uninit_reads.load(Ordering::Relaxed),
            retire_overflows: self.retire_overflows.load(Ordering::Relaxed),
            plan_retire_overflows: self.plan_retire_overflows.load(Ordering::Relaxed),
            fault_flags: self.fault_flags.load(Ordering::Relaxed),
            active_generation: self.active_generation.load(Ordering::Relaxed),
        }
    }
}

/// Publish/activate failure — nothing was published and no registry
/// state changed (last-good rollback).
#[non_exhaustive]
#[derive(Debug)]
pub enum ActivationError {
    /// The command channel could not accept every install/remove.
    CommandChannelFull,
    /// The event channel is full.
    EventChannelFull,
}

impl core::fmt::Display for ActivationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CommandChannelFull => f.write_str("command channel full — nothing published"),
            Self::EventChannelFull => f.write_str("event channel full"),
        }
    }
}

impl core::error::Error for ActivationError {}

/// Runtime-side process failure.
#[non_exhaustive]
#[derive(Debug)]
pub enum ProcessError {
    /// `frames` exceeded `CompilerConfig::max_frames`.
    FramesExceedMax {
        /// Presented.
        got: usize,
        /// Bound.
        max: usize,
    },
}

impl core::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FramesExceedMax { got, max } => {
                write!(f, "callback frames {got} exceed prepared max {max}")
            }
        }
    }
}

impl core::error::Error for ProcessError {}

/// Interior mutable runtime state — `RefCell`-wrapped so the
/// [`SnapshotGuard`]'s borrow of `reader` stays disjoint from
/// everything the callback mutates. Single-threaded by construction
/// (the whole [`RtRuntime`] is `!Sync`).
struct RtState {
    /// Install/remove command inbound side.
    commands: Consumer<RuntimeCommand>,
    /// Event inbound side.
    events: Consumer<NodeEvent>,
    /// Stable processor registry — `processors[pslot]`.
    processors: Vec<Option<Box<dyn GraphProcessor>>>,
    /// Deferred processor destruction.
    procs_retirer: Retirer<Box<dyn GraphProcessor>>,
    /// Generation currently reflected by `processors`.
    active_generation: u64,
    /// Callback counter — the slot write-stamp clock.
    seq: u64,
}

impl RtState {
    /// Deactivates then retires a displaced processor — the `Box`
    /// destructor runs on the non-RT side via the retirer, never here.
    fn retire_processor(&mut self, status: &SharedStatus, processor: Box<dyn GraphProcessor>) {
        let before = self.procs_retirer.overflows();
        self.procs_retirer.retire(processor);
        if self.procs_retirer.overflows() > before {
            status.record_proc_retire_overflow();
        }
    }

    /// Applies install/remove commands for `plan.generation` and
    /// reconfigures slot/delay *metadata* (channel counts and delay
    /// amounts only — re-shaping never allocates).
    fn activate_generation(
        &mut self,
        plan: &PreparedPlan,
        arena: &[RefCell<Slot>],
        delays: &[RefCell<DelayLine>],
        status: &SharedStatus,
        ctx: &RtContext,
    ) {
        while let Ok(cmd) = self.commands.try_pop() {
            let applies = match &cmd {
                RuntimeCommand::Install { generation, .. }
                | RuntimeCommand::Remove { generation, .. } => *generation == plan.generation,
            };
            if !applies {
                status.record_stale_commands();
                continue;
            }
            match cmd {
                RuntimeCommand::Install {
                    slot, processor, ..
                } => {
                    if slot < self.processors.len() {
                        if let Some(mut old) = self.processors[slot].replace(processor) {
                            old.deactivate(ctx);
                            self.retire_processor(status, old);
                        }
                        if let Some(p) = self.processors[slot].as_mut() {
                            p.activate(ctx);
                        }
                    }
                }
                RuntimeCommand::Remove { slot, .. } => {
                    if let Some(mut old) = self.processors.get_mut(slot).and_then(Option::take) {
                        old.deactivate(ctx);
                        self.retire_processor(status, old);
                    }
                }
            }
        }
        for (i, spec) in plan.slots.iter().enumerate() {
            if let Some(cell) = arena.get(i) {
                cell.borrow_mut().configure(spec.channels);
            }
        }
        for (i, spec) in plan.delays.iter().enumerate() {
            if let Some(cell) = delays.get(i) {
                cell.borrow_mut().configure(
                    spec.channels,
                    usize::try_from(spec.delay).unwrap_or(usize::MAX),
                );
            }
        }
        self.active_generation = plan.generation;
        status
            .active_generation
            .store(plan.generation, Ordering::Relaxed);
    }

    /// Deactivates and retires every installed processor — shutdown
    /// path (`RtRuntime::drop`); destructor work stays deferred.
    fn shutdown(&mut self, status: &SharedStatus) {
        let ctx = RtContext::mint();
        for i in 0..self.processors.len() {
            if let Some(mut processor) = self.processors[i].take() {
                processor.deactivate(&ctx);
                self.retire_processor(status, processor);
            }
        }
    }
}

/// Non-RT owner: compiles, pushes install/remove commands, publishes
/// plans, and drains every retire channel.
///
/// **RT contract:** every method is non-RT only.
pub struct GraphRuntime {
    writer: SnapshotWriter<PreparedPlan>,
    commands: Producer<RuntimeCommand>,
    events: Producer<NodeEvent>,
    procs_retired: DropCollector<Box<dyn GraphProcessor>>,
    registry: BTreeMap<NodeId, InstalledInfo>,
    alloc: SlotAllocator,
    status: Arc<SharedStatus>,
    generation: u64,
}

/// The single RT owner — moves to the callback thread once. `Send`
/// (hand-off) and `!Sync` (interior `RefCell`), so exactly one
/// thread can ever own or invoke a processor instance.
///
/// **RT contract:** [`process`](Self::process),
/// [`input_writer`](Self::input_writer),
/// [`output_reader`](Self::output_reader) are RT-suitable; drop is
/// bounded retire work — destructors still run off the RT side.
pub struct RtRuntime {
    reader: SnapshotReader<PreparedPlan>,
    state: RefCell<RtState>,
    /// Buffer slot arena — direct field so `input_writer` can return
    /// a `RefMut` tied to `&self`, and so `process` can borrow it
    /// disjointly from `state`.
    arena: Vec<RefCell<Slot>>,
    /// PDC delay-line arena.
    delays: Vec<RefCell<DelayLine>>,
    status: Arc<SharedStatus>,
    max_frames: usize,
}

impl GraphRuntime {
    /// Creates the non-RT/RT pair. The entire arena (`max_slots`
    /// slots × `max_channels × max_frames`, `max_delay_lines` rings)
    /// and the `max_processors` registry are allocated here, so the
    /// RT side performs zero allocation for the life of the pair.
    ///
    /// **RT contract:** non-RT only — allocates.
    #[must_use]
    pub fn new(
        config: &CompilerConfig,
        command_capacity: usize,
        event_capacity: usize,
    ) -> (Self, RtRuntime) {
        let empty = PreparedPlan {
            generation: 0,
            max_frames: config.max_frames,
            slots: Vec::new(),
            delays: Vec::new(),
            schedule: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            total_latency: 0,
        };
        let (writer, reader) = ruviform_rt::slot(Arc::new(empty), 4);
        let (commands, cmd_consumer) = ruviform_rt::channel(command_capacity.max(1));
        let (events, ev_consumer) = ruviform_rt::channel(event_capacity.max(1));
        let (retirer, collector) =
            retire_channel::<Box<dyn GraphProcessor>>(command_capacity.max(1).max(4));
        let status = Arc::new(SharedStatus::default());
        (
            Self {
                writer,
                commands,
                events,
                procs_retired: collector,
                registry: BTreeMap::new(),
                alloc: SlotAllocator {
                    free: BTreeSet::new(),
                    next: 0,
                    max: config.max_processors,
                },
                status: Arc::clone(&status),
                generation: 0,
            },
            RtRuntime {
                reader,
                state: RefCell::new(RtState {
                    commands: cmd_consumer,
                    events: ev_consumer,
                    processors: (0..config.max_processors).map(|_| None).collect(),
                    procs_retirer: retirer,
                    active_generation: 0,
                    seq: 0,
                }),
                arena: (0..config.max_slots)
                    .map(|_| {
                        RefCell::new(Slot::new(
                            config.max_channels,
                            config.max_frames,
                            config.alignment,
                        ))
                    })
                    .collect(),
                delays: (0..config.max_delay_lines)
                    .map(|_| {
                        RefCell::new(DelayLine::new(
                            config.max_channels,
                            usize::try_from(config.max_delay_frames).unwrap_or(usize::MAX)
                                + config.max_frames,
                            config.alignment,
                        ))
                    })
                    .collect(),
                status,
                max_frames: config.max_frames,
            },
        )
    }

    /// Allocator state the compiler needs for pslot assignment.
    #[must_use]
    pub fn allocator(&self) -> &SlotAllocator {
        &self.alloc
    }

    /// Registry mirror for `Compiler::compile`.
    #[must_use]
    pub fn registry(&self) -> &BTreeMap<NodeId, InstalledInfo> {
        &self.registry
    }

    /// Generation of the last successfully published plan (the next
    /// compile should stamp `generation() + 1`).
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Shared fault/counter status.
    #[must_use]
    pub fn status(&self) -> &SharedStatus {
        &self.status
    }

    /// Queues install/remove commands then publishes the plan.
    ///
    /// Every command carries `generation`; if any `try_push` fails,
    /// nothing is published, the registry and allocator stay
    /// untouched, and the already-queued commands are discarded as
    /// stale by the RT side on the next activation (counted on
    /// `stale_commands`) — last-good rollback by construction. The
    /// unconsumed remainder of `compiled` (including processor
    /// boxes) is dropped right here on the non-RT side.
    ///
    /// # Errors
    /// [`ActivationError::CommandChannelFull`] when the command
    /// channel cannot hold the whole delta.
    pub fn publish(&mut self, compiled: CompiledPlan) -> Result<(), ActivationError> {
        let generation = compiled.plan.generation;
        let mut registry_new: Vec<(NodeId, usize, u32)> =
            Vec::with_capacity(compiled.installs.len().min(1024));
        for install in compiled.installs {
            let Install {
                pslot,
                node,
                processor,
                latency,
            } = install;
            registry_new.push((node, pslot, latency));
            self.commands
                .try_push(RuntimeCommand::Install {
                    generation,
                    slot: pslot,
                    processor,
                })
                .map_err(|_| ActivationError::CommandChannelFull)?;
        }
        for &(_, pslot) in &compiled.removes {
            self.commands
                .try_push(RuntimeCommand::Remove {
                    generation,
                    slot: pslot,
                })
                .map_err(|_| ActivationError::CommandChannelFull)?;
        }
        // All commands queued — commit bookkeeping, then publish.
        for (node, _) in &compiled.removes {
            self.registry.remove(node);
        }
        for (node, pslot, latency) in registry_new {
            self.registry.insert(node, InstalledInfo { pslot, latency });
        }
        self.alloc = compiled.alloc_after;
        self.writer.publish(Arc::new(compiled.plan));
        self.generation = generation;
        Ok(())
    }

    /// Sends a control event. `ev.generation` must equal the target
    /// plan generation — stale events are discarded and counted by
    /// the RT side.
    ///
    /// # Errors
    /// [`ActivationError::EventChannelFull`] on a full channel.
    pub fn send_event(&mut self, ev: NodeEvent) -> Result<(), ActivationError> {
        self.events
            .try_push(ev)
            .map_err(|_| ActivationError::EventChannelFull)
    }

    /// Drains every retirement path — displaced and guard-released
    /// plan `Arc`s (snapshot collector) plus retired processors.
    /// Destructors run on this thread. Returns total drops. Also
    /// mirrors the snapshot writer's cumulative retire-overflow
    /// count into [`SharedStatus`] for the fault flags.
    ///
    /// **RT contract:** non-RT only — runs destructors.
    pub fn collect(&mut self) -> usize {
        let overflows = self.writer.retire_overflows() as u64;
        self.status
            .plan_retire_overflows
            .store(overflows, Ordering::Relaxed);
        if overflows > 0 {
            self.status.flag(RuntimeFault::PlanRetireOverflow);
        }
        self.writer.collect() + self.procs_retired.drain()
    }

    /// Retire overflows reported by the snapshot writer side.
    #[must_use]
    pub fn writer_retire_overflows(&self) -> usize {
        self.writer.retire_overflows()
    }
}

impl RtRuntime {
    /// Boundary input writer — stamps the slot for the *next*
    /// `process` call (`seq + 1`) and declares `frames` valid so the
    /// caller-fed slot always satisfies the uninitialized-read audit
    /// of the callback it is written for. The caller then fills
    /// `slot.data.channel_mut(c)[..frames]` per channel.
    ///
    /// `port` must be one of `PreparedPlan::inputs`.
    ///
    /// **RT contract:** RT-suitable — one guard read + bounded
    /// `RefCell` borrows, no allocation.
    pub fn input_writer(&self, port: PortId, frames: usize) -> Option<RefMut<'_, Slot>> {
        let idx = self.boundary_slot(port, true)?;
        let seq = self.state.borrow().seq;
        let mut cell = self.arena.get(idx)?.borrow_mut();
        cell.mark_written(
            seq.wrapping_add(1),
            u32::try_from(frames).unwrap_or(u32::MAX),
        );
        Some(cell)
    }

    /// Boundary output reader — the caller drains the slot after
    /// `process`. `port` must be one of `PreparedPlan::outputs`.
    ///
    /// **RT contract:** RT-suitable — one guard read + bounded
    /// `RefCell` borrows, no allocation.
    pub fn output_reader(&self, port: PortId) -> Option<Ref<'_, Slot>> {
        let idx = self.boundary_slot(port, false)?;
        Some(self.arena.get(idx)?.borrow())
    }

    /// Resolves the active plan's boundary table for `port`. The
    /// guard's drop retires the `Arc` through the reader's own
    /// `Retirer` — nothing is dropped on RT.
    fn boundary_slot(&self, port: PortId, input: bool) -> Option<usize> {
        let plan = self.reader.read();
        if input {
            plan.input_slot(port)
        } else {
            plan.output_slot(port)
        }
    }

    /// Drives the schedule for `frames`.
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking
    /// lock, no blocking, no OS I/O, no destructors (displaced
    /// objects take `Retirer` `try_push` paths; the plan `Arc` leaves
    /// through the guard's `Drop`).
    ///
    /// `frames == 0` is a defined no-op: no processor runs, nothing
    /// is stamped. `frames > max_frames` is rejected.
    ///
    /// # Errors
    /// [`ProcessError::FramesExceedMax`] on an over-budget callback.
    pub fn process(&mut self, frames: usize, ctx: &RtContext) -> Result<(), ProcessError> {
        if frames > self.max_frames {
            return Err(ProcessError::FramesExceedMax {
                got: frames,
                max: self.max_frames,
            });
        }
        if frames == 0 {
            return Ok(());
        }

        let plan = self.reader.read();
        let mut st = self.state.borrow_mut();
        st.seq = st.seq.wrapping_add(1);

        if plan.generation != st.active_generation {
            st.activate_generation(&plan, &self.arena, &self.delays, &self.status, ctx);
        }

        // Deliver generation-checked events.
        while let Ok(ev) = st.events.try_pop() {
            if ev.generation == st.active_generation {
                let pslot = plan
                    .schedule
                    .iter()
                    .find(|s| s.node == ev.node)
                    .and_then(|s| s.pslot);
                if let Some(processor) = pslot.and_then(|p| st.processors.get_mut(p)?.as_mut()) {
                    processor.event(&ev, ctx);
                }
            } else {
                self.status.record_stale_events();
            }
        }

        for node in &plan.schedule {
            Self::materialize(
                &self.arena,
                &self.delays,
                st.seq,
                &self.status,
                node,
                frames,
            );
            if let Some(pslot) = node.pslot {
                let seq = st.seq;
                if let Some(processor) = st.processors.get_mut(pslot).and_then(Option::as_mut) {
                    let mut io = NodeIo {
                        arena: &self.arena,
                        node,
                        frames,
                        seq,
                        status: &self.status,
                    };
                    processor.process(&mut io, ctx);
                    for &slot_idx in &node.outputs {
                        if let Some(cell) = self.arena.get(slot_idx) {
                            cell.borrow_mut()
                                .mark_written(seq, u32::try_from(frames).unwrap_or(u32::MAX));
                        }
                    }
                } else {
                    self.status.record_missing_processor();
                }
            }
        }
        Ok(())
        // `plan` (SnapshotGuard) drops here → retires the `Arc`.
    }

    /// Materializes every input binding of `node`: channel map and/or
    /// PDC delay per edge, sample-sum fan-in into the scratch slot.
    /// Unconnected or direct inputs are left alone (the processor
    /// reads the producer slot in place or sees `None`). Associated
    /// fn so its field borrows stay disjoint from the plan guard.
    fn materialize(
        arena: &[RefCell<Slot>],
        delays: &[RefCell<DelayLine>],
        seq: u64,
        status: &SharedStatus,
        node: &ScheduledNode,
        frames: usize,
    ) {
        for binding in &node.inputs {
            let Some(scratch_idx) = binding.scratch else {
                continue;
            };
            let Some(scratch_cell) = arena.get(scratch_idx) else {
                continue;
            };
            let mut scratch = scratch_cell.borrow_mut();
            scratch.fill_zero(seq, frames);
            for edge in &binding.edges {
                let Some(src_cell) = arena.get(edge.slot) else {
                    continue;
                };
                let src = src_cell.borrow();
                if src.stamp() != seq {
                    // Producer has not written this callback —
                    // uninitialized read is rejected (fault + skip),
                    // never served stale data.
                    status.record_uninit_read();
                    continue;
                }
                for ch in 0..usize::from(scratch.channels) {
                    let Some(dst_plane) = scratch.data.channel_mut(ch) else {
                        continue;
                    };
                    if let Some(map) = &edge.map {
                        for &(src_ch, dst_ch) in map {
                            if usize::from(dst_ch) != ch {
                                continue;
                            }
                            let Some(src_plane) = src.data.channel(usize::from(src_ch)) else {
                                continue;
                            };
                            Self::merge_edge(
                                delays,
                                edge.delay_line,
                                usize::from(dst_ch),
                                src_plane,
                                dst_plane,
                                frames,
                            );
                        }
                    } else {
                        let Some(src_plane) = src.data.channel(ch) else {
                            continue;
                        };
                        Self::merge_edge(delays, edge.delay_line, ch, src_plane, dst_plane, frames);
                    }
                }
            }
            scratch.mark_written(seq, u32::try_from(frames).unwrap_or(u32::MAX));
        }
    }

    /// One edge's contribution into `dst_plane`: through the PDC
    /// delay line when bound, else a plain sample-sum.
    fn merge_edge(
        delays: &[RefCell<DelayLine>],
        delay_line: Option<usize>,
        ch: usize,
        src_plane: &[f32],
        dst_plane: &mut [f32],
        frames: usize,
    ) {
        match delay_line {
            Some(dl) => {
                if let Some(cell) = delays.get(dl) {
                    cell.borrow_mut().process_add(
                        ch,
                        &src_plane[..frames],
                        &mut dst_plane[..frames],
                        frames,
                    );
                }
            }
            None => {
                for (d, s) in dst_plane[..frames].iter_mut().zip(&src_plane[..frames]) {
                    *d += s;
                }
            }
        }
    }
}

impl Drop for RtRuntime {
    /// Deactivates and retires every installed processor so `Box`
    /// destructors never run here — the collector drains them on the
    /// non-RT side.
    fn drop(&mut self) {
        self.state.borrow_mut().shutdown(&self.status);
        // `arena`/`delays` are plain memory — bounded dealloc is a
        // teardown concern, not a callback destructor.
    }
}
