//! `test_fault` L1 end-to-end tests — this crate's own VST3 cdylib
//! serving the `test_fault` class out-of-process through
//! `ruviform-plugin-test-vst3-l1-runner` (Issue #630 `test_fault`
//! slice; oracle `docs/test-specs/issue-630-test-fault.md`).
//!
//! Issue #885 extends the suite to `test_editor`: the editor fixture
//! carries the same in-band `Fault` parameter, so the `abort`/`hang`
//! tests below instantiate the `test_editor` class and drive the
//! same arming law — an editor-session-capable instance dying or
//! wedging inside its own `process()` (the design's
//! real-session-crash representative fault and the real-session-hang
//! primary route).
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
//! parameter (id `0`, plain `[0.0, 8.0]`) inside a `process` block.
//! The armed block's `process()` dispatches the mode at its end —
//! `abort`/`exit`/`hang` kill or wedge the *child* (never this test
//! process), `error` returns a defined process failure. Normalized
//! encodings used below (plain = normalized × 8.0):
//!
//! | mode | plain | normalized |
//! | ---- | ----- | ---------- |
//! | `none` | `0.0` | `0.0` |
//! | `abort` | `1.0` | `0.125` |
//! | `exit(42)` | `2.42` | `0.3025` |
//! | `exit(0)` | `2.0` | `0.25` |
//! | `hang` | `3.0` | `0.375` |
//! | `error` | `4.0` | `0.5` |
//! | `rt_alloc` | `5.0` | `0.625` |
//! | `bad_time` | `6.0` | `0.75` |
//! | `invalid` | `7.0` | `0.875` |

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::io::Write;
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
    OutOfProcessSpec, PluginHandle, PluginHost, SpawnPhase,
};

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// Derives a descriptor id the way the child's class scan does —
/// the component CID's raw 16 bytes as lowercase hex (`tuid_hex`).
/// `uid` is the vendored deterministic `const fn` the fixture's CIDs
/// are built with — independently re-derived here so the hex mapping
/// is verified, not restated. `TestFaultComponent::CID` is
/// `pub(crate)` and unreachable from `tests/`; the `uid(...)` call
/// derives the equal value. The `String` is leaked once for the
/// descriptor's `'static` bound — test-scoped, bounded.
fn cid_hex(uid: vst3::Steinberg::TUID) -> &'static str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(32);
    for &byte in &uid {
        // `TUID` elements are `c_char` — `i8` on this platform — so
        // reinterpret each as its unsigned bit pattern first.
        let byte = byte.cast_unsigned();
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    String::leak(out)
}

/// The `test_fault` descriptor id — `TestFaultComponent::CID`'s
/// `uid(...)` derivation (`fault.rs`).
fn expected_fault_id() -> &'static str {
    cid_hex(vst3::uid(
        0x7C3A_91E4,
        0x5F2B_8D61,
        0xA4E7_0C93,
        0xD8B5_6F2A,
    ))
}

/// The `test_gain` descriptor id — `TestGainComponent::CID`'s
/// `uid(...)` derivation (the `l1_real_test_plugin_e2e` precedent).
fn expected_gain_id() -> &'static str {
    cid_hex(vst3::uid(
        0x6D4F_8A21,
        0x3B7C_9E05,
        0xF1A2_4C68,
        0xB9D3_7E51,
    ))
}

/// The `test_editor` descriptor id — `TestEditorComponent::CID`'s
/// `uid(...)` derivation (`editor.rs`).
fn expected_editor_id() -> &'static str {
    cid_hex(vst3::uid(
        0x3F8A_4C62,
        0x9D1E_7B05,
        0xA6F3_48D9,
        0xC2E7_5B81,
    ))
}

/// The `test_fault` descriptor the child synthesizes from the class
/// scan — the `OutOfProcessSpec` oracle must field-match it.
fn expected_fault_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_fault_id(),
        name: "test_fault",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

