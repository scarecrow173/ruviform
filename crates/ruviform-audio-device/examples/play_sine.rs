//! `play_sine` — `PoC` output-path verification on real hardware.
//!
//! Flow (design `PoC 実証フロー` section): [`default_host`] → default output
//! device → format negotiation ([`Device::default_output_format`],
//! falling back to [`select_f32_format`] over
//! [`Device::supported_output_formats`] when the default is `Err` or
//! non-`F32`) → [`Device::open_output_stream`] →
//! [`OutputStream::start`] → a few seconds of 440 Hz sine →
//! [`OutputStream::stop`].
//!
//! RT contract of the example itself: the data callback writes the
//! whole `out` view and records observations through relaxed atomics
//! only — no allocation, no I/O, no blocking. The error callback
//! likewise only increments atomics. The post-`stop()` summary is
//! printed from the control thread.
//!
//! Instrumentation: a counting `#[global_allocator]` wraps `System`
//! for the whole process; counter deltas bracketed around `start()` →
//! `stop()` give an upper bound on allocations occurring during the
//! streaming window — including inside cpal/OS code that
//! `assert_no_alloc` (adapter self-code only) cannot wrap. It counts
//! process-wide events, so the delta is an upper bound, not per-thread
//! attribution.
//!
//! Hardware-dependent manual run — never in CI.

