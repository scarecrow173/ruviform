//! Shared counter primitive — a [`Counter`] for the instrumented
//! (writing) side and a [`CounterReader`] for the non-RT observing
//! side — sharing one `Arc<AtomicU64>` cell. This is the promoted
//! shape of the Issue #104 `observability_poc` `Counter`/`CounterReader`
//! scaffold, using `u64` counters per the Issue #121 diagnostics design.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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

    /// RT lane: the operations an RT writer may call on a
    /// pre-constructed counter pair perform no allocation inside the
    /// measurement window. Construction/Clone/Drop stay outside —
    /// they are non-RT per the crate contract.
    #[test]
    fn rt_counter_ops_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let (counter, reader) = counter();
        assert_no_alloc(|| {
            counter.increment();
            let _ = counter.read();
            let _ = counter.reader();
            let _ = reader.read();
        });
        assert_eq!(reader.read(), 1);
    }
}
