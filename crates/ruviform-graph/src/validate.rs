//! Non-RT graph validation: referential integrity, direction, domain,
//! format hierarchy, explicit channel maps, capacity, required
//! connections, and cycle rejection.
//!
//! Every rule collects *all* failures into [`ValidationErrors`] — the
//! validator never stops at the first problem so a caller can repair a
//! spec in one pass. Success yields the immutable [`ValidatedGraph`]
//! snapshot, which always describes a DAG (all cycles are rejected).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ruviform_audio_core::AudioFormat;

use crate::graph::{Connection, EdgeDelay, GraphSpec, PortRef, PortSpec};
use crate::ids::{ConnectionId, NodeId, PortId};
use crate::mapping::StableHandleMap;
use crate::port::{Activation, Availability, Direction, PortFormat};

/// One validation failure — the variant identifies the violated rule.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// A connection endpoint references a node that does not exist.
    MissingNode {
        /// Offending connection.
        connection: ConnectionId,
        /// Referenced node that is absent.
        node: NodeId,
    },
    /// A connection endpoint references a port that does not exist.
    MissingPort {
        /// Offending connection.
        connection: ConnectionId,
        /// Referenced port that is absent.
        port: PortId,
    },
    /// The port exists but is owned by a different node than the
    /// endpoint's `PortRef::node`.
    PortNotOnNode {
        /// Offending connection.
        connection: ConnectionId,
        /// Node the endpoint claims owns the port.
        node: NodeId,
        /// Port actually owned elsewhere.
        port: PortId,
    },
    /// `from` is not an `Output` or `to` is not an `Input`.
    InvalidDirection {
        /// Offending connection.
        connection: ConnectionId,
        /// Actual direction of the `from` port.
        from: Direction,
        /// Actual direction of the `to` port.
        to: Direction,
    },
    /// The two ports' `PortFormat::domain()` values differ.
    DomainMismatch {
        /// Offending connection.
        connection: ConnectionId,
    },
    /// Audio formats differ in sample rate or sample format
    /// (lower-level attributes checked before layout/map).
    FormatMismatch {
        /// Offending connection.
        connection: ConnectionId,
    },
    /// Audio layouts differ and no explicit `ChannelMap` was declared.
    ImplicitConversionRequired {
        /// Offending connection.
        connection: ConnectionId,
    },
    /// A declared `ChannelMap` is malformed or inapplicable.
    InvalidChannelMap {
        /// Offending connection.
        connection: ConnectionId,
        /// Why the map is rejected.
        reason: &'static str,
    },
    /// Connection count on a port exceeds `Multiplicity::max`.
    CapacityExceeded {
        /// Over-capacity port.
        port: PortId,
        /// Whether the count is fan-in (`Input`) or fan-out (`Output`).
        direction: Direction,
        /// Observed connection count.
        have: u16,
        /// Declared maximum.
        max: u16,
    },
    /// An `Enabled` + `Always` port has fewer connections than
    /// `Multiplicity::min`.
    RequiredConnections {
        /// Under-connected port.
        port: PortId,
        /// Observed connection count.
        have: u16,
        /// Declared minimum.
        min: u16,
    },
    /// A cyclic strongly-connected component whose edges are all
    /// `EdgeDelay::Zero` — includes self-loops.
    ZeroDelayCycle {
        /// Nodes forming the cyclic SCC (sorted by ID).
        nodes: Vec<NodeId>,
    },
    /// A cyclic SCC containing at least one `EdgeDelay::Declared`
    /// edge — delayed feedback cycles are unsupported in M1.
    DelayedCycleUnsupported {
        /// Nodes forming the cyclic SCC (sorted by ID).
        nodes: Vec<NodeId>,
        /// Why the cycle is rejected.
        reason: &'static str,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { connection, node } => {
                write!(f, "{connection} references missing node {node}")
            }
            Self::MissingPort { connection, port } => {
                write!(f, "{connection} references missing port {port}")
            }
            Self::PortNotOnNode {
                connection,
                node,
                port,
            } => write!(f, "{connection} references {port} not owned by {node}"),
            Self::InvalidDirection {
                connection,
                from,
                to,
            } => write!(
                f,
                "{connection} has invalid direction {from:?} -> {to:?} (requires Output -> Input)"
            ),
            Self::DomainMismatch { connection } => {
                write!(f, "{connection} connects ports of different signal domains")
            }
            Self::FormatMismatch { connection } => {
                write!(
                    f,
                    "{connection} audio formats differ in sample rate or sample format"
                )
            }
            Self::ImplicitConversionRequired { connection } => write!(
                f,
                "{connection} audio layouts differ; an explicit ChannelMap is required"
            ),
            Self::InvalidChannelMap { connection, reason } => {
                write!(f, "{connection} channel map invalid: {reason}")
            }
            Self::CapacityExceeded {
                port,
                direction,
                have,
                max,
            } => write!(
                f,
                "{port} ({direction:?}) has {have} connections, exceeding max {max}"
            ),
            Self::RequiredConnections { port, have, min } => {
                write!(f, "{port} has {have} connections, below required min {min}")
            }
            Self::ZeroDelayCycle { nodes } => {
                write!(f, "zero-delay cycle over nodes {nodes:?}")
            }
            Self::DelayedCycleUnsupported { nodes, reason } => {
                write!(f, "delayed cycle over nodes {nodes:?}: {reason}")
            }
        }
    }
}

