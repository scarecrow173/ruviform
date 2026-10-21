//! [`Decoder`] implementation — send/receive audio decode.
//!
//! The backend audio decoder decodes synchronously: one packet in, all
//! of its frames out in a single call, so [`send_packet`](Decoder::send_packet)
//! decodes eagerly into a small output queue and
//! [`receive_frame`](Decoder::receive_frame) drains it. This keeps the
//! boundary's pump semantics without fabricating asynchronous behaviour:
//! `Ok(None)` from `receive_frame` means "needs more input", exactly as
//! the trait contract intends.

use std::collections::VecDeque;

use ruviform_audio_core::{AudioBuffer, BufferSpec, SampleRate};
use ruviform_media_core::{MediaDuration, MediaTime, Rounding, StreamId, TimeBase};
use ruviform_media_io::{AudioFrame, Decoder, Error, ErrorKind, Frame, Packet, Result};
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia::core::packet::Packet as BackendPacket;
use symphonia::core::units::{Duration as BackendDuration, Timestamp as BackendTimestamp};

use crate::error::map_error;
use crate::map;

/// Media-I/O [`Decoder`] backed by a backend audio decoder.
///
/// **RT contract:** non-RT only — `send_packet` performs unbounded codec
/// work and allocates; `receive_frame` allocates nothing itself but only
/// drains what `send_packet` queued.
pub struct SymphoniaDecoder {
    /// The backend decoder — decodes synchronously inside `send_packet`.
    decoder: Box<dyn AudioDecoder>,
    /// The decoded stream's time base; frame timestamps land on it.
    time_base: TimeBase,
    /// The stream this decoder accepts; packets for other streams are a
    /// usage error.
    stream: StreamId,
    /// Frames decoded by `send_packet`, awaiting `receive_frame`.
    pending: VecDeque<Frame>,
    /// Extrapolated timestamp of the next emitted frame in stream time
    /// base units — the fallback for packets arriving without a PTS.
    next_ts: i64,
}

impl SymphoniaDecoder {
    /// Builds a decoder for `params` via the backend codec registry.
    ///
    /// `Ok` only when the registry holds a decoder for the codec —
    /// `ErrorKind::Unsupported` otherwise, which is also the answer a
    /// `decode_codecs` capability query gives for unmapped codecs.
    ///
    /// **RT contract:** non-RT only — codec construction may allocate
    /// and run parameter parsing.
    pub(crate) fn new(
        stream: StreamId,
        time_base: TimeBase,
        params: &AudioCodecParameters,
    ) -> Result<Self> {
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(map_error)?;
        Ok(Self {
            decoder,
            time_base,
            stream,
            pending: VecDeque::new(),
            next_ts: 0,
        })
    }

    /// Resolves the frame presentation timestamp for `packet`: its own
    /// PTS converted into the stream time base when present, else the
    /// extrapolated `next_ts` anchor.
    fn packet_pts(&self, packet: &Packet) -> Result<MediaTime> {
        match packet.pts {
            Some(pts) => pts
                .convert(self.time_base, Rounding::NearestEven)
                .map_err(Error::from),
            None => Ok(MediaTime::new(self.next_ts, self.time_base)),
        }
    }

    /// Converts the boundary packet into an owned backend packet for
    /// `decode`. Timestamps are converted into stream time base units;
    /// a missing PTS/DTS falls back to the resolved PTS. Durations that
    /// cannot be represented as backend units (negative, overflowing
    /// u64) degrade to zero — duration metadata only.
    fn to_backend_packet(&self, packet: Packet, pts: MediaTime) -> BackendPacket {
        let duration = packet
            .duration
            .and_then(|duration| duration.convert(self.time_base, Rounding::NearestEven).ok())
            .and_then(|duration| u64::try_from(duration.value()).ok())
            .unwrap_or(0);
        let dts = packet
            .dts
            .and_then(|dts| dts.convert(self.time_base, Rounding::NearestEven).ok())
            .map_or_else(|| pts.value(), MediaTime::value);
        let mut backend = BackendPacket::new(
            packet.stream.0,
            BackendTimestamp::new(pts.value()),
            BackendDuration::new(duration),
            packet.data.into_boxed_slice(),
        );
        backend.dts = BackendTimestamp::new(dts);
        backend
    }
}

