//! Experimental observability scaffolding for the Issue #104 RT
//! observation-cost `PoC` — the measurement surface exercised by
//! `benches/rt_observability_poc.rs`.
//!
//! This module is a temporary experiment bridge, reachable via
//! `#[doc(hidden)] pub mod observability_poc` — the same containment
//! shape as `ruviform-math`'s `simd_poc`. It is **not** part of the
//! crate's public API contract and is **not** covered by the
//! production RT contract declared in the crate-level docs: the items
//! here are measurement scaffolding supplying decision data for the
//! #103 "要 `PoC`" verdicts (`telemetry` / `rt_bridge` / `metrics` /
//! `profiling` / `logging`), not a settled production API. Whether any
//! shape is promoted is decided by the trigger issues consuming the
//! `PoC` report.
//!
//! Shapes modelled (canonical references:
//! `docs/designs/foundation-observability-minimal-api.md`):
//!
//! - [`counter`] / [`Counter`] / [`CounterReader`] — the
//!   `foundation::telemetry` candidate sketch, generalizing the
//!   `retire_channel` shared `Arc<AtomicUsize>` overflow counter
//!   (`Relaxed` bookkeeping: `increment` = `fetch_add`, `read` =
//!   `load`).
//! - [`bridge`] / [`RtRecorder`] / [`RecordSink`] — the
//!   `observability::rt_bridge` candidate sketch: a fixed-capacity,
//!   fully pre-allocated SPSC ring built on the crate's private `rtrb`
//!   dependency, drop-on-full + shared overflow counter, bounded
//!   [`drain`](RecordSink::drain). Construction is non-RT only and
//!   `capacity` is a caller contract, like `retire_channel`'s.
//! - [`ObsRecord`] — a fixed-size `Copy` record following the
//!   `reserved: [u64; 2]` opaque-extension convention (the constructor
//!   initializes `[0; 2]`; contents are read through the
//!   [`reserved()`](ObsRecord::reserved) accessor only and callers do
//!   not interpret them).
//! - Overflow-policy variants — [`record_drop`](RtRecorder::record_drop)
//!   / [`record`](RtRecorder::record) (count) /
//!   [`record_aggregate`](RtRecorder::record_aggregate) expose the
//!   drop / count / aggregate policies on the same full-channel
//!   scenario so the harness can compare per-policy cost.
//! - Timestamp helpers — [`instant_to_epoch_nanos`] /
//!   [`timestamped_record`] perform the `instant_to_media_time`-shaped
//!   math (`Instant` → epoch-relative nanos → `i64` → record) on
//!   `std::time::Instant` as a surrogate. The real device path is
//!   `ruviform-audio-device`'s `StreamInstant`, which exists only
//!   inside a live backend callback and is out of scope here.
//!
//! `unsafe` count: 0. No `pub enum`. `std` + the crate-internal `rtrb`
//! dependency only.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Builds a shared counter pair — a [`Counter`] for the writing
/// (instrumented) side and a [`CounterReader`] for the non-RT
/// observing side.
///
/// **`PoC` scaffolding:** non-RT only — construction allocates the
/// shared `Arc` cell.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::counter;
///
/// let (counter, reader) = counter();
/// counter.increment();
/// assert_eq!(reader.read(), 1);
/// ```
#[must_use]
pub fn counter() -> (Counter, CounterReader) {
    let shared = Arc::new(AtomicUsize::new(0));
    (
        Counter {
            inner: Arc::clone(&shared),
        },
        CounterReader { inner: shared },
    )
}

/// Writing side of a shared observability counter — the
/// `foundation::telemetry` sketch shape ("書き手側"). `Send + Sync`
/// through the shared `Arc`; [`reader`](Self::reader) derives
/// additional observing handles on the same cell.
///
/// **`PoC` scaffolding:** every method is wait-free, no-alloc and
/// bounded — the prepared-counter instrumentation step under
/// measurement.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::counter;
///
/// let (counter, reader) = counter();
/// counter.increment();
/// assert_eq!(counter.read(), 1);
/// assert_eq!(counter.reader().read(), 1);
/// ```
#[derive(Debug)]
pub struct Counter {
    inner: Arc<AtomicUsize>,
}

