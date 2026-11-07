//! Shared histogram primitive — a [`Histogram`] for the instrumented
//! (writing) side and a [`HistogramReader`] for the non-RT observing
//! side — sharing one `Arc<HistogramShared>` cell holding the fixed
//! bucket boundaries and one `AtomicU64` per bucket.
//!
//! Bucket boundaries are fixed at construction (sorted, strictly
//! ascending); `boundaries.len() + 1` buckets cover
//! `(-∞, b0), [b0, b1), …, [b_{n-1}, +∞)`.
//!
//! # Read semantics
//!
//! Each bucket is an independent `AtomicU64`. A [`HistogramReader`]
//! read is a sequence of `Relaxed` loads — **not** a transactionally
//! consistent snapshot: the per-bucket counts observed together may
//! come from different instants, so the sum is not guaranteed to equal
//! the number of `observe` calls at any single instant.
//!
//! # RT contract
//!
//! - [`Histogram::observe`]: demonstrated no-alloc and bounded
//!   (O(log N) `partition_point` over the fixed boundary array plus a
//!   single `Relaxed` `fetch_add`) under the `assert_no_alloc` test
//!   lane, but **not yet declared RT-safe** — the formal declaration
//!   is deferred until measured evidence exists (Issue #277 defers it
//!   to later verification work).
//! - [`histogram`] (construction), `Clone`, `Drop`,
//!   [`HistogramReader::snapshot`]: **non-RT only** — allocation.

use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Error returned when [`histogram`] receives invalid bucket
/// boundaries.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistogramError {
    /// `boundaries` was empty — a one-bucket histogram carries no
    /// distribution information; use a [`counter`](crate::counter)
    /// instead.
    EmptyBoundaries,
    /// `boundaries` was not strictly ascending (unsorted or contains
    /// duplicates), which would leave buckets unreachable.
    UnsortedBoundaries,
}

impl fmt::Display for HistogramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EmptyBoundaries => f.write_str("histogram boundaries must be non-empty"),
            Self::UnsortedBoundaries => {
                f.write_str("histogram boundaries must be strictly ascending")
            }
        }
    }
}

impl Error for HistogramError {}

/// Shared cell behind a [`Histogram`]/[`HistogramReader`] pair.
#[derive(Debug)]
struct HistogramShared {
    /// Fixed, strictly ascending upper boundaries, validated at
    /// construction.
    boundaries: Box<[u64]>,
    /// `boundaries.len() + 1` independent bucket counters.
    buckets: Box<[AtomicU64]>,
}

/// Builds a shared histogram pair for the fixed `boundaries`.
///
/// **Non-RT only** — allocates the shared cell. `boundaries` must be
/// non-empty and strictly ascending.
///
/// # Examples
///
/// ```
/// use ruviform_observability::core::histogram;
///
/// let (histogram, reader) = histogram(&[10, 100]).unwrap();
/// histogram.observe(5);
/// histogram.observe(50);
/// histogram.observe(500);
/// assert_eq!(&*reader.snapshot(), &[1, 1, 1]);
/// ```
pub fn histogram(boundaries: &[u64]) -> Result<(Histogram, HistogramReader), HistogramError> {
    if boundaries.is_empty() {
        return Err(HistogramError::EmptyBoundaries);
    }
    if boundaries.windows(2).any(|w| w[0] >= w[1]) {
        return Err(HistogramError::UnsortedBoundaries);
    }
    let shared = Arc::new(HistogramShared {
        boundaries: boundaries.into(),
        buckets: (0..=boundaries.len()).map(|_| AtomicU64::new(0)).collect(),
    });
    Ok((
        Histogram {
            inner: Arc::clone(&shared),
        },
        HistogramReader { inner: shared },
    ))
}

/// Writing side of a shared observability histogram. `Clone` shares
/// the same cell.
#[derive(Debug, Clone)]
pub struct Histogram {
    inner: Arc<HistogramShared>,
}

