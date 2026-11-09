//! Real-device input verification lane (Issue #251) — the opt-in
//! counterpart of the Issue #114 `l1-real-verify` lane for the
//! audio-device adapter's input direction.
//!
//! Every test is `#[ignore]`-gated and the whole file is
//! `#![cfg(feature = "real-device")]`-gated: default `cargo test` runs
//! never see these tests (the file compiles empty without the feature,
//! so nothing here can break the default lane), and the only intended
//! entry point is `mise run test-audio-real-input`, which enables the
//! feature and selects `--ignored` explicitly.
//!
//! Hardware-dependent manual lane — never in CI. Requires a real
//! input device and OS capture permission. What it verifies when
//! hardware is present:
//!
//! - `open_input_stream` → `start` → planar `AudioBufferView`
//!   callbacks actually fire — open+start with zero callbacks is a
//!   FAIL, not a pass (the "no fake capture" rule).
//! - Callback timestamps are non-decreasing on the stream clock and
//!   every `InputCallbackInfo` carries the stream's `clock_domain`.
//! - Duplex domain agreement: input and output streams opened on the
//!   same host report the same `ClockDomainId`, and
//!   `same_clock_domain` renders the honest verdict — `true` only on
//!   shared-clock hosts (WASAPI/CoreAudio/PipeWire), `false` on
//!   per-device-clock or unaudited hosts (ALSA et al.).
//!
//! What this lane does NOT verify (recorded `NotRun` in the Issue
//! #251 implementation record): measured latency accuracy, xrun /
//! device-removal / permission behavior under real stress, and
//! multi-hour stability — those need dedicated hardware sessions.

#![cfg(feature = "real-device")]

