//! AC3 Test Runtime wiring — Issue #629 slice.
//!
//! Connects the deterministic Test Runtime (`ruviform-audio-test`)
//! to the production device boundary (`ruviform-audio-device` /
//! `ruviform-midi-device`) and persists every case verdict as a
//! Case Schema v1 [`RunMetadata`] JSON record. Two legs:
//!
//! - **Deterministic legs** (always run under
//!   `cargo test -p ruviform-audio-test`): `PcmFixture` →
//!   `oracle::pcm_reference_block` and `MidiScenario` →
//!   `run_midi_trace` → `oracle::midi_expected_playthrough`.
//!   These prove the typed "supply → production port contract →
//!   independent oracle → `RunMetadata`" path at
//!   `environment_kind=deterministic-backend`, `level=os-l1` —
//!   OS-L1 evidence only, never a real-OS claim.
//! - **Virtual OS endpoint legs** (`#[ignore]`, manual opt-in —
//!   canonical lane: `mise run test-os-endpoint-wiring`): drive
//!   the real `DeviceEnumerator`/`Device` streams and
//!   MIDI ports. The Test Runtime supplies the stimulus —
//!   `PcmFixture::fill_view` fills the production
//!   `AudioBufferViewMut` inside the output callback, and
//!   `run_midi_trace` entries supply the event list sent through the
//!   output port. Observed side is checked by independent
//!   contract-level oracles (marker spacing, RMS band, callback
//!   contract, ordered MIDI subsequence + stamp contract).
//!
//! Fail-closed contract (mirrors the device verifiers): unprovisioned
//! hosts record `result=not-run` and return without asserting — a
//! name match is a *candidate*, never proof of a virtual endpoint,
//! and only a run that actually opened and streamed endpoints may
//! claim `environment_kind=virtual-os-endpoint`.
//!
//! Every terminal state emits a `run_metadata:` JSON line on stderr
//! (capture with `--nocapture`) and, when `CARGO_TARGET_TMPDIR` is
//! set (integration tests), a `run-metadata-<case>.json` file beside
//! the test binary's temp dir. `RUVIFORM_RUN_REPO`/`RUVIFORM_RUN_SHA`
//! override the recorded repo/sha; unset values record `"unknown"`.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ruviform_audio_core::device::{
    BufferSize, Device, DeviceEnumerator, DeviceError, InputCallbackInfo, InputDataCallback,
    InputErrorCallback, InputStream, InputStreamConfig, OutputCallbackInfo, OutputDataCallback,
    OutputErrorCallback, OutputStream, OutputStreamConfig, SupportedInputFormat,
    SupportedOutputFormat,
};
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputInfo, MidiInputPort, TimedMidiInput,
};
use ruviform_audio_core::midi_output::{MidiOutput, MidiOutputEnumerator, MidiOutputPort};
use ruviform_audio_core::{
    AudioBufferView, AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_audio_device::{Host, default_host};
use ruviform_audio_test::clock::{JitterSpec, VirtualClockConfig};
use ruviform_audio_test::metadata::{
    CaseRequirement, CaseResult, EnvironmentKind, RunMetadata, RunParameters, VerificationLevel,
};
use ruviform_audio_test::midi::{MidiCue, MidiScenario, MidiTraceConfig, run_midi_trace};
use ruviform_audio_test::oracle::{
    PcmCompare, midi_expected_playthrough, midi_trace_eq, midi_trace_frames_monotonic,
    pcm_assert_eq, pcm_reference_block,
};
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern};
use ruviform_media_core::{
    BlockContract, ClockDomainId, DeliveryPolicy, InBlockTransportSupport, MediaTime, MidiChannel,
    MidiController, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey, ScheduleBudget, TimeBase,
    TimestampProvenance,
};
use ruviform_midi_device::backend;

// ---------------------------------------------------------------------------
// Shared record helpers
// ---------------------------------------------------------------------------

/// Scenario seed recorded in every run's `RunParameters`.
const RUN_SEED: u64 = 0x629;
/// Fixture/scenario format version for this wiring slice.
const FIXTURE_VERSION: u32 = 1;
/// How long each audio pair streams the supplied stimulus.
const CAPTURE_SECONDS: u64 = 3;
/// Requested fixed buffer size in frames (10 ms at 48 kHz).
const REQUESTED_FRAMES: usize = 480;
/// Hard bound on the audio leg's total runtime.
const MAX_RUN: Duration = Duration::from_secs(60);
/// Per-epoch startup grace window for the timestamp-regression
/// oracle, in callbacks. Shared-mode render streams re-anchor their
/// position↔QPC correlation at every `start()`, so regressions inside
/// this bounded window of an epoch start are OS startup transients;
/// deeper regressions stay fatal. Matches the rf#977 device-verifier
/// contract (`EPOCH_START_GRACE_CALLBACKS`).
const EPOCH_START_GRACE_CALLBACKS: u64 = 24;
/// Bound on the per-direction regression-index log used to classify
/// epoch transients; raw regressions past this bound cannot be
/// classified and stay fatal.
const REGRESSION_INDEX_SLOTS: usize = 32;
/// Bound on the per-callback render timestamp observation log.
const RENDER_TIMESTAMP_LOG_SLOTS: usize = 512;
/// Bound on the control-thread stream lifecycle context log. Lifecycle
/// events are few by design; overflow means the run cannot be used for
/// restart/mid-stream attribution without saying so.
const STREAM_LIFECYCLE_LOG_SLOTS: usize = 32;
/// Bound on the input/output error context log. Error callbacks can be
/// noisy, so the artifact keeps a small fixed window plus an overflow
/// count instead of allocating or logging without bound on the RT path.
const STREAM_ERROR_LOG_SLOTS: usize = 16;
/// Case id tagged on round-trip output timestamp diagnostics.
const ROUNDTRIP_AUDIO_CASE_ID: &str = "rf629.audio.virtual-endpoint.render-capture";
/// Timestamp-path tag distinguishing round-trip output rows from the
/// render-only baseline rows, whose legacy line format stays unchanged.
const ROUNDTRIP_OUTPUT_PATH: &str = "roundtrip_output";

/// Impulse-train stimulus supplied through `PcmFixture::fill_view`:
/// one 0.8 impulse every 100 ms of render time. The independent
/// oracle detects `|x| > 0.5` peaks with a refractory window and
/// checks count + mean interval + RMS band — SRC-tolerant, never
/// bit-exact (virtual cables run an internal sample-rate converter).
const IMPULSE_AMPLITUDE: f32 = 0.8;
const IMPULSE_PERIOD_MS: u64 = 100;
const MARKER_THRESHOLD: f32 = 0.5;
const MARKER_MIN_COUNT: usize = 3;
const MARKER_INTERVAL_TOLERANCE: f64 = 0.10;
const MARKER_REFRACTORY_MS: u64 = 20;
/// Provisional observed/expected RMS band — to be tightened from
/// provisioned-runner measurements (same discipline as the device
/// verifier's `MIN_LEVEL_RATIO`/`MAX_LEVEL_RATIO`).
const MIN_LEVEL_RATIO: f64 = 0.2;
const MAX_LEVEL_RATIO: f64 = 3.0;

/// `RunParameters` for a run at `sample_rate` — deterministic clock,
/// no jitter/drift, recorded as the declared reproducibility input.
const fn run_parameters(sample_rate: u32) -> RunParameters {
    RunParameters {
        seed: RUN_SEED,
        fixture_version: FIXTURE_VERSION,
        clock_config: VirtualClockConfig {
            sample_rate,
            epoch: 0,
            offset_ticks: 0,
            drift_ppm: 0,
            jitter: JitterSpec::DISABLED,
        },
    }
}

/// Repository identity — `RUVIFORM_RUN_REPO` override, else
/// `"unknown"` (honest: the test crate does not assume a remote).
fn repo_name() -> String {
    std::env::var("RUVIFORM_RUN_REPO").unwrap_or_else(|_| "unknown".to_owned())
}

/// Source revision — `RUVIFORM_RUN_SHA` → `CI_COMMIT_SHA` →
/// `GITHUB_SHA` → `"unknown"`.
fn repo_sha() -> String {
    std::env::var("RUVIFORM_RUN_SHA")
        .or_else(|_| std::env::var("CI_COMMIT_SHA"))
        .or_else(|_| std::env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// A `virtual-os-endpoint`/`ext-l3` case record at an explicit
/// negotiated `sample_rate` — the fixture/supply rate, so recorded
/// `RunParameters` match what actually ran.
fn endpoint_record_at(case_id: &str, oracle: &str, sample_rate: u32) -> RunMetadata {
    RunMetadata::for_environment(
        case_id,
        CaseRequirement::Required,
        VerificationLevel::ExtL3,
        EnvironmentKind::VirtualOsEndpoint,
        oracle,
        run_parameters(sample_rate),
        repo_name(),
        repo_sha(),
    )
    .with_platform(std::env::consts::OS, std::env::consts::ARCH)
}

/// A `virtual-os-endpoint`/`ext-l3` case record — starts `NotRun`;
/// the caller marks the actual outcome. OS/arch are filled here.
/// `sample_rate` is the *requested* rate (48 kHz preferred); the
/// Pass record is rebuilt at the negotiated rate by the caller.
fn endpoint_record(case_id: &str, oracle: &str) -> RunMetadata {
    endpoint_record_at(case_id, oracle, 48_000)
}

/// A `deterministic-backend`/`os-l1` case record at `sample_rate`.
fn deterministic_record(case_id: &str, oracle: &str, sample_rate: u32) -> RunMetadata {
    RunMetadata::for_environment(
        case_id,
        CaseRequirement::Required,
        VerificationLevel::OsL1,
        EnvironmentKind::DeterministicBackend,
        oracle,
        run_parameters(sample_rate),
        repo_name(),
        repo_sha(),
    )
    .with_platform(std::env::consts::OS, std::env::consts::ARCH)
}

/// Emits the record: JSON on stderr (for `--nocapture` evidence) and
/// a file under `CARGO_TARGET_TMPDIR` when cargo provides one.
/// Non-RT — control thread only.
fn emit_record(meta: &RunMetadata) {
    let json = meta
        .to_json()
        .unwrap_or_else(|error| format!("{{\"serialize_error\":\"{error}\"}}"));
    eprintln!("run_metadata: {json}");
    let dir = std::env::var("CARGO_TARGET_TMPDIR")
        .ok()
        .or_else(|| option_env!("CARGO_TARGET_TMPDIR").map(str::to_owned));
    if let Some(dir) = dir {
        let file = format!(
            "run-metadata-{}.json",
            meta.case_id
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>()
        );
        let path = Path::new(&dir).join(file);
        match meta.save_json(&path) {
            Ok(()) => eprintln!("run_metadata_path: {}", path.display()),
            Err(error) => eprintln!("run_metadata_path_error: {error}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic legs — always run
// ---------------------------------------------------------------------------

/// `for_environment` constructs a `virtual-os-endpoint`/`ext-l3`
/// record that round-trips through the Case Schema v1 JSON vocabulary.
#[test]
fn for_environment_builds_endpoint_case_record() {
    let meta = endpoint_record("rf629.test.metadata-roundtrip", "test-oracle")
        .with_expected("kebab-case vocabulary round-trip")
        .with_result(CaseResult::NotRun, "constructed only — never executed")
        .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs");

    assert_eq!(meta.result, CaseResult::NotRun);
    assert_eq!(meta.level, VerificationLevel::ExtL3);
    assert_eq!(meta.environment_kind, EnvironmentKind::VirtualOsEndpoint);
    assert_eq!(meta.requirement, CaseRequirement::Required);

    let json = meta.to_json().unwrap();
    assert!(json.contains("\"environment_kind\":\"virtual-os-endpoint\""));
    assert!(json.contains("\"level\":\"ext-l3\""));
    assert!(json.contains("\"result\":\"not-run\""));
    let back = RunMetadata::from_json(&json).unwrap();
    assert_eq!(back, meta);
}

fn regression_slots(indices: &[u64]) -> [AtomicU64; REGRESSION_INDEX_SLOTS] {
    let slots: [AtomicU64; REGRESSION_INDEX_SLOTS] =
        std::array::from_fn(|_| AtomicU64::new(u64::MAX));
    for (slot, index) in slots.iter().zip(indices) {
        slot.store(*index, Ordering::Relaxed);
    }
    slots
}

/// Timestamp-epoch classification is behaviorally fixed without a
/// provisioned device: grace at each epoch start is transient, while
/// mid-epoch regressions and unclassifiable slot overflow stay fatal.
#[test]
fn timestamp_epoch_classifier_distinguishes_transient_and_fatal_regressions() {
    let boundary = 100_u64;
    let slots = regression_slots(&[0, 23, 24, 50, 100, 123, 124, 130]);
    assert_eq!(effective_regressions(8, &slots, boundary), 4);

    let first_epoch_only = regression_slots(&[0, 23, 24, 50]);
    assert_eq!(
        effective_regressions(4, &first_epoch_only, u64::MAX),
        2,
        "without a restart boundary only callbacks 0..23 are epoch-start grace"
    );

    let overflow_indices = [0_u64; REGRESSION_INDEX_SLOTS];
    let overflow = regression_slots(&overflow_indices);
    assert_eq!(
        effective_regressions(
            u64::try_from(REGRESSION_INDEX_SLOTS + 1).unwrap(),
            &overflow,
            boundary,
        ),
        1,
        "the regression past the bounded index log is unclassifiable and fatal"
    );
}

#[test]
fn timestamp_classifier_keeps_mid_epoch_regression_fatal() {
    let classifier = TimestampEpochClassifier::new();
    classifier.observe(0, 1_000);
    classifier.observe(1, 2_000);
    classifier.observe(24, 3_000);
    classifier.observe(25, 2_500);

    assert_eq!(classifier.raw_regressions(), 1);
    assert_eq!(classifier.effective_regressions(), 1);
}

#[test]
fn timestamp_classifier_applies_grace_separately_after_epoch_boundary() {
    let classifier = TimestampEpochClassifier::new();
    classifier.observe(0, 1_000);
    classifier.split_epoch(100);
    classifier.observe(100, 10_000);
    classifier.observe(101, 9_000);
    assert_eq!(classifier.raw_regressions(), 1);
    assert_eq!(classifier.effective_regressions(), 0);

    classifier.observe(123, 9_500);
    classifier.observe(124, 9_400);
    assert_eq!(classifier.raw_regressions(), 3);
    assert_eq!(classifier.effective_regressions(), 1);
}

#[test]
fn render_timestamp_instrumentation_records_playback_absence_and_overflow() {
    let instrument = RenderTimestampInstrument::new();
    let total = u64::try_from(RENDER_TIMESTAMP_LOG_SLOTS + 2).unwrap();
    for index in 0..total {
        let callback_ns = 1_000 + i64::try_from(index).unwrap();
        let playback_ns = (index != 0).then_some(callback_ns + 500);
        instrument.observe(index, callback_ns, playback_ns, 480, index * 480);
    }

    assert_eq!(instrument.log_overflows(), 2);
    assert_eq!(instrument.playback_none_count(), 1);
    let events = instrument.events();
    assert_eq!(events.len(), RENDER_TIMESTAMP_LOG_SLOTS);
    assert_eq!(events[0].callback_index, 0);
    assert_eq!(events[0].playback_ns, None);
    assert_eq!(events[0].interval_ns, None);
    assert_eq!(events[1].playback_ns, Some(1_001 + 500));
    assert_eq!(events[1].interval_ns, Some(1));
    assert_eq!(
        events[RENDER_TIMESTAMP_LOG_SLOTS - 1].callback_index,
        u64::try_from(RENDER_TIMESTAMP_LOG_SLOTS - 1).unwrap()
    );
}

#[test]
fn render_timestamp_instrumentation_records_diagnostic_inputs_and_epoch_reset() {
    let instrument = RenderTimestampInstrument::new();
    instrument.observe(0, 1_000, Some(1_500), 480, 0);
    instrument.observe(1, 2_000, Some(2_600), 480, 480);
    instrument.split_epoch(2);
    instrument.observe(2, 3_000, Some(3_700), 480, 960);

    let events = instrument.events();
    assert_eq!(events[1].supply_frame_cursor, 480);
    assert_eq!(events[1].playback_interval_ns, Some(1_100));
    assert_eq!(events[1].playback_lead_ns, Some(600));
    assert!(events[1].host_interval_ns.is_some());
    assert_eq!(events[2].interval_ns, None);
    assert_eq!(events[2].host_interval_ns, None);
    assert_eq!(events[2].playback_interval_ns, None);
}

#[test]
fn render_timestamp_instrumentation_counts_playback_regression_without_changing_oracle() {
    let instrument = RenderTimestampInstrument::new();
    instrument.observe(0, 1_000, Some(2_000), 480, 0);
    instrument.observe(1, 2_000, Some(1_500), 480, 480);

    assert_eq!(instrument.playback_regression_count(), 1);
    assert_eq!(instrument.raw_regressions(), 0);
    assert_eq!(instrument.events()[1].playback_interval_ns, Some(-500));
}

#[test]
fn render_timestamp_instrumentation_detects_callback_overlap_without_blocking() {
    let instrument = RenderTimestampInstrument::new();
    let first = instrument.enter_callback();
    let second = instrument.enter_callback();
    assert_eq!(instrument.max_active_callbacks(), 2);
    assert_eq!(instrument.overlapping_callback_events(), 1);
    drop(second);
    drop(first);

    let third = instrument.enter_callback();
    assert_eq!(instrument.max_active_callbacks(), 2);
    assert_eq!(instrument.overlapping_callback_events(), 1);
    drop(third);
}

#[test]
fn render_regression_diagnostics_preserves_layer_isolation_inputs() {
    let event = |callback_index: u64,
                 callback_ns: i64,
                 host_monotonic_ns: u64,
                 regression_backstep_ns: Option<i64>,
                 playback_lead_ns: i64,
                 playback_interval_ns: Option<i64>,
                 supply_frame_cursor: u64| RenderTimestampEvent {
        callback_index,
        epoch: 0,
        frames: 480,
        callback_ns,
        playback_ns: Some(callback_ns + playback_lead_ns),
        interval_ns: None,
        regression_backstep_ns,
        host_monotonic_ns,
        host_interval_ns: (callback_index != 24).then_some(10_000_000),
        playback_interval_ns,
        playback_lead_ns: Some(playback_lead_ns),
        supply_frame_cursor,
    };
    let events = vec![
        event(24, 2_000, 0, None, 100, None, 11_520),
        event(25, 1_500, 10_000_000, Some(-500), 200, Some(600), 12_000),
        event(26, 2_600, 20_000_000, None, 150, Some(550), 12_480),
    ];

    let diagnostics = render_regression_diagnostics(&events, u64::MAX);
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = diagnostics[0];
    assert_eq!(diagnostic.callback_index, 25);
    assert_eq!(diagnostic.epoch_callback_offset, 25);
    assert!(!diagnostic.startup_grace);
    assert_eq!(diagnostic.reference_callback_index, Some(24));
    assert_eq!(diagnostic.reference_callback_ns, Some(2_000));
    assert_eq!(diagnostic.backend_two_step_ns, Some(600));
    assert_eq!(diagnostic.host_two_step_ns, Some(20_000_000));
    assert_eq!(diagnostic.supply_two_step_frames, Some(960));
    assert_eq!(diagnostic.playback_lead_before_ns, Some(100));
    assert_eq!(diagnostic.playback_lead_ns, Some(200));
    assert_eq!(diagnostic.playback_lead_after_ns, Some(150));
    assert_eq!(diagnostic.playback_interval_ns, Some(600));
    assert_eq!(diagnostic.next_playback_interval_ns, Some(550));
}

#[test]
fn roundtrip_output_diagnostics_are_tagged_and_distinct_from_baseline() {
    let probe = EndpointProbe::default();
    probe.out_timestamp.observe(0, 1_000, Some(2_000), 480, 0);
    probe.out_timestamp.observe(1, 2_000, Some(3_000), 480, 480);

    let roundtrip = roundtrip_output_diagnostic_lines(&probe, 2);
    assert!(roundtrip.iter().any(|line| line.starts_with(
        "roundtrip_output_timestamp_summary: path=roundtrip_output case=rf629.audio.virtual-endpoint.render-capture pair=2 "
    )));
    assert!(roundtrip.iter().any(|line| line.starts_with(
        "roundtrip_output_timestamp_event: path=roundtrip_output case=rf629.audio.virtual-endpoint.render-capture pair=2 index=0 "
    )));

    let baseline = baseline_timestamp_diagnostic_lines(&probe);
    assert!(
        baseline
            .iter()
            .any(|line| line.starts_with("render_timestamp_event: index=0 "))
    );
    assert!(
        baseline
            .iter()
            .all(|line| !line.contains("path=roundtrip_output"))
    );
}

#[test]
fn roundtrip_output_regression_diag_preserves_epoch_offset_and_required_fields() {
    let probe = EndpointProbe::default();
    probe.out_timestamp.observe(0, 1_000, Some(1_500), 480, 0);
    probe.out_timestamp.observe(1, 2_000, Some(2_500), 480, 480);
    probe.out_timestamp.split_epoch(2);
    for index in 2..=26_u64 {
        let callback_ns = 10_000 + i64::try_from(index).unwrap() * 100;
        probe.out_timestamp.observe(
            index,
            callback_ns,
            Some(callback_ns + 500),
            480,
            index * 480,
        );
    }
    probe
        .out_timestamp
        .observe(27, 12_500, Some(13_000), 480, 27 * 480);

    let lines = roundtrip_output_diagnostic_lines(&probe, 0);
    let diagnostic = lines
        .iter()
        .find(|line| {
            line.starts_with("roundtrip_output_timestamp_regression_diag:")
                && line.contains(" index=27 ")
        })
        .expect("round-trip regression diagnostic");
    for field in [
        "path=roundtrip_output",
        "case=rf629.audio.virtual-endpoint.render-capture",
        "pair=0",
        "epoch=1",
        "epoch_offset=25",
        "startup_grace=0",
        "reference_index=26",
        "reference_callback_ns=",
        "backstep_ns=",
        "backend_two_step_ns=",
        "host_two_step_ns=",
        "supply_two_step_frames=",
        "playback_lead_before_ns=",
        "playback_lead_ns=",
        "playback_lead_after_ns=",
        "playback_interval_ns=",
        "next_playback_interval_ns=",
    ] {
        assert!(
            diagnostic.contains(field),
            "missing {field} in {diagnostic}"
        );
    }
}

#[test]
fn roundtrip_diagnostics_remain_available_when_capture_oracle_fails() {
    let probe = EndpointProbe::default();
    probe.in_callbacks.store(1, Ordering::Relaxed);
    probe.in_frames.store(480, Ordering::Relaxed);
    probe.out_callbacks.store(1, Ordering::Relaxed);
    probe.out_frames.store(480, Ordering::Relaxed);
    probe.out_timestamp.observe(0, 1_000, Some(2_000), 480, 0);
    let format = AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    };

    let failures = analyze_endpoint(&probe, &format);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("insufficient captured audio")),
        "expected capture oracle failure, got {failures:?}"
    );
    let lines = roundtrip_output_diagnostic_lines(&probe, 0);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("roundtrip_output_timestamp_event:")),
        "capture oracle failure must not suppress round-trip output diagnostics"
    );
}

#[test]
fn roundtrip_lifecycle_and_error_context_are_bounded_and_time_correlated() {
    let probe = EndpointProbe::default();
    probe.out_callbacks.store(4, Ordering::Relaxed);
    probe.in_callbacks.store(3, Ordering::Relaxed);
    record_stream_lifecycle(
        &probe,
        StreamDirection::Input,
        StreamLifecycleAction::Start,
        true,
    );
    record_stream_error(&probe, StreamDirection::Input, StreamErrorKind::Xrun);

    let lines = roundtrip_output_diagnostic_lines(&probe, 1);
    let lifecycle = lines
        .iter()
        .find(|line| line.starts_with("roundtrip_stream_lifecycle_event:"))
        .expect("lifecycle event line");
    assert!(lifecycle.contains("direction=input"));
    assert!(lifecycle.contains("action=start"));
    assert!(lifecycle.contains("result=ok"));
    assert!(lifecycle.contains("input_callback_index=2"));
    assert!(lifecycle.contains("output_callback_index=3"));
    let error = lines
        .iter()
        .find(|line| line.starts_with("roundtrip_stream_error_event:"))
        .expect("error event line");
    assert!(error.contains("direction=input"));
    assert!(error.contains("kind=xrun"));
    assert!(error.contains("host_monotonic_ns="));

    for _ in 0..STREAM_LIFECYCLE_LOG_SLOTS + 2 {
        record_stream_lifecycle(
            &probe,
            StreamDirection::Output,
            StreamLifecycleAction::Stop,
            true,
        );
    }
    assert_eq!(probe.stream_lifecycle_log.overflow_count(), 3);
    for _ in 0..STREAM_ERROR_LOG_SLOTS + 2 {
        record_stream_error(&probe, StreamDirection::Output, StreamErrorKind::Backend);
    }
    assert_eq!(probe.stream_error_log.overflow_count(), 3);
}

/// PCM leg: `PcmFixture` block → per-channel `oracle::
/// pcm_reference_block` exact comparison → `deterministic-backend`/
/// `os-l1` `pass` record.
#[test]
fn pcm_fixture_block_records_run_metadata() {
    let rate = SampleRate::new(48_000).unwrap();
    let fixture = PcmFixture::new(
        rate,
        2,
        PcmPattern::Noise {
            seed: RUN_SEED,
            amplitude: 0.5,
        },
    )
    .unwrap();
    let frames = 512_u32;
    let block = fixture.block(0, frames).unwrap();
    assert_eq!(block.data.len(), usize::try_from(frames).unwrap() * 2);

    let mut max_diff = 0.0_f32;
    for channel in 0..fixture.channels {
        let want = pcm_reference_block(&fixture, 0, frames, channel);
        let start = usize::from(channel) * usize::try_from(frames).unwrap();
        let got = &block.data[start..start + want.len()];
        let diff = pcm_assert_eq(got, &want, PcmCompare::Exact)
            .unwrap_or_else(|error| panic!("oracle mismatch on channel {channel}: {error}"));
        max_diff = max_diff.max(diff);
    }

    let meta = deterministic_record(
        "rf628.pcm.fixture-block",
        "oracle::pcm_reference_block",
        48_000,
    )
    .with_expected(format!(
        "pcm_reference_block Exact, {} frames × {} channels",
        block.frames, block.channels
    ))
    .with_result(
        CaseResult::Pass,
        format!(
            "max_abs_diff={max_diff} frames={} channels={} start_frame={}",
            block.frames, block.channels, block.start_frame
        ),
    )
    .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs");
    emit_record(&meta);
    assert_eq!(meta.result, CaseResult::Pass);
}

/// MIDI leg: `MidiScenario` → `run_midi_trace` →
/// `oracle::midi_expected_playthrough` → `deterministic-backend`/
/// `os-l1` `pass` record.
#[test]
fn midi_scenario_trace_records_run_metadata() {
    let base = TimeBase::sample_rate(48_000).unwrap();
    let scenario = MidiScenario::new(
        base,
        vec![
            cue(base, 2_400, 2, cc_event(10, 90)),
            cue(base, 0, 0, note_on_event(64, 80)),
            cue(base, 4_800, 3, pitch_bend_event(0x2A00)),
            cue(base, 7_200, 4, note_off_event(64)),
        ],
    );
    let trace = run_midi_trace(&scenario, &trace_config(9_600)).unwrap();
    assert!(midi_trace_frames_monotonic(trace.entries()));

    let expected = midi_expected_playthrough(base, &scenario.cues, 9_600);
    assert!(
        midi_trace_eq(trace.entries(), &expected),
        "trace diverged from the independent oracle"
    );

    let meta = deterministic_record(
        "rf628.midi.scenario-trace",
        "oracle::midi_expected_playthrough",
        48_000,
    )
    .with_expected(format!(
        "midi_expected_playthrough, {} cues",
        scenario.cues.len()
    ))
    .with_result(
        CaseResult::Pass,
        format!(
            "entries={} blocks_run={} undeliverable_spans={}",
            trace.entries().len(),
            trace.blocks_run(),
            trace.undeliverable_spans()
        ),
    )
    .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs");
    emit_record(&meta);
    assert_eq!(meta.result, CaseResult::Pass);
}

// ---------------------------------------------------------------------------
// MIDI deterministic helpers (scenario/oracle side)
// ---------------------------------------------------------------------------

fn ch() -> MidiChannel {
    MidiChannel::new(3).unwrap()
}

fn cue(base: TimeBase, pos: i64, seq: u64, event: MidiEvent) -> MidiCue {
    MidiCue {
        position: MediaTime::new(pos, base),
        source_sequence: seq,
        event,
    }
}

fn note_on_event(key: u8, vel: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(vel),
        },
    }
}

fn note_off_event(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(0),
        },
    }
}

fn cc_event(controller: u8, value: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::ControlChange {
            controller: MidiController::new(controller).unwrap(),
            value: MidiValue::from_u7(value),
        },
    }
}

fn pitch_bend_event(value14: u16) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: ch(),
        kind: MidiKind::PitchBend {
            value: MidiValue::bend_from_u14(value14),
        },
    }
}

fn trace_config(total_frames: u64) -> MidiTraceConfig {
    MidiTraceConfig {
        block_frames: 480,
        total_frames,
        contract: BlockContract::new(1, 4_096, InBlockTransportSupport::Unknown),
        policy: DeliveryPolicy::Exact,
        budget: ScheduleBudget::new(64, 1_024, 16, 0),
        commands: Vec::new(),
        trace_capacity: 64,
    }
}

// ---------------------------------------------------------------------------
// Virtual OS endpoint legs — #[ignore], manual opt-in
// ---------------------------------------------------------------------------