impl core::error::Error for ValidationError {}

/// All failures found in one validation pass — never a partial list.
#[derive(Clone, Debug, Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Every collected error, in detection order.
    #[must_use]
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Number of collected errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Whether no errors were collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} validation error(s):", self.errors.len())?;
        for error in &self.errors {
            write!(f, " {error};")?;
        }
        Ok(())
    }
}

impl core::error::Error for ValidationErrors {}

/// The non-RT validator. Stateless — construction is free and there
/// is no configuration in M1.
#[derive(Copy, Clone, Debug, Default)]
pub struct Validator;

impl Validator {
    /// Builds a stateless validator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Runs every rule over `spec`. On success returns the immutable
    /// [`ValidatedGraph`] snapshot with its stable↔runtime mapping;
    /// on failure returns every detected error.
    ///
    /// All rules are evaluated — the validator does not stop early —
    /// except that per-connection checks are skipped once an
    /// endpoint cannot be resolved (its port attributes are unknown).
    ///
    /// # Errors
    /// Returns [`ValidationErrors`] containing all detected failures.
    pub fn validate(spec: &GraphSpec) -> Result<ValidatedGraph, ValidationErrors> {
        let mut errors = ValidationErrors::default();

        // Per-endpoint resolution + structural checks.
        let mut resolved: Vec<(&Connection, &PortSpec, &PortSpec)> = Vec::new();
        for conn in spec.connections() {
            let from = resolve_endpoint(spec, conn, conn.from(), &mut errors);
            let to = resolve_endpoint(spec, conn, conn.to(), &mut errors);
            if let (Some(from), Some(to)) = (from, to) {
                check_structure(conn, from, to, &mut errors);
                resolved.push((conn, from, to));
            }
        }

        // Capacity + required checks count resolved connections only.
        check_multiplicity(spec, &resolved, &mut errors);

        // Cycle check over resolvable node-level edges (all delays).
        check_cycles(spec, &resolved, &mut errors);

        if errors.is_empty() {
            Ok(ValidatedGraph::new(spec.clone()))
        } else {
            Err(errors)
        }
    }
}

/// Resolves an endpoint's node/port/owner. Returns the `PortSpec`
/// when fully resolvable; records the first applicable error.
fn resolve_endpoint<'a>(
    spec: &'a GraphSpec,
    conn: &Connection,
    endpoint: PortRef,
    errors: &mut ValidationErrors,
) -> Option<&'a PortSpec> {
    if spec.node(endpoint.node).is_none() {
        errors.push(ValidationError::MissingNode {
            connection: conn.id(),
            node: endpoint.node,
        });
        return None;
    }
    let port = spec.port(endpoint.port);
    let Some(port) = port else {
        errors.push(ValidationError::MissingPort {
            connection: conn.id(),
            port: endpoint.port,
        });
        return None;
    };
    if port.owner() != endpoint.node {
        errors.push(ValidationError::PortNotOnNode {
            connection: conn.id(),
            node: endpoint.node,
            port: endpoint.port,
        });
        return None;
    }
    Some(port)
}

