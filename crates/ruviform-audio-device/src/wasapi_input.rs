//! WASAPI-native input stream — Issue #565.
//!
//! cpal's portable input callback (`cpal::InputCallbackInfo`) carries
//! only timestamps: the WASAPI backend reads the per-packet
//! `IAudioCaptureClient::GetBuffer` `devicePosition`/`qpcPosition`
//! out-params for its own discontinuity check and then discards the
//! device position, and `IAudioClock::GetPosition`'s position half is
//! dropped the same way (`let (callback, _position)`). The port's
//! [`InputCallbackInfo::frame_position`](ruviform_audio_core::device::InputCallbackInfo)
//! therefore can only be supplied honestly by driving the WASAPI
//! capture path natively — this module is that path, a faithful port
//! of cpal 0.18.2's `host/wasapi` input machinery plus the position
//! hand-off cpal cannot express.
//!
//! Timestamp provenance: `capture` is the per-packet `GetBuffer`
//! `qpcPosition` — the backend-reported instant the packet's first
//! frame was recorded, already on the stream clock. `callback` is a
//! fresh `QueryPerformanceCounter` read at packet-processing time —
//! the port contract's *host-observed delivery instant*, on the same
//! stream clock as [`NativeInputStream::now`]. cpal derives the
//! callback instant from `IAudioClock::GetPosition`'s qpc half, but
//! that is a position-*snapshot* instant: it reports `0` until the
//! audio clock first runs (the first packet's callback instant would
//! sit at the clock epoch, far *before* its own capture instant) and
//! it lags the just-recorded packet by up to ~1 ms, so `capture`
//! can appear after `callback`. A fresh counter read is honest on
//! both counts — the delivery instant is by definition not earlier
//! than the record instant.
//!
//! Scope: `#[cfg(windows)]` + host `"WASAPI"` input streams only
//! ([`crate::device::native_input_route`]). Every other host,
//! platform and the output direction keep the cpal path unchanged.
//! This is a platform gap-fill *inside* the same backend, not a second
//! backend crate — the crate's "second backend = separate crate"
//! rule cannot apply because the missing datum lives inside cpal's
//! private WASAPI stream state, unreachable from outside (design note:
//! `docs/designs/issue-565-wasapi-input-frame-position-design.md`).
//!
//! `frame_position` contract: each delivered packet reports
//! `Some(device_position)` — the `GetBuffer` device position of the
//! packet's first frame, a backend-reported capture-clock fact on the
//! same stream that carries the PCM (never synthesized from callback
//! counts). [`PositionTracker`] verifies per-packet contiguity
//! (`next == prev + frames`) and the `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY`
//! flag; either violation — or a stop/start position epoch reset —
//! surfaces as [`DeviceError::ClockDiscontinuity`] on the error
//! callback while the packet itself is still delivered at its real
//! position (the consumer decides where the epoch boundary is).
//!
//! RT contract: the packet loop is allocation-free and lock-free —
//! the planar scratch, callbacks and tracker are all preallocated at
//! open; the only per-packet work is bounded COM calls plus the
//! existing `read_interleaved` chunking. `unsafe` is confined to this
//! module (COM calls, the `GetBuffer` data pointer, event handles and
//! the `#[implement]` vtables) — each block carries a SAFETY comment.

// The whole module is the Win32/WASAPI FFI boundary: every `unsafe`
// block below is individually justified at its site, and the lint is
// scoped to this file only (workspace policy: local allowance +
// SAFETY comments — precedent `ruviform-midi-device/src/os/windows.rs`).
#![expect(unsafe_code)]
// `clippy::ref_as_ptr`/`clippy::inline_always` fire on code the
// `windows::core::implement` macro generates for the two COM callback
// vtables below (`IActivateAudioInterfaceCompletionHandler`,
// `IMMNotificationClient`). The lint spans resolve to the attribute
// position, so no item-level expectation can capture them — the
// expectation is scoped to this module, which contains no other use
// of either pattern.
#![expect(
    clippy::ref_as_ptr,
    clippy::inline_always,
    reason = "windows::core::implement macro output; lint spans point at the attribute"
)]

use std::mem;
use std::ptr;
use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use cpal::{InputCallbackInfo as CpalInputCallbackInfo, InputStreamTimestamp, StreamInstant};
use ruviform_audio_core::AudioFormat;
use ruviform_audio_core::device::{
    BufferSize, DeviceError, DeviceResult, InputDataCallback, InputErrorCallback, InputStream,
    LatencyCoverage, StreamLatency,
};
use ruviform_media_core::{ClockDomainId, MediaTime, TimestampProvenance};

use windows::Win32::Foundation::{self, PROPERTYKEY};
use windows::Win32::Media::{Audio, KernelStreaming, Multimedia};
use windows::Win32::System::{Com, Performance, SystemServices, Threading};
use windows::core::Interface;

use crate::error::ErrorCtx;
use crate::stream::{InputCallbackState, instant_to_media_time};

/// `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` as the raw `u32` bit the
/// `GetBuffer` flags out-param carries.
const DISCONTINUITY_FLAG: u32 = Audio::AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32;

thread_local! {
    /// Per-thread COM apartment guard — lazily initialised on first
    /// use (STA, matching cpal's `host::com`) and balanced by
    /// [`ComGuard`]'s `Drop` on thread exit.
    static COM_GUARD: ComGuard = ComGuard::acquire();
}

/// RAII record of this thread's `CoInitializeEx` outcome.
struct ComGuard {
    /// `true` when our `CoInitializeEx` succeeded (`S_OK`/`S_FALSE`)
    /// and must be balanced by `CoUninitialize`. `false` for
    /// `RPC_E_CHANGED_MODE` (COM was already initialised elsewhere —
    /// not ours to uninitialise) and for hard failures.
    uninit_on_drop: bool,
    /// The raw result — [`ensure_com`] re-checks it so a hard COM
    /// failure surfaces as [`DeviceError`], never a panic.
    hr: windows::core::HRESULT,
}

impl ComGuard {
    fn acquire() -> Self {
        // SAFETY: standard COM apartment initialisation; the result is
        // recorded, not assumed — callers observe it through
        // `ensure_com`.
        let hr = unsafe { Com::CoInitializeEx(None, Com::COINIT_APARTMENTTHREADED) };
        Self {
            uninit_on_drop: hr.is_ok(),
            hr,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.uninit_on_drop {
            // SAFETY: balances the successful `CoInitializeEx` above —
            // called once per successful init on this thread.
            unsafe { Com::CoUninitialize() };
        }
    }
}

/// Ensures COM is initialised on the calling thread (idempotent).
fn ensure_com() -> DeviceResult<()> {
    COM_GUARD.with(|guard| {
        // `S_OK`/`S_FALSE`: initialised (by us or already — either way
        // usable). `RPC_E_CHANGED_MODE`: COM is live in another
        // apartment; calls still work through COM marshalling.
        if guard.hr.is_ok() || guard.hr == Foundation::RPC_E_CHANGED_MODE {
            Ok(())
        } else {
            Err(DeviceError::Backend("com_unavailable"))
        }
    })
}

/// RAII wrapper freeing a `CoTaskMem`-allocated `PWSTR` — ported from
/// cpal's `host::com::ComString`.
struct ComString(windows::core::PWSTR);

impl Drop for ComString {
    fn drop(&mut self) {
        // SAFETY: `self.0` was allocated by `StringFromIID` (a
        // `CoTaskMemAlloc`-family call) and is freed exactly once here.
        unsafe { Com::CoTaskMemFree(Some(self.0.as_ptr().cast())) }
    }
}

/// RAII guard for a `CreateEvent` handle during stream open — closes
/// the handle if a later open step fails. Once ownership moves into
/// [`WorkerEvents`] the worker-side `Drop` takes over (exactly-once).
struct HandleGuard(Foundation::HANDLE);

impl HandleGuard {
    fn create() -> DeviceResult<Self> {
        create_event().map(Self)
    }

    /// The raw handle value — `HANDLE` is `Copy`, so callers keep
    /// using copies while the guard retains close ownership.
    fn get(&self) -> Foundation::HANDLE {
        self.0
    }

    /// Releases close ownership — the callee becomes responsible for
    /// exactly one `CloseHandle`.
    fn into_inner(self) -> Foundation::HANDLE {
        let handle = self.0;
        mem::forget(self);
        handle
    }
}

impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a live event handle created by
        // `create_event`; closing it here is the guard's single
        // ownership duty on the open-error path.
        unsafe {
            let _ = Foundation::CloseHandle(self.0);
        }
    }
}

