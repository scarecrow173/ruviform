//! OS-endpoint → attach → L1 `test_instrument` end-to-end verifier —
//! Issue #253 (Windows virtual-first MIDI acceptance, AC1/AC6 slice).
//!
//! This `#[ignore]`-gated, manual/opt-in test drives the *real* chain:
//!
//! ```text
//! real OS backend (`ruviform_midi_device::backend()` — `WinMM` on
//!   Windows, `DeviceError::Unsupported` elsewhere)
//!   → name-matched loopback-style port pair (a *candidate*, never
//!     proof of a virtual endpoint — see the verdict notes)
//!   → `InputTx::pump` → node `InputStage` → `attach()`/
//!     `AttachConfig::input` → `PluginNode` → typed `GraphRuntime`
//!   → out-of-process real-ABI VST3 `test_instrument` fixture
//!   → drained sink PCM + stage counters (independent oracle)
//! ```
//!
//! Fail-closed contract (mirrors `ruviform-midi-device/tests/loopback.rs`
//! and this file's sibling `l1_real_test_plugin_e2e.rs`):
//!
//! - Missing/unsupported backend, empty port lists, or zero
//!   name-matched pairs → `verdict: NotRun`, early return, no asserts.
//!   A name match is a loopback *candidate* — `MidiPortOrigin::Unknown`
//!   cannot prove a virtual endpoint by itself. The
//!   `environment_kind=virtual-os-endpoint` acceptance claim therefore
//!   relies on external provisioning provenance — the manual workflow
//!   supplies the `RvfMidi` evidence; an unverified manual run records
//!   that weaker provenance explicitly.
//! - Missing fixture artifacts → `assert!` failure, same as the L1
//!   suite (distinct from the honest `NotRun` above).
//! - A matched pair that opens but fails to deliver the scripted
//!   channel-voice sequence, or whose PCM/counter observations violate
//!   the independent oracle → `verdict: Fail` (panic).
//!
//! The stamped-offset oracle: `WinMM` stamps are millisecond ticks on
//! `TimeBase::new(1, 1_000)` in the port's declared clock domain. A
//! passive [`ObservingInput`] tap records the real `TimedMidiInput`s
//! the pump drains; the test then publishes a `ClockAnchor` that maps
//! the *observed* `NoteOn` stamp onto engine frame [`ANCHOR_FRAME`],
//! and asserts the fixture's deterministic triangle attack lands at
//! exactly that intra-block offset in the drained PCM, the release
//! completes near the stamped `NoteOff` + [`RELEASE_FRAMES`], and
//! every stage/`InputTx` counter agrees.
//!
//! Opt-in run (the canonical L1 lane does not collect this file — it
//! is selected by name):
//! `cargo test -p ruviform-plugin-test-vst3 --features l1-runner \
//!   --test os_endpoint_midi_e2e -- --ignored`

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio::typed::{Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime, RtRuntime};
use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputInfo, MidiInputPort, MidiReadStats, TimedMidiInput,
};
use ruviform_audio_core::midi_output::{
    MidiOutput, MidiOutputEnumerator, MidiOutputInfo, MidiOutputPort,
};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_plugin_bridge::{
    AttachConfig, InputConfig, InputTx, PluginNodeControl, PluginNodeFactory, attach,
};
use ruviform_audio_test::clock::{JitterSpec, VirtualClockConfig};
use ruviform_audio_test::metadata::{
    CaseRequirement, CaseResult, EnvironmentKind, RunMetadata, RunParameters, VerificationLevel,
};
use ruviform_audio_test::midi::{
    MidiCue, MidiScenario, MidiTraceConfig, MidiTraceEntry, run_midi_trace,
};
use ruviform_audio_test::oracle::{
    midi_expected_playthrough, midi_trace_eq, midi_trace_frames_monotonic,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity, NodeDef,
    NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
};
use ruviform_media_core::{
    BlockContract, ClockAnchor, ClockDomainId, ClockEpochId, DeliveryPolicy, EngineFrame,
    InBlockTransportSupport, MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup,
    MidiKind, MidiValue, NoteKey, ScheduleBudget, TimeBase, TimestampProvenance,
};
use ruviform_plugin_core::{
    BusDescriptor, BusDirection, BusRole, FailureCause, HostError, PluginDescriptor, ProcessConfig,
};
use ruviform_plugin_host::{
    DiagnosticQuery, EndpointBounds, HostCommand, HostReply, OutOfProcessSpec, PluginHandle,
    PluginHost, SpawnPhase,
};
use ruviform_rt::RtContext;

