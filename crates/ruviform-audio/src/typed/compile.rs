//! Non-RT compiler: `ValidatedGraph` → immutable [`PreparedPlan`].
//!
//! The compiler never mutates shared state and never publishes: it
//! produces a self-contained [`CompiledPlan`] — deterministic
//! schedule, bounded slot layout, fan-in merge bindings, merge-point
//! PDC delay lines, and the processor install/remove delta keyed by
//! [`NodeId`]. Publishing (and rolling back) is the caller's
//! responsibility via [`GraphRuntime`](crate::typed::GraphRuntime).

use std::collections::{BTreeMap, BTreeSet};

use ruviform_graph::{
    ConnectionId, Direction, EdgeDelay, GraphSpec, NodeId, NodeKind, NodeSpec, PortFormat, PortId,
    PortSpec, ValidatedGraph,
};

use super::buffer::SlotSpec;
use super::runtime::GraphProcessor;

/// Compile-time bounds and buffer budget. The runtime arena is built
/// from the same values, so a compiled plan can never exceed what the
/// RT side can execute without allocation.
#[derive(Clone, Debug)]
pub struct CompilerConfig {
    /// Maximum frames any callback may present (`frames > max` is a
    /// `ProcessError::FramesExceedMax` at runtime).
    pub max_frames: usize,
    /// Byte alignment of every slot/delay allocation (power of two,
    /// `>= 4`; default 64).
    pub alignment: usize,
    /// Maximum buffer slots the arena holds (producer + scratch).
    pub max_slots: usize,
    /// Maximum channel capacity per slot/ring allocation.
    pub max_channels: usize,
    /// Maximum PDC delay lines (one per compensated edge).
    pub max_delay_lines: usize,
    /// Maximum total graph latency in frames — overflow is
    /// `CompileError::DelayOverflow`.
    pub max_delay_frames: u32,
    /// Maximum processor registry slots.
    pub max_processors: usize,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            max_frames: 512,
            alignment: super::buffer::DEFAULT_ALIGNMENT,
            max_slots: 64,
            max_channels: 8,
            max_delay_lines: 32,
            max_delay_frames: 1 << 20,
            max_processors: 64,
        }
    }
}

/// Bookkeeping for one installed processor — the non-RT mirror of
/// the RT registry entry (`latency` is captured so recompiles can
/// compute PDC without touching the instance).
#[derive(Copy, Clone, Debug)]
pub struct InstalledInfo {
    /// Processor registry slot.
    pub pslot: usize,
    /// Last reported processing latency in frames.
    pub latency: u32,
}

/// Deterministic processor-slot allocator mirrored on the non-RT
/// side — free slots first (`BTreeSet` order), then `next` upward,
/// bounded by `max`.
#[derive(Clone, Debug, Default)]
pub struct SlotAllocator {
    /// Reclaimed slots (`BTreeSet` for deterministic reuse order).
    pub free: BTreeSet<usize>,
    /// First never-issued slot index.
    pub next: usize,
    /// Hard bound (`CompilerConfig::max_processors`).
    pub max: usize,
}

impl SlotAllocator {
    /// Peeks the slot a new install would take without mutating.
    fn peek(&self) -> Option<usize> {
        self.free
            .first()
            .copied()
            .or_else(|| (self.next < self.max).then_some(self.next))
    }

    /// Commits one allocation; `None` when at bound.
    fn take(&mut self) -> Option<usize> {
        let p = self.peek()?;
        if self.free.remove(&p) && p >= self.next {
            self.next = p + 1;
        } else if p == self.next {
            self.next += 1;
        }
        Some(p)
    }

    /// Returns a slot to the free set.
    pub fn release(&mut self, pslot: usize) {
        if pslot < self.next {
            self.free.insert(pslot);
        }
    }
}

/// What the factory decided for one node during a compile pass.
#[non_exhaustive]
pub enum PrepareOutcome {
    /// Keep the already-installed instance (DSP state continues).
    Reuse,
    /// No processor — a graph boundary node (caller-fed `Source`
    /// outputs / tap-only `Sink` inputs).
    Boundary,
    /// A new processor instance to install.
    New(Box<dyn GraphProcessor>),
}

