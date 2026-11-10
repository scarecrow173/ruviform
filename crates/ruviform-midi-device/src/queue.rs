//! Bounded callback→reader event queue with observable loss counters.
//!
//! The OS driver callback pushes [`TimedMidiInput`]s here on a system
//! thread; the control thread drains them in `MidiInput::read`. The
//! queue is pre-reserved to a fixed capacity at open so `push` never
//! allocates — overflow increments `dropped` instead of growing, which
//! is what makes loss observable through [`MidiReadStats`]. Received
//! messages that cannot be translated into the channel-voice
//! vocabulary are counted as `unsupported`, separately from `dropped`
//! (steady-state excluded traffic, not overload).
//!
//! **RT contract:** `push*` are RT-safe — they take a short mutex and
//! perform no allocation (capacity is reserved up front); `drain` is
//! non-RT control-path work.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_input::{MidiReadStats, TimedMidiInput};
use ruviform_media_core::{ClockDomainId, TimeBase};

/// Default queue capacity — comfortably above real-time MIDI event
/// rates while bounding worst-case memory per subscription.
pub(crate) const QUEUE_CAPACITY: usize = 1024;

/// Shared state behind the subscription: the bounded event queue, the
/// stamp domain every pushed event carries, and the death flag the
/// driver callback sets on stream close.
///
/// `Mutex` (not lock-free) is deliberate: the OS callback contract
/// permits critical-section-class synchronization, and the critical
/// section is held only for a bounded `VecDeque` push/drain — the same
/// shape the `midi_input` port contract documents for `read`.
pub(crate) struct Shared {
    inner: Mutex<Inner>,
    /// Clock domain stamped onto every queued event — fixed for the
    /// subscription's lifetime.
    pub(crate) domain: ClockDomainId,
    /// Time base stamped alongside `domain` (`WinMM`: 1/1000 s).
    pub(crate) stamp_base: TimeBase,
    /// Set once `close` begins; the callback checks it first so no new
    /// events are enqueued while the subscription tears down. The
    /// `Arc<Shared>` outlives the native handle, so the callback's raw
    /// pointer stays valid for the handle's whole lifetime.
    pub(crate) closing: AtomicBool,
}

struct Inner {
    events: VecDeque<TimedMidiInput>,
    capacity: usize,
    dropped: u64,
    unsupported: u64,
    dead: bool,
}

impl Shared {
    /// Creates the shared state with a pre-reserved queue.
    pub(crate) fn new(domain: ClockDomainId) -> Self {
        Self {
            inner: Mutex::new(Inner {
                events: VecDeque::with_capacity(QUEUE_CAPACITY),
                capacity: QUEUE_CAPACITY,
                dropped: 0,
                unsupported: 0,
                dead: false,
            }),
            domain,
            stamp_base: TimeBase::new(1, 1_000).expect("1/1000 is a valid timebase"),
            closing: AtomicBool::new(false),
        }
    }

    /// Callback-side push — RT-safe: short lock, no allocation while
    /// within capacity, overflow counted as `dropped`.
    pub(crate) fn push(&self, event: TimedMidiInput) {
        let mut inner = self.lock();
        if inner.events.len() < inner.capacity {
            inner.events.push_back(event);
        } else {
            inner.dropped = inner.dropped.saturating_add(1);
        }
    }

    /// Callback-side counter for a driver-reported loss
    /// (`MIM_MOREDATA`, `MIM_LONGERROR`).
    pub(crate) fn push_dropped(&self) {
        let mut inner = self.lock();
        inner.dropped = inner.dropped.saturating_add(1);
    }

    /// Callback-side counter for received-but-untranslatable traffic
    /// (`SysEx` via `MIM_LONGDATA`, non-channel-voice `MIM_DATA`).
    pub(crate) fn push_unsupported(&self) {
        let mut inner = self.lock();
        inner.unsupported = inner.unsupported.saturating_add(1);
    }

    /// Callback-side stream-death notification (`MIM_CLOSE`).
    pub(crate) fn mark_dead(&self) {
        self.lock().dead = true;
    }

