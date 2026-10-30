//! Public-surface E2E for the video L1 path (Issue #118 T4 —
//! `docs/tasks/l1-video-fixture-and-e2e.md`; oracles
//! `docs/test-specs/issue-118-video-e2e-tests.md` and
//! `docs/test-specs/issue-118-video-failed-parity-tests.md`; Issue
//! #121 T11 — `docs/tasks/verify-l1-video-diagnostics-parity.md`,
//! oracle `docs/test-specs/issue-121-l1-parity-nonregression-tests.md`).
//!
//! [`PluginHost::video`] is the only surface under test — the
//! `#[path]` harness `l1_proxy_e2e.rs` needs is deliberately absent:
//! this file compiles against the **public** API alone, pinning the
//! "video is a first-class facade over the unified handle space"
//! contract at the type level (the `l1_supervisor_e2e` precedent).
//!
//! Child lanes:
//!
//! * `l1-test-video-worker` — the hand-rolled wire-faithful fault
//!   fixture (`env!("CARGO_BIN_EXE_l1-test-video-worker")`, same
//!   package). Its argv matrix drives the wire-level fault cases
//!   `run_video` cannot produce (noncanonical `Rendered.output`,
//!   crash/hang/over-limit replies) plus the `Err(PluginFailed)`
//!   hook-failure family.
//! * `ruviform-plugin-l1-video-runner` — the real `run_video`
//!   runner from `ruviform-plugin-export` (a different package, so
//!   [`runner_program`] resolves it the `RUVIFORM_L1_VIDEO_RUNNER`
//!   way — the convention `l1_proxy_e2e.rs` pins for
//!   `ruviform-plugin-l1-runner`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::SampleRate;
use ruviform_media_core::{FrameRate, MediaDuration, MediaTime, TimeBase};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, FailureCause, HostError, LifecycleState, ParamId,
    ParamInfo, PlainParamValue, PluginDescriptor, PrepareError, ProcessConfig, ProcessContext,
    ProcessResult, ProcessStatus, TransportInfo, VideoProcessConfig,
};
use ruviform_plugin_host::{
    AudioCounterSnapshot, ChildExit, DiagnosticQuery, EndpointBounds, ErasedPlugin, HostCommand,
    HostNotification, HostReply, OutOfProcessSpec, Plugin, PluginFactory, PluginHandle, PluginHost,
    SpawnPhase, VideoHostCommand, VideoHostReply, VideoOutOfProcessSpec, erase,
};
use ruviform_video_core::{
    ColorInfo, ColorPrimaries, ColorRange, MatrixCoefficients, PixelFormat,
    TransferCharacteristics, VideoFrame,
};

/// The descriptor `l1-test-video-worker` reports for `Scan` — the
/// `VideoOutOfProcessSpec.descriptor` oracle must field-match it.
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-video-worker",
    name: "L1 Test Video Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &["video-effect"],
};

/// The descriptor `l1-test-worker` (audio) reports — the audio L1
/// route's registration oracle for the cross-kind suite, and the
/// contrast handle the counter-isolation test instantiates.
const AUDIO_WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// The L0 audio fixture's descriptor — a distinct audio id for the
/// cross-kind suite.
const L0_AUDIO_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l0-gain",
    name: "L0 Gain",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// The `ruviform-plugin-l1-video-runner`'s Invert fixture — the
/// real-runner lane's happy-path plugin.
const RUNNER_INVERT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.invert",
    name: "Invert",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["video-effect"],
};

/// The runner's deterministic-panic fixture — `initialize` always
/// panics (adapter-normalized to `Err(PluginFailed)`, child alive).
const RUNNER_PANIC_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.panic",
    name: "PanicVideo",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// The runner's encode-obligation-(c) fixture — `parameters()`
/// reports `WIRE_MAX_ELEMENTS + 1` ids, so its adapter answers
/// `Err(PluginFailed)` rather than emitting an unencodable list.
const RUNNER_OVERLIMIT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.overlimit",
    name: "OverLimit",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// The fixture's param ids (worker lane and runner Invert lane
/// share the `Amount`/`Mode` pair; the runner adds ids 2–4).
const PARAM_AMOUNT: ParamId = ParamId(0);
const PARAM_MODE: ParamId = ParamId(1);
/// Runner-lane only: `InvertVideo`'s commit-then-advisory
/// `deactivate` failure switch.
const RUNNER_PARAM_FAIL_DEACTIVATE: ParamId = ParamId(3);
/// Runner-lane only: `InvertVideo`'s defined `render` failure
/// switch.
const RUNNER_PARAM_FAIL_RENDER: ParamId = ParamId(4);

/// How long tests poll for an asynchronous `Failed`/`InstanceFailed`
/// — convergence needs at most a driver poll interval plus
/// scheduling; 10 s is generous headroom for loaded CI hosts.
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// Grace window for "no notification" assertions — a negative
/// answer is only meaningful after the child had time to produce
/// one.
const NO_NOTIFY_GRACE: Duration = Duration::from_millis(500);

/// Short timeout injected into the spec for the wedge paths — long
/// enough that spawn+handshake stays deterministic on a loaded
/// host (every handshake stage gets its own `command_timeout`
/// window), short enough that a mistargeted wait is observable:
/// the 2 s wedge still fires well inside the 10 s default.
const SHORT_TIMEOUT: Duration = Duration::from_secs(2);

// ------------------------------------------------------------------
// Programs, specs and fixtures
// ------------------------------------------------------------------

/// The `l1-test-video-worker` executable — same package, so
/// `CARGO_BIN_EXE_*` names it directly.
fn worker_program() -> OsString {
    OsString::from(env!("CARGO_BIN_EXE_l1-test-video-worker"))
}

/// The `l1-test-worker` (audio) executable — the cross-kind audio
/// L1 route's payload; only the counter-isolation test spawns it
/// (one live audio child as the `audio_counters` contrast oracle).
fn audio_worker_program() -> OsString {
    OsString::from(env!("CARGO_BIN_EXE_l1-test-worker"))
}