/// The fixed channel-voice sequence verified on the receive side —
/// an independent literal oracle, identical in spirit to
/// `ruviform-midi-device/tests/loopback.rs`. The *send* list is
/// supplied by `run_midi_trace`; this list is what the port traffic
/// must reproduce.
fn expected_channel_voice_sequence() -> [MidiEvent; 4] {
    let channel = MidiChannel::new(3).unwrap();
    let key = NoteKey::new(60).unwrap();
    [
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::NoteOn {
                key,
                velocity: MidiValue::from_u7(100),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::ControlChange {
                controller: MidiController::new(10).unwrap(),
                value: MidiValue::from_u7(90),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::PitchBend {
                value: MidiValue::bend_from_u14(0x2A00),
            },
        },
        MidiEvent {
            group: MidiGroup::ZERO,
            channel,
            kind: MidiKind::NoteOff {
                key,
                velocity: MidiValue::from_u7(64),
            },
        },
    ]
}

/// Order-preserving subsequence check — same rule as the device
/// verifier: every expected event must appear in `observed` in
/// order; unrelated traffic between matches is tolerated.
fn is_ordered_subsequence(observed: &[TimedMidiInput], expected: &[MidiEvent]) -> bool {
    let mut cursor = observed.iter();
    expected
        .iter()
        .all(|want| cursor.any(|got| &got.event == want))
}

/// Drains the open input until the expected sequence is observed or
/// the deadline expires. Returns observed events plus the loss
/// counters.
fn drain_until_match(
    input: &mut dyn MidiInput,
    expected: &[MidiEvent],
    deadline: Duration,
) -> (Vec<TimedMidiInput>, u64, u64) {
    let started = Instant::now();
    let mut observed = Vec::new();
    let mut buf = Vec::new();
    let (mut dropped, mut unsupported) = (0u64, 0u64);
    while started.elapsed() < deadline {
        buf.clear();
        match input.read(&mut buf, 64) {
            Ok(stats) => {
                dropped += stats.dropped;
                unsupported += stats.unsupported;
                observed.append(&mut buf);
            }
            Err(error) => panic!("verdict: Fail — read on open input failed: {error:?}"),
        }
        if is_ordered_subsequence(&observed, expected) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    (observed, dropped, unsupported)
}

/// Timestamp contract for a stamped port — mirrors
/// `loopback.rs`: declared clock domain, `Carried` provenance,
/// non-decreasing driver ticks. Unstamped ports skip the check.
fn assert_stamp_contract(
    observed: &[TimedMidiInput],
    provides_timestamps: bool,
    clock_domain: Option<ClockDomainId>,
) {
    if !provides_timestamps {
        return;
    }
    let declared_domain = clock_domain.expect("stamped port declares a domain");
    let mut prev_ticks = None;
    for event in observed {
        assert_eq!(
            event.stamp.domain, declared_domain,
            "events must be stamped in the port's declared domain"
        );
        assert_eq!(event.stamp.provenance, TimestampProvenance::Carried);
        if let Some(prev) = prev_ticks {
            assert!(
                event.stamp.ticks >= prev,
                "driver ticks must be non-decreasing"
            );
        }
        prev_ticks = Some(event.stamp.ticks);
    }
}

/// MIDI virtual-endpoint leg: Test Runtime trace supplies the send
/// list; the observed port traffic is checked against the independent
/// literal oracle. `not-run` records on every unprovisioned path.
#[test]
#[ignore = "requires a provisioned MIDI loopback pair; manual opt-in check"]
fn midi_virtual_endpoint_loopback() {
    let meta = endpoint_record(
        "rf629.midi.virtual-endpoint.loopback",
        "ordered-subsequence+stamp-contract-v1",
    );
    let Some((in_port, out_port)) = select_midi_pair(&meta) else {
        return;
    };
    run_midi_endpoint_case(in_port.as_ref(), out_port.as_ref(), meta);
}

/// Enumerates both MIDI directions and selects the first
/// bidirectional-name pair candidate — the same rule as
/// `loopback.rs` (kept in sync by hand). Every `NotRun` record is
/// emitted here; `None` means the environment cannot run the case.
fn select_midi_pair(
    meta: &RunMetadata,
) -> Option<(Box<dyn MidiInputPort>, Box<dyn MidiOutputPort>)> {
    let backend_in: Box<dyn MidiInputEnumerator> = Box::new(backend());
    let backend_out: Box<dyn MidiOutputEnumerator> = Box::new(backend());
    let (inputs, outputs) = match (backend_in.input_ports(), backend_out.output_ports()) {
        (Ok(inputs), Ok(outputs)) => (inputs, outputs),
        (Err(error), _) | (_, Err(error)) => {
            let record = meta.clone().with_result(
                CaseResult::NotRun,
                format!("port enumeration unavailable: {error:?}"),
            );
            emit_record(&record);
            eprintln!("verdict: NotRun — port enumeration unavailable ({error:?})");
            return None;
        }
    };
    eprintln!(
        "enumerated: {} input ports, {} output ports",
        inputs.len(),
        outputs.len()
    );

    let mut inputs = inputs;
    let mut pairs = Vec::new();
    for out_port in outputs {
        let out_name = out_port.info().name.clone();
        if let Some(index) = inputs.iter().position(|p| p.info().name == out_name) {
            let in_port = inputs.swap_remove(index);
            pairs.push((in_port, out_port));
        }
    }
    if pairs.is_empty() {
        let record = meta
            .clone()
            .with_result(
                CaseResult::NotRun,
                "NOT_PROVISIONED — no bidirectional name match",
            )
            .with_expected("4-event channel-voice sequence round-trip");
        emit_record(&record);
        eprintln!("verdict: NotRun — NOT_PROVISIONED (no bidirectional name match)");
        return None;
    }
    eprintln!("  pair candidates: {}", pairs.len());
    Some(pairs.swap_remove(0))
}

/// Opens the matched ports (input first so nothing sent can arrive
/// before the subscription exists), supplies the trace-derived send
/// list, drains, verifies, and records the terminal verdict.
fn run_midi_endpoint_case(
    in_port: &dyn MidiInputPort,
    out_port: &dyn MidiOutputPort,
    meta: RunMetadata,
) {
    let in_info = in_port.info();
    let out_info = out_port.info();
    eprintln!(
        "  pair: in id={} name={:?} | out id={} name={:?} (candidate, unconfirmed)",
        in_info.id, in_info.name, out_info.id, out_info.name
    );

    let mut input = open_midi_input(in_port, &meta);
    let mut output = open_midi_output(out_port, &meta);

    // Test Runtime supply: the deterministic trace's delivered events
    // are the send list — the adapter under test never generates them.
    let (send_list, expected) = midi_supply_list();
    let started = Instant::now();
    for event in &send_list {
        output
            .send(*event)
            .expect("send loopback event through matched output port");
        // WinMM short messages are queued driver-side; a small gap
        // keeps the loopback's forwarding observable per event.
        thread::sleep(Duration::from_millis(30));
    }

    finish_midi_case(
        &mut *input,
        &mut *output,
        &in_info,
        &expected,
        &send_list,
        started,
        meta,
    );
}

fn open_midi_input(port: &dyn MidiInputPort, meta: &RunMetadata) -> Box<dyn MidiInput> {
    port.open().unwrap_or_else(|error| {
        let record = meta.clone().with_result(
            CaseResult::Fail,
            format!("matched input port failed to open: {error:?}"),
        );
        emit_record(&record);
        panic!("verdict: Fail — matched input port failed to open: {error:?}")
    })
}

fn open_midi_output(port: &dyn MidiOutputPort, meta: &RunMetadata) -> Box<dyn MidiOutput> {
    port.open().unwrap_or_else(|error| {
        let record = meta.clone().with_result(
            CaseResult::Fail,
            format!("matched output port failed to open: {error:?}"),
        );
        emit_record(&record);
        panic!("verdict: Fail — matched output port failed to open: {error:?}")
    })
}

/// The send list supplied by the Test Runtime: a `MidiScenario`
/// mirroring the expected literal sequence is compiled and run through
/// `run_midi_trace`; the delivered trace entries — not a hand-built
/// list — are what the output port receives. Returns
/// `(send_list, expected_oracle)`.
fn midi_supply_list() -> (Vec<MidiEvent>, [MidiEvent; 4]) {
    let expected = expected_channel_voice_sequence();
    let base = TimeBase::sample_rate(48_000).unwrap();
    let scenario = MidiScenario::new(
        base,
        expected
            .iter()
            .enumerate()
            .map(|(index, event)| {
                cue(
                    base,
                    i64::try_from(index).unwrap() * 2_400,
                    index as u64,
                    *event,
                )
            })
            .collect(),
    );
    let trace = run_midi_trace(&scenario, &trace_config(9_600))
        .expect("scenario trace must compile and run");
    let send_list: Vec<MidiEvent> = trace.entries().iter().map(|entry| entry.event).collect();
    assert_eq!(
        send_list.len(),
        expected.len(),
        "trace must deliver every scenario event"
    );
    (send_list, expected)
}

/// Drain → ordered-subsequence + stamp-contract verification →
/// close/invalidation checks → terminal `RunMetadata` record.
fn finish_midi_case(
    input: &mut dyn MidiInput,
    output: &mut dyn MidiOutput,
    in_info: &MidiInputInfo,
    expected: &[MidiEvent; 4],
    send_list: &[MidiEvent],
    started: Instant,
    meta: RunMetadata,
) {
    let (observed, dropped, unsupported) =
        drain_until_match(input, expected, Duration::from_secs(3));

    if !is_ordered_subsequence(&observed, expected) {
        let record = meta
            .clone()
            .with_result(
                CaseResult::Fail,
                format!(
                    "expected sequence did not round-trip (observed={} dropped={} unsupported={})",
                    observed.len(),
                    dropped,
                    unsupported
                ),
            )
            .with_expected("4-event channel-voice sequence round-trip");
        emit_record(&record);
        eprintln!("verdict: Fail — expected sequence did not round-trip");
        eprintln!("  expected: {expected:?}");
        eprintln!(
            "  observed ({} events): {:?}",
            observed.len(),
            observed.iter().map(|e| &e.event).collect::<Vec<_>>()
        );
        panic!("loopback round-trip did not reproduce the expected sequence");
    }

    assert_stamp_contract(
        &observed,
        in_info.capabilities.provides_timestamps,
        in_info.capabilities.clock_domain,
    );
    let tick_span = match (observed.first(), observed.last()) {
        (Some(first), Some(last)) => Some(last.stamp.ticks - first.stamp.ticks),
        _ => None,
    };

    input.close();
    let invalidated_in = matches!(
        input.read(&mut Vec::new(), 64),
        Err(DeviceError::StreamInvalidated)
    );
    output.close();
    let invalidated_out = matches!(
        output.send(expected[0]),
        Err(DeviceError::StreamInvalidated)
    );

    let record = meta
        .with_expected("4-event channel-voice sequence round-trip (ordered subsequence)")
        .with_result(
            CaseResult::Pass,
            format!(
                "sent={} observed={} dropped={} unsupported={} elapsed={:?} tick_span={tick_span:?} invalidated_in={invalidated_in} invalidated_out={invalidated_out}",
                send_list.len(),
                observed.len(),
                dropped,
                unsupported,
                started.elapsed(),
            ),
        )
        .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs")
        .with_evidence("name match is a candidate, not proof of a virtual endpoint");
    emit_record(&record);
    eprintln!("verdict: Pass — round-trip observed through real OS ports");
    eprintln!(
        "  evidence note: name match is a candidate, not proof of a virtual \
         endpoint — this is OS-L1 round-trip evidence at best, recorded \
         under virtual-os-endpoint pending human-confirmed provisioning"
    );
}

// ---------------------------------------------------------------------------
// Audio virtual OS endpoint leg
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct TimestampObservation {
    interval_ns: Option<i64>,
    regression_backstep_ns: Option<i64>,
    epoch: u64,
}

struct TimestampEpochClassifier {
    last_callback_ns: AtomicI64,
    last_observed_callback_ns: AtomicI64,
    regressions: AtomicU64,
    epoch_resets: AtomicU64,
    regression_first_cb: AtomicU64,
    regression_last_cb: AtomicU64,
    regression_max_backstep_ns: AtomicI64,
    epoch_boundary_cb: AtomicU64,
    regression_cbs: [AtomicU64; REGRESSION_INDEX_SLOTS],
}

impl TimestampEpochClassifier {
    fn new() -> Self {
        Self {
            last_callback_ns: AtomicI64::new(i64::MIN),
            last_observed_callback_ns: AtomicI64::new(i64::MIN),
            regressions: AtomicU64::new(0),
            epoch_resets: AtomicU64::new(0),
            regression_first_cb: AtomicU64::new(u64::MAX),
            regression_last_cb: AtomicU64::new(0),
            regression_max_backstep_ns: AtomicI64::new(i64::MAX),
            epoch_boundary_cb: AtomicU64::new(u64::MAX),
            regression_cbs: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
        }
    }

    fn observe(&self, callback_index: u64, callback_ns: i64) -> TimestampObservation {
        let previous_observed = self
            .last_observed_callback_ns
            .swap(callback_ns, Ordering::Relaxed);
        let previous = self
            .last_callback_ns
            .fetch_max(callback_ns, Ordering::Relaxed);
        let mut regression_backstep_ns = None;
        if callback_ns < previous {
            let slot = self.regressions.fetch_add(1, Ordering::Relaxed);
            if let Ok(index) = usize::try_from(slot)
                && index < REGRESSION_INDEX_SLOTS
            {
                self.regression_cbs[index].store(callback_index, Ordering::Relaxed);
            }
            self.regression_first_cb
                .fetch_min(callback_index, Ordering::Relaxed);
            self.regression_last_cb
                .fetch_max(callback_index, Ordering::Relaxed);
            let backstep = callback_ns.saturating_sub(previous);
            self.regression_max_backstep_ns
                .fetch_min(backstep, Ordering::Relaxed);
            regression_backstep_ns = Some(backstep);
        }
        TimestampObservation {
            interval_ns: (previous_observed != i64::MIN)
                .then_some(callback_ns.saturating_sub(previous_observed)),
            regression_backstep_ns,
            epoch: self.epoch_resets.load(Ordering::Relaxed),
        }
    }

    fn split_epoch(&self, boundary_cb: u64) {
        self.last_callback_ns.store(i64::MIN, Ordering::Relaxed);
        self.last_observed_callback_ns
            .store(i64::MIN, Ordering::Relaxed);
        self.epoch_resets.fetch_add(1, Ordering::Relaxed);
        self.epoch_boundary_cb.store(boundary_cb, Ordering::Relaxed);
    }

    fn raw_regressions(&self) -> u64 {
        self.regressions.load(Ordering::Relaxed)
    }

    fn effective_regressions(&self) -> u64 {
        effective_regressions(
            self.raw_regressions(),
            &self.regression_cbs,
            self.epoch_boundary_cb.load(Ordering::Relaxed),
        )
    }

    fn regression_index_log(&self) -> String {
        regression_index_log(&self.regression_cbs, self.raw_regressions())
    }
}

impl Default for TimestampEpochClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RenderTimestampEvent {
    callback_index: u64,
    epoch: u64,
    frames: u64,
    callback_ns: i64,
    playback_ns: Option<i64>,
    interval_ns: Option<i64>,
    regression_backstep_ns: Option<i64>,
    host_monotonic_ns: u64,
    host_interval_ns: Option<u64>,
    playback_interval_ns: Option<i64>,
    playback_lead_ns: Option<i64>,
    supply_frame_cursor: u64,
}

struct RenderTimestampLog {
    next: AtomicU64,
    overflow: AtomicU64,
    callback_index: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    epoch: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    frames: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    callback_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    playback_present: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    playback_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    interval_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    regression_backstep_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    host_monotonic_ns: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    host_interval_ns: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
    playback_interval_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    playback_lead_ns: [AtomicI64; RENDER_TIMESTAMP_LOG_SLOTS],
    supply_frame_cursor: [AtomicU64; RENDER_TIMESTAMP_LOG_SLOTS],
}

impl RenderTimestampLog {
    fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            overflow: AtomicU64::new(0),
            callback_index: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            epoch: std::array::from_fn(|_| AtomicU64::new(0)),
            frames: std::array::from_fn(|_| AtomicU64::new(0)),
            callback_ns: std::array::from_fn(|_| AtomicI64::new(i64::MIN)),
            playback_present: std::array::from_fn(|_| AtomicU64::new(0)),
            playback_ns: std::array::from_fn(|_| AtomicI64::new(i64::MIN)),
            interval_ns: std::array::from_fn(|_| AtomicI64::new(i64::MIN)),
            regression_backstep_ns: std::array::from_fn(|_| AtomicI64::new(i64::MAX)),
            host_monotonic_ns: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            host_interval_ns: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            playback_interval_ns: std::array::from_fn(|_| AtomicI64::new(i64::MIN)),
            playback_lead_ns: std::array::from_fn(|_| AtomicI64::new(i64::MIN)),
            supply_frame_cursor: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
        }
    }

    fn record(&self, event: RenderTimestampEvent) {
        let slot = self.next.fetch_add(1, Ordering::Relaxed);
        let Ok(index) = usize::try_from(slot) else {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if index >= RENDER_TIMESTAMP_LOG_SLOTS {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.callback_index[index].store(event.callback_index, Ordering::Relaxed);
        self.epoch[index].store(event.epoch, Ordering::Relaxed);
        self.frames[index].store(event.frames, Ordering::Relaxed);
        self.callback_ns[index].store(event.callback_ns, Ordering::Relaxed);
        self.playback_present[index]
            .store(u64::from(event.playback_ns.is_some()), Ordering::Relaxed);
        self.playback_ns[index].store(event.playback_ns.unwrap_or(i64::MIN), Ordering::Relaxed);
        self.interval_ns[index].store(event.interval_ns.unwrap_or(i64::MIN), Ordering::Relaxed);
        self.regression_backstep_ns[index].store(
            event.regression_backstep_ns.unwrap_or(i64::MAX),
            Ordering::Relaxed,
        );
        self.host_monotonic_ns[index].store(event.host_monotonic_ns, Ordering::Relaxed);
        self.host_interval_ns[index].store(
            event.host_interval_ns.unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        self.playback_interval_ns[index].store(
            event.playback_interval_ns.unwrap_or(i64::MIN),
            Ordering::Relaxed,
        );
        self.playback_lead_ns[index].store(
            event.playback_lead_ns.unwrap_or(i64::MIN),
            Ordering::Relaxed,
        );
        self.supply_frame_cursor[index].store(event.supply_frame_cursor, Ordering::Relaxed);
    }

    fn events(&self) -> Vec<RenderTimestampEvent> {
        let len = self
            .next
            .load(Ordering::Relaxed)
            .min(u64::try_from(RENDER_TIMESTAMP_LOG_SLOTS).unwrap_or(u64::MAX));
        (0..len)
            .map(|slot| {
                let index = usize::try_from(slot).unwrap_or(usize::MAX);
                let playback_present = self.playback_present[index].load(Ordering::Relaxed) != 0;
                let playback_ns = self.playback_ns[index].load(Ordering::Relaxed);
                let interval_ns = self.interval_ns[index].load(Ordering::Relaxed);
                let backstep_ns = self.regression_backstep_ns[index].load(Ordering::Relaxed);
                let host_interval_ns = self.host_interval_ns[index].load(Ordering::Relaxed);
                let playback_interval_ns = self.playback_interval_ns[index].load(Ordering::Relaxed);
                let playback_lead_ns = self.playback_lead_ns[index].load(Ordering::Relaxed);
                RenderTimestampEvent {
                    callback_index: self.callback_index[index].load(Ordering::Relaxed),
                    epoch: self.epoch[index].load(Ordering::Relaxed),
                    frames: self.frames[index].load(Ordering::Relaxed),
                    callback_ns: self.callback_ns[index].load(Ordering::Relaxed),
                    playback_ns: playback_present.then_some(playback_ns),
                    interval_ns: (interval_ns != i64::MIN).then_some(interval_ns),
                    regression_backstep_ns: (backstep_ns != i64::MAX).then_some(backstep_ns),
                    host_monotonic_ns: self.host_monotonic_ns[index].load(Ordering::Relaxed),
                    host_interval_ns: (host_interval_ns != u64::MAX).then_some(host_interval_ns),
                    playback_interval_ns: (playback_interval_ns != i64::MIN)
                        .then_some(playback_interval_ns),
                    playback_lead_ns: (playback_lead_ns != i64::MIN).then_some(playback_lead_ns),
                    supply_frame_cursor: self.supply_frame_cursor[index].load(Ordering::Relaxed),
                }
            })
            .collect()
    }

    fn overflow_count(&self) -> u64 {
        self.overflow.load(Ordering::Relaxed)
    }
}

impl Default for RenderTimestampLog {
    fn default() -> Self {
        Self::new()
    }
}

struct RenderTimestampInstrument {
    classifier: TimestampEpochClassifier,
    log: RenderTimestampLog,
    playback_some: AtomicU64,
    playback_none: AtomicU64,
    playback_regressions: AtomicU64,
    host_origin: Instant,
    last_host_monotonic_ns: AtomicU64,
    last_playback_ns: AtomicI64,
    active_callbacks: AtomicU64,
    max_active_callbacks: AtomicU64,
    overlapping_callback_events: AtomicU64,
}

impl RenderTimestampInstrument {
    fn new() -> Self {
        Self {
            classifier: TimestampEpochClassifier::new(),
            log: RenderTimestampLog::new(),
            playback_some: AtomicU64::new(0),
            playback_none: AtomicU64::new(0),
            playback_regressions: AtomicU64::new(0),
            host_origin: Instant::now(),
            last_host_monotonic_ns: AtomicU64::new(u64::MAX),
            last_playback_ns: AtomicI64::new(i64::MIN),
            active_callbacks: AtomicU64::new(0),
            max_active_callbacks: AtomicU64::new(0),
            overlapping_callback_events: AtomicU64::new(0),
        }
    }

    fn observe(
        &self,
        callback_index: u64,
        callback_ns: i64,
        playback_ns: Option<i64>,
        frames: u64,
        supply_frame_cursor: u64,
    ) {
        let observation = self.classifier.observe(callback_index, callback_ns);
        let host_monotonic_ns = self.host_monotonic_ns();
        let previous_host = self
            .last_host_monotonic_ns
            .swap(host_monotonic_ns, Ordering::Relaxed);
        let host_interval_ns =
            (previous_host != u64::MAX).then_some(host_monotonic_ns.saturating_sub(previous_host));
        let previous_playback = playback_ns.map_or(i64::MIN, |playback| {
            self.last_playback_ns.swap(playback, Ordering::Relaxed)
        });
        let playback_interval_ns = playback_ns.and_then(|playback| {
            (previous_playback != i64::MIN).then_some(playback.saturating_sub(previous_playback))
        });
        if playback_interval_ns.is_some_and(|interval| interval < 0) {
            self.playback_regressions.fetch_add(1, Ordering::Relaxed);
        }
        if playback_ns.is_some() {
            self.playback_some.fetch_add(1, Ordering::Relaxed);
        } else {
            self.playback_none.fetch_add(1, Ordering::Relaxed);
        }
        self.log.record(RenderTimestampEvent {
            callback_index,
            epoch: observation.epoch,
            frames,
            callback_ns,
            playback_ns,
            interval_ns: observation.interval_ns,
            regression_backstep_ns: observation.regression_backstep_ns,
            host_monotonic_ns,
            host_interval_ns,
            playback_interval_ns,
            playback_lead_ns: playback_ns.map(|playback| playback.saturating_sub(callback_ns)),
            supply_frame_cursor,
        });
    }

    fn host_monotonic_ns(&self) -> u64 {
        u64::try_from(
            self.host_origin
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)),
        )
        .unwrap_or(u64::MAX)
    }

    fn enter_callback(&self) -> RenderCallbackActivity<'_> {
        let previous_active = self.active_callbacks.fetch_add(1, Ordering::Relaxed);
        let active = previous_active.saturating_add(1);
        self.max_active_callbacks
            .fetch_max(active, Ordering::Relaxed);
        if previous_active != 0 {
            self.overlapping_callback_events
                .fetch_add(1, Ordering::Relaxed);
        }
        RenderCallbackActivity { instrument: self }
    }

    fn split_epoch(&self, boundary_cb: u64) {
        self.classifier.split_epoch(boundary_cb);
        self.last_host_monotonic_ns
            .store(u64::MAX, Ordering::Relaxed);
        self.last_playback_ns.store(i64::MIN, Ordering::Relaxed);
    }

    fn raw_regressions(&self) -> u64 {
        self.classifier.raw_regressions()
    }

    fn effective_regressions(&self) -> u64 {
        self.classifier.effective_regressions()
    }

    fn regression_index_log(&self) -> String {
        self.classifier.regression_index_log()
    }

    fn playback_some_count(&self) -> u64 {
        self.playback_some.load(Ordering::Relaxed)
    }

    fn playback_none_count(&self) -> u64 {
        self.playback_none.load(Ordering::Relaxed)
    }

    fn playback_regression_count(&self) -> u64 {
        self.playback_regressions.load(Ordering::Relaxed)
    }

    fn max_active_callbacks(&self) -> u64 {
        self.max_active_callbacks.load(Ordering::Relaxed)
    }

    fn overlapping_callback_events(&self) -> u64 {
        self.overlapping_callback_events.load(Ordering::Relaxed)
    }

    fn events(&self) -> Vec<RenderTimestampEvent> {
        self.log.events()
    }

    fn log_overflows(&self) -> u64 {
        self.log.overflow_count()
    }
}

impl Default for RenderTimestampInstrument {
    fn default() -> Self {
        Self::new()
    }
}

struct RenderCallbackActivity<'a> {
    instrument: &'a RenderTimestampInstrument,
}

impl Drop for RenderCallbackActivity<'_> {
    fn drop(&mut self) {
        self.instrument
            .active_callbacks
            .fetch_sub(1, Ordering::Relaxed);
    }
}

// rf#1034 backend OS/driver diagnostic trace control.

const BACKEND_TRACE_MAX_LOG_MB: &str = "64";
const BACKEND_TRACE_KEYWORD_MASK_ALL: &str = "0xffffffffffffffff";
const BACKEND_TRACE_LEVEL: &str = "0xff";
const RVF_POSITION_ETW_PRESENTATION_POSITION_V1: u64 = 0x2001;
const RVF_XRUN_ETW_SCENARIO_SNAPSHOT: u64 = 0x1001;
const RVF_XRUN_ETW_ZERO_RANGE: u64 = 0x1002;
const RVF_XRUN_ETW_SUPPRESSION_TRANSITION: u64 = 0x1003;
const RVF_XRUN_ETW_ZERO_FILL_BOUNDARY: u64 = 0x1004;
const RVF_XRUN_ETW_OS_PACKET_GAP: u64 = 0x1005;
const RVF_POSITION_ETW_FREQUENCY_SHIFT: u32 = 16;
const MICROSOFT_WINDOWS_AUDIO_GUID: &str = "{AE4BD3BE-F36F-45B6-8D21-BDD6FB832853}";
const MICROSOFT_WINDOWS_KERNELSTREAMING_GUID: &str = "{548C4417-CE45-41FF-99DD-528F01CE0FE1}";
const MICROSOFT_WINDOWS_AUDIOENGINEVERBOSE_GUID: &str = "{A6A00EFD-21F2-4A99-807E-9B3BF1D90285}";
const MICROSOFT_WINDOWS_KERNEL_EVENTTRACING_GUID: &str = "{B675EC37-BDB6-4648-BC92-F3FDC74D3CA2}";
const PORTCLS_AUDIOENGINE_CONTROL_GUID: &str = "{10EB6007-818C-4DB6-A694-B518E589D07A}";
const PORTCLS_AUDIOENGINE_EVENT_GUID: &str = "{97BCD203-6286-4974-801D-A293B921F84B}";
const PORTCLS_POSITION_EVENT_GUID: &str = "{93BB152D-DF9E-4282-B709-1696392F2557}";
const PORTCLS_ROOT_EVENT_GUID: &str = "{9502CBC6-AA74-4EFF-BA91-D9329BCCE758}";
const CLASSIC_SELFTEST_PROVIDER_GUID: &str = "{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2001}";
const CLASSIC_SELFTEST_EVENT_CLASS_GUID: &str = "{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2002}";
const DRIVER_WMI_PROBE_CONTROL_GUID: &str = "{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2101}";
const DRIVER_WMI_PROBE_EVENT_GUID: &str = "{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2102}";
const KERNEL_EVENTTRACING_ENABLE_EVENT_ID: u64 = 14;
const KERNEL_EVENTTRACING_DISABLE_EVENT_ID: u64 = 15;
const KERNEL_EVENTTRACING_ENABLE_OPCODE: u64 = 18;
const KERNEL_EVENTTRACING_DISABLE_OPCODE: u64 = 19;
const PORTCLS_MINIPORT_TRACE_TYPE_BASE: u64 = 96;
const PORTCLS_MINIPORT_MAX_ORDINAL: u64 = 10;
const PORTCLS_MINIPORT_PAYLOAD_U64S: usize = 5;
const DRIVER_POSITION_EVENT_LIMIT: usize = 16_384;
const DRIVER_WMI_PROBE_EVENT_LIMIT: usize = 4_096;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BackendTraceProviderRole {
    KernelEventTracingControl,
    ManifestControl,
    ClassicOrWmiControl,
    ClassicSelftestControl,
    DriverWmiProbeControl,
    WmiEventClass,
}

impl BackendTraceProviderRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::KernelEventTracingControl => "kernel-eventtracing-control",
            Self::ManifestControl => "manifest-control",
            Self::ClassicOrWmiControl => "classic-or-wmi-control",
            Self::ClassicSelftestControl => "classic-selftest-control",
            Self::DriverWmiProbeControl => "driver-wmi-probe-control",
            Self::WmiEventClass => "wmi-event-class",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct BackendTraceProviderSpec {
    name: &'static str,
    guid: &'static str,
    keyword_mask: &'static str,
    level: &'static str,
    role: BackendTraceProviderRole,
    enabled_in_session: bool,
    selection_reason: &'static str,
    require_registered: bool,
}

const BACKEND_TRACE_PROVIDERS: [BackendTraceProviderSpec; 10] = [
    BackendTraceProviderSpec {
        name: "Microsoft-Windows-Kernel-EventTracing",
        guid: MICROSOFT_WINDOWS_KERNEL_EVENTTRACING_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::KernelEventTracingControl,
        enabled_in_session: true,
        selection_reason: "ETW subsystem analytic provider used only as an enable/disable control-request witness; absence is not proof that a WMI control IRP did not reach PortCls",
        require_registered: true,
    },
    BackendTraceProviderSpec {
        name: "Microsoft-Windows-Audio",
        guid: MICROSOFT_WINDOWS_AUDIO_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::ManifestControl,
        enabled_in_session: true,
        selection_reason: "manifest-registered OS audio engine/audiodg correlation provider",
        require_registered: true,
    },
    BackendTraceProviderSpec {
        name: "Microsoft-Windows-KernelStreaming",
        guid: MICROSOFT_WINDOWS_KERNELSTREAMING_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::ManifestControl,
        enabled_in_session: true,
        selection_reason: "manifest-registered KernelStreaming provider relevant to PortCls streaming correlation",
        require_registered: true,
    },
    BackendTraceProviderSpec {
        name: "Microsoft-Windows-AudioEngineVerbose",
        guid: MICROSOFT_WINDOWS_AUDIOENGINEVERBOSE_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::ClassicOrWmiControl,
        enabled_in_session: true,
        selection_reason: "unmanifested audio-engine control GUID used by Microsoft core-audio trace guidance",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "PortCls-AudioEngineWmiTraceControl",
        guid: PORTCLS_AUDIOENGINE_CONTROL_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::ClassicOrWmiControl,
        enabled_in_session: true,
        selection_reason: "PortCls WMI trace control GUID: embedded in portcls.sys registration path and used by Microsoft core-audio trace guidance",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "PortCls-AudioEngine-WMIEvent",
        guid: PORTCLS_AUDIOENGINE_EVENT_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::WmiEventClass,
        enabled_in_session: true,
        selection_reason: "WMI event-class GUID for PortCls_AudioEngine / Miniport_AudioEngine_Event; enabled as a classic WMI control candidate because WMI trace-control registration may use a registered event GUID, while decode still treats it as event identity rather than a manifest provider",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "Rvf1034-ClassicSelftest-Control",
        guid: CLASSIC_SELFTEST_PROVIDER_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::ClassicSelftestControl,
        enabled_in_session: true,
        selection_reason: "test-only classic ETW provider control GUID; emits one bounded known-payload event before stream open as a user-mode capture/decode positive control",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "Rvf1034-DriverWmiProbe-Control",
        guid: DRIVER_WMI_PROBE_CONTROL_GUID,
        keyword_mask: BACKEND_TRACE_KEYWORD_MASK_ALL,
        level: BACKEND_TRACE_LEVEL,
        role: BackendTraceProviderRole::DriverWmiProbeControl,
        enabled_in_session: true,
        selection_reason: "driver-owned diagnostic WMI trace control GUID registered through IPortWMIRegistration; enable callback captures the kernel logger handle and emits bounded driver-owned positive-control/callsite events",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "PortCls-Position-WMIEvent",
        guid: PORTCLS_POSITION_EVENT_GUID,
        keyword_mask: "not-enabled-event-class",
        level: "not-enabled-event-class",
        role: BackendTraceProviderRole::WmiEventClass,
        enabled_in_session: false,
        selection_reason: "WMI event-class GUID for PortCls_Position / PC_Position; event identity only, payload schema not claimed by this decoder",
        require_registered: false,
    },
    BackendTraceProviderSpec {
        name: "PortCls-Root-WMIEvent",
        guid: PORTCLS_ROOT_EVENT_GUID,
        keyword_mask: "not-enabled-event-class",
        level: "not-enabled-event-class",
        role: BackendTraceProviderRole::WmiEventClass,
        enabled_in_session: false,
        selection_reason: "abstract root WMIEvent class GUID for PortClsEvent; inheritance root only, not a session control GUID",
        require_registered: false,
    },
];

fn rvf_position_diagnostic_data1(schema_id: u64, qpc_frequency: u64) -> u64 {
    (schema_id & 0xffff) | (qpc_frequency << RVF_POSITION_ETW_FREQUENCY_SHIFT)
}

fn rvf_position_diagnostic_data1_decode(data1: u64) -> (u64, u64) {
    (data1 & 0xffff, data1 >> RVF_POSITION_ETW_FREQUENCY_SHIFT)
}

fn rvf_position_pin_facts(device_type: u64, pin: u64, capture: bool) -> u64 {
    ((device_type & 0xffff) << 16) | ((pin & 0xff) << 8) | u64::from(capture)
}

fn rvf_position_pin_facts_decode(pin_facts: u64) -> (u64, u64, bool) {
    (
        (pin_facts >> 16) & 0xffff,
        (pin_facts >> 8) & 0xff,
        pin_facts & 1 != 0,
    )
}

