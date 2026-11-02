//! [`Decoder`] — the [`Packet`] → [`Frame`] abstraction, separated from
//! [`Demuxer`](crate::Demuxer) (container) per the four-family split.
//!
//! The contract is a send/receive pump, matching codecs whose output is
//! not 1:1 with input units (delay, reordering, multi-frame units):
//! callers feed units with [`send_packet`](Decoder::send_packet) and
//! pull decoded frames with
//! [`receive_frame`](Decoder::receive_frame) until it reports
//! `Ok(None)`. [`flush`](Decoder::flush) is the required
//! residual-state reset engines use after a seek.

use crate::error::Result;
use crate::frame::Frame;
use crate::packet::Packet;

/// Which decoded representation a decoder is built to emit.
///
/// Decoders constructed through
/// [`Demuxer::make_decoder`](crate::Demuxer::make_decoder) take this
/// choice explicitly so the canonical playback path and the lossless
/// native observation path share one construction port — neither is
/// implied by the other, and choosing one never suppresses the other
/// at the boundary.
///
/// * [`Canonical`](Self::Canonical) — [`Frame::Audio`](crate::Frame)
///   output: planar normalized `f32` in the stream's declared rate and
///   channel layout. This is the playback representation.
/// * [`Native`](Self::Native) — [`Frame::NativeAudio`](crate::Frame)
///   output: interleaved `f64` samples on the source's native scale
///   with the recorded [`PcmTransform`](crate::PcmTransform) history.
///   This is the high-fidelity observation representation — requesting
///   it must not be treated as a request for `f32` normalization.
///
/// A backend that cannot honour a mode reports
/// [`ErrorKind::Unsupported`](crate::ErrorKind) from
/// [`Demuxer::make_decoder`](crate::Demuxer::make_decoder) — the error
/// half of capability negotiation.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum DecodeMode {
    /// Canonical `f32` planar decoded output.
    #[default]
    Canonical,
    /// Native-scale `f64` interleaved decoded output.
    Native,
}

/// Packet → frame boundary: sends encoded units in, receives decoded
/// frames out, and resets residual codec state on request.
///
/// One decoder consumes the packets of **one** stream — the caller
/// pairs a [`Demuxer`](crate::Demuxer) stream
/// ([`StreamInfo`](crate::StreamInfo)) with the decoder built for it.
/// Feeding a unit of the wrong codec/stream kind is an
/// [`ErrorKind::InvalidData`](crate::ErrorKind)-class usage error.
///
/// # Send/receive protocol
///
/// 1. [`send_packet`](Self::send_packet) hands the decoder one unit;
///    the decoder may consume it immediately or retain it (owned —
///    no copy is forced by the API).
/// 2. [`receive_frame`](Self::receive_frame) yields decoded frames in
///    presentation order, `Ok(None)` when nothing is pending. Callers
///    drain until `None` before feeding the next unit; a backend may
///    still accept further packets while frames remain pending
///    (internal queueing is an implementation detail).
/// 3. At end of input, [`drain`](Self::drain) signals "no more units"
///    so delayed codecs emit their held frames; keep calling
///    `receive_frame` until `Ok(None)`.
///
/// `flush` is the reset port for seeks — it discards every retained
/// unit and all decoder-internal reference state so post-seek output
/// carries nothing from before the seek. Reordering policy, A/V
/// resync and presentation-time management around it are engines'
/// responsibilities.
///
/// The trait object is `Send`: a decoder is owned by the decode thread
/// while [`Frame`]s travel onward through a bounded queue.
///
/// **RT contract:** non-RT only — every method may allocate and do
/// codec work of unbounded cost.
pub trait Decoder: Send {
    /// Hands the decoder one encoded unit of its stream.
    ///
    /// * `Err(_)` — the unit could not be accepted:
    ///   [`ErrorKind::InvalidData`](crate::ErrorKind) for a malformed
    ///   or misdirected unit, [`ErrorKind::Unsupported`](crate::ErrorKind)
    ///   for codec parameters the implementation cannot handle.
    ///
    /// **RT contract:** non-RT only — may decode eagerly and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidData`](crate::ErrorKind),
    /// [`ErrorKind::Unsupported`](crate::ErrorKind) — see above.
    fn send_packet(&mut self, packet: Packet) -> Result<()>;

    /// Receives the next decoded frame in presentation order.
    ///
    /// * `Ok(Some(frame))` — a decoded [`Frame`].
    /// * `Ok(None)` — nothing pending right now: either more input is
    ///   needed or, after [`drain`](Self::drain), decoding is complete.
    ///   `None` is a state report, never an error.
    /// * `Err(_)` — decode failed (corrupt unit data maps to
    ///   [`ErrorKind::InvalidData`](crate::ErrorKind)).
    ///
    /// **RT contract:** non-RT only — may decode and allocate.
    fn receive_frame(&mut self) -> Result<Option<Frame>>;

