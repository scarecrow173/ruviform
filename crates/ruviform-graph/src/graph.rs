//! Graph structure: node/port definitions, connections, the builder
//! that issues stable IDs, and the completed `GraphSpec`.

use crate::ids::{ConnectionId, NodeId, PortId};
use crate::port::{
    Activation, Availability, ChannelMap, Direction, Multiplicity, PortCapability, PortDef,
    PortFormat, PortRole,
};

/// Structural classification of a node.
///
/// `NodeKind` imposes *no* direction constraint — a `Source` may
/// declare inputs and a `Sink` may declare outputs; semantics are the
/// M2+ consumer's concern.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// Transforms inputs into outputs.
    Processor,
    /// Produces signal (typically outputs only — unconstrained).
    Source,
    /// Consumes signal (typically inputs only — unconstrained).
    Sink,
}

/// Builder-side node declaration — ID-free.
#[derive(Clone, Debug)]
pub struct NodeDef {
    /// Structural classification.
    pub kind: NodeKind,
    /// Declared ports, in declaration order.
    pub ports: Vec<PortDef>,
}

/// A node endpoint of a connection: owner node plus the port issued
/// to it. The validator rejects a `port` issued to a different node
/// ([`crate::ValidationError::PortNotOnNode`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PortRef {
    /// Owning node.
    pub node: NodeId,
    /// Port on that node.
    pub port: PortId,
}

/// Declared propagation delay of a connection edge.
///
/// `Declared` delay is representable in IR, but *delayed feedback
/// cycles are unsupported in M1*: any cyclic SCC containing a declared
/// edge is rejected as
/// [`crate::ValidationError::DelayedCycleUnsupported`]. A future
/// payload (delay amount/unit) is deferred to a later migration.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum EdgeDelay {
    /// Zero propagation delay.
    #[default]
    Zero,
    /// Some declared non-zero delay (payload deferred).
    Declared,
}

/// Builder-side connection declaration — ID-free.
#[derive(Clone, Debug)]
pub struct ConnDef {
    /// Producing endpoint (`Direction::Output` required).
    pub from: PortRef,
    /// Consuming endpoint (`Direction::Input` required).
    pub to: PortRef,
    /// Edge delay (zero unless declared).
    pub delay: EdgeDelay,
    /// Explicit channel map — required when audio layouts differ.
    pub channel_map: Option<ChannelMap>,
}

/// Completed, immutable node with issued IDs.
#[derive(Clone, Debug)]
pub struct NodeSpec {
    id: NodeId,
    kind: NodeKind,
    ports: Vec<PortSpec>,
}

impl NodeSpec {
    /// Stable node ID.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    /// Structural classification.
    #[must_use]
    pub const fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Declared ports in declaration order.
    #[must_use]
    pub fn ports(&self) -> &[PortSpec] {
        &self.ports
    }

    /// Looks up a port owned by this node.
    #[must_use]
    pub fn port(&self, id: PortId) -> Option<&PortSpec> {
        self.ports.iter().find(|p| p.id == id)
    }
}

/// Completed, immutable port with its issued ID and owner.
#[derive(Clone, Debug)]
pub struct PortSpec {
    id: PortId,
    owner: NodeId,
    direction: Direction,
    role: PortRole,
    format: PortFormat,
    multiplicity: Multiplicity,
    capabilities: Vec<PortCapability>,
    availability: Availability,
    activation: Activation,
}

impl PortSpec {
    /// Stable port ID.
    #[must_use]
    pub const fn id(&self) -> PortId {
        self.id
    }

    /// Node this port was issued to.
    #[must_use]
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    /// Port direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// Structural bus role.
    #[must_use]
    pub const fn role(&self) -> PortRole {
        self.role
    }

    /// Payload format — domain derives via
    /// [`PortFormat::domain`].
    #[must_use]
    pub const fn format(&self) -> PortFormat {
        self.format
    }

    /// Connection-count bounds.
    #[must_use]
    pub const fn multiplicity(&self) -> Multiplicity {
        self.multiplicity
    }

    /// Advertised capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &[PortCapability] {
        &self.capabilities
    }

    /// Mandatory vs. optional presence.
    #[must_use]
    pub const fn availability(&self) -> Availability {
        self.availability
    }

    /// Enabled vs. declared-but-inactive.
    #[must_use]
    pub const fn activation(&self) -> Activation {
        self.activation
    }
}