/// Opaque factory failure; the compiler wraps it in
/// [`CompileError::Factory`] with the node context.
#[derive(Debug)]
pub struct FactoryError(pub String);

impl core::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl core::error::Error for FactoryError {}

impl From<&str> for FactoryError {
    fn from(reason: &str) -> Self {
        Self(reason.to_owned())
    }
}

/// Non-RT processor provider. Called once per node per compile;
/// receives the existing registry entry when the same [`NodeId`] was
/// previously installed.
pub trait NodeFactory {
    /// Returns the outcome for `node`. `existing` carries the
    /// installed slot/latency when this node is already live.
    fn prepare(
        &mut self,
        node: &NodeSpec,
        existing: Option<&InstalledInfo>,
    ) -> Result<PrepareOutcome, FactoryError>;
}

impl<F> NodeFactory for F
where
    F: FnMut(&NodeSpec, Option<&InstalledInfo>) -> Result<PrepareOutcome, FactoryError>,
{
    fn prepare(
        &mut self,
        node: &NodeSpec,
        existing: Option<&InstalledInfo>,
    ) -> Result<PrepareOutcome, FactoryError> {
        self(node, existing)
    }
}

/// One incoming edge of an input port binding.
#[derive(Clone, Debug)]
pub struct EdgeBinding {
    /// Producer slot index.
    pub slot: usize,
    /// Explicit channel map `(src_ch, dst_ch)` when declared.
    pub map: Option<Vec<(u16, u16)>>,
    /// PDC delay line index + amount (`None`/`0` = no compensation).
    pub delay_line: Option<usize>,
    /// Frames of PDC delay applied on this edge.
    pub delay_frames: u32,
}

/// All connections arriving at one input port.
#[derive(Clone, Debug)]
pub struct InputBinding {
    /// Bound input port.
    pub port: PortId,
    /// Incoming edges in connection declaration order.
    pub edges: Vec<EdgeBinding>,
    /// Merge/materialization scratch slot — set when the input needs
    /// summing (fan-in > 1), a channel map, or PDC delay.
    pub scratch: Option<usize>,
}

/// One node in schedule order.
#[derive(Clone, Debug)]
pub struct ScheduledNode {
    /// Stable node ID.
    pub node: NodeId,
    /// Processor registry slot (`None` = boundary node).
    pub pslot: Option<usize>,
    /// Audio input port bindings (declaration order).
    pub inputs: Vec<InputBinding>,
    /// Arena slot per audio output port (declaration order).
    pub outputs: Vec<usize>,
}

/// PDC ring spec — one per compensated edge.
#[derive(Clone, Debug)]
pub struct DelaySpec {
    /// Ring channel count (bound input's layout).
    pub channels: u16,
    /// Applied delay in frames.
    pub delay: u32,
}

/// The immutable, fully-prepared runtime plan — the payload carried
/// by the `Arc` snapshot. Wiring metadata only; it owns no processor
/// state, which is what makes `Arc` sharing sound.
#[derive(Clone, Debug)]
pub struct PreparedPlan {
    /// Plan generation stamped at compile; stale events/commands are
    /// rejected against the runtime's active generation.
    pub generation: u64,
    /// Callback frame bound this plan was prepared for.
    pub max_frames: usize,
    /// Arena shape — logical channels per slot.
    pub slots: Vec<SlotSpec>,
    /// PDC ring specs (prefix of the runtime's delay arena).
    pub delays: Vec<DelaySpec>,
    /// Schedule in `StableHandleMap` runtime index order.
    pub schedule: Vec<ScheduledNode>,
    /// Graph input boundary `(PortId, slot)` — caller-fed slots.
    pub inputs: Vec<(PortId, usize)>,
    /// Graph output boundary `(PortId, slot)` — drain after process.
    pub outputs: Vec<(PortId, usize)>,
    /// End-to-end accumulated latency in frames.
    pub total_latency: u32,
}

impl PreparedPlan {
    /// Boundary slot of graph-input `port` (for `input_writer`).
    #[must_use]
    pub fn input_slot(&self, port: PortId) -> Option<usize> {
        self.inputs
            .iter()
            .find(|(p, _)| *p == port)
            .map(|(_, s)| *s)
    }

