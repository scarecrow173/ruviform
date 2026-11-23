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
    Event, EventKind, EventList, FailureCause, GuiSize, HostError, HostInput, HostOutput,
    LifecycleState, ParamEvent, ParamId, ParamValue, ParentLease, PluginDescriptor, ProcessConfig,
    ProcessResult, TransportInfo,
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
) -> (Result<ProcessResult, HostError>, LegDeltas) {
    let in_left = [0.0_f32; FRAMES];
    let in_right = [0.0_f32; FRAMES];
    let input = [&in_left[..], &in_right[..]];
    let input_view = PlanarBufferView::new(&input[..]).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut *out_left, &mut *out_right];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    let counters = |h: &PluginHost| {
        h.audio_counters(handle)
            .map_or((0, 0), |c| (c.passthrough_submits, c.dropped_submits))
    };
    let before = counters(host);
    let result = host.process(handle, &inputs, &mut outputs, events, transport);
    let after = counters(host);
    (
        result,
        LegDeltas {
            passthrough: after.0 - before.0,
            dropped: after.1 - before.1,
        },
    )
}

/// Per-call L1 leg counter deltas observed around one `process` —
/// the delivered-block oracle's provenance record (shared by the
/// instrument and non-instrument `host_block*` helpers).
/// `passthrough_submits` increments when the leg substitutes
/// hold-last/silence output instead of a matched child block;
/// `dropped_submits` increments when this call's input block (audio
/// plus queued events) never reached the child at all — later
/// matched outputs then reflect a child state the test's event
/// script did not produce.
#[derive(Clone, Copy, Default)]
struct LegDeltas {
    /// `passthrough_submits` delta: `> 0` ⇒ substituted output.
    passthrough: u64,
    /// `dropped_submits` delta: `> 0` ⇒ this call's submit dropped.
    dropped: u64,
}

/// `lookahead_blocks` of the instrument route — `OutOfProcessSpec::
/// new` keeps the `SpawnSpec` default of 2, so a matched block
/// delivered at call `c` is the child's rendering of source block
/// `c - L1_LOOKAHEAD`.
const L1_LOOKAHEAD: u64 = 2;

/// Arrived blocks required after a landed note-off before a matched
/// tail block is assertable as exact silence. The fixture's release
/// is 2400 frames ≈ 4.7 blocks of [`FRAMES`]; once the note-off's own
/// block has rendered, four more arrived blocks complete it and a
/// fifth guarantees the whole block is silent — 6 keeps margin.
const RELEASE_ARRIVED_BLOCKS: u64 = 6;

/// Cap on note-off resends — each resend is one extra call carrying
/// the event; if none lands within the window the release oracle
/// reports an honest failure instead of looping forever.
const NOTE_OFF_RESEND_CALLS: u32 = 8;

/// Calls per transport phase in `transport_clicks` — 4 phases of
/// [`TRANSPORT_PHASE_CALLS`] calls each.
const TRANSPORT_PHASE_CALLS: u32 = 12;