fn classic_selftest_payload() -> [u8; 32] {
    let mut payload = [0_u8; 32];
    payload[..8].copy_from_slice(b"RVF1034S");
    payload[8..16].copy_from_slice(&0x1034_0000_0000_2001_u64.to_le_bytes());
    payload[16..24].copy_from_slice(&0x5e1f_0000_0000_2001_u64.to_le_bytes());
    payload[24..32].copy_from_slice(&0x0000_0000_0000_2001_u64.to_le_bytes());
    payload
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn classic_selftest_payload_hex() -> String {
    hex_encode(&classic_selftest_payload())
}

fn compact_hex_upper(hex: &str) -> String {
    let trimmed = hex.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    without_prefix
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

fn hex_contains_payload(hex: &str, expected_payload_hex: &str) -> bool {
    compact_hex_upper(hex).contains(expected_payload_hex)
}

const CLASSIC_SELFTEST_IMPLEMENTED: u32 = 1 << 0;
const CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED: u32 = 1 << 1;
const CLASSIC_SELFTEST_REGISTRATION_HANDLE_PRESENT: u32 = 1 << 2;
const CLASSIC_SELFTEST_ENABLED: u32 = 1 << 3;
const CLASSIC_SELFTEST_SESSION_HANDLE_PRESENT: u32 = 1 << 4;
const CLASSIC_SELFTEST_EMIT_ATTEMPTED: u32 = 1 << 5;
const CLASSIC_SELFTEST_UNREGISTER_ATTEMPTED: u32 = 1 << 6;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ClassicSelftestObservation {
    flags: u32,
    registration_status: Option<u32>,
    enable_callback_count: u64,
    disable_callback_count: u64,
    enable_level: Option<u8>,
    enable_flags: Option<u32>,
    emit_api_status: Option<u32>,
    emit_count: u64,
    unregister_status: Option<u32>,
    error: Option<String>,
}

impl ClassicSelftestObservation {
    fn not_attempted(reason: &str) -> Self {
        Self {
            error: Some(reason.to_owned()),
            ..Self::default()
        }
    }

    fn flag(&self, bit: u32) -> bool {
        self.flags & bit != 0
    }

    fn set_flag(&mut self, bit: u32, value: bool) {
        if value {
            self.flags |= bit;
        } else {
            self.flags &= !bit;
        }
    }
}

fn classic_selftest_observation_json(
    observation: &ClassicSelftestObservation,
) -> serde_json::Value {
    serde_json::json!({
        "implemented": observation.flag(CLASSIC_SELFTEST_IMPLEMENTED),
        "registration_attempted": observation.flag(CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED),
        "registration_status": observation.registration_status,
        "registration_handle_present": observation.flag(CLASSIC_SELFTEST_REGISTRATION_HANDLE_PRESENT),
        "enable_callback_count": observation.enable_callback_count,
        "disable_callback_count": observation.disable_callback_count,
        "enabled": observation.flag(CLASSIC_SELFTEST_ENABLED),
        "enable_level": observation.enable_level,
        "enable_flags": observation.enable_flags,
        "session_handle_present": observation.flag(CLASSIC_SELFTEST_SESSION_HANDLE_PRESENT),
        "emit_attempted": observation.flag(CLASSIC_SELFTEST_EMIT_ATTEMPTED),
        "emit_api_status": observation.emit_api_status,
        "emit_count": observation.emit_count,
        "unregister_attempted": observation.flag(CLASSIC_SELFTEST_UNREGISTER_ATTEMPTED),
        "unregister_status": observation.unregister_status,
        "error": observation.error,
    })
}

#[cfg(windows)]
mod classic_etw_selftest {
    #![expect(unsafe_code)]

    use super::{
        CLASSIC_SELFTEST_EMIT_ATTEMPTED, CLASSIC_SELFTEST_ENABLED, CLASSIC_SELFTEST_IMPLEMENTED,
        CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED, CLASSIC_SELFTEST_REGISTRATION_HANDLE_PRESENT,
        CLASSIC_SELFTEST_SESSION_HANDLE_PRESENT, CLASSIC_SELFTEST_UNREGISTER_ATTEMPTED,
        ClassicSelftestObservation, classic_selftest_payload,
    };
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::sync::Mutex;
    use std::thread;
    use std::time::{Duration, Instant};

    const ERROR_SUCCESS: u32 = 0;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    const WMI_ENABLE_EVENTS: u32 = 4;
    const WMI_DISABLE_EVENTS: u32 = 5;
    const EVENT_TRACE_TYPE_INFO: u32 = 0;
    const TRACE_LEVEL_VERBOSE: u32 = 5;
    const WNODE_FLAG_TRACED_GUID: u32 = 0x0002_0000;
    const WNODE_FLAG_USE_MOF_PTR: u32 = 0x0010_0000;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const fn guid(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Guid {
        Guid {
            data1,
            data2,
            data3,
            data4,
        }
    }

    // {6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2001}
    const CONTROL_GUID: Guid = guid(
        0x6f1d_2b3a,
        0x1034,
        0x4a7f,
        [0x9e, 0x2c, 0x5a, 0x11, 0xc0, 0xff, 0x20, 0x01],
    );
    // {6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2002}
    const EVENT_CLASS_GUID: Guid = guid(
        0x6f1d_2b3a,
        0x1034,
        0x4a7f,
        [0x9e, 0x2c, 0x5a, 0x11, 0xc0, 0xff, 0x20, 0x02],
    );

    #[repr(C)]
    struct EventTraceHeader {
        size: u16,
        field_type_flags: u16,
        class_version: u32,
        thread_id: u32,
        process_id: u32,
        time_stamp: i64,
        guid: Guid,
        client_context: u32,
        flags: u32,
    }

    #[repr(C)]
    struct MofField {
        data_ptr: u64,
        length: u32,
        data_type: u32,
    }

    #[repr(C)]
    struct SelftestEvent {
        header: EventTraceHeader,
        mof: [MofField; 1],
    }

    #[repr(C)]
    struct TraceGuidRegistration {
        guid: *const Guid,
        reg_handle: *mut c_void,
    }

    type WmiRequestCallback = extern "system" fn(u32, *mut c_void, *mut u32, *mut c_void) -> u32;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegisterTraceGuidsW(
            request_address: WmiRequestCallback,
            request_context: *mut c_void,
            control_guid: *const Guid,
            guid_count: u32,
            trace_guid_reg: *mut TraceGuidRegistration,
            mof_image_path: *const u16,
            mof_resource_name: *const u16,
            registration_handle: *mut u64,
        ) -> u32;
        fn UnregisterTraceGuids(registration_handle: u64) -> u32;
        fn GetTraceLoggerHandle(buffer: *mut c_void) -> u64;
        fn GetTraceEnableLevel(trace_handle: u64) -> u8;
        fn GetTraceEnableFlags(trace_handle: u64) -> u32;
        fn TraceEvent(trace_handle: u64, event_trace: *mut EventTraceHeader) -> u32;
    }

    #[derive(Default)]
    struct CallbackState {
        session_handle: u64,
        enable_level: u8,
        enable_flags: u32,
        trace_on: bool,
        enable_callback_count: u64,
        disable_callback_count: u64,
    }

    static CALLBACK_STATE: Mutex<CallbackState> = Mutex::new(CallbackState {
        session_handle: 0,
        enable_level: 0,
        enable_flags: 0,
        trace_on: false,
        enable_callback_count: 0,
        disable_callback_count: 0,
    });

    extern "system" fn control_callback(
        request_code: u32,
        _context: *mut c_void,
        _reserved: *mut u32,
        header: *mut c_void,
    ) -> u32 {
        let session_handle = unsafe { GetTraceLoggerHandle(header) };
        if session_handle == u64::MAX {
            return ERROR_INVALID_PARAMETER;
        }
        let Ok(mut state) = CALLBACK_STATE.lock() else {
            return ERROR_INVALID_PARAMETER;
        };
        match request_code {
            WMI_ENABLE_EVENTS => {
                state.session_handle = session_handle;
                state.enable_level = unsafe { GetTraceEnableLevel(session_handle) };
                state.enable_flags = unsafe { GetTraceEnableFlags(session_handle) };
                state.trace_on = true;
                state.enable_callback_count = state.enable_callback_count.saturating_add(1);
                ERROR_SUCCESS
            }
            WMI_DISABLE_EVENTS => {
                if state.session_handle == session_handle {
                    state.trace_on = false;
                    state.session_handle = 0;
                }
                state.disable_callback_count = state.disable_callback_count.saturating_add(1);
                ERROR_SUCCESS
            }
            _ => ERROR_INVALID_PARAMETER,
        }
    }

    pub(super) struct ClassicEtwSelftest {
        registration_handle: u64,
        session_handle: u64,
        observation: ClassicSelftestObservation,
    }

    impl ClassicEtwSelftest {
        pub(super) fn register() -> Self {
            let mut registration_handle = 0_u64;
            let mut registration = TraceGuidRegistration {
                guid: &EVENT_CLASS_GUID,
                reg_handle: std::ptr::null_mut(),
            };
            if let Ok(mut state) = CALLBACK_STATE.lock() {
                *state = CallbackState::default();
            }
            let status = unsafe {
                RegisterTraceGuidsW(
                    control_callback,
                    std::ptr::null_mut(),
                    &CONTROL_GUID,
                    1,
                    std::ptr::from_mut(&mut registration),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::from_mut(&mut registration_handle),
                )
            };
            let mut observation = ClassicSelftestObservation {
                flags: CLASSIC_SELFTEST_IMPLEMENTED | CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED,
                registration_status: Some(status),
                ..ClassicSelftestObservation::default()
            };
            observation.set_flag(
                CLASSIC_SELFTEST_REGISTRATION_HANDLE_PRESENT,
                registration_handle != 0,
            );
            if status != ERROR_SUCCESS {
                observation.error = Some(format!("RegisterTraceGuidsW failed status={status}"));
            }
            Self {
                registration_handle,
                session_handle: 0,
                observation,
            }
        }

        fn snapshot_callback_state(&mut self) {
            if let Ok(state) = CALLBACK_STATE.lock() {
                self.observation.enable_callback_count = state.enable_callback_count;
                self.observation.disable_callback_count = state.disable_callback_count;
                self.observation
                    .set_flag(CLASSIC_SELFTEST_ENABLED, state.trace_on);
                self.observation.enable_level = Some(state.enable_level);
                self.observation.enable_flags = Some(state.enable_flags);
                self.observation.set_flag(
                    CLASSIC_SELFTEST_SESSION_HANDLE_PRESENT,
                    state.session_handle != 0,
                );
                if state.trace_on && state.session_handle != 0 {
                    self.session_handle = state.session_handle;
                }
            }
        }

        pub(super) fn wait_for_enable(&mut self, timeout: Duration) {
            if self.observation.registration_status != Some(ERROR_SUCCESS) {
                return;
            }
            let start = Instant::now();
            loop {
                self.snapshot_callback_state();
                if self.observation.flag(CLASSIC_SELFTEST_ENABLED) && self.session_handle != 0 {
                    return;
                }
                if start.elapsed() >= timeout {
                    if self.observation.error.is_none() {
                        self.observation.error =
                            Some("classic selftest enable callback timeout".to_owned());
                    }
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
        }

        pub(super) fn emit_once(&mut self) {
            if self.observation.flag(CLASSIC_SELFTEST_EMIT_ATTEMPTED) {
                return;
            }
            self.observation
                .set_flag(CLASSIC_SELFTEST_EMIT_ATTEMPTED, true);
            if self.session_handle == 0 {
                self.observation.error =
                    Some("classic selftest emit skipped: no enabled session handle".to_owned());
                return;
            }
            let payload = classic_selftest_payload();
            let mut event = SelftestEvent {
                header: EventTraceHeader {
                    size: u16::try_from(size_of::<SelftestEvent>()).unwrap_or(u16::MAX),
                    field_type_flags: 0,
                    class_version: EVENT_TRACE_TYPE_INFO
                        | (TRACE_LEVEL_VERBOSE << 8)
                        | (1_u32 << 16),
                    thread_id: 0,
                    process_id: 0,
                    time_stamp: 0,
                    guid: EVENT_CLASS_GUID,
                    client_context: 0,
                    flags: WNODE_FLAG_TRACED_GUID | WNODE_FLAG_USE_MOF_PTR,
                },
                mof: [MofField {
                    data_ptr: payload.as_ptr() as u64,
                    length: u32::try_from(payload.len()).unwrap_or(u32::MAX),
                    data_type: 0,
                }],
            };
            let status =
                unsafe { TraceEvent(self.session_handle, std::ptr::from_mut(&mut event.header)) };
            self.observation.emit_api_status = Some(status);
            if status == ERROR_SUCCESS {
                self.observation.emit_count = 1;
            } else {
                self.observation.error = Some(format!("TraceEvent failed status={status}"));
            }
        }

        pub(super) fn unregister(&mut self) {
            if self.registration_handle == 0
                || self.observation.flag(CLASSIC_SELFTEST_UNREGISTER_ATTEMPTED)
            {
                return;
            }
            self.observation
                .set_flag(CLASSIC_SELFTEST_UNREGISTER_ATTEMPTED, true);
            let status = unsafe { UnregisterTraceGuids(self.registration_handle) };
            self.observation.unregister_status = Some(status);
            self.registration_handle = 0;
            self.snapshot_callback_state();
        }

        pub(super) fn observation(&self) -> ClassicSelftestObservation {
            self.observation.clone()
        }
    }

    impl Drop for ClassicEtwSelftest {
        fn drop(&mut self) {
            self.unregister();
        }
    }
}

#[cfg(not(windows))]
mod classic_etw_selftest_stub {
    use super::ClassicSelftestObservation;
    use std::time::Duration;

    pub(super) struct ClassicEtwSelftest {
        observation: ClassicSelftestObservation,
    }

    impl ClassicEtwSelftest {
        pub(super) fn register() -> Self {
            Self {
                observation: ClassicSelftestObservation {
                    error: Some("non-windows host: classic ETW selftest unsupported".to_owned()),
                    ..ClassicSelftestObservation::default()
                },
            }
        }
        pub(super) fn wait_for_enable(&mut self, _timeout: Duration) {}
        pub(super) fn emit_once(&mut self) {}
        pub(super) fn unregister(&mut self) {}
        pub(super) fn observation(&self) -> ClassicSelftestObservation {
            self.observation.clone()
        }
    }
}

#[cfg(windows)]
use classic_etw_selftest::ClassicEtwSelftest;
#[cfg(not(windows))]
use classic_etw_selftest_stub::ClassicEtwSelftest;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BackendTracePaths {
    artifact_dir: PathBuf,
    provider_discovery: PathBuf,
    provider_list: PathBuf,
    precheck_log: PathBuf,
    start_log: PathBuf,
    session_query_log: PathBuf,
    stop_log: PathBuf,
    delete_log: PathBuf,
    convert_log: PathBuf,
    etl: PathBuf,
    xml: PathBuf,
    summary_json: PathBuf,
    driver_position_events_json: PathBuf,
    driver_position_correlation_json: PathBuf,
    event_split_json: PathBuf,
    provider_logs: Vec<(String, PathBuf)>,
}

impl BackendTracePaths {
    fn new(artifact_dir: &Path, base_name: &str) -> Self {
        let path = |suffix: &str| artifact_dir.join(format!("{base_name}{suffix}"));
        let provider_logs = BACKEND_TRACE_PROVIDERS
            .iter()
            .map(|provider| {
                (
                    provider.name.to_owned(),
                    path(&format!(
                        "-provider-{}.txt",
                        backend_trace_file_component(provider.name)
                    )),
                )
            })
            .collect();
        Self {
            artifact_dir: artifact_dir.to_path_buf(),
            provider_discovery: path("-provider-discovery.txt"),
            provider_list: path("-provider-list.txt"),
            precheck_log: path("-precheck-log.txt"),
            start_log: path("-start-log.txt"),
            session_query_log: path("-session-query-log.txt"),
            stop_log: path("-stop-log.txt"),
            delete_log: path("-delete-log.txt"),
            convert_log: path("-tracerpt-log.txt"),
            etl: path(".etl"),
            xml: path(".xml"),
            summary_json: path("-summary.json"),
            driver_position_events_json: path("-driver-position-events.json"),
            driver_position_correlation_json: path("-driver-position-correlation.json"),
            event_split_json: path("-event-split.json"),
            provider_logs,
        }
    }
}

fn backend_trace_file_component(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn backend_trace_session_name(pair_index: usize) -> String {
    format!("rvf1034_bt_{}_{}", process::id(), pair_index)
}

fn backend_trace_base_name(pair_index: usize) -> String {
    format!("backend-os-trace-pair{pair_index}-{}", process::id())
}

fn discover_backend_trace_artifact_dir(
    github_workspace: Option<&Path>,
    cargo_target_tmpdir: Option<&Path>,
) -> Result<(PathBuf, &'static str), String> {
    if let Some(workspace) = github_workspace
        && workspace.is_dir()
    {
        let mut candidates = fs::read_dir(workspace)
            .map_err(|error| format!("GITHUB_WORKSPACE read_dir failed: {error}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("os-endpoint-run-metadata-"))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        if let Some(candidate) = candidates.into_iter().next() {
            return Ok((candidate, "github-workspace-os-endpoint-run-metadata"));
        }
    }
    if let Some(tmpdir) = cargo_target_tmpdir
        && tmpdir.is_dir()
    {
        return Ok((tmpdir.to_path_buf(), "cargo-target-tmpdir"));
    }
    Err("no artifact directory: no existing os-endpoint-run-metadata-* under GITHUB_WORKSPACE and no CARGO_TARGET_TMPDIR".to_owned())
}

fn backend_trace_artifact_dir_from_env() -> Result<(PathBuf, &'static str), String> {
    let workspace = std::env::var_os("GITHUB_WORKSPACE").map(PathBuf::from);
    let target_tmp = std::env::var("CARGO_TARGET_TMPDIR")
        .ok()
        .or_else(|| option_env!("CARGO_TARGET_TMPDIR").map(str::to_owned))
        .map(PathBuf::from);
    discover_backend_trace_artifact_dir(workspace.as_deref(), target_tmp.as_deref())
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct BackendTraceCalibration {
    host_monotonic_ns: u64,
    system_time_unix_ns: u64,
}

fn backend_trace_calibration(host_origin: Instant) -> BackendTraceCalibration {
    let host_monotonic_ns =
        u64::try_from(host_origin.elapsed().as_nanos().min(u128::from(u64::MAX)))
            .unwrap_or(u64::MAX);
    let system_time_unix_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_nanos().min(u128::from(u64::MAX))).unwrap_or(u64::MAX)
        });
    BackendTraceCalibration {
        host_monotonic_ns,
        system_time_unix_ns,
    }
}

#[derive(Clone, Debug)]
struct BackendTraceCommandObservation {
    label: String,
    program: String,
    args: Vec<String>,
    exit_code: Option<i32>,
    success: bool,
    error: Option<String>,
    log_file: Option<PathBuf>,
}

struct BackendTraceCommandOutput {
    exit_code: Option<i32>,
    success: bool,
    error: Option<String>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn write_artifact_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("write {} failed: {error}", path.display()))
}

fn spawn_pipe_reader(mut pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = pipe.as_mut() {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

fn wait_child_with_timeout(
    child: &mut process::Child,
    timeout: Duration,
) -> (Option<process::ExitStatus>, Option<String>) {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return (Some(status), None),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let status = child.wait().ok();
                return (
                    status,
                    Some(format!("command timed out after {}s", timeout.as_secs())),
                );
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(wait_error) => {
                let _ = child.kill();
                let _ = child.wait();
                return (None, Some(format!("wait failed: {wait_error}")));
            }
        }
    }
}

fn run_process_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> BackendTraceCommandOutput {
    let spawn_result = process::Command::new(program)
        .args(args)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn();
    let mut child = match spawn_result {
        Ok(child) => child,
        Err(spawn_error) => {
            return BackendTraceCommandOutput {
                exit_code: None,
                success: false,
                error: Some(spawn_error.to_string()),
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
        }
    };
    let stdout_reader = spawn_pipe_reader(child.stdout.take());
    let stderr_reader = spawn_pipe_reader(child.stderr.take());
    let (status, error) = wait_child_with_timeout(&mut child, timeout);
    BackendTraceCommandOutput {
        exit_code: status.and_then(|status| status.code()),
        success: error.is_none() && status.is_some_and(|status| status.success()),
        error,
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
    }
}

fn run_backend_trace_command(
    label: &str,
    program: &str,
    args: &[&str],
    log_file: Option<&Path>,
) -> BackendTraceCommandObservation {
    let rendered_command = format!(
        "{} {}",
        program,
        args.iter()
            .map(|arg| format!("\"{arg}\""))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let timeout = if label == "tracerpt_xml" {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(20)
    };
    let output = run_process_with_timeout(program, args, timeout);
    if let Some(path) = log_file {
        let mut bytes = format!("$ {rendered_command}\n").into_bytes();
        if let Some(code) = output.exit_code {
            bytes.extend_from_slice(format!("exit_code={code}\n").as_bytes());
        } else {
            bytes.extend_from_slice(b"exit_code=spawn-error\n");
        }
        if let Some(error) = &output.error {
            bytes.extend_from_slice(format!("spawn_error={error}\n").as_bytes());
        }
        bytes.extend_from_slice(b"--- stdout ---\n");
        bytes.extend_from_slice(&output.stdout);
        bytes.extend_from_slice(b"\n--- stderr ---\n");
        bytes.extend_from_slice(&output.stderr);
        let _ = write_artifact_bytes(path, &bytes);
    }
    BackendTraceCommandObservation {
        label: label.to_owned(),
        program: program.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        exit_code: output.exit_code,
        success: output.success,
        error: output.error,
        log_file: log_file.map(Path::to_path_buf),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct PortClsMiniportEvent {
    trace_event_type: u64,
    miniport_event_ordinal: u64,
    event_kind: String,
    data1: u64,
    data2: u64,
    data3: u64,
    data4: u64,
    system_time: Option<String>,
    provider_guid: Option<String>,
    event_guid: Option<String>,
    level: Option<u64>,
    opcode: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct RvfDriverPositionEvent {
    source_event_index: usize,
    schema_id: u64,
    frequency_hz: u64,
    capture: bool,
    pin: u64,
    device_type: u64,
    presentation_position_blocks: u64,
    raw_qpc: u64,
    raw_qpc_ns: Option<u64>,
    system_time: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct DriverWmiProbeEvent {
    schema_version: u32,
    kind: u32,
    sequence: u64,
    status: u32,
    flags: u32,
    miniport_event_ordinal: u32,
    miniport_schema_id: u32,
    data1: u64,
    data2: u64,
    data3: u64,
    data4: u64,
    write_etw_event_calls: u64,
    helper_null_calls: u64,
    forward_error_calls: u64,
    mirror_suppressed_calls: u64,
    mirror_key_calls: u64,
    mirror_key_emit_calls: u64,
    system_time: Option<String>,
    provider_guid: Option<String>,
    event_guid: Option<String>,
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let raw = bytes.get(offset..end)?;
    Some(u32::from_le_bytes(raw.try_into().ok()?))
}

fn decode_driver_wmi_probe_payload(
    hex: &str,
    system_time: Option<String>,
    provider_guid: Option<String>,
    event_guid: Option<String>,
) -> Option<DriverWmiProbeEvent> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes = decode_hex_bytes(hex)?;
    if bytes.len() != 120 || bytes.get(..8)? != b"RVF1034W" {
        return None;
    }
    let schema_version = read_le_u32(&bytes, 8)?;
    if schema_version != 1 {
        return None;
    }
    Some(DriverWmiProbeEvent {
        schema_version,
        kind: read_le_u32(&bytes, 12)?,
        sequence: read_le_u64(&bytes, 16)?,
        status: read_le_u32(&bytes, 24)?,
        flags: read_le_u32(&bytes, 28)?,
        miniport_event_ordinal: read_le_u32(&bytes, 32)?,
        miniport_schema_id: read_le_u32(&bytes, 36)?,
        data1: read_le_u64(&bytes, 40)?,
        data2: read_le_u64(&bytes, 48)?,
        data3: read_le_u64(&bytes, 56)?,
        data4: read_le_u64(&bytes, 64)?,
        write_etw_event_calls: read_le_u64(&bytes, 72)?,
        helper_null_calls: read_le_u64(&bytes, 80)?,
        forward_error_calls: read_le_u64(&bytes, 88)?,
        mirror_suppressed_calls: read_le_u64(&bytes, 96)?,
        mirror_key_calls: read_le_u64(&bytes, 104)?,
        mirror_key_emit_calls: read_le_u64(&bytes, 112)?,
        system_time,
        provider_guid,
        event_guid,
    })
}

#[derive(Clone, Debug, Default)]
struct BackendTraceXmlInventory {
    events_total: u64,
    audio_provider_events: u64,
    kernel_streaming_provider_events: u64,
    audio_engine_provider_events: u64,
    portcls_provider_events: u64,
    other_provider_events: u64,
    kernel_eventtracing_events: u64,
    kernel_eventtracing_enable_events: u64,
    kernel_eventtracing_disable_events: u64,
    kernel_eventtracing_enable_opcode18_events: u64,
    kernel_eventtracing_disable_opcode19_events: u64,
    kernel_eventtracing_raw_message_available: u64,
    kernel_witness_matched_session_enable_events: u64,
    kernel_witness_matched_session_disable_events: u64,
    kernel_witness_portcls_control_enable_events: u64,
    kernel_witness_portcls_control_disable_events: u64,
    kernel_witness_portcls_event_enable_events: u64,
    kernel_witness_portcls_event_disable_events: u64,
    kernel_witness_selftest_enable_events: u64,
    kernel_witness_selftest_disable_events: u64,
    classic_selftest_events: u64,
    classic_selftest_identity_match_events: u64,
    classic_selftest_payload_match_events: u64,
    classic_selftest_missing_payload_events: u64,
    classic_selftest_payload_mismatch_events: u64,
    kernel_witness_driver_probe_enable_events: u64,
    kernel_witness_driver_probe_disable_events: u64,
    driver_wmi_probe_events: u64,
    driver_wmi_probe_identity_match_events: u64,
    driver_wmi_probe_payload_match_events: u64,
    driver_wmi_probe_missing_payload_events: u64,
    driver_wmi_probe_payload_mismatch_events: u64,
    driver_wmi_probe_control_enable_events: u64,
    driver_wmi_probe_control_disable_events: u64,
    driver_wmi_probe_write_mirror_events: u64,
    driver_wmi_probe_event_overflow: u64,
    driver_wmi_probe_mirror_ordinal_counts: [u64; 16],
    driver_wmi_probe_control_enable_snapshot: Option<DriverWmiProbeEvent>,
    driver_wmi_probe_control_disable_snapshot: Option<DriverWmiProbeEvent>,
    driver_wmi_probe_stored_events: Vec<DriverWmiProbeEvent>,
    ae_position_events: u64,
    set_current_timestamp_events: u64,
    glitch_events: u64,
    binary_event_data_events: u64,
    miniport_events_total: u64,
    portcls_events_missing_binary_payload: u64,
    undecoded_portcls_binary_events: u64,
    unmodeled_portcls_event_class_events: u64,
    invalid_rvf_position_events: u64,
    driver_position_event_overflow: u64,
    rvf_position_events: u64,
    standard_presentation_position_events: u64,
    miniport_ordinal_counts: [u64; 11],
    rvf_xrun_schema_1001_events: u64,
    rvf_xrun_schema_1002_events: u64,
    rvf_xrun_schema_1003_events: u64,
    rvf_xrun_schema_1004_events: u64,
    rvf_xrun_schema_1005_events: u64,
    miniport_events: Vec<PortClsMiniportEvent>,
    rvf_driver_position_events: Vec<RvfDriverPositionEvent>,
    perf_freq: Option<u64>,
    start_time: Option<u64>,
    decode_error: Option<String>,
}

fn extract_trace_data_text<'a>(chunk: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("<Data Name=\"{name}\"");
    let start = chunk.find(&needle)? + needle.len();
    let tail = &chunk[start..];
    let content_start = tail.find('>')? + 1;
    let content_tail = &tail[content_start..];
    let end = content_tail.find("</Data>")?;
    Some(content_tail[..end].trim())
}

fn extract_trace_data_u64(chunk: &str, name: &str) -> Option<u64> {
    let raw = extract_trace_data_text(chunk, name)?;
    if let Some(hex) = raw.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u64>().ok()
    }
}

fn extract_xml_element_text<'a>(chunk: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let start = chunk.find(&open)? + open.len();
    let tail = &chunk[start..];
    let end = tail.find(&format!("</{tag}>"))?;
    Some(tail[..end].trim())
}

fn extract_xml_element_u64(chunk: &str, tag: &str) -> Option<u64> {
    extract_xml_element_text(chunk, tag)?.parse::<u64>().ok()
}

fn extract_xml_attribute(chunk: &str, element: &str, attribute: &str) -> Option<String> {
    let element_start = chunk.find(&format!("<{element}"))?;
    let tail = &chunk[element_start..];
    let element_end = tail.find('>')?;
    let header = &tail[..element_end];
    let needle = format!("{attribute}=\"");
    let value_start = header.find(&needle)? + needle.len();
    let value_tail = &header[value_start..];
    let value_end = value_tail.find('"')?;
    Some(value_tail[..value_end].to_owned())
}

fn decode_hex_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || hex.is_empty() {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok())
        .collect()
}

fn read_le_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let raw = bytes.get(offset..end)?;
    Some(u64::from_le_bytes(raw.try_into().ok()?))
}

fn portcls_miniport_event_kind(ordinal: u64) -> &'static str {
    match ordinal {
        0 => "eMINIPORT_IHV_DEFINED",
        1 => "eMINIPORT_BUFFER_COMPLETE",
        2 => "eMINIPORT_PIN_STATE",
        3 => "eMINIPORT_GET_STREAM_POSITION",
        4 => "eMINIPORT_SET_WAVERT_BUFFER_WRITE_POSITION",
        5 => "eMINIPORT_GET_PRESENTATION_POSITION",
        6 => "eMINIPORT_PROGRAM_DMA",
        7 => "eMINIPORT_GLITCH_REPORT",
        8 => "eMINIPORT_LAST_BUFFER_RENDERED",
        9 => "eMINIPORT_PROCESSING_MODE",
        10 => "eMINIPORT_FX_CLSID",
        _ => "unknown",
    }
}

fn decode_portcls_miniport_payload(
    hex: &str,
    system_time: Option<String>,
    provider_guid: Option<String>,
    event_guid: Option<String>,
    level: Option<u64>,
    opcode: Option<u64>,
) -> Option<PortClsMiniportEvent> {
    let bytes = decode_hex_bytes(hex)?;
    if bytes.len() < PORTCLS_MINIPORT_PAYLOAD_U64S * 8 {
        return None;
    }
    let trace_event_type = read_le_u64(&bytes, 0)?;
    let miniport_event_ordinal = trace_event_type.checked_sub(PORTCLS_MINIPORT_TRACE_TYPE_BASE)?;
    if miniport_event_ordinal > PORTCLS_MINIPORT_MAX_ORDINAL {
        return None;
    }
    Some(PortClsMiniportEvent {
        trace_event_type,
        miniport_event_ordinal,
        event_kind: portcls_miniport_event_kind(miniport_event_ordinal).to_owned(),
        data1: read_le_u64(&bytes, 8)?,
        data2: read_le_u64(&bytes, 16)?,
        data3: read_le_u64(&bytes, 24)?,
        data4: read_le_u64(&bytes, 32)?,
        system_time,
        provider_guid,
        event_guid,
        level,
        opcode,
    })
}

fn qpc_ticks_to_ns(raw_qpc: u64, frequency_hz: u64) -> Option<u64> {
    if frequency_hz == 0 {
        return None;
    }
    let nanos = u128::from(raw_qpc).saturating_mul(1_000_000_000) / u128::from(frequency_hz);
    u64::try_from(nanos).ok()
}

fn rvf_driver_position_from_miniport(
    source_event_index: usize,
    event: &PortClsMiniportEvent,
) -> Option<RvfDriverPositionEvent> {
    if event.miniport_event_ordinal != 0 {
        return None;
    }
    let (schema_id, frequency_hz) = rvf_position_diagnostic_data1_decode(event.data1);
    if schema_id != RVF_POSITION_ETW_PRESENTATION_POSITION_V1
        || !(100_000..=100_000_000).contains(&frequency_hz)
    {
        return None;
    }
    let (device_type, pin, capture) = rvf_position_pin_facts_decode(event.data2);
    Some(RvfDriverPositionEvent {
        source_event_index,
        schema_id,
        frequency_hz,
        capture,
        pin,
        device_type,
        presentation_position_blocks: event.data3,
        raw_qpc: event.data4,
        raw_qpc_ns: qpc_ticks_to_ns(event.data4, frequency_hz),
        system_time: event.system_time.clone(),
    })
}

fn guid_eq(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn analyze_portcls_event_chunk(
    chunk: &str,
    binary_hex: Option<&str>,
    inventory: &mut BackendTraceXmlInventory,
) {
    let system_time = extract_xml_attribute(chunk, "TimeCreated", "SystemTime");
    let provider_guid = extract_xml_attribute(chunk, "Provider", "Guid");
    let event_guid = extract_xml_element_text(chunk, "EventGuid").map(str::to_owned);
    let level = extract_xml_element_u64(chunk, "Level");
    let opcode = extract_xml_element_u64(chunk, "Opcode");
    let audioengine_event_identity = if event_guid.is_some() {
        guid_eq(event_guid.as_deref(), PORTCLS_AUDIOENGINE_EVENT_GUID)
    } else {
        guid_eq(provider_guid.as_deref(), PORTCLS_AUDIOENGINE_CONTROL_GUID)
            || guid_eq(provider_guid.as_deref(), PORTCLS_AUDIOENGINE_EVENT_GUID)
    };
    if !audioengine_event_identity {
        inventory.unmodeled_portcls_event_class_events = inventory
            .unmodeled_portcls_event_class_events
            .saturating_add(1);
        return;
    }
    let Some(binary_hex) = binary_hex else {
        inventory.portcls_events_missing_binary_payload = inventory
            .portcls_events_missing_binary_payload
            .saturating_add(1);
        return;
    };
    let Some(event) = decode_portcls_miniport_payload(
        binary_hex,
        system_time,
        provider_guid,
        event_guid,
        level,
        opcode,
    ) else {
        inventory.undecoded_portcls_binary_events =
            inventory.undecoded_portcls_binary_events.saturating_add(1);
        return;
    };
    inventory.miniport_events_total = inventory.miniport_events_total.saturating_add(1);
    if let Some(count) = inventory
        .miniport_ordinal_counts
        .get_mut(usize::try_from(event.miniport_event_ordinal).unwrap_or(usize::MAX))
    {
        *count = count.saturating_add(1);
    }
    if event.miniport_event_ordinal == 5 {
        inventory.standard_presentation_position_events = inventory
            .standard_presentation_position_events
            .saturating_add(1);
    }
    let source_event_index = inventory.miniport_events.len();
    if event.miniport_event_ordinal == 0 {
        let schema_id = event.data1 & 0xffff;
        match schema_id {
            RVF_XRUN_ETW_SCENARIO_SNAPSHOT => {
                inventory.rvf_xrun_schema_1001_events =
                    inventory.rvf_xrun_schema_1001_events.saturating_add(1);
            }
            RVF_XRUN_ETW_ZERO_RANGE => {
                inventory.rvf_xrun_schema_1002_events =
                    inventory.rvf_xrun_schema_1002_events.saturating_add(1);
            }
            RVF_XRUN_ETW_SUPPRESSION_TRANSITION => {
                inventory.rvf_xrun_schema_1003_events =
                    inventory.rvf_xrun_schema_1003_events.saturating_add(1);
            }
            RVF_XRUN_ETW_ZERO_FILL_BOUNDARY => {
                inventory.rvf_xrun_schema_1004_events =
                    inventory.rvf_xrun_schema_1004_events.saturating_add(1);
            }
            RVF_XRUN_ETW_OS_PACKET_GAP => {
                inventory.rvf_xrun_schema_1005_events =
                    inventory.rvf_xrun_schema_1005_events.saturating_add(1);
            }
            _ => {}
        }
        if let Some(driver_event) = rvf_driver_position_from_miniport(source_event_index, &event) {
            inventory.rvf_position_events = inventory.rvf_position_events.saturating_add(1);
            if inventory.rvf_driver_position_events.len() < DRIVER_POSITION_EVENT_LIMIT {
                inventory.rvf_driver_position_events.push(driver_event);
            } else {
                inventory.driver_position_event_overflow =
                    inventory.driver_position_event_overflow.saturating_add(1);
            }
        } else if !(RVF_XRUN_ETW_SCENARIO_SNAPSHOT..=RVF_XRUN_ETW_OS_PACKET_GAP)
            .contains(&schema_id)
        {
            inventory.invalid_rvf_position_events =
                inventory.invalid_rvf_position_events.saturating_add(1);
        }
    }
    if inventory.miniport_events.len() < DRIVER_POSITION_EVENT_LIMIT {
        inventory.miniport_events.push(event);
    } else {
        inventory.driver_position_event_overflow =
            inventory.driver_position_event_overflow.saturating_add(1);
    }
}

fn record_kernel_eventtracing_witness(
    enabled: bool,
    witnessed_provider: Option<&str>,
    inventory: &mut BackendTraceXmlInventory,
) {
    let is_portcls_control = guid_eq(witnessed_provider, PORTCLS_AUDIOENGINE_CONTROL_GUID);
    let is_portcls_event = guid_eq(witnessed_provider, PORTCLS_AUDIOENGINE_EVENT_GUID);
    let is_selftest = guid_eq(witnessed_provider, CLASSIC_SELFTEST_PROVIDER_GUID);
    let is_driver_probe = guid_eq(witnessed_provider, DRIVER_WMI_PROBE_CONTROL_GUID);
    if is_portcls_control {
        if enabled {
            inventory.kernel_witness_portcls_control_enable_events = inventory
                .kernel_witness_portcls_control_enable_events
                .saturating_add(1);
        } else {
            inventory.kernel_witness_portcls_control_disable_events = inventory
                .kernel_witness_portcls_control_disable_events
                .saturating_add(1);
        }
    } else if is_portcls_event {
        if enabled {
            inventory.kernel_witness_portcls_event_enable_events = inventory
                .kernel_witness_portcls_event_enable_events
                .saturating_add(1);
        } else {
            inventory.kernel_witness_portcls_event_disable_events = inventory
                .kernel_witness_portcls_event_disable_events
                .saturating_add(1);
        }
    } else if is_selftest {
        if enabled {
            inventory.kernel_witness_selftest_enable_events = inventory
                .kernel_witness_selftest_enable_events
                .saturating_add(1);
        } else {
            inventory.kernel_witness_selftest_disable_events = inventory
                .kernel_witness_selftest_disable_events
                .saturating_add(1);
        }
    } else if is_driver_probe {
        if enabled {
            inventory.kernel_witness_driver_probe_enable_events = inventory
                .kernel_witness_driver_probe_enable_events
                .saturating_add(1);
        } else {
            inventory.kernel_witness_driver_probe_disable_events = inventory
                .kernel_witness_driver_probe_disable_events
                .saturating_add(1);
        }
    }
}

fn analyze_kernel_eventtracing_chunk(
    chunk: &str,
    inventory: &mut BackendTraceXmlInventory,
    session_name: &str,
) {
    let provider_guid = extract_xml_attribute(chunk, "Provider", "Guid");
    let lower = chunk.to_ascii_lowercase();
    if !guid_eq(
        provider_guid.as_deref(),
        MICROSOFT_WINDOWS_KERNEL_EVENTTRACING_GUID,
    ) && !lower.contains("microsoft-windows-kernel-eventtracing")
    {
        return;
    }
    inventory.kernel_eventtracing_events = inventory.kernel_eventtracing_events.saturating_add(1);
    let Some(event_id) = extract_xml_element_u64(chunk, "EventID") else {
        return;
    };
    if event_id != KERNEL_EVENTTRACING_ENABLE_EVENT_ID
        && event_id != KERNEL_EVENTTRACING_DISABLE_EVENT_ID
    {
        return;
    }
    let opcode = extract_xml_element_u64(chunk, "Opcode");
    let enabled = event_id == KERNEL_EVENTTRACING_ENABLE_EVENT_ID;
    if enabled {
        inventory.kernel_eventtracing_enable_events = inventory
            .kernel_eventtracing_enable_events
            .saturating_add(1);
        if opcode == Some(KERNEL_EVENTTRACING_ENABLE_OPCODE) {
            inventory.kernel_eventtracing_enable_opcode18_events = inventory
                .kernel_eventtracing_enable_opcode18_events
                .saturating_add(1);
        }
    } else {
        inventory.kernel_eventtracing_disable_events = inventory
            .kernel_eventtracing_disable_events
            .saturating_add(1);
        if opcode == Some(KERNEL_EVENTTRACING_DISABLE_OPCODE) {
            inventory.kernel_eventtracing_disable_opcode19_events = inventory
                .kernel_eventtracing_disable_opcode19_events
                .saturating_add(1);
        }
    }
    if chunk.contains("<RenderingInfo") || chunk.contains("<Message") {
        inventory.kernel_eventtracing_raw_message_available = inventory
            .kernel_eventtracing_raw_message_available
            .saturating_add(1);
    }
    let witnessed_provider = extract_trace_data_text(chunk, "ProviderName");
    let witnessed_session = extract_trace_data_text(chunk, "SessionName");
    if witnessed_session.is_none_or(|value| value != session_name) {
        return;
    }
    if enabled {
        inventory.kernel_witness_matched_session_enable_events = inventory
            .kernel_witness_matched_session_enable_events
            .saturating_add(1);
    } else {
        inventory.kernel_witness_matched_session_disable_events = inventory
            .kernel_witness_matched_session_disable_events
            .saturating_add(1);
    }
    record_kernel_eventtracing_witness(enabled, witnessed_provider, inventory);
}

fn analyze_classic_selftest_chunk(
    chunk: &str,
    binary_hex: Option<&str>,
    inventory: &mut BackendTraceXmlInventory,
) {
    let provider_guid = extract_xml_attribute(chunk, "Provider", "Guid");
    let event_guid = extract_xml_element_text(chunk, "EventGuid").map(str::to_owned);
    if !guid_eq(provider_guid.as_deref(), CLASSIC_SELFTEST_PROVIDER_GUID)
        && !guid_eq(event_guid.as_deref(), CLASSIC_SELFTEST_EVENT_CLASS_GUID)
    {
        return;
    }
    inventory.classic_selftest_events = inventory.classic_selftest_events.saturating_add(1);
    inventory.classic_selftest_identity_match_events = inventory
        .classic_selftest_identity_match_events
        .saturating_add(1);
    let payload_hex = binary_hex.or_else(|| extract_trace_data_text(chunk, "win:ExtraPayload"));
    let Some(payload_hex) = payload_hex else {
        inventory.classic_selftest_missing_payload_events = inventory
            .classic_selftest_missing_payload_events
            .saturating_add(1);
        return;
    };
    if hex_contains_payload(payload_hex, &classic_selftest_payload_hex()) {
        inventory.classic_selftest_payload_match_events = inventory
            .classic_selftest_payload_match_events
            .saturating_add(1);
    } else {
        inventory.classic_selftest_payload_mismatch_events = inventory
            .classic_selftest_payload_mismatch_events
            .saturating_add(1);
    }
}

fn analyze_driver_wmi_probe_chunk(
    chunk: &str,
    binary_hex: Option<&str>,
    inventory: &mut BackendTraceXmlInventory,
) {
    let provider_guid = extract_xml_attribute(chunk, "Provider", "Guid");
    let event_guid = extract_xml_element_text(chunk, "EventGuid").map(str::to_owned);
    if !guid_eq(provider_guid.as_deref(), DRIVER_WMI_PROBE_CONTROL_GUID)
        && !guid_eq(provider_guid.as_deref(), DRIVER_WMI_PROBE_EVENT_GUID)
        && !guid_eq(event_guid.as_deref(), DRIVER_WMI_PROBE_EVENT_GUID)
    {
        return;
    }
    inventory.driver_wmi_probe_events = inventory.driver_wmi_probe_events.saturating_add(1);
    inventory.driver_wmi_probe_identity_match_events = inventory
        .driver_wmi_probe_identity_match_events
        .saturating_add(1);
    let payload_hex = binary_hex.or_else(|| extract_trace_data_text(chunk, "win:ExtraPayload"));
    let Some(payload_hex) = payload_hex else {
        inventory.driver_wmi_probe_missing_payload_events = inventory
            .driver_wmi_probe_missing_payload_events
            .saturating_add(1);
        return;
    };
    let system_time = extract_xml_attribute(chunk, "TimeCreated", "SystemTime");
    let Some(event) =
        decode_driver_wmi_probe_payload(payload_hex, system_time, provider_guid, event_guid)
    else {
        inventory.driver_wmi_probe_payload_mismatch_events = inventory
            .driver_wmi_probe_payload_mismatch_events
            .saturating_add(1);
        return;
    };
    inventory.driver_wmi_probe_payload_match_events = inventory
        .driver_wmi_probe_payload_match_events
        .saturating_add(1);
    match event.kind {
        1 => {
            inventory.driver_wmi_probe_control_enable_events = inventory
                .driver_wmi_probe_control_enable_events
                .saturating_add(1);
            if inventory.driver_wmi_probe_control_enable_snapshot.is_none() {
                inventory.driver_wmi_probe_control_enable_snapshot = Some(event.clone());
            }
        }
        2 => {
            inventory.driver_wmi_probe_control_disable_events = inventory
                .driver_wmi_probe_control_disable_events
                .saturating_add(1);
            inventory.driver_wmi_probe_control_disable_snapshot = Some(event.clone());
        }
        3 => {
            inventory.driver_wmi_probe_write_mirror_events = inventory
                .driver_wmi_probe_write_mirror_events
                .saturating_add(1);
            if (event.miniport_event_ordinal as usize) < 16 {
                inventory.driver_wmi_probe_mirror_ordinal_counts
                    [event.miniport_event_ordinal as usize] += 1;
            }
        }
        _ => {}
    }
    if inventory.driver_wmi_probe_stored_events.len() < DRIVER_WMI_PROBE_EVENT_LIMIT {
        inventory.driver_wmi_probe_stored_events.push(event);
    } else {
        inventory.driver_wmi_probe_event_overflow =
            inventory.driver_wmi_probe_event_overflow.saturating_add(1);
    }
}

fn analyze_backend_trace_event_chunk_for_session(
    chunk: &str,
    inventory: &mut BackendTraceXmlInventory,
    session_name: &str,
) {
    inventory.events_total = inventory.events_total.saturating_add(1);
    let lower = chunk.to_ascii_lowercase();
    let audio_guid = MICROSOFT_WINDOWS_AUDIO_GUID.to_ascii_lowercase();
    let kernel_guid = MICROSOFT_WINDOWS_KERNELSTREAMING_GUID.to_ascii_lowercase();
    let audio_engine_guid = MICROSOFT_WINDOWS_AUDIOENGINEVERBOSE_GUID.to_ascii_lowercase();
    let provider_guid = extract_xml_attribute(chunk, "Provider", "Guid");
    let event_guid = extract_xml_element_text(chunk, "EventGuid").map(str::to_owned);
    let is_portcls = [
        PORTCLS_AUDIOENGINE_CONTROL_GUID,
        PORTCLS_AUDIOENGINE_EVENT_GUID,
        PORTCLS_POSITION_EVENT_GUID,
        PORTCLS_ROOT_EVENT_GUID,
    ]
    .iter()
    .any(|guid| guid_eq(provider_guid.as_deref(), guid) || guid_eq(event_guid.as_deref(), guid));
    if is_portcls {
        inventory.portcls_provider_events = inventory.portcls_provider_events.saturating_add(1);
    } else if lower.contains(&audio_engine_guid) || lower.contains("audioengine") {
        inventory.audio_engine_provider_events =
            inventory.audio_engine_provider_events.saturating_add(1);
    } else if lower.contains(&audio_guid) || lower.contains("microsoft-windows-audio") {
        inventory.audio_provider_events = inventory.audio_provider_events.saturating_add(1);
    } else if lower.contains(&kernel_guid) || lower.contains("microsoft-windows-kernelstreaming") {
        inventory.kernel_streaming_provider_events =
            inventory.kernel_streaming_provider_events.saturating_add(1);
    } else {
        inventory.other_provider_events = inventory.other_provider_events.saturating_add(1);
    }
    if lower.contains("aeposition") {
        inventory.ae_position_events = inventory.ae_position_events.saturating_add(1);
    }
    if lower.contains("setcurrenttimestamp") {
        inventory.set_current_timestamp_events =
            inventory.set_current_timestamp_events.saturating_add(1);
    }
    if lower.contains("glitch") {
        inventory.glitch_events = inventory.glitch_events.saturating_add(1);
    }

    let binary_hex = extract_xml_element_text(chunk, "BinaryEventData");
    if binary_hex.is_some() {
        inventory.binary_event_data_events = inventory.binary_event_data_events.saturating_add(1);
    }
    analyze_kernel_eventtracing_chunk(chunk, inventory, session_name);
    analyze_classic_selftest_chunk(chunk, binary_hex, inventory);
    analyze_driver_wmi_probe_chunk(chunk, binary_hex, inventory);
    if is_portcls {
        analyze_portcls_event_chunk(chunk, binary_hex, inventory);
    }
    if inventory.perf_freq.is_none() {
        inventory.perf_freq = extract_trace_data_u64(chunk, "PerfFreq");
    }
    if inventory.start_time.is_none() {
        inventory.start_time = extract_trace_data_u64(chunk, "StartTime");
    }
}

fn analyze_backend_trace_event_chunk(chunk: &str, inventory: &mut BackendTraceXmlInventory) {
    analyze_backend_trace_event_chunk_for_session(chunk, inventory, "");
}

fn scan_backend_trace_xml(
    path: &Path,
    session_name: &str,
) -> Result<BackendTraceXmlInventory, String> {
    let file =
        fs::File::open(path).map_err(|error| format!("open {} failed: {error}", path.display()))?;
    let reader = BufReader::new(file);
    let mut inventory = BackendTraceXmlInventory::default();
    let mut chunk = String::new();
    let mut in_event = false;
    for line in reader.lines() {
        let line = line.map_err(|error| format!("read {} failed: {error}", path.display()))?;
        if line.starts_with("<Event ") {
            in_event = true;
            chunk.clear();
        }
        if in_event {
            chunk.push_str(&line);
            chunk.push('\n');
        }
        if in_event && line.starts_with("</Event>") {
            analyze_backend_trace_event_chunk_for_session(&chunk, &mut inventory, session_name);
            in_event = false;
            chunk.clear();
        }
    }
    Ok(inventory)
}

fn backend_trace_driver_coverage(
    inventory: &BackendTraceXmlInventory,
    xml_present: bool,
) -> &'static str {
    if inventory.rvf_position_events != 0 {
        "driver-diagnostic-event-decoded"
    } else if inventory.standard_presentation_position_events != 0 {
        "standard-miniport-event-decoded"
    } else if inventory.miniport_events_total != 0 {
        "miniport-event-decoded-no-position-schema"
    } else if inventory.portcls_provider_events != 0 {
        "incomplete-portcls-event-present-undecoded"
    } else if xml_present {
        "incomplete-no-driver-event-decoded"
    } else {
        "unavailable-no-xml"
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct EventSplitStatuses {
    control_witness: &'static str,
    classic_selftest: &'static str,
    miniport_lifecycle: &'static str,
    position_event: &'static str,
    verdict: &'static str,
}

fn control_witness_status(inventory: &BackendTraceXmlInventory) -> &'static str {
    if inventory.kernel_witness_portcls_control_enable_events != 0
        || inventory.kernel_witness_portcls_event_enable_events != 0
    {
        "control-request-witnessed"
    } else if inventory.kernel_eventtracing_events == 0 {
        "control-request-unresolved"
    } else {
        "control-request-witness-absent"
    }
}

fn classic_selftest_status(
    observation: &ClassicSelftestObservation,
    inventory: &BackendTraceXmlInventory,
) -> &'static str {
    if inventory.classic_selftest_identity_match_events != 0
        && inventory.classic_selftest_payload_match_events != 0
    {
        return "classic-selftest-decoded";
    }
    if !observation.flag(CLASSIC_SELFTEST_IMPLEMENTED) {
        return "classic-selftest-unsupported";
    }
    if !observation.flag(CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED) {
        return "classic-selftest-notrun";
    }
    if observation.registration_status != Some(0)
        || !observation.flag(CLASSIC_SELFTEST_ENABLED)
        || observation.emit_api_status != Some(0)
        || observation.emit_count == 0
    {
        return "classic-selftest-unsupported";
    }
    "classic-selftest-missing"
}

fn driver_wmi_probe_control_window_write_delta(
    inventory: &BackendTraceXmlInventory,
) -> Option<u64> {
    let enable = inventory
        .driver_wmi_probe_control_enable_snapshot
        .as_ref()?;
    let disable = inventory
        .driver_wmi_probe_control_disable_snapshot
        .as_ref()?;
    Some(
        disable
            .write_etw_event_calls
            .saturating_sub(enable.write_etw_event_calls),
    )
}

fn driver_wmi_probe_callsite_evidence_present(inventory: &BackendTraceXmlInventory) -> bool {
    inventory.driver_wmi_probe_write_mirror_events != 0
        || driver_wmi_probe_control_window_write_delta(inventory).is_some_and(|delta| delta != 0)
}

fn driver_wmi_probe_status(inventory: &BackendTraceXmlInventory) -> &'static str {
    if inventory.driver_wmi_probe_control_enable_events != 0
        && driver_wmi_probe_callsite_evidence_present(inventory)
    {
        "driver-wmi-control-and-callsite-decoded"
    } else if inventory.driver_wmi_probe_control_enable_events != 0 {
        "driver-wmi-control-decoded-callsite-absent"
    } else if inventory.driver_wmi_probe_payload_match_events != 0 {
        "driver-wmi-payload-decoded-control-absent"
    } else if inventory.driver_wmi_probe_identity_match_events != 0 {
        "driver-wmi-identity-present-payload-undecoded"
    } else if inventory.kernel_witness_driver_probe_enable_events != 0 {
        "driver-wmi-control-request-witnessed-callback-absent"
    } else if inventory.kernel_eventtracing_events == 0 {
        "driver-wmi-control-unresolved"
    } else {
        "driver-wmi-control-absent"
    }
}

fn miniport_lifecycle_marker_count(inventory: &BackendTraceXmlInventory) -> u64 {
    inventory
        .rvf_xrun_schema_1001_events
        .saturating_add(inventory.rvf_xrun_schema_1002_events)
        .saturating_add(inventory.miniport_ordinal_counts[1])
        .saturating_add(inventory.miniport_ordinal_counts[2])
        .saturating_add(inventory.miniport_ordinal_counts[4])
}

fn miniport_lifecycle_status(
    inventory: &BackendTraceXmlInventory,
    stream_success: bool,
) -> &'static str {
    if miniport_lifecycle_marker_count(inventory) != 0 {
        "miniport-lifecycle-decoded"
    } else if stream_success {
        "miniport-lifecycle-absent-after-stream-success"
    } else {
        "miniport-lifecycle-inconclusive"
    }
}

