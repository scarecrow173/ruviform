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
//! - [`Device::open_input_stream`] runs the same negotiation and opens
//!   a real cpal input stream: interleaved backend data is unpacked to
//!   planar views per callback, capture timestamps carry the host's
//!   audited provenance, and `clock_domain` reports the host profile
//!   resolved at enumeration ([`clock_profile`]).
//!
//! Everything in this module is a control-path (non-RT) operation.

use cpal::traits::{DeviceTrait, StreamTrait};
use ruviform_audio_core::device::{
    Device, DeviceError, DeviceId, DeviceResult, InputDataCallback, InputErrorCallback,
    InputStream, InputStreamConfig, OutputDataCallback, OutputErrorCallback, OutputStream,
    OutputStreamConfig, SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{AudioFormat, SampleFormat};
use ruviform_media_core::{ClockDomainId, TimestampProvenance};

use crate::convert::{
    f32_format, map_sample_format, map_supported_input_range, map_supported_range, negotiate,
    negotiate_input, synthesize_default_format, synthesize_default_input_format,
};
use crate::error::{ErrorCtx, map_error};
use crate::stream::{
    CpalInputStream, CpalOutputStream, DataCallbackState, FRAMES_CAPACITY_HARD_CAP,
    InputCallbackState,
};

/// The clock/timestamp profile a cpal host's stream instants belong
/// to — resolved once per enumerated device from the host name.
///
/// `domain` is what [`InputStream::clock_domain`] /
/// [`OutputStream::clock_domain`] report and what every
/// [`InputCallbackInfo`](ruviform_audio_core::device::InputCallbackInfo)
/// carries; `capture_provenance` labels the backend's capture
/// timestamp. Both values are claims about the *backend's* clock —
/// never fabricated: hosts whose `StreamInstant` clock cannot be
/// proven shared across streams report [`ClockDomainId::UNKNOWN`],
/// which `same_clock_domain` correctly treats as *not* same-clock
/// (duplex correlation then falls back rather than risking a false
/// same-clock claim).
#[derive(Copy, Clone, Debug)]
pub(crate) struct ClockProfile {
    /// The `ClockDomainId` reported for streams on this host.
    pub(crate) domain: ClockDomainId,
    /// The provenance applied to backend capture timestamps.
    pub(crate) capture_provenance: TimestampProvenance,
}

/// Resolves the [`ClockProfile`] for a cpal host name (e.g. `"WASAPI"`).
///
/// Audited against cpal 0.18.2 host sources (`src/host/*/…`):
///
/// | host | `StreamInstant` clock | capture timestamp | domain |
/// | ---- | --------------------- | ----------------- | ------ |
/// | `WASAPI` | `QueryPerformanceCounter` — system-wide, shared by all streams | `GetBuffer` `qpc_position` — hardware-reported capture instant | `MONOTONIC` / `Carried` |
/// | `CoreAudio` | `mach_absolute_time` — system-wide | `callback - declared latency` | `MONOTONIC` / `Derived` |
/// | `PipeWire` | `pw_stream_get_time_n` — graph clock, shared across the graph | `now - pw delay` | `MONOTONIC` / `Derived` |
/// | `ALSA` | `snd_pcm_status_get_htstamp` — per-substream device clock; cross-stream/cross-direction comparability is unprovable | `callback - declared delay` | `UNKNOWN` / `Derived` |
/// | other/unknown | unaudited | unaudited | `UNKNOWN` / `Derived` |
///
/// `MONOTONIC` is claimed only where the cpal clock is a shared
/// monotonic clock (the well-known id names exactly these clocks —
/// QPC, `mach_absolute_time`, `CLOCK_MONOTONIC`-family). ALSA's
/// per-device clocks cannot prove same-clock even between the two
/// directions of one card, so duplex on ALSA correctly falls back.
/// Unaudited hosts stay `UNKNOWN` — an unverifiable domain is never
/// upgraded to a claim.
pub(crate) fn clock_profile(host_name: &str) -> ClockProfile {
    let monotonic = matches!(host_name, "WASAPI" | "CoreAudio" | "PipeWire");
    ClockProfile {
        domain: if monotonic {
            ClockDomainId::MONOTONIC
        } else {
            ClockDomainId::UNKNOWN
        },
        // WASAPI's `capture` is the `GetBuffer` `qpc_position` — a
        // backend-reported capture fact (`Carried`). Every other
        // audited host computes `capture = callback - declared
        // latency`, a derivation from declared parameters (`Derived`).
        capture_provenance: if host_name == "WASAPI" {
            TimestampProvenance::Carried
        } else {
            TimestampProvenance::Derived
        },
    }
}

/// A live handle to one backend output device — behind
/// `Box<dyn Device>` on the port surface.
pub(crate) struct CpalDevice {
    inner: cpal::Device,
    /// The clock/timestamp profile of the host this device was
    /// enumerated from — resolved once at enumeration, shared by every
    /// stream the device opens (input and output report the same
    /// domain, which is what makes `same_clock_domain` duplex
    /// correlation possible on shared-clock hosts).
    clock_profile: ClockProfile,
    /// The backend host display name (e.g. `"WASAPI"`) — retained for
    /// platform-specific input routing ([`native_input_route`]).
    host_name: &'static str,
    /// Whether this device handle is the host's *default input*
    /// device — selects reroutable async activation plus the
    /// default-device monitor on the Windows WASAPI-native input path
    /// (pinned endpoints activate their `IMMDevice` directly instead).
    default_input: bool,
}

impl CpalDevice {
    pub(crate) fn new(inner: cpal::Device, host_name: &'static str, default_input: bool) -> Self {
        Self {
            inner,
            clock_profile: clock_profile(host_name),
            host_name,
            default_input,
        }
    }
}

/// Whether [`CpalDevice::open_input_stream`] takes the
/// platform-native input path instead of the portable cpal path.
///
/// Only Windows/WASAPI implements a native path today
/// ([`crate::wasapi_input`], Issue #565 — it is the only route that
/// can carry the per-packet `IAudioCaptureClient::GetBuffer` device
/// position into `InputCallbackInfo::frame_position`). Every other
/// host and platform keeps the cpal path unchanged.
#[cfg(windows)]
fn native_input_route(host_name: &str, _default_input: bool) -> bool {
    host_name == "WASAPI"
}

/// Non-Windows hosts have no native input path — always `false`.
#[cfg(not(windows))]
fn native_input_route(_host_name: &str, _default_input: bool) -> bool {
    false
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
        let ranges = match self.inner.supported_output_configs() {
            Ok(ranges) => ranges,
            // `UnsupportedOperation` is the backend's "this device has
            // no output capability" — that is the contract's
            // `Ok(vec![])`, not a failed query.
            Err(e) if e.kind() == cpal::ErrorKind::UnsupportedOperation => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(map_error(&e, ErrorCtx::Control)),
        };
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
        let mut state = DataCallbackState::new(
            data_callback,
            scratch,
            channels,
            frames_capacity,
            negotiated.stream.sample_rate,
        );
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
            self.clock_profile.domain,
        )))
    }

    fn default_input_format(&self) -> DeviceResult<AudioFormat> {
        let default = self
            .inner
            .default_input_config()
            .map_err(|e| map_error(&e, ErrorCtx::Control))?;
        // Same rules as `default_output_format`: fast path when the
        // backend default is itself an openable F32 form, else
        // synthesize from the supported space.
        if map_sample_format(default.sample_format()) == Some(SampleFormat::F32)
            && let Some(format) = f32_format(default.sample_rate(), u32::from(default.channels()))
        {
            return Ok(format);
        }
        let supported = self.supported_input_formats()?;
        synthesize_default_input_format(
            &supported,
            u32::from(default.channels()),
            default.sample_rate(),
        )
        .ok_or(DeviceError::UnsupportedConfig)
    }

    fn supported_input_formats(&self) -> DeviceResult<Vec<SupportedInputFormat>> {
        let ranges = match self.inner.supported_input_configs() {
            Ok(ranges) => ranges,
            // `UnsupportedOperation` is the backend's "this device has
            // no input capability" — that is the contract's
            // `Ok(vec![])`, not a failed query.
            Err(e) if e.kind() == cpal::ErrorKind::UnsupportedOperation => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(map_error(&e, ErrorCtx::Control)),
        };
        // Same untrusted-report validation as output — empty `Vec`
        // means "no usable F32-capable input range" (including devices
        // with no input capability), distinct from a failed query.
        Ok(ranges
            .filter_map(|range| map_supported_input_range(&range))
            .collect())
    }

    fn open_input_stream(
        &self,
        config: &InputStreamConfig,
        data_callback: InputDataCallback,
        error_callback: InputErrorCallback,
    ) -> DeviceResult<Box<dyn InputStream>> {
        // The adapter opens F32 only — F64/integer paths are
        // "unsupported" expressed as an error, not a silent conversion.
        if config.format.sample_format != SampleFormat::F32 {
            return Err(DeviceError::UnsupportedConfig);
        }
        // A device with no input capability at all is `Unsupported`
        // (the backend cannot drive input), distinct from a config the
        // device could not satisfy (`UnsupportedConfig`).
        if !self.inner.supports_input() {
            return Err(DeviceError::Unsupported);
        }
        let supported = self.supported_input_formats()?;
        let negotiated =
            negotiate_input(config, &supported).ok_or(DeviceError::UnsupportedConfig)?;

        // Size the planar scratch once, at open (non-RT) — identical
        // rule to the output direction: the bounded maximum a callback
        // can ask for in one pass; larger actual frame counts are
        // absorbed by chunking, not reallocation.
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

        // Route to the platform-native input path where one exists —
        // currently only Windows/WASAPI (`wasapi_input`, Issue #565).
        // `native_input` is always `false` off Windows.
        let native_input = native_input_route(self.host_name, self.default_input);
        #[cfg(windows)]
        if native_input {
            return crate::wasapi_input::open_input_stream(
                &self.inner,
                self.default_input,
                &negotiated.stream,
                config.format,
                scratch,
                frames_capacity,
                data_callback,
                error_callback,
                self.clock_profile.domain,
                self.clock_profile.capture_provenance,
            );
        }
        #[cfg(not(windows))]
        let _ = native_input;

        // Ownership split mirrors the output direction: the data
        // closure owns `InputCallbackState` (callback + scratch +
        // clock profile) outright — no `Arc`/`Mutex` sharing with the
        // error closure, which owns `error_callback` alone.
        let mut state = InputCallbackState::new(
            data_callback,
            scratch,
            channels,
            frames_capacity,
            negotiated.stream.sample_rate,
            self.clock_profile.domain,
            self.clock_profile.capture_provenance,
        );
        let mut error_callback = error_callback;
        let stream = self
            .inner
            .build_input_stream::<f32, _, _>(
                negotiated.stream,
                move |data: &[f32], info: &cpal::InputCallbackInfo| {
                    // cpal exposes no frame position — `None` is the
                    // honest "unsupported", never a synthesized counter
                    // that would hide a real discontinuity.
                    state.handle(data, info, None);
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
        Ok(Box::new(CpalInputStream::new(
            stream,
            config.format,
            epoch,
            self.clock_profile.domain,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::device::same_clock_domain;

    #[test]
    fn clock_profile_shared_monotonic_hosts_claim_monotonic() {
        // WASAPI (QPC), CoreAudio (mach_absolute_time) and PipeWire
        // (graph clock) all use a clock shared across streams — the
        // honest claim that enables `same_clock_domain` duplex
        // correlation.
        for host in ["WASAPI", "CoreAudio", "PipeWire"] {
            let profile = clock_profile(host);
            assert_eq!(profile.domain, ClockDomainId::MONOTONIC, "{host}");
        }
    }

    #[test]
    fn clock_profile_unverifiable_hosts_stay_unknown() {
        // ALSA's per-substream device clocks cannot prove
        // cross-stream comparability; unaudited hosts are never
        // upgraded to a claim. `UNKNOWN == UNKNOWN` must not read as
        // same-clock — the fallback path consumers rely on.
        for host in ["ALSA", "JACK", "PulseAudio", "ASIO", "null", "custom"] {
            let profile = clock_profile(host);
            assert_eq!(profile.domain, ClockDomainId::UNKNOWN, "{host}");
        }
        assert!(!same_clock_domain(
            clock_profile("ALSA").domain,
            clock_profile("ALSA").domain,
        ));
    }

    #[test]
    fn clock_profile_input_and_output_share_the_domain() {
        // Both stream directions of one host report the same domain —
        // the precondition `same_clock_domain(in, out)` checks.
        let wasapi = clock_profile("WASAPI");
        assert!(same_clock_domain(wasapi.domain, wasapi.domain));
        let alsa = clock_profile("ALSA");
        assert!(!same_clock_domain(alsa.domain, alsa.domain));
    }

    #[test]
    fn clock_profile_capture_provenance_matches_backend_semantics() {
        // WASAPI's capture is the `GetBuffer` `qpc_position` — a
        // carried fact. Every other audited host computes
        // `capture = callback - declared latency` — derived.
        assert_eq!(
            clock_profile("WASAPI").capture_provenance,
            TimestampProvenance::Carried,
        );
        for host in ["CoreAudio", "PipeWire", "ALSA", "unknown-host"] {
            assert_eq!(
                clock_profile(host).capture_provenance,
                TimestampProvenance::Derived,
                "{host}"
            );
        }
    }
}
