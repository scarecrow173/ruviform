//! [`Demuxer`] — the container → [`Packet`] abstraction — plus the
//! container/stream identification and observation-fact types it
//! reports: [`StreamInfo`], [`StreamFacts`], [`ContainerInfo`],
//! [`ContainerFormat`], [`ContainerDetail`], [`CodecTag`],
//! [`VideoStreamFacts`] and [`BackendInfo`].
//!
//! `Demuxer` is one of the four separated trait families
//! (container demux/mux and codec decode/encode are deliberately *not*
//! merged into a single read/write abstraction — media-io ADR, design
//! baseline "メディア I/O 抽象"). A demuxer is constructed by the
//! backend's own entry point over a [`MediaSource`](crate::MediaSource);
//! construction-time probing is where container-format failures surface.
//!
//! ## Observation contract
//!
//! Everything a demuxer reports is a *fact about the input*, classified
//! — never defaulted. Where a container does not declare a value, or
//! the backend cannot supply one, fields are [`Option`] (plain
//! "not supplied") or [`Fact`] (with the classified reason:
//! `NotDeclared` / `Unsupported` / `Malformed`). A `0` or an empty
//! string is only ever a *real* value the container carried.

use ruviform_media_core::{Fact, FrameRate, MediaTime, StreamDescriptor, StreamId};

use crate::codec::CodecDescriptor;
use crate::error::Result;
use crate::packet::Packet;

/// Which container family a [`Demuxer`] is reading.
///
/// `Other` is the honest fallback when the backend does not identify
/// its container format — it is never a guess.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContainerFormat {
    /// RIFF/WAVE audio container.
    Wav,
    /// RIFF/AVI audio-video container.
    Avi,
    /// ISO Base Media File Format family (`.mov`/`.mp4`/`.m4a`/`.3gp`).
    Isobmff,
    /// Ogg container family (Vorbis/Opus/FLAC-in-Ogg).
    Ogg,
    /// Matroska/WebM container.
    Matroska,
    /// Native FLAC container.
    Flac,
    /// A container the backend did not identify as any named family.
    #[default]
    Other,
}

impl ContainerFormat {
    /// Stable lowercase identification label (e.g. `"isobmff"`).
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Avi => "avi",
            Self::Isobmff => "isobmff",
            Self::Ogg => "ogg",
            Self::Matroska => "matroska",
            Self::Flac => "flac",
            Self::Other => "other",
        }
    }
}

/// Container-family-specific identification facts, carried verbatim.
///
/// New variants may be added per container family as backends learn to
/// report them; the enum is `#[non_exhaustive]`.
///
/// **RT contract:** non-RT only — may own allocated lists.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerDetail {
    /// ISO-BMFF brand identification (`ftyp`).
    Isobmff {
        /// The major brand, as the 4 raw code bytes (not assumed UTF-8).
        major_brand: [u8; 4],
        /// Brand minor version field.
        minor_version: u32,
        /// All declared compatible brands, as raw 4-byte codes.
        compatible_brands: Vec<[u8; 4]>,
    },
}

/// What container a [`Demuxer`] is reading, with the identification
/// facts the backend could supply.
///
/// **Send contract:** `ContainerInfo` is `Send`.
///
/// **RT contract:** non-RT only — may own allocated detail.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContainerInfo {
    /// The container family — [`ContainerFormat::Other`] when the
    /// backend does not identify it.
    pub format: ContainerFormat,
    /// Container-family-specific facts, when supplied.
    pub detail: Option<ContainerDetail>,
}

/// Identity of the backend implementing a [`Demuxer`].
///
/// Provenance for observations: which backend produced the facts, at
/// which version. Both fields are static strings chosen by the backend
/// crate.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BackendInfo {
    /// Backend implementation name (e.g. `"symphonia"`).
    pub name: &'static str,
    /// Backend implementation version label — the backend crate pins
    /// the mapping to its vendored library version.
    pub version: &'static str,
}

/// A codec tag exactly as the container recorded it.
///
/// Containers identify codecs with a 4-byte code (ISOBMFF/AVI fourCC)
/// or a numeric tag (WAVE format tag); both are preserved verbatim —
/// the canonical [`CodecId`](crate::CodecId) translation is a *separate*
/// fact in [`CodecDescriptor::codec`].
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CodecTag {
    /// A 4-byte codec code (ISOBMFF/AVI fourCC), not assumed UTF-8.
    FourCc([u8; 4]),
    /// A numeric codec tag (e.g. the WAVE `wFormatTag`).
    Numeric(u64),
}