use std::io::Write as _;
use std::process;
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use ruviform_audio_core::device::{
    BufferSize, Device, DeviceEnumerator, DeviceError, InputCallbackInfo, InputDataCallback,
    InputErrorCallback, InputStream, InputStreamConfig, OutputCallbackInfo, OutputStreamConfig,
    SupportedInputFormat, SupportedOutputFormat, same_clock_domain,
};
use ruviform_audio_core::{
    AudioBufferView, AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_audio_device::default_host;
use ruviform_media_core::ClockDomainId;

/// How long the input stream captures.
const CAPTURE_SECONDS: Duration = Duration::from_secs(2);
/// Hard bound on total runtime — the watchdog aborts the process if
/// any control-path call (enumerate/open/start/stop) hangs.
const MAX_RUN: Duration = Duration::from_secs(30);
/// Requested fixed buffer size in frames (10 ms at 48 kHz).
const REQUESTED_FRAMES: usize = 480;

/// Data-callback invocations observed.
static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
/// Total frames delivered across all callbacks.
static TOTAL_FRAMES: AtomicU64 = AtomicU64::new(0);
/// Smallest `input.frames()` observed (`usize::MAX` before the first
/// callback).
static MIN_FRAMES: AtomicUsize = AtomicUsize::new(usize::MAX);
/// Largest `input.frames()` observed.
static MAX_FRAMES: AtomicUsize = AtomicUsize::new(0);
/// Callbacks whose channel count differed from the negotiated format.
static CHANNEL_MISMATCHES: AtomicUsize = AtomicUsize::new(0);
/// `MediaTime::value()` (nanoseconds) of the largest callback instant
/// observed so far — monotonicity is checked against it.
static LAST_CALLBACK_NS: AtomicI64 = AtomicI64::new(i64::MIN);
/// Callback instants smaller than a previously observed one.
static TIMESTAMP_REGRESSIONS: AtomicUsize = AtomicUsize::new(0);
/// Callbacks whose `InputCallbackInfo::clock_domain` differed from
/// `stream.clock_domain()`.
static DOMAIN_MISMATCHES: AtomicUsize = AtomicUsize::new(0);
/// Callbacks carrying `capture: Some` / `capture: None`.
static CAPTURE_SOME: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_NONE: AtomicUsize = AtomicUsize::new(0);
/// Callbacks carrying `frame_position: Some` (expected 0 — this
/// adapter reports `None`; a nonzero count is a finding).
static FRAME_POSITION_SOME: AtomicUsize = AtomicUsize::new(0);
/// Non-finite (`NaN`/`±inf`) samples observed across all callbacks.
static NON_FINITE_SAMPLES: AtomicU64 = AtomicU64::new(0);
/// Bits of the largest `|sample|` observed (non-negative `f32` bits
/// order monotonically, so `fetch_max` is exact).
static PEAK_SAMPLE_BITS: AtomicU32 = AtomicU32::new(0);
/// Total error-callback events.
static ERROR_EVENTS: AtomicUsize = AtomicUsize::new(0);
/// `DeviceError::Xrun` events seen on the error callback.
static XRUNS: AtomicUsize = AtomicUsize::new(0);
/// [`error_kind_code`] of the most recent error event (`0` = none).
static LAST_ERROR_KIND: AtomicUsize = AtomicUsize::new(0);
/// The `ClockDomainId` raw value the stream reported before `start()`
/// — every callback's `info.clock_domain` must match it.
static EXPECTED_DOMAIN: AtomicU64 = AtomicU64::new(u64::MAX);
/// Negotiated channel count — every callback's `input.channels()`
/// must match it.
static EXPECTED_CHANNELS: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Spawns the watchdog: if the run has not finished within
/// [`MAX_RUN`], prints a marker and aborts the process — a manual lane
/// run must never hang a session.
fn spawn_watchdog() {
    thread::spawn(|| {
        thread::sleep(MAX_RUN);
        eprintln!("real_input_e2e: watchdog timeout after {MAX_RUN:?} — aborting");
        let _ = std::io::stdout().flush();
        process::exit(3);
    });
}

/// The capture-path verification: open → start → planar callbacks →
/// stop, asserting the delivery contract end to end on real hardware.
///
/// Skips loudly (printed `SKIP`, test passes) only when the host has
/// no usable input device — an environmental absence, not a defect.
/// Once a device exists, every contract violation is a hard failure:
/// this lane exists to *find* problems, not to tolerate them.
#[test]
#[ignore = "real-device lane: requires audio hardware + capture permission (run `mise run test-audio-real-input`)"]
fn real_input_stream_captures_planar_audio() {
    spawn_watchdog();
    println!("real_input_e2e: input capture verification");
    println!("  capture {CAPTURE_SECONDS:?}, watchdog {MAX_RUN:?}");

    let host = match default_host() {
        Ok(host) => host,
        Err(error) => {
            println!("  default_host() failed: {error}");
            return;
        }
    };
    println!("  host: {}", host.name());

    let Some(device) = default_input_device(&host) else {
        return;
    };
    let name = device
        .name()
        .unwrap_or_else(|_| "<name query failed>".to_owned());
    println!("  device: \"{name}\"");

    let Some(format) = negotiate_input_format(device.as_ref()) else {
        println!("  no F32-openable input format — cannot capture");
        return;
    };
    println!("  negotiated format: {format:?}");
    EXPECTED_CHANNELS.store(format.layout.channels() as usize, Ordering::Relaxed);

    let stream = open_input_with_fallback(device.as_ref(), format);

    println!("  stream.format(): {:?}", stream.format());
    match stream.buffer_size() {
        Ok(size) => println!("  stream.buffer_size(): {size:?}"),
        Err(error) => println!("  stream.buffer_size(): query failed: {error}"),
    }
    match stream.now() {
        Ok(now) => println!("  stream.now() pre-start: {} ns", now.value()),
        Err(error) => println!("  stream.now(): query failed: {error}"),
    }
    let domain = stream.clock_domain();
    println!("  stream.clock_domain(): {domain:?} (raw {})", domain.get());
    EXPECTED_DOMAIN.store(domain.get(), Ordering::Relaxed);
    match stream.input_latency() {
        Ok(latency) => println!("  stream.input_latency(): {latency:?}"),
        Err(error) => println!("  stream.input_latency(): query failed: {error}"),
    }

    if let Err(error) = stream.start() {
        panic!("start() failed on a real input device: {error}");
    }
    println!("  capturing {CAPTURE_SECONDS:?}…");
    thread::sleep(CAPTURE_SECONDS);

    if let Err(error) = stream.stop() {
        println!("  stop() failed: {error}");
    }
    match stream.now() {
        Ok(now) => println!("  stream.now() post-stop: {} ns", now.value()),
        Err(error) => println!("  stream.now() post-stop: query failed: {error}"),
    }

    print_capture_summary(domain);

    // The verification contract — anything less is a finding, not a
    // pass. "Open+start but silence" is itself a failed verification.
    assert!(
        CALLBACKS.load(Ordering::Relaxed) > 0,
        "stream started but no input callback observed"
    );
    assert!(
        TOTAL_FRAMES.load(Ordering::Relaxed) > 0,
        "callbacks fired but delivered zero frames"
    );
    assert!(
        MIN_FRAMES.load(Ordering::Relaxed) > 0,
        "a callback delivered an empty view"
    );
    assert_eq!(
        CHANNEL_MISMATCHES.load(Ordering::Relaxed),
        0,
        "callback channel count diverged from the negotiated format"
    );
    assert_eq!(
        DOMAIN_MISMATCHES.load(Ordering::Relaxed),
        0,
        "callback clock_domain diverged from stream.clock_domain()"
    );
    assert_eq!(
        TIMESTAMP_REGRESSIONS.load(Ordering::Relaxed),
        0,
        "callback timestamps went backwards on the stream clock"
    );
    assert_eq!(
        NON_FINITE_SAMPLES.load(Ordering::Relaxed),
        0,
        "backend delivered NaN/inf samples"
    );
}

/// The duplex-domain check: input and output streams opened on the
/// same host must report the *same* `ClockDomainId` — the precondition
/// `same_clock_domain` evaluates. The verdict itself is printed, not
/// asserted: `true` is expected on shared-clock hosts and `false` is
/// the honest answer elsewhere.
///
/// Open-only (no `start`) — `clock_domain` is a stored value, so no
/// audio actually flows. Skips loudly when either direction has no
/// usable device.
#[test]
#[ignore = "real-device lane: requires audio hardware (run `mise run test-audio-real-input`)"]
fn real_duplex_streams_agree_on_clock_domain() {
    println!("real_input_e2e: duplex clock-domain agreement");

    let host = match default_host() {
        Ok(host) => host,
        Err(error) => {
            println!("  default_host() failed: {error}");
            return;
        }
    };
    println!("  host: {}", host.name());

    let Some(input_device) = default_input_device(&host) else {
        return;
    };
    let Some(output_device) = host.default_output_device().ok().flatten() else {
        println!("  SKIP: no default output device — duplex check impossible");
        return;
    };

    let Some(input_format) = negotiate_input_format(input_device.as_ref()) else {
        println!("  SKIP: no F32-openable input format");
        return;
    };
    let Some(output_format) = negotiate_output_format(output_device.as_ref()) else {
        println!("  SKIP: no F32-openable output format");
        return;
    };

    let input = match input_device.open_input_stream(
        &InputStreamConfig {
            format: input_format,
            buffer_size: BufferSize::Default,
        },
        Box::new(|_: AudioBufferView<'_, f32>, _: InputCallbackInfo| {}),
        Box::new(|_: DeviceError| {}),
    ) {
        Ok(stream) => stream,
        Err(error) => panic!("open_input_stream failed: {error}"),
    };
    let output = match output_device.open_output_stream(
        &OutputStreamConfig {
            format: output_format,
            buffer_size: BufferSize::Default,
        },
        Box::new(|_: AudioBufferViewMut<'_, f32>, _: OutputCallbackInfo| {}),
        Box::new(|_: DeviceError| {}),
    ) {
        Ok(stream) => stream,
        Err(error) => panic!("open_output_stream failed: {error}"),
    };

    let in_domain = input.clock_domain();
    let out_domain = output.clock_domain();
    println!(
        "  input.clock_domain():  {in_domain:?} (raw {})",
        in_domain.get()
    );
    println!(
        "  output.clock_domain(): {out_domain:?} (raw {})",
        out_domain.get()
    );
    assert_eq!(
        in_domain, out_domain,
        "input/output streams on one host must report the same clock domain"
    );
    println!(
        "  same_clock_domain(input, output) = {}",
        same_clock_domain(in_domain, out_domain)
    );
    match input.input_latency() {
        Ok(latency) => println!("  input.input_latency(): {latency:?}"),
        Err(error) => println!("  input.input_latency(): query failed: {error}"),
    }
    match output.output_latency() {
        Ok(latency) => println!("  output.output_latency(): {latency:?}"),
        Err(error) => println!("  output.output_latency(): query failed: {error}"),
    }
    // Streams drop unstarted — opening was the whole check.
}

/// Returns the host's default input device, or prints a loud `SKIP`
/// and returns `None` when the host cannot provide one.
fn default_input_device(host: &ruviform_audio_device::Host) -> Option<Box<dyn Device>> {
    match host.default_input_device() {
        Ok(Some(device)) => Some(device),
        Ok(None) => {
            println!("  SKIP: no default input device on this host");
            None
        }
        Err(error) => {
            println!("  SKIP: default_input_device() failed: {error}");
            None
        }
    }
}

/// Input-direction mirror of `play_sine`'s `negotiate_format`: prefer
/// the adapter-negotiated default when it is `F32`; otherwise pick an
/// `F32` range out of the supported space.
fn negotiate_input_format(device: &dyn Device) -> Option<AudioFormat> {
    match device.default_input_format() {
        Ok(format) if format.sample_format == SampleFormat::F32 => Some(format),
        non_f32_or_err => {
            match &non_f32_or_err {
                Ok(format) => println!("  default_input_format is non-F32: {format:?}"),
                Err(error) => println!("  default_input_format() failed: {error}"),
            }
            println!("  falling back to supported_input_formats()");
            let supported = match device.supported_input_formats() {
                Ok(supported) => supported,
                Err(error) => {
                    println!("  supported_input_formats() failed: {error}");
                    return None;
                }
            };
            select_f32_input_format(&supported)
        }
    }
}

/// Output-direction counterpart of [`negotiate_input_format`] —
/// identical rules over `default_output_format` /
/// `supported_output_formats`.
fn negotiate_output_format(device: &dyn Device) -> Option<AudioFormat> {
    match device.default_output_format() {
        Ok(format) if format.sample_format == SampleFormat::F32 => Some(format),
        non_f32_or_err => {
            match &non_f32_or_err {
                Ok(format) => println!("  default_output_format is non-F32: {format:?}"),
                Err(error) => println!("  default_output_format() failed: {error}"),
            }
            println!("  falling back to supported_output_formats()");
            let supported = match device.supported_output_formats() {
                Ok(supported) => supported,
                Err(error) => {
                    println!("  supported_output_formats() failed: {error}");
                    return None;
                }
            };
            select_f32_output_format(&supported)
        }
    }
}

/// Picks an openable `F32` [`AudioFormat`] out of the supported input
/// space — preferring a stereo range, then one containing 48 kHz, and
/// clamping 48 kHz into the picked range.
fn select_f32_input_format(supported: &[SupportedInputFormat]) -> Option<AudioFormat> {
    supported
        .iter()
        .filter(|range| range.sample_format == SampleFormat::F32)
        .filter(|range| {
            range.min_sample_rate <= range.max_sample_rate && range.max_sample_rate != 0
        })
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

/// Output-direction mirror of [`select_f32_input_format`].
fn select_f32_output_format(supported: &[SupportedOutputFormat]) -> Option<AudioFormat> {
    supported
        .iter()
        .filter(|range| range.sample_format == SampleFormat::F32)
        .filter(|range| {
            range.min_sample_rate <= range.max_sample_rate && range.max_sample_rate != 0
        })
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

/// Opens the input stream, first requesting `BufferSize::Fixed(480)`;
/// a `DeviceError::UnsupportedConfig` retry with `BufferSize::Default`
/// separates "fixed size rejected" from "stream cannot open". Every
/// failure is a panic — this lane verifies, it does not tolerate.
fn open_input_with_fallback(device: &dyn Device, format: AudioFormat) -> Box<dyn InputStream> {
    let requested = InputStreamConfig {
        format,
        buffer_size: BufferSize::Fixed(REQUESTED_FRAMES),
    };
    match device.open_input_stream(&requested, data_callback(), error_callback()) {
        Ok(stream) => {
            println!("  open_input_stream: Ok (requested {requested:?})");
            stream
        }
        Err(DeviceError::UnsupportedConfig) => {
            println!("  open Fixed({REQUESTED_FRAMES}) → UnsupportedConfig; retrying with Default");
            let fallback = InputStreamConfig {
                format,
                buffer_size: BufferSize::Default,
            };
            match device.open_input_stream(&fallback, data_callback(), error_callback()) {
                Ok(stream) => {
                    println!("  open_input_stream: Ok (requested {fallback:?})");
                    stream
                }
                Err(error) => {
                    panic!("open_input_stream failed on retry: {error}");
                }
            }
        }
        Err(error) => {
            panic!("open_input_stream failed: {error}");
        }
    }
}

/// The RT data callback — runs on the backend's real-time thread.
/// Records observations through relaxed atomics only: counts, frame
/// bounds, timestamp monotonicity, domain/channel agreement,
/// non-finite sample scan and peak amplitude. No allocation, no I/O,
/// no blocking, no panicking. Bounded per-callback work (one pass
/// over the delivered slab) — acceptable for a manual verification
/// lane.
fn data_callback() -> InputDataCallback {
    Box::new(
        move |input: AudioBufferView<'_, f32>, info: InputCallbackInfo| {
            let frames = input.frames();
            let channels = input.channels();
            CALLBACKS.fetch_add(1, Ordering::Relaxed);
            TOTAL_FRAMES.fetch_add(frames as u64, Ordering::Relaxed);
            MIN_FRAMES.fetch_min(frames, Ordering::Relaxed);
            MAX_FRAMES.fetch_max(frames, Ordering::Relaxed);
            if channels != EXPECTED_CHANNELS.load(Ordering::Relaxed) {
                CHANNEL_MISMATCHES.fetch_add(1, Ordering::Relaxed);
            }

            let callback_ns = info.timestamp.callback.value();
            let previous = LAST_CALLBACK_NS.fetch_max(callback_ns, Ordering::Relaxed);
            if callback_ns < previous {
                TIMESTAMP_REGRESSIONS.fetch_add(1, Ordering::Relaxed);
            }
            if info.clock_domain.get() != EXPECTED_DOMAIN.load(Ordering::Relaxed) {
                DOMAIN_MISMATCHES.fetch_add(1, Ordering::Relaxed);
            }
            match info.timestamp.capture {
                Some(_) => {
                    CAPTURE_SOME.fetch_add(1, Ordering::Relaxed);
                }
                None => {
                    CAPTURE_NONE.fetch_add(1, Ordering::Relaxed);
                }
            }
            if info.frame_position.is_some() {
                FRAME_POSITION_SOME.fetch_add(1, Ordering::Relaxed);
            }

            // One bounded pass over the slab: non-finite scan + peak.
            // `data()` is the channel-major planar slab.
            let mut non_finite = 0_u64;
            let mut peak = 0.0_f32;
            for sample in input.data() {
                if sample.is_finite() {
                    peak = peak.max(sample.abs());
                } else {
                    non_finite += 1;
                }
            }
            NON_FINITE_SAMPLES.fetch_add(non_finite, Ordering::Relaxed);
            // Non-negative `f32` bit patterns order monotonically —
            // `fetch_max` on the bits is an exact atomic max.
            PEAK_SAMPLE_BITS.fetch_max(peak.to_bits(), Ordering::Relaxed);
        },
    )
}

/// The error callback — counts events, records the latest error kind
/// and tallies xruns. [`DeviceError`] is `Copy`, so this path never
/// allocates; matching the error-callback RT-adjacent contract.
fn error_callback() -> InputErrorCallback {
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
/// `DeviceError` is `#[non_exhaustive]` — future variants land on `99`.
fn error_kind_code(error: DeviceError) -> usize {
    match error {
        DeviceError::DeviceNotFound => 1,
        DeviceError::DeviceBusy => 2,
        DeviceError::UnsupportedConfig => 3,
        DeviceError::Unsupported => 4,
        DeviceError::Xrun => 5,
        DeviceError::RealtimeDenied => 6,
        DeviceError::StreamInvalidated => 7,
        DeviceError::ClockDiscontinuity => 8,
        DeviceError::Backend(_) => 9,
        _ => 99,
    }
}

/// Prints the post-`stop()` observation summary from the control
/// thread.
fn print_capture_summary(domain: ClockDomainId) {
    println!("--- summary ---");
    println!("  host clock domain: {domain:?} (raw {})", domain.get());
    println!("  callbacks: {}", CALLBACKS.load(Ordering::Relaxed));
    println!("  total frames: {}", TOTAL_FRAMES.load(Ordering::Relaxed));
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
    println!(
        "  channel mismatches: {}",
        CHANNEL_MISMATCHES.load(Ordering::Relaxed)
    );
    println!(
        "  timestamp regressions: {}",
        TIMESTAMP_REGRESSIONS.load(Ordering::Relaxed)
    );
    println!(
        "  domain mismatches: {}",
        DOMAIN_MISMATCHES.load(Ordering::Relaxed)
    );
    println!(
        "  capture timestamp: some={} none={}",
        CAPTURE_SOME.load(Ordering::Relaxed),
        CAPTURE_NONE.load(Ordering::Relaxed),
    );
    println!(
        "  frame_position reported: {}",
        FRAME_POSITION_SOME.load(Ordering::Relaxed)
    );
    println!(
        "  non-finite samples: {}",
        NON_FINITE_SAMPLES.load(Ordering::Relaxed)
    );
    println!(
        "  peak |sample|: {}",
        f32::from_bits(PEAK_SAMPLE_BITS.load(Ordering::Relaxed))
    );
    println!(
        "  error events: {} (xrun={}, last kind code={})",
        ERROR_EVENTS.load(Ordering::Relaxed),
        XRUNS.load(Ordering::Relaxed),
        LAST_ERROR_KIND.load(Ordering::Relaxed),
    );
}