    /// Boundary slot of graph-output `port` (for `output_reader`).
    #[must_use]
    pub fn output_slot(&self, port: PortId) -> Option<usize> {
        self.outputs
            .iter()
            .find(|(p, _)| *p == port)
            .map(|(_, s)| *s)
    }
}

/// One processor install — the RT side receives it as a
/// `RuntimeCommand::Install`.
pub struct Install {
    /// Registry slot to occupy.
    pub pslot: usize,
    /// Owning node.
    pub node: NodeId,
    /// Instance to install (`Send`; moves to the RT thread once).
    pub processor: Box<dyn GraphProcessor>,
    /// Captured latency for the non-RT registry mirror.
    pub latency: u32,
}

/// Everything the non-RT side needs to activate a compile result:
/// the plan plus the registry delta (installs/removes) and the
/// post-activation allocator state.
pub struct CompiledPlan {
    /// Immutable plan to publish.
    pub plan: PreparedPlan,
    /// Processor installs (fresh or replacing).
    pub installs: Vec<Install>,
    /// `(NodeId, pslot)` whose processors leave the registry.
    pub removes: Vec<(NodeId, usize)>,
    /// Allocator state to commit after a successful publish.
    pub alloc_after: SlotAllocator,
}

/// Defined compile failures — every rejection is a distinct variant,
/// never a silent partial plan.
#[non_exhaustive]
#[derive(Debug)]
pub enum CompileError {
    /// `EdgeDelay::Declared` carries no quantity in M1; acyclic
    /// declared-delay edges have no execution semantics in M2.
    UnsupportedEdgeDelay {
        /// Rejected connection.
        connection: ConnectionId,
    },
    /// `NodeKind::Processor` where the factory returned `Boundary`.
    MissingProcessor {
        /// Rejected node.
        node: NodeId,
    },
    /// A node's input and output slot sets intersect.
    BufferAlias {
        /// Offending node.
        node: NodeId,
    },
    /// Slot demand exceeds `CompilerConfig::max_slots`.
    SlotOverflow {
        /// Required.
        needed: usize,
        /// Bound.
        max: usize,
    },
    /// PDC line demand exceeds `CompilerConfig::max_delay_lines`.
    DelayLineOverflow {
        /// Required.
        needed: usize,
        /// Bound.
        max: usize,
    },
    /// Total latency exceeds `CompilerConfig::max_delay_frames`.
    DelayOverflow {
        /// Required.
        needed: u32,
        /// Bound.
        max: u32,
    },
    /// Registry demand exceeds `CompilerConfig::max_processors`.
    ProcessorOverflow {
        /// Bound.
        max: usize,
    },
    /// Factory failure, wrapped with node context.
    Factory {
        /// Node whose prepare failed.
        node: NodeId,
        /// Original failure.
        source: FactoryError,
    },
    /// Structural inconsistency the validator should have ruled out
    /// — surfaced rather than silently tolerated.
    Inconsistent {
        /// What was found.
        reason: &'static str,
    },
}

impl core::fmt::Display for CompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedEdgeDelay { connection } => {
                write!(
                    f,
                    "declared-delay edge {connection} has no M2 execution semantics"
                )
            }
            Self::MissingProcessor { node } => {
                write!(f, "processor node {node} produced no processor")
            }
            Self::BufferAlias { node } => {
                write!(f, "node {node} input/output buffer sets alias")
            }
            Self::SlotOverflow { needed, max } => {
                write!(f, "slot demand {needed} exceeds max {max}")
            }
            Self::DelayLineOverflow { needed, max } => {
                write!(f, "delay line demand {needed} exceeds max {max}")
            }
            Self::DelayOverflow { needed, max } => {
                write!(f, "graph latency {needed} exceeds max {max}")
            }
            Self::ProcessorOverflow { max } => {
                write!(f, "processor registry bound {max} exceeded")
            }
            Self::Factory { node, source } => {
                write!(f, "factory failed for node {node}: {source}")
            }
            Self::Inconsistent { reason } => write!(f, "inconsistent graph: {reason}"),
        }
    }
}

impl core::error::Error for CompileError {}

/// The non-RT compiler. All work is allocation-permitted preparation
/// — nothing here is callable from the RT side.
pub struct Compiler {
    /// Budget and bounds.
    pub config: CompilerConfig,
}

