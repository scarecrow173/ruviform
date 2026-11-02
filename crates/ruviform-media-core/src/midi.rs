//! Standards-neutral MIDI vocabulary: channel/group/key validated
//! newtypes, [`MidiKind`]/[`MidiEvent`] message values, scale-to-max
//! canonical value conversion ([`MidiValue`]) and the bounded
//! [`NoteLedger`] active-note tracker used for stuck-note fail-safe
//! release.
//!
//! Values use a canonical scale-to-max representation so that both
//! MIDI 1.0 widths (7-bit data, 14-bit pitch bend) and MIDI 2.0 / UMP
//! widths (16-bit velocity, 32-bit pitch bend) round-trip losslessly;
//! narrowing to a target protocol's width is the adapter's
//! responsibility, never a silent truncation here.
//!
//! **RT contract:** every type is `Copy` and allocation-free; all
//! validation and ledger operations are bounded pure integer work —
//! no allocation, no blocking, no I/O.

/// The bounded capacity of [`NoteLedger`] — active `(group, channel,
/// key)` notes tracked simultaneously. Sized generously for MIDI 1.0
/// (16 channels × 128 keys cannot all be legibly held on real devices)
/// while remaining a small fixed allocation.
pub const NOTE_LEDGER_CAPACITY: usize = 128;

/// A validated 0..=127 value used by [`MidiKind`] variants that carry a
/// seven-bit controller number or program number.
macro_rules! u7_newtype {
    ($name:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// **RT contract:** RT-suitable — `const`, pure.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u8);

        impl $name {
            /// Largest representable value.
            pub const MAX: u8 = 127;

            /// Creates the value when `v <= 127`.
            ///
            /// **RT contract:** RT-suitable — `const`, pure.
            pub const fn new(v: u8) -> Result<Self, MidiRangeError> {
                if v <= Self::MAX {
                    Ok(Self(v))
                } else {
                    Err(MidiRangeError::$variant(v))
                }
            }

            /// The raw value.
            ///
            /// **RT contract:** RT-suitable — `const`, pure.
            #[must_use]
            pub const fn get(self) -> u8 {
                self.0
            }
        }

        impl TryFrom<u8> for $name {
            type Error = MidiRangeError;
            fn try_from(v: u8) -> Result<Self, Self::Error> {
                Self::new(v)
            }
        }
    };
}

u7_newtype!(
    NoteKey,
    Key,
    "A MIDI note key number, `0..=127` (60 = middle C)."
);
u7_newtype!(
    MidiController,
    Controller,
    "A MIDI 1.0 control-change controller number, `0..=127`."
);
u7_newtype!(
    MidiProgram,
    Program,
    "A MIDI 1.0 program number, `0..=127`."
);

/// A MIDI channel number, `0..=15` (MIDI 1.0 channels 1–16 map to
/// `0..=15`; MIDI 2.0/UMP channels are the same range per group).
///
/// **RT contract:** RT-suitable — `const`, pure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MidiChannel(u8);

impl MidiChannel {
    /// Largest representable channel.
    pub const MAX: u8 = 15;

    /// Creates a channel when `v <= 15`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub const fn new(v: u8) -> Result<Self, MidiRangeError> {
        if v <= Self::MAX {
            Ok(Self(v))
        } else {
            Err(MidiRangeError::Channel(v))
        }
    }

    /// The raw channel number.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for MidiChannel {
    type Error = MidiRangeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// A UMP MIDI group number, `0..=15`. MIDI 1.0 traffic always belongs to
/// group `0` ([`MidiGroup::ZERO`]).
///
/// **RT contract:** RT-suitable — `const`, pure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MidiGroup(u8);

impl MidiGroup {
    /// Largest representable group.
    pub const MAX: u8 = 15;
    /// Group `0` — the only group MIDI 1.0 traffic can occupy.
    pub const ZERO: Self = Self(0);

    /// Creates a group when `v <= 15`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    pub const fn new(v: u8) -> Result<Self, MidiRangeError> {
        if v <= Self::MAX {
            Ok(Self(v))
        } else {
            Err(MidiRangeError::Group(v))
        }
    }

    /// The raw group number.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for MidiGroup {
    type Error = MidiRangeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// The reason a raw value failed range validation.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MidiRangeError {
    /// Channel number exceeded 15.
    Channel(u8),
    /// Group number exceeded 15.
    Group(u8),
    /// Note key number exceeded 127.
    Key(u8),
    /// Controller number exceeded 127.
    Controller(u8),
    /// Program number exceeded 127.
    Program(u8),
}