/// The `ruviform-plugin-l1-video-runner` executable — a
/// `ruviform-plugin-export` package bin, so `CARGO_BIN_EXE_*`
/// cannot name it here (cargo scopes the variable to same-package
/// bins; no cross-package compile-time mechanism exists).
/// Resolution mirrors the #114 fixture convention: a
/// `RUVIFORM_L1_VIDEO_RUNNER` env override wins verbatim, otherwise
/// the binary is located relative to this test binary — its own
/// directory first (the layout when the test exe sits directly in
/// `target/<profile>/`), then `deps/`'s parent (the usual
/// `cargo test` layout).
fn runner_program() -> OsString {
    if let Some(path) = std::env::var_os("RUVIFORM_L1_VIDEO_RUNNER") {
        return path;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let file_name = format!(
        "ruviform-plugin-l1-video-runner{}",
        std::env::consts::EXE_SUFFIX
    );
    let candidates: Vec<PathBuf> = exe
        .parent()
        .into_iter()
        .chain(exe.parent().and_then(Path::parent))
        .map(|dir| dir.join(&file_name))
        .collect();
    for path in &candidates {
        if path.is_file() {
            return path.clone().into_os_string();
        }
    }
    panic!(
        "ruviform-plugin-l1-video-runner missing — searched {}; run \
         `mise run test` (workspace test builds all member bins) \
         or set RUVIFORM_L1_VIDEO_RUNNER",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// A `VideoOutOfProcessSpec` for the hand-rolled worker with argv
/// `args` — the default timeouts (10 s / 60 s) stay unless a wedge
/// test overrides them.
fn video_spec(args: &[&str]) -> VideoOutOfProcessSpec {
    let mut spec = VideoOutOfProcessSpec::new(WORKER_DESCRIPTOR, worker_program());
    spec.args = args.iter().map(OsString::from).collect();
    spec
}

/// A `VideoOutOfProcessSpec` for the real-runner lane: `descriptor`
/// is one of the runner's fixture descriptors (`Scan` must
/// field-match one entry of the runner's multi-factory reply).
fn runner_spec(descriptor: PluginDescriptor) -> VideoOutOfProcessSpec {
    VideoOutOfProcessSpec::new(descriptor, runner_program())
}

/// A `PluginHost` carrying only the video worker route.
fn video_host(args: &[&str]) -> PluginHost {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(video_spec(args))
        .unwrap();
    host
}

/// A `PluginHost` carrying the video worker route plus both audio
/// route kinds — an L0 factory and an L1 `l1-test-worker` route —
/// for the cross-kind suite (the counter-isolation test also
/// instantiates the audio L1 route as its `Some` contrast).
fn mixed_host(args: &[&str]) -> PluginHost {
    let mut host = PluginHost::new();
    host.register_factory(Box::new(GainFactory));
    host.register_out_of_process(OutOfProcessSpec::new(
        AUDIO_WORKER_DESCRIPTOR,
        audio_worker_program(),
    ))
    .unwrap();
    host.video()
        .register_out_of_process(video_spec(args))
        .unwrap();
    host
}

/// The L0 audio fixture — an in-process audio instance the
/// cross-kind matrix points video verbs at.
struct Gain;

impl ruviform_plugin_core::PluginProcessor for Gain {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        _inputs: &[I],
        _outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
    }
}

impl Plugin for Gain {
    fn descriptor(&self) -> PluginDescriptor {
        L0_AUDIO_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }
}

struct GainFactory;

impl PluginFactory for GainFactory {
    fn descriptor(&self) -> PluginDescriptor {
        L0_AUDIO_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedPlugin> {
        erase(Gain)
    }
}

// ------------------------------------------------------------------
// Config, frame and command helpers
// ------------------------------------------------------------------

/// 64×64 Rgba8 @ 30 fps — a small deterministic frame shape.
fn config() -> VideoProcessConfig {
    VideoProcessConfig::new(PixelFormat::Rgba8, 64, 64, FrameRate::new(30, 1).unwrap())
        .expect("valid config")
}

fn audio_config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
}

/// The pinned `ColorInfo` — non-default on every field so a
/// pass-through miss is observable.
fn color() -> ColorInfo {
    ColorInfo {
        primaries: ColorPrimaries::Bt709,
        transfer: TransferCharacteristics::Bt709,
        matrix: MatrixCoefficients::Bt709,
        range: ColorRange::Full,
        hdr: None,
    }
}

/// A deterministic input frame: tightly packed plane bytes cycling
/// `0..=250` plus pinned metadata (pts/duration/color) — the pixel
/// oracle and the metadata pass-through in one object.
fn input_frame() -> VideoFrame {
    let pts = MediaTime::new(90_000, TimeBase::COMMON_90K);
    let mut frame =
        VideoFrame::new_owned(PixelFormat::Rgba8, 64, 64, pts).expect("valid frame dims");
    for (index, byte) in frame
        .plane_mut(0)
        .expect("single-plane Rgba8")
        .data_mut()
        .iter_mut()
        .enumerate()
    {
        *byte = u8::try_from(index % 251).expect("mod 251 fits u8");
    }
    frame.set_duration(Some(MediaDuration::new(3_000, TimeBase::COMMON_90K)));
    frame.set_color(color());
    frame
}

/// The `Render.at` timestamp — deliberately distinct from the
/// input's `pts` so a metadata leak (`at` landing on the output)
/// would be observable.
fn render_at() -> MediaTime {
    MediaTime::new(45_000, TimeBase::COMMON_90K)
}

fn vex(host: &mut PluginHost, command: VideoHostCommand) -> Result<VideoHostReply, HostError> {
    host.video().execute(command)
}

/// `VideoHostReply` carries no `PartialEq` — the Err-side assert is
/// the uniform comparison helper.
fn expect_err(result: Result<VideoHostReply, HostError>, expected: HostError) {
    assert_eq!(result.unwrap_err(), expected);
}

/// The `Ok(VideoHostReply::Ok)` assert — the reply enum carries no
/// `PartialEq`.
fn expect_ok(result: Result<VideoHostReply, HostError>) {
    match result {
        Ok(VideoHostReply::Ok) => {}
        other => panic!("expected Ok, got {other:?}"),
    }
}

fn instantiate(host: &mut PluginHost, plugin_id: &str, config: VideoProcessConfig) -> PluginHandle {
    match vex(
        host,
        VideoHostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
            config,
        },
    )
    .unwrap()
    {
        VideoHostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// `Initialize → Activate → StartProcessing` — the drive to
/// `Processing` every render/param test shares.
fn drive_to_processing(host: &mut PluginHost, handle: PluginHandle) {
    expect_ok(vex(host, VideoHostCommand::Initialize { handle }));
    expect_ok(vex(host, VideoHostCommand::Activate { handle }));
    expect_ok(vex(host, VideoHostCommand::StartProcessing { handle }));
}

/// One `Render` through the facade — unwraps to the output frame.
fn render(host: &mut PluginHost, handle: PluginHandle, input: VideoFrame) -> VideoFrame {
    match vex(
        host,
        VideoHostCommand::Render {
            handle,
            input,
            at: render_at(),
        },
    )
    .unwrap()
    {
        VideoHostReply::Rendered { frame } => frame,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// Polls the composite's `lifecycle_state` until `Failed`.
fn wait_failed(host: &PluginHost, handle: PluginHandle) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls `try_recv_notification` until the `InstanceFailed`
/// notification arrives or `NOTIFY_TIMEOUT` elapses — the full
/// `{handle, exit, cause}` payload so the diagnostics-parity
/// asserts can pin the first-observed `FailureCause` beside the
/// reaped status.
fn wait_notification(host: &mut PluginHost) -> (PluginHandle, ChildExit, FailureCause) {
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

/// Asserts `try_recv_notification` stays empty for the grace
/// window — a negative answer is only meaningful after the child
/// had time to produce one.
fn assert_no_notification(host: &mut PluginHost) {
    let deadline = Instant::now() + NO_NOTIFY_GRACE;
    while Instant::now() < deadline {
        assert!(
            host.try_recv_notification().is_none(),
            "unexpected failure notification"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

// ------------------------------------------------------------------
// Happy path — kind-filtered scan, lifecycle, frame transport, params
// ------------------------------------------------------------------

/// `video()` facade happy path: `Scan` lists the video route,
/// `Instantiate` mints on the `Creating` gate's
/// `CreateVideoInstance` request, the full lifecycle verb chain
/// transitions the mirror, and `Destroy` frees the slot — a clean
/// teardown that never notifies.
#[test]
fn happy_path_lifecycle_and_scan() {
    let mut host = video_host(&[]);

    match vex(&mut host, VideoHostCommand::Scan).unwrap() {
        VideoHostReply::Scanned(descriptors) => {
            assert_eq!(descriptors, vec![WORKER_DESCRIPTOR]);
        }
        other => panic!("unexpected reply: {other:?}"),
    }

    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Created));

    expect_ok(vex(&mut host, VideoHostCommand::Initialize { handle }));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Initialized)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Activate { handle }));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Activated)
    );
    expect_ok(vex(&mut host, VideoHostCommand::StartProcessing { handle }));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Activated)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Initialized)
    );

    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_eq!(host.lifecycle_state(handle), None);
    // A clean teardown never notifies — the post-removal/
    // post-`Shutdown` frames are non-events.
    assert_no_notification(&mut host);
    // A second `Destroy` on the freed slot is the defined
    // `InvalidHandle` — kind-independent.
    expect_err(
        vex(&mut host, VideoHostCommand::Destroy { handle }),
        HostError::InvalidHandle,
    );
}

