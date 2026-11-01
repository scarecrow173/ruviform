//! [`WavEncoder`] — the [`Frame`] → [`Packet`] half of the WAV backend.
//!
//! The encoder accepts canonical planar `f32` [`AudioFrame`]s, converts
//! each to interleaved little-endian wire bytes in the declared
//! [`SampleFormat`], and emits one [`Packet`] per input frame — PCM
//! needs no codec delay, reordering or lookahead, so the packet stream
//! is exactly the frame stream with wire-encoded payloads.
//!
//! The stream declaration rule mirrors the decode side:
//! `StreamInfo::codec.format` is the *canonical* frame format the
//! encoder consumes (`Audio` at `F32`), while
//! `CodecDescriptor::native_format` declares the *wire* encoding the
//! file will carry (`Fact::Known(CodecFormat::Audio(..))` with a
//! format from [`crate::wav_capabilities`]). Both sides must agree on
//! sample rate and channel layout.

use crate::convert::write_sample;
use crate::wav::Wire;
use ruviform_audio_core::{AudioFormat, SampleFormat};
use ruviform_media_core::{
    Fact, StreamId, StreamKind, TickTimestamp, TimeBase, TimestampProvenance,
};
use ruviform_media_io::{
    CodecDescriptor, CodecFormat, CodecId, Encoder, Error, ErrorKind, Frame, Packet, PacketFlags,
    Result, StreamInfo,
};

/// Extracts the canonical (`format`) and wire (`native_format`) audio
/// formats from a stream declaration, applying the encode-side rules.
pub(crate) fn declared_formats(descriptor: &CodecDescriptor) -> Result<(AudioFormat, AudioFormat)> {
    if descriptor.codec != CodecId::Pcm {
        return Err(Error::with_source(
            ErrorKind::Unsupported,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("codec {:?} is not Pcm", descriptor.codec),
            ),
        ));
    }
    let canonical = match descriptor.format {
        CodecFormat::Audio(format) if format.sample_format == SampleFormat::F32 => format,
        _ => {
            return Err(Error::with_source(
                ErrorKind::Unsupported,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "encoder consumes canonical planar F32 frames only",
                ),
            ));
        }
    };
    let native = match &descriptor.native_format {
        Fact::Known(CodecFormat::Audio(format)) => *format,
        _ => {
            return Err(Error::with_source(
                ErrorKind::InvalidValue,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "native_format must declare the wire AudioFormat (Fact::Known)",
                ),
            ));
        }
    };
    if native.sample_rate != canonical.sample_rate || native.layout != canonical.layout {
        return Err(Error::with_source(
            ErrorKind::InvalidValue,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "canonical and wire formats must share sample rate and layout",
            ),
        ));
    }
    // `Wire::new` returns `Unsupported` for formats outside the
    // advertised set — the capability boundary stays honest.
    Wire::new(&native)?;
    Ok((canonical, native))
}

/// Validates an output stream declaration: exactly the checks an
/// encoder/muxer can enforce at construction — audio kind, PCM codec,
/// a well-formed canonical+wire format pair.
///
/// Returns `(canonical, native, wire)` on success.
pub(crate) fn validate_stream(info: &StreamInfo) -> Result<(AudioFormat, AudioFormat, Wire)> {
    if info.stream.kind != StreamKind::Audio {
        return Err(Error::with_source(
            ErrorKind::InvalidValue,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("stream kind {:?} is not Audio", info.stream.kind),
            ),
        ));
    }
    let (canonical, native) = declared_formats(&info.codec)?;
    let wire = Wire::new(&native)?;
    Ok((canonical, native, wire))
}

/// PCM WAV encoder: canonical `f32` [`AudioFrame`] → interleaved wire
/// [`Packet`].
///
/// **RT contract:** non-RT only — every method may allocate and does
/// conversion work.
///
/// [`AudioFrame`]: ruviform_media_io::AudioFrame
#[derive(Debug)]
pub struct WavEncoder {
    /// Stream id stamped on emitted packets — from the declaration.
    stream: StreamId,
    /// Stream time base (`1/rate`) for packet pts/duration ticks.
    time_base: TimeBase,
    /// Wire parameters.
    wire: Wire,
    /// Completed packets awaiting `receive_packet`, in emit order.
    pending: std::collections::VecDeque<Packet>,
    /// Packet counter.
    next_index: u64,
    /// Whether `drain` was signalled — end of input.
    drained: bool,
}

