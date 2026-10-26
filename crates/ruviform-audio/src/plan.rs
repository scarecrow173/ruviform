//! Immutable graph snapshot + `try_read` swap (ADR
//! `audio-engine-snapshot-swap.md`): the control side publishes a
//! fully-prepared `GraphPlan` via `Arc`; the RT side takes `try_read`
//! and falls back to the last-good cache on contention (≤ 1 block late).
//!
//! ## Retirement seam
//!
//! The RT side must never run a plan's destructor: a `GraphPlan` owns
//! every node in the graph, and dropping the last `Arc` on the
//! callback thread would run unbounded destructor work there. The
//! [`RtPlan`] therefore hands every displaced `cached` plan — and, on
//! its own drop, the still-cached plan — to a
//! [`Retirer<Arc<GraphPlan>>`](ruviform_rt::Retirer), whose
//! [`DropCollector`](ruviform_rt::DropCollector) lives behind
//! [`PlanSwap`]. Every non-RT entry point (`swap`, `drain_retired`,
//! and teardown via the `Session` field order) drains the collector,
//! so retired plans are dropped on the non-RT side. The retire
//! channel is fixed-capacity: on saturation `retire` leaks the value
//! (`mem::forget` — leak-on-full, never RT-side drop) and counts it
//! on [`PlanSwap::retired_overflows`].

use std::sync::{Arc, Mutex, RwLock};

use ruviform_rt::{DropCollector, Retirer, retire_channel};

use crate::graph::Graph;

/// Default retire-channel capacity. `RtPlan` retires at most one
/// displaced `cached` plan per `active()` call plus one on drop, and
/// every non-RT entry point drains — the pending count stays ≪
/// capacity in the normal cadence.
const DEFAULT_RETIRE_CAPACITY: usize = 4;

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
    /// blocks; any `Err` — `WouldBlock` (contention, impossible in
    /// practice with a single driver) *or* `Poisoned` — makes the
    /// block a silent no-op leaving `out` as handed in.
    ///
    /// `Poisoned` is the load-bearing case: a panicking node unwinds
    /// through the guard and poisons the mutex, so *every* subsequent
    /// `drive` silently no-ops — the plan goes permanently silent.
    /// That degraded behavior is deliberate: driving a poisoned graph
    /// would risk re-entering a panicking node on the RT thread. The
    /// observed silence is transitive, not inherent to `drive` —
    /// `drive` itself writes nothing; the device port's interleave
    /// path zero-fills whatever the callback leaves unwritten
    /// (`ruviform-audio-device` `convert.rs`).
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

/// Control-side handle for swapping the active plan — and the
/// collecting side of the plan-retire channel.
///
/// [`PlanSwap::new`] / [`PlanSwap::with_retire_capacity`] mint a
/// `retire_channel<Arc<GraphPlan>>` pair: the [`Retirer`] moves into
/// the [`RtPlan`] on the RT side, while the [`DropCollector`] sits
/// behind this handle's `Arc<Mutex<…>>` so `Clone`d handles and the
/// `&self` entry points share one collector. `swap` drains the
/// collector before publishing, `drain_retired` is the explicit drain
/// point for swap-free operation, and the collector's own drop (still
/// on the non-RT thread — see `Session`'s field-order note) drops any
/// plans still queued at teardown.
///
/// Re-entrancy: destructors of retired plans run while the collector
/// mutex is held — `Drop` impls of plan nodes must not call
/// `PlanSwap` methods (the `Arc<Mutex<…>>` collector is
/// non-reentrant: a `Drop` re-entering `swap`/`drain_retired`/
/// `retired_overflows` would deadlock on the held lock).
///
/// **RT contract:** non-RT only — `write`/`lock` take locks and
/// `drain` runs destructors.
#[derive(Clone)]
pub struct PlanSwap {
    current: Arc<RwLock<Arc<GraphPlan>>>,
    retired: Arc<Mutex<DropCollector<Arc<GraphPlan>>>>,
}

/// RT-side handle — moved into the device callback. `try_read` +
/// last-good cache, never blocks.
///
/// Retirement: when `active()` displaces a cached plan it is pushed
/// to the `Retirer` (wait-free push only — the destructor runs later,
/// on the collector's drain). `Drop for RtPlan` likewise retires the
/// still-cached plan, so an `RtPlan` dropped on the callback thread
/// never runs a `GraphPlan` destructor.
///
/// Drop order: the `RtPlan` (or the callback owning it) must drop
/// before the last [`PlanSwap`] clone — the collector must outlive
/// the retirer, or `Drop`'s `retire` lands on a dead channel and the
/// plan leaks instead of being collected. `Session`'s field order
/// already pins the in-crate path.
///
/// **RT contract:** `active()` and `drop` are RT-suitable; creating
/// an `RtPlan` is non-RT.
pub struct RtPlan {
    current: Arc<RwLock<Arc<GraphPlan>>>,
    /// The last-good plan the RT side actually drives — `pub(crate)`
    /// so the in-crate tests can pin the borrow [`active`](Self::active)
    /// returns against the slot's own `Arc` (`Arc::as_ptr`).
    pub(crate) cached: Option<Arc<GraphPlan>>,
    retirer: Retirer<Arc<GraphPlan>>,
}

