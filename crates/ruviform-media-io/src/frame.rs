//! Decoded-frame types: [`AudioFrame`] — the named bundle of an
//! [`AudioBuffer`] with its [`MediaTime`] presentation timestamp and
//! optional [`MediaDuration`] — and [`Frame`], the sum type decoded
//! frames travel as.
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
//! at the boundary, per the audio-core canonical-format rule.

use ruviform_audio_core::AudioBuffer;
use ruviform_media_core::{MediaDuration, MediaTime};
use ruviform_video_core::VideoFrame;

/// A decoded audio frame: an [`AudioBuffer`] plus its timestamps.
///
/// Mirrors [`VideoFrame`]'s shape — the buffer, a [`MediaTime`] PTS in
/// the stream's time base, and `duration` as an `Option` only
/// (explicitly carried or derived downstream).
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
/// use ruviform_media_core::{MediaDuration, MediaTime, TimeBase};
/// use ruviform_media_io::AudioFrame;
///
/// let buffer = AudioBuffer::new(BufferSpec {
///     layout: ChannelLayout::Stereo,
///     frames: 1_024,
///     sample_rate: SampleRate::new(48_000).unwrap(),
/// });
/// let mut frame = AudioFrame::new(buffer, MediaTime::new(0, TimeBase::COMMON_90K));
/// frame.set_duration(Some(MediaDuration::new(1_920, TimeBase::COMMON_90K)));
/// assert_eq!(frame.pts().value(), 0);
/// assert_eq!(frame.duration().unwrap().value(), 1_920);
/// assert_eq!(frame.buffer().frames(), 1_024);
/// ```
#[derive(Debug)]
pub struct AudioFrame {
    /// The decoded samples — canonical planar `f32`.
    buffer: AudioBuffer,
    /// Presentation timestamp in the stream's time base.
    pts: MediaTime,
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
    pub const fn new(buffer: AudioBuffer, pts: MediaTime) -> Self {
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

    /// Presentation timestamp in the stream's time base.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    #[must_use]
    pub const fn pts(&self) -> MediaTime {
        self.pts
    }

    /// Sets the presentation timestamp.
    ///
    /// **RT contract:** non-RT only — `Copy` write.
    pub const fn set_pts(&mut self, pts: MediaTime) {
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

/// A decoded frame — the output of [`Decoder::receive_frame`] and the
/// input of [`Encoder::send_frame`].
///
/// The sum type keeps the two decoded-frame representations behind one
/// boundary type instead of splitting the decoder contract per media
/// kind (spec Deferred Design Concerns: the enum-split representation
/// was chosen over per-kind typed ports — `#[non_exhaustive]` leaves
/// room for further frame kinds such as `Data`-stream payloads).
///
/// **Send contract:** `Frame` is `Send` for the same reason as
/// [`AudioFrame`].
///
/// **RT contract:** non-RT only — part of the media-I/O boundary.
///
/// [`Decoder::receive_frame`]: crate::Decoder::receive_frame
/// [`Encoder::send_frame`]: crate::Encoder::send_frame
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
/// assert_eq!(frame.pts(), pts);
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub enum Frame {
    /// A decoded audio frame.
    Audio(AudioFrame),
    /// A decoded video frame.
    Video(VideoFrame),
}

impl Frame {
    /// The frame's presentation timestamp, regardless of media kind.
    ///
    /// **RT contract:** non-RT only — `Copy` value access.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    /// use ruviform_media_core::{MediaTime, TimeBase};
    /// use ruviform_media_io::{AudioFrame, Frame};
    ///
    /// let buffer = AudioBuffer::new(BufferSpec {
    ///     layout: ChannelLayout::Mono,
    ///     frames: 512,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// });
    /// let pts = MediaTime::new(45_000, TimeBase::COMMON_90K);
    /// let frame = Frame::Audio(AudioFrame::new(buffer, pts));
    /// assert_eq!(frame.pts(), pts);
    /// ```
    #[must_use]
    pub const fn pts(&self) -> MediaTime {
        match self {
            Self::Audio(frame) => frame.pts(),
            Self::Video(frame) => frame.pts(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{BufferSpec, ChannelLayout, SampleRate};
    use ruviform_media_core::TimeBase;
    use ruviform_video_core::PixelFormat;

    fn check_send<T: Send>() {}

    fn stereo_buffer(frames: usize) -> AudioBuffer {
        AudioBuffer::new(BufferSpec {
            layout: ChannelLayout::Stereo,
            frames,
            sample_rate: SampleRate::new(48_000).unwrap(),
        })
    }

    #[test]
    fn audio_frame_carries_pts_and_duration() {
        let mut frame = AudioFrame::new(
            stereo_buffer(256),
            MediaTime::new(9_000, TimeBase::COMMON_90K),
        );
        assert_eq!(frame.pts().value(), 9_000);
        assert_eq!(frame.duration(), None);

        frame.set_duration(Some(MediaDuration::new(480, TimeBase::COMMON_90K)));
        assert_eq!(frame.duration().unwrap().value(), 480);
        frame.set_pts(MediaTime::new(18_000, TimeBase::COMMON_90K));
        assert_eq!(frame.pts().value(), 18_000);
    }

    #[test]
    fn audio_frame_buffer_access_and_take() {
        let mut frame = AudioFrame::new(stereo_buffer(4), MediaTime::ZERO);
        frame.buffer_mut().channel_mut(0).unwrap()[0] = 0.5;
        assert_eq!(frame.buffer().channel(0).unwrap()[0], 0.5);
        let buffer = frame.into_buffer();
        assert_eq!(buffer.channels(), 2);
        assert_eq!(buffer.frames(), 4);
    }

    #[test]
    fn frame_enum_pts_matches_inner() {
        let pts = MediaTime::new(123, TimeBase::COMMON_90K);
        let audio = Frame::Audio(AudioFrame::new(stereo_buffer(8), pts));
        assert_eq!(audio.pts(), pts);

        let video_frame = VideoFrame::new_owned(PixelFormat::Rgba8, 2, 2, pts).unwrap();
        let video = Frame::Video(video_frame);
        assert_eq!(video.pts(), pts);
    }

    #[test]
    fn frame_send_bounds() {
        check_send::<AudioFrame>();
        check_send::<Frame>();
    }
}
