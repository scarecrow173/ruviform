//! [`Demuxer`] implementation — probe + packet read + time-based seek.
//!
//! The demuxer wraps the backend's `FormatReader`, which owns the
//! adapted media source for the reader's lifetime. Stream metadata is
//! enumerated once at construction into boundary [`StreamInfo`] values —
//! [`Demuxer::streams`] then answers from the snapshot (the trait
//! contract returns `&[StreamInfo]`, not a `Result`).

use ruviform_media_core::{MediaDuration, MediaTime, StreamDescriptor, StreamId, TimeBase};
use ruviform_media_io::{
    CodecDescriptor, CodecFormat, Demuxer, Error, ErrorKind, Packet, PacketFlags, Result,
    StreamInfo,
};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::units::Time;

use crate::decoder::SymphoniaDecoder;
use crate::error::map_error;
use crate::map;

/// Media-I/O [`Demuxer`] backed by the backend's `FormatReader`.
///
/// **RT contract:** non-RT only — construction probes and may read the
/// source to its end looking for trailing metadata; `read_packet` and
/// `seek` perform I/O and unbounded container work.
pub struct SymphoniaDemuxer {
    /// The backend format reader — owns the adapted source.
    reader: Box<dyn FormatReader>,
    /// Stream snapshot taken at probe time; `streams()` answers from it.
    streams: Vec<StreamInfo>,
}

impl SymphoniaDemuxer {
    /// Takes ownership of a probed backend reader and snapshots its
    /// stream metadata.
    pub(crate) fn new(reader: Box<dyn FormatReader>) -> Self {
        let streams = reader.tracks().iter().map(stream_info).collect();
        Self { reader, streams }
    }

    /// Builds a decoder for the stream `id`, using the codec parameters
    /// the probe recorded for it.
    ///
    /// This is the construction path for [`SymphoniaDecoder`]: the
    /// decoder needs the backend's codec parameters, which only the
    /// demuxer's reader still holds — the boundary `StreamInfo`
    /// intentionally carries only backend-agnostic data.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidValue`] — `id` does not name one of this
    ///   demuxer's streams.
    /// - [`ErrorKind::Unsupported`] — the stream is not audio, or the
    ///   backend has no decoder registered for its codec. This is the
    ///   error half of capability negotiation, alongside
    ///   [`crate::capabilities`]' `decode_codecs` query.
    pub fn make_decoder(&self, id: StreamId) -> Result<SymphoniaDecoder> {
        let track = self
            .reader
            .tracks()
            .iter()
            .find(|track| track.id == id.0)
            .ok_or_else(|| Error::from_kind(ErrorKind::InvalidValue))?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(CodecParameters::audio)
            .ok_or_else(|| Error::from_kind(ErrorKind::Unsupported))?;
        SymphoniaDecoder::new(id, self.time_base_for(id), params)
    }

    /// The stream's time base as recorded at probe time — falls back to
    /// the canonical 90 kHz base when the container carried none (the
    /// same fallback `stream_info` applies).
    fn time_base_for(&self, id: StreamId) -> TimeBase {
        self.streams
            .iter()
            .find(|info| info.stream.id == id)
            .map_or(TimeBase::COMMON_90K, |info| info.stream.time_base)
    }
}

impl Demuxer for SymphoniaDemuxer {
    /// The streams enumerated at probe time.
    ///
    /// **RT contract:** non-RT only.
    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// Reads the next packet in container order — all tracks, not just
    /// the default.
    ///
    /// `Ok(None)` is a *normal* end of stream; a truncated container
    /// surfaces as [`ErrorKind::UnexpectedEos`], corrupt structures as
    /// [`ErrorKind::InvalidData`], and source failures as
    /// [`ErrorKind::Io`]. The backend reports no keyframe/corruption
    /// attributes, so packets carry [`PacketFlags::NONE`] (documented
    /// in the crate docs — not fabricated).
    ///
    /// **RT contract:** non-RT only — source I/O plus per-packet
    /// allocation.
    fn read_packet(&mut self) -> Result<Option<Packet>> {
        match self.reader.next_packet() {
            Ok(Some(packet)) => Ok(Some(self.to_packet(packet))),
            Ok(None) => Ok(None),
            Err(error) => Err(map_error(error)),
        }
    }