impl Compiler {
    /// Creates a compiler over `config`.
    #[must_use]
    pub const fn new(config: CompilerConfig) -> Self {
        Self { config }
    }

    /// Compiles `graph` into a [`CompiledPlan`].
    ///
    /// `registry`/`alloc` mirror the non-RT ownership bookkeeping —
    /// they are *read only*: `CompiledPlan::alloc_after` carries the
    /// state to commit on publish, so a failed compile consumes
    /// nothing.
    ///
    /// # Errors
    /// Every rejection is a distinct [`CompileError`] variant; no
    /// partial plan escapes.
    ///
    /// # Panics
    /// Never in isolation — internal consistency between the
    /// `ValidatedGraph`'s runtime order and its `GraphSpec` is an IR
    /// invariant; a violated invariant surfaces as
    /// [`CompileError::Inconsistent`] except the slot→port reverse
    /// lookup, which is derivable only when the map is complete.
    pub fn compile(
        &self,
        graph: &ValidatedGraph,
        factory: &mut dyn NodeFactory,
        registry: &BTreeMap<NodeId, InstalledInfo>,
        alloc: &SlotAllocator,
        generation: u64,
    ) -> Result<CompiledPlan, CompileError> {
        let spec = graph.spec();
        let order = graph.mapping().node_order();
        let mut scratch_alloc = alloc.clone();

        // Pass A — per-node factory + pslot assignment + own latency.
        let assigned = Self::assign_processors(spec, order, factory, registry, &mut scratch_alloc)?;

        // Removes — installed nodes absent from (or boundary in) the
        // new spec.
        let removes: Vec<(NodeId, usize)> = registry
            .iter()
            .filter(|(node, _)| !assigned.used_nodes.contains(node))
            .map(|(node, info)| (*node, info.pslot))
            .collect();
        for (_, pslot) in &removes {
            scratch_alloc.release(*pslot);
        }

        // Pass B — port → slot bindings + schedule shape.
        let mut bound = Self::bind_ports(spec, order, &assigned.pslots)?;

        // Pass C — accumulated latency + merge-point PDC.
        let (delay_specs, total_latency) = self.apply_pdc(
            spec,
            &bound.port_slot,
            &assigned.own_latency,
            &mut bound.slot_specs,
            &mut bound.schedule,
        )?;

        if bound.slot_specs.len() > self.config.max_slots {
            return Err(CompileError::SlotOverflow {
                needed: bound.slot_specs.len(),
                max: self.config.max_slots,
            });
        }
        if delay_specs.len() > self.config.max_delay_lines {
            return Err(CompileError::DelayLineOverflow {
                needed: delay_specs.len(),
                max: self.config.max_delay_lines,
            });
        }

        // Aliasing audit — every node's read set must be disjoint
        // from its write set (true by construction; verified anyway).
        audit_aliasing(&bound.schedule)?;

        // Boundary ports — processor-less Source outputs are graph
        // inputs; processor-less Sink inputs are graph outputs.
        let boundary = collect_boundaries(spec, &bound.schedule)?;

        Ok(CompiledPlan {
            plan: PreparedPlan {
                generation,
                max_frames: self.config.max_frames,
                slots: bound.slot_specs,
                delays: delay_specs,
                schedule: bound.schedule,
                inputs: boundary.inputs,
                outputs: boundary.outputs,
                total_latency,
            },
            installs: assigned.installs,
            removes,
            alloc_after: scratch_alloc,
        })
    }

