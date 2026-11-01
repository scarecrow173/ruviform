//! `ruviform-io-wav` — pure-Rust PCM WAV backend for the
//! `ruviform-media-io` abstraction boundary (Issue #191).
//!
//! This crate owns the WAV/RIFF **output** family: the first real
//! [`Encoder`]/[`Muxer`] implementations behind the media-I/O
//! boundary. It serializes canonical planar `f32` [`AudioFrame`]s into
//! a standard little-endian RIFF/WAVE file that any compliant reader —
//! including `ruviform-io-symphonia` — can reopen and decode.
//!
//! ## Capability scope (honest declaration)
//!
//! - **Encodes/muxes**: PCM WAV only. Wire formats
//!   [`SampleFormat::U8`], [`S16`], [`S24`], [`S32`] (integer PCM,
//!   `wFormatTag` 1) and [`F32`] (IEEE float, tag 3). Mono/Stereo emit
//!   classic 16-byte `WAVEFORMATEX`; any other layout emits
//!   `WAVEFORMATEXTENSIBLE` with the layout's speaker mask.
//! - **One stream**: exactly one audio stream per file; stream time
//!   base is `1/rate` (PCM units are interleaved frames).
//! - **Classic RIFF only**: `data` chunks cap at `u32::MAX` — RF64 and
//!   BWF `bext` are explicitly `Unsupported`.
//! - **Not provided**: demux/decode (use `ruviform-io-symphonia`),
//!   dither (upstream processing), resampling, channel remapping.
//!
//! ## Stream declaration rule
//!
//! `WavEncoder::new` / `open_muxer` take a [`StreamInfo`] whose `codec`
//! follows the same shape a decoder reports: `format` = canonical
//! `Audio(F32)` (the frames the encoder consumes),
//! `native_format` = `Fact::Known(Audio(<wire>))` (the encoding the
//! file carries). Both must agree on rate and layout.
//! [`WavOutputSpec`] is the ergonomic builder for that declaration.
//!
//! ## Completion contract
//!
//! [`WavMuxer::finalize`] is the **only** checked completion path:
//! pad → size backpatch → flush → [`WavReceipt`]. `Drop` is
//! best-effort cleanup and never a receipt. Durability (fsync + rename
//! into place) belongs to
//! [`StagedOutput::publish`](ruviform_media_io::StagedOutput) — the
//! boundary's staged-publish primitive.
//!
//! ## Conversion policy
//!
//! `f32` → integer: saturate to full scale, `NaN → 0`,
//! `±Inf → ±full scale`, round half away from zero, no dither.
//! `f32` → `F32` wire: verbatim IEEE-754 (no clipping — float WAV can
//! carry out-of-range values).
//!
//! ## RT contract
//!
//! **Non-RT only** — every public function may block on I/O and
//! allocate; encode/mux/file/FS must never be called from an RT audio
//! callback.
//!
//! [`S16`]: ruviform_audio_core::SampleFormat::S16
//! [`S24`]: ruviform_audio_core::SampleFormat::S24
//! [`S32`]: ruviform_audio_core::SampleFormat::S32
//! [`F32`]: ruviform_audio_core::SampleFormat::F32
//! [`AudioFrame`]: ruviform_media_io::AudioFrame

mod convert;
mod encoder;
mod muxer;
mod wav;

pub use encoder::WavEncoder;
pub use muxer::{BoxedWavMuxer, WavMuxer, WavReceipt, open_boxed_muxer};

use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_media_core::{Fact, StreamDescriptor, StreamId, StreamKind};
use ruviform_media_io::{
    Capabilities, CodecDescriptor, CodecFormat, CodecId, Features, MediaSink, Result, StreamFacts,
    StreamInfo,
};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Format-level capability for the WAV backend — the honest,
/// queryable statement of what the RIFF/WAVE writer accepts, beyond
/// the codec granularity of [`Capabilities`].
///
/// **RT contract:** non-RT only.
#[derive(Clone, Copy, Debug)]
pub struct WavCapabilities {
    /// The single container this backend writes.
    pub container: ruviform_media_io::ContainerFormat,
    /// Wire sample encodings accepted in `native_format`.
    pub sample_formats: &'static [SampleFormat],
    /// `false` — RF64 (>4 GiB) output is `Unsupported`.
    pub rf64: bool,
    /// `false` — BWF `bext` metadata is `Unsupported`.
    pub bwf: bool,
    /// Maximum `data` chunk size (`u32::MAX`, classic RIFF).
    pub max_data_bytes: u64,
    /// Maximum streams per file — WAV output is single-stream.
    pub max_streams: usize,
    /// Whether `ChannelLayout::Custom` masks are emitted (via
    /// `WAVEFORMATEXTENSIBLE`).
    pub custom_layouts: bool,
}

/// The declared WAV capability set — queryable before construction so
/// callers can reject out-of-scope exports up front instead of at
/// first write.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SampleFormat;
///
/// let caps = ruviform_io_wav::wav_capabilities();
/// assert_eq!(caps.container, ruviform_media_io::ContainerFormat::Wav);
/// assert!(caps.sample_formats.contains(&SampleFormat::S16));
/// assert!(!caps.rf64);
/// assert!(!caps.bwf);
/// assert_eq!(caps.max_streams, 1);
/// ```
#[must_use]
pub fn wav_capabilities() -> WavCapabilities {
    WavCapabilities {
        container: ruviform_media_io::ContainerFormat::Wav,
        sample_formats: wav::WIRE_FORMATS,
        rf64: false,
        bwf: false,
        max_data_bytes: u64::from(u32::MAX),
        max_streams: 1,
        custom_layouts: true,
    }
}