fn position_event_status(inventory: &BackendTraceXmlInventory) -> &'static str {
    if inventory.rvf_position_events != 0 || inventory.standard_presentation_position_events != 0 {
        "position-event-decoded"
    } else if inventory.events_total == 0 || inventory.decode_error.is_some() {
        "position-event-inconclusive"
    } else {
        "position-event-absent"
    }
}

fn event_split_statuses(
    inventory: &BackendTraceXmlInventory,
    observation: &ClassicSelftestObservation,
    stream_success: bool,
) -> EventSplitStatuses {
    let control_witness = control_witness_status(inventory);
    let classic_selftest = classic_selftest_status(observation, inventory);
    let miniport_lifecycle = miniport_lifecycle_status(inventory, stream_success);
    let position_event = position_event_status(inventory);
    let verdict = if position_event == "position-event-decoded" {
        "driver-position-path-positive"
    } else if miniport_lifecycle == "miniport-lifecycle-decoded"
        && position_event == "position-event-absent"
    {
        "miniport-lifecycle-positive-position-fire-miss"
    } else if classic_selftest == "classic-selftest-decoded"
        && miniport_lifecycle == "miniport-lifecycle-absent-after-stream-success"
    {
        "driver-upstream-or-portcls-emission-unresolved"
    } else if classic_selftest == "classic-selftest-missing"
        && miniport_lifecycle != "miniport-lifecycle-decoded"
    {
        "capture-decode-failed"
    } else {
        "inconclusive-blocked"
    };
    EventSplitStatuses {
        control_witness,
        classic_selftest,
        miniport_lifecycle,
        position_event,
        verdict,
    }
}

fn backend_trace_bounded_text(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '\\' | '/') {
                c
            } else {
                '_'
            }
        })
        .take(160)
        .collect()
}

fn backend_trace_provider_file_text() -> String {
    BACKEND_TRACE_PROVIDERS
        .iter()
        .filter(|provider| provider.enabled_in_session)
        .map(|provider| {
            format!(
                "{} {} {}",
                provider.guid, provider.keyword_mask, provider.level
            )
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        )
        + "
"
}

fn backend_trace_file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn backend_trace_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned()
}

fn clamp_i128_to_i64(value: i128) -> i64 {
    i64::try_from(value.clamp(i128::from(i64::MIN), i128::from(i64::MAX))).unwrap_or(i64::MAX)
}

fn driver_event_reference_json(
    rf_event_index: usize,
    event: &RvfDriverPositionEvent,
    callback_ns: Option<i64>,
) -> serde_json::Value {
    let delta_ns = callback_ns.and_then(|callback_ns| {
        let callback_ns = u64::try_from(callback_ns).ok()?;
        event
            .raw_qpc_ns
            .map(|raw_qpc_ns| clamp_i128_to_i64(i128::from(raw_qpc_ns) - i128::from(callback_ns)))
    });
    serde_json::json!({
        "rf_event_index": rf_event_index,
        "source_event_index": event.source_event_index,
        "capture": event.capture,
        "pin": event.pin,
        "device_type": event.device_type,
        "frequency_hz": event.frequency_hz,
        "presentation_position_blocks": event.presentation_position_blocks,
        "raw_qpc": event.raw_qpc,
        "raw_qpc_ns": event.raw_qpc_ns,
        "delta_from_callback_ns": delta_ns,
        "system_time": event.system_time,
    })
}

fn nearest_driver_event(
    events: &[RvfDriverPositionEvent],
    capture: bool,
    callback_ns: i64,
    before: bool,
) -> Option<(usize, &RvfDriverPositionEvent)> {
    let callback_ns = u64::try_from(callback_ns).ok()?;
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.capture == capture)
        .filter(|(_, event)| {
            event.raw_qpc_ns.is_some_and(|raw_qpc_ns| {
                if before {
                    raw_qpc_ns <= callback_ns
                } else {
                    raw_qpc_ns >= callback_ns
                }
            })
        })
        .min_by_key(|(_, event)| {
            event
                .raw_qpc_ns
                .map_or(u64::MAX, |raw| raw.abs_diff(callback_ns))
        })
}

fn nearby_driver_events_json(
    events: &[RvfDriverPositionEvent],
    capture: bool,
    callback_ns: i64,
) -> Vec<serde_json::Value> {
    let Ok(callback_ns_u64) = u64::try_from(callback_ns) else {
        return Vec::new();
    };
    let mut nearest = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.capture == capture && event.raw_qpc_ns.is_some())
        .map(|(index, event)| {
            (
                index,
                event,
                event
                    .raw_qpc_ns
                    .map_or(u64::MAX, |raw| raw.abs_diff(callback_ns_u64)),
            )
        })
        .collect::<Vec<_>>();
    nearest.sort_by_key(|(_, _, distance)| *distance);
    nearest
        .into_iter()
        .take(8)
        .map(|(index, event, _)| driver_event_reference_json(index, event, Some(callback_ns)))
        .collect()
}

fn direction_window_stats_json(
    events: &[RvfDriverPositionEvent],
    capture: bool,
    callback_ns: i64,
    span_ns: u64,
) -> serde_json::Value {
    let Ok(center) = u64::try_from(callback_ns) else {
        return serde_json::json!({"capture": capture, "status": "callback-ns-negative"});
    };
    let lower = center.saturating_sub(span_ns);
    let upper = center.saturating_add(span_ns);
    let window = events
        .iter()
        .filter(|event| event.capture == capture)
        .filter(|event| {
            event
                .raw_qpc_ns
                .is_some_and(|raw| (lower..=upper).contains(&raw))
        })
        .collect::<Vec<_>>();
    let mut qpc_negative_steps = 0_u64;
    let mut position_negative_steps = 0_u64;
    let mut previous_qpc = None;
    let mut previous_position = None;
    for event in &window {
        if let Some(raw_qpc_ns) = event.raw_qpc_ns {
            if previous_qpc.is_some_and(|previous| raw_qpc_ns < previous) {
                qpc_negative_steps = qpc_negative_steps.saturating_add(1);
            }
            previous_qpc = Some(raw_qpc_ns);
        }
        if previous_position.is_some_and(|previous| event.presentation_position_blocks < previous) {
            position_negative_steps = position_negative_steps.saturating_add(1);
        }
        previous_position = Some(event.presentation_position_blocks);
    }
    serde_json::json!({
        "capture": capture,
        "window_half_span_ns": span_ns,
        "event_count": window.len(),
        "first_raw_qpc_ns": window.first().and_then(|event| event.raw_qpc_ns),
        "last_raw_qpc_ns": window.last().and_then(|event| event.raw_qpc_ns),
        "first_presentation_position_blocks": window.first().map(|event| event.presentation_position_blocks),
        "last_presentation_position_blocks": window.last().map(|event| event.presentation_position_blocks),
        "qpc_negative_steps_in_xml_order": qpc_negative_steps,
        "position_negative_steps_in_xml_order": position_negative_steps,
    })
}