impl core::fmt::Display for MidiRangeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Channel(v) => write!(f, "midi channel out of range: {v}"),
            Self::Group(v) => write!(f, "midi group out of range: {v}"),
            Self::Key(v) => write!(f, "midi note key out of range: {v}"),
            Self::Controller(v) => write!(f, "midi controller out of range: {v}"),
            Self::Program(v) => write!(f, "midi program out of range: {v}"),
        }
    }
}

impl core::error::Error for MidiRangeError {}

/// Canonical value conversion helpers.
///
/// MIDI 1.0 data bytes are seven-bit and pitch bend is fourteen-bit;
/// MIDI 2.0/UMP velocity and pressures are sixteen-bit and pitch bend is
/// thirty-two-bit. Canonical fields preserve the source losslessly
/// **and** keep equal semantic magnitudes equal across widths:
///
/// - unsigned values (velocity, pressure, CC) are **scaled to the full
///   `u32` range** — `from_u7(0x7F) == from_u16(0xFFFF) == u32::MAX` is
///   "maximum", `0` is minimum.
/// - pitch bend is a signed `i32` where `0` is center; the asymmetric
///   source range (`-8192..=8191`) maps onto the equally asymmetric
///   `i32` range (`i32::MIN..=i32::MAX`).
///
/// `as_*` narrow with round-to-nearest — the defined, documented
/// narrowing an adapter applies when the target protocol is narrower
/// than the source. Narrowing a value that came from a wider source
/// loses information by definition; adapters that care must check
/// capability first (honest capability, never silent truncation).
///
/// **RT contract:** RT-suitable — `const` integer math (`u64`
/// intermediates; source widths are ≤ 32 bits so products cannot
/// overflow `u64`).
pub struct MidiValue;

impl MidiValue {
    /// Widens a 7-bit source value to canonical `u32` (scale-to-max).
    /// The result is provably `<= u32::MAX` — the `as` never truncates.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn from_u7(v: u8) -> u32 {
        (v as u64 * u32::MAX as u64 / 0x7F) as u32
    }

    /// Widens a 14-bit source value to canonical `u32` (scale-to-max).
    /// The result is provably `<= u32::MAX` — the `as` never truncates.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn from_u14(v: u16) -> u32 {
        (v as u64 * u32::MAX as u64 / 0x3FFF) as u32
    }

    /// Widens a 16-bit source value to canonical `u32` (exact
    /// replication — identical to scale-to-max).
    #[must_use]
    pub const fn from_u16(v: u16) -> u32 {
        (v as u32) << 16 | v as u32
    }

    /// Canonical `u32` passthrough (identity).
    #[must_use]
    pub const fn from_u32(v: u32) -> u32 {
        v
    }

    /// Narrows a canonical value to 7 bits, round-to-nearest.
    /// The numerator fits `u64` and the quotient `<= 0x7F`.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn as_u7(v: u32) -> u8 {
        ((v as u64 * 0x7F + (u32::MAX as u64 >> 1)) / u32::MAX as u64) as u8
    }

    /// Narrows a canonical value to 14 bits, round-to-nearest.
    /// The quotient is `<= 0x3FFF` — the `as` never truncates.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn as_u14(v: u32) -> u16 {
        ((v as u64 * 0x3FFF + (u32::MAX as u64 >> 1)) / u32::MAX as u64) as u16
    }

    /// Narrows a canonical value to 16 bits, round-to-nearest.
    /// The quotient is `<= 0xFFFF` — the `as` never truncates.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn as_u16(v: u32) -> u16 {
        ((v as u64 * 0xFFFF + (u32::MAX as u64 >> 1)) / u32::MAX as u64) as u16
    }

    /// Widens a raw 14-bit pitch bend (`0..=0x3FFF`, `0x2000` = center)
    /// to canonical `i32` (`0` = center). Negative side scales by the
    /// exact `0x40000` factor, positive side scales to `i32::MAX`.
    #[must_use]
    pub const fn bend_from_u14(v: u16) -> i32 {
        let clamped = if v > 0x3FFF { 0x3FFF } else { v };
        let d = clamped as i32 - 0x2000;
        if d < 0 {
            d << 18
        } else {
            // `d <= 0x1FFF` so the scaled result fits `i32`.
            #[expect(clippy::cast_possible_truncation)]
            let narrowed = (d as i64 * i32::MAX as i64 / 0x1FFF) as i32;
            narrowed
        }
    }

    /// Widens a raw 32-bit UMP pitch bend (`0x8000_0000` = center) to
    /// canonical `i32` — the wrap is the intended signed re-centering.
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub const fn bend_from_u32(v: u32) -> i32 {
        v.wrapping_sub(0x8000_0000) as i32
    }

    /// Narrows a canonical `i32` pitch bend to raw 14-bit
    /// (`0x2000` = center), round-to-nearest. Both branches produce
    /// `0..=0x3FFF` by construction.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub const fn bend_as_u14(v: i32) -> u16 {
        if v < 0 {
            // (v + 2^31) * 0x2000 / 2^31, round-to-nearest.
            (((v as i64 + (1 << 31)) * 0x2000 + (1 << 30)) >> 31) as u16
        } else {
            (0x2000 + (v as i64 * 0x1FFF + (i32::MAX as i64 >> 1)) / i32::MAX as i64) as u16
        }
    }

    /// Narrows a canonical `i32` pitch bend to raw 32-bit UMP
    /// (`0x8000_0000` = center) — the wrap is the intended
    /// unsigned re-centering.
    #[must_use]
    #[expect(clippy::cast_sign_loss)]
    pub const fn bend_as_u32(v: i32) -> u32 {
        (v as u32).wrapping_add(0x8000_0000)
    }
}

