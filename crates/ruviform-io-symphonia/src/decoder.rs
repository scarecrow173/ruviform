//! [`Decoder`] implementation — send/receive audio decode.
//!
//! The backend audio decoder decodes synchronously: one packet in, all
//! of its frames out in a single call, so [`send_packet`](Decoder::send_packet)
//! decodes eagerly into a small output queue and
//! [`receive_frame`](Decoder::receive_frame) drains it. This keeps the
//! boundary's pump semantics without fabricating asynchronous behaviour:
//! `Ok(None)` from `receive_frame` means "needs more input", exactly as
//! the trait contract intends.
//!
//! The decoder runs in one of two [`Mode`]s chosen at construction:
//! `Canonical` emits [`Frame::Audio`] (planar normalized `f32`), and
//! `Native` emits [`Frame::NativeAudio`] — interleaved `f64` samples in
//! the decoder's native scale with an explicit [`PcmTransform`]
//! history, so precision-sensitive consumers are never forced through
//! the `f32` normalization.

use std::collections::VecDeque;

use ruviform_audio_core::{AudioBuffer, AudioFormat, BufferSpec, SampleFormat, SampleRate};
use ruviform_media_core::{
    MediaDuration, MediaTime, MediaTimestamp, StreamId, TickTimestamp, TimeBase,
};
use ruviform_media_io::{
    AudioFrame, Decoder, Error, ErrorKind, Frame, NativeAudioFrame, Packet, PcmTransform, Result,
};
use symphonia::core::audio::sample::{i24, u24};
use symphonia::core::audio::{Audio, GenericAudioBufferRef};
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia::core::packet::Packet as BackendPacket;
use symphonia::core::units::{Duration as BackendDuration, Timestamp as BackendTimestamp};

use crate::error::map_error;
use crate::map;

/// Which frame representation a [`SymphoniaDecoder`] emits — chosen at
/// construction via the demuxer's `make_decoder` / `make_native_decoder`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Canonical decode: planar normalized `f32` [`AudioFrame`].
    Canonical,
    /// Native decode: native-scale interleaved `f64`
    /// [`NativeAudioFrame`] — the non-destructive path.
    Native,
}