    /// Non-RT drain for `MidiInput::read`: appends up to `max_events`
    /// in arrival order and returns the since-last-read counters.
    ///
    /// Returns `Err(StreamInvalidated)` once the stream is dead *and*
    /// the queue is fully drained *and* no unreported counters remain —
    /// pending events and final loss counters are still delivered as
    /// `Ok` first so nothing received is silently abandoned.
    pub(crate) fn drain(
        &self,
        out: &mut Vec<TimedMidiInput>,
        max_events: usize,
    ) -> DeviceResult<MidiReadStats> {
        let mut inner = self.lock();
        if inner.dead && inner.events.is_empty() && inner.dropped == 0 && inner.unsupported == 0 {
            return Err(DeviceError::StreamInvalidated);
        }
        let n = inner.events.len().min(max_events);
        out.extend(inner.events.drain(..n));
        let stats = MidiReadStats::with_unsupported(
            core::mem::take(&mut inner.dropped),
            core::mem::take(&mut inner.unsupported),
        );
        Ok(stats)
    }

    /// Locks the inner state, recovering from poisoning — a panicking
    /// callback must not wedge the subscription.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{
        InputStamp, MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey, TimeBase,
        TimedInputEvent, TimestampProvenance,
    };

    fn timed(ticks: u64) -> TimedMidiInput {
        TimedInputEvent {
            stamp: InputStamp {
                ticks,
                base: TimeBase::new(1, 1_000).unwrap(),
                domain: ClockDomainId::new(7),
                provenance: TimestampProvenance::Carried,
            },
            event: MidiEvent {
                group: MidiGroup::ZERO,
                channel: MidiChannel::new(0).unwrap(),
                kind: MidiKind::NoteOn {
                    key: NoteKey::new(60).unwrap(),
                    velocity: MidiValue::from_u7(100),
                },
            },
        }
    }

    #[test]
    fn drains_in_arrival_order_up_to_max() {
        let shared = Shared::new(ClockDomainId::new(7));
        for i in 0..5 {
            shared.push(timed(i));
        }
        let mut out = Vec::new();
        let stats = shared.drain(&mut out, 3).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].stamp.ticks, 0);
        assert_eq!(out[2].stamp.ticks, 2);
        assert_eq!(stats, MidiReadStats::new(0));
        shared.drain(&mut out, 10).unwrap();
        assert_eq!(out.len(), 5);
        assert_eq!(out[4].stamp.ticks, 4);
    }

    #[test]
    fn overflow_counts_dropped_without_growing() {
        let shared = Shared::new(ClockDomainId::new(7));
        for i in 0..(QUEUE_CAPACITY + 3) {
            shared.push(timed(i as u64));
        }
        let mut out = Vec::new();
        let stats = shared.drain(&mut out, QUEUE_CAPACITY * 2).unwrap();
        assert_eq!(out.len(), QUEUE_CAPACITY);
        assert_eq!(stats.dropped, 3);
        assert_eq!(stats.unsupported, 0);
    }

    #[test]
    fn unsupported_is_tracked_separately_and_consumed_per_read() {
        let shared = Shared::new(ClockDomainId::new(7));
        shared.push_unsupported();
        shared.push_unsupported();
        shared.push_dropped();
        let mut out = Vec::new();
        let stats = shared.drain(&mut out, 8).unwrap();
        assert_eq!(stats, MidiReadStats::with_unsupported(1, 2));
        // Counters are since-last-read: the next read reports zero.
        let stats = shared.drain(&mut out, 8).unwrap();
        assert_eq!(stats, MidiReadStats::new(0));
    }

    #[test]
    fn dead_stream_reports_invalidation_only_after_drain() {
        let shared = Shared::new(ClockDomainId::new(7));
        shared.push(timed(1));
        shared.mark_dead();
        let mut out = Vec::new();
        // Pending event still drains first — received data is not
        // silently abandoned by the death notification.
        shared.drain(&mut out, 8).unwrap();
        assert_eq!(out.len(), 1);
        // Queue empty, counters consumed → StreamInvalidated.
        assert_eq!(
            shared.drain(&mut out, 8).unwrap_err(),
            DeviceError::StreamInvalidated
        );
    }

    #[test]
    fn final_loss_counters_are_delivered_before_invalidation() {
        let shared = Shared::new(ClockDomainId::new(7));
        shared.push_dropped();
        shared.mark_dead();
        let mut out = Vec::new();
        // The last dropped count is still reported once.
        let stats = shared.drain(&mut out, 8).unwrap();
        assert_eq!(stats.dropped, 1);
        assert_eq!(
            shared.drain(&mut out, 8).unwrap_err(),
            DeviceError::StreamInvalidated
        );
    }
}