impl Counter {
    /// Increments the counter — a single `Relaxed` `fetch_add`.
    /// The counter wraps modulo `usize::MAX + 1` on overflow — the
    /// same `fetch_add` bookkeeping the crate's existing primitives
    /// (e.g. `retire`'s overflow counter) use; a saturation policy
    /// is deliberately not chosen at `PoC` scope and is deferred to
    /// the promotion issue.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    pub fn increment(&self) {
        self.inner.fetch_add(1, Ordering::Relaxed);
    }

    /// Reads the current count — a `Relaxed` load.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::observability_poc::counter;
    ///
    /// let (counter, _reader) = counter();
    /// assert_eq!(counter.read(), 0);
    /// ```
    #[must_use]
    pub fn read(&self) -> usize {
        self.inner.load(Ordering::Relaxed)
    }

    /// Derives an additional observing handle on the same cell — the
    /// sketch's "clone で Reader と共有" shape. An `Arc` refcount bump,
    /// not an allocation.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn reader(&self) -> CounterReader {
        CounterReader {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Non-RT observing side of a shared counter — load only, matching
/// the sketch's "非 RT 観測側" role.
///
/// **`PoC` scaffolding:** `read` is wait-free, no-alloc and bounded.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::counter;
///
/// let (counter, reader) = counter();
/// counter.increment();
/// assert_eq!(reader.read(), 1);
/// ```
#[derive(Debug)]
pub struct CounterReader {
    inner: Arc<AtomicUsize>,
}

impl CounterReader {
    /// Reads the current count — a `Relaxed` load.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn read(&self) -> usize {
        self.inner.load(Ordering::Relaxed)
    }
}

/// Builds an `rt_bridge`-shaped record channel: a fixed-capacity,
/// fully pre-allocated SPSC ring plus a shared overflow counter and a
/// shared aggregate cell.
///
/// `R: Copy + Send + 'static` mirrors `retire_channel`'s endpoint
/// contract — records carry no destructor or owned resource, so the
/// full-channel policy is *discard* (not `mem::forget` leak).
/// `capacity` is a caller contract: it bounds the number of records
/// buffered per [`drain`](RecordSink::drain) cycle before overflow
/// recording engages.
///
/// **`PoC` scaffolding:** non-RT only — construction allocates the ring
/// and the shared cells.
///
/// # Panics
///
/// Panics when `capacity` is zero — a degenerate always-full bridge,
/// rejected in all builds (construction is a non-RT call).
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::bridge;
///
/// let (mut recorder, mut sink) = bridge::<u64>(4);
/// recorder.record(7);
/// assert_eq!(sink.drain(|_| {}), 1);
/// ```
#[must_use]
pub fn bridge<R: Copy + Send + 'static>(capacity: usize) -> (RtRecorder<R>, RecordSink<R>) {
    assert!(capacity > 0, "bridge capacity must be non-zero");
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    let overflows = Arc::new(AtomicUsize::new(0));
    let aggregate = Arc::new(AtomicU64::new(0));
    (
        RtRecorder {
            inner: producer,
            overflows: Arc::clone(&overflows),
            aggregate: Arc::clone(&aggregate),
        },
        RecordSink {
            inner: consumer,
            overflows,
            aggregate,
        },
    )
}