/// Creates an unnamed auto-reset event, initially non-signalled.
fn create_event() -> DeviceResult<Foundation::HANDLE> {
    // SAFETY: NULL security attributes, auto-reset, initially clear,
    // unnamed — the standard event shape cpal uses for both the audio
    // and command-wake events.
    unsafe {
        Threading::CreateEventW(None, false, false, windows::core::PCWSTR::null())
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))
    }
}

/// Normalizes a `windows::core::Error` (HRESULT) onto the port's
/// [`DeviceError`] — the WASAPI-native counterpart of
/// [`crate::error::map_error`], following the same per-context rules
/// (device-loss reads differently on the control vs error-callback
/// path).
fn map_win_error(e: &windows::core::Error, ctx: ErrorCtx) -> DeviceError {
    let hr = e.code();
    if hr == Audio::AUDCLNT_E_SERVICE_NOT_RUNNING {
        // The audio service itself is gone — host-level, not a
        // per-device failure.
        DeviceError::Backend("host_unavailable")
    } else if hr == Audio::AUDCLNT_E_DEVICE_INVALIDATED
        || hr == Audio::AUDCLNT_E_ENDPOINT_CREATE_FAILED
    {
        match ctx {
            ErrorCtx::Control => DeviceError::DeviceNotFound,
            ErrorCtx::ErrorCallback => DeviceError::StreamInvalidated,
        }
    } else if hr == Audio::AUDCLNT_E_DEVICE_IN_USE {
        DeviceError::DeviceBusy
    } else if hr == Audio::AUDCLNT_E_RESOURCES_INVALIDATED {
        DeviceError::StreamInvalidated
    } else if hr == Audio::AUDCLNT_E_UNSUPPORTED_FORMAT {
        DeviceError::UnsupportedConfig
    } else if hr == Audio::AUDCLNT_E_WRONG_ENDPOINT_TYPE {
        DeviceError::Unsupported
    } else if hr == Foundation::E_ACCESSDENIED {
        // The OS-level access denial (e.g. microphone privacy
        // settings) — classified the same as the cpal path's
        // `ErrorKind::PermissionDenied`, so one OS refusal reads
        // identically on both routes.
        DeviceError::Backend("permission_denied")
    } else {
        // Any other COM/Win32 failure — classification only, the
        // backend message is deliberately not captured (port rule).
        DeviceError::Backend("backend")
    }
}

/// Converts a QPC-derived count into the stream clock's
/// [`StreamInstant`] — the same 100 ns grid WASAPI `QPCPosition`
/// values already use (cpal `Stream::now` parity).
fn perf_units_to_stream_instant(units_100ns: u128) -> StreamInstant {
    let nanos = units_100ns * 100;
    StreamInstant::new(
        u64::try_from(nanos / 1_000_000_000).unwrap_or(u64::MAX),
        u32::try_from(nanos % 1_000_000_000).unwrap_or(u32::MAX),
    )
}

/// A QPC position value (already in 100 ns units, as delivered by
/// `IAudioClock::GetPosition`/`IAudioCaptureClient::GetBuffer`) to a
/// [`StreamInstant`]. `0` maps to [`StreamInstant::ZERO`] — the
/// "not reported" sentinel `map_input_timestamp` already treats as
/// `capture: None`.
fn qpc100ns_to_stream_instant(units_100ns: u64) -> StreamInstant {
    perf_units_to_stream_instant(u128::from(units_100ns))
}

/// `QueryPerformanceFrequency` — constant for the system lifetime,
/// cached once at open.
fn query_qpc_frequency() -> DeviceResult<u64> {
    let mut frequency = 0i64;
    // SAFETY: `frequency` is a valid i64 slot for the call.
    unsafe {
        Performance::QueryPerformanceFrequency(&raw mut frequency)
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    }
    u64::try_from(frequency)
        .ok()
        .filter(|f| *f != 0)
        .ok_or(DeviceError::Backend("backend"))
}

/// The current `QueryPerformanceCounter` instant on the stream clock.
fn perf_counter_instant(qpc_frequency: u64) -> DeviceResult<StreamInstant> {
    let mut counter = 0i64;
    // SAFETY: `counter` is a valid i64 slot for the call.
    unsafe {
        Performance::QueryPerformanceCounter(&raw mut counter)
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    }
    // Convert to 100 ns units first so `now()` shares the exact grid
    // of the WASAPI `QPCPosition` callback/capture instants (cpal
    // `Stream::now` parity — avoids false sub-100 ns deltas). QPC
    // counters are non-negative; a negative value would be a broken
    // clock and clamps to 0 rather than wrapping.
    let units_100ns = u128::try_from(counter).unwrap_or(0) * 10_000_000 / u128::from(qpc_frequency);
    Ok(perf_units_to_stream_instant(units_100ns))
}

/// The `WAVEFORMATEXTENSIBLE` descriptor for 32-bit float PCM — the
/// only encoding this adapter opens. Ported from cpal's
/// `config_to_waveformatextensible` reduced to the F32 case (IEEE
/// float subtype, `KSAUDIO_SPEAKER_DIRECTOUT` channel mask — no
/// speaker-position remapping, matching cpal).
fn wave_format_extensible_f32(
    channels: u16,
    sample_rate: u32,
) -> DeviceResult<Audio::WAVEFORMATEXTENSIBLE> {
    const SAMPLE_BYTES: u16 = 4; // f32
    let block_align = channels
        .checked_mul(SAMPLE_BYTES)
        .ok_or(DeviceError::UnsupportedConfig)?;
    let avg_bytes_per_sec = u64::from(channels) * u64::from(sample_rate) * u64::from(SAMPLE_BYTES);
    let avg_bytes_per_sec =
        u32::try_from(avg_bytes_per_sec).map_err(|_| DeviceError::UnsupportedConfig)?;
    let cb_size =
        u16::try_from(size_of::<Audio::WAVEFORMATEXTENSIBLE>() - size_of::<Audio::WAVEFORMATEX>())
            .map_err(|_| DeviceError::UnsupportedConfig)?;
    Ok(Audio::WAVEFORMATEXTENSIBLE {
        Format: Audio::WAVEFORMATEX {
            wFormatTag: u16::try_from(KernelStreaming::WAVE_FORMAT_EXTENSIBLE)
                .map_err(|_| DeviceError::UnsupportedConfig)?,
            nChannels: channels,
            nSamplesPerSec: sample_rate,
            nAvgBytesPerSec: avg_bytes_per_sec,
            nBlockAlign: block_align,
            wBitsPerSample: 32,
            cbSize: cb_size,
        },
        Samples: Audio::WAVEFORMATEXTENSIBLE_0 {
            wValidBitsPerSample: 32,
        },
        dwChannelMask: KernelStreaming::KSAUDIO_SPEAKER_DIRECTOUT,
        SubFormat: Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
    })
}

/// `BufferSize` → buffer duration in 100 ns units for
/// `IAudioClient::Initialize` — cpal `buffer_size_to_duration` parity
/// (round-to-nearest; in shared mode the value only sizes the
/// ring buffer — the callback period always comes from
/// `GetDevicePeriod`).
fn buffer_duration_100ns(buffer_size: cpal::BufferSize, sample_rate: u32) -> i64 {
    match buffer_size {
        cpal::BufferSize::Fixed(frames) => {
            let rate = i64::from(sample_rate);
            (i64::from(frames) * 10_000_000 + rate / 2) / rate
        }
        cpal::BufferSize::Default => 0,
    }
}

/// The callback period in frames for a shared-mode stream —
/// `GetDevicePeriod` converted to frames, falling back to the total
/// buffer size (cpal `shared_mode_period_frames` parity).
fn shared_mode_period_frames(
    audio_client: &Audio::IAudioClient,
    sample_rate: u32,
    max_frames_in_buffer: u32,
) -> u32 {
    let mut default_period = 0i64;
    // SAFETY: `default_period` is a valid i64 slot for the call.
    let ok = unsafe {
        audio_client
            .GetDevicePeriod(Some(&raw mut default_period), None)
            .is_ok()
    };
    if ok && default_period > 0 {
        // 100 ns duration → frames at the stream rate, rounded to
        // nearest (cpal `buffer_duration_to_frames` formula).
        let frames = (default_period * i64::from(sample_rate) * 100 + 500_000_000) / 1_000_000_000;
        u32::try_from(frames).unwrap_or(u32::MAX)
    } else {
        max_frames_in_buffer
    }
}

