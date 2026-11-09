//! Thread-local allocation measurement scopes.
//!
//! An [`AllocationScope`] owns a counter node that is linked into a
//! per-thread chain while an [`AllocationWindow`] is alive. Every
//! [`CountingAllocator`] operation on the calling thread updates all
//! active scopes on that thread — nested scopes are each counted
//! independently.
//!
//! Scopes observe only the thread that owns them. Allocations on other
//! threads never appear in a scope's statistics; use
//! [`CountingAllocator::stats`] for a process-wide aggregate instead.
//!
//! The chain is a linked list of nodes kept alive by their owning
//! [`AllocationScope`]. Nodes are unlinked before they are freed, so
//! windows and scopes may be dropped in any order. If thread-local
//! storage is already being destroyed (for example an allocation
//! triggered by another TLS destructor), counting silently degrades
//! to a no-op rather than panicking.
//!
//! [`CountingAllocator`]: super::allocator::CountingAllocator
//! [`CountingAllocator::stats`]: super::allocator::CountingAllocator::stats

use core::cell::Cell;
use std::thread_local;

use super::stats::{AllocationCounters, AllocationStats, Op, bump_cell, current_add_cell};

thread_local! {
    /// Head of the active-scope chain for this thread, stored as the
    /// exposed provenance address of the head [`ScopeNode`]. `0` means
    /// no active scope.
    static HEAD: Cell<usize> = const { Cell::new(0) };
}

/// Per-operation `Cell` counters for one scope node.
#[derive(Debug)]
struct ScopeOpCounters {
    calls: Cell<u64>,
    succeeded: Cell<u64>,
    failed: Cell<u64>,
    bytes: Cell<u64>,
}

impl ScopeOpCounters {
    const fn new() -> Self {
        Self {
            calls: Cell::new(0),
            succeeded: Cell::new(0),
            failed: Cell::new(0),
            bytes: Cell::new(0),
        }
    }

    fn snapshot(&self) -> super::stats::OpStats {
        super::stats::OpStats {
            calls: self.calls.get(),
            succeeded: self.succeeded.get(),
            failed: self.failed.get(),
            bytes: self.bytes.get(),
        }
    }
}

/// The counter node owned by an [`AllocationScope`] and linked into
/// the per-thread chain while a window is active.
///
/// `parent` stores the exposed-provenance address of the next node
/// toward the chain head at link time; it doubles as the "previous
/// head" needed to unlink the node in any drop order.
#[derive(Debug)]
pub(crate) struct ScopeNode {
    parent: Cell<usize>,
    /// Whether a window currently links this node. Guards against
    /// re-entering a scope whose previous window was forgotten.
    active: Cell<bool>,
    alloc: ScopeOpCounters,
    alloc_zeroed: ScopeOpCounters,
    dealloc: ScopeOpCounters,
    realloc: ScopeOpCounters,
    current_bytes: Cell<i64>,
    peak_bytes: Cell<u64>,
}

impl ScopeNode {
    fn new() -> Self {
        Self {
            parent: Cell::new(0),
            active: Cell::new(false),
            alloc: ScopeOpCounters::new(),
            alloc_zeroed: ScopeOpCounters::new(),
            dealloc: ScopeOpCounters::new(),
            realloc: ScopeOpCounters::new(),
            current_bytes: Cell::new(0),
            peak_bytes: Cell::new(0),
        }
    }

    fn op_counters(&self, op: Op) -> &ScopeOpCounters {
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
            current_bytes: self.current_bytes.get(),
            peak_bytes: self.peak_bytes.get(),
        }
    }
}

impl AllocationCounters for ScopeNode {
    fn request(&self, op: Op, bytes: u64) {
        let counters = self.op_counters(op);
        bump_cell(&counters.calls, 1);
        bump_cell(&counters.bytes, bytes);
    }

    fn succeeded(&self, op: Op) {
        bump_cell(&self.op_counters(op).succeeded, 1);
    }

    fn failed(&self, op: Op) {
        bump_cell(&self.op_counters(op).failed, 1);
    }

    fn current_add(&self, delta: i64) {
        current_add_cell(&self.current_bytes, &self.peak_bytes, delta);
    }
}

/// Returns the exposed-provenance address of `node`.
fn node_addr(node: &ScopeNode) -> usize {
    core::ptr::from_ref(node).expose_provenance()
}

