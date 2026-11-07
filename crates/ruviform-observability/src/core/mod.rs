//! Lightweight observation primitives — shared writer/reader pairs
//! over `Arc<Atomic*>` cells.
//!
//! | Operation | RT suitability |
//! |---|---|
//! | [`Counter::increment`], [`Counter::read`], [`Counter::reader`], [`CounterReader::read`] | wait-free, no-alloc, bounded |
//! | [`Gauge::set`], [`Gauge::read`], [`Gauge::reader`], [`GaugeReader::read`] | wait-free, no-alloc, bounded |
//! | [`Histogram::observe`] | no-alloc/bounded under `assert_no_alloc` lane; **not yet declared RT-safe** |
//! | [`Histogram::boundaries`], [`Histogram::bucket_count`], [`Histogram::reader`], [`HistogramReader::boundaries`], [`HistogramReader::bucket_count`] | wait-free, no-alloc, bounded |
//! | [`counter`], [`gauge`], [`histogram`], `Clone`, `Drop`, [`HistogramReader::snapshot`] | **non-RT only** (allocation) |

mod counter;
mod gauge;
mod histogram;

#[cfg(test)]
pub(crate) use counter::counter_reader_on_cell;
pub use counter::{Counter, CounterReader, counter};
pub use gauge::{Gauge, GaugeReader, gauge};
pub use histogram::{Histogram, HistogramError, HistogramReader, histogram};
