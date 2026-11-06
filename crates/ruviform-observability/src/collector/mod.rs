//! Collector — the authoritative non-RT observation path over the
//! OBS-02 registration contract (Issue #289 / OBS-07).
//!
//! One [`Collector`] owns `poll`: a single `Mutex`-serialized pass
//! that reads every registered [`Counter`](crate::Counter)/
//! [`Gauge`](crate::core::Gauge)/[`Histogram`](crate::core::Histogram)
//! reader, applies baseline/delta/rate/discontinuity semantics,
//! drives `Active → Retiring → FinalDrain → Removed`, commits bounded
//! history, and publishes the generation-tagged [`ObservationBatch`]
//! and [`MetricsSnapshot`]. [`CollectorRegistrar`] implements
//! [`ObservationRegistrar`](crate::source::ObservationRegistrar) and
//! [`CollectorHandle`] gives shareable read/shutdown access — neither
//! can poll.
//!
//! Everything here is **non-RT only**: polls, registrations,
//! snapshots, history reads and health all allocate and/or lock.
//! Tracing/logging/subscriber types appear nowhere in this module —
//! the same typed results are consumable with the `logging` feature
//! off and zero external dependencies.
//!
//! See the module-level semantics in [`crate::metrics`] for
//! measurement validity/precision/provenance rules.

mod batch;
mod config;
mod engine;
mod health;
mod history;
mod registry;
mod snapshot;
mod window;

pub use batch::ObservationBatch;
pub use config::{CollectorConfig, CollectorConfigError};
pub use engine::{Collector, CollectorHandle, CollectorRegistrar, PollError, PollRunner};
pub use health::{CollectorHealth, CollectorState};
pub use snapshot::MetricsSnapshot;
pub use window::ObservationWindow;

#[cfg(test)]
mod tests;
