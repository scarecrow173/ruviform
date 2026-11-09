//! RT timing — measured-interval observation for real-time code
//! (Issue #290 / OBS-08 RT Timing slice).
//!
//! [`timing_bridge`] prepares a bounded SPSC path (reusing
//! [`crate::rt_bridge`]) plus a fixed-bucket duration
//! [`Histogram`](crate::core::Histogram) (reusing
//! [`crate::core::histogram`]) and returns two endpoints:
//!
//! - [`TimingRecorder`] — the **RT** side. [`TimingRecorder::record`]
//!   builds a fixed-size [`TimingRecord`] (an OBS-05
//!   [`RecordHeader`](crate::record::RecordHeader) + `end_tick` +
//!   `budget_ticks`) and offers it to the bridge: no allocation, no
//!   lock, no panic path, no tracing/subscriber call, no I/O. Full
//!   rings drop the record and count the loss — never block, never
//!   fabricate.
//! - [`TimingSink`] — the **non-RT** side. [`TimingSink::poll`]
//!   drains queued records into a [`TimingWindow`];
//!   [`TimingSink::snapshot`] reports cumulative exact statistics;
//!   [`TimingSink::histogram_reader`] exposes the duration
//!   distribution to OBS-02/OBS-07 registration as
//!   `MetricValue::Histogram`.
//!
//! # Semantics
//!
//! - **Clock**: [`TimingClock`] fixes domain/timebase/provenance/epoch
//!   at preparation; every record's
//!   [`ClockContext`](crate::record::ClockContext) carries it with
//!   `position = start_tick`. One bridge
//!   = one clock domain = one distribution; foreign-clock records are
//!   unreachable by construction. `end_tick < start_tick` is a clock
//!   anomaly — counted, never interpreted as a duration.
//! - **Budget**: `budget_ticks == 0` asserts no budget (utilization
//!   fields stay `None`). Otherwise `duration * 1000 / budget_ticks`
//!   (permille) is the *measured-interval budget utilization* and
//!   `duration > budget` counts `over_budget`. This is **not** a
//!   deadline overrun, XRUN, or device-scheduling verdict.
//! - **Loss**: `TimingWindow::dropped` carries positional
//!   `dropped_before` evidence; `TimingSnapshot::dropped_total` is the
//!   authoritative bridge counter. A lossy window's distribution
//!   fields describe surviving samples only.
//! - **Exact vs estimated**: exact scalar statistics cover every
//!   sample; quantiles are [`QuantileEstimate`] bucket-resolution
//!   intervals — never presented as exact percentiles.
//! - **Units**: all durations are in the record clock's
//!   [`TimeBase`](ruviform_media_core::TimeBase) ticks. This module
//!   performs no tick→time conversion (`ClockAnchor` is the domain's
//!   responsibility).
//!
//! Construction ([`timing_bridge`]) is non-RT only; endpoint `Drop`s
//! are non-RT — never drop a recorder or sink on an RT thread.

mod bridge;
mod record;
mod sink;

pub use bridge::{TimingRecorder, timing_bridge};
pub use record::{TIMING_SCHEMA_VERSION, TimingClock, TimingRecord, TimingSample};
pub use sink::{QuantileEstimate, TimingSink, TimingSnapshot, TimingWindow};

#[cfg(test)]
mod tests;