/// The `test_gain` descriptor — for the dedicated-isolation pairing.
fn expected_gain_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_gain_id(),
        name: "test_gain",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

/// The `test_editor` descriptor the child synthesizes from the class
/// scan — the `OutOfProcessSpec` oracle must field-match it.
fn expected_editor_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_editor_id(),
        name: "test_editor",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

/// How long tests poll for an asynchronous `Failed` notification —
/// crash convergence needs at most a driver poll plus scheduling;
/// 10 s is generous headroom for loaded CI hosts (the
/// `l1_proxy_e2e` precedent's bound).
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
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-vst3-l1-runner"));
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
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-vst3-l1-runner"))
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
/// equal bit-for-bit (no epsilon needed or wanted; `l1_proxy_e2e`
/// precedent).
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

/// Bound on `Instantiate` retries after a pre-mint
/// `SpawnFailed{Timeout}` — the rf#841 C1 signature. The failure is
/// an environment deadline event (a stalled spawn-path wait), not a
/// contract verdict: `SpawnFailed` is pre-mint only, so a fresh
/// attempt is the defined recovery — the same "pre-contract
/// environment retry" shape as `ARM_RESEND_CALLS` (rf#766). The
/// bound keeps a persistent stall an honest failure instead of an
/// absorbed flake (`docs/discovery/issue-841-spawn-deadline-flake.md`
/// Decision C — assertion/gate unchanged).
const SPAWN_TIMEOUT_RETRIES: u32 = 3;

/// `Instantiate` with a bounded retry on the rf#841 C1 signature —
/// `SpawnFailed{cause: Timeout}` proven pre-mint by its retained
/// diagnostics record. The signature check is explicit per Decision
/// C: the error's `cause` is `Timeout` AND the record is `Found`
/// with `phase` in {`PreSpawn`, `Spawn`, `DrainInit`, `Handshake`}
/// and `cause == Some(Timeout)`. A record frozen at `Live` (a
/// `Scan`/`CreateInstance`-wait timeout — the phase advances to
/// `Live` before those waits run), a non-`Timeout` cause, or an
/// `Evicted`/`NotFound` query result is signature-missed and fails
/// at once — never retried. Every `SpawnFailed` attempt is traced
/// to the real stderr — bypassing the harness's per-test capture so
/// the trace lands in lane logs even when a later attempt recovers
/// the test — carrying the attempt index plus the record's
/// `diagnostics_id`/`phase`/`cause` (the deadline-phase attribution
/// the `SpawnFailed` payload alone cannot show — rf#841 Residual).
/// Bound exhaustion is an honest failure — the flake is never
/// absorbed silently.
fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    for attempt in 0..=SPAWN_TIMEOUT_RETRIES {
        match host.execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        }) {
            Ok(HostReply::Instantiated { handle }) => return handle,
            Ok(other) => panic!("unexpected reply: {other:?}"),
            Err(HostError::SpawnFailed {
                cause,
                diagnostics_id,
            }) => {
                let query = host.diagnostics_query(diagnostics_id);
                let (phase, record_cause) = match &query {
                    DiagnosticQuery::Found(record) => (Some(record.phase), record.cause),
                    _ => (None, None),
                };
                let _ = writeln!(
                    std::io::stderr(),
                    "instantiate SpawnFailed trace: plugin={plugin_id} \
                     attempt={attempt} cause={cause:?} \
                     diagnostics_id={diagnostics_id} phase={phase:?} \
                     record_cause={record_cause:?}"
                );
                let signature = cause == FailureCause::Timeout
                    && matches!(
                        &query,
                        DiagnosticQuery::Found(record)
                            if matches!(
                                record.phase,
                                SpawnPhase::PreSpawn
                                    | SpawnPhase::Spawn
                                    | SpawnPhase::DrainInit
                                    | SpawnPhase::Handshake
                            ) && record.cause == Some(FailureCause::Timeout)
                    );
                assert!(
                    signature,
                    "instantiate failed: SpawnFailed{{cause:{cause:?}, \
                     diagnostics_id:{diagnostics_id}}} is outside the \
                     pre-mint timeout signature (phase={phase:?} \
                     record_cause={record_cause:?}) — not retried"
                );
                assert!(
                    attempt < SPAWN_TIMEOUT_RETRIES,
                    "instantiate failed: the pre-mint \
                     SpawnFailed{{cause:Timeout}} signature persisted \
                     through the {SPAWN_TIMEOUT_RETRIES}-retry bound \
                     (phase={phase:?}) — an honest environment \
                     failure, not an absorbed flake"
                );
            }
            Err(error) => panic!("instantiate failed: {error:?}"),
        }
    }
    unreachable!("the loop exits only through `return` or `panic!`")
}

