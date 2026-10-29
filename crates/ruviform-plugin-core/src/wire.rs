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
//! | 1 | [`Message::Command`] | parent → child | `u64 seq`, `u32 instance`, [`Command`] |
//! | 2 | [`Message::Reply`] | child → parent | `u64 seq`, `u32 instance`, `u8` result tag + [`Reply`] or [`HostError`] |
//! | 3 | [`Message::BlockIn`] | parent → child | `u64 seq`, `u32 instance` — doorbell for shm in-slot `seq % D` |
//! | 4 | [`Message::BlockDone`] | child → parent | `u64 seq`, `u32 instance`, [`ProcessResult`] — doorbell for shm out-slot `seq % D` |
//! | 5 | [`Message::Shutdown`] | parent → child | empty — child teardown |
//!
//! The `u32 instance` field is the wire-v2 envelope (Issue #117): one
//! child hosts up to [`WIRE_MAX_INSTANCES`] plugin instances in a
//! runner-side registry, and the field is the registry key the frame
//! addresses. [`Message::Hello`] and [`Message::Shutdown`] carry none
//! — they are child-scoped by kind. `Reply`/`BlockDone` echo the
//! received `instance` verbatim. The codec enforces the verb↔scope
//! table on [`decode`]: only [`Command::Scan`] is child-scoped
//! (envelope [`WIRE_INSTANCE_NONE`]); every other verb and both
//! doorbells require a live range instance
//! (`< WIRE_MAX_INSTANCES`). A scope violation or an out-of-range
//! `instance` is [`WireError::Malformed`].
//!
//! ## Versioning
//!
//! `Hello` alone carries `version` ([`WIRE_VERSION`]); no other
//! message repeats it. A version mismatch is a handshake failure
//! decided by the proxy (kill + `HostError::Ipc`), not by the codec —
//! [`decode`] accepts any `u16` version. Payload field-set changes
//! therefore always ride a `WIRE_VERSION` bump. The shm header's
//! `wire_version` field is validated against [`WIRE_VERSION`] with
//! the same strict equality on attach.
//!
//! The supported protocol is always the single latest version (the
//! Issue #119 policy): no decode, translation, fallback or
//! coexistence for older revisions is implemented, and there is no
//! version- or limit-negotiation mechanism. `WIRE_VERSION` is a plain
//! integer generation of the communication contract — bumped on
//! incompatible protocol changes only, not tied to `SemVer` or crate
//! versions — and a bump means host and runner ship the same
//! revision together. `Hello.capabilities` stays reserved and must
//! be `0`; enabling a capability is itself designed as a protocol
//! change. The full policy is recorded in
//! `docs/designs/plugin-l1-out-of-process.md`.
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
//! `HostCommand::Instantiate` maps to spawn + handshake +
//! [`Command::CreateInstance`] and `HostCommand::Destroy` to
//! [`Command::DestroyInstance`] + `Shutdown` + reap (on the child's
//! last instance). The remaining host commands map one-to-one onto
//! the instance-scoped [`Command`] variants, with the public handle
//! replaced by the envelope `instance` — a child-local `u32` the host
//! mints.

use core::fmt;
use std::io::{self, Read, Write};

use ruviform_audio_core::SampleRate;
use ruviform_media_core::{
    FrameRate, MediaDuration, MediaRange, MediaTime, MusicalTimeInfo, TimeBase, TransportStatus,
};
use ruviform_video_core::{
    Chromaticity, ColorInfo, ColorPrimaries, ColorRange, ContentLightLevel, HdrMetadata,
    MasteringDisplayInfo, MatrixCoefficients, PixelFormat, TransferCharacteristics,
};

use crate::{
    Event, EventKind, EventList, GuiHandle, GuiParentToken, HostError, LifecycleError,
    LifecycleEvent, LifecycleState, ParamEvent, ParamFlags, ParamId, ParamValue,
    PlainParamInfoOwned, PlainParamValue, PluginDescriptor, PluginHandle, PrepareError,
    ProcessConfig, ProcessResult, ProcessStatus, TransportInfo, VideoProcessConfig,
};

/// The protocol version carried by [`Message::Hello`]. A proxy and a
/// runner must agree on it exactly — a mismatch fails the handshake
/// (`HostError::Ipc`); payload changes always ride a version bump.
/// `2` was the Issue #117 instance-envelope revision; `3` is the
/// Issue #118 video revision — the video verbs ([`Command`] tags
/// 12-22), video replies ([`Reply`] tags 4-7), the video type codecs
/// and the [`vshm`] segment layout are additive on top of the v2
/// tags, which are unchanged.
pub const WIRE_VERSION: u16 = 3;

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

/// Maximum frame width/height dimension on the wire (pixels) — the
/// video bound (wire v3, Issue #118) that keeps hostile dims out of
/// segment-size math, where they act as multipliers. Covers 8K UHD
/// (7680×4320).
pub const WIRE_VIDEO_MAX_DIM: u32 = 8_192;

/// Maximum plane count a [`VideoFrameDesc`] may declare — the
/// current `PixelFormat` maximum is 3; one more for headroom.
pub const WIRE_VIDEO_MAX_PLANES: u32 = 4;

/// Maximum per-plane stride a [`VideoFrameDesc`] may declare
/// (256 KiB — twice the 8K `RgbaF32` row of 131,072 B, so canonical
/// packing, where stride equals row bytes, always fits).
pub const WIRE_VIDEO_MAX_STRIDE: u32 = 262_144;

/// Maximum total plane bytes of one frame — the `vshm` in/out
/// region capacity bound (256 MiB). 4K `RgbaF32` (≈ 126.6 MiB) and
/// 8K `RgbaF16` (≈ 253.1 MiB) fit; 8K `RgbaF32` (≈ 506 MiB) honestly
/// exceeds it and is rejected.
pub const WIRE_VIDEO_MAX_FRAME_BYTES: u64 = 268_435_456;

/// Absolute upper bound of [`vshm::segment_len`] — the largest video
/// segment a host may create or a child may attach
/// (`vshm::HEADER_LEN` + in + out regions).
pub const WIRE_VIDEO_MAX_SEGMENT_BYTES: u64 =
    vshm::HEADER_LEN as u64 + 2 * WIRE_VIDEO_MAX_FRAME_BYTES;

/// Maximum live plugin-instance handles per child process (wire v2).
/// The `u32 instance` envelope field is the runner's registry key;
/// an instance-scoped frame is [`WireError::Malformed`] when it is
/// neither `< WIRE_MAX_INSTANCES` nor [`WIRE_INSTANCE_NONE`].
pub const WIRE_MAX_INSTANCES: u32 = 64;

/// The `u32 instance` value addressing the child as a whole —
/// only [`Command::Scan`] uses it. Every other verb and both
/// doorbells require a live-range instance `< WIRE_MAX_INSTANCES`.
pub const WIRE_INSTANCE_NONE: u32 = u32::MAX;

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
/// ([`shm::HEADER_LEN`] bytes); each slot is [`shm::SLOT_BYTES`]:
///
/// | offset | field | size | notes |
/// | --- | --- | --- | --- |
/// | +0 | `seq` | `u64` | echoes the doorbell `seq` |
/// | +8 | `frames` | `u32` | ≤ [`WIRE_MAX_FRAMES`] |
/// | +12 | `channels` | `u32` | ≤ [`WIRE_MAX_CHANNELS`] |
/// | +16 | `event_count` | `u32` | ≤ [`WIRE_MAX_EVENTS`] |
/// | +20 | `flags` | `u32` | bit0 ([`shm::FLAG_TRANSPORT`]) |
/// | +24 | `transport` | [`shm::TRANSPORT_LEN`] | wire `TransportInfo` when flagged |
/// | +120 | `result` | [`shm::RESULT_LEN`] | wire `ProcessResult` (out-slots) |
/// | +128 | `events` | `event_count * `[`shm::EVENT_WIRE_SIZE`] | wire `Event` records |
/// | +33920 | `samples` | [`shm::SAMPLES_LEN`] | planar `f32` LE, channel stride [`shm::SAMPLE_STRIDE`] |
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
    /// wire size of `TransportInfo` under `WIRE_VERSION` 3 is 91 B.
    pub const TRANSPORT_LEN: usize = 96;
    /// Byte offset of the slot `result` region.
    pub const RESULT_OFF: usize = TRANSPORT_OFF + TRANSPORT_LEN;
    /// Reserved capacity of the `result` region — a wire
    /// `ProcessResult` is 5 B.
    pub const RESULT_LEN: usize = 8;
    /// Byte offset of the slot `events` region.
    pub const EVENTS_OFF: usize = RESULT_OFF + RESULT_LEN;
    /// Byte length of one wire-encoded `Event` record under
    /// `WIRE_VERSION` 3 (`u32` offset + `u8` kind + `u32` param id +
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
    /// Fixed byte length of one in/out slot under `WIRE_VERSION` 3.
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

/// The L1 video shared-memory data-plane layout (wire v3, Issue
/// #118) — the **authoritative wire-side definition** of the
/// `[header][in region][out region]` per-instance segment both ends
/// consume: `ruviform-plugin-host::l1::vshm` (the creating owner) and
/// `ruviform-plugin-export::vshm` (the attaching runner). The audio
/// [`shm`] layout is untouched — video uses no slots or doorbells:
/// one in-place render per request→reply window under a dedicated
/// `Command`/`Reply` exchange.
///
/// All fields are little-endian `u32`. The header is 7 fields
/// ([`vshm::HEADER_LEN`] bytes):
///
/// | offset | field | notes |
/// | --- | --- | --- |
/// | +0 | `magic` | [`vshm::MAGIC`] (`"RVLV"`) |
/// | +4 | `wire_version` | [`WIRE_VERSION`] |
/// | +8 | `in_off` | `HEADER_LEN` (28) |
/// | +12 | `in_bytes` | `frame_bytes(config)` |
/// | +16 | `out_off` | `28 + in_bytes` |
/// | +20 | `out_bytes` | `frame_bytes(config)` |
/// | +24 | `reserved` | `0` |
///
/// Both regions hold one frame in **canonical packing**: plane `p`
/// starts at `offset(p) = Σ len(0..p)` with `stride = row_bytes`
/// (tightly packed — a caller's padded frame is repacked on copy-in)
/// and `len = stride × plane_height`. [`vshm::plane_layout`] is the
/// shared computation; plane grid dimensions follow
/// [`PixelFormat::chroma_subsampling`] with ceil division, exactly as
/// `VideoFrame` derives them.
pub mod vshm {
    use ruviform_video_core::PixelFormat;

    use super::{VideoProcessConfig, WIRE_VERSION};

    /// Segment magic — `"RVLV"` (Ruviform L1 Video) as four LE
    /// bytes, validated on attach.
    pub const MAGIC: u32 = u32::from_le_bytes(*b"RVLV");

    /// Byte length of the fixed segment header (7 LE `u32` fields:
    /// `magic`, `wire_version`, `in_off`, `in_bytes`, `out_off`,
    /// `out_bytes`, `reserved`).
    pub const HEADER_LEN: usize = 28;

    /// Header field offsets — every field is one LE `u32`.
    pub const OFF_MAGIC: usize = 0;
    /// Header `wire_version` field offset.
    pub const OFF_WIRE_VERSION: usize = 4;
    /// Header `in_off` field offset.
    pub const OFF_IN_OFF: usize = 8;
    /// Header `in_bytes` field offset.
    pub const OFF_IN_BYTES: usize = 12;
    /// Header `out_off` field offset.
    pub const OFF_OUT_OFF: usize = 16;
    /// Header `out_bytes` field offset.
    pub const OFF_OUT_BYTES: usize = 20;
    /// Header `reserved` field offset — must be `0`.
    pub const OFF_RESERVED: usize = 24;