/// Recording endpoint of a `bridge` — the RT-side handle.
///
/// `Send` when `R: Send` — the endpoint is meant to move to the RT
/// thread. `!Sync`: single-threaded ownership, inherited from the
/// wrapped `rtrb` endpoint. Created together with its [`RecordSink`]
/// counterpart by [`bridge`].
///
/// **`PoC` scaffolding:** every method is wait-free, no-alloc and
/// bounded — the measured RT record path.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::bridge;
///
/// let (mut recorder, _sink) = bridge::<u64>(1);
/// recorder.record(1);
/// recorder.record(2); // Full — dropped and counted.
/// assert_eq!(recorder.overflows(), 1);
/// ```
#[derive(Debug)]
pub struct RtRecorder<R> {
    inner: rtrb::Producer<R>,
    overflows: Arc<AtomicUsize>,
    aggregate: Arc<AtomicU64>,
}

impl<R: Copy + Send + 'static> RtRecorder<R> {
    /// Records `rec` — the canonical `rt_bridge` shape, a drop-on-full
    /// with counting. On a full ring the record is discarded (for
    /// `R: Copy` the discard is a semantic no-op — no destructor runs)
    /// and the shared overflow counter is incremented. The counter
    /// wraps modulo `usize::MAX + 1` on overflow — the same
    /// `fetch_add` bookkeeping as `retire`'s overflow counter; a
    /// saturation policy is deferred to the promotion issue.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    pub fn record(&mut self, rec: R) {
        // Dropping the returned `Err` discards the record — for
        // `R: Copy` there is no destructor, so the discard is free.
        if self.inner.push(rec).is_err() {
            self.overflows.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Overflow-policy variant — **silent drop**. On a full ring the
    /// record is discarded with no bookkeeping; the policy cost this
    /// variant isolates is the full-channel check itself.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    pub fn record_drop(&mut self, rec: R) {
        // Dropping the returned `Err` discards the record — the
        // silent-drop policy performs no bookkeeping.
        let _ = self.inner.push(rec);
    }

    /// Overflow-policy variant — **aggregate**. On a full ring the
    /// discarded record is folded via `fold` into the shared `u64`
    /// aggregate instead of bumping an occurrence count. The cell
    /// wraps modulo `u64::MAX + 1` on overflow — same policy note as
    /// the overflow counter.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded (`fold` is
    /// caller math — keep it allocation-free on the measured path).
    pub fn record_aggregate(&mut self, rec: R, fold: impl FnOnce(R) -> u64) {
        if let Err(rtrb::PushError::Full(rec)) = self.inner.push(rec) {
            let value = fold(rec);
            self.aggregate.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Cumulative number of records dropped by [`record`](Self::record)
    /// because the ring was full — the shared saturation signal,
    /// observable from either endpoint.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn overflows(&self) -> usize {
        self.overflows.load(Ordering::Relaxed)
    }

    /// Cumulative folded value from
    /// [`record_aggregate`](Self::record_aggregate) overflows —
    /// observable from either endpoint.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn aggregate(&self) -> u64 {
        self.aggregate.load(Ordering::Relaxed)
    }
}

/// Collecting endpoint of a `bridge` — owned by the non-RT side.
///
/// `Send` when `R: Send` — the endpoint is meant to move to the
/// collecting thread. `!Sync`: single-threaded ownership, inherited
/// from the wrapped `rtrb` endpoint. Created together with its
/// [`RtRecorder`] counterpart by [`bridge`].
///
/// **`PoC` scaffolding:** `drain`'s pop loop is wait-free, no-alloc
/// and bounded (at most `capacity` pops per call) — the `f`
/// callback's own allocation, blocking and work are the caller's
/// contract. The accessors are non-blocking atomic loads.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::bridge;
///
/// let (mut recorder, mut sink) = bridge::<u64>(2);
/// recorder.record(5);
/// let mut seen = Vec::new();
/// assert_eq!(sink.drain(|rec| seen.push(rec)), 1);
/// assert_eq!(seen, [5]);
/// ```
#[derive(Debug)]
pub struct RecordSink<R> {
    inner: rtrb::Consumer<R>,
    overflows: Arc<AtomicUsize>,
    aggregate: Arc<AtomicU64>,
}