fn build_driver_position_correlation(
    probe: &EndpointProbe,
    inventory: &BackendTraceXmlInventory,
) -> serde_json::Value {
    let callback_events = probe.out_timestamp.events();
    let boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    let diagnostics = render_regression_diagnostics(&callback_events, boundary_cb);
    let rows = diagnostics
        .iter()
        .map(|diagnostic| {
            let callback_event = callback_events
                .iter()
                .find(|event| event.callback_index == diagnostic.callback_index);
            let callback_ns = callback_event.map(|event| event.callback_ns);
            let render_before = callback_ns.and_then(|callback_ns| {
                nearest_driver_event(
                    &inventory.rvf_driver_position_events,
                    false,
                    callback_ns,
                    true,
                )
            });
            let render_after = callback_ns.and_then(|callback_ns| {
                nearest_driver_event(
                    &inventory.rvf_driver_position_events,
                    false,
                    callback_ns,
                    false,
                )
            });
            let capture_before = callback_ns.and_then(|callback_ns| {
                nearest_driver_event(
                    &inventory.rvf_driver_position_events,
                    true,
                    callback_ns,
                    true,
                )
            });
            let capture_after = callback_ns.and_then(|callback_ns| {
                nearest_driver_event(
                    &inventory.rvf_driver_position_events,
                    true,
                    callback_ns,
                    false,
                )
            });
            serde_json::json!({
                "callback_index": diagnostic.callback_index,
                "effective_regression": !diagnostic.startup_grace,
                "epoch": diagnostic.epoch,
                "epoch_callback_offset": diagnostic.epoch_callback_offset,
                "startup_grace": diagnostic.startup_grace,
                "callback_ns": callback_ns,
                "reference_callback_index": diagnostic.reference_callback_index,
                "reference_callback_ns": diagnostic.reference_callback_ns,
                "regression_backstep_ns": diagnostic.regression_backstep_ns,
                "backend_two_step_ns": diagnostic.backend_two_step_ns,
                "host_two_step_ns": diagnostic.host_two_step_ns,
                "supply_two_step_frames": diagnostic.supply_two_step_frames,
                "playback_lead_before_ns": diagnostic.playback_lead_before_ns,
                "playback_lead_ns": diagnostic.playback_lead_ns,
                "playback_lead_after_ns": diagnostic.playback_lead_after_ns,
                "playback_interval_ns": diagnostic.playback_interval_ns,
                "next_playback_interval_ns": diagnostic.next_playback_interval_ns,
                "render_driver_event_before": render_before.map(|(index, event)| driver_event_reference_json(index, event, callback_ns)),
                "render_driver_event_after": render_after.map(|(index, event)| driver_event_reference_json(index, event, callback_ns)),
                "capture_driver_event_before": capture_before.map(|(index, event)| driver_event_reference_json(index, event, callback_ns)),
                "capture_driver_event_after": capture_after.map(|(index, event)| driver_event_reference_json(index, event, callback_ns)),
                "nearby_render_driver_events": callback_ns.map_or_else(Vec::new, |callback_ns| nearby_driver_events_json(&inventory.rvf_driver_position_events, false, callback_ns)),
                "nearby_capture_driver_events": callback_ns.map_or_else(Vec::new, |callback_ns| nearby_driver_events_json(&inventory.rvf_driver_position_events, true, callback_ns)),
                "render_window_stats": callback_ns.map_or_else(|| serde_json::json!({"status":"missing-callback-ns"}), |callback_ns| direction_window_stats_json(&inventory.rvf_driver_position_events, false, callback_ns, 20_000_000)),
                "capture_window_stats": callback_ns.map_or_else(|| serde_json::json!({"status":"missing-callback-ns"}), |callback_ns| direction_window_stats_json(&inventory.rvf_driver_position_events, true, callback_ns, 20_000_000)),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema": "rf1034-driver-position-correlation/v1",
        "diagnostic_artifact": true,
        "not_run_metadata_schema": true,
        "case": ROUNDTRIP_AUDIO_CASE_ID,
        "path": ROUNDTRIP_OUTPUT_PATH,
        "clock_conversion": "driver raw_qpc_ns = floor(raw_qpc * 1e9 / frequency_hz); callback ns are cpal StreamInstant values derived from WASAPI qpc_position 100ns units multiplied by 100; correlation assumes both are QPC-derived. ETW perf clock to host Instant calibration remains unresolved.",
        "raw_regressions": probe.out_timestamp.raw_regressions(),
        "effective_regressions": probe.out_timestamp.effective_regressions(),
        "epoch_boundary_cb": boundary_cb,
        "decoded_rvf_driver_position_events": inventory.rvf_position_events,
        "stored_rvf_driver_position_events": inventory.rvf_driver_position_events.len(),
        "driver_position_event_overflow": inventory.driver_position_event_overflow,
        "regressions": rows,
    })
}

struct BackendOsTraceControl {
    pair_index: usize,
    session_name: String,
    host_origin: Instant,
    probe: Option<Arc<EndpointProbe>>,
    artifact_dir: Option<PathBuf>,
    artifact_dir_source: Option<&'static str>,
    paths: Option<BackendTracePaths>,
    status: String,
    started: bool,
    finished: bool,
    notes: Vec<String>,
    commands: Vec<BackendTraceCommandObservation>,
    start_calibration: Option<BackendTraceCalibration>,
    stop_before_calibration: Option<BackendTraceCalibration>,
    stop_after_calibration: Option<BackendTraceCalibration>,
    xml_inventory: BackendTraceXmlInventory,
    xml_present: bool,
    selftest: Option<ClassicEtwSelftest>,
}

fn backend_trace_xml_inventory_json(inventory: &BackendTraceXmlInventory) -> serde_json::Value {
    serde_json::json!({
        "events_total": inventory.events_total,
        "audio_provider_events": inventory.audio_provider_events,
        "kernel_streaming_provider_events": inventory.kernel_streaming_provider_events,
        "audio_engine_provider_events": inventory.audio_engine_provider_events,
        "portcls_provider_events": inventory.portcls_provider_events,
        "other_provider_events": inventory.other_provider_events,
        "kernel_eventtracing_events": inventory.kernel_eventtracing_events,
        "classic_selftest_events": inventory.classic_selftest_events,
        "classic_selftest_payload_match_events": inventory.classic_selftest_payload_match_events,
        "driver_wmi_probe_events": inventory.driver_wmi_probe_events,
        "driver_wmi_probe_payload_match_events": inventory.driver_wmi_probe_payload_match_events,
        "driver_wmi_probe_control_enable_events": inventory.driver_wmi_probe_control_enable_events,
        "driver_wmi_probe_control_disable_events": inventory.driver_wmi_probe_control_disable_events,
        "driver_wmi_probe_write_mirror_events": inventory.driver_wmi_probe_write_mirror_events,
        "stored_driver_wmi_probe_events": inventory.driver_wmi_probe_stored_events.len(),
        "ae_position_events": inventory.ae_position_events,
        "set_current_timestamp_events": inventory.set_current_timestamp_events,
        "glitch_events": inventory.glitch_events,
        "binary_event_data_events": inventory.binary_event_data_events,
        "miniport_events_total": inventory.miniport_events_total,
        "portcls_events_missing_binary_payload": inventory.portcls_events_missing_binary_payload,
        "undecoded_portcls_binary_events": inventory.undecoded_portcls_binary_events,
        "unmodeled_portcls_event_class_events": inventory.unmodeled_portcls_event_class_events,
        "invalid_rvf_position_events": inventory.invalid_rvf_position_events,
        "driver_position_event_overflow": inventory.driver_position_event_overflow,
        "rvf_position_events": inventory.rvf_position_events,
        "standard_presentation_position_events": inventory.standard_presentation_position_events,
        "miniport_ordinal_counts": inventory.miniport_ordinal_counts,
        "rvf_xrun_schema_1001_events": inventory.rvf_xrun_schema_1001_events,
        "rvf_xrun_schema_1002_events": inventory.rvf_xrun_schema_1002_events,
        "stored_miniport_events": inventory.miniport_events.len(),
        "stored_rvf_driver_position_events": inventory.rvf_driver_position_events.len(),
        "decode_error": inventory.decode_error,
    })
}

fn driver_wmi_probe_control_window_json(inventory: &BackendTraceXmlInventory) -> serde_json::Value {
    let enable = inventory.driver_wmi_probe_control_enable_snapshot.as_ref();
    let disable = inventory.driver_wmi_probe_control_disable_snapshot.as_ref();
    let status = match (enable, disable) {
        (Some(_), Some(_)) => "enable-disable-snapshot-pair",
        (Some(_), None) => "enable-without-disable",
        (None, Some(_)) => "disable-without-enable",
        (None, None) => "no-control-snapshot",
    };
    let counter = |select: fn(&DriverWmiProbeEvent) -> u64| {
        let enable_value = enable.map(select);
        let disable_value = disable.map(select);
        serde_json::json!({
            "enable": enable_value,
            "disable": disable_value,
            "delta": enable_value.zip(disable_value).map(|(start, end)| end.saturating_sub(start)),
        })
    };
    serde_json::json!({
        "status": status,
        "write_etw_event_calls": counter(|event| event.write_etw_event_calls),
        "helper_null_calls": counter(|event| event.helper_null_calls),
        "forward_error_calls": counter(|event| event.forward_error_calls),
        "mirror_suppressed_calls": counter(|event| event.mirror_suppressed_calls),
        "mirror_key_calls": counter(|event| event.mirror_key_calls),
        "mirror_key_emit_calls": counter(|event| event.mirror_key_emit_calls),
    })
}

fn driver_wmi_probe_decode_json(inventory: &BackendTraceXmlInventory) -> serde_json::Value {
    serde_json::json!({
        "identity_event_count": inventory.driver_wmi_probe_events,
        "identity_match_count": inventory.driver_wmi_probe_identity_match_events,
        "payload_match_count": inventory.driver_wmi_probe_payload_match_events,
        "missing_payload_count": inventory.driver_wmi_probe_missing_payload_events,
        "payload_mismatch_count": inventory.driver_wmi_probe_payload_mismatch_events,
        "control_enable_count": inventory.driver_wmi_probe_control_enable_events,
        "control_disable_count": inventory.driver_wmi_probe_control_disable_events,
        "write_mirror_count": inventory.driver_wmi_probe_write_mirror_events,
        "callsite_evidence": driver_wmi_probe_callsite_evidence_present(inventory),
        "control_window": driver_wmi_probe_control_window_json(inventory),
        "mirror_ordinal_counts": inventory.driver_wmi_probe_mirror_ordinal_counts,
        "stored_events": inventory.driver_wmi_probe_stored_events.len(),
        "storage_overflow": inventory.driver_wmi_probe_event_overflow,
        "decoded_events": inventory.driver_wmi_probe_stored_events.clone(),
    })
}

fn event_split_details_json(
    inventory: &BackendTraceXmlInventory,
    paths: &BackendTracePaths,
) -> serde_json::Value {
    let file_entry = |path: &Path| serde_json::json!({"file": backend_trace_file_name(path), "bytes": backend_trace_file_size(path)});
    serde_json::json!({
        "provider_matrix": BACKEND_TRACE_PROVIDERS.iter().map(|provider| serde_json::json!({
            "name": provider.name,
            "guid": provider.guid,
            "role": provider.role.as_str(),
            "enabled_in_session": provider.enabled_in_session,
            "keyword_mask": provider.keyword_mask,
            "level": provider.level,
        })).collect::<Vec<_>>(),
        "selftest_identity": {
            "provider_guid": CLASSIC_SELFTEST_PROVIDER_GUID,
            "event_class_guid": CLASSIC_SELFTEST_EVENT_CLASS_GUID,
            "payload_hex": classic_selftest_payload_hex(),
        },
        "driver_wmi_probe_identity": {
            "control_guid": DRIVER_WMI_PROBE_CONTROL_GUID,
            "event_guid": DRIVER_WMI_PROBE_EVENT_GUID,
            "payload_signature": "RVF1034W",
            "payload_schema_version": 1,
            "payload_bytes": 120,
            "sampling": "first 4 calls per ordinal/schema key, then every 32nd call while the probe control is enabled",
        },
        "control_witness_counts": {
            "kernel_eventtracing_events": inventory.kernel_eventtracing_events,
            "enable_event_id": KERNEL_EVENTTRACING_ENABLE_EVENT_ID,
            "disable_event_id": KERNEL_EVENTTRACING_DISABLE_EVENT_ID,
            "enable_events": inventory.kernel_eventtracing_enable_events,
            "disable_events": inventory.kernel_eventtracing_disable_events,
            "enable_opcode18_events": inventory.kernel_eventtracing_enable_opcode18_events,
            "disable_opcode19_events": inventory.kernel_eventtracing_disable_opcode19_events,
            "raw_message_available": inventory.kernel_eventtracing_raw_message_available,
            "matched_session_enable": inventory.kernel_witness_matched_session_enable_events,
            "matched_session_disable": inventory.kernel_witness_matched_session_disable_events,
            "portcls_control_enable": inventory.kernel_witness_portcls_control_enable_events,
            "portcls_control_disable": inventory.kernel_witness_portcls_control_disable_events,
            "portcls_event_enable": inventory.kernel_witness_portcls_event_enable_events,
            "portcls_event_disable": inventory.kernel_witness_portcls_event_disable_events,
            "selftest_enable": inventory.kernel_witness_selftest_enable_events,
            "selftest_disable": inventory.kernel_witness_selftest_disable_events,
            "driver_probe_enable": inventory.kernel_witness_driver_probe_enable_events,
            "driver_probe_disable": inventory.kernel_witness_driver_probe_disable_events,
        },
        "selftest_decode": {
            "events": inventory.classic_selftest_events,
            "identity_match_count": inventory.classic_selftest_identity_match_events,
            "payload_match_count": inventory.classic_selftest_payload_match_events,
            "missing_payload_count": inventory.classic_selftest_missing_payload_events,
            "payload_mismatch_count": inventory.classic_selftest_payload_mismatch_events,
        },
        "driver_wmi_probe_decode": driver_wmi_probe_decode_json(inventory),
        "miniport_counts": {
            "ordinal": {
                "0_ihv_defined": inventory.miniport_ordinal_counts[0],
                "1_buffer_complete": inventory.miniport_ordinal_counts[1],
                "2_pin_state": inventory.miniport_ordinal_counts[2],
                "3_get_stream_position": inventory.miniport_ordinal_counts[3],
                "4_set_wavert_buffer_write_position": inventory.miniport_ordinal_counts[4],
                "5_get_presentation_position": inventory.miniport_ordinal_counts[5],
            },
            "total_decoded": inventory.miniport_events_total,
            "xrun_schema_1001": inventory.rvf_xrun_schema_1001_events,
            "xrun_schema_1002": inventory.rvf_xrun_schema_1002_events,
            "xrun_schema_1003": inventory.rvf_xrun_schema_1003_events,
            "xrun_schema_1004": inventory.rvf_xrun_schema_1004_events,
            "xrun_schema_1005": inventory.rvf_xrun_schema_1005_events,
            "rf_position_schema_2001": inventory.rvf_position_events,
            "standard_presentation_position": inventory.standard_presentation_position_events,
            "missing_binary_payload": inventory.portcls_events_missing_binary_payload,
            "undecoded_binary": inventory.undecoded_portcls_binary_events,
            "unmodeled_class": inventory.unmodeled_portcls_event_class_events,
        },
        "raw_artifacts": {
            "etl": file_entry(&paths.etl),
            "xml": file_entry(&paths.xml),
            "provider_discovery": file_entry(&paths.provider_discovery),
            "provider_list": file_entry(&paths.provider_list),
            "session_query_log": file_entry(&paths.session_query_log),
            "start_log": file_entry(&paths.start_log),
            "stop_log": file_entry(&paths.stop_log),
            "delete_log": file_entry(&paths.delete_log),
            "tracerpt_log": file_entry(&paths.convert_log),
        },
        "decode_notes": [
            "classic-selftest-decoded is a user-mode classic ETW capture/decode positive control, not proof of kernel-mode WMI payload compatibility",
            "miniport-lifecycle-absent-after-stream-success does not separate driver emit drop, control delivery failure, PortCls buffering, and kernel WMI capture incompatibility",
            "driver-owned WMI probe events are bounded diagnostic observations; mirror sampling/counters expose callsite reachability but do not make ETW emission lossless",
        ],
    })
}

impl BackendOsTraceControl {
    fn new(pair_index: usize, host_origin: Instant) -> Self {
        Self {
            pair_index,
            session_name: backend_trace_session_name(pair_index),
            host_origin,
            probe: None,
            artifact_dir: None,
            artifact_dir_source: None,
            paths: None,
            status: "unsupported".to_owned(),
            started: false,
            finished: false,
            notes: Vec::new(),
            commands: Vec::new(),
            start_calibration: None,
            stop_before_calibration: None,
            stop_after_calibration: None,
            xml_inventory: BackendTraceXmlInventory::default(),
            xml_present: false,
            selftest: None,
        }
    }

    fn command_exit(&self, label: &str) -> Option<i32> {
        self.commands
            .iter()
            .find(|command| command.label == label)
            .and_then(|command| command.exit_code)
    }

    fn prepare_providers(&mut self, paths: &BackendTracePaths) -> bool {
        let discovery = run_backend_trace_command(
            "provider_discovery",
            "logman.exe",
            &["query", "providers"],
            Some(&paths.provider_discovery),
        );
        if !discovery.success {
            self.notes.push(format!(
                "provider discovery failed exit={:?} error={:?}",
                discovery.exit_code, discovery.error
            ));
            self.commands.push(discovery);
            return false;
        }
        let discovery_text = fs::read(&paths.provider_discovery)
            .map(|bytes| String::from_utf8_lossy(&bytes).to_ascii_lowercase())
            .unwrap_or_default();
        let missing = BACKEND_TRACE_PROVIDERS
            .iter()
            .filter(|provider| {
                if !provider.require_registered {
                    return false;
                }
                let name = provider.name.to_ascii_lowercase();
                let guid = provider.guid.to_ascii_lowercase();
                !discovery_text.contains(&name) && !discovery_text.contains(&guid)
            })
            .map(|provider| provider.name)
            .collect::<Vec<_>>();
        self.commands.push(discovery);
        if !missing.is_empty() {
            self.notes.push(format!(
                "required provider missing from discovery: {}",
                missing.join(",")
            ));
            return false;
        }

        for (provider, log_path) in &paths.provider_logs {
            let observation = run_backend_trace_command(
                "wevtutil_gp",
                "wevtutil.exe",
                &["gp", provider],
                Some(log_path),
            );
            if !observation.success {
                self.notes.push(format!(
                    "wevtutil gp failed for {provider} exit={:?}",
                    observation.exit_code
                ));
            }
            self.commands.push(observation);
        }

        let provider_list = backend_trace_provider_file_text();
        if let Err(error) = write_artifact_bytes(&paths.provider_list, provider_list.as_bytes()) {
            self.notes.push(error);
            return false;
        }
        true
    }

    fn precheck_session(&mut self, paths: &BackendTracePaths) -> bool {
        let precheck = run_backend_trace_command(
            "precheck_query",
            "logman.exe",
            &["query", &self.session_name, "-ets"],
            Some(&paths.precheck_log),
        );
        if precheck.success {
            self.notes.push(format!(
                "trace session residue exists for {}; refusing to reuse it",
                self.session_name
            ));
            self.commands.push(precheck);
            return false;
        }
        self.commands.push(precheck);
        true
    }

    fn start(&mut self) {
        if !cfg!(windows) {
            self.notes
                .push("non-windows host: backend ETW trace unsupported".to_owned());
            return;
        }
        let (artifact_dir, source) = match backend_trace_artifact_dir_from_env() {
            Ok(result) => result,
            Err(error) => {
                self.notes.push(error);
                return;
            }
        };
        self.artifact_dir = Some(artifact_dir.clone());
        self.artifact_dir_source = Some(source);
        let paths =
            BackendTracePaths::new(&artifact_dir, &backend_trace_base_name(self.pair_index));
        self.paths = Some(paths.clone());
        if !self.prepare_providers(&paths) || !self.precheck_session(&paths) {
            return;
        }
        self.selftest = Some(ClassicEtwSelftest::register());

        self.start_calibration = Some(backend_trace_calibration(self.host_origin));
        let etl = paths.etl.to_string_lossy().to_string();
        let provider_file = paths.provider_list.to_string_lossy().to_string();
        let start = run_backend_trace_command(
            "logman_start",
            "logman.exe",
            &[
                "start",
                &self.session_name,
                "-o",
                &etl,
                "-pf",
                &provider_file,
                "-max",
                BACKEND_TRACE_MAX_LOG_MB,
                "-ct",
                "perf",
                "-ets",
            ],
            Some(&paths.start_log),
        );
        if !start.success {
            self.notes.push(format!(
                "logman start failed exit={:?} error={:?}",
                start.exit_code, start.error
            ));
            self.commands.push(start);
            return;
        }
        self.commands.push(start);
        let session_query = run_backend_trace_command(
            "logman_query_session",
            "logman.exe",
            &["query", &self.session_name, "-ets"],
            Some(&paths.session_query_log),
        );
        if !session_query.success {
            self.notes.push(format!(
                "logman session query failed exit={:?} error={:?}",
                session_query.exit_code, session_query.error
            ));
        }
        self.commands.push(session_query);
        self.started = true;
        if let Some(selftest) = self.selftest.as_mut() {
            selftest.wait_for_enable(Duration::from_secs(2));
            selftest.emit_once();
        }
        "incomplete".clone_into(&mut self.status);
    }

    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.stop_before_calibration = Some(backend_trace_calibration(self.host_origin));
        if let Some(selftest) = self.selftest.as_mut() {
            selftest.unregister();
        }
        if self.started
            && let Some(paths) = self.paths.clone()
        {
            let stop = run_backend_trace_command(
                "logman_stop",
                "logman.exe",
                &["stop", &self.session_name, "-ets"],
                Some(&paths.stop_log),
            );
            let stop_success = stop.success;
            self.commands.push(stop);
            let delete = run_backend_trace_command(
                "logman_delete",
                "logman.exe",
                &["delete", &self.session_name, "-ets"],
                Some(&paths.delete_log),
            );
            self.commands.push(delete);
            let etl_size = backend_trace_file_size(&paths.etl);
            if stop_success && etl_size.is_some_and(|size| size != 0) {
                let etl = paths.etl.to_string_lossy().to_string();
                let xml = paths.xml.to_string_lossy().to_string();
                let convert = run_backend_trace_command(
                    "tracerpt_xml",
                    "tracerpt.exe",
                    &[&etl, "-o", &xml, "-of", "XML", "-lr"],
                    Some(&paths.convert_log),
                );
                let convert_success = convert.success;
                self.commands.push(convert);
                self.xml_present = convert_success
                    && backend_trace_file_size(&paths.xml).is_some_and(|size| size != 0);
                if self.xml_present {
                    match scan_backend_trace_xml(&paths.xml, &self.session_name) {
                        Ok(inventory) => self.xml_inventory = inventory,
                        Err(error) => {
                            self.xml_inventory.decode_error = Some(error.clone());
                            self.notes
                                .push(format!("xml inventory scan failed: {error}"));
                        }
                    }
                } else {
                    self.notes.push(
                        "tracerpt XML conversion failed or empty; raw ETL retained".to_owned(),
                    );
                }
                "captured".clone_into(&mut self.status);
            } else {
                self.notes.push(format!(
                    "trace stop/etl incomplete stop_success={stop_success} etl_size={etl_size:?}"
                ));
            }
        }
        self.stop_after_calibration = Some(backend_trace_calibration(self.host_origin));
        self.write_event_split_sidecar();
        self.write_driver_position_sidecars();
        self.write_summary();
        for line in self.diagnostic_lines() {
            eprintln!("{line}");
        }
    }

    fn driver_coverage(&self) -> &'static str {
        backend_trace_driver_coverage(&self.xml_inventory, self.xml_present)
    }

    fn driver_wmi_probe_diagnostic_line(&self, context: &str) -> String {
        format!(
            "roundtrip_driver_wmi_control_probe: {context}status={} witness_enable={} witness_disable={} events={} payload_match={} control_enable={} control_disable={} mirrors={} mirror_ordinal0={} mirror_ordinal1={} mirror_ordinal2={} mirror_ordinal4={} mirror_ordinal5={} missing_payload={} payload_mismatch={}",
            driver_wmi_probe_status(&self.xml_inventory),
            self.xml_inventory.kernel_witness_driver_probe_enable_events,
            self.xml_inventory
                .kernel_witness_driver_probe_disable_events,
            self.xml_inventory.driver_wmi_probe_events,
            self.xml_inventory.driver_wmi_probe_payload_match_events,
            self.xml_inventory.driver_wmi_probe_control_enable_events,
            self.xml_inventory.driver_wmi_probe_control_disable_events,
            self.xml_inventory.driver_wmi_probe_write_mirror_events,
            self.xml_inventory.driver_wmi_probe_mirror_ordinal_counts[0],
            self.xml_inventory.driver_wmi_probe_mirror_ordinal_counts[1],
            self.xml_inventory.driver_wmi_probe_mirror_ordinal_counts[2],
            self.xml_inventory.driver_wmi_probe_mirror_ordinal_counts[4],
            self.xml_inventory.driver_wmi_probe_mirror_ordinal_counts[5],
            self.xml_inventory.driver_wmi_probe_missing_payload_events,
            self.xml_inventory.driver_wmi_probe_payload_mismatch_events,
        )
    }

    fn diagnostic_lines(&self) -> Vec<String> {
        let context = roundtrip_output_context(self.pair_index);
        let (etl_name, xml_name, summary_name) = self.paths.as_ref().map_or_else(
            || ("none".to_owned(), "none".to_owned(), "none".to_owned()),
            |paths| {
                (
                    backend_trace_file_name(&paths.etl),
                    backend_trace_file_name(&paths.xml),
                    backend_trace_file_name(&paths.summary_json),
                )
            },
        );
        let notes = backend_trace_bounded_text(&self.notes.join(";"));
        let statuses = self.event_split_statuses();
        vec![
            format!(
                "roundtrip_backend_os_trace: {context}status={} coverage={} session={} artifact_dir_source={} etl={} etl_bytes={} xml={} xml_bytes={} summary={} provider_events_audio={} provider_events_kernelstreaming={} provider_events_audioengine={} provider_events_portcls={} ae_position_events={} set_current_timestamp_events={} glitch_events={} rvf_position_events={} standard_presentation_position_events={} miniport_events_total={} portcls_missing_payload={} portcls_undecoded_binary={} invalid_rvf_position_events={} driver_position_event_overflow={} start_exit={} stop_exit={} delete_exit={} tracerpt_exit={} notes={}",
                self.status,
                self.driver_coverage(),
                backend_trace_bounded_text(&self.session_name),
                self.artifact_dir_source.unwrap_or("none"),
                etl_name,
                self.paths
                    .as_ref()
                    .and_then(|paths| backend_trace_file_size(&paths.etl))
                    .map_or_else(|| "none".to_owned(), |size| size.to_string()),
                xml_name,
                self.paths
                    .as_ref()
                    .and_then(|paths| backend_trace_file_size(&paths.xml))
                    .map_or_else(|| "none".to_owned(), |size| size.to_string()),
                summary_name,
                self.xml_inventory.audio_provider_events,
                self.xml_inventory.kernel_streaming_provider_events,
                self.xml_inventory.audio_engine_provider_events,
                self.xml_inventory.portcls_provider_events,
                self.xml_inventory.ae_position_events,
                self.xml_inventory.set_current_timestamp_events,
                self.xml_inventory.glitch_events,
                self.xml_inventory.rvf_position_events,
                self.xml_inventory.standard_presentation_position_events,
                self.xml_inventory.miniport_events_total,
                self.xml_inventory.portcls_events_missing_binary_payload,
                self.xml_inventory.undecoded_portcls_binary_events,
                self.xml_inventory.invalid_rvf_position_events,
                self.xml_inventory.driver_position_event_overflow,
                self.command_exit("logman_start")
                    .map_or_else(|| "none".to_owned(), |code| code.to_string()),
                self.command_exit("logman_stop")
                    .map_or_else(|| "none".to_owned(), |code| code.to_string()),
                self.command_exit("logman_delete")
                    .map_or_else(|| "none".to_owned(), |code| code.to_string()),
                self.command_exit("tracerpt_xml")
                    .map_or_else(|| "none".to_owned(), |code| code.to_string()),
                notes,
            ),
            format!(
                "roundtrip_portcls_event_split: {context}verdict={} control_witness={} classic_selftest={} miniport_lifecycle={} position_event={} kernel_events={} witness_portcls_control_enable={} witness_portcls_event_enable={} witness_selftest_enable={} selftest_emit={} selftest_payload_match={} xrun1001={} xrun1002={} ordinal1={} ordinal2={} ordinal4={}",
                statuses.verdict,
                statuses.control_witness,
                statuses.classic_selftest,
                statuses.miniport_lifecycle,
                statuses.position_event,
                self.xml_inventory.kernel_eventtracing_events,
                self.xml_inventory
                    .kernel_witness_portcls_control_enable_events,
                self.xml_inventory
                    .kernel_witness_portcls_event_enable_events,
                self.xml_inventory.kernel_witness_selftest_enable_events,
                self.selftest_observation().emit_count,
                self.xml_inventory.classic_selftest_payload_match_events,
                self.xml_inventory.rvf_xrun_schema_1001_events,
                self.xml_inventory.rvf_xrun_schema_1002_events,
                self.xml_inventory.miniport_ordinal_counts[1],
                self.xml_inventory.miniport_ordinal_counts[2],
                self.xml_inventory.miniport_ordinal_counts[4],
            ),
            self.driver_wmi_probe_diagnostic_line(&context),
            format!(
                "roundtrip_backend_os_trace_calibration: {context}start_host_monotonic_ns={} start_system_time_unix_ns={} stop_before_host_monotonic_ns={} stop_before_system_time_unix_ns={} stop_after_host_monotonic_ns={} stop_after_system_time_unix_ns={} etw_perf_freq={} etw_start_time={}",
                self.start_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.host_monotonic_ns.to_string()),
                self.start_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.system_time_unix_ns.to_string()),
                self.stop_before_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.host_monotonic_ns.to_string()),
                self.stop_before_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.system_time_unix_ns.to_string()),
                self.stop_after_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.host_monotonic_ns.to_string()),
                self.stop_after_calibration
                    .map_or_else(|| "none".to_owned(), |c| c.system_time_unix_ns.to_string()),
                self.xml_inventory
                    .perf_freq
                    .map_or_else(|| "none".to_owned(), |value| value.to_string()),
                self.xml_inventory
                    .start_time
                    .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            ),
        ]
    }

    fn selftest_observation(&self) -> ClassicSelftestObservation {
        self.selftest.as_ref().map_or_else(
            || {
                ClassicSelftestObservation::not_attempted(
                    "classic selftest registration not attempted",
                )
            },
            ClassicEtwSelftest::observation,
        )
    }

    fn stream_success(&self) -> bool {
        self.probe.as_ref().is_some_and(|probe| {
            probe.in_callbacks.load(Ordering::Relaxed) != 0
                && probe.out_callbacks.load(Ordering::Relaxed) != 0
        })
    }

    fn event_split_statuses(&self) -> EventSplitStatuses {
        event_split_statuses(
            &self.xml_inventory,
            &self.selftest_observation(),
            self.stream_success(),
        )
    }

    fn write_event_split_sidecar(&mut self) {
        let Some(paths) = self.paths.clone() else {
            return;
        };
        let observation = self.selftest_observation();
        let statuses = self.event_split_statuses();
        let mut split = serde_json::json!({
            "schema": "rf1034-portcls-event-split/v1",
            "diagnostic_artifact": true,
            "not_run_metadata_schema": true,
            "case": ROUNDTRIP_AUDIO_CASE_ID,
            "path": ROUNDTRIP_OUTPUT_PATH,
            "pair": self.pair_index,
            "session": self.session_name,
            "trace_status": self.status,
            "driver_event_coverage": self.driver_coverage(),
            "stream_success_observed": self.stream_success(),
            "split_verdict": statuses.verdict,
            "boundary_status": {
                "controller_enablement": statuses.control_witness,
                "classic_selftest": statuses.classic_selftest,
                "miniport_lifecycle": statuses.miniport_lifecycle,
                "position_event": statuses.position_event,
                "driver_wmi_probe": driver_wmi_probe_status(&self.xml_inventory),
                "kernel_wmi_capture_note": "user-mode classic positive control is not proof of kernel-mode WMI payload capture compatibility",
            },
            "selftest": classic_selftest_observation_json(&observation),
        });
        if let (serde_json::Value::Object(root), serde_json::Value::Object(details)) = (
            &mut split,
            event_split_details_json(&self.xml_inventory, &paths),
        ) {
            root.extend(details);
        }
        match serde_json::to_vec_pretty(&split) {
            Ok(bytes) => {
                if let Err(error) = write_artifact_bytes(&paths.event_split_json, &bytes) {
                    self.notes.push(error);
                }
            }
            Err(error) => self
                .notes
                .push(format!("event-split json serialization failed: {error}")),
        }
    }

    fn write_driver_position_sidecars(&mut self) {
        let Some(paths) = self.paths.clone() else {
            return;
        };
        let Some(probe) = self.probe.clone() else {
            self.notes
                .push("driver-position sidecar skipped: probe unavailable".to_owned());
            return;
        };
        let events = serde_json::json!({
            "schema": "rf1034-driver-position-events/v1",
            "diagnostic_artifact": true,
            "not_run_metadata_schema": true,
            "case": ROUNDTRIP_AUDIO_CASE_ID,
            "path": ROUNDTRIP_OUTPUT_PATH,
            "pair": self.pair_index,
            "session": self.session_name,
            "trace_status": self.status,
            "driver_event_coverage": self.driver_coverage(),
            "payload_layout": "PortCls EVENT_TRACE_HEADER uses WNODE_FLAG_USE_MOF_PTR; tracerpt BinaryEventData is the referenced 0x30-byte little-endian block: u64 trace_event_type, u64 data1, u64 data2, u64 data3, u64 data4, u64 trailing_zero",
            "trace_event_type_base": PORTCLS_MINIPORT_TRACE_TYPE_BASE,
            "miniport_event_counts": {
                "total_decoded": self.xml_inventory.miniport_events_total,
                "stored": self.xml_inventory.miniport_events.len(),
                "standard_presentation_position": self.xml_inventory.standard_presentation_position_events,
                "rf_position_schema": self.xml_inventory.rvf_position_events,
                "invalid_rvf_position_schema": self.xml_inventory.invalid_rvf_position_events,
                "missing_binary_payload": self.xml_inventory.portcls_events_missing_binary_payload,
                "undecoded_binary_payload": self.xml_inventory.undecoded_portcls_binary_events,
                "storage_overflow": self.xml_inventory.driver_position_event_overflow,
            },
            "miniport_events": self.xml_inventory.miniport_events.clone(),
            "rf_driver_position_events": self.xml_inventory.rvf_driver_position_events.clone(),
        });
        match serde_json::to_vec_pretty(&events) {
            Ok(bytes) => {
                if let Err(error) = write_artifact_bytes(&paths.driver_position_events_json, &bytes)
                {
                    self.notes.push(error);
                }
            }
            Err(error) => self.notes.push(format!(
                "driver-position events json serialization failed: {error}"
            )),
        }
        let correlation = build_driver_position_correlation(&probe, &self.xml_inventory);
        match serde_json::to_vec_pretty(&correlation) {
            Ok(bytes) => {
                if let Err(error) =
                    write_artifact_bytes(&paths.driver_position_correlation_json, &bytes)
                {
                    self.notes.push(error);
                }
            }
            Err(error) => self.notes.push(format!(
                "driver-position correlation json serialization failed: {error}"
            )),
        }
    }

    fn write_summary(&mut self) {
        let Some(paths) = self.paths.clone() else {
            return;
        };
        let commands = self
            .commands
            .iter()
            .map(|command| {
                serde_json::json!({
                    "label": command.label,
                    "program": command.program,
                    "args": command.args,
                    "exit_code": command.exit_code,
                    "success": command.success,
                    "error": command.error,
                    "log_file": command.log_file.as_ref().map(|path| backend_trace_file_name(path)),
                })
            })
            .collect::<Vec<_>>();
        let providers = BACKEND_TRACE_PROVIDERS
            .iter()
            .map(|provider| {
                serde_json::json!({
                    "name": provider.name,
                    "guid": provider.guid,
                    "keyword_mask": provider.keyword_mask,
                    "level": provider.level,
                    "role": provider.role.as_str(),
                    "enabled_in_session": provider.enabled_in_session,
                    "selection_reason": provider.selection_reason,
                })
            })
            .collect::<Vec<_>>();
        let file_entry = |path: &Path| serde_json::json!({"file": backend_trace_file_name(path), "bytes": backend_trace_file_size(path)});
        let summary = serde_json::json!({
            "schema": "rf1034-backend-os-trace/v1",
            "diagnostic_artifact": true,
            "not_run_metadata_schema": true,
            "case": ROUNDTRIP_AUDIO_CASE_ID,
            "path": ROUNDTRIP_OUTPUT_PATH,
            "pair": self.pair_index,
            "session": self.session_name,
            "status": self.status,
            "driver_event_coverage": self.driver_coverage(),
            "coverage_note": "coverage absence is incomplete observation, never driver innocence evidence",
            "artifact_dir": self.artifact_dir.as_ref().map(|path| path.display().to_string()),
            "artifact_dir_source": self.artifact_dir_source,
            "max_etl_mb": BACKEND_TRACE_MAX_LOG_MB,
            "trace_clock_type": "perf",
            "providers": providers,
            "files": {
                "provider_discovery": file_entry(&paths.provider_discovery),
                "provider_list": file_entry(&paths.provider_list),
                "precheck_log": file_entry(&paths.precheck_log),
                "start_log": file_entry(&paths.start_log),
                "session_query_log": file_entry(&paths.session_query_log),
                "stop_log": file_entry(&paths.stop_log),
                "delete_log": file_entry(&paths.delete_log),
                "convert_log": file_entry(&paths.convert_log),
                "etl": file_entry(&paths.etl),
                "xml": file_entry(&paths.xml),
                "driver_position_events_json": file_entry(&paths.driver_position_events_json),
                "driver_position_correlation_json": file_entry(&paths.driver_position_correlation_json),
                "event_split_json": file_entry(&paths.event_split_json),
            },
            "commands": commands,
            "calibration": {
                "system_time_unit": "unix epoch nanoseconds (UTC timescale)",
                "host_clock": "std::time::Instant elapsed since probe host_origin",
                "start": self.start_calibration,
                "stop_before": self.stop_before_calibration,
                "stop_after": self.stop_after_calibration,
                "etw_header_perf_freq": self.xml_inventory.perf_freq,
                "etw_header_start_time": self.xml_inventory.start_time,
                "conversion": "unresolved unless ETW perf clock and host Instant relationship is independently established",
            },
            "xml_inventory": backend_trace_xml_inventory_json(&self.xml_inventory),
            "event_split": {
                "schema": "rf1034-portcls-event-split/v1",
                "verdict": self.event_split_statuses().verdict,
                "controller_enablement": self.event_split_statuses().control_witness,
                "classic_selftest": self.event_split_statuses().classic_selftest,
                "miniport_lifecycle": self.event_split_statuses().miniport_lifecycle,
                "position_event": self.event_split_statuses().position_event,
                "driver_wmi_probe": driver_wmi_probe_status(&self.xml_inventory),
            },
            "notes": self.notes,
        });
        match serde_json::to_vec_pretty(&summary) {
            Ok(bytes) => {
                if let Err(error) = write_artifact_bytes(&paths.summary_json, &bytes) {
                    self.notes.push(error);
                }
            }
            Err(error) => self
                .notes
                .push(format!("summary json serialization failed: {error}")),
        }
    }
}

