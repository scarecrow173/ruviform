//! `test_fault` L1 end-to-end tests — this crate's own CLAP cdylib
//! serving the `test_fault` plugin out-of-process through
//! `ruviform-plugin-test-clap-l1-runner` (Issue #630 `test_fault`
//! slice; oracle `docs/test-specs/issue-630-test-fault.md`).
//! Structural mirror of `l1_fault_test_plugin_e2e.rs` in the VST3
//! fixture crate — the fault law is identical by design so both ABIs
//! present the same contract to the host.
//!
//! Every test is `#[ignore]`-gated: they require the fixture cdylib
//! and the `l1-runner`-feature runner binary, which only
//! `mise run test-l1-real-plugin` builds deterministically before
//! selecting `--ignored`. The file-level `#![cfg(feature = "l1-runner")]`
//! keeps the file empty — and `env!("CARGO_BIN_EXE_*")` undefined —
//! without the feature, so `cargo test` default runs never see these
//! tests. Missing artifacts are `assert!` failures, never silent
//! skips.
//!
//! ## Arming law under test
//!
//! The fault arms **in-band**: a `ParamValue` event on the `Fault`
//! parameter (id `0`, plain `[0.0, 4.0]`) inside a `process` block.
//! Normalized encodings used below (plain = normalized × 4.0):
//!
//! | mode | plain | normalized |
//! | ---- | ----- | ---------- |
//! | `none` | `0.0` | `0.0` |
//! | `abort` | `1.0` | `0.25` |
//! | `exit(42)` | `2.42` | `0.605` |
//! | `exit(0)` | `2.0` | `0.5` |
//! | `hang` | `3.0` | `0.75` |
//! | `error` | `4.0` | `1.0` |

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::{
    Event, EventKind, EventList, FailureCause, HostError, HostInput, HostOutput, LifecycleState,
    ParamEvent, ParamId, ParamValue, PluginDescriptor, ProcessConfig, ProcessResult, ProcessStatus,
};
use ruviform_plugin_host::{
    ChildExit, DiagnosticQuery, HostCommand, HostNotification, HostReply, OutOfProcessIsolation,
    OutOfProcessSpec, PluginHandle, PluginHost,
};

/// The `test_fault` descriptor the child synthesizes from a real
/// `clap_plugin_factory` scan — the `OutOfProcessSpec` oracle must
/// field-match it. The parent pins the fixture's known values as a
/// const while the child computes them from the actual binary — the
/// handshake is two independent sources agreeing
/// (`fault.rs` `TestFaultClapPlugin::get_descriptor`).
const EXPECTED_FAULT: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-fault",
    name: "test_fault",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["audio-effect", "stereo"],
};

/// The `test_gain` descriptor — for the dedicated-isolation pairing.
const EXPECTED_GAIN: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-gain",
    name: "test_gain",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["audio-effect", "stereo"],
};

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}`.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// How long tests poll for an asynchronous `Failed` notification —
/// 10 s is generous headroom for loaded CI hosts (the `l1_proxy_e2e`
/// precedent's bound).
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// Block shape for every test — 512 max frames, so the defined L1
/// latency is `lookahead_blocks` (2) × 512 = 1024 frames.
const FRAMES: usize = 512;
const FRAMES_U32: u32 = 512;

/// The runner executable — same package, so `CARGO_BIN_EXE_*` names
/// it directly (defined only under `--features l1-runner`, which this
/// file's `#![cfg]` guarantees). A missing binary is an `assert!`
/// failure, never a silent skip.
fn runner_program() -> OsString {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-clap-l1-runner"));
    assert!(
        path.is_file(),
        "L1 runner missing at {} — `mise run test-l1-real-plugin` builds it",
        path.display()
    );
    path.into_os_string()
}

/// The fixture cdylib the runner will dlopen — resolved the way the
/// runner resolves it with `RUVIFORM_L1_REAL_PLUGIN_DIR` unset: a
/// sibling of the runner executable.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-clap-l1-runner"))
        .parent()
        .expect("the runner lives under target/<profile>/")
        .join(fixture_lib_name())
}