use std::alloc::{GlobalAlloc, Layout, System};
use std::f64::consts::TAU;
use std::io::Write as _;
use std::process::{self, ExitCode};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use ruviform_audio_core::device::{
    BufferSize, Device, DeviceEnumerator, DeviceError, OutputCallbackInfo, OutputDataCallback,
    OutputErrorCallback, OutputStream, OutputStreamConfig, SupportedOutputFormat,
};
use ruviform_audio_core::{
    AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_audio_device::default_host;

/// Sine frequency in Hz.
const SINE_HZ: f64 = 440.0;
/// Sine amplitude (linear, well below full scale).
const AMPLITUDE: f64 = 0.25;
/// How long the stream plays.
const PLAY_SECONDS: Duration = Duration::from_secs(5);
/// Hard bound on total runtime — the watchdog aborts the process if
/// any control-path call (enumerate/open/start/stop) hangs.
const MAX_RUN: Duration = Duration::from_secs(30);
/// Requested fixed buffer size in frames (10 ms at 48 kHz).
const REQUESTED_FRAMES: usize = 480;

/// Process-wide allocation events — the counting-allocator probe.
static ALLOC_EVENTS: AtomicUsize = AtomicUsize::new(0);
/// Process-wide deallocation events.
static DEALLOC_EVENTS: AtomicUsize = AtomicUsize::new(0);

/// Data-callback invocations observed.
static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
/// Smallest `out.frames()` observed (`usize::MAX` before the first
/// callback).
static MIN_FRAMES: AtomicUsize = AtomicUsize::new(usize::MAX);
/// Largest `out.frames()` observed.
static MAX_FRAMES: AtomicUsize = AtomicUsize::new(0);
/// `MediaTime::value()` (nanoseconds) of the first callback instant;
/// `i64::MIN` before it is observed.
static FIRST_CALLBACK_NS: AtomicI64 = AtomicI64::new(i64::MIN);
/// `MediaTime::value()` of the most recent callback instant.
static LAST_CALLBACK_NS: AtomicI64 = AtomicI64::new(i64::MIN);
/// `playback` instant of the most recent callback (`i64::MIN` when the
/// latest callback carried `None`).
static LAST_PLAYBACK_NS: AtomicI64 = AtomicI64::new(i64::MIN);
/// Callbacks whose `playback` field was `None`.
static PLAYBACK_NONE: AtomicUsize = AtomicUsize::new(0);
/// Total error-callback events.
static ERROR_EVENTS: AtomicUsize = AtomicUsize::new(0);
/// `DeviceError::Xrun` events seen on the error callback.
static XRUNS: AtomicUsize = AtomicUsize::new(0);
/// [`error_kind_code`] of the most recent error event (`0` = none).
static LAST_ERROR_KIND: AtomicUsize = AtomicUsize::new(0);

/// Process-wide counting allocator: forwards every operation to
/// [`System`] and bumps relaxed counters, so allocations made inside
/// the backend's own callback thread are still counted.
///
/// SAFETY: every method is a verbatim `System` forward; the only added
/// work is relaxed atomic increments, which cannot affect the validity
/// of any allocation.
#[expect(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_EVENTS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: verbatim forward of the caller's `layout` to `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_EVENTS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: verbatim forward of the caller's `ptr`/`layout` to `System`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// See [`ALLOC_EVENTS`]; a unit type — all state lives in the
/// counters.
struct CountingAlloc;

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn main() -> ExitCode {
    spawn_watchdog();
    run()
}

/// Spawns the watchdog: if the run has not finished within
/// [`MAX_RUN`], prints a marker and aborts the process — a manual `PoC`
/// run must never hang a session.
fn spawn_watchdog() {
    thread::spawn(|| {
        thread::sleep(MAX_RUN);
        eprintln!("play_sine: watchdog timeout after {MAX_RUN:?} — aborting");
        let _ = std::io::stdout().flush();
        process::exit(3);
    });
}

/// The `PoC` session. Returns [`ExitCode::SUCCESS`] only when at least
/// one data callback was observed — "open+start but silence" is itself
/// a failed verification, not a pass.
fn run() -> ExitCode {
    println!("play_sine: device PoC output verification");
    println!("  duration {PLAY_SECONDS:?}, watchdog {MAX_RUN:?}");

    let host = default_host();
    println!("  host: {}", host.name());

    let device = match host.default_output_device() {
        Ok(Some(device)) => device,
        Ok(None) => {
            println!("  default output device: none — cannot play");
            return ExitCode::from(2);
        }
        Err(error) => {
            println!("  default_output_device() failed: {error}");
            return ExitCode::from(2);
        }
    };
    let name = device
        .name()
        .unwrap_or_else(|_| "<name query failed>".to_owned());
    println!("  device: \"{name}\"");

    let Some(format) = negotiate_format(device.as_ref()) else {
        println!("  no F32-openable format — cannot play");
        return ExitCode::from(2);
    };
    println!("  negotiated format: {format:?}");

    let step = TAU * SINE_HZ / f64::from(format.sample_rate.get());
    let alloc_before_open = ALLOC_EVENTS.load(Ordering::Relaxed);
    let Some(stream) = open_with_fallback(device.as_ref(), format, step) else {
        return ExitCode::from(2);
    };

    println!("  stream.format(): {:?}", stream.format());
    match stream.buffer_size() {
        Ok(size) => println!("  stream.buffer_size(): {size:?}"),
        Err(error) => println!("  stream.buffer_size(): query failed: {error}"),
    }
    match stream.now() {
        Ok(now) => println!("  stream.now() pre-start: {} ns", now.value()),
        Err(error) => println!("  stream.now(): query failed: {error}"),
    }

    let alloc_before_start = ALLOC_EVENTS.load(Ordering::Relaxed);
    let dealloc_before_start = DEALLOC_EVENTS.load(Ordering::Relaxed);
    if let Err(error) = stream.start() {
        println!("  start() failed: {error}");
        return ExitCode::from(2);
    }
    println!("  playing {SINE_HZ} Hz sine…");
    thread::sleep(PLAY_SECONDS);

    let alloc_after_play = ALLOC_EVENTS.load(Ordering::Relaxed);
    let dealloc_after_play = DEALLOC_EVENTS.load(Ordering::Relaxed);
    match stream.stop() {
        Ok(()) => println!("  stream stopped"),
        Err(error) => println!("  stop() failed: {error}"),
    }
    match stream.now() {
        Ok(now) => println!("  stream.now() post-stop: {} ns", now.value()),
        Err(error) => println!("  stream.now() post-stop: query failed: {error}"),
    }

    print_summary(
        alloc_before_open,
        alloc_before_start,
        alloc_after_play,
        dealloc_before_start,
        dealloc_after_play,
    );

    if CALLBACKS.load(Ordering::Relaxed) == 0 {
        println!("  RESULT: FAIL — stream started but no data callback observed");
        return ExitCode::from(4);
    }
    println!("  RESULT: OK — callbacks observed");
    ExitCode::SUCCESS
}

/// Format negotiation: prefer the adapter-negotiated default when it
/// is `F32`; on `Err` or a non-`F32` default, synthesize an `F32`
/// [`AudioFormat`] from the supported space via [`select_f32_format`].
fn negotiate_format(device: &dyn Device) -> Option<AudioFormat> {
    match device.default_output_format() {
        Ok(format) if format.sample_format == SampleFormat::F32 => Some(format),
        non_f32_or_err => {
            match &non_f32_or_err {
                Ok(format) => println!("  default_output_format is non-F32: {format:?}"),
                Err(error) => println!("  default_output_format() failed: {error}"),
            }
            println!("  falling back to supported_output_formats() + select_f32_format");
            let supported = match device.supported_output_formats() {
                Ok(supported) => supported,
                Err(error) => {
                    println!("  supported_output_formats() failed: {error}");
                    return None;
                }
            };
            select_f32_format(&supported)
        }
    }
}

/// Example-side fallback helper: picks an openable `F32`
/// [`AudioFormat`] out of the supported space — preferring a stereo
/// range, then one containing 48 kHz, and clamping 48 kHz into the
/// picked range. `None` when no `F32` range with a mappable channel
/// count exists.
fn select_f32_format(supported: &[SupportedOutputFormat]) -> Option<AudioFormat> {
    supported
        .iter()
        .filter(|range| range.sample_format == SampleFormat::F32)
        .filter(|range| layout_for_channels(range.channels).is_some())
        .max_by_key(|range| {
            (
                range.channels == 2,
                range.min_sample_rate <= 48_000 && 48_000 <= range.max_sample_rate,
            )
        })
        .and_then(|range| {
            let layout = layout_for_channels(range.channels)?;
            Some(AudioFormat {
                sample_rate: SampleRate::new(
                    48_000.clamp(range.min_sample_rate, range.max_sample_rate),
                )
                .ok()?,
                layout,
                sample_format: SampleFormat::F32,
            })
        })
}

/// Conventional channel-count → layout mapping (the same subset the
/// adapter maps; unknown counts yield `None`).
fn layout_for_channels(channels: u32) -> Option<ChannelLayout> {
    match channels {
        1 => Some(ChannelLayout::Mono),
        2 => Some(ChannelLayout::Stereo),
        4 => Some(ChannelLayout::Quad),
        6 => Some(ChannelLayout::FivePointOne),
        8 => Some(ChannelLayout::SevenPointOne),
        _ => None,
    }
}

/// Opens the stream, first requesting `BufferSize::Fixed(480)`; a
/// `DeviceError::UnsupportedConfig` retry with `BufferSize::Default`
/// separates "fixed size rejected" from "stream cannot open".
fn open_with_fallback(
    device: &dyn Device,
    format: AudioFormat,
    step: f64,
) -> Option<Box<dyn OutputStream>> {
    let requested = OutputStreamConfig {
        format,
        buffer_size: BufferSize::Fixed(REQUESTED_FRAMES),
    };
    match device.open_output_stream(&requested, data_callback(step), error_callback()) {
        Ok(stream) => {
            println!("  open_output_stream: Ok (requested {requested:?})");
            Some(stream)
        }
        Err(DeviceError::UnsupportedConfig) => {
            println!("  open Fixed({REQUESTED_FRAMES}) → UnsupportedConfig; retrying with Default");
            let fallback = OutputStreamConfig {
                format,
                buffer_size: BufferSize::Default,
            };
            match device.open_output_stream(&fallback, data_callback(step), error_callback()) {
                Ok(stream) => {
                    println!("  open_output_stream: Ok (requested {fallback:?})");
                    Some(stream)
                }
                Err(error) => {
                    println!("  open_output_stream: failed again: {error}");
                    None
                }
            }
        }
        Err(error) => {
            println!("  open_output_stream: failed: {error}");
            None
        }
    }
}

/// One sine sample at `phase` radians. The `f64 → f32` rounding is
/// deliberate: the canonical boundary format is `f32` and a `PoC` tone
/// needs no `f64` output.
#[allow(clippy::cast_possible_truncation)]
fn sine_sample(phase: f64) -> f32 {
    (phase.sin() * AMPLITUDE) as f32
}

/// The RT data callback — the code that runs on the backend's
/// real-time thread. Writes a continuous-phase 440 Hz sine into every
/// channel of `out` (same sample on all channels), then records
/// observations through relaxed atomics. No allocation, no I/O, no
/// blocking, no panicking.
///
/// `step` is the per-frame phase increment in radians, derived in
/// `run` from the negotiated format's sample rate. The closure owns
/// `phase`/`step`; both are plain `f64` (trivial `Drop`, safe wherever
/// the backend drops the callback object).
#[allow(clippy::cast_precision_loss)]
fn data_callback(step: f64) -> OutputDataCallback {
    let mut phase = 0.0_f64;
    Box::new(
        move |mut out: AudioBufferViewMut<'_, f32>, info: OutputCallbackInfo| {
            let frames = out.frames();
            CALLBACKS.fetch_add(1, Ordering::Relaxed);
            MIN_FRAMES.fetch_min(frames, Ordering::Relaxed);
            MAX_FRAMES.fetch_max(frames, Ordering::Relaxed);

            let callback_ns = info.timestamp.callback.value();
            FIRST_CALLBACK_NS.fetch_min(callback_ns, Ordering::Relaxed);
            LAST_CALLBACK_NS.store(callback_ns, Ordering::Relaxed);
            match info.timestamp.playback {
                Some(playback) => LAST_PLAYBACK_NS.store(playback.value(), Ordering::Relaxed),
                None => {
                    PLAYBACK_NONE.fetch_add(1, Ordering::Relaxed);
                }
            }

            // Same sine on every channel: re-derive each channel's
            // phase from the callback start, then advance the owned
            // phase once per whole frame block.
            let start_phase = phase;
            for channel in 0..out.channels() {
                let mut channel_phase = start_phase;
                if let Some(samples) = out.channel_mut(channel) {
                    for sample in samples.iter_mut() {
                        *sample = sine_sample(channel_phase);
                        channel_phase += step;
                    }
                }
            }
            phase = (start_phase + step * frames as f64) % TAU;
        },
    )
}

