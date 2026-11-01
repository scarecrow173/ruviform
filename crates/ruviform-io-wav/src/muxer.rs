//! [`WavMuxer`] — the [`Packet`] → RIFF/WAVE half of the WAV backend.
//!
//! Construction writes the complete `RIFF/WAVE` header immediately
//! (`fmt ` + `data` with zeroed sizes); `write_packet` appends packet
//! payloads to the `data` chunk; [`WavMuxer::finalize`] seeks back and
//! patches both size fields, pads the data chunk to even alignment,
//! flushes, and returns a [`WavReceipt`] — the only completion
//! receipt this backend issues. `Drop` retries a best-effort finalize
//! but its result is unobservable and never claimed as completion.
//!
//! Classic RIFF only: a `data` chunk crossing `u32::MAX` is
//! [`ErrorKind::InvalidData`] — RF64 is outside the advertised
//! capability set, so there is no silent larger-format fallback.

use crate::encoder::validate_stream;
use crate::wav::{self, Wire};
use ruviform_media_core::{Fact, MediaDuration};
use ruviform_media_io::{Error, ErrorKind, MediaSink, Muxer, Packet, Result, StreamInfo};
use std::io::SeekFrom;

/// Checked-completion receipt for a finalized WAV file — returned by
/// [`WavMuxer::finalize`], the only path that may claim the output is
/// complete.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavReceipt {
    /// Whole interleaved frames written to the `data` chunk
    /// (`data_bytes / block_align`).
    pub total_frames: u64,
    /// Bytes written to the `data` chunk (payload only, excluding
    /// header and pad).
    pub total_bytes: u64,
    /// Stream duration in the stream's time base (`1/rate` ticks =
    /// frames).
    pub duration: MediaDuration,
    /// First packet's pts in stream ticks, when any packet carried
    /// one — reported for diagnostics because plain WAV cannot
    /// represent a nonzero start offset.
    pub first_pts_ticks: Option<i64>,
}

/// PCM WAV muxer over a [`MediaSink`]: header at open, packet append,
/// size-backpatch finalize.
///
/// **RT contract:** non-RT only — every method may block on sink I/O.
#[derive(Debug)]
pub struct WavMuxer<S: MediaSink> {
    /// The output sink — `Option` only so `into_sink` can take it
    /// out of a `Drop`-implementing type.
    sink: Option<S>,
    /// The declared stream, normalized to the container's actual
    /// time base (`1/rate`).
    streams: Vec<StreamInfo>,
    /// Wire parameters from the declared stream.
    wire: Wire,
    /// Byte offset of the `data` chunk's size field.
    data_size_offset: u64,
    /// Bytes written to the `data` chunk.
    data_bytes: u64,
    /// First packet pts seen, for the receipt.
    first_pts_ticks: Option<i64>,
    /// Completion receipt once finalized.
    receipt: Option<WavReceipt>,
}

impl<S: MediaSink> WavMuxer<S> {
    /// Opens a WAV file on `sink` for exactly one declared audio
    /// stream, writing the header immediately.
    ///
    /// The declared stream follows the same rules as
    /// [`WavEncoder::new`](crate::WavEncoder::new): `kind = Audio`,
    /// `codec = Pcm`, canonical `format` = `Audio(F32)`, wire encoding
    /// in `native_format`.
    ///
    /// **RT contract:** non-RT only — writes the header to the sink.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`] — anything outside
    /// `wav_capabilities` (non-PCM codec, unsupported wire format,
    /// more than one stream).
    /// [`ErrorKind::InvalidValue`] — malformed stream declaration.
    /// [`ErrorKind::Io`] — header write failed.
    pub fn new(mut sink: S, streams: &[StreamInfo]) -> Result<Self> {
        if streams.len() != 1 {
            return Err(Error::with_source(
                ErrorKind::Unsupported,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "WAV carries exactly one audio stream, got {}",
                        streams.len()
                    ),
                ),
            ));
        }
        let (_, native, wire) = validate_stream(&streams[0])?;
        let mut declared = streams[0].clone();
        // The container's time base is `1/rate` — derived fact, not a
        // fabricated guess: PCM WAV units are interleaved frames.
        declared.stream.time_base = Fact::known(native.sample_rate.time_base());
        declared.facts.decodable = Fact::NotDeclared;
        let (header, data_size_offset) = wav::header(&wire);
        sink.write_all(&header)
            .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        Ok(Self {
            sink: Some(sink),
            streams: vec![declared],
            wire,
            data_size_offset,
            data_bytes: 0,
            first_pts_ticks: None,
            receipt: None,
        })
    }

    /// Finalizes the file: pads the data chunk to even, backpatches
    /// the `RIFF` and `data` sizes, flushes the sink, and returns the
    /// completion receipt.
    ///
    /// This is the checked completion path — a `WavReceipt` exists
    /// only when every byte reached the sink. Durability (fsync +
    /// rename into place) is the caller's next step, e.g.
    /// [`StagedOutput::publish`](ruviform_media_io::StagedOutput).
    ///
    /// **RT contract:** non-RT only — seeks and writes.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidData`] — `finalize` was already called.
    /// [`ErrorKind::Io`] — seek/write/flush failed.
    pub fn finalize(&mut self) -> Result<WavReceipt> {
        if self.receipt.is_some() {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "finalize already completed",
                ),
            ));
        }
        let receipt = self.finalize_inner()?;
        self.receipt = Some(receipt);
        Ok(receipt)
    }

    fn finalize_inner(&mut self) -> Result<WavReceipt> {
        let Some(sink) = self.sink.as_mut() else {
            return Err(Error::from_kind(ErrorKind::InvalidData));
        };
        if self.data_bytes % 2 == 1 {
            // RIFF chunks are even-aligned; pad with a zero byte.
            sink.write_all(&[0])
                .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        }
        let file_len = sink
            .seek(SeekFrom::End(0))
            .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        let riff_size =
            u32::try_from(file_len - 8).map_err(|_| Error::from_kind(ErrorKind::InvalidData))?;
        let data_size =
            u32::try_from(self.data_bytes).map_err(|_| Error::from_kind(ErrorKind::InvalidData))?;
        sink.seek(SeekFrom::Start(wav::RIFF_SIZE_OFFSET))
            .and_then(|_| sink.write_all(&riff_size.to_le_bytes()))
            .and_then(|()| sink.seek(SeekFrom::Start(self.data_size_offset)))
            .and_then(|_| sink.write_all(&data_size.to_le_bytes()))
            .and_then(|()| sink.seek(SeekFrom::End(0)))
            .and_then(|_| sink.flush())
            .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        let frames = self.data_bytes / u64::from(self.wire.block_align);
        Ok(WavReceipt {
            total_frames: frames,
            total_bytes: self.data_bytes,
            duration: MediaDuration::new(
                i64::try_from(frames).unwrap_or(i64::MAX),
                self.streams[0]
                    .stream
                    .time_base
                    .get()
                    .copied()
                    .unwrap_or(ruviform_media_core::TimeBase::COMMON_90K),
            ),
            first_pts_ticks: self.first_pts_ticks,
        })
    }

    /// The completion receipt, once `finalize` has succeeded.
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub const fn receipt(&self) -> Option<&WavReceipt> {
        self.receipt.as_ref()
    }

    /// Unwraps into the sink — useful after `finalize` when the caller
    /// wants the sink back (e.g. to hand the file to a staging
    /// primitive).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Panics
    ///
    /// Panics only if the sink was already taken — impossible through
    /// the public API (`into_sink` consumes `self`).
    pub fn into_sink(mut self) -> S {
        // `Drop` runs on `self` after this returns, with `sink` set to
        // `None` — the best-effort finalize inside `Drop` short-circuits
        // on the missing sink.
        self.sink.take().expect("sink present until into_sink")
    }
}

