//! `CountingAllocator` — a `GlobalAlloc` wrapper that counts
//! allocation events.
//!
//! The wrapper delegates every `GlobalAlloc` method to an inner
//! allocator `A` and records call counts, success/failure counts,
//! requested bytes, and live/peak requested bytes — both in
//! process-wide atomics ([`CountingAllocator::stats`]) and in every
//! active [`AllocationScope`] on the calling thread.
//!
//! The instrumentation itself never allocates, never panics, never
//! locks, and never calls into tracing or subscribers; the wrapped
//! allocator's own characteristics remain its own responsibility.
//!
//! This type only *provides* an allocator — installing it as
//! `#[global_allocator]` is the consuming binary's or test harness's
//! decision, never the library's.
//!
//! # Example
//!
//! ```
//! use core::alloc::{GlobalAlloc, Layout};
//! use std::alloc::System;
//!
//! use ruviform_observability::profiling::CountingAllocator;
//!
//! let alloc = CountingAllocator::new(System);
//! let layout = Layout::from_size_align(32, 8).unwrap();
//! // SAFETY: `layout` is a valid non-zero-size layout and `alloc`
//! // forwards the `GlobalAlloc` contract to `System` unchanged.
//! let ptr = unsafe { alloc.alloc(layout) };
//! assert!(!ptr.is_null());
//! // SAFETY: `ptr` was returned by `alloc` for `layout` and is freed
//! // exactly once here.
//! unsafe { alloc.dealloc(ptr, layout) };
//!
//! let stats = alloc.stats();
//! assert_eq!(stats.alloc.succeeded, 1);
//! assert_eq!(stats.dealloc.succeeded, 1);
//! assert_eq!(stats.current_bytes, 0);
//! ```

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use super::scope;
use super::stats::{
    AllocationCounters, AllocationStats, Op, OpStats, bump_atomic, current_add_atomic,
};

/// Process-wide allocation counters stored in atomics.
///
/// Every field is a plain saturating counter; a snapshot is taken by
/// loading each atomic once. Slightly torn snapshots across concurrent
/// allocators are acceptable for a development probe — per-op counts
/// remain exact because each event updates one op's counters in a
/// fixed order.
#[derive(Debug)]
struct GlobalStats {
    alloc: AtomicOpCounters,
    alloc_zeroed: AtomicOpCounters,
    dealloc: AtomicOpCounters,
    realloc: AtomicOpCounters,
    current_bytes: AtomicI64,
    peak_bytes: AtomicU64,
}

impl GlobalStats {
    const fn new() -> Self {
        Self {
            alloc: AtomicOpCounters::new(),
            alloc_zeroed: AtomicOpCounters::new(),
            dealloc: AtomicOpCounters::new(),
            realloc: AtomicOpCounters::new(),
            current_bytes: AtomicI64::new(0),
            peak_bytes: AtomicU64::new(0),
        }
    }

    fn op_counters(&self, op: Op) -> &AtomicOpCounters {
        match op {
            Op::Alloc => &self.alloc,
            Op::AllocZeroed => &self.alloc_zeroed,
            Op::Dealloc => &self.dealloc,
            Op::Realloc => &self.realloc,
        }
    }

    fn snapshot(&self) -> AllocationStats {
        AllocationStats {
            alloc: self.alloc.snapshot(),
            alloc_zeroed: self.alloc_zeroed.snapshot(),
            dealloc: self.dealloc.snapshot(),
            realloc: self.realloc.snapshot(),
            current_bytes: self.current_bytes.load(Ordering::Relaxed),
            peak_bytes: self.peak_bytes.load(Ordering::Relaxed),
        }
    }
}

impl AllocationCounters for GlobalStats {
    fn request(&self, op: Op, bytes: u64) {
        let counters = self.op_counters(op);
        bump_atomic(&counters.calls, 1);
        bump_atomic(&counters.bytes, bytes);
    }

    fn succeeded(&self, op: Op) {
        bump_atomic(&self.op_counters(op).succeeded, 1);
    }

    fn failed(&self, op: Op) {
        bump_atomic(&self.op_counters(op).failed, 1);
    }

    fn current_add(&self, delta: i64) {
        current_add_atomic(&self.current_bytes, &self.peak_bytes, delta);
    }
}

/// Atomic per-operation counters.
#[derive(Debug)]
struct AtomicOpCounters {
    calls: AtomicU64,
    succeeded: AtomicU64,
    failed: AtomicU64,
    bytes: AtomicU64,
}