/// `GetStreamLatency` (100 ns units) → frames at the stream rate.
fn latency_frames(hns: i64, sample_rate: u32) -> u64 {
    u64::try_from(hns.max(0))
        .unwrap_or(0)
        .saturating_mul(u64::from(sample_rate))
        / 10_000_000
}

/// Activates an `IAudioClient` for the *default* capture endpoint via
/// `ActivateAudioInterfaceAsync` — the reroutable path cpal uses for
/// `DeviceHandle::DefaultInput`, so a default-device change keeps the
/// stream alive on the new endpoint (the monitor reports the change).
fn activate_default_capture_client() -> DeviceResult<Audio::IAudioClient> {
    // SAFETY: `StringFromIID` allocates a `CoTaskMem` `PWSTR` that
    // `ComString` frees exactly once on scope exit.
    let path = unsafe { Com::StringFromIID(&Audio::DEVINTERFACE_AUDIO_CAPTURE) }
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    let _guard = ComString(path);
    // SAFETY: `path` is a valid `PWSTR` for the duration of the call;
    // the activation is awaited synchronously inside.
    unsafe { activate_audio_interface_sync(path) }
}

/// Synchronous wrapper around `ActivateAudioInterfaceAsync` — ported
/// from cpal's `activate_audio_interface_sync` (the completion
/// handler delivers the activated `IUnknown` over an mpsc channel).
///
/// # Safety
///
/// `device_interface_path` must be a valid `PWSTR` for the duration
/// of the call (the `ComString` guard in the caller covers this).
unsafe fn activate_audio_interface_sync(
    device_interface_path: windows::core::PWSTR,
) -> DeviceResult<Audio::IAudioClient> {
    #[windows::core::implement(Audio::IActivateAudioInterfaceCompletionHandler)]
    struct CompletionHandler(Sender<windows::core::Result<windows::core::IUnknown>>);

    fn retrieve_result(
        operation: &Audio::IActivateAudioInterfaceAsyncOperation,
    ) -> windows::core::Result<windows::core::IUnknown> {
        let mut result = windows::core::HRESULT::default();
        let mut interface: Option<windows::core::IUnknown> = None;
        // SAFETY: both out-params are valid slots for the call.
        unsafe {
            operation.GetActivateResult(&raw mut result, &raw mut interface)?;
        }
        result.ok()?;
        interface.ok_or_else(|| {
            windows::core::Error::new(
                Audio::AUDCLNT_E_DEVICE_INVALIDATED,
                "audio interface not available after activation",
            )
        })
    }

    impl Audio::IActivateAudioInterfaceCompletionHandler_Impl for CompletionHandler_Impl {
        fn ActivateCompleted(
            &self,
            operation: windows::core::Ref<'_, Audio::IActivateAudioInterfaceAsyncOperation>,
        ) -> windows::core::Result<()> {
            let result = operation.ok().and_then(retrieve_result);
            // The receiver may be gone if the caller gave up — the
            // result is simply dropped then.
            let _ = self.0.send(result);
            Ok(())
        }
    }

    let (tx, rx) = channel();
    let handler: Audio::IActivateAudioInterfaceCompletionHandler = CompletionHandler(tx).into();
    // SAFETY: `device_interface_path` is valid per the fn contract;
    // `handler` is a live COM object for the call's duration.
    unsafe {
        Audio::ActivateAudioInterfaceAsync(
            device_interface_path,
            &Audio::IAudioClient::IID,
            None,
            &handler,
        )
    }
    .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    // `handler` stays alive on the stack until `recv` returns, so the
    // sender cannot be dropped before the callback fires.
    let result = rx
        .recv()
        .map_err(|_| DeviceError::Backend("activation_channel_closed"))?;
    result
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))?
        .cast::<Audio::IAudioClient>()
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))
}

/// Activates the `IAudioClient` of a *pinned* endpoint — the cpal
/// `DeviceHandle::Specific` path: resolve the `IMMDevice` behind the
/// enumerated `cpal::Device` and `Activate` it directly (no
/// rerouting, no default-device monitor).
fn activate_specific_capture_client(device: &cpal::Device) -> DeviceResult<Audio::IAudioClient> {
    // On Windows cpal 0.18 enables only the WASAPI host, so
    // `DeviceInner` has exactly this variant — the match stays
    // exhaustive without a wildcard; a future second host variant
    // would fail compilation here rather than silently misroute.
    let imm_device = match device.as_inner() {
        cpal::platform::DeviceInner::Wasapi(d) => d.immdevice(),
    }
    .ok_or(DeviceError::DeviceNotFound)?;
    // SAFETY: `imm_device` is a valid `IMMDevice` COM object owned by
    // cpal's device handle; `Activate` returns a new reference.
    unsafe {
        imm_device
            .Activate::<Audio::IAudioClient>(Com::CLSCTX_ALL, None)
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))
    }
}

/// Default-device change monitor — the cpal `DefaultDeviceMonitor`
/// port, scoped to capture (`eCapture`). Registers an
/// `IMMNotificationClient` whose callbacks only `SetEvent` (or set a
/// fallback flag); the worker thread turns the wake into a
/// [`DeviceError::StreamInvalidated`] error-callback notification —
/// notification callbacks never invoke user code (deadlock rule,
/// cpal parity).
struct DefaultDeviceMonitor {
    enumerator: Audio::IMMDeviceEnumerator,
    /// Held so the registration stays alive for the monitor's
    /// lifetime — unregistering is what releases it.
    client: Audio::IMMNotificationClient,
    /// Auto-reset event waited on by the worker's handle array.
    event: Foundation::HANDLE,
    /// Set by a notification callback when `SetEvent` fails — polled
    /// by the worker as the fallback signal.
    pending_device_changed: Arc<AtomicBool>,
}

// SAFETY: the COM objects are used only for register/unregister (on
// the control thread, COM-initialised) and `SetEvent`/`GetDefaultAudioEndpoint`
// (thread-safe COM calls from notification threads). The `HANDLE` is
// a Windows synchronization primitive valid until unregister+close,
// which happens in `Drop` after the worker has joined.
unsafe impl Send for DefaultDeviceMonitor {}

impl DefaultDeviceMonitor {
    fn new() -> DeviceResult<Self> {
        ensure_com()?;
        // SAFETY: `CoCreateInstance` on the control path with COM
        // initialised; the enumerator is a fresh COM reference.
        let enumerator = unsafe {
            Com::CoCreateInstance::<_, Audio::IMMDeviceEnumerator>(
                &Audio::MMDeviceEnumerator,
                None,
                Com::CLSCTX_ALL,
            )
        }
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
        let event_guard = HandleGuard::create()?;
        let pending_device_changed = Arc::new(AtomicBool::new(false));
        let client: Audio::IMMNotificationClient = DefaultDeviceNotificationImpl {
            flow: Audio::eCapture,
            event: event_guard.get(),
            enumerator: enumerator.clone(),
            pending_device_changed: pending_device_changed.clone(),
        }
        .into();
        // SAFETY: `client` is a valid `IMMNotificationClient` COM
        // object; registration is the only call made with it.
        match unsafe { enumerator.RegisterEndpointNotificationCallback(&client) } {
            Ok(()) => Ok(Self {
                enumerator,
                client,
                event: event_guard.into_inner(),
                pending_device_changed,
            }),
            // Registration failed — `event_guard` closes the event on
            // drop and `client` releases the COM object.
            Err(e) => Err(map_win_error(&e, ErrorCtx::Control)),
        }
    }
}

impl Drop for DefaultDeviceMonitor {
    fn drop(&mut self) {
        // Best-effort COM init — `Drop` can run on any thread.
        let _ = ensure_com();
        // SAFETY: unregister is synchronous — it waits for any
        // in-progress notification callback before returning, so the
        // event handle can be closed immediately afterwards without
        // racing a callback's `SetEvent`. The handle is closed only
        // on successful unregister (cpal parity: on failure the
        // callback may still hold the handle value).
        unsafe {
            if self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.client)
                .is_ok()
            {
                let _ = Foundation::CloseHandle(self.event);
            }
        }
    }
}