/// A channel voice message payload.
///
/// `velocity`/`pressure`/`value` fields are canonical scale-to-max
/// `u32`; `PitchBend::value` is a canonical `i32` with `0` at center —
/// see [`MidiValue`].
///
/// System messages and `SysEx` are deliberately absent: the wire
/// vocabulary is channel-voice only. Future variants extend this
/// `#[non_exhaustive]` enum additively.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MidiKind {
    /// Key press. `velocity == 0` is the MIDI 1.0 running-status
    /// convention for Note Off — consumers that only track note state
    /// should use [`MidiEvent::effective_note_state`].
    NoteOn {
        /// The struck key.
        key: NoteKey,
        /// Canonical velocity (scale-to-max).
        velocity: u32,
    },
    /// Key release.
    NoteOff {
        /// The released key.
        key: NoteKey,
        /// Canonical release velocity (scale-to-max).
        velocity: u32,
    },
    /// Control change.
    ControlChange {
        /// Controller number.
        controller: MidiController,
        /// Canonical controller value (scale-to-max).
        value: u32,
    },
    /// Channel pitch bend, `0` at center.
    PitchBend {
        /// Canonical signed bend (0 = center).
        value: i32,
    },
    /// Per-key polyphonic pressure.
    PolyPressure {
        /// The pressed key.
        key: NoteKey,
        /// Canonical pressure (scale-to-max).
        pressure: u32,
    },
    /// Whole-channel pressure.
    ChannelPressure {
        /// Canonical pressure (scale-to-max).
        pressure: u32,
    },
    /// Program change.
    ProgramChange {
        /// Program number.
        program: MidiProgram,
    },
}

/// A single MIDI message with its routing (group + channel).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MidiEvent {
    /// UMP group; [`MidiGroup::ZERO`] for MIDI 1.0 traffic.
    pub group: MidiGroup,
    /// Channel `0..=15`.
    pub channel: MidiChannel,
    /// The message payload.
    pub kind: MidiKind,
}

impl MidiEvent {
    /// The identity under which note state is tracked.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn ledger_key(&self) -> Option<LedgerKey> {
        match self.kind {
            MidiKind::NoteOn { key, .. }
            | MidiKind::NoteOff { key, .. }
            | MidiKind::PolyPressure { key, .. } => Some(LedgerKey {
                group: self.group,
                channel: self.channel,
                key,
            }),
            _ => None,
        }
    }

    /// Whether this event turns a note on (`Some(true)`), turns it off
    /// (`Some(false)` — including `NoteOn` with `velocity == 0`), or does
    /// not affect note state (`None`). The MIDI 1.0
    /// velocity-zero-means-off convention is resolved here once.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn effective_note_state(&self) -> Option<bool> {
        match self.kind {
            MidiKind::NoteOn { velocity, .. } => Some(velocity != 0),
            MidiKind::NoteOff { .. } => Some(false),
            _ => None,
        }
    }

    /// Whether this is a Note Off for ordering purposes — including
    /// `NoteOn` with `velocity == 0`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_note_off(&self) -> bool {
        matches!(self.effective_note_state(), Some(false))
    }
}

