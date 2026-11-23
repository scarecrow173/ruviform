//! Real-adapter L1 end-to-end tests — the workspace-built CLAP
//! fixture cdylib served out-of-process by
//! `ruviform-plugin-clap-l1-runner` (Issue #114, task
//! `docs/evidence/plugin-l1-evidence.md`; oracle
//! `docs/test-specs/real-adapter-l1-e2e-tests.md`).
//!
//! Every test is `#[ignore]`-gated: they require the fixture cdylib
//! and the `host`-feature runner binary, which only
//! `mise run test-l1-real-plugin` builds deterministically before
//! selecting `--ignored`. The file-level `#![cfg(feature = "host")]`
//! keeps the file empty — and `env!("CARGO_BIN_EXE_*")` undefined —
//! without the feature, so `cargo test` default runs never see these
//! tests. Missing artifacts are `assert!` failures, never silent
//! skips.

#![cfg(feature = "host")]

use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_clap::host::fixture_lib_name;
use ruviform_plugin_core::{
    Event, EventKind, EventList, FailureCause, GuiSize, HostError, HostInput, HostOutput,
    LifecycleState, ParamEvent, ParamId, ParamValue, ParentLease, PluginDescriptor, ProcessConfig,
    ProcessResult, wire,
};
use ruviform_plugin_host::{
    ChildExit, DiagnosticQuery, HostCommand, HostNotification, HostReply, OutOfProcessSpec,
    PluginHandle, PluginHost,
};

/// The descriptor the child synthesizes from a real
/// `clap_plugin_factory` scan — the `OutOfProcessSpec` oracle must
/// field-match it (`id`/`name`/`vendor`/`version`/`features`,
/// order-preserving — `Descriptor::matches`, wire.rs). The parent
/// pins the fixture's known values as a const while the child
/// computes them from the actual binary — the handshake is two
/// independent sources agreeing, not the same scan on both sides
/// (`plugin.rs` `GainClapPlugin::get_descriptor`; `version` is the
/// crate's `env!("CARGO_PKG_VERSION")`).
const EXPECTED: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.clap-gain-poc",
    name: "Ruviform Gain PoC",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &["audio-effect", "stereo"],
};

/// How long tests poll for an asynchronous `Failed` notification —
/// crash convergence needs at most a driver poll plus scheduling;
/// 10 s is generous headroom for loaded CI hosts (the
/// `l1_proxy_e2e` precedent's bound).
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// Block shape for every test — 512 max frames, so the defined L1
/// latency is `lookahead_blocks` (2) × 512 = 1024 frames.
const FRAMES: usize = 512;
const FRAMES_U32: u32 = 512;

/// The fixture's normalized→plain law: `GAIN_PARAM` is plain
/// `[0.0, 2.0]`, so `ParamValue::new(0.25)` reaches the plugin as a
/// `0.5` gain multiplier (CLAP side of the spec's per-standard
/// coefficient pin).
const EXPECTED_GAIN: f32 = 0.5;

/// The runner executable — same package, so `CARGO_BIN_EXE_*` names
/// it directly (defined only under `--features host`, which this
/// file's `#![cfg]` guarantees). A missing binary is an `assert!`
/// failure, never a silent skip.
fn runner_program() -> OsString {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-clap-l1-runner"));
    assert!(
        path.is_file(),
        "L1 runner missing at {} — `mise run test-l1-real-plugin` builds it",
        path.display()
    );
    path.into_os_string()
}

/// The fixture cdylib the runner will dlopen — resolved the way the
/// runner resolves it with `RUVIFORM_L1_REAL_PLUGIN_DIR` unset: a
/// sibling of the runner executable (cargo uplifts both the bin and
/// the cdylib into `target/<profile>/`).
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-clap-l1-runner"))
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
         `cargo build --features host`",
        path.display()
    );
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap()
}

