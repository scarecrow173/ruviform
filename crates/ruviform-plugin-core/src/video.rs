//! Video plugin contract (Issue #98).
//!
//! Neutral, NONRT video plugin port. All operations — including
//! [`VideoPluginInstance::render`] — are called from host control-plane
//! threads; none carry an audio real-time guarantee.
//!
//! Depends downward on `ruviform-video-core` / `ruviform-media-core` only;
//! no adapter or foreign-ABI type appears here.

use crate::descriptor::PluginDescriptorView;
use crate::lifecycle::LifecycleState;
use crate::param::ParamId;
use crate::plain_param::{PlainParamInfo, PlainParamValue};
use ruviform_media_core::{FrameRate, MediaTime, Result as MediaResult};
use ruviform_video_core::{PixelFormat, VideoFrame};

/// Video processing configuration handed to `initialize`.
///
/// Validates only non-zero dimensions — the neutral core does not restrict
/// pixel format, stride, or layout (those are adapter-level negotiations).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VideoProcessConfig {
    format: PixelFormat,
    width: u32,
    height: u32,
    frame_rate: FrameRate,
}

impl VideoProcessConfig {
    /// Create a config; `width` and `height` must be non-zero.
    pub fn new(
        format: PixelFormat,
        width: u32,
        height: u32,
        frame_rate: FrameRate,
    ) -> MediaResult<Self> {
        if width == 0 || height == 0 {
            return Err(ruviform_media_core::Error::InvalidValue(
                "VideoProcessConfig dimensions",
            ));
        }
        Ok(Self {
            format,
            width,
            height,
            frame_rate,
        })
    }

    /// Pixel format.
    #[must_use]
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Frame rate.
    #[must_use]
    pub fn frame_rate(&self) -> FrameRate {
        self.frame_rate
    }
}

/// Video plugin instance — NONRT host-facing contract.
///
/// The borrowed [`PluginDescriptorView`] never outlives the instance:
///
/// ```compile_fail
/// use ruviform_plugin_core::{PluginDescriptorView, VideoPluginInstance};
/// fn escape<T: VideoPluginInstance + 'static>(owner: T) -> PluginDescriptorView<'static> {
///     owner.descriptor()
/// }
/// ```
///
/// `state`/`descriptor`/`parameters`/`parameter` are metadata reads legal in
/// every lifecycle state including `Destroyed` (owned caches / read-only
/// views — no foreign calls). `parameter_value`/`set_parameter` semantics per
/// lifecycle state are the adapter's responsibility within the common
/// [`crate::lifecycle::Lifecycle`] protocol.
///
/// All methods are called from the host control plane; `render` is a
/// synchronous NONRT call, not a real-time audio callback.
pub trait VideoPluginInstance {
    /// Instance error type.
    type Error: std::error::Error + 'static;

    /// Current lifecycle state.
    fn state(&self) -> LifecycleState;

    /// Borrowed descriptor view.
    fn descriptor(&self) -> PluginDescriptorView<'_>;

    /// Stable list of plain-parameter identifiers.
    fn parameters(&self) -> &[ParamId];

    /// Borrowed parameter metadata for `id`, if known.
    fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>>;

    /// Current plain parameter value.
    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, Self::Error>;

    /// Set a plain parameter value (validation per adapter contract).
    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), Self::Error>;

    /// Configure processing dimensions/format; begins the instance lifecycle.
    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), Self::Error>;

    /// Activate resources after successful initialize.
    fn activate(&mut self) -> Result<(), Self::Error>;

    /// Enter the processing-capable state.
    fn start_processing(&mut self) -> Result<(), Self::Error>;

    /// Render one frame: `input` is the source frame, `output` the target.
    /// NONRT synchronous call.
    fn render(
        &mut self,
        input: &VideoFrame,
        output: &mut VideoFrame,
        at: MediaTime,
    ) -> Result<(), Self::Error>;

    /// Leave the processing state; stays activated.
    fn stop_processing(&mut self) -> Result<(), Self::Error>;

    /// Deactivate resources; returns to initialized.
    fn deactivate(&mut self) -> Result<(), Self::Error>;

    /// Terminal destruction; the instance is no longer usable.
    fn destroy(&mut self) -> Result<(), Self::Error>;
}