// ---------------------------------------------------------------------------
// Fixture plumbing — instrument-only mirror of `l1_real_test_plugin_e2e.rs`.
// ---------------------------------------------------------------------------

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`, resolved as a
/// sibling of the runner executable (cargo uplifts both into
/// `target/<profile>/`).
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The `test_instrument` component CID — `TestInstrumentComponent::CID`
/// in `instrument.rs` (`pub(crate)`), re-derived here through the same
/// vendored `vst3::uid` `const fn` so the hex pin stays independent of
/// the fixture's own constant.
fn instrument_cid() -> vst3::Steinberg::TUID {
    vst3::uid(0x4A7E_1C93, 0x8D5F_2B46, 0xE9A3_7C81, 0x5F2D_8B64)
}

/// The instrument descriptor the child synthesizes from the class
/// scan — same handshake rule as the sibling file's
/// `instrument_descriptor`: the parent pins the fixture's known values
/// while the child computes them from the actual binary.
fn instrument_descriptor() -> PluginDescriptor {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let cid = instrument_cid();
    let mut out = String::with_capacity(32);
    for &byte in &cid {
        let byte = byte.cast_unsigned();
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    PluginDescriptor {
        id: String::leak(out),
        name: "test_instrument",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Instrument", "Fx"],
    }
}

/// The runner executable — same package, so `CARGO_BIN_EXE_*` names it
/// directly (defined only under `--features l1-runner`, which this
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

/// The fixture cdylib the runner will dlopen — a sibling of the runner
/// executable.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-vst3-l1-runner"))
        .parent()
        .expect("the runner lives under target/<profile>/")
        .join(fixture_lib_name())
}

/// Every run asserts the fixture exists first — a missing artifact is
/// a failure, not a skip.
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
/// with `cause == Some(Timeout)` AND `phase` is one of the
/// `spawn_timeout`-bounded waits' freeze phases — `{PreSpawn,
/// Spawn, DrainInit, Handshake, Live}`, every variant the enum
/// currently declares. `Live` is eligible here — and only inside
/// this signature — because `SpawnFailed` is definitionally
/// pre-mint (the variant exists only on the spawn/create paths, so
/// a post-mint command timeout can never produce it): the phase
/// tracks *child* lifecycle and goes `Live` the instant `Hello`
/// passes, so the `Scan`/`CreateInstance` waits — themselves
/// `spawn_timeout`-bounded mint-window waits — freeze `Live`
/// records on both a fresh child and a pooled re-mint. A
/// non-`Timeout` cause or an `Evicted`/`NotFound` query result is
/// signature-missed and fails at once — never retried. Every
/// `SpawnFailed` attempt is traced to the real stderr — bypassing
/// the harness's per-test capture so the trace lands in lane logs
/// even when a later attempt recovers the test — carrying the
/// attempt index plus the record's `diagnostics_id`/`phase`/`cause`
/// (the deadline-phase attribution the `SpawnFailed` payload alone
/// cannot show — rf#841 Residual). Bound exhaustion is an honest
/// failure — the flake is never absorbed silently.
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
                                    | SpawnPhase::Live
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

/// Spawns the real child for the `test_instrument` descriptor and
/// drives one instance to `Processing`.
fn instrument_host() -> (PluginHost, PluginHandle) {
    let expected = instrument_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    (host, handle)
}

// ---------------------------------------------------------------------------
// Attach rig — mirror of `attach_integration` in the sibling file.
// ---------------------------------------------------------------------------

/// Block shape — equals the activated `max_frames`, so each
/// `rt.process` is exactly one physical plugin call.
const BLOCK: usize = 512;
const FRAMES_U32: u32 = 512;

/// The fixture declares one stereo main bus pair (audio in + audio
/// out) — the consumer-declared bus table `attach` maps onto typed
/// graph ports (L1 reports `BusReport::Unavailable`, so the declared
/// table is the only checked surface).
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

/// Engine frame the observed `NoteOn` stamp is anchored to. Chosen
/// inside block 0 so the attack lands at a non-zero intra-block
/// offset — proving stamped-offset precision, not just block
/// alignment (the scripted precedent pins the same offset-100 shape).
const ANCHOR_FRAME: u64 = 100;

/// `WinMM` stamp base: 1 tick = 1 ms.
const STAMP_BASE_TICKS: u32 = 1_000;

/// Frame ratio for the anchor math: stamp delta in ms × (48 000 /
/// 1 000) = ×48 engine frames — valid because the stamp contract
/// below pins `stamp.base == TimeBase::new(1, 1_000)` while
/// `frame_base` is `TimeBase::sample_rate(48_000)`.
const FRAMES_PER_STAMP_TICK: u64 = 48;

/// `test_instrument` release length in frames (fixture constant).
const RELEASE_FRAMES: u64 = 2_400;

/// L1 lookahead in delivered blocks: `OutOfProcessSpec` default
/// `lookahead_blocks = 2` × `max_frames = 512`. Engine block `b`
/// surfaces in drained slot `b + 2` once the pipeline is full.
const DELIVERED_LOOKAHEAD: usize = 2;

/// Upper bound on process blocks — a matched pair whose stamped span
/// exceeds this is outside the verifier's envelope and fails honestly.
const MAX_BLOCKS: u64 = 48;

// ---------------------------------------------------------------------------
// Case Schema v1 run metadata — rf#1001 semantics, lease-local copy.
// ---------------------------------------------------------------------------

const RUN_SEED: u64 = 0x253;
const FIXTURE_VERSION: u32 = 1;
const CASE_ID: &str = "rf253.midi.virtual-endpoint.host-plugin-e2e";
const ORACLE_ID: &str = "rf628-trace-supply+ordered-subsequence+stamped-pcm-counter-v1";
static TERMINAL_RECORD_EMITTED: AtomicBool = AtomicBool::new(false);

const fn run_parameters() -> RunParameters {
    RunParameters {
        seed: RUN_SEED,
        fixture_version: FIXTURE_VERSION,
        clock_config: VirtualClockConfig {
            sample_rate: 48_000,
            epoch: 0,
            offset_ticks: 0,
            drift_ppm: 0,
            jitter: JitterSpec::DISABLED,
        },
    }
}

fn repo_name() -> String {
    std::env::var("RUVIFORM_RUN_REPO").unwrap_or_else(|_| "unknown".to_owned())
}

fn repo_sha() -> String {
    std::env::var("RUVIFORM_RUN_SHA")
        .or_else(|_| std::env::var("CI_COMMIT_SHA"))
        .or_else(|_| std::env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn endpoint_record() -> RunMetadata {
    let record = RunMetadata::for_environment(
        CASE_ID,
        CaseRequirement::Required,
        VerificationLevel::ExtL3,
        EnvironmentKind::VirtualOsEndpoint,
        ORACLE_ID,
        run_parameters(),
        repo_name(),
        repo_sha(),
    )
    .with_platform(std::env::consts::OS, std::env::consts::ARCH)
    .with_expected(
        "4-event channel-voice sequence supplied by the RF#628 trace; stamped PCM attack/release, counter agreement, and close invalidation",
    )
    .with_evidence("crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs")
    .with_evidence("workflow stage os-endpoint-midi-e2e in virtual-audio-driver-probe.yml")
    .with_evidence(
        "name match is only a loopback candidate; RvfMidi provisioning provenance in workflow evidence is the basis for the virtual-os-endpoint claim",
    );
    let provenance = std::env::var("RUVIFORM_VIRTUAL_ENDPOINT_PROVENANCE")
        .unwrap_or_else(|_| "manual-unverified-loopback-candidate".to_owned());
    record.with_evidence(format!("virtual endpoint provenance: {provenance}"))
}

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
    TERMINAL_RECORD_EMITTED.store(true, Ordering::SeqCst);
}

fn emit_notrun(meta: &RunMetadata, reason: impl Into<String>) {
    let record = meta.clone().with_result(CaseResult::NotRun, reason.into());
    emit_record(&record);
}

fn fail_case(meta: &RunMetadata, reason: impl Into<String>) -> ! {
    let reason = reason.into();
    let record = meta.clone().with_result(CaseResult::Fail, reason.clone());
    emit_record(&record);
    panic!("verdict: Fail — {reason}");
}

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

/// `attach` config — identical shape to the scripted precedent,
/// including the opt-in live-`MIDI` input leg.
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

/// The attached rig: L1-hosted `test_instrument` inside a
/// Source → plugin → Sink typed graph, plus the non-RT halves the
/// driver pumps per block.
struct AttachRig {
    _host: PluginHost,
    _runtime: GraphRuntime,
    rt: RtRuntime,
    control: PluginNodeControl,
    input_tx: InputTx,
    src_out: PortId,
    sink_in: PortId,
}

/// Spawns the real child, drives `test_instrument` to `Processing`,
/// `attach`es it with the `MIDI` input leg, and compiles the graph.
fn build_rig() -> AttachRig {
    assert_fixture_present();
    let (mut host, handle) = instrument_host();
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
        _host: host,
        _runtime: runtime,
        rt,
        control: attached.control,
        input_tx,
        src_out: sp[0],
        sink_in: kp[0],
    }
}

// ---------------------------------------------------------------------------
// Graph pumping helpers (same contract as the scripted precedent).
// ---------------------------------------------------------------------------

/// Zero input into the graph's source port (the instrument ignores
/// it — the bus exists because the fixture declares a stereo pair).
fn feed_source(rt: &mut RtRuntime, port: PortId) {
    let zeros = [0.0f32; BLOCK];
    let mut slot = rt.input_writer(port, BLOCK).expect("input bound");
    for ch in 0..2 {
        let plane = slot.data.channel_mut(ch).expect("channel");
        plane[..BLOCK].copy_from_slice(&zeros);
    }
}

/// Drains the sink's input boundary slot — `frames_valid` samples per
/// channel, exactly what this callback delivered.
fn drain_sink(rt: &mut RtRuntime, port: PortId) -> Vec<Vec<f32>> {
    let view = rt.output_reader(port).expect("output bound");
    let frames = view.frames_valid as usize;
    (0..usize::from(view.channels))
        .map(|ch| view.data.channel(ch).expect("channel")[..frames].to_vec())
        .collect()
}

fn is_silent(block: &[Vec<f32>]) -> bool {
    block.iter().all(|ch| ch.iter().all(|&s| s == 0.0))
}

/// One `rt.process` step: feed silence, pace the child, process,
/// drain. First blocks get extra pacing so the L1 child keeps up —
/// the same 25/5 ms cadence as the scripted precedent (a result
/// observed after its `expected` seq is discarded as stale, so the
/// early window matters).
fn pump_block(rig: &mut AttachRig, block_index: u64) -> Vec<Vec<f32>> {
    feed_source(&mut rig.rt, rig.src_out);
    let pace_ms = if block_index < 4 { 25 } else { 5 };
    thread::sleep(Duration::from_millis(pace_ms));
    rig.rt.process(BLOCK, &RtContext::mint()).expect("process");
    drain_sink(&mut rig.rt, rig.sink_in)
}

// ---------------------------------------------------------------------------
// OS endpoint selection — same name-match rule as `tests/loopback.rs`.
// ---------------------------------------------------------------------------

/// The fixed channel-voice sequence sent through the pair and verified
/// downstream — channel 3, non-center pitch bend, values built from
/// wire width (`from_u7`/`bend_from_u14`) so encode→decode is the
/// identity the codec contract guarantees. Independent literal oracle:
/// the path under test never generates expected values.
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

fn cue(base: TimeBase, pos: i64, seq: u64, event: MidiEvent) -> MidiCue {
    MidiCue {
        position: MediaTime::new(pos, base),
        source_sequence: seq,
        event,
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

/// Test Runtime supply: compile the independent literal shape into a
/// `MidiScenario`, run it through the production scheduler trace, and
/// send the delivered trace entries. The independent literal remains
/// the receive-side oracle; this function only proves the supply is a
/// valid deterministic rendering of that shape.
fn midi_supply_list() -> (Vec<MidiTraceEntry>, [MidiEvent; 4], String) {
    const SPACING_FRAMES: i64 = 384; // 8 ms at 48 kHz, matching the stimulus pacing.
    const TOTAL_FRAMES: u64 = 2_400;

    let expected = expected_channel_voice_sequence();
    let base = frame_base();
    let scenario = MidiScenario::new(
        base,
        expected
            .iter()
            .enumerate()
            .map(|(index, event)| {
                cue(
                    base,
                    i64::try_from(index).unwrap() * SPACING_FRAMES,
                    index as u64,
                    *event,
                )
            })
            .collect(),
    );
    let trace = run_midi_trace(&scenario, &trace_config(TOTAL_FRAMES))
        .expect("scenario trace must compile and run");
    let oracle_entries = midi_expected_playthrough(base, &scenario.cues, TOTAL_FRAMES);
    assert!(
        midi_trace_frames_monotonic(trace.entries()),
        "RF#628 trace entries must be frame-monotonic"
    );
    assert!(
        midi_trace_eq(trace.entries(), &oracle_entries),
        "RF#628 trace must match the independent playthrough oracle"
    );
    let send_events: Vec<MidiEvent> = trace.entries().iter().map(|entry| entry.event).collect();
    assert_eq!(
        send_events.as_slice(),
        expected.as_slice(),
        "trace-derived send list must match the independent literal sequence"
    );
    let summary = format!(
        "trace_entries={} blocks_run={} spacing_frames={SPACING_FRAMES} total_frames={TOTAL_FRAMES} block_frames=480",
        trace.entries().len(),
        trace.blocks_run()
    );
    (trace.entries().to_vec(), expected, summary)
}

/// Pairs each output port with the first still-unclaimed input port of
/// the same `info().name` — the same bidirectional-name signature the
/// loopback verifier uses. Port ids differ per direction, so the name
/// is the only honest cross-direction key `WinMM` exposes.
fn loopback_pair_candidates(
    mut inputs: Vec<Box<dyn MidiInputPort>>,
    outputs: Vec<Box<dyn MidiOutputPort>>,
) -> Vec<(Box<dyn MidiInputPort>, Box<dyn MidiOutputPort>)> {
    let mut pairs = Vec::new();
    for out_port in outputs {
        let out_name = out_port.info().name.clone();
        if let Some(index) = inputs.iter().position(|p| p.info().name == out_name) {
            let in_port = inputs.swap_remove(index);
            pairs.push((in_port, out_port));
        }
    }
    pairs
}

/// Enumerates both directions through the production enumerators and
/// selects the first loopback pair candidate. Every `NotRun` verdict
/// is printed here; `None` means the environment cannot run the
/// verifier (honest skip — never a failure).
fn select_loopback_pair(
    meta: &RunMetadata,
) -> Option<(Box<dyn MidiInputPort>, Box<dyn MidiOutputPort>)> {
    let backend_in: Box<dyn MidiInputEnumerator> = Box::new(ruviform_midi_device::backend());
    let backend_out: Box<dyn MidiOutputEnumerator> = Box::new(ruviform_midi_device::backend());
    let inputs = match backend_in.input_ports() {
        Ok(ports) => ports,
        Err(error) => {
            let reason = format!("input enumeration unavailable ({error:?})");
            emit_notrun(meta, reason.clone());
            eprintln!("verdict: NotRun — {reason}");
            return None;
        }
    };
    let outputs = match backend_out.output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            let reason = format!("output enumeration unavailable ({error:?})");
            emit_notrun(meta, reason.clone());
            eprintln!("verdict: NotRun — {reason}");
            return None;
        }
    };
    eprintln!(
        "enumerated: {} input ports, {} output ports",
        inputs.len(),
        outputs.len()
    );

    let input_count = inputs.len();
    let output_count = outputs.len();
    let mut pairs = loopback_pair_candidates(inputs, outputs);
    if pairs.is_empty() {
        let reason = format!(
            "NOT_PROVISIONED — no bidirectional name match (inputs={input_count} outputs={output_count})"
        );
        emit_notrun(meta, reason.clone());
        eprintln!("verdict: NotRun — {reason}; provision the RvfMidi virtual loopback pair first");
        return None;
    }
    for (index, (in_port, out_port)) in pairs.iter().enumerate() {
        let in_info = in_port.info();
        let out_info = out_port.info();
        eprintln!(
            "  candidate[{index}]: in id={} name={:?} origin={:?} | \
             out id={} name={:?} origin={:?}",
            in_info.id, in_info.name, in_info.origin, out_info.id, out_info.name, out_info.origin
        );
    }
    Some(pairs.swap_remove(0))
}

// ---------------------------------------------------------------------------
// Stamp contract, anchor calibration, observed-event tap.
// ---------------------------------------------------------------------------

/// Classification of the observed stamp contract.
enum StampContract {
    Holds { tick_span: Option<u64> },
    ForeignBase(String),
    Violation(String),
}

/// Stamp contract over every observed event — declared clock domain,
/// `Carried` provenance, non-decreasing ticks, and the `WinMM`
/// millisecond `base` this verifier's frame math (`×48` ticks→frames)
/// is written for. A foreign stamp base is outside the envelope
/// (`NotRun`); domain, provenance and monotonicity violations are
/// contract failures (`Fail`). Measurements are returned rather than
/// hidden inside assertions so terminal metadata can carry them.
fn validate_stamp_contract(observed: &[TimedMidiInput], domain: ClockDomainId) -> StampContract {
    let stamp_base = TimeBase::new(1, STAMP_BASE_TICKS).unwrap();
    let foreign = observed
        .iter()
        .filter(|event| event.stamp.base != stamp_base)
        .count();
    let domain_mismatches = observed
        .iter()
        .filter(|event| event.stamp.domain != domain)
        .count();
    let provenance_violations = observed
        .iter()
        .filter(|event| event.stamp.provenance != TimestampProvenance::Carried)
        .count();
    let monotonicity_violations = observed
        .windows(2)
        .filter(|pair| pair[1].stamp.ticks < pair[0].stamp.ticks)
        .count();
    let tick_span = match (observed.first(), observed.last()) {
        (Some(first), Some(last)) => Some(last.stamp.ticks.saturating_sub(first.stamp.ticks)),
        _ => None,
    };
    let measurements = format!(
        "events={} tick_span={tick_span:?} foreign_base={foreign} domain_mismatches={domain_mismatches} provenance_violations={provenance_violations} monotonicity_violations={monotonicity_violations}",
        observed.len()
    );
    if foreign != 0 {
        return StampContract::ForeignBase(measurements);
    }
    if domain_mismatches != 0 || provenance_violations != 0 || monotonicity_violations != 0 {
        return StampContract::Violation(measurements);
    }
    StampContract::Holds { tick_span }
}

/// Ordered subsequence match: returns the observed events corresponding
/// to `expected`, in order, or `None` if the sequence has not fully
/// arrived yet. Extra traffic between matches is tolerated.
fn match_expected<'a>(
    observed: &'a [TimedMidiInput],
    expected: &[MidiEvent],
) -> Option<Vec<&'a TimedMidiInput>> {
    let mut cursor = observed.iter();
    let mut matched = Vec::with_capacity(expected.len());
    for want in expected {
        matched.push(cursor.find(|got| got.event == *want)?);
    }
    Some(matched)
}