/// The `(group, channel, key)` identity [`NoteLedger`] tracks.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LedgerKey {
    /// UMP group.
    pub group: MidiGroup,
    /// Channel.
    pub channel: MidiChannel,
    /// Note key.
    pub key: NoteKey,
}

#[derive(Copy, Clone, Debug)]
struct LedgerEntry {
    key: LedgerKey,
    /// Re-trigger count for MIDI 1.0 duplicate-pitch (multiple Note Ons
    /// on the same key before its Note Offs). Voice-level identity
    /// cannot be reconstructed from MIDI 1.0 traffic; the count is the
    /// honest tracking unit.
    count: u32,
    /// Insertion order for oldest-first force release.
    order: u32,
}

/// Bounded active-note ledger — the generic stuck-note fail-safe
/// authority.
///
/// Tracks `(group, channel, key)` state fed by delivered [`MidiEvent`]s.
/// MIDI 1.0 duplicate-pitch (a second Note On before the first Note Off)
/// is tracked as a re-trigger **count** on the same [`LedgerKey`]: a
/// single Note Off decrements but does not clear the entry, matching the
/// common instrument retrigger behaviour. Voice-level identity cannot be
/// reconstructed from MIDI 1.0 traffic — that ambiguity is a protocol
/// fact, not a ledger deficiency.
///
/// On capacity exhaustion the **oldest** entry is force-released (a
/// synthetic Note Off is emitted on the next [`Self::release_all`] drain
/// and recorded via [`Self::take_overflow`]) — releasing is the safe
/// failure mode; dropping the new note or silently forgetting the old
/// one would both risk stuck notes.
///
/// **RT contract:** RT-suitable — fixed-size, no allocation, bounded
/// scans (`O(NOTE_LEDGER_CAPACITY)`).
pub struct NoteLedger {
    entries: [Option<LedgerEntry>; NOTE_LEDGER_CAPACITY],
    order: u32,
    overflow: bool,
}

