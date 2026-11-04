//! Shared observability primitives — the first concrete consumer surface
//! of the `foundation-observability-minimal-api` sketch (Issue #101,
//! triggered into existence by Issue #121's L1 diagnostics).
//!
//! The crate currently provides exactly one primitive: the shared
//! [`counter`] pair — a [`Counter`] for the instrumented (writing) side
//! and a [`CounterReader`] for the non-RT observing side — sharing one
//! `Arc<AtomicU64>` cell. This is the promoted shape of the Issue #104
//! `observability_poc` `Counter`/`CounterReader` scaffold, using `u64`
//! counters per the Issue #121 diagnostics design.
//!
//! # RT contract
//!
//! - [`Counter::increment`], [`Counter::read`], [`Counter::reader`],
//!   [`CounterReader::read`]: wait-free, no-alloc, bounded — `Relaxed`
//!   atomic ops / an `Arc` refcount bump only. These are the methods an
//!   RT writer may call.
//! - [`counter`] (construction): **non-RT only** — allocates the shared
//!   `Arc` cell. `Clone` and `Drop` on either handle are likewise
//!   refcount ops (the final drop frees the cell; callers must not hold
//!   a counter pair through an RT section boundary).
//!
//! # Read semantics
//!
//! Each `read` is an independent `Relaxed` load. Reading several
//! counters (or several readers) sequentially is **not** a
//! transactionally consistent snapshot — values observed together may
//! come from different instants. Callers needing correlated values must
//! define that invariant themselves; this crate only guarantees each
//! individual counter is monotonically non-decreasing.
//!
//! `u64` wraps modulo 2^64 — at ~5.8×10⁸ increments/second that is
//! unreachable in any realistic configuration, so no saturation policy
//! is provided.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Generic host-owned logging adapter on the standard `tracing` stack —
/// Issue #276 (OBS-01). Only available with the `logging` feature, which is
/// off by default so the counter pair keeps zero external dependencies.
#[cfg(feature = "logging")]
pub mod logging;

/// Builds a shared counter pair — a [`Counter`] for the writing
/// (instrumented) side and a [`CounterReader`] for the non-RT
/// observing side.
///
/// **Non-RT only** — allocates the shared `Arc` cell.
///
/// # Examples
///
/// ```
/// use ruviform_observability::counter;
///
/// let (counter, reader) = counter();
/// counter.increment();
/// assert_eq!(reader.read(), 1);
/// ```
#[must_use]
pub fn counter() -> (Counter, CounterReader) {
    let shared = Arc::new(AtomicU64::new(0));
    (
        Counter {
            inner: Arc::clone(&shared),
        },
        CounterReader { inner: shared },
    )
}

/// Writing side of a shared observability counter. `Clone` shares the
/// same cell, so additional writers can be handed to other instrumented
/// sites (e.g. an `RtLeg` and a reader-side enqueue path both writing
/// into the same instance's counters).
///
/// Every method is wait-free, no-alloc and bounded.
#[derive(Debug, Clone)]
pub struct Counter {
    inner: Arc<AtomicU64>,
}

impl Counter {
    /// Increments the counter — a single `Relaxed` `fetch_add`.
    /// Wait-free, no-alloc, bounded.
    pub fn increment(&self) {
        self.inner.fetch_add(1, Ordering::Relaxed);
    }

    /// Reads the current count — a `Relaxed` load.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn read(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }

    /// Derives an additional observing handle on the same cell — an
    /// `Arc` refcount bump, not an allocation. Wait-free, no-alloc,
    /// bounded.
    #[must_use]
    pub fn reader(&self) -> CounterReader {
        CounterReader {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Non-RT observing side of a shared counter — `Relaxed` load only.
///
/// `Clone` shares the same cell (an `Arc` refcount bump on clone).
#[derive(Debug, Clone)]
pub struct CounterReader {
    inner: Arc<AtomicU64>,
}

impl CounterReader {
    /// Reads the current count — a `Relaxed` load.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn read(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_pair_shares_one_cell() {
        let (counter, reader) = counter();
        assert_eq!(counter.read(), 0);
        assert_eq!(reader.read(), 0);
        counter.increment();
        counter.increment();
        assert_eq!(counter.read(), 2);
        assert_eq!(reader.read(), 2);
    }

    #[test]
    fn counter_clone_shares_the_cell() {
        let (counter, reader) = counter();
        let other_writer = counter.clone();
        other_writer.increment();
        assert_eq!(reader.read(), 1);
    }

    #[test]
    fn reader_clone_and_derived_readers_share_the_cell() {
        let (counter, reader) = counter();
        let extra = counter.reader();
        let cloned = reader.clone();
        counter.increment();
        assert_eq!(extra.read(), 1);
        assert_eq!(cloned.read(), 1);
    }

    #[test]
    fn pair_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Counter>();
        assert_send_sync::<CounterReader>();
    }
}
