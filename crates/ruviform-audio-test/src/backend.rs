//! Scripted port-trait backend (Issue #628, 2nd slice).
//!
//! A deterministic, test-only implementation of the production device
//! boundary ports — [`DeviceEnumerator`], [`Device`],
//! [`OutputStream`](device::OutputStream) and
//! [`InputStream`](device::InputStream) from
//! `ruviform_audio_core::device` — driven entirely by a test-side
//! script and manual pump. No OS API, no threads, no real hardware:
//! every callback fires only when the test driver calls
//! [`ScriptedDeviceHandle::pump_output_next`] /
//! [`ScriptedDeviceHandle::pump_input_next`].
//!
//! # Honest limits
//!
//! - **OS-L1 evidence only.** Passing tests here prove the *contract*
//!   is implementable deterministically; they are not evidence about
//!   any real OS audio path (WASAPI/WinMM/…).
//! - **Callback firing is manual.** `start()` only marks the stream
//!   started — delivery happens on `pump_*_next`, on the caller's
//!   thread. The pump path is a non-RT test-driver path; the
//!   allocation-free guarantee proven by `rt_` tests covers the
//!   callback firing section (view construction, timestamp build,
//!   callback invocation), not RT-thread scheduling.
//! - **No mid-stream format change.** The port contract fixes
//!   `format()` at open; a rate/channel change is expressed as
//!   [`DeviceError::ClockDiscontinuity`] + stream death + re-open with
//!   a new config.
//! - **Permission denial** maps honestly onto
//!   [`DeviceError::Backend`] `"permission_denied"` or
//!   [`DeviceError::DeviceBusy`] — the port has no dedicated variant.
//! - **Queue-full** is scripted as [`DeviceError::Backend`]
//!   `"queue_full"` / [`DeviceError::Xrun`] on the audio side, and via
//!   `MockPort::inject_dropped`/`inject_unsupported` on the MIDI side.
//!
//! See `docs/designs/issue-628-scripted-backend.md` for the full
//! design and `docs/test-specs/issue-628-scripted-backend.md` for the
//! verified behaviors.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ruviform_audio_core::device::{
    self, BufferSize, Device, DeviceEnumerator, DeviceError, DeviceId, DeviceResult,
    InputDataCallback, InputErrorCallback, InputStreamConfig, OutputDataCallback,
    OutputErrorCallback, OutputStreamConfig, StreamLatency, SupportedBufferSize,
    SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{AudioFormat, SampleFormat};
use ruviform_media_core::{ClockDomainId, Error, Result};

use crate::clock::VirtualClock;
use crate::pcm::{PcmBlock, PcmScenario};

mod midi;
mod stream;

pub use midi::MidiFeed;

/// Per-open-attempt errors for one stream direction.
///
/// `persistent` refuses every open until cleared (e.g. a permission
/// denial that stays in force); `once` is consumed one error per open
/// attempt, modelling transient failures followed by success
/// (reconnect scenarios).
#[derive(Clone, Debug, Default)]
pub struct OpenFaults {
    /// Returned by every open while `Some`.
    pub persistent: Option<DeviceError>,
    /// Consumed in FIFO order — one per open attempt.
    pub once: VecDeque<DeviceError>,
}

