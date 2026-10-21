//! The snapshot exchange — atomic replacement of immutable `Arc<T>`
//! snapshots built on the non-RT side and read non-blocking on the
//! RT path.
//!
//! Configurations and execution plans are rebuilt off the RT thread
//! while the RT loop keeps reading the current one. [`slot`] builds a
//! connected [`SnapshotWriter`] (non-RT) / [`SnapshotReader`] (RT)
//! pair sharing an `arc_swap::ArcSwap<T>` cell:
//! [`publish`](SnapshotWriter::publish) atomically replaces the
//! snapshot and [`read`](SnapshotReader::read) hands the RT side a
//! [`SnapshotGuard`] — a lock-free load plus an `Arc` clone: no
//! blocking, and no allocation *after warm-up* (see below).
//!
//! Warm-up contract: the internal `arc_swap` cell allocates a
//! per-thread debt node the first time a given thread reads a given
//! slot. Call [`SnapshotReader::read`] once on the RT thread during
//! setup — before the RT contract interval begins (before `RtContext`
//! mint / the first `process`) — so subsequent reads are
//! allocation-free.
//!
//! Teardown contract: drop the [`SnapshotReader`] on the non-RT side
//! (or before retiring the writer). If the writer is dropped first
//! and the reader is then dropped on the RT thread, the last
//! `Arc<ArcSwap<T>>` destroys the live snapshot and any `Arc`s still
//! queued in the reader's retire channel — running `T` destructors on
//! the RT thread.
//!
//! The core contract — *a snapshot's destructor never runs on the RT
//! thread* — needs more than `arc-swap` alone. A raw `Arc` taken from
//! `load_full` could still become the last reference on the reader
//! side (the reader-side last-drop hole): after the writer swaps and
//! the non-RT drain frees its own copies, releasing that final
//! reference would deallocate inline on the RT thread.
//! [`SnapshotGuard`]'s `Drop` closes the hole by routing its `Arc`
//! into the reader's [`Retirer`] instead of releasing it inline, and
//! `publish` routes the displaced `Arc` into the writer's
//! [`DropCollector`] through the crate-internal `push_local` seam.
//! Deallocation therefore happens only inside
//! [`collect`](SnapshotWriter::collect) on the non-RT thread — the
//! single drain path shared by both retirement sources.
//!
//! `arc-swap` is a private implementation detail: no `arc_swap` type
//! appears in the public API (dependency rationale:
//! `docs/adr/rt-foundation-primitives-implementation.md`).

use std::cell::RefCell;
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::retire::{DropCollector, Retirer, retire_channel};

/// Builds a snapshot-exchange slot and returns the connected
/// [`SnapshotWriter`] / [`SnapshotReader`] pair.
///
/// `initial` is the snapshot readers observe until the first
/// [`publish`](SnapshotWriter::publish). `retire_capacity` is the
/// fixed capacity of the reader-side retire channel — a caller
/// contract provisioning for the maximum number of guard drops per
/// [`collect`](SnapshotWriter::collect) cycle before the
/// bounded-leak fallback engages (see [`Retirer::retire`]).
///
/// `T: Send + Sync` is the API contract: the shared `Arc<T>`
/// contents cross between the writer's non-RT thread and the
/// reader's RT thread.
///
/// **RT contract:** non-RT only — construction allocates the cell
/// and the retire channel.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// let (_writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
/// assert_eq!(*reader.read(), 1);
/// ```
#[must_use]
pub fn slot<T: Send + Sync + 'static>(
    initial: Arc<T>,
    retire_capacity: usize,
) -> (SnapshotWriter<T>, SnapshotReader<T>) {
    debug_assert!(retire_capacity > 0, "slot retire_capacity must be non-zero");
    let (retirer, collector) = retire_channel(retire_capacity);
    let cell = Arc::new(ArcSwap::new(initial));
    (
        SnapshotWriter {
            cell: Arc::clone(&cell),
            collector,
        },
        SnapshotReader {
            cell,
            retirer: RefCell::new(retirer),
        },
    )
}

