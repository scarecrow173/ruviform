//! RT record bridge — a bounded, non-blocking path that moves fixed-size
//! records from one RT writer to a non-RT collector. Issue #280 (OBS-05).
//!
//! [`bridge`] builds a single-producer/single-consumer pair over
//! [`ruviform_rt::channel`]'s bounded SPSC ring — the workspace's single
//! ring implementation; this module only adds record envelopes and loss
//! accounting on top of it. The full-channel policy is **drop-on-full**:
//! a record that does not fit is discarded and counted, never recovered
//! and never implied to be recoverable.
//!
//! Loss is visible two ways: the shared [`dropped`](RtRecorder::dropped)
//! counter (whole-bridge totals, registrable as an OBS-02 counter source
//! via [`RecordSink::dropped_reader`]) and the per-record
//! [`ReceivedRecord::dropped_before`] envelope field (how many records
//! were dropped immediately before this one — positional loss a
//! collector or exporter can re-attach).
//!
//! # Session boundary
//!
//! | stage | operation | where |
//! | --- | --- | --- |
//! | `Prepare` | [`bridge`] | non-RT (the only allocation point) |
//! | `Publish` | move the [`RtRecorder`] to the RT producer | non-RT |
//! | `ON` | [`RtRecorder::try_record`], [`RtRecorder::dropped`],
//!   [`RtRecorder::recorded`] | RT |
//! | `OFF` | move the recorder back out of the RT section | non-RT |
//! | `Retire` | drop the recorder, then [`RecordSink::drain_final`]
//!   after writer quiescence | non-RT |
//! | `FinalDestroy` | drop the [`RecordSink`] — frees the ring | non-RT |
//!
//! "Writer quiescence" means the recorder has left the RT path (moved
//! out or dropped). `drain_final` is contractually restricted to that
//! state; plain [`RecordSink::drain`] is also legal while the writer is
//! live. Dropping either endpoint on an RT thread is a contract
//! violation.

use crate::core::{Counter, CounterReader, counter};
use ruviform_rt::{Consumer, Producer, channel};

/// Builds a bounded record bridge — an [`RtRecorder`] for the RT writer
/// and a [`RecordSink`] for the non-RT collector.
///
/// `capacity` is the ring's slot count. Construction is the bridge's only
/// allocation point.
///
/// **Non-RT only** — allocates the ring and the shared counter cells.
///
/// # Panics
///
/// Panics when `capacity == 0` (same contract as
/// [`ruviform_rt::channel`]).
///
/// # Examples
///
/// ```
/// use ruviform_observability::rt_bridge::bridge;
///
/// let (mut recorder, mut sink) = bridge::<u64>(8);
/// assert!(recorder.try_record(42));
/// let mut received = Vec::new();
/// sink.drain(|rec| received.push((rec.record(), rec.dropped_before())));
/// assert_eq!(received, vec![(42, 0)]);
/// ```
#[must_use]
pub fn bridge<R>(capacity: usize) -> (RtRecorder<R>, RecordSink<R>)
where
    R: Copy + Send + 'static,
{
    let (producer, consumer) = channel(capacity);
    let (recorded_w, recorded_r) = counter();
    let (dropped_w, dropped_r) = counter();
    (
        RtRecorder {
            producer,
            recorded: recorded_w,
            dropped: dropped_w,
            pending_dropped: 0,
        },
        RecordSink {
            consumer,
            recorded: recorded_r,
            dropped: dropped_r,
            drained_total: 0,
        },
    )
}

/// A record received at the sink — the payload plus positional loss.
///
/// `dropped_before` counts the records that were dropped immediately
/// before this one was accepted: `0` for no recent loss, `n` for a gap
/// of `n` dropped records between the previously delivered record and
/// this one.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceivedRecord<R> {
    record: R,
    dropped_before: u64,
}

impl<R> ReceivedRecord<R> {
    /// The delivered payload.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn record(&self) -> R
    where
        R: Copy,
    {
        self.record
    }

    /// Records dropped immediately before this one — **RT contract:**
    /// RT-suitable.
    #[must_use]
    pub const fn dropped_before(&self) -> u64 {
        self.dropped_before
    }
}