/// Passive tap on the production `MidiInput`: delegates `read`/`close`
/// to the real subscription and records every `TimedMidiInput` each
/// read appended — the stamps the anchor is calibrated from and the
/// evidence the oracle checks. Never mutates, reorders or fabricates.
struct ObservingInput<'a> {
    inner: &'a mut dyn MidiInput,
    observed: Vec<TimedMidiInput>,
}

impl MidiInput for ObservingInput<'_> {
    fn read(
        &mut self,
        out: &mut Vec<TimedMidiInput>,
        max_events: usize,
    ) -> DeviceResult<MidiReadStats> {
        let before = out.len();
        let stats = self.inner.read(out, max_events)?;
        self.observed.extend_from_slice(&out[before..]);
        Ok(stats)
    }

    fn close(&mut self) {
        self.inner.close();
    }
}

// ---------------------------------------------------------------------------
// Verifier phases.
// ---------------------------------------------------------------------------

/// An opened, capability-gated port pair plus its declared metadata.
struct OpenedPair {
    input: Box<dyn MidiInput>,
    output: Box<dyn MidiOutput>,
    in_info: MidiInputInfo,
    out_info: MidiOutputInfo,
    domain: ClockDomainId,
}

/// Select → capability-gate → open. `None` means a `NotRun` verdict was
/// already printed (unsupported backend, no ports, no name-matched
/// pair, unstamped port); open failures are `verdict: Fail` panics.
fn open_loopback_pair(meta: &RunMetadata) -> Option<OpenedPair> {
    let (in_port, out_port) = select_loopback_pair(meta)?;
    let in_info = in_port.info();
    let out_info = out_port.info();
    let Some(domain) = in_info.capabilities.clock_domain else {
        let reason = format!(
            "matched input port declares no clock domain (capabilities: {:?})",
            in_info.capabilities
        );
        emit_notrun(meta, reason.clone());
        eprintln!("verdict: NotRun — {reason}");
        return None;
    };
    if !in_info.capabilities.provides_timestamps {
        let reason = format!(
            "matched input port does not provide timestamps (capabilities: {:?}; stamped-offset oracle requires them)",
            in_info.capabilities
        );
        emit_notrun(meta, reason.clone());
        eprintln!("verdict: NotRun — {reason}");
        return None;
    }
    // Input opens first so nothing sent below can arrive before the
    // subscription exists (loopback.rs precedent).
    let input = in_port.open().unwrap_or_else(|error| {
        fail_case(
            meta,
            format!("matched input port failed to open: {error:?}"),
        )
    });
    let output = out_port.open().unwrap_or_else(|error| {
        fail_case(
            meta,
            format!("matched output port failed to open: {error:?}"),
        )
    });
    Some(OpenedPair {
        input,
        output,
        in_info,
        out_info,
        domain,
    })
}

