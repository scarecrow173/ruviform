//! Scripted stream implementations — the manual-pump half of the
//! scripted backend.
//!
//! `OutputSlot`/`InputSlot` are the shared state between the
//! `Box<dyn …Stream>` trait object and the test-side
//! [`ScriptedDeviceHandle`](super::ScriptedDeviceHandle) pump. The pump
//! is a non-RT test-driver path: it runs on the caller's thread, may
//! allocate for capture work, and is the *only* way callbacks fire —
//! `start()` merely marks the stream started.
//!
//! The allocation-free section proven by `rt_` tests is the callback
//! firing path itself: scratch reuse, `try_new` view construction,
//! timestamp build, `data_callback`/`error_callback` invocation, and
//! counter updates — no `Vec` growth, no `String`, no I/O.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, DeviceError, DeviceResult, InputCallbackInfo,
    InputDataCallback, InputErrorCallback, InputStream, InputStreamConfig, InputTimestamp,
    OutputCallbackInfo, OutputDataCallback, OutputErrorCallback, OutputStream, OutputStreamConfig,
    OutputTimestamp, StreamLatency,
};
use ruviform_audio_core::{AudioBufferView, AudioBufferViewMut, AudioFormat};
use ruviform_media_core::{ClockDomainId, MediaTime, MediaTimestamp, TimestampProvenance};

use super::{
    DeviceShared, FormatRange, PumpReport, StreamDirection, StreamFault, StreamScript,
    config_supported,
};
use crate::clock::VirtualClock;
use crate::pcm::{PcmBlock, PcmScenario};

/// Nanoseconds per second as `u128` — latency frame→nanos math.
const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Shared slot for an open output stream: the mutex-protected core
/// plus a lock-free `dead` flag the device checks without locking the
/// core (invalidate/drop mark it so the direction can be re-opened).
pub(crate) struct OutputSlot {
    pub core: Mutex<OutputCore>,
    pub dead: AtomicBool,
}

/// Shared slot for an open input stream — see [`OutputSlot`].
pub(crate) struct InputSlot {
    pub core: Mutex<InputCore>,
    pub dead: AtomicBool,
}

/// Everything an output stream needs after open — resolved script
/// values copied out of the spec so the pump never touches
/// [`DeviceShared`].
pub(crate) struct OutputCore {
    pub started: bool,
    pub invalidated: bool,
    pub callback_index: u64,
    pub position: u64,
    pub format: AudioFormat,
    pub channels: usize,
    pub clock: Arc<VirtualClock>,
    pub clock_domain: ClockDomainId,
    /// Frames per callback when `frames_cycle` is empty.
    pub resolved_frames: u32,
    /// Per-callback frame cycle — empty means fixed `resolved_frames`.
    pub frames_cycle: Vec<u32>,
    pub latency: Option<StreamLatency>,
    /// `latency.frames` converted to nanoseconds at `sample_rate`.
    pub latency_nanos: i64,
    pub record_output: bool,
    /// Live fault list — cloned from the spec at open, appended by
    /// `inject_output_fault`.
    pub faults: Vec<StreamFault>,
    pub start_faults: VecDeque<DeviceError>,
    pub stop_faults: VecDeque<DeviceError>,
    /// Device presence flag — vanish mid-stream invalidates on pump.
    pub present: Arc<AtomicBool>,
    /// Reused planar scratch (`channels * max_frames` capacity).
    pub scratch: Vec<f32>,
    /// Post-callback captures, oldest first (`record_output` only).
    pub captured: Vec<PcmBlock>,
    pub data_callback: Option<OutputDataCallback>,
    pub error_callback: Option<OutputErrorCallback>,
}

/// Input-direction core — [`OutputCore`] plus the PCM scenario and
/// capture-reporting policy.
pub(crate) struct InputCore {
    pub started: bool,
    pub invalidated: bool,
    pub callback_index: u64,
    pub position: u64,
    pub channels: usize,
    pub clock: Arc<VirtualClock>,
    pub clock_domain: ClockDomainId,
    pub resolved_frames: u32,
    pub frames_cycle: Vec<u32>,
    pub latency_nanos: i64,
    /// `capture = Some(callback − latency)` when `true`.
    pub capture_reporting: bool,
    pub pcm: PcmScenario,
    pub faults: Vec<StreamFault>,
    pub start_faults: VecDeque<DeviceError>,
    pub stop_faults: VecDeque<DeviceError>,
    pub present: Arc<AtomicBool>,
    pub scratch: Vec<f32>,
    pub data_callback: Option<InputDataCallback>,
    pub error_callback: Option<InputErrorCallback>,
}

