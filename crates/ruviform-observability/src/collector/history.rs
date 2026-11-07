//! Bounded measurement history — a single process-local FIFO ring of
//! [`Measurement`]s, capped by both sample count and an approximate
//! byte budget.
//!
//! History is non-persistent by design: it exists so pull consumers
//! can look backwards after the fact, and every eviction or refused
//! insert is counted in [`CollectorHealth`]. A single global ring is
//! the deliberate trade-off — a chatty series can dominate it, which
//! is diagnosed through `history_evicted_total` and per-series reads
//! rather than prevented by more bookkeeping.

use std::collections::VecDeque;

use crate::metrics::{Measurement, MetricValue};

/// Approximate byte cost of one history entry — the fixed
/// [`Measurement`] footprint plus the histogram bucket payload when
/// present.
fn entry_cost(m: &Measurement) -> usize {
    let dynamic = match &m.value {
        Some(MetricValue::Histogram { buckets, .. }) => buckets.len() * 8,
        _ => 0,
    };
    size_of::<Measurement>() + dynamic
}

/// The bounded ring — collector-internal.
#[derive(Debug)]
pub(crate) struct History {
    ring: VecDeque<Measurement>,
    bytes: usize,
    max_samples: usize,
    budget_bytes: usize,
}

/// What happened when a measurement was offered to history.
#[derive(Debug, Default)]
pub(crate) struct PushOutcome {
    /// Entries evicted to make room.
    pub evicted: usize,
    /// The entry was refused entirely (it alone exceeds the byte
    /// budget) — still delivered in the batch/snapshot.
    pub oversized: bool,
}

impl History {
    /// Creates a ring; `0` on either bound disables retention.
    pub(crate) fn new(max_samples: usize, budget_bytes: usize) -> Self {
        Self {
            ring: VecDeque::new(),
            bytes: 0,
            max_samples,
            budget_bytes,
        }
    }

    /// Whether history retention is enabled.
    pub(crate) fn enabled(&self) -> bool {
        self.max_samples > 0 && self.budget_bytes > 0
    }

    /// Commits `m` — evicts oldest entries until within both bounds,
    /// or refuses the entry when it alone exceeds the byte budget.
    pub(crate) fn push(&mut self, m: &Measurement) -> PushOutcome {
        let mut outcome = PushOutcome::default();
        if !self.enabled() {
            return outcome;
        }
        let cost = entry_cost(m);
        if cost > self.budget_bytes {
            outcome.oversized = true;
            return outcome;
        }
        while self.ring.len() >= self.max_samples || self.bytes + cost > self.budget_bytes {
            match self.ring.pop_front() {
                Some(old) => {
                    self.bytes = self.bytes.saturating_sub(entry_cost(&old));
                    outcome.evicted += 1;
                }
                None => break,
            }
        }
        self.bytes += cost;
        self.ring.push_back(m.clone());
        outcome
    }

    /// Current entry count.
    pub(crate) fn len(&self) -> usize {
        self.ring.len()
    }

    /// Current approximate bytes.
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    /// Chronological clone of everything retained.
    pub(crate) fn all(&self) -> Vec<Measurement> {
        self.ring.iter().cloned().collect()
    }

    /// Chronological clone of one registration's retained
    /// measurements.
    pub(crate) fn series(&self, identity: crate::source::SourceIdentity) -> Vec<Measurement> {
        self.ring
            .iter()
            .filter(|m| m.identity == identity)
            .cloned()
            .collect()
    }
}