/// A scripted fault injected at a specific callback index.
///
/// Matching is by `at_callback == callback_index` (0-based firing
/// count) at pump time; faults whose index has already passed never
/// fire. Multiple faults may share an index — all apply in script
/// order. Injected mid-stream via
/// [`ScriptedDeviceHandle::inject_output_fault`] /
/// [`ScriptedDeviceHandle::inject_input_fault`]; pre-scripted via
/// [`StreamScript::faults`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StreamFault {
    /// Fire `error` on the direction's error callback *after* the data
    /// callback of that firing (e.g. `Xrun`, `ClockDiscontinuity`,
    /// `Backend("queue_full")`).
    Error {
        /// 0-based callback index at which to fire.
        at_callback: u64,
        /// The error delivered.
        error: DeviceError,
    },
    /// Set the stream's frame position to `position` starting at that
    /// callback — a persistent discontinuity (the data window and the
    /// reported `frame_position` both follow). Input streams only;
    /// ignored on output. Pair with
    /// [`StreamFault::Error`] `ClockDiscontinuity` to model the
    /// contract's discontinuity notification.
    FramePosition {
        /// 0-based callback index at which the jump applies.
        at_callback: u64,
        /// New absolute frame position.
        position: u64,
    },
    /// Report `capture: None` for that callback instead of the
    /// scripted capture instant. Input streams only; ignored on
    /// output.
    CaptureNone {
        /// 0-based callback index at which capture is suppressed.
        at_callback: u64,
    },
    /// Deliver `frames` frames on that callback instead of the
    /// negotiated/cycled count — models a variable-frame delivery
    /// (`BufferSize::Default` semantics). Must be `> 0`.
    Frames {
        /// 0-based callback index at which the override applies.
        at_callback: u64,
        /// Frame count for that callback.
        frames: u32,
    },
    /// Invalidate the stream at that callback: `StreamInvalidated` is
    /// delivered on the error callback and the stream stops delivering
    /// — subsequent pumps fail and the device may re-open the
    /// direction after the dead stream is dropped (reconnect).
    Invalidate {
        /// 0-based callback index at which invalidation happens.
        at_callback: u64,
    },
}

/// Which stream direction a [`PumpReport`] / fault injection targets.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StreamDirection {
    /// An [`device::OutputStream`].
    Output,
    /// An [`device::InputStream`].
    Input,
}

/// What one manual pump firing delivered — the observed side of
/// Expected-vs-Observed fault tests.
///
/// `Copy` value type, allocation-free to produce. Direction-specific
/// fields are `None`/`Some` per [`Self::direction`]. Actual error
/// values are observed through the test-registered error callback —
/// the backend keeps no error log.
#[derive(Copy, Clone, Debug)]
pub struct PumpReport {
    /// Which direction fired.
    pub direction: StreamDirection,
    /// 0-based callback index of this firing.
    pub callback_index: u64,
    /// Frames delivered in this callback (after `Frames` faults).
    pub frames: u32,
    /// Stream frame position of frame 0 used for this window
    /// (post-`FramePosition` faults).
    pub position: u64,
    /// The `frame_position` reported to the consumer — input only.
    pub reported_frame_position: Option<u64>,
    /// The clock domain reported with this callback.
    pub clock_domain: ClockDomainId,
    /// `callback` instant in `DEVICE_CLOCK_TIMEBASE` nanoseconds.
    pub callback_nanos: i64,
    /// `capture` instant (callback − input latency) — input only,
    /// `None` when capture reporting is off or suppressed.
    pub capture_nanos: Option<i64>,
    /// `playback` instant (callback + output latency) — output only.
    pub playback_nanos: Option<i64>,
    /// Input frames in this window not covered by any scenario
    /// segment (delivered as silence; `Xrun` was reported).
    pub gap_frames: u64,
    /// Errors fired on the error callback during this pump.
    pub errors_fired: u32,
    /// The stream was invalidated by this pump.
    pub invalidated: bool,
}

/// Per-direction stream behavior shared by output and input specs.
#[derive(Clone, Debug)]
pub struct StreamScript {
    /// Frames per callback when `config.buffer_size == Default` (or as
    /// the base when `frames_cycle` is empty). Must be `> 0`.
    pub default_frames: u32,
    /// Optional per-callback frame-count cycle (wrapping). Empty =
    /// fixed `default_frames`/`Fixed(n)` count. Each value `> 0`.
    /// Non-empty makes `buffer_size()` report [`BufferSize::Default`]
    /// (backend-chosen, possibly varying per callback).
    pub frames_cycle: Vec<u32>,
    /// The `*_latency()` report. Also derives the `playback`
    /// (output = callback + latency) and `capture` (input = callback −
    /// latency) timestamp offsets at the stream's sample rate.
    pub latency: Option<StreamLatency>,
    /// Clock domain reported by `clock_domain()` and every callback's
    /// `clock_domain` field. `None` → `ClockDomainId::new(clock.epoch())`
    /// (devices sharing a [`VirtualClock`] then share a domain —
    /// `same_clock_domain` holds). Set explicitly to model distinct or
    /// well-known domains.
    pub clock_domain: Option<ClockDomainId>,
    /// Pre-scripted faults (see [`StreamFault`]).
    pub faults: Vec<StreamFault>,
    /// Errors returned by `open_*_stream` before a stream exists.
    pub open_faults: OpenFaults,
    /// Errors consumed one-per-call by `start()`.
    pub start_faults: VecDeque<DeviceError>,
    /// Errors consumed one-per-call by `stop()`.
    pub stop_faults: VecDeque<DeviceError>,
}