/// `BufferSize` report for a stream — `Fixed` for a fixed negotiated
/// count, `Default` when a `frames_cycle` makes delivery vary.
fn buffer_report(script: &StreamScript, resolved: u32) -> BufferSize {
    if script.frames_cycle.is_empty() {
        BufferSize::Fixed(usize::try_from(resolved).unwrap_or(usize::MAX))
    } else {
        BufferSize::Default
    }
}

/// Frames for callback `index`: cycle value when scripted, else the
/// resolved fixed count.
fn frames_at(cycle: &[u32], resolved: u32, index: u64) -> u32 {
    if cycle.is_empty() {
        return resolved;
    }
    let len = u64::try_from(cycle.len()).unwrap_or(u64::MAX);
    cycle[usize::try_from(index % len).unwrap_or(0)]
}

/// `latency.frames` at `sample_rate` → nanoseconds, saturating.
fn latency_nanos(latency: Option<StreamLatency>, sample_rate: u32) -> i64 {
    latency.map_or(0, |l| {
        let nanos = (u128::from(l.frames) * NANOS_PER_SEC) / u128::from(sample_rate.max(1));
        i64::try_from(nanos).unwrap_or(i64::MAX)
    })
}

/// Device-clock nanos → `i64` `MediaTime` ticks (saturating).
fn clock_nanos(clock: &VirtualClock) -> i64 {
    i64::try_from(clock.monotonic_nanos()).unwrap_or(i64::MAX)
}

fn media_nanos(nanos: i64) -> MediaTime {
    MediaTime::new(nanos, DEVICE_CLOCK_TIMEBASE)
}

/// Effects collected from the [`StreamFault`]s matching one callback
/// index. `errors` is a caller-owned `Vec` kept across pumps so the
/// steady-state path performs no allocation.
#[derive(Default)]
struct FaultEffects {
    /// `Frames` override for this callback.
    frames: Option<u32>,
    /// `FramePosition` jump applied before this callback.
    position: Option<u64>,
    /// `CaptureNone` suppression (input only).
    capture_none: bool,
    /// `Invalidate` — terminate after error delivery.
    invalidate: bool,
    /// `Cancellation` — abort data delivery, report
    /// `Backend("cancelled")`, terminate.
    cancel: bool,
}

/// Applies the [`StreamFault`]s matching `index` — collects mutations,
/// the ordered error list, and the invalidate/cancel flags, and clears
/// `errors` for reuse.
fn collect_faults(
    faults: &[StreamFault],
    index: u64,
    errors: &mut Vec<DeviceError>,
) -> FaultEffects {
    errors.clear();
    let mut effects = FaultEffects::default();
    for &fault in faults {
        match fault {
            StreamFault::Error { at_callback, error } if at_callback == index => errors.push(error),
            StreamFault::FramePosition {
                at_callback,
                position: p,
            } if at_callback == index => effects.position = Some(p),
            StreamFault::CaptureNone { at_callback } if at_callback == index => {
                effects.capture_none = true;
            }
            StreamFault::Frames {
                at_callback,
                frames: f,
            } if at_callback == index => effects.frames = Some(f),
            StreamFault::Invalidate { at_callback } if at_callback == index => {
                effects.invalidate = true;
            }
            StreamFault::Cancellation { at_callback } if at_callback == index => {
                effects.cancel = true;
            }
            _ => {}
        }
    }
    effects
}