    /// Pass A — walks the runtime order, asks the factory for every
    /// node, assigns registry slots, and records each node's own
    /// latency. Mutates only `scratch_alloc` (the scratch allocator —
    /// never committed until publish).
    fn assign_processors(
        spec: &GraphSpec,
        order: &[NodeId],
        factory: &mut dyn NodeFactory,
        registry: &BTreeMap<NodeId, InstalledInfo>,
        scratch_alloc: &mut SlotAllocator,
    ) -> Result<ProcessAssignments, CompileError> {
        let mut pslots: BTreeMap<NodeId, Option<usize>> = BTreeMap::new();
        let mut own_latency: BTreeMap<NodeId, u32> = BTreeMap::new();
        let mut installs: Vec<Install> = Vec::new();
        let mut used_nodes: BTreeSet<NodeId> = BTreeSet::new();
        for node_id in order {
            let node = spec.node(*node_id).ok_or(CompileError::Inconsistent {
                reason: "runtime order references a missing node",
            })?;
            let existing = registry.get(node_id);
            let outcome =
                factory
                    .prepare(node, existing)
                    .map_err(|source| CompileError::Factory {
                        node: *node_id,
                        source,
                    })?;
            let (pslot, latency, install) = match (outcome, existing) {
                (PrepareOutcome::Reuse, Some(info)) => (Some(info.pslot), info.latency, None),
                (PrepareOutcome::New(processor), _) => {
                    let pslot = existing.map_or_else(
                        || {
                            scratch_alloc.take().ok_or(CompileError::ProcessorOverflow {
                                max: scratch_alloc.max,
                            })
                        },
                        |info| Ok(info.pslot),
                    )?;
                    let latency = processor.latency();
                    (
                        Some(pslot),
                        latency,
                        Some(Install {
                            pslot,
                            node: *node_id,
                            processor,
                            latency,
                        }),
                    )
                }
                (PrepareOutcome::Boundary, existing) => {
                    if node.kind() == NodeKind::Processor {
                        return Err(CompileError::MissingProcessor { node: *node_id });
                    }
                    if let Some(info) = existing {
                        // A previously installed node going boundary
                        // vacates its slot.
                        scratch_alloc.release(info.pslot);
                    }
                    (None, 0, None)
                }
                (PrepareOutcome::Reuse, None) => {
                    return Err(CompileError::Inconsistent {
                        reason: "factory reused a node that was never installed",
                    });
                }
            };
            pslots.insert(*node_id, pslot);
            own_latency.insert(*node_id, latency);
            if pslot.is_some() {
                used_nodes.insert(*node_id);
            }
            if let Some(install) = install {
                installs.push(install);
            }
        }
        Ok(ProcessAssignments {
            pslots,
            own_latency,
            installs,
            used_nodes,
        })
    }

    /// Pass B — binds every audio output port to a fresh slot, binds
    /// every audio input to the producer slots feeding it
    /// (producer-before-consumer order is audited), and emits the
    /// `ScheduledNode` list in runtime order.
    fn bind_ports(
        spec: &GraphSpec,
        order: &[NodeId],
        pslots: &BTreeMap<NodeId, Option<usize>>,
    ) -> Result<PortBindings, CompileError> {
        let mut port_slot: BTreeMap<PortId, usize> = BTreeMap::new();
        let mut slot_specs: Vec<SlotSpec> = Vec::new();
        let mut schedule: Vec<ScheduledNode> = Vec::new();
        // node_id → position in `order` (producer-before-consumer audit).
        let position: BTreeMap<NodeId, usize> =
            order.iter().enumerate().map(|(i, n)| (*n, i)).collect();

        for node_id in order {
            let node = spec.node(*node_id).ok_or(CompileError::Inconsistent {
                reason: "runtime order references a missing node",
            })?;
            let mut outputs = Vec::new();
            let mut inputs = Vec::new();
            for port in node.ports() {
                let PortFormat::Audio(format) = port.format() else {
                    continue; // TimedEvent/Control carry no M2 buffer (M3).
                };
                match port.direction() {
                    Direction::Output => {
                        let channels = u16_channels(format.layout.channels())?;
                        let idx = slot_specs.len();
                        slot_specs.push(SlotSpec { channels });
                        port_slot.insert(port.id(), idx);
                        outputs.push(idx);
                    }
                    Direction::Input => {
                        let edges: Vec<EdgeBinding> = spec
                            .connections()
                            .iter()
                            .filter(|c| c.to().port == port.id())
                            .map(|conn| {
                                if conn.delay() == EdgeDelay::Declared {
                                    return Err(CompileError::UnsupportedEdgeDelay {
                                        connection: conn.id(),
                                    });
                                }
                                let slot = *port_slot.get(&conn.from().port).ok_or(
                                    CompileError::Inconsistent {
                                        reason: "edge producer slot missing",
                                    },
                                )?;
                                // Producer must precede the consumer in
                                // schedule order (topo guarantee, audited).
                                if position[&conn.from().node] >= position[node_id] {
                                    return Err(CompileError::Inconsistent {
                                        reason: "producer scheduled after consumer",
                                    });
                                }
                                Ok(EdgeBinding {
                                    slot,
                                    map: conn.channel_map().map(|m| m.entries.clone()),
                                    delay_line: None,
                                    delay_frames: 0,
                                })
                            })
                            .collect::<Result<_, _>>()?;
                        inputs.push(InputBinding {
                            port: port.id(),
                            edges,
                            scratch: None,
                        });
                    }
                    _ => {
                        return Err(CompileError::Inconsistent {
                            reason: "unknown port direction",
                        });
                    }
                }
            }
            schedule.push(ScheduledNode {
                node: *node_id,
                pslot: pslots[node_id],
                inputs,
                outputs,
            });
        }
        Ok(PortBindings {
            port_slot,
            slot_specs,
            schedule,
        })
    }

