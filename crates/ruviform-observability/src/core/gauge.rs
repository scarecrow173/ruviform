//! Shared gauge primitive — a [`Gauge`] for the instrumented (writing)
//! side and a [`GaugeReader`] for the non-RT observing side — sharing
//! one `Arc<AtomicU64>` cell, mirroring the [`counter`](crate::counter)
//! pair shape.
//!
//! Unlike [`Counter`](crate::Counter), a gauge is a **state value**: it
//! may increase *and* decrease, so monotonic non-decreasing observation
//! is explicitly not promised. The only safety claim made here is
//! single-`AtomicU64` atomicity — floating-point payloads and
//! multi-value transactional consistency are out of scope (callers
//! needing correlated values must define that invariant themselves).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Builds a shared gauge pair — a [`Gauge`] for the writing
/// (instrumented) side and a [`GaugeReader`] for the non-RT observing
/// side — initialized to `initial`.
///
/// **Non-RT only** — allocates the shared `Arc` cell.
///
/// # Examples
///
/// ```
/// use ruviform_observability::core::gauge;
///
/// let (gauge, reader) = gauge(0);
/// gauge.set(7);
/// assert_eq!(reader.read(), 7);
/// gauge.set(3); // gauges may decrease — no monotonicity promise
/// assert_eq!(reader.read(), 3);
/// ```
#[must_use]
pub fn gauge(initial: u64) -> (Gauge, GaugeReader) {
    let shared = Arc::new(AtomicU64::new(initial));
    (
        Gauge {
            inner: Arc::clone(&shared),
        },
        GaugeReader { inner: shared },
    )
}

/// Writing side of a shared observability gauge. `Clone` shares the
/// same cell, so additional writers can be handed to other
/// instrumented sites.
///
/// `set`, `read` and `reader` are wait-free, no-alloc and bounded —
/// the operations an RT writer may call on a pre-constructed gauge.
#[derive(Debug, Clone)]
pub struct Gauge {
    inner: Arc<AtomicU64>,
}

impl Gauge {
    /// Stores `value` — a single `Relaxed` `store`.
    /// Wait-free, no-alloc, bounded.
    pub fn set(&self, value: u64) {
        self.inner.store(value, Ordering::Relaxed);
    }

    /// Reads the current value — a `Relaxed` load.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn read(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }

    /// Derives an additional observing handle on the same cell — an
    /// `Arc` refcount bump, not an allocation. Wait-free, no-alloc,
    /// bounded.
    #[must_use]
    pub fn reader(&self) -> GaugeReader {
        GaugeReader {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Non-RT observing side of a shared gauge — `Relaxed` load only.
///
/// `Clone` shares the same cell (an `Arc` refcount bump on clone).
#[derive(Debug, Clone)]
pub struct GaugeReader {
    inner: Arc<AtomicU64>,
}

impl GaugeReader {
    /// Reads the current value — a `Relaxed` load.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn read(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }

    /// The raw shared-cell address — used by the collector to reject
    /// re-registration of an already-registered cell
    /// (`SourceReused`). Valid only while the cell is alive, which the
    /// registry guarantees by holding this `Arc` for the claim's
    /// duration.
    pub(crate) fn shared_cell_ptr(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_pair_shares_one_cell() {
        let (gauge, reader) = gauge(0);
        assert_eq!(gauge.read(), 0);
        assert_eq!(reader.read(), 0);
        gauge.set(7);
        assert_eq!(gauge.read(), 7);
        assert_eq!(reader.read(), 7);
    }

    #[test]
    fn gauge_is_not_monotonic() {
        let (gauge, reader) = gauge(7);
        gauge.set(3);
        assert_eq!(reader.read(), 3);
    }

    #[test]
    fn gauge_clone_and_derived_readers_share_the_cell() {
        let (gauge, reader) = gauge(0);
        let other_writer = gauge.clone();
        let extra = gauge.reader();
        other_writer.set(42);
        assert_eq!(reader.read(), 42);
        assert_eq!(extra.read(), 42);
    }

    #[test]
    fn pair_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Gauge>();
        assert_send_sync::<GaugeReader>();
    }

    /// RT lane: `set`/`read`/`reader`/`GaugeReader::read` on a
    /// pre-constructed gauge pair perform no allocation inside the
    /// measurement window.
    #[test]
    fn rt_gauge_ops_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let (gauge, reader) = gauge(0);
        assert_no_alloc(|| {
            gauge.set(7);
            let _ = gauge.read();
            let _ = gauge.reader();
            let _ = reader.read();
        });
        assert_eq!(reader.read(), 7);
    }
}
