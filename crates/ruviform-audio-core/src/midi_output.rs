//! MIDI output boundary ports — the trait surface between the engine
//! (`ruviform-audio`, not yet created) and MIDI backend adapters, plus
//! the port description type they exchange. Mirror of
//! [`crate::midi_input`]: the same three-stage lifecycle split
//! ([`crate::device`]) in the send direction.
//!
//! The port is three object-safe traits: [`MidiOutputEnumerator`]
//! (one-shot enumeration), [`MidiOutputPort`] (a lightweight live
//! handle that describes and opens) and [`MidiOutput`] (owns the
//! backend subscription and sends events to the device). The engine
//! exchanges them as `Box<dyn …>` so backends can be substituted
//! without API breakage.
//!
//! Thread model: enumeration, `info`, `open`, `send`, `send_sysex`,
//! `close` are control-thread operations — non-RT. A backend may queue
//! onto an OS driver (e.g. `WinMM` `midiOutShortMsg`) but never blocks
//! on realtime resources; per-API guarantees are documented as
//! `**RT contract:**` lines.
//!
//! Shared vocabulary: [`MidiPortOrigin`] and [`MidiPortCapabilities`]
//! are re-exported from [`crate::midi_input`] — the same concepts in
//! the opposite direction, deliberately not duplicated types.
//!
//! Contract source: `docs/specs/issue-629-midi-output.md`,
//! `docs/designs/issue-629-midi-output.md` (Issue #629, 2nd slice —
//! discovery gap #1 `midiOut*` send path).
//!
//! [`mock`] is the deterministic scripted backend fixture — the
//! send-direction counterpart of `midi_input::mock`. Test/PoC
//! infrastructure, not a real OS backend.

pub mod mock;

use ruviform_media_core::MidiEvent;

use crate::device::DeviceResult;

pub use crate::midi_input::{MidiPortCapabilities, MidiPortOrigin};

/// Identity, origin and capability description of one output port —
/// obtained through non-RT enumeration and `MidiOutputPort::info`.
///
/// Same shape as [`crate::midi_input::MidiInputInfo`]; kept as a
/// separate type so direction-specific fields can diverge without
/// breaking the input contract.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiOutputInfo {
    /// Backend-scoped stable identifier (persists across enumeration).
    pub id: String,
    /// Human-readable port name for UI and logs.
    pub name: String,
    /// External vs virtual vs internal classification.
    pub origin: MidiPortOrigin,
    /// Honest backend capability declaration.
    pub capabilities: MidiPortCapabilities,
}

impl MidiOutputInfo {
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

/// MIDI output enumeration — non-RT.
///
/// Implemented by adapters. `Send`/`Sync` are deliberately not
/// required, mirroring [`crate::midi_input::MidiInputEnumerator`].
///
/// **RT contract:** non-RT object — every method calls OS APIs and
/// may allocate.
pub trait MidiOutputEnumerator {
    /// Output ports currently present — an empty `Vec` when none.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] on enumeration failure; an empty
    /// port set is `Ok(vec![])`, not an error.
    fn output_ports(&self) -> DeviceResult<Vec<Box<dyn MidiOutputPort>>>;
}

/// A lightweight live handle for one enumerated output port — non-RT.
///
/// `info` is cheap enough for UI refresh; `open` takes the send
/// subscription. One handle opens at most once; re-enumerate for a
/// fresh handle.
///
/// **RT contract:** non-RT object — calls OS APIs and may allocate.
pub trait MidiOutputPort {
    /// The port's identity and declared capabilities.
    ///
    /// **RT contract:** non-RT only — may allocate.
    fn info(&self) -> MidiOutputInfo;

