//! MIDI input boundary ports — the trait surface between the engine
//! (`ruviform-audio`, not yet created) and MIDI backend adapters,
//! plus the capability and provenance types they exchange.
//!
//! The port is three object-safe traits mirroring the device
//! boundary's lifecycle split ([`crate::device`]):
//! [`MidiInputEnumerator`] (one-shot enumeration), [`MidiInputPort`]
//! (a lightweight live handle that describes and opens) and
//! [`MidiInput`] (owns the backend subscription and hands events to
//! the engine). The engine exchanges them as `Box<dyn …>` so backends
//! can be substituted without API breakage.
//!
//! Thread model: enumeration, `info`, `open`, `read`, `close` are
//! control-thread operations — non-RT. A backend that timestamps on a
//! realtime thread pushes [`TimedInputEvent`]s into an
//! adapter-internal bounded queue; `read` drains that queue into an
//! engine-owned buffer at a non-RT or block-boundary call. Overflow
//! inside the backend queue is observable through
//! [`MidiReadStats::dropped`], never silent.
//!
//! Source timestamps arrive in the backend's declared
//! [`MidiPortCapabilities::clock_domain`]; translation into the audio
//! sample clock is the engine's job via
//! `ruviform_media_core::timed_input`, not the adapter's.
//!
//! Contract source: `docs/specs/issue-188-timed-midi.md` (R1, R3, R8),
//! `docs/designs/issue-188-timed-midi-detailed.md`.
//!
//! [`mock`] is the deterministic scripted backend fixture — the
//! spec's `MockMidiEnumerator`/`MockMidiInput` (Issue #253). Test/PoC
//! infrastructure, not a real OS backend.

pub mod mock;

use ruviform_media_core::{ClockDomainId, MidiEvent, TimedInputEvent};

/// The channel-voice MIDI event envelope this port carries. Other
/// payload kinds (system/SysEx) are additive future extensions —
/// today's subscription yields this vocabulary only.
pub type TimedMidiInput = TimedInputEvent<MidiEvent>;

use crate::device::DeviceResult;

/// Where an input port's events originate — the "external vs virtual"
/// distinction the issue requires. The variant set is extensible.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MidiPortOrigin {
    /// A physical or bus-attached device (USB/DIN/Bluetooth endpoint).
    External,
    /// A port created by software for other software to connect to
    /// (loopMIDI/IAC-style, or an engine-published virtual input).
    Virtual,
    /// A port owned by the host itself (e.g. an embedded test source)
    /// rather than any external peer.
    Internal,
    /// The backend cannot determine the origin. Backends that expose
    /// no origin distinction (e.g. `WinMM`) report this rather than
    /// guessing.
    Unknown,
}

/// What a backend honestly claims about one input port. Every field
/// is a *declared capability* — the engine must treat absent features
/// as rejections or explicit fallbacks, not silent degradations.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MidiPortCapabilities {
    /// The backend can carry UMP/MIDI 2.0 channel-voice messages on
    /// this port (group field meaningful, 16-/32-bit payloads).
    /// `false` means MIDI 1.0 byte stream only — the adapter widens
    /// values into the canonical representation.
    pub supports_ump: bool,
    /// The backend stamps each event with a device/host timestamp in
    /// [`Self::clock_domain`]. `false` means events arrive unstamped —
    /// the engine must then apply an explicit fallback stamp policy.
    pub provides_timestamps: bool,
    /// The clock domain the backend's stamps live in. Required when
    /// `provides_timestamps` is true; `None` when unstamped.
    pub clock_domain: Option<ClockDomainId>,
}

impl MidiPortCapabilities {
    /// The minimal declaration — MIDI 1.0 bytes, no stamps.
    ///
    /// **RT contract:** non-RT — declarative metadata.
    #[must_use]
    pub const fn midi1_unstamped() -> Self {
        Self {
            supports_ump: false,
            provides_timestamps: false,
            clock_domain: None,
        }
    }

    /// A stamped-port declaration.
    ///
    /// **RT contract:** non-RT — declarative metadata.
    #[must_use]
    pub const fn stamped(supports_ump: bool, clock_domain: ClockDomainId) -> Self {
        Self {
            supports_ump,
            provides_timestamps: true,
            clock_domain: Some(clock_domain),
        }
    }
}

/// Identity, origin and capability description of one input port —
/// obtained through non-RT enumeration and `MidiInputPort::info`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiInputInfo {
    /// Backend-scoped stable identifier (persists across enumeration).
    pub id: String,
    /// Human-readable port name for UI and logs.
    pub name: String,
    /// External vs virtual vs internal classification.
    pub origin: MidiPortOrigin,
    /// Honest backend capability declaration.
    pub capabilities: MidiPortCapabilities,
}

impl MidiInputInfo {
    /// Full port description — adapters construct through here since
    /// the struct is `#[non_exhaustive]` (future fields default to
    /// "unknown" rather than breaking adapter source).
    ///
    /// **RT contract:** non-RT — owns `String`s.
    #[must_use]
    pub fn new(
        id: String,
        name: String,
        origin: MidiPortOrigin,
        capabilities: MidiPortCapabilities,
    ) -> Self {
        Self {
            id,
            name,
            origin,
            capabilities,
        }
    }
}