impl<R: Copy + Send + 'static> RecordSink<R> {
    /// Drains pending records, handing each to `f` and returning the
    /// drained count.
    ///
    /// Pops at most `capacity` records per call — the same bounded-loop
    /// rule as [`Consumer::pop_latest`](crate::Consumer::pop_latest) —
    /// so the work stays bounded even while a recorder pushes
    /// concurrently.
    ///
    /// **`PoC` scaffolding:** the pop loop is wait-free, no-alloc
    /// and bounded by `capacity` — `f`'s allocation, blocking and
    /// work are the caller's contract (the measured harness passes
    /// a trivial callback).
    pub fn drain(&mut self, mut f: impl FnMut(R)) -> usize {
        let mut drained = 0_usize;
        for _ in 0..self.inner.buffer().capacity() {
            match self.inner.pop() {
                Ok(rec) => f(rec),
                Err(rtrb::PopError::Empty) => break,
            }
            drained += 1;
        }
        drained
    }

    /// Cumulative number of records the [`RtRecorder`] dropped because
    /// the ring was full — the non-RT observation point for the
    /// shared saturation signal. Poll it next to [`drain`](Self::drain).
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn overflows(&self) -> usize {
        self.overflows.load(Ordering::Relaxed)
    }

    /// Cumulative folded value from the recorder's
    /// [`record_aggregate`](RtRecorder::record_aggregate) overflows —
    /// the non-RT observation point for the aggregate cell.
    ///
    /// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
    #[must_use]
    pub fn aggregate(&self) -> u64 {
        self.aggregate.load(Ordering::Relaxed)
    }
}

/// Fixed-size `Copy` observability record — the `PoC`'s concrete shape
/// for the `reserved: [u64; 2]` convention: a `kind` identifier, an
/// `i64` timestamp in nanoseconds, a `u64` value, and an opaque
/// extension tail.
///
/// `#[non_exhaustive]` plays the struct-side role of the public-enum
/// policy: the private fields already force construction through
/// [`new`](Self::new), and `reserved` accepts future payload without
/// breaking downstream sites. Callers must not interpret the tail.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::ObsRecord;
///
/// let rec = ObsRecord::new(3, 1_500, 42);
/// assert_eq!((rec.kind(), rec.timestamp_nanos(), rec.value()), (3, 1_500, 42));
/// assert_eq!(rec.reserved(), [0, 0]);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObsRecord {
    kind: u32,
    timestamp_nanos: i64,
    value: u64,
    reserved: [u64; 2],
}

impl ObsRecord {
    /// Builds a record; `reserved` initializes to `[0; 2]` per the
    /// opaque-extension convention.
    #[must_use]
    pub const fn new(kind: u32, timestamp_nanos: i64, value: u64) -> Self {
        Self {
            kind,
            timestamp_nanos,
            value,
            reserved: [0; 2],
        }
    }

    /// Record kind identifier (schema-defined meaning).
    #[must_use]
    pub fn kind(self) -> u32 {
        self.kind
    }

    /// Timestamp in nanoseconds relative to the record's epoch.
    #[must_use]
    pub fn timestamp_nanos(self) -> i64 {
        self.timestamp_nanos
    }

    /// Recorded value (schema-defined meaning).
    #[must_use]
    pub fn value(self) -> u64 {
        self.value
    }

    /// Opaque extension tail — initialized to `[0; 2]` by
    /// [`new`](Self::new); callers must not interpret the contents.
    #[must_use]
    pub fn reserved(self) -> [u64; 2] {
        self.reserved
    }
}

/// `Instant` → epoch-relative nanos → `i64` — the same conversion
/// shape as `ruviform-audio-device`'s `instant_to_media_time`
/// (saturating delta → `u128` nanos → `i64`), evaluated on
/// `std::time::Instant` as a surrogate for the backend `StreamInstant`
/// (which exists only inside a live device callback).
///
/// Returns `None` only on conversion overflow (~292 years of uptime).
///
/// **`PoC` scaffolding:** wait-free, no-alloc, bounded — pure math.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::instant_to_epoch_nanos;
/// use std::time::Instant;
///
/// let epoch = Instant::now();
/// assert_eq!(instant_to_epoch_nanos(epoch, epoch), Some(0));
/// ```
#[must_use]
pub fn instant_to_epoch_nanos(instant: Instant, epoch: Instant) -> Option<i64> {
    i64::try_from(instant.saturating_duration_since(epoch).as_nanos()).ok()
}