#[windows::core::implement(Audio::IMMNotificationClient)]
struct DefaultDeviceNotificationImpl {
    flow: Audio::EDataFlow,
    event: Foundation::HANDLE,
    enumerator: Audio::IMMDeviceEnumerator,
    pending_device_changed: Arc<AtomicBool>,
}

impl DefaultDeviceNotificationImpl {
    /// Whether a default capture endpoint currently exists — used to
    /// skip signalling when `OnDefaultDeviceChanged` will fire anyway
    /// (avoids the double wakeup cpal's monitor also avoids).
    fn current_default_exists(&self) -> bool {
        let _ = ensure_com();
        // SAFETY: COM calls from the notification callback thread —
        // `ensure_com` initialises COM there on demand; `enumerator`
        // is a valid COM object kept alive by the monitor.
        unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(self.flow, Audio::eConsole)
                .is_ok()
        }
    }

    /// Signals the worker — `SetEvent` first, the atomic flag as the
    /// fallback when the handle is already closed.
    fn signal_worker(&self) {
        // SAFETY: `event` is valid for the monitor's lifetime, which
        // outlives every notification callback (unregister is
        // synchronous before the handle closes in `Drop`).
        unsafe {
            if Threading::SetEvent(self.event).is_err() {
                self.pending_device_changed.store(true, Ordering::Relaxed);
            }
        }
    }
}

impl Audio::IMMNotificationClient_Impl for DefaultDeviceNotificationImpl_Impl {
    fn OnDefaultDeviceChanged(
        &self,
        flow: Audio::EDataFlow,
        role: Audio::ERole,
        _pwstrdefaultdeviceid: &windows::core::PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == self.flow && role == Audio::eConsole {
            self.signal_worker();
        }
        Ok(())
    }

    fn OnDeviceStateChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        dwnewstate: Audio::DEVICE_STATE,
    ) -> windows::core::Result<()> {
        // `DEVICE_STATE_UNPLUGGED`: physical jack disconnected — the
        // endpoint persists but produces no audio and
        // `OnDeviceRemoved` does not fire. `NOTPRESENT`: hardware
        // absent. `DISABLED`: user-disabled. Only signal when no
        // replacement default exists (`OnDefaultDeviceChanged` is the
        // sole notifier otherwise — cpal parity).
        let is_unavailable = dwnewstate == Audio::DEVICE_STATE_DISABLED
            || dwnewstate == Audio::DEVICE_STATE_NOTPRESENT
            || dwnewstate == Audio::DEVICE_STATE_UNPLUGGED;
        if is_unavailable && !self.current_default_exists() {
            self.signal_worker();
        }
        Ok(())
    }

    fn OnDeviceAdded(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        if !self.current_default_exists() {
            self.signal_worker();
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Per-packet capture-clock position tracker — verifies that each
/// packet's `device_position` continues the previous packet's
/// `device_position + frames` expectation.
///
/// The first packet after open anchors the epoch unconditionally
/// (`expected == None` — a first-packet `device_position == 0` is a
/// normal value, and the discontinuity *flag* is undefined there per
/// cpal's guard). Every later packet must match `expected` exactly;
/// a mismatch is an unverifiable jump — reported as
/// [`DeviceError::ClockDiscontinuity`] — after which the tracker
/// re-anchors to the observed position so subsequent packets are
/// judged against the new epoch. A stop/start position reset surfaces
/// here honestly rather than being hidden.
#[derive(Default)]
struct PositionTracker {
    /// The `device_position` expected for the next packet's first
    /// frame — `device_position + frames` of the previous packet.
    /// `None` until the first packet anchors the epoch.
    expected: Option<u64>,
}

impl PositionTracker {
    /// Checks `device_position` against the running expectation and
    /// re-anchors to `device_position + frames`. Returns `true` when
    /// the position broke contiguity.
    fn check(&mut self, device_position: u64, frames: u64) -> bool {
        let discontinuity = self
            .expected
            .is_some_and(|expected| expected != device_position);
        self.expected = Some(device_position.saturating_add(frames));
        discontinuity
    }
}

enum Command {
    Play,
    Pause,
    Terminate,
}

/// Everything the worker thread owns — COM interfaces, events,
/// callbacks and preallocated state. Constructed on the control
/// thread (all activation/`Initialize` calls complete there first),
/// then moved wholesale into the worker.
struct RunContext {
    audio_client: Audio::IAudioClient,
    capture_client: Audio::IAudioCaptureClient,
    /// `QueryPerformanceFrequency`, cached at open — the conversion
    /// base for each packet's `perf_counter_instant` delivery stamp.
    qpc_frequency: u64,
    commands: Receiver<Command>,
    /// Wait handles: `[pending, audio, monitor?]` — index 0 wakes for
    /// commands, 1 for capture packets, 2+ for default-device changes.
    handles: Vec<Foundation::HANDLE>,
    /// Fallback flag set by a notification callback when `SetEvent`
    /// fails — polled each loop iteration.
    pending_device_changed: Option<Arc<AtomicBool>>,
    /// Data callback state — user callback + preallocated planar
    /// scratch + clock profile. Nothing here allocates after open.
    state: InputCallbackState,
    error_callback: InputErrorCallback,
    tracker: PositionTracker,
    /// Whether `IAudioClient::Start` has run — avoids redundant
    /// start/stop calls (cpal `playing` parity).
    playing: bool,
    /// Negotiated channel count — sizes the interleaved `f32` slice
    /// `GetBuffer` returns.
    channels: usize,
    /// The worker-owned event handles — declared **last** so the
    /// struct's field drop order releases every COM client (which may
    /// still reference `audio` internally via `SetEventHandle`)
    /// before the handles close. Held only for its `Drop`.
    _events: WorkerEvents,
}

/// The two event handles the worker waits on — `audio` is the
/// `SetEventHandle` target WASAPI signals, `pending` is the
/// command-wake event `push_command` signals. Grouped so their `Drop`
/// runs after all COM fields of [`RunContext`] are released.
struct WorkerEvents {
    audio: Foundation::HANDLE,
    pending: Foundation::HANDLE,
}

impl Drop for WorkerEvents {
    fn drop(&mut self) {
        // SAFETY: both handles were created for this stream and are
        // closed exactly once — by the time this runs the worker has
        // exited and every `RunContext` field declared before `events`
        // (including the COM clients) has been released.
        unsafe {
            let _ = Foundation::CloseHandle(self.audio);
            let _ = Foundation::CloseHandle(self.pending);
        }
    }
}

// SAFETY: the worker thread is the sole owner of every COM interface
// in this context once `open_input_stream` returns — activation and
// `Initialize` complete on the control thread first, and WASAPI
// client objects are free-threaded (cpal moves the same set into its
// `RunContext` under an identical `unsafe impl Send`). `HANDLE`s are
// Windows synchronization primitives safe across threads; all other
// fields are `Send` value types.
unsafe impl Send for RunContext {}

/// The capture worker loop — cpal `run_input` shape: drain commands,
/// honour the device-change flag, wait on the handle set, then drain
/// every pending packet per wake.
fn run_worker(mut ctx: RunContext) {
    loop {
        match process_commands(&mut ctx) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                (ctx.error_callback)(error);
                return;
            }
        }
        if let Some(flag) = &ctx.pending_device_changed
            && flag.swap(false, Ordering::Relaxed)
        {
            // The default capture endpoint changed — the stream was
            // rerouted by WASAPI; the position epoch almost certainly
            // broke too (the tracker reports `ClockDiscontinuity` on
            // the next packet). `StreamInvalidated` matches what cpal
            // reports here (its `DeviceNotAvailable`/`StreamInvalidated`
            // pair both normalize to the same port variant).
            (ctx.error_callback)(DeviceError::StreamInvalidated);
        }
        let handle_idx = match wait_for_handle_signal(&ctx.handles) {
            Ok(idx) => idx,
            Err(error) => {
                (ctx.error_callback)(error);
                return;
            }
        };
        if handle_idx == 0 {
            // Command wake — loop back and drain the queue.
            continue;
        }
        if handle_idx >= 2 {
            // Default-device monitor event.
            (ctx.error_callback)(DeviceError::StreamInvalidated);
            continue;
        }
        if let Err(error) = process_input(&mut ctx) {
            (ctx.error_callback)(error);
            return;
        }
    }
}