impl Histogram {
    /// Records one observation of `value` into its bucket — a bounded
    /// `partition_point` over the fixed boundaries plus a `Relaxed`
    /// `fetch_add`.
    ///
    /// Demonstrated no-alloc and bounded under the `assert_no_alloc`
    /// test lane, but **not yet declared RT-safe** — the formal
    /// declaration is deferred pending measured evidence.
    pub fn observe(&self, value: u64) {
        let idx = self.inner.boundaries.partition_point(|&b| value >= b);
        self.inner.buckets[idx].fetch_add(1, Ordering::Relaxed);
    }

    /// The fixed bucket boundaries configured at construction.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn boundaries(&self) -> &[u64] {
        &self.inner.boundaries
    }

    /// The number of buckets — `boundaries().len() + 1`.
    /// Wait-free, no-alloc, bounded.
    #[must_use]
    pub fn bucket_count(&self) -> usize {
        self.inner.buckets.len()
    }

    /// Derives an additional observing handle on the same cell — an
    /// `Arc` refcount bump, not an allocation. Wait-free, no-alloc,
    /// bounded.
    #[must_use]
    pub fn reader(&self) -> HistogramReader {
        HistogramReader {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Non-RT observing side of a shared histogram — per-bucket `Relaxed`
/// loads only.
///
/// `Clone` shares the same cell (an `Arc` refcount bump on clone).
#[derive(Debug, Clone)]
pub struct HistogramReader {
    inner: Arc<HistogramShared>,
}

impl HistogramReader {
    /// The fixed bucket boundaries configured at construction.
    #[must_use]
    pub fn boundaries(&self) -> &[u64] {
        &self.inner.boundaries
    }

    /// The number of buckets — `boundaries().len() + 1`.
    #[must_use]
    pub fn bucket_count(&self) -> usize {
        self.inner.buckets.len()
    }

    /// Reads every bucket into a fresh array — one `Relaxed` load per
    /// bucket.
    ///
    /// **Non-RT only** — allocates. Not a transactionally consistent
    /// snapshot: per-bucket counts may come from different instants.
    #[must_use]
    pub fn snapshot(&self) -> Box<[u64]> {
        self.inner
            .buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect()
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
    fn histogram_rejects_empty_and_unsorted_boundaries() {
        assert_eq!(histogram(&[]).unwrap_err(), HistogramError::EmptyBoundaries);
        assert_eq!(
            histogram(&[5, 5]).unwrap_err(),
            HistogramError::UnsortedBoundaries
        );
        assert_eq!(
            histogram(&[100, 10]).unwrap_err(),
            HistogramError::UnsortedBoundaries
        );
    }

    #[test]
    fn observe_lands_in_the_expected_buckets() {
        let (histogram, reader) = histogram(&[10, 100]).unwrap();
        assert_eq!(reader.bucket_count(), 3);
        histogram.observe(5); // < 10
        histogram.observe(10); // == boundary → [10, 100)
        histogram.observe(99);
        histogram.observe(500); // >= 100
        assert_eq!(&*reader.snapshot(), &[1, 2, 1]);
    }

    #[test]
    fn histogram_clone_and_derived_readers_share_the_cell() {
        let (histogram, reader) = histogram(&[1]).unwrap();
        let other_writer = histogram.clone();
        let extra = histogram.reader();
        other_writer.observe(0);
        histogram.observe(7);
        assert_eq!(&*reader.snapshot(), &[1, 1]);
        assert_eq!(&*extra.snapshot(), &[1, 1]);
    }

    #[test]
    fn pair_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Histogram>();
        assert_send_sync::<HistogramReader>();
    }

    /// RT lane: `observe` (and the metadata reads) perform no
    /// allocation inside the measurement window — the no-alloc half
    /// of the `observe` RT-suitability evidence. `snapshot`
    /// allocates and is deliberately excluded.
    #[test]
    fn rt_histogram_observe_allocates_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let (histogram, reader) = histogram(&[10, 100]).unwrap();
        assert_no_alloc(|| {
            histogram.observe(5);
            histogram.observe(500);
            let _ = histogram.boundaries();
            let _ = histogram.bucket_count();
            let _ = histogram.reader();
            let _ = reader.boundaries();
            let _ = reader.bucket_count();
        });
        assert_eq!(&*reader.snapshot(), &[1, 0, 1]);
    }
}