/// The RT writer endpoint — single-producer only.
///
/// `Send` when `R: Send`: the recorder is meant to cross to its producing
/// thread. `!Sync` and not `Clone` — the bridge is SPSC; multi-producer
/// is out of scope.
///
/// Only the methods below are RT-suitable. Moving a recorder *into* or
/// *out of* the RT path, cloning it, and dropping it are non-RT
/// operations.
pub struct RtRecorder<R> {
    producer: Producer<ReceivedRecord<R>>,
    recorded: Counter,
    dropped: Counter,
    /// Drops accrued since the last accepted record — burned into the
    /// next successful envelope's `dropped_before`.
    pending_dropped: u64,
}

impl<R> RtRecorder<R>
where
    R: Copy + Send + 'static,
{
    /// Tries to accept `record` into the bridge.
    ///
    /// On success the record is queued with the accrued `dropped_before`
    /// and `recorded` increments; returns `true`. On a full ring the
    /// record is **dropped** (never recoverable), `dropped` increments,
    /// the pending drop count accrues into the next accepted record's
    /// envelope, and `false` is returned.
    ///
    /// **RT contract:** RT-suitable — one bounded `try_push` plus at most
    /// two `Relaxed` counter updates. No alloc, no lock, no I/O, no
    /// subscriber.
    pub fn try_record(&mut self, record: R) -> bool {
        let envelope = ReceivedRecord {
            record,
            dropped_before: self.pending_dropped,
        };
        if self.producer.try_push(envelope).is_ok() {
            self.pending_dropped = 0;
            self.recorded.increment();
            true
        } else {
            self.pending_dropped += 1;
            self.dropped.increment();
            false
        }
    }

    /// Total records dropped on a full ring — shared `Relaxed` counter.
    ///
    /// **RT contract:** RT-suitable — wait-free `Relaxed` load.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.read()
    }

    /// Total records accepted — shared `Relaxed` counter.
    ///
    /// **RT contract:** RT-suitable — wait-free `Relaxed` load.
    #[must_use]
    pub fn recorded(&self) -> u64 {
        self.recorded.read()
    }
}

/// The non-RT collector endpoint — sole consumer and ring owner.
///
/// `Send` when `R: Send`; `!Sync` and not `Clone`. Dropping the sink
/// frees the ring — never drop it on an RT thread.
pub struct RecordSink<R> {
    consumer: Consumer<ReceivedRecord<R>>,
    recorded: CounterReader,
    dropped: CounterReader,
    drained_total: u64,
}

