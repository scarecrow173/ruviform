//! Deterministic scripted MIDI backend — the spec's
//! `MockMidiEnumerator`/`MockMidiInput` fixture, finally given a
//! concrete home (Issue #253).
//!
//! This is **not a backend**: it is the controlled
//! `TimedInputEvent` source the Issue #188 spec reserved for
//! PoC/bridge tests (`docs/specs/issue-188-timed-midi.md`). It
//! implements the three port traits over an `Arc<Mutex<…>>` state so
//! a test can script enumeration results, inject timestamped events
//! and backend-side loss/untranslatable counters, fail `open`, and
//! observe `close` — the same surface a real OS adapter exposes,
//! with none of the OS.
//!
//! Thread model mirrors the real contract: `read` takes a short
//! internal queue lock and never blocks; the test-side handle may be
//! used from any thread (`Send`/`Sync` fall out of `Arc<Mutex>`).
//!
//! **RT contract:** non-RT fixture — every method may lock and
//! allocate; nothing here runs on the audio thread.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::device::{DeviceError, DeviceResult};

use super::{
    MidiInput, MidiInputEnumerator, MidiInputInfo, MidiInputPort, MidiReadStats, TimedMidiInput,
};

/// Per-port scripted state shared by the enumerator's port objects,
/// the opened subscription and the test-side handle.
struct PortState {
    info: MidiInputInfo,
    pending: VecDeque<TimedMidiInput>,
    dropped: u64,
    unsupported: u64,
    opened: bool,
    closed: bool,
    open_error: Option<DeviceError>,
}

/// Deterministic scripted MIDI backend — the
/// [`MidiInputEnumerator`] implementation.
///
/// Register ports with [`Self::add_port`]; each returned [`MockPort`]
/// is simultaneously the `MidiInputPort` trait object handed to the
/// engine and the test-side injector handle — the same `Arc`-backed
/// state, mirroring how a real adapter's port handle stays live while
/// its subscription is open.
///
/// ```
/// use ruviform_audio_core::midi_input::mock::MockBackend;
/// use ruviform_audio_core::midi_input::{
///     MidiInputEnumerator, MidiInputInfo, MidiPortCapabilities, MidiPortOrigin,
/// };
/// use ruviform_media_core::ClockDomainId;
///
/// let mut backend = MockBackend::new();
/// let port = backend.add_port(MidiInputInfo::new(
///     "mock-0".to_owned(),
///     "Mock Port 0".to_owned(),
///     MidiPortOrigin::External,
///     MidiPortCapabilities::stamped(false, ClockDomainId::new(7)),
/// ));
/// // `input_ports` yields `Box<dyn MidiInputPort>` like any real
/// // enumerator; the returned `port` handle injects events.
/// let ports = backend.input_ports().unwrap();
/// assert_eq!(ports.len(), 1);
/// assert_eq!(port.info().id, "mock-0");
/// ```
#[derive(Default)]
pub struct MockBackend {
    ports: Vec<MockPort>,
}

impl MockBackend {
    /// An empty backend — register ports with [`Self::add_port`].
    #[must_use]
    pub fn new() -> Self {
        Self { ports: Vec::new() }
    }

    /// Registers a scripted port and returns its test-side handle.
    /// Events pushed through the handle before `open` wait in the
    /// port's queue — like a backend that buffers between
    /// enumeration and subscription.
    ///
    /// **RT contract:** non-RT — allocates.
    pub fn add_port(&mut self, info: MidiInputInfo) -> MockPort {
        let port = MockPort::new(info);
        self.ports.push(port.clone());
        port
    }

    /// Number of registered ports — test convenience so assertions do
    /// not need to enumerate just to count.
    #[must_use]
    pub fn port_count(&self) -> usize {
        self.ports.len()
    }
}

impl MidiInputEnumerator for MockBackend {
    /// The registered ports in registration order. Never fails —
    /// enumeration of a scripted backend cannot hit an OS error.
    fn input_ports(&self) -> DeviceResult<Vec<Box<dyn MidiInputPort>>> {
        Ok(self
            .ports
            .iter()
            .cloned()
            .map(|port| Box::new(port) as Box<dyn MidiInputPort>)
            .collect())
    }
}

