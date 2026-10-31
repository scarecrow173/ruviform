//! [`Packet`] — one encoded unit crossing the demux ↔ codec boundary —
//! and [`PacketFlags`], its encoded-unit attribute set.
//!
//! A `Packet` is an owned value type: it references its container stream
//! via [`StreamId`], carries the timestamps the container supplied
//! (raw [`TickTimestamp`] presentation/decode ticks plus their
//! provenance, an optional tick `duration`) and owns its encoded payload
//! bytes. Borrowed / zero-copy packet backing is deliberately deferred
//! (spec Deferred Design Concerns; Issue #26 family).

use ruviform_media_core::{StreamId, TickTimestamp};

/// Encoded-unit attribute flags of a [`Packet`] — a `Copy` bit set.
///
/// Modeled on the `SpeakerMask` pattern in `ruviform-audio-core`: a
/// `u32` newtype with `const` constructors and bit-set queries. More
/// flag constants may be added without breaking users of
/// [`PacketFlags::from_bits`]/[`PacketFlags::bits`].
///
/// **RT contract:** non-RT only — part of the media-I/O boundary
/// (pure `Copy` value type; the contract regulates the API context it
/// is used in, not the value itself).
///
/// # Examples
///
/// ```
/// use ruviform_media_io::PacketFlags;
///
/// let flags = PacketFlags::KEYFRAME.union(PacketFlags::CORRUPT);
/// assert!(flags.is_keyframe());
/// assert!(flags.is_corrupt());
/// assert_eq!(flags.bits(), 0b11);
/// assert_eq!(PacketFlags::from_bits(0b11), flags);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PacketFlags(u32);

impl PacketFlags {
    /// No attributes — the default for an undistinguished unit.
    pub const NONE: Self = Self(0x0);
    /// Random-access point: decoding can start from this unit without
    /// reference to earlier units (keyframe / sync sample).
    pub const KEYFRAME: Self = Self(0x1);
    /// The unit is marked corrupt or damaged by the container — a
    /// decoder may attempt concealment rather than rejecting it.
    pub const CORRUPT: Self = Self(0x2);

    /// The empty flag set — same value as [`PacketFlags::NONE`].
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn empty() -> Self {
        Self::NONE
    }

    /// Builds a flag set from raw bits. Unknown bits are preserved
    /// verbatim — they are inert until a matching constant exists.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The raw bit representation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether no flags are set.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every flag in `other` is set in `self`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// The union of two flag sets.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether [`PacketFlags::KEYFRAME`] is set — the random-access
    /// point predicate seek logic keys on.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn is_keyframe(self) -> bool {
        self.contains(Self::KEYFRAME)
    }

    /// Whether [`PacketFlags::CORRUPT`] is set.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn is_corrupt(self) -> bool {
        self.contains(Self::CORRUPT)
    }
}