impl<R> RecordSink<R>
where
    R: Copy + Send + 'static,
{
    /// Delivers every queued record, FIFO, to `f`; returns the count
    /// delivered (bounded by the ring's capacity).
    ///
    /// Safe to call while the writer is live. Records the writer pushes
    /// concurrently may or may not be included — `lag()` afterwards
    /// reports what remains in flight.
    ///
    /// **Non-RT only** — bounded `try_pop` loop; no blocking, no alloc.
    pub fn drain(&mut self, mut f: impl FnMut(ReceivedRecord<R>)) -> usize {
        let mut n = 0;
        while let Ok(record) = self.consumer.try_pop() {
            f(record);
            n += 1;
        }
        self.drained_total += n as u64;
        n
    }

    /// The final drain — contractually identical to [`drain`](Self::drain)
    /// but restricted to after writer quiescence (the recorder left the
    /// RT path or was dropped). Records queued when the recorder drops
    /// remain deliverable: the sink owns the ring.
    ///
    /// **Non-RT only** — call only after writer quiescence.
    pub fn drain_final(&mut self, f: impl FnMut(ReceivedRecord<R>)) -> usize {
        self.drain(f)
    }

    /// Total records accepted by the writer — shared `Relaxed` counter.
    ///
    /// **Non-RT only** — collector-side accounting.
    #[must_use]
    pub fn recorded(&self) -> u64 {
        self.recorded.read()
    }

    /// Total records dropped on a full ring — shared `Relaxed` counter.
    ///
    /// **Non-RT only** — collector-side accounting.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.read()
    }

    /// Total records delivered through `drain`/`drain_final` so far.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn drained_total(&self) -> u64 {
        self.drained_total
    }

    /// Consumer lag — records accepted but not yet delivered
    /// (`recorded() - drained_total()`). Lag is *delay*, not loss;
    /// dropped records are reported by [`dropped`](Self::dropped) and
    /// `ReceivedRecord::dropped_before`, never folded into lag.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn lag(&self) -> u64 {
        self.recorded().saturating_sub(self.drained_total)
    }

    /// A [`CounterReader`] on the accepted-record count — cloneable for
    /// OBS-02 `CounterSource` registration.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn recorded_reader(&self) -> CounterReader {
        self.recorded.clone()
    }

    /// A [`CounterReader`] on the dropped-record count — cloneable for
    /// OBS-02 `CounterSource` registration.
    ///
    /// **Non-RT only.**
    #[must_use]
    pub fn dropped_reader(&self) -> CounterReader {
        self.dropped.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_delivers_fifo_with_zero_dropped() {
        let (mut recorder, mut sink) = bridge::<u64>(4);
        for i in 0..4 {
            assert!(recorder.try_record(i));
        }
        assert_eq!(recorder.recorded(), 4);
        assert_eq!(recorder.dropped(), 0);
        let mut got = Vec::new();
        assert_eq!(sink.drain(|r| got.push(r)), 4);
        assert_eq!(
            got.iter().map(ReceivedRecord::record).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert!(got.iter().all(|r| r.dropped_before() == 0));
        assert_eq!(sink.drained_total(), 4);
        assert_eq!(sink.lag(), 0);
    }

    #[test]
    fn full_ring_drops_and_reports_positional_loss() {
        let (mut recorder, mut sink) = bridge::<u64>(2);
        assert!(recorder.try_record(1));
        assert!(recorder.try_record(2));
        // Full: drops.
        assert!(!recorder.try_record(3));
        assert!(!recorder.try_record(4));
        assert_eq!(recorder.dropped(), 2);
        assert_eq!(sink.dropped(), 2);

        let mut got = Vec::new();
        assert_eq!(sink.drain(|r| got.push(r)), 2);
        // The two drops accrued while the ring was full — the next
        // accepted record carries them.
        assert!(recorder.try_record(5));
        assert_eq!(sink.drain(|r| got.push(r)), 1);
        assert_eq!(
            got.iter()
                .map(|r| (r.record(), r.dropped_before()))
                .collect::<Vec<_>>(),
            vec![(1, 0), (2, 0), (5, 2)]
        );
    }

    #[test]
    fn consumer_lag_is_delay_not_loss() {
        let (mut recorder, mut sink) = bridge::<u64>(8);
        for i in 0..5 {
            assert!(recorder.try_record(i));
        }
        // Nothing drained yet — all 5 accepted records are in flight.
        assert_eq!(sink.lag(), 5);
        assert_eq!(sink.recorded(), 5);
        assert_eq!(sink.drained_total(), 0);
        assert_eq!(sink.drain(|_| {}), 5);
        assert_eq!(sink.lag(), 0);
        assert_eq!(sink.dropped(), 0);
        assert_eq!(sink.drained_total(), 5);
    }

    #[test]
    fn recorder_drop_leaves_sink_drainable() {
        let (mut recorder, mut sink) = bridge::<u64>(4);
        assert!(recorder.try_record(7));
        assert!(recorder.try_record(8));
        // Writer quiescence: the recorder is dropped on the non-RT side.
        drop(recorder);
        let mut got = Vec::new();
        assert_eq!(sink.drain_final(|r| got.push(r)), 2);
        assert_eq!(
            got.iter().map(ReceivedRecord::record).collect::<Vec<_>>(),
            vec![7, 8]
        );
        assert_eq!(sink.lag(), 0);
        assert_eq!(sink.recorded(), 2);
    }

    #[test]
    fn counters_survive_as_registrable_readers() {
        let (mut recorder, sink) = bridge::<u64>(2);
        let recorded_r = sink.recorded_reader();
        let dropped_r = sink.dropped_reader();
        assert!(recorder.try_record(1));
        assert!(recorder.try_record(2));
        assert!(!recorder.try_record(3));
        assert_eq!(recorded_r.read(), 2);
        assert_eq!(dropped_r.read(), 1);
    }

    #[test]
    fn bridge_cross_thread_delivery() {
        let (mut recorder, mut sink) = bridge::<u64>(16);
        let handle = std::thread::spawn(move || {
            for i in 0..10 {
                assert!(recorder.try_record(i));
            }
            recorder
        });
        let recorder = handle.join().unwrap();
        assert_eq!(recorder.recorded(), 10);
        assert_eq!(recorder.dropped(), 0);
        let mut got = Vec::new();
        assert_eq!(sink.drain_final(|r| got.push(r)), 10);
        assert_eq!(
            got.iter().map(ReceivedRecord::record).collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
    }

    /// OBS-02 integration: the bridge's counters register as a source
    /// whose `Active → Retiring → FinalDrain → Removed` lifecycle lines
    /// up with writer quiescence and the sink's `drain_final`.
    #[test]
    fn obs02_lifecycle_retire_final_drain_removed() {
        use crate::source::{
            RegistrationLease, SourceGeneration, SourceId, SourceIdentity, SourceLifecycle,
            SourceState,
        };
        use std::sync::Arc;

        let (mut recorder, mut sink) = bridge::<u64>(4);
        // The collector registers the bridge's shared counters — reader
        // ownership stays on the collector side.
        let recorded_r = sink.recorded_reader();
        let dropped_r = sink.dropped_reader();
        let state = Arc::new(SourceState::new());
        let lease = RegistrationLease::new(
            SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0)),
            Arc::clone(&state),
        );

        assert!(recorder.try_record(11));
        assert!(recorder.try_record(12));
        assert_eq!(state.lifecycle(), SourceLifecycle::Active);

        // Writer quiescence declaration + recorder leaves the RT path.
        let retired = lease.retire();
        drop(recorder);
        assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);

        // Final drain: committed only after quiescence.
        state.begin_final_drain().unwrap();
        let mut got = Vec::new();
        assert_eq!(sink.drain_final(|r| got.push(r)), 2);
        assert_eq!(
            got.iter().map(ReceivedRecord::record).collect::<Vec<_>>(),
            vec![11, 12]
        );
        assert_eq!(recorded_r.read(), 2);
        assert_eq!(dropped_r.read(), 0);
        assert_eq!(sink.lag(), 0);

        state.mark_removed().unwrap();
        assert_eq!(retired.lifecycle(), SourceLifecycle::Removed);
    }

    #[test]
    #[should_panic(expected = "non-zero")]
    fn zero_capacity_panics() {
        let _ = bridge::<u64>(0);
    }

    #[test]
    fn endpoints_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<RtRecorder<u64>>();
        assert_send::<RecordSink<u64>>();
        assert_send::<ReceivedRecord<u64>>();
    }

    /// An `EventRecord` — header with clock, correlation, source
    /// identity and classification — travels the bridge unchanged.
    #[test]
    fn event_record_travels_the_bridge() {
        use crate::record::{ClockContext, EventId, EventRecord, RecordHeader};
        use crate::source::{SourceGeneration, SourceId, SourceIdentity};
        use ruviform_media_core::{
            ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance,
        };

        let source = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0));
        let clock = ClockContext::new(
            InputStamp {
                ticks: 96_000,
                base: TimeBase::new(1, 48_000).unwrap(),
                domain: ClockDomainId::SAMPLE_CLOCK,
                provenance: TimestampProvenance::Carried,
            },
            ClockEpochId::new(2),
        );
        let header = RecordHeader::new(EventId::new(3), 1, source)
            .with_generation(5)
            .with_clock(Some(clock));
        let record = EventRecord::new(header, 480);

        let (mut recorder, mut sink) = bridge::<EventRecord>(4);
        assert!(recorder.try_record(record));
        let mut got = Vec::new();
        assert_eq!(sink.drain(|r| got.push(r)), 1);
        assert_eq!(got[0].record(), record);
        assert_eq!(got[0].dropped_before(), 0);
        let header = got[0].record().header();
        assert_eq!(header.clock(), Some(clock));
        assert_eq!(header.correlation(), None);
        assert_eq!(header.generation(), 5);
    }

    /// RT lane: every operation an RT writer may call performs no
    /// allocation inside the measurement window — `try_record` on both
    /// the accept and the drop path, plus the counter reads.
    #[test]
    fn rt_try_record_and_counters_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let (mut recorder, _sink) = bridge::<u64>(2);
        assert_no_alloc(|| {
            assert!(recorder.try_record(1));
            assert!(recorder.try_record(2));
            // Full ring: the drop path is also alloc-free.
            assert!(!recorder.try_record(3));
            let _ = recorder.dropped();
            let _ = recorder.recorded();
        });
        assert_eq!(recorder.dropped(), 1);
        assert_eq!(recorder.recorded(), 2);
    }
}