impl Drop for BackendOsTraceControl {
    fn drop(&mut self) {
        self.finish();
    }
}

fn start_backend_os_trace(
    pair_index: usize,
    host_origin: Instant,
    probe: &Arc<EndpointProbe>,
) -> BackendOsTraceControl {
    let mut control = BackendOsTraceControl::new(pair_index, host_origin);
    control.probe = Some(Arc::clone(probe));
    control.start();
    control
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum StreamDirection {
    Input,
    Output,
    Unknown,
}

impl StreamDirection {
    fn from_stored(value: u64) -> Self {
        match value {
            1 => Self::Input,
            2 => Self::Output,
            _ => Self::Unknown,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Unknown => "unknown",
        }
    }

    fn store_value(self) -> u64 {
        match self {
            Self::Input => 1,
            Self::Output => 2,
            Self::Unknown => u64::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum StreamLifecycleAction {
    Start,
    Stop,
    EpochSplit,
    Restart,
    ReStop,
    Unknown,
}

impl StreamLifecycleAction {
    fn from_stored(value: u64) -> Self {
        match value {
            1 => Self::Start,
            2 => Self::Stop,
            3 => Self::EpochSplit,
            4 => Self::Restart,
            5 => Self::ReStop,
            _ => Self::Unknown,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::EpochSplit => "epoch_split",
            Self::Restart => "restart",
            Self::ReStop => "re_stop",
            Self::Unknown => "unknown",
        }
    }

    fn store_value(self) -> u64 {
        match self {
            Self::Start => 1,
            Self::Stop => 2,
            Self::EpochSplit => 3,
            Self::Restart => 4,
            Self::ReStop => 5,
            Self::Unknown => u64::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum StreamErrorKind {
    Xrun,
    ClockDiscontinuity,
    StreamInvalidated,
    Backend,
    Other,
    Unknown,
}

impl StreamErrorKind {
    fn from_stored(value: u64) -> Self {
        match value {
            1 => Self::Xrun,
            2 => Self::ClockDiscontinuity,
            3 => Self::StreamInvalidated,
            4 => Self::Backend,
            5 => Self::Other,
            _ => Self::Unknown,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Xrun => "xrun",
            Self::ClockDiscontinuity => "clock_discontinuity",
            Self::StreamInvalidated => "stream_invalidated",
            Self::Backend => "backend",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }

    fn store_value(self) -> u64 {
        match self {
            Self::Xrun => 1,
            Self::ClockDiscontinuity => 2,
            Self::StreamInvalidated => 3,
            Self::Backend => 4,
            Self::Other => 5,
            Self::Unknown => u64::MAX,
        }
    }
}

fn stream_error_kind(error: DeviceError) -> StreamErrorKind {
    match error {
        DeviceError::Xrun => StreamErrorKind::Xrun,
        DeviceError::ClockDiscontinuity => StreamErrorKind::ClockDiscontinuity,
        DeviceError::StreamInvalidated => StreamErrorKind::StreamInvalidated,
        DeviceError::Backend(_) => StreamErrorKind::Backend,
        _ => StreamErrorKind::Other,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct StreamLifecycleEvent {
    sequence: u64,
    direction: StreamDirection,
    action: StreamLifecycleAction,
    succeeded: bool,
    host_monotonic_ns: u64,
    input_callback_index: u64,
    output_callback_index: u64,
}

struct StreamLifecycleLog {
    next: AtomicU64,
    overflow: AtomicU64,
    sequence: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    direction: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    action: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    succeeded: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    host_monotonic_ns: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    input_callback_index: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
    output_callback_index: [AtomicU64; STREAM_LIFECYCLE_LOG_SLOTS],
}

impl StreamLifecycleLog {
    fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            overflow: AtomicU64::new(0),
            sequence: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            direction: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            action: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            succeeded: std::array::from_fn(|_| AtomicU64::new(0)),
            host_monotonic_ns: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            input_callback_index: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            output_callback_index: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
        }
    }

    fn record(&self, event: StreamLifecycleEvent) {
        let slot = self.next.fetch_add(1, Ordering::Relaxed);
        let Ok(index) = usize::try_from(slot) else {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if index >= STREAM_LIFECYCLE_LOG_SLOTS {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.sequence[index].store(slot, Ordering::Relaxed);
        self.direction[index].store(event.direction.store_value(), Ordering::Relaxed);
        self.action[index].store(event.action.store_value(), Ordering::Relaxed);
        self.succeeded[index].store(u64::from(event.succeeded), Ordering::Relaxed);
        self.host_monotonic_ns[index].store(event.host_monotonic_ns, Ordering::Relaxed);
        self.input_callback_index[index].store(event.input_callback_index, Ordering::Relaxed);
        self.output_callback_index[index].store(event.output_callback_index, Ordering::Relaxed);
    }

    fn events(&self) -> Vec<StreamLifecycleEvent> {
        let len = self
            .next
            .load(Ordering::Relaxed)
            .min(u64::try_from(STREAM_LIFECYCLE_LOG_SLOTS).unwrap_or(u64::MAX));
        (0..len)
            .map(|slot| {
                let index = usize::try_from(slot).unwrap_or(usize::MAX);
                StreamLifecycleEvent {
                    sequence: self.sequence[index].load(Ordering::Relaxed),
                    direction: StreamDirection::from_stored(
                        self.direction[index].load(Ordering::Relaxed),
                    ),
                    action: StreamLifecycleAction::from_stored(
                        self.action[index].load(Ordering::Relaxed),
                    ),
                    succeeded: self.succeeded[index].load(Ordering::Relaxed) != 0,
                    host_monotonic_ns: self.host_monotonic_ns[index].load(Ordering::Relaxed),
                    input_callback_index: self.input_callback_index[index].load(Ordering::Relaxed),
                    output_callback_index: self.output_callback_index[index]
                        .load(Ordering::Relaxed),
                }
            })
            .collect()
    }

    fn event_count(&self) -> u64 {
        self.next
            .load(Ordering::Relaxed)
            .min(u64::try_from(STREAM_LIFECYCLE_LOG_SLOTS).unwrap_or(u64::MAX))
    }

    fn total_count(&self) -> u64 {
        self.next.load(Ordering::Relaxed)
    }

    fn overflow_count(&self) -> u64 {
        self.overflow.load(Ordering::Relaxed)
    }
}

impl Default for StreamLifecycleLog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct StreamErrorEvent {
    sequence: u64,
    direction: StreamDirection,
    kind: StreamErrorKind,
    host_monotonic_ns: u64,
    input_callback_index: u64,
    output_callback_index: u64,
}

struct StreamErrorLog {
    next: AtomicU64,
    overflow: AtomicU64,
    sequence: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
    direction: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
    kind: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
    host_monotonic_ns: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
    input_callback_index: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
    output_callback_index: [AtomicU64; STREAM_ERROR_LOG_SLOTS],
}

impl StreamErrorLog {
    fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            overflow: AtomicU64::new(0),
            sequence: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            direction: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            kind: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            host_monotonic_ns: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            input_callback_index: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            output_callback_index: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
        }
    }

    fn record(&self, event: StreamErrorEvent) {
        let slot = self.next.fetch_add(1, Ordering::Relaxed);
        let Ok(index) = usize::try_from(slot) else {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if index >= STREAM_ERROR_LOG_SLOTS {
            self.overflow.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.sequence[index].store(slot, Ordering::Relaxed);
        self.direction[index].store(event.direction.store_value(), Ordering::Relaxed);
        self.kind[index].store(event.kind.store_value(), Ordering::Relaxed);
        self.host_monotonic_ns[index].store(event.host_monotonic_ns, Ordering::Relaxed);
        self.input_callback_index[index].store(event.input_callback_index, Ordering::Relaxed);
        self.output_callback_index[index].store(event.output_callback_index, Ordering::Relaxed);
    }

    fn events(&self) -> Vec<StreamErrorEvent> {
        let len = self
            .next
            .load(Ordering::Relaxed)
            .min(u64::try_from(STREAM_ERROR_LOG_SLOTS).unwrap_or(u64::MAX));
        (0..len)
            .map(|slot| {
                let index = usize::try_from(slot).unwrap_or(usize::MAX);
                StreamErrorEvent {
                    sequence: self.sequence[index].load(Ordering::Relaxed),
                    direction: StreamDirection::from_stored(
                        self.direction[index].load(Ordering::Relaxed),
                    ),
                    kind: StreamErrorKind::from_stored(self.kind[index].load(Ordering::Relaxed)),
                    host_monotonic_ns: self.host_monotonic_ns[index].load(Ordering::Relaxed),
                    input_callback_index: self.input_callback_index[index].load(Ordering::Relaxed),
                    output_callback_index: self.output_callback_index[index]
                        .load(Ordering::Relaxed),
                }
            })
            .collect()
    }

    fn event_count(&self) -> u64 {
        self.next
            .load(Ordering::Relaxed)
            .min(u64::try_from(STREAM_ERROR_LOG_SLOTS).unwrap_or(u64::MAX))
    }

    fn total_count(&self) -> u64 {
        self.next.load(Ordering::Relaxed)
    }

    fn overflow_count(&self) -> u64 {
        self.overflow.load(Ordering::Relaxed)
    }
}

impl Default for StreamErrorLog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RenderRegressionDiagnostic {
    callback_index: u64,
    epoch: u64,
    epoch_callback_offset: u64,
    startup_grace: bool,
    reference_callback_index: Option<u64>,
    reference_callback_ns: Option<i64>,
    regression_backstep_ns: i64,
    backend_two_step_ns: Option<i64>,
    host_two_step_ns: Option<u64>,
    supply_two_step_frames: Option<u64>,
    playback_lead_before_ns: Option<i64>,
    playback_lead_ns: Option<i64>,
    playback_lead_after_ns: Option<i64>,
    playback_interval_ns: Option<i64>,
    next_playback_interval_ns: Option<i64>,
}

fn epoch_start_for_callback(callback_index: u64, boundary_cb: u64) -> u64 {
    if callback_index < boundary_cb {
        0
    } else {
        boundary_cb
    }
}

fn is_epoch_start_grace(callback_index: u64, boundary_cb: u64) -> bool {
    callback_index.saturating_sub(epoch_start_for_callback(callback_index, boundary_cb))
        < EPOCH_START_GRACE_CALLBACKS
}

/// Control-thread diagnostic projection of regression events already
/// classified by `TimestampEpochClassifier`. This never changes the
/// Pass/Fail oracle; it only preserves the values needed to separate
/// scheduling, backend clock bookkeeping, timestamp conversion, and
/// consumer-harness hypotheses in the provisioned artifact.
fn render_regression_diagnostics(
    events: &[RenderTimestampEvent],
    boundary_cb: u64,
) -> Vec<RenderRegressionDiagnostic> {
    events
        .iter()
        .enumerate()
        .filter_map(|(position, event)| {
            let regression_backstep_ns = event.regression_backstep_ns?;
            let previous = position.checked_sub(1).and_then(|index| events.get(index));
            let next = events.get(position + 1);
            let reference = events[..position]
                .iter()
                .filter(|candidate| candidate.epoch == event.epoch)
                .max_by_key(|candidate| candidate.callback_ns);
            let epoch_start = epoch_start_for_callback(event.callback_index, boundary_cb);
            Some(RenderRegressionDiagnostic {
                callback_index: event.callback_index,
                epoch: event.epoch,
                epoch_callback_offset: event.callback_index.saturating_sub(epoch_start),
                startup_grace: is_epoch_start_grace(event.callback_index, boundary_cb),
                reference_callback_index: reference.map(|event| event.callback_index),
                reference_callback_ns: reference.map(|event| event.callback_ns),
                regression_backstep_ns,
                backend_two_step_ns: previous
                    .zip(next)
                    .map(|(previous, next)| next.callback_ns.saturating_sub(previous.callback_ns)),
                host_two_step_ns: previous.zip(next).map(|(previous, next)| {
                    next.host_monotonic_ns
                        .saturating_sub(previous.host_monotonic_ns)
                }),
                supply_two_step_frames: previous.map(|previous| {
                    event
                        .supply_frame_cursor
                        .saturating_add(event.frames)
                        .saturating_sub(previous.supply_frame_cursor)
                }),
                playback_lead_before_ns: previous.and_then(|event| event.playback_lead_ns),
                playback_lead_ns: event.playback_lead_ns,
                playback_lead_after_ns: next.and_then(|event| event.playback_lead_ns),
                playback_interval_ns: event.playback_interval_ns,
                next_playback_interval_ns: next.and_then(|event| event.playback_interval_ns),
            })
        })
        .collect()
}

/// Per-pair observation state shared with the RT callbacks — the
/// wiring test's contract-level subset of the device verifier's
/// `Probe` (deep tolerance oracle stays in
/// `ruviform-audio-device/tests/virtual_endpoint.rs`; this probe
/// keeps only what the wiring oracle + lifecycle need).
#[derive(Default)]
struct EndpointProbe {
    in_callbacks: AtomicU64,
    in_frames: AtomicU64,
    in_channel_mismatches: AtomicU64,
    in_timestamp: TimestampEpochClassifier,
    in_domain_mismatches: AtomicU64,
    in_non_finite: AtomicU64,
    in_expected_domain: AtomicU64,
    in_errors: AtomicU64,
    in_xruns: AtomicU64,
    in_discontinuities: AtomicU64,
    in_invalidated: AtomicU64,
    in_backend_errors: AtomicU64,
    out_callbacks: AtomicU64,
    out_frames: AtomicU64,
    out_channel_mismatches: AtomicU64,
    out_timestamp: RenderTimestampInstrument,
    out_fill_errors: AtomicU64,
    out_errors: AtomicU64,
    out_xruns: AtomicU64,
    out_invalidated: AtomicU64,
    out_backend_errors: AtomicU64,
    stream_lifecycle_log: StreamLifecycleLog,
    stream_error_log: StreamErrorLog,
    /// Absolute render-frame cursor for `PcmFixture::fill_view`.
    out_render_cursor: AtomicU64,
    /// Bounded channel-0 capture handoff.
    capture: Mutex<CaptureBuf>,
    capture_lock_failures: AtomicU64,
    in_expected_channels: usize,
    out_expected_channels: usize,
}

impl EndpointProbe {
    fn new(input_format: &AudioFormat, output_format: &AudioFormat, capture_seconds: u64) -> Self {
        let capacity =
            usize::try_from(u64::from(input_format.sample_rate.get()) * (capture_seconds + 1))
                .unwrap_or(usize::MAX);
        Self {
            capture: Mutex::new(CaptureBuf::new(capacity)),
            in_expected_channels: usize::try_from(input_format.layout.channels())
                .unwrap_or(usize::MAX),
            out_expected_channels: usize::try_from(output_format.layout.channels())
                .unwrap_or(usize::MAX),
            ..Self::default()
        }
    }

    fn render_only(output_format: &AudioFormat) -> Self {
        Self {
            out_expected_channels: usize::try_from(output_format.layout.channels())
                .unwrap_or(usize::MAX),
            ..Self::default()
        }
    }
}

fn last_callback_index(callbacks: &AtomicU64) -> u64 {
    callbacks
        .load(Ordering::Relaxed)
        .checked_sub(1)
        .unwrap_or(u64::MAX)
}

fn record_stream_lifecycle(
    probe: &EndpointProbe,
    direction: StreamDirection,
    action: StreamLifecycleAction,
    succeeded: bool,
) {
    let sequence = probe.stream_lifecycle_log.total_count();
    probe.stream_lifecycle_log.record(StreamLifecycleEvent {
        sequence,
        direction,
        action,
        succeeded,
        host_monotonic_ns: probe.out_timestamp.host_monotonic_ns(),
        input_callback_index: last_callback_index(&probe.in_callbacks),
        output_callback_index: last_callback_index(&probe.out_callbacks),
    });
}

fn record_stream_error(probe: &EndpointProbe, direction: StreamDirection, kind: StreamErrorKind) {
    let sequence = probe.stream_error_log.total_count();
    probe.stream_error_log.record(StreamErrorEvent {
        sequence,
        direction,
        kind,
        host_monotonic_ns: probe.out_timestamp.host_monotonic_ns(),
        input_callback_index: last_callback_index(&probe.in_callbacks),
        output_callback_index: last_callback_index(&probe.out_callbacks),
    });
}

/// Bounded channel-0 capture buffer — same discipline as the device
/// verifier: overflow is counted, never silent.
#[derive(Default)]
struct CaptureBuf {
    capacity: usize,
    data: Vec<f32>,
    len: usize,
    overflow_frames: u64,
}

impl CaptureBuf {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: vec![0.0; capacity],
            len: 0,
            overflow_frames: 0,
        }
    }

    fn push(&mut self, input: &AudioBufferView<'_, f32>) {
        let frames = input.frames();
        let room = self.capacity.saturating_sub(self.len);
        let n = frames.min(room);
        if let Some(src) = input.channel(0) {
            self.data[self.len..self.len + n].copy_from_slice(&src[..n]);
            self.len += n;
        }
        self.overflow_frames += (frames - n) as u64;
    }
}

/// One enumerated endpoint reduced to what pairing needs — mirrors
/// `virtual_endpoint.rs`'s `EndpointCandidate` (kept in sync by
/// hand; the device→audio-test dev-dep direction is not approved,
/// so the heuristic cannot be shared).
struct EndpointCandidate {
    name: String,
    id: String,
    stem: Option<String>,
    format: Option<AudioFormat>,
    device: Box<dyn Device>,
}

struct Pair {
    render: EndpointCandidate,
    capture: EndpointCandidate,
}

/// Name fragments flagging a *candidate* virtual audio endpoint —
/// kept in sync by hand with `virtual_endpoint.rs` /
/// `examples/endpoint_probe.rs` (`VIRTUAL_NAME_HINTS`). A hit is a
/// candidate, never proof of provisioning.
const VIRTUAL_NAME_HINTS: &[&str] = &[
    "vb-cable",
    "vb audio",
    "vb-audio",
    "cable input",
    "cable output",
    "hi-fi cable",
    "hifi-cable",
    "virtual audio cable",
    "voicemeeter",
];

/// Direction words stripped before stem matching — same list as the
/// device verifier.
const DIRECTION_TOKENS: &[&str] = &["input", "output", "render", "capture", "in", "out"];

#[derive(Copy, Clone)]
enum Direction {
    Render,
    Capture,
}

fn is_virtual_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    VIRTUAL_NAME_HINTS.iter().any(|hint| lower.contains(hint))
}

fn virtual_stem(name: &str) -> Option<String> {
    if !is_virtual_name(name) {
        return None;
    }
    let stem = name
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| !DIRECTION_TOKENS.contains(token))
        .collect::<Vec<_>>()
        .join(" ");
    if stem.is_empty() { None } else { Some(stem) }
}

fn negotiate_input_format(device: &dyn Device) -> Option<AudioFormat> {
    match device.default_input_format() {
        Ok(format) if format.sample_format == SampleFormat::F32 => Some(format),
        _ => {
            let supported = device.supported_input_formats().ok()?;
            select_f32_input_format(&supported)
        }
    }
}

fn negotiate_output_format(device: &dyn Device) -> Option<AudioFormat> {
    match device.default_output_format() {
        Ok(format) if format.sample_format == SampleFormat::F32 => Some(format),
        _ => {
            let supported = device.supported_output_formats().ok()?;
            select_f32_output_format(&supported)
        }
    }
}

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

const fn layout_for_channels(channels: u32) -> Option<ChannelLayout> {
    match channels {
        1 => Some(ChannelLayout::Mono),
        2 => Some(ChannelLayout::Stereo),
        4 => Some(ChannelLayout::Quad),
        6 => Some(ChannelLayout::FivePointOne),
        8 => Some(ChannelLayout::SevenPointOne),
        _ => None,
    }
}

fn describe_candidate(device: Box<dyn Device>, direction: Direction) -> Option<EndpointCandidate> {
    let name = device.name().ok()?;
    let id = device
        .id()
        .ok()
        .flatten()
        .map_or_else(|| "?".to_owned(), |device_id| format!("{device_id:?}"));
    let format = match direction {
        Direction::Render => negotiate_output_format(device.as_ref()),
        Direction::Capture => negotiate_input_format(device.as_ref()),
    };
    let stem = virtual_stem(&name);
    Some(EndpointCandidate {
        name,
        id,
        stem,
        format,
        device,
    })
}

fn enumerate_devices(host: &Host, direction: Direction) -> Result<Vec<EndpointCandidate>, String> {
    let (devices, label) = match direction {
        Direction::Render => (host.output_devices(), "output"),
        Direction::Capture => (host.input_devices(), "input"),
    };
    let devices = devices.map_err(|error| format!("{label}_devices() failed: {error}"))?;
    if devices.is_empty() {
        return Err(format!("no {label} devices on this host"));
    }
    Ok(devices
        .into_iter()
        .filter_map(|device| describe_candidate(device, direction))
        .collect())
}

fn pair_candidates(renders: Vec<EndpointCandidate>, captures: Vec<EndpointCandidate>) -> Vec<Pair> {
    let mut free: Vec<Option<EndpointCandidate>> = captures.into_iter().map(Some).collect();
    let mut pairs = Vec::new();
    for render in renders {
        let Some(stem) = &render.stem else {
            continue;
        };
        let Some(index) = free.iter().position(|candidate| {
            candidate
                .as_ref()
                .is_some_and(|capture| capture.stem.as_deref() == Some(stem.as_str()))
        }) else {
            continue;
        };
        if let Some(capture) = free[index].take() {
            pairs.push(Pair { render, capture });
        }
    }
    pairs
}

fn spawn_watchdog() {
    thread::spawn(|| {
        thread::sleep(MAX_RUN);
        eprintln!("os_endpoint: watchdog timeout after {MAX_RUN:?} — aborting");
        process::exit(3);
    });
}

/// Audio virtual-endpoint leg — `PcmFixture` supplies the stimulus
/// through the production output callback; the captured channel-0 PCM
/// is checked by the independent contract oracle.
#[test]
#[ignore = "requires a provisioned virtual audio endpoint pair; manual opt-in check"]
fn audio_virtual_endpoint_round_trip() {
    spawn_watchdog();
    let meta = endpoint_record(
        "rf629.audio.virtual-endpoint.render-capture",
        "marker-rms-contract-v1",
    );
    eprintln!("os_endpoint: audio render→capture wiring verifier");

    let host = match default_host() {
        Ok(host) => host,
        Err(error) => {
            let record = meta.clone().with_result(
                CaseResult::NotRun,
                format!("default_host() failed: {error}"),
            );
            emit_record(&record);
            eprintln!("verdict: NotRun — default_host() failed: {error}");
            return;
        }
    };
    eprintln!("  host: {}", host.name());

    let renders = match enumerate_devices(&host, Direction::Render) {
        Ok(candidates) => candidates,
        Err(reason) => {
            let record = meta.clone().with_result(CaseResult::NotRun, reason.clone());
            emit_record(&record);
            eprintln!("verdict: NotRun — {reason}");
            return;
        }
    };
    let captures = match enumerate_devices(&host, Direction::Capture) {
        Ok(candidates) => candidates,
        Err(reason) => {
            let record = meta.clone().with_result(CaseResult::NotRun, reason.clone());
            emit_record(&record);
            eprintln!("verdict: NotRun — {reason}");
            return;
        }
    };

    let pairs = pair_candidates(renders, captures);
    run_audio_pair_candidates(pairs, &meta);
}

/// Render-only baseline for Issue #1034: supply the deterministic
/// stimulus to a virtual render endpoint and record the output callback
/// timestamp series without depending on capture loopback or faults.
#[test]
#[ignore = "requires a provisioned virtual render endpoint; manual opt-in check"]
fn audio_render_timestamp_baseline() {
    spawn_watchdog();
    let meta = endpoint_record(
        "rf1034.audio.virtual-endpoint.render-timestamp-baseline",
        "render-callback-timestamp-contract-v1",
    );
    eprintln!("os_endpoint: render-only timestamp baseline verifier");

    let host = match default_host() {
        Ok(host) => host,
        Err(error) => {
            let record = meta.clone().with_result(
                CaseResult::NotRun,
                format!("default_host() failed: {error}"),
            );
            emit_record(&record);
            eprintln!("verdict: NotRun — default_host() failed: {error}");
            return;
        }
    };
    eprintln!("  host: {}", host.name());

    let renders = match enumerate_devices(&host, Direction::Render) {
        Ok(candidates) => candidates,
        Err(reason) => {
            let record = meta.clone().with_result(CaseResult::NotRun, reason.clone());
            emit_record(&record);
            eprintln!("verdict: NotRun — {reason}");
            return;
        }
    };
    let candidates: Vec<EndpointCandidate> = renders
        .into_iter()
        .filter(|candidate| candidate.stem.is_some())
        .collect();
    run_render_baseline_candidates(candidates, &meta);
}

fn run_render_baseline_candidates(candidates: Vec<EndpointCandidate>, meta: &RunMetadata) {
    if candidates.is_empty() {
        let record = meta.clone().with_result(
            CaseResult::NotRun,
            "NOT_PROVISIONED — no virtual render endpoint candidate",
        );
        emit_record(&record);
        eprintln!(
            "verdict: NotRun — NOT_PROVISIONED (no virtual render endpoint candidate; provision a virtual audio endpoint first)"
        );
        return;
    }
    eprintln!("  render candidates: {}", candidates.len());

    let mut candidate_failures: Vec<String> = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        eprintln!(
            "render[{index}]: \"{}\" (candidate, unconfirmed)",
            candidate.name
        );
        match run_render_timestamp_baseline(&candidate) {
            Ok((rate, summary)) => {
                let record = endpoint_record_at(
                    "rf1034.audio.virtual-endpoint.render-timestamp-baseline",
                    "render-callback-timestamp-contract-v1",
                    rate,
                )
                .with_expected(
                    "bounded render callback timestamp series; monotonic callback instants inside each stream epoch after the bounded epoch-start grace window",
                )
                .with_result(
                    CaseResult::Pass,
                    format!("render[{index}] baseline observed at {rate} Hz ({summary})"),
                )
                .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs")
                .with_evidence("name match is a candidate, not proof of a virtual endpoint");
                emit_record(&record);
                eprintln!("verdict: Pass — render timestamp baseline observed");
                return;
            }
            Err(failures) => {
                eprintln!("  render[{index}] failed:");
                for failure in &failures {
                    eprintln!("    - {failure}");
                }
                candidate_failures.push(format!("render[{index}]: {}", failures.join("; ")));
            }
        }
    }
    let record = meta.clone().with_result(
        CaseResult::Fail,
        format!(
            "no render candidate completed the timestamp baseline: {}",
            candidate_failures.join("; ")
        ),
    );
    emit_record(&record);
    panic!(
        "verdict: Fail — no render candidate completed the timestamp baseline:\n{}",
        candidate_failures.join("\n")
    );
}

fn run_render_timestamp_baseline(render: &EndpointCandidate) -> Result<(u32, String), Vec<String>> {
    eprintln!("  render: \"{}\" id={}", render.name, render.id);
    let Some(out_format) = render.format else {
        return Err(vec!["no F32-openable output format".to_owned()]);
    };
    eprintln!("  negotiated out={out_format:?}");

    let probe = Arc::new(EndpointProbe::render_only(&out_format));
    let channels = u16::try_from(out_format.layout.channels()).unwrap_or(u16::MAX);
    let render_rate = out_format.sample_rate.get();
    let period_frames = (u64::from(render_rate) * IMPULSE_PERIOD_MS / 1_000).max(1);
    let fixture = PcmFixture::new(
        out_format.sample_rate,
        channels,
        PcmPattern::Impulse {
            period_frames: u32::try_from(period_frames).unwrap_or(u32::MAX),
            amplitude: IMPULSE_AMPLITUDE,
        },
    )
    .map_err(|error| vec![format!("fixture construction failed: {error}")])?;

    let out_stream =
        match open_output_with_fallback(render.device.as_ref(), out_format, &probe, fixture) {
            Ok(stream) => stream,
            Err(error) => return Err(vec![format!("output open failed: {error}")]),
        };
    let out_domain = out_stream.clock_domain();
    eprintln!(
        "  out stream: format={:?} domain={out_domain:?}",
        out_stream.format(),
    );

    if let Err(error) = out_stream.start() {
        return Err(vec![format!("stream start failed: {error}")]);
    }
    thread::sleep(Duration::from_secs(CAPTURE_SECONDS));
    let lifecycle = run_render_baseline_lifecycle(&probe, &*out_stream);
    drop(out_stream);

    let mut failures = analyze_render_baseline(&probe);
    failures.extend(lifecycle);
    let summary = render_baseline_summary(&probe);
    if failures.is_empty() {
        Ok((render_rate, summary))
    } else {
        Err(failures)
    }
}

fn run_render_baseline_lifecycle(
    probe: &EndpointProbe,
    out_stream: &dyn OutputStream,
) -> Vec<String> {
    let mut lifecycle = Vec::new();
    if let Err(error) = out_stream.stop() {
        lifecycle.push(format!("output stop failed: {error}"));
    }
    for (label, result) in [
        ("out buffer_size", out_stream.buffer_size().map(|_| ())),
        ("out now", out_stream.now().map(|_| ())),
    ] {
        if let Err(error) = result {
            lifecycle.push(format!("post-stop {label} query failed: {error}"));
        }
    }
    probe
        .out_timestamp
        .split_epoch(probe.out_callbacks.load(Ordering::Relaxed));
    match out_stream.start() {
        Ok(()) => {
            thread::sleep(Duration::from_millis(300));
            if let Err(error) = out_stream.stop() {
                lifecycle.push(format!("output re-stop failed: {error}"));
            }
        }
        Err(error) => lifecycle.push(format!("restart start failed: {error}")),
    }
    lifecycle
}

fn analyze_render_baseline(probe: &EndpointProbe) -> Vec<String> {
    let mut failures = Vec::new();
    let out_cb = probe.out_callbacks.load(Ordering::Relaxed);
    if out_cb == 0 {
        failures.push("open+start but zero output data callbacks".to_owned());
    }
    if probe.out_frames.load(Ordering::Relaxed) == 0 {
        failures.push("output stream delivered zero frames".to_owned());
    }
    for (label, count) in [
        (
            "output channel mismatches",
            probe.out_channel_mismatches.load(Ordering::Relaxed),
        ),
        (
            "output timestamp regressions",
            probe.out_timestamp.effective_regressions(),
        ),
        (
            "output fixture fill errors",
            probe.out_fill_errors.load(Ordering::Relaxed),
        ),
    ] {
        if count > 0 {
            failures.push(format!("{label}: {count}"));
        }
    }
    log_render_baseline_stats(probe, out_cb);
    emit_render_timestamp_events(probe);
    failures
}

fn render_baseline_summary(probe: &EndpointProbe) -> String {
    format!(
        "callbacks={} frames={} raw_regressions={} effective_regressions={} playback_some={} playback_none={} playback_regressions={} max_active_callbacks={} overlap_events={} log_overflow={}",
        probe.out_callbacks.load(Ordering::Relaxed),
        probe.out_frames.load(Ordering::Relaxed),
        probe.out_timestamp.raw_regressions(),
        probe.out_timestamp.effective_regressions(),
        probe.out_timestamp.playback_some_count(),
        probe.out_timestamp.playback_none_count(),
        probe.out_timestamp.playback_regression_count(),
        probe.out_timestamp.max_active_callbacks(),
        probe.out_timestamp.overlapping_callback_events(),
        probe.out_timestamp.log_overflows(),
    )
}

fn log_render_baseline_stats(probe: &EndpointProbe, out_cb: u64) {
    let boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    eprintln!(
        "  render baseline stats: {}",
        render_baseline_summary(probe)
    );
    eprintln!(
        "  render baseline timestamps: out_cb={out_cb} epoch_resets={} boundary_cb={} first..last={}..{} max_backstep_ns={} idx=[{}]",
        probe
            .out_timestamp
            .classifier
            .epoch_resets
            .load(Ordering::Relaxed),
        boundary_cb,
        probe
            .out_timestamp
            .classifier
            .regression_first_cb
            .load(Ordering::Relaxed),
        probe
            .out_timestamp
            .classifier
            .regression_last_cb
            .load(Ordering::Relaxed),
        probe
            .out_timestamp
            .classifier
            .regression_max_backstep_ns
            .load(Ordering::Relaxed),
        probe.out_timestamp.regression_index_log(),
    );
}

fn optional_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn timestamp_event_lines(
    prefix: &str,
    context: &str,
    events: &[RenderTimestampEvent],
    boundary_cb: u64,
) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            format!(
                "{prefix}: {context}index={} epoch={} epoch_boundary_cb={} is_epoch_boundary={} frames={} supply_frame_cursor={} callback_ns={} playback_present={} playback_ns={} interval_ns={} regression_backstep_ns={} host_monotonic_ns={} host_interval_ns={} playback_interval_ns={} playback_lead_ns={}",
                event.callback_index,
                event.epoch,
                boundary_cb,
                u8::from(event.callback_index == boundary_cb),
                event.frames,
                event.supply_frame_cursor,
                event.callback_ns,
                u8::from(event.playback_ns.is_some()),
                optional_i64(event.playback_ns),
                optional_i64(event.interval_ns),
                optional_i64(event.regression_backstep_ns),
                event.host_monotonic_ns,
                optional_u64(event.host_interval_ns),
                optional_i64(event.playback_interval_ns),
                optional_i64(event.playback_lead_ns),
            )
        })
        .collect()
}

fn timestamp_regression_diagnostic_lines(
    prefix: &str,
    context: &str,
    events: &[RenderTimestampEvent],
    boundary_cb: u64,
) -> Vec<String> {
    render_regression_diagnostics(events, boundary_cb)
        .iter()
        .map(|diagnostic| {
            format!(
                "{prefix}: {context}index={} epoch={} epoch_offset={} startup_grace={} reference_index={} reference_callback_ns={} backstep_ns={} backend_two_step_ns={} host_two_step_ns={} supply_two_step_frames={} playback_lead_before_ns={} playback_lead_ns={} playback_lead_after_ns={} playback_interval_ns={} next_playback_interval_ns={}",
                diagnostic.callback_index,
                diagnostic.epoch,
                diagnostic.epoch_callback_offset,
                u8::from(diagnostic.startup_grace),
                optional_u64(diagnostic.reference_callback_index),
                optional_i64(diagnostic.reference_callback_ns),
                diagnostic.regression_backstep_ns,
                optional_i64(diagnostic.backend_two_step_ns),
                optional_u64(diagnostic.host_two_step_ns),
                optional_u64(diagnostic.supply_two_step_frames),
                optional_i64(diagnostic.playback_lead_before_ns),
                optional_i64(diagnostic.playback_lead_ns),
                optional_i64(diagnostic.playback_lead_after_ns),
                optional_i64(diagnostic.playback_interval_ns),
                optional_i64(diagnostic.next_playback_interval_ns),
            )
        })
        .collect()
}

fn baseline_timestamp_diagnostic_lines(probe: &EndpointProbe) -> Vec<String> {
    let boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    let events = probe.out_timestamp.events();
    let mut lines = timestamp_event_lines("render_timestamp_event", "", &events, boundary_cb);
    lines.extend(timestamp_regression_diagnostic_lines(
        "render_timestamp_regression_diag",
        "",
        &events,
        boundary_cb,
    ));
    lines.push(format!(
        "render_timestamp_log: events={} overflow={} bounded_slots={}",
        events.len(),
        probe.out_timestamp.log_overflows(),
        RENDER_TIMESTAMP_LOG_SLOTS,
    ));
    lines
}

fn emit_render_timestamp_events(probe: &EndpointProbe) {
    for line in baseline_timestamp_diagnostic_lines(probe) {
        eprintln!("{line}");
    }
}

fn callback_index_text(index: u64) -> String {
    if index == u64::MAX {
        "none".to_owned()
    } else {
        index.to_string()
    }
}

fn roundtrip_output_context(pair_index: usize) -> String {
    format!("path={ROUNDTRIP_OUTPUT_PATH} case={ROUNDTRIP_AUDIO_CASE_ID} pair={pair_index} ")
}

fn roundtrip_output_summary_line(
    probe: &EndpointProbe,
    pair_index: usize,
    timestamp_events: usize,
) -> String {
    let context = roundtrip_output_context(pair_index);
    let classifier = &probe.out_timestamp.classifier;
    format!(
        "roundtrip_output_timestamp_summary: {context}input_callbacks={} output_callbacks={} output_frames={} raw_regressions={} effective_regressions={} epoch_resets={} boundary_cb={} regression_first_cb={} regression_last_cb={} regression_max_backstep_ns={} regression_idx=[{}] playback_some={} playback_none={} playback_regressions={} max_active_callbacks={} overlap_events={} timestamp_events={} timestamp_log_overflow={} input_errors={} input_xruns={} input_discontinuities={} input_invalidated={} input_backend_errors={} output_errors={} output_xruns={} output_invalidated={} output_backend_errors={} error_events={} error_log_overflow={} lifecycle_events={} lifecycle_log_overflow={}",
        probe.in_callbacks.load(Ordering::Relaxed),
        probe.out_callbacks.load(Ordering::Relaxed),
        probe.out_frames.load(Ordering::Relaxed),
        probe.out_timestamp.raw_regressions(),
        probe.out_timestamp.effective_regressions(),
        classifier.epoch_resets.load(Ordering::Relaxed),
        classifier.epoch_boundary_cb.load(Ordering::Relaxed),
        classifier.regression_first_cb.load(Ordering::Relaxed),
        classifier.regression_last_cb.load(Ordering::Relaxed),
        classifier
            .regression_max_backstep_ns
            .load(Ordering::Relaxed),
        probe.out_timestamp.regression_index_log(),
        probe.out_timestamp.playback_some_count(),
        probe.out_timestamp.playback_none_count(),
        probe.out_timestamp.playback_regression_count(),
        probe.out_timestamp.max_active_callbacks(),
        probe.out_timestamp.overlapping_callback_events(),
        timestamp_events,
        probe.out_timestamp.log_overflows(),
        probe.in_errors.load(Ordering::Relaxed),
        probe.in_xruns.load(Ordering::Relaxed),
        probe.in_discontinuities.load(Ordering::Relaxed),
        probe.in_invalidated.load(Ordering::Relaxed),
        probe.in_backend_errors.load(Ordering::Relaxed),
        probe.out_errors.load(Ordering::Relaxed),
        probe.out_xruns.load(Ordering::Relaxed),
        probe.out_invalidated.load(Ordering::Relaxed),
        probe.out_backend_errors.load(Ordering::Relaxed),
        probe.stream_error_log.event_count(),
        probe.stream_error_log.overflow_count(),
        probe.stream_lifecycle_log.event_count(),
        probe.stream_lifecycle_log.overflow_count(),
    )
}

fn stream_lifecycle_event_lines(probe: &EndpointProbe, context: &str) -> Vec<String> {
    probe
        .stream_lifecycle_log
        .events()
        .iter()
        .map(|event| {
            format!(
                "roundtrip_stream_lifecycle_event: {context}sequence={} direction={} action={} result={} host_monotonic_ns={} input_callback_index={} output_callback_index={}",
                event.sequence,
                event.direction.as_str(),
                event.action.as_str(),
                if event.succeeded { "ok" } else { "error" },
                event.host_monotonic_ns,
                callback_index_text(event.input_callback_index),
                callback_index_text(event.output_callback_index),
            )
        })
        .collect()
}

fn stream_error_event_lines(probe: &EndpointProbe, context: &str) -> Vec<String> {
    probe
        .stream_error_log
        .events()
        .iter()
        .map(|event| {
            format!(
                "roundtrip_stream_error_event: {context}sequence={} direction={} kind={} host_monotonic_ns={} input_callback_index={} output_callback_index={}",
                event.sequence,
                event.direction.as_str(),
                event.kind.as_str(),
                event.host_monotonic_ns,
                callback_index_text(event.input_callback_index),
                callback_index_text(event.output_callback_index),
            )
        })
        .collect()
}

fn roundtrip_output_diagnostic_lines(probe: &EndpointProbe, pair_index: usize) -> Vec<String> {
    let boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    let events = probe.out_timestamp.events();
    let context = roundtrip_output_context(pair_index);
    let mut lines = vec![roundtrip_output_summary_line(
        probe,
        pair_index,
        events.len(),
    )];
    lines.extend(stream_lifecycle_event_lines(probe, &context));
    lines.extend(stream_error_event_lines(probe, &context));
    lines.extend(timestamp_event_lines(
        "roundtrip_output_timestamp_event",
        &context,
        &events,
        boundary_cb,
    ));
    lines.extend(timestamp_regression_diagnostic_lines(
        "roundtrip_output_timestamp_regression_diag",
        &context,
        &events,
        boundary_cb,
    ));
    lines.push(format!(
        "roundtrip_output_timestamp_log: {context}events={} overflow={} bounded_slots={}",
        events.len(),
        probe.out_timestamp.log_overflows(),
        RENDER_TIMESTAMP_LOG_SLOTS,
    ));
    lines
}

fn emit_roundtrip_output_diagnostics(probe: &EndpointProbe, pair_index: usize) {
    for line in roundtrip_output_diagnostic_lines(probe, pair_index) {
        eprintln!("{line}");
    }
}

/// Runs every stem-matched pair candidate until one completes the
/// round-trip. Empty candidate list → `NotRun NOT_PROVISIONED`;
/// all candidates failing → `Fail` record + panic.
fn run_audio_pair_candidates(pairs: Vec<Pair>, meta: &RunMetadata) {
    if pairs.is_empty() {
        let record = meta.clone().with_result(
            CaseResult::NotRun,
            "NOT_PROVISIONED — no render↔capture virtual name-stem pair",
        );
        emit_record(&record);
        eprintln!(
            "verdict: NotRun — NOT_PROVISIONED (no render↔capture virtual \
             name-stem pair; provision VB-CABLE/VAC-style endpoints first)"
        );
        return;
    }
    eprintln!("  pair candidates: {}", pairs.len());

    let mut pair_failures: Vec<String> = Vec::new();
    for (index, pair) in pairs.into_iter().enumerate() {
        eprintln!(
            "pair[{index}]: render=\"{}\" capture=\"{}\" (candidate, unconfirmed)",
            pair.render.name, pair.capture.name
        );
        match run_audio_round_trip(pair, index) {
            Ok((in_rate, out_rate)) => {
                let record = endpoint_record_at(
                    "rf629.audio.virtual-endpoint.render-capture",
                    "marker-rms-contract-v1",
                    out_rate,
                )
                .with_expected(
                    "impulse train 0.8/100ms via PcmFixture::fill_view; \
                     marker spacing + RMS band + callback contract",
                )
                .with_result(
                    CaseResult::Pass,
                    format!(
                        "pair[{index}] round-trip observed through real OS streams \
                         (negotiated in={in_rate} Hz out={out_rate} Hz)"
                    ),
                )
                .with_evidence("crates/ruviform-audio-test/tests/os_endpoint.rs")
                .with_evidence("name match is a candidate, not proof of a virtual endpoint");
                emit_record(&record);
                eprintln!("verdict: Pass — render→capture round-trip observed");
                return;
            }
            Err(failures) => {
                eprintln!("  pair[{index}] failed:");
                for failure in &failures {
                    eprintln!("    - {failure}");
                }
                pair_failures.push(format!("pair[{index}]: {}", failures.join("; ")));
            }
        }
    }
    let record = meta.clone().with_result(
        CaseResult::Fail,
        format!(
            "no pair candidate completed the round-trip: {}",
            pair_failures.join("; ")
        ),
    );
    emit_record(&record);
    panic!(
        "verdict: Fail — no pair candidate completed the round-trip:\n{}",
        pair_failures.join("\n")
    );
}

/// One full audio round-trip: negotiate → open → stream (fixture
/// supply) → observe → oracle → lifecycle. `Ok` carries the
/// negotiated `(input, output)` sample rates so the Pass record's
/// `RunParameters` reflect what actually ran; `Err` carries every
/// violation found.
fn run_audio_round_trip(pair: Pair, pair_index: usize) -> Result<(u32, u32), Vec<String>> {
    let Pair { render, capture } = pair;
    eprintln!(
        "  render: \"{}\" id={} | capture: \"{}\" id={}",
        render.name, render.id, capture.name, capture.id
    );
    let Some(in_format) = capture.format else {
        return Err(vec!["no F32-openable input format".to_owned()]);
    };
    let Some(out_format) = render.format else {
        return Err(vec!["no F32-openable output format".to_owned()]);
    };
    eprintln!("  negotiated in={in_format:?} out={out_format:?}");

    let probe = Arc::new(EndpointProbe::new(&in_format, &out_format, CAPTURE_SECONDS));

    // Test Runtime supply: the fixture is built for the negotiated
    // output format — `fill_view` runs inside the RT callback.
    let channels = u16::try_from(out_format.layout.channels()).unwrap_or(u16::MAX);
    let render_rate = out_format.sample_rate.get();
    let period_frames = (u64::from(render_rate) * IMPULSE_PERIOD_MS / 1_000).max(1);
    let fixture = PcmFixture::new(
        out_format.sample_rate,
        channels,
        PcmPattern::Impulse {
            period_frames: u32::try_from(period_frames).unwrap_or(u32::MAX),
            amplitude: IMPULSE_AMPLITUDE,
        },
    )
    .map_err(|error| vec![format!("fixture construction failed: {error}")])?;

    let mut backend_trace =
        start_backend_os_trace(pair_index, probe.out_timestamp.host_origin, &probe);

    let in_stream = match open_input_with_fallback(capture.device.as_ref(), in_format, &probe) {
        Ok(stream) => stream,
        Err(error) => {
            emit_roundtrip_output_diagnostics(&probe, pair_index);
            return Err(vec![format!("input open failed: {error}")]);
        }
    };
    let out_stream =
        match open_output_with_fallback(render.device.as_ref(), out_format, &probe, fixture) {
            Ok(stream) => stream,
            Err(error) => {
                drop(in_stream);
                emit_roundtrip_output_diagnostics(&probe, pair_index);
                return Err(vec![format!("output open failed: {error}")]);
            }
        };

    let in_domain = in_stream.clock_domain();
    let out_domain = out_stream.clock_domain();
    probe
        .in_expected_domain
        .store(in_domain.get(), Ordering::Relaxed);
    eprintln!(
        "  in stream: format={:?} domain={in_domain:?} | out stream: format={:?} domain={out_domain:?}",
        in_stream.format(),
        out_stream.format(),
    );

    let input_start = in_stream.start();
    record_stream_lifecycle(
        &probe,
        StreamDirection::Input,
        StreamLifecycleAction::Start,
        input_start.is_ok(),
    );
    if let Err(error) = input_start {
        drop(in_stream);
        drop(out_stream);
        emit_roundtrip_output_diagnostics(&probe, pair_index);
        return Err(vec![format!("input stream start failed: {error}")]);
    }
    let output_start = out_stream.start();
    record_stream_lifecycle(
        &probe,
        StreamDirection::Output,
        StreamLifecycleAction::Start,
        output_start.is_ok(),
    );
    if let Err(error) = output_start {
        drop(in_stream);
        drop(out_stream);
        emit_roundtrip_output_diagnostics(&probe, pair_index);
        return Err(vec![format!("output stream start failed: {error}")]);
    }
    thread::sleep(Duration::from_secs(CAPTURE_SECONDS));
    let lifecycle = run_audio_lifecycle(&probe, &*in_stream, &*out_stream);
    drop(in_stream);
    drop(out_stream);
    emit_roundtrip_output_diagnostics(&probe, pair_index);
    backend_trace.finish();

    let mut failures = analyze_endpoint(&probe, &in_format);
    failures.extend(lifecycle);
    if failures.is_empty() {
        Ok((in_format.sample_rate.get(), out_format.sample_rate.get()))
    } else {
        Err(failures)
    }
}

fn run_audio_lifecycle(
    probe: &EndpointProbe,
    in_stream: &dyn InputStream,
    out_stream: &dyn OutputStream,
) -> Vec<String> {
    let mut lifecycle = Vec::new();
    let input_stop = in_stream.stop();
    record_stream_lifecycle(
        probe,
        StreamDirection::Input,
        StreamLifecycleAction::Stop,
        input_stop.is_ok(),
    );
    if let Err(error) = input_stop {
        lifecycle.push(format!("input stop failed: {error}"));
    }
    let output_stop = out_stream.stop();
    record_stream_lifecycle(
        probe,
        StreamDirection::Output,
        StreamLifecycleAction::Stop,
        output_stop.is_ok(),
    );
    if let Err(error) = output_stop {
        lifecycle.push(format!("output stop failed: {error}"));
    }
    for (label, result) in [
        ("in buffer_size", in_stream.buffer_size().map(|_| ())),
        ("in now", in_stream.now().map(|_| ())),
        ("out buffer_size", out_stream.buffer_size().map(|_| ())),
        ("out now", out_stream.now().map(|_| ())),
    ] {
        if let Err(error) = result {
            lifecycle.push(format!("post-stop {label} query failed: {error}"));
        }
    }
    // Epoch boundary: stop/start re-anchors the OS-side timestamp
    // epoch on shared-mode streams. Split both baselines so the
    // oracle keeps measuring monotonicity inside each continuous run;
    // the reset itself is lifecycle evidence, not hidden.
    probe
        .in_timestamp
        .split_epoch(probe.in_callbacks.load(Ordering::Relaxed));
    record_stream_lifecycle(
        probe,
        StreamDirection::Input,
        StreamLifecycleAction::EpochSplit,
        true,
    );
    probe
        .out_timestamp
        .split_epoch(probe.out_callbacks.load(Ordering::Relaxed));
    record_stream_lifecycle(
        probe,
        StreamDirection::Output,
        StreamLifecycleAction::EpochSplit,
        true,
    );

    let input_restart = in_stream.start();
    record_stream_lifecycle(
        probe,
        StreamDirection::Input,
        StreamLifecycleAction::Restart,
        input_restart.is_ok(),
    );
    let output_restart = if input_restart.is_ok() {
        let result = out_stream.start();
        record_stream_lifecycle(
            probe,
            StreamDirection::Output,
            StreamLifecycleAction::Restart,
            result.is_ok(),
        );
        result
    } else {
        Err(DeviceError::StreamInvalidated)
    };
    match input_restart.and(output_restart) {
        Ok(()) => {
            thread::sleep(Duration::from_millis(300));
            let input_restop = in_stream.stop();
            record_stream_lifecycle(
                probe,
                StreamDirection::Input,
                StreamLifecycleAction::ReStop,
                input_restop.is_ok(),
            );
            if let Err(error) = input_restop {
                lifecycle.push(format!("input re-stop failed: {error}"));
            }
            let output_restop = out_stream.stop();
            record_stream_lifecycle(
                probe,
                StreamDirection::Output,
                StreamLifecycleAction::ReStop,
                output_restop.is_ok(),
            );
            if let Err(error) = output_restop {
                lifecycle.push(format!("output re-stop failed: {error}"));
            }
        }
        Err(error) => lifecycle.push(format!("restart start failed: {error}")),
    }
    lifecycle
}

fn open_input_with_fallback(
    device: &dyn Device,
    format: AudioFormat,
    probe: &Arc<EndpointProbe>,
) -> Result<Box<dyn InputStream>, DeviceError> {
    let requested = InputStreamConfig {
        format,
        buffer_size: BufferSize::Fixed(REQUESTED_FRAMES),
    };
    match device.open_input_stream(
        &requested,
        input_data_callback(probe),
        input_error_callback(probe),
    ) {
        Ok(stream) => Ok(stream),
        Err(DeviceError::UnsupportedConfig) => {
            let fallback = InputStreamConfig {
                format,
                buffer_size: BufferSize::Default,
            };
            device.open_input_stream(
                &fallback,
                input_data_callback(probe),
                input_error_callback(probe),
            )
        }
        Err(error) => Err(error),
    }
}

fn open_output_with_fallback(
    device: &dyn Device,
    format: AudioFormat,
    probe: &Arc<EndpointProbe>,
    fixture: PcmFixture,
) -> Result<Box<dyn OutputStream>, DeviceError> {
    let requested = OutputStreamConfig {
        format,
        buffer_size: BufferSize::Fixed(REQUESTED_FRAMES),
    };
    match device.open_output_stream(
        &requested,
        output_data_callback(probe, fixture),
        output_error_callback(probe),
    ) {
        Ok(stream) => Ok(stream),
        Err(DeviceError::UnsupportedConfig) => {
            let fallback = OutputStreamConfig {
                format,
                buffer_size: BufferSize::Default,
            };
            device.open_output_stream(
                &fallback,
                output_data_callback(probe, fixture),
                output_error_callback(probe),
            )
        }
        Err(error) => Err(error),
    }
}

fn input_data_callback(probe: &Arc<EndpointProbe>) -> InputDataCallback {
    let probe = Arc::clone(probe);
    Box::new(move |input, info| observe_input(&probe, input, info))
}

fn observe_input(probe: &EndpointProbe, input: AudioBufferView<'_, f32>, info: InputCallbackInfo) {
    let frames = input.frames();
    let callback_index = probe.in_callbacks.fetch_add(1, Ordering::Relaxed);
    probe.in_frames.fetch_add(frames as u64, Ordering::Relaxed);
    if input.channels() != probe.in_expected_channels {
        probe.in_channel_mismatches.fetch_add(1, Ordering::Relaxed);
    }
    let callback_ns = info.timestamp.callback.value();
    probe.in_timestamp.observe(callback_index, callback_ns);
    if info.clock_domain.get() != probe.in_expected_domain.load(Ordering::Relaxed) {
        probe.in_domain_mismatches.fetch_add(1, Ordering::Relaxed);
    }

    let mut non_finite = 0_u64;
    for sample in input.data() {
        if !sample.is_finite() {
            non_finite += 1;
        }
    }
    probe.in_non_finite.fetch_add(non_finite, Ordering::Relaxed);

    match probe.capture.try_lock() {
        Ok(mut buf) => buf.push(&input),
        Err(_) => {
            probe.capture_lock_failures.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn output_data_callback(probe: &Arc<EndpointProbe>, fixture: PcmFixture) -> OutputDataCallback {
    let probe = Arc::clone(probe);
    Box::new(move |mut out, info| observe_output(&probe, &mut out, info, fixture))
}

/// The output callback's supply point: `PcmFixture::fill_view`
/// writes the deterministic impulse train into the production
/// `AudioBufferViewMut` — the Test Runtime's signal entering the
/// real OS render path. RT-safe: pure sample math, no allocation.
fn observe_output(
    probe: &EndpointProbe,
    out: &mut AudioBufferViewMut<'_, f32>,
    info: OutputCallbackInfo,
    fixture: PcmFixture,
) {
    let frames = out.frames();
    let callback_index = probe.out_callbacks.fetch_add(1, Ordering::Relaxed);
    probe.out_frames.fetch_add(frames as u64, Ordering::Relaxed);
    if out.channels() != probe.out_expected_channels {
        probe.out_channel_mismatches.fetch_add(1, Ordering::Relaxed);
    }
    let _activity = probe.out_timestamp.enter_callback();
    let callback_ns = info.timestamp.callback.value();
    let base = probe
        .out_render_cursor
        .fetch_add(frames as u64, Ordering::Relaxed);
    probe.out_timestamp.observe(
        callback_index,
        callback_ns,
        info.timestamp.playback.map(MediaTime::value),
        frames as u64,
        base,
    );
    if fixture.fill_view(base, out).is_err() {
        probe.out_fill_errors.fetch_add(1, Ordering::Relaxed);
    }
}

fn input_error_callback(probe: &Arc<EndpointProbe>) -> InputErrorCallback {
    let probe = Arc::clone(probe);
    Box::new(move |error: DeviceError| {
        probe.in_errors.fetch_add(1, Ordering::Relaxed);
        record_stream_error(&probe, StreamDirection::Input, stream_error_kind(error));
        match error {
            DeviceError::Xrun => {
                probe.in_xruns.fetch_add(1, Ordering::Relaxed);
            }
            DeviceError::ClockDiscontinuity => {
                probe.in_discontinuities.fetch_add(1, Ordering::Relaxed);
            }
            DeviceError::StreamInvalidated => {
                probe.in_invalidated.fetch_add(1, Ordering::Relaxed);
            }
            DeviceError::Backend(_) => {
                probe.in_backend_errors.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    })
}

fn output_error_callback(probe: &Arc<EndpointProbe>) -> OutputErrorCallback {
    let probe = Arc::clone(probe);
    Box::new(move |error: DeviceError| {
        probe.out_errors.fetch_add(1, Ordering::Relaxed);
        record_stream_error(&probe, StreamDirection::Output, stream_error_kind(error));
        match error {
            DeviceError::Xrun => {
                probe.out_xruns.fetch_add(1, Ordering::Relaxed);
            }
            DeviceError::StreamInvalidated => {
                probe.out_invalidated.fetch_add(1, Ordering::Relaxed);
            }
            DeviceError::Backend(_) => {
                probe.out_backend_errors.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    })
}

/// Effective regression count: raw regressions minus those inside
/// the per-epoch startup grace window. `boundary_cb` splits the run
/// into epoch 1 (indices below it) and epoch 2 (at/above it);
/// `u64::MAX` means no restart boundary was recorded. Only the first
/// [`REGRESSION_INDEX_SLOTS`] regressions have a recorded index —
/// anything past that cannot be classified and stays fatal.
fn effective_regressions(raw: u64, slots: &[AtomicU64], boundary_cb: u64) -> u64 {
    let mut transients = 0_u64;
    for slot in slots
        .iter()
        .take(usize::try_from(raw).unwrap_or(usize::MAX))
    {
        let cb = slot.load(Ordering::Relaxed);
        if is_epoch_start_grace(cb, boundary_cb) {
            transients += 1;
        }
    }
    raw - transients
}

/// Control-thread contract + oracle analysis — returns the failure
/// list (empty = pass). Contract violations are hard gates; error
/// counters are recorded in the evidence text, not gated (AC5
/// territory, same as the device verifier).
fn analyze_endpoint(probe: &EndpointProbe, in_format: &AudioFormat) -> Vec<String> {
    let mut failures = Vec::new();
    let in_cb = probe.in_callbacks.load(Ordering::Relaxed);
    let out_cb = probe.out_callbacks.load(Ordering::Relaxed);
    if in_cb == 0 || out_cb == 0 {
        failures.push(format!(
            "open+start but zero data callbacks (in={in_cb} out={out_cb})"
        ));
        return failures;
    }
    if probe.in_frames.load(Ordering::Relaxed) == 0 {
        failures.push("input stream delivered zero frames".to_owned());
    }
    for (label, count) in [
        (
            "input channel mismatches",
            probe.in_channel_mismatches.load(Ordering::Relaxed),
        ),
        (
            "output channel mismatches",
            probe.out_channel_mismatches.load(Ordering::Relaxed),
        ),
        (
            "input timestamp regressions",
            probe.in_timestamp.effective_regressions(),
        ),
        (
            "output timestamp regressions",
            probe.out_timestamp.effective_regressions(),
        ),
        (
            "input clock-domain mismatches",
            probe.in_domain_mismatches.load(Ordering::Relaxed),
        ),
        (
            "non-finite input samples",
            probe.in_non_finite.load(Ordering::Relaxed),
        ),
        (
            "output fixture fill errors",
            probe.out_fill_errors.load(Ordering::Relaxed),
        ),
    ] {
        if count > 0 {
            failures.push(format!("{label}: {count}"));
        }
    }

    let rate = in_format.sample_rate.get();
    let samples = capture_samples(probe);
    if samples.len() < usize::try_from(u64::from(rate) / 2).unwrap_or(usize::MAX) {
        failures.push(format!(
            "insufficient captured audio for oracle: {} frames at {rate} Hz",
            samples.len()
        ));
        return failures;
    }
    failures.extend(oracle_markers(&samples, rate));
    failures.extend(oracle_level(&samples));

    log_endpoint_stats(probe, samples.len(), in_cb, out_cb);
    failures
}

fn regression_index_log(slots: &[AtomicU64], raw: u64) -> String {
    slots
        .iter()
        .take(usize::try_from(raw).unwrap_or(usize::MAX))
        .map(|slot| slot.load(Ordering::Relaxed).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn log_endpoint_stats(probe: &EndpointProbe, capture_stored: usize, in_cb: u64, out_cb: u64) {
    let in_raw_regressions = probe.in_timestamp.raw_regressions();
    let out_raw_regressions = probe.out_timestamp.raw_regressions();
    let in_boundary_cb = probe.in_timestamp.epoch_boundary_cb.load(Ordering::Relaxed);
    let out_boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    eprintln!(
        "  stats: in_cb={in_cb} out_cb={out_cb} in_frames={} out_frames={} \
         in_err={} out_err={} capture_stored={} overflow={} lock_failures={}",
        probe.in_frames.load(Ordering::Relaxed),
        probe.out_frames.load(Ordering::Relaxed),
        probe.in_errors.load(Ordering::Relaxed),
        probe.out_errors.load(Ordering::Relaxed),
        capture_stored,
        capture_overflow(probe),
        probe.capture_lock_failures.load(Ordering::Relaxed),
    );
    eprintln!(
        "  error kinds: in xrun={} discontinuity={} invalidated={} backend={} | out xrun={} invalidated={} backend={}",
        probe.in_xruns.load(Ordering::Relaxed),
        probe.in_discontinuities.load(Ordering::Relaxed),
        probe.in_invalidated.load(Ordering::Relaxed),
        probe.in_backend_errors.load(Ordering::Relaxed),
        probe.out_xruns.load(Ordering::Relaxed),
        probe.out_invalidated.load(Ordering::Relaxed),
        probe.out_backend_errors.load(Ordering::Relaxed),
    );
    eprintln!(
        "  timestamps: in regressions={} effective={} epoch_resets={} boundary_cb={} first..last={}..{} max_backstep_ns={} idx=[{}] | out regressions={} effective={} epoch_resets={} boundary_cb={} first..last={}..{} max_backstep_ns={} idx=[{}]",
        in_raw_regressions,
        probe.in_timestamp.effective_regressions(),
        probe.in_timestamp.epoch_resets.load(Ordering::Relaxed),
        in_boundary_cb,
        probe
            .in_timestamp
            .regression_first_cb
            .load(Ordering::Relaxed),
        probe
            .in_timestamp
            .regression_last_cb
            .load(Ordering::Relaxed),
        probe
            .in_timestamp
            .regression_max_backstep_ns
            .load(Ordering::Relaxed),
        probe.in_timestamp.regression_index_log(),
        out_raw_regressions,
        probe.out_timestamp.effective_regressions(),
        probe
            .out_timestamp
            .classifier
            .epoch_resets
            .load(Ordering::Relaxed),
        out_boundary_cb,
        probe
            .out_timestamp
            .classifier
            .regression_first_cb
            .load(Ordering::Relaxed),
        probe
            .out_timestamp
            .classifier
            .regression_last_cb
            .load(Ordering::Relaxed),
        probe
            .out_timestamp
            .classifier
            .regression_max_backstep_ns
            .load(Ordering::Relaxed),
        probe.out_timestamp.regression_index_log(),
    );
}

fn capture_samples(probe: &EndpointProbe) -> Vec<f32> {
    let guard = probe
        .capture
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.data[..guard.len].to_vec()
}

fn capture_overflow(probe: &EndpointProbe) -> u64 {
    probe
        .capture
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .overflow_frames
}

/// Marker-spacing oracle: `|x| > MARKER_THRESHOLD` peaks with a
/// refractory window; requires >= `MARKER_MIN_COUNT` markers whose
/// mean interval matches 100 ms within tolerance. Independent of the
/// adapter — it only knows the supplied stimulus spec.
#[expect(clippy::cast_precision_loss)]
fn oracle_markers(samples: &[f32], rate_hz: u32) -> Vec<String> {
    let refractory = (u64::from(rate_hz) * MARKER_REFRACTORY_MS / 1_000).max(1);
    let mut markers: Vec<u64> = Vec::new();
    let mut next_allowed = 0_u64;
    for (index, &sample) in samples.iter().enumerate() {
        let frame = index as u64;
        if sample.abs() > MARKER_THRESHOLD && frame >= next_allowed {
            markers.push(frame);
            next_allowed = frame + refractory;
        }
    }
    if markers.len() < MARKER_MIN_COUNT {
        return vec![format!(
            "marker oracle: only {} markers detected (need {})",
            markers.len(),
            MARKER_MIN_COUNT
        )];
    }
    let intervals: Vec<f64> = markers
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) as f64 / f64::from(rate_hz))
        .collect();
    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    // 100 ms marker period — the stimulus spec, rate-independent.
    let expected = 0.1_f64;
    if (mean - expected).abs() > expected * MARKER_INTERVAL_TOLERANCE {
        return vec![format!(
            "marker oracle: mean interval {mean:.4}s deviates from {expected}s"
        )];
    }
    Vec::new()
}

/// Level oracle: observed RMS vs the expected RMS of the supplied
/// impulse train. The stimulus is one `IMPULSE_AMPLITUDE` sample per
/// `IMPULSE_PERIOD_MS` of render time; the cable's SRC preserves
/// energy, so expected mean-square is `amplitude² / period_frames`.
/// The render rate is not known on the capture side — the reference
/// below assumes 48 kHz (period = 4800 frames → rms ≈ 0.0115); rates
/// in the 44.1–96 kHz range move it by less than √2, well inside the
/// provisional band (same discipline as the device verifier).
fn oracle_level(samples: &[f32]) -> Vec<String> {
    let observed = rms(samples);
    let expected = f64::from(IMPULSE_AMPLITUDE) / 4_800.0_f64.sqrt();
    let ratio = if expected > 0.0 {
        observed / expected
    } else {
        0.0
    };
    eprintln!(
        "  oracle level: observed_rms={observed:.5} expected_rms≈{expected:.5} ratio={ratio:.3}"
    );
    if !(MIN_LEVEL_RATIO..=MAX_LEVEL_RATIO).contains(&ratio) {
        return vec![format!(
            "level oracle: rms ratio {ratio:.3} outside [{MIN_LEVEL_RATIO}, {MAX_LEVEL_RATIO}]"
        )];
    }
    Vec::new()
}

#[expect(clippy::cast_precision_loss)]
fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|&sample| f64::from(sample) * f64::from(sample))
        .sum();
    (sum_sq / samples.len() as f64).sqrt()
}

#[test]
fn backend_trace_session_and_file_names_are_unique() {
    let pid = process::id().to_string();
    let session0 = backend_trace_session_name(0);
    let session1 = backend_trace_session_name(1);
    assert_ne!(session0, session1);
    assert!(session0.contains(&pid));
    assert!(session0.starts_with("rvf1034_bt_"));
    let base0 = backend_trace_base_name(0);
    let base1 = backend_trace_base_name(1);
    assert_ne!(base0, base1);
    assert!(base0.starts_with("backend-os-trace-pair0-"));
    assert!(base0.contains(&pid));
}

#[test]
fn backend_trace_artifact_dir_failure_is_honest() {
    let missing = Path::new("definitely-missing-rvf1034-backend-trace-workspace");
    let error = discover_backend_trace_artifact_dir(Some(missing), None).unwrap_err();
    assert!(error.contains("no artifact directory"));
}

fn portcls_binary_payload_hex(
    trace_event_type: u64,
    data1: u64,
    data2: u64,
    data3: u64,
    data4: u64,
) -> String {
    let mut bytes = Vec::new();
    for value in [trace_event_type, data1, data2, data3, data4, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "{byte:02X}");
            out
        })
}

fn portcls_binary_event_chunk(binary_hex: &str) -> String {
    format!(
        r#"<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
<System>
<Provider Name="PortCls-AudioEngineWmiTraceControl" Guid="{{10EB6007-818C-4DB6-A694-B518E589D07A}}" />
<EventID>0</EventID>
<Version>0</Version>
<Level>5</Level>
<Task>0</Task>
<Opcode>96</Opcode>
<Keywords>0x0</Keywords>
<TimeCreated SystemTime="2026-11-27T00:00:00.000000000+00:00" />
</System>
<BinaryEventData>{binary_hex}</BinaryEventData>
<ExtendedTracingInfo xmlns="http://schemas.microsoft.com/win/2004/08/events/trace">
<EventGuid>{{97BCD203-6286-4974-801D-A293B921F84B}}</EventGuid>
</ExtendedTracingInfo>
</Event>
"#
    )
}

#[test]
fn backend_trace_provider_matrix_separates_control_and_event_guids() {
    let enabled = BACKEND_TRACE_PROVIDERS
        .iter()
        .filter(|provider| provider.enabled_in_session)
        .collect::<Vec<_>>();
    assert_eq!(enabled.len(), 8);
    assert!(
        enabled
            .iter()
            .all(|provider| provider.keyword_mask == BACKEND_TRACE_KEYWORD_MASK_ALL)
    );
    assert!(
        enabled
            .iter()
            .any(|provider| provider.guid == PORTCLS_AUDIOENGINE_CONTROL_GUID)
    );
    assert!(
        enabled
            .iter()
            .any(|provider| provider.guid == PORTCLS_AUDIOENGINE_EVENT_GUID)
    );
    assert!(
        enabled
            .iter()
            .any(|provider| provider.guid == DRIVER_WMI_PROBE_CONTROL_GUID)
    );
    let event_class = BACKEND_TRACE_PROVIDERS
        .iter()
        .find(|provider| provider.guid == PORTCLS_AUDIOENGINE_EVENT_GUID)
        .expect("PortCls_AudioEngine event class matrix entry");
    assert_eq!(event_class.role, BackendTraceProviderRole::WmiEventClass);
    let control = BACKEND_TRACE_PROVIDERS
        .iter()
        .find(|provider| provider.guid == PORTCLS_AUDIOENGINE_CONTROL_GUID)
        .expect("PortCls WMI trace control matrix entry");
    assert_eq!(control.role, BackendTraceProviderRole::ClassicOrWmiControl);

    let provider_file = backend_trace_provider_file_text();
    assert_eq!(provider_file.lines().count(), 8);
    assert!(provider_file.contains(PORTCLS_AUDIOENGINE_CONTROL_GUID));
    assert!(provider_file.contains(PORTCLS_AUDIOENGINE_EVENT_GUID));
    assert!(
        provider_file
            .lines()
            .all(|line| line.contains(BACKEND_TRACE_KEYWORD_MASK_ALL))
    );
}

#[test]
fn backend_trace_etw_field_packing_and_binary_decode_round_trip() {
    let frequency = 10_000_000_u64;
    let data1 = rvf_position_diagnostic_data1(RVF_POSITION_ETW_PRESENTATION_POSITION_V1, frequency);
    assert_eq!(
        rvf_position_diagnostic_data1_decode(data1),
        (RVF_POSITION_ETW_PRESENTATION_POSITION_V1, frequency)
    );
    let pin_facts = rvf_position_pin_facts(7, 3, true);
    assert_eq!(rvf_position_pin_facts_decode(pin_facts), (7, 3, true));
    let render_facts = rvf_position_pin_facts(7, 3, false);
    assert_eq!(rvf_position_pin_facts_decode(render_facts), (7, 3, false));

    let raw_qpc = 987_654_321_u64;
    let payload = portcls_binary_payload_hex(96, data1, pin_facts, 12_345, raw_qpc);
    let chunk = portcls_binary_event_chunk(&payload);
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&chunk, &mut inventory);
    assert_eq!(inventory.portcls_provider_events, 1);
    assert_eq!(inventory.miniport_events_total, 1);
    assert_eq!(inventory.rvf_position_events, 1);
    assert_eq!(inventory.undecoded_portcls_binary_events, 0);
    assert_eq!(inventory.invalid_rvf_position_events, 0);
    let event = inventory
        .rvf_driver_position_events
        .first()
        .expect("decoded rf driver position event");
    assert_eq!(event.schema_id, RVF_POSITION_ETW_PRESENTATION_POSITION_V1);
    assert_eq!(event.frequency_hz, frequency);
    assert!(event.capture);
    assert_eq!((event.device_type, event.pin), (7, 3));
    assert_eq!(event.presentation_position_blocks, 12_345);
    assert_eq!(event.raw_qpc, raw_qpc);
    assert_eq!(event.raw_qpc_ns, Some(98_765_432_100));
}

#[test]
fn backend_trace_standard_miniport_binary_decode_is_distinct() {
    let payload = portcls_binary_payload_hex(101, 111, 222, 12_345, 0);
    let chunk = portcls_binary_event_chunk(&payload);
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&chunk, &mut inventory);
    assert_eq!(inventory.miniport_events_total, 1);
    assert_eq!(inventory.standard_presentation_position_events, 1);
    assert_eq!(inventory.rvf_position_events, 0);
    let event = inventory
        .miniport_events
        .first()
        .expect("standard miniport event");
    assert_eq!(event.event_kind, "eMINIPORT_GET_PRESENTATION_POSITION");
    assert_eq!(
        (event.data1, event.data2, event.data3, event.data4),
        (111, 222, 12_345, 0)
    );
}

#[test]
fn backend_trace_rvf_decode_rejects_token_scan_false_positives() {
    let frequency = 10_000_000_u64;
    let valid_data1 =
        rvf_position_diagnostic_data1(RVF_POSITION_ETW_PRESENTATION_POSITION_V1, frequency);
    let mut inventory = BackendTraceXmlInventory::default();
    let token_only = format!(
        "<Event xmlns=\"x\"><System><Provider Name=\"PortCls-AudioEngineWmiTraceControl\" Guid=\"{PORTCLS_AUDIOENGINE_CONTROL_GUID}\" /></System><EventData><Data Name=\"token\">{valid_data1}</Data></EventData><ExtendedTracingInfo><EventGuid>{PORTCLS_AUDIOENGINE_EVENT_GUID}</EventGuid></ExtendedTracingInfo></Event>"
    );
    analyze_backend_trace_event_chunk(&token_only, &mut inventory);
    assert_eq!(inventory.rvf_position_events, 0);
    assert_eq!(inventory.portcls_events_missing_binary_payload, 1);

    let wrong_schema = rvf_position_diagnostic_data1(0x2002, frequency);
    let payload = portcls_binary_payload_hex(96, wrong_schema, 0, 0, 0);
    let chunk = portcls_binary_event_chunk(&payload);
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&chunk, &mut inventory);
    assert_eq!(inventory.miniport_events_total, 1);
    assert_eq!(inventory.rvf_position_events, 0);
    assert_eq!(inventory.invalid_rvf_position_events, 1);

    let implausible_frequency =
        rvf_position_diagnostic_data1(RVF_POSITION_ETW_PRESENTATION_POSITION_V1, 1);
    let payload = portcls_binary_payload_hex(96, implausible_frequency, 0, 0, 0);
    let chunk = portcls_binary_event_chunk(&payload);
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&chunk, &mut inventory);
    assert_eq!(inventory.rvf_position_events, 0);
    assert_eq!(inventory.invalid_rvf_position_events, 1);
}

#[test]
fn backend_trace_diagnostic_lines_are_bounded_and_not_oracle_lines() {
    let mut control = BackendOsTraceControl::new(0, Instant::now());
    control.finished = true;
    control.notes.push("x".repeat(1_000));
    let lines = control.diagnostic_lines();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("roundtrip_backend_os_trace: "));
    assert!(lines[1].starts_with("roundtrip_portcls_event_split: "));
    assert!(lines[2].starts_with("roundtrip_driver_wmi_control_probe: "));
    assert!(lines[3].starts_with("roundtrip_backend_os_trace_calibration: "));
    assert!(lines.iter().all(|line| line.len() < 2_048));
    assert!(
        lines
            .iter()
            .all(|line| !line.starts_with("roundtrip_output_timestamp_"))
    );
}