/// Pump prologue shared by both directions: dead/invalidated →
/// `StreamInvalidated`; vanished device → latch invalidation, notify
/// the error callback once, then fail; not started →
/// `Backend("not_started")`.
fn output_prologue(core: &mut OutputCore, dead: &AtomicBool) -> DeviceResult<()> {
    if core.invalidated || dead.load(Ordering::Acquire) {
        return Err(DeviceError::StreamInvalidated);
    }
    if !core.present.load(Ordering::Acquire) {
        core.invalidated = true;
        dead.store(true, Ordering::Release);
        if let Some(cb) = core.error_callback.as_mut() {
            cb(DeviceError::StreamInvalidated);
        }
        return Err(DeviceError::StreamInvalidated);
    }
    if !core.started {
        return Err(DeviceError::Backend("not_started"));
    }
    Ok(())
}

/// Input-direction prologue — see [`output_prologue`].
fn input_prologue(core: &mut InputCore, dead: &AtomicBool) -> DeviceResult<()> {
    if core.invalidated || dead.load(Ordering::Acquire) {
        return Err(DeviceError::StreamInvalidated);
    }
    if !core.present.load(Ordering::Acquire) {
        core.invalidated = true;
        dead.store(true, Ordering::Release);
        if let Some(cb) = core.error_callback.as_mut() {
            cb(DeviceError::StreamInvalidated);
        }
        return Err(DeviceError::StreamInvalidated);
    }
    if !core.started {
        return Err(DeviceError::Backend("not_started"));
    }
    Ok(())
}

/// The scripted `ScriptedOutputStream`: trait object + pump target.
pub(crate) struct ScriptedOutputStream {
    pub slot: Arc<OutputSlot>,
    pub device: Arc<Mutex<DeviceShared>>,
    /// Stored (lock-free) trait answers.
    pub format: AudioFormat,
    pub clock_domain: ClockDomainId,
    pub buffer_size_report: BufferSize,
    pub clock: Arc<VirtualClock>,
    pub latency: Option<StreamLatency>,
}

impl OutputStream for ScriptedOutputStream {
    fn start(&self) -> DeviceResult<()> {
        let mut core = super::lock(&self.slot.core);
        if core.invalidated || self.slot.dead.load(Ordering::Acquire) {
            return Err(DeviceError::StreamInvalidated);
        }
        if core.started {
            return Ok(());
        }
        if let Some(error) = core.start_faults.pop_front() {
            return Err(error);
        }
        core.started = true;
        Ok(())
    }

    fn stop(&self) -> DeviceResult<()> {
        let mut core = super::lock(&self.slot.core);
        if !core.started {
            return Ok(());
        }
        if let Some(error) = core.stop_faults.pop_front() {
            return Err(error);
        }
        core.started = false;
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        Ok(self.buffer_size_report)
    }

    fn now(&self) -> DeviceResult<MediaTime> {
        Ok(media_nanos(clock_nanos(&self.clock)))
    }

    fn clock_domain(&self) -> ClockDomainId {
        self.clock_domain
    }

    fn output_latency(&self) -> DeviceResult<Option<StreamLatency>> {
        Ok(self.latency)
    }
}

impl Drop for ScriptedOutputStream {
    fn drop(&mut self) {
        self.slot.dead.store(true, Ordering::Release);
        let mut d = super::lock(&self.device);
        if d.open_output
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &self.slot))
        {
            d.open_output = None;
        }
    }
}

/// The scripted `ScriptedInputStream` — see [`ScriptedOutputStream`].
pub(crate) struct ScriptedInputStream {
    pub slot: Arc<InputSlot>,
    pub device: Arc<Mutex<DeviceShared>>,
    pub format: AudioFormat,
    pub clock_domain: ClockDomainId,
    pub buffer_size_report: BufferSize,
    pub clock: Arc<VirtualClock>,
    pub latency: Option<StreamLatency>,
}

impl InputStream for ScriptedInputStream {
    fn start(&self) -> DeviceResult<()> {
        let mut core = super::lock(&self.slot.core);
        if core.invalidated || self.slot.dead.load(Ordering::Acquire) {
            return Err(DeviceError::StreamInvalidated);
        }
        if core.started {
            return Ok(());
        }
        if let Some(error) = core.start_faults.pop_front() {
            return Err(error);
        }
        core.started = true;
        Ok(())
    }