/// Declared facts that only video streams carry.
///
/// Each field is `Option`: an absent `Option` is "not supplied" — a
/// video stream with a declared dimension reports `Some`, one without
/// reports `None` rather than `0`.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VideoStreamFacts {
    /// Declared coded frame width in pixels.
    pub width: Option<u32>,
    /// Declared coded frame height in pixels.
    pub height: Option<u32>,
    /// Nominal declared frame rate, when the container supplies one.
    /// Real cadence is observed from packet timestamps — this is the
    /// *declared* rate only.
    pub nominal_frame_rate: Option<FrameRate>,
}

/// Extended per-stream facts a demuxer observed while probing.
///
/// These complement the minimal [`StreamDescriptor`] (id, kind, time
/// base, duration) with the container-track facts an analysis consumer
/// needs to identify and interpret a stream — without fabricating
/// anything the container did not carry.
///
/// - [`StreamFacts::index`] / [`StreamFacts::container_id`] distinguish
///   enumeration order from container-native track identity (they can
///   differ — e.g. ISOBMFF `tkhd` track ids).
/// - [`StreamFacts::start`] is the stream's declared clock origin —
///   the timestamp the stream's presentation begins at (edit-list
///   media time, header start fields), in `StreamDescriptor::time_base`
///   ticks.
/// - [`StreamFacts::leading_trim`]/[`StreamFacts::trailing_trim`] carry
///   declared codec delay/padding in decoded-frame units.
/// - [`StreamFacts::decodable`] reports whether this backend can decode
///   the stream — `Unsupported` is a capability fact, never an error
///   surfaced later.
///
/// **Send contract:** `StreamFacts` is `Send`.
///
/// **RT contract:** non-RT only — may own an allocated `language`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamFacts {
    /// Zero-based index of the stream in container enumeration order —
    /// the position of this stream in [`Demuxer::streams`].
    pub index: u32,
    /// The container's own track/stream identifier, when it differs
    /// conceptually from [`StreamId`] (e.g. an ISOBMFF `tkhd` track id).
    pub container_id: Option<u64>,
    /// The codec tag as the container recorded it (fourCC / numeric
    /// tag) — provenance for the canonical [`CodecId`](crate::CodecId)
    /// mapping.
    pub codec_tag: Option<CodecTag>,
    /// The stream's declared language tag, when carried.
    pub language: Option<String>,
    /// The stream's declared clock origin — the tick the stream's
    /// presentation begins at — in `StreamDescriptor::time_base` ticks.
    /// `Fact` because a container may carry an origin concept the
    /// backend cannot read.
    pub start: Fact<i64>,
    /// The declared count of encoded units (samples/frames) in the
    /// stream — for comparison against the actual decoded-frame count.
    pub declared_units: Fact<u64>,
    /// Declared codec delay: decoded frames to skip at the stream's
    /// start (encoder priming), when carried.
    pub leading_trim: Option<u32>,
    /// Declared codec padding: decoded frames to drop at the stream's
    /// end, when carried.
    pub trailing_trim: Option<u32>,
    /// Declared facts specific to video streams (`None` for other
    /// kinds).
    pub video: Option<VideoStreamFacts>,
    /// Whether the backend can decode this stream —
    /// [`Fact::Known`](()) means a decoder can be built;
    /// `Unsupported`/`NotDeclared`/`Malformed` carry the classified
    /// reason it cannot.
    pub decodable: Fact<()>,
}