/// One scripted port — the `MidiInputPort` trait object *and* the
/// test-side injection handle (clone shares the state).
#[derive(Clone)]
pub struct MockPort {
    state: Arc<Mutex<PortState>>,
}

impl MockPort {
    fn new(info: MidiInputInfo) -> Self {
        Self {
            state: Arc::new(Mutex::new(PortState {
                info,
                pending: VecDeque::new(),
                dropped: 0,
                unsupported: 0,
                opened: false,
                closed: false,
                open_error: None,
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PortState> {
        self.state.lock().expect("mock MIDI port state poisoned")
    }

    /// The port's declared identity — same value `info()` reports.
    #[must_use]
    pub fn info(&self) -> MidiInputInfo {
        self.lock().info.clone()
    }

    /// Queue one event for the next `read`, preserving arrival order.
    /// Works before and after `open` — the queue is the port's, not
    /// the subscription's.
    pub fn push(&self, event: TimedMidiInput) {
        self.lock().pending.push_back(event);
    }

    /// Queue several events in order.
    pub fn push_batch(&self, events: impl IntoIterator<Item = TimedMidiInput>) {
        self.lock().pending.extend(events);
    }

    /// Script `n` backend-side drops the next `read` must surface via
    /// [`MidiReadStats::dropped`] — models internal queue overflow.
    pub fn inject_dropped(&self, n: u64) {
        self.lock().dropped += n;
    }

    /// Script `n` received-but-untranslatable messages the next `read`
    /// must surface via [`MidiReadStats::unsupported`] — models
    /// `SysEx`, system common/realtime and malformed traffic the
    /// channel-voice vocabulary excludes by design. Counted
    /// separately from `dropped` (not an overload signal).
    pub fn inject_unsupported(&self, n: u64) {
        self.lock().unsupported += n;
    }

    /// Make the *next* `open` fail with `error` (one-shot — consumed
    /// by the failing call, so a retry can succeed).
    pub fn fail_next_open(&self, error: DeviceError) {
        self.lock().open_error = Some(error);
    }

    /// Whether `open` has succeeded at least once.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.lock().opened
    }

    /// Whether `close` has run on the current subscription.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.lock().closed
    }

    /// Events still queued for the next `read`.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.lock().pending.len()
    }
}

impl MidiInputPort for MockPort {
    fn info(&self) -> MidiInputInfo {
        MockPort::info(self)
    }

    /// Opens the scripted subscription. A scripted `fail_next_open`
    /// error is consumed and returned once; a second `open` after a
    /// successful one is [`DeviceError::DeviceBusy`] — one handle
    /// opens at most once, per the port contract.
    fn open(&self) -> DeviceResult<Box<dyn MidiInput>> {
        let mut state = self.lock();
        if let Some(error) = state.open_error.take() {
            return Err(error);
        }
        if state.opened {
            return Err(DeviceError::DeviceBusy);
        }
        state.opened = true;
        Ok(Box::new(MockInput {
            state: Arc::clone(&self.state),
        }))
    }
}

/// The opened scripted subscription — drains the shared port queue.
struct MockInput {
    state: Arc<Mutex<PortState>>,
}

impl MockInput {
    fn lock(&self) -> std::sync::MutexGuard<'_, PortState> {
        self.state.lock().expect("mock MIDI port state poisoned")
    }
}

impl MidiInput for MockInput {
    /// Drains up to `max_events` in arrival order and surfaces the
    /// scripted backend-side drop count exactly once (consumed by the
    /// first read that reports it). After `close`, reads fail with
    /// [`DeviceError::StreamInvalidated`] — the subscription is dead,
    /// matching a real backend whose handle was released.
    fn read(
        &mut self,
        out: &mut Vec<TimedMidiInput>,
        max_events: usize,
    ) -> DeviceResult<MidiReadStats> {
        let mut state = self.lock();
        if state.closed {
            return Err(DeviceError::StreamInvalidated);
        }
        let n = state.pending.len().min(max_events);
        out.extend(state.pending.drain(..n));
        Ok(MidiReadStats::with_unsupported(
            std::mem::take(&mut state.dropped),
            std::mem::take(&mut state.unsupported),
        ))
    }

