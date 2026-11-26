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

use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
        instrument.observe(index, callback_ns, playback_ns, 480);
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
                RenderTimestampEvent {
                    callback_index: self.callback_index[index].load(Ordering::Relaxed),
                    epoch: self.epoch[index].load(Ordering::Relaxed),
                    frames: self.frames[index].load(Ordering::Relaxed),
                    callback_ns: self.callback_ns[index].load(Ordering::Relaxed),
                    playback_ns: playback_present.then_some(playback_ns),
                    interval_ns: (interval_ns != i64::MIN).then_some(interval_ns),
                    regression_backstep_ns: (backstep_ns != i64::MAX).then_some(backstep_ns),
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
}

impl RenderTimestampInstrument {
    fn new() -> Self {
        Self {
            classifier: TimestampEpochClassifier::new(),
            log: RenderTimestampLog::new(),
            playback_some: AtomicU64::new(0),
            playback_none: AtomicU64::new(0),
        }
    }

    fn observe(
        &self,
        callback_index: u64,
        callback_ns: i64,
        playback_ns: Option<i64>,
        frames: u64,
    ) {
        let observation = self.classifier.observe(callback_index, callback_ns);
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
        });
    }

    fn split_epoch(&self, boundary_cb: u64) {
        self.classifier.split_epoch(boundary_cb);
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
        "callbacks={} frames={} raw_regressions={} effective_regressions={} playback_some={} playback_none={} log_overflow={}",
        probe.out_callbacks.load(Ordering::Relaxed),
        probe.out_frames.load(Ordering::Relaxed),
        probe.out_timestamp.raw_regressions(),
        probe.out_timestamp.effective_regressions(),
        probe.out_timestamp.playback_some_count(),
        probe.out_timestamp.playback_none_count(),
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

fn emit_render_timestamp_events(probe: &EndpointProbe) {
    let boundary_cb = probe
        .out_timestamp
        .classifier
        .epoch_boundary_cb
        .load(Ordering::Relaxed);
    let events = probe.out_timestamp.events();
    for event in &events {
        eprintln!(
            "render_timestamp_event: index={} epoch={} epoch_boundary_cb={} is_epoch_boundary={} frames={} callback_ns={} playback_present={} playback_ns={} interval_ns={} regression_backstep_ns={}",
            event.callback_index,
            event.epoch,
            boundary_cb,
            u8::from(event.callback_index == boundary_cb),
            event.frames,
            event.callback_ns,
            u8::from(event.playback_ns.is_some()),
            optional_i64(event.playback_ns),
            optional_i64(event.interval_ns),
            optional_i64(event.regression_backstep_ns),
        );
    }
    eprintln!(
        "render_timestamp_log: events={} overflow={} bounded_slots={}",
        events.len(),
        probe.out_timestamp.log_overflows(),
        RENDER_TIMESTAMP_LOG_SLOTS,
    );
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
        match run_audio_round_trip(pair) {
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
fn run_audio_round_trip(pair: Pair) -> Result<(u32, u32), Vec<String>> {
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

    let in_stream = match open_input_with_fallback(capture.device.as_ref(), in_format, &probe) {
        Ok(stream) => stream,
        Err(error) => return Err(vec![format!("input open failed: {error}")]),
    };
    let out_stream =
        match open_output_with_fallback(render.device.as_ref(), out_format, &probe, fixture) {
            Ok(stream) => stream,
            Err(error) => return Err(vec![format!("output open failed: {error}")]),
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

    if let Err(error) = in_stream.start().and_then(|()| out_stream.start()) {
        return Err(vec![format!("stream start failed: {error}")]);
    }
    thread::sleep(Duration::from_secs(CAPTURE_SECONDS));
    let lifecycle = run_audio_lifecycle(&probe, &*in_stream, &*out_stream);
    drop(in_stream);
    drop(out_stream);

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
    for (label, result) in [
        ("input stop", in_stream.stop()),
        ("output stop", out_stream.stop()),
    ] {
        if let Err(error) = result {
            lifecycle.push(format!("{label} failed: {error}"));
        }
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
    probe
        .out_timestamp
        .split_epoch(probe.out_callbacks.load(Ordering::Relaxed));
    match in_stream.start().and_then(|()| out_stream.start()) {
        Ok(()) => {
            thread::sleep(Duration::from_millis(300));
            for (label, result) in [
                ("input re-stop", in_stream.stop()),
                ("output re-stop", out_stream.stop()),
            ] {
                if let Err(error) = result {
                    lifecycle.push(format!("{label} failed: {error}"));
                }
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
    let callback_ns = info.timestamp.callback.value();
    probe.out_timestamp.observe(
        callback_index,
        callback_ns,
        info.timestamp.playback.map(MediaTime::value),
        frames as u64,
    );
    let base = probe
        .out_render_cursor
        .fetch_add(frames as u64, Ordering::Relaxed);
    if fixture.fill_view(base, out).is_err() {
        probe.out_fill_errors.fetch_add(1, Ordering::Relaxed);
    }
}

fn input_error_callback(probe: &Arc<EndpointProbe>) -> InputErrorCallback {
    let probe = Arc::clone(probe);
    Box::new(move |error: DeviceError| {
        probe.in_errors.fetch_add(1, Ordering::Relaxed);
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
        let epoch_start = if cb < boundary_cb { 0 } else { boundary_cb };
        if cb.saturating_sub(epoch_start) < EPOCH_START_GRACE_CALLBACKS {
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
