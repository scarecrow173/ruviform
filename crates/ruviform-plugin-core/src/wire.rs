//! The L1 out-of-process wire protocol (Issue #76): the framed
//! little-endian codec and the message vocabulary shared by the
//! host-side proxy (`ruviform-plugin-host`) and a plugin-side runner
//! (`ruviform-plugin-export`).
//!
//! The wire types are the single authoritative definition of the
//! protocol — hand-written little-endian, no serde or other external
//! codec dependency, keeping this crate's dependency tree limited to
//! the Ruviform foundations.
//!
//! ## Framing
//!
//! Every message is one frame: `[u32 LE payload_len][u8
//! kind][payload]`. `payload_len` counts only the payload bytes — the
//! kind byte and the length field itself are excluded, so a frame is
//! `4 + 1 + payload_len` bytes.
//!
//! ## Messages
//!
//! | `kind` | message | direction | payload |
//! | --- | --- | --- | --- |
//! | 0 | [`Message::Hello`] | child → parent | `u16 version`, `u32 capabilities` |
//! | 1 | [`Message::Command`] | parent → child | `u64 seq`, [`Command`] |
//! | 2 | [`Message::Reply`] | child → parent | `u64 seq`, `u8` result tag + [`Reply`] or [`HostError`] |
//! | 3 | [`Message::BlockIn`] | parent → child | `u64 seq` — doorbell for shm in-slot `seq % D` |
//! | 4 | [`Message::BlockDone`] | child → parent | `u64 seq`, [`ProcessResult`] — doorbell for shm out-slot `seq % D` |
//! | 5 | [`Message::Shutdown`] | parent → child | empty — `Destroy`'s wire meaning |
//!
//! ## Versioning
//!
//! `Hello` alone carries `version` ([`WIRE_VERSION`]); no other
//! message repeats it. A version mismatch is a handshake failure
//! decided by the proxy (kill + `HostError::Ipc`), not by the codec —
//! [`decode`] accepts any `u16` version. Payload field-set changes
//! therefore always ride a `WIRE_VERSION` bump.
//!
//! ## Bounded reads and defined rejections
//!
//! The codec is the validation boundary for untrusted peer input:
//! every length-prefixed field is capped by a `WIRE_MAX_*` constant,
//! decode never panics and never partially consumes — a frame that is
//! truncated, length-mismatched, over a declared limit or otherwise
//! malformed is rejected as `Err`([`WireError`]) before any unbounded
//! allocation is attempted. Shared value types are decoded through
//! their constructors/accessors only — no `repr(Rust)` memcpy crosses
//! the wire — so type invariants (ranges, normalized bases, zero
//! `reserved` fields) are re-validated on decode.
//!
//! ## `Instantiate`/`Destroy` vocabulary mapping
//!
//! `HostCommand::Instantiate` maps to spawn + handshake and
//! `HostCommand::Destroy` to `Shutdown` + reap, so [`Command`] carries
//! neither verb — the remaining ten host commands map one-to-one onto
//! its variants, with the handle dropped because a runner child is a
//! singleton instance.

use core::fmt;
use std::io::{self, Read, Write};

use ruviform_audio_core::SampleRate;
use ruviform_media_core::{
    MediaDuration, MediaRange, MediaTime, MusicalTimeInfo, TimeBase, TransportStatus,
};

use crate::{
    Event, EventKind, EventList, GuiHandle, GuiParentToken, HostError, LifecycleError,
    LifecycleEvent, LifecycleState, ParamEvent, ParamId, ParamValue, PluginDescriptor,
    PluginHandle, PrepareError, ProcessConfig, ProcessResult, ProcessStatus, TransportInfo,
};

/// The protocol version carried by [`Message::Hello`]. A proxy and a
/// runner must agree on it exactly — a mismatch fails the handshake
/// (`HostError::Ipc`); payload changes always ride a version bump.
pub const WIRE_VERSION: u16 = 1;

/// Maximum frames per processing block carried on the wire
/// (`Activate` configs and shm block payloads share this bound).
pub const WIRE_MAX_FRAMES: u32 = 8_192;

/// Maximum channel count of a single bus on the wire.
pub const WIRE_MAX_CHANNELS: u32 = 16;

/// Maximum number of events in one length-prefixed event list.
pub const WIRE_MAX_EVENTS: u32 = 1_024;

/// Maximum `payload_len` a frame may declare — larger declarations
/// are [`WireError::LimitExceeded`] and never allocate.
pub const WIRE_MAX_PAYLOAD: u32 = 16 * 1_024 * 1_024;

/// Maximum byte length of one length-prefixed UTF-8 string
/// (descriptor fields, feature names).
pub const WIRE_MAX_STRING_BYTES: u32 = 4_096;

/// Maximum element count of a length-prefixed list of variable-size
/// items (scanned descriptors, descriptor features).
pub const WIRE_MAX_ELEMENTS: u32 = 1_024;

/// Maximum byte length of an opaque plugin-state blob
/// (`SaveState`/`RestoreState` replies and commands).
pub const WIRE_MAX_STATE_BYTES: u32 = 16 * 1_024 * 1_024;

/// Byte length of the frame header: `u32` payload length plus `u8`
/// kind. A stream reader first reads 4 length bytes, then
/// `1 + payload_len` bytes, and hands the whole frame to [`decode`].
pub const WIRE_FRAME_HEADER_LEN: usize = 5;

/// The L1 shared-memory data-plane layout — the **authoritative
/// wire-side definition** of the `[header][in-slots][out-slots]`
/// segment and the fixed slot record both ends consume:
/// `ruviform-plugin-host::l1::shm` (the creating owner) and
/// `ruviform-plugin-export::shm` (the attaching runner). Neither side
/// re-declares these constants — the layout is wire data, so the
/// wire contract owner defines it.
///
/// All fields are little-endian. The header is 7 `u32` fields
/// ([`HEADER_LEN`] bytes); each slot is [`SLOT_BYTES`]:
///
/// | offset | field | size | notes |
/// | --- | --- | --- | --- |
/// | +0 | `seq` | `u64` | echoes the doorbell `seq` |
/// | +8 | `frames` | `u32` | ≤ [`WIRE_MAX_FRAMES`] |
/// | +12 | `channels` | `u32` | ≤ [`WIRE_MAX_CHANNELS`] |
/// | +16 | `event_count` | `u32` | ≤ [`WIRE_MAX_EVENTS`] |
/// | +20 | `flags` | `u32` | bit0 ([`FLAG_TRANSPORT`]) |
/// | +24 | `transport` | [`TRANSPORT_LEN`] | wire `TransportInfo` when flagged |
/// | +120 | `result` | [`RESULT_LEN`] | wire `ProcessResult` (out-slots) |
/// | +128 | `events` | `event_count * `[`EVENT_WIRE_SIZE`] | wire `Event` records |
/// | +33920 | `samples` | [`SAMPLES_LEN`] | planar `f32` LE, channel stride [`SAMPLE_STRIDE`] |
///
/// The segment holds no shared control state — slot selection is the
/// doorbell `seq` (`seq % depth`) under strict in/out alternation.
pub mod shm {
    use super::{WIRE_MAX_CHANNELS, WIRE_MAX_EVENTS, WIRE_MAX_FRAMES};

    /// Segment magic — `"RVL1"` (Ruviform L1) as four LE bytes,
    /// validated on attach.
    pub const MAGIC: u32 = u32::from_le_bytes(*b"RVL1");

    /// Byte length of the fixed segment header (7 LE `u32` fields:
    /// `magic`, `wire_version`, `in_depth`, `out_depth`, `slot_bytes`,
    /// `in_off`, `out_off`).
    pub const HEADER_LEN: usize = 28;

    /// Header field offsets — every field is one LE `u32`.
    pub const OFF_MAGIC: usize = 0;
    /// Header `wire_version` field offset.
    pub const OFF_WIRE_VERSION: usize = 4;
    /// Header `in_depth` field offset.
    pub const OFF_IN_DEPTH: usize = 8;
    /// Header `out_depth` field offset.
    pub const OFF_OUT_DEPTH: usize = 12;
    /// Header `slot_bytes` field offset.
    pub const OFF_SLOT_BYTES: usize = 16;
    /// Header `in_off` field offset.
    pub const OFF_IN_OFF: usize = 20;
    /// Header `out_off` field offset.
    pub const OFF_OUT_OFF: usize = 24;

    /// `flags` bit 0: the slot carries a wire-encoded `TransportInfo`
    /// in its `transport` region. All other bits are reserved and
    /// must be zero.
    pub const FLAG_TRANSPORT: u32 = 1;

    /// Slot field offsets — `seq` is `u64`, the rest are `u32`.
    pub const SLOT_OFF_SEQ: usize = 0;
    /// Slot `frames` field offset.
    pub const SLOT_OFF_FRAMES: usize = 8;
    /// Slot `channels` field offset.
    pub const SLOT_OFF_CHANNELS: usize = 12;
    /// Slot `event_count` field offset.
    pub const SLOT_OFF_EVENT_COUNT: usize = 16;
    /// Slot `flags` field offset.
    pub const SLOT_OFF_FLAGS: usize = 20;

    /// Byte offset of the slot `transport` region.
    pub const TRANSPORT_OFF: usize = 24;
    /// Reserved capacity of the `transport` region — the maximum
    /// wire size of `TransportInfo` under `WIRE_VERSION` 1 is 91 B.
    pub const TRANSPORT_LEN: usize = 96;
    /// Byte offset of the slot `result` region.
    pub const RESULT_OFF: usize = TRANSPORT_OFF + TRANSPORT_LEN;
    /// Reserved capacity of the `result` region — a wire
    /// `ProcessResult` is 5 B.
    pub const RESULT_LEN: usize = 8;
    /// Byte offset of the slot `events` region.
    pub const EVENTS_OFF: usize = RESULT_OFF + RESULT_LEN;
    /// Byte length of one wire-encoded `Event` record under
    /// `WIRE_VERSION` 1 (`u32` offset + `u8` kind + `u32` param id +
    /// `f64` value + `u64` reserved ×2).
    pub const EVENT_WIRE_SIZE: usize = 33;
    /// Byte offset of the slot `samples` region.
    pub const SAMPLES_OFF: usize = EVENTS_OFF + EVENT_WIRE_SIZE * WIRE_MAX_EVENTS as usize;
    /// Byte stride of one channel plane inside `samples`
    /// (`WIRE_MAX_FRAMES` LE `f32`s).
    pub const SAMPLE_STRIDE: usize = WIRE_MAX_FRAMES as usize * 4;
    /// Byte length of the fixed-capacity `samples` region
    /// (`WIRE_MAX_CHANNELS` planes).
    pub const SAMPLES_LEN: usize = WIRE_MAX_CHANNELS as usize * SAMPLE_STRIDE;
    /// Fixed byte length of one in/out slot under `WIRE_VERSION` 1.
    pub const SLOT_BYTES: usize = SAMPLES_OFF + SAMPLES_LEN;