impl WavEncoder {
    /// Builds an encoder from an output stream declaration.
    ///
    /// See the module doc for the declaration rules; the wire encoding
    /// is `info.codec.native_format`'s `AudioFormat::sample_format`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`] — codec is not `Pcm`, the canonical
    /// format is not `Audio` at `F32`, or the wire sample format is
    /// outside `wav_capabilities`.
    /// [`ErrorKind::InvalidValue`] — non-audio stream kind, undeclared
    /// wire format, or rate/layout disagreement between canonical and
    /// wire formats.
    pub fn new(info: &StreamInfo) -> Result<Self> {
        let (_, native, wire) = validate_stream(info)?;
        Ok(Self {
            stream: info.stream.id,
            time_base: native.sample_rate.time_base(),
            wire,
            pending: std::collections::VecDeque::new(),
            next_index: 0,
            drained: false,
        })
    }

    /// The wire parameters this encoder was built with.
    ///
    /// **RT contract:** non-RT only — trivial accessor.
    #[must_use]
    pub const fn stream_id(&self) -> StreamId {
        self.stream
    }

    /// Converts a frame's pts into stream-base ticks. Exact rational
    /// conversion first; on overflow/range failure falls back to
    /// seconds-based rounding — documented, deterministic.
    #[allow(clippy::cast_possible_truncation)]
    fn pts_ticks(&self, frame: &ruviform_media_io::AudioFrame) -> (i64, TimestampProvenance) {
        let pts = frame.pts();
        let ticks = pts
            .time
            .convert(self.time_base, ruviform_media_core::Rounding::NearestEven)
            .map_or_else(
                |_| (pts.time.to_seconds_f64() * f64::from(self.wire.rate)).round() as i64,
                ruviform_media_core::MediaTime::value,
            );
        (ticks, pts.provenance)
    }
}

impl Encoder for WavEncoder {
    fn send_frame(&mut self, frame: Frame) -> Result<()> {
        if self.drained {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "send_frame after drain"),
            ));
        }
        let Frame::Audio(frame) = frame else {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "WavEncoder consumes Frame::Audio only",
                ),
            ));
        };
        let buffer = frame.buffer();
        if buffer.channels() != usize::from(self.wire.channels) {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "frame has {} channels, stream declares {}",
                        buffer.channels(),
                        self.wire.channels
                    ),
                ),
            ));
        }
        if buffer.sample_rate().get() != self.wire.rate {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "frame rate {} does not match declared {}",
                        buffer.sample_rate().get(),
                        self.wire.rate
                    ),
                ),
            ));
        }
        let frames = buffer.frames();
        if frames == 0 {
            // A zero-frame buffer produces no encoded unit — the
            // packet stream simply has nothing to carry.
            return Ok(());
        }
        let (ticks, provenance) = self.pts_ticks(&frame);
        let mut data = Vec::with_capacity(frames * usize::from(self.wire.block_align));
        for frame_index in 0..frames {
            for channel in 0..buffer.channels() {
                let sample = buffer
                    .channel(channel)
                    .map_or(0.0, |plane| plane[frame_index]);
                write_sample(&mut data, self.wire.format, sample);
            }
        }
        let index = self.next_index;
        self.next_index += 1;
        self.pending.push_back(Packet {
            stream: self.stream,
            index,
            pts: Some(TickTimestamp { ticks, provenance }),
            dts: None,
            duration: Some(frames as u64),
            trim_start: 0,
            trim_end: 0,
            flags: PacketFlags::KEYFRAME,
            data,
        });
        Ok(())
    }

    fn receive_packet(&mut self) -> Result<Option<Packet>> {
        Ok(self.pending.pop_front())
    }

    fn drain(&mut self) -> Result<()> {
        self.drained = true;
        Ok(())
    }
}