    /// Pass C — accumulated per-node latency, per-edge compensation
    /// delay (`D_i = max(L_j) - L_i` per merge point), delay-line
    /// specs, and scratch-slot assignment for fan-in / channel-mapped
    /// / delayed inputs.
    fn apply_pdc(
        &self,
        spec: &GraphSpec,
        port_slot: &BTreeMap<PortId, usize>,
        own_latency: &BTreeMap<NodeId, u32>,
        slot_specs: &mut Vec<SlotSpec>,
        schedule: &mut [ScheduledNode],
    ) -> Result<(Vec<DelaySpec>, u32), CompileError> {
        let mut accum: BTreeMap<NodeId, u32> = BTreeMap::new();
        for snode in schedule.iter() {
            let mut parent_max = 0u32;
            for input in &snode.inputs {
                for edge in &input.edges {
                    let producer = spec
                        .port(slot_port(port_slot, edge.slot))
                        .map(PortSpec::owner);
                    if let Some(prod) = producer {
                        parent_max = parent_max.max(*accum.get(&prod).unwrap_or(&0));
                    }
                }
            }
            accum.insert(
                snode.node,
                own_latency[&snode.node].saturating_add(parent_max),
            );
        }
        let total_latency = *accum.values().max().unwrap_or(&0);
        if total_latency > self.config.max_delay_frames {
            return Err(CompileError::DelayOverflow {
                needed: total_latency,
                max: self.config.max_delay_frames,
            });
        }

        // Per-edge compensation delay + scratch assignment.
        let mut delay_specs: Vec<DelaySpec> = Vec::new();
        for snode in schedule {
            for input in &mut snode.inputs {
                if input.edges.is_empty() {
                    continue;
                }
                let target = input
                    .edges
                    .iter()
                    .map(|e| {
                        *accum
                            .get(
                                &spec
                                    .port(slot_port(port_slot, e.slot))
                                    .expect("edge slot owned by a port")
                                    .owner(),
                            )
                            .unwrap_or(&0)
                    })
                    .max()
                    .unwrap_or(0);
                let needs_scratch =
                    input.edges.len() > 1 || input.edges.iter().any(|e| e.map.is_some());
                for edge in &mut input.edges {
                    let prod = spec
                        .port(slot_port(port_slot, edge.slot))
                        .expect("edge slot owned by a port")
                        .owner();
                    edge.delay_frames = target.saturating_sub(accum[&prod]);
                    if edge.delay_frames > 0 {
                        edge.delay_line = Some(delay_specs.len());
                        delay_specs.push(DelaySpec {
                            channels: 0, // patched below with input layout
                            delay: edge.delay_frames,
                        });
                    }
                }
                if needs_scratch || input.edges.iter().any(|e| e.delay_frames > 0) {
                    let input_port = spec.port(input.port).ok_or(CompileError::Inconsistent {
                        reason: "bound input port missing",
                    })?;
                    let PortFormat::Audio(fmt) = input_port.format() else {
                        return Err(CompileError::Inconsistent {
                            reason: "audio binding on non-audio port",
                        });
                    };
                    let channels = u16_channels(fmt.layout.channels())?;
                    let scratch = slot_specs.len();
                    slot_specs.push(SlotSpec { channels });
                    input.scratch = Some(scratch);
                    for edge in &input.edges {
                        if let Some(dl) = edge.delay_line {
                            delay_specs[dl].channels = channels;
                        }
                    }
                }
            }
        }
        Ok((delay_specs, total_latency))
    }
}