    /// Byte offset of the in-slot array — always right after the
    /// header.
    pub const IN_SLOTS_OFF: usize = HEADER_LEN;

    /// Byte offset of the out-slot array for `in_depth` in-slots —
    /// the value stored in the header `out_off` field.
    #[must_use]
    pub const fn out_slots_off(in_depth: u32) -> usize {
        HEADER_LEN + in_depth as usize * SLOT_BYTES
    }

    /// The declared segment length: header plus `in_depth` +
    /// `out_depth` slots (saturating — callers size mappings with it;
    /// on Windows the mapped length may round up to the allocation
    /// granularity, so attach checks coverage, not equality).
    #[must_use]
    pub const fn segment_len(in_depth: u32, out_depth: u32) -> usize {
        HEADER_LEN
            .saturating_add((in_depth as usize).saturating_mul(SLOT_BYTES))
            .saturating_add((out_depth as usize).saturating_mul(SLOT_BYTES))
    }
}

/// One frame on the wire — the six message kinds of the L1 protocol.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy; the wire itself is strictly versioned by [`WIRE_VERSION`],
/// so new messages arrive only with a protocol revision.
///
/// **RT contract:** non-RT only — the codec allocates on the
/// control-plane pipe.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message};
///
/// let mut frame = Vec::new();
/// wire::encode(&Message::Shutdown, &mut frame).unwrap();
/// assert_eq!(wire::decode(&frame), Ok(Message::Shutdown));
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// Child → parent: the first frame after spawn. `version` must
    /// equal [`WIRE_VERSION`] or the handshake fails.
    Hello {
        /// Protocol version — [`WIRE_VERSION`] on a live handshake.
        version: u16,
        /// Capability bitmask reserved for protocol extensions;
        /// must be `0` under [`WIRE_VERSION`] `1` — decode rejects a
        /// nonzero value as [`WireError::Malformed`], like any other
        /// nonzero reserved field.
        capabilities: u32,
    },
    /// Parent → child: one serialized command, correlated by `seq`.
    Command {
        /// Correlation id echoed by the matching [`Message::Reply`].
        seq: u64,
        /// The command verb and its payload.
        cmd: Command,
    },
    /// Child → parent: the result of a [`Message::Command`] — either a
    /// reply payload or a shared [`HostError`].
    Reply {
        /// Correlation id of the command being answered.
        seq: u64,
        /// `Ok` carries a [`Reply`]; `Err` carries a [`HostError`].
        result: Result<Reply, HostError>,
    },
    /// Parent → child: doorbell for shm input slot `seq % D`.
    BlockIn {
        /// Block sequence number; selects the shm slot.
        seq: u64,
    },
    /// Child → parent: doorbell for shm output slot `seq % D`.
    BlockDone {
        /// Block sequence number; selects the shm slot.
        seq: u64,
        /// The plugin's processing status for this block.
        result: ProcessResult,
    },
    /// Parent → child: clean teardown (`HostCommand::Destroy`'s wire
    /// meaning). The runner normalizes the lifecycle and exits.
    Shutdown,
}

/// The wire mirror of the host's handle-qualified commands.
///
/// One variant per dispatchable verb; `Instantiate`/`Destroy` have no
/// wire verb (spawn + handshake and `Shutdown` + reap carry them) and
/// no variant carries a [`PluginHandle`] — a runner child is a
/// singleton instance.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new verbs arrive only with a [`WIRE_VERSION`] bump.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Enumerate the runner's built-in descriptors →
    /// [`Reply::Scanned`].
    Scan,
    /// `Created → Initialized` — run the plugin's init hook.
    Initialize,
    /// `Initialized → Activated` — run `prepare` under `config`.
    Activate {
        /// Processing configuration for activation.
        config: ProcessConfig,
    },
    /// `Activated → Processing` — begin the `process` stream.
    StartProcessing,
    /// `Processing → Activated` — stop the `process` stream.
    StopProcessing,
    /// `Activated → Initialized` — release activation resources.
    Deactivate,
    /// Serialize the instance's state → [`Reply::State`].
    SaveState,
    /// Restore a previously saved state byte stream.
    RestoreState {
        /// Bytes produced by an earlier `SaveState`.
        bytes: Vec<u8>,
    },
    /// Open the plugin GUI under an opaque parent window →
    /// [`Reply::Opened`].
    OpenGui {
        /// Platform parent window token.
        parent: GuiParentToken,
    },
    /// Close the plugin GUI.
    CloseGui,
}

/// The reply payload of a successful wire [`Command`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new replies arrive only with a [`WIRE_VERSION`] bump.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The command completed with no payload.
    Ok,
    /// `Scan` result: descriptors in factory order.
    Scanned(Vec<Descriptor>),
    /// `SaveState` result: plugin-defined state bytes.
    State {
        /// Serialized state.
        bytes: Vec<u8>,
    },
    /// `OpenGui` result: token for the opened surface.
    Opened {
        /// Handle of the opened GUI surface.
        gui: GuiHandle,
    },
}

/// The owned wire form of a [`PluginDescriptor`].
///
/// `PluginDescriptor` fields are `&'static` — a child process cannot
/// receive them over the wire, so descriptors travel as owned
/// strings. The `&'static` → owned conversion happens exactly once at
/// encode via [`From`]; the reverse conversion never happens (the
/// public `HostReply::Scanned` keeps returning registration-supplied
/// `'static` values — no `Box::leak`, no forged statics).
///
/// **RT contract:** non-RT only — owns `String`/`Vec` storage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Descriptor {
    /// Stable standard-neutral identifier (reverse-DNS style).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Vendor / author name.
    pub vendor: String,
    /// Version string in a standard-neutral form.
    pub version: String,
    /// Feature descriptors (category tags); may be empty.
    pub features: Vec<String>,
}

impl Descriptor {
    /// Field-wise equality against a static [`PluginDescriptor`] —
    /// the check the proxy runs when matching a scanned child
    /// descriptor against its registered spec.
    ///
    /// **RT contract:** non-RT only — string comparisons.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_plugin_core::PluginDescriptor;
    /// use ruviform_plugin_core::wire::Descriptor;
    ///
    /// const GAIN: PluginDescriptor = PluginDescriptor {
    ///     id: "org.example.gain",
    ///     name: "Gain",
    ///     vendor: "Ruviform",
    ///     version: "1.0.0",
    ///     features: &["audio-effect"],
    /// };
    /// assert!(Descriptor::from(GAIN).matches(&GAIN));
    /// ```
    #[must_use]
    pub fn matches(&self, descriptor: &PluginDescriptor) -> bool {
        self.id == descriptor.id
            && self.name == descriptor.name
            && self.vendor == descriptor.vendor
            && self.version == descriptor.version
            && self.features.len() == descriptor.features.len()
            && self
                .features
                .iter()
                .zip(descriptor.features.iter())
                .all(|(owned, static_str)| owned == static_str)
    }
}

impl From<PluginDescriptor> for Descriptor {
    fn from(descriptor: PluginDescriptor) -> Self {
        Self {
            id: descriptor.id.to_owned(),
            name: descriptor.name.to_owned(),
            vendor: descriptor.vendor.to_owned(),
            version: descriptor.version.to_owned(),
            features: descriptor
                .features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
        }
    }
}

/// Rejection of a malformed or out-of-bounds wire frame.
///
/// Every variant is a defined rejection — decode returns `Err` for
/// truncated input, length mismatches, unassigned tags, over-limit
/// declarations and constructor-invariant violations; it never panics
/// and never partially consumes.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WireError {
    /// The frame or a nested payload ended before the declared or
    /// required bytes were present.
    Truncated,
    /// Bytes were present but violate the wire format: a declared
    /// `payload_len` that does not match the actual payload bytes,
    /// trailing bytes after a complete item, an unassigned tag in a
    /// typed field, invalid UTF-8, a non-zero reserved field, or a
    /// constructor-invariant rejection (e.g. `min_frames >
    /// max_frames`, a zero sample rate or a zero time-base part —
    /// non-canonical time bases normalize on decode instead).
    Malformed,
    /// The frame's `kind` byte is not one of the six assigned message
    /// kinds.
    UnknownKind(u8),
    /// A declared length or count exceeded a `WIRE_MAX_*` bound — the
    /// allocation is never attempted.
    LimitExceeded,
    /// A value has no representation under [`WIRE_VERSION`] (a
    /// `#[non_exhaustive]` variant added after this protocol
    /// revision) — a wire revision is required to carry it.
    UnsupportedVariant,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("wire frame truncated"),
            Self::Malformed => f.write_str("malformed wire payload"),
            Self::UnknownKind(kind) => write!(f, "unknown wire message kind {kind}"),
            Self::LimitExceeded => f.write_str("declared wire length exceeds a WIRE_MAX bound"),
            Self::UnsupportedVariant => {
                f.write_str("value has no representation in this wire version")
            }
        }
    }
}

impl core::error::Error for WireError {}