/// The error callback — counts events, records the latest error kind
/// and tallies xruns. [`DeviceError`] is `Copy`, so this path never
/// allocates; matching the error-callback RT-adjacent contract.
fn error_callback() -> OutputErrorCallback {
    Box::new(|error: DeviceError| {
        ERROR_EVENTS.fetch_add(1, Ordering::Relaxed);
        LAST_ERROR_KIND.store(error_kind_code(error), Ordering::Relaxed);
        if error == DeviceError::Xrun {
            XRUNS.fetch_add(1, Ordering::Relaxed);
        }
    })
}

/// Stable numeric codes for [`DeviceError`] kinds, so the most recent
/// error can be recorded in an [`AtomicUsize`] without allocation.
/// `DeviceError` is `#[non_exhaustive]` — future variants land on `9`.
fn error_kind_code(error: DeviceError) -> usize {
    match error {
        DeviceError::DeviceNotFound => 1,
        DeviceError::DeviceBusy => 2,
        DeviceError::UnsupportedConfig => 3,
        DeviceError::Unsupported => 4,
        DeviceError::Xrun => 5,
        DeviceError::RealtimeDenied => 6,
        DeviceError::StreamInvalidated => 7,
        DeviceError::Backend(_) => 8,
        _ => 9,
    }
}