/// Direction, domain, format, layout, and channel-map checks for a
/// fully resolved connection.
fn check_structure(
    conn: &Connection,
    from: &PortSpec,
    to: &PortSpec,
    errors: &mut ValidationErrors,
) {
    if from.direction() != Direction::Output || to.direction() != Direction::Input {
        errors.push(ValidationError::InvalidDirection {
            connection: conn.id(),
            from: from.direction(),
            to: to.direction(),
        });
    }

    if from.format().domain() != to.format().domain() {
        errors.push(ValidationError::DomainMismatch {
            connection: conn.id(),
        });
        return;
    }

    match (from.format(), to.format()) {
        (PortFormat::Audio(fa), PortFormat::Audio(ta)) => {
            check_audio(conn, fa, ta, errors);
        }
        _ => {
            // Matched non-audio domain — only the map rule applies.
            if conn.channel_map().is_some() {
                errors.push(ValidationError::InvalidChannelMap {
                    connection: conn.id(),
                    reason: "channel map declared on a non-audio connection",
                });
            }
        }
    }
}

/// Audio-only checks: sample rate / sample format first
/// (`FormatMismatch`), then layout (`ImplicitConversionRequired` or
/// `ChannelMap` validation). An explicit map on equal layouts is
/// validated as a declared permutation.
fn check_audio(
    conn: &Connection,
    from: AudioFormat,
    to: AudioFormat,
    errors: &mut ValidationErrors,
) {
    if from.sample_rate != to.sample_rate || from.sample_format != to.sample_format {
        errors.push(ValidationError::FormatMismatch {
            connection: conn.id(),
        });
        return;
    }

    let layouts_differ = from.layout != to.layout;
    match conn.channel_map() {
        None => {
            if layouts_differ {
                errors.push(ValidationError::ImplicitConversionRequired {
                    connection: conn.id(),
                });
            }
        }
        Some(map) => {
            let src_ch = from.layout.channels();
            let dst_ch = to.layout.channels();
            let mut seen_dst = BTreeSet::new();
            let mut ok = !map.entries.is_empty();
            for &(src, dst) in &map.entries {
                if u32::from(src) >= src_ch {
                    errors.push(ValidationError::InvalidChannelMap {
                        connection: conn.id(),
                        reason: "source channel index out of range",
                    });
                    ok = false;
                }
                if u32::from(dst) >= dst_ch {
                    errors.push(ValidationError::InvalidChannelMap {
                        connection: conn.id(),
                        reason: "destination channel index out of range",
                    });
                    ok = false;
                } else if !seen_dst.insert(dst) {
                    errors.push(ValidationError::InvalidChannelMap {
                        connection: conn.id(),
                        reason: "destination channel mapped more than once",
                    });
                    ok = false;
                }
            }
            if ok && seen_dst.len() != usize::try_from(dst_ch).unwrap_or(usize::MAX) {
                errors.push(ValidationError::InvalidChannelMap {
                    connection: conn.id(),
                    reason: "destination channels not fully covered",
                });
            }
        }
    }
}

/// `max` (capacity) and `min` (required) checks, counting only
/// connections whose endpoints resolved.
fn check_multiplicity(
    spec: &GraphSpec,
    resolved: &[(&Connection, &PortSpec, &PortSpec)],
    errors: &mut ValidationErrors,
) {
    let mut counts: BTreeMap<PortId, u16> = BTreeMap::new();
    for (_, from, to) in resolved {
        *counts.entry(from.id()).or_default() += 1;
        *counts.entry(to.id()).or_default() += 1;
    }
    for node in spec.nodes() {
        for port in node.ports() {
            let have = counts.get(&port.id()).copied().unwrap_or(0);
            let mult = port.multiplicity();
            if have > mult.max {
                errors.push(ValidationError::CapacityExceeded {
                    port: port.id(),
                    direction: port.direction(),
                    have,
                    max: mult.max,
                });
            }
            if port.activation() == Activation::Enabled
                && port.availability() == Availability::Always
                && have < mult.min
            {
                errors.push(ValidationError::RequiredConnections {
                    port: port.id(),
                    have,
                    min: mult.min,
                });
            }
        }
    }
}