/// A bounded cursor over a payload for the `decode_*` functions.
///
/// Reads are checked: every accessor returns [`WireError::Truncated`]
/// once the remaining bytes run out, and length-prefixed accessors
/// cap declarations against a `WIRE_MAX_*` bound before copying.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Reader};
///
/// let mut payload = Vec::new();
/// wire::encode_media_time(&ruviform_media_core::MediaTime::ZERO, &mut payload);
/// let mut reader = Reader::new(&payload);
/// assert_eq!(wire::decode_media_time(&mut reader).unwrap().value(), 0);
/// assert!(reader.is_empty());
/// ```
#[derive(Debug)]
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader over `buf`, positioned at the start.
    #[must_use]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { rest: buf }
    }

    /// Bytes not yet consumed.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Whether every byte has been consumed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// Consumes `len` bytes — [`WireError::Truncated`] when fewer
    /// remain.
    pub fn take(&mut self, len: usize) -> Result<&'a [u8], WireError> {
        let Some((head, tail)) = self.rest.split_at_checked(len) else {
            return Err(WireError::Truncated);
        };
        self.rest = tail;
        Ok(head)
    }

    /// Reads one `u8`.
    pub fn read_u8(&mut self) -> Result<u8, WireError> {
        let [byte] = *self.array::<1>()?;
        Ok(byte)
    }

    /// Reads one little-endian `u16`.
    pub fn read_u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_le_bytes(*self.array::<2>()?))
    }

    /// Reads one little-endian `u32`.
    pub fn read_u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(*self.array::<4>()?))
    }

    /// Reads one little-endian `u64`.
    pub fn read_u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(*self.array::<8>()?))
    }

    /// Reads one little-endian `i64`.
    pub fn read_i64(&mut self) -> Result<i64, WireError> {
        Ok(i64::from_le_bytes(*self.array::<8>()?))
    }

    /// Reads one little-endian `f64`.
    pub fn read_f64(&mut self) -> Result<f64, WireError> {
        Ok(f64::from_le_bytes(*self.array::<8>()?))
    }

    /// Reads a `u32` element count bounded by `max` —
    /// [`WireError::LimitExceeded`] when the declared count exceeds
    /// the bound, [`WireError::Truncated`] when it cannot possibly be
    /// satisfied by the remaining bytes (every element occupies at
    /// least one byte).
    pub fn read_count(&mut self, max: u32) -> Result<u32, WireError> {
        let count = self.read_u32()?;
        if count > max {
            return Err(WireError::LimitExceeded);
        }
        if count as usize > self.remaining() {
            return Err(WireError::Truncated);
        }
        Ok(count)
    }

    /// Reads a length-prefixed byte string bounded by `max`.
    pub fn read_bytes(&mut self, max: u32) -> Result<Vec<u8>, WireError> {
        let len = self.read_u32()?;
        if len > max {
            return Err(WireError::LimitExceeded);
        }
        Ok(self.take(len as usize)?.to_vec())
    }

    /// Reads a length-prefixed UTF-8 string bounded by
    /// [`WIRE_MAX_STRING_BYTES`].
    pub fn read_string(&mut self) -> Result<String, WireError> {
        let bytes = self.read_bytes(WIRE_MAX_STRING_BYTES)?;
        String::from_utf8(bytes).map_err(|_| WireError::Malformed)
    }

    /// Consumes exactly `N` bytes as an array — the primitive-read
    /// workhorse; never panics.
    fn array<const N: usize>(&mut self) -> Result<&'a [u8; N], WireError> {
        match self.rest.split_first_chunk::<N>() {
            Some((head, tail)) => {
                self.rest = tail;
                Ok(head)
            }
            None => Err(WireError::Truncated),
        }
    }
}

/// Appends `message` to `out` as one `[u32 LE payload_len][u8
/// kind][payload]` frame.
///
/// Encoding never fails on wire-representable values — every wire
/// type serializes verbatim. Encode is the trusted side: it does not
/// enforce the per-field `WIRE_MAX_*` bounds, so e.g. a state blob
/// over [`WIRE_MAX_STATE_BYTES`] or a string over
/// [`WIRE_MAX_STRING_BYTES`] produces a frame the peer's [`decode`]
/// must reject.
///
/// The append is **all-or-nothing**: on failure `out` is truncated
/// back to its pre-call length, so no partial frame (e.g. a dangling
/// `REPLY_ERR` tag) is left behind.
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when a nested value has no
/// representation under [`WIRE_VERSION`] — currently a
/// [`Message::Reply`] whose `Err` carries the local-only
/// [`HostError::EndpointUnavailable`] (or a `#[non_exhaustive]`
/// variant added after this protocol revision). `out` is left
/// unchanged.
///
/// # Panics
///
/// Panics only if the encoded payload exceeds `u32::MAX` bytes
/// (~4 GiB) or a length-prefixed field exceeds `u32::MAX` elements —
/// unreachable for real plugin data, which is bounded far below by
/// `WIRE_MAX_PAYLOAD` on the decode side anyway.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message};
///
/// let mut frame = Vec::new();
/// wire::encode(
///     &Message::Hello {
///         version: wire::WIRE_VERSION,
///         capabilities: 0,
///     },
///     &mut frame,
/// )
/// .unwrap();
/// assert_eq!(frame.len(), wire::WIRE_FRAME_HEADER_LEN + 6);
/// ```
pub fn encode(message: &Message, out: &mut Vec<u8>) -> Result<(), WireError> {
    let start = out.len();
    out.extend_from_slice(&0_u32.to_le_bytes()); // payload_len placeholder
    if let Err(error) = encode_body(message, out) {
        // All-or-nothing append: roll the partial frame (e.g. a
        // dangling `REPLY_ERR` tag) back so the caller's `out` is
        // left exactly as found — every `Err` exit from `encode_body`
        // lands here, not just the arms fallible today.
        out.truncate(start);
        return Err(error);
    }
    let payload_len = len_u32(out.len() - start - WIRE_FRAME_HEADER_LEN);
    out[start..start + 4].copy_from_slice(&payload_len.to_le_bytes());
    Ok(())
}

/// Writes the `kind` byte plus payload fields of `message` after the
/// length placeholder [`encode`] already emitted. Fallible only
/// through a nested value with no [`WIRE_VERSION`] representation —
/// `encode` owns the all-or-nothing rollback for every `Err` from
/// here.
fn encode_body(message: &Message, out: &mut Vec<u8>) -> Result<(), WireError> {
    match message {
        Message::Hello {
            version,
            capabilities,
        } => {
            push_u8(out, KIND_HELLO);
            push_u16(out, *version);
            push_u32(out, *capabilities);
        }
        Message::Command { seq, cmd } => {
            push_u8(out, KIND_COMMAND);
            push_u64(out, *seq);
            encode_command(cmd, out);
        }
        Message::Reply { seq, result } => {
            push_u8(out, KIND_REPLY);
            push_u64(out, *seq);
            match result {
                Ok(reply) => {
                    push_u8(out, REPLY_OK);
                    encode_reply(reply, out);
                }
                Err(error) => {
                    push_u8(out, REPLY_ERR);
                    // The only fallible step today — propagates to
                    // `encode`, which performs the rollback.
                    encode_host_error(error, out)?;
                }
            }
        }
        Message::BlockIn { seq } => {
            push_u8(out, KIND_BLOCK_IN);
            push_u64(out, *seq);
        }
        Message::BlockDone { seq, result } => {
            push_u8(out, KIND_BLOCK_DONE);
            push_u64(out, *seq);
            encode_process_result(result, out);
        }
        Message::Shutdown => {
            push_u8(out, KIND_SHUTDOWN);
        }
    }
    Ok(())
}

/// Decodes one complete frame (`[u32 LE payload_len][u8
/// kind][payload]`).
///
/// The frame must be exactly one message: a `payload_len` that does
/// not match the supplied bytes, an unassigned `kind` byte, a payload
/// that is truncated or malformed, or a declared length over a
/// `WIRE_MAX_*` bound are all defined rejections — `Err`, never a
/// panic, never a partial consume.
///
/// # Errors
///
/// [`WireError`] — see each variant for the failure condition.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message, WireError};
///
/// let mut frame = Vec::new();
/// wire::encode(&Message::BlockIn { seq: 9 }, &mut frame).unwrap();
/// assert_eq!(wire::decode(&frame), Ok(Message::BlockIn { seq: 9 }));
/// // A trailing garbage byte makes the declared length a lie.
/// frame.push(0);
/// assert_eq!(wire::decode(&frame), Err(WireError::Malformed));
/// ```
pub fn decode(frame: &[u8]) -> Result<Message, WireError> {
    let mut head = Reader::new(frame);
    let payload_len = head.read_u32()? as usize;
    if payload_len > WIRE_MAX_PAYLOAD as usize {
        return Err(WireError::LimitExceeded);
    }
    let kind = head.read_u8()?;
    let payload = head.take(payload_len)?;
    if !head.is_empty() {
        // Declared length disagrees with the supplied bytes.
        return Err(WireError::Malformed);
    }
    let mut reader = Reader::new(payload);
    let message = match kind {
        KIND_HELLO => {
            let version = reader.read_u16()?;
            let capabilities = reader.read_u32()?;
            // Reserved under this protocol revision — no capability
            // bits are defined, so a nonzero value is the same defined
            // rejection as any other nonzero reserved field.
            if capabilities != 0 {
                return Err(WireError::Malformed);
            }
            Message::Hello {
                version,
                capabilities,
            }
        }
        KIND_COMMAND => Message::Command {
            seq: reader.read_u64()?,
            cmd: decode_command(&mut reader)?,
        },
        KIND_REPLY => Message::Reply {
            seq: reader.read_u64()?,
            result: match reader.read_u8()? {
                REPLY_OK => Ok(decode_reply(&mut reader)?),
                REPLY_ERR => Err(decode_host_error(&mut reader)?),
                _ => return Err(WireError::Malformed),
            },
        },
        KIND_BLOCK_IN => Message::BlockIn {
            seq: reader.read_u64()?,
        },
        KIND_BLOCK_DONE => Message::BlockDone {
            seq: reader.read_u64()?,
            result: decode_process_result(&mut reader)?,
        },
        KIND_SHUTDOWN => Message::Shutdown,
        _ => return Err(WireError::UnknownKind(kind)),
    };
    if !reader.is_empty() {
        // Trailing bytes inside a declared-complete payload.
        return Err(WireError::Malformed);
    }
    Ok(message)
}

// --- Frame kind bytes --------------------------------------------------

const KIND_HELLO: u8 = 0;
const KIND_COMMAND: u8 = 1;
const KIND_REPLY: u8 = 2;
const KIND_BLOCK_IN: u8 = 3;
const KIND_BLOCK_DONE: u8 = 4;
const KIND_SHUTDOWN: u8 = 5;

/// [`Message::Reply`] result discriminant: `Ok(Reply)`.
const REPLY_OK: u8 = 0;
/// [`Message::Reply`] result discriminant: `Err(HostError)`.
const REPLY_ERR: u8 = 1;

// --- Encode helpers ----------------------------------------------------

/// `usize` → `u32` for wire length prefixes. Lengths beyond `u32::MAX`
/// cannot exist for real wire data; the `expect` documents that
/// encode-side contract (untrusted-input panics are impossible —
/// untrusted bytes only ever flow through `decode_*`). `WIRE_MAX_*`
/// field caps are not checked here either — an over-bound field
/// encodes verbatim and becomes the peer's defined rejection.
fn len_u32(len: usize) -> u32 {
    u32::try_from(len).expect("wire length prefix fits u32")
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// `u32` length prefix + raw bytes.
fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, len_u32(bytes.len()));
    out.extend_from_slice(bytes);
}