/// What the pump phase observed: the matched `NoteOn`/`NoteOff` stamp
/// ticks (anchor calibration inputs), the match count, and the
/// cumulative backend loss counters.
struct PumpOutcome {
    on_stamp: u64,
    note_off_stamp: u64,
    matched: usize,
    backend_dropped: u64,
    backend_unsupported: u64,
}

/// Pump `InputTx` against the observing tap until the expected
/// sequence arrives as an ordered subsequence, or fail on deadline.
fn pump_until_sequence(
    rig: &mut AttachRig,
    observing: &mut ObservingInput<'_>,
    expected: &[MidiEvent],
    meta: &RunMetadata,
) -> PumpOutcome {
    let mut backend_dropped = 0u64;
    let mut backend_unsupported = 0u64;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let stats = rig
            .input_tx
            .pump(observing)
            .expect("input pump through OS endpoint");
        backend_dropped += stats.backend_dropped;
        backend_unsupported += stats.backend_unsupported;
        if let Some(matched) = match_expected(&observing.observed, expected) {
            return PumpOutcome {
                on_stamp: matched[0].stamp.ticks,
                note_off_stamp: matched[3].stamp.ticks,
                matched: matched.len(),
                backend_dropped,
                backend_unsupported,
            };
        }
        if Instant::now() >= deadline {
            fail_case(
                meta,
                format!(
                    "expected sequence did not arrive through the matched OS pair (observed {} events: {:?})",
                    observing.observed.len(),
                    observing.observed
                ),
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Run `blocks_needed` graph blocks and collect every drained sink
/// PCM block.
fn process_blocks(rig: &mut AttachRig, blocks_needed: u64) -> Vec<Vec<Vec<f32>>> {
    let mut drained = Vec::with_capacity(usize::try_from(blocks_needed).unwrap());
    for i in 0..blocks_needed {
        drained.push(pump_block(rig, i));
    }
    drained
}

/// The PCM half of the oracle — see the module docs for the chain
/// being pinned. `f_off` is the stamped `NoteOff`'s expected engine
/// frame.
fn assert_pcm_oracle(drained: &[Vec<Vec<f32>>], f_off: u64) {
    // Engine block 0 carries the `NoteOn` at offset `ANCHOR_FRAME`; the
    // L1 lookahead surfaces it at drained index `DELIVERED_LOOKAHEAD`.
    // A missing/late block-0 result is *discarded* as stale by the
    // endpoint — it can never reappear at a later index, so the first
    // sounding block must be exactly the lookahead slot and must show
    // the stamped-offset attack shape.
    let first_sound = drained
        .iter()
        .position(|b| !is_silent(b))
        .expect("verdict: Fail — the note-on never sounded through the OS endpoint path");
    assert_eq!(
        first_sound, DELIVERED_LOOKAHEAD,
        "engine block 0 output must surface at drained index \
         {DELIVERED_LOOKAHEAD} (lookahead_blocks=2 × max_frames=512)"
    );
    let attack = &drained[first_sound];
    let anchor_off = usize::try_from(ANCHOR_FRAME).unwrap();
    let (left, right) = (&attack[0], &attack[1]);
    assert!(
        left[..anchor_off].iter().all(|&s| s == 0.0),
        "voice must not start before the stamped intra-block offset — \
         first samples: {:?}",
        &left[..left.len().min(anchor_off + 4)]
    );
    // Fixture voice frame 0: triangle −1 × velocity × envelope 1 ×
    // gain 1 × volume 1 × tremolo 1. `velocity` is the canonical `u32`
    // the codec round-trips (`from_u7`→wire u7→`from_u7` is identity
    // for u7-origin values), normalized by the fixture as
    // `v as f32 / 4_294_967_295.0` — the same expression, bit-exact.
    #[expect(clippy::cast_precision_loss)]
    let expected_amp = -(MidiValue::from_u7(100) as f32 / 4_294_967_295.0);
    assert_eq!(
        left[anchor_off].to_bits(),
        expected_amp.to_bits(),
        "voice frame 0 must land exactly at the stamped offset"
    );
    assert_eq!(left, right, "test_instrument output must be dual mono");

    // Release oracle: the last non-zero sample maps back through
    // `drained[i] ↔ engine block i − DELIVERED_LOOKAHEAD` to an engine
    // frame inside [f_off, f_off + RELEASE_FRAMES + 1 block). A lost
    // `NoteOff` leaves the voice sustaining to the run's end (far past
    // the bound); an early release violates the lower bound.
    let mut last_nonzero_engine: Option<u64> = None;
    for (i, block) in drained.iter().enumerate().skip(DELIVERED_LOOKAHEAD) {
        for ch in block {
            for (s, &v) in ch.iter().enumerate() {
                if v != 0.0 {
                    last_nonzero_engine =
                        Some((i as u64 - DELIVERED_LOOKAHEAD as u64) * BLOCK as u64 + s as u64);
                }
            }
        }
    }
    let last_nonzero = last_nonzero_engine.expect("voice must sound after attack");
    assert!(
        last_nonzero >= f_off,
        "release cannot complete before the stamped note-off frame \
         (last non-zero at engine frame {last_nonzero}, note-off at {f_off})"
    );
    let release_bound = f_off + RELEASE_FRAMES + BLOCK as u64;
    assert!(
        last_nonzero < release_bound,
        "release must complete within ~1 block of stamped note-off + \
         RELEASE_FRAMES (last non-zero at {last_nonzero}, bound {release_bound})"
    );
    for b in drained.iter().rev().take(3) {
        assert!(is_silent(b), "tail blocks must be silent after release");
    }
}

/// The counter half of the oracle — stage snapshot plus `InputTx`
/// transparency, all against independently computed expectations.
fn assert_counter_oracle(
    rig: &AttachRig,
    observed: &[TimedMidiInput],
    outcome: &PumpOutcome,
    expected_len: usize,
) {
    let input = rig.control.snapshot().input;
    assert_eq!(input.late, 0, "no event may arrive after its stamped block");
    assert_eq!(
        input.anchor_rejected, 0,
        "stamp domain/base/epoch must be consistent for every observed event"
    );
    assert_eq!(input.dropped_capacity, 0, "input channel must not overflow");
    assert_eq!(input.mapped, input.delivered, "mapped/delivered invariant");
    assert!(
        input.delivered >= expected_len as u64,
        "all {expected_len} expected events must reach plugin block lists \
         (delivered={})",
        input.delivered
    );
    // Stage-side backend counters stay zero on the channel path —
    // `InputStage::read` is not used; the real transparency lives on
    // `InputTx` (asserted below against the pump-observed totals).
    assert_eq!(input.dropped_backend, 0);
    assert_eq!(input.unsupported_backend, 0);
    // Every observed event with a stamp earlier than the anchor stamp
    // rejects as `BeforeAnchor` — the only defined rejection this run
    // can produce. All events are pumped before block 0, so the count
    // is exact.
    assert_eq!(rig.input_tx.deferred(), 0, "no staged events may remain");
    let pre_anchor = observed
        .iter()
        .filter(|e| e.stamp.ticks < outcome.on_stamp)
        .count() as u64;
    assert_eq!(
        input.rejected, pre_anchor,
        "only stamps earlier than the anchor may reject (BeforeAnchor)"
    );
    assert_eq!(
        rig.input_tx.dropped_backend(),
        outcome.backend_dropped,
        "InputTx backend-drop counter must equal the pump-observed total"
    );
    assert_eq!(
        rig.input_tx.unsupported_backend(),
        outcome.backend_unsupported,
        "InputTx unsupported counter must equal the pump-observed total"
    );
}

/// Close semantics on both ends of the pair — the same
/// `StreamInvalidated` contract `loopback.rs` pins.
fn assert_close_semantics(
    rig: &mut AttachRig,
    observing: &mut ObservingInput<'_>,
    output: &mut dyn MidiOutput,
    first_event: MidiEvent,
) -> (bool, bool) {
    observing.close();
    let invalidated_in = matches!(
        rig.input_tx.pump(observing),
        Err(DeviceError::StreamInvalidated)
    );
    assert!(
        invalidated_in,
        "pump after input close must report StreamInvalidated"
    );
    output.close();
    let invalidated_out = matches!(
        output.send(first_event),
        Err(DeviceError::StreamInvalidated)
    );
    assert!(
        invalidated_out,
        "send after output close must report StreamInvalidated"
    );
    (invalidated_in, invalidated_out)
}

/// OS-endpoint → attach → L1 `test_instrument` e2e — the Issue #253
/// AC1/AC6 synthetic-path verifier.
///
/// Manual opt-in: `cargo test -p ruviform-plugin-test-vst3
/// --features l1-runner --test os_endpoint_midi_e2e -- --ignored`.
/// Honest `NotRun` without a provisioned loopback-style pair or a
/// stamped-capable input port; a matched pair that fails the chain is
/// a real `Fail`.
#[test]
#[ignore = "manual opt-in: requires the built test_instrument fixture \
            (cargo build -p ruviform-plugin-test-vst3 --features l1-runner) \
            and a provisioned loopback-style MIDI port pair; \
            verdict is fail-closed NotRun without a pair"]
fn os_endpoint_midi_loopback_drives_test_instrument() {
    TERMINAL_RECORD_EMITTED.store(false, Ordering::SeqCst);
    let meta = endpoint_record();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_os_endpoint_midi_case(meta.clone());
    }));
    if let Err(payload) = outcome {
        if !TERMINAL_RECORD_EMITTED.load(Ordering::SeqCst) {
            let detail = panic_payload_text(&*payload);
            let record = meta.with_result(
                CaseResult::Fail,
                format!("harness failure before terminal verdict: {detail}"),
            );
            emit_record(&record);
        }
        std::panic::resume_unwind(payload);
    }
}

