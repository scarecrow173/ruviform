//! [`Demuxer`] — the container → [`Packet`] abstraction — and
//! [`StreamInfo`], the per-stream description it enumerates.
//!
//! `Demuxer` is one of the four separated trait families
//! (container demux/mux and codec decode/encode are deliberately *not*
//! merged into a single read/write abstraction — media-io ADR, design
//! baseline "メディア I/O 抽象"). A demuxer is constructed by the
//! backend's own entry point over a [`MediaSource`](crate::MediaSource);
//! construction-time probing is where container-format failures surface.

use ruviform_media_core::{MediaTime, StreamDescriptor, StreamId};

use crate::codec::CodecDescriptor;
use crate::error::Result;
use crate::packet::Packet;

/// One enumerated stream of a container: the codec-agnostic
/// [`StreamDescriptor`] plus the codec-level [`CodecDescriptor`].
///
/// Stream facts (id, kind, time base, duration) live in
/// [`StreamInfo::stream`]; codec facts (id, private parameters, decoded
/// format mapping) live in [`StreamInfo::codec`]. The split keeps
/// media-core's descriptor codec-free while media-io still reports
/// everything a caller needs to pick streams and build decoders.
///
/// **Send contract:** `StreamInfo` is `Send` — stream metadata crosses
/// the thread boundary with the demuxer that produced it.
///
/// **RT contract:** non-RT only — owns a `Vec<u8>` inside
/// [`CodecDescriptor::codec_private`].
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaDuration, StreamDescriptor, StreamId, StreamKind, TimeBase};
/// use ruviform_media_io::{CodecDescriptor, CodecFormat, CodecId, StreamInfo};
/// use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
///
/// let info = StreamInfo {
///     stream: StreamDescriptor {
///         id: StreamId(0),
///         kind: StreamKind::Audio,
///         time_base: TimeBase::COMMON_90K,
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
    /// decoded-format mapping.
    pub codec: CodecDescriptor,
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
/// **RT contract:** non-RT only — every method may block on I/O and
/// allocate.
pub trait Demuxer: Send {
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
    use ruviform_media_core::{StreamKind, TimeBase};

    fn check_send<T: Send>() {}

    /// Minimal canned demuxer exercising the trait through an object.
    struct Stub {
        streams: Vec<StreamInfo>,
        packets: Vec<Packet>,
        seeked_to: Option<MediaTime>,
    }

    impl Stub {
        fn new() -> Self {
            Self {
                streams: vec![StreamInfo {
                    stream: StreamDescriptor {
                        id: StreamId(0),
                        kind: StreamKind::Audio,
                        time_base: TimeBase::COMMON_90K,
                        duration: None,
                    },
                    codec: CodecDescriptor {
                        codec: CodecId::Pcm,
                        codec_private: Vec::new(),
                        format: CodecFormat::Data,
                    },
                }],
                packets: vec![
                    Packet {
                        stream: StreamId(0),
                        pts: None,
                        dts: None,
                        duration: None,
                        flags: crate::packet::PacketFlags::NONE,
                        data: vec![1, 2],
                    },
                    Packet {
                        stream: StreamId(0),
                        pts: None,
                        dts: None,
                        duration: None,
                        flags: crate::packet::PacketFlags::NONE,
                        data: vec![3, 4],
                    },
                ],
                seeked_to: None,
            }
        }
    }

    impl Demuxer for Stub {
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
    fn demux_send_bounds() {
        check_send::<StreamInfo>();
        check_send::<Box<dyn Demuxer>>();
    }
}
