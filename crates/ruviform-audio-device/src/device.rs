//! [`CpalDevice`] — wraps the backend's device handle privately and
//! implements the port's [`Device`] trait (describe + open).
//!
//! Format duties owned here (design "マッピング設計"):
//!
//! - [`Device::default_output_format`] returns an openable
//!   `F32` form preferentially — even when the backend default is
//!   `F64`/integer-encoded, an [`AudioFormat`] synthesized from an
//!   F32-capable range keeps the caller's open flow off an immediate
//!   [`DeviceError::UnsupportedConfig`].
//! - [`Device::supported_output_formats`] reports F32-capable ranges
//!   only: an empty `Vec` means "no F32-capable range", distinct from a
//!   failed query (`Err`).
//! - [`Device::open_output_stream`] rejects non-F32 configs and
//!   out-of-space parameters with [`DeviceError::UnsupportedConfig`]
//!   before touching the backend.
//!
//! Everything in this module is a control-path (non-RT) operation.

use cpal::traits::{DeviceTrait, StreamTrait};
use ruviform_audio_core::device::{
    Device, DeviceError, DeviceId, DeviceResult, OutputDataCallback, OutputErrorCallback,
    OutputStream, OutputStreamConfig, SupportedOutputFormat,
};
use ruviform_audio_core::{AudioFormat, SampleFormat};

use crate::convert::{
    f32_format, map_sample_format, map_supported_range, negotiate, synthesize_default_format,
};
use crate::error::{ErrorCtx, map_error};
use crate::stream::{CpalOutputStream, DataCallbackState, FRAMES_CAPACITY_HARD_CAP};

/// A live handle to one backend output device — behind
/// `Box<dyn Device>` on the port surface.
pub(crate) struct CpalDevice {
    inner: cpal::Device,
}

impl CpalDevice {
    pub(crate) fn new(inner: cpal::Device) -> Self {
        Self { inner }
    }
}

impl Device for CpalDevice {
    fn name(&self) -> DeviceResult<String> {
        // `Device::to_string()` would lose the query error (Display
        // maps it to `fmt::Error` and `to_string` panics on it) — ask
        // for the fallible description directly.
        self.inner
            .description()
            .map(|description| description.name().to_owned())
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn id(&self) -> DeviceResult<Option<DeviceId>> {
        // `Display` for cpal's `DeviceId` renders `<host>:<device>` —
        // a host-qualified identifier, opaque to the port.
        self.inner
            .id()
            .map(|id| Some(DeviceId::new(id.to_string())))
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn default_output_format(&self) -> DeviceResult<AudioFormat> {
        let default = self
            .inner
            .default_output_config()
            .map_err(|e| map_error(&e, ErrorCtx::Control))?;
        // Fast path: the backend default is itself an openable F32
        // form (the overwhelmingly common case).
        if map_sample_format(default.sample_format()) == Some(SampleFormat::F32)
            && let Some(format) = f32_format(default.sample_rate(), u32::from(default.channels()))
        {
            return Ok(format);
        }
        // The default is non-F32 or its channel count maps to no named
        // layout: synthesize an F32 form from the supported space so
        // the caller can still open without an immediate
        // `UnsupportedConfig`.
        let supported = self.supported_output_formats()?;
        synthesize_default_format(
            &supported,
            u32::from(default.channels()),
            default.sample_rate(),
        )
        .ok_or(DeviceError::UnsupportedConfig)
    }

    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
        let ranges = self
            .inner
            .supported_output_configs()
            .map_err(|e| map_error(&e, ErrorCtx::Control))?;
        // Report usable F32-capable ranges only — malformed reports
        // (non-F32, inverted/zero rate bounds, contradictory buffer
        // bounds) are dropped at this untrusted-data boundary, so an
        // empty `Vec` means "no usable F32-capable range" (distinct
        // from a failed query, which is `Err`). A bound report the
        // backend itself contradicts must not degrade to `Unknown`:
        // `Unknown` means "no bound reported" and legitimately admits
        // any `Fixed` request — a corrupt report must not gain that
        // admission.
        Ok(ranges
            .filter_map(|range| map_supported_range(&range))
            .collect())
    }

    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>> {
        // The PoC opens F32 only — F64/integer paths are "unsupported"
        // expressed as an error, not a silent conversion.
        if config.format.sample_format != SampleFormat::F32 {
            return Err(DeviceError::UnsupportedConfig);
        }
        let supported = self.supported_output_formats()?;
        let negotiated = negotiate(config, &supported).ok_or(DeviceError::UnsupportedConfig)?;

        // Size the planar scratch once, at open (non-RT): the bounded
        // maximum a callback can ask for in one pass. Larger actual
        // frame counts are absorbed by chunking, not reallocation.
        let frames_capacity = negotiated
            .effective_max_frames
            .min(FRAMES_CAPACITY_HARD_CAP);
        let channels = usize::from(negotiated.stream.channels);
        let scratch_len = frames_capacity
            .checked_mul(channels)
            .ok_or(DeviceError::UnsupportedConfig)?;
        let mut scratch = Vec::new();
        // Fallible allocation: a backing-store failure is reported as
        // `resource_exhausted`, never an abort mid-open.
        scratch
            .try_reserve_exact(scratch_len)
            .map_err(|_| DeviceError::Backend("resource_exhausted"))?;
        scratch.resize(scratch_len, 0.0);

        // Ownership split: the data closure owns `DataCallbackState`
        // (callback + scratch) outright — no `Arc`/`Mutex` sharing
        // with the error closure, which owns `error_callback` alone.
        // A lock on the RT thread would violate the RT contract.
        let mut state = DataCallbackState::new(data_callback, scratch, channels, frames_capacity);
        let mut error_callback = error_callback;
        let stream = self
            .inner
            .build_output_stream::<f32, _, _>(
                negotiated.stream,
                move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    state.handle(data, info);
                },
                move |e: cpal::Error| {
                    error_callback(map_error(&e, ErrorCtx::ErrorCallback));
                },
                None,
            )
            .map_err(|e| map_error(&e, ErrorCtx::Control))?;

        // The epoch anchor for the saturating timestamp fallback —
        // recorded post-open on the stream's own clock.
        let epoch = stream.now();
        Ok(Box::new(CpalOutputStream::new(
            stream,
            config.format,
            epoch,
        )))
    }
}
