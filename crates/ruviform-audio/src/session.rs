//! Device session: owns the `OutputStream` and wires the RT data
//! callback to the active [`GraphPlan`].

use ruviform_audio_core::device::{
    Device, DeviceError, OutputErrorCallback, OutputStream, OutputStreamConfig,
};
use ruviform_rt::RtContext;

use crate::plan::PlanSwap;

/// Session construction/operation failure.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum SessionError {
    /// The device rejected the stream open.
    Open(DeviceError),
    /// `start`/`stop` was refused.
    Stream(DeviceError),
}

impl core::fmt::Display for SessionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Open(e) => write!(f, "open output stream failed: {e:?}"),
            Self::Stream(e) => write!(f, "stream control failed: {e:?}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// A live output session. Owns the stream; the callback keeps running
/// until the session is dropped.
///
/// **RT contract:** construction/`start`/`stop`/drop are non-RT only.
pub struct Session {
    stream: Box<dyn OutputStream>,
    swap: PlanSwap,
}

impl Session {
    /// Opens an output stream on `device` and wires the RT callback to
    /// drive the active plan. `plan` becomes the initial plan; further
    /// plans are published through [`Session::swap`].
    ///
    /// # Errors
    ///
    /// [`SessionError::Open`] when the device refuses the stream.
    pub fn open(
        device: &dyn Device,
        config: OutputStreamConfig,
        plan: crate::plan::GraphPlan,
    ) -> Result<Self, SessionError> {
        let (swap, rt_plan) = PlanSwap::new(plan);
        let mut rt_plan = Some(rt_plan);
        let callback = Box::new(
            move |mut out: ruviform_audio_core::AudioBufferViewMut<'_, f32>,
                  _info: ruviform_audio_core::device::OutputCallbackInfo| {
                let Some(rt_plan) = rt_plan.as_mut() else {
                    return;
                };
                let Some(plan) = rt_plan.active() else { return };
                let ctx = RtContext::mint();
                plan.drive(&mut out, &ctx);
            },
        );
        let error_cb: OutputErrorCallback = Box::new(|_e: DeviceError| {});
        let stream = device
            .open_output_stream(&config, callback, error_cb)
            .map_err(SessionError::Open)?;
        Ok(Self { stream, swap })
    }

    /// Publishes a replacement plan; the RT side picks it up on the next
    /// callback block.
    ///
    /// **RT contract:** non-RT only.
    pub fn swap(&self, plan: crate::plan::GraphPlan) {
        self.swap.swap(plan);
    }

    /// Starts output delivery.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SessionError::Stream`] when the backend refuses.
    pub fn start(&self) -> Result<(), SessionError> {
        self.stream.start().map_err(SessionError::Stream)
    }

    /// Stops output delivery.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`SessionError::Stream`] when the backend refuses.
    pub fn stop(&self) -> Result<(), SessionError> {
        self.stream.stop().map_err(SessionError::Stream)
    }

    /// The stream's negotiated format.
    ///
    /// **RT contract:** RT-suitable — returns a stored value.
    #[must_use]
    pub fn format(&self) -> ruviform_audio_core::AudioFormat {
        self.stream.format()
    }
}