/// Frame transport: the input `VideoFrame` travels the `vshm` `in`
/// region, the fixture's invert transform lands in `out`, and the
/// restored `Rendered` frame carries the input's `pts`/`duration`/
/// `color` verbatim. A `set_parameter` mode switch then flips the
/// transform to the `0x80` fill — the param surface observable in
/// the pixel path.
#[test]
fn render_content_and_metadata_passthrough() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    let input = input_frame();
    let expected_pts = input.pts();
    let expected_duration = input.duration();
    let expected_color = input.color();
    let input_bytes: Vec<u8> = input.plane(0).unwrap().data().to_vec();

    let output = render(&mut host, handle, input);
    assert_eq!(output.format(), PixelFormat::Rgba8);
    assert_eq!((output.width(), output.height()), (64, 64));
    // The invert oracle — every canonical region byte negated.
    let output_bytes = output.plane(0).unwrap().data();
    assert_eq!(output_bytes.len(), input_bytes.len());
    for (index, (src, dst)) in input_bytes.iter().zip(output_bytes.iter()).enumerate() {
        assert_eq!(*dst, !*src, "byte {index} not inverted");
    }
    // Metadata pass-through — seeded from the *input* descriptor,
    // never `at` and never the plugin's own report.
    assert_eq!(output.pts(), expected_pts);
    assert_eq!(output.duration(), expected_duration);
    assert_eq!(output.color(), expected_color);

    // `Mode = Fill` via the param channel — the next render is the
    // constant `0x80` plane, not the invert.
    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: PARAM_MODE,
            value: PlainParamValue::Choice(1),
        },
    ));
    let filled = render(&mut host, handle, input_frame());
    assert!(
        filled
            .plane(0)
            .unwrap()
            .data()
            .iter()
            .all(|byte| *byte == 0x80),
        "fill mode must produce a constant 0x80 plane"
    );
    assert_eq!(filled.pts(), expected_pts);
    assert_eq!(filled.duration(), expected_duration);
    assert_eq!(filled.color(), expected_color);

    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// The facade's pre-wire gates: lifecycle verbs outside their
/// legal source state are `Err(Lifecycle)` with the mirror
/// untouched (`Activate` from `Created`, `Render` outside
/// `Processing`), and an input frame whose format/dims disagree
/// with the instance config is `Err(InvalidValue)` — wire never
/// sent (the child answers the next verb).
#[test]
fn render_gates_and_config_match() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());

    // `Activate`/`Render` from `Created` — the mirror's source-state
    // and `Processing` gates.
    assert!(matches!(
        vex(&mut host, VideoHostCommand::Activate { handle }),
        Err(HostError::Lifecycle(_))
    ));
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        Err(HostError::Lifecycle(_))
    ));
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Created));

    expect_ok(vex(&mut host, VideoHostCommand::Initialize { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Activate { handle }));

    // Input format mismatch — a caller-supplied domain violation,
    // `Err(InvalidValue)` without the wire.
    let mismatched_format =
        VideoFrame::new_owned(PixelFormat::Yuv420p, 64, 64, render_at()).unwrap();
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: mismatched_format,
                at: render_at(),
            },
        ),
        HostError::InvalidValue,
    );
    // Input dims mismatch — same defined rejection.
    let mismatched_dims = VideoFrame::new_owned(PixelFormat::Rgba8, 32, 64, render_at()).unwrap();
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: mismatched_dims,
                at: render_at(),
            },
        ),
        HostError::InvalidValue,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Activated)
    );

    // The child is alive — `StartProcessing` still answers, then a
    // conforming `Render` succeeds.
    expect_ok(vex(&mut host, VideoHostCommand::StartProcessing { handle }));
    let output = render(&mut host, handle, input_frame());
    assert_eq!(output.pts(), input_frame().pts());
}

/// The plain-parameter channel's id list and metadata verbs:
/// `GetParams` lists the fixture's ids, `GetParamInfo` returns
/// `Some` for a known id and the plugin-declared `None` for an
/// unknown one (never an `Err`).
#[test]
fn params_ids_and_metadata() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    match vex(&mut host, VideoHostCommand::GetParams { handle }).unwrap() {
        VideoHostReply::ParamIds(ids) => {
            assert_eq!(ids, vec![PARAM_AMOUNT, PARAM_MODE]);
        }
        other => panic!("unexpected reply: {other:?}"),
    }

    match vex(
        &mut host,
        VideoHostCommand::GetParamInfo {
            handle,
            id: PARAM_MODE,
        },
    )
    .unwrap()
    {
        VideoHostReply::ParamInfo(Some(info)) => {
            assert_eq!(info.id, PARAM_MODE);
            assert_eq!(info.name, "Mode");
            assert_eq!(info.default_value, PlainParamValue::Choice(0));
            assert_eq!(info.choices, vec!["Invert", "Fill"]);
        }
        other => panic!("unexpected reply: {other:?}"),
    }
    match vex(
        &mut host,
        VideoHostCommand::GetParamInfo {
            handle,
            id: PARAM_AMOUNT,
        },
    )
    .unwrap()
    {
        VideoHostReply::ParamInfo(Some(info)) => {
            assert_eq!(info.id, PARAM_AMOUNT);
            assert_eq!(info.default_value, PlainParamValue::Double(1.0));
            assert_eq!(info.choices, Vec::<String>::new());
        }
        other => panic!("unexpected reply: {other:?}"),
    }
    // The plugin-declared unknown-id reply — `ParamInfo(None)`, not
    // an `Err`.
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamInfo {
                handle,
                id: ParamId(999),
            },
        ),
        Ok(VideoHostReply::ParamInfo(None))
    ));
}

/// `GetParamValue`/`SetParamValue` round-trip, and an unknown id is
/// `Err(InvalidValue)` on the value verbs.
#[test]
fn param_values_round_trip_and_unknown_ids() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(1.0)
    ));
    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: PARAM_AMOUNT,
            value: PlainParamValue::Double(0.25),
        },
    ));
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(0.25)
    ));

    // Unknown ids on the value verbs — `Err(InvalidValue)`, the
    // runner-side `id ∈ parameters()` membership rejection passed
    // through verbatim.
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: ParamId(999),
            },
        ),
        HostError::InvalidValue,
    );
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::SetParamValue {
                handle,
                id: ParamId(999),
                value: PlainParamValue::Boolean(true),
            },
        ),
        HostError::InvalidValue,
    );
}

/// Encode obligation (a): a non-finite `PlainParamValue::Double` is
/// the facade's own `Err(InvalidValue)` — the value never reaches
/// the wire (a sent one would decode-fail child-side and kill the
/// instance). Proven by the child staying alive with the stored
/// value unchanged.
#[test]
fn set_param_value_non_finite_rejected_without_wire() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        expect_err(
            vex(
                &mut host,
                VideoHostCommand::SetParamValue {
                    handle,
                    id: PARAM_AMOUNT,
                    value: PlainParamValue::Double(value),
                },
            ),
            HostError::InvalidValue,
        );
    }
    // The wire was never touched — the child still answers and the
    // value is untouched.
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(1.0)
    ));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
}