    /// Signals end of input: no further
    /// [`send_packet`](Self::send_packet) calls will follow.
    ///
    /// Codecs with internal delay emit their held frames in response;
    /// afterwards [`receive_frame`](Self::receive_frame) drains to
    /// `Ok(None)`. The default is a no-op — correct for decoders with
    /// no inter-unit delay (e.g. simple per-packet audio codecs).
    ///
    /// **RT contract:** non-RT only — may decode and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidData`](crate::ErrorKind) when flushing held
    /// units fails.
    fn drain(&mut self) -> Result<()> {
        Ok(())
    }

    /// Discards all residual state: retained units, reference frames
    /// and inter-unit decoder history.
    ///
    /// This is the seek-side reset port only — what the engine does
    /// around it (demuxer seek, A/V resync, VFR handling) is engines'
    /// responsibility and outside this contract.
    ///
    /// **RT contract:** non-RT only — may free codec-internal state.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`](crate::ErrorKind)-class backend failures; most
    /// implementations cannot fail, but the signature stays uniform
    /// with the rest of the boundary.
    fn flush(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::AudioFrame;
    use crate::packet::PacketFlags;
    use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    use ruviform_media_core::{MediaTime, MediaTimestamp, StreamId, TickTimestamp, TimeBase};

    fn check_send<T: Send>() {}

    /// Canned decoder: one packet in → one empty audio frame out;
    /// `flush` drops the pending queue.
    struct Stub {
        pending: Vec<Frame>,
        flushed: usize,
    }

    impl Stub {
        fn new() -> Self {
            Self {
                pending: Vec::new(),
                flushed: 0,
            }
        }
    }

    impl Decoder for Stub {
        fn send_packet(&mut self, packet: Packet) -> Result<()> {
            let buffer = AudioBuffer::new(BufferSpec {
                layout: ChannelLayout::Mono,
                frames: packet.data.len(),
                sample_rate: SampleRate::new(48_000).unwrap(),
            });
            let pts = packet
                .pts
                .map_or(MediaTimestamp::estimated(MediaTime::ZERO), |ts| {
                    MediaTimestamp {
                        time: ts.at(TimeBase::COMMON_90K),
                        provenance: ts.provenance,
                    }
                });
            self.pending
                .push(Frame::Audio(AudioFrame::new(buffer, pts)));
            Ok(())
        }

        fn receive_frame(&mut self) -> Result<Option<Frame>> {
            Ok(if self.pending.is_empty() {
                None
            } else {
                Some(self.pending.remove(0))
            })
        }

        fn flush(&mut self) -> Result<()> {
            self.pending.clear();
            self.flushed += 1;
            Ok(())
        }
    }

    fn data_packet(data: &[u8]) -> Packet {
        Packet {
            stream: StreamId(0),
            index: 0,
            pts: Some(TickTimestamp::carried(0)),
            dts: None,
            duration: None,
            trim_start: 0,
            trim_end: 0,
            flags: PacketFlags::NONE,
            data: data.to_vec(),
        }
    }

    #[test]
    fn stub_decoder_pumps_packets_to_frames() {
        let mut decoder = Stub::new();
        decoder.send_packet(data_packet(&[1, 2, 3])).unwrap();
        decoder.send_packet(data_packet(&[4])).unwrap();

        let frame = decoder.receive_frame().unwrap().unwrap();
        assert_eq!(frame.pts(), Some(MediaTime::ZERO));
        assert!(decoder.receive_frame().unwrap().is_some());
        // Queue drained → Ok(None), the non-error EOS convention.
        assert!(decoder.receive_frame().unwrap().is_none());
    }

    #[test]
    fn flush_discards_pending_output() {
        let mut decoder = Stub::new();
        decoder.send_packet(data_packet(&[1])).unwrap();
        decoder.flush().unwrap();
        assert!(decoder.receive_frame().unwrap().is_none());
        assert_eq!(decoder.flushed, 1);
    }

    #[test]
    fn drain_defaults_to_noop() {
        struct NoDelay;
        impl Decoder for NoDelay {
            fn send_packet(&mut self, _packet: Packet) -> Result<()> {
                Ok(())
            }
            fn receive_frame(&mut self) -> Result<Option<Frame>> {
                Ok(None)
            }
            fn flush(&mut self) -> Result<()> {
                Ok(())
            }
        }
        // `drain` is not implemented — the default runs.
        NoDelay.drain().unwrap();
    }

    #[test]
    fn decode_send_bounds() {
        check_send::<Box<dyn Decoder>>();
    }
}