/// Prints the post-`stop()` observation summary from the control
/// thread: callback statistics, timestamp bounds, error events and the
/// process-wide allocator deltas measured during `open` and during the
/// streaming window (`start()` → `stop()`). The streaming-window delta
/// includes cpal/OS-internal allocations — the upper-bound probe
/// described in the module docs.
fn print_summary(
    alloc_before_open: usize,
    alloc_before_start: usize,
    alloc_after_play: usize,
    dealloc_before_start: usize,
    dealloc_after_play: usize,
) {
    println!("--- summary ---");
    println!("  callbacks: {}", CALLBACKS.load(Ordering::Relaxed));
    let min_frames = MIN_FRAMES.load(Ordering::Relaxed);
    println!(
        "  frames per callback: min={} max={}",
        if min_frames == usize::MAX {
            0
        } else {
            min_frames
        },
        MAX_FRAMES.load(Ordering::Relaxed),
    );
    let first_ns = FIRST_CALLBACK_NS.load(Ordering::Relaxed);
    println!(
        "  callback timestamp (ns): first={} last={}",
        if first_ns == i64::MIN { 0 } else { first_ns },
        LAST_CALLBACK_NS.load(Ordering::Relaxed),
    );
    println!(
        "  playback timestamp: last={} none_count={}",
        LAST_PLAYBACK_NS.load(Ordering::Relaxed),
        PLAYBACK_NONE.load(Ordering::Relaxed),
    );
    println!(
        "  error events: {} (xrun={}, last kind code={})",
        ERROR_EVENTS.load(Ordering::Relaxed),
        XRUNS.load(Ordering::Relaxed),
        LAST_ERROR_KIND.load(Ordering::Relaxed),
    );
    println!(
        "  alloc events during open: {}",
        alloc_before_start.saturating_sub(alloc_before_open),
    );
    println!(
        "  alloc events during streaming (process-wide, incl. backend): {}",
        alloc_after_play.saturating_sub(alloc_before_start),
    );
    println!(
        "  dealloc events during streaming: {}",
        dealloc_after_play.saturating_sub(dealloc_before_start),
    );
}
