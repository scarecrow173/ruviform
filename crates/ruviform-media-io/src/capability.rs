//! Capability negotiation — the *query* half.
//!
//! A backend reports what it provides through a [`Capabilities`] value;
//! requesting a feature it does not provide fails with
//! [`ErrorKind::Unsupported`](crate::ErrorKind) — the two are the paired
//! mechanisms of capability negotiation (the spec's capability
//! requirement: the queryable side and the error classification side;
//! panic and `unimplemented!()` are prohibited representations).
//!
//! `Capabilities` deliberately does not universalize every feature
//! (ADR Consequences — no flat generalization): it reports the four
//! separated trait families via [`Features`] and the codec set each
//! direction supports. Anything beyond that is backend-specific and
//! stays behind the boundary.

use crate::codec::CodecId;

/// Backend feature flags — which of the four separated trait families
/// are provided.
///
/// A `u32` bit set in the same style as
/// [`PacketFlags`](crate::PacketFlags): `const` constructors and
/// bit-set queries. More flags may be added without breaking users of
/// [`Features::from_bits`]/[`Features::bits`].
///
/// **RT contract:** non-RT only — part of the media-I/O boundary
/// (`Copy` value type; the contract regulates the API context it is
/// used in, not the value itself).
///
/// # Examples
///
/// ```
/// use ruviform_media_io::Features;
///
/// let decode_side = Features::DEMUX.union(Features::DECODE);
/// assert!(decode_side.contains(Features::DEMUX));
/// assert!(!decode_side.contains(Features::MUX));
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Features(u32);

impl Features {
    /// No features — the empty set.
    pub const NONE: Self = Self(0x0);
    /// Container demuxing — [`Demuxer`](crate::Demuxer) is provided.
    pub const DEMUX: Self = Self(0x1);
    /// Container muxing — [`Muxer`](crate::Muxer) is provided.
    pub const MUX: Self = Self(0x2);
    /// Packet → frame decoding — [`Decoder`](crate::Decoder) is
    /// provided.
    pub const DECODE: Self = Self(0x4);
    /// Frame → packet encoding — [`Encoder`](crate::Encoder) is
    /// provided.
    pub const ENCODE: Self = Self(0x8);
    /// Native (non-canonical) audio decode — the backend can build
    /// decoders that emit [`Frame::NativeAudio`](crate::Frame)
    /// carrying [`NativeAudioFrame`](crate::NativeAudioFrame) values
    /// instead of the canonical planar `f32` [`AudioFrame`]. Meaningful
    /// only together with [`Features::DECODE`].
    ///
    /// [`AudioFrame`]: crate::AudioFrame
    pub const NATIVE_AUDIO: Self = Self(0x10);

    /// The empty flag set — same value as [`Features::NONE`].
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
}

/// What a backend provides — the queryable half of capability
/// negotiation.
///
/// Constructed by the backend's own entry point (e.g. a
/// `capabilities()` function on the backend crate); consumers read it
/// to decide which of the four trait families —
/// [`Demuxer`](crate::Demuxer) / [`Muxer`](crate::Muxer) /
/// [`Decoder`](crate::Decoder) / [`Encoder`](crate::Encoder) — and
/// which codecs are available. `Default` reports *nothing* provided —
/// the safe baseline a backend opts individual features into.
///
/// **Send contract:** `Capabilities` is `Send` — it travels with the
/// backend objects it describes.
///
/// **RT contract:** non-RT only — owns `Vec` fields; cloning allocates.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::{Capabilities, CodecId, Features};
///
/// // A demux + decode only backend (mux/encode unsupported).
/// let capabilities = Capabilities {
///     functions: Features::DEMUX.union(Features::DECODE),
///     decode_codecs: vec![CodecId::Aac, CodecId::Mp3, CodecId::Flac],
///     ..Capabilities::default()
/// };
/// assert!(capabilities.can_demux());
/// assert!(!capabilities.can_mux());
/// assert!(capabilities.can_decode(CodecId::Flac));
/// assert!(!capabilities.can_encode(CodecId::Flac));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// Which of the four trait families the backend provides.
    pub functions: Features,
    /// Codec IDs the backend can decode. Meaningful only when
    /// [`Features::DECODE`] is set.
    pub decode_codecs: Vec<CodecId>,
    /// Codec IDs the backend can encode. Meaningful only when
    /// [`Features::ENCODE`] is set.
    pub encode_codecs: Vec<CodecId>,
}