// ------------------------------------------------------------------
// `Instantiate` bound rejections — wire never sent, no child spawned
// ------------------------------------------------------------------

/// `check_video_bounds` runs before any allocation or spawn: a
/// config past `WIRE_VIDEO_MAX_DIM` or `WIRE_VIDEO_MAX_FRAME_BYTES`
/// is `Err(Prepare(UnsupportedConfig))`, no handle is minted and no
/// child ever existed (nothing can fail → no notification).
#[test]
fn instantiate_config_bound_rejections() {
    let mut host = video_host(&[]);

    // width past WIRE_VIDEO_MAX_DIM (8192).
    let over_dim = VideoProcessConfig::new(
        PixelFormat::Rgba8,
        8_193,
        64,
        FrameRate::new(30, 1).unwrap(),
    )
    .unwrap();
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Instantiate {
                plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
                config: over_dim,
            },
        ),
        HostError::Prepare(PrepareError::UnsupportedConfig),
    );

    // frame_bytes over WIRE_VIDEO_MAX_FRAME_BYTES:
    // 8192² × RgbaF32(16 B) = 1 GiB > 256 MiB.
    let over_bytes = VideoProcessConfig::new(
        PixelFormat::RgbaF32,
        8_192,
        8_192,
        FrameRate::new(30, 1).unwrap(),
    )
    .unwrap();
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Instantiate {
                plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
                config: over_bytes,
            },
        ),
        HostError::Prepare(PrepareError::UnsupportedConfig),
    );

    // Neither rejection spawned a child — there is nothing that
    // could ever fail or notify.
    assert_no_notification(&mut host);
    // An unknown id under a legal config is still `UnknownPluginId`
    // — the bound check never masked the route lookup.
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Instantiate {
                plugin_id: "org.example.missing".to_owned(),
                config: config(),
            },
        ),
        HostError::UnknownPluginId,
    );
}

// ------------------------------------------------------------------
// Cross-kind surface — kind-filtered scan, id collisions, verbs
// ------------------------------------------------------------------

/// Route-kind separation, the scan/instantiate half: the audio
/// `Scan` lists audio routes only, the video `Scan` video routes
/// only, and `Instantiate` across kinds is `Err(UnknownPluginId)`
/// either way.
#[test]
fn cross_kind_scan_and_instantiate() {
    let mut host = mixed_host(&[]);

    // Kind-filtered scans — audio descriptors never leak into the
    // video surface and vice versa.
    match host.execute(HostCommand::Scan).unwrap() {
        HostReply::Scanned(descriptors) => {
            assert!(descriptors.contains(&L0_AUDIO_DESCRIPTOR));
            assert!(descriptors.contains(&AUDIO_WORKER_DESCRIPTOR));
            assert!(!descriptors.contains(&WORKER_DESCRIPTOR));
        }
        other => panic!("unexpected reply: {other:?}"),
    }
    match vex(&mut host, VideoHostCommand::Scan).unwrap() {
        VideoHostReply::Scanned(descriptors) => {
            assert_eq!(descriptors, vec![WORKER_DESCRIPTOR]);
        }
        other => panic!("unexpected reply: {other:?}"),
    }

    // `Instantiate` across kinds — `UnknownPluginId` both ways.
    assert_eq!(
        host.execute(HostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
        }),
        Err(HostError::UnknownPluginId)
    );
    for audio_id in [L0_AUDIO_DESCRIPTOR.id, AUDIO_WORKER_DESCRIPTOR.id] {
        expect_err(
            vex(
                &mut host,
                VideoHostCommand::Instantiate {
                    plugin_id: audio_id.to_owned(),
                    config: config(),
                },
            ),
            HostError::UnknownPluginId,
        );
    }
}

/// The cross-kind verb half: every dispatched verb aimed across
/// kinds is the defined `Err(Unsupported)` — wire never sent (the
/// slot-kind branch answers). `Destroy` is the shared verb: it
/// frees either kind from either surface.
#[test]
fn cross_kind_verbs_unsupported_and_shared_destroy() {
    let mut host = mixed_host(&[]);

    // Mint one handle per kind.
    let video_handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    let audio_handle = match host
        .execute(HostCommand::Instantiate {
            plugin_id: L0_AUDIO_DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    };

    assert_audio_verbs_unsupported(&mut host, video_handle);
    assert_video_verbs_unsupported(&mut host, audio_handle);

    // `Destroy` is the shared verb — either surface frees either
    // kind (both slots are `Created`-legal here).
    expect_ok(vex(
        &mut host,
        VideoHostCommand::Destroy {
            handle: audio_handle,
        },
    ));
    assert_eq!(host.lifecycle_state(audio_handle), None);
    assert_eq!(
        host.execute(HostCommand::Destroy {
            handle: video_handle
        }),
        Ok(HostReply::Ok)
    );
    assert_eq!(host.lifecycle_state(video_handle), None);
}

/// Audio `HostCommand`s aimed at a video handle — every dispatched
/// verb is the defined `Unsupported` (slot-kind branch, no wire).
fn assert_audio_verbs_unsupported(host: &mut PluginHost, video_handle: PluginHandle) {
    for (command, label) in [
        (
            HostCommand::Initialize {
                handle: video_handle,
            },
            "Initialize",
        ),
        (
            HostCommand::Activate {
                handle: video_handle,
                config: audio_config(),
            },
            "Activate",
        ),
        (
            HostCommand::StartProcessing {
                handle: video_handle,
            },
            "StartProcessing",
        ),
        (
            HostCommand::StopProcessing {
                handle: video_handle,
            },
            "StopProcessing",
        ),
        (
            HostCommand::Deactivate {
                handle: video_handle,
            },
            "Deactivate",
        ),
        (
            HostCommand::SaveState {
                handle: video_handle,
            },
            "SaveState",
        ),
        (
            HostCommand::RestoreState {
                handle: video_handle,
                bytes: vec![0x01],
            },
            "RestoreState",
        ),
    ] {
        assert_eq!(
            host.execute(command),
            Err(HostError::Unsupported),
            "audio {label} on a video handle must be Unsupported"
        );
    }
}

/// Video `VideoHostCommand`s aimed at an audio handle — the same
/// `Unsupported` (slot-kind branch, no wire).
fn assert_video_verbs_unsupported(host: &mut PluginHost, audio_handle: PluginHandle) {
    expect_err(
        vex(
            host,
            VideoHostCommand::Initialize {
                handle: audio_handle,
            },
        ),
        HostError::Unsupported,
    );
    expect_err(
        vex(
            host,
            VideoHostCommand::Activate {
                handle: audio_handle,
            },
        ),
        HostError::Unsupported,
    );
    expect_err(
        vex(
            host,
            VideoHostCommand::Render {
                handle: audio_handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::Unsupported,
    );
    expect_err(
        vex(
            host,
            VideoHostCommand::GetParams {
                handle: audio_handle,
            },
        ),
        HostError::Unsupported,
    );
    expect_err(
        vex(
            host,
            VideoHostCommand::SetParamValue {
                handle: audio_handle,
                id: PARAM_AMOUNT,
                value: PlainParamValue::Double(0.5),
            },
        ),
        HostError::Unsupported,
    );
}

/// One `plugin_id` claims one route of one kind — a same-id
/// registration across kinds is `Err(DuplicatePluginId)` in both
/// orders.
#[test]
fn duplicate_plugin_id_both_directions() {
    // Video first, audio second.
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(video_spec(&[]))
        .unwrap();
    assert_eq!(
        host.register_out_of_process(OutOfProcessSpec::new(
            WORKER_DESCRIPTOR,
            audio_worker_program(),
        )),
        Err(HostError::DuplicatePluginId)
    );

    // Audio first, video second.
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(
        AUDIO_WORKER_DESCRIPTOR,
        audio_worker_program(),
    ))
    .unwrap();
    assert_eq!(
        host.video()
            .register_out_of_process(VideoOutOfProcessSpec::new(
                AUDIO_WORKER_DESCRIPTOR,
                worker_program(),
            )),
        Err(HostError::DuplicatePluginId)
    );
}

/// The audio-only `PluginHost` methods' defined answers for a video
/// handle: `process` is `Err(Unsupported)`, `latency_frames`/
/// `plugin_mut` are `None`, `prepare_endpoint` is
/// `Err(EndpointUnavailable)` — while `lifecycle_state` mirrors and
/// `kill_child` works (with its `Failed` + notification result).
#[test]
fn direct_plugin_host_methods_on_video_handle() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );

    let events = EventList::new(&[], 0).unwrap();
    assert_eq!(
        host.process(handle, &[], &mut [], events, None),
        Err(HostError::Unsupported)
    );
    assert_eq!(host.latency_frames(handle), None);
    assert!(host.plugin_mut(handle).is_none());
    assert_eq!(
        host.prepare_endpoint(
            handle,
            EndpointBounds {
                max_frames: 512,
                channels: 2,
            },
        )
        .map(|_| ()),
        Err(HostError::EndpointUnavailable)
    );

    // `kill_child` works on the video slot — `Failed` convergence
    // plus its one `InstanceFailed` land asynchronously, carrying
    // the real exit status and the first-observed cause.
    host.kill_child(handle).unwrap();
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    assert!(exit.code.is_some() || exit.signal.is_some());
    // Reaped exit or stdout EOF — whichever the driver observed
    // first (the `ExitObserved`/`Transport` race).
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));
    assert_no_notification(&mut host);
}