/// Bit-exact sample comparison — the shm path round-trips `f32` bit
/// patterns through `to_bits`/`from_bits` and the fixture gain (×0.5)
/// is exactly representable, so defined values compare equal
/// bit-for-bit (no epsilon needed or wanted; `l1_proxy_e2e`
/// precedent).
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    match host.execute(HostCommand::Instantiate {
        plugin_id: plugin_id.to_owned(),
    }) {
        Ok(HostReply::Instantiated { handle }) => handle,
        // A pre-mint failure carries a retained diagnostics record —
        // print it so CI logs show the frozen phase/cause/stderr at
        // the failure point (Issue #207 diagnosis aid; harmless on
        // the success path since this arm never runs there).
        Err(HostError::SpawnFailed {
            cause,
            diagnostics_id,
        }) => {
            eprintln!("SpawnFailed: cause={cause:?} diagnostics_id={diagnostics_id}");
            match host.diagnostics_query(diagnostics_id) {
                DiagnosticQuery::Found(record) => {
                    eprintln!("diagnostics record: {record:#?}");
                    eprintln!(
                        "child stderr (lossy): {}",
                        String::from_utf8_lossy(&record.stderr.bytes)
                    );
                }
                other => eprintln!("diagnostics query result: {other:?}"),
            }
            panic!("instantiate failed: SpawnFailed {cause:?}");
        }
        Ok(other) => panic!("unexpected reply: {other:?}"),
        Err(error) => panic!("instantiate failed: {error:?}"),
    }
}

/// Per-call counter deltas recorded around one `process` — the
/// oracle's provenance record for the delivered block.
/// `passthrough_submits` increments when the leg substitutes
/// hold-last/silence output instead of a matched child block;
/// `dropped_submits` increments when this call's input block —
/// audio plus its queued events — never reached the child at all.
/// A dropped submit also substitutes output, so `passthrough > 0`
/// marks every non-matched block and `dropped` distinguishes event
/// loss. Delivery evidence must count matched blocks only: a
/// substituted block replays `last_output`, whose samples satisfy
/// the `assert_defined_channels` membership check without the
/// child having rendered anything new.
#[derive(Clone, Copy, Default)]
struct LegDeltas {
    /// `passthrough_submits` delta: `> 0` ⇒ substituted output.
    passthrough: u64,
    /// `dropped_submits` delta: `> 0` ⇒ this call's submit dropped.
    dropped: u64,
}

/// One stereo block through `PluginHost::process` — the fixture
/// declares a single stereo port/bus pair, so both `HostInput` and
/// `HostOutput` carry two channels. The output channel list is
/// reborrowed locally (the `l1_proxy_e2e` `out_list` precedent —
/// `PlanarBufferViewMut` needs a fresh `&'a mut [&'a mut [f32]]`).
/// Returns the `ProcessResult` plus the leg's per-call counter
/// deltas so callers can classify the delivered block as matched
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

/// Delivered-block membership check — every output sample must be
/// `EXPECTED_GAIN` × an earlier input (channel sign applied). A
/// stale-slot replay is a defined underrun output, so exact
/// `call − lookahead_blocks` attribution is deliberately not
/// required — only defined values may ever appear. Callers apply
/// this to matched blocks only (`passthrough == 0`): a substituted
/// replay satisfies membership without fresh child work, which is
/// why it must be classified out before this check runs.
fn assert_defined_channels(call: u32, inputs: &[f32], out_left: &[f32], out_right: &[f32]) {
    for (channel, sign) in [(out_left, 1.0_f32), (out_right, -1.0_f32)] {
        assert!(
            channel.iter().all(|&s| inputs
                .iter()
                .any(|&v| sample_eq(s, EXPECTED_GAIN * sign * v))),
            "call {call}: undefined output sample {}",
            channel[0]
        );
    }
}

