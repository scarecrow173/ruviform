//! Playback session — demux → decode → pipeline mediation + seek.
//!
//! Owns the non-RT side of video playback: packet pull, decoder
//! driving, pipeline application, and seek orchestration
//! (`Demuxer::seek` + `Decoder::flush` + scheduler resync).

use ruviform_media_core::MediaTime;
use ruviform_media_io::{Decoder, Demuxer, Frame, Result};
use ruviform_video_core::VideoFrame;

use crate::{FramePipeline, FrameScheduler, SyncDecision};

/// Mediates demuxer, decoder, pipeline, and scheduler for one video
/// stream. Generic over the boundary traits — no backend types.
///
/// **RT contract:** non-RT only — I/O and codec calls may block and
/// allocate. The A/V sync face reads [`ruviform_rt::MasterClock`]
/// non-blockingly.
pub struct VideoSession<D: Demuxer, C: Decoder> {
    demuxer: D,
    decoder: C,
    pipeline: FramePipeline,
    scheduler: FrameScheduler,
    eos: bool,
}

impl<D: Demuxer, C: Decoder> VideoSession<D, C> {
    /// Creates a session from the boundary implementations.
    ///
    /// **RT contract:** non-RT only.
    pub fn new(demuxer: D, decoder: C, pipeline: FramePipeline, scheduler: FrameScheduler) -> Self {
        Self {
            demuxer,
            decoder,
            pipeline,
            scheduler,
            eos: false,
        }
    }

    /// Pulls the next pipeline-processed video frame.
    ///
    /// Audio frames are skipped (A/V sync is the audio engine's
    /// responsibility). Returns `Ok(None)` at end of stream — packets
    /// stop, the decoder is drained once, and subsequent calls keep
    /// returning `None` until [`Self::seek`].
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Propagates demuxer/decoder/pipeline errors.
    pub fn next_frame(&mut self) -> Result<Option<VideoFrame>> {
        loop {
            match self.decoder.receive_frame()? {
                Some(Frame::Video(frame)) => {
                    return Ok(Some(self.pipeline.run(frame)?));
                }
                // Frame is non_exhaustive — future kinds are skipped
                // like audio: they belong to a different engine.
                Some(_) => {}
                None => {
                    if self.eos {
                        return Ok(None);
                    }
                    if let Some(packet) = self.demuxer.read_packet()? {
                        self.decoder.send_packet(packet)?;
                    } else {
                        self.decoder.drain()?;
                        self.eos = true;
                    }
                }
            }
        }
    }

    /// A/V sync decision for `frame` against the master clock.
    ///
    /// **RT contract:** non-RT only — non-blocking clock read.
    #[must_use]
    pub fn decide(&self, frame: &VideoFrame, clock: &ruviform_rt::MasterClock) -> SyncDecision {
        self.scheduler
            .decide(frame.pts(), FrameScheduler::master_time(clock))
    }

    /// Seeks the stream to `position` and resynchronizes.
    ///
    /// Order: `Demuxer::seek` → `Decoder::flush` → scheduler floor at
    /// `position` (stale frames predating the seek are dropped).
    ///
    /// **RT contract:** non-RT only — may block on I/O.
    ///
    /// # Errors
    ///
    /// Propagates demuxer/decoder errors; on seek failure the decoder
    /// is left un-flushed and the scheduler floor unchanged.
    pub fn seek(&mut self, position: MediaTime) -> Result<()> {
        self.demuxer.seek(position)?;
        self.decoder.flush()?;
        self.scheduler.resync_at(position);
        self.eos = false;
        Ok(())
    }

    /// The session's scheduler.
    #[must_use]
    pub fn scheduler(&self) -> &FrameScheduler {
        &self.scheduler
    }

    /// The session's pipeline.
    #[must_use]
    pub fn pipeline(&self) -> &FramePipeline {
        &self.pipeline
    }
}