/// Drains queued commands — `Start`/`Stop` on the `IAudioClient` for
/// play/pause, `Terminate` ends the worker. Mirrors cpal's
/// `process_commands`: a failed command is reported to the caller,
/// which emits it and ends the loop.
fn process_commands(ctx: &mut RunContext) -> Result<bool, DeviceError> {
    for command in ctx.commands.try_iter() {
        match command {
            Command::Play => {
                if !ctx.playing {
                    // SAFETY: `audio_client` is owned by this worker
                    // thread for the context's lifetime.
                    unsafe {
                        ctx.audio_client
                            .Start()
                            .map_err(|e| map_win_error(&e, ErrorCtx::ErrorCallback))?;
                    }
                    ctx.playing = true;
                }
            }
            Command::Pause => {
                if ctx.playing {
                    // SAFETY: same ownership argument as `Play`.
                    unsafe {
                        ctx.audio_client
                            .Stop()
                            .map_err(|e| map_win_error(&e, ErrorCtx::ErrorCallback))?;
                    }
                    ctx.playing = false;
                }
            }
            Command::Terminate => return Ok(false),
        }
    }
    Ok(true)
}

/// Waits for the first signalled handle — index into `handles`
/// (cpal `wait_for_handle_signal` parity).
fn wait_for_handle_signal(handles: &[Foundation::HANDLE]) -> Result<usize, DeviceError> {
    debug_assert!(handles.len() <= SystemServices::MAXIMUM_WAIT_OBJECTS as usize);
    // SAFETY: every entry is a live event handle owned for the
    // worker's lifetime — `pending`/`audio` by `WorkerEvents`, the
    // monitor event by `DefaultDeviceMonitor`, which outlives the
    // worker's join.
    let result =
        unsafe { Threading::WaitForMultipleObjectsEx(handles, false, Threading::INFINITE, false) };
    if result == Foundation::WAIT_FAILED {
        return Err(DeviceError::StreamInvalidated);
    }
    Ok(usize::try_from(result.0 - Foundation::WAIT_OBJECT_0.0).unwrap_or(usize::MAX))
}

/// One captured packet → [`InputCallbackInfo`] + planar delivery.
///
/// Discontinuity is evaluated *before* delivery (cpal parity): the
/// backend `DATA_DISCONTINUITY` flag — guarded by
/// `device_position != 0` because the flag is undefined on the first
/// packet after `Start` — or a [`PositionTracker`] contiguity break
/// emits [`DeviceError::ClockDiscontinuity`] on the error callback,
/// then the packet is still delivered at its real position so the
/// consumer sees the boundary in-order.
///
/// **RT contract:** RT-suitable — integer math and one `Copy` metadata
/// build; no allocation, no locks.
#[expect(
    clippy::too_many_arguments,
    reason = "the packet's backend-reported fields are passed individually so the seam stays unit-testable without COM objects"
)]
fn handle_packet(
    state: &mut InputCallbackState,
    tracker: &mut PositionTracker,
    error_callback: &mut InputErrorCallback,
    data: &[f32],
    packet_frames: u64,
    device_position: u64,
    qpc_position: u64,
    callback: StreamInstant,
    buffer_flags: u32,
) {
    let flagged = device_position != 0 && buffer_flags & DISCONTINUITY_FLAG != 0;
    let jumped = tracker.check(device_position, packet_frames);
    if flagged || jumped {
        error_callback(DeviceError::ClockDiscontinuity);
    }
    let info = CpalInputCallbackInfo::new(InputStreamTimestamp {
        callback,
        capture: qpc100ns_to_stream_instant(qpc_position),
    });
    state.handle(data, &info, Some(device_position));
}

/// One wake of the capture event: drains every pending packet —
/// `GetNextPacketSize`/`GetBuffer`/deliver/`ReleaseBuffer` per packet,
/// the cpal `process_input` shape with the per-packet device position
/// carried into `InputCallbackInfo::frame_position`.
///
/// **RT contract:** RT-suitable — bounded by the packets WASAPI
/// reports; no allocation, no locks; all state is preallocated.
fn process_input(ctx: &mut RunContext) -> Result<(), DeviceError> {
    let channels = ctx.channels;
    // SAFETY: every COM call below uses interfaces owned by this
    // worker thread; out-params are valid slots for the call's
    // duration; the capture `buffer` slice is consumed before
    // `ReleaseBuffer` (see the slice's own SAFETY comment).
    unsafe {
        loop {
            let mut frames_available = ctx
                .capture_client
                .GetNextPacketSize()
                .map_err(|e| map_win_error(&e, ErrorCtx::ErrorCallback))?;
            if frames_available == 0 {
                return Ok(());
            }
            let mut buffer: *mut u8 = ptr::null_mut();
            let mut flags = mem::MaybeUninit::<u32>::uninit();
            let mut device_position = 0u64;
            let mut qpc_position = 0u64;
            match ctx.capture_client.GetBuffer(
                &raw mut buffer,
                &raw mut frames_available,
                flags.as_mut_ptr(),
                Some(&raw mut device_position),
                Some(&raw mut qpc_position),
            ) {
                // `AUDCLNT_S_BUFFER_EMPTY` is a failure-severity
                // HRESULT the engine returns when the packet drained
                // between `GetNextPacketSize` and `GetBuffer` — retry
                // the packet loop, not an error (cpal parity).
                Err(e) if e.code() == Audio::AUDCLNT_S_BUFFER_EMPTY => continue,
                Err(e) => return Err(map_win_error(&e, ErrorCtx::ErrorCallback)),
                Ok(()) => {}
            }
            let flags = flags.assume_init();
            // The delivery instant on the stream clock — a fresh QPC
            // read, not `IAudioClock::GetPosition`'s position-snapshot
            // instant (0 before the clock first runs, ~1 ms stale
            // against the just-recorded packet — see the module
            // provenance note).
            let callback = perf_counter_instant(ctx.qpc_frequency)?;
            let packet_frames = u64::from(frames_available);
            let sample_count = usize::try_from(frames_available)
                .ok()
                .and_then(|frames| frames.checked_mul(channels))
                .ok_or(DeviceError::Backend("capture_overflow"))?;
            debug_assert!(!buffer.is_null());
            // SAFETY: `GetBuffer` returned `frames_available` frames of
            // the F32 format this stream initialized —
            // `frames_available * channels` contiguous `f32` values,
            // valid and read-only until `ReleaseBuffer`. The slice
            // only feeds `handle_packet` → `read_interleaved`, which
            // copies into the preallocated planar scratch before
            // `ReleaseBuffer` runs. `cast_ptr_alignment` is asserted
            // away: WASAPI capture buffers are frame-aligned and f32
            // is 4-byte aligned (cpal's i24 path makes the same
            // assumption for its wider element type).
            #[expect(
                clippy::cast_ptr_alignment,
                reason = "WASAPI guarantees the capture buffer is aligned to a frame boundary"
            )]
            let data = slice::from_raw_parts(buffer.cast::<f32>(), sample_count);
            handle_packet(
                &mut ctx.state,
                &mut ctx.tracker,
                &mut ctx.error_callback,
                data,
                packet_frames,
                device_position,
                qpc_position,
                callback,
                flags,
            );
            ctx.capture_client
                .ReleaseBuffer(frames_available)
                .map_err(|e| map_win_error(&e, ErrorCtx::ErrorCallback))?;
        }
    }
}

/// An opened WASAPI-native input stream — behind `Box<dyn
/// InputStream>` on the port surface. `Send` so the stream can be
/// controlled from another thread.
pub(crate) struct NativeInputStream {
    commands: Sender<Command>,
    /// Stale copy of the worker's `pending` event handle, used only to
    /// `SetEvent` after queueing a command. The worker's
    /// [`WorkerEvents`] owns and closes the real handle on exit — the
    /// same ownership split cpal's WASAPI `Stream` uses (a dead worker
    /// fails the channel `send` before `SetEvent` is reached).
    pending_event: Foundation::HANDLE,
    thread: Option<JoinHandle<()>>,
    format: AudioFormat,
    /// Stream-clock instant captured at open — the epoch anchor for
    /// the saturating timestamp fallback in `instant_to_media_time`.
    epoch: StreamInstant,
    /// `QueryPerformanceFrequency` cached at open (constant for the
    /// system lifetime).
    qpc_frequency: u64,
    /// Callback period in frames (`GetDevicePeriod`-derived — cpal
    /// `period_frames` parity), reported by `buffer_size()`.
    period_frames: u32,
    /// `GetStreamLatency` converted to frames at the stream rate —
    /// reported by `input_latency()` with `Unknown` coverage (the
    /// port's honest "composition unspecified" label for a
    /// backend-reported count whose components are not enumerated).
    input_latency: StreamLatency,
    /// The host clock profile resolved at enumeration — reported by
    /// `clock_domain()` and stamped on every callback's
    /// `InputCallbackInfo` (WASAPI = `MONOTONIC`/`Carried`).
    clock_domain: ClockDomainId,
    /// Present for default-device streams only — dropped as a field
    /// *after* `Drop` joins the worker, so the notification handle
    /// outlives the wait loop (cpal `_default_device_monitor` parity).
    _monitor: Option<DefaultDeviceMonitor>,
}

