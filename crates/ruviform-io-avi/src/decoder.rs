//! [`Decoder`] implementation for AVI streams.
//!
//! Two decoders behind one type: an audio path that turns `wb` bodies
//! into canonical planar `f32` frames (normalized to [-1, 1]) or
//! non-destructive [`NativeAudioFrame`]s (interleaved `f64` in the
//! container's native integer/float scale, [`PcmTransform`] history
//! recorded — Issue #151), and a video path that turns `db`/`dc` DIB
//! bodies into `Rgba8` [`VideoFrame`]s (BGR(A) → RGBA swizzle, bottom-up
//! row flip, opaque alpha). A zero-length video body repeats the
//! previous frame — the AVI drop-frame encoding — sharing its `Arc`
//! backing rather than copying.
//!
//! Packet PTS/DTS arrive as derived ticks; canonical output carries them
//! resolved to `MediaTime` with [`TimestampProvenance::Derived`], the
//! native path keeps the raw `TickTimestamp`. Frame duration is the
//! packet's tick advance resolved against the stream time base.

use std::collections::VecDeque;

use ruviform_audio_core::{AudioBuffer, BufferSpec, SampleFormat, SampleRate};
use ruviform_media_core::{
    MediaDuration, MediaTime, MediaTimestamp, StreamId, TickTimestamp, TimeBase,
    TimestampProvenance,
};
use ruviform_media_io::{
    AudioFrame, Decoder, Error, ErrorKind, Frame, NativeAudioFrame, Packet, PcmTransform,
};
use ruviform_video_core::{PixelFormat, SharedPlane, VideoFrame};

use crate::format::{DibHeader, WavePcm};

/// The decodable per-stream parameter set a demuxer snapshot computed
/// at probe — `StreamRecord::params` is what `make_decoder` consumes.
#[derive(Copy, Clone, Debug)]
pub(crate) enum StreamParams {
    /// `auds` with a layout-consistent PCM/IEEE-float `WAVEFORMATEX`.
    Audio(WavePcm),
    /// `vids` with a `BI_RGB` 24/32-bpp `BITMAPINFOHEADER`.
    Video(DibHeader),
    /// A parseable stream declaring an encoding this backend cannot
    /// decode (`ErrorKind::Unsupported` at decoder construction).
    Undecodable,
    /// A structurally malformed stream (`ErrorKind::InvalidData`).
    Malformed,
}

/// Canonical vs native audio output mode.
#[derive(Copy, Clone, Debug)]
pub(crate) enum AudioMode {
    /// Planar `f32` normalized to [-1, 1].
    Canonical,
    /// Interleaved `f64` in native scale — Issue-#151 non-destructive.
    Native,
}

enum Kind {
    Audio {
        pcm: WavePcm,
        mode: AudioMode,
    },
    Video {
        dib: DibHeader,
        last: Option<VideoFrame>,
    },
}

/// AVI decoder — one packet in, zero or one frame out, matching the
/// boundary's send/receive pump contract.
pub struct AviDecoder {
    stream: StreamId,
    time_base: Option<TimeBase>,
    next_ts: i64,
    kind: Kind,
    pending: VecDeque<Frame>,
}

impl AviDecoder {
    pub(crate) fn audio(
        stream: StreamId,
        pcm: WavePcm,
        time_base: Option<TimeBase>,
        mode: AudioMode,
    ) -> Self {
        Self {
            stream,
            time_base,
            next_ts: 0,
            kind: Kind::Audio { pcm, mode },
            pending: VecDeque::new(),
        }
    }

    pub(crate) fn video(stream: StreamId, dib: DibHeader, time_base: Option<TimeBase>) -> Self {
        Self {
            stream,
            time_base,
            next_ts: 0,
            kind: Kind::Video { dib, last: None },
            pending: VecDeque::new(),
        }
    }

    /// Packet presentation ticks — the carried `derived` value, else
    /// the `next_ts` extrapolation anchor marked `Estimated` (the same
    /// convention `ruviform-io-symphonia` uses).
    fn packet_pts(&self, packet: &Packet) -> TickTimestamp {
        packet.pts.unwrap_or(TickTimestamp {
            ticks: self.next_ts,
            provenance: TimestampProvenance::Estimated,
        })
    }