impl Default for NoteLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteLedger {
    /// An empty ledger.
    ///
    /// **RT contract:** construct on non-RT (large fixed array); all
    /// `&mut` methods are RT-suitable.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [None; NOTE_LEDGER_CAPACITY],
            order: 0,
            overflow: false,
        }
    }

    /// Number of distinct active keys.
    ///
    /// **RT contract:** RT-suitable — bounded scan.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    /// Whether `key` currently has an active note.
    ///
    /// **RT contract:** RT-suitable — bounded scan.
    #[must_use]
    pub fn is_active(&self, key: LedgerKey) -> bool {
        self.find(key).is_some()
    }

    fn find(&self, key: LedgerKey) -> Option<usize> {
        self.entries
            .iter()
            .position(|slot| matches!(slot, Some(e) if e.key == key))
    }

    /// Feeds one delivered event. Returns `true` when note state changed.
    ///
    /// `NoteOn` with `velocity == 0` resolves to note-off here
    /// ([`MidiEvent::effective_note_state`]). Note-offs on inactive keys
    /// are ignored (state was already clear). Capacity exhaustion
    /// force-releases the oldest entry and sets the overflow flag.
    ///
    /// **RT contract:** RT-suitable — bounded scan, no allocation.
    pub fn feed(&mut self, event: &MidiEvent) -> bool {
        let Some(key) = event.ledger_key() else {
            return false;
        };
        match event.effective_note_state() {
            Some(true) => self.note_on(key),
            Some(false) => self.note_off(key),
            None => false,
        }
    }

    fn note_on(&mut self, key: LedgerKey) -> bool {
        if let Some(i) = self.find(key) {
            // Duplicate pitch: same key re-triggered before its Note
            // Off. Bump the count — the note stays active until as many
            // Note Offs arrive (see type docs for the ambiguity).
            if let Some(e) = &mut self.entries[i] {
                e.count = e.count.saturating_add(1);
            }
            return false;
        }
        if let Some(slot) = self.entries.iter_mut().find(|s| s.is_none()) {
            *slot = Some(LedgerEntry {
                key,
                count: 1,
                order: self.order,
            });
            self.order = self.order.wrapping_add(1);
            return true;
        }
        // Capacity exhausted: force-release the oldest entry, flag it,
        // and still register the incoming note. Releasing is the safe
        // failure mode (silent drop would stick the note).
        let oldest = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|e| (i, e.order)))
            .min_by_key(|&(_, order)| order)
            .map(|(i, _)| i);
        if let Some(i) = oldest {
            self.entries[i] = None;
        }
        self.overflow = true;
        self.note_on(key)
    }

    fn note_off(&mut self, key: LedgerKey) -> bool {
        let Some(i) = self.find(key) else {
            return false;
        };
        let entry = &mut self.entries[i];
        if let Some(e) = entry {
            e.count = e.count.saturating_sub(1);
            if e.count > 0 {
                return false;
            }
        }
        self.entries[i] = None;
        true
    }

    /// Whether a capacity force-release happened since the last call —
    /// report this as a fault (an active note was cut early).
    ///
    /// **RT contract:** RT-suitable — flag read.
    pub fn take_overflow(&mut self) -> bool {
        core::mem::replace(&mut self.overflow, false)
    }

    /// Emits a release `MidiEvent` (Note Off, canonical zero velocity)
    /// for every active entry into `scratch`, clears the ledger, and
    /// returns the number written. Callers deliver these at block offset
    /// `0` on transport discontinuity (stop/seek/generation change/
    /// device restart) — the stuck-note fail-safe.
    ///
    /// Releases beyond `scratch.len()` are still cleared from the
    /// ledger (the fail-safe must not retain phantom notes); the caller
    /// detects the shortfall via the return count.
    ///
    /// **RT contract:** RT-suitable — bounded write, no allocation.
    pub fn release_all(&mut self, scratch: &mut [MidiEvent]) -> usize {
        let mut written = 0usize;
        for slot in &mut self.entries {
            if let Some(entry) = slot.take() {
                if let Some(out) = scratch.get_mut(written) {
                    *out = MidiEvent {
                        group: entry.key.group,
                        channel: entry.key.channel,
                        kind: MidiKind::NoteOff {
                            key: entry.key.key,
                            velocity: 0,
                        },
                    };
                }
                written += 1;
            }
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(v: u8) -> MidiChannel {
        MidiChannel::new(v).unwrap()
    }

    fn key(v: u8) -> NoteKey {
        NoteKey::new(v).unwrap()
    }

    fn on(k: u8, v: u32) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: ch(0),
            kind: MidiKind::NoteOn {
                key: key(k),
                velocity: v,
            },
        }
    }

    fn off(k: u8) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: ch(0),
            kind: MidiKind::NoteOff {
                key: key(k),
                velocity: 0,
            },
        }
    }

    #[test]
    fn validated_newtypes_reject_out_of_range() {
        assert_eq!(MidiChannel::new(15).unwrap().get(), 15);
        assert_eq!(MidiChannel::new(16), Err(MidiRangeError::Channel(16)));
        assert_eq!(MidiGroup::new(16), Err(MidiRangeError::Group(16)));
        assert_eq!(NoteKey::new(128), Err(MidiRangeError::Key(128)));
        assert_eq!(
            MidiController::new(128),
            Err(MidiRangeError::Controller(128))
        );
        assert_eq!(MidiProgram::new(128), Err(MidiRangeError::Program(128)));
    }

    #[test]
    fn midi_value_scale_to_max_roundtrips() {
        // Semantic equality across widths: max is max, zero is zero.
        assert_eq!(MidiValue::from_u7(0x7F), u32::MAX);
        assert_eq!(MidiValue::from_u16(0xFFFF), u32::MAX);
        assert_eq!(MidiValue::from_u14(0x3FFF), u32::MAX);
        assert_eq!(MidiValue::from_u7(0), 0);
        // Exhaustive round-trips for MIDI 1.0 widths.
        for v in 0..=0x7Fu8 {
            assert_eq!(MidiValue::as_u7(MidiValue::from_u7(v)), v);
        }
        for v in 0..=0x3FFFu16 {
            assert_eq!(MidiValue::as_u14(MidiValue::from_u14(v)), v);
        }
        for v in [0u16, 1, 0x1234, 0x7FFF, 0x8000, 0xBEEF, 0xFFFF] {
            assert_eq!(MidiValue::as_u16(MidiValue::from_u16(v)), v);
        }
        // Bend center/extremes.
        assert_eq!(MidiValue::bend_from_u14(0x2000), 0);
        assert_eq!(MidiValue::bend_from_u14(0), i32::MIN);
        assert_eq!(MidiValue::bend_from_u14(0x3FFF), i32::MAX);
        assert_eq!(MidiValue::bend_as_u14(0), 0x2000);
        assert_eq!(MidiValue::bend_as_u14(i32::MIN), 0);
        assert_eq!(MidiValue::bend_as_u14(i32::MAX), 0x3FFF);
        // 32-bit UMP bend is a reinterpretation: exact round-trip.
        for v in [0u32, 0x8000_0000, 0xFFFF_FFFF, 0x1234_5678] {
            assert_eq!(MidiValue::bend_as_u32(MidiValue::bend_from_u32(v)), v);
        }
        // 14-bit round-trip through canonical.
        for v in [0u16, 0x2000, 0x3FFF, 0x1000, 0x3000] {
            assert_eq!(MidiValue::bend_as_u14(MidiValue::bend_from_u14(v)), v);
        }
    }

    #[test]
    fn effective_note_state_resolves_velocity_zero() {
        assert_eq!(on(60, 1).effective_note_state(), Some(true));
        assert_eq!(on(60, 0).effective_note_state(), Some(false));
        assert!(on(60, 0).is_note_off());
        assert!(off(60).is_note_off());
    }

    #[test]
    fn ledger_tracks_on_off_and_retrigger_count() {
        let mut l = NoteLedger::new();
        assert!(l.feed(&on(60, 1)));
        assert!(l.is_active(LedgerKey {
            group: MidiGroup::ZERO,
            channel: ch(0),
            key: key(60),
        }));
        // Duplicate pitch: second Note On bumps count.
        assert!(!l.feed(&on(60, 1)));
        // First Note Off decrements but stays active.
        assert!(!l.feed(&off(60)));
        assert_eq!(l.active_count(), 1);
        // Second Note Off clears.
        assert!(l.feed(&off(60)));
        assert_eq!(l.active_count(), 0);
        // Velocity-zero Note On is a note-off.
        assert!(l.feed(&on(61, 1)));
        assert!(l.feed(&on(61, 0)));
        assert_eq!(l.active_count(), 0);
    }

    #[test]
    fn ledger_release_all_emits_and_clears() {
        let mut l = NoteLedger::new();
        l.feed(&on(60, 1));
        l.feed(&on(64, 1));
        let mut scratch = [MidiEvent {
            group: MidiGroup::ZERO,
            channel: ch(0),
            kind: MidiKind::ProgramChange {
                program: MidiProgram::new(0).unwrap(),
            },
        }; 8];
        let n = l.release_all(&mut scratch);
        assert_eq!(n, 2);
        assert_eq!(l.active_count(), 0);
        assert!(scratch[..2].iter().all(MidiEvent::is_note_off));
    }

    #[test]
    fn ledger_overflow_force_releases_oldest() {
        let mut l = NoteLedger::new();
        for i in 0..NOTE_LEDGER_CAPACITY {
            l.feed(&MidiEvent {
                group: MidiGroup::ZERO,
                channel: ch(0),
                kind: MidiKind::NoteOn {
                    key: key(u8::try_from(i).unwrap()),
                    velocity: 1,
                },
            });
        }
        assert_eq!(l.active_count(), NOTE_LEDGER_CAPACITY);
        // Next distinct key (same channel only has 128 keys; use ch1).
        l.feed(&MidiEvent {
            group: MidiGroup::ZERO,
            channel: ch(1),
            kind: MidiKind::NoteOn {
                key: key(0),
                velocity: 1,
            },
        });
        assert!(l.take_overflow());
        assert!(!l.take_overflow());
        // Oldest entry (key 0 on ch 0) was released.
        assert!(!l.is_active(LedgerKey {
            group: MidiGroup::ZERO,
            channel: ch(0),
            key: key(0),
        }));
        assert!(l.is_active(LedgerKey {
            group: MidiGroup::ZERO,
            channel: ch(1),
            key: key(0),
        }));
    }

    /// RT contract: ledger feed/release scans are bounded and
    /// allocation-free.
    #[test]
    fn rt_ledger_no_alloc() {
        let mut l = NoteLedger::new();
        let mut scratch = [off(0); NOTE_LEDGER_CAPACITY];
        assert_no_alloc::assert_no_alloc(|| {
            for i in 0..64u8 {
                l.feed(&on(i, 1));
            }
            l.release_all(&mut scratch);
            l.feed(&off(10));
        });
    }
}