// ------------------------------------------------------------------
// Failed parity — convergence, notifications, timeout separation
// ------------------------------------------------------------------

/// `kill_child` on a live video slot: the instance converges
/// `Failed`, `InstanceFailed{handle}` arrives exactly once, every
/// non-`Destroy` verb is `Err(Failed)`, and `Destroy` still frees
/// the slot.
#[test]
fn kill_child_converges_failed_and_notifies_once() {
    let mut host = video_host(&[]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    host.kill_child(handle).unwrap();
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    // A real OS kill — the notification carries the reaped status
    // (signal on unix, code on Windows) and the first-observed
    // cause (`ExitObserved`/`Transport` — the observation race).
    assert!(exit.code.is_some() || exit.signal.is_some());
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));

    // Every verb but `Destroy` is `Err(Failed)` on the converged
    // instance — the same vocabulary the audio route uses.
    for command in [
        VideoHostCommand::Initialize { handle },
        VideoHostCommand::Activate { handle },
        VideoHostCommand::StartProcessing { handle },
        VideoHostCommand::StopProcessing { handle },
        VideoHostCommand::Deactivate { handle },
        VideoHostCommand::GetParams { handle },
        VideoHostCommand::GetParamValue {
            handle,
            id: PARAM_AMOUNT,
        },
        VideoHostCommand::GetParamInfo {
            handle,
            id: PARAM_AMOUNT,
        },
    ] {
        expect_err(vex(&mut host, command), HostError::Failed);
    }
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::Failed,
    );
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::SetParamValue {
                handle,
                id: PARAM_AMOUNT,
                value: PlainParamValue::Double(0.5),
            },
        ),
        HostError::Failed,
    );

    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    // Exactly once — no second notification for the freed slot.
    assert_no_notification(&mut host);
}

/// Mid-`Render` child death: the waiter is woken `Err(Ipc)` by the
/// EOF/convergence (the `out` region is never read), the instance
/// converges `Failed` and notifies once. The worker exits upon
/// *receiving* the 6th parent→child message — Scan(1),
/// CreateVideoInstance(2), Initialize(3), Activate(4),
/// StartProcessing(5), VideoRender(6) — before dispatch.
#[test]
fn mid_render_child_death() {
    let mut host = video_host(&["--crash-after-msgs=6"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::Ipc,
    );
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    // `--crash-after-msgs` exits with the default code 3 — the
    // reaped status reaches the notification; the cause is
    // `try_wait` or the racing stdout EOF.
    assert_eq!(exit.code, Some(3));
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));

    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}

/// Mid-lifecycle-verb death (a non-`Render` waiter): `Initialize`
/// is message 3 — the crash fires before its dispatch, the waiter
/// wakes `Err(Ipc)`, the minted instance converges `Failed` and
/// notifies.
#[test]
fn mid_verb_child_death_notifies() {
    let mut host = video_host(&["--crash-after-msgs=3"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());

    expect_err(
        vex(&mut host, VideoHostCommand::Initialize { handle }),
        HostError::Ipc,
    );
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    assert_eq!(exit.code, Some(3));
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}

/// Mid-`DestroyInstance` death: the reclaim request (message 3 on a
/// `Created` instance) kills the child mid-verb — `Destroy` still
/// frees the slot `Ok` (teardown is unconditional). The instance's
/// own `InstanceFailed` is unobservable: its facade is freed with
/// the slot — exactly the audio `bundled_mid_destroy` shape where
/// the dying instance is never asserted (only a *sibling* carries
/// an observable notification, and Dedicated has no siblings).
#[test]
fn mid_destroy_child_death_frees_slot() {
    let mut host = video_host(&["--crash-after-msgs=3"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());

    // The `DestroyInstance` request itself is the crash message —
    // the request fails, but `Destroy` is `Ok` (slot freed).
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_eq!(host.lifecycle_state(handle), None);
    assert_no_notification(&mut host);
}

/// Mid-`CreateVideoInstance` death: the create request itself is
/// message 2 — the child exits before dispatch, `Instantiate` is
/// `SpawnFailed` carrying the convergence's cause plus a
/// registered attempt record, no handle is minted and (the
/// `minted() == false` silent-converge rule) no `InstanceFailed`
/// ever arrives.
#[test]
fn mid_create_child_death_no_handle_no_notification() {
    let mut host = video_host(&["--crash-after-msgs=2"]);
    let Err(HostError::SpawnFailed {
        cause,
        diagnostics_id,
    }) = vex(
        &mut host,
        VideoHostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
            config: config(),
        },
    )
    else {
        panic!("a dead create must surface as SpawnFailed")
    };
    // Exit or broken pipe — whichever the driver observed first.
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));
    assert!(matches!(
        host.diagnostics_query(diagnostics_id),
        DiagnosticQuery::Found(_)
    ));
    // The unminted instance converges silently — the queue stays
    // empty through the grace window.
    assert_no_notification(&mut host);
}