    /// Resolves ticks into the canonical `MediaTimestamp` — `Estimated`
    /// only when the stream declared no time base (never a fabricated
    /// base).
    fn frame_pts(&self, ticks: TickTimestamp) -> MediaTimestamp {
        match self.time_base {
            Some(base) => MediaTimestamp {
                time: ticks.at(base),
                provenance: ticks.provenance,
            },
            None => MediaTimestamp::estimated(MediaTime::new(self.next_ts, TimeBase::COMMON_90K)),
        }
    }

    /// Packet duration resolved against the stream base — `None` when
    /// the tick count exceeds `i64` (unrepresentable, never clamped).
    fn frame_duration(&self, packet: &Packet) -> Option<MediaDuration> {
        packet.duration.and_then(|ticks| {
            self.time_base.and_then(|base| {
                i64::try_from(ticks)
                    .ok()
                    .map(|value| MediaDuration::new(value, base))
            })
        })
    }
}

impl Decoder for AviDecoder {
    /// Feeds one packet and decodes it eagerly.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidData`] — packet for a different stream, a
    ///   `wb` body ending mid-sample-frame, or a short DIB body.
    /// - [`ErrorKind::TimeOverflow`] — timestamp arithmetic overflowed.
    ///
    /// **RT contract:** non-RT only — unbounded codec work, allocates.
    fn send_packet(&mut self, packet: Packet) -> Result<(), Error> {
        if packet.stream != self.stream {
            return Err(crate::err(
                ErrorKind::InvalidData,
                "avi: packet belongs to a different stream",
            ));
        }
        // Resolve timestamps before fields are mutably borrowed; the
        // field-level destructuring keeps `kind`, `pending` and
        // `next_ts` as disjoint borrows inside the match arms.
        let pts = self.packet_pts(&packet);
        let duration = self.frame_duration(&packet);
        let pts_media = self.frame_pts(pts);
        let Self {
            kind,
            pending,
            next_ts,
            ..
        } = self;

        match kind {
            Kind::Audio { pcm, mode } => {
                let (pcm, mode) = (*pcm, *mode);
                if !packet
                    .data
                    .len()
                    .is_multiple_of(usize::from(pcm.block_align))
                {
                    return Err(crate::err(
                        ErrorKind::InvalidData,
                        "avi: `wb` body ends mid sample frame",
                    ));
                }
                let frames = packet.data.len() / usize::from(pcm.block_align);
                let frame = match mode {
                    AudioMode::Canonical => {
                        let buffer = decode_canonical(&packet.data, pcm, frames)?;
                        let mut frame = AudioFrame::new(buffer, pts_media);
                        frame.set_duration(duration);
                        Frame::Audio(frame)
                    }
                    AudioMode::Native => {
                        let (format, samples, transforms) = decode_native(&packet.data, pcm);
                        Frame::NativeAudio(NativeAudioFrame::new(format, samples, pts, transforms))
                    }
                };
                *next_ts = next_ts
                    .checked_add(duration_ticks(&packet)?)
                    .ok_or_else(|| Error::from_kind(ErrorKind::TimeOverflow))?;
                pending.push_back(frame);
            }
            Kind::Video { dib, last } => {
                let dib = *dib;
                let frame = if packet.data.is_empty() {
                    // Zero-length `db`/`dc` — the AVI drop-frame marker:
                    // repeat the previous frame at the new timestamp.
                    last.as_ref().map(|prev| {
                        let mut repeat = prev.clone();
                        repeat.set_pts(pts_media.time);
                        repeat.set_duration(duration);
                        repeat
                    })
                } else {
                    Some(decode_dib(&packet.data, dib, pts_media.time, duration)?)
                };
                *next_ts = next_ts
                    .checked_add(duration_ticks(&packet)?)
                    .ok_or_else(|| Error::from_kind(ErrorKind::TimeOverflow))?;
                if let Some(frame) = frame {
                    *last = Some(frame.clone());
                    pending.push_back(Frame::Video(frame));
                }
            }
        }
        Ok(())
    }

    /// Pops the next decoded frame — `Ok(None)` when the pump needs
    /// another `send_packet`.
    ///
    /// **RT contract:** non-RT only.
    fn receive_frame(&mut self) -> Result<Option<Frame>, Error> {
        Ok(self.pending.pop_front())
    }

    /// Drops queued output and decoder-held state (the repeated-video
    /// frame, the extrapolation anchor).
    fn flush(&mut self) -> Result<(), Error> {
        self.pending.clear();
        self.next_ts = 0;
        if let Kind::Video { last, .. } = &mut self.kind {
            *last = None;
        }
        Ok(())
    }
}