/// Polls the composite's `lifecycle_state` until `Failed`.
fn wait_host_failed(host: &PluginHost, handle: PluginHandle) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls `try_recv_notification` until the `InstanceFailed`
/// notification arrives or `NOTIFY_TIMEOUT` elapses.
fn wait_host_notification(host: &mut PluginHost) -> (PluginHandle, ChildExit) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    loop {
        if let Some(HostNotification::InstanceFailed { handle, exit, .. }) =
            host.try_recv_notification()
        {
            return (handle, exit);
        }
        assert!(
            Instant::now() < deadline,
            "InstanceFailed notification never arrived"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// The `OpenGui`/`CloseGui` refusals a GUI-less fixture instance
/// gives under the negotiated `WIRE_CAPABILITY_GUI_FLOATING` bit
/// (wire v6): a child that never advertised it — every non-Windows
/// runner today — is refused `Err(Unsupported)` host-side before
/// the wire; an advertising child carries the verb over the wire,
/// where `OpenGui` still returns `Err(Unsupported)` (the fixture's
/// `open_gui` hook declines — no GUI surface) and `CloseGui` on a
/// surfaceless instance answers the defined `Err(InvalidValue)`
/// (wire.rs `CloseGui` doc).
fn assert_gui_refusals(host: &mut PluginHost, handle: PluginHandle) {
    let gui_floating_advertised = host
        .negotiated_gui_capabilities(handle)
        .is_ok_and(|mask| mask & wire::WIRE_CAPABILITY_GUI_FLOATING != 0);
    assert_eq!(
        host.execute(HostCommand::OpenGui {
            handle,
            parent: ParentLease::new(0, 0),
            size: GuiSize {
                width: 0.0,
                height: 0.0
            },
        }),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        host.execute(HostCommand::CloseGui { handle }),
        Err(if gui_floating_advertised {
            HostError::InvalidValue
        } else {
            HostError::Unsupported
        })
    );
}

/// The full L1 series over a real `.clap` binary:
/// `Scan` → `Instantiate` (spawn + handshake — the child-synthesized
/// descriptor field-matches the parent's const pin) → `Initialize`
/// → `Activate` → `StartProcessing` → N stereo blocks through shm
/// with a `ParamValue` event → `SaveState`/`RestoreState` defined
/// `Err(Unsupported)` wire replies + `OpenGui`/`CloseGui` defined
/// refusals under the negotiated `GUI_FLOATING` gate →
/// `StopProcessing` → `Deactivate` → `Destroy` → clean child exit.
#[test]
#[ignore = "requires the workspace-built fixture cdylib and the `host`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_adapter_l1_full_series() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    // `spec.args` stays empty and `lookahead_blocks`/`command_timeout`
    // keep their `OutOfProcessSpec::new` defaults (2 blocks / 10 s) —
    // wire v2's `run` takes no argv at all (plugin id/shm key ride the
    // `CreateInstance` payload; any argument is a usage reject).
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();

    let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
        panic!("unexpected reply");
    };
    // `Scan` is parent-side — it replays the composite's registered
    // descriptor surface, so this assert pins the registration, not
    // the child's scan. The field-equality proof against the child's
    // real `clap_plugin_factory` scan rides `Instantiate`'s
    // handshake (`Descriptor::matches` against the spawned child's
    // `Scanned` wire reply) — the spawn below is where the two
    // independent sources actually have to agree.
    assert_eq!(descriptors, vec![EXPECTED]);

    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    // The defined L1 latency: lookahead_blocks (2) × activated
    // max_frames (512) — the pipeline depth the delivered-block check
    // below relies on.
    assert_eq!(host.latency_frames(handle), Some(1024));

    // `ParamValue` 0.25 → plain 0.5 gain. Delivered blocks are
    // exactly `0.5 ×` an earlier input (the lookahead pipeline);
    // underruns are defined pass-through (silence/last received) and
    // every call is `Ok(CONTINUE)`. The same `event_list` rides
    // every call, so a dropped submit re-lands the identical
    // `ParamValue` on the next call — resending is structural, no
    // bounded-resend window is needed (the fixture's gain law is a
    // plain overwrite, so resending one event is idempotent).
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.25).unwrap(),
        }),
    )];
    let event_list = EventList::new(&events, FRAMES_U32).unwrap();
    let mut inputs: Vec<f32> = Vec::new();
    let mut delivered = 0_u32;
    let mut substituted = 0_u32;
    let mut dropped_calls = 0_u32;
    for call in 0..32_u32 {
        let value = 0.2 + 0.01 * f32::from(u8::try_from(call % 16).unwrap() + 1);
        let left = vec![value; FRAMES];
        let right = vec![-value; FRAMES];
        let input = [&left[..], &right[..]];
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        let (result, deltas) = host_block(
            &mut host,
            handle,
            &input,
            &mut out_left,
            &mut out_right,
            event_list,
        );
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        if call < 2 {
            // `seq - lookahead` underflow — the first
            // `lookahead_blocks` (2) calls are defined silence, no
            // result could have completed yet (the `l1_proxy_e2e` /
            // `plugin_graph_e2e` precedent pin).
            assert!(out_left.iter().all(|&s| sample_eq(s, 0.0)));
            assert!(out_right.iter().all(|&s| sample_eq(s, 0.0)));
        } else if deltas.passthrough > 0 {
            // Substituted output — a dropped submit (`dropped > 0`)
            // or a result not yet landed replays `last_output`/
            // silence. Defined, but a stale replay satisfies
            // `assert_defined_channels`'s membership check without
            // fresh child work, so it is not delivery evidence.
            substituted += 1;
            if deltas.dropped > 0 {
                dropped_calls += 1;
            }
        } else {
            // Matched — the child's render of an earlier source call
            // reached this call. Underrun silence is already
            // classified as substituted, so an all-zero block here
            // is a child-side defect the membership check reports.
            assert_defined_channels(call, &inputs, &out_left, &out_right);
            delivered += 1;
        }
        inputs.push(value);
    }
    // The pipeline must actually deliver: the `l1_proxy_e2e` /
    // `plugin_graph_e2e` precedents pin `matched >= 16` of 30
    // eligible calls on exact `call−2` attribution. Only *matched*
    // calls count here (`passthrough == 0`) — a substituted replay
    // of `last_output` satisfies the membership check without fresh
    // child work, and counting it was the staleness gap Issue #768
    // audits. A real spawned adapter may pace slower than the
    // in-repo stub worker — 8 of 30 still proves sustained
    // delivery, not a one-off block.
    assert!(
        delivered >= 8,
        "the L1 pipeline barely delivered — {delivered} matched calls, {substituted} substituted ({dropped_calls} dropped)"
    );

    // `SaveState`/`RestoreState` ride the wire and come back as
    // defined `Err(Unsupported)` replies — the bridge never calls
    // the standard's state verbs (the fixture has no real state
    // surface; no fake round-trip is claimed).
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: vec![0xAA, 0xBB],
        }),
        Err(HostError::Unsupported)
    );
    // `OpenGui`/`CloseGui` follow the negotiated `GUI_FLOATING`
    // gate — the refusal vocabulary lives in `assert_gui_refusals`.
    assert_gui_refusals(&mut host, handle);

    // Clean teardown — the child exits 0 on the defined shutdown
    // path, so no `InstanceFailed` notification exists.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// `kill_child` on the real-adapter child drives the documented