/// `u32` length prefix + UTF-8 bytes.
fn push_str(out: &mut Vec<u8>, value: &str) {
    push_bytes(out, value.as_bytes());
}

// --- Command / Reply payloads ------------------------------------------

fn encode_command(command: &Command, out: &mut Vec<u8>) {
    match command {
        Command::Scan => push_u8(out, 0),
        Command::Initialize => push_u8(out, 1),
        Command::Activate { config } => {
            push_u8(out, 2);
            encode_process_config(config, out);
        }
        Command::StartProcessing => push_u8(out, 3),
        Command::StopProcessing => push_u8(out, 4),
        Command::Deactivate => push_u8(out, 5),
        Command::SaveState => push_u8(out, 6),
        Command::RestoreState { bytes } => {
            push_u8(out, 7);
            push_bytes(out, bytes);
        }
        Command::OpenGui { parent } => {
            push_u8(out, 8);
            encode_gui_parent_token(parent, out);
        }
        Command::CloseGui => push_u8(out, 9),
    }
}

fn decode_command(reader: &mut Reader<'_>) -> Result<Command, WireError> {
    match reader.read_u8()? {
        0 => Ok(Command::Scan),
        1 => Ok(Command::Initialize),
        2 => Ok(Command::Activate {
            config: decode_process_config(reader)?,
        }),
        3 => Ok(Command::StartProcessing),
        4 => Ok(Command::StopProcessing),
        5 => Ok(Command::Deactivate),
        6 => Ok(Command::SaveState),
        7 => Ok(Command::RestoreState {
            bytes: reader.read_bytes(WIRE_MAX_STATE_BYTES)?,
        }),
        8 => Ok(Command::OpenGui {
            parent: decode_gui_parent_token(reader)?,
        }),
        9 => Ok(Command::CloseGui),
        _ => Err(WireError::Malformed),
    }
}

fn encode_reply(reply: &Reply, out: &mut Vec<u8>) {
    match reply {
        Reply::Ok => push_u8(out, 0),
        Reply::Scanned(descriptors) => {
            push_u8(out, 1);
            push_u32(out, len_u32(descriptors.len()));
            for descriptor in descriptors {
                encode_descriptor(descriptor, out);
            }
        }
        Reply::State { bytes } => {
            push_u8(out, 2);
            push_bytes(out, bytes);
        }
        Reply::Opened { gui } => {
            push_u8(out, 3);
            encode_gui_handle(gui, out);
        }
    }
}

fn decode_reply(reader: &mut Reader<'_>) -> Result<Reply, WireError> {
    match reader.read_u8()? {
        0 => Ok(Reply::Ok),
        1 => {
            let count = reader.read_count(WIRE_MAX_ELEMENTS)?;
            let mut descriptors = Vec::with_capacity(count as usize);
            for _ in 0..count {
                descriptors.push(decode_descriptor(reader)?);
            }
            Ok(Reply::Scanned(descriptors))
        }
        2 => Ok(Reply::State {
            bytes: reader.read_bytes(WIRE_MAX_STATE_BYTES)?,
        }),
        3 => Ok(Reply::Opened {
            gui: decode_gui_handle(reader)?,
        }),
        _ => Err(WireError::Malformed),
    }
}

// --- Fixed wire layouts ------------------------------------------------
//
// Shared value types have a repr-fixed wire layout: values encode via
// accessors and decode via constructors, so type invariants are
// re-validated at the boundary and no `repr(Rust)` memory image ever
// crosses the wire. `reserved` fields are written for layout
// stability and must decode as zero.

/// Appends a [`ProcessConfig`] (`u32` sample rate, `u32` min frames,
/// `u32` max frames, `u64` reserved ×2).
///
/// # Panics
///
/// Never — every field serializes verbatim.
pub fn encode_process_config(config: &ProcessConfig, out: &mut Vec<u8>) {
    push_u32(out, config.sample_rate().get());
    push_u32(out, config.min_frames());
    push_u32(out, config.max_frames());
    for reserved in config.reserved() {
        push_u64(out, reserved);
    }
}

/// Reads a [`ProcessConfig`] — rejects a zero sample rate,
/// `min_frames > max_frames` and non-zero reserved words.
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_process_config(reader: &mut Reader<'_>) -> Result<ProcessConfig, WireError> {
    let rate = reader.read_u32()?;
    let min_frames = reader.read_u32()?;
    let max_frames = reader.read_u32()?;
    let reserved = [reader.read_u64()?, reader.read_u64()?];
    if reserved != [0; 2] {
        return Err(WireError::Malformed);
    }
    let sample_rate = SampleRate::new(rate).map_err(|_| WireError::Malformed)?;
    ProcessConfig::new(sample_rate, min_frames, max_frames).map_err(|_| WireError::Malformed)
}

/// Appends a [`ProcessResult`] (`u8` status tag, `u32` tail frames).
pub fn encode_process_result(result: &ProcessResult, out: &mut Vec<u8>) {
    push_u8(out, encode_process_status(result.status));
    push_u32(out, result.tail_frames);
}

/// Reads a [`ProcessResult`].
///
/// # Errors
///
/// [`WireError`] — truncated, or an unassigned status tag.
pub fn decode_process_result(reader: &mut Reader<'_>) -> Result<ProcessResult, WireError> {
    let status = match reader.read_u8()? {
        0 => ProcessStatus::Continue,
        1 => ProcessStatus::Sleep,
        2 => ProcessStatus::Error,
        _ => return Err(WireError::Malformed),
    };
    let tail_frames = reader.read_u32()?;
    Ok(ProcessResult {
        status,
        tail_frames,
    })
}

fn encode_process_status(status: ProcessStatus) -> u8 {
    match status {
        ProcessStatus::Continue => 0,
        ProcessStatus::Sleep => 1,
        ProcessStatus::Error => 2,
    }
}

/// Appends a [`GuiHandle`] (`u32` raw token).
pub fn encode_gui_handle(handle: &GuiHandle, out: &mut Vec<u8>) {
    push_u32(out, handle.raw());
}

/// Reads a [`GuiHandle`].
///
/// # Errors
///
/// [`WireError::Truncated`].
pub fn decode_gui_handle(reader: &mut Reader<'_>) -> Result<GuiHandle, WireError> {
    Ok(GuiHandle::new(reader.read_u32()?))
}

/// Appends a [`GuiParentToken`] (`u64` raw token).
pub fn encode_gui_parent_token(token: &GuiParentToken, out: &mut Vec<u8>) {
    push_u64(out, token.raw());
}

/// Reads a [`GuiParentToken`].
///
/// # Errors
///
/// [`WireError::Truncated`].
pub fn decode_gui_parent_token(reader: &mut Reader<'_>) -> Result<GuiParentToken, WireError> {
    Ok(GuiParentToken::new(reader.read_u64()?))
}

/// Appends a [`PluginHandle`] (`u32` slot, `u16` generation).
pub fn encode_plugin_handle(handle: &PluginHandle, out: &mut Vec<u8>) {
    push_u32(out, handle.slot());
    push_u16(out, handle.generation());
}

/// Reads a [`PluginHandle`].
///
/// # Errors
///
/// [`WireError::Truncated`].
pub fn decode_plugin_handle(reader: &mut Reader<'_>) -> Result<PluginHandle, WireError> {
    let slot = reader.read_u32()?;
    let generation = reader.read_u16()?;
    Ok(PluginHandle::new(slot, generation))
}

/// Appends a [`HostError`] (`u8` variant tag + variant payload:
/// `Lifecycle` carries `u8` lifecycle-error tag, `u8` state, `u8`
/// event; `Prepare` carries `u8` prepare-error tag).
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when `error` has no
/// representation under [`WIRE_VERSION`] — the local-only
/// [`HostError::EndpointUnavailable`] (a host-local topology fact,
/// deliberately not collapsed into `Ipc`/`Unsupported`) or a
/// `#[non_exhaustive]` variant added after this protocol revision.
/// Nothing is appended on failure.
pub fn encode_host_error(error: &HostError, out: &mut Vec<u8>) -> Result<(), WireError> {
    let start = out.len();
    let result: Result<(), WireError> = match error {
        HostError::UnknownPluginId => {
            push_u8(out, 0);
            Ok(())
        }
        HostError::InvalidHandle => {
            push_u8(out, 1);
            Ok(())
        }
        HostError::Lifecycle(lifecycle) => {
            push_u8(out, 2);
            encode_lifecycle_error(*lifecycle, out);
            Ok(())
        }
        HostError::Prepare(prepare) => {
            push_u8(out, 3);
            push_u8(out, encode_prepare_error(*prepare));
            Ok(())
        }
        HostError::PluginFailed => {
            push_u8(out, 4);
            Ok(())
        }
        HostError::Unsupported => {
            push_u8(out, 5);
            Ok(())
        }
        HostError::Failed => {
            push_u8(out, 6);
            Ok(())
        }
        HostError::Ipc => {
            push_u8(out, 7);
            Ok(())
        }
        HostError::DuplicatePluginId => {
            push_u8(out, 8);
            Ok(())
        }
        _ => Err(WireError::UnsupportedVariant),
    };
    if let Err(e) = result {
        // "Nothing is appended on failure" is a per-call guarantee:
        // roll back whatever this call wrote so a rejection after a
        // partial field can never leak into the caller's buffer.
        out.truncate(start);
        return Err(e);
    }
    Ok(())
}

/// Reads a [`HostError`].
///
/// # Errors
///
/// [`WireError`] — truncated, or an unassigned variant/field tag.
pub fn decode_host_error(reader: &mut Reader<'_>) -> Result<HostError, WireError> {
    match reader.read_u8()? {
        0 => Ok(HostError::UnknownPluginId),
        1 => Ok(HostError::InvalidHandle),
        2 => Ok(HostError::Lifecycle(decode_lifecycle_error(reader)?)),
        3 => {
            let prepare = match reader.read_u8()? {
                0 => PrepareError::UnsupportedConfig,
                1 => PrepareError::ActivationFailed,
                _ => return Err(WireError::Malformed),
            };
            Ok(HostError::Prepare(prepare))
        }
        4 => Ok(HostError::PluginFailed),
        5 => Ok(HostError::Unsupported),
        6 => Ok(HostError::Failed),
        7 => Ok(HostError::Ipc),
        8 => Ok(HostError::DuplicatePluginId),
        _ => Err(WireError::Malformed),
    }
}