impl StreamScript {
    /// A script with `default_frames` per callback and no faults.
    #[must_use]
    pub fn new(default_frames: u32) -> Self {
        Self {
            default_frames,
            frames_cycle: Vec::new(),
            latency: None,
            clock_domain: None,
            faults: Vec::new(),
            open_faults: OpenFaults::default(),
            start_faults: VecDeque::new(),
            stop_faults: VecDeque::new(),
        }
    }
}

/// Output capability of a scripted device: format space, default
/// format, stream behavior, and whether delivered buffers are
/// captured for verification.
#[derive(Clone, Debug)]
pub struct OutputSpec {
    /// `default_output_format()` report — must be `SampleFormat::F32`
    /// (the port's callback views carry `f32` only).
    pub default_format: AudioFormat,
    /// `supported_output_formats()` report — must be non-empty and
    /// F32-only; open configs are validated against it.
    pub supported_formats: Vec<SupportedOutputFormat>,
    /// Stream behavior (frames/latency/domain/faults).
    pub stream: StreamScript,
    /// When `true` (default-friendly for tests), each fired callback's
    /// post-callback scratch is captured as a [`PcmBlock`] retrievable
    /// via [`ScriptedDeviceHandle::captured_output`]. Set `false` for
    /// allocation-free pump measurement (`rt_` tests).
    pub record_output: bool,
}

/// Input capability of a scripted device: format space, the PCM
/// scenario played into `InputDataCallback`, stream behavior, and
/// capture-timestamp reporting.
#[derive(Clone, Debug)]
pub struct InputSpec {
    /// `default_input_format()` report — must be `SampleFormat::F32`.
    pub default_format: AudioFormat,
    /// `supported_input_formats()` report — must be non-empty and
    /// F32-only; open configs are validated against it.
    pub supported_formats: Vec<SupportedInputFormat>,
    /// The capture source played into callbacks. Every segment's
    /// fixture must match the opened stream's sample rate and channel
    /// count (validated at open → `UnsupportedConfig` on mismatch);
    /// frames outside all segments are delivered as silence with an
    /// `Xrun` report.
    pub pcm: PcmScenario,
    /// Stream behavior (frames/latency/domain/faults).
    pub stream: StreamScript,
    /// When `true`, callbacks report `capture = Some(callback −
    /// latency)` with `TimestampProvenance::Derived`; when `false`,
    /// `capture: None` always (a backend that cannot report the ADC
    /// instant — consumers must not conflate `callback` for it).
    pub capture_reporting: bool,
}

/// The scripted description of one device: identity, optional
/// per-device clock override, and directional capabilities.
///
/// `output: None` / `input: None` model a device without that
/// capability — the direction reports `Unsupported` on open and an
/// empty supported-format list, honestly.
#[derive(Clone, Debug)]
pub struct DeviceSpec {
    /// `name()` report.
    pub name: String,
    /// `id()` report — `None` → `Ok(None)` (backend reports no stable
    /// id).
    pub id: Option<String>,
    /// Clock override for this device's streams — `None` uses the
    /// [`ScriptedBackend`]'s clock. Two devices with different clocks
    /// model distinct clock domains for `same_clock_domain` tests.
    pub clock: Option<Arc<VirtualClock>>,
    /// Output capability, if any.
    pub output: Option<OutputSpec>,
    /// Input capability, if any.
    pub input: Option<InputSpec>,
}

/// State shared between the enumerated device handle and its open
/// streams.
pub(crate) struct DeviceShared {
    /// The (possibly mutated) spec — faults injected post-registration
    /// land here and are cloned into streams at open.
    pub spec: DeviceSpec,
    /// The backend-level clock — used by streams whose spec has no
    /// per-device override.
    pub backend_clock: Arc<VirtualClock>,
    /// `false` while the device is vanished (hot-unplug model).
    pub present: Arc<AtomicBool>,
    /// The open output stream slot, if any (dead slots are replaced).
    pub open_output: Option<Arc<stream::OutputSlot>>,
    /// The open input stream slot, if any.
    pub open_input: Option<Arc<stream::InputSlot>>,
}