#[test]
fn backend_trace_unsupported_status_does_not_change_timestamp_classifier() {
    let classifier = TimestampEpochClassifier::new();
    let epoch_resets_before = classifier.epoch_resets.load(Ordering::Relaxed);
    let regressions_before = classifier.raw_regressions();
    let mut control = BackendOsTraceControl::new(0, Instant::now());
    control.status = "unsupported".to_owned();
    control.finished = true;
    assert_eq!(control.status, "unsupported");
    assert_eq!(
        classifier.epoch_resets.load(Ordering::Relaxed),
        epoch_resets_before
    );
    assert_eq!(classifier.raw_regressions(), regressions_before);
}

fn kernel_eventtracing_enable_chunk(session_name: &str, provider_guid: &str) -> String {
    format!(
        r#"<Event><System><Provider Name="Microsoft-Windows-Kernel-EventTracing" Guid="{MICROSOFT_WINDOWS_KERNEL_EVENTTRACING_GUID}" /><EventID>14</EventID><Opcode>18</Opcode></System><EventData><Data Name="ProviderName">{provider_guid}</Data><Data Name="SessionName">{session_name}</Data></EventData><RenderingInfo><Message>Provider enabled</Message></RenderingInfo></Event>"#
    )
}

#[test]
fn kernel_eventtracing_witness_requires_session_and_guid_fields() {
    let session_name = "rvf1034_bt_123_0";
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk_for_session(
        &kernel_eventtracing_enable_chunk(session_name, PORTCLS_AUDIOENGINE_CONTROL_GUID),
        &mut inventory,
        session_name,
    );
    analyze_backend_trace_event_chunk_for_session(
        &kernel_eventtracing_enable_chunk("other-session", PORTCLS_AUDIOENGINE_EVENT_GUID),
        &mut inventory,
        session_name,
    );
    assert_eq!(inventory.kernel_eventtracing_events, 2);
    assert_eq!(inventory.kernel_eventtracing_enable_events, 2);
    assert_eq!(inventory.kernel_eventtracing_enable_opcode18_events, 2);
    assert_eq!(inventory.kernel_eventtracing_raw_message_available, 2);
    assert_eq!(inventory.kernel_witness_matched_session_enable_events, 1);
    assert_eq!(inventory.kernel_witness_portcls_control_enable_events, 1);
    assert_eq!(inventory.kernel_witness_portcls_event_enable_events, 0);
    assert_eq!(inventory.portcls_provider_events, 0);
    assert_eq!(
        control_witness_status(&inventory),
        "control-request-witnessed"
    );

    let absent = BackendTraceXmlInventory {
        kernel_eventtracing_events: 1,
        ..BackendTraceXmlInventory::default()
    };
    assert_eq!(
        control_witness_status(&absent),
        "control-request-witness-absent"
    );
}