impl Capabilities {
    /// Whether container demuxing is provided —
    /// [`Features::DEMUX`] is set.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_io::{Capabilities, Features};
    ///
    /// let capabilities = Capabilities {
    ///     functions: Features::DEMUX,
    ///     ..Capabilities::default()
    /// };
    /// assert!(capabilities.can_demux());
    /// assert!(!capabilities.can_mux());
    /// ```
    #[must_use]
    pub const fn can_demux(&self) -> bool {
        self.functions.contains(Features::DEMUX)
    }

    /// Whether container muxing is provided — [`Features::MUX`] is set.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn can_mux(&self) -> bool {
        self.functions.contains(Features::MUX)
    }

    /// Whether `codec` is decodable — `true` only when
    /// [`Features::DECODE`] is set *and* the codec is in
    /// [`decode_codecs`](Self::decode_codecs).
    ///
    /// **RT contract:** non-RT only — linear scan over the codec list.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_io::{Capabilities, CodecId, Features};
    ///
    /// let capabilities = Capabilities {
    ///     functions: Features::DECODE,
    ///     decode_codecs: vec![CodecId::Vorbis],
    ///     ..Capabilities::default()
    /// };
    /// assert!(capabilities.can_decode(CodecId::Vorbis));
    /// assert!(!capabilities.can_decode(CodecId::Opus));
    /// ```
    #[must_use]
    pub fn can_decode(&self, codec: CodecId) -> bool {
        self.functions.contains(Features::DECODE) && self.decode_codecs.contains(&codec)
    }

    /// Whether `codec` is encodable — `true` only when
    /// [`Features::ENCODE`] is set *and* the codec is in
    /// [`encode_codecs`](Self::encode_codecs).
    ///
    /// **RT contract:** non-RT only — linear scan over the codec list.
    #[must_use]
    pub fn can_encode(&self, codec: CodecId) -> bool {
        self.functions.contains(Features::ENCODE) && self.encode_codecs.contains(&codec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_send<T: Send>() {}

    #[test]
    fn default_reports_nothing_provided() {
        let capabilities = Capabilities::default();
        assert!(!capabilities.can_demux());
        assert!(!capabilities.can_mux());
        assert!(!capabilities.can_decode(CodecId::Pcm));
        assert!(!capabilities.can_encode(CodecId::Pcm));
        assert!(capabilities.functions.is_empty());
    }

    #[test]
    fn codec_lookup_needs_both_flag_and_listing() {
        // Codec listed but the family flag is off → not reported.
        let capabilities = Capabilities {
            decode_codecs: vec![CodecId::Aac],
            ..Capabilities::default()
        };
        assert!(!capabilities.can_decode(CodecId::Aac));

        // Flag on but codec not listed → not reported.
        let capabilities = Capabilities {
            functions: Features::ENCODE,
            encode_codecs: vec![CodecId::Opus],
            ..Capabilities::default()
        };
        assert!(capabilities.can_encode(CodecId::Opus));
        assert!(!capabilities.can_encode(CodecId::Aac));
    }

    #[test]
    fn features_combine_as_a_bit_set() {
        let all = Features::DEMUX
            .union(Features::MUX)
            .union(Features::DECODE)
            .union(Features::ENCODE);
        assert_eq!(all.bits(), 0xF);
        assert!(all.contains(Features::DECODE));
        assert!(!Features::NONE.contains(Features::DEMUX));
        // Unknown bits survive a from_bits/bits round trip.
        let mystery = Features::from_bits(0x40);
        assert_eq!(mystery.bits(), 0x40);
    }

    #[test]
    fn capabilities_send_bounds() {
        check_send::<Capabilities>();
        check_send::<Features>();
    }
}
