//! Decoded-frame types: [`AudioFrame`] — the named bundle of an
//! [`AudioBuffer`] with its [`MediaTimestamp`] presentation timestamp
//! and optional [`MediaDuration`] — [`NativeAudioFrame`], the
//! non-destructive (native-scale `f64`) PCM view, and [`Frame`], the
//! sum type decoded frames travel as.
//!
//! [`AudioBuffer`] (ruviform-audio-core) is a pure processing buffer
//! and carries no timestamp, while [`VideoFrame`] embeds its
//! `MediaTime` PTS and `Option<MediaDuration>` duration. `AudioFrame`
//! restores the symmetry at the boundary instead of returning tuples
//! (spec `AudioFrame` decision): same shape as `VideoFrame`, so PTS and
//! duration transport is a type-level contract on both paths.
//!
//! The buffer inside an `AudioFrame` is always the canonical planar
//! `f32` [`AudioBuffer`] — decoder backends convert their native output
//! at the boundary, per the audio-core canonical-format rule. That
//! conversion is what the canonical playback contract *is*: it is never
//! claimed to be lossless. [`NativeAudioFrame`] is the separate
//! precision-preserving path for offline analysis: it keeps the
//! decoder's native-scale values in `f64` storage (exact for every
//! ≤32-bit integer encoding and for `f32`/`f64` sources) with an
//! explicit conversion history, so precision-sensitive consumers are
//! never forced through the `f32` normalization.

use ruviform_audio_core::{AudioBuffer, AudioFormat};
use ruviform_media_core::{MediaDuration, MediaTime, MediaTimestamp, TickTimestamp};
use ruviform_video_core::VideoFrame;

/// A decoded audio frame: an [`AudioBuffer`] plus its timestamps.
///
/// Mirrors [`VideoFrame`]'s shape — the buffer, a [`MediaTime`] PTS in
/// the stream's time base (wrapped in [`MediaTimestamp`] so the
/// timestamp's provenance crosses the boundary), and `duration` as an
/// `Option` only (explicitly carried or derived downstream).
///
/// **Send contract:** `AudioFrame` is `Send` — decoded frames are
/// produced on the I/O/decode thread and handed to the processing side
/// through a bounded queue (design baseline, thread-model section).
///
/// **RT contract:** non-RT only — construction, cloning of the
/// underlying buffer type and dropping may allocate; the timestamps
/// are pure `Copy` values.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
/// use ruviform_media_core::{MediaDuration, MediaTime, MediaTimestamp, TimeBase};
/// use ruviform_media_io::AudioFrame;
///
/// let buffer = AudioBuffer::new(BufferSpec {
///     layout: ChannelLayout::Stereo,
///     frames: 1_024,
///     sample_rate: SampleRate::new(48_000).unwrap(),
/// });
/// let pts = MediaTimestamp::carried(MediaTime::new(0, TimeBase::COMMON_90K));
/// let mut frame = AudioFrame::new(buffer, pts);
/// frame.set_duration(Some(MediaDuration::new(1_920, TimeBase::COMMON_90K)));
/// assert_eq!(frame.pts().time.value(), 0);
/// assert_eq!(frame.duration().unwrap().value(), 1_920);
/// assert_eq!(frame.buffer().frames(), 1_024);
/// ```
#[derive(Debug)]
pub struct AudioFrame {
    /// The decoded samples — canonical planar `f32`.
    buffer: AudioBuffer,
    /// Presentation timestamp in the stream's time base, with the
    /// provenance of the value (carried / derived / estimated).
    pts: MediaTimestamp,
    /// Frame duration, when carried — `Option` only.
    duration: Option<MediaDuration>,
}

impl AudioFrame {
    /// Wraps a decoded `buffer` with its presentation timestamp; the
    /// duration starts empty — set it via
    /// [`set_duration`](Self::set_duration) when the decoder carries one.
    ///
    /// **RT contract:** non-RT only — moves an owned buffer.
    #[must_use]
    pub const fn new(buffer: AudioBuffer, pts: MediaTimestamp) -> Self {
        Self {
            buffer,
            pts,
            duration: None,
        }
    }

    /// The decoded audio buffer (canonical planar `f32`).
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub const fn buffer(&self) -> &AudioBuffer {
        &self.buffer
    }