/// Writing endpoint of a snapshot slot — owned by the non-RT side.
///
/// `Send` when `T: Send + Sync` — the endpoint is meant to move to
/// the driving thread. `!Sync`: single-threaded ownership inherited
/// from the wrapped collector. Created together with its
/// [`SnapshotReader`] counterpart by [`slot`].
///
/// The wrapped [`DropCollector`] is the single drain path for both
/// retirement sources: `Arc`s retired by [`SnapshotGuard`] drops
/// (channel-received) and `Arc`s displaced by
/// [`publish`](Self::publish) (local pending queue via the
/// crate-internal `push_local` seam).
///
/// **RT contract:** every method is non-RT only — `publish` queues
/// the displaced `Arc` and `collect` runs the deferred destructors,
/// so the writer must never live on the RT thread.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
/// writer.publish(Arc::new(2_u32));
/// let guard = reader.read();
/// assert_eq!(*guard, 2);
/// // The displaced initial `Arc` is waiting in the drain path.
/// assert_eq!(writer.collect(), 1);
/// ```
#[derive(Debug)]
pub struct SnapshotWriter<T> {
    cell: Arc<ArcSwap<T>>,
    collector: DropCollector<Arc<T>>,
}

impl<T: Send + Sync + 'static> SnapshotWriter<T> {
    /// Atomically replaces the current snapshot.
    ///
    /// The swap itself is non-blocking; the displaced old `Arc` is
    /// handed to the collector's local pending queue (the
    /// crate-internal `push_local` seam) instead of being dropped
    /// here — routing it through the same drain path keeps
    /// deallocation inside [`collect`](Self::collect). Readers
    /// holding older guards keep their snapshot; subsequent
    /// [`read`](SnapshotReader::read)s observe the published one.
    ///
    /// **RT contract:** non-RT only — the swap is non-blocking but
    /// the pending `Vec` may allocate, and the displaced `Arc`'s
    /// eventual dealloc must stay on the drain side.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
    /// writer.publish(Arc::new(2_u32));
    /// writer.publish(Arc::new(3_u32));
    /// // Both displaced `Arc`s wait in the drain path.
    /// assert_eq!(writer.collect(), 2);
    /// assert_eq!(*reader.read(), 3);
    /// ```
    pub fn publish(&mut self, snapshot: Arc<T>) {
        let displaced = self.cell.swap(snapshot);
        self.collector.push_local(displaced);
    }

    /// Drops every `Arc` retired so far — both guard-release
    /// retirements received through the channel and
    /// `publish`-displaced `Arc`s in the local pending queue —
    /// running each destructor on the calling thread. Returns the
    /// number of dropped objects.
    ///
    /// The driving non-RT thread calls this periodically (e.g. each
    /// control tick, or before `publish` batches) to keep the retire
    /// channel below `retire_capacity`.
    ///
    /// **RT contract:** non-RT only — runs destructor work,
    /// including deallocation, bounded by the queued count.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
    /// drop(reader.read()); // guard release retires its `Arc`
    /// assert_eq!(writer.collect(), 1);
    /// assert_eq!(writer.collect(), 0);
    /// ```
    pub fn collect(&mut self) -> usize {
        self.collector.drain()
    }

    /// Returns the cumulative number of `Arc`s the reader side forgot
    /// because the retire channel was full — the saturation signal of
    /// the slot's bounded-leak overflow policy (guard drops retire
    /// their `Arc` through the reader's `Retirer`; a full channel
    /// forgets it via `mem::forget` and counts it here).
    ///
    /// This is the non-RT-side observation point: poll it next to
    /// [`collect`](Self::collect). A zero count is the healthy steady
    /// state; any non-zero or growing value means `retire_capacity`
    /// or the collect cadence is undersized for the actual guard-drop
    /// rate, and each forgotten `Arc` keeps its snapshot alive until
    /// process exit — enlarge the slot's retire capacity or collect
    /// more often.
    ///
    /// **RT contract:** non-RT only — a monitoring accessor for the
    /// writing side (an atomic load, technically non-blocking).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 1);
    /// drop(reader.read()); // fills the retire channel (capacity 1)
    /// drop(reader.read()); // overflow — forgotten, counted
    /// assert_eq!(writer.retire_overflows(), 1);
    /// ```
    #[must_use]
    pub fn retire_overflows(&self) -> usize {
        self.collector.overflows()
    }
}

