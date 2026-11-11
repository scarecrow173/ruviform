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
//! - Latency observation (Issue #251 Stage W closeout): on the
//!   WASAPI-native path `input_latency()` answers `Ok(Some)`
//!   (`IAudioClient::GetStreamLatency`), and the effective input
//!   latency is measured per callback as `callback - capture` —
//!   the values are recorded, not asserted (environment-dependent),
//!   while `capture > callback` (a capture instant after its
//!   delivery instant) is a contract violation that hard-fails.
//! - Format-mismatch rejection: non-F32 / out-of-space configs are
//!   verified against the real device's supported space —
//!   `Err(DeviceError::UnsupportedConfig)` before the backend.
//! - Device removal (interactive, double opt-in): with
//!   `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` set, the removal test
//!   asks the operator — live on the console via `CONOUT$`, so the
//!   prompt survives `cargo test`'s output capture — to physically
//!   unplug the input device, then asserts that
//!   `DeviceError::StreamInvalidated` reaches the error callback.
//!   Without the env var the test prints the manual procedure and
//!   skips (a plain lane run never stalls).
//!
//! What this lane does NOT verify (recorded `NotRun` in the Issue
//! #251 implementation record): permission-denied reproduction
//! (needs OS privacy-settings toggling), a real non-same-domain
//! duplex pair (every WASAPI stream reports `MONOTONIC` — only the
//! contract-level `UNKNOWN` rejection verdict is observable), and
//! multi-hour stability — those need dedicated hardware sessions.
//! Stage M/L (macOS/Linux) remain `NotRun` — this lane records
//! Stage W (Windows+WASAPI) only.

#![cfg(feature = "real-device")]

