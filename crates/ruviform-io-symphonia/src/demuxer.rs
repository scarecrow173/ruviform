//! [`Demuxer`] implementation — probe + packet read + time-based seek.
//!
//! The demuxer wraps the backend's `FormatReader`, which owns the
//! adapted media source for the reader's lifetime. Stream metadata is
//! enumerated once at construction into boundary [`StreamInfo`] values —
//! [`Demuxer::streams`] then answers from the snapshot (the trait
//! contract returns `&[StreamInfo]`, not a `Result`).
//!
//! Every reported value is a classified fact about the input: a missing
//! backend time base is `Fact::NotDeclared` (never a substituted
//! default), packet timestamps stay as raw ticks in the stream's own
//! base with `Carried` provenance, and container identification comes
//! from real magic bytes captured while probing.

use std::collections::HashMap;

use ruviform_media_core::{
    Fact, MediaDuration, MediaTime, StreamDescriptor, StreamId, TickTimestamp, TimeBase,
};
use ruviform_media_io::{
    BackendInfo, CodecDescriptor, CodecFormat, ContainerInfo, Demuxer, Error, ErrorKind, Packet,
    PacketFlags, Result, StreamFacts, StreamInfo,
};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::units::Time;

use crate::decoder::{Mode, SymphoniaDecoder};
use crate::error::map_error;
use crate::map;

/// Provenance of everything this backend reports through the boundary.
///
/// The version label is the backend library version pinned by this
/// crate's dependency declaration — backend version reporting is
/// recorded as a follow-up (the backend exposes no runtime query).
pub(crate) const BACKEND_INFO: BackendInfo = BackendInfo {
    name: "symphonia",
    version: "0.6.1",
};

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
    /// Container identification from the probed header bytes.
    container: ContainerInfo,
    /// Next packet index per stream (decode order) — `Packet::index`.
    indices: HashMap<u32, u64>,
}

impl SymphoniaDemuxer {
    /// Takes ownership of a probed backend reader and snapshots its
    /// stream metadata; `container` was classified from the bytes the
    /// probe itself read.
    pub(crate) fn new(reader: Box<dyn FormatReader>, container: ContainerInfo) -> Self {
        let streams = reader
            .tracks()
            .iter()
            .enumerate()
            .map(|(index, track)| stream_info(track, u32::try_from(index).unwrap_or(u32::MAX)))
            .collect();
        Self {
            reader,
            streams,
            container,
            indices: HashMap::new(),
        }
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
        self.make_decoder_in(id, Mode::Canonical)
    }

    /// Builds a *native-mode* decoder for the stream `id`: same pump
    /// contract as [`make_decoder`](Self::make_decoder), but
    /// `receive_frame` yields [`Frame::NativeAudio`] frames carrying
    /// the decoder's native-scale samples (no `f32` normalization).
    ///
    /// This is the request half of the [`Features::NATIVE_AUDIO`]
    /// capability.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// [`Frame::NativeAudio`]: ruviform_media_io::Frame::NativeAudio
    /// [`Features::NATIVE_AUDIO`]: ruviform_media_io::Features::NATIVE_AUDIO
    ///
    /// # Errors
    ///
    /// Same as [`make_decoder`](Self::make_decoder).
    pub fn make_native_decoder(&self, id: StreamId) -> Result<SymphoniaDecoder> {
        self.make_decoder_in(id, Mode::Native)
    }

    /// Shared decoder construction for both output modes.
    fn make_decoder_in(&self, id: StreamId, mode: Mode) -> Result<SymphoniaDecoder> {
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
        SymphoniaDecoder::new(id, self.time_base_for(id), params, mode)
    }

    /// The stream's time base as recorded at probe time — `None` when
    /// the container declared none. No fallback is applied: packets and
    /// frames keep raw ticks, and the decoder marks timestamp output
    /// `Estimated` instead of resolving against an invented base.
    fn time_base_for(&self, id: StreamId) -> Option<TimeBase> {
        self.streams
            .iter()
            .find(|info| info.stream.id == id)
            .and_then(|info| info.stream.time_base.value())
    }
}

impl Demuxer for SymphoniaDemuxer {
    /// The container family identified from the header bytes the probe
    /// read (see [`crate`]'s sniffing adapter).
    ///
    /// **RT contract:** non-RT only.
    fn container(&self) -> &ContainerInfo {
        &self.container
    }

    /// The backend identity behind every reported fact.
    ///
    /// **RT contract:** non-RT only.
    fn backend_info(&self) -> BackendInfo {
        BACKEND_INFO
    }

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
            Ok(Some(packet)) => Ok(Some(self.boundary_packet(packet))),
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
    /// Timestamps stay as raw ticks in the emitting stream's time base
    /// with `Carried` provenance — the backend hands demuxed values to
    /// us unchanged. `dur == 0` means "not supplied" for the readers in
    /// the default build and reports `None`; `trim_start`/`trim_end`
    /// carry codec trim verbatim.
    fn boundary_packet(&mut self, packet: symphonia::core::packet::Packet) -> Packet {
        let stream = packet.track_id;
        let counter = self.indices.entry(stream).or_insert(0);
        let index = *counter;
        *counter += 1;
        Packet {
            stream: StreamId(stream),
            index,
            pts: Some(TickTimestamp::carried(packet.pts.get())),
            dts: Some(TickTimestamp::carried(packet.dts.get())),
            duration: (packet.dur.get() != 0).then(|| packet.dur.get()),
            trim_start: packet.trim_start.get(),
            trim_end: packet.trim_end.get(),
            flags: PacketFlags::NONE,
            data: packet.data.into_vec(),
        }
    }
}

/// Snapshots one backend track as a boundary [`StreamInfo`].
///
/// The mapping is deliberately total — probe-time enumeration never
/// fails. Unknown codecs → [`CodecId::Other`], non-audio or
/// unrepresentable formats → `CodecFormat::Data`, a missing time base →
/// `Fact::NotDeclared` (never a substituted default), and an
/// unrepresentable duration → `None`.
fn stream_info(track: &Track, index: u32) -> StreamInfo {
    let params = track.codec_params.as_ref();
    let time_base = Fact::from_option(track.time_base.map(map::to_time_base));
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
    let native_format = match params {
        Some(CodecParameters::Audio(params)) => {
            Fact::from_option(map::native_audio_format(params).map(CodecFormat::Audio))
        }
        _ => Fact::NotDeclared,
    };
    let codec = map::codec_id(params);
    let decodable = if crate::capabilities().can_decode(codec) {
        Fact::known(())
    } else {
        Fact::Unsupported
    };
    StreamInfo {
        stream: StreamDescriptor {
            id: StreamId(track.id),
            kind: map::stream_kind(params),
            time_base,
            duration: track
                .duration
                .and_then(|dur| i64::try_from(dur.get()).ok())
                .zip(time_base.value())
                .map(|(value, base)| MediaDuration::new(value, base)),
        },
        codec: CodecDescriptor {
            codec,
            codec_private,
            format,
            native_format,
        },
        facts: StreamFacts {
            index,
            // For this backend the track id *is* the container's own
            // identifier — reported as the same value, not left empty.
            container_id: Some(u64::from(track.id)),
            // The backend does not expose a container codec tag.
            codec_tag: None,
            language: track.language.clone(),
            start: Fact::known(track.start_ts.get()),
            declared_units: Fact::from_option(track.num_frames),
            leading_trim: track.delay,
            trailing_trim: track.padding,
            video: None,
            decodable,
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