/// Per-call counter deltas recorded around one `process` — the
/// oracle's provenance record for the delivered block.
/// `passthrough_submits` increments when the leg substitutes
/// hold-last/silence output instead of a matched child block;
/// `dropped_submits` increments when this call's input block —
/// audio plus its queued events — never reached the child at all.
/// A dropped submit also substitutes output, so `passthrough > 0`
/// marks every non-matched block and `dropped` distinguishes event
/// loss. Content/status asserts must count matched blocks only: a
/// substituted block replays `last_output`, which can satisfy a
/// pass-through sample, a `Continue` status, or a zero
/// `tail_frames` without the child having rendered anything new.
#[derive(Clone, Copy, Default)]
struct LegDeltas {
    /// `passthrough_submits` delta: `> 0` ⇒ substituted output.
    passthrough: u64,
    /// `dropped_submits` delta: `> 0` ⇒ this call's submit dropped.
    dropped: u64,
}

/// One stereo block through `PluginHost::process` — `test_fault`
/// declares the same single stereo bus pair as `test_gain`. Returns
/// the `ProcessResult` plus the leg's per-call counter deltas so
/// callers can classify the delivered block as matched
/// (`passthrough == 0`) or substituted (`passthrough > 0`).
fn host_block(
    host: &mut PluginHost,
    handle: PluginHandle,
    input: &[&[f32]; 2],
    out_left: &mut [f32],
    out_right: &mut [f32],
    events: EventList<'_>,
) -> (Result<ProcessResult, HostError>, LegDeltas) {
    let counters = |h: &PluginHost| {
        h.audio_counters(handle)
            .map_or((0, 0), |c| (c.passthrough_submits, c.dropped_submits))
    };
    let before = counters(host);
    let input_view = PlanarBufferView::new(&input[..]).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut *out_left, &mut *out_right];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    let result = host.process(handle, &inputs, &mut outputs, events, None);
    let after = counters(host);
    (
        result,
        LegDeltas {
            passthrough: after.0 - before.0,
            dropped: after.1 - before.1,
        },
    )
}

/// One `Fault` `ParamValue` event at frame offset 0 — `normalized`
/// is the wire value; the fixture's plain domain is `[0.0, 8.0]`
/// (`plain = normalized × 8.0`).
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
/// actually processing before a fault is armed. Only *matched*
/// calls count: substituted output replays an older `last_output`
/// block — defined, but not this call's child rendering — so a
/// stale replay must not accrue delivery evidence.
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
        let (result, deltas) = host_block(
            &mut *host,
            handle,
            &input,
            &mut out_left,
            &mut out_right,
            empty,
        );
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        if deltas.passthrough > 0 {
            continue;
        }
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
/// and returns the `ProcessResult` *plus* the delivered output
/// channels and the leg's counter deltas — the oracle for the
/// marker/state-bearing modes (`rt_alloc`'s `200.0` sample,
/// `invalid`'s NaN block), classified per call.
fn submit_block_full(
    host: &mut PluginHost,
    handle: PluginHandle,
    events: &[Event],
) -> (ProcessResult, [f32; FRAMES], [f32; FRAMES], LegDeltas) {
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
    let (result, deltas) = host_block(host, handle, &input, &mut out_left, &mut out_right, list);
    let result = result.expect("process submit must be accepted");
    (result, out_left, out_right, deltas)
}