/// A scripted device: both the test-side handle (vanish/reappear,
/// fault injection, manual pump, capture inspection) and the
/// [`Device`] trait implementation handed to consumers as
/// `Box<dyn Device>`.
///
/// `Clone` shares state — clones behave as the same physical device.
#[derive(Clone)]
pub struct ScriptedDeviceHandle {
    shared: Arc<Mutex<DeviceShared>>,
}

impl ScriptedDeviceHandle {
    fn present_is(&self, present: bool) {
        lock(&self.shared).present.store(present, Ordering::Release);
    }

    /// Marks the device vanished: enumeration excludes it, `open_*`
    /// fails `DeviceNotFound`, `name()`/`id()`/describe fail, and open
    /// streams invalidate on their next pump. [`Self::reappear`]
    /// restores it (reconnect scenario).
    pub fn vanish(&self) {
        self.present_is(false);
    }

    /// Marks a vanished device present again.
    pub fn reappear(&self) {
        self.present_is(true);
    }

    /// Whether the device is currently present.
    #[must_use]
    pub fn is_present(&self) -> bool {
        lock(&self.shared).present.load(Ordering::Acquire)
    }

    /// Queue a one-shot open error for the next `open_output_stream`
    /// (consumed FIFO with spec-scripted `once` faults).
    pub fn fail_next_open_output(&self, error: DeviceError) {
        let mut d = lock(&self.shared);
        if let Some(output) = d.spec.output.as_mut() {
            output.stream.open_faults.once.push_back(error);
        }
    }

    /// Queue a one-shot open error for the next `open_input_stream`.
    pub fn fail_next_open_input(&self, error: DeviceError) {
        let mut d = lock(&self.shared);
        if let Some(input) = d.spec.input.as_mut() {
            input.stream.open_faults.once.push_back(error);
        }
    }

    /// Set (or clear with `None`) the persistent open error for output
    /// streams — every open fails while `Some` (e.g.
    /// `Backend("permission_denied")`).
    pub fn set_open_output_error(&self, error: Option<DeviceError>) {
        let mut d = lock(&self.shared);
        if let Some(output) = d.spec.output.as_mut() {
            output.stream.open_faults.persistent = error;
        }
    }

    /// Set (or clear) the persistent open error for input streams.
    pub fn set_open_input_error(&self, error: Option<DeviceError>) {
        let mut d = lock(&self.shared);
        if let Some(input) = d.spec.input.as_mut() {
            input.stream.open_faults.persistent = error;
        }
    }

    /// Inject a fault into the output direction — recorded in the spec
    /// (applies to future opens) and, when a stream is currently open,
    /// into its live fault list.
    pub fn inject_output_fault(&self, fault: StreamFault) {
        let mut d = lock(&self.shared);
        if let Some(output) = d.spec.output.as_mut() {
            output.stream.faults.push(fault);
        }
        if let Some(slot) = d.open_output.as_ref()
            && !slot.dead.load(Ordering::Acquire)
        {
            lock(&slot.core).faults.push(fault);
        }
    }

    /// Inject a fault into the input direction (see
    /// [`Self::inject_output_fault`]).
    pub fn inject_input_fault(&self, fault: StreamFault) {
        let mut d = lock(&self.shared);
        if let Some(input) = d.spec.input.as_mut() {
            input.stream.faults.push(fault);
        }
        if let Some(slot) = d.open_input.as_ref()
            && !slot.dead.load(Ordering::Acquire)
        {
            lock(&slot.core).faults.push(fault);
        }
    }

    /// Whether an output stream is currently open and alive.
    #[must_use]
    pub fn is_output_open(&self) -> bool {
        lock(&self.shared)
            .open_output
            .as_ref()
            .is_some_and(|s| !s.dead.load(Ordering::Acquire))
    }

    /// Whether an input stream is currently open and alive.
    #[must_use]
    pub fn is_input_open(&self) -> bool {
        lock(&self.shared)
            .open_input
            .as_ref()
            .is_some_and(|s| !s.dead.load(Ordering::Acquire))
    }