/// One encoded unit — the currency of the demux ↔ codec boundary.
///
/// Produced by [`Demuxer::read_packet`](crate::Demuxer::read_packet)
/// (and by [`Encoder::receive_packet`](crate::Encoder::receive_packet)
/// on the encode path), consumed by
/// [`Decoder::send_packet`](crate::Decoder::send_packet) (and by
/// [`Muxer::write_packet`](crate::Muxer::write_packet)). `Packet` is a
/// plain owned value type with public fields — literal construction is
/// the intended construction, mirroring the domain crates' descriptor
/// types.
///
/// ## Raw timestamps
///
/// `pts`/`dts`/`duration` are **raw tick counts** in the containing
/// stream's time base — not [`MediaTime`]. They pair with
/// `StreamDescriptor::time_base` (keyed via [`Packet::stream`]) when a
/// base is known, but stay observable when the base is unavailable —
/// a packet never fabricates a base to make its timestamps expressible.
/// [`TickTimestamp::provenance`](ruviform_media_core::TickTimestamp::provenance)
/// records whether each value was container-carried, derived from
/// container structure, or estimated by the backend.
///
/// `index` is the zero-based ordinal of this unit within its stream in
/// demux (decode) order — combined with `pts` it yields both the decode
/// sequence and the presentation sequence a container carried, which is
/// the raw input timestamp-gap/duplicate/VFR observation needs.
///
/// `trim_start`/`trim_end` carry codec-level trim (encoder delay /
/// padding) in decoded-frame units as declared for this unit; they are
/// `0` when the container declares none.
///
/// **Send contract:** `Packet` is `Send` — it is designed to cross the
/// I/O/decode-thread → bounded-queue boundary to the processing side
/// (design baseline, thread-model section).
///
/// **RT contract:** non-RT only — the boundary it crosses is non-RT;
/// `Packet` itself is a plain data value.
///
/// [`MediaTime`]: ruviform_media_core::MediaTime
/// [`StreamDescriptor::time_base`]: ruviform_media_core::StreamDescriptor::time_base
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{StreamId, TickTimestamp};
/// use ruviform_media_io::{Packet, PacketFlags};
///
/// let packet = Packet {
///     stream: StreamId(0),
///     index: 0,
///     pts: Some(TickTimestamp::carried(3_000)),
///     dts: Some(TickTimestamp::carried(2_970)),
///     duration: Some(30),
///     trim_start: 0,
///     trim_end: 0,
///     flags: PacketFlags::KEYFRAME,
///     data: vec![0xDE, 0xAD, 0xBE, 0xEF],
/// };
/// assert_eq!(packet.stream, StreamId(0));
/// assert!(packet.flags.is_keyframe());
/// assert_eq!(packet.pts.unwrap().ticks, 3_000);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    /// Container stream this unit belongs to — keys into the demuxer's
    /// [`StreamInfo`](crate::StreamInfo) list.
    pub stream: StreamId,
    /// Zero-based ordinal of this unit within its stream in demux
    /// (decode) order. Combined with `pts` it reconstructs both the
    /// decode sequence and the presentation sequence. Encoder-produced
    /// packets use their own emission order.
    pub index: u64,
    /// Presentation timestamp (ticks in the stream's time base) when
    /// the container carries one — the authoritative ordering key for
    /// presentation.
    pub pts: Option<TickTimestamp>,
    /// Decode timestamp (ticks in the stream's time base) when the
    /// container carries one — meaningful for reordered codecs
    /// (e.g. B-frames); usually `None` or equal to `pts` for audio and
    /// non-reordered video.
    pub dts: Option<TickTimestamp>,
    /// Duration of this encoded unit in stream-base ticks when carried
    /// — engines may derive it from the next unit's `pts` when absent.
    /// For reordered codecs this is the duration of the *valid* decoded
    /// frames the unit produces (i.e. excluding `trim_start`/`trim_end`).
    pub duration: Option<u64>,
    /// Decoded frames the decoder should drop from the start of this
    /// unit's decoded output (encoder delay). `0` when undeclared.
    pub trim_start: u64,
    /// Decoded frames the decoder should drop from the end of this
    /// unit's decoded output (encoder padding). `0` when undeclared.
    pub trim_end: u64,
    /// Encoded-unit attribute flags (keyframe, corruption marks, …).
    pub flags: PacketFlags,
    /// The encoded payload — owned bytes.
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::TimestampProvenance;

    fn check_send<T: Send>() {}

    fn sample_packet() -> Packet {
        Packet {
            stream: StreamId(3),
            index: 7,
            pts: Some(TickTimestamp::carried(1_024)),
            dts: Some(TickTimestamp::derived(1_000)),
            duration: Some(24),
            trim_start: 0,
            trim_end: 0,
            flags: PacketFlags::KEYFRAME,
            data: vec![1, 2, 3, 4],
        }
    }

    #[test]
    fn packet_holds_every_field_verbatim() {
        let packet = sample_packet();
        assert_eq!(packet.stream, StreamId(3));
        assert_eq!(packet.index, 7);
        assert_eq!(packet.pts.unwrap().ticks, 1_024);
        assert_eq!(packet.pts.unwrap().provenance, TimestampProvenance::Carried);
        assert_eq!(packet.dts.unwrap().ticks, 1_000);
        assert_eq!(packet.dts.unwrap().provenance, TimestampProvenance::Derived);
        assert_eq!(packet.duration, Some(24));
        assert_eq!((packet.trim_start, packet.trim_end), (0, 0));
        assert_eq!(packet.flags, PacketFlags::KEYFRAME);
        assert_eq!(packet.data, [1, 2, 3, 4]);
    }

    #[test]
    fn timestamps_may_be_absent() {
        let mut packet = sample_packet();
        packet.pts = None;
        packet.dts = None;
        packet.duration = None;
        assert!(packet.pts.is_none());
        assert!(packet.dts.is_none());
        assert!(packet.duration.is_none());
    }

    #[test]
    fn flags_combine_and_query() {
        let flags = PacketFlags::KEYFRAME.union(PacketFlags::CORRUPT);
        assert!(flags.contains(PacketFlags::KEYFRAME));
        assert!(flags.is_keyframe());
        assert!(flags.is_corrupt());
        assert!(!PacketFlags::NONE.is_keyframe());
        assert!(PacketFlags::empty().is_empty());
        // Unknown bits survive a from_bits/bits round trip.
        let mystery = PacketFlags::from_bits(0x80);
        assert_eq!(mystery.bits(), 0x80);
        assert!(!mystery.is_keyframe());
    }

    #[test]
    fn packet_send_bounds() {
        check_send::<Packet>();
        check_send::<PacketFlags>();
    }
}