    fn stop(&self) -> DeviceResult<()> {
        let mut core = super::lock(&self.slot.core);
        if !core.started {
            return Ok(());
        }
        if let Some(error) = core.stop_faults.pop_front() {
            return Err(error);
        }
        core.started = false;
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        Ok(self.buffer_size_report)
    }

    fn now(&self) -> DeviceResult<MediaTime> {
        Ok(media_nanos(clock_nanos(&self.clock)))
    }

    fn clock_domain(&self) -> ClockDomainId {
        self.clock_domain
    }

    fn input_latency(&self) -> DeviceResult<Option<StreamLatency>> {
        Ok(self.latency)
    }
}

impl Drop for ScriptedInputStream {
    fn drop(&mut self) {
        self.slot.dead.store(true, Ordering::Release);
        let mut d = super::lock(&self.device);
        if d.open_input
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &self.slot))
        {
            d.open_input = None;
        }
    }
}

/// Opens an output stream — called from `Device::open_output_stream`.
/// Validation order is the honest contract order: presence →
/// capability → persistent open fault → busy → one-shot open fault →
/// format/buffer negotiation.
///
/// A consumed `once` fault is spent even when a later check fails —
/// an open attempt is an open attempt.
pub(crate) fn open_output(
    device: &Arc<Mutex<DeviceShared>>,
    config: &OutputStreamConfig,
    data_callback: OutputDataCallback,
    error_callback: OutputErrorCallback,
) -> DeviceResult<Box<dyn OutputStream>> {
    let mut d = super::lock(device);
    if !d.present.load(Ordering::Acquire) {
        return Err(DeviceError::DeviceNotFound);
    }
    // Phase A — consume open faults (the only mutation needed from
    // the spec; scoped so the borrow ends before later reads).
    let once_error = {
        let Some(output) = d.spec.output.as_mut() else {
            return Err(DeviceError::Unsupported);
        };
        if let Some(error) = output.stream.open_faults.persistent {
            return Err(error);
        }
        output.stream.open_faults.once.pop_front()
    };
    // Phase B — immutable reads only (all coexist under shared borrows).
    if d.open_output
        .as_ref()
        .is_some_and(|s| !s.dead.load(Ordering::Acquire))
    {
        return Err(DeviceError::DeviceBusy);
    }
    if let Some(error) = once_error {
        return Err(error);
    }
    let Some(output) = d.spec.output.as_ref() else {
        return Err(DeviceError::Unsupported);
    };
    let script = &output.stream;
    let ranges: Vec<FormatRange> = output
        .supported_formats
        .iter()
        .map(FormatRange::from)
        .collect();
    if !config_supported(&config.format, config.buffer_size, &ranges) {
        return Err(DeviceError::UnsupportedConfig);
    }
    let record_output = output.record_output;
    let resolved = resolved_frames(config.buffer_size, script.default_frames)?;
    let max = max_frames(script, resolved);
    let frames_cycle = script.frames_cycle.clone();
    let latency = script.latency;
    let clock_domain_script = script.clock_domain;
    let faults = script.faults.clone();
    let start_faults = script.start_faults.clone();
    let stop_faults = script.stop_faults.clone();
    let buffer_size_report = buffer_report(script, resolved);
    let channels = config.format.layout.channels() as usize;
    let sample_rate = config.format.sample_rate.get();
    let clock = d
        .spec
        .clock
        .clone()
        .unwrap_or_else(|| Arc::clone(&d.backend_clock));
    let clock_domain = clock_domain_script.unwrap_or_else(|| ClockDomainId::new(clock.epoch()));
    let lat_nanos = latency_nanos(latency, sample_rate);
    let present = Arc::clone(&d.present);
    let scratch_len = channels.saturating_mul(max as usize);
    let core = OutputCore {
        started: false,
        invalidated: false,
        callback_index: 0,
        position: 0,
        format: config.format,
        channels,
        clock: Arc::clone(&clock),
        clock_domain,
        resolved_frames: resolved,
        frames_cycle,
        latency,
        latency_nanos: lat_nanos,
        record_output,
        faults,
        start_faults,
        stop_faults,
        present,
        scratch: vec![0.0; scratch_len],
        captured: Vec::new(),
        data_callback: Some(data_callback),
        error_callback: Some(error_callback),
    };
    let slot = Arc::new(OutputSlot {
        core: Mutex::new(core),
        dead: AtomicBool::new(false),
    });
    let stream = ScriptedOutputStream {
        slot: Arc::clone(&slot),
        device: Arc::clone(device),
        format: config.format,
        clock_domain,
        buffer_size_report,
        clock,
        latency,
    };
    d.open_output = Some(slot);
    Ok(Box::new(stream))
}