fn encode_lifecycle_error(error: LifecycleError, out: &mut Vec<u8>) {
    match error {
        LifecycleError::IllegalTransition { from, event } => {
            push_u8(out, 0);
            push_u8(out, encode_lifecycle_state(from));
            push_u8(out, encode_lifecycle_event(event));
        }
    }
}

fn decode_lifecycle_error(reader: &mut Reader<'_>) -> Result<LifecycleError, WireError> {
    match reader.read_u8()? {
        0 => Ok(LifecycleError::IllegalTransition {
            from: decode_lifecycle_state(reader)?,
            event: decode_lifecycle_event(reader)?,
        }),
        _ => Err(WireError::Malformed),
    }
}

fn encode_lifecycle_state(state: LifecycleState) -> u8 {
    match state {
        LifecycleState::Created => 0,
        LifecycleState::Initialized => 1,
        LifecycleState::Activated => 2,
        LifecycleState::Processing => 3,
        LifecycleState::Destroyed => 4,
        LifecycleState::Failed => 5,
    }
}

fn decode_lifecycle_state(reader: &mut Reader<'_>) -> Result<LifecycleState, WireError> {
    match reader.read_u8()? {
        0 => Ok(LifecycleState::Created),
        1 => Ok(LifecycleState::Initialized),
        2 => Ok(LifecycleState::Activated),
        3 => Ok(LifecycleState::Processing),
        4 => Ok(LifecycleState::Destroyed),
        5 => Ok(LifecycleState::Failed),
        _ => Err(WireError::Malformed),
    }
}

fn encode_lifecycle_event(event: LifecycleEvent) -> u8 {
    match event {
        LifecycleEvent::Initialize => 0,
        LifecycleEvent::InitFailed => 1,
        LifecycleEvent::Activate => 2,
        LifecycleEvent::ActivateFailed => 3,
        LifecycleEvent::StartProcessing => 4,
        LifecycleEvent::StopProcessing => 5,
        LifecycleEvent::Deactivate => 6,
        LifecycleEvent::Process => 7,
        LifecycleEvent::Destroy => 8,
        LifecycleEvent::Failed => 9,
    }
}

fn decode_lifecycle_event(reader: &mut Reader<'_>) -> Result<LifecycleEvent, WireError> {
    match reader.read_u8()? {
        0 => Ok(LifecycleEvent::Initialize),
        1 => Ok(LifecycleEvent::InitFailed),
        2 => Ok(LifecycleEvent::Activate),
        3 => Ok(LifecycleEvent::ActivateFailed),
        4 => Ok(LifecycleEvent::StartProcessing),
        5 => Ok(LifecycleEvent::StopProcessing),
        6 => Ok(LifecycleEvent::Deactivate),
        7 => Ok(LifecycleEvent::Process),
        8 => Ok(LifecycleEvent::Destroy),
        9 => Ok(LifecycleEvent::Failed),
        _ => Err(WireError::Malformed),
    }
}

fn encode_prepare_error(error: PrepareError) -> u8 {
    match error {
        PrepareError::UnsupportedConfig => 0,
        PrepareError::ActivationFailed => 1,
    }
}

/// Appends a [`MediaTime`] (`i64` tick value, `u32` base numerator,
/// `u32` base denominator).
pub fn encode_media_time(time: &MediaTime, out: &mut Vec<u8>) {
    push_i64(out, time.value());
    push_u32(out, time.base().numerator());
    push_u32(out, time.base().denominator());
}

/// Reads a [`MediaTime`] — a zero numerator or denominator in the
/// declared base is rejected, so only normalized bases cross.
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_media_time(reader: &mut Reader<'_>) -> Result<MediaTime, WireError> {
    let value = reader.read_i64()?;
    let numerator = reader.read_u32()?;
    let denominator = reader.read_u32()?;
    let base = TimeBase::new(numerator, denominator).map_err(|_| WireError::Malformed)?;
    Ok(MediaTime::new(value, base))
}

fn encode_media_duration(duration: &MediaDuration, out: &mut Vec<u8>) {
    push_i64(out, duration.value());
    push_u32(out, duration.base().numerator());
    push_u32(out, duration.base().denominator());
}

fn decode_media_duration(reader: &mut Reader<'_>) -> Result<MediaDuration, WireError> {
    let value = reader.read_i64()?;
    let numerator = reader.read_u32()?;
    let denominator = reader.read_u32()?;
    let base = TimeBase::new(numerator, denominator).map_err(|_| WireError::Malformed)?;
    Ok(MediaDuration::new(value, base))
}

fn encode_media_range(range: &MediaRange, out: &mut Vec<u8>) {
    encode_media_time(&range.start(), out);
    encode_media_duration(&range.duration(), out);
}

fn decode_media_range(reader: &mut Reader<'_>) -> Result<MediaRange, WireError> {
    let start = decode_media_time(reader)?;
    let duration = decode_media_duration(reader)?;
    Ok(MediaRange::new(start, duration))
}

fn encode_musical_time_info(musical: &MusicalTimeInfo, out: &mut Vec<u8>) {
    push_i64(out, musical.position_ticks);
    push_i64(out, musical.bar_start_tick);
    push_u32(out, musical.bpm_milli);
    push_u16(out, musical.ppqn);
    push_u8(out, musical.sig_num);
    push_u8(out, musical.sig_den);
}

fn decode_musical_time_info(reader: &mut Reader<'_>) -> Result<MusicalTimeInfo, WireError> {
    let position_ticks = reader.read_i64()?;
    let bar_start_tick = reader.read_i64()?;
    let bpm_milli = reader.read_u32()?;
    let ppqn = reader.read_u16()?;
    let sig_num = reader.read_u8()?;
    let sig_den = reader.read_u8()?;
    Ok(MusicalTimeInfo {
        position_ticks,
        bar_start_tick,
        bpm_milli,
        ppqn,
        sig_num,
        sig_den,
    })
}

/// Appends a [`TransportInfo`] (`u8` status tag, [`MediaTime`]
/// position, `u8` presence tag + [`MediaRange`] loop, `u8` presence
/// tag + [`MusicalTimeInfo`] musical, `u64` reserved ×2).
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when `status` is a
/// `#[non_exhaustive]` variant added after [`WIRE_VERSION`] `1` — it
/// has no representation under this protocol revision. Nothing is
/// appended on failure.
pub fn encode_transport(info: &TransportInfo, out: &mut Vec<u8>) -> Result<(), WireError> {
    let status = match info.status() {
        TransportStatus::Stopped => 0,
        TransportStatus::Playing => 1,
        TransportStatus::Paused => 2,
        _ => return Err(WireError::UnsupportedVariant),
    };
    push_u8(out, status);
    encode_media_time(&info.position(), out);
    match info.loop_range() {
        None => push_u8(out, 0),
        Some(range) => {
            push_u8(out, 1);
            encode_media_range(&range, out);
        }
    }
    match info.musical() {
        None => push_u8(out, 0),
        Some(musical) => {
            push_u8(out, 1);
            encode_musical_time_info(&musical, out);
        }
    }
    for reserved in info.reserved() {
        push_u64(out, reserved);
    }
    Ok(())
}

/// Reads a [`TransportInfo`] — unassigned status/presence tags and
/// non-zero reserved words are rejected.
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_transport(reader: &mut Reader<'_>) -> Result<TransportInfo, WireError> {
    let status = match reader.read_u8()? {
        0 => TransportStatus::Stopped,
        1 => TransportStatus::Playing,
        2 => TransportStatus::Paused,
        _ => return Err(WireError::Malformed),
    };
    let position = decode_media_time(reader)?;
    let loop_range = match reader.read_u8()? {
        0 => None,
        1 => Some(decode_media_range(reader)?),
        _ => return Err(WireError::Malformed),
    };
    let musical = match reader.read_u8()? {
        0 => None,
        1 => Some(decode_musical_time_info(reader)?),
        _ => return Err(WireError::Malformed),
    };
    let reserved = [reader.read_u64()?, reader.read_u64()?];
    if reserved != [0; 2] {
        return Err(WireError::Malformed);
    }
    let mut info = TransportInfo::new(status, position);
    if let Some(range) = loop_range {
        info = info.with_loop_range(range);
    }
    if let Some(musical) = musical {
        info = info.with_musical(musical);
    }
    Ok(info)
}

/// Appends an [`Event`] (`u32` frame offset, `u8` kind tag +
/// payload, `u64` reserved ×2). `ParamValue` events carry `u32`
/// param id + `f64` normalized value.
pub fn encode_event(event: &Event, out: &mut Vec<u8>) {
    push_u32(out, event.frame_offset());
    match event.kind() {
        EventKind::ParamValue(param) => {
            push_u8(out, 0);
            push_u32(out, param.param_id.0);
            push_f64(out, param.value.get());
        }
    }
    for reserved in event.reserved() {
        push_u64(out, reserved);
    }
}

/// Reads an [`Event`] — an unassigned kind tag, a non-finite or
/// unrepresentable parameter value and non-zero reserved words are
/// rejected.
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_event(reader: &mut Reader<'_>) -> Result<Event, WireError> {
    let frame_offset = reader.read_u32()?;
    let kind = match reader.read_u8()? {
        0 => EventKind::ParamValue(ParamEvent {
            param_id: ParamId(reader.read_u32()?),
            value: ParamValue::new(reader.read_f64()?).map_err(|_| WireError::Malformed)?,
        }),
        _ => return Err(WireError::Malformed),
    };
    let reserved = [reader.read_u64()?, reader.read_u64()?];
    if reserved != [0; 2] {
        return Err(WireError::Malformed);
    }
    Ok(Event::new(frame_offset, kind))
}

/// Appends an [`EventList`] as `u32` block frames + `u32` event count
/// (≤ [`WIRE_MAX_EVENTS`]) + each [`Event`] in order.
pub fn encode_event_list(list: EventList<'_>, out: &mut Vec<u8>) {
    push_u32(out, list.block_frames());
    push_u32(out, len_u32(list.len()));
    for event in list.iter() {
        encode_event(&event, out);
    }
}

/// Reads a wire event list into an owned `Vec<Event>` plus its block
/// frame count.
///
/// The [`EventList`] contract is borrowed — decode returns the pieces
/// and the caller validates them back into a list
/// (`EventList::new(&events, block_frames)`), which re-checks the
/// ordering/offset bounds at the use boundary.
///
/// # Errors
///
/// [`WireError`] — truncated, a count over [`WIRE_MAX_EVENTS`], or a
/// malformed element.
pub fn decode_event_list(reader: &mut Reader<'_>) -> Result<(u32, Vec<Event>), WireError> {
    let block_frames = reader.read_u32()?;
    let count = reader.read_count(WIRE_MAX_EVENTS)?;
    let mut events = Vec::with_capacity(count as usize);
    for _ in 0..count {
        events.push(decode_event(reader)?);
    }
    Ok((block_frames, events))
}