    /// The canonical placement of one plane inside an in/out region.
    ///
    /// `stride` is always `row_bytes` under canonical packing (v1
    /// carries no padding); `offset` is the sum of the preceding
    /// planes' `len`s, so `offset + len` gives the plane's end.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct PlaneLayout {
        /// Byte offset of the plane within its region
        /// (`offset(p) = Σ len(0..p)`).
        pub offset: u64,
        /// Byte length the plane occupies (`stride × plane_height`).
        pub len: u64,
        /// Row pitch in bytes — the canonical `row_bytes`.
        pub stride: u64,
        /// Samples per row on this plane's grid (chroma-subsampled
        /// for non-luma planes).
        pub width: u32,
        /// Rows on this plane's grid (chroma-subsampled for non-luma
        /// planes).
        pub height: u32,
    }

    /// The canonical [`PlaneLayout`] of plane `plane` for `format` at
    /// `width`×`height` — `None` when `plane` is out of range for
    /// `format`. All byte math is `u64` and saturating: hostile dims
    /// can only inflate the value, never wrap it below a bound.
    ///
    /// **RT contract:** non-RT only — loop over up to `plane`
    /// predecessors for the offset sum.
    #[must_use]
    pub fn plane_layout(
        format: PixelFormat,
        width: u32,
        height: u32,
        plane: usize,
    ) -> Option<PlaneLayout> {
        let (plane_width, plane_height) = plane_dims(format, width, height, plane)?;
        let stride = u64::from(plane_width).saturating_mul(format.bytes_per_sample(plane) as u64);
        let len = stride.saturating_mul(u64::from(plane_height));
        let mut offset = 0_u64;
        for previous in 0..plane {
            offset = offset.saturating_add(plane_len(format, width, height, previous).unwrap_or(0));
        }
        Some(PlaneLayout {
            offset,
            len,
            stride,
            width: plane_width,
            height: plane_height,
        })
    }

    /// The total plane bytes of one frame under canonical packing —
    /// the size each of the `in` and `out` regions needs.
    ///
    /// Saturating `u64` math: a pathological config (e.g.
    /// `u32::MAX` dims, which [`VideoProcessConfig::new`] admits)
    /// yields a value above [`super::WIRE_VIDEO_MAX_FRAME_BYTES`]
    /// instead of wrapping, so the callers' bound check still
    /// rejects it — never a panic, never an underestimate.
    ///
    /// **RT contract:** non-RT only — bounded loop over
    /// `format.plane_count()` planes.
    #[must_use]
    pub fn frame_bytes(config: &VideoProcessConfig) -> u64 {
        let format = config.format();
        let mut total = 0_u64;
        for plane in 0..format.plane_count() {
            total = total.saturating_add(
                plane_len(format, config.width(), config.height(), plane).unwrap_or(0),
            );
        }
        total
    }

    /// The declared segment length for `frame_bytes`-sized in and
    /// out regions: `HEADER_LEN + 2 × frame_bytes` (saturating —
    /// callers compare against
    /// [`super::WIRE_VIDEO_MAX_SEGMENT_BYTES`]).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn segment_len(frame_bytes: u64) -> u64 {
        (HEADER_LEN as u64).saturating_add(frame_bytes.saturating_mul(2))
    }

    /// The canonical header bytes for an `in`/`out` region pair —
    /// `magic`, this revision's `wire_version`, the fixed region
    /// offsets and a zero `reserved`.
    ///
    /// **RT contract:** non-RT only — fixed-size array write.
    ///
    /// # Panics
    ///
    /// Panics when `in_bytes` is so large that
    /// `HEADER_LEN + in_bytes` does not fit the `u32` `out_off`
    /// field — a caller-side bound violation (region sizes are
    /// bounded by [`super::WIRE_VIDEO_MAX_FRAME_BYTES`] far below
    /// `u32::MAX`, so reaching it is a bug, not untrusted input).
    #[must_use]
    pub fn header_bytes(in_bytes: u32, out_bytes: u32) -> [u8; HEADER_LEN] {
        let mut header = [0_u8; HEADER_LEN];
        put_u32(&mut header, OFF_MAGIC, MAGIC);
        put_u32(&mut header, OFF_WIRE_VERSION, u32::from(WIRE_VERSION));
        put_u32(
            &mut header,
            OFF_IN_OFF,
            u32::try_from(HEADER_LEN).expect("header length fits u32"),
        );
        put_u32(&mut header, OFF_IN_BYTES, in_bytes);
        put_u32(
            &mut header,
            OFF_OUT_OFF,
            u32::try_from(HEADER_LEN as u64 + u64::from(in_bytes))
                .expect("out region offset fits u32"),
        );
        put_u32(&mut header, OFF_OUT_BYTES, out_bytes);
        // OFF_RESERVED is already zeroed.
        header
    }

    /// Validates a mapped segment's fixed header against this
    /// revision: `magic`, `wire_version == `[`WIRE_VERSION`],
    /// `in_off == HEADER_LEN`, `out_off == HEADER_LEN + in_bytes`,
    /// `reserved == 0` and `bytes.len() >= out_off + out_bytes`
    /// (`>=`, not `==` — on Windows the mapped length may round up
    /// to the allocation granularity, so the invariant is "the
    /// mapping covers the declared layout").
    ///
    /// Returns the declared `(in_bytes, out_bytes)` on success —
    /// this is only the structural half of attach validation;
    /// callers still compare them against `frame_bytes(config)` and
    /// [`super::WIRE_VIDEO_MAX_FRAME_BYTES`].
    ///
    /// **RT contract:** non-RT only — fixed-offset reads.
    #[must_use]
    pub fn validate_header(bytes: &[u8]) -> Option<(u32, u32)> {
        if bytes.len() < HEADER_LEN {
            return None;
        }
        if get_u32(bytes, OFF_MAGIC) != MAGIC
            || get_u32(bytes, OFF_WIRE_VERSION) != u32::from(WIRE_VERSION)
            || get_u32(bytes, OFF_RESERVED) != 0
        {
            return None;
        }
        let in_off = get_u32(bytes, OFF_IN_OFF) as usize;
        if in_off != HEADER_LEN {
            return None;
        }
        let in_bytes = get_u32(bytes, OFF_IN_BYTES);
        let out_off = get_u32(bytes, OFF_OUT_OFF) as usize;
        if out_off != HEADER_LEN + in_bytes as usize {
            return None;
        }
        let out_bytes = get_u32(bytes, OFF_OUT_BYTES);
        match out_off.checked_add(out_bytes as usize) {
            Some(end) if bytes.len() >= end => Some((in_bytes, out_bytes)),
            _ => None,
        }
    }

    /// Grid dimensions `(width, height)` of `plane` of `format` at
    /// `width`×`height` — plane 0 is the full-resolution grid, the
    /// rest are chroma-subsampled with ceil division (the same rule
    /// `VideoFrame` derives its plane grids by).
    fn plane_dims(
        format: PixelFormat,
        width: u32,
        height: u32,
        plane: usize,
    ) -> Option<(u32, u32)> {
        if plane == 0 {
            return Some((width, height));
        }
        if plane >= format.plane_count() {
            return None;
        }
        let (sub_w, sub_h) = format.chroma_subsampling()?;
        Some((width.div_ceil(sub_w), height.div_ceil(sub_h)))
    }

    /// Canonical byte length of `plane` (`stride = row_bytes`, so
    /// `len = row_bytes × plane_height`).
    fn plane_len(format: PixelFormat, width: u32, height: u32, plane: usize) -> Option<u64> {
        let (plane_width, plane_height) = plane_dims(format, width, height, plane)?;
        let row_bytes =
            u64::from(plane_width).saturating_mul(format.bytes_per_sample(plane) as u64);
        Some(row_bytes.saturating_mul(u64::from(plane_height)))
    }

    /// Reads one LE `u32` header field — callers ensure `bytes`
    /// covers `HEADER_LEN`, so the slice is never short.
    fn get_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("header field is 4 bytes"),
        )
    }

    /// Writes one LE `u32` header field.
    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// One frame on the wire — the six message kinds of the L1 protocol.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy; the wire itself is strictly versioned by [`WIRE_VERSION`],
/// so new messages arrive only with a protocol revision. It is
/// `PartialEq` but not `Eq`: wire-v3 [`Reply`] payloads carry
/// [`PlainParamValue`], which contains an `f64`.
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
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// Child → parent: the first frame after spawn. `version` must
    /// equal [`WIRE_VERSION`] or the handshake fails.
    Hello {
        /// Protocol version — [`WIRE_VERSION`] on a live handshake.
        version: u16,
        /// Capability bitmask reserved for protocol extensions;
        /// must be `0` under [`WIRE_VERSION`] `3` — decode rejects a
        /// nonzero value as [`WireError::Malformed`], like any other
        /// nonzero reserved field.
        capabilities: u32,
    },
    /// Parent → child: one serialized command, correlated by `seq`.
    Command {
        /// Correlation id echoed by the matching [`Message::Reply`].
        seq: u64,
        /// The registry slot the command addresses —
        /// [`WIRE_INSTANCE_NONE`] for [`Command::Scan`], a live-range
        /// `< WIRE_MAX_INSTANCES` handle for every other verb
        /// (decode-enforced).
        instance: u32,
        /// The command verb and its payload.
        cmd: Command,
    },
    /// Child → parent: the result of a [`Message::Command`] — either a
    /// reply payload or a shared [`HostError`].
    Reply {
        /// Correlation id of the command being answered.
        seq: u64,
        /// Echo of the request's `instance` — the host correlates it
        /// against the waiter's expected handle.
        instance: u32,
        /// `Ok` carries a [`Reply`]; `Err` carries a [`HostError`].
        result: Result<Reply, HostError>,
    },
    /// Parent → child: doorbell for shm input slot `seq % D`.
    BlockIn {
        /// Block sequence number; selects the shm slot.
        seq: u64,
        /// Live-range instance the block belongs to (never
        /// [`WIRE_INSTANCE_NONE`]; decode-enforced).
        instance: u32,
    },
    /// Child → parent: doorbell for shm output slot `seq % D`.
    BlockDone {
        /// Block sequence number; selects the shm slot.
        seq: u64,
        /// Echo of the `BlockIn` `instance`.
        instance: u32,
        /// The plugin's processing status for this block.
        result: ProcessResult,
    },
    /// Parent → child: clean teardown. The runner normalizes every
    /// live instance and exits; after `DestroyInstance` has drained
    /// the registry this is the only frame left to send.
    Shutdown,
}

/// The wire mirror of the host's handle-qualified commands.
///
/// One variant per dispatchable verb. `Instantiate` rides
/// [`Command::CreateInstance`] (after spawn + handshake + `Scan`) and
/// `Destroy` rides [`Command::DestroyInstance`] followed by the
/// instance-less `Shutdown`. The public [`PluginHandle`] does not
/// appear in payloads — the envelope `instance` field on
/// [`Message::Command`] carries the child-local handle instead.
///
/// Tags 12-22 are the wire-v3 (Issue #118) video verbs — the
/// `VideoPluginInstance` lifecycle and its plain-parameter channel.
/// They are instance-scoped like every non-`Scan` verb; the frame
/// bytes they describe travel a per-instance [`vshm`] segment, never
/// the pipe.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new verbs arrive only with a [`WIRE_VERSION`] bump. It is
/// `PartialEq` but not `Eq`: [`PlainParamValue`] contains an `f64`.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Enumerate the runner's built-in descriptors →
    /// [`Reply::Scanned`]. Child-scoped: the envelope `instance`
    /// must be [`WIRE_INSTANCE_NONE`].
    Scan,
    /// Register a new plugin instance in the child under the
    /// envelope `instance` handle → [`Reply::Ok`]. Carries the v1
    /// bootstrap arguments (plugin selection + shm attachment) that
    /// wire v1 passed as `--ruviform-*` argv flags. Fails with
    /// [`HostError::UnknownPluginId`] (unscanned id),
    /// [`HostError::InstanceLimit`] (registry full),
    /// [`HostError::Ipc`] (shm attach failure or a duplicate
    /// `instance` handle) or [`HostError::PluginFailed`] (factory
    /// panic) — the child stays alive on every defined failure.
    CreateInstance {
        /// Registered plugin id (matched against `factory.descriptor().id`).
        plugin_id: String,
        /// OS shared-memory key the host created for this instance's
        /// data plane — the child attaches it.
        shm_key: String,
        /// Input-slot ring depth the host created the segment with.
        in_depth: u32,
        /// Output-slot ring depth the host created the segment with.
        out_depth: u32,
    },
    /// Normalize (best-effort lifecycle teardown), detach the shm
    /// segment and remove the envelope `instance` handle from the
    /// registry → [`Reply::Ok`]. An unregistered handle is
    /// [`HostError::InvalidHandle`].
    DestroyInstance,
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
    /// Register a new video plugin instance in the child under the
    /// envelope `instance` handle → [`Reply::Ok`] — the
    /// `VideoPluginInstance` counterpart of
    /// [`Command::CreateInstance`] (wire tag 12). Carries a
    /// [`VideoProcessConfig`] in place of slot depths: the
    /// per-instance [`vshm`] segment (one `in` + one `out` region)
    /// is sized from it, and the runner stashes it to replay into
    /// `VideoPluginInstance::initialize` on
    /// [`Command::VideoInitialize`]. `HostError::UnknownPluginId`
    /// (unscanned id), `HostError::InstanceLimit` (registry full),
    /// `HostError::Ipc` (vshm attach failure, the child's symmetric
    /// `frame_bytes` bound re-check, or a duplicate `instance`
    /// handle) or `HostError::PluginFailed` (factory panic) are the
    /// defined rejections — the child stays alive on every one. An
    /// over-bound `config` never reaches the wire: the host rejects
    /// it earlier as `Err(Prepare(UnsupportedConfig))`.
    CreateVideoInstance {
        /// Registered video plugin id (matched against the video
        /// adapter's descriptor `id`).
        plugin_id: String,
        /// OS shared-memory key of the host-created `vshm` segment
        /// — the child attaches it.
        shm_key: String,
        /// Processing configuration the segment was sized from.
        config: VideoProcessConfig,
    },
    /// `VideoPluginInstance::initialize(config)` → [`Reply::Ok`] —
    /// wire tag 13. The runner replays the stashed
    /// `CreateVideoInstance` `config`; `HostError::Prepare` or
    /// `HostError::PluginFailed` on plugin rejection.
    VideoInitialize,
    /// `VideoPluginInstance::activate()` → [`Reply::Ok`] — wire tag
    /// 14.
    VideoActivate,
    /// `VideoPluginInstance::start_processing()` → [`Reply::Ok`] —
    /// wire tag 15.
    VideoStartProcessing,
    /// `VideoPluginInstance::render(&mut frame, at)` →
    /// [`Reply::Rendered`] — wire tag 16. `input` describes the frame
    /// the host packed into the segment's `in` region; the plugin
    /// renders into the `out` region and `Rendered` describes what it
    /// wrote. A dims/format disagreement with the instance config
    /// answers `HostError::InvalidValue` host-side (no wire send)
    /// or `HostError::Prepare(UnsupportedConfig)` when the child
    /// receives a non-canonical `input`; `HostError::PluginFailed`
    /// on a plugin render error.
    VideoRender {
        /// Presentation timestamp passed verbatim to `render`.
        at: MediaTime,
        /// Byte-placement descriptor of the `in`-region frame.
        input: VideoFrameDesc,
    },
    /// `VideoPluginInstance::stop_processing()` → [`Reply::Ok`] —
    /// wire tag 17.
    VideoStopProcessing,
    /// `VideoPluginInstance::deactivate()` → [`Reply::Ok`] — wire
    /// tag 18.
    VideoDeactivate,
    /// List the instance's parameter ids → [`Reply::Params`] — wire
    /// tag 19.
    VideoGetParams,
    /// `VideoPluginInstance::parameter_info(id)` →
    /// [`Reply::ParamInfo`] — wire tag 20.
    VideoGetParamInfo {
        /// Parameter to describe.
        id: u32,
    },
    /// `VideoPluginInstance::parameter_value(id)` →
    /// [`Reply::ParamValue`] — wire tag 21.
    /// `HostError::InvalidValue` on an unknown `id`.
    VideoGetParamValue {
        /// Parameter to read.
        id: u32,
    },
    /// `VideoPluginInstance::set_parameter(id, value)` →
    /// [`Reply::Ok`] — wire tag 22. `HostError::InvalidValue` on an
    /// unknown `id`. A non-finite [`PlainParamValue::Double`] is a
    /// host-side `Err(InvalidValue)` (no wire send); on the wire
    /// itself it fails `decode_plain_param_value` as `Malformed` —
    /// a protocol violation, not a reply.
    VideoSetParamValue {
        /// Parameter to write.
        id: u32,
        /// New parameter value.
        value: PlainParamValue,
    },
}