/// Backend-level capability: `MUX | ENCODE` for `CodecId::Pcm` only.
/// No `DEMUX`/`DECODE` — this backend is output-only, and encode
/// capability is never inferred from decode capability.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::CodecId;
///
/// let caps = ruviform_io_wav::capabilities();
/// assert!(caps.can_mux());
/// assert!(caps.can_encode(CodecId::Pcm));
/// assert!(!caps.can_demux());
/// assert!(!caps.can_decode(CodecId::Pcm));
/// ```
#[must_use]
pub fn capabilities() -> Capabilities {
    Capabilities {
        functions: Features::MUX.union(Features::ENCODE),
        decode_codecs: Vec::new(),
        encode_codecs: vec![CodecId::Pcm],
    }
}

/// A declared WAV output target — the ergonomic way to build the
/// [`StreamInfo`] an encoder/muxer consumes.
///
/// `sample_format` is the *wire* encoding (must be in
/// [`wav_capabilities`]`().sample_formats`); canonical frames are
/// always planar `F32`.
///
/// **RT contract:** non-RT only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavOutputSpec {
    /// Samples per second.
    pub sample_rate: SampleRate,
    /// Channel layout — Mono/Stereo emit `WAVEFORMATEX`, others
    /// `WAVEFORMATEXTENSIBLE`.
    pub layout: ChannelLayout,
    /// Wire sample encoding.
    pub sample_format: SampleFormat,
}

impl WavOutputSpec {
    /// Builds the codec descriptor declaring this output format:
    /// `codec = Pcm`, `format` = canonical `Audio(F32)`,
    /// `native_format` = `Known(Audio(<wire>))`.
    ///
    /// **RT contract:** non-RT only — allocates `codec_private`'s
    /// empty `Vec`.
    #[must_use]
    pub fn codec_descriptor(&self) -> CodecDescriptor {
        CodecDescriptor {
            codec: CodecId::Pcm,
            codec_private: Vec::new(),
            format: CodecFormat::Audio(AudioFormat {
                sample_rate: self.sample_rate,
                layout: self.layout,
                sample_format: SampleFormat::F32,
            }),
            native_format: Fact::known(CodecFormat::Audio(AudioFormat {
                sample_rate: self.sample_rate,
                layout: self.layout,
                sample_format: self.sample_format,
            })),
        }
    }

    /// Builds the full stream declaration (`stream id 0`, time base
    /// `1/rate`) for [`open_muxer`]/[`WavEncoder::new`].
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn stream_info(&self) -> StreamInfo {
        StreamInfo {
            stream: StreamDescriptor {
                id: StreamId(0),
                kind: StreamKind::Audio,
                time_base: Fact::known(self.sample_rate.time_base()),
                duration: None,
            },
            codec: self.codec_descriptor(),
            facts: StreamFacts::default(),
        }
    }
}

/// Builds a [`WavEncoder`] from an output stream declaration.
///
/// See the crate-level declaration rules; the wire encoding comes from
/// `info.codec.native_format`.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// [`ruviform_media_io::ErrorKind::Unsupported`] — codec not `Pcm`,
/// canonical format not `Audio(F32)`, or wire format outside
/// `wav_capabilities`.
/// [`ruviform_media_io::ErrorKind::InvalidValue`] — malformed
/// declaration (non-audio kind, undeclared wire, rate/layout
/// disagreement).
pub fn make_encoder(info: &StreamInfo) -> Result<WavEncoder> {
    WavEncoder::new(info)
}

/// Opens a [`WavMuxer`] writing to `sink` for exactly one declared
/// audio stream, writing the header immediately.
///
/// **RT contract:** non-RT only — writes the header to the sink.
///
/// # Errors
///
/// [`ruviform_media_io::ErrorKind::Unsupported`] — anything outside
/// `wav_capabilities` (non-PCM codec, unsupported wire format, more
/// than one stream).
/// [`ruviform_media_io::ErrorKind::InvalidValue`] — malformed stream
/// declaration.
/// [`ruviform_media_io::ErrorKind::Io`] — header write failed.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};
/// use ruviform_io_wav::WavOutputSpec;
///
/// let spec = WavOutputSpec {
///     sample_rate: SampleRate::new(48_000).unwrap(),
///     layout: ChannelLayout::Stereo,
///     sample_format: SampleFormat::S16,
/// };
/// let cursor = std::io::Cursor::new(Vec::new());
/// let mut muxer = ruviform_io_wav::open_muxer(cursor, &[spec.stream_info()])?;
/// ruviform_media_io::Muxer::finish(&mut muxer)?; // checked finalize
/// # Ok::<(), ruviform_media_io::Error>(())
/// ```
pub fn open_muxer<S: MediaSink>(sink: S, streams: &[StreamInfo]) -> Result<WavMuxer<S>> {
    WavMuxer::new(sink, streams)
}