/// What a single `read` drained — including observable loss.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MidiReadStats {
    /// Events the backend dropped before `read` could see them
    /// (internal queue overflow, or a driver-level loss notification
    /// such as `WinMM` `MIM_MOREDATA`). Nonzero is an observable
    /// overload signal for the engine's control/status surface — the
    /// bridge reports it, the engine decides policy.
    pub dropped: u64,
    /// Messages the backend received but could not translate into the
    /// declared channel-voice vocabulary (`SysEx`, system common/
    /// realtime, malformed bytes). Tracked separately from `dropped`
    /// because it is steady-state traffic the vocabulary excludes by
    /// design, not an overload signal — adapters must still count it
    /// so the loss is observable, never silent.
    pub unsupported: u64,
}

impl MidiReadStats {
    /// Stats with `dropped` lost events and no unsupported traffic —
    /// adapters construct through here (the struct is
    /// `#[non_exhaustive]`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(dropped: u64) -> Self {
        Self {
            dropped,
            unsupported: 0,
        }
    }

    /// Stats with both counters set — for backends that track
    /// received-but-untranslatable messages separately from drops.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_unsupported(dropped: u64, unsupported: u64) -> Self {
        Self {
            dropped,
            unsupported,
        }
    }
}

/// MIDI input enumeration — non-RT.
///
/// Implemented by adapters. `Send`/`Sync` are deliberately not
/// required, mirroring [`crate::device::DeviceEnumerator`].
///
/// **RT contract:** non-RT object — every method calls OS APIs and
/// may allocate.
pub trait MidiInputEnumerator {
    /// Input ports currently present — an empty `Vec` when none.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] on enumeration failure; an empty
    /// port set is `Ok(vec![])`, not an error.
    fn input_ports(&self) -> DeviceResult<Vec<Box<dyn MidiInputPort>>>;
}

/// A lightweight live handle for one enumerated input port — non-RT.
///
/// `info` is cheap enough for UI refresh; `open` takes the
/// subscription. One handle opens at most once; re-enumerate for a
/// fresh handle.
///
/// **RT contract:** non-RT object — calls OS APIs and may allocate.
pub trait MidiInputPort {
    /// The port's identity and declared capabilities.
    ///
    /// **RT contract:** non-RT only — may allocate.
    fn info(&self) -> MidiInputInfo;

    /// Start receiving events from this port.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] when the port vanished, is
    /// already open, or the backend refused the subscription.
    fn open(&self) -> DeviceResult<Box<dyn MidiInput>>;
}

/// An open input subscription — control-thread event drain.
///
/// `read` appends up to `max_events` events to `out` in arrival
/// order. The buffer is engine-owned: `out` pre-allocates capacity
/// once, so `read` itself performs no allocation on the hot path.
/// The adapter must stamp events in the declared
/// [`MidiPortCapabilities::clock_domain`] or emit
/// [`TimedInputEvent`]s whose stamp provenance honestly reports an
/// unstamped/fallback source.
///
/// **RT contract:** non-RT object — `read` may lock the adapter's
/// internal queue but must not block on OS resources; `close` may
/// call OS APIs.
pub trait MidiInput {
    /// Drain pending events into `out` (up to `max_events`); returns
    /// observable statistics including backend-side drops.
    ///
    /// **RT contract:** non-RT — bounded against `max_events`; may
    /// take a short internal queue lock.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] when the subscription died;
    /// returned stats from prior reads remain valid.
    fn read(
        &mut self,
        out: &mut Vec<TimedMidiInput>,
        max_events: usize,
    ) -> DeviceResult<MidiReadStats>;

    /// Close the subscription and release backend resources.
    ///
    /// **RT contract:** non-RT only — calls OS APIs.
    fn close(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{ClockDomainId, InputStamp};

    /// A minimal scripted source — the same shape a mock for bridge
    /// tests takes: a queued `Vec<TimedMidiInput>` + drop counter.
    struct ScriptedInput {
        pending: std::collections::VecDeque<TimedMidiInput>,
        dropped: u64,
        closed: bool,
    }

    impl MidiInput for ScriptedInput {
        fn read(
            &mut self,
            out: &mut Vec<TimedMidiInput>,
            max_events: usize,
        ) -> DeviceResult<MidiReadStats> {
            let n = self.pending.len().min(max_events);
            out.extend(self.pending.drain(..n));
            let stats = MidiReadStats::new(self.dropped);
            self.dropped = 0;
            Ok(stats)
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }

    fn timed(i: u64) -> TimedMidiInput {
        use ruviform_media_core::{
            MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey, TimeBase,
            TimestampProvenance,
        };
        TimedInputEvent {
            stamp: InputStamp {
                ticks: i,
                base: TimeBase::new(1, 1_000_000).unwrap(),
                domain: ClockDomainId::new(1),
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
    fn bounded_read_drains_in_order_and_reports_drops() {
        let mut input = ScriptedInput {
            pending: (0..5).map(timed).collect(),
            dropped: 7,
            closed: false,
        };
        let mut out = Vec::with_capacity(8);
        let stats = input.read(&mut out, 3).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(stats.dropped, 7);
        // Remaining two drain next call; the drop count was consumed.
        let stats = input.read(&mut out, 3).unwrap();
        assert_eq!(out.len(), 5);
        assert_eq!(stats.dropped, 0);
        input.close();
        assert!(input.closed);
    }
}