    /// Mutable access to the decoded buffer — e.g. for a decoder
    /// writing into a frame it constructed up front.
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub const fn buffer_mut(&mut self) -> &mut AudioBuffer {
        &mut self.buffer
    }

    /// Consumes the frame and returns the owned buffer.
    ///
    /// **RT contract:** non-RT only — moves an owned buffer.
    #[must_use]
    pub fn into_buffer(self) -> AudioBuffer {
        self.buffer
    }

    /// Presentation timestamp in the stream's time base, with
    /// provenance. `pts().time` is the [`MediaTime`];
    /// `pts().provenance` says whether it was carried, derived, or
    /// estimated.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn pts(&self) -> MediaTimestamp {
        self.pts
    }

    /// Sets the presentation timestamp.
    ///
    /// **RT contract:** non-RT only — `Copy` write.
    pub const fn set_pts(&mut self, pts: MediaTimestamp) {
        self.pts = pts;
    }

    /// Frame duration, when one is carried — `Option` only (explicit or
    /// derived downstream), mirroring [`VideoFrame::duration`].
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn duration(&self) -> Option<MediaDuration> {
        self.duration
    }

    /// Sets or clears the frame duration.
    ///
    /// **RT contract:** non-RT only — `Copy` write.
    pub const fn set_duration(&mut self, duration: Option<MediaDuration>) {
        self.duration = duration;
    }
}

/// One step in the conversion chain that produced a
/// [`NativeAudioFrame`]'s stored `f64` samples.
///
/// The chain is an explicit, ordered history: a decoder output stored
/// verbatim has an empty chain; each entry records a real conversion so
/// precision loss is visible rather than implicit.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PcmTransform {
    /// Integer PCM widened into `f64` storage. Exact for every ≤32-bit
    /// integer value.
    IntegerWidenedToF64,
    /// `f32` widened into `f64` storage. Exact.
    Float32WidenedToF64,
    /// Amplitude was re-scaled into the `[-1, 1]` normalized float
    /// range (the canonical `f32` contract, or a normalization applied
    /// by a backend). Lossy for integer sources wider than 24 bits and
    /// destroys the original scale.
    Normalized,
    /// Values were quantized to a lower-precision encoding. Lossy.
    Quantized,
    /// Values were resampled to a different rate. Lossy in general.
    Resampled,
}

impl PcmTransform {
    /// Whether the step can discard or alter sample information.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn is_lossy(self) -> bool {
        matches!(self, Self::Normalized | Self::Quantized | Self::Resampled)
    }
}

/// A decoded audio frame in the decoder's native scale — the
/// non-destructive PCM path.
///
/// Where [`AudioFrame`] always carries canonical planar `f32` (a
/// normalized playback representation), `NativeAudioFrame` carries the
/// decoder's output as interleaved `f64` values in the **native scale
/// of [`NativeAudioFrame::format`]'s encoding** — e.g. an `S16` decode
/// yields `f64` values in `[-32768, 32767]`. No `[-1, 1]` normalization
/// is applied. `f64` storage is exact for every ≤32-bit integer value
/// and for `f32`/`f64` sources, so this view is lossless; the
/// [`transforms`](Self::transforms) history records every conversion
/// actually performed (e.g. `S16` → `f64` storage lists
/// [`PcmTransform::IntegerWidenedToF64`]). `NaN`/`Inf` values are never
/// replaced with zero.
///
/// Unsigned integer encodings (`U8`, `U16`, `U24`, `U32`) are reported
/// as signed-offset values — the wire value minus half the range —
/// matching decoded-audio convention (a silent `U8` is `0.0`, not
/// `128.0`).
///
/// `pts` is a [`TickTimestamp`] — raw ticks in the owning stream's time
/// base with provenance — so a native frame stays fully describable
/// even when the stream's time base is unavailable (a native decoder
/// does not need a base to emit sample data).
///
/// **Send contract:** `NativeAudioFrame` is `Send` for the same reason
/// as [`AudioFrame`].
///
/// **RT contract:** non-RT only — owns an allocated sample vector.
#[derive(Debug)]
pub struct NativeAudioFrame {
    /// The decoder's native output format: sample rate, channel layout,
    /// and the sample encoding the `samples` values are scaled to.
    format: AudioFormat,
    /// Interleaved native-scale samples: `samples[i * channels + c]`.
    samples: Vec<f64>,
    /// Presentation timestamp in the owning stream's time-base ticks,
    /// with provenance.
    pts: TickTimestamp,
    /// Conversion history — empty when the decoder's output is stored
    /// verbatim.
    transforms: Vec<PcmTransform>,
}