/// The reply payload of a successful wire [`Command`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new replies arrive only with a [`WIRE_VERSION`] bump.
/// Tags 4-7 are the wire-v3 (Issue #118) video replies. It is
/// `PartialEq` but not `Eq`: [`PlainParamValue`] contains an `f64`.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
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
    /// `VideoRender` result — wire tag 4. The byte-placement
    /// descriptor of the frame the plugin rendered into the `vshm`
    /// `out` region; the host copies it out into a fresh
    /// `VideoFrame` to return.
    Rendered {
        /// `out`-region frame the plugin produced.
        output: VideoFrameDesc,
    },
    /// `VideoGetParams` result — wire tag 5.
    Params {
        /// Parameter ids the instance exposes.
        ids: Vec<u32>,
    },
    /// `VideoGetParamInfo` result — wire tag 6 (`None` is the
    /// plugin-declared unknown-id reply).
    ParamInfo {
        /// Owned parameter metadata, or `None`.
        info: Option<PlainParamInfoOwned>,
    },
    /// `VideoGetParamValue` result — wire tag 7.
    ParamValue {
        /// Current parameter value.
        value: PlainParamValue,
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

/// Byte placement of one frame plane inside a `vshm` region — the
/// wire form shared by [`VideoFrameDesc`] (wire v3, Issue #118).
///
/// Under canonical packing `stride` equals the plane's `row_bytes`
/// and `offset(p) = Σ len(0..p)` — see [`vshm::plane_layout`]. The
/// codec bounds every field but does *not* verify canonicality or
/// region fit: those are the runner/host's semantic layer.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VideoFramePlane {
    /// Byte offset of the plane within its region.
    pub offset: u32,
    /// Byte length the plane occupies (`stride × plane_height`
    /// under canonical packing).
    pub len: u32,
    /// Row pitch in bytes (`≤` [`WIRE_VIDEO_MAX_STRIDE`] on the
    /// wire).
    pub stride: u32,
}

