//! [`Muxer`] — the [`Packet`] → container abstraction, mirror of
//! [`Demuxer`](crate::Demuxer).
//!
//! **Unverified contract — no implementor yet:** no backend implements this
//! trait yet — the spec defers mux-side verification to the first
//! encode-capable backend (`ruviform-io-symphonia` is demux/decode
//! only and reports [`ErrorKind::Unsupported`](crate::ErrorKind) for
//! it). The signature is the agreed boundary shape and is free to
//! change under `publish = false` until a real implementation
//! exercises it.

use crate::demux::StreamInfo;
use crate::error::Result;
use crate::packet::Packet;

/// Packet → container boundary: stream declaration and packet writes.
///
/// Output streams are declared at backend construction (the backend's
/// own entry point takes the [`StreamInfo`] list and the sink); this
/// trait only covers the steady-state write path. The trait object is
/// `Send` for the same thread-model reason as
/// [`Demuxer`](crate::Demuxer).
///
/// **RT contract:** non-RT only — every method may block on I/O and
/// allocate.
pub trait Muxer: Send {
    /// The streams declared for the output container, in the order
    /// they were declared at construction.
    ///
    /// **RT contract:** non-RT only — borrow over declared state.
    fn streams(&self) -> &[StreamInfo];

    /// Writes one encoded unit into the container.
    ///
    /// Ownership transfers to the muxer — a backend may retain the
    /// packet to interleave it against other streams. Callers control
    /// arrival order; any interleaving policy beyond that is the
    /// backend's own (this contract is unverified — see the module
    /// doc).
    ///
    /// * `Err(_)` — write/serialization failure:
    ///   [`ErrorKind::Io`](crate::ErrorKind) for sink errors,
    ///   [`ErrorKind::InvalidData`](crate::ErrorKind) for a unit the
    ///   container format cannot express.
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`](crate::ErrorKind) on sink failure;
    /// [`ErrorKind::InvalidData`](crate::ErrorKind) or
    /// [`ErrorKind::InvalidValue`](crate::ErrorKind) for a unit the
    /// container cannot take.
    fn write_packet(&mut self, packet: Packet) -> Result<()>;

    /// Finalizes the container — writes trailer/index structures the
    /// format requires and flushes the sink.
    ///
    /// A backend should also make a best-effort finalize on `Drop`;
    /// `finish` is the *checked* path whose errors are observable.
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`](crate::ErrorKind) on sink failure.
    fn finish(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{CodecDescriptor, CodecFormat, CodecId};
    use crate::packet::PacketFlags;
    use ruviform_media_core::{StreamDescriptor, StreamId, StreamKind, TimeBase};

    fn check_send<T: Send>() {}

    struct Stub {
        streams: Vec<StreamInfo>,
        written: Vec<Packet>,
        finished: bool,
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
                written: Vec::new(),
                finished: false,
            }
        }
    }

    impl Muxer for Stub {
        fn streams(&self) -> &[StreamInfo] {
            &self.streams
        }

        fn write_packet(&mut self, packet: Packet) -> Result<()> {
            self.written.push(packet);
            Ok(())
        }

        fn finish(&mut self) -> Result<()> {
            self.finished = true;
            Ok(())
        }
    }

    #[test]
    fn stub_muxer_takes_ownership_and_finishes() {
        let mut muxer = Stub::new();
        assert_eq!(muxer.streams().len(), 1);

        muxer
            .write_packet(Packet {
                stream: StreamId(0),
                pts: None,
                dts: None,
                duration: None,
                flags: PacketFlags::KEYFRAME,
                data: vec![0xAB],
            })
            .unwrap();
        muxer.finish().unwrap();

        assert_eq!(muxer.written.len(), 1);
        assert_eq!(muxer.written[0].data, [0xAB]);
        assert!(muxer.finished);
    }

    #[test]
    fn object_form_is_usable() {
        let mut muxer: Box<dyn Muxer> = Box::new(Stub::new());
        assert_eq!(muxer.streams().len(), 1);
        muxer.finish().unwrap();
    }

    #[test]
    fn mux_send_bounds() {
        check_send::<Box<dyn Muxer>>();
    }
}