fn panic_payload_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_owned();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "non-string panic payload".to_owned()
}

fn send_trace_events(output: &mut dyn MidiOutput, entries: &[MidiTraceEntry], meta: &RunMetadata) {
    let mut previous_frame = entries.first().map_or(0, |entry| entry.frame.frame());
    for entry in entries {
        let frame = entry.frame.frame();
        let delta_frames = frame.saturating_sub(previous_frame);
        if delta_frames > 0 {
            thread::sleep(Duration::from_micros(delta_frames * 1_000_000 / 48_000));
        }
        if let Err(error) = output.send(entry.event) {
            fail_case(
                meta,
                format!("send through matched output port failed: {error:?}"),
            );
        }
        previous_frame = frame;
    }
}

fn drain_stale_input(input: &mut dyn MidiInput) {
    let mut scratch = Vec::new();
    for _ in 0..10 {
        let _ = input.read(&mut scratch, 256);
        scratch.clear();
        thread::sleep(Duration::from_millis(20));
    }
}

fn stamped_frame_bounds(outcome: &PumpOutcome, meta: &RunMetadata) -> (u64, u64) {
    let f_off = ANCHOR_FRAME + (outcome.note_off_stamp - outcome.on_stamp) * FRAMES_PER_STAMP_TICK;
    let release_end = f_off + RELEASE_FRAMES;
    let blocks_needed = (release_end / BLOCK as u64 + DELIVERED_LOOKAHEAD as u64 + 5).max(18);
    if blocks_needed > MAX_BLOCKS {
        fail_case(
            meta,
            format!(
                "stamped span implies {blocks_needed} blocks, beyond the {MAX_BLOCKS}-block verifier envelope (note-off at engine frame {f_off})"
            ),
        );
    }
    (f_off, blocks_needed)
}