impl AtomicOpCounters {
    const fn new() -> Self {
        Self {
            calls: AtomicU64::new(0),
            succeeded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> OpStats {
        OpStats {
            calls: self.calls.load(Ordering::Relaxed),
            succeeded: self.succeeded.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
        }
    }
}

/// A [`GlobalAlloc`] wrapper that counts allocation events.
///
/// `CountingAllocator<A>` forwards all four `GlobalAlloc` methods to
/// the inner allocator `A` unchanged and records, per operation:
/// call count, success/failure count, requested bytes, and the live
/// and peak requested-byte totals. `realloc` is counted as a single
/// event — it is never decomposed into an alloc plus a dealloc — and
/// its bytes contribution is `new_size`.
///
/// Statistics are available process-wide via [`Self::stats`] (all
/// threads that use this instance) and per calling thread via
/// [`AllocationScope`](super::scope::AllocationScope). Both count
/// *requested* bytes only; neither claims RSS, heap occupancy, or
/// exact leak counts.
///
/// `CountingAllocator` does not install itself anywhere: the host
/// binary or test harness declares it as `#[global_allocator]`.
/// `CountingAllocator<System>` is a drop-in replacement for `System`
/// in tests and examples.
///
/// The counting path is allocation-free, panic-free, lock-free, and
/// recursion-free; whether the wrapped allocator `A` shares those
/// properties is `A`'s own contract.
#[derive(Debug)]
pub struct CountingAllocator<A: GlobalAlloc> {
    inner: A,
    stats: GlobalStats,
}

impl<A: GlobalAlloc> CountingAllocator<A> {
    /// Wraps `inner`, starting all counters at zero.
    ///
    /// `const`-constructible so it can back a `static` in a test or
    /// example binary that installs it as `#[global_allocator]`.
    #[must_use]
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            stats: GlobalStats::new(),
        }
    }

    /// Returns the wrapped allocator.
    #[must_use]
    pub const fn inner(&self) -> &A {
        &self.inner
    }

    /// Returns a snapshot of the process-wide counters accumulated so
    /// far, across every thread that used this instance.
    ///
    /// This is a handful of atomic loads: no allocation, no lock, no
    /// panic.
    #[must_use]
    pub fn stats(&self) -> AllocationStats {
        self.stats.snapshot()
    }
}