/// The packet's tick duration as `i64` — `TimeOverflow` rather than
/// truncation when the value does not fit.
fn duration_ticks(packet: &Packet) -> Result<i64, Error> {
    i64::try_from(packet.duration.unwrap_or(0))
        .map_err(|_| Error::from_kind(ErrorKind::TimeOverflow))
}

/// Canonical decode: interleaved wire PCM → planar normalized `f32`.
fn decode_canonical(data: &[u8], pcm: WavePcm, frames: usize) -> Result<AudioBuffer<f32>, Error> {
    let spec = BufferSpec {
        layout: pcm.layout,
        frames,
        sample_rate: SampleRate::new(pcm.rate)
            .map_err(|_| crate::err(ErrorKind::InvalidData, "avi: invalid rate"))?,
    };
    let mut buffer = AudioBuffer::<f32>::try_new(spec)
        .map_err(|_| crate::err(ErrorKind::InvalidValue, "avi: buffer shape"))?;
    let channels = usize::from(pcm.channels);
    match pcm.sample {
        SampleFormat::U8 => fill(&mut buffer, data, channels, |s| {
            f32::from(s[0].wrapping_sub(128).cast_signed()) / 128.0
        }),
        SampleFormat::S16 => fill(&mut buffer, data, channels, |s| {
            f32::from(i16::from_le_bytes([s[0], s[1]])) / 32_768.0
        }),
        SampleFormat::S24 => fill(&mut buffer, data, channels, |s| {
            let v = i32::from(s[0]) | i32::from(s[1]) << 8 | i32::from(s[2].cast_signed()) << 16;
            // The i24 range fits f32's 24-bit mantissa exactly.
            #[allow(clippy::cast_precision_loss)]
            (v as f32 / 8_388_608.0)
        }),
        SampleFormat::S32 => fill(&mut buffer, data, channels, |s| {
            // Canonical f32 output: narrowing is the representation,
            // not a defect — the native path keeps full scale.
            #[allow(clippy::cast_precision_loss)]
            (i32::from_le_bytes(s.try_into().unwrap()) as f32 / 2_147_483_648.0)
        }),
        SampleFormat::F32 => fill(&mut buffer, data, channels, |s| {
            f32::from_le_bytes(s.try_into().unwrap())
        }),
        SampleFormat::F64 => fill(&mut buffer, data, channels, |s| {
            #[allow(clippy::cast_possible_truncation)]
            (f64::from_le_bytes(s.try_into().unwrap()) as f32)
        }),
        _ => return Err(Error::from_kind(ErrorKind::Unsupported)),
    }
    Ok(buffer)
}

/// Fills each channel plane from interleaved wire bytes.
fn fill(
    buffer: &mut AudioBuffer<f32>,
    data: &[u8],
    channels: usize,
    convert: impl Fn(&[u8]) -> f32,
) {
    let frames = buffer.frames();
    if frames == 0 {
        return;
    }
    let sample_bytes = data.len() / (frames * channels);
    for ch in 0..channels {
        let plane = buffer.channel_mut(ch).expect("channel index");
        for (i, slot) in plane.iter_mut().enumerate() {
            let off = (i * channels + ch) * sample_bytes;
            *slot = convert(&data[off..off + sample_bytes]);
        }
    }
}