fn publish_clock_anchor(
    rig: &mut AttachRig,
    domain: ClockDomainId,
    outcome: &PumpOutcome,
    meta: &RunMetadata,
) {
    if let Err(error) = rig.control.set_clock_anchor(Some(ClockAnchor {
        domain,
        stamp_ticks: outcome.on_stamp,
        stamp_base: TimeBase::new(1, STAMP_BASE_TICKS).unwrap(),
        frame: EngineFrame::new(ClockEpochId::new(0), ANCHOR_FRAME),
        calibration: TimestampProvenance::Estimated,
    })) {
        fail_case(meta, format!("publish clock anchor failed: {error:?}"));
    }
}

fn run_os_endpoint_midi_case(meta: RunMetadata) {
    assert_fixture_present();

    let Some(mut pair) = open_loopback_pair(&meta) else {
        return;
    };

    let mut rig = build_rig();

    drain_stale_input(&mut *pair.input);

    let mut observing = ObservingInput {
        inner: &mut *pair.input,
        observed: Vec::new(),
    };

    // RF#628 trace supplies the send list; the independent literal remains the oracle.
    let (send_entries, expected, supply_summary) = midi_supply_list();
    send_trace_events(&mut *pair.output, &send_entries, &meta);

    let outcome = pump_until_sequence(&mut rig, &mut observing, &expected, &meta);

    let stamp_tick_span = match validate_stamp_contract(&observing.observed, pair.domain) {
        StampContract::Holds { tick_span } => tick_span,
        StampContract::ForeignBase(measurements) => {
            let reason = format!(
                "stamp base is not the `WinMM` 1/1000 s domain this verifier anchors against; {measurements}"
            );
            emit_notrun(&meta, reason.clone());
            eprintln!("verdict: NotRun — {reason}");
            return;
        }
        StampContract::Violation(measurements) => {
            fail_case(&meta, format!("stamp contract violation: {measurements}"));
        }
    };

    publish_clock_anchor(&mut rig, pair.domain, &outcome, &meta);

    let (f_off, blocks_needed) = stamped_frame_bounds(&outcome, &meta);

    let drained = process_blocks(&mut rig, blocks_needed);
    assert_pcm_oracle(&drained, f_off);
    assert_counter_oracle(&rig, &observing.observed, &outcome, expected.len());
    let (invalidated_in, invalidated_out) =
        assert_close_semantics(&mut rig, &mut observing, &mut *pair.output, expected[0]);

    let input = rig.control.snapshot().input;
    let note_tick_span = outcome.note_off_stamp - outcome.on_stamp;
    let capability = format!(
        "provides_timestamps={} clock_domain={:?} stamp_base=1/{STAMP_BASE_TICKS} provenance=Carried monotonic=true",
        pair.in_info.capabilities.provides_timestamps, pair.in_info.capabilities.clock_domain,
    );
    let observed = format!(
        "sent={} observed={} matched={} mapped={} delivered={} rejected={} late={} dropped_backend={} unsupported_backend={} backend_dropped={} backend_unsupported={} tick_span={stamp_tick_span:?} note_tick_span={note_tick_span} blocks_needed={blocks_needed} invalidated_in={invalidated_in} invalidated_out={invalidated_out} {capability} {supply_summary}",
        send_entries.len(),
        observing.observed.len(),
        outcome.matched,
        input.mapped,
        input.delivered,
        input.rejected,
        input.late,
        input.dropped_backend,
        input.unsupported_backend,
        outcome.backend_dropped,
        outcome.backend_unsupported,
    );
    let record = meta
        .with_result(CaseResult::Pass, observed)
        .with_evidence(format!(
            "pair candidate: in={:?} out={:?}; workflow RvfMidi provisioning provenance is required for acceptance",
            pair.in_info.name, pair.out_info.name
        ));
    emit_record(&record);

    eprintln!("verdict: Pass — OS endpoint → attach → L1 test_instrument");
    eprintln!(
        "  pair: in id={} name={:?} origin={:?} | out id={} name={:?} origin={:?}",
        pair.in_info.id,
        pair.in_info.name,
        pair.in_info.origin,
        pair.out_info.id,
        pair.out_info.name,
        pair.out_info.origin
    );
    eprintln!(
        "  events: sent={} observed={} matched={} | mapped={} delivered={} \
         rejected={} late={} dropped_backend={} unsupported_backend={}",
        send_entries.len(),
        observing.observed.len(),
        outcome.matched,
        input.mapped,
        input.delivered,
        input.rejected,
        input.late,
        input.dropped_backend,
        input.unsupported_backend,
    );
    eprintln!(
        "  evidence note: the name-matched pair is a loopback candidate, \
         not proof of a virtual endpoint (`MidiPortOrigin` is \
         backend-reported). The `virtual-os-endpoint` acceptance claim \
         relies on the workflow's RvfMidi provisioning provenance; \
         physical-device specifics are unguaranteed"
    );
}