/// Appends a [`Descriptor`] (four length-prefixed strings +
/// `u32` feature count + each feature string).
pub fn encode_descriptor(descriptor: &Descriptor, out: &mut Vec<u8>) {
    push_str(out, &descriptor.id);
    push_str(out, &descriptor.name);
    push_str(out, &descriptor.vendor);
    push_str(out, &descriptor.version);
    push_u32(out, len_u32(descriptor.features.len()));
    for feature in &descriptor.features {
        push_str(out, feature);
    }
}

/// Reads a [`Descriptor`].
///
/// # Errors
///
/// [`WireError`] — truncated, a field over [`WIRE_MAX_STRING_BYTES`],
/// a feature count over [`WIRE_MAX_ELEMENTS`], or invalid UTF-8.
pub fn decode_descriptor(reader: &mut Reader<'_>) -> Result<Descriptor, WireError> {
    let id = reader.read_string()?;
    let name = reader.read_string()?;
    let vendor = reader.read_string()?;
    let version = reader.read_string()?;
    let count = reader.read_count(WIRE_MAX_ELEMENTS)?;
    let mut features = Vec::with_capacity(count as usize);
    for _ in 0..count {
        features.push(reader.read_string()?);
    }
    Ok(Descriptor {
        id,
        name,
        vendor,
        version,
        features,
    })
}

// --- Stream transport ----------------------------------------------------

/// Reads one frame — `[u32 LE payload_len][u8 kind][payload]` — from
/// `reader` into `buf`, which is replaced wholesale: on `Ok(true)` it
/// holds exactly the complete frame, header included, ready for
/// [`decode`]. The `buf` parameter lets a serving loop reuse one
/// allocation across frames.
///
/// `Ok(false)` is a *clean* EOF at a frame boundary — the peer's
/// stream ended with zero bytes pending where a length field would
/// start. Anything else is `Err`: a partial length field, a truncated
/// body, a declared `payload_len` over [`WIRE_MAX_PAYLOAD`], or the
/// reader's own error. The clean-EOF/truncation distinction is
/// load-bearing for runners: a parent gone at a boundary is a normal
/// teardown, a mid-frame cut is a transport violation.
///
/// **RT contract:** non-RT — blocking `Read` calls plus a `buf`
/// resize that may allocate.
///
/// # Errors
///
/// `std::io::Error` — `InvalidData` for an over-limit declared
/// length, `UnexpectedEof` for a partial header or truncated body, or
/// the reader's own error verbatim.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message};
///
/// let mut frame = Vec::new();
/// wire::encode(&Message::Shutdown, &mut frame).unwrap();
///
/// let mut buf = Vec::new();
/// let mut cursor = std::io::Cursor::new(frame);
/// assert!(wire::read_frame(&mut cursor, &mut buf).unwrap());
/// assert_eq!(wire::decode(&buf), Ok(Message::Shutdown));
/// // A second read at the boundary reports clean EOF.
/// assert!(!wire::read_frame(&mut cursor, &mut buf).unwrap());
/// ```
pub fn read_frame(reader: &mut impl Read, buf: &mut Vec<u8>) -> io::Result<bool> {
    let mut header = [0_u8; 4];
    let mut read = 0;
    while read < header.len() {
        match reader.read(&mut header[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            // `read_exact` retries `Interrupted` internally — the
            // manual length-field loop does the same.
            Err(ref error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    if read == 0 {
        // Clean EOF at a frame boundary.
        return Ok(false);
    }
    if read < header.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "partial frame header",
        ));
    }
    let payload_len = u32::from_le_bytes(header) as usize;
    if payload_len > WIRE_MAX_PAYLOAD as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "declared payload_len exceeds WIRE_MAX_PAYLOAD",
        ));
    }
    buf.clear();
    buf.extend_from_slice(&header);
    buf.resize(WIRE_FRAME_HEADER_LEN + payload_len, 0);
    // Kind byte plus payload — a mid-body EOF reports `UnexpectedEof`,
    // the same kind as a partial header.
    reader.read_exact(&mut buf[4..])?;
    Ok(true)
}

/// Reads and decodes one [`Message`] — the convenience over
/// [`read_frame`] for callers that treat a clean EOF at a frame
/// boundary as an error too (the proxy's converge-on-`Failed`
/// contract; the fixture worker maps any read failure to its
/// clean-teardown exit anyway).
///
/// **RT contract:** non-RT — blocking `Read`, a `buf` resize that may
/// allocate, plus [`decode`]'s payload allocations.
///
/// # Errors
///
/// `std::io::Error` — `UnexpectedEof` on EOF (clean-at-boundary or
/// mid-frame), `InvalidData` for an over-limit declared length or an
/// undecodable frame, or the reader's own error verbatim.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message};
///
/// let mut frame = Vec::new();
/// wire::encode(&Message::BlockIn { seq: 7 }, &mut frame).unwrap();
///
/// let mut buf = Vec::new();
/// let mut cursor = std::io::Cursor::new(frame);
/// assert_eq!(
///     wire::read_message(&mut cursor, &mut buf).unwrap(),
///     Message::BlockIn { seq: 7 },
/// );
/// // EOF at the boundary is `UnexpectedEof` at this level.
/// assert_eq!(
///     wire::read_message(&mut cursor, &mut buf)
///         .unwrap_err()
///         .kind(),
///     std::io::ErrorKind::UnexpectedEof,
/// );
/// ```
pub fn read_message(reader: &mut impl Read, buf: &mut Vec<u8>) -> io::Result<Message> {
    if !read_frame(reader, buf)? {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "clean EOF at a frame boundary",
        ));
    }
    decode(buf).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Encodes `message` into `buf` (replaced wholesale) and writes the