/// Submits one block carrying `events` (empty slice = clean block)
/// and returns the `ProcessResult` plus the leg's counter deltas —
/// asserts only that the submit was accepted (`Ok`); the status is
/// the caller's oracle (an armed `error` block legitimately reports
/// `ProcessStatus::Error`).
fn submit_block_result(
    host: &mut PluginHost,
    handle: PluginHandle,
    events: &[Event],
) -> (ProcessResult, LegDeltas) {
    let (result, _out_left, _out_right, deltas) = submit_block_full(host, handle, events);
    (result, deltas)
}

/// Submits one clean block — every call must stay
/// `Ok(ProcessResult::CONTINUE)`; substituted output values are the
/// host's defined choice, so callers assert on counters/lifecycle,
/// not samples.
fn submit_clean_block(host: &mut PluginHost, handle: PluginHandle) {
    let (result, _deltas) = submit_block_result(host, handle, &[]);
    assert_eq!(result, ProcessResult::CONTINUE);
}

/// Cap on armed-submit resends — each resend is one call carrying
/// the fault event; if none lands within the window the test fails
/// honestly at the arm instead of polling an unarmed child
/// downstream (the `NOTE_OFF_RESEND_CALLS` bound in
/// `l1_real_test_plugin_e2e.rs`).
const ARM_RESEND_CALLS: u32 = 8;