/// Every test that spawns the real child asserts the fixture exists
/// first — a missing artifact is a failure, not a skip.
fn assert_fixture_present() {
    let path = fixture_path();
    assert!(
        path.is_file(),
        "fixture cdylib missing: {} — `mise run test-l1-real-plugin` builds it via \
         `cargo build --features l1-runner`",
        path.display()
    );
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap()
}

/// Bit-exact sample comparison — the shm path round-trips `f32` bit
/// patterns through `to_bits`/`from_bits`, so defined values compare
/// equal bit-for-bit.
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// One stereo block through `PluginHost::process` — `test_fault`
/// declares the same single stereo port pair as `test_gain`.
fn host_block(
    host: &mut PluginHost,
    handle: PluginHandle,
    input: &[&[f32]; 2],
    out_left: &mut [f32],
    out_right: &mut [f32],
    events: EventList<'_>,
) -> Result<ProcessResult, HostError> {
    let input_view = PlanarBufferView::new(&input[..]).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut *out_left, &mut *out_right];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    host.process(handle, &inputs, &mut outputs, events, None)
}

/// One `Fault` `ParamValue` event at frame offset 0 — `normalized`
/// is the wire value; the fixture's plain domain is `[0.0, 4.0]`
/// (`plain = normalized × 4.0`).
fn fault_event(normalized: f64) -> Event {
    Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(normalized).unwrap(),
        }),
    )
}