/// Media-I/O [`Decoder`] backed by a backend audio decoder.
///
/// `send_packet` decodes eagerly and queues every resulting frame in an
/// **unbounded** output queue: callers must interleave `receive_frame`
/// until `Ok(None)` or memory grows without limit. Whether the boundary
/// gains a bounded-queue/backpressure contract (input conditions,
/// wait/collect procedure, limit, or failure behavior) is a follow-up
/// design item.
///
/// **RT contract:** non-RT only — `send_packet` performs unbounded codec
/// work and allocates; `receive_frame` allocates nothing itself but only
/// drains what `send_packet` queued.
pub struct SymphoniaDecoder {
    /// The backend decoder — decodes synchronously inside `send_packet`.
    decoder: Box<dyn AudioDecoder>,
    /// The decoded stream's time base — `None` when the container
    /// declared none (output timestamps are then `Estimated`).
    time_base: Option<TimeBase>,
    /// The stream this decoder accepts; packets for other streams are a
    /// usage error.
    stream: StreamId,
    /// Which frame representation `receive_frame` produces.
    mode: Mode,
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
        time_base: Option<TimeBase>,
        params: &AudioCodecParameters,
        mode: Mode,
    ) -> Result<Self> {
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(map_error)?;
        Ok(Self {
            decoder,
            time_base,
            stream,
            mode,
            pending: VecDeque::new(),
            next_ts: 0,
        })
    }

    /// Resolves the packet's presentation timestamp in raw stream ticks:
    /// the carried tick value when present, else the extrapolated
    /// `next_ts` anchor marked `Estimated`.
    fn packet_pts(&self, packet: &Packet) -> TickTimestamp {
        packet
            .pts
            .unwrap_or_else(|| TickTimestamp::estimated(self.next_ts))
    }

    /// Resolves the frame timestamp for canonical output: the raw ticks
    /// resolved against the stream time base. When the container
    /// declared no base, or the packet carried no PTS, the timestamp is
    /// `Estimated` — never a fabricated base.
    fn frame_pts(&self, ticks: TickTimestamp) -> MediaTimestamp {
        match self.time_base {
            Some(base) => MediaTimestamp {
                time: ticks.at(base),
                provenance: ticks.provenance,
            },
            // No declared base: the carried ticks cannot be resolved
            // into a `MediaTime`, so anchor on the extrapolation and
            // mark it estimated.
            None => MediaTimestamp::estimated(MediaTime::new(self.next_ts, TimeBase::COMMON_90K)),
        }
    }

    /// Converts the boundary packet into an owned backend packet for
    /// `decode`. Boundary timestamps are already raw ticks in the
    /// stream's time base — they pass through without conversion. A
    /// missing PTS/DTS falls back to the resolved PTS ticks; a missing
    /// duration becomes zero (duration metadata only).
    fn to_backend_packet(packet: Packet, pts_ticks: i64) -> BackendPacket {
        let dts = packet.dts.map_or(pts_ticks, |dts| dts.ticks);
        let mut backend = BackendPacket::new(
            packet.stream.0,
            BackendTimestamp::new(pts_ticks),
            BackendDuration::new(packet.duration.unwrap_or(0)),
            packet.data.into_boxed_slice(),
        );
        backend.dts = BackendTimestamp::new(dts);
        backend.trim_start = BackendDuration::new(packet.trim_start);
        backend.trim_end = BackendDuration::new(packet.trim_end);
        backend
    }

    /// The duration of `frames` decoded samples in stream-base ticks —
    /// `None` when the stream base or rate cannot express it.
    fn frame_ticks(&self, frames: usize, rate: u32) -> Option<i64> {
        let base = self.time_base?;
        let value = i64::try_from(frames).ok()?;
        let sample_base = TimeBase::sample_rate(rate).ok()?;
        MediaDuration::new(value, sample_base)
            .convert(base, ruviform_media_core::Rounding::NearestEven)
            .ok()
            .map(MediaDuration::value)
    }

    /// Advances the extrapolation anchor by `duration_ticks`; an
    /// overflowing add surfaces as `TimeOverflow`, never saturates.
    fn advance_anchor(&mut self, duration_ticks: Option<i64>) -> Result<()> {
        self.next_ts = self
            .next_ts
            .checked_add(duration_ticks.unwrap_or(0))
            .ok_or_else(|| Error::from_kind(ErrorKind::TimeOverflow))?;
        Ok(())
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
        let pts = self.packet_pts(&packet);
        let backend_packet = Self::to_backend_packet(packet, pts.ticks);

        // `decoded` borrows `self.decoder` for its lifetime — every
        // value frame construction needs is produced inside this scope,
        // then the borrow ends before `self` is mutated for queueing.
        let decoded = self.decoder.decode(&backend_packet).map_err(map_error)?;
        let rate = decoded.spec().rate();
        let (layout, order) = map::channel_layout(decoded.spec().channels())
            .ok_or_else(|| Error::from_kind(ErrorKind::Unsupported))?;
        let frames = decoded.frames();

        // The backend decoded nothing presentable for this packet
        // (e.g. pre-skip priming) — input consumed, no output.
        if frames == 0 {
            return Ok(());
        }

        let output = match self.mode {
            Mode::Canonical => {
                Output::Canonical(canonical_output(&decoded, layout, &order, frames, rate)?)
            }
            Mode::Native => native_output(&decoded, layout, &order, frames, rate)?,
        };
        let duration_ticks = self.frame_ticks(frames, rate);
        self.advance_anchor(duration_ticks)?;

        match output {
            Output::Canonical(buffer) => {
                let mut frame = AudioFrame::new(buffer, self.frame_pts(pts));
                frame.set_duration(
                    duration_ticks
                        .zip(self.time_base)
                        .map(|(ticks, base)| MediaDuration::new(ticks, base)),
                );
                self.pending.push_back(Frame::Audio(frame));
            }
            Output::Native {
                format,
                samples,
                transforms,
            } => {
                self.pending
                    .push_back(Frame::NativeAudio(NativeAudioFrame::new(
                        format, samples, pts, transforms,
                    )));
            }
        }
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

/// The sample payload one decode produced, before queue bookkeeping.
///
/// Carrying it as a small enum lets `send_packet` end the
/// `self.decoder` borrow before the output queue is touched.
enum Output {
    /// Planar normalized `f32` buffer for an [`AudioFrame`].
    Canonical(AudioBuffer),
    /// Native-scale samples + format + transform history for a
    /// [`NativeAudioFrame`].
    Native {
        /// The decoder's native output format.
        format: AudioFormat,
        /// Interleaved native-scale samples.
        samples: Vec<f64>,
        /// Conversion history.
        transforms: Vec<PcmTransform>,
    },
}

/// Canonical path: `AudioBufferRef` → planar normalized `f32`
/// [`AudioBuffer`] via the backend's own conversion — the canonical
/// playback contract, never claimed to be lossless.
fn canonical_output(
    decoded: &GenericAudioBufferRef<'_>,
    layout: ruviform_audio_core::ChannelLayout,
    order: &[usize],
    frames: usize,
    rate: u32,
) -> Result<AudioBuffer> {
    let mut planes: Vec<Vec<f32>> = Vec::new();
    decoded.copy_to_vecs_planar(&mut planes);
    let mut buffer = AudioBuffer::try_new(BufferSpec {
        layout,
        frames,
        sample_rate: SampleRate::new(rate).map_err(Error::from)?,
    })
    .map_err(Error::from)?;
    if planes.len() != buffer.channels() {
        return Err(Error::from_kind(ErrorKind::InvalidData));
    }
    // Plane i carries backend canonical channel i; `order[i]` is its
    // slot in the boundary layout — non-standard masks reorder.
    for (index, plane) in planes.iter().enumerate() {
        if let Some(channel) = order.get(index).and_then(|slot| buffer.channel_mut(*slot)) {
            channel.copy_from_slice(plane);
        }
    }
    Ok(buffer)
}

/// Native path: typed `AudioBufferRef` → interleaved `f64` samples +
/// format + transform history.
fn native_output(
    decoded: &GenericAudioBufferRef<'_>,
    layout: ruviform_audio_core::ChannelLayout,
    order: &[usize],
    frames: usize,
    rate: u32,
) -> Result<Output> {
    let sample_rate = SampleRate::new(rate).map_err(Error::from)?;
    let (sample_format, samples, transforms) = native_samples(decoded, layout, order, frames);
    Ok(Output::Native {
        format: AudioFormat {
            sample_rate,
            layout,
            sample_format,
        },
        samples,
        transforms,
    })
}

/// The interleaved `f64` slot a typed variant writes into —
/// `samples[i * channels + order[c]] = plane[c][i]`, the same channel
/// permutation the canonical path applies.
struct Dest<'a> {
    /// Interleaved destination.
    samples: &'a mut [f64],
    /// Backend plane index → boundary channel slot.
    order: &'a [usize],
    /// Boundary channel count.
    channels: usize,
    /// Frames to copy per plane.
    frames: usize,
}