/// Arms the fault mode `normalized`. A single armed block is not a
/// reliable arm: the L1 submit path is fire-and-forget, and a busy
/// slot or full channel drops the submit wholesale — events
/// included (`dropped_submits`). The event therefore rides every
/// call until a `dropped` delta of 0 reports a landing; re-sends
/// are idempotent because the fixture keeps the *last* `Fault`
/// event of a block and persists it across blocks (the `fault.rs`
/// arming law). Crash/hang modes fire on the first block that
/// lands, `error`/`rt_alloc`/`bad_time`/`invalid` stay armed until
/// a `none` event, and post-fault submits stay defined
/// (`Ok(CONTINUE)` or `Err(Failed)`). Asserts a landing within
/// `ARM_RESEND_CALLS` — a silent unarmed run must fail here, not
/// downstream.
fn arm_fault(host: &mut PluginHost, handle: PluginHandle, normalized: f64) {
    let events = [fault_event(normalized)];
    let mut landed = false;
    for _ in 0..ARM_RESEND_CALLS {
        let in_left = [0.5_f32; FRAMES];
        let in_right = [-0.25_f32; FRAMES];
        let input = [&in_left[..], &in_right[..]];
        let mut out_left = [0.0_f32; FRAMES];
        let mut out_right = [0.0_f32; FRAMES];
        let list = EventList::new(&events, FRAMES_U32).unwrap();
        // The submit itself is accepted — the fault fires inside the
        // child's `process()`, asynchronously. A `dropped` delta of 0
        // means this call's block — event included — reached the
        // child-side queue.
        let (_result, deltas) =
            host_block(host, handle, &input, &mut out_left, &mut out_right, list);
        if deltas.dropped == 0 {
            landed = true;
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        landed,
        "the armed submit never landed within {ARM_RESEND_CALLS} calls"
    );
}

/// Lifecycle sanity + descriptor pin + clean pass-through: the
/// child-synthesized `test_fault` descriptor field-matches the
/// parent's pin through the real handshake, the instance reaches
/// `Processing`, and unarmed blocks pass audio through bitwise.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_descriptor_and_clean_block() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();

    let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
        panic!("unexpected reply");
    };
    // `Scan` replays the registered descriptor surface; the
    // field-equality proof against the child's real class scan rides
    // `Instantiate`'s handshake (`Descriptor::matches`).
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

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.125 → plain 1.0 → mode `abort`.
    arm_fault(&mut host, handle, 0.125);

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
    let (result, _deltas) = host_block(
        &mut host,
        handle,
        &input,
        &mut out_left,
        &mut out_right,
        EventList::empty(FRAMES_U32),
    );
    assert_eq!(result, Ok(ProcessResult::CONTINUE));
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

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.3025 → plain 2.42 → mode `exit`, code 42.
    arm_fault(&mut host, handle, 0.3025);

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
/// plugin. normalized `0.25` → plain `2.0` → `exit(0)`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_exit_zero_is_failure() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.25 → plain 2.0 → mode `exit`, code 0.
    arm_fault(&mut host, handle, 0.25);

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

    let expected = expected_fault_descriptor();
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

    // normalized 0.375 → plain 3.0 → mode `hang` — the child's
    // `process()` never returns.
    arm_fault(&mut host, handle, 0.375);

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

    let expected = expected_fault_descriptor();
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
    arm_fault(&mut host, first, 0.125);

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

    let fault_expected = expected_fault_descriptor();
    let gain_expected = expected_gain_descriptor();
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
    arm_fault(&mut host, fault, 0.125);
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
        let (result, deltas) = host_block(
            &mut host,
            gain,
            &input,
            &mut out_left,
            &mut out_right,
            empty,
        );
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        // A substituted replay of an older delivered block is not
        // evidence the isolated leg keeps rendering — count matched
        // calls only.
        if deltas.passthrough == 0 && out_left.iter().any(|&s| !sample_eq(s, 0.0)) {
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
/// `0.5` → plain `4.0` → the plugin returns a process error that
/// surfaces to the caller as `Ok(ProcessResult { status: Error })`
/// — a *defined* rejection, not a crash: the instance **stays**
/// `Processing` (never `Failed`), and a follow-up `none` event
/// restores `Continue` results.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_error_is_defined_rejection() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.5 → plain 4.0 → mode `error` — armed persistently.
    // Every poll block re-carries the event so a dropped submit
    // cannot lose the arm (same law as `arm_fault`), and results
    // land asynchronously behind the lookahead window — poll until
    // `ProcessStatus::Error` has been observed on at least two
    // separate calls (sustained rejection, not a fluke).
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut error_results = 0_u32;
    while Instant::now() < deadline && error_results < 2 {
        let (result, deltas) = submit_block_result(&mut host, handle, &[fault_event(0.5)]);
        // `ProcessStatus::Error` only reaches the wire on a matched
        // result — pass-through substitution always reports
        // `Continue` — but classify anyway so the count's provenance
        // stays uniform across the suite.
        if deltas.passthrough == 0 && result.status == ProcessStatus::Error {
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
        let (result, deltas) = submit_block_result(&mut host, handle, &[]);
        if deltas.passthrough > 0 {
            // Substituted — `Ok(CONTINUE)` replay, not evidence the
            // child disarmed: neither counts nor breaks the streak.
            continue;
        }
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

/// `rt_alloc` mode in the L1 child: normalized `0.625` → plain `5.0`
/// → the armed block's `process()` heap-allocates and writes the
/// `200.0` marker (`f32::from(0xC8)`, read *from* the allocated
/// buffer) into `output[0][0]`. The allocation itself is
/// host-unobservable — the marker is the fixture's own report that
/// the violation ran; the sample domain carries it across the wire.
/// The instance stays `Processing`; disarm restores pass-through.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_rt_alloc_marks_armed_blocks() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // Poll armed blocks (normalized 0.625 → plain 5.0) until the
    // `200.0` marker lands in `out_left[0]` on at least two calls —
    // sustained violation, not a fluke behind the lookahead window.
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut marked = 0_u32;
    while Instant::now() < deadline && marked < 2 {
        let (result, out_left, out_right, deltas) =
            submit_block_full(&mut host, handle, &[fault_event(0.625)]);
        // Substituted replays can carry a stale marker — only a
        // matched block is fresh evidence the armed child rendered.
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && sample_eq(out_left[0], 200.0)
            && out_left[1..].iter().all(|&s| sample_eq(s, 0.5))
            && out_right.iter().all(|&s| sample_eq(s, -0.25))
        {
            marked += 1;
        }
    }
    assert_eq!(marked, 2, "rt_alloc's heap marker must reach the host");
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing),
        "rt_alloc stays a defined, non-fatal fault"
    );

    // Disarm — normalized 0.0 → `none` restores pass-through.
    arm_fault(&mut host, handle, 0.0);
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut clean = 0_u32;
    while Instant::now() < deadline && clean < 6 {
        let (result, out_left, out_right, deltas) = submit_block_full(&mut host, handle, &[]);
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && out_left.iter().all(|&s| sample_eq(s, 0.5))
            && out_right.iter().all(|&s| sample_eq(s, -0.25))
        {
            clean += 1;
        }
    }
    assert_eq!(clean, 6, "a disarmed instance must pass audio through");

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// `bad_time` mode in the L1 child: normalized `0.75` → plain `6.0`
/// → a zero-tail pass-through fixture claims `TAIL_INFINITE` —
/// `getTailSamples` reports `0xFFFFFFFF`, which the wire carries as
/// `tail_frames == u32::MAX` — the bad-timing lie a host oracle
/// detects. The instance stays `Processing`; disarm restores the
/// zero-tail truth.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_bad_time_claims_infinite_tail() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // Poll armed blocks (normalized 0.75 → plain 6.0) until the
    // `TAIL_INFINITE` claim lands on at least two calls.
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut claimed = 0_u32;
    while Instant::now() < deadline && claimed < 2 {
        let (result, deltas) = submit_block_result(&mut host, handle, &[fault_event(0.75)]);
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && result.tail_frames == u32::MAX
        {
            claimed += 1;
        }
    }
    assert_eq!(claimed, 2, "bad_time's tail lie must reach the host");
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );

    // Disarm — the zero-tail truth returns.
    arm_fault(&mut host, handle, 0.0);
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut clean = 0_u32;
    while Instant::now() < deadline && clean < 6 {
        let (result, deltas) = submit_block_result(&mut host, handle, &[]);
        // A substituted call answers `Ok(CONTINUE)` with a zero tail
        // — indistinguishable from a disarmed render until classified.
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && result.tail_frames == 0
        {
            clean += 1;
        }
    }
    assert_eq!(clean, 6, "a disarmed instance must report a zero tail");

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// `invalid` mode in the L1 child: normalized `0.875` → plain `7.0`
/// → the armed block delivers a full-NaN output — the invalid audio
/// state a host must detect and isolate. The instance stays
/// `Processing`; disarm restores clean pass-through — the
/// isolate/recover pair a host exercises.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_fault_l1_invalid_emits_nan_block() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // Poll armed blocks (normalized 0.875 → plain 7.0) until a
    // full-NaN block lands on at least two calls.
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut nan_blocks = 0_u32;
    while Instant::now() < deadline && nan_blocks < 2 {
        let (result, out_left, out_right, deltas) =
            submit_block_full(&mut host, handle, &[fault_event(0.875)]);
        // Substituted replays can carry a stale NaN block — only a
        // matched block is fresh evidence the armed child rendered.
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && out_left.iter().all(|s| s.is_nan())
            && out_right.iter().all(|s| s.is_nan())
        {
            nan_blocks += 1;
        }
    }
    assert_eq!(nan_blocks, 2, "invalid's NaN block must reach the host");
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing),
        "invalid stays a defined, non-fatal fault"
    );

    // Disarm — pass-through returns.
    arm_fault(&mut host, handle, 0.0);
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut clean = 0_u32;
    while Instant::now() < deadline && clean < 6 {
        let (result, out_left, out_right, deltas) = submit_block_full(&mut host, handle, &[]);
        if deltas.passthrough == 0
            && result.status == ProcessStatus::Continue
            && out_left.iter().all(|&s| sample_eq(s, 0.5))
            && out_right.iter().all(|&s| sample_eq(s, -0.25))
        {
            clean += 1;
        }
    }
    assert_eq!(clean, 6, "a disarmed instance must pass audio through");

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

