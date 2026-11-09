//! Shared test helpers for the profiling tests.
//!
//! `ProbeAlloc` is a `GlobalAlloc` that records which API was invoked
//! and returns aligned fake pointers without touching real memory, so
//! the counting path around it can be verified with `assert_no_alloc`.
//! Real-memory behavior (zeroing, realloc contents, alignment, actual
//! failure) is covered separately through `System`.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::allocator::CountingAllocator;

/// Bump address for fake allocations; starts high enough to look like
/// a real heap address and only ever increases.
static NEXT_FAKE: AtomicUsize = AtomicUsize::new(0x4000_0000);

/// Returns an aligned, never-dereferenced fake pointer for `layout`.
fn fake_ptr(layout: Layout) -> *mut u8 {
    let align = layout.align();
    let size = layout.size();
    loop {
        let cur = NEXT_FAKE.load(Ordering::Relaxed);
        let aligned = cur.next_multiple_of(align);
        let next = aligned + size;
        if NEXT_FAKE
            .compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return aligned as *mut u8;
        }
    }
}

/// A `GlobalAlloc` that counts invocations per API and can be told to
/// fail. It never allocates real memory.
#[derive(Debug, Default)]
pub(crate) struct ProbeAlloc {
    /// Number of `alloc` calls seen.
    pub(crate) alloc_calls: Cell<u64>,
    /// Number of `alloc_zeroed` calls seen.
    pub(crate) alloc_zeroed_calls: Cell<u64>,
    /// Number of `dealloc` calls seen.
    pub(crate) dealloc_calls: Cell<u64>,
    /// Number of `realloc` calls seen.
    pub(crate) realloc_calls: Cell<u64>,
    /// When set, `alloc` returns null.
    pub(crate) fail_alloc: Cell<bool>,
    /// When set, `alloc_zeroed` returns null.
    pub(crate) fail_alloc_zeroed: Cell<bool>,
    /// When set, `realloc` returns null (the old block stays live).
    pub(crate) fail_realloc: Cell<bool>,
}

// Safety: test-only fake. Returned pointers are aligned and distinct
// but point at no real memory; tests using `ProbeAlloc` must never
// dereference them — they only verify delegation and counting.
#[expect(unsafe_code)]
unsafe impl GlobalAlloc for ProbeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_calls.set(self.alloc_calls.get() + 1);
        if self.fail_alloc.get() {
            ptr::null_mut()
        } else {
            fake_ptr(layout)
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc_zeroed_calls
            .set(self.alloc_zeroed_calls.get() + 1);
        if self.fail_alloc_zeroed.get() {
            ptr::null_mut()
        } else {
            fake_ptr(layout)
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        self.dealloc_calls.set(self.dealloc_calls.get() + 1);
    }

    unsafe fn realloc(&self, _ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.realloc_calls.set(self.realloc_calls.get() + 1);
        if self.fail_realloc.get() {
            ptr::null_mut()
        } else {
            // Callers reach this only through `CountingAllocator::
            // realloc`, which forwards contract-valid `new_size`/`align`
            // pairs, so the checked constructor cannot fail here.
            let new_layout = Layout::from_size_align(new_size, layout.align()).unwrap();
            fake_ptr(new_layout)
        }
    }
}

/// Calls `CountingAllocator::alloc`. Test helper so tests exercise
/// the same `GlobalAlloc` path a `#[global_allocator]` install would.
#[expect(unsafe_code)]
pub(crate) fn call_alloc<A: GlobalAlloc>(alloc: &CountingAllocator<A>, layout: Layout) -> *mut u8 {
    // SAFETY: tests pass contract-valid layouts; the helper exists so
    // call sites stay readable.
    unsafe { alloc.alloc(layout) }
}

/// Calls `CountingAllocator::alloc_zeroed`.
#[expect(unsafe_code)]
pub(crate) fn call_alloc_zeroed<A: GlobalAlloc>(
    alloc: &CountingAllocator<A>,
    layout: Layout,
) -> *mut u8 {
    // SAFETY: same contract as `call_alloc`.
    unsafe { alloc.alloc_zeroed(layout) }
}

/// Calls `CountingAllocator::dealloc`.
#[expect(unsafe_code)]
pub(crate) fn call_dealloc<A: GlobalAlloc>(
    alloc: &CountingAllocator<A>,
    ptr: *mut u8,
    layout: Layout,
) {
    // SAFETY: tests pass pointers previously returned by the same
    // allocator instance for the same layout.
    unsafe { alloc.dealloc(ptr, layout) };
}

/// Calls `CountingAllocator::realloc`.
#[expect(unsafe_code)]
pub(crate) fn call_realloc<A: GlobalAlloc>(
    alloc: &CountingAllocator<A>,
    ptr: *mut u8,
    layout: Layout,
    new_size: usize,
) -> *mut u8 {
    // SAFETY: same contract as `call_alloc`; `new_size` keeps the
    // layout's alignment in tests.
    unsafe { alloc.realloc(ptr, layout, new_size) }
}
