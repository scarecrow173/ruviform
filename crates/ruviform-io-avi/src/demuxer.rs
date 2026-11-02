//! [`Demuxer`] implementation for AVI (RIFF `AVI ` form).
//!
//! Probe parses the top-level RIFF chunk list once: `LIST hdrl` yields
//! one boundary stream per `LIST strl`, and `LIST movi` is retained as a
//! byte region that `read_packet` walks lazily — bodies of non-emitted
//! chunks are skipped (by seek when the source is seekable), never
//! materialized.
//!
//! Timestamps: AVI `movi` chunks carry no per-packet timing, so the
//! demuxer reproduces the classic AVI clock — per-stream `1/dwRate`
//! ticks, the declared `dwStart * dwScale` origin, advanced by `dwScale`
//! ticks per non-audio chunk and by `body_bytes * dwScale /
//! dwSampleSize` ticks (with a persistent fractional remainder) on
//! `dwSampleSize` streams — the same derivation `AudioLens` `al-media`
//! proved on its fixture matrix. All packet PTS/DTS leave the demuxer
//! as [`TimestampProvenance::Derived`]. `idx1` seeking, `OpenDML` `rec `
//! segments and `strn`/`wLanguage` facts are declared gaps, never
//! fabricated.

use std::io::SeekFrom;

use ruviform_media_core::{
    Fact, FrameRate, MediaDuration, MediaTime, StreamDescriptor, StreamId, StreamKind,
    TickTimestamp, TimeBase, TimestampProvenance,
};
use ruviform_media_io::{
    CodecDescriptor, CodecFormat, CodecId, CodecTag, ContainerFormat, ContainerInfo, DecodeMode,
    Decoder, Demuxer, Error, ErrorKind, MediaSource, Packet, PacketFlags, StreamFacts, StreamInfo,
    VideoStreamFacts,
};
use ruviform_video_core::{ColorInfo, PixelFormat, VideoFormat};

use crate::decoder::{AudioMode, AviDecoder, StreamParams};
use crate::format::{
    DibClass, StreamHead, WaveClass, dib_codec_id, dib_header, stream_head, wave_format,
};
use crate::riff::{self, ChunkHead, FORM_AVI, LIST, RIFF};
use crate::{backend_info, err};

/// `LIST` form types the scanner recognizes.
const HDRL: [u8; 4] = *b"hdrl";
const STRL: [u8; 4] = *b"strl";
const MOVI: [u8; 4] = *b"movi";
const REC: [u8; 4] = *b"rec ";

/// `strh`/`strf` chunk ids inside `LIST strl`.
const STRH: [u8; 4] = *b"strh";
const STRF: [u8; 4] = *b"strf";

/// `strh` `fccType` values.
const VIDS: [u8; 4] = *b"vids";
const AUDS: [u8; 4] = *b"auds";

/// Per-stream playback state — the boundary snapshot lives in
/// `AviDemuxer::streams`; this record keeps what only the backend needs
/// (`strh` timing fields and the probe-time decode classification).
struct StreamState {
    head: Option<StreamHead>,
    /// Cached decode classification so `make_decoder` agrees exactly
    /// with the reported `decodable` fact.
    params: StreamParams,
}

/// Per-stream derived-clock state for `movi` playback.
struct Clock {
    /// Next packet's start tick (`dwStart * dwScale` initially).
    next: u64,
    /// Fractional `(bytes * scale) % sample_size` carry between chunks.
    rem: u128,
}

/// AVI demuxer — streams packets in `LIST movi` order.
pub struct AviDemuxer {
    source: Box<dyn MediaSource>,
    container: ContainerInfo,
    /// Probed boundary snapshots — `streams()` borrows this slice.
    streams: Vec<StreamInfo>,
    /// Parallel per-stream backend state (same index as `streams`).
    state: Vec<StreamState>,
    movi_end: u64,
    /// Current scan position inside the movi region.
    pos: u64,
    /// Per-stream derived clocks and emitted-packet ordinals.
    clocks: Vec<Clock>,
    ordinals: Vec<u64>,
}

