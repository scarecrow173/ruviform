//! The retire channel — deferred `Drop` for objects retired on the
//! RT path.
//!
//! Heavy objects retired during RT processing (replaced buffers,
//! superseded configs) must not run their destructors on the RT
//! thread: `Drop` can deallocate, lock, or run unbounded user code.
//! [`Retirer::retire`] hands the object across the SPSC channel to a
//! [`DropCollector`] owned by a non-RT thread, whose
//! [`drain`](DropCollector::drain) runs the destructors where
//! deallocation is allowed — a self-built basedrop-style pattern on
//! the [`spsc`](crate::spsc) primitive (the `basedrop` dependency was
//! rejected — ADR).
//!
//! The full-queue overflow policy is a documented bounded-leak via
//! `core::mem::forget`: dropping the rejected value on the RT thread
//! is the rejected alternative (drop-on-full is RT-side dealloc —
//! ADR). `capacity` is therefore a caller contract — provision for
//! the maximum number of retirements per drain cycle.
//!
//! "Bounded" scopes the leak **per drain interval**, not the process
//! lifetime: at most `capacity` values sit queued between drains and
//! each overflow forgets one more value, so sustained saturation
//! leaks without an absolute ceiling. The contract is therefore
//! *observable saturation*, not guaranteed reclamation —
//! [`overflows()`](DropCollector::overflows) exposes the cumulative
//! forgotten count on both endpoints. A non-zero or growing count
//! means `capacity` or drain cadence is undersized for the real
//! retire rate; the fix is operational (bigger capacity, faster
//! drain), not an API change. Residual risk: each forgotten value
//! stays allocated until process exit, so heavy payloads and a
//! misconfigured cadence can accumulate real memory — long-duration
//! soak verification of drain cadence belongs to the RT verification
//! infrastructure (Issue #22), not to per-crate unit tests.
//!
//! Teardown note: values still queued when the *last* channel
//! endpoint is dropped run their destructors on the dropping thread
//! (wrapped-ring semantics). Keep the [`DropCollector`] alive until
//! after the [`Retirer`] is dropped so teardown deallocation stays on
//! the non-RT side.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::spsc::{Consumer, Producer, TryPushError, channel};

/// Builds a retire channel with the given fixed `capacity`.
///
/// The capacity is fixed at construction — the channel never grows.
/// `capacity` is a caller contract: it bounds the number of
/// retirements between drains before the [`Retirer::retire`]
/// bounded-leak fallback engages (see its doc).
///
/// `T: Send + 'static` is the API contract for retired payloads:
/// each value crosses to the collecting thread and leaves no borrow
/// behind at the non-RT drain point.
///
/// **RT contract:** non-RT only — construction allocates the ring.
///
/// # Examples
///
/// ```
/// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<Vec<u8>>(4);
///
/// retirer.retire(vec![0_u8; 1024]);
/// // The buffer's destructor did not run on this (retiring) thread —
/// // it runs where `drain` is called.
/// assert_eq!(collector.drain(), 1);
/// ```
#[must_use]
pub fn retire_channel<T: Send + 'static>(capacity: usize) -> (Retirer<T>, DropCollector<T>) {
    debug_assert!(capacity > 0, "retire_channel capacity must be non-zero");
    let (producer, consumer) = channel(capacity);
    let overflows = Arc::new(AtomicUsize::new(0));
    (
        Retirer {
            inner: producer,
            overflows: Arc::clone(&overflows),
        },
        DropCollector {
            inner: consumer,
            pending: Vec::new(),
            overflows,
        },
    )
}

/// Retiring endpoint of a retire channel — the RT side.
///
/// `Send` when `T: Send` — the endpoint is meant to move to the RT
/// thread. `!Sync`: single-threaded ownership, inherited from the
/// wrapped endpoint. Created together with its [`DropCollector`]
/// counterpart by [`retire_channel`].
///
/// **RT contract:** [`retire`](Self::retire) is RT-suitable;
/// construction via [`retire_channel`] is non-RT only.
///
/// # Examples
///
/// ```
/// let (mut retirer, _collector) = ruviform_rt::retire_channel::<u32>(2);
///
/// retirer.retire(1);
/// ```
#[derive(Debug)]
pub struct Retirer<T> {
    inner: Producer<T>,
    overflows: Arc<AtomicUsize>,
}