/// Drive one instance through `Processing`.
fn drive_to_processing(host: &mut PluginHost, handle: PluginHandle) {
    host.execute(HostCommand::Initialize { handle }).unwrap();
    host.execute(HostCommand::Activate {
        handle,
        config: config(),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
}

/// Polls the composite's `lifecycle_state` until `Failed`.
fn wait_host_failed(host: &PluginHost, handle: PluginHandle) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls `try_recv_notification` until the next `InstanceFailed`
/// notification arrives or `NOTIFY_TIMEOUT` elapses.
fn wait_host_notification(host: &mut PluginHost) -> (PluginHandle, ChildExit, FailureCause) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    loop {
        if let Some(HostNotification::InstanceFailed {
            handle,
            exit,
            cause,
            ..
        }) = host.try_recv_notification()
        {
            return (handle, exit, cause);
        }
        assert!(
            Instant::now() < deadline,
            "InstanceFailed notification never arrived"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// Submits `blocks` clean pass-through blocks, returning how many
/// produced non-silent (delivered) output — proves the child is
/// actually processing before a fault is armed.
fn drive_clean_blocks(host: &mut PluginHost, handle: PluginHandle, blocks: u32) -> u32 {
    let empty = EventList::empty(FRAMES_U32);
    let mut delivered = 0_u32;
    for _ in 0..blocks {
        let in_left = [0.5_f32; FRAMES];
        let in_right = [-0.25_f32; FRAMES];
        let input = [&in_left[..], &in_right[..]];
        let mut out_left = [0.0_f32; FRAMES];
        let mut out_right = [0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block(
                &mut *host,
                handle,
                &input,
                &mut out_left,
                &mut out_right,
                empty,
            ),
            Ok(ProcessResult::CONTINUE)
        );
        if !(out_left.iter().all(|&s| sample_eq(s, 0.0))
            && out_right.iter().all(|&s| sample_eq(s, 0.0)))
        {
            // Unarmed fixtures are pass-through: delivered output
            // equals the input bitwise.
            assert!(
                out_left.iter().all(|&s| sample_eq(s, 0.5))
                    && out_right.iter().all(|&s| sample_eq(s, -0.25)),
                "the fixture must pass audio through bitwise while unarmed"
            );
            delivered += 1;
        }
    }
    delivered
}

/// Submits one block carrying `events` (empty slice = clean block)
/// and returns the `ProcessResult` — asserts only that the submit
/// was accepted (`Ok`); the status is the caller's oracle (an armed
/// `error` block legitimately reports `ProcessStatus::Error`).
fn submit_block_result(
    host: &mut PluginHost,
    handle: PluginHandle,
    events: &[Event],
) -> ProcessResult {
    let in_left = [0.5_f32; FRAMES];
    let in_right = [-0.25_f32; FRAMES];
    let input = [&in_left[..], &in_right[..]];
    let mut out_left = [0.0_f32; FRAMES];
    let mut out_right = [0.0_f32; FRAMES];
    let list = if events.is_empty() {
        EventList::empty(FRAMES_U32)
    } else {
        EventList::new(events, FRAMES_U32).unwrap()
    };
    thread::sleep(Duration::from_millis(5));
    host_block(host, handle, &input, &mut out_left, &mut out_right, list)
        .expect("process submit must be accepted")
}

/// Submits one clean block — every call must stay
/// `Ok(ProcessResult::CONTINUE)`; substituted output values are the
/// host's defined choice, so callers assert on counters/lifecycle,
/// not samples.
fn submit_clean_block(host: &mut PluginHost, handle: PluginHandle) {
    assert_eq!(
        submit_block_result(host, handle, &[]),
        ProcessResult::CONTINUE
    );
}

/// Arms the fault mode `normalized` — the L1 submit path is
/// fire-and-forget and a busy child may drop a submit, so a single
/// armed block is not a reliable arm (the `full_series` precedent
/// rides the event on every block for the same reason). Repeating
/// covers drops: crash/hang modes fire on the first block that
/// lands, `error` stays armed until a `none` event, and post-fault
/// submits stay defined (`Ok(CONTINUE)` or `Err(Failed)`).
fn arm_fault(host: &mut PluginHost, handle: PluginHandle, normalized: f64) {
    let events = [fault_event(normalized)];
    for _ in 0..4 {
        let in_left = [0.5_f32; FRAMES];
        let in_right = [-0.25_f32; FRAMES];
        let input = [&in_left[..], &in_right[..]];
        let mut out_left = [0.0_f32; FRAMES];
        let mut out_right = [0.0_f32; FRAMES];
        let list = EventList::new(&events, FRAMES_U32).unwrap();
        // The submit itself is accepted — the fault fires inside the
        // child's `process()`, asynchronously.
        let _ = host_block(host, handle, &input, &mut out_left, &mut out_right, list);
        thread::sleep(Duration::from_millis(5));
    }
}

/// Lifecycle sanity + descriptor pin + clean pass-through: the
/// child-synthesized `test_fault` descriptor field-matches the
/// parent's pin through the real handshake, the instance reaches
/// `Processing`, and unarmed blocks pass audio through bitwise.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_descriptor_and_clean_block() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();

    let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
        panic!("unexpected reply");
    };
    // `Scan` replays the registered descriptor surface; the
    // field-equality proof against the child's real factory scan
    // rides `Instantiate`'s handshake (`Descriptor::matches`).
    assert_eq!(descriptors, vec![expected]);

    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );

    // Sustained unarmed delivery — pass-through is bitwise.
    let delivered = drive_clean_blocks(&mut host, handle, 16);
    assert!(
        delivered >= 4,
        "the L1 pipeline barely delivered — {delivered} non-silent calls"
    );

    // Clean teardown — no `InstanceFailed` notification exists.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// `abort` mode: the armed block's `process()` calls
/// `std::process::abort()` inside the child — the parent observes
/// `Failed` convergence, exactly one `InstanceFailed` notification
/// carrying an exit status, defined `Err(Failed)` rejections, and a
/// bounded `Destroy`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_abort_fails_instance() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.25 → plain 1.0 → mode `abort`.
    arm_fault(&mut host, handle, 0.25);

    wait_host_failed(&host, handle);
    let (notified, exit, _cause) = wait_host_notification(&mut host);
    assert_eq!(notified, handle);
    // Windows reports abort as an exit code, unix as a signal.
    assert!(
        exit.code.is_some() || exit.signal.is_some(),
        "notification carried no exit status"
    );
    assert!(host.try_recv_notification().is_none());

    // Defined rejections on a `Failed` instance — no restart.
    assert_eq!(
        host.execute(HostCommand::Initialize { handle }),
        Err(HostError::Failed)
    );
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Failed)
    );
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));

    // `process` on the failed leg: `Ok(CONTINUE)` + silence.
    let in_left = [1.0_f32; FRAMES];
    let in_right = [-1.0_f32; FRAMES];
    let input = [&in_left[..], &in_right[..]];
    let mut out_left = [7.0_f32; FRAMES];
    let mut out_right = [7.0_f32; FRAMES];
    assert_eq!(
        host_block(
            &mut host,
            handle,
            &input,
            &mut out_left,
            &mut out_right,
            EventList::empty(FRAMES_U32),
        ),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(out_left.iter().all(|&s| sample_eq(s, 0.0)));
    assert!(out_right.iter().all(|&s| sample_eq(s, 0.0)));

    // `Destroy` is the escape — `Ok`; the notification was exactly
    // once.
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// `exit` mode with a nonzero code: normalized `0.605` → plain
/// `2.42` → `std::process::exit(42)` inside the child — the parent
/// observes `Failed` and `exit.code == Some(42)`, proving the exit
/// code rides the parameter encoding end to end.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_voluntary_exit_reports_code() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.605 → plain 2.42 → mode `exit`, code 42.
    arm_fault(&mut host, handle, 0.605);

    wait_host_failed(&host, handle);
    let (notified, exit, _cause) = wait_host_notification(&mut host);
    assert_eq!(notified, handle);
    assert_eq!(
        exit.code,
        Some(42),
        "the armed exit code must reach the parent verbatim"
    );

    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// `exit(0)` is still a failure: a voluntary clean exit while live
/// converges to `Failed` exactly like a crash — the
/// `voluntary_exit_zero_is_failure` precedent applied to a real
/// plugin. normalized `0.5` → plain `2.0` → `exit(0)`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_exit_zero_is_failure() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.5 → plain 2.0 → mode `exit`, code 0.
    arm_fault(&mut host, handle, 0.5);

    wait_host_failed(&host, handle);
    let (notified, exit, _cause) = wait_host_notification(&mut host);
    assert_eq!(notified, handle);
    assert_eq!(exit.code, Some(0));

    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// `hang` mode + a short `command_timeout`: the armed block wedges
/// the child's `process()` forever — audio keeps flowing as defined
/// passthrough/silence (`Ok(CONTINUE)`), then the next command hits
/// `command_timeout` → `Err(Ipc)` → `Failed` with
/// `cause: Timeout`, the watchdog kills the child, and `Destroy`
/// stays bounded.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_hang_converges_on_timeout() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut spec = OutOfProcessSpec::new(expected, runner_program());
    spec.command_timeout = Duration::from_millis(500);
    let mut host = PluginHost::new();
    host.register_out_of_process(spec).unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);
    let passthrough_before = host
        .audio_counters(handle)
        .map_or(0, |c| c.passthrough_submits);

    // normalized 0.75 → plain 3.0 → mode `hang` — the child's
    // `process()` never returns.
    arm_fault(&mut host, handle, 0.75);

    // The audio leg keeps answering `Ok(CONTINUE)` with substituted
    // (passthrough/silence) output while the child is wedged — a few
    // submits prove the data plane stays defined.
    for _ in 0..6 {
        submit_clean_block(&mut host, handle);
    }

    // The wedged child cannot answer commands — `command_timeout`
    // (500 ms) bounds the wait into `Err(Ipc)` and `Failed`.
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Ipc)
    );
    wait_host_failed(&host, handle);
    let (notified, _exit, cause) = wait_host_notification(&mut host);
    assert_eq!(notified, handle);
    assert_eq!(cause, FailureCause::Timeout);

    // The watchdog killed the child — substituted output kept
    // accruing while it was wedged.
    let passthrough_after = host
        .audio_counters(handle)
        .map_or(0, |c| c.passthrough_submits);
    assert!(
        passthrough_after > passthrough_before,
        "wedged-leg submits must substitute passthrough output"
    );

    // `Destroy` on the killed child stays bounded.
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// Bundled-route fan-out: two `test_fault` instances share one child
/// (`Bundled` pools up to `WIRE_MAX_INSTANCES`). Arming `abort` on
/// one kills the child — **both** co-resident instances converge to
/// `Failed` and each reports its own `InstanceFailed` notification.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_bundled_crash_fans_out() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let first = instantiate(&mut host, expected.id);
    let second = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, first);
    drive_to_processing(&mut host, second);
    assert!(drive_clean_blocks(&mut host, first, 16) >= 4);
    assert!(drive_clean_blocks(&mut host, second, 16) >= 4);

    // Arm `abort` on the first instance only — the shared child dies.
    arm_fault(&mut host, first, 0.25);

    // Both handles converge to `Failed` — the crash is child-scoped.
    wait_host_failed(&host, first);
    wait_host_failed(&host, second);

    // Exactly two `InstanceFailed` notifications — one per instance.
    let mut notified = Vec::new();
    for _ in 0..2 {
        let (handle, _exit, _cause) = wait_host_notification(&mut host);
        notified.push(handle);
    }
    assert!(
        notified.contains(&first) && notified.contains(&second),
        "both co-resident instances must be notified: {notified:?}"
    );
    assert!(host.try_recv_notification().is_none());

    host.execute(HostCommand::Destroy { handle: first })
        .unwrap();
    host.execute(HostCommand::Destroy { handle: second })
        .unwrap();
    assert_eq!(host.lifecycle_state(first), None);
    assert_eq!(host.lifecycle_state(second), None);
}