// ---------------------------------------------------------------------------
// `test_editor` in-band fault — Issue #885
// ---------------------------------------------------------------------------

/// `test_editor` `abort` mode (Issue #885): the editor fixture
/// carries the same in-band `Fault` law — the armed block's
/// `process()` calls `std::process::abort()` inside the child. The
/// parent observes `Failed` convergence, exactly one
/// `InstanceFailed` notification carrying an exit status, defined
/// `Err(Failed)` rejections, and a bounded `Destroy` — the same
/// crash vocabulary `test_fault` produces, now on an
/// editor-session-capable class.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_editor_l1_fault_abort_fails_instance() {
    assert_fixture_present();

    let expected = expected_editor_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert!(drive_clean_blocks(&mut host, handle, 16) >= 4);

    // normalized 0.125 → plain 1.0 → mode `abort`.
    arm_fault(&mut host, handle, 0.125);

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
    let (result, _deltas) = host_block(
        &mut host,
        handle,
        &input,
        &mut out_left,
        &mut out_right,
        EventList::empty(FRAMES_U32),
    );
    assert_eq!(result, Ok(ProcessResult::CONTINUE));
    assert!(out_left.iter().all(|&s| sample_eq(s, 0.0)));
    assert!(out_right.iter().all(|&s| sample_eq(s, 0.0)));

    // `Destroy` is the escape — `Ok`; the notification was exactly
    // once.
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// `test_editor` `hang` mode (Issue #885) + a short
/// `command_timeout`: the armed block wedges the child's `process()`
/// forever — audio keeps flowing as defined passthrough/silence
/// (`Ok(CONTINUE)`), then the next command hits `command_timeout` →
/// `Err(Ipc)` → `Failed` with `cause: Timeout`, the watchdog kills
/// the child, and `Destroy` stays bounded. On an
/// editor-session-capable class this is the design's only primary
/// route to a real-session hang.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_editor_l1_fault_hang_converges_on_timeout() {
    assert_fixture_present();

    let expected = expected_editor_descriptor();
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

    // normalized 0.375 → plain 3.0 → mode `hang` — the child's
    // `process()` never returns.
    arm_fault(&mut host, handle, 0.375);

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

