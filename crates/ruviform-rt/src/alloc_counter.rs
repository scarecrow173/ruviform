//! Test-only counting [`GlobalAlloc`] for the crate's test binary —
//! std-only, no external dependencies.
//!
//! The counter state is thread-local: every `#[test]` runs on its own
//! freshly spawned thread, so a test that enables counting can never
//! miscount allocations made by tests running in parallel on other
//! threads. The measured window is the region between [`enable`] and
//! [`disable`]; [`count`] reports the events recorded by the calling
//! thread.
//!
//! The `const`-initialized thread-local performs no lazy registration
//! and no allocation on access, so the allocator can touch it without
//! recursing into itself; `try_with` degrades to "not counted" during
//! thread teardown instead of panicking inside the allocator.
//!
//! Helper names deliberately avoid the `rt_` substring, which is
//! reserved for the RT-contract test lane (`cargo test rt_`).

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use std::alloc::System;

std::thread_local! {
    /// `(enabled, events)` for the current thread.
    static STATE: Cell<(bool, usize)> = const { Cell::new((false, 0)) };
}

/// Records one alloc/dealloc event when counting is enabled on the
/// current thread. Never allocates, never panics — both properties
/// are required inside the allocator path.
fn note_event() {
    let _ = STATE.try_with(|state| {
        let (enabled, events) = state.get();
        if enabled {
            state.set((enabled, events + 1));
        }
    });
}

/// Counting allocator installed for the crate's test binary only.
/// The shipped library imposes no global allocator on consumers.
struct Counting;

// SAFETY: every operation is delegated to `std::alloc::System`, so the
// `GlobalAlloc` contract (layout handling, null-on-failure, alignment)
// is inherited unchanged. The only added work is a non-allocating,
// non-panicking thread-local flag check and counter bump.
#[expect(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_event();
        // SAFETY: forwards the caller's `layout` contract unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note_event();
        // SAFETY: forwards the pointer/`layout` contract unchanged.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Enables counting on the current thread and resets its counter.
pub(crate) fn enable() {
    STATE.with(|state| state.set((true, 0)));
}

/// Disables counting on the current thread; the count stays readable
/// via [`count`].
pub(crate) fn disable() {
    STATE.with(|state| {
        let (_, events) = state.get();
        state.set((false, events));
    });
}

/// Returns the number of alloc/dealloc events recorded on the calling
/// thread since [`enable`].
pub(crate) fn count() -> usize {
    STATE.with(|state| state.get().1)
}