    /// Fire one output callback on the open output stream — the
    /// deterministic replacement for a backend's RT thread. Returns the
    /// [`PumpReport`] for Expected-vs-Observed assertions.
    ///
    /// Fails with [`DeviceError::Backend`] `"no_open_stream"` when no
    /// output stream is open, [`DeviceError::StreamInvalidated`] after
    /// invalidation, and the stream's scripted start/vanish errors.
    pub fn pump_output_next(&self) -> DeviceResult<PumpReport> {
        let slot = lock(&self.shared).open_output.clone();
        match slot {
            Some(slot) => stream::pump_output(&slot),
            None => Err(DeviceError::Backend("no_open_stream")),
        }
    }

    /// Fire one input callback — see [`Self::pump_output_next`].
    pub fn pump_input_next(&self) -> DeviceResult<PumpReport> {
        let slot = lock(&self.shared).open_input.clone();
        match slot {
            Some(slot) => stream::pump_input(&slot),
            None => Err(DeviceError::Backend("no_open_stream")),
        }
    }

    /// Captured output blocks (`record_output` streams only), oldest
    /// first. Empty when no output stream is open or recording is off.
    #[must_use]
    pub fn captured_output(&self) -> Vec<PcmBlock> {
        let d = lock(&self.shared);
        d.open_output
            .as_ref()
            .filter(|s| !s.dead.load(Ordering::Acquire))
            .map(|s| lock(&s.core).captured.clone())
            .unwrap_or_default()
    }

    /// Drop all captured output blocks.
    pub fn clear_output_capture(&self) {
        let d = lock(&self.shared);
        if let Some(slot) = d.open_output.as_ref() {
            lock(&slot.core).captured.clear();
        }
    }
}

/// A scripted enumerator: a fixed device set plus enumerate-level
/// faults and default-device selection, all driven by the test.
pub struct ScriptedBackend {
    clock: Arc<VirtualClock>,
    devices: Vec<ScriptedDeviceHandle>,
    enumerate_error: Option<DeviceError>,
    default_output: Option<usize>,
    default_input: Option<usize>,
}

impl ScriptedBackend {
    /// Creates an enumerator whose devices default to `clock` for
    /// timestamps (per-device override via [`DeviceSpec::clock`]).
    #[must_use]
    pub fn new(clock: Arc<VirtualClock>) -> Self {
        Self {
            clock,
            devices: Vec::new(),
            enumerate_error: None,
            default_output: None,
            default_input: None,
        }
    }

    /// Registers `spec` as a device and returns its handle.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the spec is self-inconsistent:
    /// empty name, non-F32 default/supported formats, empty supported
    /// space for a declared direction, malformed supported ranges
    /// (`channels == 0`, `min > max`, `min == 0`), zero
    /// `default_frames`/`frames_cycle` entries, `Frames` faults with
    /// `frames == 0`, or a `Frames` fault index that can never fire is
    /// *not* rejected (harmless).
    pub fn add_device(&mut self, spec: DeviceSpec) -> Result<ScriptedDeviceHandle> {
        validate_spec(&spec)?;
        let handle = ScriptedDeviceHandle {
            shared: Arc::new(Mutex::new(DeviceShared {
                spec,
                backend_clock: Arc::clone(&self.clock),
                present: Arc::new(AtomicBool::new(true)),
                open_output: None,
                open_input: None,
            })),
        };
        self.devices.push(handle.clone());
        Ok(handle)
    }

    /// Inject an enumerate-level error returned by all four
    /// enumeration methods until cleared — e.g.
    /// `Backend("permission_denied")` for an OS permission denial.
    pub fn set_enumerate_error(&mut self, error: Option<DeviceError>) {
        self.enumerate_error = error;
    }

    /// Pin the default output device to registration index `index`
    /// (`None` = first present output-capable device).
    pub fn set_default_output(&mut self, index: Option<usize>) {
        self.default_output = index;
    }

    /// Pin the default input device — see [`Self::set_default_output`].
    pub fn set_default_input(&mut self, index: Option<usize>) {
        self.default_input = index;
    }

    /// Number of registered devices (including vanished ones).
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    fn enumerated(&self, output: bool) -> DeviceResult<Vec<Box<dyn Device>>> {
        if let Some(error) = self.enumerate_error {
            return Err(error);
        }
        Ok(self
            .devices
            .iter()
            .filter(|h| {
                h.is_present()
                    && if output {
                        h.has_output_capability()
                    } else {
                        h.has_input_capability()
                    }
            })
            .map(|h| Box::new(h.clone()) as Box<dyn Device>)
            .collect())
    }

