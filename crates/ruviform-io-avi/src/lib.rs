//! `ruviform-io-avi` — pure-Rust AVI backend for the
//! `ruviform-media-io` abstraction boundary (Issue #154).
//!
//! This crate owns the AVI/DIB input family `AudioLens` consumes — the
//! formats `ruviform-io-symphonia` cannot cover because Symphonia has
//! no RIFF-`AVI ` format reader at all: streaming demux of
//! `LIST movi` interleaved audio/video chunks, WAVEFORMATEX PCM audio
//! decode (canonical planar `f32` **and** the Issue-#151 non-destructive
//! native-scale `f64` path), and `BI_RGB` DIB video decode to
//! `Rgba8` [`ruviform_video_core::VideoFrame`].
//!
//! ## Capability scope (honest declaration)
//!
//! - **Decodes**: `auds` PCM `wFormatTag` 1 at 8/16/24/32-bit and
//!   IEEE-float tag 3 at 32/64-bit (interleaved wire bodies → planar or
//!   interleaved `f64`); `vids` uncompressed `BI_RGB` DIB at 24/32 bpp
//!   (BGR(A) swizzle + bottom-up flip → `Rgba8`).
//! - **Reports but does not decode**: compressed `vids` fourCCs keep
//!   their verbatim `CodecTag::FourCc` (`MJPG`→[`CodecId::Mjpeg`],
//!   `H264` family→[`CodecId::H264`], others→[`CodecId::Other`]) and
//!   `decodable = Fact::Unsupported`; non-PCM/odd-bit WAVEFORMATEX
//!   encodings likewise. Malformed headers surface as
//!   `Fact::Malformed`, never as successful decoders.
//! - **Explicitly unsupported**: mux/encode (constructors return
//!   `ErrorKind::Unsupported`), `Demuxer::seek` (no `idx1` consult),
//!   `OpenDML` nested `LIST rec ` multi-segment AVI, `idx1` index chunks
//!   (skipped), `WAVE_FORMAT_EXTENSIBLE` masks, palette/RLE DIBs.
//!
//! ## Type purity
//!
//! The public surface is only the boundary vocabulary — `AviDemuxer`,
//! `AviDecoder`, `open_demuxer`, `capabilities`. The parsers are
//! private and the crate has no external dependencies.

mod decoder;
mod demuxer;
mod format;
mod riff;

pub use decoder::AviDecoder;
pub use demuxer::AviDemuxer;

use ruviform_media_io::{
    BackendInfo, Capabilities, CodecId, Encoder, Error, ErrorKind, Features, MediaSource, Muxer,
};

/// A `&'static str` backend diagnostic — carried through
/// [`Error::with_source`] so operators can see *why* a probe/decode
/// failed while the stable `ErrorKind` stays the matchable surface.
#[derive(Debug)]
struct Detail(&'static str);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Detail {}

/// Builds a boundary [`Error`] carrying a static diagnostic detail.
pub(crate) fn err(kind: ErrorKind, detail: &'static str) -> Error {
    Error::with_source(kind, Detail(detail))
}

/// Backend crate version for `BackendInfo`.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Backend identity for `Demuxer::backend_info`.
pub(crate) fn backend_info() -> BackendInfo {
    BackendInfo {
        name: "ruviform-io-avi",
        version: version(),
    }
}

/// The capability query side: demux + decode + native audio for AVI —
/// mux/encode absent.
///
/// Decode codecs: [`CodecId::Pcm`] (WAVEFORMATEX PCM/IEEE-float bodies)
/// and [`CodecId::Uncompressed`] (`BI_RGB` DIB → `Rgba8`). Container
/// scope is expressed at probe (`open_demuxer` accepts RIFF `AVI `
/// only) and through `ContainerInfo::format` — this backend's entire
/// demux scope is the AVI family.
#[must_use]
pub fn capabilities() -> Capabilities {
    Capabilities {
        functions: Features::DEMUX
            .union(Features::DECODE)
            .union(Features::NATIVE_AUDIO),
        decode_codecs: vec![CodecId::Pcm, CodecId::Uncompressed],
        encode_codecs: vec![],
    }
}

/// Opens `source` as a RIFF `AVI ` container.
///
/// # Errors
///
/// - [`ErrorKind::Unsupported`] — not a RIFF `AVI ` form, or the `movi`
///   region cannot be reached on a non-seekable source.
/// - [`ErrorKind::InvalidData`] — RIFF size mismatch, malformed chunk
///   framing, or no `movi` list.
/// - [`ErrorKind::Io`] — underlying source failure.
///
/// **RT contract:** non-RT only — reads and may allocate.
pub fn open_demuxer(source: impl MediaSource + 'static) -> Result<AviDemuxer, Error> {
    AviDemuxer::open(Box::new(source))
}

/// Requests a [`Muxer`] from this backend — always reports unsupported.
///
/// # Errors
///
/// Always [`ErrorKind::Unsupported`].
pub fn make_muxer() -> Result<Box<dyn Muxer>, Error> {
    Err(Error::from_kind(ErrorKind::Unsupported))
}

/// Requests an [`Encoder`] from this backend — always reports
/// unsupported.
///
/// # Errors
///
/// Always [`ErrorKind::Unsupported`].
pub fn make_encoder() -> Result<Box<dyn Encoder>, Error> {
    Err(Error::from_kind(ErrorKind::Unsupported))
}