impl PlanSwap {
    /// Creates the pair over an initial plan with the default retire
    /// capacity (small — proportional to swap cadence).
    #[must_use]
    pub fn new(plan: GraphPlan) -> (Self, RtPlan) {
        Self::with_retire_capacity(plan, DEFAULT_RETIRE_CAPACITY)
    }

    /// Creates the pair with a caller-tuned retire-channel capacity.
    /// `capacity` bounds how many plan retirements can queue between
    /// drains before the leak-on-full fallback engages (counted by
    /// [`retired_overflows`](Self::retired_overflows)).
    ///
    /// # Panics
    ///
    /// Panics when `capacity` is zero — a degenerate always-full
    /// channel would leak every retirement.
    #[must_use]
    pub fn with_retire_capacity(plan: GraphPlan, capacity: usize) -> (Self, RtPlan) {
        let (retirer, collector) = retire_channel::<Arc<GraphPlan>>(capacity);
        let current = Arc::new(RwLock::new(Arc::new(plan)));
        (
            Self {
                current: Arc::clone(&current),
                retired: Arc::new(Mutex::new(collector)),
            },
            RtPlan {
                current,
                cached: None,
                retirer,
            },
        )
    }

    /// Publishes a new plan; the RT side picks it up on the next block
    /// (or the block after under contention). Drains the retire
    /// collector first — every non-RT entry point drains.
    pub fn swap(&self, plan: GraphPlan) {
        self.drain_retired();
        *self
            .current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(plan);
    }

    /// Drops every plan retired by the RT side so far, running the
    /// destructors on the calling thread — the explicit drain point
    /// for operation that never calls [`swap`](Self::swap).
    ///
    /// Returns the number of dropped plans.
    ///
    /// **RT contract:** non-RT only — takes the collector lock and
    /// runs destructors, bounded by the queued count.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio::{Graph, GraphPlan, PlanSwap};
    ///
    /// let (swap, mut rt_plan) = PlanSwap::new(GraphPlan::new(Graph::new()));
    /// let _plan = rt_plan.active();
    /// // The plan is still published — nothing was retired, nothing
    /// // leaked.
    /// assert_eq!(swap.drain_retired(), 0);
    /// assert_eq!(swap.retired_overflows(), 0);
    /// ```
    pub fn drain_retired(&self) -> usize {
        self.retired
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain()
    }

    /// Cumulative count of retired plans leaked because the retire
    /// channel was full (`mem::forget` — the leak-on-full policy; the
    /// plans are never dropped on the RT side). Zero is the healthy
    /// steady state; a non-zero or growing value means capacity or
    /// drain cadence is undersized for the real swap rate.
    ///
    /// **RT contract:** non-RT only — monitoring accessor on the
    /// collecting side.
    #[must_use]
    pub fn retired_overflows(&self) -> usize {
        self.retired
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .overflows()
    }
}

impl RtPlan {
    /// Returns the active plan — `try_read`, else the last cached
    /// snapshot. First call before any successful read yields `None`
    /// only if the initial lock is contended.
    ///
    /// The return borrows the `cached` slot, never an `Arc`: the
    /// borrow cannot outlive the next `active()` call, so no
    /// `Arc<GraphPlan>` can be retained on the RT side and dropped
    /// where a last-refcount `GraphPlan` destructor would run on the
    /// callback thread.
    ///
    /// When the read succeeds and displaces a different cached plan,
    /// the old `Arc` is pushed to the retire channel (re-reading the
    /// same `Arc` — `ptr_eq` — does not retire, and a failed
    /// `try_read` leaves `cached` untouched so nothing is displaced).
    ///
    /// **RT contract:** RT-suitable — `try_read`, no allocation beyond
    /// the `Arc` refcount bump; `retire` is a wait-free push.
    pub fn active(&mut self) -> Option<&GraphPlan> {
        if let Ok(guard) = self.current.try_read() {
            let plan = Arc::clone(&guard);
            if let Some(old) = self.cached.replace(Arc::clone(&plan)) {
                // Displaced a genuinely different plan — retire it.
                // On `ptr_eq` (same plan re-read) `old` is a redundant
                // clone: dropping it only decrements the refcount —
                // the destructor cannot run since `plan`/`cached`
                // still reference it.
                if !Arc::ptr_eq(&old, &plan) {
                    self.retirer.retire(old);
                }
            }
        }
        // Both paths answer from `cached` — a successful read just
        // refreshed it, a failed one keeps the last-good.
        self.cached.as_deref()
    }
}

impl Drop for RtPlan {
    // Retires the still-cached plan so its destructor runs on the
    // collector side, never here. `take` leaves `None` — no double
    // retire — and the shared `current` slot is not touched.
    fn drop(&mut self) {
        if let Some(cached) = self.cached.take() {
            self.retirer.retire(cached);
        }
    }
}