    /// Marks the subscription closed — subsequent `read`s report
    /// [`DeviceError::StreamInvalidated`]. Idempotent.
    fn close(&mut self) {
        self.lock().closed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_input::{MidiPortCapabilities, MidiPortOrigin};
    use ruviform_media_core::{
        ClockDomainId, InputStamp, MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey,
        TimeBase, TimestampProvenance,
    };

    fn port_info(id: &str) -> MidiInputInfo {
        MidiInputInfo::new(
            id.to_owned(),
            format!("Mock {id}"),
            MidiPortOrigin::External,
            MidiPortCapabilities::stamped(false, ClockDomainId::new(7)),
        )
    }

    fn note_on() -> TimedMidiInput {
        TimedMidiInput {
            stamp: InputStamp {
                ticks: 1,
                base: TimeBase::new(1, 1_000_000).unwrap(),
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
    fn enumeration_returns_registered_ports_in_order() {
        let mut backend = MockBackend::new();
        backend.add_port(port_info("a"));
        backend.add_port(port_info("b"));
        assert_eq!(backend.port_count(), 2);

        let ports = backend.input_ports().unwrap();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].info().id, "a");
        assert_eq!(ports[1].info().id, "b");
        // Capabilities pass through verbatim — the engine reads them
        // to decide stamp policy, so the fixture must not soften them.
        assert!(ports[0].info().capabilities.provides_timestamps);
        assert_eq!(
            ports[0].info().capabilities.clock_domain,
            Some(ClockDomainId::new(7))
        );
    }

    #[test]
    fn open_read_close_round_trip() {
        let mut backend = MockBackend::new();
        let port = backend.add_port(port_info("p"));
        port.push_batch(vec![note_on(), note_on()]);

        let ports = backend.input_ports().unwrap();
        let mut input = ports[0].open().unwrap();
        assert!(port.is_open());

        let mut out = Vec::with_capacity(4);
        let stats = input.read(&mut out, 1).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(stats.dropped, 0);
        assert_eq!(port.pending_len(), 1);

        input.close();
        assert!(port.is_closed());
        assert!(matches!(
            input.read(&mut out, 4),
            Err(DeviceError::StreamInvalidated)
        ));
    }

    #[test]
    fn scripted_open_failure_is_consumed_once() {
        let mut backend = MockBackend::new();
        let port = backend.add_port(port_info("p"));
        port.fail_next_open(DeviceError::DeviceBusy);

        let ports = backend.input_ports().unwrap();
        assert!(matches!(ports[0].open(), Err(DeviceError::DeviceBusy)));
        // One-shot: the retry succeeds — models a transient backend
        // refusal the engine may legitimately retry.
        assert!(ports[0].open().is_ok());
    }

    #[test]
    fn second_open_is_busy_and_injected_drops_surface_once() {
        let mut backend = MockBackend::new();
        let port = backend.add_port(port_info("p"));
        port.inject_dropped(3);

        let ports = backend.input_ports().unwrap();
        let mut first = ports[0].open().unwrap();
        assert!(matches!(ports[0].open(), Err(DeviceError::DeviceBusy)));

        let mut out = Vec::new();
        let stats = first.read(&mut out, 8).unwrap();
        assert_eq!(stats.dropped, 3);
        // Consumed — the next read reports zero, matching a real
        // backend whose counter resets per drain.
        let stats = first.read(&mut out, 8).unwrap();
        assert_eq!(stats.dropped, 0);
    }

    #[test]
    fn injected_unsupported_surfaces_once_independently_of_dropped() {
        let mut backend = MockBackend::new();
        let port = backend.add_port(port_info("p"));
        port.inject_dropped(2);
        port.inject_unsupported(5);

        let ports = backend.input_ports().unwrap();
        let mut input = ports[0].open().unwrap();

        let mut out = Vec::new();
        let stats = input.read(&mut out, 8).unwrap();
        // Both counters surface through the same since-last-read
        // semantics, tracked separately — `unsupported` models
        // excluded vocabulary traffic, not overload loss.
        assert_eq!(
            stats,
            MidiReadStats::with_unsupported(2, 5),
            "dropped and unsupported drain independently"
        );
        // Consumed — the next read reports zero for both.
        let stats = input.read(&mut out, 8).unwrap();
        assert_eq!(stats, MidiReadStats::new(0));
    }
}