impl AviDemuxer {
    /// Probes `source` as RIFF `AVI ` and parses `hdrl` + the `movi`
    /// region bounds.
    pub(crate) fn open(source: Box<dyn MediaSource>) -> Result<Self, Error> {
        let mut source = source;
        let mut header = [0u8; 12];
        if source.read_exact(&mut header).is_err() {
            return Err(err(
                ErrorKind::Unsupported,
                "avi: too short for a RIFF AVI header",
            ));
        }
        if header[0..4] != RIFF || header[8..12] != FORM_AVI {
            return Err(err(ErrorKind::Unsupported, "avi: not a RIFF AVI form"));
        }
        let riff_size = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap()));
        let riff_end = 8 + riff_size;

        // Strict container-size check (al-media parity): when the
        // source length is knowable, `RIFF` size must equal
        // `byte_len - 8`. Mismatch is corruption, not a capability gap.
        if let Some(byte_len) = byte_len(source.as_mut())
            && (byte_len < 8 || riff_size != byte_len - 8)
        {
            return Err(err(
                ErrorKind::InvalidData,
                "avi: RIFF size does not match source length",
            ));
        }

        // Scan top-level chunks for `hdrl` (streams) and `movi`.
        let mut streams: Vec<StreamInfo> = Vec::new();
        let mut state: Vec<StreamState> = Vec::new();
        let mut hdrl_seen = false;
        let mut movi: Option<(u64, u64)> = None;
        let mut pos = 12;
        while pos < riff_end && (movi.is_none() || !hdrl_seen) {
            let Some(head) = riff::read_head_at(&mut *source, riff_end, pos)? else {
                break;
            };
            let mut cur = pos + 8;
            if head.id == LIST && head.size >= 4 {
                let mut ty = [0u8; 4];
                riff::read_body(&mut *source, &mut ty)?;
                cur += 4;
                match ty {
                    HDRL => {
                        (streams, state) = parse_hdrl(&mut *source, cur, pos + 8 + head.size)?;
                        // The sub-parser consumed the whole list body —
                        // only the pad byte may remain.
                        cur = pos + 8 + head.size;
                        hdrl_seen = true;
                    }
                    MOVI => {
                        movi = Some((cur, pos + 8 + head.size));
                        if hdrl_seen {
                            // `hdrl` first (the standard layout): the
                            // cursor already sits at the movi body —
                            // stop scanning without skipping it, so a
                            // non-seekable source still demuxes.
                            pos = cur;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            skip_to(&mut *source, cur, head.next_off(pos))
                .map_err(|()| err(ErrorKind::Io, "avi: top-level chunk skip failed"))?;
            pos = head.next_off(pos);
        }
        let Some((movi_begin, movi_end)) = movi else {
            return Err(err(ErrorKind::InvalidData, "avi: no movi list"));
        };

        // A `movi`-before-`hdrl` layout needs a real seek back to the
        // movi body — non-seekable sources cannot rewind, which is an
        // honest capability failure rather than corruption.
        if pos != movi_begin && source.seek(SeekFrom::Start(movi_begin)).is_err() {
            return Err(err(
                ErrorKind::Unsupported,
                "avi: source cannot seek to the movi region",
            ));
        }

        let clocks = state
            .iter()
            .map(|stream| Clock {
                next: stream
                    .head
                    .map_or(0, |head| u64::from(head.start) * u64::from(head.scale)),
                rem: 0,
            })
            .collect();
        let ordinals = vec![0; state.len()];
        Ok(Self {
            source,
            container: ContainerInfo {
                format: ContainerFormat::Avi,
                detail: None,
            },
            streams,
            state,
            movi_end,
            pos: movi_begin,
            clocks,
            ordinals,
        })
    }

    /// Canonical decoder over stream `id` — planar `f32` audio or
    /// `Rgba8` video frames.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidValue`] — unknown stream id.
    /// - [`ErrorKind::Unsupported`] — the stream's declared codec is
    ///   outside this backend's decode set.
    /// - [`ErrorKind::InvalidData`] — malformed stream headers.
    ///
    /// **RT contract:** non-RT only.
    pub fn make_decoder(&self, id: StreamId) -> Result<AviDecoder, Error> {
        self.make_decoder_in(id, false)
    }

    /// Issue-#151 non-destructive decoder over stream `id` —
    /// interleaved `f64` samples in the container's native PCM scale
    /// ([`crate::AviDecoder`] emits `Frame::NativeAudio`). Audio-only:
    /// a video stream reports [`ErrorKind::Unsupported`].
    ///
    /// # Errors
    ///
    /// Same as [`make_decoder`](Self::make_decoder).
    ///
    /// **RT contract:** non-RT only.
    pub fn make_native_decoder(&self, id: StreamId) -> Result<AviDecoder, Error> {
        self.make_decoder_in(id, true)
    }

    /// Boundary decoder over stream `id`. `native` selects the
    /// Issue-#151 non-destructive `f64` path (audio-only).
    fn make_decoder_in(&self, id: StreamId, native: bool) -> Result<AviDecoder, Error> {
        let index = self
            .state
            .iter()
            .zip(&self.streams)
            .position(|(_, info)| info.stream.id == id)
            .ok_or_else(|| err(ErrorKind::InvalidValue, "avi: unknown stream id"))?;
        let base = self.streams[index].stream.time_base.get().copied();
        match (self.state[index].params, native) {
            (StreamParams::Audio(pcm), _) => Ok(AviDecoder::audio(
                id,
                pcm,
                base,
                if native {
                    AudioMode::Native
                } else {
                    AudioMode::Canonical
                },
            )),
            (StreamParams::Video(dib), false) => Ok(AviDecoder::video(id, dib, base)),
            (StreamParams::Video(_), true) => Err(err(
                ErrorKind::Unsupported,
                "avi: the native `f64` path is audio-only",
            )),
            (StreamParams::Undecodable, _) => Err(err(
                ErrorKind::Unsupported,
                "avi: stream codec/format is not decodable",
            )),
            (StreamParams::Malformed, _) => Err(err(
                ErrorKind::InvalidData,
                "avi: stream headers are malformed",
            )),
        }
    }
}

impl Demuxer for AviDemuxer {
    fn container(&self) -> &ContainerInfo {
        &self.container
    }

    fn backend_info(&self) -> ruviform_media_io::BackendInfo {
        backend_info()
    }

    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// Walks the `movi` region. Media chunks (`NNwb` on `auds`,
    /// `NNdb`/`NNdc` on `vids`) become packets carrying derived ticks;
    /// every digit-prefixed chunk on a declared stream advances that
    /// stream's clock whether or not it is emitted; all other chunks
    /// are skipped without materializing their bodies.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidData`] — malformed chunk framing inside
    ///   `movi`.
    /// - [`ErrorKind::Unsupported`] — an `OpenDML` nested `LIST rec `
    ///   (multi-segment AVI this backend does not implement).
    /// - [`ErrorKind::Io`] — underlying source failure.
    fn read_packet(&mut self) -> Result<Option<Packet>, Error> {
        loop {
            if self.pos >= self.movi_end {
                return Ok(None);
            }
            let Some(head) = riff::read_head_at(&mut *self.source, self.movi_end, self.pos)? else {
                return Ok(None);
            };
            let next = head.next_off(self.pos);

            // A nested LIST inside `movi`: `rec ` marks OpenDML
            // multi-segment AVI (unsupported); any other list is
            // skipped leniently.
            if head.id == LIST {
                let mut cur = self.pos + 8;
                if head.size >= 4 {
                    let mut ty = [0u8; 4];
                    riff::read_body(&mut *self.source, &mut ty)?;
                    cur += 4;
                    if ty == REC {
                        return Err(err(
                            ErrorKind::Unsupported,
                            "avi: OpenDML nested `rec ` list",
                        ));
                    }
                }
                skip_to(&mut *self.source, cur, next)
                    .map_err(|()| err(ErrorKind::Io, "avi: list skip failed"))?;
                self.pos = next;
                continue;
            }

            if let Some(index) = head.stream_chunk() {
                let index = index as usize;
                if let Some(stream) = self.state.get(index) {
                    let advance = tick_advance(head, stream.head, &mut self.clocks[index]);
                    let start = advance_clock(&mut self.clocks[index], advance)?;
                    if emits(head, stream.head) {
                        let mut data = vec![
                            0u8;
                            usize::try_from(head.size).map_err(|_| {
                                err(ErrorKind::InvalidData, "avi: chunk body overflows usize")
                            })?
                        ];
                        riff::read_body(&mut *self.source, &mut data)?;
                        skip_to(&mut *self.source, self.pos + 8 + head.size, next).ok();
                        self.pos = next;
                        let ordinal = self.ordinals[index];
                        self.ordinals[index] += 1;
                        let ticks = Some(TickTimestamp {
                            ticks: i64::try_from(start)
                                .map_err(|_| Error::from_kind(ErrorKind::TimeOverflow))?,
                            provenance: TimestampProvenance::Derived,
                        });
                        // A BI_RGB DIB frame is independently decodable
                        // — every emitted video packet is a keyframe for
                        // the codecs this backend decodes.
                        let flags = if matches!(stream.params, StreamParams::Video(_)) {
                            PacketFlags::KEYFRAME
                        } else {
                            PacketFlags::NONE
                        };
                        return Ok(Some(Packet {
                            stream: StreamId(u32::try_from(index).unwrap_or(u32::MAX)),
                            index: ordinal,
                            pts: ticks,
                            dts: ticks,
                            duration: Some(advance),
                            trim_start: 0,
                            trim_end: 0,
                            flags,
                            data,
                        }));
                    }
                    skip_to(&mut *self.source, self.pos + 8, next)
                        .map_err(|()| err(ErrorKind::Io, "avi: chunk skip failed"))?;
                    self.pos = next;
                    continue;
                }
            }

            // Non-digit chunk or a digit chunk on an undeclared stream.
            skip_to(&mut *self.source, self.pos + 8, next)
                .map_err(|()| err(ErrorKind::Io, "avi: chunk skip failed"))?;
            self.pos = next;
        }
    }

    /// AVI seek is not implemented: without consulting `idx1` the
    /// demuxer cannot map a `MediaTime` onto a `movi` offset, and a
    /// fabricated linear scan would misreport capability.
    fn seek(&mut self, _position: MediaTime) -> Result<(), Error> {
        Err(err(
            ErrorKind::Unsupported,
            "avi: seeking is not implemented",
        ))
    }

    /// Seek is declared `Unsupported` up front — the same capability
    /// `seek` reports, as a construction-time fact.
    fn seekable(&self) -> Fact<()> {
        Fact::Unsupported
    }

    /// Builds a boundary [`Decoder`] for stream `id` in `mode` —
    /// `Canonical` dispatches to [`make_decoder`](Self::make_decoder)
    /// and `Native` to [`make_native_decoder`](Self::make_native_decoder).
    fn make_decoder(&self, id: StreamId, mode: DecodeMode) -> Result<Box<dyn Decoder>, Error> {
        match mode {
            DecodeMode::Canonical => Ok(Box::new(self.make_decoder(id)?)),
            DecodeMode::Native => Ok(Box::new(self.make_native_decoder(id)?)),
            _ => Err(err(ErrorKind::Unsupported, "avi: unknown decode mode")),
        }
    }
}

/// Applies `advance` to the clock, returning the packet's start tick.
fn advance_clock(clock: &mut Clock, advance: u64) -> Result<u64, Error> {
    let start = clock.next;
    clock.next = clock
        .next
        .checked_add(advance)
        .ok_or_else(|| Error::from_kind(ErrorKind::TimeOverflow))?;
    Ok(start)
}

/// Tick advance for one `movi` chunk on a declared stream — the
/// al-media derivation: `body_bytes * dwScale / dwSampleSize` on
/// `dwSampleSize` streams (fractional remainder persisted), `dwScale`
/// ticks otherwise.
fn tick_advance(head: ChunkHead, stream: Option<StreamHead>, clock: &mut Clock) -> u64 {
    let Some(stream) = stream else { return 0 };
    if stream.sample_size != 0 {
        let numerator = u128::from(head.size) * u128::from(stream.scale) + clock.rem;
        clock.rem = numerator % u128::from(stream.sample_size);
        u64::try_from(numerator / u128::from(stream.sample_size)).unwrap_or(u64::MAX)
    } else {
        u64::from(stream.scale)
    }
}

/// `true` when a `movi` chunk is a media packet the boundary emits:
/// `wb` on `auds`, `db`/`dc` on `vids`.
fn emits(head: ChunkHead, stream: Option<StreamHead>) -> bool {
    let kind = &head.id[2..];
    match stream.map(|stream| stream.fcc_type) {
        Some(AUDS) => kind == b"wb",
        Some(VIDS) => kind == b"db" || kind == b"dc",
        _ => false,
    }
}

/// Source byte length when the position can be discovered — `None`
/// on non-seekable sources.
fn byte_len(source: &mut dyn MediaSource) -> Option<u64> {
    let pos = source.stream_position().ok()?;
    let end = source.seek(SeekFrom::End(0)).ok()?;
    source.seek(SeekFrom::Start(pos)).ok()?;
    Some(end)
}

/// Repositions the source from cursor `cur` to absolute offset `end` —
/// one seek on seekable sources, discard reads otherwise. `cur` is the
/// caller-tracked read position (never queried, since `stream_position`
/// itself requires seek).
fn skip_to(source: &mut dyn MediaSource, cur: u64, end: u64) -> Result<(), ()> {
    if cur == end {
        return Ok(());
    }
    if source.seek(SeekFrom::Start(end)).is_ok() {
        return Ok(());
    }
    if end <= cur {
        return Err(());
    }
    riff::skip(source, end - cur).map_err(|_| ())
}

/// Parses `LIST hdrl`'s inner chunks — one boundary snapshot + backend
/// state per `LIST strl`.
fn parse_hdrl(
    source: &mut dyn MediaSource,
    mut pos: u64,
    end: u64,
) -> Result<(Vec<StreamInfo>, Vec<StreamState>), Error> {
    let mut infos = Vec::new();
    let mut states = Vec::new();
    while let Some(head) = riff::read_head_at(source, end, pos)? {
        let mut cur = pos + 8;
        if head.id == LIST && head.size >= 4 {
            let mut ty = [0u8; 4];
            riff::read_body(source, &mut ty)?;
            cur += 4;
            if ty == STRL {
                let index = u32::try_from(infos.len())
                    .map_err(|_| err(ErrorKind::InvalidData, "avi: too many streams"))?;
                let (info, state) = parse_strl(source, cur, pos + 8 + head.size, index)?;
                cur = pos + 8 + head.size; // sub-parser consumed the body
                infos.push(info);
                states.push(state);
            }
        }
        skip_to(source, cur, head.next_off(pos))
            .map_err(|()| err(ErrorKind::Io, "avi: hdrl chunk skip failed"))?;
        pos = head.next_off(pos);
    }
    Ok((infos, states))
}

/// Parses `LIST strl`'s `strh` + `strf` into the boundary snapshot.
fn parse_strl(
    source: &mut dyn MediaSource,
    mut pos: u64,
    end: u64,
    index: u32,
) -> Result<(StreamInfo, StreamState), Error> {
    let mut head: Option<StreamHead> = None;
    let mut head_malformed = false;
    let mut format = Vec::new();
    while let Some(chunk) = riff::read_head_at(source, end, pos)? {
        let mut cur = pos + 8;
        match chunk.id {
            STRH => {
                let mut body = vec![
                    0u8;
                    usize::try_from(chunk.size).map_err(|_| {
                        err(ErrorKind::InvalidData, "avi: `strh` body overflows usize")
                    })?
                ];
                riff::read_body(source, &mut body)?;
                cur += chunk.size;
                match stream_head(&body) {
                    Some(parsed) => head = Some(parsed),
                    None => head_malformed = true,
                }
            }
            STRF => {
                format = vec![
                    0u8;
                    usize::try_from(chunk.size).map_err(|_| {
                        err(ErrorKind::InvalidData, "avi: `strf` body overflows usize")
                    })?
                ];
                riff::read_body(source, &mut format)?;
                cur += chunk.size;
            }
            _ => {}
        }
        skip_to(source, cur, chunk.next_off(pos))
            .map_err(|()| err(ErrorKind::Io, "avi: strl chunk skip failed"))?;
        pos = chunk.next_off(pos);
    }
    Ok(build_stream(index, head, head_malformed, &format))
}

/// Assembles the boundary [`StreamInfo`]/[`StreamState`] from parsed
/// `strh`/`strf` — the honest per-stream capability declaration.
fn build_stream(
    index: u32,
    head: Option<StreamHead>,
    head_malformed: bool,
    format: &[u8],
) -> (StreamInfo, StreamState) {
    let (kind, params, codec, codec_tag) = match head.map(|head| head.fcc_type) {
        Some(VIDS) => {
            let (params, codec, codec_tag) = video_codec(format);
            (StreamKind::Video, params, codec, codec_tag)
        }
        Some(AUDS) => {
            let (params, codec, codec_tag) = audio_codec(format);
            (StreamKind::Audio, params, codec, codec_tag)
        }
        _ => (
            StreamKind::Data,
            if head_malformed {
                StreamParams::Malformed
            } else {
                StreamParams::Undecodable
            },
            unknown_codec(format),
            None,
        ),
    };
    let decodable = match params {
        StreamParams::Audio(_) | StreamParams::Video(_) => Fact::Known(()),
        StreamParams::Undecodable => Fact::Unsupported,
        StreamParams::Malformed => Fact::Malformed,
    };
    let base = head
        .filter(|head| head.rate != 0)
        .and_then(|head| TimeBase::new(1, head.rate).ok());
    let video = match params {
        StreamParams::Video(dib) => Some(VideoStreamFacts {
            width: Some(dib.frame_width()),
            height: Some(dib.frame_height()),
            nominal_frame_rate: head
                .filter(|head| head.scale != 0 && head.rate != 0)
                .and_then(|head| FrameRate::new(head.rate, head.scale).ok()),
        }),
        _ => None,
    };
    let info = StreamInfo {
        stream: StreamDescriptor {
            id: StreamId(index),
            kind,
            time_base: base.map_or(Fact::NotDeclared, Fact::Known),
            duration: base.and_then(|base| {
                head.and_then(|head| {
                    i64::try_from(u64::from(head.length) * u64::from(head.scale))
                        .ok()
                        .map(|value| MediaDuration::new(value, base))
                })
            }),
        },
        codec,
        facts: StreamFacts {
            index,
            container_id: None,
            codec_tag,
            language: None,
            start: head.map_or(Fact::NotDeclared, |head| {
                i64::try_from(u64::from(head.start) * u64::from(head.scale))
                    .map_or(Fact::Malformed, Fact::Known)
            }),
            declared_units: head.map_or(Fact::NotDeclared, |head| {
                Fact::Known(u64::from(head.length))
            }),
            leading_trim: None,
            trailing_trim: None,
            video,
            decodable,
        },
    };
    (info, StreamState { head, params })
}

/// Fallback descriptor for data/malformed streams — the `strf` body is
/// still carried verbatim via `codec_private`.
fn unknown_codec(format: &[u8]) -> CodecDescriptor {
    CodecDescriptor {
        codec: CodecId::Other,
        codec_private: format.to_vec(),
        format: CodecFormat::Data,
        native_format: Fact::NotDeclared,
    }
}

/// `vids` `strf` → (`StreamParams`, [`CodecDescriptor`]) — `BI_RGB`
/// DIBs are decodable (boundary `Rgba8` target; the native BGR(A)
/// encoding cannot be named by the pixel-format model, so
/// `native_format = Unsupported` rather than a relabel). Nonzero
/// `biCompression` keeps its verbatim fourCC tag but is reported
/// not-decodable.
fn video_codec(format: &[u8]) -> (StreamParams, CodecDescriptor, Option<CodecTag>) {
    let tag = |compression: u32| CodecTag::FourCc(compression.to_le_bytes());
    match dib_header(format) {
        DibClass::Rgb(dib) => (
            StreamParams::Video(dib),
            CodecDescriptor {
                codec: CodecId::Uncompressed,
                codec_private: format.to_vec(),
                format: CodecFormat::Video(VideoFormat {
                    width: dib.frame_width(),
                    height: dib.frame_height(),
                    format: PixelFormat::Rgba8,
                    color: ColorInfo::default(),
                }),
                native_format: Fact::Unsupported,
            },
            Some(tag(dib.compression)),
        ),
        DibClass::Unsupported(dib) => (
            StreamParams::Undecodable,
            CodecDescriptor {
                codec: dib_codec_id(dib.compression),
                codec_private: format.to_vec(),
                format: CodecFormat::Data,
                native_format: Fact::NotDeclared,
            },
            Some(tag(dib.compression)),
        ),
        DibClass::Malformed => (StreamParams::Malformed, unknown_codec(format), None),
    }
}

/// `auds` `strf` → (`StreamParams`, [`CodecDescriptor`]) — PCM/IEEE-
/// float WAVEFORMATEX layouts are decodable (canonical target `f32`;
/// native scale declared via `native_format`).
fn audio_codec(format: &[u8]) -> (StreamParams, CodecDescriptor, Option<CodecTag>) {
    let tag = format
        .get(0..2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()));
    let codec = match tag {
        Some(0x0001 | 0x0003) => CodecId::Pcm,
        _ => CodecId::Other,
    };
    let codec_tag = tag.map(|tag| CodecTag::Numeric(u64::from(tag)));
    match wave_format(format) {
        WaveClass::Pcm(pcm) => (
            StreamParams::Audio(pcm),
            CodecDescriptor {
                codec: CodecId::Pcm,
                codec_private: format.to_vec(),
                format: CodecFormat::Audio(ruviform_audio_core::AudioFormat {
                    sample_rate: ruviform_audio_core::SampleRate::new(pcm.rate).unwrap(),
                    layout: pcm.layout,
                    sample_format: ruviform_audio_core::SampleFormat::F32,
                }),
                native_format: Fact::Known(CodecFormat::Audio(ruviform_audio_core::AudioFormat {
                    sample_rate: ruviform_audio_core::SampleRate::new(pcm.rate).unwrap(),
                    layout: pcm.layout,
                    sample_format: pcm.sample,
                })),
            },
            codec_tag,
        ),
        WaveClass::Unsupported => (
            StreamParams::Undecodable,
            CodecDescriptor {
                codec,
                codec_private: format.to_vec(),
                format: CodecFormat::Data,
                native_format: Fact::NotDeclared,
            },
            codec_tag,
        ),
        WaveClass::Malformed => (
            StreamParams::Malformed,
            CodecDescriptor {
                codec,
                codec_private: format.to_vec(),
                format: CodecFormat::Data,
                native_format: Fact::NotDeclared,
            },
            codec_tag,
        ),
    }
}