impl<T: Send + 'static> Retirer<T> {
    /// Retires `value`, routing its destructor to the non-RT side.
    ///
    /// Internally `try_push` semantics — non-blocking, no
    /// allocation, bounded. The queued value is dropped later by
    /// [`DropCollector::drain`] on the collecting thread; `retire`
    /// itself never runs `T`'s destructor.
    ///
    /// Overflow policy: on a full channel the value is **not**
    /// dropped here — dropping would run `T`'s destructor (and any
    /// deallocation) on the RT thread. The value is leaked via
    /// `core::mem::forget` instead: a bounded-leak fallback, bounded
    /// per drain interval (at most `capacity` queued plus one
    /// forgotten per overflow), **not** lifetime-bounded. Every
    /// forgotten value increments the shared
    /// [`overflows`](Self::overflows) counter — saturation is
    /// observable, not silent. Provision `capacity` for the maximum
    /// number of retirements per drain cycle and monitor the counter
    /// (drop-on-full is rejected because it is RT-side dealloc — ADR).
    ///
    /// **RT contract:** RT-suitable — wait-free, no allocation,
    /// bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<String>(2);
    ///
    /// retirer.retire(String::from("replaced config"));
    /// // Still alive on this side — nothing was dropped here.
    /// assert_eq!(collector.drain(), 1);
    /// ```
    pub fn retire(&mut self, value: T) {
        if let Err(TryPushError::Full(value)) = self.inner.try_push(value) {
            self.overflows.fetch_add(1, Ordering::Relaxed);
            core::mem::forget(value);
        }
    }

    /// Returns the cumulative number of values forgotten because the
    /// channel was full — the saturation signal of the bounded-leak
    /// overflow policy. Shared with the [`DropCollector`]: either
    /// endpoint can observe the same count.
    ///
    /// The count is `Relaxed`-ordered bookkeeping, not synchronized
    /// with channel state. A zero count is the healthy steady state;
    /// any non-zero or growing value means `capacity` or drain
    /// cadence is undersized for the actual retire rate — each
    /// forgotten value stays allocated until process exit.
    ///
    /// **RT contract:** RT-suitable — atomic load, no allocation,
    /// bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut retirer, _collector) = ruviform_rt::retire_channel::<u32>(1);
    ///
    /// retirer.retire(1);
    /// retirer.retire(2); // Full — forgotten, counter increments.
    /// assert_eq!(retirer.overflows(), 1);
    /// ```
    #[must_use]
    pub fn overflows(&self) -> usize {
        self.overflows.load(Ordering::Relaxed)
    }
}

/// Collecting endpoint of a retire channel — owned by the non-RT
/// side only.
///
/// `Send` when `T: Send` — the endpoint is meant to move to the
/// collecting thread. `!Sync`: single-threaded ownership, inherited
/// from the wrapped endpoint. Holds the receiving side plus a local
/// pending queue: the `snapshot` module feeds `Arc`s displaced by a
/// publish into that queue through the crate-internal `push_local`
/// seam, so both sources share the same [`drain`](Self::drain) path.
/// Created together with its [`Retirer`] counterpart by
/// [`retire_channel`].
///
/// **RT contract:** every method is non-RT only — `drain` is the
/// point where deferred deallocation is allowed to happen, so the
/// collector must never live on the RT thread.
///
/// # Examples
///
/// ```
/// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<u32>(2);
///
/// retirer.retire(7);
/// assert!(!collector.is_empty());
/// assert_eq!(collector.drain(), 1);
/// ```
#[derive(Debug)]
pub struct DropCollector<T> {
    inner: Consumer<T>,
    pending: Vec<T>,
    overflows: Arc<AtomicUsize>,
}