/// Dedicated-route isolation: `test_fault` and `test_gain` each get
/// their own child (`Dedicated`). Arming `abort` on the fault
/// instance kills only its child — the gain instance keeps
/// `Processing` and keeps delivering defined audio.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_dedicated_isolation() {
    assert_fixture_present();

    let fault_expected = EXPECTED_FAULT;
    let gain_expected = EXPECTED_GAIN;
    let mut fault_spec = OutOfProcessSpec::new(fault_expected, runner_program());
    fault_spec.isolation = OutOfProcessIsolation::Dedicated;
    let mut gain_spec = OutOfProcessSpec::new(gain_expected, runner_program());
    gain_spec.isolation = OutOfProcessIsolation::Dedicated;

    let mut host = PluginHost::new();
    host.register_out_of_process(fault_spec).unwrap();
    host.register_out_of_process(gain_spec).unwrap();

    let fault = instantiate(&mut host, fault_expected.id);
    let gain = instantiate(&mut host, gain_expected.id);
    drive_to_processing(&mut host, fault);
    drive_to_processing(&mut host, gain);
    assert!(drive_clean_blocks(&mut host, fault, 16) >= 4);
    assert!(drive_clean_blocks(&mut host, gain, 16) >= 4);

    // Arm `abort` on the fault instance — its dedicated child dies.
    arm_fault(&mut host, fault, 0.25);
    wait_host_failed(&host, fault);
    let (notified, _exit, _cause) = wait_host_notification(&mut host);
    assert_eq!(notified, fault);

    // The gain instance is untouched: still `Processing`, still
    // delivering defined audio (unity gain → `out == in`).
    assert_eq!(host.lifecycle_state(gain), Some(LifecycleState::Processing));
    let empty = EventList::empty(FRAMES_U32);
    let mut delivered = 0_u32;
    for _ in 0..8 {
        let in_left = [0.5_f32; FRAMES];
        let in_right = [-0.25_f32; FRAMES];
        let input = [&in_left[..], &in_right[..]];
        let mut out_left = [0.0_f32; FRAMES];
        let mut out_right = [0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block(
                &mut host,
                gain,
                &input,
                &mut out_left,
                &mut out_right,
                empty,
            ),
            Ok(ProcessResult::CONTINUE)
        );
        if out_left.iter().any(|&s| !sample_eq(s, 0.0)) {
            assert!(
                out_left.iter().all(|&s| sample_eq(s, 0.5))
                    && out_right.iter().all(|&s| sample_eq(s, -0.25)),
                "gain instance must keep delivering unity-gain audio"
            );
            delivered += 1;
        }
    }
    assert!(
        delivered >= 2,
        "the isolated gain instance barely delivered — {delivered} blocks"
    );
    assert_eq!(host.lifecycle_state(gain), Some(LifecycleState::Processing));

    // Clean teardown of both legs.
    host.execute(HostCommand::StopProcessing { handle: gain })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle: gain })
        .unwrap();
    host.execute(HostCommand::Destroy { handle: gain }).unwrap();
    host.execute(HostCommand::Destroy { handle: fault })
        .unwrap();
    assert_eq!(host.lifecycle_state(gain), None);
    assert_eq!(host.lifecycle_state(fault), None);
    assert!(host.try_recv_notification().is_none());
}