impl Decoder for SymphoniaDecoder {
    /// Feeds one packet and decodes it eagerly into the output queue.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidData`] — the packet belongs to a different
    ///   stream, or the bitstream is corrupt.
    /// - [`ErrorKind::Unsupported`] — decoded output cannot be
    ///   represented in the boundary model (unmappable channel set).
    /// - [`ErrorKind::Io`] — a decoder-internal I/O failure.
    /// - [`ErrorKind::InvalidValue`] / [`ErrorKind::TimeOverflow`] —
    ///   timestamp or format arithmetic overflowed the value model.
    ///
    /// **RT contract:** non-RT only — unbounded codec work, allocates.
    fn send_packet(&mut self, packet: Packet) -> Result<()> {
        if packet.stream != self.stream {
            return Err(Error::from_kind(ErrorKind::InvalidData));
        }
        let pts = self.packet_pts(&packet)?;
        let backend_packet = self.to_backend_packet(packet, pts);

        // `decoded` borrows `self.decoder`; everything needed for frame
        // construction is copied out before the borrow ends.
        let decoded = self.decoder.decode(&backend_packet).map_err(map_error)?;
        let rate = decoded.spec().rate();
        let layout = map::channel_layout(decoded.spec().channels())
            .ok_or_else(|| Error::from_kind(ErrorKind::Unsupported))?;
        let frames = decoded.frames();
        let mut planes: Vec<Vec<f32>> = Vec::new();
        decoded.copy_to_vecs_planar(&mut planes);

        // The backend decoded nothing presentable for this packet
        // (e.g. pre-skip priming) — input consumed, no output.
        if frames == 0 {
            return Ok(());
        }

        let mut buffer = AudioBuffer::try_new(BufferSpec {
            layout,
            frames,
            sample_rate: SampleRate::new(rate).map_err(Error::from)?,
        })
        .map_err(Error::from)?;
        if planes.len() != buffer.channels() {
            return Err(Error::from_kind(ErrorKind::InvalidData));
        }
        for (index, plane) in planes.iter().enumerate() {
            if let Some(channel) = buffer.channel_mut(index) {
                channel.copy_from_slice(plane);
            }
        }

        // Frame duration: `frames` sample instants at the decoded rate,
        // converted into the stream's time base. Duration is optional —
        // an unrepresentable value degrades to `None`.
        let duration = i64::try_from(frames)
            .ok()
            .and_then(|value| TimeBase::sample_rate(rate).ok().map(|base| (value, base)))
            .and_then(|(value, base)| {
                MediaDuration::new(value, base)
                    .convert(self.time_base, Rounding::NearestEven)
                    .ok()
            });
        let mut frame = AudioFrame::new(buffer, pts);
        frame.set_duration(duration);
        self.next_ts = pts
            .value()
            .saturating_add(duration.map_or(0, MediaDuration::value));
        self.pending.push_back(Frame::Audio(frame));
        Ok(())
    }

    /// Pops the next decoded frame — `Ok(None)` when the queue is empty,
    /// i.e. the pump needs another `send_packet` (or `drain`).
    ///
    /// **RT contract:** non-RT only.
    fn receive_frame(&mut self) -> Result<Option<Frame>> {
        Ok(self.pending.pop_front())
    }

    /// Resets codec reference state and discards queued output and the
    /// timestamp extrapolation anchor.
    ///
    /// **RT contract:** non-RT only.
    fn flush(&mut self) -> Result<()> {
        self.decoder.reset();
        self.pending.clear();
        self.next_ts = 0;
        Ok(())
    }
}