/// Rebuilds a `&ScopeNode` from its exposed-provenance address.
///
/// # Safety
///
/// `addr` must be the address of a live [`ScopeNode`], produced by
/// [`node_addr`]. Every address stored in the chain belongs to a node
/// kept alive by its owning [`AllocationScope`]; nodes are unlinked
/// before being freed (see module docs), so any address reachable from
/// the chain is valid for the duration of the walk.
#[expect(unsafe_code)]
unsafe fn node_ref(addr: usize) -> &'static ScopeNode {
    // SAFETY: upheld by the caller contract documented above.
    unsafe { &*core::ptr::with_exposed_provenance::<ScopeNode>(addr) }
}

/// Invokes `f` on every active scope node of the calling thread, from
/// the chain head downward. Does nothing when TLS is already being
/// destroyed.
#[expect(unsafe_code)]
pub(crate) fn for_each_active_scope(mut f: impl FnMut(&ScopeNode)) {
    let _ = HEAD.try_with(|head| {
        let mut cur = head.get();
        while cur != 0 {
            let node = unsafe { node_ref(cur) };
            f(node);
            cur = node.parent.get();
        }
    });
}

/// Unlinks `node_addr` from the chain headed at `head`, in any
/// position. `node_parent` is the node's stored parent — the node
/// that preceded it in the chain at link time.
#[expect(unsafe_code)]
fn unlink_node(head: &Cell<usize>, node_addr: usize, node_parent: usize) {
    if head.get() == node_addr {
        head.set(node_parent);
        return;
    }
    let mut cur = head.get();
    while cur != 0 {
        // SAFETY: same chain invariant as `for_each_active_scope` —
        // every reachable address is a live node.
        let node = unsafe { node_ref(cur) };
        let next = node.parent.get();
        if next == node_addr {
            node.parent.set(node_parent);
            return;
        }
        cur = next;
    }
}

/// A thread-local accumulation point for allocation statistics.
///
/// Create with [`AllocationScope::new`], then bracket the code to
/// measure with [`AllocationScope::enter`]. While an
/// [`AllocationWindow`] is alive, every [`CountingAllocator`]
/// operation performed **on the same thread** updates this scope's
/// counters in addition to the allocator's process-wide statistics.
///
/// Counters accumulate from scope creation (or the last
/// [`AllocationScope::reset`]) across all windows; they are not reset
/// when a window closes.
///
/// `AllocationScope` is neither `Send` nor `Sync`: a scope only ever
/// counts allocations on the thread that enters it, so sharing one
/// across threads would silently misattribute results — and a
/// forgotten window would leave the node linked in the originating
/// thread's chain, where another thread's drop could not unlink it.
/// Create the scope on the thread that measures.
///
/// [`CountingAllocator`]: super::allocator::CountingAllocator
#[derive(Debug)]
pub struct AllocationScope {
    node: Box<ScopeNode>,
    // `*const ()` is !Send + !Sync; the scope is bound to the thread
    // whose TLS chain its node is linked into.
    _thread_affine: core::marker::PhantomData<*const ()>,
}