/// `error` mode is a *defined rejection*, not a crash: normalized
/// `1.0` → plain `4.0` → the plugin returns `CLAP_PROCESS_ERROR` that
/// surfaces to the caller as `Ok(ProcessResult { status: Error })`
/// — a *defined* rejection, not a crash: the instance **stays**
/// `Processing` (never `Failed`), and a follow-up `none` event
/// restores `Continue` results.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_error_is_defined_rejection() {
    assert_fixture_present();

    let expected = EXPECTED_FAULT;
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 1.0 → plain 4.0 → mode `error` — armed persistently.
    // Every poll block re-carries the event so a dropped submit
    // cannot lose the arm (same law as `arm_fault`), and results
    // land asynchronously behind the lookahead window — poll until
    // `ProcessStatus::Error` has been observed on at least two
    // separate calls (sustained rejection, not a fluke).
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut error_results = 0_u32;
    while Instant::now() < deadline && error_results < 2 {
        let result = submit_block_result(&mut host, handle, &[fault_event(1.0)]);
        if result.status == ProcessStatus::Error {
            error_results += 1;
        }
    }
    assert!(
        error_results >= 2,
        "armed error mode must surface as ProcessStatus::Error"
    );

    // The defined rejection is observable — and the instance is
    // alive, not `Failed`.
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing),
        "a defined rejection must not converge to Failed"
    );
    assert!(host.try_recv_notification().is_none());

    // Disarm — normalized 0.0 → plain 0.0 → mode `none`. In-flight
    // armed blocks (bounded by the lookahead window) may still
    // report `Error`; once they drain every result is `Continue`.
    arm_fault(&mut host, handle, 0.0);
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut consecutive_continue = 0_u32;
    while Instant::now() < deadline && consecutive_continue < 6 {
        let result = submit_block_result(&mut host, handle, &[]);
        consecutive_continue = if result.status == ProcessStatus::Continue {
            consecutive_continue + 1
        } else {
            0
        };
    }
    assert_eq!(
        consecutive_continue, 6,
        "a disarmed instance must stop reporting process errors"
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

// ---------------------------------------------------------------------------
// Spawn / pre-Hello fault injection — `--ruviform-test-fault=MODE` argv
// ---------------------------------------------------------------------------

/// The `test_spawn_fail` descriptor — listed by the child's
/// `clap_plugin_factory` scan but deliberately unmatched in
/// `create_plugin`, so `CreateInstance` returns `None` (the
/// spawn-phase create-rejection oracle).
const EXPECTED_SPAWN_FAIL: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-spawn-fail",
    name: "test_spawn_fail",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["audio-effect", "stereo"],
};