/// One typed `AudioBufferRef` variant: iterate backend planes, copy
/// each into its boundary slot via `scale`, return (format, transform).
fn extract<S: symphonia::core::audio::sample::Sample>(
    buffer: &symphonia::core::audio::AudioBuffer<S>,
    format: SampleFormat,
    transform: Vec<PcmTransform>,
    dest: &mut Dest<'_>,
    scale: impl Fn(S) -> f64,
) -> (SampleFormat, Vec<PcmTransform>) {
    for index in 0..buffer.spec().channels().count() {
        if let Some(plane) = buffer.plane(index)
            && let Some(&dst) = dest.order.get(index)
        {
            for (frame, &value) in plane.iter().take(dest.frames).enumerate() {
                dest.samples[frame * dest.channels + dst] = scale(value);
            }
        }
    }
    (format, transform)
}

/// Extracts the decoded `AudioBufferRef` into interleaved `f64` samples
/// in the native scale of its declared encoding.
///
/// `samples[i * channels + order[c]] = plane[c][i]` — the same channel
/// permutation the canonical path applies. Signed integers widen
/// exactly (`PcmTransform::IntegerWidenedToF64`); unsigned integers are
/// offset to the signed midpoint convention (`PcmTransform::
/// IntegerWidenedToF64` after the offset — the offset itself is exact);
/// `f32` widens exactly (`Float32WidenedToF64`); `f64` stores verbatim
/// (empty history). `NaN`/`Inf` are never touched.
fn native_samples(
    decoded: &GenericAudioBufferRef<'_>,
    layout: ruviform_audio_core::ChannelLayout,
    order: &[usize],
    frames: usize,
) -> (SampleFormat, Vec<f64>, Vec<PcmTransform>) {
    let mut samples = vec![0.0; frames * layout.channels() as usize];
    let mut dest = Dest {
        channels: layout.channels() as usize,
        samples: &mut samples,
        order,
        frames,
    };
    let int = || vec![PcmTransform::IntegerWidenedToF64];
    let (sample_format, transform) = match decoded {
        // Unsigned: wire value → signed-offset (`v - midpoint`), exact
        // in f64 for every ≤32-bit encoding.
        GenericAudioBufferRef::U8(buf) => {
            extract(buf, SampleFormat::U8, int(), &mut dest, |v: u8| {
                f64::from(v) - 128.0
            })
        }
        GenericAudioBufferRef::U16(buf) => {
            extract(buf, SampleFormat::U16, int(), &mut dest, |v: u16| {
                f64::from(v) - 32_768.0
            })
        }
        GenericAudioBufferRef::U24(buf) => {
            extract(buf, SampleFormat::U24, int(), &mut dest, |v: u24| {
                f64::from(v.0) - 8_388_608.0
            })
        }
        GenericAudioBufferRef::U32(buf) => {
            extract(buf, SampleFormat::U32, int(), &mut dest, |v: u32| {
                f64::from(v) - 2_147_483_648.0
            })
        }
        // Signed: exact f64 widening.
        GenericAudioBufferRef::S8(buf) => {
            extract(buf, SampleFormat::S8, int(), &mut dest, |v: i8| {
                f64::from(v)
            })
        }
        GenericAudioBufferRef::S16(buf) => {
            extract(buf, SampleFormat::S16, int(), &mut dest, |v: i16| {
                f64::from(v)
            })
        }
        GenericAudioBufferRef::S24(buf) => {
            extract(buf, SampleFormat::S24, int(), &mut dest, |v: i24| {
                f64::from(v.0)
            })
        }
        GenericAudioBufferRef::S32(buf) => {
            extract(buf, SampleFormat::S32, int(), &mut dest, |v: i32| {
                f64::from(v)
            })
        }
        GenericAudioBufferRef::F32(buf) => extract(
            buf,
            SampleFormat::F32,
            vec![PcmTransform::Float32WidenedToF64],
            &mut dest,
            |v: f32| f64::from(v),
        ),
        GenericAudioBufferRef::F64(buf) => {
            extract(buf, SampleFormat::F64, Vec::new(), &mut dest, |v: f64| v)
        }
    };
    (sample_format, samples, transform)
}
