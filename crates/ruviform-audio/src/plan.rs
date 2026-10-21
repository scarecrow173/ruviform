//! Immutable graph snapshot + `try_read` swap (ADR
//! `audio-engine-snapshot-swap.md`): the control side publishes a
//! fully-prepared `GraphPlan` via `Arc`; the RT side takes `try_read`
//! and falls back to the last-good cache on contention (≤ 1 block late).

use std::sync::{Arc, Mutex, RwLock};

use crate::graph::Graph;

/// An immutable, fully-prepared execution plan. The graph inside is
/// mutex-guarded so the callback can take `&mut` access without blocking
/// the control side — contention resolves by keeping the last plan.
///
/// **RT contract:** construct on the non-RT side only (allocates).
pub struct GraphPlan {
    graph: Mutex<Graph>,
}

impl GraphPlan {
    /// Wraps a fully-constructed graph.
    #[must_use]
    pub fn new(graph: Graph) -> Self {
        Self {
            graph: Mutex::new(graph),
        }
    }

    /// Drives the plan's graph over `out`. RT-suitable: `try_lock` never
    /// blocks; on (impossible in practice) contention the block is a
    /// silent no-op — `out` is left as handed in.
    pub fn drive(
        &self,
        out: &mut ruviform_audio_core::AudioBufferViewMut<'_, f32>,
        ctx: &ruviform_rt::RtContext,
    ) {
        if let Ok(mut graph) = self.graph.try_lock() {
            graph.pull(out, ctx);
        }
    }
}

/// Control-side handle for swapping the active plan. Non-RT only —
/// `write` takes the lock.
///
/// **RT contract:** non-RT only.
#[derive(Clone)]
pub struct PlanSwap {
    current: Arc<RwLock<Arc<GraphPlan>>>,
}

/// RT-side handle — moved into the device callback. `try_read` +
/// last-good cache, never blocks.
///
/// **RT contract:** `active()` is RT-suitable; creating it is non-RT.
pub struct RtPlan {
    current: Arc<RwLock<Arc<GraphPlan>>>,
    cached: Option<Arc<GraphPlan>>,
}

impl PlanSwap {
    /// Creates the pair over an initial plan.
    #[must_use]
    pub fn new(plan: GraphPlan) -> (Self, RtPlan) {
        let current = Arc::new(RwLock::new(Arc::new(plan)));
        (
            Self {
                current: Arc::clone(&current),
            },
            RtPlan {
                current,
                cached: None,
            },
        )
    }

    /// Publishes a new plan; the RT side picks it up on the next block
    /// (or the block after under contention).
    pub fn swap(&self, plan: GraphPlan) {
        *self
            .current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(plan);
    }
}

impl RtPlan {
    /// Returns the active plan — `try_read`, else the last cached
    /// snapshot. First call before any successful read yields `None`
    /// only if the initial lock is contended.
    ///
    /// **RT contract:** RT-suitable — `try_read`, no allocation beyond
    /// the `Arc` refcount bump.
    pub fn active(&mut self) -> Option<Arc<GraphPlan>> {
        if let Ok(guard) = self.current.try_read() {
            let plan = Arc::clone(&guard);
            self.cached = Some(Arc::clone(&plan));
            Some(plan)
        } else {
            self.cached.clone()
        }
    }
}