/// Opens an input stream — see [`open_output`]. Additionally every
/// scenario segment's fixture must match the negotiated sample rate
/// and channel count (`UnsupportedConfig` otherwise — a mid-scenario
/// format change is expressed by closing and re-opening the stream).
pub(crate) fn open_input(
    device: &Arc<Mutex<DeviceShared>>,
    config: &InputStreamConfig,
    data_callback: InputDataCallback,
    error_callback: InputErrorCallback,
) -> DeviceResult<Box<dyn InputStream>> {
    let mut d = super::lock(device);
    if !d.present.load(Ordering::Acquire) {
        return Err(DeviceError::DeviceNotFound);
    }
    // Phase A — consume open faults (scoped mutable borrow).
    let once_error = {
        let Some(input) = d.spec.input.as_mut() else {
            return Err(DeviceError::Unsupported);
        };
        if let Some(error) = input.stream.open_faults.persistent {
            return Err(error);
        }
        input.stream.open_faults.once.pop_front()
    };
    // Phase B — immutable reads only.
    if d.open_input
        .as_ref()
        .is_some_and(|s| !s.dead.load(Ordering::Acquire))
    {
        return Err(DeviceError::DeviceBusy);
    }
    if let Some(error) = once_error {
        return Err(error);
    }
    let Some(input) = d.spec.input.as_ref() else {
        return Err(DeviceError::Unsupported);
    };
    let script = &input.stream;
    let ranges: Vec<FormatRange> = input
        .supported_formats
        .iter()
        .map(FormatRange::from)
        .collect();
    if !config_supported(&config.format, config.buffer_size, &ranges) {
        return Err(DeviceError::UnsupportedConfig);
    }
    let channels = config.format.layout.channels() as usize;
    if input.pcm.segments().iter().any(|seg| {
        usize::from(seg.fixture.channels) != channels
            || seg.fixture.sample_rate != config.format.sample_rate
    }) {
        return Err(DeviceError::UnsupportedConfig);
    }
    let capture_reporting = input.capture_reporting;
    let pcm = input.pcm.clone();
    let resolved = resolved_frames(config.buffer_size, script.default_frames)?;
    let max = max_frames(script, resolved);
    let frames_cycle = script.frames_cycle.clone();
    let latency = script.latency;
    let clock_domain_script = script.clock_domain;
    let faults = script.faults.clone();
    let start_faults = script.start_faults.clone();
    let stop_faults = script.stop_faults.clone();
    let buffer_size_report = buffer_report(script, resolved);
    let sample_rate = config.format.sample_rate.get();
    let clock = d
        .spec
        .clock
        .clone()
        .unwrap_or_else(|| Arc::clone(&d.backend_clock));
    let clock_domain = clock_domain_script.unwrap_or_else(|| ClockDomainId::new(clock.epoch()));
    let lat_nanos = latency_nanos(latency, sample_rate);
    let present = Arc::clone(&d.present);
    let scratch_len = channels.saturating_mul(max as usize);
    let core = InputCore {
        started: false,
        invalidated: false,
        callback_index: 0,
        position: 0,
        channels,
        clock: Arc::clone(&clock),
        clock_domain,
        resolved_frames: resolved,
        frames_cycle,
        latency_nanos: lat_nanos,
        capture_reporting,
        pcm,
        faults,
        start_faults,
        stop_faults,
        present,
        scratch: vec![0.0; scratch_len],
        data_callback: Some(data_callback),
        error_callback: Some(error_callback),
    };
    let slot = Arc::new(InputSlot {
        core: Mutex::new(core),
        dead: AtomicBool::new(false),
    });
    let stream = ScriptedInputStream {
        slot: Arc::clone(&slot),
        device: Arc::clone(device),
        format: config.format,
        clock_domain,
        buffer_size_report,
        clock,
        latency,
    };
    d.open_input = Some(slot);
    Ok(Box::new(stream))
}

