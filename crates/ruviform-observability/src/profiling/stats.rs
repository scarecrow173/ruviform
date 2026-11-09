//! Allocation counter primitives shared by the process-wide statistics
//! and the per-scope TLS counters.
//!
//! Every counter saturates at its type's maximum instead of wrapping.
//! Byte totals count *requested* bytes — `Layout::size()` for `alloc`,
//! `alloc_zeroed`, and `dealloc`, and `new_size` for `realloc` — never
//! resident memory, RSS, or allocator-internal overhead.

use core::cell::Cell;
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// Per-operation counters for one [`GlobalAlloc`] method.
///
/// `bytes` sums the requested size of every call, including failed
/// ones. `dealloc` cannot fail, so its `failed` counter always stays
/// zero.
///
/// [`GlobalAlloc`]: core::alloc::GlobalAlloc
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpStats {
    /// Number of calls observed, including failures.
    pub calls: u64,
    /// Number of calls that produced a usable result.
    pub succeeded: u64,
    /// Number of calls that failed (returned null).
    pub failed: u64,
    /// Sum of requested bytes across all calls (saturating).
    pub bytes: u64,
}

/// Snapshot of allocation statistics.
///
/// This is a point-in-time copy of plain counters; it carries no
/// interior mutability and is cheap to read from any thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationStats {
    /// Counters for `GlobalAlloc::alloc`.
    pub alloc: OpStats,
    /// Counters for `GlobalAlloc::alloc_zeroed`.
    pub alloc_zeroed: OpStats,
    /// Counters for `GlobalAlloc::dealloc`.
    pub dealloc: OpStats,
    /// Counters for `GlobalAlloc::realloc`.
    pub realloc: OpStats,
    /// Live requested bytes: `+size` on a successful `alloc` or
    /// `alloc_zeroed`, `-size` on `dealloc`, and `+(new_size - old_size)`
    /// on a successful `realloc`.
    ///
    /// A scope that frees a block allocated before the scope started
    /// observes a negative value; `i64` represents that honestly
    /// instead of wrapping.
    pub current_bytes: i64,
    /// Maximum non-negative `current_bytes` observed so far.
    pub peak_bytes: u64,
}

/// The four `GlobalAlloc` operations, used to select a counter set.
#[derive(Clone, Copy)]
pub(crate) enum Op {
    /// `GlobalAlloc::alloc`.
    Alloc,
    /// `GlobalAlloc::alloc_zeroed`.
    AllocZeroed,
    /// `GlobalAlloc::dealloc`.
    Dealloc,
    /// `GlobalAlloc::realloc`.
    Realloc,
}

/// Counter cell contract implemented by both the process-wide
/// statistics and each per-scope TLS node, so the recording policy
/// lives in exactly one place.
pub(crate) trait AllocationCounters {
    /// Records one call request: `calls += 1`, `bytes += bytes`.
    fn request(&self, op: Op, bytes: u64);
    /// Records a successful call.
    fn succeeded(&self, op: Op);
    /// Records a failed call.
    fn failed(&self, op: Op);
    /// Adds `delta` to the live-bytes counter and raises the peak to
    /// the new non-negative maximum.
    fn current_add(&self, delta: i64);

    /// Records an `alloc`/`alloc_zeroed` call and its result.
    fn record_alloc(&self, op: Op, size: u64, ok: bool) {
        self.request(op, size);
        if ok {
            self.succeeded(op);
            self.current_add(size_to_i64(size));
        } else {
            self.failed(op);
        }
    }

    /// Records a `dealloc` call. Deallocation cannot fail, so it is
    /// always a success and always decrements live bytes.
    fn record_dealloc(&self, size: u64) {
        self.request(Op::Dealloc, size);
        self.succeeded(Op::Dealloc);
        self.current_add(-size_to_i64(size));
    }

    /// Records a `realloc` call and its result. On success the live
    /// bytes move by `new_size - old_size`; on failure the old block
    /// stays alive and nothing else changes.
    fn record_realloc(&self, old_size: u64, new_size: u64, ok: bool) {
        self.request(Op::Realloc, new_size);
        if ok {
            self.succeeded(Op::Realloc);
            self.current_add(size_to_i64(new_size) - size_to_i64(old_size));
        } else {
            self.failed(Op::Realloc);
        }
    }
}

/// Converts a requested size to `i64` for the signed live-bytes
/// counter. `GlobalAlloc` contracts bound `Layout` sizes to
/// `isize::MAX`, so this only saturates for inputs that already
/// violate that contract; it never wraps.
fn size_to_i64(size: u64) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

/// Converts a possibly negative live-byte count to the non-negative
/// peak counter.
fn non_negative(v: i64) -> u64 {
    u64::try_from(v.max(0)).unwrap_or(0)
}

/// Saturating `+=` on an atomic counter via a CAS loop. Written out
/// instead of `fetch_update`/`try_update` so the code stays within the
/// workspace MSRV.
pub(crate) fn bump_atomic(cell: &AtomicU64, delta: u64) {
    let mut cur = cell.load(Ordering::Relaxed);
    while let Err(actual) = cell.compare_exchange_weak(
        cur,
        cur.saturating_add(delta),
        Ordering::Relaxed,
        Ordering::Relaxed,
    ) {
        cur = actual;
    }
}

/// Saturating `+=` on a `Cell` counter.
pub(crate) fn bump_cell(cell: &Cell<u64>, delta: u64) {
    cell.set(cell.get().saturating_add(delta));
}

/// Adds `delta` to the atomic live-bytes counter and raises the atomic
/// peak to the new non-negative maximum.
pub(crate) fn current_add_atomic(current: &AtomicI64, peak: &AtomicU64, delta: i64) {
    let mut cur = current.load(Ordering::Relaxed);
    let new = loop {
        let new = cur.saturating_add(delta);
        match current.compare_exchange_weak(cur, new, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break new,
            Err(actual) => cur = actual,
        }
    };
    peak.fetch_max(non_negative(new), Ordering::Relaxed);
}

/// Adds `delta` to the `Cell` live-bytes counter and raises the `Cell`
/// peak to the new non-negative maximum.
pub(crate) fn current_add_cell(current: &Cell<i64>, peak: &Cell<u64>, delta: i64) {
    let new = current.get().saturating_add(delta);
    current.set(new);
    peak.set(peak.get().max(non_negative(new)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_to_i64_saturates_instead_of_wrapping() {
        assert_eq!(size_to_i64(64), 64);
        assert_eq!(size_to_i64(u64::MAX), i64::MAX);
    }

    #[test]
    fn atomic_counters_saturate() {
        let cell = AtomicU64::new(u64::MAX - 1);
        bump_atomic(&cell, 5);
        assert_eq!(cell.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn cell_counters_saturate() {
        let cell = Cell::new(u64::MAX - 1);
        bump_cell(&cell, 5);
        assert_eq!(cell.get(), u64::MAX);
    }

    #[test]
    fn current_add_tracks_peak_and_negative_values() {
        let current = AtomicI64::new(0);
        let peak = AtomicU64::new(0);
        current_add_atomic(&current, &peak, 100);
        current_add_atomic(&current, &peak, -30);
        current_add_atomic(&current, &peak, -100);
        assert_eq!(current.load(Ordering::Relaxed), -30);
        assert_eq!(peak.load(Ordering::Relaxed), 100);
    }
}