impl<T: Send + 'static> DropCollector<T> {
    /// Drops every retired object received so far — both the values
    /// that arrived through the channel and the ones handed to the
    /// local pending queue — running each `T` destructor on the
    /// calling thread. Returns the number of dropped objects.
    ///
    /// The channel part pops at most `capacity` elements per call
    /// (same bounded-loop rule as
    /// [`Consumer::pop_latest`](crate::Consumer::pop_latest)):
    /// everything queued at call time is dropped, and values a
    /// concurrent retirer pushes past that bound stay queued for the
    /// next drain.
    ///
    /// **RT contract:** non-RT only — runs destructor work,
    /// including deallocation, bounded by the queued count.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<u32>(4);
    ///
    /// retirer.retire(1);
    /// retirer.retire(2);
    /// assert_eq!(collector.drain(), 2);
    /// assert_eq!(collector.drain(), 0);
    /// ```
    pub fn drain(&mut self) -> usize {
        let mut dropped = 0_usize;
        for _ in 0..self.inner.capacity() {
            let Ok(value) = self.inner.try_pop() else {
                break;
            };
            drop(value);
            dropped += 1;
        }
        dropped += self.pending.len();
        self.pending.clear();
        dropped
    }

    /// Returns `true` if nothing is waiting to be collected — the
    /// channel looks empty and the local pending queue is empty.
    ///
    /// Diagnostic accessor: the channel part is an observation-time
    /// approximation (the retirer may concurrently push) — its
    /// accuracy is not part of the contract.
    ///
    /// **RT contract:** non-RT only — diagnostic for the collecting
    /// side.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<u32>(1);
    ///
    /// assert!(collector.is_empty());
    /// retirer.retire(9);
    /// assert!(!collector.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty() && self.pending.is_empty()
    }

    /// Returns the cumulative number of values the [`Retirer`] forgot
    /// because the channel was full — the saturation signal of the
    /// bounded-leak overflow policy, shared with the retiring
    /// endpoint.
    ///
    /// This is the non-RT-side observation point: poll it next to
    /// [`drain`](Self::drain). A zero count is the healthy steady
    /// state; any non-zero or growing value means `capacity` or drain
    /// cadence is undersized for the actual retire rate, and each
    /// forgotten value stays allocated until process exit — resize
    /// the channel or drain more often.
    ///
    /// **RT contract:** non-RT only — a monitoring accessor for the
    /// collecting side (an atomic load, technically non-blocking).
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut retirer, mut collector) = ruviform_rt::retire_channel::<u32>(1);
    ///
    /// retirer.retire(1);
    /// retirer.retire(2); // Full — forgotten.
    /// assert_eq!(collector.overflows(), 1);
    /// assert_eq!(collector.drain(), 1);
    /// ```
    #[must_use]
    pub fn overflows(&self) -> usize {
        self.overflows.load(Ordering::Relaxed)
    }

    /// Hands `value` to the local pending queue — the crate-internal
    /// seam the `snapshot` module uses to route `Arc`s displaced by
    /// `SnapshotWriter::publish` into the same [`drain`](Self::drain)
    /// path as channel-received retirements.
    ///
    /// Deliberately `pub(crate)`, not public API: non-RT only by
    /// ownership — the [`DropCollector`] lives on the collecting
    /// thread, and the pending `Vec` may allocate on push. Called by
    /// `SnapshotWriter::publish` in the `snapshot` module.
    pub(crate) fn push_local(&mut self, value: T) {
        self.pending.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc_counter;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Compile-time bound check: `T` must be `Send`. (`expect_*`
    /// naming — helper names must not contain the `rt_` substring
    /// reserved for the RT-contract test lane.)
    fn expect_send<T: Send>() {}

    /// Test-only probe whose `Drop` flips a shared flag — the side
    /// channel that observes whether (and via `drain`, when) the
    /// destructor ran.
    struct DropFlag(Arc<AtomicBool>);

    impl DropFlag {
        /// Returns the probe value plus the flag its `Drop` sets.
        fn flagged() -> (Self, Arc<AtomicBool>) {
            let flag = Arc::new(AtomicBool::new(false));
            (Self(Arc::clone(&flag)), flag)
        }
    }

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// RT contract: `retire` queues the value without running its
    /// destructor — the Drop flag stays unset on the retiring side
    /// and flips only when `drain` runs it on the collecting thread,
    /// which also reports the dropped count.
    #[test]
    fn rt_retire_then_drain_drops() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(4);
        let (probe, flag) = DropFlag::flagged();

        retirer.retire(probe);
        assert!(!flag.load(Ordering::SeqCst));
        assert!(!collector.is_empty());

        assert_eq!(collector.drain(), 1);
        assert!(flag.load(Ordering::SeqCst));
        assert!(collector.is_empty());
        assert_eq!(collector.drain(), 0);
    }

    /// RT contract: on a full channel `retire` falls back to
    /// `core::mem::forget` — the overflowed value's Drop flag never
    /// sets, proving the RT side ran no destructor (the documented
    /// bounded-leak; drop-on-full would be RT-side dealloc).
    #[test]
    fn rt_retire_full_forgets_not_drops() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(2);
        let (queued_a, flag_a) = DropFlag::flagged();
        let (queued_b, flag_b) = DropFlag::flagged();
        retirer.retire(queued_a);
        retirer.retire(queued_b);

        let (overflow, flag_c) = DropFlag::flagged();
        retirer.retire(overflow);
        assert!(!flag_c.load(Ordering::SeqCst));

        assert_eq!(collector.drain(), 2);
        assert!(flag_a.load(Ordering::SeqCst));
        assert!(flag_b.load(Ordering::SeqCst));
        // The forgotten value stays leaked — it was never dropped.
        assert!(!flag_c.load(Ordering::SeqCst));
    }

    /// RT contract: `retire` performs no allocation or deallocation
    /// on either the success path or the full-channel `mem::forget`
    /// fallback — measured inside a counting-allocator window after
    /// warm-up.
    #[test]
    fn rt_retire_no_alloc() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(2);

        // Warm the endpoints outside the measured window.
        {
            let (warm, _warm_flag) = DropFlag::flagged();
            retirer.retire(warm);
        }
        assert_eq!(collector.drain(), 1);

        let (queued_a, flag_a) = DropFlag::flagged();
        let (queued_b, flag_b) = DropFlag::flagged();
        let (overflow, flag_c) = DropFlag::flagged();

        alloc_counter::enable();
        retirer.retire(queued_a);
        retirer.retire(queued_b);
        retirer.retire(overflow);
        alloc_counter::disable();

        assert_eq!(alloc_counter::count(), 0);
        assert!(!flag_a.load(Ordering::SeqCst));
        assert!(!flag_b.load(Ordering::SeqCst));
        assert!(!flag_c.load(Ordering::SeqCst));
    }

    /// Thread-boundary bounds are API contract: each endpoint is
    /// `Send` (moves to its own thread) when `T: Send`. The `!Sync`
    /// single-threaded ownership is inherited from the wrapped
    /// endpoint — stable Rust cannot negate a `Sync` bound in a
    /// test, so it is asserted by signature review.
    #[test]
    fn rt_endpoint_send_bounds() {
        expect_send::<Retirer<u8>>();
        expect_send::<DropCollector<u8>>();
    }

    /// RT contract: the `overflows` counter is the saturation signal —
    /// each full-channel `mem::forget` increments it exactly once, it
    /// is shared between both endpoints, and `drain` does not touch it.
    #[test]
    fn rt_retire_overflow_counted_and_shared() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(1);
        let (queued, flag_a) = DropFlag::flagged();
        let (overflow, flag_b) = DropFlag::flagged();

        assert_eq!(retirer.overflows(), 0);
        assert_eq!(collector.overflows(), 0);

        retirer.retire(queued);
        retirer.retire(overflow);
        assert_eq!(retirer.overflows(), 1);
        assert_eq!(collector.overflows(), 1);
        assert!(!flag_b.load(Ordering::SeqCst));

        // Draining the queued value does not reset the signal.
        assert_eq!(collector.drain(), 1);
        assert!(flag_a.load(Ordering::SeqCst));
        assert_eq!(collector.overflows(), 1);
    }

    /// RT contract: the counter accumulates across drain cycles —
    /// "bounded-leak" bounds the per-interval queue, not the lifetime
    /// leak; repeated saturation is observable as a growing count.
    #[test]
    fn rt_retire_repeated_saturation_accumulates() {
        let (mut retirer, mut collector) = retire_channel::<u32>(2);

        for cycle in 1..=3_usize {
            retirer.retire(1);
            retirer.retire(2);
            retirer.retire(3); // overflow each cycle
            assert_eq!(collector.drain(), 2);
            assert_eq!(collector.overflows(), cycle);
        }
    }

    /// RT contract: teardown order — values queued when the `Retirer`
    /// is dropped are still delivered to `drain`, so their
    /// destructors run on the collecting thread (the safe order the
    /// module doc prescribes: collector outlives retirer).
    #[test]
    fn rt_retire_teardown_collector_outlives() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(2);
        let (probe, flag) = DropFlag::flagged();
        retirer.retire(probe);

        drop(retirer);
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(collector.drain(), 1);
        assert!(flag.load(Ordering::SeqCst));
    }

    /// `capacity` must be non-zero — the degenerate always-full
    /// channel is rejected in debug builds.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "capacity must be non-zero")]
    fn zero_capacity_panics_in_debug() {
        let _unused = retire_channel::<u32>(0);
    }

    /// `push_local` is the crate-internal seam: locally handed
    /// values share the same `drain` path as channel-received ones,
    /// count toward the dropped total, and are visible to `is_empty`
    /// on their own. (Non-RT test — the name keeps the `rt_`
    /// substring out of the RT-contract lane.)
    #[test]
    fn local_pending_drops_on_drain() {
        let (mut retirer, mut collector) = retire_channel::<DropFlag>(2);
        let (local, flag_a) = DropFlag::flagged();
        let (sent, flag_b) = DropFlag::flagged();

        collector.push_local(local);
        // The pending queue alone already reports non-empty.
        assert!(!collector.is_empty());
        retirer.retire(sent);

        assert_eq!(collector.drain(), 2);
        assert!(flag_a.load(Ordering::SeqCst));
        assert!(flag_b.load(Ordering::SeqCst));
        assert!(collector.is_empty());
    }
}