/// `config.buffer_size` → concrete frames; `Default` uses the script's
/// `default_frames`. `Fixed(0)`/overflow already rejected by
/// `config_supported`, re-checked here for defense in depth.
fn resolved_frames(buffer_size: BufferSize, default_frames: u32) -> DeviceResult<u32> {
    match buffer_size {
        BufferSize::Default => Ok(default_frames),
        BufferSize::Fixed(n) => u32::try_from(n)
            .ok()
            .filter(|&n| n > 0)
            .ok_or(DeviceError::UnsupportedConfig),
        // `#[non_exhaustive]` — a future variant is not negotiable.
        _ => Err(DeviceError::UnsupportedConfig),
    }
}

/// Largest frame count any callback may deliver — scratch capacity
/// (cycle max, `Frames`-fault max, and the resolved base).
fn max_frames(script: &StreamScript, resolved: u32) -> u32 {
    let mut max = script
        .frames_cycle
        .iter()
        .copied()
        .max()
        .unwrap_or(resolved)
        .max(resolved);
    for fault in &script.faults {
        if let StreamFault::Frames { frames, .. } = fault {
            max = max.max(*frames);
        }
    }
    max
}

/// Plays `pcm` into `scratch` for the window `[position, win_end)`,
/// returning the covered frame count. Frames outside every segment
/// stay silence — the caller counts them as `gap_frames`.
fn play_scenario(
    pcm: &PcmScenario,
    scratch: &mut [f32],
    channels: usize,
    frames_usize: usize,
    position: u64,
    win_end: u64,
) -> u64 {
    let mut covered = 0u64;
    for seg in pcm.segments() {
        let seg_end = seg.frame_start.saturating_add(seg.frame_len);
        let a = seg.frame_start.max(position);
        let b = seg_end.min(win_end);
        if a >= b {
            continue;
        }
        covered = covered.saturating_add(b - a);
        let offset = usize::try_from(a - position).unwrap_or(usize::MAX);
        let len = usize::try_from(b - a).unwrap_or(usize::MAX);
        for ch in 0..channels {
            let base = ch * frames_usize + offset;
            let channel = u16::try_from(ch).unwrap_or(u16::MAX);
            seg.fixture
                .fill_channel(channel, a, &mut scratch[base..base + len]);
        }
    }
    covered
}

/// Shared body of the cancellation path: delivers the same-index
/// `Error` faults, then `Backend("cancelled")`, then
/// `StreamInvalidated` when `Invalidate` is also scripted there, and
/// terminates the stream. `callback_index` advances — the cancelled
/// firing consumed one callback — while `position`/`frames` do not
/// advance (zero frames delivered).
fn deliver_cancel_errors(
    error_callback: &mut Option<impl FnMut(DeviceError)>,
    fault_errors: &[DeviceError],
    invalidate: bool,
) -> u32 {
    let mut errors_fired = 0u32;
    for &error in fault_errors {
        if let Some(cb) = error_callback.as_mut() {
            cb(error);
        }
        errors_fired += 1;
    }
    if let Some(cb) = error_callback.as_mut() {
        cb(DeviceError::Backend("cancelled"));
    }
    errors_fired += 1;
    if invalidate {
        if let Some(cb) = error_callback.as_mut() {
            cb(DeviceError::StreamInvalidated);
        }
        errors_fired += 1;
    }
    errors_fired
}

/// Output-direction cancellation path — see
/// [`StreamFault::Cancellation`]. Returns a zero-frame `PumpReport`
/// with `invalidated: true`; the stream is dead afterwards.
fn cancelled_pump_output(
    core: &mut OutputCore,
    slot: &OutputSlot,
    index: u64,
    effects: &FaultEffects,
    fault_errors: &[DeviceError],
) -> PumpReport {
    let callback_nanos = clock_nanos(&core.clock);
    let errors_fired =
        deliver_cancel_errors(&mut core.error_callback, fault_errors, effects.invalidate);
    core.invalidated = true;
    slot.dead.store(true, Ordering::Release);
    core.callback_index = index + 1;
    PumpReport {
        direction: StreamDirection::Output,
        callback_index: index,
        frames: 0,
        position: core.position,
        reported_frame_position: None,
        clock_domain: core.clock_domain,
        callback_nanos,
        capture_nanos: None,
        playback_nanos: None,
        gap_frames: 0,
        errors_fired,
        invalidated: true,
    }
}