/// `--noncanonical-output`: a well-formed `Rendered` reply whose
/// `output` descriptor fails the host's post-decode semantic gate
/// is the instance-attributable violation — the caller gets
/// `Err(Ipc)`, *that* instance converges `Failed` and notifies
/// once. A sibling instance on its own dedicated child keeps
/// serving non-render verbs — the convergence is per-instance,
/// never child-scope-wide. (The sibling's own worker carries the
/// same flag, so its liveness is pinned through lifecycle verbs —
/// its `Render` would itself violate; never issued.)
#[test]
fn noncanonical_output_converges_that_instance_only() {
    let mut host = video_host(&["--noncanonical-output"]);
    let violating = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    let sibling = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, violating);
    drive_to_processing(&mut host, sibling);

    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle: violating,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::Ipc,
    );
    wait_failed(&host, violating);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, violating);
    // `converge_instance`'s synthesized exit — the child is alive,
    // so there is no status to report (`{code: None, signal: None}`)
    // — and the attributable violation's `Protocol` cause.
    assert_eq!(exit, ChildExit::default());
    assert_eq!(cause, FailureCause::Protocol);

    // The sibling survived — the violation was attributable to one
    // instance only: its mirror is still `Processing` and its
    // lifecycle verbs keep answering through `Destroy`.
    assert_eq!(
        host.lifecycle_state(sibling),
        Some(LifecycleState::Processing)
    );
    expect_ok(vex(
        &mut host,
        VideoHostCommand::StopProcessing { handle: sibling },
    ));
    expect_ok(vex(
        &mut host,
        VideoHostCommand::Deactivate { handle: sibling },
    ));
    expect_ok(vex(
        &mut host,
        VideoHostCommand::Destroy { handle: sibling },
    ));
    expect_ok(vex(
        &mut host,
        VideoHostCommand::Destroy { handle: violating },
    ));
    assert_no_notification(&mut host);
}