    /// Requests a time-based seek. `position` is interpreted as an
    /// absolute media time (its own time base is converted to seconds
    /// internally, so any base works).
    ///
    /// The request is the boundary operation; where the reader actually
    /// lands is returned by the backend but deliberately discarded —
    /// media-io's `seek` is request-only. Coordinating the landing point
    /// with decoding (decode-forward to the target PTS, decoder flush,
    /// stale-queue discard) is caller/engine-side work — part of the
    /// frame-accurate seek design, not this `PoC`'s scope. An unseekable
    /// source or a container without a seek index reports
    /// [`ErrorKind::Unsupported`]; an out-of-range position reports
    /// [`ErrorKind::InvalidValue`].
    ///
    /// **RT contract:** non-RT only — may read a prebuilt seek index or
    /// scan the source.
    fn seek(&mut self, position: MediaTime) -> Result<()> {
        let time = media_time_to_time(position)?;
        self.reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: None,
                },
            )
            .map_err(map_error)?;
        Ok(())
    }
}

impl SymphoniaDemuxer {
    /// Converts a backend packet into an owned boundary [`Packet`].
    ///
    /// Timestamps land on the emitting track's time base as recorded at
    /// probe time. `dur > i64::MAX` (unrepresentable in `MediaDuration`)
    /// degrades to `None` — the field is optional by contract.
    fn to_packet(&self, packet: symphonia::core::packet::Packet) -> Packet {
        let stream = StreamId(packet.track_id);
        let base = self.time_base_for(stream);
        Packet {
            stream,
            pts: Some(MediaTime::new(packet.pts.get(), base)),
            dts: Some(MediaTime::new(packet.dts.get(), base)),
            duration: i64::try_from(packet.dur.get())
                .ok()
                .map(|value| MediaDuration::new(value, base)),
            flags: PacketFlags::NONE,
            data: packet.data.into_vec(),
        }
    }
}

/// Snapshots one backend track as a boundary [`StreamInfo`].
///
/// The mapping is deliberately total — probe-time enumeration never
/// fails. Unknown codecs → [`CodecId::Other`], non-audio or
/// unrepresentable formats → `CodecFormat::Data`, missing time bases →
/// the canonical 90 kHz fallback (packets without a container time base
/// carry meaningless timestamps anyway), and unrepresentable durations
/// → `None`.
fn stream_info(track: &Track) -> StreamInfo {
    let params = track.codec_params.as_ref();
    let time_base = track
        .time_base
        .map_or(TimeBase::COMMON_90K, map::to_time_base);
    let format = match params {
        Some(CodecParameters::Audio(params)) => {
            map::audio_format(params).map_or(CodecFormat::Data, CodecFormat::Audio)
        }
        // Video codec parameters carry no pixel format, so a boundary
        // `VideoFormat` cannot be produced honestly — `Data` it is.
        _ => CodecFormat::Data,
    };
    let codec_private = match params {
        Some(CodecParameters::Audio(params)) => params
            .extra_data
            .as_deref()
            .map_or_else(Vec::new, <[u8]>::to_vec),
        _ => Vec::new(),
    };
    StreamInfo {
        stream: StreamDescriptor {
            id: StreamId(track.id),
            kind: map::stream_kind(params),
            time_base,
            duration: track
                .duration
                .and_then(|dur| i64::try_from(dur.get()).ok())
                .map(|value| MediaDuration::new(value, time_base)),
        },
        codec: CodecDescriptor {
            codec: map::codec_id(params),
            codec_private,
            format,
        },
    }
}

/// Converts a boundary [`MediaTime`] to the backend's `Time` (seconds +
/// fractional part), via total nanoseconds.
///
/// **RT contract:** non-RT only.
fn media_time_to_time(position: MediaTime) -> Result<Time> {
    let base = position.base();
    let nanos = i128::from(position.value()) * i128::from(base.numerator()) * 1_000_000_000_i128
        / i128::from(base.denominator());
    Time::try_from_nanos_i128(nanos).ok_or_else(|| Error::from_kind(ErrorKind::TimeOverflow))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::MediaTime;

    #[test]
    fn media_time_converts_to_seconds() {
        // 48_000 frames at 1/48_000 = exactly 1 second.
        let position = MediaTime::new(48_000, TimeBase::sample_rate(48_000).ok().unwrap());
        let time = media_time_to_time(position).unwrap();
        assert_eq!(time.as_secs(), 1);
    }

    #[test]
    fn media_time_fractional_position_converts() {
        // 0.5 s at 90 kHz = 45_000 ticks.
        let position = MediaTime::new(45_000, TimeBase::COMMON_90K);
        let time = media_time_to_time(position).unwrap();
        assert_eq!(time.as_secs(), 0);
        assert!((time.as_secs_f64() - 0.5).abs() < 1e-9);
    }
}