/// Full-graph SCC analysis. Every cyclic SCC is rejected: all-zero
/// edges → `ZeroDelayCycle`; any declared-delay edge →
/// `DelayedCycleUnsupported`. A validated graph is therefore always
/// a DAG.
fn check_cycles(
    spec: &GraphSpec,
    resolved: &[(&Connection, &PortSpec, &PortSpec)],
    errors: &mut ValidationErrors,
) {
    // Node-level adjacency, keeping the edge delay per edge.
    let mut adjacency: BTreeMap<NodeId, Vec<(NodeId, EdgeDelay)>> = BTreeMap::new();
    for (conn, from, to) in resolved {
        adjacency
            .entry(from.owner())
            .or_default()
            .push((to.owner(), conn.delay()));
    }

    let sccs = tarjan(spec, &adjacency);
    for component in sccs {
        let is_cycle = component.len() > 1
            || component.iter().any(|n| {
                adjacency
                    .get(n)
                    .is_some_and(|edges| edges.iter().any(|(dst, _)| dst == n))
            });
        if !is_cycle {
            continue;
        }
        let member: BTreeSet<NodeId> = component.iter().copied().collect();
        let mut nodes: Vec<NodeId> = component;
        nodes.sort_unstable();
        let has_declared = adjacency.iter().any(|(src, edges)| {
            member.contains(src)
                && edges
                    .iter()
                    .any(|(dst, delay)| member.contains(dst) && *delay == EdgeDelay::Declared)
        });
        if has_declared {
            errors.push(ValidationError::DelayedCycleUnsupported {
                nodes,
                reason: "delayed feedback cycles are unsupported in M1",
            });
        } else {
            errors.push(ValidationError::ZeroDelayCycle { nodes });
        }
    }
}

/// Tarjan's SCC algorithm over the node adjacency. Returns each
/// strongly connected component as a `Vec<NodeId>`.
fn tarjan(
    spec: &GraphSpec,
    adjacency: &BTreeMap<NodeId, Vec<(NodeId, EdgeDelay)>>,
) -> Vec<Vec<NodeId>> {
    struct State<'a> {
        adjacency: &'a BTreeMap<NodeId, Vec<(NodeId, EdgeDelay)>>,
        next: usize,
        stack: Vec<NodeId>,
        on_stack: BTreeSet<NodeId>,
        index: BTreeMap<NodeId, usize>,
        lowlink: BTreeMap<NodeId, usize>,
        out: Vec<Vec<NodeId>>,
    }

    fn strongconnect(state: &mut State<'_>, v: NodeId) {
        state.index.insert(v, state.next);
        state.lowlink.insert(v, state.next);
        state.next += 1;
        state.stack.push(v);
        state.on_stack.insert(v);

        if let Some(edges) = state.adjacency.get(&v) {
            for &(w, _) in edges {
                if !state.index.contains_key(&w) {
                    strongconnect(state, w);
                    let lw = state.lowlink[&w];
                    let lv = state.lowlink[&v];
                    state.lowlink.insert(v, lv.min(lw));
                } else if state.on_stack.contains(&w) {
                    let iw = state.index[&w];
                    let lv = state.lowlink[&v];
                    state.lowlink.insert(v, lv.min(iw));
                }
            }
        }

        if state.lowlink[&v] == state.index[&v] {
            let mut component = Vec::new();
            while let Some(w) = state.stack.pop() {
                state.on_stack.remove(&w);
                component.push(w);
                if w == v {
                    break;
                }
            }
            state.out.push(component);
        }
    }

    let mut state = State {
        adjacency,
        next: 0,
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        index: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        out: Vec::new(),
    };
    for node in spec.nodes() {
        if !state.index.contains_key(&node.id()) {
            strongconnect(&mut state, node.id());
        }
    }
    state.out
}

/// Immutable validated snapshot: the spec plus the deterministic
/// stable↔runtime mapping. Only produced by [`Validator::validate`]
/// on a clean pass — always a DAG.
#[derive(Clone, Debug)]
pub struct ValidatedGraph {
    spec: GraphSpec,
    mapping: StableHandleMap,
}

impl ValidatedGraph {
    fn new(spec: GraphSpec) -> Self {
        let mapping = StableHandleMap::new(&spec);
        Self { spec, mapping }
    }

    /// The validated structure.
    #[must_use]
    pub const fn spec(&self) -> &GraphSpec {
        &self.spec
    }

    /// Stable↔runtime mapping.
    #[must_use]
    pub const fn mapping(&self) -> &StableHandleMap {
        &self.mapping
    }
}