// SAFETY: `pending_event` is a Windows event handle used only for
// `SetEvent` (a thread-safe synchronisation primitive), valid while
// the worker lives and harmlessly stale after it exits — the command
// `send` fails first in that case. `DefaultDeviceMonitor` carries its
// own `unsafe impl Send`; every other field is `Send`.
unsafe impl Send for NativeInputStream {}

impl NativeInputStream {
    /// Queues a worker command and wakes it — the cpal
    /// `push_command` shape.
    fn push_command(&self, command: Command) -> DeviceResult<()> {
        self.commands
            .send(command)
            .map_err(|_| DeviceError::StreamInvalidated)?;
        // SAFETY: `pending_event` is a valid event handle while the
        // worker lives (it owns the close). If the worker already
        // exited, `send` above failed first — the residual race (worker
        // exits between send and SetEvent) mirrors cpal's accepted
        // window: `SetEvent` on a closed/reused handle reports an
        // error mapped to `StreamInvalidated`.
        unsafe {
            Threading::SetEvent(self.pending_event).map_err(|_| DeviceError::StreamInvalidated)
        }
    }
}

impl InputStream for NativeInputStream {
    fn start(&self) -> DeviceResult<()> {
        self.push_command(Command::Play)
    }

    fn stop(&self) -> DeviceResult<()> {
        self.push_command(Command::Pause)
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        Ok(BufferSize::Fixed(
            usize::try_from(self.period_frames).unwrap_or(usize::MAX),
        ))
    }

    fn now(&self) -> DeviceResult<MediaTime> {
        let instant = perf_counter_instant(self.qpc_frequency)?;
        instant_to_media_time(instant, self.epoch).ok_or(DeviceError::Backend("timestamp_overflow"))
    }

    fn clock_domain(&self) -> ClockDomainId {
        // The WASAPI host profile resolved at enumeration — QPC is a
        // shared monotonic clock, so `MONOTONIC` stays true on this
        // native path (the callback/capture instants are the same QPC
        // values cpal would have produced).
        self.clock_domain
    }

    /// `IAudioClient::GetStreamLatency` is a real backend-reported
    /// count — reported with `LatencyCoverage::Unknown` (which path
    /// components it includes is unspecified) and `Carried`
    /// provenance, the honest labels for a value the port must not
    /// have consumers add unconditionally.
    fn input_latency(&self) -> DeviceResult<Option<StreamLatency>> {
        Ok(Some(self.input_latency))
    }
}

impl Drop for NativeInputStream {
    fn drop(&mut self) {
        // Terminate is best-effort — the worker may already be gone
        // (the send then fails and there is nothing to wake).
        let _ = self.push_command(Command::Terminate);
        if let Some(handle) = self.thread.take() {
            // Prevent self-join: the port forbids re-entering control
            // APIs from the callback, but guard anyway (cpal parity).
            if handle.thread().id() != std::thread::current().id() {
                let _ = handle.join();
            }
        }
        // `_monitor` drops as a field after this — unregister + event
        // close happen once the worker is joined.
    }
}

/// An `IAudioClient` activated and initialised for shared-mode
/// event-driven F32 capture, plus the stream values derived from it —
/// everything that must complete on the control thread before the
/// capture worker takes ownership of the COM objects.
struct InitializedClient {
    audio_client: Audio::IAudioClient,
    /// Callback period in frames (`GetDevicePeriod`-derived — cpal
    /// `period_frames` parity).
    period_frames: u32,
    /// `IAudioClient::GetStreamLatency` in 100 ns units.
    latency_hns: i64,
}

/// Endpoint activation + `IAudioClient::Initialize` for the negotiated
/// F32 format. `default_device` selects the activation path:
/// `true` (the device came from `default_input_device()`) uses
/// reroutable `ActivateAudioInterfaceAsync`; `false` (an enumerated
/// endpoint) activates the pinned `IMMDevice`.
fn initialize_audio_client(
    device: &cpal::Device,
    default_device: bool,
    stream_config: &cpal::StreamConfig,
) -> DeviceResult<InitializedClient> {
    ensure_com()?;
    let audio_client = if default_device {
        activate_default_capture_client()?
    } else {
        activate_specific_capture_client(device)?
    };
    let channels_u16 = stream_config.channels;
    if channels_u16 == 0 {
        return Err(DeviceError::UnsupportedConfig);
    }
    let sample_rate = stream_config.sample_rate;
    let waveformatextensible = wave_format_extensible_f32(channels_u16, sample_rate)?;

    // SAFETY: `audio_client` is a live COM object; COM is
    // initialised; `waveformatextensible.Format` outlives the call.
    unsafe {
        audio_client
            .Initialize(
                Audio::AUDCLNT_SHAREMODE_SHARED,
                Audio::AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                buffer_duration_100ns(stream_config.buffer_size, sample_rate),
                0,
                &raw const waveformatextensible.Format,
                None,
            )
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    }
    // SAFETY: `audio_client` is live; the out-param is a valid u32.
    let max_frames_in_buffer = unsafe { audio_client.GetBufferSize() }
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    let period_frames = shared_mode_period_frames(&audio_client, sample_rate, max_frames_in_buffer);
    // SAFETY: `audio_client` is live; the out-param is a valid i64.
    let latency_hns = unsafe { audio_client.GetStreamLatency() }
        .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    Ok(InitializedClient {
        audio_client,
        period_frames,
        latency_hns,
    })
}