impl NativeAudioFrame {
    /// Wraps decoded native-scale `samples` (interleaved, in
    /// `format.sample_format`'s scale) with their presentation
    /// timestamp and conversion history.
    ///
    /// # Panics
    ///
    /// Debug-asserts that `samples.len()` is a multiple of the layout's
    /// channel count — a malformed frame is a construction-site bug.
    ///
    /// **RT contract:** non-RT only — moves owned storage.
    #[must_use]
    pub fn new(
        format: AudioFormat,
        samples: Vec<f64>,
        pts: TickTimestamp,
        transforms: Vec<PcmTransform>,
    ) -> Self {
        debug_assert_eq!(samples.len() % format.layout.channels() as usize, 0);
        Self {
            format,
            samples,
            pts,
            transforms,
        }
    }

    /// The decoder's native output format.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn format(&self) -> AudioFormat {
        self.format
    }

    /// Interleaved native-scale samples.
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub fn samples(&self) -> &[f64] {
        &self.samples
    }

    /// Number of sample frames (`samples.len() / channels`) — the
    /// *actual* decoded frame count, independently of any declared
    /// count.
    ///
    /// **RT contract:** non-RT only — arithmetic on stored values.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / self.format.layout.channels() as usize
    }

    /// Presentation timestamp: raw ticks in the owning stream's time
    /// base plus provenance. Pair with the stream's
    /// `StreamDescriptor::time_base` via
    /// [`TickTimestamp::at`](ruviform_media_core::TickTimestamp::at)
    /// when a base is known.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn pts(&self) -> TickTimestamp {
        self.pts
    }

    /// The conversion history of [`samples`](Self::samples) — ordered,
    /// empty when the decoder output is stored verbatim.
    /// [`PcmTransform::is_lossy`] classifies each step.
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub fn transforms(&self) -> &[PcmTransform] {
        &self.transforms
    }

    /// Whether any recorded transform step loses precision.
    ///
    /// **RT contract:** non-RT only — scans the history.
    #[must_use]
    pub fn is_lossy(&self) -> bool {
        self.transforms.iter().any(|t| t.is_lossy())
    }
}

/// A decoded frame — the output of [`Decoder::receive_frame`] and the
/// input of [`Encoder::send_frame`].
///
/// The sum type keeps the decoded-frame representations behind one
/// boundary type instead of splitting the decoder contract per media
/// kind (spec Deferred Design Concerns: the enum-split representation
/// was chosen over per-kind typed ports — `#[non_exhaustive]` leaves
/// room for further frame kinds such as `Data`-stream payloads).
///
/// A decoder emits exactly one representation per media kind, chosen
/// when it is built: a canonical decoder emits [`Frame::Audio`]
/// (planar `f32`), a decoder built for the native path emits
/// [`Frame::NativeAudio`]. See [`Features::NATIVE_AUDIO`] for the
/// capability flag.
///
/// **Send contract:** `Frame` is `Send` for the same reason as
/// [`AudioFrame`].
///
/// **RT contract:** non-RT only — part of the media-I/O boundary.
///
/// [`Decoder::receive_frame`]: crate::Decoder::receive_frame
/// [`Encoder::send_frame`]: crate::Encoder::send_frame
/// [`Features::NATIVE_AUDIO`]: crate::Features::NATIVE_AUDIO
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_media_io::Frame;
/// use ruviform_video_core::{PixelFormat, VideoFrame};
///
/// let pts = MediaTime::new(90_000, TimeBase::COMMON_90K);
/// let frame = Frame::Video(VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts).unwrap());
/// assert_eq!(frame.pts(), Some(pts));
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub enum Frame {
    /// A decoded audio frame — canonical planar `f32`.
    Audio(AudioFrame),
    /// A decoded video frame.
    Video(VideoFrame),
    /// A decoded audio frame in the decoder's native scale — the
    /// non-destructive path emitted by native-mode decoders.
    NativeAudio(NativeAudioFrame),
}

