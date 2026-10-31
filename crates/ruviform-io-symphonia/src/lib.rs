//! `ruviform-io-symphonia` — the first backend proof of concept for the
//! `ruviform-media-io` abstraction boundary: demux + decode only.
//!
//! This crate is the reference implementation proving the backend
//! abstraction (Issue #20, ADR `docs/adr/media-io-backend-abstraction.md`):
//! it implements the boundary's [`Demuxer`] and [`Decoder`] traits on top
//! of a pure-Rust media backend, while `Muxer`/`Encoder` are exercised
//! through capability negotiation — the query half
//! ([`capabilities`]) reports demux/decode only, and the request half
//! ([`make_muxer`]/[`make_encoder`]) returns a normalized
//! [`ErrorKind::Unsupported`].
//!
//! ## Type purity
//!
//! The public surface exposes only Ruviform abstractions. No backend
//! type, error, or feature name appears in any public signature: the
//! dependency is a private implementation detail (`cargo tree -p
//! ruviform-media-io` must show no edge to it), backend errors are
//! normalized to [`Error`] with the detail kept only as a type-erased
//! `source`, and there are no re-exports of external types.
//!
//! ## RT contract
//!
//! **Non-RT only.** The whole crate sits on the I/O/decode side of the
//! thread boundary: probing, packet reads, seeks and decode may block on
//! I/O, allocate and do unbounded codec work. Per-API labels read
//! `**RT contract:** non-RT only`. The data types crossing the boundary
//! (`Packet`, `Frame`, `StreamInfo`, `Error`) are `Send` so they can
//! travel to the processing side through bounded queues.
//!
//! ## Backend feature scope
//!
//! Default backend features only — the royalty-free codec/format set
//! (WAV/PCM, FLAC, Vorbis/Ogg, Matroska, ADPCM). Container probing is
//! format-agnostic: whatever the default probe detects gets demuxed and
//! stream-enumerated; decode succeeds when the registry holds an audio
//! decoder for the stream's codec. Capability advertisement
//! ([`capabilities`]) intentionally under-claims: it lists only codecs
//! the boundary [`CodecId`] registry names *and* the default build
//! actually registers — ADPCM decodes but is not advertised since the
//! Ruviform registry has no `Adpcm` variant and claiming `Other` would
//! over-claim.
//!
//! ## Known limitations
//!
//! `PoC` scope, recorded as follow-ups rather than hidden:
//!
//! - Chained Ogg (concatenated physical streams, e.g. Icecast) ends
//!   playback at the first chain link: the backend signals
//!   `ResetRequired`, which normalizes to [`ErrorKind::InvalidData`].
//!   Re-enumerating streams mid-stream conflicts with the snapshot
//!   `streams()` model and is deferred.
//! - Codec-level trim (`trim_start`/`trim_end`) crosses the boundary
//!   on packets and stream facts, but applying it to canonical decoded
//!   output is caller-side work — the decoder does not drop samples.
//! - Unseekable sources are reported honestly via the adapter's
//!   construction-time probe; `Demuxer::seek` then surfaces
//!   [`ErrorKind::Unsupported`] per contract.
//!
//! [`Demuxer`]: ruviform_media_io::Demuxer
//! [`Decoder`]: ruviform_media_io::Decoder
//! [`Muxer`]: ruviform_media_io::Muxer
//! [`Error`]: ruviform_media_io::Error
//! [`CodecId`]: ruviform_media_io::CodecId
//! [`Packet`]: ruviform_media_io::Packet
//! [`Frame`]: ruviform_media_io::Frame
//! [`StreamInfo`]: ruviform_media_io::StreamInfo

// The backend type names (`SymphoniaDemuxer`, `SymphoniaDecoder`)
// deliberately echo the crate identity — they are Ruviform types, not
// re-exports, so the repetition lint does not apply.
#![allow(clippy::module_name_repetitions)]

mod decoder;
mod demuxer;
mod error;
mod map;
mod source;

pub use decoder::SymphoniaDecoder;
pub use demuxer::SymphoniaDemuxer;

use ruviform_media_io::{
    Capabilities, CodecId, Encoder, Error, ErrorKind, Features, MediaSource, Muxer, Result,
};
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

use crate::error::map_error;
use crate::source::{Sniffed, SniffingSource, SourceAdapter};