/// Opens a WASAPI-native input stream — called from
/// [`crate::device::CpalDevice::open_input_stream`] only when the host
/// is `"WASAPI"` (Windows). Negotiation, F32 validation and scratch
/// sizing already happened on the caller side; this function owns
/// endpoint activation, `IAudioClient` initialisation, the capture
/// worker thread and the port-facing stream object.
///
/// `default_device` selects the activation path: `true` (the device
/// came from `default_input_device()`) uses reroutable
/// `ActivateAudioInterfaceAsync` + the default-device monitor;
/// `false` (an enumerated endpoint) activates the pinned `IMMDevice`.
#[expect(
    clippy::too_many_arguments,
    reason = "open-time parameters mirror the cpal build_*_stream_raw signature they replace"
)]
pub(crate) fn open_input_stream(
    device: &cpal::Device,
    default_device: bool,
    stream_config: &cpal::StreamConfig,
    format: AudioFormat,
    scratch: Vec<f32>,
    frames_capacity: usize,
    data_callback: InputDataCallback,
    error_callback: InputErrorCallback,
    clock_domain: ClockDomainId,
    capture_provenance: TimestampProvenance,
) -> DeviceResult<Box<dyn InputStream>> {
    let InitializedClient {
        audio_client,
        period_frames,
        latency_hns,
    } = initialize_audio_client(device, default_device, stream_config)?;

    let channels = usize::from(stream_config.channels);
    let sample_rate = stream_config.sample_rate;

    // Guards keep both event handles owned until they move into
    // `WorkerEvents` — any failure below closes them exactly once.
    let pending_guard = HandleGuard::create()?;
    let audio_guard = create_event().map(HandleGuard)?;
    // SAFETY: `audio_guard.get()` is a live event handle; the client
    // records the handle value for signalling (it does not take
    // ownership — `WorkerEvents` closes it).
    unsafe {
        audio_client
            .SetEventHandle(audio_guard.get())
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))?;
    }
    // SAFETY: `audio_client` is live; `GetService` returns a new COM
    // reference for each requested interface.
    let capture_client = unsafe {
        audio_client
            .GetService::<Audio::IAudioCaptureClient>()
            .map_err(|e| map_win_error(&e, ErrorCtx::Control))?
    };
    // Default-device streams get the change monitor; pinned endpoints
    // do not (cpal `default_device_monitor` parity).
    let monitor = if default_device {
        Some(DefaultDeviceMonitor::new()?)
    } else {
        None
    };

    let qpc_frequency = query_qpc_frequency()?;
    let epoch = perf_counter_instant(qpc_frequency)?;

    let state = InputCallbackState::new(
        data_callback,
        scratch,
        channels,
        frames_capacity,
        sample_rate,
        clock_domain,
        capture_provenance,
    );

    let (tx, rx) = channel();
    let pending_event = pending_guard.into_inner();
    let audio_event = audio_guard.into_inner();
    let mut handles = vec![pending_event, audio_event];
    let pending_device_changed = monitor.as_ref().map(|m| m.pending_device_changed.clone());
    if let Some(monitor) = &monitor {
        handles.push(monitor.event);
    }
    let context = RunContext {
        audio_client,
        capture_client,
        qpc_frequency,
        commands: rx,
        handles,
        pending_device_changed,
        state,
        error_callback,
        tracker: PositionTracker::default(),
        playing: false,
        channels,
        _events: WorkerEvents {
            audio: audio_event,
            pending: pending_event,
        },
    };
    let thread = std::thread::Builder::new()
        .name("ruviform_wasapi_in".to_owned())
        .spawn(move || run_worker(context))
        .map_err(|_| DeviceError::Backend("resource_exhausted"))?;

    Ok(Box::new(NativeInputStream {
        commands: tx,
        pending_event,
        thread: Some(thread),
        format,
        epoch,
        qpc_frequency,
        period_frames,
        input_latency: StreamLatency {
            frames: latency_frames(latency_hns, sample_rate),
            coverage: LatencyCoverage::Unknown,
            provenance: TimestampProvenance::Carried,
        },
        clock_domain,
        _monitor: monitor,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::device::{InputCallbackInfo, InputTimestamp};
    use ruviform_media_core::MediaTimestamp;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicU64;

    fn test_error_callback() -> InputErrorCallback {
        Box::new(|_e: DeviceError| {})
    }

    #[test]
    fn position_tracker_first_packet_anchors_unconditionally() {
        let mut tracker = PositionTracker::default();
        // First packet — even at position 0 — anchors without a
        // discontinuity report.
        assert!(!tracker.check(0, 480));
        assert_eq!(tracker.expected, Some(480));
    }

    #[test]
    fn position_tracker_contiguous_packets_do_not_flag() {
        let mut tracker = PositionTracker::default();
        assert!(!tracker.check(0, 480));
        assert!(!tracker.check(480, 480));
        assert!(!tracker.check(960, 240));
        assert_eq!(tracker.expected, Some(1200));
    }

    #[test]
    fn position_tracker_detects_jump_and_resyncs() {
        let mut tracker = PositionTracker::default();
        assert!(!tracker.check(0, 480));
        // 480 frames lost — position jumped ahead.
        assert!(tracker.check(960, 480));
        // Re-anchored: the following contiguous packet is clean.
        assert!(!tracker.check(1440, 480));
    }

    #[test]
    fn position_tracker_detects_position_reset() {
        let mut tracker = PositionTracker::default();
        assert!(!tracker.check(100_000, 480));
        // A stop/start that resets the device epoch reports as a
        // discontinuity — the honest epoch boundary.
        assert!(tracker.check(0, 480));
        assert!(!tracker.check(480, 480));
    }

    #[test]
    fn position_tracker_saturates_at_u64_max() {
        let mut tracker = PositionTracker::default();
        assert!(!tracker.check(u64::MAX - 10, 20));
        // expected saturates at u64::MAX; a real device never reaches
        // this, but the arithmetic must not wrap.
        assert!(tracker.check(0, 1));
    }

    #[test]
    fn qpc100ns_to_stream_instant_converts_units() {
        // 10^7 units of 100 ns = 1 s.
        let instant = qpc100ns_to_stream_instant(10_000_000);
        assert_eq!(instant.as_nanos(), 1_000_000_000);
        // Zero is the "not reported" sentinel.
        assert_eq!(qpc100ns_to_stream_instant(0), StreamInstant::ZERO);
    }

    #[test]
    fn wave_format_extensible_f32_is_ieee_float() {
        let wfx = wave_format_extensible_f32(2, 48_000).unwrap();
        // `WAVEFORMATEXTENSIBLE` is `#[repr(packed)]` — copy fields to
        // locals before asserting so no unaligned reference is formed
        // (E0793), and the union field read stays inside `unsafe`.
        let format_tag = wfx.Format.wFormatTag;
        let channels = wfx.Format.nChannels;
        let sample_rate = wfx.Format.nSamplesPerSec;
        let avg_bytes = wfx.Format.nAvgBytesPerSec;
        let block_align = wfx.Format.nBlockAlign;
        let bits = wfx.Format.wBitsPerSample;
        // SAFETY: `Samples` was initialised as `wValidBitsPerSample`
        // by `wave_format_extensible_f32` — reading that same union
        // field observes the initialised arm.
        let valid_bits = unsafe { wfx.Samples.wValidBitsPerSample };
        let channel_mask = wfx.dwChannelMask;
        let sub_format = wfx.SubFormat;
        assert_eq!(
            format_tag,
            u16::try_from(KernelStreaming::WAVE_FORMAT_EXTENSIBLE).unwrap()
        );
        assert_eq!(channels, 2);
        assert_eq!(sample_rate, 48_000);
        assert_eq!(avg_bytes, 48_000 * 2 * 4);
        assert_eq!(block_align, 8);
        assert_eq!(bits, 32);
        assert_eq!(valid_bits, 32);
        assert_eq!(channel_mask, KernelStreaming::KSAUDIO_SPEAKER_DIRECTOUT);
        assert_eq!(sub_format, Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT);
    }

    #[test]
    fn buffer_duration_100ns_matches_cpal_formula() {
        // Fixed(480) at 48 kHz ≈ 10 ms → 100_000 units of 100 ns.
        let duration = buffer_duration_100ns(cpal::BufferSize::Fixed(480), 48_000);
        assert_eq!(duration, 100_000);
        assert_eq!(buffer_duration_100ns(cpal::BufferSize::Default, 48_000), 0);
    }

    #[test]
    fn latency_frames_converts_hns() {
        // 10^7 hns = 1 s → 48_000 frames at 48 kHz.
        assert_eq!(latency_frames(10_000_000, 48_000), 48_000);
        assert_eq!(latency_frames(-5, 48_000), 0);
    }

    #[test]
    fn map_win_error_maps_audclnt_codes_per_context() {
        fn err(hr: windows::core::HRESULT) -> windows::core::Error {
            hr.ok().unwrap_err()
        }
        // Device-loss reads differently per path.
        assert_eq!(
            map_win_error(&err(Audio::AUDCLNT_E_DEVICE_INVALIDATED), ErrorCtx::Control),
            DeviceError::DeviceNotFound,
        );
        assert_eq!(
            map_win_error(
                &err(Audio::AUDCLNT_E_DEVICE_INVALIDATED),
                ErrorCtx::ErrorCallback
            ),
            DeviceError::StreamInvalidated,
        );
        assert_eq!(
            map_win_error(&err(Audio::AUDCLNT_E_DEVICE_IN_USE), ErrorCtx::Control),
            DeviceError::DeviceBusy,
        );
        assert_eq!(
            map_win_error(&err(Audio::AUDCLNT_E_UNSUPPORTED_FORMAT), ErrorCtx::Control),
            DeviceError::UnsupportedConfig,
        );
        assert_eq!(
            map_win_error(
                &err(Audio::AUDCLNT_E_RESOURCES_INVALIDATED),
                ErrorCtx::ErrorCallback
            ),
            DeviceError::StreamInvalidated,
        );
        assert_eq!(
            map_win_error(
                &err(Audio::AUDCLNT_E_SERVICE_NOT_RUNNING),
                ErrorCtx::Control
            ),
            DeviceError::Backend("host_unavailable"),
        );
        // `E_ACCESSDENIED` is the OS privacy denial — it classifies
        // the same as the cpal path's `ErrorKind::PermissionDenied`
        // on both contexts (`error.rs::map_error` parity).
        for ctx in [ErrorCtx::Control, ErrorCtx::ErrorCallback] {
            assert_eq!(
                map_win_error(&err(Foundation::E_ACCESSDENIED), ctx),
                DeviceError::Backend("permission_denied"),
            );
        }
        // Unmapped HRESULTs land in the backend bucket.
        assert_eq!(
            map_win_error(&err(windows::core::HRESULT(-1)), ErrorCtx::Control),
            DeviceError::Backend("backend"),
        );
    }

    /// The packet→`InputCallbackInfo` seam: `frame_position` carries
    /// the per-packet `GetBuffer` device position verbatim, capture
    /// carries the `qpc_position` instant with `Carried` provenance,
    /// and the domain is the stream profile.
    #[test]
    fn handle_packet_supplies_frame_position_and_carried_capture() {
        let infos = Arc::new(Mutex::new(Vec::new()));
        let infos_cb = infos.clone();
        let mut state = InputCallbackState::new(
            Box::new(
                move |view: ruviform_audio_core::AudioBufferView<'_, f32>,
                      info: InputCallbackInfo| {
                    infos_cb.lock().unwrap().push((view.frames(), info));
                },
            ),
            vec![0.0; 32],
            2,
            8,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Carried,
        );
        let mut tracker = PositionTracker::default();
        let mut errors = test_error_callback();
        let data = [0.25_f32; 16]; // 8 frames × 2ch — fits scratch (8×2)
        let callback = StreamInstant::new(10, 0);

        handle_packet(
            &mut state,
            &mut tracker,
            &mut errors,
            &data,
            8,
            4096,
            10_000_000, // 1 s in 100 ns units
            callback,
            0,
        );

        let captured = infos.lock().unwrap();
        assert_eq!(captured.len(), 1);
        let (frames, info) = captured[0];
        assert_eq!(frames, 8);
        assert_eq!(info.frame_position, Some(4096));
        assert_eq!(info.clock_domain, ClockDomainId::MONOTONIC);
        let capture = info.timestamp.capture.expect("qpc_position reported");
        assert_eq!(capture.time.value(), 1_000_000_000);
        assert_eq!(capture.provenance, TimestampProvenance::Carried);
        // callback instant maps verbatim through the epoch path.
        assert_eq!(info.timestamp.callback.value(), 10_000_000_000);
    }

    /// A packet larger than the scratch capacity is chunked by
    /// `read_interleaved`/`chunk_input_timestamp_info` — every chunk
    /// callback must carry the position advanced by its frame offset,
    /// so consecutive callbacks still satisfy
    /// `pos_{n+1} == pos_n + frames_n` (the real-input lane's gap
    /// invariant).
    #[test]
    fn handle_packet_chunked_delivery_advances_frame_position() {
        let infos = Arc::new(Mutex::new(Vec::new()));
        let infos_cb = infos.clone();
        let mut state = InputCallbackState::new(
            Box::new(
                move |view: ruviform_audio_core::AudioBufferView<'_, f32>,
                      info: InputCallbackInfo| {
                    infos_cb
                        .lock()
                        .unwrap()
                        .push((view.frames(), info.frame_position));
                },
            ),
            vec![0.0; 8], // capacity 4 frames × 2ch
            2,
            4,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Carried,
        );
        let mut tracker = PositionTracker::default();
        let mut errors = test_error_callback();
        // One 10-frame packet at device position 100 → chunks of 4+4+2.
        let data = [0.5_f32; 20];
        handle_packet(
            &mut state,
            &mut tracker,
            &mut errors,
            &data,
            10,
            100,
            10_000_000,
            StreamInstant::new(1, 0),
            0,
        );

        let captured = infos.lock().unwrap();
        assert_eq!(
            captured.as_slice(),
            &[(4, Some(100)), (4, Some(104)), (2, Some(108))]
        );
    }

    /// Discontinuity reporting: the backend flag (guarded) or a
    /// tracker jump emits `ClockDiscontinuity` *before* the packet is
    /// still delivered at its real position.
    #[test]
    fn handle_packet_reports_discontinuity_and_still_delivers() {
        let infos = Arc::new(Mutex::new(Vec::new()));
        let infos_cb = infos.clone();
        let mut state = InputCallbackState::new(
            Box::new(
                move |_view: ruviform_audio_core::AudioBufferView<'_, f32>,
                      info: InputCallbackInfo| {
                    infos_cb.lock().unwrap().push(info);
                },
            ),
            vec![0.0; 32],
            2,
            8,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Carried,
        );
        let mut tracker = PositionTracker::default();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_cb = errors.clone();
        let mut error_callback: InputErrorCallback =
            Box::new(move |e: DeviceError| errors_cb.lock().unwrap().push(e));
        let data = [0.0_f32; 16];
        let callback = StreamInstant::new(1, 0);

        // Packet 1 anchors at position 0 (first packet — flag guard
        // suppresses even a set flag).
        handle_packet(
            &mut state,
            &mut tracker,
            &mut error_callback,
            &data,
            8,
            0,
            0,
            callback,
            DISCONTINUITY_FLAG,
        );
        // Packet 2 jumps: expected 8, got 64 — tracker discontinuity.
        handle_packet(
            &mut state,
            &mut tracker,
            &mut error_callback,
            &data,
            8,
            64,
            0,
            callback,
            0,
        );

        let reported = errors.lock().unwrap();
        assert_eq!(reported.as_slice(), &[DeviceError::ClockDiscontinuity]);
        // Both packets were still delivered with their real positions.
        let captured = infos.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].frame_position, Some(0));
        assert_eq!(captured[1].frame_position, Some(64));
    }

    /// `qpc_position == 0` surfaces as `capture: None` — the backend's
    /// "not reported" sentinel, never a clock-origin timestamp.
    #[test]
    fn handle_packet_zero_qpc_is_unreported_capture() {
        let infos = Arc::new(Mutex::new(Vec::new()));
        let infos_cb = infos.clone();
        let mut state = InputCallbackState::new(
            Box::new(
                move |_view: ruviform_audio_core::AudioBufferView<'_, f32>,
                      info: InputCallbackInfo| {
                    infos_cb.lock().unwrap().push(info);
                },
            ),
            vec![0.0; 32],
            2,
            8,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Carried,
        );
        let mut tracker = PositionTracker::default();
        let mut errors = test_error_callback();
        let data = [0.0_f32; 16];
        handle_packet(
            &mut state,
            &mut tracker,
            &mut errors,
            &data,
            8,
            0,
            0,
            StreamInstant::new(1, 0),
            0,
        );
        let captured = infos.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert!(captured[0].timestamp.capture.is_none());
        assert_eq!(captured[0].frame_position, Some(0));
    }

    /// RT-lane check: the whole packet→info→`read_interleaved` path —
    /// tracker check, timestamp build, `InputCallbackInfo` construction
    /// and planar delivery — must not allocate. Mirrors the `rt_`
    /// prefix convention selected by `mise run test-rt`; this module
    /// only compiles on Windows so the lane evidence is a local
    /// Windows `cargo test` run.
    #[test]
    fn rt_wasapi_packet_path_allocates_nothing() {
        let frames_seen = Arc::new(AtomicU64::new(0));
        let frames_cb = frames_seen.clone();
        let mut state = InputCallbackState::new(
            Box::new(
                move |view: ruviform_audio_core::AudioBufferView<'_, f32>,
                      _info: InputCallbackInfo| {
                    frames_cb.fetch_add(view.frames() as u64, Ordering::Relaxed);
                },
            ),
            vec![0.0; 64],
            2,
            16,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Carried,
        );
        let mut tracker = PositionTracker::default();
        let mut errors = test_error_callback();
        let data = vec![0.1_f32; 32]; // 16 frames × 2ch
        let callback = StreamInstant::new(42, 0);

        assert_no_alloc::assert_no_alloc(|| {
            handle_packet(
                &mut state,
                &mut tracker,
                &mut errors,
                &data,
                16,
                1024,
                10_000_000,
                callback,
                0,
            );
        });
        assert_eq!(frames_seen.load(Ordering::Relaxed), 16);
        assert_eq!(tracker.expected, Some(1040));
    }

    /// Sanity: `InputTimestamp`/`MediaTimestamp` imports used above
    /// stay referenced even if assertions evolve.
    #[test]
    fn timestamp_types_align_with_port_contract() {
        let _ = InputTimestamp {
            callback: MediaTime::new(0, ruviform_audio_core::device::DEVICE_CLOCK_TIMEBASE),
            capture: Some(MediaTimestamp {
                time: MediaTime::new(0, ruviform_audio_core::device::DEVICE_CLOCK_TIMEBASE),
                provenance: TimestampProvenance::Carried,
            }),
        };
    }
}