impl AllocationScope {
    /// Creates a new scope with zeroed counters.
    ///
    /// This allocates one node on the heap, so it is **not** RT-safe;
    /// create scopes outside the real-time path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            node: Box::new(ScopeNode::new()),
            _thread_affine: core::marker::PhantomData,
        }
    }

    /// Opens a measurement window on the calling thread.
    ///
    /// The returned [`AllocationWindow`] links this scope into the
    /// thread's active-scope chain; closing the window (by drop)
    /// unlinks it. Nested scopes on the same thread are all updated
    /// for each allocation event.
    ///
    /// Entering is a bounded pointer write plus a TLS access: no
    /// allocation, no lock, and no panic on the normal path, so it is
    /// usable on RT threads when the scope itself was created earlier.
    ///
    /// # Panics
    ///
    /// Panics if the scope already has a live window — i.e. a previous
    /// [`AllocationWindow`] was leaked with `mem::forget` and never
    /// closed. Drop the scope and create a new one to recover.
    #[must_use]
    pub fn enter(&mut self) -> AllocationWindow<'_> {
        assert!(
            !self.node.active.replace(true),
            "AllocationScope::enter called while a previous AllocationWindow is still alive"
        );
        let addr = node_addr(&self.node);
        // Push this node as the new head and learn the previous head
        // (our parent link) in one TLS access. No allocation can
        // interleave between the two stores: nothing on this path
        // allocates.
        let pushed = HEAD.try_with(|head| {
            let prev = head.get();
            head.set(addr);
            prev
        });
        match pushed {
            Ok(prev) => self.node.parent.set(prev),
            // TLS is being destroyed: leave the scope usable but inert.
            Err(_) => self.node.active.set(false),
        }
        AllocationWindow {
            scope: self,
            _not_send: core::marker::PhantomData,
        }
    }

    /// Returns a snapshot of the counters accumulated so far.
    ///
    /// This is a plain read of `Cell` values: no allocation, no lock,
    /// no panic.
    #[must_use]
    pub fn stats(&self) -> AllocationStats {
        self.node.snapshot()
    }

    /// Zeroes all counters, including `current_bytes` and
    /// `peak_bytes`, without unlinking an active window.
    ///
    /// This is a plain write of `Cell` values: no allocation, no lock,
    /// no panic.
    pub fn reset(&mut self) {
        self.node.alloc = ScopeOpCounters::new();
        self.node.alloc_zeroed = ScopeOpCounters::new();
        self.node.dealloc = ScopeOpCounters::new();
        self.node.realloc = ScopeOpCounters::new();
        self.node.current_bytes.set(0);
        self.node.peak_bytes.set(0);
    }

    /// Creates a scope, measures `f` on the calling thread, and
    /// returns `(result, stats)`.
    ///
    /// Equivalent to `enter` + `f()` + drop window + `stats()`, but
    /// the scope node is dropped immediately afterwards.
    ///
    /// This allocates the scope node up front, so it is **not**
    /// RT-safe as a whole; the measured region itself only adds the
    /// bounded enter/exit cost documented on [`AllocationScope::enter`].
    #[must_use]
    pub fn measure<R>(f: impl FnOnce() -> R) -> (R, AllocationStats) {
        let mut scope = Self::new();
        let result = {
            let window = scope.enter();
            let result = f();
            drop(window);
            result
        };
        (result, scope.stats())
    }
}

impl Default for AllocationScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AllocationScope {
    fn drop(&mut self) {
        // Unlink in case a forgotten window left the node linked.
        // If TLS is already gone this is a no-op and the node is freed
        // with the scope anyway.
        let addr = node_addr(&self.node);
        let parent = self.node.parent.get();
        let _ = HEAD.try_with(|head| unlink_node(head, addr, parent));
    }
}

/// An active measurement window returned by [`AllocationScope::enter`].
///
/// While alive, the owning scope counts allocations on the calling
/// thread. Dropping the window unlinks the scope; windows and scopes
/// may be dropped in any order.
///
/// The window borrows its scope mutably, so it cannot outlive it. If
/// it is leaked with `mem::forget`, the scope stays linked and keeps
/// counting, `active` remains set, and the scope can no longer be
/// re-entered — drop the scope (which unlinks the node) and create a
/// new one to recover.
///
/// `AllocationWindow` is neither `Send` nor `Sync`: it is bound to
/// the thread-local chain it was entered on.
pub struct AllocationWindow<'a> {
    scope: &'a mut AllocationScope,
    // `*mut ()` is !Send + !Sync; the window is meaningless on any
    // thread but the one that entered it.
    _not_send: core::marker::PhantomData<*mut ()>,
}

impl AllocationWindow<'_> {
    /// Returns a snapshot of the owning scope's counters so far.
    ///
    /// This is a plain read of `Cell` values: no allocation, no lock,
    /// no panic.
    #[must_use]
    pub fn stats(&self) -> AllocationStats {
        self.scope.node.snapshot()
    }
}

