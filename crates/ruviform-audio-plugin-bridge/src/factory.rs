//! `PluginNodeFactory` — hands pre-attached [`PluginNode`]s to the
//! typed compiler's `NodeFactory` hook.
//!
//! Ownership rule (M4-06): a node enters the graph exactly once, via
//! `PrepareOutcome::New`. A recompile for a node the runtime already
//! installed with **no fresh `insert`** returns `Reuse` — the live
//! instance keeps its state. A caller that re-inserts after a
//! configuration change gets `New` and the runtime's generation-safe
//! install path swaps without double-driving the old instance.

use std::collections::HashMap;

use ruviform_audio::typed::{FactoryError, InstalledInfo, NodeFactory, PrepareOutcome};
use ruviform_graph::{NodeId, NodeKind, NodeSpec};

use crate::node::PluginNode;

/// Non-RT provider keyed by [`NodeId`]: `insert` a fresh node before
/// compiling; `prepare` consumes it once.
#[derive(Default)]
pub struct PluginNodeFactory {
    pending: HashMap<NodeId, PluginNode>,
}

impl PluginNodeFactory {
    /// An empty factory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stages `node` for `id`'s next compile — any previously staged
    /// node for `id` is dropped (non-RT — the staged node owns an
    /// endpoint and plan vault).
    ///
    /// **RT contract:** non-RT only.
    pub fn insert(&mut self, id: NodeId, node: PluginNode) {
        self.pending.insert(id, node);
    }

    /// Whether a fresh node is staged for `id`.
    #[must_use]
    pub fn is_pending(&self, id: NodeId) -> bool {
        self.pending.contains_key(&id)
    }
}

impl NodeFactory for PluginNodeFactory {
    fn prepare(
        &mut self,
        node: &NodeSpec,
        existing: Option<&InstalledInfo>,
    ) -> Result<PrepareOutcome, FactoryError> {
        if node.kind() != NodeKind::Processor {
            return Ok(PrepareOutcome::Boundary);
        }
        if let Some(node) = self.pending.remove(&node.id()) {
            return Ok(PrepareOutcome::New(Box::new(node)));
        }
        if existing.is_some() {
            return Ok(PrepareOutcome::Reuse);
        }
        Err(FactoryError(format!(
            "no plugin node prepared for {:?}",
            node.id()
        )))
    }
}