/// The causes a pre-`Hello` child death may surface as — pipe EOF
/// (`Transport`), `try_wait` observation (`ExitObserved`), or a
/// channel-disconnect/reap `Internal`, whichever the supervisor
/// records first. `Timeout`/`Handshake` stay strict — the hang and
/// mismatch tests assert those modes separately.
const PRE_HELLO_DEATH: &[FailureCause] = &[
    FailureCause::Transport,
    FailureCause::ExitObserved,
    FailureCause::Internal,
];

/// An `OutOfProcessSpec` on the real runner carrying `args` — the
/// spawn-fault argv channel.
fn spec_with_args(expected: PluginDescriptor, args: &[&str]) -> OutOfProcessSpec {
    let mut spec = OutOfProcessSpec::new(expected, runner_program());
    spec.args = args.iter().map(OsString::from).collect();
    spec
}

/// Runs `Instantiate` expecting the defined pre-mint
/// `Err(HostError::SpawnFailed{cause, diagnostics_id})` rejection whose
/// cause is one of `causes`; returns the diagnostics id for record
/// queries.
fn instantiate_spawn_failed(
    host: &mut PluginHost,
    plugin_id: &str,
    causes: &[FailureCause],
) -> u64 {
    match host.execute(HostCommand::Instantiate {
        plugin_id: plugin_id.to_owned(),
    }) {
        Err(HostError::SpawnFailed {
            cause,
            diagnostics_id,
        }) => {
            assert!(
                causes.contains(&cause),
                "unexpected FailureCause {cause:?} — expected one of {causes:?}"
            );
            diagnostics_id
        }
        other => panic!("expected SpawnFailed, got {other:?}"),
    }
}