/// Reading endpoint of a snapshot slot — the RT-side handle.
///
/// `Send` when `T: Send + Sync` — the endpoint is meant to move to
/// the single RT thread. `!Sync`: the interior `RefCell` makes it
/// single-threaded, dedicated to handover to one RT thread. Created
/// together with its [`SnapshotWriter`] counterpart by [`slot`].
///
/// **RT contract:** [`read`](Self::read) is RT-suitable;
/// construction via [`slot`] is non-RT only.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// let (_writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
/// let guard = reader.read();
/// assert_eq!(*guard, 1);
/// ```
#[derive(Debug)]
pub struct SnapshotReader<T> {
    cell: Arc<ArcSwap<T>>,
    retirer: RefCell<Retirer<Arc<T>>>,
}

impl<T: Send + Sync + 'static> SnapshotReader<T> {
    /// Returns a [`SnapshotGuard`] referencing the current snapshot.
    ///
    /// Lock-free load plus an `Arc` clone — no allocation, no
    /// blocking (an `arc-swap` load can in theory retry during a
    /// rare concurrent `publish`; the single-writer low-frequency
    /// update pattern keeps that bounded).
    ///
    /// Takes `&self`, not `&mut`: the interior `RefCell` lets
    /// multiple guards coexist — an `&mut` receiver would make a
    /// live guard block the next `read`, conflicting with the
    /// concurrent-guards contract.
    ///
    /// **RT contract:** RT-suitable — lock-free load + `Arc` clone,
    /// bounded. Allocation-free *after warm-up*: the first `read` on
    /// a given RT thread for a given slot may allocate an internal
    /// debt node, so call `read` once during setup before the RT
    /// contract interval.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
    /// let current = reader.read();
    /// writer.publish(Arc::new(2_u32));
    /// assert_eq!(*reader.read(), 2);
    /// // A guard taken before the publish still sees its snapshot.
    /// assert_eq!(*current, 1);
    /// ```
    #[must_use]
    pub fn read(&self) -> SnapshotGuard<'_, T> {
        SnapshotGuard {
            inner: Some(self.cell.load_full()),
            retirer: &self.retirer,
        }
    }
}

/// Reference guard for a snapshot read — dereferences to `T`.
///
/// `T: Send + Sync + 'static` mirrors the [`slot`] contract — the
/// bound also lets `Drop` route the `Arc` through [`Retirer`].
///
/// The `&'a RefCell<Retirer<Arc<T>>>` field makes the guard `!Send`
/// and `!Sync`: it cannot escape the RT thread it was taken on.
/// Obtained from [`SnapshotReader::read`]; several guards may
/// coexist (their `Drop`s take the `RefCell`'s momentary
/// `borrow_mut` sequentially, so they never contend).
///
/// **RT contract:** RT-suitable — `Drop` never runs `T`'s destructor
/// or deallocates inline: it sends the inner `Arc` to the reader's
/// [`Retirer`] (`try_push` plus a `mem::forget` bounded-leak on a
/// full channel — the overflow policy `Retirer` already owns). The
/// deferred drop runs on the non-RT side via
/// [`SnapshotWriter::collect`].
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// let (mut writer, reader) = ruviform_rt::slot(Arc::new(1_u32), 4);
/// {
///     let guard = reader.read();
///     assert_eq!(*guard, 1);
/// } // The `Arc` goes to the retire channel — nothing drops here.
/// assert_eq!(writer.collect(), 1);
/// ```
#[derive(Debug)]
pub struct SnapshotGuard<'a, T: Send + Sync + 'static> {
    inner: Option<Arc<T>>,
    retirer: &'a RefCell<Retirer<Arc<T>>>,
}

