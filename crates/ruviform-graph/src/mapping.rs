//! Deterministic stable↔runtime mapping.
//!
//! Node runtime order is Kahn topological order with a `BTreeMap`
//! (ascending `NodeId`) tie-break, so the same `GraphSpec` always
//! maps to the same indexes. Ports take insertion order over
//! nodes→ports; connections take insertion order. Because validated
//! graphs are DAGs, the topological pass always places every node.

use std::collections::{BTreeMap, BTreeSet};

use crate::graph::{Connection, GraphSpec, PortSpec};
use crate::ids::{
    ConnectionId, NodeId, PortId, RuntimeConnectionIndex, RuntimeNodeIndex, RuntimePortIndex,
};

/// Bidirectional map between stable logical IDs and deterministic
/// runtime indexes.
#[derive(Clone, Debug)]
pub struct StableHandleMap {
    node_order: Vec<NodeId>,
    port_order: Vec<PortId>,
    conn_order: Vec<ConnectionId>,
    node_index: BTreeMap<NodeId, RuntimeNodeIndex>,
    port_index: BTreeMap<PortId, RuntimePortIndex>,
    conn_index: BTreeMap<ConnectionId, RuntimeConnectionIndex>,
}

impl StableHandleMap {
    /// Builds the mapping for a spec. Called by
    /// [`crate::ValidatedGraph`] after validation — the spec is a DAG
    /// there, so every node is placed; any leftover (only possible on
    /// an unvalidated cyclic spec) is appended in ID order so the map
    /// stays total.
    pub(crate) fn new(spec: &GraphSpec) -> Self {
        let node_order = topo_order(spec);
        let port_order: Vec<PortId> = spec
            .nodes()
            .iter()
            .flat_map(|n| n.ports().iter().map(PortSpec::id))
            .collect();
        let conn_order: Vec<ConnectionId> = spec.connections().iter().map(Connection::id).collect();

        let node_index = node_order
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, RuntimeNodeIndex::new(i)))
            .collect();
        let port_index = port_order
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, RuntimePortIndex::new(i)))
            .collect();
        let conn_index = conn_order
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, RuntimeConnectionIndex::new(i)))
            .collect();
        Self {
            node_order,
            port_order,
            conn_order,
            node_index,
            port_index,
            conn_index,
        }
    }

    /// Runtime position of a node.
    #[must_use]
    pub fn node_index(&self, id: NodeId) -> Option<RuntimeNodeIndex> {
        self.node_index.get(&id).copied()
    }

    /// Node at a runtime position.
    #[must_use]
    pub fn node_id(&self, index: RuntimeNodeIndex) -> Option<NodeId> {
        self.node_order.get(index.get()).copied()
    }

    /// Runtime position of a port.
    #[must_use]
    pub fn port_index(&self, id: PortId) -> Option<RuntimePortIndex> {
        self.port_index.get(&id).copied()
    }

    /// Port at a runtime position.
    #[must_use]
    pub fn port_id(&self, index: RuntimePortIndex) -> Option<PortId> {
        self.port_order.get(index.get()).copied()
    }

    /// Runtime position of a connection.
    #[must_use]
    pub fn connection_index(&self, id: ConnectionId) -> Option<RuntimeConnectionIndex> {
        self.conn_index.get(&id).copied()
    }

    /// Connection at a runtime position.
    #[must_use]
    pub fn connection_id(&self, index: RuntimeConnectionIndex) -> Option<ConnectionId> {
        self.conn_order.get(index.get()).copied()
    }

    /// Nodes in deterministic runtime (topological) order.
    #[must_use]
    pub fn node_order(&self) -> &[NodeId] {
        &self.node_order
    }
}

/// Kahn topological order; the ready set is a `BTreeMap` keyed by
/// `NodeId`, so ties always resolve to the smallest outstanding ID —
/// fully deterministic.
fn topo_order(spec: &GraphSpec) -> Vec<NodeId> {
    let mut indegree: BTreeMap<NodeId, usize> = spec.nodes().iter().map(|n| (n.id(), 0)).collect();
    for conn in spec.connections() {
        // Only edges between existing nodes can resolve post-validation;
        // unresolved endpoints contribute no edge.
        if indegree.contains_key(&conn.from().node) && indegree.contains_key(&conn.to().node) {
            *indegree.entry(conn.to().node).or_default() += 1;
        }
    }

    let mut ready: BTreeSet<NodeId> = indegree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut order = Vec::with_capacity(indegree.len());

    // Precompute node→successors.
    let mut successors: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    for conn in spec.connections() {
        if indegree.contains_key(&conn.from().node) && indegree.contains_key(&conn.to().node) {
            successors
                .entry(conn.from().node)
                .or_default()
                .push(conn.to().node);
        }
    }

    while let Some(&id) = ready.iter().next() {
        ready.remove(&id);
        order.push(id);
        if let Some(succs) = successors.get(&id) {
            for &s in succs {
                if let Some(deg) = indegree.get_mut(&s) {
                    *deg -= 1;
                    if *deg == 0 {
                        ready.insert(s);
                    }
                }
            }
        }
    }

    // Unvalidated cyclic leftovers (unreachable via ValidatedGraph):
    // append in ID order to keep the map total.
    for (&id, _) in indegree.iter().filter(|(_, deg)| **deg > 0) {
        order.push(id);
    }
    order
}