/// Native decode: interleaved wire PCM → interleaved `f64` in native
/// scale — the Issue-#151 non-destructive contract (`U8` centered on
/// the signed midpoint exactly as `ruviform-io-symphonia`; signed
/// integers widened exactly; `f64` verbatim).
fn decode_native(
    data: &[u8],
    pcm: WavePcm,
) -> (
    ruviform_audio_core::AudioFormat,
    Vec<f64>,
    Vec<PcmTransform>,
) {
    let channels = usize::from(pcm.channels);
    let frames = data.len() / usize::from(pcm.block_align);
    let mut samples = vec![0.0f64; frames * channels];
    let int = || vec![PcmTransform::IntegerWidenedToF64];
    let transforms = match pcm.sample {
        SampleFormat::U8 => {
            expand(&mut samples, data, channels, |s| f64::from(s[0]) - 128.0);
            int()
        }
        SampleFormat::S16 => {
            expand(&mut samples, data, channels, |s| {
                f64::from(i16::from_le_bytes([s[0], s[1]]))
            });
            int()
        }
        SampleFormat::S24 => {
            expand(&mut samples, data, channels, |s| {
                f64::from(
                    i32::from(s[0]) | i32::from(s[1]) << 8 | i32::from(s[2].cast_signed()) << 16,
                )
            });
            int()
        }
        SampleFormat::S32 => {
            expand(&mut samples, data, channels, |s| {
                f64::from(i32::from_le_bytes(s.try_into().unwrap()))
            });
            int()
        }
        SampleFormat::F32 => {
            expand(&mut samples, data, channels, |s| {
                f64::from(f32::from_le_bytes(s.try_into().unwrap()))
            });
            vec![PcmTransform::Float32WidenedToF64]
        }
        SampleFormat::F64 => {
            expand(&mut samples, data, channels, |s| {
                f64::from_le_bytes(s.try_into().unwrap())
            });
            Vec::new()
        }
        // `make_decoder` only constructs audio decoders for the
        // supported PCM set; an unreachable encoding stays a
        // construction-site bug.
        _ => Vec::new(),
    };
    let format = ruviform_audio_core::AudioFormat {
        sample_rate: SampleRate::new(pcm.rate).unwrap(),
        layout: pcm.layout,
        sample_format: pcm.sample,
    };
    (format, samples, transforms)
}

/// Writes converted wire samples into interleaved `f64` slots —
/// `samples[i * channels + ch]`.
fn expand(samples: &mut [f64], data: &[u8], channels: usize, convert: impl Fn(&[u8]) -> f64) {
    let frames = samples.len() / channels;
    if frames == 0 {
        return;
    }
    let sample_bytes = data.len() / (frames * channels);
    for i in 0..frames {
        for ch in 0..channels {
            let off = (i * channels + ch) * sample_bytes;
            samples[i * channels + ch] = convert(&data[off..off + sample_bytes]);
        }
    }
}

/// Decodes one DIB body into an `Rgba8` frame: BGR(A) byte swizzle,
/// row-order flip for bottom-up DIBs, opaque alpha.
fn decode_dib(
    data: &[u8],
    dib: DibHeader,
    pts: MediaTime,
    duration: Option<MediaDuration>,
) -> Result<VideoFrame, Error> {
    let to_usize = |v: u64| {
        usize::try_from(v)
            .map_err(|_| crate::err(ErrorKind::InvalidData, "avi: DIB dimensions overflow"))
    };
    let width = to_usize(u64::from(dib.frame_width()))?;
    let height = to_usize(u64::from(dib.frame_height()))?;
    let stride = to_usize(dib.row_stride())?;
    let need = to_usize(dib.frame_bytes())?;
    if data.len() < need {
        return Err(crate::err(
            ErrorKind::InvalidData,
            "avi: `db`/`dc` body shorter than one DIB frame",
        ));
    }
    let bytes_pp = usize::from(dib.bit_count / 8);
    let mut out = vec![0u8; width * height * 4];
    for row in 0..height {
        // Positive `biHeight` stores rows bottom-up.
        let src_row = if dib.bottom_up() {
            height - 1 - row
        } else {
            row
        };
        let src = &data[src_row * stride..src_row * stride + width * bytes_pp];
        let dst = &mut out[row * width * 4..(row + 1) * width * 4];
        for px in 0..width {
            dst[px * 4] = src[px * bytes_pp + 2]; // R
            dst[px * 4 + 1] = src[px * bytes_pp + 1]; // G
            dst[px * 4 + 2] = src[px * bytes_pp]; // B
            dst[px * 4 + 3] = if bytes_pp == 4 { src[px * 4 + 3] } else { 255 };
        }
    }
    let mut frame = VideoFrame::from_shared_planes(
        PixelFormat::Rgba8,
        u32::try_from(width)
            .map_err(|_| crate::err(ErrorKind::InvalidData, "avi: DIB width overflow"))?,
        u32::try_from(height)
            .map_err(|_| crate::err(ErrorKind::InvalidData, "avi: DIB height overflow"))?,
        pts,
        vec![SharedPlane {
            data: std::sync::Arc::from(out.into_boxed_slice()),
            stride: width * 4,
        }],
    )
    .map_err(|_| crate::err(ErrorKind::InvalidValue, "avi: video frame shape"))?;
    frame.set_duration(duration);
    Ok(frame)
}