/// The wire descriptor of one video frame inside a `vshm` region —
/// the `VideoFrame` metadata that crosses the pipe with
/// [`Command::VideoRender`]/[`Reply::Rendered`] (wire v3, Issue
/// #118). Frame *bytes* never travel in this type: they live in the
/// segment's in/out region the `planes` offsets address.
///
/// On the wire (≤ 127 B): `u8` format tag, `u32` width, `u32`
/// height, `u8` plane count, `{u32 offset, u32 len, u32 stride}`
/// per plane, [`MediaTime`] `pts`, `u8` presence tag +
/// [`MediaDuration`], and a bounded [`ColorInfo`].
///
/// **RT contract:** non-RT only — owns the `planes` vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoFrameDesc {
    /// Pixel format — determines `planes.len()` on the wire
    /// (`format.plane_count()`; a mismatch is
    /// [`WireError::Malformed`]).
    pub format: PixelFormat,
    /// Frame width on the full-resolution grid (`1..=
    /// WIRE_VIDEO_MAX_DIM` on the wire).
    pub width: u32,
    /// Frame height on the full-resolution grid (`1..=
    /// WIRE_VIDEO_MAX_DIM`).
    pub height: u32,
    /// Per-plane placement inside the region — one entry per
    /// `format.plane_count()` plane, `1..=WIRE_VIDEO_MAX_PLANES`
    /// on the wire.
    pub planes: Vec<VideoFramePlane>,
    /// Presentation timestamp.
    pub pts: MediaTime,
    /// Optional frame duration.
    pub duration: Option<MediaDuration>,
    /// Color space and HDR metadata.
    pub color: ColorInfo,
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

    /// Reads one little-endian `i32`.
    pub fn read_i32(&mut self) -> Result<i32, WireError> {
        Ok(i32::from_le_bytes(*self.array::<4>()?))
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

    /// Reads the wire-v2 `u32 instance` envelope field —
    /// [`WireError::Malformed`] when the value is neither a live
    /// handle `< WIRE_MAX_INSTANCES` nor [`WIRE_INSTANCE_NONE`]. The
    /// verb↔scope table (`Scan`↔`NONE`, doorbells↔live) is checked by
    /// the caller, which knows the frame kind.
    pub fn read_instance(&mut self) -> Result<u32, WireError> {
        let instance = self.read_u32()?;
        if instance != WIRE_INSTANCE_NONE && instance >= WIRE_MAX_INSTANCES {
            return Err(WireError::Malformed);
        }
        Ok(instance)
    }

    /// Reads a `u32 instance` that must name a live handle —
    /// [`WIRE_INSTANCE_NONE`] is a scope violation
    /// ([`WireError::Malformed`]) for doorbells and every non-`Scan`
    /// command.
    pub fn read_live_instance(&mut self) -> Result<u32, WireError> {
        match self.read_instance()? {
            WIRE_INSTANCE_NONE => Err(WireError::Malformed),
            instance => Ok(instance),
        }
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
        Message::Command { seq, instance, cmd } => {
            push_u8(out, KIND_COMMAND);
            push_u64(out, *seq);
            push_u32(out, *instance);
            encode_command(cmd, out)?;
        }
        Message::Reply {
            seq,
            instance,
            result,
        } => {
            push_u8(out, KIND_REPLY);
            push_u64(out, *seq);
            push_u32(out, *instance);
            match result {
                Ok(reply) => {
                    push_u8(out, REPLY_OK);
                    encode_reply(reply, out)?;
                }
                Err(error) => {
                    push_u8(out, REPLY_ERR);
                    // Propagates to `encode`, which performs the
                    // rollback.
                    encode_host_error(error, out)?;
                }
            }
        }
        Message::BlockIn { seq, instance } => {
            push_u8(out, KIND_BLOCK_IN);
            push_u64(out, *seq);
            push_u32(out, *instance);
        }
        Message::BlockDone {
            seq,
            instance,
            result,
        } => {
            push_u8(out, KIND_BLOCK_DONE);
            push_u64(out, *seq);
            push_u32(out, *instance);
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
/// wire::encode(
///     &Message::BlockIn {
///         seq: 9,
///         instance: 0,
///     },
///     &mut frame,
/// )
/// .unwrap();
/// assert_eq!(
///     wire::decode(&frame),
///     Ok(Message::BlockIn {
///         seq: 9,
///         instance: 0
///     })
/// );
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
        KIND_COMMAND => {
            let seq = reader.read_u64()?;
            let instance = reader.read_instance()?;
            let verb = reader.read_u8()?;
            // Verb↔scope table: `Scan` is the only child-scoped verb
            // (`instance == WIRE_INSTANCE_NONE`); every other verb
            // requires a live-range instance. Scope is settled by the
            // envelope and verb tag — both fixed-position — before the
            // untrusted verb payload is interpreted, so a scope
            // violation is the defined `Malformed` rejection even on
            // an incomplete body.
            if (verb == VERB_SCAN) != (instance == WIRE_INSTANCE_NONE) {
                return Err(WireError::Malformed);
            }
            let cmd = decode_command(verb, &mut reader)?;
            Message::Command { seq, instance, cmd }
        }
        KIND_REPLY => Message::Reply {
            seq: reader.read_u64()?,
            // `Scan` replies echo `WIRE_INSTANCE_NONE`; all others
            // echo a live-range instance — either is legal here, the
            // host's per-waiter check attributes it.
            instance: reader.read_instance()?,
            result: match reader.read_u8()? {
                REPLY_OK => Ok(decode_reply(&mut reader)?),
                REPLY_ERR => Err(decode_host_error(&mut reader)?),
                _ => return Err(WireError::Malformed),
            },
        },
        KIND_BLOCK_IN => Message::BlockIn {
            seq: reader.read_u64()?,
            // Doorbells are always instance-scoped — `NONE` is a
            // scope violation, not an unset field.
            instance: reader.read_live_instance()?,
        },
        KIND_BLOCK_DONE => Message::BlockDone {
            seq: reader.read_u64()?,
            instance: reader.read_live_instance()?,
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

/// `Command` verb tag 0: `Scan` — the only child-scoped verb, named
/// here because the decode-time scope table keys on it.
const VERB_SCAN: u8 = 0;

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

fn push_i32(out: &mut Vec<u8>, value: i32) {
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

/// Writes `command`'s verb tag plus payload fields. Fallible through
/// the video verbs' nested codecs (a `#[non_exhaustive]` `PixelFormat`
/// or color enum added after this revision has no `WIRE_VERSION` 3
/// representation) — `encode` owns the all-or-nothing rollback for
/// every `Err` from here.
fn encode_command(command: &Command, out: &mut Vec<u8>) -> Result<(), WireError> {
    match command {
        Command::Scan => push_u8(out, VERB_SCAN),
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
        Command::CreateInstance {
            plugin_id,
            shm_key,
            in_depth,
            out_depth,
        } => {
            push_u8(out, 10);
            push_str(out, plugin_id);
            push_str(out, shm_key);
            push_u32(out, *in_depth);
            push_u32(out, *out_depth);
        }
        Command::DestroyInstance => push_u8(out, 11),
        Command::CreateVideoInstance {
            plugin_id,
            shm_key,
            config,
        } => {
            push_u8(out, 12);
            push_str(out, plugin_id);
            push_str(out, shm_key);
            encode_video_process_config(config, out)?;
        }
        Command::VideoInitialize => push_u8(out, 13),
        Command::VideoActivate => push_u8(out, 14),
        Command::VideoStartProcessing => push_u8(out, 15),
        Command::VideoRender { at, input } => {
            push_u8(out, 16);
            encode_media_time(at, out);
            encode_video_frame_desc(input, out)?;
        }
        Command::VideoStopProcessing => push_u8(out, 17),
        Command::VideoDeactivate => push_u8(out, 18),
        Command::VideoGetParams => push_u8(out, 19),
        Command::VideoGetParamInfo { id } => {
            push_u8(out, 20);
            push_u32(out, *id);
        }
        Command::VideoGetParamValue { id } => {
            push_u8(out, 21);
            push_u32(out, *id);
        }
        Command::VideoSetParamValue { id, value } => {
            push_u8(out, 22);
            push_u32(out, *id);
            encode_plain_param_value(value, out);
        }
    }
    Ok(())
}

/// Decodes one verb payload; `verb` is the tag byte the caller
/// already consumed for the scope check.
fn decode_command(verb: u8, reader: &mut Reader<'_>) -> Result<Command, WireError> {
    match verb {
        VERB_SCAN => Ok(Command::Scan),
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
        10 => {
            let plugin_id = reader.read_string()?;
            let shm_key = reader.read_string()?;
            let in_depth = reader.read_u32()?;
            let out_depth = reader.read_u32()?;
            // Depth declarations are bounded like every other scalar
            // that sizes an allocation — `0` is invalid data,
            // over-bound is the defined length rejection.
            if in_depth > WIRE_MAX_ELEMENTS || out_depth > WIRE_MAX_ELEMENTS {
                return Err(WireError::LimitExceeded);
            }
            if in_depth == 0 || out_depth == 0 {
                return Err(WireError::Malformed);
            }
            Ok(Command::CreateInstance {
                plugin_id,
                shm_key,
                in_depth,
                out_depth,
            })
        }
        11 => Ok(Command::DestroyInstance),
        12 => {
            let plugin_id = reader.read_string()?;
            let shm_key = reader.read_string()?;
            let config = decode_video_process_config(reader)?;
            Ok(Command::CreateVideoInstance {
                plugin_id,
                shm_key,
                config,
            })
        }
        13 => Ok(Command::VideoInitialize),
        14 => Ok(Command::VideoActivate),
        15 => Ok(Command::VideoStartProcessing),
        16 => Ok(Command::VideoRender {
            at: decode_media_time(reader)?,
            input: decode_video_frame_desc(reader)?,
        }),
        17 => Ok(Command::VideoStopProcessing),
        18 => Ok(Command::VideoDeactivate),
        19 => Ok(Command::VideoGetParams),
        20 => Ok(Command::VideoGetParamInfo {
            id: reader.read_u32()?,
        }),
        21 => Ok(Command::VideoGetParamValue {
            id: reader.read_u32()?,
        }),
        22 => Ok(Command::VideoSetParamValue {
            id: reader.read_u32()?,
            value: decode_plain_param_value(reader)?,
        }),
        _ => Err(WireError::Malformed),
    }
}

/// Writes `reply`'s tag plus payload fields. Fallible through the
/// video replies' [`VideoFrameDesc`] codec — `encode` owns the
/// all-or-nothing rollback for every `Err` from here.
fn encode_reply(reply: &Reply, out: &mut Vec<u8>) -> Result<(), WireError> {
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
        Reply::Rendered { output } => {
            push_u8(out, 4);
            encode_video_frame_desc(output, out)?;
        }
        Reply::Params { ids } => {
            push_u8(out, 5);
            push_u32(out, len_u32(ids.len()));
            for id in ids {
                push_u32(out, *id);
            }
        }
        Reply::ParamInfo { info } => {
            push_u8(out, 6);
            match info {
                None => push_u8(out, 0),
                Some(info) => {
                    push_u8(out, 1);
                    encode_plain_param_info(info, out);
                }
            }
        }
        Reply::ParamValue { value } => {
            push_u8(out, 7);
            encode_plain_param_value(value, out);
        }
    }
    Ok(())
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
        4 => Ok(Reply::Rendered {
            output: decode_video_frame_desc(reader)?,
        }),
        5 => {
            let count = reader.read_count(WIRE_MAX_ELEMENTS)?;
            let mut ids = Vec::with_capacity(count as usize);
            for _ in 0..count {
                ids.push(reader.read_u32()?);
            }
            Ok(Reply::Params { ids })
        }
        6 => Ok(Reply::ParamInfo {
            info: match reader.read_u8()? {
                0 => None,
                1 => Some(decode_plain_param_info(reader)?),
                _ => return Err(WireError::Malformed),
            },
        }),
        7 => Ok(Reply::ParamValue {
            value: decode_plain_param_value(reader)?,
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
        HostError::InstanceLimit => {
            push_u8(out, 9);
            Ok(())
        }
        HostError::InvalidValue => {
            push_u8(out, 10);
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
        9 => Ok(HostError::InstanceLimit),
        10 => Ok(HostError::InvalidValue),
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
/// `#[non_exhaustive]` variant added after [`WIRE_VERSION`] `3` — it
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

// --- Video wire types (WIRE_VERSION 3, Issue #118) ------------------------
//
// The video domain types are `#[non_exhaustive]` in
// `ruviform-video-core`, so their tag codecs are fallible on the
// encode side too: a variant added after this revision has no
// `WIRE_VERSION` 3 representation (`WireError::UnsupportedVariant`)
// rather than a silently wrong tag.

/// Appends a [`VideoProcessConfig`] (`u8` format tag, `u32` width,
/// `u32` height, `u32` `FrameRate` numerator, `u32` denominator — 17 B).
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when `config.format()` is a
/// `#[non_exhaustive]` [`PixelFormat`] variant added after this
/// protocol revision. Nothing is appended on failure.
pub fn encode_video_process_config(
    config: &VideoProcessConfig,
    out: &mut Vec<u8>,
) -> Result<(), WireError> {
    let format = encode_pixel_format(config.format())?;
    push_u8(out, format);
    push_u32(out, config.width());
    push_u32(out, config.height());
    push_u32(out, config.frame_rate().numerator());
    push_u32(out, config.frame_rate().denominator());
    Ok(())
}

/// Reads a [`VideoProcessConfig`] — an unassigned format tag, dims
/// outside `1..=WIRE_VIDEO_MAX_DIM` and a zero `FrameRate` component
/// are rejected (the ctor re-checks the domain invariants).
///
/// # Errors
///
/// [`WireError`] — truncated input, [`WireError::LimitExceeded`] for
/// dims over [`WIRE_VIDEO_MAX_DIM`], or [`WireError::Malformed`] for
/// a bad tag/zero field.
pub fn decode_video_process_config(
    reader: &mut Reader<'_>,
) -> Result<VideoProcessConfig, WireError> {
    let format = decode_pixel_format(reader)?;
    let width = reader.read_u32()?;
    let height = reader.read_u32()?;
    if width > WIRE_VIDEO_MAX_DIM || height > WIRE_VIDEO_MAX_DIM {
        return Err(WireError::LimitExceeded);
    }
    let frame_rate =
        FrameRate::new(reader.read_u32()?, reader.read_u32()?).map_err(|_| WireError::Malformed)?;
    // `new` rejects the zero dims the wire bound left through —
    // zero is invalid data, not an over-bound declaration.
    VideoProcessConfig::new(format, width, height, frame_rate).map_err(|_| WireError::Malformed)
}

/// The `u8` tag of a [`PixelFormat`] — declaration order on the
/// enum (0 `Yuv420p` … 5 `RgbaF32`).
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] for a variant added after this
/// revision.
fn encode_pixel_format(format: PixelFormat) -> Result<u8, WireError> {
    match format {
        PixelFormat::Yuv420p => Ok(0),
        PixelFormat::Yuv422p => Ok(1),
        PixelFormat::Yuv444p => Ok(2),
        PixelFormat::Rgba8 => Ok(3),
        PixelFormat::RgbaF16 => Ok(4),
        PixelFormat::RgbaF32 => Ok(5),
        _ => Err(WireError::UnsupportedVariant),
    }
}

/// Reads a [`PixelFormat`] tag — an unassigned tag is
/// [`WireError::Malformed`].
fn decode_pixel_format(reader: &mut Reader<'_>) -> Result<PixelFormat, WireError> {
    match reader.read_u8()? {
        0 => Ok(PixelFormat::Yuv420p),
        1 => Ok(PixelFormat::Yuv422p),
        2 => Ok(PixelFormat::Yuv444p),
        3 => Ok(PixelFormat::Rgba8),
        4 => Ok(PixelFormat::RgbaF16),
        5 => Ok(PixelFormat::RgbaF32),
        _ => Err(WireError::Malformed),
    }
}

/// Appends a [`VideoFrameDesc`] — the `≤ 127 B` layout documented on
/// the type.
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when the format or a color enum
/// has no representation under this revision — [`encode`]'s
/// frame-level rollback discards the partial payload.
///
/// # Panics
///
/// Panics when `desc.planes.len()` exceeds `u8::MAX` — a caller-side
/// bug (a well-formed desc holds `format.plane_count()` ≤ 4 entries;
/// untrusted bytes only ever flow through `decode_*`).
pub fn encode_video_frame_desc(desc: &VideoFrameDesc, out: &mut Vec<u8>) -> Result<(), WireError> {
    let format = encode_pixel_format(desc.format)?;
    let plane_count = u8::try_from(desc.planes.len()).expect("plane count fits u8");
    push_u8(out, format);
    push_u32(out, desc.width);
    push_u32(out, desc.height);
    push_u8(out, plane_count);
    for plane in &desc.planes {
        push_u32(out, plane.offset);
        push_u32(out, plane.len);
        push_u32(out, plane.stride);
    }
    encode_media_time(&desc.pts, out);
    match desc.duration {
        None => push_u8(out, 0),
        Some(duration) => {
            push_u8(out, 1);
            encode_media_duration(&duration, out);
        }
    }
    // `encode_color_info` resolves every fallible tag before its
    // first push, so a `?` here still satisfies "nothing is appended
    // on failure" *of the color payload*; earlier fields are this
    // call's own output — encode()'s frame-level rollback covers the
    // `Err` path.
    encode_color_info(&desc.color, out)
}

/// Reads a [`VideoFrameDesc`] — an unassigned format/presence tag,
/// dims outside `1..=WIRE_VIDEO_MAX_DIM`, a plane count of `0`, over
/// [`WIRE_VIDEO_MAX_PLANES`] or disagreeing with
/// `format.plane_count()`, a stride over [`WIRE_VIDEO_MAX_STRIDE`],
/// or a malformed nested field are all defined rejections.
///
/// Bounds only: canonical offsets/region-fit/`len == stride ×
/// plane_height` are the runner/host's semantic layer, not the
/// codec's.
///
/// # Errors
///
/// [`WireError`] — truncated input, [`WireError::LimitExceeded`] for
/// an over-bound declared value, or [`WireError::Malformed`].
pub fn decode_video_frame_desc(reader: &mut Reader<'_>) -> Result<VideoFrameDesc, WireError> {
    let format = decode_pixel_format(reader)?;
    let width = reader.read_u32()?;
    let height = reader.read_u32()?;
    if width > WIRE_VIDEO_MAX_DIM || height > WIRE_VIDEO_MAX_DIM {
        return Err(WireError::LimitExceeded);
    }
    if width == 0 || height == 0 {
        return Err(WireError::Malformed);
    }
    let plane_count = reader.read_u8()?;
    if u32::from(plane_count) > WIRE_VIDEO_MAX_PLANES {
        return Err(WireError::LimitExceeded);
    }
    if plane_count == 0 {
        return Err(WireError::Malformed);
    }
    if usize::from(plane_count) != format.plane_count() {
        // The declared plane list must match the format's layout —
        // e.g. `Rgba8` has exactly one plane.
        return Err(WireError::Malformed);
    }
    let mut planes = Vec::with_capacity(usize::from(plane_count));
    for _ in 0..plane_count {
        let offset = reader.read_u32()?;
        let len = reader.read_u32()?;
        let stride = reader.read_u32()?;
        if stride > WIRE_VIDEO_MAX_STRIDE {
            return Err(WireError::LimitExceeded);
        }
        planes.push(VideoFramePlane {
            offset,
            len,
            stride,
        });
    }
    let pts = decode_media_time(reader)?;
    let duration = match reader.read_u8()? {
        0 => None,
        1 => Some(decode_media_duration(reader)?),
        _ => return Err(WireError::Malformed),
    };
    let color = decode_color_info(reader)?;
    Ok(VideoFrameDesc {
        format,
        width,
        height,
        planes,
        pts,
        duration,
        color,
    })
}

/// Appends a [`ColorInfo`] — `u8` primaries, transfer, matrix and
/// range tags; then `u8` `has_hdr` and, when set, an [`HdrMetadata`]
/// (`u8` `has_mastering` + 24 B [`MasteringDisplayInfo`], then
/// `u8` `has_cll` + `u16` `max_cll` + `u16` `max_fall`).
///
/// # Errors
///
/// [`WireError::UnsupportedVariant`] when a color enum carries a
/// `#[non_exhaustive]` variant added after this revision. Nothing is
/// appended on failure — every fallible tag resolves before the
/// first push.
pub fn encode_color_info(color: &ColorInfo, out: &mut Vec<u8>) -> Result<(), WireError> {
    let primaries = encode_color_primaries(color.primaries)?;
    let transfer = encode_transfer_characteristics(color.transfer)?;
    let matrix = encode_matrix_coefficients(color.matrix)?;
    let range = encode_color_range(color.range)?;
    push_u8(out, primaries);
    push_u8(out, transfer);
    push_u8(out, matrix);
    push_u8(out, range);
    match &color.hdr {
        None => push_u8(out, 0),
        Some(hdr) => {
            push_u8(out, 1);
            match &hdr.mastering_display {
                None => push_u8(out, 0),
                Some(mastering) => {
                    push_u8(out, 1);
                    encode_mastering_display_info(mastering, out);
                }
            }
            match &hdr.content_light_level {
                None => push_u8(out, 0),
                Some(cll) => {
                    push_u8(out, 1);
                    push_u16(out, cll.max_cll);
                    push_u16(out, cll.max_fall);
                }
            }
        }
    }
    Ok(())
}

/// Reads a [`ColorInfo`] — unassigned enum tags and any presence
/// byte outside the strict `{0,1}` set are [`WireError::Malformed`].
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_color_info(reader: &mut Reader<'_>) -> Result<ColorInfo, WireError> {
    let primaries = decode_color_primaries(reader)?;
    let transfer = decode_transfer_characteristics(reader)?;
    let matrix = decode_matrix_coefficients(reader)?;
    let range = decode_color_range(reader)?;
    let hdr = match reader.read_u8()? {
        0 => None,
        1 => Some(HdrMetadata {
            mastering_display: match reader.read_u8()? {
                0 => None,
                1 => Some(decode_mastering_display_info(reader)?),
                _ => return Err(WireError::Malformed),
            },
            content_light_level: match reader.read_u8()? {
                0 => None,
                1 => Some(ContentLightLevel {
                    max_cll: reader.read_u16()?,
                    max_fall: reader.read_u16()?,
                }),
                _ => return Err(WireError::Malformed),
            },
        }),
        _ => return Err(WireError::Malformed),
    };
    Ok(ColorInfo {
        primaries,
        transfer,
        matrix,
        range,
        hdr,
    })
}

fn encode_mastering_display_info(info: &MasteringDisplayInfo, out: &mut Vec<u8>) {
    for chromaticity in &info.primaries {
        push_u16(out, chromaticity.x);
        push_u16(out, chromaticity.y);
    }
    push_u16(out, info.white_point.x);
    push_u16(out, info.white_point.y);
    push_u32(out, info.max_luminance);
    push_u32(out, info.min_luminance);
}

fn decode_mastering_display_info(
    reader: &mut Reader<'_>,
) -> Result<MasteringDisplayInfo, WireError> {
    let mut primaries = [Chromaticity { x: 0, y: 0 }; 3];
    for chromaticity in &mut primaries {
        chromaticity.x = reader.read_u16()?;
        chromaticity.y = reader.read_u16()?;
    }
    Ok(MasteringDisplayInfo {
        primaries,
        white_point: Chromaticity {
            x: reader.read_u16()?,
            y: reader.read_u16()?,
        },
        max_luminance: reader.read_u32()?,
        min_luminance: reader.read_u32()?,
    })
}

// Color-enum tag codecs — declaration-order tags, `Malformed` on an
// unassigned value, `UnsupportedVariant` on a variant added after
// this revision (every enum is `#[non_exhaustive]` in
// ruviform-video-core).

fn encode_color_primaries(primaries: ColorPrimaries) -> Result<u8, WireError> {
    match primaries {
        ColorPrimaries::Unspecified => Ok(0),
        ColorPrimaries::Bt709 => Ok(1),
        ColorPrimaries::Smpte170m => Ok(2),
        ColorPrimaries::Bt470Bg => Ok(3),
        ColorPrimaries::Bt2020 => Ok(4),
        ColorPrimaries::P3D65 => Ok(5),
        _ => Err(WireError::UnsupportedVariant),
    }
}

fn decode_color_primaries(reader: &mut Reader<'_>) -> Result<ColorPrimaries, WireError> {
    match reader.read_u8()? {
        0 => Ok(ColorPrimaries::Unspecified),
        1 => Ok(ColorPrimaries::Bt709),
        2 => Ok(ColorPrimaries::Smpte170m),
        3 => Ok(ColorPrimaries::Bt470Bg),
        4 => Ok(ColorPrimaries::Bt2020),
        5 => Ok(ColorPrimaries::P3D65),
        _ => Err(WireError::Malformed),
    }
}

fn encode_transfer_characteristics(transfer: TransferCharacteristics) -> Result<u8, WireError> {
    match transfer {
        TransferCharacteristics::Unspecified => Ok(0),
        TransferCharacteristics::Bt709 => Ok(1),
        TransferCharacteristics::Smpte170m => Ok(2),
        TransferCharacteristics::Linear => Ok(3),
        TransferCharacteristics::Pq => Ok(4),
        TransferCharacteristics::Hlg => Ok(5),
        _ => Err(WireError::UnsupportedVariant),
    }
}

fn decode_transfer_characteristics(
    reader: &mut Reader<'_>,
) -> Result<TransferCharacteristics, WireError> {
    match reader.read_u8()? {
        0 => Ok(TransferCharacteristics::Unspecified),
        1 => Ok(TransferCharacteristics::Bt709),
        2 => Ok(TransferCharacteristics::Smpte170m),
        3 => Ok(TransferCharacteristics::Linear),
        4 => Ok(TransferCharacteristics::Pq),
        5 => Ok(TransferCharacteristics::Hlg),
        _ => Err(WireError::Malformed),
    }
}

fn encode_matrix_coefficients(matrix: MatrixCoefficients) -> Result<u8, WireError> {
    match matrix {
        MatrixCoefficients::Unspecified => Ok(0),
        MatrixCoefficients::Identity => Ok(1),
        MatrixCoefficients::Bt601 => Ok(2),
        MatrixCoefficients::Bt709 => Ok(3),
        MatrixCoefficients::Bt2020 => Ok(4),
        _ => Err(WireError::UnsupportedVariant),
    }
}

fn decode_matrix_coefficients(reader: &mut Reader<'_>) -> Result<MatrixCoefficients, WireError> {
    match reader.read_u8()? {
        0 => Ok(MatrixCoefficients::Unspecified),
        1 => Ok(MatrixCoefficients::Identity),
        2 => Ok(MatrixCoefficients::Bt601),
        3 => Ok(MatrixCoefficients::Bt709),
        4 => Ok(MatrixCoefficients::Bt2020),
        _ => Err(WireError::Malformed),
    }
}

fn encode_color_range(range: ColorRange) -> Result<u8, WireError> {
    match range {
        ColorRange::Unspecified => Ok(0),
        ColorRange::Limited => Ok(1),
        ColorRange::Full => Ok(2),
        _ => Err(WireError::UnsupportedVariant),
    }
}

fn decode_color_range(reader: &mut Reader<'_>) -> Result<ColorRange, WireError> {
    match reader.read_u8()? {
        0 => Ok(ColorRange::Unspecified),
        1 => Ok(ColorRange::Limited),
        2 => Ok(ColorRange::Full),
        _ => Err(WireError::Malformed),
    }
}

// --- Plain-parameter wire types -------------------------------------------

/// Appends a [`PlainParamValue`] — `u8` kind tag + payload (`f64`
/// Double, `i32` Integer, `u8` 0/1 Boolean, `u32` Choice index).
///
/// # Panics
///
/// Never — every variant serializes verbatim.
pub fn encode_plain_param_value(value: &PlainParamValue, out: &mut Vec<u8>) {
    match value {
        PlainParamValue::Double(v) => {
            push_u8(out, 0);
            push_f64(out, *v);
        }
        PlainParamValue::Integer(v) => {
            push_u8(out, 1);
            push_i32(out, *v);
        }
        PlainParamValue::Boolean(v) => {
            push_u8(out, 2);
            push_u8(out, u8::from(*v));
        }
        PlainParamValue::Choice(index) => {
            push_u8(out, 3);
            push_u32(out, *index);
        }
    }
}

/// Reads a [`PlainParamValue`] — an unassigned kind tag, a non-finite
/// `Double` (`NaN`/`±Inf` can never be a parameter value — the same
/// finiteness rule [`ParamValue`] applies), or a `Boolean` byte
/// outside the strict `{0,1}` set are [`WireError::Malformed`].
///
/// # Errors
///
/// [`WireError`] — truncated or malformed input.
pub fn decode_plain_param_value(reader: &mut Reader<'_>) -> Result<PlainParamValue, WireError> {
    match reader.read_u8()? {
        0 => {
            let value = reader.read_f64()?;
            if !value.is_finite() {
                return Err(WireError::Malformed);
            }
            Ok(PlainParamValue::Double(value))
        }
        1 => Ok(PlainParamValue::Integer(reader.read_i32()?)),
        2 => Ok(PlainParamValue::Boolean(match reader.read_u8()? {
            0 => false,
            1 => true,
            _ => return Err(WireError::Malformed),
        })),
        3 => Ok(PlainParamValue::Choice(reader.read_u32()?)),
        _ => Err(WireError::Malformed),
    }
}

/// Appends a [`PlainParamInfoOwned`] — `u32` id, `name`, `unit`,
/// `f64` `min_value`/`max_value`/`display_min_value`/
/// `display_max_value`, [`PlainParamValue`] default, `u32` flags
/// bits, `u32` choice count + each choice string.
///
/// # Panics
///
/// Never — every field serializes verbatim.
pub fn encode_plain_param_info(info: &PlainParamInfoOwned, out: &mut Vec<u8>) {
    push_u32(out, info.id.0);
    push_str(out, &info.name);
    push_str(out, &info.unit);
    push_f64(out, info.min_value);
    push_f64(out, info.max_value);
    push_f64(out, info.display_min_value);
    push_f64(out, info.display_max_value);
    encode_plain_param_value(&info.default_value, out);
    push_u32(out, info.flags.bits());
    push_u32(out, len_u32(info.choices.len()));
    for choice in &info.choices {
        push_str(out, choice);
    }
}

/// Reads a [`PlainParamInfoOwned`] — the wire is the trust boundary,
/// so the full metadata contract is re-validated here:
///
/// * strings/choice list bounded before allocation
///   ([`WIRE_MAX_STRING_BYTES`]/[`WIRE_MAX_ELEMENTS`]);
/// * `min_value`, `max_value` and the default's scalar must be
///   finite, with `min_value <= max_value` and the default inside
///   `[min_value, max_value]` — `display_*` are UI hints and stay
///   lenient (non-finite accepted);
/// * `default_value` is `Choice` ⇔ `choices` is non-empty, and a
///   `Choice` default must index into `choices`;
/// * `flags` must contain only bits this revision defines
///   ([`ParamFlags::from_bits`] is strict, like a reserved field).
///
/// # Errors
///
/// [`WireError`] — truncated input, [`WireError::LimitExceeded`] for
/// an over-bound declared length/count, or [`WireError::Malformed`]
/// for a contract violation.
pub fn decode_plain_param_info(reader: &mut Reader<'_>) -> Result<PlainParamInfoOwned, WireError> {
    let id = ParamId(reader.read_u32()?);
    let name = reader.read_string()?;
    let unit = reader.read_string()?;
    let min_value = reader.read_f64()?;
    let max_value = reader.read_f64()?;
    let display_min_value = reader.read_f64()?;
    let display_max_value = reader.read_f64()?;
    if !min_value.is_finite() || !max_value.is_finite() || min_value > max_value {
        return Err(WireError::Malformed);
    }
    let default_value = decode_plain_param_value(reader)?;
    let default_scalar = match default_value {
        PlainParamValue::Double(v) => v,
        PlainParamValue::Integer(v) => f64::from(v),
        PlainParamValue::Boolean(v) => f64::from(u8::from(v)),
        // The range check for `Choice` is the in-bounds index check
        // against `choices` below — its "scalar" is an index.
        PlainParamValue::Choice(_) => min_value,
    };
    if !(min_value..=max_value).contains(&default_scalar) {
        return Err(WireError::Malformed);
    }
    let flags = ParamFlags::from_bits(reader.read_u32()?).ok_or(WireError::Malformed)?;
    let count = reader.read_count(WIRE_MAX_ELEMENTS)?;
    let mut choices = Vec::with_capacity(count as usize);
    for _ in 0..count {
        choices.push(reader.read_string()?);
    }
    match default_value {
        PlainParamValue::Choice(index) => {
            if index as usize >= choices.len() {
                // A Choice default must name a real choice — and
                // `choices.len() == 0` makes every index out of
                // range, folding the "empty choice list" rejection
                // into this one check.
                return Err(WireError::Malformed);
            }
        }
        _ => {
            if !choices.is_empty() {
                return Err(WireError::Malformed);
            }
        }
    }
    Ok(PlainParamInfoOwned {
        id,
        name,
        unit,
        min_value,
        max_value,
        display_min_value,
        display_max_value,
        default_value,
        flags,
        choices,
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
/// wire::encode(
///     &Message::BlockIn {
///         seq: 7,
///         instance: 0,
///     },
///     &mut frame,
/// )
/// .unwrap();
///
/// let mut buf = Vec::new();
/// let mut cursor = std::io::Cursor::new(frame);
/// assert_eq!(
///     wire::read_message(&mut cursor, &mut buf).unwrap(),
///     Message::BlockIn {
///         seq: 7,
///         instance: 0
///     },
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
/// wire::write_message(
///     &mut out,
///     &Message::BlockIn {
///         seq: 7,
///         instance: 0,
///     },
///     &mut Vec::new(),
/// )
/// .unwrap();
/// assert_eq!(
///     wire::decode(&out),
///     Ok(Message::BlockIn {
///         seq: 7,
///         instance: 0
///     })
/// );
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
        // `WIRE_MAX_INSTANCES` at the default depth (3 slots each
        // way, `lookahead_blocks` 2 + 1) caps one child's named
        // mappings at ~204.4 MiB — the Issue #117 capacity rationale
        // (design L375), pinned so a layout change re-weighs it.
        const {
            assert!(shm::segment_len(3, 3) * WIRE_MAX_INSTANCES as usize == 214_353_664);
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
        // `Scan` is the only child-scoped verb — its envelope is
        // `WIRE_INSTANCE_NONE`; every other verb rides a live
        // instance handle.
        for (cmd, instance) in [
            (Command::Scan, WIRE_INSTANCE_NONE),
            (
                Command::CreateInstance {
                    plugin_id: "org.ruviform.test.gain".to_owned(),
                    shm_key: "rvf-test-segment".to_owned(),
                    in_depth: 4,
                    out_depth: 4,
                },
                0,
            ),
            (Command::DestroyInstance, 0),
            (Command::Initialize, 0),
            (Command::Activate { config: config() }, 0),
            (Command::StartProcessing, 0),
            (Command::StopProcessing, 0),
            (Command::Deactivate, 0),
            (Command::SaveState, 0),
            (
                Command::RestoreState {
                    bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
                },
                0,
            ),
            (
                Command::OpenGui {
                    parent: GuiParentToken::new(0xABCD_EF12_3456_7890),
                },
                0,
            ),
            (Command::CloseGui, 0),
        ] {
            let message = Message::Command {
                seq: 7,
                instance,
                cmd,
            };
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
                instance: 3,
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
            HostError::InstanceLimit,
            HostError::InvalidValue,
        ];
        for error in errors {
            let message = Message::Reply {
                seq: 7,
                instance: 3,
                result: Err(error),
            };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn doorbells_roundtrip() {
        assert_eq!(
            roundtrip(&Message::BlockIn {
                seq: u64::MAX,
                instance: 0
            }),
            Message::BlockIn {
                seq: u64::MAX,
                instance: 0
            }
        );
        let done = Message::BlockDone {
            seq: 1_023,
            instance: WIRE_MAX_INSTANCES - 1,
            result: ProcessResult::new(ProcessStatus::Sleep, 2_048),
        };
        assert_eq!(roundtrip(&done), done);
        assert_eq!(roundtrip(&Message::Shutdown), Message::Shutdown);
    }

    // --- wire-v2 instance envelope -------------------------------------

    /// Builds one `[len][kind][payload]` frame by hand — the
    /// decode-side envelope tests need encodings `encode` cannot
    /// produce (scope violations, out-of-range instances).
    fn raw_frame(kind: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        push_u32(&mut frame, len_u32(payload.len()));
        push_u8(&mut frame, kind);
        frame.extend_from_slice(payload);
        frame
    }

    /// A `Command` payload: `u64 seq`, `u32 instance`, `u8` verb tag
    /// (+ `u8` verb-tag payload byte when `tagged`).
    fn command_payload(seq: u64, instance: u32, tag: u8) -> Vec<u8> {
        let mut payload = Vec::new();
        push_u64(&mut payload, seq);
        push_u32(&mut payload, instance);
        push_u8(&mut payload, tag);
        payload
    }

    #[test]
    fn scan_is_the_only_child_scoped_verb() {
        // Scan with a live instance → scope violation.
        assert_eq!(
            decode(&raw_frame(KIND_COMMAND, &command_payload(1, 0, 0))),
            Err(WireError::Malformed)
        );
        // Scan with WIRE_INSTANCE_NONE → legal.
        assert_eq!(
            decode(&raw_frame(
                KIND_COMMAND,
                &command_payload(1, WIRE_INSTANCE_NONE, 0)
            )),
            Ok(Message::Command {
                seq: 1,
                instance: WIRE_INSTANCE_NONE,
                cmd: Command::Scan,
            })
        );
        // Initialize (verb tag 1) with WIRE_INSTANCE_NONE → scope
        // violation.
        assert_eq!(
            decode(&raw_frame(
                KIND_COMMAND,
                &command_payload(1, WIRE_INSTANCE_NONE, 1)
            )),
            Err(WireError::Malformed)
        );
        // CreateInstance (tag 10) with NONE → same scope violation:
        // registration names a live handle in the envelope.
        assert_eq!(
            decode(&raw_frame(
                KIND_COMMAND,
                &command_payload(1, WIRE_INSTANCE_NONE, 10)
            )),
            Err(WireError::Malformed)
        );
    }

    #[test]
    fn instance_envelope_rejects_out_of_range_values() {
        // `WIRE_MAX_INSTANCES` itself and everything above it —
        // except `WIRE_INSTANCE_NONE` — is out of range. Check
        // Command, Reply, and both doorbells.
        for bad in [
            WIRE_MAX_INSTANCES,
            WIRE_MAX_INSTANCES + 1,
            WIRE_INSTANCE_NONE - 1,
        ] {
            assert_eq!(
                decode(&raw_frame(KIND_COMMAND, &command_payload(1, bad, 0))),
                Err(WireError::Malformed),
                "instance {bad} must be rejected",
            );

            let mut payload = Vec::new();
            push_u64(&mut payload, 1);
            push_u32(&mut payload, bad);
            push_u8(&mut payload, REPLY_OK);
            push_u8(&mut payload, 0); // Reply::Ok
            assert_eq!(
                decode(&raw_frame(KIND_REPLY, &payload)),
                Err(WireError::Malformed),
                "reply instance {bad} must be rejected",
            );
        }
        // A Scan reply legitimately echoes WIRE_INSTANCE_NONE — only
        // Command/Reply allow it at all.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, WIRE_INSTANCE_NONE);
        push_u8(&mut payload, REPLY_OK);
        push_u8(&mut payload, 1); // Scanned
        push_u32(&mut payload, 0); // zero descriptors
        assert_eq!(
            decode(&raw_frame(KIND_REPLY, &payload)),
            Ok(Message::Reply {
                seq: 1,
                instance: WIRE_INSTANCE_NONE,
                result: Ok(Reply::Scanned(Vec::new())),
            })
        );
    }

    #[test]
    fn doorbells_require_a_live_instance() {
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, WIRE_INSTANCE_NONE);
        assert_eq!(
            decode(&raw_frame(KIND_BLOCK_IN, &payload)),
            Err(WireError::Malformed)
        );
        // BlockDone carries a ProcessResult after the envelope.
        push_u8(&mut payload, 0); // status
        push_u32(&mut payload, 0); // tail
        assert_eq!(
            decode(&raw_frame(KIND_BLOCK_DONE, &payload)),
            Err(WireError::Malformed)
        );
        // Out-of-range (≥ WIRE_MAX_INSTANCES, ≠ NONE) is equally
        // rejected on doorbells.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, WIRE_MAX_INSTANCES);
        assert_eq!(
            decode(&raw_frame(KIND_BLOCK_IN, &payload)),
            Err(WireError::Malformed)
        );
    }

    #[test]
    fn create_instance_payload_bounds_are_enforced() {
        // Zero depth on either side → Malformed.
        for (in_depth, out_depth) in [(0_u32, 4_u32), (4, 0)] {
            let mut payload = Vec::new();
            push_u64(&mut payload, 1);
            push_u32(&mut payload, 0); // instance
            push_u8(&mut payload, 10); // CreateInstance
            push_str(&mut payload, "id");
            push_str(&mut payload, "key");
            push_u32(&mut payload, in_depth);
            push_u32(&mut payload, out_depth);
            assert_eq!(
                decode(&raw_frame(KIND_COMMAND, &payload)),
                Err(WireError::Malformed)
            );
        }
        // Over-bound depth → LimitExceeded.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u8(&mut payload, 10);
        push_str(&mut payload, "id");
        push_str(&mut payload, "key");
        push_u32(&mut payload, WIRE_MAX_ELEMENTS + 1);
        push_u32(&mut payload, 4);
        assert_eq!(
            decode(&raw_frame(KIND_COMMAND, &payload)),
            Err(WireError::LimitExceeded)
        );
        // Over-cap string → LimitExceeded, not an allocation.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u8(&mut payload, 10);
        push_u32(&mut payload, WIRE_MAX_STRING_BYTES + 1);
        assert_eq!(
            decode(&raw_frame(KIND_COMMAND, &payload)),
            Err(WireError::LimitExceeded)
        );
    }

    #[test]
    fn frame_layout_is_little_endian_and_length_prefixed() {
        // Hello{version: WIRE_VERSION, capabilities: 3} → len 6,
        // kind 0, LE fields.
        let mut frame = Vec::new();
        encode(
            &Message::Hello {
                version: WIRE_VERSION,
                capabilities: 3,
            },
            &mut frame,
        )
        .unwrap();
        let mut expected = vec![6, 0, 0, 0, KIND_HELLO];
        expected.extend_from_slice(&WIRE_VERSION.to_le_bytes());
        expected.extend_from_slice(&3_u32.to_le_bytes());
        assert_eq!(frame, expected);
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
        encode(
            &Message::BlockIn {
                seq: 1,
                instance: 0,
            },
            &mut buf,
        )
        .unwrap();
        // First frame is exactly 5 bytes; the rest is the second frame.
        assert_eq!(decode(&buf[..WIRE_FRAME_HEADER_LEN]), Ok(Message::Shutdown));
        assert_eq!(
            decode(&buf[WIRE_FRAME_HEADER_LEN..]),
            Ok(Message::BlockIn {
                seq: 1,
                instance: 0
            })
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
        push_u32(&mut payload, 0); // instance
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
        push_u32(&mut command_bytes, WIRE_MAX_STATE_BYTES + 1);
        let mut command_reader = Reader::new(&command_bytes);
        assert_eq!(
            decode_command(7, &mut command_reader), // RestoreState
            Err(WireError::LimitExceeded)
        );
    }

    #[test]
    fn decode_rejects_impossible_counts_without_allocating() {
        // A count within the cap but unsatisfiable by the remaining
        // bytes is truncated before any elements are read.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0); // instance
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
        // Unknown Command verb tag — seq(8) + instance(4) + tag.
        assert_eq!(
            decode(&[
                13,
                0,
                0,
                0,
                KIND_COMMAND,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                200
            ]),
            Err(WireError::Malformed)
        );
        // Unknown Reply result tag (not REPLY_OK/REPLY_ERR).
        assert_eq!(
            decode(&[
                13, 0, 0, 0, KIND_REPLY, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2
            ]),
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
            HostError::InstanceLimit,
            HostError::InvalidValue,
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
            instance: 0,
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
            instance: 0,
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

    /// The `Ok(false)`/`Err` split is load-bearing: the export runner
    /// maps a clean EOF at a frame boundary to a normal exit and any
    /// `Err` to a transport-violation exit, so the taxonomy is pinned
    /// here at the wire layer that owns it.
    #[test]
    fn read_frame_distinguishes_clean_eof_from_error_kinds() {
        let mut buf = Vec::new();

        // Zero bytes pending where a length field would start — a
        // clean EOF at a frame boundary.
        let mut cursor = io::Cursor::new(Vec::new());
        assert!(!read_frame(&mut cursor, &mut buf).unwrap());

        // A partial length field is a mid-header cut —
        // `UnexpectedEof`, never clean EOF.
        for stream in [&[0x01_u8][..], &[0x01, 0x02, 0x03][..]] {
            let mut cursor = io::Cursor::new(stream.to_vec());
            assert_eq!(
                read_frame(&mut cursor, &mut buf).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
            );
        }

        // A complete header declaring a `payload_len` over the cap is
        // `InvalidData` — rejected before any body read/allocation.
        let mut cursor = io::Cursor::new((WIRE_MAX_PAYLOAD + 1).to_le_bytes().to_vec());
        assert_eq!(
            read_frame(&mut cursor, &mut buf).unwrap_err().kind(),
            io::ErrorKind::InvalidData,
        );

        // A complete header whose body never arrives is `UnexpectedEof`.
        let mut frame = 1_u32.to_le_bytes().to_vec();
        frame.push(0); // kind byte only — the declared payload is missing
        let mut cursor = io::Cursor::new(frame);
        assert_eq!(
            read_frame(&mut cursor, &mut buf).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof,
        );
    }

    // --- wire v3: video verbs and replies (Issue #118) -------------------

    fn video_config(format: PixelFormat, width: u32, height: u32) -> VideoProcessConfig {
        VideoProcessConfig::new(format, width, height, FrameRate::new(30, 1).unwrap()).unwrap()
    }

    /// A canonical-packed 1920×1080 `Yuv420p` descriptor — luma
    /// 1920×1080, two 960×540 chroma planes contiguous from offset 0.
    fn video_desc() -> VideoFrameDesc {
        VideoFrameDesc {
            format: PixelFormat::Yuv420p,
            width: 1_920,
            height: 1_080,
            planes: vec![
                VideoFramePlane {
                    offset: 0,
                    len: 2_073_600,
                    stride: 1_920,
                },
                VideoFramePlane {
                    offset: 2_073_600,
                    len: 518_400,
                    stride: 960,
                },
                VideoFramePlane {
                    offset: 2_592_000,
                    len: 518_400,
                    stride: 960,
                },
            ],
            pts: MediaTime::new(90_000, TimeBase::COMMON_90K),
            duration: Some(MediaDuration::new(3_000, TimeBase::COMMON_90K)),
            color: ColorInfo {
                primaries: ColorPrimaries::Bt709,
                transfer: TransferCharacteristics::Bt709,
                matrix: MatrixCoefficients::Bt709,
                range: ColorRange::Limited,
                hdr: None,
            },
        }
    }

    fn param_info() -> PlainParamInfoOwned {
        PlainParamInfoOwned {
            id: ParamId(3),
            name: "Threshold".to_owned(),
            unit: "dB".to_owned(),
            min_value: -60.0,
            max_value: 12.0,
            display_min_value: -60.0,
            display_max_value: 12.0,
            default_value: PlainParamValue::Double(0.0),
            flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
            choices: Vec::new(),
        }
    }

    #[test]
    fn wire_version_is_v3() {
        assert_eq!(WIRE_VERSION, 3);
    }

    #[test]
    fn video_verbs_roundtrip() {
        // Every video verb is instance-scoped — the envelope carries
        // a live handle for all of them.
        for cmd in [
            Command::CreateVideoInstance {
                plugin_id: "org.ruviform.test.video".to_owned(),
                shm_key: "rvf-test-video-segment".to_owned(),
                config: video_config(PixelFormat::Rgba8, 1_920, 1_080),
            },
            Command::VideoInitialize,
            Command::VideoActivate,
            Command::VideoStartProcessing,
            Command::VideoRender {
                at: MediaTime::new(123, TimeBase::COMMON_90K),
                input: video_desc(),
            },
            Command::VideoStopProcessing,
            Command::VideoDeactivate,
            Command::VideoGetParams,
            Command::VideoGetParamInfo { id: 7 },
            Command::VideoGetParamValue { id: 7 },
            Command::VideoSetParamValue {
                id: 7,
                value: PlainParamValue::Choice(2),
            },
        ] {
            let message = Message::Command {
                seq: 7,
                instance: 0,
                cmd,
            };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn video_replies_roundtrip() {
        for reply in [
            Reply::Rendered {
                output: video_desc(),
            },
            Reply::Params {
                ids: vec![0, 3, u32::MAX],
            },
            Reply::ParamInfo { info: None },
            Reply::ParamInfo {
                info: Some(param_info()),
            },
            Reply::ParamValue {
                value: PlainParamValue::Double(-12.5),
            },
        ] {
            let message = Message::Reply {
                seq: 7,
                instance: 3,
                result: Ok(reply),
            };
            assert_eq!(roundtrip(&message), message);
        }
    }

    #[test]
    fn video_reply_tags_and_presence_are_strict() {
        // `ParamInfo`'s `Option` tag is {0,1}-strict like every other
        // presence tag — corrupt a live encoding at the tag byte
        // (len 4 + kind 1 + seq 8 + instance 4 + REPLY_OK 1 + tag 6).
        let message = Message::Reply {
            seq: 7,
            instance: 3,
            result: Ok(Reply::ParamInfo {
                info: Some(param_info()),
            }),
        };
        let mut frame = Vec::new();
        encode(&message, &mut frame).unwrap();
        let presence_at = 4 + 1 + 8 + 4 + 1 + 1;
        assert_eq!(frame[presence_at], 1);
        for tag in [2_u8, 0xFF] {
            frame[presence_at] = tag;
            assert_eq!(
                decode(&frame),
                Err(WireError::Malformed),
                "ParamInfo presence tag {tag} must be rejected",
            );
        }
        // Reply tags > 7 are outside the v3 vocabulary.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u8(&mut payload, REPLY_OK);
        push_u8(&mut payload, 8);
        assert_eq!(
            decode(&raw_frame(KIND_REPLY, &payload)),
            Err(WireError::Malformed)
        );
    }

    #[test]
    fn video_verbs_are_instance_scoped() {
        // `WIRE_INSTANCE_NONE` is a scope violation for every video
        // verb — checked before the verb payload is interpreted, so
        // an empty body still reports the scope error.
        for tag in 12..=22_u8 {
            assert_eq!(
                decode(&raw_frame(
                    KIND_COMMAND,
                    &command_payload(1, WIRE_INSTANCE_NONE, tag)
                )),
                Err(WireError::Malformed),
                "video verb {tag} must reject WIRE_INSTANCE_NONE",
            );
            // Out-of-range instance values are the same defined
            // rejection.
            assert_eq!(
                decode(&raw_frame(
                    KIND_COMMAND,
                    &command_payload(1, WIRE_MAX_INSTANCES, tag)
                )),
                Err(WireError::Malformed),
                "video verb {tag} must reject out-of-range instances",
            );
        }
    }

    // --- wire v3: VideoProcessConfig -------------------------------------

    /// `field_roundtrip` for the fallible encoders — the video codecs
    /// can report `UnsupportedVariant`.
    fn fallible_field_roundtrip<T>(
        value: &T,
        encode: impl Fn(&T, &mut Vec<u8>) -> Result<(), WireError>,
        decode: impl Fn(&mut Reader<'_>) -> Result<T, WireError>,
    ) -> T
    where
        T: PartialEq + fmt::Debug,
    {
        let mut payload = Vec::new();
        encode(value, &mut payload).unwrap();
        let mut reader = Reader::new(&payload);
        let decoded = decode(&mut reader).unwrap();
        assert!(reader.is_empty(), "trailing bytes after field decode");
        decoded
    }

    /// A `VideoProcessConfig` payload: `u8` format + `u32` width +
    /// `u32` height + `u32` numerator + `u32` denominator.
    fn video_config_payload(format: u8, width: u32, height: u32, num: u32, den: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        push_u8(&mut payload, format);
        push_u32(&mut payload, width);
        push_u32(&mut payload, height);
        push_u32(&mut payload, num);
        push_u32(&mut payload, den);
        payload
    }

    #[test]
    fn video_process_config_codec_roundtrip_and_bounds() {
        // The 17 B fixed layout and per-format tags.
        let config = video_config(PixelFormat::Yuv420p, 1_920, 1_080);
        let mut payload = Vec::new();
        encode_video_process_config(&config, &mut payload).unwrap();
        assert_eq!(payload.len(), 17);
        assert_eq!(
            fallible_field_roundtrip(
                &config,
                encode_video_process_config,
                decode_video_process_config
            ),
            config
        );
        for format in [
            PixelFormat::Yuv420p,
            PixelFormat::Yuv422p,
            PixelFormat::Yuv444p,
            PixelFormat::Rgba8,
            PixelFormat::RgbaF16,
            PixelFormat::RgbaF32,
        ] {
            let config = video_config(format, 320, 240);
            assert_eq!(
                fallible_field_roundtrip(
                    &config,
                    encode_video_process_config,
                    decode_video_process_config
                ),
                config
            );
        }

        // Unassigned format tag → Malformed.
        let payload = video_config_payload(6, 320, 240, 30, 1);
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_video_process_config(&mut reader),
            Err(WireError::Malformed)
        );
        // Zero dims → Malformed (ctor invariant); over
        // `WIRE_VIDEO_MAX_DIM` → LimitExceeded.
        for (width, height, error) in [
            (0, 240, WireError::Malformed),
            (320, 0, WireError::Malformed),
            (WIRE_VIDEO_MAX_DIM + 1, 240, WireError::LimitExceeded),
            (320, WIRE_VIDEO_MAX_DIM + 1, WireError::LimitExceeded),
        ] {
            let payload = video_config_payload(3, width, height, 30, 1);
            let mut reader = Reader::new(&payload);
            assert_eq!(
                decode_video_process_config(&mut reader),
                Err(error),
                "dims {width}x{height} must be {error:?}",
            );
        }
        // `WIRE_VIDEO_MAX_DIM` itself is legal.
        let payload = video_config_payload(3, WIRE_VIDEO_MAX_DIM, WIRE_VIDEO_MAX_DIM, 30, 1);
        let mut reader = Reader::new(&payload);
        assert!(decode_video_process_config(&mut reader).is_ok());
        // Zero FrameRate components → Malformed.
        for (num, den) in [(0_u32, 1_u32), (30, 0), (0, 0)] {
            let payload = video_config_payload(3, 320, 240, num, den);
            let mut reader = Reader::new(&payload);
            assert_eq!(
                decode_video_process_config(&mut reader),
                Err(WireError::Malformed),
                "frame rate {num}/{den} must be rejected",
            );
        }
    }

    // --- wire v3: VideoFrameDesc -------------------------------------------

    /// Re-encodes `desc`, splices `edit` into the payload bytes and
    /// decodes — the rejection tests need encodings the honest
    /// encoder cannot produce.
    fn decode_edited_desc(
        desc: &VideoFrameDesc,
        edit: impl Fn(&mut Vec<u8>),
    ) -> Result<VideoFrameDesc, WireError> {
        let mut payload = Vec::new();
        encode_video_frame_desc(desc, &mut payload).unwrap();
        edit(&mut payload);
        let mut reader = Reader::new(&payload);
        decode_video_frame_desc(&mut reader)
    }

    #[test]
    fn video_frame_desc_codec_roundtrip_and_bounds() {
        let desc = video_desc();
        assert_eq!(
            fallible_field_roundtrip(&desc, encode_video_frame_desc, decode_video_frame_desc),
            desc
        );
        // A one-plane format with `duration: None` round-trips too.
        let mut single = desc.clone();
        single.format = PixelFormat::RgbaF32;
        single.width = 64;
        single.height = 64;
        single.planes = vec![VideoFramePlane {
            offset: 0,
            len: 64 * 64 * 16,
            stride: 64 * 16,
        }];
        single.duration = None;
        assert_eq!(
            fallible_field_roundtrip(&single, encode_video_frame_desc, decode_video_frame_desc),
            single
        );

        // Format tag > 5 → Malformed.
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[0] = 6),
            Err(WireError::Malformed)
        );
        // Dims: 0 → Malformed, over `WIRE_VIDEO_MAX_DIM` →
        // LimitExceeded (width is at bytes 1..5, height at 5..9).
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[1..5]
                .copy_from_slice(&0_u32.to_le_bytes())),
            Err(WireError::Malformed)
        );
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[5..9]
                .copy_from_slice(&(WIRE_VIDEO_MAX_DIM + 1).to_le_bytes())),
            Err(WireError::LimitExceeded)
        );
        // `plane_count` (byte 9): 0 → Malformed, 5 → LimitExceeded,
        // mismatch (1 vs Yuv420p's 3) → Malformed.
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[9] = 0),
            Err(WireError::Malformed)
        );
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[9] = 5),
            Err(WireError::LimitExceeded)
        );
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[9] = 1),
            Err(WireError::Malformed)
        );
        // Stride over `WIRE_VIDEO_MAX_STRIDE` → LimitExceeded (first
        // plane's stride sits at bytes 10 + 8..12).
        assert_eq!(
            decode_edited_desc(&desc, |payload| payload[18..22]
                .copy_from_slice(&(WIRE_VIDEO_MAX_STRIDE + 1).to_le_bytes())),
            Err(WireError::LimitExceeded)
        );
        // `has_duration` (byte 10 header + 3×12 planes + 16 pts = 62)
        // is a strict {0,1} presence tag.
        for tag in [2_u8, 0xFF] {
            assert_eq!(
                decode_edited_desc(&desc, |payload| payload[62] = tag),
                Err(WireError::Malformed),
                "has_duration {tag} must be rejected",
            );
        }
        // The codec bounds, not semantics: non-canonical offsets are
        // accepted (the runner/host layer verifies canonicality).
        assert!(
            decode_edited_desc(&desc, |payload| payload[10..14]
                .copy_from_slice(&123_u32.to_le_bytes()))
            .is_ok()
        );
    }

    // --- wire v3: ColorInfo -------------------------------------------------

    #[test]
    fn color_info_codec_roundtrip_and_strict_presence() {
        for color in [
            ColorInfo::default(),
            video_desc().color,
            ColorInfo {
                primaries: ColorPrimaries::Bt2020,
                transfer: TransferCharacteristics::Pq,
                matrix: MatrixCoefficients::Bt2020,
                range: ColorRange::Full,
                hdr: Some(HdrMetadata {
                    mastering_display: Some(MasteringDisplayInfo {
                        primaries: [
                            Chromaticity {
                                x: 34_000,
                                y: 16_000,
                            },
                            Chromaticity {
                                x: 7_500,
                                y: 30_000,
                            },
                            Chromaticity {
                                x: 15_000,
                                y: 6_000,
                            },
                        ],
                        white_point: Chromaticity {
                            x: 15_635,
                            y: 16_450,
                        },
                        max_luminance: 10_000_000,
                        min_luminance: 50,
                    }),
                    content_light_level: Some(ContentLightLevel {
                        max_cll: 1_000,
                        max_fall: 400,
                    }),
                }),
            },
            // `hdr` present with only `content_light_level` set —
            // the presence flags are independent.
            ColorInfo {
                primaries: ColorPrimaries::P3D65,
                transfer: TransferCharacteristics::Hlg,
                matrix: MatrixCoefficients::Identity,
                range: ColorRange::Unspecified,
                hdr: Some(HdrMetadata {
                    mastering_display: None,
                    content_light_level: Some(ContentLightLevel {
                        max_cll: 400,
                        max_fall: 100,
                    }),
                }),
            },
        ] {
            assert_eq!(
                fallible_field_roundtrip(&color, encode_color_info, decode_color_info),
                color
            );
        }

        // Every presence byte is strict {0,1} — byte 4 is
        // `has_hdr`; inside an `hdr: Some` payload byte 5 is
        // `has_mastering` and byte 30 (after the 24-byte mastering
        // block) is `has_cll`.
        let color = ColorInfo {
            hdr: Some(HdrMetadata {
                mastering_display: Some(MasteringDisplayInfo {
                    primaries: [
                        Chromaticity { x: 1, y: 2 },
                        Chromaticity { x: 3, y: 4 },
                        Chromaticity { x: 5, y: 6 },
                    ],
                    white_point: Chromaticity { x: 7, y: 8 },
                    max_luminance: 9,
                    min_luminance: 10,
                }),
                content_light_level: Some(ContentLightLevel {
                    max_cll: 11,
                    max_fall: 12,
                }),
            }),
            ..ColorInfo::default()
        };
        let mut payload = Vec::new();
        encode_color_info(&color, &mut payload).unwrap();
        for (index, name) in [(4_usize, "has_hdr"), (5, "has_mastering"), (30, "has_cll")] {
            for tag in [2_u8, 0xFF] {
                let mut corrupted = payload.clone();
                corrupted[index] = tag;
                let mut reader = Reader::new(&corrupted);
                assert_eq!(
                    decode_color_info(&mut reader),
                    Err(WireError::Malformed),
                    "{name} = {tag} must be rejected",
                );
            }
        }
        // Unassigned color enum tags → Malformed.
        for index in 0..4_usize {
            let mut corrupted = payload.clone();
            corrupted[index] = 9;
            let mut reader = Reader::new(&corrupted);
            assert_eq!(
                decode_color_info(&mut reader),
                Err(WireError::Malformed),
                "color enum tag at byte {index} must be bounded",
            );
        }
    }

    // --- wire v3: PlainParamValue / PlainParamInfoOwned ----------------------

    #[test]
    fn plain_param_value_codec_roundtrip_and_rejections() {
        for value in [
            PlainParamValue::Double(-12.5),
            PlainParamValue::Integer(i32::MIN),
            PlainParamValue::Boolean(true),
            PlainParamValue::Boolean(false),
            PlainParamValue::Choice(u32::MAX),
        ] {
            assert_eq!(
                field_roundtrip(&value, encode_plain_param_value, decode_plain_param_value),
                value
            );
        }
        // Non-finite `Double` — NaN and both infinities — is
        // Malformed; a parameter value is always finite.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut payload = vec![0_u8];
            push_f64(&mut payload, bad);
            let mut reader = Reader::new(&payload);
            assert_eq!(
                decode_plain_param_value(&mut reader),
                Err(WireError::Malformed),
                "non-finite {bad} must be rejected",
            );
        }
        // `Boolean` is a strict {0,1} byte, not a C-style bool.
        for tag in [2_u8, 0xFF] {
            let payload = [2, tag];
            let mut reader = Reader::new(&payload);
            assert_eq!(
                decode_plain_param_value(&mut reader),
                Err(WireError::Malformed),
                "Boolean {tag} must be rejected",
            );
        }
        // Unassigned kind tag.
        let mut reader = Reader::new(&[4]);
        assert_eq!(
            decode_plain_param_value(&mut reader),
            Err(WireError::Malformed)
        );
    }

    /// A `PlainParamInfoOwned` payload built field-by-field — the
    /// rejection tests need encodings the honest encoder cannot
    /// produce.
    fn param_info_payload(info: &PlainParamInfoOwned) -> Vec<u8> {
        let mut payload = Vec::new();
        encode_plain_param_info(info, &mut payload);
        payload
    }

    #[test]
    fn plain_param_info_owned_codec_roundtrip_and_contract() {
        for info in [
            param_info(),
            // A `Choice` default with its choice list.
            PlainParamInfoOwned {
                id: ParamId(9),
                name: "Mode".to_owned(),
                unit: String::new(),
                min_value: 0.0,
                max_value: 2.0,
                display_min_value: 0.0,
                display_max_value: 2.0,
                default_value: PlainParamValue::Choice(1),
                flags: ParamFlags::NONE,
                choices: vec!["off".to_owned(), "half".to_owned(), "full".to_owned()],
            },
            // `display_*` are UI hints: non-finite and inverted
            // values are legal there even though `min`/`max` are
            // strict.
            PlainParamInfoOwned {
                display_min_value: f64::INFINITY,
                display_max_value: f64::NEG_INFINITY,
                ..param_info()
            },
        ] {
            assert_eq!(
                field_roundtrip(&info, encode_plain_param_info, decode_plain_param_info),
                info
            );
        }
    }

    #[test]
    fn plain_param_info_owned_decode_enforces_the_contract() {
        let base = param_info();
        // The f64 fields sit after `u32` id + two strings: offset of
        // `min_value` = 4 + 4 + name.len() + 4 + unit.len().
        let min_off = 4 + 4 + base.name.len() + 4 + base.unit.len();
        let offsets = [
            ("min", min_off),
            ("max", min_off + 8),
            ("default", min_off + 8 * 4 + 1), // f64 inside the Double(0) tag+payload
        ];
        // Non-finite min/max → Malformed.
        for (name, offset) in &offsets[..2] {
            for bad in [f64::NAN, f64::INFINITY] {
                let mut payload = param_info_payload(&base);
                payload[*offset..*offset + 8].copy_from_slice(&bad.to_le_bytes());
                let mut reader = Reader::new(&payload);
                assert_eq!(
                    decode_plain_param_info(&mut reader),
                    Err(WireError::Malformed),
                    "non-finite {name} must be rejected",
                );
            }
        }
        // Non-finite default scalar → Malformed (tagged Double at
        // `default - 1`, f64 at `default`).
        for bad in [f64::NAN, f64::NEG_INFINITY] {
            let mut payload = param_info_payload(&base);
            payload[offsets[2].1..offsets[2].1 + 8].copy_from_slice(&bad.to_le_bytes());
            let mut reader = Reader::new(&payload);
            assert_eq!(
                decode_plain_param_info(&mut reader),
                Err(WireError::Malformed),
                "non-finite default must be rejected",
            );
        }
        // min > max → Malformed.
        let mut payload = param_info_payload(&base);
        payload[min_off..min_off + 8].copy_from_slice(&100.0_f64.to_le_bytes());
        payload[min_off + 8..min_off + 16].copy_from_slice(&(-100.0_f64).to_le_bytes());
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_plain_param_info(&mut reader),
            Err(WireError::Malformed)
        );
        // Default outside [min, max] → Malformed.
        let mut payload = param_info_payload(&base);
        payload[offsets[2].1..offsets[2].1 + 8].copy_from_slice(&500.0_f64.to_le_bytes());
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_plain_param_info(&mut reader),
            Err(WireError::Malformed)
        );
        // Unknown `flags` bit → Malformed (reserved-field strictness).
        let flags_off = min_off + 8 * 4 + 9; // after default Double(9 B)
        let mut payload = param_info_payload(&base);
        payload[flags_off..flags_off + 4].copy_from_slice(&(1_u32 << 31).to_le_bytes());
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_plain_param_info(&mut reader),
            Err(WireError::Malformed)
        );
        // Choice-list contract: a `Choice` default must index a
        // non-empty list; a non-Choice default must have none.
        let mut empty_choices = PlainParamInfoOwned {
            default_value: PlainParamValue::Choice(0),
            choices: Vec::new(),
            min_value: 0.0,
            max_value: 1.0,
            ..param_info()
        };
        assert_eq!(
            decode_plain_param_info(&mut Reader::new(&param_info_payload(&empty_choices))),
            Err(WireError::Malformed)
        );
        empty_choices.choices = vec!["a".to_owned()];
        empty_choices.default_value = PlainParamValue::Choice(1); // out of range
        assert_eq!(
            decode_plain_param_info(&mut Reader::new(&param_info_payload(&empty_choices))),
            Err(WireError::Malformed)
        );
        let stray_choices = PlainParamInfoOwned {
            default_value: PlainParamValue::Double(0.0),
            choices: vec!["a".to_owned()],
            ..param_info()
        };
        assert_eq!(
            decode_plain_param_info(&mut Reader::new(&param_info_payload(&stray_choices))),
            Err(WireError::Malformed)
        );
        // Choice-list count over `WIRE_MAX_ELEMENTS` → LimitExceeded
        // before any allocation.
        let mut payload = param_info_payload(&base);
        let count_off = flags_off + 4;
        payload.truncate(count_off);
        push_u32(&mut payload, WIRE_MAX_ELEMENTS + 1);
        let mut reader = Reader::new(&payload);
        assert_eq!(
            decode_plain_param_info(&mut reader),
            Err(WireError::LimitExceeded)
        );
    }

    // --- wire v3: HostError::InvalidValue -----------------------------------

    #[test]
    fn invalid_value_roundtrips_as_wire_tag_10() {
        // Field-level: tag 10 both directions — not the
        // `UnsupportedVariant`/`Malformed` wildcard arms.
        let mut payload = Vec::new();
        encode_host_error(&HostError::InvalidValue, &mut payload).unwrap();
        assert_eq!(payload, [10]);
        assert_eq!(
            decode_host_error(&mut Reader::new(&payload)),
            Ok(HostError::InvalidValue)
        );
        // Frame-level: a `Reply{Err(InvalidValue)}` round-trips
        // through encode→decode unchanged.
        let message = Message::Reply {
            seq: 7,
            instance: 3,
            result: Err(HostError::InvalidValue),
        };
        assert_eq!(roundtrip(&message), message);
    }

    // --- wire v3: vshm segment -----------------------------------------------

    /// The `vshm` layout constants form one chain — same rule as the
    /// audio `shm` layout: every offset is derived, so the host-side
    /// owner and the runner-side attach compute the same segment
    /// byte-for-byte.
    #[test]
    fn vshm_header_layout_is_self_consistent() {
        assert_eq!(vshm::MAGIC, u32::from_le_bytes(*b"RVLV"));
        assert_eq!(vshm::OFF_MAGIC + 4, vshm::OFF_WIRE_VERSION);
        assert_eq!(vshm::OFF_WIRE_VERSION + 4, vshm::OFF_IN_OFF);
        assert_eq!(vshm::OFF_IN_OFF + 4, vshm::OFF_IN_BYTES);
        assert_eq!(vshm::OFF_IN_BYTES + 4, vshm::OFF_OUT_OFF);
        assert_eq!(vshm::OFF_OUT_OFF + 4, vshm::OFF_OUT_BYTES);
        assert_eq!(vshm::OFF_OUT_BYTES + 4, vshm::OFF_RESERVED);
        assert_eq!(vshm::OFF_RESERVED + 4, vshm::HEADER_LEN);
        assert_eq!(vshm::HEADER_LEN, 28);
        // The segment cap is exactly header + two maximal regions.
        const {
            assert!(WIRE_VIDEO_MAX_SEGMENT_BYTES == 28 + 2 * WIRE_VIDEO_MAX_FRAME_BYTES);
        }
    }

    #[test]
    fn vshm_frame_bytes_and_segment_len_math() {
        // The spec's pinned byte-math examples.
        assert_eq!(
            vshm::frame_bytes(&video_config(PixelFormat::Rgba8, 3_840, 2_160)),
            33_177_600
        );
        assert_eq!(
            vshm::frame_bytes(&video_config(PixelFormat::Yuv420p, 3_840, 2_160)),
            12_441_600
        );
        assert_eq!(
            vshm::frame_bytes(&video_config(PixelFormat::Rgba8, 1_920, 1_080)),
            8_294_400
        );
        // Odd dims: chroma planes ceil-divide (Yuv420p 3×3 →
        // luma 3×3 + two 2×2).
        assert_eq!(
            vshm::frame_bytes(&video_config(PixelFormat::Yuv420p, 3, 3)),
            9 + 4 + 4
        );
        // 8K `RgbaF16` fits the frame cap; its segment fits the
        // segment cap. 8K `RgbaF32` honestly exceeds both.
        let f16 = vshm::frame_bytes(&video_config(PixelFormat::RgbaF16, 7_680, 4_320));
        assert_eq!(f16, 265_420_800);
        assert!(f16 <= WIRE_VIDEO_MAX_FRAME_BYTES);
        assert_eq!(vshm::segment_len(f16), 530_841_628);
        assert!(vshm::segment_len(f16) <= WIRE_VIDEO_MAX_SEGMENT_BYTES);
        let f32 = vshm::frame_bytes(&video_config(PixelFormat::RgbaF32, 7_680, 4_320));
        assert!(f32 > WIRE_VIDEO_MAX_FRAME_BYTES);
        assert!(vshm::segment_len(f32) > WIRE_VIDEO_MAX_SEGMENT_BYTES);
        // `segment_len` shape and saturation — never a panic.
        assert_eq!(vshm::segment_len(100), 28 + 200);
        assert_eq!(vshm::segment_len(u64::MAX), u64::MAX);
        // Overflow in `frame_bytes` saturates over the cap instead of
        // wrapping — `VideoProcessConfig::new` admits huge dims, so
        // the math must stay honest without a bound.
        let huge = VideoProcessConfig::new(
            PixelFormat::RgbaF32,
            u32::MAX,
            u32::MAX,
            FrameRate::new(30, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(vshm::frame_bytes(&huge), u64::MAX);
        assert!(vshm::frame_bytes(&huge) > WIRE_VIDEO_MAX_FRAME_BYTES);

        // `plane_layout` — canonical packing: `stride = row_bytes`,
        // `offset(p) = Σ len(0..p)`.
        assert_eq!(
            vshm::plane_layout(PixelFormat::Yuv420p, 1_920, 1_080, 99),
            None
        );
        let chroma = vshm::plane_layout(PixelFormat::Yuv420p, 1_920, 1_080, 1).unwrap();
        assert_eq!(chroma.width, 960);
        assert_eq!(chroma.height, 540);
        assert_eq!(chroma.stride, 960);
        assert_eq!(chroma.len, 518_400);
        assert_eq!(chroma.offset, 2_073_600);
        let luma = vshm::plane_layout(PixelFormat::Yuv420p, 1_920, 1_080, 0).unwrap();
        assert_eq!((luma.offset, luma.stride, luma.len), (0, 1_920, 2_073_600));
    }

    #[test]
    fn vshm_header_write_and_validate() {
        // A canonical header validates and returns the declared
        // region sizes.
        let in_bytes = 1_000_u32;
        let out_bytes = 2_000_u32;
        let header = vshm::header_bytes(in_bytes, out_bytes);
        assert_eq!(header.len(), vshm::HEADER_LEN);
        // A mapped segment = header + regions (possibly longer).
        let mut segment = vec![0_u8; 28 + 1_000 + 2_000 + 64];
        segment[..28].copy_from_slice(&header);
        assert_eq!(vshm::validate_header(&segment), Some((in_bytes, out_bytes)));
        // Exactly `out_off + out_bytes` long also validates (the
        // coverage check is `>=`, not `==`).
        assert_eq!(
            vshm::validate_header(&segment[..28 + 1_000 + 2_000]),
            Some((in_bytes, out_bytes))
        );
        // A mapping shorter than the header itself, or shorter than
        // the declared `out` extent.
        assert_eq!(vshm::validate_header(&header[..27]), None);
        assert_eq!(
            vshm::validate_header(&segment[..28 + 1_000 + 2_000 - 1]),
            None
        );

        // Every declared header mutation is a defined rejection —
        // each case rewrites one `u32` field.
        for (name, offset, field) in [
            ("bad magic", vshm::OFF_MAGIC, *b"XXXX"),
            (
                "stale wire_version",
                vshm::OFF_WIRE_VERSION,
                2_u32.to_le_bytes(),
            ),
            ("in_off != 28", vshm::OFF_IN_OFF, 32_u32.to_le_bytes()),
            (
                "out_off != 28 + in_bytes",
                vshm::OFF_OUT_OFF,
                2_000_u32.to_le_bytes(),
            ),
            ("nonzero reserved", vshm::OFF_RESERVED, 1_u32.to_le_bytes()),
            (
                "declared out exceeds u32",
                vshm::OFF_OUT_BYTES,
                u32::MAX.to_le_bytes(),
            ),
        ] {
            let mut segment = vec![0_u8; 28 + 1_000 + 2_000];
            segment[..28].copy_from_slice(&header);
            segment[offset..offset + 4].copy_from_slice(&field);
            assert_eq!(
                vshm::validate_header(&segment),
                None,
                "{name} must reject the header",
            );
        }
    }

    // --- wire v3: Hello ----------------------------------------------------

    #[test]
    fn hello_v3_keeps_version_verbatim_and_capabilities_reserved() {
        // `version` is carried verbatim — the handshake, not the
        // codec, compares it against `WIRE_VERSION`. A v2 hello
        // decodes as version 2 (≠ 3 → handshake-fails downstream).
        assert_eq!(
            roundtrip(&Message::Hello {
                version: 2,
                capabilities: 0
            }),
            Message::Hello {
                version: 2,
                capabilities: 0
            }
        );
        // `capabilities` stays reserved under v3.
        let mut frame = Vec::new();
        encode(
            &Message::Hello {
                version: WIRE_VERSION,
                capabilities: 1,
            },
            &mut frame,
        )
        .unwrap();
        assert_eq!(decode(&frame), Err(WireError::Malformed));
    }

    // --- wire v3: bounded decode -----------------------------------------------

    #[test]
    fn video_decode_is_bounded_against_adversarial_payloads() {
        // `CreateVideoInstance` with an over-cap `plugin_id` length —
        // rejected before any allocation.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u8(&mut payload, 12);
        push_u32(&mut payload, WIRE_MAX_STRING_BYTES + 1);
        assert_eq!(
            decode(&raw_frame(KIND_COMMAND, &payload)),
            Err(WireError::LimitExceeded)
        );
        // `Params` reply with an id count over `WIRE_MAX_ELEMENTS`.
        let mut payload = Vec::new();
        push_u64(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u8(&mut payload, REPLY_OK);
        push_u8(&mut payload, 5);
        push_u32(&mut payload, WIRE_MAX_ELEMENTS + 1);
        assert_eq!(
            decode(&raw_frame(KIND_REPLY, &payload)),
            Err(WireError::LimitExceeded)
        );
        // A `VideoFrameDesc` truncated mid-planes is `Truncated`,
        // never a panic or partial decode.
        let desc = video_desc();
        let mut command = command_payload(1, 0, 16);
        encode_media_time(&desc.pts, &mut command);
        let mut desc_bytes = Vec::new();
        encode_video_frame_desc(&desc, &mut desc_bytes).unwrap();
        // Cut inside the second plane record (10 B header + 15 B).
        command.extend_from_slice(&desc_bytes[..10 + 15]);
        assert_eq!(
            decode(&raw_frame(KIND_COMMAND, &command)),
            Err(WireError::Truncated)
        );
    }
}