// ---------------------------------------------------------------------------
// Spawn / pre-Hello fault injection — `--ruviform-test-fault=MODE` argv
// ---------------------------------------------------------------------------

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

/// The `test_spawn_fail` descriptor id — `SPAWN_FAIL_CID`'s `uid(...)`
/// derivation (`factory.rs`); the class is listed by the scan but its
/// `createInstance` always declines.
fn expected_spawn_fail_id() -> &'static str {
    cid_hex(vst3::uid(
        0x5E2A_9F14,
        0x7C3D_8B61,
        0xA4E9_1F27,
        0xD8B3_6C05,
    ))
}

/// The `test_spawn_fail` descriptor the child synthesizes from the
/// class scan — listed, but `createInstance` rejects it.
fn expected_spawn_fail_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_spawn_fail_id(),
        name: "test_spawn_fail",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

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
/// path, `Vst3Library::load` fails, and the child exits 1 before
/// `Hello` — `SpawnFailed` with a retained diagnostics record whose
/// stderr carries the load-failure diagnostic.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_bad_fixture_is_spawn_failed() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        expected,
        &["--ruviform-test-fault=bad-fixture"],
    ))
    .unwrap();

    let diagnostics_id = instantiate_spawn_failed(
        &mut host,
        expected.id,
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

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(expected, &["--ruviform-test-fault=exit:3"]))
        .unwrap();

    instantiate_spawn_failed(&mut host, expected.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// `abort`: the child loads the fixture then aborts before `Hello` —
/// same defined convergence as `exit`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_abort_before_hello() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(expected, &["--ruviform-test-fault=abort"]))
        .unwrap();

    instantiate_spawn_failed(&mut host, expected.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// `stderr:N`: the child emits marker bytes on stderr then exits —
/// the retained diagnostics record proves the parent's stderr capture
/// works on the pre-`Hello` path.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_stderr_is_captured() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(
        expected,
        &["--ruviform-test-fault=stderr:4096"],
    ))
    .unwrap();

    let diagnostics_id = instantiate_spawn_failed(&mut host, expected.id, PRE_HELLO_DEATH);
    match host.diagnostics_query(diagnostics_id) {
        DiagnosticQuery::Found(record) => {
            assert!(
                record
                    .stderr
                    .bytes
                    .windows(b"ruviform-test-vst3-spawn-fault-stderr".len())
                    .any(|w| w == b"ruviform-test-vst3-spawn-fault-stderr"),
                "diagnostics stderr missing the spawn-fault marker"
            );
        }
        other => panic!("diagnostics record missing: {other:?}"),
    }
}