/// One enumerated stream of a container: the codec-agnostic
/// [`StreamDescriptor`], the codec-level [`CodecDescriptor`] and the
/// extended [`StreamFacts`].
///
/// Stream facts (id, kind, time base, duration) live in
/// [`StreamInfo::stream`]; codec facts (id, private parameters, decoded
/// and native format mappings) live in [`StreamInfo::codec`];
/// extended observation facts (container identity, clock origin,
/// declared counts, decode capability) live in [`StreamInfo::facts`].
/// The split keeps media-core's descriptor codec-free while media-io
/// still reports everything a caller needs to pick streams, build
/// decoders and record provenance.
///
/// **Send contract:** `StreamInfo` is `Send` — stream metadata crosses
/// the thread boundary with the demuxer that produced it.
///
/// **RT contract:** non-RT only — owns allocated values inside
/// [`CodecDescriptor::codec_private`] and [`StreamFacts`].
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{Fact, MediaDuration, StreamDescriptor, StreamId, StreamKind, TimeBase};
/// use ruviform_media_io::{CodecDescriptor, CodecFormat, CodecId, StreamFacts, StreamInfo};
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
///
/// let info = StreamInfo {
///     stream: StreamDescriptor {
///         id: StreamId(0),
///         kind: StreamKind::Audio,
///         time_base: Fact::known(TimeBase::COMMON_90K),
///         duration: Some(MediaDuration::new(90_000, TimeBase::COMMON_90K)),
///     },
///     codec: CodecDescriptor {
///         codec: CodecId::Flac,
///         codec_private: Vec::new(),
///         format: CodecFormat::Audio(AudioFormat {
///             sample_rate: SampleRate::new(48_000).unwrap(),
///             layout: ChannelLayout::Stereo,
///             sample_format: SampleFormat::F32,
///         }),
///         native_format: Fact::Unsupported,
///     },
///     facts: StreamFacts {
///         index: 0,
///         decodable: Fact::known(()),
///         ..StreamFacts::default()
///     },
/// };
/// assert_eq!(info.stream.kind, StreamKind::Audio);
/// assert_eq!(info.codec.codec, CodecId::Flac);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamInfo {
    /// Codec-agnostic container-level stream description.
    pub stream: StreamDescriptor,
    /// Codec-level description — id, opaque private data and the
    /// decoded/native format mappings.
    pub codec: CodecDescriptor,
    /// Extended observation facts — container identity, clock origin,
    /// declared counts, decode capability.
    pub facts: StreamFacts,
}

/// Container → [`Packet`] boundary: stream enumeration, packet reads
/// and a time-based seek *request*.
///
/// Implementations live in `ruviform-io-*` backend crates, constructed
/// from a [`MediaSource`](crate::MediaSource). The trait object is
/// `Send`: a demuxer is owned by the I/O thread while `Packet`s and
/// [`StreamInfo`] travel onward to the decode side.
///
/// This trait is a *boundary port only*: it defines no container
/// behavior beyond reading units in container order and accepting a
/// seek request. Resynchronization, presentation ordering and VFR
/// handling after a seek are engines' responsibilities (design
/// baseline L239-L240).
///
/// `streams()` yields `Option`-/`Fact`-classified facts — packets read
/// incrementally supply the raw tick series
/// ([`Packet::index`], [`Packet::pts`], [`Packet::dts`]) an observation
/// pass joins with `StreamDescriptor::time_base`, so large inputs are
/// consumed sequentially rather than buffered whole.
///
/// **RT contract:** non-RT only — every method may block on I/O and
/// allocate.
pub trait Demuxer: Send {
    /// Which container this demuxer is reading, with the
    /// identification facts the backend could supply.
    ///
    /// **RT contract:** non-RT only — borrow over probed state.
    fn container(&self) -> &ContainerInfo;

    /// Identity of the backend implementing this demuxer —
    /// provenance for everything the boundary reports.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    fn backend_info(&self) -> BackendInfo;

    /// The streams discovered in the container, in container order.
    ///
    /// Probing happens at backend construction time, so enumeration is
    /// a borrow, not a fallible call.
    ///
    /// **RT contract:** non-RT only — borrow over probed state.
    fn streams(&self) -> &[StreamInfo];

    /// Convenience lookup of one stream by id.
    ///
    /// **RT contract:** non-RT only — linear scan over probed state.
    fn stream(&self, id: StreamId) -> Option<&StreamInfo> {
        self.streams().iter().find(|info| info.stream.id == id)
    }

    /// Reads the next encoded unit in container order.
    ///
    /// * `Ok(Some(packet))` — the next unit.
    /// * `Ok(None)` — **normal end of stream**. Not an error.
    /// * `Err(_)` — abnormal termination or data failure: truncated
    ///   input maps to [`ErrorKind::UnexpectedEos`](crate::ErrorKind),
    ///   corrupt container structures to
    ///   [`ErrorKind::InvalidData`](crate::ErrorKind).
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate.
    fn read_packet(&mut self) -> Result<Option<Packet>>;

    /// Requests a time-based seek to `position`.
    ///
    /// This is the *request port only* (spec: the boundary provides
    /// the entry point, not the runtime behavior): after a successful
    /// call the next [`read_packet`](Self::read_packet) yields units
    /// from the seeked region, but decoder resynchronization
    /// ([`Decoder::flush`](crate::Decoder::flush)), A/V resync and
    /// VFR/presentation-order handling are engines' responsibilities.
    ///
    /// An unseekable source or backend reports
    /// [`ErrorKind::Unsupported`](crate::ErrorKind) — the error half of
    /// capability negotiation; it never panics.
    ///
    /// **RT contract:** non-RT only — may block on I/O.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`](crate::ErrorKind) when seeking is not
    /// provided; [`ErrorKind::Io`](crate::ErrorKind) on underlying
    /// source failure.
    fn seek(&mut self, position: MediaTime) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{CodecFormat, CodecId};
    use crate::packet::PacketFlags;
    use ruviform_media_core::{StreamKind, TickTimestamp, TimeBase};