fn classic_selftest_event_chunk(payload_hex: &str) -> String {
    format!(
        r#"<Event><System><Provider Name="Rvf1034-ClassicSelftest-Control" Guid="{CLASSIC_SELFTEST_PROVIDER_GUID}" /><EventID>1</EventID></System><BinaryEventData>{payload_hex}</BinaryEventData><ExtendedTracingInfo><EventGuid>{CLASSIC_SELFTEST_EVENT_CLASS_GUID}</EventGuid></ExtendedTracingInfo></Event>"#
    )
}

#[test]
fn classic_selftest_identity_and_payload_decode_are_strict() {
    let expected = classic_selftest_payload_hex();
    assert_eq!(expected.len(), 64);
    assert!(expected.starts_with("52564631303334"));
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&classic_selftest_event_chunk(&expected), &mut inventory);
    assert_eq!(inventory.classic_selftest_events, 1);
    assert_eq!(inventory.classic_selftest_identity_match_events, 1);
    assert_eq!(inventory.classic_selftest_payload_match_events, 1);

    let mut false_positive = BackendTraceXmlInventory::default();
    let token_only = format!(
        "<Event><System><Provider Guid=\"{CLASSIC_SELFTEST_PROVIDER_GUID}\" /></System><EventData><Data Name=\"token\">{expected}</Data></EventData></Event>"
    );
    analyze_backend_trace_event_chunk(&token_only, &mut false_positive);
    assert_eq!(false_positive.classic_selftest_events, 1);
    assert_eq!(false_positive.classic_selftest_payload_match_events, 0);
    assert_eq!(false_positive.classic_selftest_missing_payload_events, 1);

    let mut mismatch = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&classic_selftest_event_chunk("0000"), &mut mismatch);
    assert_eq!(mismatch.classic_selftest_payload_mismatch_events, 1);
}

#[test]
fn classic_selftest_status_requires_successful_register_enable_and_emit() {
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(
        &classic_selftest_event_chunk(&classic_selftest_payload_hex()),
        &mut inventory,
    );
    let mut observation = ClassicSelftestObservation {
        flags: CLASSIC_SELFTEST_IMPLEMENTED
            | CLASSIC_SELFTEST_REGISTRATION_ATTEMPTED
            | CLASSIC_SELFTEST_REGISTRATION_HANDLE_PRESENT
            | CLASSIC_SELFTEST_ENABLED
            | CLASSIC_SELFTEST_SESSION_HANDLE_PRESENT
            | CLASSIC_SELFTEST_EMIT_ATTEMPTED,
        registration_status: Some(0),
        emit_api_status: Some(0),
        emit_count: 1,
        ..ClassicSelftestObservation::default()
    };
    assert_eq!(
        classic_selftest_status(&observation, &inventory),
        "classic-selftest-decoded"
    );
    inventory.classic_selftest_payload_match_events = 0;
    inventory.classic_selftest_identity_match_events = 0;
    assert_eq!(
        classic_selftest_status(&observation, &inventory),
        "classic-selftest-missing"
    );
    observation.emit_api_status = Some(87);
    assert_eq!(
        classic_selftest_status(&observation, &inventory),
        "classic-selftest-unsupported"
    );
}

fn driver_wmi_probe_payload_hex(kind: u32, ordinal: u32, schema: u32) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RVF1034W");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0x1f_u32.to_le_bytes());
    bytes.extend_from_slice(&ordinal.to_le_bytes());
    bytes.extend_from_slice(&schema.to_le_bytes());
    bytes.extend_from_slice(&11_u64.to_le_bytes());
    bytes.extend_from_slice(&22_u64.to_le_bytes());
    bytes.extend_from_slice(&33_u64.to_le_bytes());
    bytes.extend_from_slice(&44_u64.to_le_bytes());
    bytes.extend_from_slice(&50_u64.to_le_bytes());
    bytes.extend_from_slice(&2_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&6_u64.to_le_bytes());
    bytes.extend_from_slice(&5_u64.to_le_bytes());
    bytes.extend_from_slice(&2_u64.to_le_bytes());
    assert_eq!(bytes.len(), 120);
    hex_encode(&bytes)
}

fn driver_wmi_probe_payload_hex_with_counters(
    kind: u32,
    write_calls: u64,
    mirror_key_calls: u64,
    mirror_key_emit_calls: u64,
) -> String {
    let mut bytes = decode_hex_bytes(&driver_wmi_probe_payload_hex(kind, 0xffff_ffff, 0))
        .expect("base driver WMI probe payload");
    bytes[72..80].copy_from_slice(&write_calls.to_le_bytes());
    bytes[104..112].copy_from_slice(&mirror_key_calls.to_le_bytes());
    bytes[112..120].copy_from_slice(&mirror_key_emit_calls.to_le_bytes());
    hex_encode(&bytes)
}

fn driver_wmi_probe_event_chunk(payload_hex: &str) -> String {
    format!(
        r#"<Event><System><Provider /><EventID>0</EventID><TimeCreated SystemTime="2026-01-01T00:00:00.000000000+00:00" /></System><BinaryEventData>{payload_hex}</BinaryEventData><ExtendedTracingInfo><EventGuid>{DRIVER_WMI_PROBE_EVENT_GUID}</EventGuid></ExtendedTracingInfo></Event>"#
    )
}

#[test]
fn driver_wmi_probe_binary_decode_is_strict_and_counts_kinds() {
    let mirror_payload = driver_wmi_probe_payload_hex(3, 5, 0x2001);
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(
        &driver_wmi_probe_event_chunk(&mirror_payload),
        &mut inventory,
    );
    assert_eq!(inventory.driver_wmi_probe_events, 1);
    assert_eq!(inventory.driver_wmi_probe_identity_match_events, 1);
    assert_eq!(inventory.driver_wmi_probe_payload_match_events, 1);
    assert_eq!(inventory.driver_wmi_probe_write_mirror_events, 1);
    assert_eq!(inventory.driver_wmi_probe_mirror_ordinal_counts[5], 1);
    let event = inventory
        .driver_wmi_probe_stored_events
        .first()
        .expect("stored driver WMI probe event");
    assert_eq!(event.sequence, 7);
    assert_eq!(event.miniport_event_ordinal, 5);
    assert_eq!(event.miniport_schema_id, 0x2001);
    assert_eq!(
        (event.data1, event.data2, event.data3, event.data4),
        (11, 22, 33, 44)
    );
    assert_eq!(event.write_etw_event_calls, 50);
    assert_eq!(event.helper_null_calls, 2);
    assert_eq!(event.forward_error_calls, 1);
    assert_eq!(event.mirror_suppressed_calls, 6);
    assert_eq!(
        (event.mirror_key_calls, event.mirror_key_emit_calls),
        (5, 2)
    );

    let control_payload = driver_wmi_probe_payload_hex(1, 0xffff_ffff, 0);
    analyze_backend_trace_event_chunk(
        &driver_wmi_probe_event_chunk(&control_payload),
        &mut inventory,
    );
    assert_eq!(inventory.driver_wmi_probe_control_enable_events, 1);
    assert_eq!(
        driver_wmi_probe_status(&inventory),
        "driver-wmi-control-and-callsite-decoded"
    );

    let mut mismatch = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk(&driver_wmi_probe_event_chunk("0000"), &mut mismatch);
    assert_eq!(mismatch.driver_wmi_probe_payload_mismatch_events, 1);
    assert_eq!(mismatch.driver_wmi_probe_payload_match_events, 0);

    let mut missing = BackendTraceXmlInventory::default();
    let token_only = format!(
        "<Event><System><Provider Guid=\"{DRIVER_WMI_PROBE_CONTROL_GUID}\" /></System><EventData><Data Name=\"token\">{mirror_payload}</Data></EventData></Event>"
    );
    analyze_backend_trace_event_chunk(&token_only, &mut missing);
    assert_eq!(missing.driver_wmi_probe_events, 1);
    assert_eq!(missing.driver_wmi_probe_missing_payload_events, 1);
    assert_eq!(missing.driver_wmi_probe_payload_match_events, 0);
}

#[test]
fn driver_wmi_probe_control_window_counters_classify_callsite_without_mirror() {
    let mut inventory = BackendTraceXmlInventory::default();
    for payload in [
        driver_wmi_probe_payload_hex_with_counters(1, 100, 0, 0),
        driver_wmi_probe_payload_hex_with_counters(2, 113, 11, 4),
    ] {
        analyze_backend_trace_event_chunk(&driver_wmi_probe_event_chunk(&payload), &mut inventory);
    }
    assert_eq!(inventory.driver_wmi_probe_write_mirror_events, 0);
    assert_eq!(
        driver_wmi_probe_control_window_write_delta(&inventory),
        Some(13)
    );
    assert!(driver_wmi_probe_callsite_evidence_present(&inventory));
    assert_eq!(
        driver_wmi_probe_status(&inventory),
        "driver-wmi-control-and-callsite-decoded"
    );
    let window = driver_wmi_probe_control_window_json(&inventory);
    assert_eq!(window["write_etw_event_calls"]["delta"], 13);
    assert_eq!(window["mirror_key_emit_calls"]["delta"], 4);

    let mut unchanged = BackendTraceXmlInventory::default();
    for payload in [
        driver_wmi_probe_payload_hex_with_counters(1, 100, 0, 0),
        driver_wmi_probe_payload_hex_with_counters(2, 100, 0, 0),
    ] {
        analyze_backend_trace_event_chunk(&driver_wmi_probe_event_chunk(&payload), &mut unchanged);
    }
    assert!(!driver_wmi_probe_callsite_evidence_present(&unchanged));
    assert_eq!(
        driver_wmi_probe_status(&unchanged),
        "driver-wmi-control-decoded-callsite-absent"
    );
}

#[test]
fn driver_wmi_probe_kernel_witness_is_classified_separately() {
    let session_name = "rvf1034_bt_123_0";
    let mut inventory = BackendTraceXmlInventory::default();
    analyze_backend_trace_event_chunk_for_session(
        &kernel_eventtracing_enable_chunk(session_name, DRIVER_WMI_PROBE_CONTROL_GUID),
        &mut inventory,
        session_name,
    );
    assert_eq!(inventory.kernel_witness_driver_probe_enable_events, 1);
    assert_eq!(inventory.kernel_witness_portcls_control_enable_events, 0);
    assert_eq!(inventory.kernel_witness_selftest_enable_events, 0);
    assert_eq!(
        driver_wmi_probe_status(&inventory),
        "driver-wmi-control-request-witnessed-callback-absent"
    );
    assert_eq!(
        control_witness_status(&inventory),
        "control-request-witness-absent"
    );
}

#[test]
fn miniport_lifecycle_decode_counts_xrun_and_standard_ordinals() {
    let mut inventory = BackendTraceXmlInventory::default();
    for payload in [
        portcls_binary_payload_hex(96, RVF_XRUN_ETW_SCENARIO_SNAPSHOT, 0, 0, 0),
        portcls_binary_payload_hex(96, RVF_XRUN_ETW_ZERO_RANGE, 0, 0, 0),
        portcls_binary_payload_hex(97, 0, 0, 0, 0),
        portcls_binary_payload_hex(98, 0, 0, 0, 0),
        portcls_binary_payload_hex(100, 0, 0, 0, 0),
    ] {
        let chunk = portcls_binary_event_chunk(&payload);
        analyze_backend_trace_event_chunk(&chunk, &mut inventory);
    }
    assert_eq!(inventory.rvf_xrun_schema_1001_events, 1);
    assert_eq!(inventory.rvf_xrun_schema_1002_events, 1);
    assert_eq!(inventory.miniport_ordinal_counts[1], 1);
    assert_eq!(inventory.miniport_ordinal_counts[2], 1);
    assert_eq!(inventory.miniport_ordinal_counts[4], 1);
    assert_eq!(inventory.invalid_rvf_position_events, 0);
    assert_eq!(miniport_lifecycle_marker_count(&inventory), 5);
    assert_eq!(
        miniport_lifecycle_status(&inventory, true),
        "miniport-lifecycle-decoded"
    );

    let empty = BackendTraceXmlInventory::default();
    assert_eq!(
        miniport_lifecycle_status(&empty, true),
        "miniport-lifecycle-absent-after-stream-success"
    );
    assert_eq!(
        miniport_lifecycle_status(&empty, false),
        "miniport-lifecycle-inconclusive"
    );
}

#[test]
fn event_split_verdict_does_not_turn_zero_events_into_pass() {
    let inventory = BackendTraceXmlInventory::default();
    let observation = ClassicSelftestObservation::not_attempted("unit-test-notrun");
    let statuses = event_split_statuses(&inventory, &observation, true);
    assert_eq!(statuses.verdict, "inconclusive-blocked");

    let lifecycle_inventory = BackendTraceXmlInventory {
        rvf_xrun_schema_1002_events: 1,
        events_total: 1,
        ..BackendTraceXmlInventory::default()
    };
    let statuses = event_split_statuses(&lifecycle_inventory, &observation, true);
    assert_eq!(
        statuses.verdict,
        "miniport-lifecycle-positive-position-fire-miss"
    );

    let position_inventory = BackendTraceXmlInventory {
        rvf_position_events: 1,
        events_total: 1,
        ..BackendTraceXmlInventory::default()
    };
    let statuses = event_split_statuses(&position_inventory, &observation, true);
    assert_eq!(statuses.verdict, "driver-position-path-positive");
}
