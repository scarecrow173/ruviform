//! `PlanVault` — keeps one `Arc` clone per installed transport plan
//! alive for the node's whole lifetime so no RT-side apply, reject or
//! `device_restart` drop can ever run a plan destructor.
//!
//! Plan installs are graph-swap-rate, not per-block: a bounded
//! hold-forever vault is the honest RT-tradeoff. When the node is
//! retired (non-RT via `GraphRuntime::collect`), the vault and every
//! plan drop on the non-RT side.

use std::sync::Arc;

use ruviform_media_core::TransportPlan;
use ruviform_plugin_core::EventKind;

/// Bounded keep-alive for installed plans. `install` clones before the
/// scheduler sees the plan; overflow rejects with `false` (the caller
/// reports `VaultExceeded` and skips the install — a defined rejection,
/// not a silent leak).
#[derive(Debug)]
pub struct PlanVault {
    plans: Vec<Arc<TransportPlan<EventKind>>>,
}

impl PlanVault {
    /// A vault holding at most `capacity` plan refs.
    ///
    /// **RT contract:** non-RT — allocates the store.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            plans: Vec::with_capacity(capacity),
        }
    }

    /// Keeps `plan` alive for the node's lifetime — `false` when full.
    ///
    /// **RT contract:** RT-suitable — `push` within `capacity` never
    /// re-allocates; the reject path drops only the *fresh clone*
    /// (refcount decrement, never a destructor — some other ref always
    /// exists because the caller just cloned).
    pub fn install(&mut self, plan: Arc<TransportPlan<EventKind>>) -> bool {
        if self.plans.len() < self.plans.capacity() {
            self.plans.push(plan);
            true
        } else {
            false
        }
    }

    /// Number of held plans — observability only.
    #[must_use]
    pub fn len(&self) -> usize {
        self.plans.len()
    }

    /// `true` when no plans are held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }
}