/// Completed, immutable connection with issued ID.
#[derive(Clone, Debug)]
pub struct Connection {
    id: ConnectionId,
    from: PortRef,
    to: PortRef,
    delay: EdgeDelay,
    channel_map: Option<ChannelMap>,
}

impl Connection {
    /// Stable connection ID.
    #[must_use]
    pub const fn id(&self) -> ConnectionId {
        self.id
    }

    /// Producing endpoint.
    #[must_use]
    pub const fn from(&self) -> PortRef {
        self.from
    }

    /// Consuming endpoint.
    #[must_use]
    pub const fn to(&self) -> PortRef {
        self.to
    }

    /// Declared edge delay.
    #[must_use]
    pub const fn delay(&self) -> EdgeDelay {
        self.delay
    }

    /// Explicit channel map, if declared.
    #[must_use]
    pub fn channel_map(&self) -> Option<&ChannelMap> {
        self.channel_map.as_ref()
    }
}

/// Completed graph structure — fields are private and only built by
/// [`GraphBuilder`], so every ID inside is builder-issued. The spec
/// carries no revision; [`crate::GraphProject`] stamps revisions.
#[derive(Clone, Debug)]
pub struct GraphSpec {
    nodes: Vec<NodeSpec>,
    connections: Vec<Connection>,
}

impl GraphSpec {
    /// Nodes in issue order.
    #[must_use]
    pub fn nodes(&self) -> &[NodeSpec] {
        &self.nodes
    }

    /// Connections in issue order.
    #[must_use]
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// Looks up a node by ID.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&NodeSpec> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Looks up a port by ID across all nodes.
    #[must_use]
    pub fn port(&self, id: PortId) -> Option<&PortSpec> {
        self.nodes
            .iter()
            .flat_map(NodeSpec::ports)
            .find(|p| p.id == id)
    }
}

/// Builder issuing deterministic stable IDs (each space starts at
/// `u64` value `1`). Callers supply ID-free `NodeDef`/`ConnDef`;
/// completed structures are only reachable through [`GraphSpec`].
#[derive(Debug)]
pub struct GraphBuilder {
    next_node: u64,
    next_port: u64,
    next_conn: u64,
    nodes: Vec<NodeSpec>,
    connections: Vec<Connection>,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    /// Empty builder — each ID space issues from `1`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_node: 1,
            next_port: 1,
            next_conn: 1,
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Issues the node ID and each port ID, returning them in
    /// declaration order.
    pub fn add_node(&mut self, def: &NodeDef) -> (NodeId, Vec<PortId>) {
        let node = NodeId::from_issued(self.next_node);
        self.next_node += 1;
        let ports = def
            .ports
            .iter()
            .map(|p| {
                let id = PortId::from_issued(self.next_port);
                self.next_port += 1;
                PortSpec {
                    id,
                    owner: node,
                    direction: p.direction,
                    role: p.role,
                    format: p.format,
                    multiplicity: p.multiplicity,
                    capabilities: p.capabilities.clone(),
                    availability: p.availability,
                    activation: p.activation,
                }
            })
            .collect::<Vec<_>>();
        let ids = ports.iter().map(|p| p.id).collect();
        self.nodes.push(NodeSpec {
            id: node,
            kind: def.kind,
            ports,
        });
        (node, ids)
    }

    /// Issues a connection ID and records the endpoints as declared.
    /// Referential/structural checks belong to the
    /// [`crate::Validator`], not the builder — this never fails.
    pub fn connect(&mut self, def: &ConnDef) -> ConnectionId {
        let id = ConnectionId::from_issued(self.next_conn);
        self.next_conn += 1;
        self.connections.push(Connection {
            id,
            from: def.from,
            to: def.to,
            delay: def.delay,
            channel_map: def.channel_map.clone(),
        });
        id
    }

    /// Completes the spec. Infallible — validation is a separate,
    /// explicit step so an invalid spec can still be inspected.
    #[must_use]
    pub fn build(self) -> GraphSpec {
        GraphSpec {
            nodes: self.nodes,
            connections: self.connections,
        }
    }
}
