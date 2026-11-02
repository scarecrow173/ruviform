//! [`SourceMetadata`] — the capability/metadata projection a reader
//! reports for one stream.
//!
//! This is a *projection*, not a copy of everything: the full
//! observation surface stays in [`StreamInfo`]/[`ContainerInfo`]
//! (which the metadata carries by value), while the fields a playback
//! consumer needs up front — canonical output format, declared frame
//! count, seek capability — are surfaced as classified [`Fact`]s.
//!
//! **RT contract:** non-RT only — metadata owns allocated stream
//! facts and is produced while probing.

use ruviform_audio_core::{AudioFormat, SampleFormat};
use ruviform_media_core::StreamId;
use ruviform_media_core::{Fact, Rounding, TimeBase};
use ruviform_media_io::{
    BackendInfo, CodecFormat, ContainerInfo, DecodeMode, Demuxer, Error, ErrorKind, Result,
    StreamInfo,
};

/// Everything a playback/observation consumer needs to declare about
/// one stream *before* consuming it.
///
/// * `backend`/`container`/`stream` are the boundary's own reports —
///   provenance included.
/// * `mode` records which decode representation was requested;
///   `canonical` is the `f32` planar format the canonical path emits.
/// * `frame_count` is the *declared* frame count: the stream's
///   declared duration converted into the canonical rate —
///   `Fact::Known` only when both are known. Measured truth is the
///   decoded total, never this field alone.
/// * `seekable` mirrors [`Demuxer::seekable`]: `Known(())`,
///   `Unsupported`, or `NotDeclared` — the declaration half of the
///   seek contract.
///
/// **Send contract:** `Send` — metadata crosses with the demuxer.
///
/// **RT contract:** non-RT only — owns the stream's facts.
#[derive(Clone, Debug)]
pub struct SourceMetadata {
    /// Which backend produced every fact here.
    pub backend: BackendInfo,
    /// The container family and its identification facts.
    pub container: ContainerInfo,
    /// The selected stream's full observation record (codec, formats,
    /// `StreamFacts` incl. `native_format`/`decodable`/`declared_units`).
    pub stream: StreamInfo,
    /// The decode representation this reader was opened with.
    pub mode: DecodeMode,
    /// The canonical `f32` planar output format (rate + layout) —
    /// what [`SourceReader::read_planar`](crate::SourceReader::read_planar)
    /// and [`PcmBlock`](crate::PcmBlock) channels carry.
    ///
    /// [`SourceReader::read_planar`]: crate::SourceReader::read_planar
    pub canonical: AudioFormat,
    /// Declared total source frames — `Fact`-classified, never
    /// fabricated (see the type docs).
    pub frame_count: Fact<u64>,
    /// Declared seek capability — `Fact`-classified (see the type docs).
    pub seekable: Fact<()>,
}

/// Builds the [`SourceMetadata`] projection for stream `id` of an
/// opened `demuxer`, for the given decode `mode`.
///
/// This is the shared entry point both `SourceReader` and
/// metadata-only consumers use — one authoritative derivation.
///
/// **RT contract:** non-RT only — borrows probed state, may clone.
///
/// # Errors
///
/// * [`ErrorKind::InvalidValue`] — `id` does not name one of the
///   demuxer's streams.
/// * [`ErrorKind::Unsupported`] — the stream is not an audio stream or
///   its decoded format is not representable as an [`AudioFormat`]
///   (the reader is an audio path).
pub fn stream_metadata(
    demuxer: &dyn Demuxer,
    id: StreamId,
    mode: DecodeMode,
) -> Result<SourceMetadata> {
    let stream = demuxer
        .stream(id)
        .ok_or_else(|| Error::from_kind(ErrorKind::InvalidValue))?
        .clone();
    let decoded = match &stream.codec.format {
        CodecFormat::Audio(format) => *format,
        _ => return Err(Error::from_kind(ErrorKind::Unsupported)),
    };
    let canonical = AudioFormat {
        sample_rate: decoded.sample_rate,
        layout: decoded.layout,
        sample_format: SampleFormat::F32,
    };
    let frame_count = declared_frame_count(&stream, decoded.sample_rate.get());
    Ok(SourceMetadata {
        backend: demuxer.backend_info(),
        container: demuxer.container().clone(),
        stream,
        mode,
        canonical,
        frame_count,
        seekable: demuxer.seekable(),
    })
}

/// The declared frame count: `stream.duration` converted into the
/// canonical rate's base — `Fact::Known` only when the container
/// declared a duration and the conversion resolves.
fn declared_frame_count(stream: &StreamInfo, sample_rate: u32) -> Fact<u64> {
    let Some(duration) = stream.stream.duration else {
        return Fact::NotDeclared;
    };
    let Ok(base) = TimeBase::sample_rate(sample_rate) else {
        return Fact::NotDeclared;
    };
    match duration.convert(base, Rounding::Floor) {
        Ok(frames) => u64::try_from(frames.value()).map_or(Fact::NotDeclared, Fact::known),
        Err(_) => Fact::NotDeclared,
    }
}