/// Input-direction cancellation path — see [`StreamFault::Cancellation`].
/// No capture timestamp or frame position is reported: the callback was
/// aborted before data delivery.
fn cancelled_pump_input(
    core: &mut InputCore,
    slot: &InputSlot,
    index: u64,
    effects: &FaultEffects,
    fault_errors: &[DeviceError],
) -> PumpReport {
    let callback_nanos = clock_nanos(&core.clock);
    let errors_fired =
        deliver_cancel_errors(&mut core.error_callback, fault_errors, effects.invalidate);
    core.invalidated = true;
    slot.dead.store(true, Ordering::Release);
    core.callback_index = index + 1;
    PumpReport {
        direction: StreamDirection::Input,
        callback_index: index,
        frames: 0,
        position: core.position,
        reported_frame_position: None,
        clock_domain: core.clock_domain,
        callback_nanos,
        capture_nanos: None,
        playback_nanos: None,
        gap_frames: 0,
        errors_fired,
        invalidated: true,
    }
}

/// Fires one output callback — see
/// [`ScriptedDeviceHandle::pump_output_next`](super::ScriptedDeviceHandle::pump_output_next).
pub(crate) fn pump_output(slot: &Arc<OutputSlot>) -> DeviceResult<PumpReport> {
    let mut guard = super::lock(&slot.core);
    let core = &mut *guard;
    output_prologue(core, &slot.dead)?;
    let index = core.callback_index;

    // Reused per-core fault scratch would be ideal; a fresh `Vec::new`
    // allocates only when a fault actually fires (steady state: none).
    let mut fault_errors = Vec::new();
    let effects = collect_faults(&core.faults, index, &mut fault_errors);
    if effects.cancel {
        return Ok(cancelled_pump_output(
            core,
            slot,
            index,
            &effects,
            &fault_errors,
        ));
    }
    if let Some(p) = effects.position {
        core.position = p;
    }
    let position = core.position;
    let frames = effects
        .frames
        .unwrap_or_else(|| frames_at(&core.frames_cycle, core.resolved_frames, index));
    let channels = core.channels;
    let frames_usize = usize::try_from(frames).unwrap_or(usize::MAX);
    let sample_count = channels.saturating_mul(frames_usize);

    if core.scratch.len() < sample_count {
        core.scratch.resize(sample_count, 0.0);
    }
    core.scratch[..sample_count].fill(0.0);

    let callback_nanos = clock_nanos(&core.clock);
    let playback_nanos = core
        .latency
        .map(|_| callback_nanos.saturating_add(core.latency_nanos));
    let info = OutputCallbackInfo {
        timestamp: OutputTimestamp {
            callback: media_nanos(callback_nanos),
            playback: playback_nanos.map(media_nanos),
        },
    };

    {
        let OutputCore {
            scratch,
            data_callback,
            ..
        } = &mut *core;
        let view =
            AudioBufferViewMut::try_new(&mut scratch[..sample_count], channels, frames_usize)
                .map_err(|_| DeviceError::UnsupportedConfig)?;
        if let Some(cb) = data_callback.as_mut() {
            cb(view, info);
        }
    }

    let mut errors_fired = 0u32;
    for &error in &fault_errors {
        if let Some(cb) = core.error_callback.as_mut() {
            cb(error);
        }
        errors_fired += 1;
    }
    if effects.invalidate {
        if let Some(cb) = core.error_callback.as_mut() {
            cb(DeviceError::StreamInvalidated);
        }
        errors_fired += 1;
        core.invalidated = true;
        slot.dead.store(true, Ordering::Release);
    }

    if core.record_output {
        let data = core.scratch[..sample_count].to_vec();
        core.captured.push(PcmBlock {
            start_frame: position,
            frames,
            channels: u16::try_from(channels).unwrap_or(u16::MAX),
            sample_rate: core.format.sample_rate,
            data,
        });
    }

    core.callback_index = index + 1;
    core.position = position.saturating_add(u64::from(frames));

    Ok(PumpReport {
        direction: StreamDirection::Output,
        callback_index: index,
        frames,
        position,
        reported_frame_position: None,
        clock_domain: core.clock_domain,
        callback_nanos,
        capture_nanos: None,
        playback_nanos,
        gap_frames: 0,
        errors_fired,
        invalidated: core.invalidated,
    })
}