    /// Start sending events to this port.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] when the port vanished, is
    /// already open, or the backend refused the subscription.
    fn open(&self) -> DeviceResult<Box<dyn MidiOutput>>;
}

/// An open output subscription — control-thread send path.
///
/// `send` delivers one channel-voice [`MidiEvent`] in call order.
/// `send_sysex` delivers a complete System Exclusive byte string
/// (first byte `0xF0`, last byte `0xF7`) — the out-of-vocabulary
/// extension point for message kinds [`MidiEvent`] deliberately does
/// not model; backends without long-message support return
/// [`crate::device::DeviceError::Unsupported`] via the default
/// implementation.
///
/// Both sends are fire-and-forget at this boundary: the backend
/// reports queue acceptance, not device-side consumption. Ordering is
/// the caller's contract — implementations must not reorder.
pub trait MidiOutput {
    /// Send one channel-voice event to the device.
    ///
    /// **RT contract:** non-RT — calls OS APIs; must not block on
    /// realtime resources.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError`] when the event cannot be
    /// represented on this port (e.g. a non-zero UMP group on a
    /// `supports_ump = false` port), when a scripted/driver failure
    /// was injected, or when the subscription died
    /// ([`crate::device::DeviceError::StreamInvalidated`]).
    fn send(&mut self, event: MidiEvent) -> DeviceResult<()>;

    /// Send one complete `SysEx` byte string (`0xF0` … `0xF7`).
    ///
    /// **RT contract:** non-RT — calls OS APIs and may allocate.
    ///
    /// # Errors
    ///
    /// [`crate::device::DeviceError::Unsupported`] by default —
    /// backends without a long-message path inherit this honest
    /// refusal. Other variants follow [`Self::send`].
    fn send_sysex(&mut self, _bytes: &[u8]) -> DeviceResult<()> {
        Err(crate::device::DeviceError::Unsupported)
    }

    /// Close the subscription and release backend resources.
    ///
    /// **RT contract:** non-RT only — calls OS APIs.
    fn close(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DeviceError;
    use ruviform_media_core::{MidiChannel, MidiGroup, MidiKind, MidiValue, NoteKey};

    /// The port description reuses the shared origin/capability
    /// vocabulary — same concepts as the input direction, not a
    /// duplicated type.
    #[test]
    fn output_info_reuses_shared_origin_and_capabilities() {
        let info = MidiOutputInfo::new(
            "out-0".to_owned(),
            "Out 0".to_owned(),
            MidiPortOrigin::External,
            MidiPortCapabilities::midi1_unstamped(),
        );
        assert_eq!(info.origin, MidiPortOrigin::External);
        assert!(!info.capabilities.supports_ump);
        assert!(!info.capabilities.provides_timestamps);
        assert_eq!(info.capabilities.clock_domain, None);
    }

    /// Default `send_sysex` is an honest `Unsupported` — a backend
    /// that does not override it refuses rather than silently
    /// dropping long messages.
    #[test]
    fn default_send_sysex_is_unsupported() {
        struct Minimal;
        impl MidiOutput for Minimal {
            fn send(&mut self, _event: MidiEvent) -> DeviceResult<()> {
                Ok(())
            }
        }
        let mut out = Minimal;
        assert_eq!(
            out.send_sysex(&[0xF0, 0x7E, 0xF7]),
            Err(DeviceError::Unsupported)
        );
        // `close` defaults to a no-op.
        out.close();
    }

    fn note_on(channel: u8) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(channel).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        }
    }

    /// Object-safety smoke through the mock: the full
    /// enumerator → port → output path composes as `Box<dyn …>`.
    #[test]
    fn port_traits_compose_as_boxed_trait_objects() {
        use crate::midi_output::mock::MockBackend;
        let mut backend = MockBackend::new();
        backend.add_port(MidiOutputInfo::new(
            "mock-out".to_owned(),
            "Mock Out".to_owned(),
            MidiPortOrigin::Virtual,
            MidiPortCapabilities::midi1_unstamped(),
        ));
        let enumerator: &dyn MidiOutputEnumerator = &backend;
        let ports = enumerator.output_ports().unwrap();
        assert_eq!(ports.len(), 1);
        let mut output = ports[0].open().unwrap();
        output.send(note_on(3)).unwrap();
        output.close();
    }
}