/// Blocks the child actually rendered strictly inside `(a, b]` —
/// `drops[i]` is call `i`'s `dropped_submits` delta, so the span minus
/// its drops is the arrived-block count.
fn arrived_between(drops: &[u64], a: u64, b: u64) -> u64 {
    let lo = usize::try_from(a + 1).expect("call index fits usize");
    let hi = usize::try_from(b).expect("call index fits usize");
    (b - a) - drops[lo..=hi].iter().sum::<u64>()
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
    // call is `Ok(CONTINUE)`. The same `event_list` rides every
    // call, so a dropped submit re-lands the identical `ParamValue`
    // on the next call — resending is structural, no bounded-resend
    // window is needed (the fixture's gain law is a plain
    // overwrite, so resending one event is idempotent).
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
    // audits. A real spawned fixture may pace slower than the
    // in-repo stub worker — 8 of 30 still proves sustained
    // delivery, not a one-off block.
    assert!(
        delivered >= 8,
        "the L1 pipeline barely delivered — {delivered} matched calls, {substituted} substituted ({dropped_calls} dropped)"
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
            size: GuiSize {
                width: 0.0,
                height: 0.0
            },
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

/// Issue #895: `dropped_submits` permanently attributes every drop
/// to its cause. An unpaced burst provably sheds submits on the
/// slot-busy arm (the #767 finding's persistent form), the two
/// cause counters sum back to the aggregate, and `chan_full` stays
/// at its defensive zero — a full submit channel implies every
/// slot busy while the channel capacity equals `depth`, so the
/// alternation gate intercepts first.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_test_plugin_l1_dropped_submits_attributes_causes() {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();

    // Unpaced back-to-back calls: `depth = L1_LOOKAHEAD + 1 = 3`
    // slots fill with the first three submits, and no child
    // round-trip can complete inside the microsecond-scale burst —
    // every later call drops on the slot-busy arm.
    let in_left = [0.5_f32; FRAMES];
    let in_right = [0.5_f32; FRAMES];
    let input = [&in_left[..], &in_right[..]];
    let empty = EventList::empty(FRAMES_U32);
    for call in 0..24_u32 {
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        let (result, _deltas) = host_block(
            &mut host,
            handle,
            &input,
            &mut out_left,
            &mut out_right,
            empty,
        );
        assert_eq!(result, Ok(ProcessResult::CONTINUE), "call {call}");
    }

    let counters = host.audio_counters(handle).unwrap();
    assert!(
        counters.dropped_submits > 0,
        "the unpaced burst must shed submits — {counters:?}"
    );
    assert_eq!(
        counters.dropped_submits,
        counters.dropped_submits_slot_busy + counters.dropped_submits_chan_full,
        "the aggregate is exactly its two cause counters — {counters:?}"
    );
    // Every observed drop attributes to slot-busy — the #767
    // 60-run finding as a permanent pin; `chan_full` is the
    // defensive arm that must stay zero.
    assert_eq!(
        counters.dropped_submits_slot_busy, counters.dropped_submits,
        "every observed drop attributes to slot-busy — {counters:?}"
    );
    assert_eq!(counters.dropped_submits_chan_full, 0);
    assert_eq!(counters.results_dropped, 0);
    assert_eq!(counters.results_failed_observed, 0);

    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
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
    for call in 0..12_u32 {
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        let (result, deltas) =
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None);
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        // Substituted output replays an older `last_output` block —
        // defined, but not this call's child rendering, so the
        // content asserts and the delivered count apply to matched
        // blocks only (`passthrough_submits` delta == 0).
        if deltas.passthrough > 0 {
            continue;
        }
        delivered += 1;
        // Frame 0: phase 0 → triangle −1 → −1 × vel 1 × gain 1 × vol 1.
        assert_eq!(
            out_left[0].to_bits(),
            (-1.0_f32).to_bits(),
            "call {call}: matched block must retrigger at frame 0 \
             (passthrough={} dropped={})",
            deltas.passthrough,
            deltas.dropped
        );
        // Dual mono — every output channel identical.
        assert_eq!(out_left, out_right);
        match &reference {
            None => reference = Some(out_left.clone()),
            Some(expected) => {
                assert_eq!(
                    &out_left, expected,
                    "call {call}: delivered blocks must be deterministic"
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
/// then the note-off — resent until one submit lands — then empty
/// blocks. The fixed 2400-frame release decays linearly to exact
/// silence. Substituted blocks (`passthrough_submits` delta > 0)
/// replay an older `last_output` and are defined but not assertable;
/// a matched block (`delta == 0`) is the child's rendering of source
/// call `call - L1_LOOKAHEAD` — `next_block` consumes a seq per
/// process call even when the submit drops, so seq space is call
/// space. A dropped submit loses its events with it, so a lone
/// note-off could silently never arrive and leave the voice
/// sustaining: the note-off is therefore resent — law-abiding, every
/// `NoteOff` re-arms the same release — until `dropped_submits`
/// reports a landing, bounded by `NOTE_OFF_RESEND_CALLS`; if none
/// lands the tail oracle fails honestly instead of looping.
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
    // 8 sounding blocks, then the note-off resend window (calls 8
    // through 8 + NOTE_OFF_RESEND_CALLS - 1), then empty blocks.
    // `drops[call]` records that call's `dropped_submits` delta so the
    // tail oracle can measure release progress in *arrived* blocks —
    // dropped submits stretch the release over more call seqs.
    let mut sounding = 0_u32;
    let mut tail_verified = 0_u32;
    let mut drops = [0_u64; 44];
    let mut off_pending = true;
    let mut off_landed: Option<u64> = None;
    for call in 0..44_u32 {
        let list = if call < 8 {
            on_list
        } else if off_pending && call < 8 + NOTE_OFF_RESEND_CALLS {
            off_list
        } else {
            empty
        };
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        let (result, deltas) =
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None);
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        drops[call as usize] = deltas.dropped;
        if off_pending && (8..8 + NOTE_OFF_RESEND_CALLS).contains(&call) && deltas.dropped == 0 {
            off_landed = Some(u64::from(call));
            off_pending = false;
        }
        // Calls 4..=8: every deliverable block is a retrigger — a
        // matched source is a note-on call, and a substituted replay
        // can only echo an earlier on-phase `last_output` — so any
        // non-zero block here must retrigger at frame 0.
        if (4..=8).contains(&call) && !out_left.iter().all(|&s| s == 0.0) {
            assert_eq!(
                out_left[0].to_bits(),
                (-1.0_f32).to_bits(),
                "call {call}: sounding block must retrigger at frame 0                  (passthrough={} dropped={})",
                deltas.passthrough,
                deltas.dropped
            );
            sounding += 1;
        }
        // Substituted output replays `last_output` — defined, but a
        // stale block crossing the release boundary, so no content
        // assert applies below.
        if deltas.passthrough > 0 {
            continue;
        }
        // Calls 36..=43, matched only: source call is 34..=41 — many
        // blocks past the note-off. The release is complete only in
        // rendered space: the note-off must have landed and enough
        // submits after it must have arrived for the 2400-frame
        // release to finish, so the tail asserts matched blocks with
        // `arrived_between >= RELEASE_ARRIVED_BLOCKS`.
        if (36..=43).contains(&call) {
            let Some(landed) = off_landed else {
                continue;
            };
            let Some(source) = u64::from(call).checked_sub(L1_LOOKAHEAD) else {
                continue;
            };
            if source <= landed {
                continue;
            }
            if arrived_between(&drops, landed, source) < RELEASE_ARRIVED_BLOCKS {
                continue; // release still decaying in rendered space
            }
            let bad = out_left
                .iter()
                .position(|&s| s != 0.0)
                .unwrap_or(usize::MAX);
            assert!(
                bad == usize::MAX && out_right.iter().all(|&s| s == 0.0),
                "call {call}: release tail must be exact silence,                  first nonzero at {bad}: {:?}                  (dropped={} off_landed={landed} arrived={})",
                &out_left[bad.min(FRAMES - 1)..(bad + 4).min(FRAMES)],
                deltas.dropped,
                arrived_between(&drops, landed, source)
            );
            tail_verified += 1;
        }
    }
    assert!(sounding >= 2, "the note never sounded through the ABI");
    assert!(
        tail_verified >= 2,
        "release tail never verified — note-off landed at {off_landed:?}"
    );
    // A Failed child emits defined silence that classifies as
    // "matched" (`passthrough_submits` never moves on the early
    // return), so the release-tail oracle needs the same explicit
    // liveness proof the attach-path twins carry. The settle sleep
    // covers the driver's ~200µs exit-poll convergence.
    thread::sleep(Duration::from_millis(5));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing),
        "the plugin instance must still be Processing — a Failed \
         child emits defined silence indistinguishable from \
         matched output"
    );

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
    // silence. Phase 2 (calls 8..22): same note-on, CC7 max → offset
    // gating visible. Every call resends the full event set, so a
    // dropped submit only delays convergence — matched blocks
    // (`passthrough_submits` delta == 0) render source call
    // `call - L1_LOOKAHEAD` exactly, so every matched block's phase
    // is known: muted sources must be silent, full sources must
    // retrigger at offset 100. Substituted blocks replay
    // `last_output` and are skipped for content asserts. The run is
    // extended past the old 16-call window so the `verified` gate
    // keeps a comfortable matched-sample count under substitution.
    let mut verified = 0_u32;
    let mut reference: Option<Vec<f32>> = None;
    for call in 0..22_u32 {
        let list = if call < 8 { muted_list } else { full_list };
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        let (result, deltas) =
            host_block_transport(&mut host, handle, &mut out_left, &mut out_right, list, None);
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        // Substituted output replays `last_output` — defined, but not
        // this call's child rendering, so no content assert applies.
        if deltas.passthrough > 0 {
            continue;
        }
        let Some(source) = u64::from(call).checked_sub(L1_LOOKAHEAD) else {
            continue;
        };
        if source < 8 {
            // Muted-phase source: every landed block up to it carried
            // CC7 = 0, so the child state at render time is muted —
            // the block must be exact silence (`-0.0` satisfies
            // `== 0.0`; any real signal fails).
            assert!(
                out_left.iter().all(|&s| s == 0.0) && out_right.iter().all(|&s| s == 0.0),
                "call {call} source {source}: CC7=0 must silence \
                 (dropped={})",
                deltas.dropped
            );
            continue;
        }
        // Full-phase source: the block itself carries CC7 max + the
        // offset-100 note-on, so the child state at render time is
        // unambiguous regardless of earlier drops. The voice is
        // stateful: retriggered at offset 100 every block, it
        // sustains into the next block's frames 0..100 — so the
        // oracle pins the retrigger frame itself (`-1` extreme)
        // and bit-identical post-offset segments across matched
        // blocks (frames 100..511 restart from phase 0 each block).
        assert_eq!(
            out_left[100].to_bits(),
            (-1.0_f32).to_bits(),
            "call {call} source {source}: offset frame sounds \
             (dropped={})",
            deltas.dropped
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

    // Phases of 12 calls each: stopped → click@100 → seek click@500 →
    // looped click@250. A matched block (`passthrough_submits` delta
    // == 0) is the child's rendering of source call
    // `call - L1_LOOKAHEAD` exactly, so every matched block's source
    // phase is known — each phase asserts on the source's own click
    // (or silence for the stopped phase) with no transition windows.
    // Substituted blocks replay `last_output` — under load an
    // arbitrarily old block, including a previous phase's click — so
    // they carry no content assert. The transport snapshot rides its
    // submit, so a matched block's source phase is exact; every call
    // resends the snapshot, so dropped submits only delay
    // convergence. The run is extended past the old 32-call window so
    // the `verified` gate keeps a comfortable matched-sample count
    // under substitution.
    let phase_transports = [stopped, click_at_100, click_at_500, looped];
    let click_frames = [100_usize, 500, 250]; // source phases 1..=3
    let mut verified = [0_u32; 4];
    for call in 0..4 * TRANSPORT_PHASE_CALLS {
        let transport = Some(phase_transports[(call / TRANSPORT_PHASE_CALLS) as usize]);
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        thread::sleep(Duration::from_millis(5));
        let (result, deltas) = host_block_transport(
            &mut host,
            handle,
            &mut out_left,
            &mut out_right,
            empty,
            transport,
        );
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        // Substituted output replays `last_output` — defined, but not
        // this call's child rendering, so no content assert applies.
        if deltas.passthrough > 0 {
            continue;
        }
        let Some(source) = u64::from(call).checked_sub(L1_LOOKAHEAD) else {
            continue;
        };
        let source_phase = usize::try_from(source / u64::from(TRANSPORT_PHASE_CALLS))
            .expect("phase index fits usize");
        let all_zero = out_left.iter().all(|&s| s == 0.0) && out_right.iter().all(|&s| s == 0.0);
        if source_phase == 0 {
            // Stopped-transport source: no voice was ever triggered,
            // so the matched block must be exact silence.
            assert!(
                all_zero,
                "call {call} source {source}: stopped transport must be silent \
                 (dropped={})",
                deltas.dropped
            );
            continue;
        }
        let frame = click_frames[source_phase - 1];
        // A matched playing-transport block always carries its click —
        // an all-zero block here is a child-side defect, not an
        // underrun, and fails honestly.
        assert_eq!(
            out_left[frame].to_bits(),
            0.5_f32.to_bits(),
            "call {call} source {source}: click impulse at frame {frame} \
             (dropped={})",
            deltas.dropped
        );
        assert!(
            out_left
                .iter()
                .enumerate()
                .all(|(i, &s)| i == frame || s == 0.0),
            "call {call} source {source}: click block must be silent outside frame {frame}"
        );
        assert_eq!(out_left, out_right);
        verified[source_phase] += 1;
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

// ---------------------------------------------------------------------------
// attach() runtime-integration leg — the Issue #628 deferred slice owned
// by Issue #630: the audio-test runtime's scripted MIDI backend
// (`MidiFeed` → `MockPort` → production `MidiInput`) drives the
// production `InputTx::pump` → node `InputStage` → `PluginNode` →
// `GraphRuntime` → L1 endpoint path into the real fixture ABI. This is
// the first leg that combines an out-of-process plugin, the bridge
// `attach()` input surface and the typed graph runtime — in-process
// real plugins cannot attach (`create_sendable` is `None` for the
// `!Send` bridge instances, so `prepare_endpoint` only succeeds on the
// L1 route).
// ---------------------------------------------------------------------------

mod attach_integration {
    use super::{
        FRAMES, FRAMES_U32, L1_LOOKAHEAD, LegDeltas, NOTE_OFF_RESEND_CALLS, RELEASE_ARRIVED_BLOCKS,
        arrived_between, assert_fixture_present, config, instrument_host,
    };
    use ruviform_audio::typed::{
        Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime, RtRuntime,
    };
    use ruviform_audio_core::midi_input::mock::{MockBackend, MockPort};
    use ruviform_audio_core::midi_input::{
        MidiInput, MidiInputInfo, MidiInputPort, MidiPortCapabilities, MidiPortOrigin,
    };
    use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
    use ruviform_audio_plugin_bridge::{
        AttachConfig, InputConfig, InputTx, PluginNodeControl, PluginNodeFactory, attach,
    };
    use ruviform_audio_test::backend::MidiFeed;
    use ruviform_audio_test::midi::{MidiCue, MidiScenario};
    use ruviform_graph::{
        Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity,
        NodeDef, NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
    };
    use ruviform_media_core::{
        ClockAnchor, ClockDomainId, ClockEpochId, DeliveryPolicy, EngineFrame, MediaTime,
        MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey,
        ScheduleBudget, TimeBase, TimestampProvenance,
    };
    use ruviform_plugin_core::LifecycleState;
    use ruviform_plugin_core::{BusDescriptor, BusDirection, BusRole};
    use ruviform_plugin_host::{EndpointBounds, PluginHandle, PluginHost};
    use ruviform_rt::RtContext;

    /// The fixture declares one stereo main port pair (audio in +
    /// audio out) — the consumer-declared bus table `attach` maps
    /// onto typed graph ports (L1 reports `BusReport::Unavailable`,
    /// so the declared table is the only checked surface).
    const BUSES: [BusDescriptor; 2] = [
        BusDescriptor {
            direction: BusDirection::Input,
            layout: ChannelLayout::Stereo,
            name: "In",
            role: BusRole::Main,
            enabled: true,
        },
        BusDescriptor {
            direction: BusDirection::Output,
            layout: ChannelLayout::Stereo,
            name: "Out",
            role: BusRole::Main,
            enabled: true,
        },
    ];

    /// The scripted backend's stamp domain — arbitrary but fixed; the
    /// published anchor maps it onto engine frame 0.
    const MIDI_DOMAIN: ClockDomainId = ClockDomainId::new(7);

    /// One attach leg block — equals the activated `max_frames`, so
    /// each `rt.process` is exactly one physical plugin call.
    const BLOCK: usize = FRAMES;

    fn frame_base() -> TimeBase {
        TimeBase::sample_rate(48_000).unwrap()
    }

    fn fmt() -> AudioFormat {
        AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::F32,
        }
    }

    fn port(direction: Direction, format: PortFormat) -> PortDef {
        PortDef {
            direction,
            role: ruviform_graph::PortRole::Main,
            format,
            multiplicity: Multiplicity::unbounded(0),
            capabilities: Vec::new(),
            availability: Availability::Always,
            activation: Activation::Enabled,
        }
    }

    /// `attach` config: the fixture's activated `ProcessConfig`
    /// (48 kHz, 1..=512 frames), stereo bounds covering both buses,
    /// and the opt-in live-MIDI input leg.
    fn attach_config() -> AttachConfig {
        AttachConfig {
            buses: BUSES.to_vec(),
            port_format: fmt(),
            bounds: EndpointBounds {
                max_frames: FRAMES_U32,
                channels: 2,
            },
            process_config: config(),
            frame_base: frame_base(),
            policy: DeliveryPolicy::Exact,
            budget: ScheduleBudget::default(),
            input: Some(InputConfig::new(64)),
        }
    }

    /// Identity anchor: stamp 0 in `frame_base` ticks on `MIDI_DOMAIN`
    /// == engine frame 0 of epoch 0 — the epoch `attach` mints the
    /// node's scheduler with. Cue positions are already frame ticks,
    /// so `map_stamp_to_frame` reduces to identity and the expected
    /// PCM offsets are computed independently of the mapping.
    fn anchor() -> ClockAnchor {
        ClockAnchor {
            domain: MIDI_DOMAIN,
            stamp_ticks: 0,
            stamp_base: frame_base(),
            frame: EngineFrame::new(ClockEpochId::new(0), 0),
            calibration: TimestampProvenance::Carried,
        }
    }

    /// The attached rig: L1-hosted `test_instrument` inside a
    /// Source → plugin → Sink typed graph, plus the non-RT halves the
    /// driver pumps per block.
    struct AttachRig {
        host: PluginHost,
        handle: PluginHandle,
        _runtime: GraphRuntime,
        rt: RtRuntime,
        control: PluginNodeControl,
        input_tx: InputTx,
        src_out: PortId,
        sink_in: PortId,
    }

    /// Spawns the real child, drives `test_instrument` to `Processing`,
    /// `attach`es it with the MIDI input leg, and compiles the graph.
    fn build_rig() -> AttachRig {
        assert_fixture_present();
        let (mut host, handle): (PluginHost, PluginHandle) = instrument_host();
        let cfg = attach_config();
        let attached = attach(&mut host, handle, &cfg).expect("attach");
        let input_tx = attached.input_tx.expect("input leg enabled");

        let mut b = GraphBuilder::new();
        let (src, sp) = b.add_node(&NodeDef {
            kind: NodeKind::Source,
            ports: vec![port(Direction::Output, PortFormat::Audio(fmt()))],
        });
        let (proc, pp) = b.add_node(&NodeDef {
            kind: NodeKind::Processor,
            ports: attached.ports.clone(),
        });
        let (sink, kp) = b.add_node(&NodeDef {
            kind: NodeKind::Sink,
            ports: vec![port(Direction::Input, PortFormat::Audio(fmt()))],
        });
        for (from_node, from_port, to_node, to_port) in
            [(src, sp[0], proc, pp[0]), (proc, pp[1], sink, kp[0])]
        {
            b.connect(&ConnDef {
                from: PortRef {
                    node: from_node,
                    port: from_port,
                },
                to: PortRef {
                    node: to_node,
                    port: to_port,
                },
                delay: EdgeDelay::Zero,
                channel_map: None,
            });
        }
        let g = Validator::validate(&b.build()).expect("graph validates");

        let compiler_config = CompilerConfig {
            max_frames: BLOCK,
            alignment: DEFAULT_ALIGNMENT,
            max_slots: 64,
            max_channels: 8,
            max_delay_lines: 16,
            max_delay_frames: 4096,
            max_processors: 16,
        };
        let mut factory = PluginNodeFactory::new();
        factory.insert(proc, attached.node);
        let (mut runtime, rt) = GraphRuntime::new(&compiler_config, 8, 8);
        let compiled = Compiler::new(compiler_config)
            .compile(
                &g,
                &mut factory,
                runtime.registry(),
                runtime.allocator(),
                runtime.generation() + 1,
            )
            .expect("compiles");
        runtime.publish(compiled).expect("publish");
        AttachRig {
            host,
            handle,
            _runtime: runtime,
            rt,
            control: attached.control,
            input_tx,
            src_out: sp[0],
            sink_in: kp[0],
        }
    }

    /// Scripted MIDI leg: `MidiScenario` cues → `MidiFeed` →
    /// `MockPort` subscription opened through the production
    /// `MidiPort`/`MidiInput` traits — the same surface
    /// `pipeline_midi.rs` exercises, here feeding `InputTx::pump`
    /// instead of the leg's own `InputStage`.
    struct MidiDriver {
        port: MockPort,
        input: Box<dyn MidiInput>,
        feed: MidiFeed,
    }

    impl MidiDriver {
        fn open(cues: Vec<MidiCue>) -> Self {
            let mut backend = MockBackend::new();
            let port = backend.add_port(MidiInputInfo::new(
                "attach-midi-0".to_owned(),
                "Attach MIDI".to_owned(),
                MidiPortOrigin::Internal,
                MidiPortCapabilities::stamped(false, MIDI_DOMAIN),
            ));
            let input = port.open().expect("mock port open");
            let scenario = MidiScenario::new(frame_base(), cues);
            let feed = MidiFeed::new(&scenario, MIDI_DOMAIN);
            Self { port, input, feed }
        }
    }

    fn note_on(key: u8) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(key).unwrap(),
                velocity: MidiValue::from_u7(127),
            },
        }
    }

    fn note_off(key: u8) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOff {
                key: NoteKey::new(key).unwrap(),
                velocity: 0,
            },
        }
    }

    fn cc(controller: u8, value: u8) -> MidiEvent {
        MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::ControlChange {
                controller: MidiController::new(controller).unwrap(),
                value: MidiValue::from_u7(value),
            },
        }
    }

    fn cue(ticks: i64, seq: u64, event: MidiEvent) -> MidiCue {
        MidiCue {
            position: MediaTime::new(ticks, frame_base()),
            source_sequence: seq,
            event,
        }
    }

    /// Zero input into the graph's source port (the instrument
    /// ignores it — the bus exists because the fixture declares a
    /// stereo pair).
    fn feed_source(rt: &mut RtRuntime, port: PortId) {
        let zeros = [0.0f32; BLOCK];
        let mut slot = rt.input_writer(port, BLOCK).expect("input bound");
        for ch in 0..2 {
            let plane = slot.data.channel_mut(ch).expect("channel");
            plane[..BLOCK].copy_from_slice(&zeros);
        }
    }

    /// Drains the sink's input boundary slot — `frames_valid`
    /// samples per channel, exactly what this callback delivered.
    fn drain_sink(rt: &mut RtRuntime, port: PortId) -> Vec<Vec<f32>> {
        let view = rt.output_reader(port).expect("output bound");
        let frames = view.frames_valid as usize;
        (0..usize::from(view.channels))
            .map(|ch| view.data.channel(ch).expect("channel")[..frames].to_vec())
            .collect()
    }

    /// One leg block: feed cues due inside `[cursor, cursor+BLOCK)`
    /// into the port, pump the production `InputTx` into the node's
    /// handoff channel, drive the graph, drain the sink PCM.
    ///
    /// The L1 data plane is asynchronous — `rt.process` submits a
    /// block and drains whatever results have landed (the defined
    /// `lookahead_blocks` latency plus child scheduling). The e2e
    /// precedent paces each call ~5 ms so the out-of-process child
    /// keeps up; back-to-back calls overflow the slot depth and every
    /// submit drops to pass-through silence. The first calls get a
    /// wider window: the child's first `process` round-trip can
    /// exceed the lookahead span, and a result observed after its
    /// `expected` seq is *discarded* as stale — losing call 0's
    /// output entirely rather than merely delaying it.
    fn pump_block(
        rig: &mut AttachRig,
        midi: &mut MidiDriver,
        block_index: u64,
    ) -> (Vec<Vec<f32>>, LegDeltas) {
        let upto = (block_index + 1) * BLOCK as u64 - 1;
        midi.feed.pump_into_port(&midi.port, upto);
        rig.input_tx.pump(&mut *midi.input).expect("input pump");
        feed_source(&mut rig.rt, rig.src_out);
        let pace_ms = if block_index < 4 { 25 } else { 5 };
        std::thread::sleep(std::time::Duration::from_millis(pace_ms));
        let counters = |h: &PluginHost| {
            h.audio_counters(rig.handle)
                .map_or((0, 0), |c| (c.passthrough_submits, c.dropped_submits))
        };
        let before = counters(&rig.host);
        rig.rt.process(BLOCK, &RtContext::mint()).expect("process");
        let after = counters(&rig.host);
        (
            drain_sink(&mut rig.rt, rig.sink_in),
            LegDeltas {
                passthrough: after.0 - before.0,
                dropped: after.1 - before.1,
            },
        )
    }

    /// A failed child converges the endpoint to defined silence that
    /// the `passthrough_submits` oracle reads as "matched" — the
    /// lifecycle state is the attach leg's only honest liveness
    /// proof, asserted after every run.
    fn assert_liveness(rig: &AttachRig) {
        assert_eq!(
            rig.host.lifecycle_state(rig.handle),
            Some(LifecycleState::Processing),
            "the plugin instance must still be Processing — a Failed \
             child emits defined silence indistinguishable from \
             matched output"
        );
    }

    fn is_silent(block: &[Vec<f32>]) -> bool {
        block.iter().all(|ch| ch.iter().all(|&s| s == 0.0))
    }

    /// A `NoteOn` stamped at frame 100 reaches the fixture at call
    /// offset 100 — the voice's frame 0 (`-1.0` at full velocity)
    /// lands at PCM sample 100 of the delivered block, with exact
    /// silence before it. This pins the whole chain: scenario stamp →
    /// `MidiFeed`/`MockPort` → `InputTx` → node `InputStage` anchor
    /// mapping → `PluginNode` event merge → L1 wire record → CLAP
    /// `clap_event_input` → fixture voice start → sink PCM.
    #[test]
    #[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
    fn attach_midi_note_on_lands_at_stamped_frame_offset() {
        assert_fixture_present();
        let mut rig = build_rig();
        rig.control
            .set_clock_anchor(Some(anchor()))
            .expect("anchor");
        let mut midi = MidiDriver::open(vec![cue(100, 0, note_on(69))]);

        let mut drained: Vec<Vec<Vec<f32>>> = Vec::new();
        let mut deltas = Vec::new();
        for i in 0..10_u64 {
            let (block, d) = pump_block(&mut rig, &mut midi, i);
            drained.push(block);
            deltas.push(d);
        }

        // The stamped attack is the oracle: the first sounding
        // delivered block must carry 100 silent frames then the
        // voice's frame-0 `-1.0` — a whole-block-early or -late
        // delivery, an offset-0 stamp, or a pass-through replay of
        // unrelated audio cannot produce that shape. The L1
        // lookahead only delays which drained index carries it.
        let sounding = drained
            .iter()
            .position(|b| !is_silent(b))
            .expect("the note must sound through the attach path");
        for (i, block) in drained[..sounding].iter().enumerate() {
            assert!(
                is_silent(block),
                "block {i} must be silent before first delivery"
            );
        }
        let block = &drained[sounding];
        let left = &block[0];
        let right = &block[1];
        assert!(
            left[..100].iter().all(|&s| s == 0.0),
            "voice must not start before the stamped offset — first samples: {:?}",
            &left[..left.len().min(104)]
        );
        assert_eq!(
            left[100].to_bits(),
            (-1.0_f32).to_bits(),
            "voice frame 0 = triangle -1 at full velocity/gain/volume"
        );
        assert_eq!(left, right, "dual mono");
        // The voice keeps sounding — sustained *matched* delivery,
        // not a one-off block or a `last_output` replay chain.
        let sustained = (sounding..drained.len())
            .filter(|&i| !is_silent(&drained[i]) && deltas[i].passthrough == 0)
            .count();
        assert!(
            sustained >= 2,
            "sustained matched delivery expected after the first sounding block"
        );

        // The input leg really carried the event: one stamp mapped,
        // one emitted into a delivered block list, nothing late,
        // rejected, anchor-missed, or capacity/backend-dropped.
        let input = rig.control.snapshot().input;
        assert_eq!(input.mapped, 1, "the note-on stamp must map");
        assert_eq!(input.delivered, 1, "the note-on must reach a block");
        assert_liveness(&rig);
        assert_eq!(input.late, 0);
        assert_eq!(input.rejected, 0);
        assert_eq!(input.anchor_rejected, 0);
        assert_eq!(input.dropped_capacity, 0);
        assert_eq!(input.dropped_backend, 0);
    }

    /// `NoteOff` traverses the same attach leg. A dropped L1 submit
    /// loses the block's staged events with it, so a single note-off
    /// cue could silently never reach the child and leave the voice
    /// sustaining — the fixture law re-arms the same 2400-frame
    /// release on every `NoteOff`, so the cue is resent once per
    /// block over `NOTE_OFF_RESEND_CALLS` blocks (bounded: the last
    /// *landed* resend starts the final release, and the tail oracle
    /// fails honestly if none lands). Substituted blocks
    /// (`passthrough_submits` delta > 0) replay `last_output` and are
    /// skipped; matched blocks render source call
    /// `block_index - L1_LOOKAHEAD` and the tail asserts only once
    /// enough submits after the landed note-off have arrived for the
    /// release to complete in rendered space.
    #[test]
    #[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
    fn attach_midi_note_off_releases_to_silence() {
        assert_fixture_present();
        let mut rig = build_rig();
        rig.control
            .set_clock_anchor(Some(anchor()))
            .expect("anchor");
        // note-on at frame 0 (voice frame 0 → `-1.0` at sample 0),
        // then the note-off resent at tick 1500 + 512·k — one cue per
        // block for blocks 2..=9, each at the same in-block offset.
        let mut cues = vec![cue(0, 0, note_on(69))];
        for k in 0..NOTE_OFF_RESEND_CALLS {
            cues.push(cue(
                1500 + 512 * i64::from(k),
                u64::from(k) + 1,
                note_off(69),
            ));
        }
        let mut midi = MidiDriver::open(cues);

        // 26 blocks: the resend window ends at block 9, the last
        // landed resend's release completes ~6 arrived blocks later,
        // and the tail keeps a wide matched-silence margin past the
        // 2-block L1 lookahead even with dropped submits stretching
        // the release in call space.
        let mut drained: Vec<Vec<Vec<f32>>> = Vec::new();
        let mut deltas = Vec::new();
        for i in 0..26_u64 {
            let (block, d) = pump_block(&mut rig, &mut midi, i);
            drained.push(block);
            deltas.push(d);
        }

        let sounding = drained
            .iter()
            .position(|b| !is_silent(b))
            .expect("the note must sound through the attach path");
        let first = &drained[sounding];
        assert_eq!(
            first[0][0].to_bits(),
            (-1.0_f32).to_bits(),
            "note-on at offset 0 → voice frame 0 at sample 0"
        );
        assert_eq!(first[0], first[1], "dual mono");

        // The last landed note-off resend — cue k rides block 2 + k,
        // and its events arrived iff that block's submit did not
        // drop. No landing in the whole window is an honest failure:
        // the release oracle below then verifies nothing.
        let drops: Vec<u64> = deltas.iter().map(|d| d.dropped).collect();
        let pts: Vec<u64> = deltas.iter().map(|d| d.passthrough).collect();
        let off_landed = (2..=9_u64)
            .rev()
            .find(|&j| drops[usize::try_from(j).expect("block index fits usize")] == 0)
            .expect("at least one note-off resend must land");

        // Every matched block whose source call is past the landed
        // note-off AND far enough in arrived-block space that the
        // 2400-frame release has completed must be exact silence.
        let mut tail_verified = 0_u32;
        for (idx, block) in drained.iter().enumerate() {
            let i = u64::try_from(idx).expect("block index fits u64");
            if pts[idx] > 0 {
                continue; // substituted `last_output` replay
            }
            let Some(source) = i.checked_sub(L1_LOOKAHEAD) else {
                continue;
            };
            if source <= off_landed {
                continue; // renders at/before the note-off — may sound
            }
            if arrived_between(&drops, off_landed, source) < RELEASE_ARRIVED_BLOCKS {
                continue; // release still decaying in rendered space
            }
            assert!(
                is_silent(block),
                "block {i} (source {source}) must be exact silence                  after the release completed — first samples: {:?}",
                &block[0][..block[0].len().min(8)]
            );
            tail_verified += 1;
        }
        assert!(
            tail_verified >= 2,
            "release tail never verified — last landed note-off block {off_landed}"
        );

        // Every scripted event traversed the leg cleanly — the
        // silence tail is a released voice, not an input path that
        // never delivered. `delivered` counts node-side block-list
        // emission; the L1 drop oracle above covers the last hop.
        let input = rig.control.snapshot().input;
        assert_eq!(input.mapped, 1 + u64::from(NOTE_OFF_RESEND_CALLS));
        assert_eq!(
            input.delivered,
            1 + u64::from(NOTE_OFF_RESEND_CALLS),
            "note-on + every note-off resend must reach blocks"
        );
        assert_liveness(&rig);
        assert_eq!(input.late, 0);
        assert_eq!(input.rejected, 0);
        assert_eq!(input.dropped_capacity, 0);
    }

    /// `CC7` rides the same leg: volume 0 silences an otherwise
    /// sounding voice — the controller event reaches the fixture
    /// through `InputTx` → `InputStage` → wire → `clap_event_input`.
    #[test]
    #[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
    fn attach_midi_cc7_mute_reaches_fixture() {
        assert_fixture_present();
        let mut rig = build_rig();
        rig.control
            .set_clock_anchor(Some(anchor()))
            .expect("anchor");
        // CC7 = 0 at frame 0 then note-on at frame 100 — the voice
        // must stay silent for the whole run.
        let mut midi = MidiDriver::open(vec![cue(0, 0, cc(7, 0)), cue(100, 1, note_on(69))]);

        for i in 0..10_u64 {
            let (block, _deltas) = pump_block(&mut rig, &mut midi, i);
            assert!(
                is_silent(&block),
                "block {i}: CC7=0 must silence the voice — got {:?}",
                &block[0][..block[0].len().min(8)]
            );
        }

        // Silence alone cannot distinguish "CC7 muted the voice" from
        // "nothing ever arrived" — the stage counters prove both
        // events were mapped and emitted into delivered block lists.
        let input = rig.control.snapshot().input;
        assert_eq!(input.mapped, 2, "CC + note-on must map");
        assert_eq!(input.delivered, 2, "both events must reach blocks");
        assert_liveness(&rig);
        assert_eq!(input.late, 0);
        assert_eq!(input.rejected, 0);
        assert_eq!(input.dropped_capacity, 0);
    }
}