    fn check_send<T: Send>() {}

    /// Minimal canned demuxer exercising the trait through an object.
    struct Stub {
        container: ContainerInfo,
        streams: Vec<StreamInfo>,
        packets: Vec<Packet>,
        seeked_to: Option<MediaTime>,
    }

    impl Stub {
        fn new() -> Self {
            Self {
                container: ContainerInfo {
                    format: ContainerFormat::Wav,
                    detail: None,
                },
                streams: vec![StreamInfo {
                    stream: StreamDescriptor {
                        id: StreamId(0),
                        kind: StreamKind::Audio,
                        time_base: Fact::known(TimeBase::COMMON_90K),
                        duration: None,
                    },
                    codec: CodecDescriptor {
                        codec: CodecId::Pcm,
                        codec_private: Vec::new(),
                        format: CodecFormat::Data,
                        native_format: Fact::Unsupported,
                    },
                    facts: StreamFacts::default(),
                }],
                packets: vec![
                    Packet {
                        stream: StreamId(0),
                        index: 0,
                        pts: Some(TickTimestamp::carried(0)),
                        dts: None,
                        duration: None,
                        trim_start: 0,
                        trim_end: 0,
                        flags: PacketFlags::NONE,
                        data: vec![1, 2],
                    },
                    Packet {
                        stream: StreamId(0),
                        index: 1,
                        pts: Some(TickTimestamp::carried(24)),
                        dts: None,
                        duration: Some(24),
                        trim_start: 0,
                        trim_end: 0,
                        flags: PacketFlags::NONE,
                        data: vec![3, 4],
                    },
                ],
                seeked_to: None,
            }
        }
    }

    impl Demuxer for Stub {
        fn container(&self) -> &ContainerInfo {
            &self.container
        }

        fn backend_info(&self) -> BackendInfo {
            BackendInfo {
                name: "stub",
                version: "0",
            }
        }

        fn streams(&self) -> &[StreamInfo] {
            &self.streams
        }

        fn read_packet(&mut self) -> Result<Option<Packet>> {
            Ok(self.packets.pop())
        }

        fn seek(&mut self, position: MediaTime) -> Result<()> {
            self.seeked_to = Some(position);
            Ok(())
        }
    }

    #[test]
    fn stub_demuxer_enumerates_and_reads_to_eos() {
        let mut demuxer: Box<dyn Demuxer> = Box::new(Stub::new());
        assert_eq!(demuxer.streams().len(), 1);
        assert_eq!(demuxer.container().format, ContainerFormat::Wav);
        assert_eq!(demuxer.backend_info().name, "stub");
        assert!(demuxer.stream(StreamId(0)).is_some());
        assert!(demuxer.stream(StreamId(9)).is_none());

        assert_eq!(demuxer.read_packet().unwrap().unwrap().data, [3, 4]);
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data, [1, 2]);
        // Normal end of stream is Ok(None), never an error.
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn seek_is_accepted_as_a_request() {
        let mut demuxer = Stub::new();
        let position = MediaTime::new(45_000, TimeBase::COMMON_90K);
        demuxer.seek(position).unwrap();
        assert_eq!(demuxer.seeked_to, Some(position));
    }

    #[test]
    fn container_format_names_are_stable() {
        assert_eq!(ContainerFormat::Wav.name(), "wav");
        assert_eq!(ContainerFormat::Isobmff.name(), "isobmff");
        assert_eq!(ContainerFormat::Other.name(), "other");
    }

    #[test]
    fn stream_facts_default_is_empty_not_fabricated() {
        let facts = StreamFacts::default();
        assert_eq!(facts.index, 0);
        assert_eq!(facts.container_id, None);
        assert_eq!(facts.start, Fact::NotDeclared);
        assert_eq!(facts.declared_units, Fact::NotDeclared);
        assert_eq!(facts.decodable, Fact::NotDeclared);
    }

    #[test]
    fn demux_send_bounds() {
        check_send::<StreamInfo>();
        check_send::<ContainerInfo>();
        check_send::<CodecTag>();
        check_send::<Box<dyn Demuxer>>();
    }
}