// Safety: every method forwards the caller's `GlobalAlloc` contract —
// layout validity, pointer provenance, and size/alignment invariants —
// to `inner` unchanged. The only added behavior is counter updates,
// which cannot fail, allocate, or unwind.
#[expect(unsafe_code)]
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size() as u64;
        // SAFETY: `layout` satisfies the `GlobalAlloc` contract because
        // the caller of this method guarantees it; forwarding it to
        // `inner` preserves every invariant `inner` requires.
        let ptr = unsafe { self.inner.alloc(layout) };
        let ok = !ptr.is_null();
        self.stats.record_alloc(Op::Alloc, size, ok);
        scope::for_each_active_scope(|node| node.record_alloc(Op::Alloc, size, ok));
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size() as u64;
        // SAFETY: same forwarding argument as `alloc`.
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        let ok = !ptr.is_null();
        self.stats.record_alloc(Op::AllocZeroed, size, ok);
        scope::for_each_active_scope(|node| {
            node.record_alloc(Op::AllocZeroed, size, ok);
        });
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size() as u64;
        // SAFETY: `ptr`/`layout` satisfy the `GlobalAlloc` contract
        // because the caller guarantees they describe a live
        // allocation made through this allocator; forwarding preserves
        // every invariant `inner` requires.
        unsafe { self.inner.dealloc(ptr, layout) };
        self.stats.record_dealloc(size);
        scope::for_each_active_scope(|node| node.record_dealloc(size));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size() as u64;
        let new_size_u64 = new_size as u64;
        // SAFETY: `ptr`/`layout`/`new_size` satisfy the `GlobalAlloc`
        // contract because the caller guarantees them; forwarding
        // preserves every invariant `inner` requires.
        let out = unsafe { self.inner.realloc(ptr, layout, new_size) };
        let ok = !out.is_null();
        self.stats.record_realloc(old_size, new_size_u64, ok);
        scope::for_each_active_scope(|node| {
            node.record_realloc(old_size, new_size_u64, ok);
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    use std::alloc::System;

    use super::*;
    use crate::profiling::testutil::{
        ProbeAlloc, call_alloc, call_alloc_zeroed, call_dealloc, call_realloc,
    };

    fn layout(size: usize, align: usize) -> Layout {
        Layout::from_size_align(size, align).unwrap()
    }

    #[test]
    fn alloc_delegates_to_inner_and_counts_success() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let ptr = call_alloc(&alloc, layout(64, 8));
        assert!(!ptr.is_null());
        assert_eq!(alloc.inner().alloc_calls.get(), 1);

        let stats = alloc.stats();
        assert_eq!(
            stats.alloc,
            OpStats {
                calls: 1,
                succeeded: 1,
                failed: 0,
                bytes: 64,
            }
        );
        assert_eq!(stats.alloc_zeroed.calls, 0);
        assert_eq!(stats.current_bytes, 64);
        assert_eq!(stats.peak_bytes, 64);
    }

    #[test]
    fn alloc_zeroed_is_counted_separately() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let ptr = call_alloc_zeroed(&alloc, layout(32, 16));
        assert!(!ptr.is_null());
        assert_eq!(alloc.inner().alloc_zeroed_calls.get(), 1);
        assert_eq!(alloc.inner().alloc_calls.get(), 0);

        let stats = alloc.stats();
        assert_eq!(stats.alloc_zeroed.succeeded, 1);
        assert_eq!(stats.alloc_zeroed.bytes, 32);
        assert_eq!(stats.alloc.calls, 0);
        assert_eq!(stats.current_bytes, 32);
    }

    #[test]
    fn dealloc_counts_and_returns_live_bytes_to_zero() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let l = layout(64, 8);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
        assert_eq!(alloc.inner().dealloc_calls.get(), 1);

        let stats = alloc.stats();
        assert_eq!(stats.dealloc.calls, 1);
        assert_eq!(stats.dealloc.succeeded, 1);
        assert_eq!(stats.dealloc.failed, 0);
        assert_eq!(stats.dealloc.bytes, 64);
        assert_eq!(stats.current_bytes, 0);
        // The peak keeps the high-water mark after the free.
        assert_eq!(stats.peak_bytes, 64);
    }

    #[test]
    fn realloc_is_one_event_and_tracks_the_delta() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let l = layout(64, 8);
        let ptr = call_alloc(&alloc, l);
        let _ptr2 = call_realloc(&alloc, ptr, l, 128);

        // `realloc` must not decompose into alloc + dealloc.
        assert_eq!(alloc.inner().realloc_calls.get(), 1);
        assert_eq!(alloc.inner().alloc_calls.get(), 1);
        assert_eq!(alloc.inner().dealloc_calls.get(), 0);

        let stats = alloc.stats();
        assert_eq!(stats.realloc.calls, 1);
        assert_eq!(stats.realloc.succeeded, 1);
        assert_eq!(stats.realloc.bytes, 128);
        assert_eq!(stats.current_bytes, 128);
        assert_eq!(stats.peak_bytes, 128);
    }

    #[test]
    fn realloc_shrink_lowers_live_bytes() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let l = layout(128, 8);
        let ptr = call_alloc(&alloc, l);
        let _ptr2 = call_realloc(&alloc, ptr, l, 32);

        let stats = alloc.stats();
        assert_eq!(stats.realloc.bytes, 32);
        assert_eq!(stats.current_bytes, 32);
        assert_eq!(stats.peak_bytes, 128);
    }

    #[test]
    fn alloc_failure_counts_failed_and_keeps_live_bytes() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        alloc.inner().fail_alloc.set(true);
        let ptr = call_alloc(&alloc, layout(64, 8));
        assert!(ptr.is_null());

        let stats = alloc.stats();
        assert_eq!(stats.alloc.calls, 1);
        assert_eq!(stats.alloc.succeeded, 0);
        assert_eq!(stats.alloc.failed, 1);
        // Requested bytes are counted even on failure.
        assert_eq!(stats.alloc.bytes, 64);
        assert_eq!(stats.current_bytes, 0);
        assert_eq!(stats.peak_bytes, 0);
    }

    #[test]
    fn alloc_zeroed_failure_counts_failed() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        alloc.inner().fail_alloc_zeroed.set(true);
        let ptr = call_alloc_zeroed(&alloc, layout(64, 8));
        assert!(ptr.is_null());
        assert_eq!(alloc.stats().alloc_zeroed.failed, 1);
    }

    #[test]
    fn realloc_failure_keeps_old_block_accounting() {
        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let l = layout(64, 8);
        let ptr = call_alloc(&alloc, l);
        alloc.inner().fail_realloc.set(true);
        let out = call_realloc(&alloc, ptr, l, 128);
        assert!(out.is_null());

        let stats = alloc.stats();
        assert_eq!(stats.realloc.calls, 1);
        assert_eq!(stats.realloc.succeeded, 0);
        assert_eq!(stats.realloc.failed, 1);
        // The old block is still live: current bytes unchanged.
        assert_eq!(stats.current_bytes, 64);
        assert_eq!(stats.peak_bytes, 64);
    }

    #[test]
    fn alignment_is_forwarded_to_inner() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64, 64);
        let ptr = call_alloc(&alloc, l);
        assert!(!ptr.is_null());
        assert_eq!(ptr as usize % 64, 0);
        call_dealloc(&alloc, ptr, l);
    }

    #[test]
    #[expect(unsafe_code)]
    fn alloc_zeroed_returns_zeroed_memory() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64, 8);
        let ptr = call_alloc_zeroed(&alloc, l);
        assert!(!ptr.is_null());
        // SAFETY: `ptr` is a live 64-byte allocation for `l`.
        let bytes = unsafe { core::slice::from_raw_parts(ptr, 64) };
        assert!(bytes.iter().all(|&b| b == 0));
        call_dealloc(&alloc, ptr, l);
    }

    #[test]
    #[expect(unsafe_code)]
    fn realloc_preserves_contents() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64, 8);
        let ptr = call_alloc(&alloc, l);
        // SAFETY: `ptr` is a live 64-byte allocation for `l`.
        unsafe { core::ptr::write_bytes(ptr, 0xAB, 64) };
        let ptr2 = call_realloc(&alloc, ptr, l, 128);
        assert!(!ptr2.is_null());
        // SAFETY: `ptr2` is a live 128-byte allocation.
        let bytes = unsafe { core::slice::from_raw_parts(ptr2, 128) };
        assert!(bytes[..64].iter().all(|&b| b == 0xAB));
        call_dealloc(&alloc, ptr2, layout(128, 8));
    }

    #[test]
    fn system_alloc_failure_on_huge_layout() {
        let alloc = CountingAllocator::new(System);
        // Largest layout `GlobalAlloc` accepts: size <= isize::MAX.
        let l = Layout::from_size_align(isize::MAX as usize, 1).unwrap();
        let ptr = call_alloc(&alloc, l);
        assert!(ptr.is_null());
        assert_eq!(alloc.stats().alloc.failed, 1);
        assert_eq!(alloc.stats().alloc.succeeded, 0);
    }

    #[test]
    #[expect(unsafe_code)]
    fn system_realloc_failure_keeps_old_block_valid() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64, 8);
        let ptr = call_alloc(&alloc, l);
        assert!(!ptr.is_null());
        // new_size rounds up to align 8 and stays <= isize::MAX, so the
        // layout is contract-valid; System cannot satisfy ~8 EiB.
        let out = call_realloc(&alloc, ptr, l, isize::MAX as usize - 7);
        assert!(out.is_null());
        // The old block must still be valid and correctly accounted.
        // SAFETY: `ptr` is still a live 64-byte allocation for `l`.
        unsafe { core::ptr::write_bytes(ptr, 0xCD, 64) };
        // SAFETY: same live allocation.
        let bytes = unsafe { core::slice::from_raw_parts(ptr, 64) };
        assert!(bytes.iter().all(|&b| b == 0xCD));
        call_dealloc(&alloc, ptr, l);
        let stats = alloc.stats();
        assert_eq!(stats.realloc.failed, 1);
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn counting_path_does_not_allocate() {
        use assert_no_alloc::assert_no_alloc;

        let alloc = CountingAllocator::new(ProbeAlloc::default());
        let l = layout(64, 8);
        let ptr = assert_no_alloc(|| call_alloc(&alloc, l));
        assert!(!ptr.is_null());
        assert_no_alloc(|| call_dealloc(&alloc, ptr, l));

        let ptr = assert_no_alloc(|| call_alloc_zeroed(&alloc, l));
        assert_no_alloc(|| call_dealloc(&alloc, ptr, l));

        let ptr = assert_no_alloc(|| call_alloc(&alloc, l));
        let ptr2 = assert_no_alloc(|| call_realloc(&alloc, ptr, l, 128));
        assert_no_alloc(|| call_dealloc(&alloc, ptr2, layout(128, 8)));

        // Stats reads are plain atomic loads.
        assert_no_alloc(|| alloc.stats());
    }

    #[test]
    fn counting_allocator_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CountingAllocator<System>>();
        // `ProbeAlloc` uses `Cell` counters, so it is `Send` but not
        // `Sync`; the wrapper inherits that.
        assert_send::<CountingAllocator<ProbeAlloc>>();
    }

    #[test]
    fn counting_allocator_is_const_constructible() {
        static ALLOC: CountingAllocator<System> = CountingAllocator::new(System);
        let stats = ALLOC.stats();
        assert_eq!(stats.alloc.calls, 0);
    }
}