/// Fires one input callback — see
/// [`ScriptedDeviceHandle::pump_input_next`](super::ScriptedDeviceHandle::pump_input_next).
pub(crate) fn pump_input(slot: &Arc<InputSlot>) -> DeviceResult<PumpReport> {
    let mut guard = super::lock(&slot.core);
    let core = &mut *guard;
    input_prologue(core, &slot.dead)?;
    let index = core.callback_index;

    let mut fault_errors = Vec::new();
    let effects = collect_faults(&core.faults, index, &mut fault_errors);
    if effects.cancel {
        return Ok(cancelled_pump_input(
            core,
            slot,
            index,
            &effects,
            &fault_errors,
        ));
    }
    if let Some(p) = effects.position {
        core.position = p;
    }
    let position = core.position;
    let frames = effects
        .frames
        .unwrap_or_else(|| frames_at(&core.frames_cycle, core.resolved_frames, index));
    let channels = core.channels;
    let frames_usize = usize::try_from(frames).unwrap_or(usize::MAX);
    let sample_count = channels.saturating_mul(frames_usize);
    let win_end = position.saturating_add(u64::from(frames));

    if core.scratch.len() < sample_count {
        core.scratch.resize(sample_count, 0.0);
    }
    core.scratch[..sample_count].fill(0.0);

    // Play the scenario into the window. Frames outside every segment
    // stay silence and count toward `gap_frames` (reported as Xrun —
    // loss is always observable, never silent).
    let covered = play_scenario(
        &core.pcm,
        &mut core.scratch,
        channels,
        frames_usize,
        position,
        win_end,
    );
    let gap_frames = u64::from(frames).saturating_sub(covered);

    let callback_nanos = clock_nanos(&core.clock);
    let capture_nanos = if core.capture_reporting && !effects.capture_none {
        Some(callback_nanos.saturating_sub(core.latency_nanos))
    } else {
        None
    };
    let info = InputCallbackInfo {
        timestamp: InputTimestamp {
            callback: media_nanos(callback_nanos),
            capture: capture_nanos.map(|n| MediaTimestamp {
                time: media_nanos(n),
                provenance: TimestampProvenance::Derived,
            }),
        },
        clock_domain: core.clock_domain,
        frame_position: Some(position),
    };

    {
        let InputCore {
            scratch,
            data_callback,
            ..
        } = &mut *core;
        let view = AudioBufferView::try_new(&scratch[..sample_count], channels, frames_usize)
            .map_err(|_| DeviceError::UnsupportedConfig)?;
        if let Some(cb) = data_callback.as_mut() {
            cb(view, info);
        }
    }

    let mut errors_fired = 0u32;
    for &error in &fault_errors {
        if let Some(cb) = core.error_callback.as_mut() {
            cb(error);
        }
        errors_fired += 1;
    }
    if gap_frames > 0 {
        if let Some(cb) = core.error_callback.as_mut() {
            cb(DeviceError::Xrun);
        }
        errors_fired += 1;
    }
    if effects.invalidate {
        if let Some(cb) = core.error_callback.as_mut() {
            cb(DeviceError::StreamInvalidated);
        }
        errors_fired += 1;
        core.invalidated = true;
        slot.dead.store(true, Ordering::Release);
    }

    core.callback_index = index + 1;
    core.position = position.saturating_add(u64::from(frames));

    Ok(PumpReport {
        direction: StreamDirection::Input,
        callback_index: index,
        frames,
        position,
        reported_frame_position: Some(position),
        clock_domain: core.clock_domain,
        callback_nanos,
        capture_nanos,
        playback_nanos: None,
        gap_frames,
        errors_fired,
        invalidated: core.invalidated,
    })
}