impl<T: Send + Sync + 'static> core::ops::Deref for SnapshotGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // `inner` is `Some` for the guard's whole lifetime —
        // `Drop::drop` is the only place that takes it, and it runs
        // after the last possible `deref` call.
        self.inner
            .as_deref()
            .expect("SnapshotGuard holds its Arc until drop")
    }
}

impl<T: Send + Sync + 'static> Drop for SnapshotGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            // Momentary `borrow_mut`: guard drops are sequential on
            // the single owning thread (the reader is `!Sync`), so
            // this can never meet another live borrow.
            self.retirer.borrow_mut().retire(inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc_counter;

    /// Compile-time bound check: `T` must be `Send`. (`expect_*`
    /// naming — helper names must not contain the `rt_` substring
    /// reserved for the RT-contract test lane.)
    fn expect_send<T: Send>() {}

    /// RT contract: `publish` makes the new snapshot observable to
    /// subsequent `read`s, while a guard taken earlier keeps the
    /// snapshot it loaded.
    #[test]
    fn rt_snapshot_publish_visibility() {
        let (mut writer, reader) = slot(Arc::new(1_u32), 4);

        let before = reader.read();
        assert_eq!(*before, 1);

        writer.publish(Arc::new(2_u32));
        assert_eq!(*reader.read(), 2);
        assert_eq!(*before, 1);
    }

    /// RT contract: dropping a guard never deallocates the snapshot
    /// on the reading thread — the inner `Arc` is routed to the
    /// retire channel (`strong_count` stays above the external
    /// reference after the drop) and is freed only by
    /// `SnapshotWriter::collect` on the non-RT side.
    #[test]
    fn rt_snapshot_guard_drop_routes_to_retire() {
        let tracked = Arc::new(0_u32);
        let (mut writer, reader) = slot(Arc::clone(&tracked), 4);
        // The external `Arc` plus the slot cell.
        assert_eq!(Arc::strong_count(&tracked), 2);

        {
            let guard = reader.read();
            assert_eq!(*guard, 0);
            assert_eq!(Arc::strong_count(&tracked), 3);
        }
        // The guard retired its `Arc` instead of dropping it — the
        // `Arc` is still alive inside the channel (an inline drop
        // would have taken the count back to 2).
        assert_eq!(Arc::strong_count(&tracked), 3);

        assert_eq!(writer.collect(), 1);
        // The deferred drop ran on the collecting side — the cell
        // still holds its own reference.
        assert_eq!(Arc::strong_count(&tracked), 2);
    }

    /// RT contract: an `Arc` displaced by `publish` is routed into
    /// the same drain path (`DropCollector::push_local`) — it is not
    /// dropped inside `publish`, and `collect` frees it.
    #[test]
    fn rt_snapshot_displaced_arc_freed_on_collect() {
        let displaced = Arc::new(0_u32);
        let (mut writer, _reader) = slot(Arc::clone(&displaced), 4);
        assert_eq!(Arc::strong_count(&displaced), 2);

        writer.publish(Arc::new(1_u32));
        // Displaced into the pending queue — still alive, not
        // dropped inside `publish`.
        assert_eq!(Arc::strong_count(&displaced), 2);

        assert_eq!(writer.collect(), 1);
        assert_eq!(Arc::strong_count(&displaced), 1);
    }

    /// RT contract: `read` and guard `Drop` perform no allocation or
    /// deallocation — measured inside a counting-allocator window
    /// after warm-up (the retired `Arc` lands in the channel, which
    /// `collect` confirms).
    #[test]
    fn rt_snapshot_read_no_alloc() {
        let (mut writer, reader) = slot(Arc::new(0_u32), 4);

        // Warm the read/drop path outside the measured window.
        drop(reader.read());
        assert_eq!(writer.collect(), 1);

        alloc_counter::enable();
        drop(reader.read());
        alloc_counter::disable();

        assert_eq!(alloc_counter::count(), 0);
        assert_eq!(writer.collect(), 1);
    }

    /// Thread-boundary bounds are API contract: the writer and the
    /// reader are `Send` when `T: Send + Sync` (each crosses to its
    /// own thread — the non-RT driver and the single RT thread). The
    /// `!Sync` of both endpoints (collector / `RefCell`) and the
    /// guard's `!Send + !Sync` (`&RefCell` field) are asserted by
    /// signature review — stable Rust cannot negate a bound in a
    /// test.
    #[test]
    fn rt_endpoint_send_bounds() {
        expect_send::<SnapshotWriter<u8>>();
        expect_send::<SnapshotReader<u8>>();
    }

    /// `read` takes `&self` — not `&mut` — and the interior
    /// `RefCell` lets several guards coexist; their drops take the
    /// momentary `borrow_mut` sequentially and each retires through
    /// the channel. (Non-RT test — the name keeps the `rt_`
    /// substring out of the RT-contract lane.)
    #[test]
    fn multiple_guards_coexist() {
        let (mut writer, reader) = slot(Arc::new(1_u32), 8);

        let first = reader.read();
        let second = reader.read();
        let third = reader.read();
        assert_eq!((*first, *second, *third), (1, 1, 1));
        drop(first);
        drop(second);
        drop(third);

        // All three guard drops retired through the channel.
        assert_eq!(writer.collect(), 3);
    }

    /// On a full retire channel the guard drop falls back to the
    /// `Retirer`-internal `mem::forget` bounded-leak — the
    /// overflowed `Arc` stays allocated (still counted) instead of
    /// being dropped on the reading thread.
    #[test]
    fn guard_drop_full_channel_leaks() {
        let tracked = Arc::new(0_u32);
        let (mut writer, reader) = slot(Arc::clone(&tracked), 1);
        assert_eq!(Arc::strong_count(&tracked), 2);

        // The first guard drop fills the channel; the second
        // overflows into the bounded leak.
        drop(reader.read());
        drop(reader.read());
        // External + cell + queued + leaked — the overflowed `Arc`
        // was forgotten, not dropped.
        assert_eq!(Arc::strong_count(&tracked), 4);

        assert_eq!(writer.collect(), 1);
        // The leaked `Arc` remains allocated.
        assert_eq!(Arc::strong_count(&tracked), 3);
    }

    /// RT contract: guard-drop retirements that overflow the retire
    /// channel are counted by the shared counter — the writer's
    /// `retire_overflows` is the non-RT-side saturation signal.
    #[test]
    fn rt_snapshot_retire_overflows_observable() {
        let (mut writer, reader) = slot(Arc::new(0_u32), 1);
        assert_eq!(writer.retire_overflows(), 0);

        // First guard drop fills the channel; the second overflows.
        drop(reader.read());
        drop(reader.read());
        assert_eq!(writer.retire_overflows(), 1);

        assert_eq!(writer.collect(), 1);
        // `collect` does not reset the signal.
        assert_eq!(writer.retire_overflows(), 1);
    }

    /// Teardown contract: the reader is independent of the writer's
    /// lifetime — after the writer drops, `read` still returns the
    /// last published snapshot (the safe ordering is to drop the
    /// reader on the non-RT side; dropping it *after* the writer on
    /// an RT thread is what the module doc forbids).
    #[test]
    fn rt_snapshot_reader_outlives_writer() {
        let (mut writer, reader) = slot(Arc::new(0_u32), 4);
        writer.publish(Arc::new(42_u32));

        drop(writer);
        assert_eq!(*reader.read(), 42);
    }

    /// `retire_capacity` must be non-zero — the degenerate
    /// always-overflow slot is rejected in debug builds.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "capacity must be non-zero")]
    fn zero_retire_capacity_panics_in_debug() {
        let _unused = slot(Arc::new(0_u32), 0);
    }
}
