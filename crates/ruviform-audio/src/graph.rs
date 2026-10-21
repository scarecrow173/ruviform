//! Pull-driven processing graph.
//!
//! A node is asked by its destination to render into (or onto) the
//! output view — the pull direction is destination → upstream. The v1
//! graph is an ordered list: each node runs in sequence on the callback's
//! `out` view. Sources overwrite, transforms modify in place, and the
//! [`crate::Mixer`] accumulates.

use ruviform_audio_core::AudioBufferViewMut;
use ruviform_rt::RtContext;

/// One pull-driven audio node. Called on the RT thread once per block.
///
/// **RT contract:** implementations must not allocate, block, or call OS
/// APIs — buffers must be pre-allocated at `prepare` time.
pub trait AudioNode: Send {
    /// Renders this node onto `out` (in-place transform or accumulate).
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext);
}

/// An ordered node list; `pull` drives nodes in registration order.
///
/// **RT contract:** construction/mutation is non-RT; [`Graph::pull`]
/// is RT-suitable when every contained node is.
pub struct Graph {
    nodes: Vec<Box<dyn AudioNode>>,
}

impl Graph {
    /// Empty graph — pulling renders silence (leaves `out` untouched).
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Appends a node to the pull order. Non-RT (allocates).
    pub fn push(&mut self, node: Box<dyn AudioNode>) {
        self.nodes.push(node);
    }

    /// Number of nodes in the graph.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Drives every node over `out` in order.
    ///
    /// **RT contract:** RT-suitable iff all nodes are.
    pub fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext) {
        for node in &mut self.nodes {
            node.pull(out, ctx);
        }
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