/// `hang`: the child loads the fixture then parks before `Hello` —
/// the parent's `spawn_timeout` decides `SpawnFailed{Timeout}` and
/// the wedged child is killed and reaped, never left running.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_hang_before_hello_times_out() {
    assert_fixture_present();

    let expected = expected_fault_descriptor();
    let mut spec = spec_with_args(expected, &["--ruviform-test-fault=hang"]);
    spec.spawn_timeout = Duration::from_millis(500);
    let mut host = PluginHost::new();
    host.register_out_of_process(spec).unwrap();

    let diagnostics_id = instantiate_spawn_failed(&mut host, expected.id, &[FailureCause::Timeout]);
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

    let expected = expected_fault_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(spec_with_args(expected, &["--definitely-not-a-real-flag"]))
        .unwrap();

    instantiate_spawn_failed(&mut host, expected.id, PRE_HELLO_DEATH);
    assert!(host.try_recv_notification().is_none());
}

/// Descriptor mismatch: the spec's descriptor does not field-match
/// the child's class scan — a defined `Handshake` rejection, not a
/// transport failure.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_descriptor_mismatch_is_handshake() {
    assert_fixture_present();

    let mut mismatched = expected_fault_descriptor();
    mismatched.name = "test_fault_typo";
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(mismatched, runner_program()))
        .unwrap();

    instantiate_spawn_failed(&mut host, mismatched.id, &[FailureCause::Handshake]);
    assert!(host.try_recv_notification().is_none());
}

/// `test_spawn_fail`: the class scans cleanly (descriptor field-match
/// succeeds, wire `CreateInstance` mints a dormant handle) but the
/// real ABI `createInstance` declines inside `init` — `Initialize`
/// reports the defined `PluginFailed` rejection. The pooled child
/// stays healthy: a healthy plugin on the same host still drives to
/// `Processing`.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_spawn_l1_create_instance_rejection() {
    assert_fixture_present();

    let spawn_fail = expected_spawn_fail_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(spawn_fail, runner_program()))
        .unwrap();
    host.register_out_of_process(OutOfProcessSpec::new(
        expected_gain_descriptor(),
        runner_program(),
    ))
    .unwrap();

    // Wire `CreateInstance` is dormant for both bridges — the handle
    // mints; the ABI create runs at `Initialize`.
    let handle = instantiate(&mut host, spawn_fail.id);
    assert_eq!(
        host.execute(HostCommand::Initialize { handle }),
        Err(HostError::PluginFailed),
        "the listed-but-uncreatable class must decline at init"
    );
    // `Destroy` is the escape for the failed instance.
    host.execute(HostCommand::Destroy { handle }).unwrap();

    // A healthy plugin on the same host instantiates and initializes.
    let gain = instantiate(&mut host, expected_gain_id());
    host.execute(HostCommand::Initialize { handle: gain })
        .unwrap();
    host.execute(HostCommand::Destroy { handle: gain }).unwrap();
    assert!(host.try_recv_notification().is_none());
}