/// The crate version — for diagnostics and support matrices.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// assert_eq!(ruviform_io_symphonia::version(), env!("CARGO_PKG_VERSION"));
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The query half of capability negotiation.
///
/// Reports [`Features::DEMUX`] | [`Features::DECODE`]; mux and encode are
/// absent — the backend provides no writer/encoder infrastructure.
/// `decode_codecs` lists the boundary [`CodecId`]s the default backend
/// build both registers *and* the Ruviform registry names:
/// [`CodecId::Pcm`], [`CodecId::Flac`], [`CodecId::Vorbis`]. Decodable
/// codecs without a registry name (ADPCM, …) are deliberately not
/// advertised — see the crate docs.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::CodecId;
///
/// let caps = ruviform_io_symphonia::capabilities();
/// assert!(caps.can_demux());
/// assert!(caps.can_decode(CodecId::Pcm));
/// assert!(!caps.can_mux());
/// assert!(!caps.can_encode(CodecId::Pcm));
/// ```
#[must_use]
pub fn capabilities() -> Capabilities {
    Capabilities {
        functions: Features::DEMUX
            .union(Features::DECODE)
            .union(Features::NATIVE_AUDIO),
        decode_codecs: {
            let codecs = vec![CodecId::Pcm, CodecId::Flac, CodecId::Vorbis];
            #[cfg(feature = "aac")]
            let codecs = {
                let mut codecs = codecs;
                codecs.push(CodecId::Aac);
                codecs
            };
            codecs
        },
        encode_codecs: Vec::new(),
    }
}

/// Opens a [`Demuxer`](ruviform_media_io::Demuxer) over a media-io
/// [`MediaSource`] byte stream.
///
/// The source is adapted to the backend's own `MediaSource` trait
/// (an internal concern — the adapter never appears in the public API)
/// and probed for a container format; stream metadata is enumerated
/// into boundary [`StreamInfo`] values at construction.
///
/// **RT contract:** non-RT only — probing performs I/O, including a
/// trailing-metadata scan to end-of-source.
///
/// # Errors
///
/// - [`ErrorKind::InvalidData`] — the bytes do not match any supported
///   container, or the container structure is corrupt.
/// - [`ErrorKind::UnexpectedEos`] — truncated input.
/// - [`ErrorKind::Io`] — the source's `Read`/`Seek` failed.
/// - [`ErrorKind::Unsupported`] — the container's codec needs a backend
///   feature the default build does not include.
///
/// [`StreamInfo`]: ruviform_media_io::StreamInfo
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// // Not a container — probing fails cleanly with a normalized error,
/// // never a panic.
/// let result = ruviform_io_symphonia::open_demuxer(Cursor::new(vec![0u8; 32]));
/// assert!(result.is_err());
/// ```
pub fn open_demuxer(source: impl MediaSource + 'static) -> Result<SymphoniaDemuxer> {
    let sniffed = std::sync::Arc::new(std::sync::Mutex::new(Sniffed::new()));
    let mss = MediaSourceStream::new(
        Box::new(SniffingSource::new(
            SourceAdapter::new(Box::new(source)),
            sniffed.clone(),
        )),
        MediaSourceStreamOptions::default(),
    );
    let reader = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(map_error)?;
    let container = sniffed
        .lock()
        .map(|sniffed| map::container_info(sniffed.prefix()))
        .unwrap_or_default();
    Ok(SymphoniaDemuxer::new(reader, container))
}

/// Requests a [`Muxer`] from this backend — always reports unsupported.
///
/// The request half of capability negotiation, mirroring
/// [`capabilities`]' absent [`Features::MUX`]. The backend has no writer
/// infrastructure at all, so there is no honest input signature: the
/// function takes no arguments and always fails with
/// [`ErrorKind::Unsupported`]. Never panics.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// Always [`ErrorKind::Unsupported`].
///
/// # Examples
///
/// ```
/// use ruviform_media_io::ErrorKind;
///
/// let error = ruviform_io_symphonia::make_muxer().err().unwrap();
/// assert_eq!(error.kind(), ErrorKind::Unsupported);
/// ```
pub fn make_muxer() -> Result<Box<dyn Muxer>> {
    Err(Error::from_kind(ErrorKind::Unsupported))
}

/// Requests an [`Encoder`] from this backend — always reports
/// unsupported.
///
/// The request half of capability negotiation, mirroring
/// [`capabilities`]' absent [`Features::ENCODE`]. Always fails with
/// [`ErrorKind::Unsupported`]. Never panics.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// Always [`ErrorKind::Unsupported`].
///
/// # Examples
///
/// ```
/// use ruviform_media_io::ErrorKind;
///
/// let error = ruviform_io_symphonia::make_encoder().err().unwrap();
/// assert_eq!(error.kind(), ErrorKind::Unsupported);
/// ```
pub fn make_encoder() -> Result<Box<dyn Encoder>> {
    Err(Error::from_kind(ErrorKind::Unsupported))
}