/// `bad-fixture`: the runner resolves a guaranteed-missing fixture
/// path, `ClapLibrary::load` fails, and the child exits 1 before
/// `Hello` — `SpawnFailed` with a retained diagnostics record whose
/// stderr carries the load-failure diagnostic.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_bad_fixture_is_spawn_failed() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        EXPECTED_FAULT,
        &["--ruviform-test-fault=bad-fixture"],
    ))
    .unwrap();

    let diagnostics_id = instantiate_spawn_failed(
        &mut host,
        EXPECTED_FAULT.id,
        &[
            FailureCause::Transport,
            FailureCause::ExitObserved,
            FailureCause::Internal,
        ],
    );
    match host.diagnostics_query(diagnostics_id) {
        DiagnosticQuery::Found(record) => {
            let stderr = String::from_utf8_lossy(&record.stderr.bytes);
            assert!(
                stderr.contains("cannot load fixture"),
                "diagnostics stderr missing the load diagnostic: {stderr}"
            );
        }
        other => panic!("diagnostics record missing: {other:?}"),
    }
    // Pre-mint failure: nothing registered, no notification, the
    // route itself stays usable.
    assert!(host.try_recv_notification().is_none());
}

/// `exit:N`: the child loads the fixture then exits before `Hello` —
/// `SpawnFailed` converges on the transport/exit observation, never a
/// hang.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_exit_before_hello() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        EXPECTED_FAULT,
        &["--ruviform-test-fault=exit:3"],
    ))
    .unwrap();

    instantiate_spawn_failed(&mut host, EXPECTED_FAULT.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// `abort`: the child loads the fixture then aborts before `Hello` —
/// same defined convergence as `exit`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_abort_before_hello() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        EXPECTED_FAULT,
        &["--ruviform-test-fault=abort"],
    ))
    .unwrap();

    instantiate_spawn_failed(&mut host, EXPECTED_FAULT.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// `stderr:N`: the child emits marker bytes on stderr then exits —
/// the retained diagnostics record proves the parent's stderr capture
/// works on the pre-`Hello` path.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_stderr_is_captured() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        EXPECTED_FAULT,
        &["--ruviform-test-fault=stderr:4096"],
    ))
    .unwrap();

    let diagnostics_id = instantiate_spawn_failed(&mut host, EXPECTED_FAULT.id, PRE_HELLO_DEATH);
    match host.diagnostics_query(diagnostics_id) {
        DiagnosticQuery::Found(record) => {
            assert!(
                record
                    .stderr
                    .bytes
                    .windows(b"ruviform-test-clap-spawn-fault-stderr".len())
                    .any(|w| w == b"ruviform-test-clap-spawn-fault-stderr"),
                "diagnostics stderr missing the spawn-fault marker"
            );
        }
        other => panic!("diagnostics record missing: {other:?}"),
    }
}