impl Frame {
    /// The frame's presentation timestamp as a self-contained
    /// [`MediaTime`], where expressible.
    ///
    /// `None` for [`Frame::NativeAudio`]: a native frame carries raw
    /// ticks ([`NativeAudioFrame::pts`]) that are resolved against the
    /// owning stream's time base, and may exist when no base is known.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn pts(&self) -> Option<MediaTime> {
        match self {
            Self::Audio(frame) => Some(frame.pts().time),
            Self::Video(frame) => Some(frame.pts()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{BufferSpec, ChannelLayout, SampleFormat, SampleRate};
    use ruviform_media_core::{TimeBase, TimestampProvenance};
    use ruviform_video_core::PixelFormat;

    fn check_send<T: Send>() {}

    fn stereo_buffer(frames: usize) -> AudioBuffer {
        AudioBuffer::new(BufferSpec {
            layout: ChannelLayout::Stereo,
            frames,
            sample_rate: SampleRate::new(48_000).unwrap(),
        })
    }

    fn carried_pts(ticks: i64) -> MediaTimestamp {
        MediaTimestamp::carried(MediaTime::new(ticks, TimeBase::COMMON_90K))
    }

    #[test]
    fn audio_frame_carries_pts_and_duration() {
        let mut frame = AudioFrame::new(stereo_buffer(256), carried_pts(9_000));
        assert_eq!(frame.pts().time.value(), 9_000);
        assert_eq!(frame.pts().provenance, TimestampProvenance::Carried);
        assert_eq!(frame.duration(), None);

        frame.set_duration(Some(MediaDuration::new(480, TimeBase::COMMON_90K)));
        assert_eq!(frame.duration().unwrap().value(), 480);
        frame.set_pts(carried_pts(18_000));
        assert_eq!(frame.pts().time.value(), 18_000);
    }

    #[test]
    fn audio_frame_buffer_access_and_take() {
        let mut frame = AudioFrame::new(stereo_buffer(4), carried_pts(0));
        frame.buffer_mut().channel_mut(0).unwrap()[0] = 0.5;
        assert_eq!(frame.buffer().channel(0).unwrap()[0], 0.5);
        let buffer = frame.into_buffer();
        assert_eq!(buffer.channels(), 2);
        assert_eq!(buffer.frames(), 4);
    }

    #[test]
    fn native_frame_reports_exact_frame_count_and_history() {
        let format = AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::S16,
        };
        let frame = NativeAudioFrame::new(
            format,
            vec![-32_768.0, 0.0, 32_767.0, 1.0],
            TickTimestamp::derived(1_024),
            vec![PcmTransform::IntegerWidenedToF64],
        );
        assert_eq!(frame.frames(), 2);
        assert_eq!(frame.format().sample_format, SampleFormat::S16);
        assert_eq!(frame.samples()[0], -32_768.0);
        assert_eq!(frame.pts().ticks, 1_024);
        assert_eq!(frame.pts().provenance, TimestampProvenance::Derived);
        assert!(!frame.is_lossy());
    }

    #[test]
    fn transforms_classify_precision_loss() {
        assert!(!PcmTransform::IntegerWidenedToF64.is_lossy());
        assert!(!PcmTransform::Float32WidenedToF64.is_lossy());
        assert!(PcmTransform::Normalized.is_lossy());
        assert!(PcmTransform::Quantized.is_lossy());
        assert!(PcmTransform::Resampled.is_lossy());
    }

    #[test]
    fn frame_enum_pts_matches_inner() {
        let pts = carried_pts(123);
        let audio = Frame::Audio(AudioFrame::new(stereo_buffer(8), pts));
        assert_eq!(audio.pts(), Some(pts.time));

        let video_frame = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 2, pts.time).unwrap();
        let video = Frame::Video(video_frame);
        assert_eq!(video.pts(), Some(pts.time));
    }

    /// `Frame::pts` does not fabricate a base for the native variant:
    /// the raw tick timestamp stays on the concrete type.
    #[test]
    fn native_variant_pts_is_not_flattened() {
        let format = AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Mono,
            sample_format: SampleFormat::S24,
        };
        let frame = Frame::NativeAudio(NativeAudioFrame::new(
            format,
            vec![0.0],
            TickTimestamp::estimated(7),
            Vec::new(),
        ));
        assert_eq!(frame.pts(), None);
        match frame {
            Frame::NativeAudio(native) => assert_eq!(native.pts().ticks, 7),
            _ => panic!("expected native variant"),
        }
    }

    #[test]
    fn frame_send_bounds() {
        check_send::<AudioFrame>();
        check_send::<NativeAudioFrame>();
        check_send::<Frame>();
    }
}