/// `--fail-initialize`: the `Err(PluginFailed)` reply leaves the
/// mirror `Created` (the runner's `InitFailed` destination), the
/// child alive, and `Destroy` legal.
#[test]
fn fail_initialize_keeps_created() {
    let mut host = video_host(&["--fail-initialize"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    expect_err(
        vex(&mut host, VideoHostCommand::Initialize { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Created));
    // The child survived — a second `Initialize` answers the same
    // way (deterministic flag), and `Destroy` stays legal.
    expect_err(
        vex(&mut host, VideoHostCommand::Initialize { handle }),
        HostError::PluginFailed,
    );
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}

/// `--fail-start-processing`: the `Err(PluginFailed)` leaves the
/// mirror `Activated` (the runner's hook-failure rollback), so the
/// unwind path is `Deactivate → Destroy`.
#[test]
fn fail_start_processing_stays_activated() {
    let mut host = video_host(&["--fail-start-processing"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    expect_ok(vex(&mut host, VideoHostCommand::Initialize { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Activate { handle }));
    expect_err(
        vex(&mut host, VideoHostCommand::StartProcessing { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Activated)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// `--fail-stop-processing`: commit-then-advisory — the transition
/// to `Activated` lands *and* the caller sees `Err(PluginFailed)`.
#[test]
fn fail_stop_processing_commits_activated() {
    let mut host = video_host(&["--fail-stop-processing"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);
    expect_err(
        vex(&mut host, VideoHostCommand::StopProcessing { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Activated)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// `--fail-deactivate`: commit-then-advisory — `Initialized` lands
/// *and* the caller sees `Err(PluginFailed)`; `Destroy` from
/// `Initialized` stays legal.
#[test]
fn fail_deactivate_commits_initialized() {
    let mut host = video_host(&["--fail-deactivate"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);
    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_err(
        vex(&mut host, VideoHostCommand::Deactivate { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Initialized)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// `--fail-render`: `Err(PluginFailed)` is the defined hook error —
/// the mirror stays `Processing` (an `Err` reply never contaminates
/// it), the child answers again, and no notification is produced.
/// The `out` region was never committed.
#[test]
fn fail_render_mirror_invariant() {
    let mut host = video_host(&["--fail-render"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    // Child alive — the deterministic flag answers identically.
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::PluginFailed,
    );
    // The mirror was never touched and nothing notified.
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    assert_no_notification(&mut host);
    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// `--fail-param-value`: `GetParamValue`/`SetParamValue` on a known
/// id answer `Err(PluginFailed)` — obligation (b)'s host-observable
/// shape — while the membership check still answers
/// `Err(InvalidValue)` for an unknown id and every other verb keeps
/// the child alive.
#[test]
fn fail_param_value_known_id_only() {
    let mut host = video_host(&["--fail-param-value"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_err(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        HostError::PluginFailed,
    );
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::SetParamValue {
                handle,
                id: PARAM_AMOUNT,
                value: PlainParamValue::Double(0.5),
            },
        ),
        HostError::PluginFailed,
    );
    // Membership precedes the hook — the unknown-id rejection is
    // still `InvalidValue`, not `PluginFailed`.
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: ParamId(999),
            },
        ),
        HostError::InvalidValue,
    );
    // The child is alive and the mirror is untouched.
    assert!(matches!(
        vex(&mut host, VideoHostCommand::GetParams { handle }),
        Ok(VideoHostReply::ParamIds(ids)) if ids == vec![PARAM_AMOUNT, PARAM_MODE]
    ));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    assert_no_notification(&mut host);
}

/// `--overlimit-params`: `GetParams`/`GetParamInfo` answer
/// `Err(PluginFailed)` — obligation (c)'s adapter-normalized
/// answer, emulated fixture-side — while a verb outside the
/// over-limit surface proves the child alive.
#[test]
fn overlimit_params_plugin_failed_child_alive() {
    let mut host = video_host(&["--overlimit-params"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_err(
        vex(&mut host, VideoHostCommand::GetParams { handle }),
        HostError::PluginFailed,
    );
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::GetParamInfo {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        HostError::PluginFailed,
    );
    // The over-limit arm never touched `parameter_value` — the
    // child's `Err(PluginFailed)` replies came back as replies, not
    // as an `Err(Ipc)`/convergence, and the mirror is clean.
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(1.0)
    ));
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    assert_no_notification(&mut host);
}

/// `Render` waits on `render_timeout`, not `command_timeout`: the
/// worker hangs after `StartProcessing` (message 5) so the
/// `VideoRender` request (message 6) is never answered — the
/// injected 2 s render timeout fires `Err(Ipc)` well inside the
/// default 10 s command timeout, then converges child-scope
/// `Failed`.
#[test]
fn render_wedge_uses_render_timeout() {
    let mut host = PluginHost::new();
    let mut spec = video_spec(&["--hang-after-msgs=5"]);
    spec.render_timeout = SHORT_TIMEOUT;
    host.video().register_out_of_process(spec).unwrap();
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    let start = Instant::now();
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::Ipc,
    );
    // The 2 s render timeout fired — never the 10 s default
    // command timeout.
    assert!(
        start.elapsed() < Duration::from_secs(8),
        "render wedge must fire render_timeout, not command_timeout"
    );
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    // The timeout path converges `Failed` then kills the wedged
    // child — the `Pending` notification completes with the reaped
    // kill status while the recorded `Timeout` stands (the cause
    // is stamped before the kill issues, so the reaped exit never
    // overwrites it).
    assert!(exit.code.is_some() || exit.signal.is_some());
    assert_eq!(cause, FailureCause::Timeout);
}

/// `command_timeout` covers every non-`Render` verb: the worker
/// hangs after `Activate` (message 4) so `StartProcessing`
/// (message 5) is never answered — the injected 2 s command
/// timeout fires `Err(Ipc)` and converges child-scope `Failed`.
#[test]
fn command_wedge_uses_command_timeout() {
    let mut host = PluginHost::new();
    let mut spec = video_spec(&["--hang-after-msgs=4"]);
    spec.command_timeout = SHORT_TIMEOUT;
    host.video().register_out_of_process(spec).unwrap();
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    expect_ok(vex(&mut host, VideoHostCommand::Initialize { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Activate { handle }));

    let start = Instant::now();
    expect_err(
        vex(&mut host, VideoHostCommand::StartProcessing { handle }),
        HostError::Ipc,
    );
    assert!(
        start.elapsed() < Duration::from_secs(8),
        "command wedge must fire command_timeout"
    );
    wait_failed(&host, handle);
    let (failed_handle, exit, cause) = wait_notification(&mut host);
    assert_eq!(failed_handle, handle);
    assert!(exit.code.is_some() || exit.signal.is_some());
    assert_eq!(cause, FailureCause::Timeout);
}

/// A `Hello.version` mismatch is a handshake violation —
/// `Instantiate` fails `SpawnFailed{cause: Handshake}` before any
/// `Scan`/`Create` traffic, no handle is minted and nothing can
/// notify.
#[test]
fn hello_version_mismatch_fails_instantiate() {
    let mut host = video_host(&["--hello-version=99"]);
    let Err(HostError::SpawnFailed {
        cause,
        diagnostics_id,
    }) = vex(
        &mut host,
        VideoHostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
            config: config(),
        },
    )
    else {
        panic!("a handshake violation must surface as SpawnFailed")
    };
    assert_eq!(cause, FailureCause::Handshake);
    assert!(matches!(
        host.diagnostics_query(diagnostics_id),
        DiagnosticQuery::Found(_)
    ));
    assert_no_notification(&mut host);
}

/// A handshake wedge is also `command_timeout`:
/// `--hang-after-msgs=0` stops the child answering even the
/// handshake `Scan`, so `Instantiate` fails
/// `SpawnFailed{cause: Timeout}` inside the injected timeout — no
/// handle, no notification.
#[test]
fn handshake_wedge_uses_command_timeout() {
    let mut host = PluginHost::new();
    let mut spec = video_spec(&["--hang-after-msgs=0"]);
    spec.command_timeout = SHORT_TIMEOUT;
    host.video().register_out_of_process(spec).unwrap();

    let start = Instant::now();
    let Err(HostError::SpawnFailed {
        cause,
        diagnostics_id,
    }) = vex(
        &mut host,
        VideoHostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
            config: config(),
        },
    )
    else {
        panic!("a handshake wedge must surface as SpawnFailed")
    };
    assert!(
        start.elapsed() < Duration::from_secs(8),
        "handshake wedge must fire command_timeout"
    );
    assert_eq!(cause, FailureCause::Timeout);
    assert!(matches!(
        host.diagnostics_query(diagnostics_id),
        DiagnosticQuery::Found(_)
    ));
    assert_no_notification(&mut host);
}

// ------------------------------------------------------------------
// Diagnostics parity (Issue #121 T11) — stderr capture, the
// attempt/minted record pair, and the audio-counter boundary the
// `InstKind::Video` arm owns nothing of
// ------------------------------------------------------------------

/// The pre-mint attempt record's stderr capture: the worker emits
/// `--stderr-bytes` before `Hello`, the drain — started ahead of
/// the `Hello` wait, the same machinery the audio route uses —
/// retains them, and the mid-`CreateVideoInstance` wedge freezes
/// them into the registered record. The wedge is a *hang*, not a
/// crash, deliberately: the `command_timeout` window gives the
/// detached drain a deterministic slice to read the pre-`Hello`
/// emission before `create_rollback_error` freezes (a mid-create
/// *crash* would race the drain — the freeze can precede the
/// first read). `handle` stays `None` (nothing minted) while
/// `phase` is `Live` — the *child's* spawn phase, which passed
/// `Hello` before its create wedged — the same shape the audio
/// mid-create record carries.
#[test]
fn spawn_failed_attempt_record_captures_stderr() {
    const STDERR_BYTES: usize = 128;
    let mut host = PluginHost::new();
    let mut spec = video_spec(&["--stderr-bytes=128", "--hang-after-msgs=1"]);
    spec.command_timeout = SHORT_TIMEOUT;
    host.video().register_out_of_process(spec).unwrap();

    let start = Instant::now();
    let Err(HostError::SpawnFailed {
        cause,
        diagnostics_id,
    }) = vex(
        &mut host,
        VideoHostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
            config: config(),
        },
    )
    else {
        panic!("a wedged create must surface as SpawnFailed")
    };
    assert!(
        start.elapsed() < Duration::from_secs(8),
        "a create wedge must fire command_timeout"
    );
    // The request wait fired `command_timeout` — the cause was
    // recorded before the wedge-kill issued.
    assert_eq!(cause, FailureCause::Timeout);
    let DiagnosticQuery::Found(record) = host.diagnostics_query(diagnostics_id) else {
        panic!("the registered attempt record must resolve")
    };
    assert_eq!(record.diagnostics_id, diagnostics_id);
    // Pre-mint: no handle — and `Live` is the child's phase (it
    // passed `Hello`; the instance mint is what wedged).
    assert_eq!(record.handle, None);
    assert_eq!(record.phase, SpawnPhase::Live);
    // The record carries the same first-observation the error did.
    assert_eq!(record.cause, Some(FailureCause::Timeout));
    // The whole pre-`Hello` emission survived in the retained
    // tail — 128 bytes are far under the ring cap, so nothing
    // dropped (`eof` itself is the drain's post-exit state —
    // racing the freeze, deliberately unasserted).
    assert_eq!(record.stderr.bytes, vec![b'!'; STDERR_BYTES]);
    assert_eq!(record.stderr.total_read_bytes, STDERR_BYTES as u64);
    assert_eq!(record.stderr.dropped_bytes, 0);
    assert!(!record.stderr.drain_deadline_exceeded);
    assert!(!record.stderr.unread_bytes_observed);
    // The unminted instance converges silently — no notification.
    assert_no_notification(&mut host);
}

/// The minted-failure correlation on the video route — the same
/// handle→record contract the audio L1 route pins: a live handle
/// resolves `None`, the `InstanceFailed` pop mints a record keyed
/// by the slot carrying the notification's own first-observed
/// cause verbatim plus the child's retained stderr tail.
#[test]
fn minted_failure_registers_handle_correlated_record() {
    const STDERR_BYTES: usize = 256;
    let mut host = video_host(&["--stderr-bytes=256"]);
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    // No record exists for a live handle's slot.
    assert!(host.diagnostics_for_handle(handle).is_none());

    host.kill_child(handle).unwrap();
    wait_failed(&host, handle);
    let (notified, exit, cause) = wait_notification(&mut host);
    assert_eq!(notified, handle);
    assert!(exit.code.is_some() || exit.signal.is_some());
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));

    // The handle correlates the retained record — slot key, the
    // notification's own cause, the `Live` phase and the frozen
    // capture.
    let record = host
        .diagnostics_for_handle(handle)
        .expect("minted failure must register a record");
    assert_eq!(record.handle, Some(handle.slot()));
    assert_eq!(record.cause, Some(cause));
    assert_eq!(record.phase, SpawnPhase::Live);
    // The pre-`Hello` stderr emission survived in the frozen tail.
    assert_eq!(record.stderr.bytes, vec![b'!'; STDERR_BYTES]);
    assert_eq!(record.stderr.total_read_bytes, STDERR_BYTES as u64);
    assert_eq!(record.stderr.dropped_bytes, 0);
    // The same record is `Found` by id; a never-minted id is
    // `NotFound`.
    assert!(matches!(
        host.diagnostics_query(record.diagnostics_id),
        DiagnosticQuery::Found(found)
            if found.diagnostics_id == record.diagnostics_id
    ));
    assert!(matches!(
        host.diagnostics_query(record.diagnostics_id + 1_000),
        DiagnosticQuery::NotFound
    ));

    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}

/// The counter query surface is audio-only — the
/// non-contamination pin: `audio_counters` on a video handle is
/// `None` in every state (live, converged, freed), while an L1
/// audio instance on the same host reads its fresh pair. The
/// `InstKind::Video` arm owns no counter pair, so nothing can leak
/// onto the video route.
#[test]
fn audio_counters_absent_on_the_video_route() {
    let mut host = mixed_host(&[]);
    let video_handle = instantiate(&mut host, WORKER_DESCRIPTOR.id, config());
    let audio_handle = match host
        .execute(HostCommand::Instantiate {
            plugin_id: AUDIO_WORKER_DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    };

    // Contrast oracle: the L1 audio slot carries a fresh counter
    // pair — zero until the first block runs.
    assert_eq!(
        host.audio_counters(audio_handle),
        Some(AudioCounterSnapshot::default())
    );
    // The video slot's defined answer is the not-found `None`.
    assert_eq!(host.audio_counters(video_handle), None);

    drive_to_processing(&mut host, video_handle);
    assert_eq!(host.audio_counters(video_handle), None);

    host.kill_child(video_handle).unwrap();
    wait_failed(&host, video_handle);
    let (notified, _exit, _cause) = wait_notification(&mut host);
    assert_eq!(notified, video_handle);
    // A `Failed` video entry still owns no counters.
    assert_eq!(host.audio_counters(video_handle), None);

    expect_ok(vex(
        &mut host,
        VideoHostCommand::Destroy {
            handle: video_handle,
        },
    ));
    assert_eq!(host.audio_counters(video_handle), None);
    assert_eq!(
        host.execute(HostCommand::Destroy {
            handle: audio_handle,
        }),
        Ok(HostReply::Ok)
    );
}

// ------------------------------------------------------------------
// Real-runner lane — the same wire contract against `run_video`
// ------------------------------------------------------------------

/// The real-runner happy path: `ruviform-plugin-l1-video-runner`'s
/// Invert fixture over the identical command series — lifecycle,
/// the invert pixel oracle, metadata pass-through and the parameter
/// surface (ids 0–4 here — the runner's wider fixture).
#[test]
fn runner_lane_invert_happy_path() {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(runner_spec(RUNNER_INVERT_DESCRIPTOR))
        .unwrap();

    let handle = instantiate(&mut host, RUNNER_INVERT_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    let input = input_frame();
    let input_bytes: Vec<u8> = input.plane(0).unwrap().data().to_vec();
    let output = render(&mut host, handle, input);
    for (src, dst) in input_bytes
        .iter()
        .zip(output.plane(0).unwrap().data().iter())
    {
        assert_eq!(*dst, !*src);
    }
    assert_eq!(output.pts(), input_frame().pts());
    assert_eq!(output.duration(), input_frame().duration());
    assert_eq!(output.color(), color());

    // The runner's wider param surface (ids 0–4) — the public
    // channel over the real `erase_video` boundary.
    match vex(&mut host, VideoHostCommand::GetParams { handle }).unwrap() {
        VideoHostReply::ParamIds(ids) => {
            assert_eq!(
                ids,
                vec![
                    PARAM_AMOUNT,
                    PARAM_MODE,
                    ParamId(2),
                    RUNNER_PARAM_FAIL_DEACTIVATE,
                    RUNNER_PARAM_FAIL_RENDER,
                ]
            );
        }
        other => panic!("unexpected reply: {other:?}"),
    }
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(1.0)
    ));
    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: PARAM_AMOUNT,
            value: PlainParamValue::Double(0.5),
        },
    ));
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: PARAM_AMOUNT,
            },
        ),
        Ok(VideoHostReply::ParamValue(value)) if value == PlainParamValue::Double(0.5)
    ));
    // `EchoWidth` — the stash/replay oracle: `CreateVideoInstance`'s
    // config reached the `initialize` hook (the param reads back
    // the stashed width).
    assert!(matches!(
        vex(
            &mut host,
            VideoHostCommand::GetParamValue {
                handle,
                id: ParamId(2),
            },
        ),
        Ok(VideoHostReply::ParamValue(PlainParamValue::Integer(64)))
    ));

    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}

/// Runner-lane `Err` semantics: the Invert fixture's `FailRender`
/// parameter switches a defined `render` hook error on — caller
/// `Err(PluginFailed)`, mirror stays `Processing`, child alive —
/// and off again for a clean render.
#[test]
fn runner_lane_fail_render_via_param() {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(runner_spec(RUNNER_INVERT_DESCRIPTOR))
        .unwrap();
    let handle = instantiate(&mut host, RUNNER_INVERT_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: RUNNER_PARAM_FAIL_RENDER,
            value: PlainParamValue::Boolean(true),
        },
    ));
    expect_err(
        vex(
            &mut host,
            VideoHostCommand::Render {
                handle,
                input: input_frame(),
                at: render_at(),
            },
        ),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: RUNNER_PARAM_FAIL_RENDER,
            value: PlainParamValue::Boolean(false),
        },
    ));
    let output = render(&mut host, handle, input_frame());
    assert_eq!(output.plane(0).unwrap().data()[0], !0u8);

    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// Runner-lane commit-then-advisory: `FailDeactivate` arms a hook
/// error that must still leave the mirror `Initialized`.
#[test]
fn runner_lane_fail_deactivate_commits_initialized() {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(runner_spec(RUNNER_INVERT_DESCRIPTOR))
        .unwrap();
    let handle = instantiate(&mut host, RUNNER_INVERT_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_ok(vex(
        &mut host,
        VideoHostCommand::SetParamValue {
            handle,
            id: RUNNER_PARAM_FAIL_DEACTIVATE,
            value: PlainParamValue::Boolean(true),
        },
    ));
    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_err(
        vex(&mut host, VideoHostCommand::Deactivate { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Initialized)
    );
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// Runner-lane obligation (c): the `OverLimit` fixture's
/// `parameters()` report is unencodable verbatim, so `GetParams`
/// answers the adapter-normalized `Err(PluginFailed)` — the reply
/// arrived as a reply (no `Err(Ipc)`/convergence), the child alive.
#[test]
fn runner_lane_overlimit_params_plugin_failed_child_alive() {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(runner_spec(RUNNER_OVERLIMIT_DESCRIPTOR))
        .unwrap();
    let handle = instantiate(&mut host, RUNNER_OVERLIMIT_DESCRIPTOR.id, config());
    drive_to_processing(&mut host, handle);

    expect_err(
        vex(&mut host, VideoHostCommand::GetParams { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );
    assert_no_notification(&mut host);
    expect_ok(vex(&mut host, VideoHostCommand::StopProcessing { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Deactivate { handle }));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
}

/// Runner-lane panic normalization: the `PanicVideo` fixture's
/// `initialize` panics — the runner's `catch_unwind` boundary maps
/// it to `Err(PluginFailed)`, the mirror stays `Created`, the child
/// keeps serving (`Destroy` answers).
#[test]
fn runner_lane_panic_initialize_plugin_failed() {
    let mut host = PluginHost::new();
    host.video()
        .register_out_of_process(runner_spec(RUNNER_PANIC_DESCRIPTOR))
        .unwrap();
    let handle = instantiate(&mut host, RUNNER_PANIC_DESCRIPTOR.id, config());
    expect_err(
        vex(&mut host, VideoHostCommand::Initialize { handle }),
        HostError::PluginFailed,
    );
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Created));
    expect_ok(vex(&mut host, VideoHostCommand::Destroy { handle }));
    assert_no_notification(&mut host);
}