/// `hang`: the child loads the fixture then parks before `Hello` —
/// the parent's `command_timeout` decides `SpawnFailed{Timeout}` and
/// the wedged child is killed and reaped, never left running.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_hang_before_hello_times_out() {
    assert_fixture_present();

    let mut spec = spec_with_args(EXPECTED_FAULT, &["--ruviform-test-fault=hang"]);
    spec.command_timeout = Duration::from_millis(500);
    let mut host = PluginHost::new();
    host.register_out_of_process(spec).unwrap();

    let diagnostics_id =
        instantiate_spawn_failed(&mut host, EXPECTED_FAULT.id, &[FailureCause::Timeout]);
    match host.diagnostics_query(diagnostics_id) {
        DiagnosticQuery::Found(_) => {}
        other => panic!("diagnostics record missing: {other:?}"),
    }
    assert!(host.try_recv_notification().is_none());
}

/// Unrecognized argv: any argument the fault parser does not claim
/// falls through to `run`, whose argv grammar rejects it as a usage
/// error (exit 2) — still a defined pre-`Hello` failure through the
/// real binary.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_usage_rejection_is_spawn_failed() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        EXPECTED_FAULT,
        &["--definitely-not-a-real-flag"],
    ))
    .unwrap();

    instantiate_spawn_failed(&mut host, EXPECTED_FAULT.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// Descriptor mismatch: the spec's descriptor does not field-match
/// the child's factory scan — a defined `Handshake` rejection, not a
/// transport failure.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_descriptor_mismatch_is_handshake() {
    assert_fixture_present();

    let mut mismatched = EXPECTED_FAULT;
    mismatched.name = "test_fault_typo";
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(mismatched, runner_program()))
        .unwrap();

    instantiate_spawn_failed(&mut host, mismatched.id, &[FailureCause::Handshake]);
    assert!(host.try_recv_notification().is_none());
}

/// `test_spawn_fail`: the descriptor scans cleanly (field-match
/// succeeds, wire `CreateInstance` mints a dormant handle) but the
/// real ABI `create_plugin` declines inside `init` — `Initialize`
/// reports the defined `PluginFailed` rejection. The pooled child
/// stays healthy: a healthy plugin on the same host still drives to
/// `Processing`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_create_instance_rejection() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED_SPAWN_FAIL, runner_program()))
        .unwrap();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED_GAIN, runner_program()))
        .unwrap();

    // Wire `CreateInstance` is dormant for both bridges — the handle
    // mints; the ABI create runs at `Initialize`.
    let handle = instantiate(&mut host, EXPECTED_SPAWN_FAIL.id);
    assert_eq!(
        host.execute(HostCommand::Initialize { handle }),
        Err(HostError::PluginFailed),
        "the listed-but-uncreatable plugin must decline at init"
    );
    // `Destroy` is the escape for the failed instance.
    host.execute(HostCommand::Destroy { handle }).unwrap();

    // A healthy plugin on the same host instantiates and initializes.
    let gain = instantiate(&mut host, EXPECTED_GAIN.id);
    host.execute(HostCommand::Initialize { handle: gain })
        .unwrap();
    host.execute(HostCommand::Destroy { handle: gain }).unwrap();
    assert!(host.try_recv_notification().is_none());
}
