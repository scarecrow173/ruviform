//! `NodeStatus` — the node's lock-free fault/observation counters,
//! shared with the non-RT control handle (same pattern as
//! `ruviform_audio::typed::SharedStatus`).

use std::sync::atomic::{AtomicU64, Ordering};

/// Atomic counters the RT node bumps on defined failure paths; the
/// non-RT side snapshots them. `Relaxed` everywhere — counters are
/// observational, never ordering carriers.
#[derive(Debug, Default)]
pub struct NodeStatus {
    undeliverable_spans: AtomicU64,
    endpoint_errors: AtomicU64,
    event_scratch_overflows: AtomicU64,
    control_overflows: AtomicU64,
    report_overflows: AtomicU64,
    vault_overflows: AtomicU64,
    physical_calls: AtomicU64,
}

/// A point-in-time copy of [`NodeStatus`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeStatusSnapshot {
    /// Spans the scheduler emitted as undeliverable (silenced).
    pub undeliverable_spans: u64,
    /// `EndpointError` observations (silence fallback applied).
    pub endpoint_errors: u64,
    /// Event windows that exceeded the per-call scratch.
    pub event_scratch_overflows: u64,
    /// Control messages dropped because the queue was full.
    pub control_overflows: u64,
    /// Reports dropped because the report queue was full.
    pub report_overflows: u64,
    /// Plan installs rejected by a full [`crate::PlanVault`].
    pub vault_overflows: u64,
    /// Physical `process` calls issued.
    pub physical_calls: u64,
}

impl NodeStatus {
    /// **RT-suitable** — `Relaxed` `fetch_add(1)`.
    pub(crate) fn bump(&self, c: Counter) {
        self.field(c).fetch_add(1, Ordering::Relaxed);
    }

    fn field(&self, c: Counter) -> &AtomicU64 {
        match c {
            Counter::UndeliverableSpan => &self.undeliverable_spans,
            Counter::EndpointError => &self.endpoint_errors,
            Counter::EventScratchOverflow => &self.event_scratch_overflows,
            Counter::ControlOverflow => &self.control_overflows,
            Counter::ReportOverflow => &self.report_overflows,
            Counter::VaultOverflow => &self.vault_overflows,
            Counter::PhysicalCall => &self.physical_calls,
        }
    }

    /// Reads every counter — non-RT only.
    #[must_use]
    pub fn snapshot(&self) -> NodeStatusSnapshot {
        let load = |a: &AtomicU64| a.load(Ordering::Relaxed);
        NodeStatusSnapshot {
            undeliverable_spans: load(&self.undeliverable_spans),
            endpoint_errors: load(&self.endpoint_errors),
            event_scratch_overflows: load(&self.event_scratch_overflows),
            control_overflows: load(&self.control_overflows),
            report_overflows: load(&self.report_overflows),
            vault_overflows: load(&self.vault_overflows),
            physical_calls: load(&self.physical_calls),
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum Counter {
    UndeliverableSpan,
    EndpointError,
    EventScratchOverflow,
    ControlOverflow,
    ReportOverflow,
    VaultOverflow,
    PhysicalCall,
}
