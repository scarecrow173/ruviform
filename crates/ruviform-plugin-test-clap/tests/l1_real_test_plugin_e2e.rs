//! Real-test-plugin L1 end-to-end tests — this crate's own
//! `test_gain` CLAP cdylib served out-of-process by
//! `ruviform-plugin-test-clap-l1-runner` (Issue #630 L1 slice; oracle
//! `docs/test-specs/issue-630-l1-runner.md`). Structural mirror of the
//! adapter's `tests/l1_real_adapter_e2e.rs`.
//!
//! Every test is `#[ignore]`-gated: they require the fixture cdylib
//! and the `l1-runner`-feature runner binary, which only
//! `mise run test-l1-real-plugin` builds deterministically before
//! selecting `--ignored`. The file-level `#![cfg(feature = "l1-runner")]`
//! keeps the file empty — and `env!("CARGO_BIN_EXE_*")` undefined —
//! without the feature, so `cargo test` default runs never see these
//! tests. Missing artifacts are `assert!` failures, never silent
//! skips.

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{
    MediaDuration, MediaRange, MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup,
    MidiKind, MusicalTimeInfo, NoteKey, TimeBase, TransportStatus,
};
use ruviform_plugin_core::{
    Event, EventKind, EventList, FailureCause, HostError, HostInput, HostOutput, LifecycleState,
    ParamEvent, ParamId, ParamValue, ParentLease, PluginDescriptor, ProcessConfig, ProcessResult,
    TransportInfo,
};
use ruviform_plugin_host::{
    ChildExit, DiagnosticQuery, HostCommand, HostNotification, HostReply, OutOfProcessSpec,
    PluginHandle, PluginHost,
};

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}`. Local to the
/// test crate: the adapter's `fixture_lib_name()` is compile-time
/// fixed to the adapter fixture.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The descriptor the child synthesizes from a real
/// `clap_plugin_factory` scan — the `OutOfProcessSpec` oracle must
/// field-match it (`id`/`name`/`vendor`/`version`/`features`,
/// order-preserving — `Descriptor::matches`, wire.rs). The parent
/// pins the fixture's known values as a const while the child
/// computes them from the actual binary — the handshake is two
/// independent sources agreeing, not the same scan on both sides
/// (`plugin.rs` `TestGainClapPlugin::get_descriptor`; `version` is the
/// fixture's declared `"0.0.0"`).
const EXPECTED: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-gain",
    name: "test_gain",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["audio-effect", "stereo"],
};

/// The `test_instrument` descriptor pin — same handshake rule as
/// [`EXPECTED`]: the parent pins the fixture's declared values while
/// the child synthesizes them from the real `clap_entry` scan
/// (`instrument.rs` `TestInstrumentClapPlugin::get_descriptor`).
const INSTRUMENT_EXPECTED: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-instrument",
    name: "test_instrument",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["instrument", "synthesizer", "stereo"],
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
/// `[0.0, 2.0]` (non-identity), so `ParamValue::new(0.25)` reaches the
/// plugin as a `0.5` gain multiplier — the CLAP side of the spec's
/// per-standard coefficient pin.
const EXPECTED_GAIN: f32 = 0.5;

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
/// sibling of the runner executable (cargo uplifts both the bin and
/// the cdylib into `target/<profile>/`).
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

/// One stereo block through `PluginHost::process` — the fixture
/// declares a single stereo port/bus pair, so both `HostInput` and
/// `HostOutput` carry two channels. The output channel list is
/// reborrowed locally (the `l1_proxy_e2e` `out_list` precedent —
/// `PlanarBufferViewMut` needs a fresh `&'a mut [&'a mut [f32]]`).
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

/// One stereo block through `PluginHost::process` carrying a
/// transport snapshot — the instrument oracle's entry point. The
/// instrument ignores its input, so the input channels are silence.
fn host_block_transport(
    host: &mut PluginHost,
    handle: PluginHandle,
    out_left: &mut [f32],
    out_right: &mut [f32],
    events: EventList<'_>,
    transport: Option<TransportInfo>,
) -> Result<ProcessResult, HostError> {
    let in_left = [0.0_f32; FRAMES];
    let in_right = [0.0_f32; FRAMES];
    let input = [&in_left[..], &in_right[..]];
    let input_view = PlanarBufferView::new(&input[..]).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut *out_left, &mut *out_right];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    host.process(handle, &inputs, &mut outputs, events, transport)
}

/// Neutral MIDI event constructor for the instrument oracle —
/// channel-agnostic fixture, channel `0`, group `ZERO`.
fn midi_event(offset: u32, kind: MidiKind) -> Event {
    Event::new(
        offset,
        EventKind::Midi(MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind,
        }),
    )
}

fn note_on(offset: u32, key: u8, velocity: u32) -> Event {
    midi_event(
        offset,
        MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity,
        },
    )
}

fn note_off(offset: u32, key: u8) -> Event {
    midi_event(
        offset,
        MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: 0,
        },
    )
}

fn cc(offset: u32, controller: u8, value: u32) -> Event {
    midi_event(
        offset,
        MidiKind::ControlChange {
            controller: MidiController::new(controller).unwrap(),
            value,
        },
    )
}

/// Playing transport at 120 BPM / ppqn 960 — a beat is 24 000 frames
/// at 48 kHz (25 frames per tick). `position_ticks` places the next
/// `ppqn` boundary `delta` ticks ahead → click frame `25 * delta`.
fn playing_transport(position_ticks: i64) -> TransportInfo {
    TransportInfo::new(
        TransportStatus::Playing,
        MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
    )
    .with_musical(MusicalTimeInfo {
        position_ticks,
        bar_start_tick: 0,
        bpm_milli: 120_000,
        ppqn: 960,
        sig_num: 4,
        sig_den: 4,
    })
}

/// Spawns the real child for `descriptor` and drives one instance
/// to `Processing` — the instrument tests' shared setup.
fn instrument_host() -> (PluginHost, PluginHandle) {
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(INSTRUMENT_EXPECTED, runner_program()))
        .unwrap();
    let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
        panic!("unexpected reply");
    };
    assert_eq!(descriptors, vec![INSTRUMENT_EXPECTED]);
    let handle = instantiate(&mut host, INSTRUMENT_EXPECTED.id);
    drive_to_processing(&mut host, handle);
    (host, handle)
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
/// required — only defined values may ever appear.
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

/// The full L1 series over a real test-fixture `.clap` binary:
/// `Scan` → `Instantiate` (spawn + handshake — the child-synthesized
/// descriptor field-matches the parent's pin) → `Initialize` →
/// `Activate` → `StartProcessing` → N stereo blocks through shm
/// with a `ParamValue` event → `SaveState`/`RestoreState` defined
/// `Err(Unsupported)` wire replies + `OpenGui`/`CloseGui` host-side
/// `Err(Unsupported)` short-circuits → `StopProcessing`
/// → `Deactivate` → `Destroy` → clean child exit.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_full_series() {
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

    // `ParamValue` 0.25 → plain 0.5 gain (the fixture's non-identity
    // `[0.0, 2.0]` plain domain). Delivered blocks are exactly
    // `0.5 ×` an earlier input (the lookahead pipeline); underruns
    // are defined pass-through (silence/last received) and every
    // call is `Ok(CONTINUE)`.
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
    for call in 0..32_u32 {
        let value = 0.2 + 0.01 * f32::from(u8::try_from(call % 16).unwrap() + 1);
        let left = vec![value; FRAMES];
        let right = vec![-value; FRAMES];
        let input = [&left[..], &right[..]];
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block(
                &mut host,
                handle,
                &input,
                &mut out_left,
                &mut out_right,
                event_list,
            ),
            Ok(ProcessResult::CONTINUE)
        );
        if call < 2 {
            // `seq - lookahead` underflow — the first
            // `lookahead_blocks` (2) calls are defined silence, no
            // result could have completed yet (the `l1_proxy_e2e` /
            // `plugin_graph_e2e` precedent pin).
            assert!(out_left.iter().all(|&s| sample_eq(s, 0.0)));
            assert!(out_right.iter().all(|&s| sample_eq(s, 0.0)));
        } else if out_left.iter().all(|&s| sample_eq(s, 0.0))
            && out_right.iter().all(|&s| sample_eq(s, 0.0))
        {
            // Defined underrun silence — the child's result has not
            // landed inside the lookahead window.
        } else {
            assert_defined_channels(call, &inputs, &out_left, &out_right);
            delivered += 1;
        }
        inputs.push(value);
    }
    // The pipeline must actually deliver: the `l1_proxy_e2e` /
    // `plugin_graph_e2e` precedents pin `matched >= 16` of 30
    // eligible calls on exact `call−2` attribution. This test's
    // membership semantic is looser (a stale-slot replay is a
    // defined underrun output, so it also counts as delivered), and
    // a real spawned fixture may pace slower than the in-repo stub
    // worker — 8 of 30 still proves sustained delivery, not a
    // one-off block.
    assert!(
        delivered >= 8,
        "the L1 pipeline barely delivered — {delivered} non-silent calls"
    );

    // `SaveState`/`RestoreState` ride the wire and come back as
    // defined `Err(Unsupported)` replies — the bridge never calls
    // the standard's state verbs (the fixture declares no state
    // extension; no fake round-trip is claimed). `OpenGui`/
    // `CloseGui` never reach the wire at all:
    // `L1Instance::execute` short-circuits them host-side as
    // `Err(Unsupported)` (L1 GUI is deferred) — a defined rejection,
    // not a round-trip.
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
    assert_eq!(
        host.execute(HostCommand::OpenGui {
            handle,
            parent: ParentLease::new(0, 0),
        }),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        host.execute(HostCommand::CloseGui { handle }),
        Err(HostError::Unsupported)
    );

    // Clean teardown — the child exits 0 on the defined shutdown
    // path, so no `InstanceFailed` notification exists.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
    assert!(host.try_recv_notification().is_none());
}

/// `kill_child` on the real-test-plugin child drives the documented
/// crash path unchanged: `Failed` convergence, exactly one
/// `InstanceFailed` notification, defined `Err(Failed)` rejections
/// and pass-through silence afterwards, no automatic restart, then
/// `Destroy` frees the slot.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_kill_reports_failed_once() {
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

/// A `spec.program` that does not exist fails the spawn at
/// `Instantiate` with `SpawnFailed{cause: Spawn}` — the pre-Hello
/// path: no instance is minted, `Failed` is never reached, no
/// notification, no hang (the runner's missing-fixture `exit(1)`
/// converges the same way — both are pre-mint failures).
#[test]
#[ignore = "requires the `l1-runner`-feature build for the spec descriptor pin — `mise run test-l1-real-plugin` selects this test"]
fn real_test_plugin_l1_missing_runner_is_spawn_failed() {
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(
        EXPECTED,
        OsString::from("ruviform-plugin-test-clap-l1-runner-definitely-missing"),
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

// ---------------------------------------------------------------------------
// test_instrument — real-ABI MIDI/transport oracle
// ---------------------------------------------------------------------------

/// `test_instrument` note lifecycle through the real ABI: a retriggered
/// note-on every block makes every delivered block bit-identical —
/// frame 0 is the triangle's `-1` extreme at full velocity/gain/volume,
/// both channels carry the same mono sample, and the output is
/// deterministic across calls (the L1 lookahead only delays delivery).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_instrument_note_lifecycle() {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();

    let events = [note_on(0, 69, u32::MAX)];
    let list = EventList::new(&events, FRAMES_U32).unwrap();
    let mut reference: Option<Vec<f32>> = None;
    let mut delivered = 0_u32;
    for _ in 0..12_u32 {
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None),
            Ok(ProcessResult::CONTINUE)
        );
        // The pipeline writes defined silence on underrun — an
        // all-zero block is indistinguishable from silence, but a
        // sounding block always has `out[0] == -1`, so all-zero ⇒
        // underrun here.
        if out_left.iter().all(|&s| s == 0.0) {
            continue;
        }
        delivered += 1;
        // Frame 0: phase 0 → triangle −1 → −1 × vel 1 × gain 1 × vol 1.
        assert_eq!(out_left[0].to_bits(), (-1.0_f32).to_bits());
        // Dual mono — every output channel identical.
        assert_eq!(out_left, out_right);
        match &reference {
            None => reference = Some(out_left.clone()),
            Some(expected) => {
                assert_eq!(
                    &out_left, expected,
                    "delivered blocks must be deterministic"
                );
            }
        }
    }
    assert!(
        delivered >= 4,
        "the L1 pipeline barely delivered — {delivered} blocks"
    );

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
}

/// `test_instrument` release oracle: note-ons for the first calls,
/// then note-offs — the fixed 2400-frame release decays linearly and
/// the tail of the run is exact silence on every delivered block.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_instrument_note_off_releases_to_silence() {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();

    let on = [note_on(0, 69, u32::MAX)];
    let on_list = EventList::new(&on, FRAMES_U32).unwrap();
    let off = [note_off(0, 69)];
    let off_list = EventList::new(&off, FRAMES_U32).unwrap();
    let empty = EventList::empty(FRAMES_U32);
    // 8 sounding blocks, ONE note-off, then empty blocks: the fixture
    // law restarts the 2400-frame release on every NoteOff, so a
    // repeated note-off would hold the voice in release forever. The
    // release completes ~5 blocks (4096 frames) after the note-off
    // block; delivered blocks lag their input call by
    // `lookahead_blocks`, so the sounding proof only counts calls
    // whose every possible source call is on-phase (retriggers →
    // `out[0] == -1`), and the silence tail only asserts once every
    // in-flight source call is many blocks past the completed release.
    let mut sounding = 0_u32;
    for call in 0..44_u32 {
        let list = match call.cmp(&8) {
            core::cmp::Ordering::Less => on_list,
            core::cmp::Ordering::Equal => off_list,
            core::cmp::Ordering::Greater => empty,
        };
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None),
            Ok(ProcessResult::CONTINUE)
        );
        if out_left.iter().all(|&s| s == 0.0) {
            continue; // underrun or legitimately silent
        }
        // Calls 4..=8: for any lookahead in 1..=3 the source call is
        // ≤7 — unambiguously on-phase, so a delivered block retriggered
        // at frame 0.
        if (4..=8).contains(&call) {
            assert_eq!(
                out_left[0].to_bits(),
                (-1.0_f32).to_bits(),
                "call {call}: sounding block must retrigger at frame 0"
            );
            sounding += 1;
        }
        // Calls 36..=43: every possible source call is ≥28 blocks past
        // the first note-off — the release is complete, so a delivered
        // (non-underrun) block must be exact silence.
        if (36..=43).contains(&call) {
            let bad = out_left
                .iter()
                .position(|&s| s != 0.0)
                .unwrap_or(usize::MAX);
            assert!(
                bad == usize::MAX && out_right.iter().all(|&s| s == 0.0),
                "call {call}: release tail must be exact silence, first nonzero at {bad}: {:?}",
                &out_left[bad.min(FRAMES - 1)..(bad + 4).min(FRAMES)]
            );
        }
    }
    assert!(sounding >= 2, "the note never sounded through the ABI");

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
}

/// `test_instrument` controller oracle: CC7 = 0 silences voices; CC7 =
/// max restores full level. The note-on offset gates the voice start —
/// frames before the offset are silent, the offset frame itself
/// sounds (block-boundary law through the real ABI).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_instrument_cc_and_offset_gating() {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();

    // Phase 1: note-on at offset 100 + CC7 volume 0 → silence.
    let muted = [cc(0, 7, 0), note_on(100, 69, u32::MAX)];
    let muted_list = EventList::new(&muted, FRAMES_U32).unwrap();
    // Phase 2: same note-on, CC7 max → offset gating visible.
    let full = [cc(0, 7, u32::MAX), note_on(100, 69, u32::MAX)];
    let full_list = EventList::new(&full, FRAMES_U32).unwrap();

    // Phase 1 (calls 0..8): note-on at offset 100 + CC7 volume 0 →
    // silence. Phase 2 (calls 8..16): same note-on, CC7 max → offset
    // gating visible. Delivered blocks lag by `lookahead_blocks`, so
    // assertions use latency-margin windows and the transition calls
    // are left unasserted.
    let mut verified = 0_u32;
    let mut reference: Option<Vec<f32>> = None;
    for call in 0..16_u32 {
        let list = if call < 8 { muted_list } else { full_list };
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None),
            Ok(ProcessResult::CONTINUE)
        );
        // Muted window: every possible source call is muted-phase —
        // assert unconditionally (underrun zeros and the voice's
        // `-0.0` both satisfy `== 0.0`; any real signal fails).
        if (3..=7).contains(&call) {
            assert!(
                out_left.iter().all(|&s| s == 0.0) && out_right.iter().all(|&s| s == 0.0),
                "call {call}: CC7=0 must silence"
            );
            continue;
        }
        if out_left.iter().all(|&s| s == 0.0) {
            continue; // underrun or legitimately silent
        }
        if (12..=15).contains(&call) {
            // Every possible source call is full-phase. The voice is
            // stateful: retriggered at offset 100 every block, it
            // sustains into the next block's frames 0..100 — so the
            // oracle pins the retrigger frame itself (`-1` extreme)
            // and bit-identical post-offset segments across delivered
            // blocks (frames 100..511 restart from phase 0 each block).
            assert_eq!(
                out_left[100].to_bits(),
                (-1.0_f32).to_bits(),
                "call {call}: offset frame sounds"
            );
            assert_eq!(out_left, out_right);
            match &reference {
                None => reference = Some(out_left[100..].to_vec()),
                Some(expected) => assert_eq!(
                    &out_left[100..],
                    expected.as_slice(),
                    "call {call}: post-offset segment must be deterministic"
                ),
            }
            verified += 1;
        }
    }
    assert!(verified >= 2, "the full-volume phase never delivered");

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
}

/// `test_instrument` transport oracle: a playing snapshot with
/// musical timing clicks at the `ppqn` boundary inside the block —
/// `position_ticks = 956` puts the next boundary 4 ticks ahead →
/// frame `4 × 25 = 100`, impulse `0.5`. Stopped transport is silent;
/// a seek (position jump) moves the click; an active loop range does
/// not disturb the per-block derivation (the fixture holds no loop
/// state — chase/loop ride the delivered snapshot).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_instrument_transport_clicks() {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();

    let empty = EventList::empty(FRAMES_U32);
    // pos 956 → boundary at tick 960 → delta 4 ticks → frame 100.
    let click_at_100 = playing_transport(956);
    // pos 940 → delta 20 ticks → frame 500.
    let click_at_500 = playing_transport(940);
    // pos 950 → delta 10 ticks → frame 250 — deliberately distinct
    // from the other phases' click frames so a stale pass-through
    // replay of an earlier click block cannot satisfy this phase.
    let looped = playing_transport(950).with_loop_range(MediaRange::new(
        MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
        MediaDuration::new(48_000, TimeBase::sample_rate(48_000).unwrap()),
    ));
    let stopped = TransportInfo::new(
        TransportStatus::Stopped,
        MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
    )
    .with_musical(MusicalTimeInfo {
        position_ticks: 956,
        bar_start_tick: 0,
        bpm_milli: 120_000,
        ppqn: 960,
        sig_num: 4,
        sig_den: 4,
    });

    // Phases of 8 calls each: stopped → click@100 → seek click@500 →
    // looped click@250. Delivered blocks lag their source call by
    // `lookahead_blocks`, so each phase asserts only inside a window
    // where every possible source call (lookahead 1..=3) is inside
    // that phase; transition calls stay unasserted. An all-zero
    // block is skipped: it is either defined silence or a
    // pass-through replay whose `last_output` was itself silent —
    // indistinguishable, and asserting on it would flake.
    let mut verified = [0_u32; 4];
    for call in 0..32_u32 {
        let (transport, phase) = match call {
            c if c < 8 => (Some(stopped), 0_usize),
            c if c < 16 => (Some(click_at_100), 1),
            c if c < 24 => (Some(click_at_500), 2),
            _ => (Some(looped), 3),
        };
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            host_block_transport(
                &mut host,
                handle,
                &mut out_left,
                &mut out_right,
                empty,
                transport
            ),
            Ok(ProcessResult::CONTINUE)
        );
        if out_left.iter().all(|&s| s == 0.0) {
            continue; // underrun — or the stopped phase's defined silence
        }
        let (window, frame) = match phase {
            0 => (3..=8, None),
            1 => (11..=16, Some(100_usize)),
            2 => (19..=24, Some(500_usize)),
            _ => (27..=31, Some(250_usize)),
        };
        if !window.contains(&call) {
            continue; // transition zone — source call ambiguous
        }
        match frame {
            None => panic!("call {call}: stopped transport must be silent"),
            Some(frame) => {
                assert_eq!(
                    out_left[frame].to_bits(),
                    0.5_f32.to_bits(),
                    "call {call}: click impulse at frame {frame}"
                );
                assert!(
                    out_left
                        .iter()
                        .enumerate()
                        .all(|(i, &s)| i == frame || s == 0.0),
                    "call {call}: click block must be silent outside frame {frame}"
                );
                assert_eq!(out_left, out_right);
                verified[phase] += 1;
            }
        }
    }
    assert!(
        verified[1..].iter().all(|&n| n >= 2),
        "click phases barely delivered — {verified:?}"
    );

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
}