/// Pass A output — factory decisions, slot assignments, and own
/// latencies for every node in runtime order.
struct ProcessAssignments {
    /// node → registry slot (`None` = boundary node).
    pslots: BTreeMap<NodeId, Option<usize>>,
    /// node → own latency in frames.
    own_latency: BTreeMap<NodeId, u32>,
    /// New instances the RT side must install on activation.
    installs: Vec<Install>,
    /// Nodes that keep an occupied registry slot.
    used_nodes: BTreeSet<NodeId>,
}

/// Pass B output — every bound port, the slot arena shape, and the
/// schedule in runtime order.
struct PortBindings {
    /// Audio port → owning slot.
    port_slot: BTreeMap<PortId, usize>,
    /// Slot arena shape (producer + scratch slots).
    slot_specs: Vec<SlotSpec>,
    /// Schedule in runtime order (PDC fields patched by Pass C).
    schedule: Vec<ScheduledNode>,
}

/// Graph boundary ports — caller-fed graph inputs and tap-side
/// graph outputs as `(port, slot)` pairs.
struct BoundaryPorts {
    /// Processor-less `Source` output ports.
    inputs: Vec<(PortId, usize)>,
    /// Processor-less `Sink` input ports.
    outputs: Vec<(PortId, usize)>,
}

/// `u32` channel count → `u16` slot metadata, rejecting overflow.
fn u16_channels(channels: u32) -> Result<u16, CompileError> {
    u16::try_from(channels).map_err(|_| CompileError::Inconsistent {
        reason: "channel count exceeds u16",
    })
}

/// Aliasing audit — every node's read set must be disjoint from its
/// write set (true by construction; verified anyway).
fn audit_aliasing(schedule: &[ScheduledNode]) -> Result<(), CompileError> {
    for snode in schedule {
        let mut read_slots = BTreeSet::new();
        for input in &snode.inputs {
            if let Some(scratch) = input.scratch {
                read_slots.insert(scratch);
            }
            for edge in &input.edges {
                read_slots.insert(edge.slot);
            }
        }
        if snode.outputs.iter().any(|o| read_slots.contains(o)) {
            return Err(CompileError::BufferAlias { node: snode.node });
        }
    }
    Ok(())
}

/// Boundary ports — processor-less Source outputs are graph inputs;
/// processor-less Sink inputs are graph outputs.
fn collect_boundaries(
    spec: &GraphSpec,
    schedule: &[ScheduledNode],
) -> Result<BoundaryPorts, CompileError> {
    let mut inputs_b: Vec<(PortId, usize)> = Vec::new();
    let mut outputs_b: Vec<(PortId, usize)> = Vec::new();
    for snode in schedule {
        let node = spec.node(snode.node).ok_or(CompileError::Inconsistent {
            reason: "missing scheduled node",
        })?;
        match (node.kind(), snode.pslot.is_some()) {
            (NodeKind::Source, false) => {
                let mut audio_outs = snode.outputs.iter();
                for port in node.ports() {
                    if port.direction() == Direction::Output
                        && matches!(port.format(), PortFormat::Audio(_))
                        && let Some(&slot) = audio_outs.next()
                    {
                        inputs_b.push((port.id(), slot));
                    }
                }
            }
            (NodeKind::Sink, false) => {
                for input in &snode.inputs {
                    if let Some(slot) = input
                        .scratch
                        .or_else(|| input.edges.first().map(|e| e.slot))
                    {
                        outputs_b.push((input.port, slot));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(BoundaryPorts {
        inputs: inputs_b,
        outputs: outputs_b,
    })
}

/// Reverse lookup: slot index → its producer `PortId`.
fn slot_port(port_slot: &BTreeMap<PortId, usize>, slot: usize) -> PortId {
    port_slot
        .iter()
        .find(|(_, s)| **s == slot)
        .map(|(p, _)| *p)
        .expect("slot owned by a port")
}
