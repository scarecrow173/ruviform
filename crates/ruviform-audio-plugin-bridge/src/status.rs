//! `NodeStatus` — the node's lock-free fault/observation counters,
//! shared with the non-RT control handle (same pattern as
//! `ruviform_audio::typed::SharedStatus`).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::input::InputStageCounters;

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
    /// `NoteOff`s emitted by the delivered-note ledger on transport
    /// discontinuity.
    notes_released: AtomicU64,
    /// Note-on re-fires the chase emitted at span starts.
    notes_chased: AtomicU64,
    /// Keys the chase could not re-fire (scratch/tracker bound).
    chase_overflows: AtomicU64,
    /// Delivered-note ledger force-releases on overflow.
    ledger_overflows: AtomicU64,
    /// Mirrored [`InputStage`] counters — stored whole, never
    /// incremented (the stage is the authority).
    input_mapped: AtomicU64,
    input_delivered: AtomicU64,
    input_late: AtomicU64,
    input_rejected: AtomicU64,
    input_dropped_backend: AtomicU64,
    input_unsupported_backend: AtomicU64,
    input_dropped_capacity: AtomicU64,
    input_released: AtomicU64,
    input_ledger_overflows: AtomicU64,
    stale_epoch_controls: AtomicU64,
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
    /// `NoteOff`s emitted on transport discontinuity by the
    /// delivered-note ledger.
    pub notes_released: u64,
    /// Note-on re-fires emitted by span chase.
    pub notes_chased: u64,
    /// Keys the chase could not re-fire.
    pub chase_overflows: u64,
    /// Delivered-note ledger force-releases.
    pub ledger_overflows: u64,
    /// `SetClock` controls ignored for naming a foreign clock epoch.
    pub stale_epoch_controls: u64,
    /// Live-input stage counters mirrored at the last callback —
    /// zeros when the node has no input leg.
    pub input: InputStageCounters,
}

impl NodeStatus {
    /// **RT-suitable** — `Relaxed` `fetch_add(1)`.
    pub(crate) fn bump(&self, c: Counter) {
        self.field(c).fetch_add(1, Ordering::Relaxed);
    }

    /// **RT-suitable** — `Relaxed` `fetch_add(n)`.
    pub(crate) fn bump_by(&self, c: Counter, n: u64) {
        if n > 0 {
            self.field(c).fetch_add(n, Ordering::Relaxed);
        }
    }

    /// Mirrors the stage's counters into the node status surface —
    /// absolute stores, not deltas (the stage is the authority).
    ///
    /// **RT-suitable** — `Relaxed` stores.
    pub(crate) fn set_input(&self, c: InputStageCounters) {
        self.input_mapped.store(c.mapped, Ordering::Relaxed);
        self.input_delivered.store(c.delivered, Ordering::Relaxed);
        self.input_late.store(c.late, Ordering::Relaxed);
        self.input_rejected.store(c.rejected, Ordering::Relaxed);
        self.input_dropped_backend
            .store(c.dropped_backend, Ordering::Relaxed);
        self.input_unsupported_backend
            .store(c.unsupported_backend, Ordering::Relaxed);
        self.input_dropped_capacity
            .store(c.dropped_capacity, Ordering::Relaxed);
        self.input_released.store(c.released, Ordering::Relaxed);
        self.input_ledger_overflows
            .store(c.ledger_overflows, Ordering::Relaxed);
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
            Counter::NotesReleased => &self.notes_released,
            Counter::NotesChased => &self.notes_chased,
            Counter::ChaseOverflow => &self.chase_overflows,
            Counter::LedgerOverflow => &self.ledger_overflows,
            Counter::StaleEpochControl => &self.stale_epoch_controls,
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
            notes_released: load(&self.notes_released),
            notes_chased: load(&self.notes_chased),
            chase_overflows: load(&self.chase_overflows),
            ledger_overflows: load(&self.ledger_overflows),
            stale_epoch_controls: load(&self.stale_epoch_controls),
            input: InputStageCounters {
                mapped: load(&self.input_mapped),
                delivered: load(&self.input_delivered),
                late: load(&self.input_late),
                rejected: load(&self.input_rejected),
                dropped_backend: load(&self.input_dropped_backend),
                unsupported_backend: load(&self.input_unsupported_backend),
                dropped_capacity: load(&self.input_dropped_capacity),
                released: load(&self.input_released),
                ledger_overflows: load(&self.input_ledger_overflows),
            },
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
    NotesReleased,
    NotesChased,
    ChaseOverflow,
    LedgerOverflow,
    StaleEpochControl,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_input_mirrors_whole_counters_including_unsupported() {
        let status = NodeStatus::default();
        let counters = InputStageCounters {
            mapped: 1,
            delivered: 2,
            late: 3,
            rejected: 4,
            dropped_backend: 5,
            unsupported_backend: 6,
            dropped_capacity: 7,
            released: 8,
            ledger_overflows: 9,
        };
        status.set_input(counters);
        // Absolute stores, not deltas — the stage is the authority.
        status.set_input(counters);
        assert_eq!(status.snapshot().input, counters);
    }
}