/// crash path unchanged: `Failed` convergence, exactly one
/// `InstanceFailed` notification, defined `Err(Failed)` rejections
/// and pass-through silence afterwards, no automatic restart, then
/// `Destroy` frees the slot.
#[test]
#[ignore = "requires the workspace-built fixture cdylib and the `host`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_adapter_l1_kill_reports_failed_once() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);

    host.kill_child(handle).unwrap();
    wait_host_failed(&host, handle);

    let (notified, exit) = wait_host_notification(&mut host);
    assert_eq!(notified, handle);
    // A reaped status always carries something: Windows reports the
    // kill as an exit code, unix as a signal.
    assert!(
        exit.code.is_some() || exit.signal.is_some(),
        "notification carried no exit status"
    );
    // Exactly once — the queue is already drained.
    assert!(host.try_recv_notification().is_none());

    // Defined rejections on a `Failed` instance — and no automatic
    // restart: the state stays `Failed` and every command keeps
    // failing.
    assert_eq!(
        host.execute(HostCommand::Initialize { handle }),
        Err(HostError::Failed)
    );
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Failed)
    );
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));
    assert_eq!(
        host.execute(HostCommand::Scan),
        Ok(HostReply::Scanned(vec![EXPECTED]))
    );

    // `process` on the failed leg: `Ok(CONTINUE)` + silence — the
    // pass-through contract is unchanged.
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

/// A `spec.program` that does not exist fails the spawn at
/// `Instantiate` with `SpawnFailed{cause: Spawn}` — the pre-Hello
/// path: no instance is minted, `Failed` is never reached, no
/// notification, no hang (the runner's missing-fixture `exit(1)`
/// converges the same way — both are pre-mint failures).
#[test]
#[ignore = "requires the `host`-feature build for the spec descriptor pin — `mise run test-l1-real-plugin` selects this test"]
fn real_adapter_l1_missing_runner_is_spawn_failed() {
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(
        EXPECTED,
        OsString::from("ruviform-plugin-clap-l1-runner-definitely-missing"),
    ))
    .unwrap();

    assert!(matches!(
        host.execute(HostCommand::Instantiate {
            plugin_id: EXPECTED.id.to_owned(),
        }),
        Err(HostError::SpawnFailed {
            cause: FailureCause::Spawn,
            ..
        })
    ));
    // The route itself stays registered — only the spawn failed.
    assert_eq!(
        host.execute(HostCommand::Scan),
        Ok(HostReply::Scanned(vec![EXPECTED]))
    );
    assert!(host.try_recv_notification().is_none());
}