use std::io::Write as _;
use std::process;
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use ruviform_audio_core::device::{
    BufferSize, Device, DeviceEnumerator, DeviceError, InputCallbackInfo, InputDataCallback,
    InputErrorCallback, InputStream, InputStreamConfig, OutputCallbackInfo, OutputStreamConfig,
    SupportedBufferSize, SupportedInputFormat, SupportedOutputFormat, same_clock_domain,
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
/// Effective input latency statistics — `callback - capture` in
/// nanoseconds across callbacks that carried a `capture` instant
/// (Issue #251 Stage W: the observed buffering-path latency,
/// recorded not asserted). `i64::MAX` before the first sample.
static LATENCY_MIN_NS: AtomicI64 = AtomicI64::new(i64::MAX);
static LATENCY_MAX_NS: AtomicI64 = AtomicI64::new(i64::MIN);
static LATENCY_SUM_NS: AtomicI64 = AtomicI64::new(0);
/// Callbacks whose `capture` instant was *after* the `callback`
/// instant — an ordering violation (the capture instant must
/// precede its delivery instant; a reversal means the clock
/// reported garbage, not a fast pipeline).
static CAPTURE_ORDER_VIOLATIONS: AtomicUsize = AtomicUsize::new(0);
/// Extremes among violating `capture` instants (nanoseconds) —
/// recorded to characterise the bogus timebase the backend reports.
static CAPTURE_VIOL_MIN_INSTANT_NS: AtomicI64 = AtomicI64::new(i64::MAX);
static CAPTURE_VIOL_MAX_INSTANT_NS: AtomicI64 = AtomicI64::new(i64::MIN);
/// Per-callback `callback - capture` deltas (first 256) — the
/// violation *shape* distinguishes a bogus first-packet read from a
/// drifting timebase.
static DELTA_TRACE: [AtomicI64; 256] = [const { AtomicI64::new(i64::MAX) }; 256];
/// Callbacks carrying `frame_position: Some`. On the Windows
/// WASAPI-native input path (Issue #565) every callback must carry
/// the backend-reported device position; backends that cannot report
/// one honestly keep `None`.
static FRAME_POSITION_SOME: AtomicUsize = AtomicUsize::new(0);
/// `frame_position` values that did not continue the previous
/// packet's `position + frames` expectation. Each must coincide with
/// a `ClockDiscontinuity` error event — a jump the backend did not
/// report is a contract violation.
static FRAME_POSITION_GAPS: AtomicUsize = AtomicUsize::new(0);
/// First `frame_position` observed (`u64::MAX` before the first
/// `Some`) — printed as progression evidence.
static FIRST_FRAME_POSITION: AtomicU64 = AtomicU64::new(u64::MAX);
/// Largest `frame_position` observed.
static LAST_FRAME_POSITION: AtomicU64 = AtomicU64::new(0);
/// The `frame_position` expected for the next callback —
/// `position + frames` of the previous `Some` packet (`u64::MAX`
/// before the first `Some` anchors the epoch; WASAPI callbacks are
/// serial per stream so a plain load/store sequence suffices).
static EXPECTED_FRAME_POSITION: AtomicU64 = AtomicU64::new(u64::MAX);
/// Non-finite (`NaN`/`±inf`) samples observed across all callbacks.
static NON_FINITE_SAMPLES: AtomicU64 = AtomicU64::new(0);
/// Bits of the largest `|sample|` observed (non-negative `f32` bits
/// order monotonically, so `fetch_max` is exact).
static PEAK_SAMPLE_BITS: AtomicU32 = AtomicU32::new(0);
/// Total error-callback events.
static ERROR_EVENTS: AtomicUsize = AtomicUsize::new(0);
/// `DeviceError::Xrun` events seen on the error callback.
static XRUNS: AtomicUsize = AtomicUsize::new(0);
/// `DeviceError::ClockDiscontinuity` events seen on the error
/// callback — the honest signal a `frame_position` epoch broke.
static CLOCK_DISCONTINUITIES: AtomicUsize = AtomicUsize::new(0);
/// `DeviceError::StreamInvalidated` events seen on the error
/// callback — the typed removal/invalidation notification the
/// interactive removal test waits for.
static STREAM_INVALIDATED: AtomicUsize = AtomicUsize::new(0);
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
    // Issue #565: the Windows WASAPI-native input path must report
    // `frame_position` on every callback; other hosts/backends keep
    // `None` honestly, so the assertions below are host-gated.
    let is_wasapi = host.name() == "WASAPI";

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
    print_stream_queries(stream.as_ref(), "pre-start");
    let domain = stream.clock_domain();
    println!("  stream.clock_domain(): {domain:?} (raw {})", domain.get());
    EXPECTED_DOMAIN.store(domain.get(), Ordering::Relaxed);
    report_input_latency(stream.as_ref(), is_wasapi, format.sample_rate.get());

    if let Err(error) = stream.start() {
        panic!("start() failed on a real input device: {error}");
    }
    println!("  capturing {CAPTURE_SECONDS:?}…");
    thread::sleep(CAPTURE_SECONDS);

    if let Err(error) = stream.stop() {
        println!("  stop() failed: {error}");
    }
    print_stream_queries(stream.as_ref(), "post-stop");

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
    assert_eq!(
        CAPTURE_ORDER_VIOLATIONS.load(Ordering::Relaxed),
        0,
        "a capture instant came after its callback instant — the device clock reported garbage"
    );
    if is_wasapi {
        // Every WASAPI callback must carry the backend-reported
        // device position — a `None` would mean the native path was
        // bypassed or the position was dropped.
        assert_eq!(
            FRAME_POSITION_SOME.load(Ordering::Relaxed),
            CALLBACKS.load(Ordering::Relaxed),
            "WASAPI input must report frame_position on every callback"
        );
        // A position jump is only legitimate when the stream also
        // reported `ClockDiscontinuity` — an unreported jump would
        // mean the epoch broke silently.
        assert!(
            FRAME_POSITION_GAPS.load(Ordering::Relaxed)
                <= CLOCK_DISCONTINUITIES.load(Ordering::Relaxed),
            "frame_position jumped without a ClockDiscontinuity report"
        );
    }
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
    // Issue #251 Stage W: the non-same-domain rejection/fallback
    // verdict. A real non-same pair cannot be constructed on this
    // hardware — every stream a shared-clock host opens reports the
    // same domain (`MONOTONIC` on WASAPI) — so the contract-level
    // probe is what the lane can honestly observe: `UNKNOWN` never
    // same-clocks, not even against itself, which is what makes
    // duplex correlation *fall back* on per-device/unaudited hosts
    // instead of risking a false same-clock claim.
    assert!(
        !same_clock_domain(in_domain, ClockDomainId::UNKNOWN),
        "a real domain must not same-clock with UNKNOWN"
    );
    println!(
        "  same_clock_domain({in_domain:?}, UNKNOWN) = false (rejection/fallback verdict observed)"
    );
    assert!(
        !same_clock_domain(ClockDomainId::UNKNOWN, ClockDomainId::UNKNOWN),
        "UNKNOWN == UNKNOWN must not claim same-clock"
    );
    println!("  same_clock_domain(UNKNOWN, UNKNOWN) = false (UNKNOWN never same-clocks)");
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

/// The format-mismatch verification: configs the port's negotiation
/// must refuse are opened against the real device, and every one must
/// come back `Err(DeviceError::UnsupportedConfig)` — rejected inside
/// the port (before the backend), never silently converted.
///
/// Rows are computed from the device's own `supported_input_formats`
/// space so the assertions hold on any hardware: a non-F32 sample
/// format, a sample rate above/below every supported range, a channel
/// layout no range covers (when one exists), `BufferSize::Fixed(0)`,
/// and — only when every matching candidate reports a bounded size —
/// a `Fixed` request above the largest bound.
#[test]
#[ignore = "real-device lane: requires audio hardware (run `mise run test-audio-real-input`)"]
fn real_input_open_rejects_unsupported_configs() {
    println!("real_input_e2e: unsupported-config rejection surface");

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

    let supported = match device.supported_input_formats() {
        Ok(supported) => supported,
        Err(error) => panic!("supported_input_formats() failed: {error}"),
    };
    let Some(format) = negotiate_input_format(device.as_ref()) else {
        println!("  SKIP: no F32-openable input format — rejection rows need an openable baseline");
        return;
    };
    println!("  baseline format: {format:?}");

    let mut checked = 0_u32;
    let mut check = |label: &str, format: AudioFormat, buffer_size: BufferSize| {
        expect_rejected(
            device.as_ref(),
            label,
            &InputStreamConfig {
                format,
                buffer_size,
            },
        );
        checked += 1;
    };

    // Non-F32 sample format — refused before any backend call (the
    // adapter opens F32 only and never silently converts).
    for sample_format in [SampleFormat::S16, SampleFormat::F64] {
        check(
            &format!("non-F32 {sample_format:?}"),
            AudioFormat {
                sample_format,
                ..format
            },
            BufferSize::Default,
        );
    }

    // Sample rates outside every F32-capable range. Both directions
    // are checked against the device's own reported bounds.
    if let Some((min_rate, max_rate)) = f32_rate_bounds(&supported) {
        let above = max_rate.saturating_mul(2).max(max_rate + 1);
        check(
            &format!("rate {above} Hz > max {max_rate} Hz"),
            AudioFormat {
                sample_rate: SampleRate::new(above).unwrap(),
                ..format
            },
            BufferSize::Default,
        );
        if min_rate > 1 {
            check(
                &format!("rate 1 Hz < min {min_rate} Hz"),
                AudioFormat {
                    sample_rate: SampleRate::new(1).unwrap(),
                    ..format
                },
                BufferSize::Default,
            );
        }
    } else {
        println!("  no F32 ranges reported — rate-bounds rows skipped");
    }

    // A channel layout no range covers (e.g. `SevenPointOne` on a
    // 2-channel device) — refused at negotiation.
    if let Some(uncovered) = uncovered_layout(&supported) {
        check(
            &format!("layout {uncovered:?} in no supported range"),
            AudioFormat {
                layout: uncovered,
                ..format
            },
            BufferSize::Default,
        );
    } else {
        println!("  every named layout covered — layout row skipped");
    }

    // `BufferSize::Fixed(0)` — degenerate request, refused at
    // negotiation regardless of the device's reported bounds.
    check("BufferSize::Fixed(0)", format, BufferSize::Fixed(0));

    // `Fixed` above every reported bound — only verifiable when all
    // matching candidates are bounded (`Unknown` admits any size, so
    // the row is skipped then, not failed).
    if let Some(bound) = largest_buffer_bound(&supported, &format) {
        let over = bound.saturating_add(480);
        check(
            &format!("BufferSize::Fixed({over}) > bound {bound}"),
            format,
            BufferSize::Fixed(over),
        );
    } else {
        println!("  unbounded or Unknown buffer bound reported — Fixed-over-bound row skipped");
    }

    assert!(checked >= 4, "too few rejection rows verifiable: {checked}");
    println!("  {checked} rejection rows verified — all Err(UnsupportedConfig)");
}

/// Opens `config` against `device` and asserts the answer is exactly
/// `Err(DeviceError::UnsupportedConfig)` — the port's negotiation
/// refuses, before the backend, never silently converts.
fn expect_rejected(device: &dyn Device, label: &str, config: &InputStreamConfig) {
    match device.open_input_stream(config, Box::new(|_, _| {}), Box::new(|_| {})) {
        Err(DeviceError::UnsupportedConfig) => {
            println!("  {label}: Err(UnsupportedConfig) — OK");
        }
        Err(error) => panic!("{label}: expected Err(UnsupportedConfig), got Err({error:?})"),
        Ok(_) => panic!(
            "{label}: expected Err(UnsupportedConfig), got Ok(stream) — the port accepted a config outside its own supported space"
        ),
    }
}

/// The `(min, max)` sample-rate envelope across all F32-capable
/// input ranges — `None` when the device reports no F32 range.
fn f32_rate_bounds(supported: &[SupportedInputFormat]) -> Option<(u32, u32)> {
    supported
        .iter()
        .filter(|range| range.sample_format == SampleFormat::F32)
        .filter(|range| range.min_sample_rate <= range.max_sample_rate)
        .fold(None, |bounds, range| {
            Some(match bounds {
                None => (range.min_sample_rate, range.max_sample_rate),
                Some((lo, hi)) => (lo.min(range.min_sample_rate), hi.max(range.max_sample_rate)),
            })
        })
}

/// The first named layout whose channel count appears in no
/// supported range — `None` when every named layout is covered.
fn uncovered_layout(supported: &[SupportedInputFormat]) -> Option<ChannelLayout> {
    [
        ChannelLayout::Mono,
        ChannelLayout::Stereo,
        ChannelLayout::Quad,
        ChannelLayout::FivePointOne,
        ChannelLayout::SevenPointOne,
    ]
    .into_iter()
    .find(|layout| {
        !supported
            .iter()
            .any(|range| range.channels == layout.channels())
    })
}

/// The largest buffer bound across the ranges matching `format` —
/// `Some` only when *every* matching candidate reports a bounded
/// size (a single `Unknown` bound admits any `Fixed` request, so the
/// rejection row is unverifiable and reports `None`).
fn largest_buffer_bound(supported: &[SupportedInputFormat], format: &AudioFormat) -> Option<usize> {
    let rate = format.sample_rate.get();
    let mut largest = 0_usize;
    let mut matched = false;
    for range in supported.iter().filter(|range| {
        range.sample_format == SampleFormat::F32
            && range.channels == format.layout.channels()
            && range.min_sample_rate <= rate
            && rate <= range.max_sample_rate
    }) {
        match range.buffer_size {
            SupportedBufferSize::Range { max, .. } => {
                matched = true;
                largest = largest.max(max);
            }
            _ => return None,
        }
    }
    matched.then_some(largest)
}

/// The device-removal notification check — **interactive, doubly
/// opt-in**. It exists to observe what the port reports when the
/// capture device physically disappears mid-stream; that cannot be
/// fully automated (unplugging hardware is a human act), so the test
/// only runs when an operator arms it with
/// `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` and unplugs on request.
///
/// Armed run:
///   1. Opens + starts the default input stream and confirms live
///      callbacks for a short baseline.
///   2. Asks the operator — via `CONOUT$`, visible even under
///      `cargo test`'s output capture — to unplug the device or
///      switch the default capture endpoint away from it (both
///      invalidate the stream; the second is scriptable via Sound >
///      Recording on machines where a physical unplug is not).
///   3. Observes the error callback for the window: WASAPI reports
///      removal as `DeviceError::StreamInvalidated` (either the
///      `AUDCLNT_E_DEVICE_INVALIDATED` path or the default-device
///      monitor's `OnDeviceRemoved`/state-change path — both
///      normalize to the same typed variant). Its arrival is the
///      assertion; accompanying `ClockDiscontinuity`/`Xrun` events
///      are recorded, not required.
///   4. Post-removal control-path queries are recorded, not
///      asserted: `stop()` legitimately races the worker's exit
///      (`Ok` or `Err(StreamInvalidated)`), while `now()` is a pure
///      QPC read expected to keep answering `Ok`.
///
/// Not armed: prints the manual procedure and passes — a plain
/// `mise run test-audio-real-input` never stalls here (the matrix
/// records this row `NotRun` then).
#[test]
#[ignore = "real-device lane, interactive: unplugs the device under test — arm with RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS=<seconds>"]
fn real_input_device_removal_notifies_typed_error() {
    let Ok(armed) = std::env::var("RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS") else {
        println!("real_input_e2e: device-removal check — SKIP (not armed)");
        println!("  Manual procedure for an attended run:");
        println!("    1. `set RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS=60`");
        println!(
            "    2. `mise run test-audio-real-input` (or `cargo test -p ruviform-audio-device --features real-device --test real_input_e2e -- --ignored --test-threads=1`)"
        );
        println!("    3. Unplug the device — or set another endpoint as the default");
        println!("       capture device (Sound > Recording) — when the prompt appears;");
        println!("       the test asserts `StreamInvalidated` reaches the error callback.");
        return;
    };
    let window_secs = armed.parse::<u64>().unwrap_or(60);
    let window = Duration::from_secs(window_secs);
    spawn_watchdog();
    println!("real_input_e2e: device-removal notification check (interactive)");
    println!("  observation window: {window:?}");

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
    EXPECTED_CHANNELS.store(format.layout.channels() as usize, Ordering::Relaxed);

    let stream = open_input_with_fallback(device.as_ref(), format);
    let domain = stream.clock_domain();
    EXPECTED_DOMAIN.store(domain.get(), Ordering::Relaxed);

    if let Err(error) = stream.start() {
        panic!("start() failed on a real input device: {error}");
    }
    println!("  baseline capture 1 s…");
    // The counters are cumulative across the whole test binary, so
    // this test works in deltas from its own snapshot.
    let callbacks_before = CALLBACKS.load(Ordering::Relaxed);
    let invalidated_before = STREAM_INVALIDATED.load(Ordering::Relaxed);
    let events_before = ERROR_EVENTS.load(Ordering::Relaxed);
    thread::sleep(Duration::from_secs(1));
    let baseline = CALLBACKS.load(Ordering::Relaxed) - callbacks_before;
    assert!(
        baseline > 0,
        "stream started but no input callback observed before removal"
    );
    println!("  baseline: {baseline} callbacks");

    interactive_print(&format!(
        "\n>>> UNPLUG \"{name}\" — or switch the default capture device away from it (Sound > Recording) <<<\n>>> observing the error callback for {window_secs} s — replug/restore afterwards <<<\n"
    ));
    let deadline = std::time::Instant::now() + window;
    while std::time::Instant::now() < deadline {
        if STREAM_INVALIDATED.load(Ordering::Relaxed) > invalidated_before {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let invalidated = STREAM_INVALIDATED.load(Ordering::Relaxed) - invalidated_before;
    let events = ERROR_EVENTS.load(Ordering::Relaxed) - events_before;
    println!("--- removal observation ---");
    println!(
        "  error events: {} (StreamInvalidated={invalidated}, xrun={}, discontinuities={}, last kind code={})",
        events,
        XRUNS.load(Ordering::Relaxed),
        CLOCK_DISCONTINUITIES.load(Ordering::Relaxed),
        LAST_ERROR_KIND.load(Ordering::Relaxed),
    );
    let stop_result = match stream.stop() {
        Ok(()) => "Ok".to_owned(),
        Err(error) => format!("Err({error:?})"),
    };
    let now_result = match stream.now() {
        Ok(time) => format!("Ok({} ns)", time.value()),
        Err(error) => format!("Err({error:?})"),
    };
    let buffer_result = match stream.buffer_size() {
        Ok(size) => format!("Ok({size:?})"),
        Err(error) => format!("Err({error:?})"),
    };
    println!(
        "  post-removal control path: stop()={stop_result} now()={now_result} buffer_size()={buffer_result}"
    );
    interactive_print(">>> observation closed — you may replug the device <<<\n");

    if invalidated == 0 {
        // An unattended window is "not exercised", never a failure —
        // and stray events without `StreamInvalidated` are recorded,
        // not asserted on: the contract requires `StreamInvalidated`
        // *when the device actually left*, and the test cannot tell
        // an unperformed unplug from a silently missed one. If the
        // operator DID unplug and this still reports zero, that is a
        // finding to write into the record by hand.
        println!(
            "  NOT-EXERCISED: no StreamInvalidated observed in the window (error events={events}) — record as unexercised, not pass"
        );
        return;
    }
    println!("  StreamInvalidated arrived on the error callback — removal notification verified");
}

/// Writes a line to the real console even when `cargo test` captures
/// stdout — `CONOUT$` bypasses the harness's pipe on Windows; other
/// platforms and console-less sessions fall back to stderr (which
/// libtest only buffers per-test, but the operator is told the prompt
/// exists by the armed-run command line anyway).
fn interactive_print(message: &str) {
    if let Ok(mut console) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
        let _ = console.write_all(message.as_bytes());
        let _ = console.flush();
    }
    eprintln!("{message}");
}

/// Prints the fallible `buffer_size()`/`now()` control-path queries
/// for the record — a failed query is reported, not asserted
/// (backend-dependent, honest `Err` is a legitimate answer).
fn print_stream_queries(stream: &dyn InputStream, phase: &str) {
    match stream.buffer_size() {
        Ok(size) => println!("  stream.buffer_size() {phase}: {size:?}"),
        Err(error) => println!("  stream.buffer_size() {phase}: query failed: {error}"),
    }
    match stream.now() {
        Ok(now) => println!("  stream.now() {phase}: {} ns", now.value()),
        Err(error) => println!("  stream.now() {phase}: query failed: {error}"),
    }
}

/// Prints the stream's `input_latency()` answer for the record — and
/// on the WASAPI-native path *asserts* the Issue #565 contract: the
/// backend-reported `IAudioClient::GetStreamLatency` answer must be
/// `Ok(Some(StreamLatency))` (`Ok(None)` would mean the native path
/// silently degraded to the cpal surface; a fabricated
/// constant would be a lie the `Some` assert cannot see — only the
/// *backend-reported* value is honest, however small).
fn report_input_latency(stream: &dyn InputStream, is_wasapi: bool, sample_rate: u32) {
    match stream.input_latency() {
        Ok(latency) => {
            println!("  stream.input_latency(): {latency:?}");
            if is_wasapi {
                let Some(latency) = latency else {
                    panic!("WASAPI input must report a real StreamLatency (Ok(Some))");
                };
                // Integer µs — honest math, no float precision loss.
                let micros = latency.frames.saturating_mul(1_000_000) / u64::from(sample_rate);
                println!(
                    "  input latency: {} frames ({micros} µs @ {sample_rate} Hz), coverage {:?}, provenance {:?}",
                    latency.frames, latency.coverage, latency.provenance,
                );
            }
        }
        Err(error) => println!("  stream.input_latency(): query failed: {error}"),
    }
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
                Some(capture) => {
                    CAPTURE_SOME.fetch_add(1, Ordering::Relaxed);
                    // `callback - capture` = the effective input
                    // latency (buffering path) for this block. The
                    // magnitude is environment-dependent — recorded
                    // not asserted. A negative delta is an ordering
                    // violation: the capture instant must precede
                    // its delivery instant.
                    let delta_ns = callback_ns - capture.time.value();
                    LATENCY_MIN_NS.fetch_min(delta_ns, Ordering::Relaxed);
                    LATENCY_MAX_NS.fetch_max(delta_ns, Ordering::Relaxed);
                    // Summed clamped to ±1000 s — the mean is
                    // diagnostic; an unclamped bogus instant would
                    // wrap i64 and poison the record.
                    LATENCY_SUM_NS.fetch_add(
                        delta_ns.clamp(-1_000_000_000_000, 1_000_000_000_000),
                        Ordering::Relaxed,
                    );
                    if delta_ns < 0 {
                        CAPTURE_ORDER_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
                        let instant_ns = capture.time.value();
                        CAPTURE_VIOL_MIN_INSTANT_NS.fetch_min(instant_ns, Ordering::Relaxed);
                        CAPTURE_VIOL_MAX_INSTANT_NS.fetch_max(instant_ns, Ordering::Relaxed);
                    }
                    let seq = CAPTURE_SOME.load(Ordering::Relaxed) - 1;
                    if seq < DELTA_TRACE.len() {
                        DELTA_TRACE[seq].store(delta_ns, Ordering::Relaxed);
                    }
                }
                None => {
                    CAPTURE_NONE.fetch_add(1, Ordering::Relaxed);
                }
            }
            if let Some(position) = info.frame_position {
                FRAME_POSITION_SOME.fetch_add(1, Ordering::Relaxed);
                // The next callback's first-frame position must equal
                // this packet's `position + frames` — WASAPI delivers
                // contiguous capture-clock spans unless a discontinuity
                // was reported. Callbacks are serial per stream.
                let expected = EXPECTED_FRAME_POSITION.load(Ordering::Relaxed);
                if expected != u64::MAX && position != expected {
                    FRAME_POSITION_GAPS.fetch_add(1, Ordering::Relaxed);
                }
                EXPECTED_FRAME_POSITION
                    .store(position.wrapping_add(frames as u64), Ordering::Relaxed);
                if FIRST_FRAME_POSITION.load(Ordering::Relaxed) == u64::MAX {
                    FIRST_FRAME_POSITION.store(position, Ordering::Relaxed);
                }
                LAST_FRAME_POSITION.fetch_max(position, Ordering::Relaxed);
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
        if error == DeviceError::ClockDiscontinuity {
            CLOCK_DISCONTINUITIES.fetch_add(1, Ordering::Relaxed);
        }
        if error == DeviceError::StreamInvalidated {
            STREAM_INVALIDATED.fetch_add(1, Ordering::Relaxed);
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
    let capture_some = CAPTURE_SOME.load(Ordering::Relaxed);
    println!(
        "  capture timestamp: some={} none={}",
        capture_some,
        CAPTURE_NONE.load(Ordering::Relaxed),
    );
    let latency_min = LATENCY_MIN_NS.load(Ordering::Relaxed);
    println!(
        "  effective input latency (callback - capture): min={} ns max={} ns mean={} ns ({} samples, order violations={})",
        if latency_min == i64::MAX {
            "n/a".to_owned()
        } else {
            latency_min.to_string()
        },
        if capture_some == 0 {
            "n/a".to_owned()
        } else {
            LATENCY_MAX_NS.load(Ordering::Relaxed).to_string()
        },
        if capture_some == 0 {
            "n/a".to_owned()
        } else {
            (LATENCY_SUM_NS.load(Ordering::Relaxed)
                / i64::try_from(capture_some).unwrap_or(i64::MAX))
            .to_string()
        },
        capture_some,
        CAPTURE_ORDER_VIOLATIONS.load(Ordering::Relaxed),
    );
    if CAPTURE_ORDER_VIOLATIONS.load(Ordering::Relaxed) > 0 {
        println!(
            "  violating capture instants: min={} ns max={} ns",
            CAPTURE_VIOL_MIN_INSTANT_NS.load(Ordering::Relaxed),
            CAPTURE_VIOL_MAX_INSTANT_NS.load(Ordering::Relaxed),
        );
        let trace = DELTA_TRACE
            .iter()
            .take_while(|slot| slot.load(Ordering::Relaxed) != i64::MAX)
            .map(|slot| slot.load(Ordering::Relaxed).to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!("  delta trace (ns): {trace}");
    }
    let first_position = FIRST_FRAME_POSITION.load(Ordering::Relaxed);
    println!(
        "  frame_position: some={} gaps={} first={} last={}",
        FRAME_POSITION_SOME.load(Ordering::Relaxed),
        FRAME_POSITION_GAPS.load(Ordering::Relaxed),
        if first_position == u64::MAX {
            "none".to_owned()
        } else {
            first_position.to_string()
        },
        LAST_FRAME_POSITION.load(Ordering::Relaxed),
    );
    println!(
        "  clock discontinuities: {}",
        CLOCK_DISCONTINUITIES.load(Ordering::Relaxed)
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