/// The full record-side acquisition path: `Instant::now()` →
/// epoch-relative nanos → `i64` → [`ObsRecord`].
///
/// A doubly-saturated conversion collapses to `0`, mirroring
/// `map_output_timestamp`'s non-optional `callback` handling — no
/// panic on the record path.
///
/// **`PoC` scaffolding:** wait-free, no-alloc, bounded.
///
/// # Examples
///
/// ```
/// use ruviform_rt::observability_poc::timestamped_record;
/// use std::time::Instant;
///
/// let epoch = Instant::now();
/// let rec = timestamped_record(1, 99, epoch);
/// assert_eq!(rec.kind(), 1);
/// assert_eq!(rec.value(), 99);
/// ```
#[must_use]
pub fn timestamped_record(kind: u32, value: u64, epoch: Instant) -> ObsRecord {
    let nanos = instant_to_epoch_nanos(Instant::now(), epoch).unwrap_or(0);
    ObsRecord::new(kind, nanos, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc_counter;

    /// Compile-time bound check: `T` must be `Send`. (`expect_*`
    /// naming — helper names must not contain the `rt_` substring
    /// reserved for the RT-contract test lane.)
    fn expect_send<T: Send>() {}

    /// Compile-time bound check: `T` must be `Send + Sync`.
    /// (`expect_*` naming — helper names must not contain the `rt_`
    /// substring reserved for the RT-contract test lane.)
    fn expect_send_sync<T: Send + Sync>() {}

    /// Scaffolding semantics: `record` → `drain` preserves FIFO order
    /// and returns the drained count; empty drains return 0.
    #[test]
    fn bridge_records_and_drains_fifo() {
        let (mut recorder, mut sink) = bridge::<ObsRecord>(4);

        recorder.record(ObsRecord::new(1, 10, 100));
        recorder.record(ObsRecord::new(2, 20, 200));

        let mut seen = Vec::new();
        assert_eq!(sink.drain(|rec| seen.push(rec.value())), 2);
        assert_eq!(seen, [100, 200]);
        assert_eq!(sink.drain(|_| {}), 0);
        assert_eq!(recorder.overflows(), 0);
        assert_eq!(sink.overflows(), 0);
    }

    /// Scaffolding semantics: on a full ring `record` discards the
    /// record and increments the shared counter on both endpoints;
    /// `drain` does not reset the signal.
    #[test]
    fn bridge_drop_on_full_counts() {
        let (mut recorder, mut sink) = bridge::<u64>(2);
        recorder.record(1);
        recorder.record(2);
        recorder.record(3); // overflow — dropped and counted
        recorder.record(4);

        assert_eq!(recorder.overflows(), 2);
        assert_eq!(sink.overflows(), 2);
        assert_eq!(sink.drain(|_| {}), 2);
        assert_eq!(sink.overflows(), 2);
    }

    /// Scaffolding semantics: `drain` is bounded — one call can never
    /// pop more than `capacity` records.
    #[test]
    fn bridge_drain_bounded_by_capacity() {
        let (mut recorder, mut sink) = bridge::<u64>(8);
        for value in 0..8 {
            recorder.record(value);
        }
        assert_eq!(sink.drain(|_| {}), 8);
        assert!(sink.drain(|_| {}) <= 8);
    }

    /// Scaffolding semantics: the three overflow-policy variants move
    /// their own bookkeeping on the same full-ring scenario.
    #[test]
    fn overflow_policy_variants() {
        // Silent drop: nothing is counted or folded.
        let (mut rec_drop, mut sink_drop) = bridge::<ObsRecord>(1);
        rec_drop.record(ObsRecord::new(0, 0, 0));
        rec_drop.record_drop(ObsRecord::new(1, 1, 10));
        assert_eq!(rec_drop.overflows(), 0);
        assert_eq!(rec_drop.aggregate(), 0);
        assert_eq!(sink_drop.drain(|_| {}), 1);

        // Count (canonical `record`): occurrences are counted.
        let (mut rec_count, sink_count) = bridge::<ObsRecord>(1);
        rec_count.record(ObsRecord::new(0, 0, 0));
        rec_count.record(ObsRecord::new(1, 1, 1));
        assert_eq!(rec_count.overflows(), 1);
        assert_eq!(sink_count.overflows(), 1);

        // Aggregate: the dropped records' values are folded, not
        // counted — the occurrence counter stays at 0.
        let (mut rec_agg, sink_agg) = bridge::<ObsRecord>(1);
        rec_agg.record(ObsRecord::new(0, 0, 0));
        rec_agg.record_aggregate(ObsRecord::new(1, 0, 7), ObsRecord::value);
        rec_agg.record_aggregate(ObsRecord::new(1, 0, 3), ObsRecord::value);
        assert_eq!(rec_agg.aggregate(), 10);
        assert_eq!(sink_agg.aggregate(), 10);
        assert_eq!(rec_agg.overflows(), 0);
    }

    /// Scaffolding semantics: the counter pair shares one cell —
    /// writer increments are observable through every reader.
    #[test]
    fn counter_pair_increments_and_reads() {
        let (counter, reader) = counter();
        counter.increment();
        counter.increment();
        assert_eq!(counter.read(), 2);
        assert_eq!(reader.read(), 2);
        assert_eq!(counter.reader().read(), 2);
    }

    /// Scaffolding semantics: `timestamped_record` fills the
    /// `instant_to_epoch_nanos` conversion into the record, and
    /// `ObsRecord::new` initializes the opaque tail.
    #[test]
    fn timestamp_helpers_fill_record() {
        let epoch = Instant::now();
        let later = Instant::now();
        let nanos = instant_to_epoch_nanos(later, epoch).unwrap();
        assert!(nanos >= 0);
        // Saturating delta: an instant at-or-before the epoch maps to 0.
        assert_eq!(instant_to_epoch_nanos(epoch, later), Some(0));

        let rec = timestamped_record(7, 42, epoch);
        assert_eq!(rec.kind(), 7);
        assert_eq!(rec.value(), 42);
        assert!(rec.timestamp_nanos() >= 0);
        assert_eq!(rec.reserved(), [0, 0]);
    }

    /// Measurement-window purity (test-spec "測定窓の純粋性"): every
    /// scaffolding path the harness times — `Counter` ops, `record`
    /// and its policy variants, `drain`, `overflows`/`aggregate`
    /// reads, the timestamp helpers — performs zero alloc/dealloc
    /// inside an `alloc_counter` window. Fixture setup and warm-up
    /// happen before `enable()`.
    #[test]
    fn scaffolding_paths_alloc_free_in_window() {
        let (counter, reader) = counter();
        let (mut recorder, mut sink) = bridge::<ObsRecord>(4);
        let epoch = Instant::now();

        // Warm the record/drain path outside the measured window.
        recorder.record(ObsRecord::new(0, 0, 0));
        assert_eq!(sink.drain(|_| {}), 1);

        alloc_counter::enable();
        counter.increment();
        let counter_seen = counter.read();
        let reader_seen = reader.read();
        let extra_reader = counter.reader(); // `Arc` bump — not an allocation
        recorder.record(ObsRecord::new(1, 1, 1));
        recorder.record(ObsRecord::new(2, 2, 2));
        recorder.record(ObsRecord::new(3, 3, 3));
        recorder.record(ObsRecord::new(4, 4, 4));
        recorder.record(ObsRecord::new(5, 5, 5)); // full — drop + count
        recorder.record_drop(ObsRecord::new(6, 6, 6));
        recorder.record_aggregate(ObsRecord::new(7, 7, 7), ObsRecord::value);
        let drained = sink.drain(|rec| {
            std::hint::black_box(rec);
        });
        let overflows = recorder.overflows() + sink.overflows();
        let aggregate = recorder.aggregate() + sink.aggregate();
        let nanos = instant_to_epoch_nanos(Instant::now(), epoch);
        let stamped = timestamped_record(9, 9, epoch);
        alloc_counter::disable();

        assert_eq!(alloc_counter::count(), 0);
        // Consume the windowed values so the body is not dead code.
        assert_eq!((counter_seen, reader_seen, extra_reader.read()), (1, 1, 1));
        assert_eq!(drained, 4);
        assert_eq!(overflows, 2); // one drop counted, visible on both ends
        assert_eq!(aggregate, 14); // the folded 7, visible on both ends
        assert!(nanos.is_some());
        assert!(stamped.timestamp_nanos() >= 0);
    }

    /// Coverage-gap demonstration (test-spec "カバレッジ隙間の実証"):
    /// an `alloc_counter` window is a *thread-local* mechanism — the
    /// caller's window records zero events while a spawned thread
    /// allocates N > 0 times, and the spawned thread's own window does
    /// observe its allocations. The triple (caller = 0, spawned = N,
    /// N > 0) is printed for transcription into the `PoC` report.
    ///
    /// Ordering is pinned by two spin flags (zero-allocation
    /// signalling): `thread::spawn` runs before `enable()` because
    /// spawning allocates on the *caller* thread, `join` runs after
    /// `disable()` because it deallocates the thread packet on the
    /// caller thread — and in between, the spawned thread's
    /// allocations provably execute while the caller's window is open.
    #[test]
    fn alloc_window_covers_only_owning_thread() {
        use std::sync::atomic::AtomicBool;
        use std::thread;

        let begin = AtomicBool::new(false);
        let finished = AtomicBool::new(false);
        thread::scope(|scope| {
            let handle = scope.spawn(|| {
                // Wait until the caller's window is open, then run the
                // measured-alloc phase inside this thread's own window.
                while !begin.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                alloc_counter::enable();
                let mut owned = Vec::new();
                for value in 0..4_u32 {
                    owned.push(String::from("spawned") + &value.to_string());
                }
                drop(owned);
                alloc_counter::disable();
                let events = alloc_counter::count();
                finished.store(true, Ordering::Release);
                events
            });

            alloc_counter::enable(); // caller-side window
            begin.store(true, Ordering::Release);
            // The spawned thread's alloc phase runs entirely while this
            // window is open — its events must stay invisible here.
            while !finished.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
            alloc_counter::disable();
            let caller_events = alloc_counter::count();

            let spawned_events = handle.join().unwrap();
            println!(
                "alloc-gap demo: caller_window={caller_events} spawned_window={spawned_events} spawned_observed={}",
                spawned_events > 0
            );
            assert_eq!(caller_events, 0);
            assert!(spawned_events > 0);
        });
    }

    /// `capacity` must be non-zero — the degenerate always-full bridge
    /// is rejected unconditionally (non-RT construction).
    #[test]
    #[should_panic(expected = "capacity must be non-zero")]
    fn zero_capacity_rejected() {
        let _unused = bridge::<u64>(0);
    }

    /// Thread-boundary bounds are API shape: each endpoint is `Send`
    /// when `R: Send` (moves to its own thread), and the shared
    /// counter pair is `Send + Sync` so observing handles can be
    /// polled from any thread.
    #[test]
    fn endpoint_send_bounds() {
        expect_send::<Counter>();
        expect_send::<CounterReader>();
        expect_send_sync::<Counter>();
        expect_send_sync::<CounterReader>();
        expect_send::<RtRecorder<ObsRecord>>();
        expect_send::<RecordSink<ObsRecord>>();
    }
}
