//! [`Encoder`] — the [`Frame`] → [`Packet`] abstraction, mirror of
//! [`Decoder`](crate::Decoder).
//!
//! **Unverified contract — no implementor yet:** no backend implements this
//! trait yet — the spec defers encode-side verification to the first
//! encode-capable backend (a demux/decode-only backend reports
//! [`ErrorKind::Unsupported`](crate::ErrorKind) for
//! it). The signature is the agreed boundary shape and is free to
//! change under `publish = false` until a real implementation
//! exercises it.

use crate::error::Result;
use crate::frame::Frame;
use crate::packet::Packet;

/// Frame → packet boundary: feeds decoded frames in, receives encoded
/// units out.
///
/// One encoder produces the units of **one** stream — symmetric with
/// [`Decoder`](crate::Decoder). The frame variant fed to
/// [`send_frame`](Self::send_frame) must match the stream kind the
/// encoder was built for (`Frame::Audio` for an audio encoder, …);
/// mismatches are an [`ErrorKind::InvalidData`](crate::ErrorKind)-class
/// usage error.
///
/// The send/receive pump mirrors the decode side: feed with
/// `send_frame`, pull with
/// [`receive_packet`](Self::receive_packet) until `Ok(None)`, and
/// signal end of input with [`drain`](Self::drain) so lookahead /
/// delayed-frame codecs can finish. Emitted packets carry the encoder's
/// own stream id and codec timestamps — stream placement in the
/// container is the muxer's concern.
///
/// The trait object is `Send` for the same thread-model reason as the
/// rest of the boundary.
///
/// **RT contract:** non-RT only — every method may allocate and do
/// codec work of unbounded cost.
pub trait Encoder: Send {
    /// Hands the encoder one frame to encode.
    ///
    /// Ownership transfers — the encoder may retain the frame for
    /// lookahead or reordering.
    ///
    /// * `Err(_)` — [`ErrorKind::InvalidData`](crate::ErrorKind) for a
    ///   frame variant/shape the encoder cannot consume,
    ///   [`ErrorKind::Unsupported`](crate::ErrorKind) for parameters
    ///   beyond the codec's capability.
    ///
    /// **RT contract:** non-RT only — may encode eagerly and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidData`](crate::ErrorKind),
    /// [`ErrorKind::Unsupported`](crate::ErrorKind) — see above.
    fn send_frame(&mut self, frame: Frame) -> Result<()>;

    /// Receives the next encoded unit in stream order.
    ///
    /// * `Ok(Some(packet))` — an encoded [`Packet`].
    /// * `Ok(None)` — nothing pending right now: either more input is
    ///   needed or, after [`drain`](Self::drain), encoding is complete.
    /// * `Err(_)` — encode failed
    ///   ([`ErrorKind::InvalidData`](crate::ErrorKind)).
    ///
    /// **RT contract:** non-RT only — may encode and allocate.
    fn receive_packet(&mut self) -> Result<Option<Packet>>;

    /// Signals end of input: no further
    /// [`send_frame`](Self::send_frame) calls will follow.
    ///
    /// Encoders holding frames for delay/lookahead emit their remaining
    /// units; afterwards [`receive_packet`](Self::receive_packet)
    /// drains to `Ok(None)`. The default is a no-op — correct for
    /// encoders without internal delay.
    ///
    /// **RT contract:** non-RT only — may encode and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidData`](crate::ErrorKind) when encoding held
    /// frames fails.
    fn drain(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::AudioFrame;
    use crate::packet::PacketFlags;
    use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    use ruviform_media_core::{MediaTime, StreamId, TimeBase};

    fn check_send<T: Send>() {}

    /// Canned encoder: one audio frame in → one opaque packet out.
    struct Stub {
        pending: Vec<Packet>,
    }

    impl Stub {
        fn new() -> Self {
            Self {
                pending: Vec::new(),
            }
        }
    }

    impl Encoder for Stub {
        fn send_frame(&mut self, frame: Frame) -> Result<()> {
            let Frame::Audio(audio) = frame else {
                return Err(crate::error::Error::from_kind(
                    crate::error::ErrorKind::InvalidData,
                ));
            };
            self.pending.push(Packet {
                stream: StreamId(0),
                pts: Some(audio.pts()),
                dts: None,
                duration: audio.duration(),
                flags: PacketFlags::NONE,
                data: vec![0u8; audio.buffer().frames()],
            });
            Ok(())
        }

        fn receive_packet(&mut self) -> Result<Option<Packet>> {
            Ok(self.pending.pop())
        }

        // `drain` not implemented — exercises the default no-op.
    }

    fn mono_frame(frames: usize) -> Frame {
        Frame::Audio(AudioFrame::new(
            AudioBuffer::new(BufferSpec {
                layout: ChannelLayout::Mono,
                frames,
                sample_rate: SampleRate::new(48_000).unwrap(),
            }),
            MediaTime::new(0, TimeBase::COMMON_90K),
        ))
    }

    #[test]
    fn stub_encoder_pumps_frames_to_packets() {
        let mut encoder = Stub::new();
        encoder.send_frame(mono_frame(3)).unwrap();

        let packet = encoder.receive_packet().unwrap().unwrap();
        assert_eq!(packet.stream, StreamId(0));
        assert_eq!(packet.data.len(), 3);
        assert!(encoder.receive_packet().unwrap().is_none());
    }

    #[test]
    fn mismatched_frame_variant_is_an_error() {
        use ruviform_video_core::{PixelFormat, VideoFrame};
        let mut encoder = Stub::new();
        let video = VideoFrame::new_owned(
            PixelFormat::Rgba8,
            2,
            2,
            MediaTime::new(0, TimeBase::COMMON_90K),
        )
        .unwrap();
        let error = encoder.send_frame(Frame::Video(video)).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::InvalidData);
    }

    #[test]
    fn drain_defaults_to_noop() {
        Stub::new().drain().unwrap();
    }

    #[test]
    fn encode_send_bounds() {
        check_send::<Box<dyn Encoder>>();
    }
}
