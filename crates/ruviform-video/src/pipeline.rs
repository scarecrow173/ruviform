//! Pull-driven frame pipeline.
//!
//! Stages transform [`VideoFrame`] in registration order — the
//! destination pulls a decoded frame through each stage. All pipeline
//! APIs are non-RT: frames are owned values and stages may allocate.

use ruviform_media_io::Result;
use ruviform_video_core::VideoFrame;

/// One frame transform stage in the pipeline.
///
/// **RT contract:** non-RT only — stages may allocate.
pub trait VideoStage: Send {
    /// Transforms `frame`, returning the result frame.
    ///
    /// # Errors
    ///
    /// Propagates stage-defined failures via [`ruviform_media_io::Error`].
    fn process(&mut self, frame: VideoFrame) -> Result<VideoFrame>;
}

/// An ordered stage list; `run` applies stages in registration order.
///
/// **RT contract:** non-RT only.
pub struct FramePipeline {
    stages: Vec<Box<dyn VideoStage>>,
}

impl FramePipeline {
    /// Empty pipeline — `run` passes frames through unchanged.
    #[must_use]
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    /// Appends a stage to the pull order.
    pub fn push(&mut self, stage: Box<dyn VideoStage>) {
        self.stages.push(stage);
    }

    /// Number of stages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Whether the pipeline has no stages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Pulls `frame` through every stage in order.
    ///
    /// # Errors
    ///
    /// Propagates the first stage error; later stages do not run.
    pub fn run(&mut self, frame: VideoFrame) -> Result<VideoFrame> {
        let mut frame = frame;
        for stage in &mut self.stages {
            frame = stage.process(frame)?;
        }
        Ok(frame)
    }
}

impl Default for FramePipeline {
    fn default() -> Self {
        Self::new()
    }
}