impl Drop for AllocationWindow<'_> {
    fn drop(&mut self) {
        self.scope.node.active.set(false);
        let addr = node_addr(&self.scope.node);
        let parent = self.scope.node.parent.get();
        let _ = HEAD.try_with(|head| unlink_node(head, addr, parent));
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    use std::alloc::System;
    use std::mem;
    use std::thread;

    use super::*;
    use crate::profiling::allocator::CountingAllocator;
    use crate::profiling::testutil::{call_alloc, call_alloc_zeroed, call_dealloc, call_realloc};

    fn layout(size: usize) -> Layout {
        Layout::from_size_align(size, 8).unwrap()
    }

    #[test]
    fn scope_counts_allocations_on_the_owning_thread() {
        let alloc = CountingAllocator::new(System);
        let mut scope = AllocationScope::new();
        {
            let window = scope.enter();
            let l = layout(64);
            let ptr = call_alloc(&alloc, l);
            call_dealloc(&alloc, ptr, l);

            let stats = window.stats();
            assert_eq!(stats.alloc.calls, 1);
            assert_eq!(stats.alloc.succeeded, 1);
            assert_eq!(stats.dealloc.calls, 1);
            assert_eq!(stats.current_bytes, 0);
            assert_eq!(stats.peak_bytes, 64);
        }
        // Counters are cumulative across windows, not reset on close.
        assert_eq!(scope.stats().alloc.calls, 1);
    }

    #[test]
    fn allocations_outside_a_window_are_not_counted() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64);
        // Before any window.
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);

        let mut scope = AllocationScope::new();
        {
            let _window = scope.enter();
        }
        // After the window closed.
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);

        assert_eq!(scope.stats().alloc.calls, 0);
        assert_eq!(scope.stats().dealloc.calls, 0);
        assert_eq!(scope.stats().current_bytes, 0);
    }

    #[test]
    fn nested_scopes_each_count_independently() {
        let alloc = CountingAllocator::new(System);
        let mut outer = AllocationScope::new();
        let mut inner = AllocationScope::new();
        {
            let _outer_window = outer.enter();
            {
                let inner_window = inner.enter();
                let l = layout(64);
                let ptr = call_alloc(&alloc, l);
                call_dealloc(&alloc, ptr, l);
                assert_eq!(inner_window.stats().alloc.calls, 1);
            }
            // The inner window closed; the outer still counts.
            let l = layout(32);
            let ptr = call_alloc(&alloc, l);
            call_dealloc(&alloc, ptr, l);
        }
        assert_eq!(outer.stats().alloc.calls, 2);
        assert_eq!(inner.stats().alloc.calls, 1);
    }

    #[test]
    fn windows_may_close_out_of_order() {
        let alloc = CountingAllocator::new(System);
        let mut scope_a = AllocationScope::new();
        let mut scope_b = AllocationScope::new();
        let window_a = scope_a.enter();
        let window_b = scope_b.enter();

        // Non-LIFO: close the outer-entered window first.
        drop(window_a);
        let l = layout(64);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
        assert_eq!(window_b.stats().alloc.calls, 1);
        assert_eq!(scope_a.stats().alloc.calls, 0);

        drop(window_b);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
        assert_eq!(scope_a.stats().alloc.calls, 0);
        assert_eq!(scope_b.stats().alloc.calls, 1);
    }

    #[test]
    fn scopes_on_other_threads_are_invisible() {
        let alloc = CountingAllocator::new(System);
        let mut main_scope = AllocationScope::new();
        let window = main_scope.enter();
        // This thread's own allocation is counted while the child runs.
        let l = layout(64);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);

        let child = thread::spawn(|| {
            let child_alloc = CountingAllocator::new(System);
            let mut child_scope = AllocationScope::new();
            let in_window = {
                let child_window = child_scope.enter();
                for _ in 0..3 {
                    let l = layout(64);
                    let ptr = call_alloc(&child_alloc, l);
                    call_dealloc(&child_alloc, ptr, l);
                }
                child_window.stats().alloc.calls
            };
            (in_window, child_scope.stats().alloc.calls)
        });
        let (child_in_window, child_total) = child.join().unwrap();
        assert_eq!(child_in_window, 3);
        assert_eq!(child_total, 3);

        // Only this thread's own allocation reached its chain.
        assert_eq!(window.stats().alloc.calls, 1);
        assert_eq!(window.stats().dealloc.calls, 1);
        drop(window);
    }

    #[test]
    fn forgotten_window_keeps_counting_and_scope_drop_is_safe() {
        let alloc = CountingAllocator::new(System);
        let mut scope = AllocationScope::new();
        let window = scope.enter();
        let l = layout(64);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
        mem::forget(window);

        // The leaked window left the node linked: the scope still
        // counts, and it cannot be re-entered (covered by
        // `reentering_after_a_forgotten_window_panics`).
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
        assert_eq!(scope.stats().alloc.calls, 2);

        // Dropping the scope unlinks the node; no dangling chain entry.
        drop(scope);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);
    }

    #[test]
    #[should_panic(expected = "still alive")]
    fn reentering_after_a_forgotten_window_panics() {
        let mut scope = AllocationScope::new();
        mem::forget(scope.enter());
        let _window = scope.enter();
    }

    #[test]
    fn measure_returns_the_result_and_window_stats() {
        let alloc = CountingAllocator::new(System);
        // Outside the measured region.
        let l = layout(64);
        let ptr = call_alloc(&alloc, l);
        call_dealloc(&alloc, ptr, l);

        let (result, stats) = AllocationScope::measure(|| {
            let l = layout(32);
            let ptr = call_alloc(&alloc, l);
            call_dealloc(&alloc, ptr, l);
            42_u32
        });
        assert_eq!(result, 42);
        assert_eq!(stats.alloc.calls, 1);
        assert_eq!(stats.alloc.bytes, 32);
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn freeing_a_pre_scope_block_goes_negative() {
        let alloc = CountingAllocator::new(System);
        let l = layout(64);
        // Allocated before the scope exists: the scope never saw the
        // alloc, so freeing it inside the window drives current bytes
        // negative — the documented `i64` semantics.
        let ptr = call_alloc(&alloc, l);
        let mut scope = AllocationScope::new();
        {
            let window = scope.enter();
            call_dealloc(&alloc, ptr, l);
            let stats = window.stats();
            assert_eq!(stats.dealloc.calls, 1);
            assert_eq!(stats.current_bytes, -64);
            // The peak only tracks non-negative values.
            assert_eq!(stats.peak_bytes, 0);
        }
    }

    #[test]
    fn reset_zeroes_all_counters() {
        let alloc = CountingAllocator::new(System);
        let mut scope = AllocationScope::new();
        {
            let _window = scope.enter();
            let l = layout(64);
            let ptr = call_alloc(&alloc, l);
            let ptr2 = call_realloc(&alloc, ptr, l, 128);
            call_dealloc(&alloc, ptr2, layout(128));
        }
        assert!(scope.stats().alloc.calls > 0);
        scope.reset();
        let stats = scope.stats();
        assert_eq!(stats, AllocationStats::default());
    }

    #[test]
    fn window_enter_stats_and_exit_do_not_allocate() {
        use assert_no_alloc::assert_no_alloc;

        let mut scope = AllocationScope::new();
        assert_no_alloc(|| {
            let window = scope.enter();
            let _ = window.stats();
            drop(window);
        });
        assert_no_alloc(|| scope.stats());
        assert_no_alloc(|| scope.reset());
    }

    #[test]
    fn thread_teardown_with_forgotten_window_is_safe() {
        let child = thread::spawn(|| {
            let alloc = CountingAllocator::new(System);
            let mut scope = AllocationScope::new();
            let window = scope.enter();
            let l = layout(64);
            let ptr = call_alloc(&alloc, l);
            mem::forget(window);
            call_dealloc(&alloc, ptr, l);
            scope.stats().alloc.calls
        });
        assert_eq!(child.join().unwrap(), 1);
    }

    #[test]
    fn tls_destructor_allocation_does_not_panic() {
        /// Forces TLS-destructor ordering coverage: this value's
        /// destructor performs an allocation through a shared
        /// `CountingAllocator` while the thread is tearing down.
        struct TeardownGuard;
        impl Drop for TeardownGuard {
            fn drop(&mut self) {
                static ALLOC: CountingAllocator<System> = CountingAllocator::new(System);
                let l = Layout::from_size_align(32, 8).unwrap();
                let ptr = call_alloc(&ALLOC, l);
                call_dealloc(&ALLOC, ptr, l);
            }
        }
        thread_local! {
            static GUARD: TeardownGuard = const { TeardownGuard };
        }

        let child = thread::spawn(|| {
            GUARD.with(|_| ());
            // Give the destructor a chance to run against a partially
            // torn-down TLS area; either way it must not panic.
        });
        child.join().unwrap();
    }

    #[test]
    fn all_four_operations_reach_the_scope() {
        let alloc = CountingAllocator::new(System);
        let mut scope = AllocationScope::new();
        {
            let window = scope.enter();
            let l = layout(64);
            let ptr = call_alloc_zeroed(&alloc, l);
            let ptr2 = call_realloc(&alloc, ptr, l, 96);
            call_dealloc(&alloc, ptr2, layout(96));
            let stats = window.stats();
            assert_eq!(stats.alloc_zeroed.calls, 1);
            assert_eq!(stats.realloc.calls, 1);
            assert_eq!(stats.dealloc.calls, 1);
            assert_eq!(stats.current_bytes, 0);
        }
    }
}