/// frame to `writer`, flushing so a blocked peer reader sees it
/// immediately. The `buf` parameter lets a serving loop reuse one
/// allocation across frames.
///
/// **RT contract:** non-RT — blocking `Write`/`flush` plus
/// [`encode`]'s `buf` growth.
///
/// # Errors
///
/// `std::io::Error` — `InvalidData` wrapping the [`WireError`] when
/// `message` fails to encode, or the writer's own error verbatim.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::wire::{self, Message};
///
/// let mut out = Vec::new();
/// wire::write_message(&mut out, &Message::BlockIn { seq: 7 }, &mut Vec::new()).unwrap();
/// assert_eq!(wire::decode(&out), Ok(Message::BlockIn { seq: 7 }));
/// ```
pub fn write_message(
    writer: &mut impl Write,
    message: &Message,
    buf: &mut Vec<u8>,
) -> io::Result<()> {
    buf.clear();
    encode(message, buf).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    writer.write_all(buf)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::TransportStatus;

    const GAIN: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.gain",
        name: "Test Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &["audio-effect", "utility"],
    };

    fn config() -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
    }

    /// The `shm` layout constants form one chain — every offset is
    /// derived, so both the host-side owner and the runner-side
    /// attach compute the same segment byte-for-byte.
    #[test]
    fn shm_layout_offsets_are_self_consistent() {
        assert_eq!(shm::OFF_MAGIC + 4, shm::OFF_WIRE_VERSION);
        assert_eq!(shm::OFF_WIRE_VERSION + 4, shm::OFF_IN_DEPTH);
        assert_eq!(shm::OFF_IN_DEPTH + 4, shm::OFF_OUT_DEPTH);
        assert_eq!(shm::OFF_OUT_DEPTH + 4, shm::OFF_SLOT_BYTES);
        assert_eq!(shm::OFF_SLOT_BYTES + 4, shm::OFF_IN_OFF);
        assert_eq!(shm::OFF_OUT_OFF + 4, shm::HEADER_LEN);
        assert_eq!(shm::SLOT_OFF_FLAGS + 4, shm::TRANSPORT_OFF);
        assert_eq!(shm::TRANSPORT_OFF + shm::TRANSPORT_LEN, shm::RESULT_OFF);
        assert_eq!(shm::RESULT_OFF + shm::RESULT_LEN, shm::EVENTS_OFF);
        assert_eq!(shm::EVENTS_OFF, 128);
        assert_eq!(
            shm::EVENTS_OFF + shm::EVENT_WIRE_SIZE * WIRE_MAX_EVENTS as usize,
            shm::SAMPLES_OFF
        );
        assert_eq!(shm::SAMPLE_STRIDE, WIRE_MAX_FRAMES as usize * 4);
        assert_eq!(
            shm::SAMPLES_LEN,
            WIRE_MAX_CHANNELS as usize * shm::SAMPLE_STRIDE
        );
        assert_eq!(shm::SAMPLES_OFF + shm::SAMPLES_LEN, shm::SLOT_BYTES);
        assert_eq!(shm::IN_SLOTS_OFF, shm::HEADER_LEN);
        assert_eq!(shm::out_slots_off(3), shm::HEADER_LEN + 3 * shm::SLOT_BYTES);
        assert_eq!(
            shm::segment_len(2, 3),
            shm::HEADER_LEN + 5 * shm::SLOT_BYTES
        );
        // A maximal wire `TransportInfo` (91 B) fits the region; a
        // wire `ProcessResult` (5 B) fits `RESULT_LEN`.
        const {
            assert!(shm::TRANSPORT_LEN >= 91);
        }
        const {
            assert!(shm::RESULT_LEN >= 5);
        }
    }

    fn roundtrip(message: &Message) -> Message {
        let mut frame = Vec::new();
        encode(message, &mut frame).unwrap();
        decode(&frame).unwrap()
    }

    fn field_roundtrip<T>(
        value: &T,
        encode: impl Fn(&T, &mut Vec<u8>),
        decode: impl Fn(&mut Reader<'_>) -> Result<T, WireError>,
    ) -> T
    where
        T: PartialEq + fmt::Debug,
    {
        let mut payload = Vec::new();
        encode(value, &mut payload);
        let mut reader = Reader::new(&payload);
        let decoded = decode(&mut reader).unwrap();
        assert!(reader.is_empty(), "trailing bytes after field decode");
        decoded
    }

    // --- roundtrip: every message kind ---------------------------------

    #[test]
    fn hello_roundtrip_and_version_is_carried_verbatim() {
        assert_eq!(
            roundtrip(&Message::Hello {
                version: WIRE_VERSION,
                capabilities: 0,
            }),
            Message::Hello {
                version: WIRE_VERSION,
                capabilities: 0,
            }
        );
        // The codec carries any u16 — enforcing `== WIRE_VERSION` is
        // the handshake layer's decision, not the codec's.
        assert_eq!(
            roundtrip(&Message::Hello {
                version: WIRE_VERSION + 7,
                capabilities: 0,
            }),
            Message::Hello {
                version: WIRE_VERSION + 7,
                capabilities: 0,
            }
        );
        // `capabilities` is reserved under this revision: nonzero
        // bits are a defined rejection, not a silent carry.
        let mut frame = Vec::new();
        encode(
            &Message::Hello {
                version: WIRE_VERSION,
                capabilities: 0b1011,
            },
            &mut frame,
        )
        .unwrap();
        assert_eq!(decode(&frame), Err(WireError::Malformed));
    }

    #[test]
    fn command_roundtrip_covers_every_verb() {
        let commands = [
            Command::Scan,
            Command::Initialize,
            Command::Activate { config: config() },
            Command::StartProcessing,
            Command::StopProcessing,
            Command::Deactivate,
            Command::SaveState,
            Command::RestoreState {
                bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            },
            Command::OpenGui {
                parent: GuiParentToken::new(0xABCD_EF12_3456_7890),
            },
            Command::CloseGui,
        ];
        for cmd in commands {
            let message = Message::Command { seq: 7, cmd };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn reply_roundtrip_covers_every_payload() {
        let replies = [
            Reply::Ok,
            Reply::Scanned(vec![
                Descriptor::from(GAIN),
                Descriptor::default(), // empty strings/features are legal
            ]),
            Reply::State {
                bytes: vec![1, 2, 3, 4, 5],
            },
            Reply::Opened {
                gui: GuiHandle::new(42),
            },
        ];
        for reply in replies {
            let message = Message::Reply {
                seq: 7,
                result: Ok(reply),
            };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn reply_roundtrip_carries_every_host_error() {
        // Every wire-representable variant — `EndpointUnavailable` is
        // deliberately absent (local-only; see
        // `encode_rejects_endpoint_unavailable_and_restores_out`).
        let errors = [
            HostError::UnknownPluginId,
            HostError::InvalidHandle,
            HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: LifecycleState::Processing,
                event: LifecycleEvent::Deactivate,
            }),
            HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: LifecycleState::Failed,
                event: LifecycleEvent::Failed,
            }),
            HostError::Prepare(PrepareError::UnsupportedConfig),
            HostError::Prepare(PrepareError::ActivationFailed),
            HostError::PluginFailed,
            HostError::Unsupported,
            HostError::Failed,
            HostError::Ipc,
            HostError::DuplicatePluginId,
        ];
        for error in errors {
            let message = Message::Reply {
                seq: 7,
                result: Err(error),
            };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn doorbells_roundtrip() {
        assert_eq!(
            roundtrip(&Message::BlockIn { seq: u64::MAX }),
            Message::BlockIn { seq: u64::MAX }
        );
        let done = Message::BlockDone {
            seq: 1_023,
            result: ProcessResult::new(ProcessStatus::Sleep, 2_048),
        };
        assert_eq!(roundtrip(&done), done);
        assert_eq!(roundtrip(&Message::Shutdown), Message::Shutdown);
    }

    #[test]
    fn frame_layout_is_little_endian_and_length_prefixed() {
        // Hello{version: 1, capabilities: 3} → len 6, kind 0, LE fields.
        let mut frame = Vec::new();
        encode(
            &Message::Hello {
                version: WIRE_VERSION,
                capabilities: 3,
            },
            &mut frame,
        )
        .unwrap();
        assert_eq!(frame, [6, 0, 0, 0, KIND_HELLO, 1, 0, 3, 0, 0, 0]);
        assert_eq!(frame.len(), WIRE_FRAME_HEADER_LEN + 6);

        // Shutdown → empty payload.
        let mut frame = Vec::new();
        encode(&Message::Shutdown, &mut frame).unwrap();
        assert_eq!(frame, [0, 0, 0, 0, KIND_SHUTDOWN]);
    }

    #[test]
    fn encode_appends_one_frame_per_call() {
        let mut buf = Vec::new();
        encode(&Message::Shutdown, &mut buf).unwrap();
        encode(&Message::BlockIn { seq: 1 }, &mut buf).unwrap();
        // First frame is exactly 5 bytes; the rest is the second frame.
        assert_eq!(decode(&buf[..WIRE_FRAME_HEADER_LEN]), Ok(Message::Shutdown));
        assert_eq!(
            decode(&buf[WIRE_FRAME_HEADER_LEN..]),
            Ok(Message::BlockIn { seq: 1 })
        );
    }

    // --- descriptor equivalence ----------------------------------------

    #[test]
    fn descriptor_from_static_and_field_equivalence() {
        let owned = Descriptor::from(GAIN);
        assert!(owned.matches(&GAIN));
        assert_eq!(owned.id, GAIN.id);
        assert_eq!(owned.features, GAIN.features);

        // Every compared field rejects the match on its own.
        let mut different = owned.clone();
        different.id = "org.ruviform.test.other".to_owned();
        assert!(!different.matches(&GAIN));
        let mut different = owned.clone();
        different.name = "Other".to_owned();
        assert!(!different.matches(&GAIN));
        let mut different = owned.clone();
        different.vendor = "Other Vendor".to_owned();
        assert!(!different.matches(&GAIN));
        let mut different = owned.clone();
        different.version = "9.9.9".to_owned();
        assert!(!different.matches(&GAIN));
        let mut different = owned.clone();
        different.features.pop();
        assert!(!different.matches(&GAIN));
        let mut different = owned.clone();
        different.features[0] = "audio-effect".to_owned();
        different.features[1] = "other".to_owned();
        assert!(!different.matches(&GAIN));
    }

    // --- malformed frames ------------------------------------------------

    #[test]
    fn decode_rejects_short_and_truncated_frames() {
        assert_eq!(decode(&[]), Err(WireError::Truncated));
        assert_eq!(decode(&[0, 0, 0]), Err(WireError::Truncated));
        // Header complete but declared payload missing.
        assert_eq!(
            decode(&[6, 0, 0, 0, KIND_HELLO, 1]),
            Err(WireError::Truncated)
        );
        // Mid-payload cut inside a Command (seq truncated).
        assert_eq!(
            decode(&[3, 0, 0, 0, KIND_COMMAND, 1, 2, 3]),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn decode_rejects_length_mismatch() {
        // Declared < actual.
        let mut frame = Vec::new();
        encode(&Message::Shutdown, &mut frame).unwrap();
        frame.push(0);
        assert_eq!(decode(&frame), Err(WireError::Malformed));
        // Declared > actual.
        assert_eq!(
            decode(&[2, 0, 0, 0, KIND_SHUTDOWN]),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn decode_rejects_unknown_kind_and_trailing_payload() {
        assert_eq!(decode(&[0, 0, 0, 0, 99]), Err(WireError::UnknownKind(99)));
        // Shutdown with a stray payload byte.
        assert_eq!(
            decode(&[1, 0, 0, 0, KIND_SHUTDOWN, 0]),
            Err(WireError::Malformed)
        );
        // Hello payload longer than its 6-byte layout.
        assert_eq!(
            decode(&[7, 0, 0, 0, KIND_HELLO, 1, 0, 3, 0, 0, 0, 0]),
            Err(WireError::Malformed)
        );
    }

    #[test]
    fn decode_rejects_declared_lengths_over_wire_max() {
        // payload_len itself over the cap → rejected before anything else.
        let mut frame = (WIRE_MAX_PAYLOAD + 1).to_le_bytes().to_vec();
        frame.extend_from_slice(&1_u32.to_le_bytes());
        assert_eq!(decode(&frame), Err(WireError::LimitExceeded));

        // A nested length prefix over its field cap: descriptor count.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1); // seq
        push_u8(&mut payload, REPLY_OK);
        push_u8(&mut payload, 1); // Scanned
        push_u32(&mut payload, WIRE_MAX_ELEMENTS + 1);
        let mut frame = Vec::new();
        push_u32(&mut frame, len_u32(payload.len()));
        push_u8(&mut frame, KIND_REPLY);
        frame.extend_from_slice(&payload);
        assert_eq!(decode(&frame), Err(WireError::LimitExceeded));

        // String length over the cap inside a descriptor field.
        let mut descriptor_bytes = Vec::new();
        push_u32(&mut descriptor_bytes, WIRE_MAX_STRING_BYTES + 1);
        let mut descriptor_reader = Reader::new(&descriptor_bytes);
        assert_eq!(
            descriptor_reader.read_string(),
            Err(WireError::LimitExceeded)
        );
        // State blob over the cap.
        let mut command_bytes = Vec::new();
        push_u8(&mut command_bytes, 7); // RestoreState
        push_u32(&mut command_bytes, WIRE_MAX_STATE_BYTES + 1);
        let mut command_reader = Reader::new(&command_bytes);
        assert_eq!(
            decode_command(&mut command_reader),
            Err(WireError::LimitExceeded)
        );
    }

    #[test]
    fn decode_rejects_impossible_counts_without_allocating() {
        // A count within the cap but unsatisfiable by the remaining
        // bytes is truncated before any elements are read.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u8(&mut payload, REPLY_OK);
        push_u8(&mut payload, 1); // Scanned
        push_u32(&mut payload, 10); // count 10, zero bytes follow
        let mut frame = Vec::new();
        push_u32(&mut frame, len_u32(payload.len()));
        push_u8(&mut frame, KIND_REPLY);
        frame.extend_from_slice(&payload);
        assert_eq!(decode(&frame), Err(WireError::Truncated));
    }

    #[test]
    fn decode_rejects_bad_tags_and_invariant_violations() {
        // Unknown Command verb tag.
        assert_eq!(
            decode(&[9, 0, 0, 0, KIND_COMMAND, 1, 0, 0, 0, 0, 0, 0, 0, 200]),
            Err(WireError::Malformed)
        );
        // Unknown Reply result tag (not REPLY_OK/REPLY_ERR).
        assert_eq!(
            decode(&[9, 0, 0, 0, KIND_REPLY, 1, 0, 0, 0, 0, 0, 0, 0, 2]),
            Err(WireError::Malformed)
        );
        // Unknown HostError tag.
        let mut payload = Vec::new();
        push_u8(&mut payload, 99);
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_host_error(&mut reader), Err(WireError::Malformed));
        // Unknown ProcessStatus tag.
        let mut reader = Reader::new(&[9, 0, 0, 0, 0]);
        assert_eq!(
            decode_process_result(&mut reader),
            Err(WireError::Malformed)
        );
        // Zero sample rate and min > max are ctor violations.
        let mut payload = Vec::new();
        push_u32(&mut payload, 0); // rate 0 → invalid
        push_u32(&mut payload, 1);
        push_u32(&mut payload, 512);
        push_u64(&mut payload, 0);
        push_u64(&mut payload, 0);
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_process_config(&mut reader),
            Err(WireError::Malformed)
        );
        let mut payload = Vec::new();
        push_u32(&mut payload, 48_000);
        push_u32(&mut payload, 512);
        push_u32(&mut payload, 1); // min > max
        push_u64(&mut payload, 0);
        push_u64(&mut payload, 0);
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_process_config(&mut reader),
            Err(WireError::Malformed)
        );
    }

    #[test]
    fn decode_rejects_nonzero_reserved_and_bad_utf8() {
        // Non-zero reserved word inside ProcessConfig.
        let mut payload = Vec::new();
        push_u32(&mut payload, 48_000);
        push_u32(&mut payload, 1);
        push_u32(&mut payload, 512);
        push_u64(&mut payload, 1); // reserved must be zero
        push_u64(&mut payload, 0);
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_process_config(&mut reader),
            Err(WireError::Malformed)
        );

        // Non-zero reserved word inside TransportInfo — flip a byte
        // in the first of the two trailing reserved `u64`s of a
        // well-formed payload.
        let transport = TransportInfo::new(TransportStatus::Stopped, MediaTime::ZERO);
        let mut payload = Vec::new();
        encode_transport(&transport, &mut payload).unwrap();
        let reserved_start = payload.len() - 16;
        payload[reserved_start] = 1;
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_transport(&mut reader), Err(WireError::Malformed));

        // Non-zero reserved word inside an Event — same tail layout;
        // the last byte sits in the second reserved `u64`.
        let event = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(1),
                value: ParamValue::new(0.5).unwrap(),
            }),
        );
        let mut payload = Vec::new();
        encode_event(&event, &mut payload);
        let last = payload.len() - 1;
        payload[last] = 1;
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_event(&mut reader), Err(WireError::Malformed));

        // Invalid UTF-8 in a descriptor string field.
        let mut descriptor = Vec::new();
        push_u32(&mut descriptor, 1);
        descriptor.push(0xFF); // invalid UTF-8
        let mut reader = Reader::new(&descriptor);
        assert_eq!(reader.read_string(), Err(WireError::Malformed));
    }

    // --- fixed wire layouts ---------------------------------------------

    #[test]
    fn process_config_and_result_roundtrip() {
        let config = config();
        assert_eq!(
            field_roundtrip(&config, encode_process_config, decode_process_config),
            config
        );
        for status in [
            ProcessStatus::Continue,
            ProcessStatus::Sleep,
            ProcessStatus::Error,
        ] {
            let result = ProcessResult::new(status, 1_024);
            assert_eq!(
                field_roundtrip(&result, encode_process_result, decode_process_result),
                result
            );
        }
    }

    #[test]
    fn boundary_tokens_roundtrip() {
        let handle = PluginHandle::new(3, 0xBEEF);
        assert_eq!(
            field_roundtrip(&handle, encode_plugin_handle, decode_plugin_handle),
            handle
        );
        let gui = GuiHandle::new(u32::MAX);
        assert_eq!(
            field_roundtrip(&gui, encode_gui_handle, decode_gui_handle),
            gui
        );
        let token = GuiParentToken::new(u64::MAX);
        assert_eq!(
            field_roundtrip(&token, encode_gui_parent_token, decode_gui_parent_token),
            token
        );
    }

    #[test]
    fn host_error_field_roundtrip() {
        // Every wire-representable variant — `EndpointUnavailable` is
        // deliberately absent: it is a local-only host fact with no
        // wire representation (covered by
        // `encode_rejects_endpoint_unavailable_and_restores_out`).
        for error in [
            HostError::UnknownPluginId,
            HostError::InvalidHandle,
            HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: LifecycleState::Activated,
                event: LifecycleEvent::Destroy,
            }),
            HostError::Prepare(PrepareError::ActivationFailed),
            HostError::PluginFailed,
            HostError::Unsupported,
            HostError::Failed,
            HostError::Ipc,
            HostError::DuplicatePluginId,
        ] {
            // `encode_host_error` is fallible like `encode_transport`
            // — exercised directly rather than through the infallible
            // `field_roundtrip` helper.
            let mut payload = Vec::new();
            encode_host_error(&error, &mut payload).unwrap();
            let mut reader = Reader::new(&payload);
            assert_eq!(decode_host_error(&mut reader), Ok(error));
            assert!(reader.is_empty(), "trailing bytes after field decode");
        }
    }

    #[test]
    fn encode_rejects_endpoint_unavailable_and_restores_out() {
        // `EndpointUnavailable` is a local-only host fact — no wire
        // representation, never collapsed into `Ipc`/`Unsupported`.
        let mut payload = Vec::new();
        assert_eq!(
            encode_host_error(&HostError::EndpointUnavailable, &mut payload),
            Err(WireError::UnsupportedVariant)
        );
        assert!(payload.is_empty(), "nothing is appended on failure");

        // `encode` is an all-or-nothing append: a failed frame leaves
        // `out` exactly as found — no partial `REPLY_ERR` frame.
        let mut out = vec![0xAA, 0xBB, 0xCC];
        let before = out.clone();
        let message = Message::Reply {
            seq: 7,
            result: Err(HostError::EndpointUnavailable),
        };
        assert_eq!(
            encode(&message, &mut out),
            Err(WireError::UnsupportedVariant)
        );
        assert_eq!(out, before);

        // The rejection disturbed nothing: a following `Ok` reply
        // still appends a complete frame that decodes cleanly, with
        // the pre-existing bytes untouched.
        let ok = Message::Reply {
            seq: 8,
            result: Ok(Reply::Ok),
        };
        encode(&ok, &mut out).unwrap();
        assert_eq!(out[..before.len()], before[..]);
        assert_eq!(decode(&out[before.len()..]), Ok(ok));
    }

    #[test]
    fn media_time_roundtrip_and_bad_base_rejected() {
        let time = MediaTime::new(96_001, TimeBase::new(1_001, 30_000).unwrap());
        assert_eq!(
            field_roundtrip(&time, encode_media_time, decode_media_time),
            time
        );
        // A zero denominator cannot form a TimeBase.
        let mut payload = Vec::new();
        push_i64(&mut payload, 0);
        push_u32(&mut payload, 1);
        push_u32(&mut payload, 0);
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_media_time(&mut reader), Err(WireError::Malformed));
    }

    #[test]
    fn transport_roundtrip_minimal_and_full() {
        let minimal = TransportInfo::new(TransportStatus::Stopped, MediaTime::ZERO);
        let mut payload = Vec::new();
        encode_transport(&minimal, &mut payload).unwrap();
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_transport(&mut reader).unwrap(), minimal);

        let base = TimeBase::sample_rate(48_000).unwrap();
        let musical = MusicalTimeInfo {
            position_ticks: 3_840,
            bar_start_tick: 1_920,
            bpm_milli: 120_500,
            ppqn: ruviform_media_core::PPQN.ticks_per_quarter(),
            sig_num: 6,
            sig_den: 8,
        };
        let full = TransportInfo::new(TransportStatus::Playing, MediaTime::new(48_000, base))
            .with_loop_range(MediaRange::new(
                MediaTime::new(0, base),
                MediaDuration::new(96_000, base),
            ))
            .with_musical(musical);
        let mut payload = Vec::new();
        encode_transport(&full, &mut payload).unwrap();
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_transport(&mut reader).unwrap(), full);
        assert!(reader.is_empty());
    }

    #[test]
    fn event_and_event_list_roundtrip() {
        let event = Event::new(
            64,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(7),
                value: ParamValue::new(0.75).unwrap(),
            }),
        );
        assert_eq!(field_roundtrip(&event, encode_event, decode_event), event);

        let events = [
            Event::new(
                0,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(0),
                    value: ParamValue::new(0.25).unwrap(),
                }),
            ),
            event,
        ];
        let list = EventList::new(&events, 128).unwrap();
        let mut payload = Vec::new();
        encode_event_list(list, &mut payload);
        let mut reader = Reader::new(&payload);
        let (block_frames, decoded) = decode_event_list(&mut reader).unwrap();
        assert_eq!(block_frames, 128);
        assert_eq!(decoded, events);
        // The pieces validate back into an EventList unchanged.
        assert_eq!(EventList::new(&decoded, block_frames).unwrap().len(), 2);

        // Empty list round-trips.
        let mut payload = Vec::new();
        encode_event_list(EventList::empty(512), &mut payload);
        let mut reader = Reader::new(&payload);
        let (block_frames, decoded) = decode_event_list(&mut reader).unwrap();
        assert_eq!(block_frames, 512);
        assert_eq!(decoded.len(), 0);
    }

    #[test]
    fn event_rejects_nan_and_unknown_kind() {
        // NaN param value fails the ParamValue ctor policy.
        let mut payload = Vec::new();
        push_u32(&mut payload, 0);
        push_u8(&mut payload, 0);
        push_u32(&mut payload, 1);
        push_f64(&mut payload, f64::NAN);
        push_u64(&mut payload, 0);
        push_u64(&mut payload, 0);
        let mut reader = Reader::new(&payload);
        assert_eq!(decode_event(&mut reader), Err(WireError::Malformed));
        // Unknown event kind tag.
        let mut reader = Reader::new(&[0, 0, 0, 0, 9]);
        assert_eq!(decode_event(&mut reader), Err(WireError::Malformed));
    }

    #[test]
    fn descriptor_field_codec_roundtrip_and_bounds() {
        let descriptor = Descriptor::from(GAIN);
        assert_eq!(
            field_roundtrip(&descriptor, encode_descriptor, decode_descriptor),
            descriptor
        );
        // A string at exactly the cap still decodes.
        let long = "x".repeat(WIRE_MAX_STRING_BYTES as usize);
        let descriptor = Descriptor {
            id: long,
            ..Descriptor::default()
        };
        assert_eq!(
            field_roundtrip(&descriptor, encode_descriptor, decode_descriptor),
            descriptor
        );
    }

    #[test]
    fn wire_error_display_is_observable() {
        for error in [
            WireError::Truncated,
            WireError::Malformed,
            WireError::UnknownKind(99),
            WireError::LimitExceeded,
            WireError::UnsupportedVariant,
        ] {
            assert_ne!(error.to_string(), "");
        }
    }
}