impl<S: MediaSink> Muxer for WavMuxer<S> {
    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    fn write_packet(&mut self, packet: Packet) -> Result<()> {
        if self.receipt.is_some() {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "write_packet after finalize",
                ),
            ));
        }
        let stream = self.streams[0].stream.id;
        if packet.stream != stream {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "packet stream {:?} does not match declared {:?}",
                        packet.stream, stream
                    ),
                ),
            ));
        }
        let length = packet.data.len() as u64;
        if !length.is_multiple_of(u64::from(self.wire.block_align)) {
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "packet payload is not a whole number of frames",
                ),
            ));
        }
        if self.data_bytes + length > u64::from(u32::MAX) {
            // RF64 is outside the advertised capability set — the
            // honest answer at the 4 GiB boundary is an error, not a
            // silent larger container.
            return Err(Error::with_source(
                ErrorKind::InvalidData,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "data chunk exceeds classic RIFF 4 GiB limit (RF64 unsupported)",
                ),
            ));
        }
        if self.first_pts_ticks.is_none() {
            self.first_pts_ticks = packet.pts.map(|pts| pts.ticks);
        }
        let Some(sink) = self.sink.as_mut() else {
            return Err(Error::from_kind(ErrorKind::InvalidData));
        };
        sink.write_all(&packet.data)
            .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        self.data_bytes += length;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.finalize().map(|_| ())
    }
}

impl<S: MediaSink> Drop for WavMuxer<S> {
    /// Best-effort finalize — never a completion guarantee. The
    /// checked path is [`WavMuxer::finalize`]/[`Muxer::finish`].
    fn drop(&mut self) {
        if self.receipt.is_none() {
            let _ = self.finalize_inner();
        }
    }
}

/// Backwards-compatible alias for the concrete muxer over an owned
/// boxed sink.
pub type BoxedWavMuxer = WavMuxer<Box<dyn MediaSink>>;

/// Convenience for callers that hold their sink behind
/// `Box<dyn MediaSink>` — identical to
/// [`WavMuxer::new`] but keeps the concrete type nameable.
///
/// **RT contract:** non-RT only — writes the header to the sink.
///
/// # Errors
///
/// Same as [`WavMuxer::new`].
pub fn open_boxed_muxer(sink: Box<dyn MediaSink>, streams: &[StreamInfo]) -> Result<BoxedWavMuxer> {
    WavMuxer::new(sink, streams)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WavOutputSpec;
    use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};

    fn spec() -> WavOutputSpec {
        WavOutputSpec {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::S16,
        }
    }

    #[test]
    fn empty_file_finalizes_to_valid_header() {
        let info = spec().stream_info();
        let cursor = std::io::Cursor::new(Vec::new());
        let mut muxer = WavMuxer::new(cursor, &[info]).unwrap();
        let receipt = muxer.finalize().unwrap();
        assert_eq!(receipt.total_frames, 0);
        assert_eq!(receipt.total_bytes, 0);
        let bytes = muxer.into_sink().into_inner();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8
        );
        assert_eq!(&bytes[8..12], b"WAVE");
    }

    #[test]
    fn second_finalize_is_invalid_data() {
        let info = spec().stream_info();
        let cursor = std::io::Cursor::new(Vec::new());
        let mut muxer = WavMuxer::new(cursor, &[info]).unwrap();
        muxer.finalize().unwrap();
        assert_eq!(muxer.finalize().unwrap_err().kind(), ErrorKind::InvalidData);
    }
}