    fn default_device(
        &self,
        output: bool,
        index: Option<usize>,
    ) -> DeviceResult<Option<Box<dyn Device>>> {
        if let Some(error) = self.enumerate_error {
            return Err(error);
        }
        let capable = |h: &&ScriptedDeviceHandle| {
            h.is_present()
                && if output {
                    h.has_output_capability()
                } else {
                    h.has_input_capability()
                }
        };
        if let Some(i) = index
            && let Some(h) = self.devices.get(i).filter(|h| capable(h))
        {
            return Ok(Some(Box::new(h.clone())));
        }
        Ok(self
            .devices
            .iter()
            .find(|h| capable(h))
            .map(|h| Box::new(h.clone()) as Box<dyn Device>))
    }
}

impl DeviceEnumerator for ScriptedBackend {
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        self.enumerated(true)
    }

    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        self.default_device(true, self.default_output)
    }

    fn input_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        self.enumerated(false)
    }

    fn default_input_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        self.default_device(false, self.default_input)
    }
}

impl ScriptedDeviceHandle {
    fn capability(&self, output: bool) -> bool {
        lock(&self.shared).spec.direction_present(output)
    }

    fn has_output_capability(&self) -> bool {
        self.capability(true)
    }

    fn has_input_capability(&self) -> bool {
        self.capability(false)
    }
}

impl DeviceSpec {
    fn direction_present(&self, output: bool) -> bool {
        if output {
            self.output.is_some()
        } else {
            self.input.is_some()
        }
    }
}

/// Validates a [`DeviceSpec`] at registration — see
/// [`ScriptedBackend::add_device`].
fn validate_spec(spec: &DeviceSpec) -> Result<()> {
    if spec.name.is_empty() {
        return Err(Error::InvalidValue("name"));
    }
    if let Some(output) = &spec.output {
        validate_format(&output.default_format)?;
        validate_supported_output(&output.supported_formats)?;
        validate_stream_script(&output.stream)?;
    }
    if let Some(input) = &spec.input {
        validate_format(&input.default_format)?;
        validate_supported_input(&input.supported_formats)?;
        validate_stream_script(&input.stream)?;
    }
    Ok(())
}

fn validate_format(format: &AudioFormat) -> Result<()> {
    if format.sample_format != SampleFormat::F32 {
        return Err(Error::InvalidValue("sample_format"));
    }
    Ok(())
}

fn validate_range(
    channels: u32,
    min_rate: u32,
    max_rate: u32,
    buffer: SupportedBufferSize,
) -> Result<()> {
    if channels == 0 || min_rate == 0 || min_rate > max_rate {
        return Err(Error::InvalidValue("supported_format"));
    }
    if let SupportedBufferSize::Range { min, max } = buffer
        && (min == 0 || min > max)
    {
        return Err(Error::InvalidValue("buffer_size"));
    }
    Ok(())
}

fn validate_supported_output(ranges: &[SupportedOutputFormat]) -> Result<()> {
    if ranges.is_empty() {
        return Err(Error::InvalidValue("supported_formats"));
    }
    for r in ranges {
        if r.sample_format != SampleFormat::F32 {
            return Err(Error::InvalidValue("sample_format"));
        }
        validate_range(
            r.channels,
            r.min_sample_rate,
            r.max_sample_rate,
            r.buffer_size,
        )?;
    }
    Ok(())
}

fn validate_supported_input(ranges: &[SupportedInputFormat]) -> Result<()> {
    if ranges.is_empty() {
        return Err(Error::InvalidValue("supported_formats"));
    }
    for r in ranges {
        if r.sample_format != SampleFormat::F32 {
            return Err(Error::InvalidValue("sample_format"));
        }
        validate_range(
            r.channels,
            r.min_sample_rate,
            r.max_sample_rate,
            r.buffer_size,
        )?;
    }
    Ok(())
}

fn validate_stream_script(script: &StreamScript) -> Result<()> {
    if script.default_frames == 0 {
        return Err(Error::InvalidValue("default_frames"));
    }
    if script.frames_cycle.contains(&0) {
        return Err(Error::InvalidValue("frames_cycle"));
    }
    for fault in &script.faults {
        if let StreamFault::Frames { frames: 0, .. } = fault {
            return Err(Error::InvalidValue("frames_fault"));
        }
    }
    Ok(())
}

/// A normalized view of one supported-format range entry — the common
/// fields of [`SupportedOutputFormat`]/[`SupportedInputFormat`].
#[derive(Copy, Clone)]
pub(crate) struct FormatRange {
    pub channels: u32,
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub buffer_size: SupportedBufferSize,
}

impl From<&SupportedOutputFormat> for FormatRange {
    fn from(r: &SupportedOutputFormat) -> Self {
        Self {
            channels: r.channels,
            min_sample_rate: r.min_sample_rate,
            max_sample_rate: r.max_sample_rate,
            buffer_size: r.buffer_size,
        }
    }
}

impl From<&SupportedInputFormat> for FormatRange {
    fn from(r: &SupportedInputFormat) -> Self {
        Self {
            channels: r.channels,
            min_sample_rate: r.min_sample_rate,
            max_sample_rate: r.max_sample_rate,
            buffer_size: r.buffer_size,
        }
    }
}

/// Whether `config`'s format+buffer request is covered by `ranges` —
/// the scripted backend's honest negotiation (reject with
/// `UnsupportedConfig`, never silently coerce).
pub(crate) fn config_supported(
    format: &AudioFormat,
    buffer_size: BufferSize,
    ranges: &[FormatRange],
) -> bool {
    if format.sample_format != SampleFormat::F32 {
        return false;
    }
    let rate = format.sample_rate.get();
    let channels = format.layout.channels();
    let fixed = match buffer_size {
        BufferSize::Default => None,
        BufferSize::Fixed(n) => match u32::try_from(n) {
            Ok(n) if n > 0 => Some(usize::try_from(n).unwrap_or(usize::MAX)),
            _ => return false,
        },
        // `#[non_exhaustive]` — a future variant is not negotiable.
        _ => return false,
    };
    ranges.iter().any(|r| {
        r.channels == channels
            && r.min_sample_rate <= rate
            && rate <= r.max_sample_rate
            && match fixed {
                None => true,
                Some(n) => match r.buffer_size {
                    SupportedBufferSize::Unknown => true,
                    SupportedBufferSize::Range { min, max } => n >= min && n <= max,
                    // `#[non_exhaustive]` — unknown capability cannot
                    // confirm the request.
                    _ => false,
                },
            }
    })
}

/// `Mutex` lock that tolerates poisoning — a panicked pump must not
/// wedge the test driver; every mutation is short and panic-free by
/// construction, so recovered state stays consistent.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Device for ScriptedDeviceHandle {
    fn name(&self) -> DeviceResult<String> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        Ok(d.spec.name.clone())
    }

    fn id(&self) -> DeviceResult<Option<DeviceId>> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        Ok(d.spec.id.clone().map(DeviceId::new))
    }

    fn default_output_format(&self) -> DeviceResult<AudioFormat> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        d.spec
            .output
            .as_ref()
            .map(|o| o.default_format)
            .ok_or(DeviceError::Unsupported)
    }

    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        Ok(d.spec
            .output
            .as_ref()
            .map(|o| o.supported_formats.clone())
            .unwrap_or_default())
    }

    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn device::OutputStream>> {
        stream::open_output(&self.shared, config, data_callback, error_callback)
    }

    fn default_input_format(&self) -> DeviceResult<AudioFormat> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        d.spec
            .input
            .as_ref()
            .map(|i| i.default_format)
            .ok_or(DeviceError::Unsupported)
    }

    fn supported_input_formats(&self) -> DeviceResult<Vec<SupportedInputFormat>> {
        let d = lock(&self.shared);
        if !d.present.load(Ordering::Acquire) {
            return Err(DeviceError::DeviceNotFound);
        }
        Ok(d.spec
            .input
            .as_ref()
            .map(|i| i.supported_formats.clone())
            .unwrap_or_default())
    }

    fn open_input_stream(
        &self,
        config: &InputStreamConfig,
        data_callback: InputDataCallback,
        error_callback: InputErrorCallback,
    ) -> DeviceResult<Box<dyn device::InputStream>> {
        stream::open_input(&self.shared, config, data_callback, error_callback)
    }
}
