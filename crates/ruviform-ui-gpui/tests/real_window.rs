//! Issue #314 slice 4 — `ScenarioDriver::RealOs` adapter
//! (`RealOsGuiTest`, `real-scenario` feature); slice 4 follow-up
//! adds the canonical `test-ui-real-scenario` mise lane and the
//! full {theme, scale} story matrix.
//!
//! Each canonical Driver Lab story mounts into a *real OS window*
//! (`RealOsGuiTest::run` → `GpuiDriver::create_view_window` on the
//! platform loop) once per matrix cell — both manifest themes
//! (`synvelis-dark`/`audiolens-dark`) × both declared scale factors
//! (1.0/2.0), mirroring `stories.json` and the Fake/GPUI-simulated
//! matrices — and walks the seven shared semantic states through
//! the `SemanticGuiTest` port — the same
//! `expected_lab_story_trace()` oracle the Fake and GPUI-simulated
//! layers assert. Semantic dispatch crosses the UI-thread job seam
//! into `App` context; it is still *semantic* evidence, never
//! pointer/OS-input evidence.
//!
//! The visual layer is the point of this slice: `capture_frame`
//! reads the real window's pixels through
//! `GpuiDriver::capture_window` — `PrintWindow` primary, screen
//! `BitBlt` fallback on Windows; X11 `GetImage` on Linux — and feeds
//! them to `golden_check`/`BaselineStore::compare`. On a fresh store
//! the verdict is `blocked`/`baseline_missing` — proof the pixels
//! reached the compare path — and a second capture after
//! candidate → promote must `pass`, the same loop
//! `golden_capture.rs` verifies at the driver level.
//!
//! Interactive display required: on Windows an interactive desktop
//! session; on Linux an X display — the CI offscreen lane runs this
//! target under `xvfb-run` (virtual framebuffer, no GPU/physical
//! display) via `mise run test-ui-offscreen-xvfb`. On a headless
//! host the window cannot open and `run` reports `UiError::Host` —
//! the cell records `observed="environment_blocked"` in the
//! driver-lab evidence JSONL and panics with an
//! `environment_blocked:` tag, a distinct classification from a
//! GUI/test failure (the same verdict split the Issue #552
//! `os_input::preflight` lanes use — this lane injects no OS input,
//! so the preflight itself does not apply). This target is gated on
//! `real-scenario`, which `gpui-test` deliberately does not enable
//! (the test platform stubs OS capture), so it runs as an
//! auxiliary lane: `mise run test-ui-real-scenario` (manual, never
//! CI — Windows interactive desktop) or
//! `mise run test-ui-offscreen-xvfb` (Linux Xvfb — the RF-GT-05 CI
//! lane), or `cargo test -p ruviform-ui-gpui --features
//! real-scenario --test real_window`.

#![cfg(all(any(windows, target_os = "linux"), feature = "real-scenario"))]

use std::path::{Path, PathBuf};

use ruviform_ui::behavior::{
    CommandIntent, CommandPhase, CommandReceipt, DetachPolicy, ValueDescriptor,
};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState,
    ControlSpec, Orientation, PanelSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{Event, UiError, WindowId, WindowSpec};
use ruviform_ui_gpui::os_input::{self, OsInputOutcome, OsKey, PreflightReport, PreflightScope};
use ruviform_ui_gpui::{CaptureMeta, RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::driver_lab;
use ruviform_ui_test::golden::{
    BaselineApproval, BaselineStore, CapturePath, ChannelTolerance, DiffPolicy, EnvironmentMeta,
    GoldenKey, GoldenVerdict,
};
use ruviform_ui_test::lab_story::{self, StoryFixture};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    RunId, ScenarioCapability, ScenarioError, ScenarioId, ScenarioOutcome, ScenarioReport,
    ScenarioSpec, ScenarioTarget, ScenarioVerdict, SemanticGuiTest, TargetQuery,
    record_golden_step, run_scenario, settle_until,
};
use ruviform_ui_theme::{ResolvedTheme, ScaleContext, ThemeResolver, TokenDocument};
use serde_json::{Value, json};

/// Story matrix axes — both product themes × both declared scale
/// factors, mirroring `stories.json` (`themes` + `scaleFactor`) and
/// the Fake/GPUI-simulated matrices in `tests/lab_story.rs`.
const THEMES: [&str; 2] = ["synvelis-dark", "audiolens-dark"];
const SCALES: [f64; 2] = [1.0, 2.0];

/// Mechanism tag written into driver-lab evidence rows — names the
/// canonical mise lane that owns this test target on each OS.
#[cfg(windows)]
const EVIDENCE_MECHANISM: &str = "mise-task:test-ui-real-scenario";
#[cfg(target_os = "linux")]
const EVIDENCE_MECHANISM: &str = "mise-task:test-ui-offscreen-xvfb";

/// `drain` boundaries waited before `body` — the driver default
/// (30 ticks ≈ 0.5 s) covers Windows' first-paint settle. Linux
/// needs far more: under the CI lane the renderer is Mesa
/// lavapipe/llvmpipe on Xvfb, whose first-frame shader compilation
/// takes seconds per window, so captures before ~9.6 s (600 ticks)
/// read a still-black drawable.
#[cfg(windows)]
const STARTUP_SETTLE_TICKS: u32 = 30;
#[cfg(target_os = "linux")]
const STARTUP_SETTLE_TICKS: u32 = 600;

/// Loads a theme fixture from the shared `ruviform-ui-test` theme
/// JSON — identical to the Fake/GPUI-simulated matrix.
fn resolve_theme(name: &str) -> ResolvedTheme {
    let path = format!(
        "{}/../ruviform-ui-test/fixtures/themes/{name}.dtcg.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let doc = TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    ThemeResolver::resolve(&[doc], "dark").unwrap_or_else(|e| panic!("resolve {name}: {e}"))
}

/// Runs `body` inside one real-window session for `fixture` at the
/// `{theme_name, scale_factor}` matrix cell — mounts the canonical
/// spec as a real OS window, waits the startup settle, then hands
/// the adapter to `body` on the caller thread. The theme scale
/// enters as `ResolvedTheme::with_scale` `ScaleContext` (a
/// mapper-side hint): tokens must be invariant under it, the same
/// guard the Fake/GPUI-simulated matrices apply per cell.
fn run_story_session(
    fixture: &StoryFixture,
    theme_name: &str,
    scale_factor: f64,
    body: impl FnOnce(&mut RealOsGuiTest, WindowId, ScenarioTarget) -> Result<(), ScenarioError>,
) {
    // Surface gpui/blade platform logs when RUST_LOG is set — the
    // real-window lane depends on the host GPU/X11 stack, so CI
    // failures need the renderer's own diagnostics, not just the
    // capture verdict.
    let _ = env_logger::builder().is_test(false).try_init();
    let base = resolve_theme(theme_name);
    let themed = base.with_scale(ScaleContext {
        scale_factor,
        rem_base_px: 16.0,
    });
    assert_eq!(themed.scale().scale_factor, scale_factor);
    assert_eq!(
        themed.tokens().collect::<Vec<_>>(),
        base.tokens().collect::<Vec<_>>(),
        "{theme_name} tokens must not depend on scale context"
    );
    let view = ViewInstanceId::new(426).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(fixture.spec.clone()).expect("mount story spec");
    host.drain_events();

    let outcome = RealOsGuiTest::run(
        vec![RealWindowMount {
            window: WindowSpec::default(),
            view: ViewMount {
                host,
                theme: Some(themed),
            },
        }],
        RealWindowOptions {
            startup_settle_ticks: STARTUP_SETTLE_TICKS,
            ..RealWindowOptions::default()
        },
        |gui| {
            let window = gui
                .windows()
                .into_iter()
                .next()
                .expect("one mounted window");
            let target = gui
                .resolve_target(&TargetQuery::new(window, view))
                .expect("resolve story target");
            body(gui, window, target)
        },
    );
    match outcome {
        Ok(result) => result.expect("scenario body"),
        Err(error) => session_failure(fixture, theme_name, scale_factor, error),
    }
}

/// Classifies a `RealOsGuiTest::run` session-level failure. `Host`/
/// `BackendUnavailable` mean the real-window session itself could
/// not be established (headless host, session-0, no interactive
/// desktop, platform loop failure) — an *environment* condition,
/// recorded as `observed="environment_blocked"` in the driver-lab
/// evidence JSONL and panicked with an `environment_blocked:` tag so
/// it is never mistaken for a GUI/test failure. Every other
/// `UiError` (`Internal`/`Unsupported`/unknown) stays an ordinary
/// failure — `environment_blocked` is never claimed for it.
fn session_failure(
    fixture: &StoryFixture,
    theme_name: &str,
    scale_factor: f64,
    error: UiError,
) -> ! {
    let cell = format!("{} @ {theme_name} x scale {scale_factor}", fixture.story_id);
    match error {
        UiError::Host { message } | UiError::BackendUnavailable { message } => {
            driver_lab::emit_evidence(
                fixture.story_id,
                "real-os",
                "pass",
                "environment_blocked",
                EVIDENCE_MECHANISM,
                &format!("{cell}: real-window session could not start — {message}"),
            );
            panic!("environment_blocked: {cell} real-window session could not start: {message}");
        }
        other => panic!("{cell} real-window session failed: {other}"),
    }
}

/// The story `GoldenKey` — real story/theme ids, the declared state,
/// and the *measured* scale factor. `theme` is the cell's actual
/// mount theme (not necessarily the story's declared theme);
/// `driver_class` is `real-os`: only pixels from `capture_frame`
/// can produce a `pass`/`fail` verdict on this key.
fn story_golden_key(
    fixture: &StoryFixture,
    theme_name: &str,
    state: &str,
    meta: CaptureMeta,
) -> GoldenKey {
    GoldenKey {
        story_id: fixture.story_id.to_string(),
        theme: theme_name.to_string(),
        state: state.to_string(),
        driver_class: "real-os".to_string(),
        os_class: std::env::consts::OS.to_string(),
        gpu_class: None,
        scale_factor: meta.scale_factor,
        font_class: None,
        revision: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// `EnvironmentMeta` from the *measured* capture — backend tag and
/// scale factor come from the frame, never assumed.
fn story_environment(meta: CaptureMeta) -> EnvironmentMeta {
    EnvironmentMeta {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os_version: None,
        backend: "gpui".to_string(),
        backend_detail: Some(format!(
            "OsCapturedFrame.backend = {} ({})",
            meta.backend,
            capture_backend_note()
        )),
        gpu: None,
        scale_factor: meta.scale_factor,
        font_set: None,
        capture: CapturePath::RealWindow,
        ruviform_revision: env!("CARGO_PKG_VERSION").to_string(),
        tool_revision: None,
    }
}

/// Human-readable note for the OS capture path that produced
/// `meta.backend` — kept beside the measured tag so baseline
/// sidecars record *which* mechanism ran, per OS.
#[cfg(windows)]
fn capture_backend_note() -> &'static str {
    "PrintWindow preferred, screen BitBlt fallback"
}

/// On Linux the pixels come from `GetImage` on the X11 window
/// drawable — under the CI lane that drawable lives on an Xvfb
/// virtual framebuffer (no GPU, no physical display).
#[cfg(target_os = "linux")]
fn capture_backend_note() -> &'static str {
    "X11 GetImage on the window drawable (Xvfb virtual display in CI)"
}

/// Fresh per-run store dir — baselines never persist between runs,
/// so `baseline_missing` → candidate → promote → `pass` is exercised
/// every time.
fn golden_store(tag: &str) -> BaselineStore {
    let dir = std::env::temp_dir().join(format!(
        "ruviform-real-window-golden-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    BaselineStore::new(dir)
}

/// Resolves `RUVIFORM_REAL_WINDOW_PNG_DIR` — relative paths anchor
/// at the workspace root (`CARGO_MANIFEST_DIR/../..`), the same
/// rule `driver_lab::evidence_target` applies to the JSONL evidence
/// env vars, so `cargo test`'s package-root CWD cannot leak outputs
/// into the crate directory.
fn png_evidence_dir() -> Option<PathBuf> {
    let raw = std::env::var("RUVIFORM_REAL_WINDOW_PNG_DIR").ok()?;
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    Some(if path.is_absolute() {
        path
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    })
}

/// Optional durable pixel evidence: `RUVIFORM_REAL_WINDOW_PNG_DIR`
/// names a directory that receives `<story>-<theme>-scale<N>-
/// {idle,teardown}.png` and a `<story>-<theme>-scale<N>-meta.jsonl`
/// line — cell-qualified names so matrix cells never overwrite each
/// other. Unset → no artifacts (same env-gate convention as the
/// driver-lab/golden evidence channels).
fn dump_png_evidence(
    fixture: &StoryFixture,
    theme_name: &str,
    scale_factor: f64,
    idle: &ruviform_ui_test::golden::RgbaImage,
    teardown: &ruviform_ui_test::golden::RgbaImage,
    meta: Option<CaptureMeta>,
) {
    let Some(dir) = png_evidence_dir() else {
        return;
    };
    std::fs::create_dir_all(&dir).expect("png evidence dir");
    let cell = format!("{}-{theme_name}-scale{scale_factor:.1}", fixture.story_id);
    for (name, img) in [("idle", idle), ("teardown", teardown)] {
        let png = img.to_png().expect("encode evidence png");
        std::fs::write(dir.join(format!("{cell}-{name}.png")), png).expect("write evidence png");
    }
    if let Some(meta) = meta {
        let line = format!(
            "{{\"story\":\"{}\",\"theme\":\"{theme_name}\",\"scale\":{scale_factor:.1},\"state\":\"teardown\",\"width\":{},\"height\":{},\"scale_factor\":{},\"backend\":\"{}\"}}\n",
            fixture.story_id, meta.width, meta.height, meta.scale_factor, meta.backend
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{cell}-meta.jsonl")))
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
            .expect("write evidence meta");
    }
}

/// One `{theme, scale}` matrix cell: real-window session → shared
/// oracle walk → idle/teardown pixel capture → story-keyed
/// `golden_check` promote→pass loop → driver-lab evidence row.
fn real_window_cell(fixture: &StoryFixture, theme_name: &str, scale_factor: f64) {
    let staging_root = std::env::temp_dir().join(format!(
        "ruviform-real-window-staging-{}-{}-{}-{}",
        fixture.story_id,
        theme_name,
        scale_factor,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&staging_root).expect("staging dir");

    run_story_session(fixture, theme_name, scale_factor, |gui, window, target| {
        // Pixel layer, idle state: the populated story UI must
        // produce real, non-uniform OS pixels before any gesture.
        let idle_image = match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => image,
            other => panic!(
                "{} @ {theme_name} x scale {scale_factor} idle capture must produce real pixels on a desktop session, got {other:?}",
                fixture.story_id
            ),
        };
        assert!(idle_image.width() > 0 && idle_image.height() > 0);
        let mut distinct = std::collections::BTreeSet::new();
        for y in (0..idle_image.height()).step_by(16) {
            for x in (0..idle_image.width()).step_by(16) {
                if let Some(px) = idle_image.pixel(x, y) {
                    distinct.insert([px[0], px[1], px[2]]);
                }
            }
        }
        assert!(
            distinct.len() > 1,
            "idle frame looks uniform — not real window pixels"
        );

        // Semantic layer: the shared seven-state oracle — identical
        // evidence shape to the Fake/GPUI-simulated layers, asserted
        // per matrix cell.
        let walk_key = fixture.walk_key.clone();
        let teardown = fixture.teardown_spec.clone();
        let trace = lab_story::lab_story_walk(gui, &target, &walk_key, move |g| {
            g.update_view(window, teardown)
        })?;
        assert_eq!(
            trace,
            lab_story::expected_lab_story_trace(),
            "{} story trace diverged @ {theme_name} x scale {scale_factor}",
            fixture.story_id,
        );

        // Pixel layer, teardown state: the target pins survive the
        // spec swap (generation stays stable), so capture measures
        // the post-teardown window — real OS pixels, never a draw
        // plan. (The fader story's teardown spec is an empty panel —
        // uniform pixels are legitimate here; the non-uniform check
        // lives on the populated idle frame above.)
        let image = match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => image,
            other => panic!(
                "{} @ {theme_name} x scale {scale_factor} teardown capture must produce real pixels on a desktop session, got {other:?}",
                fixture.story_id
            ),
        };
        assert!(image.width() > 0 && image.height() > 0);
        dump_png_evidence(
            fixture,
            theme_name,
            scale_factor,
            &idle_image,
            &image,
            gui.last_capture_meta(),
        );
        golden_loop(
            gui,
            &target,
            fixture,
            theme_name,
            scale_factor,
            &image,
            &staging_root,
        )
    });
}

/// Story-keyed golden loop for one cell: `golden_check` on a fresh
/// store → `blocked`/`baseline_missing` (proof the pixels reached
/// `BaselineStore::compare`) → candidate → promote → second
/// `golden_check` `pass` — the promote→compare loop proves
/// repeatability, not a one-off framebuffer read — then the
/// driver-lab evidence row.
fn golden_loop(
    gui: &mut RealOsGuiTest,
    target: &ScenarioTarget,
    fixture: &StoryFixture,
    theme_name: &str,
    scale_factor: f64,
    image: &ruviform_ui_test::golden::RgbaImage,
    staging_root: &Path,
) -> Result<(), ScenarioError> {
    let meta = gui.last_capture_meta().expect("capture meta");
    let key = story_golden_key(fixture, theme_name, "teardown", meta);
    let env = story_environment(meta);
    let store = golden_store(&format!("{}-{theme_name}-{scale_factor}", fixture.story_id));
    let policy = DiffPolicy {
        channel_tolerance: ChannelTolerance::uniform(2),
        mask: Vec::new(),
    };
    let approval = BaselineApproval {
        approved_by: "real-window-test".to_string(),
        note: Some("self-approved e2e baseline for slice 4 verification".to_string()),
    };
    let mut report = ScenarioReport::new(
        RunId::new(format!(
            "run-{}-{theme_name}-{scale_factor}",
            fixture.story_id
        )),
        ScenarioId::new("lab-story-real-window"),
        None,
        gui.driver(),
    );

    // Golden layer step 1: story-keyed `golden_check` on a fresh
    // store → `blocked`/`baseline_missing` — proof the pixels
    // reached `BaselineStore::compare` (an `Unsupported`/
    // `NotRun` outcome would name a different verdict).
    let comparison = gui.golden_check(target, &key, Some(&env), &store)?;
    assert_eq!(comparison.report.verdict, GoldenVerdict::Blocked);
    assert_eq!(comparison.report.reason, "baseline_missing");
    record_golden_step(&mut report, &comparison);

    // Candidate → promote with the *same* measured pixels, then
    // a second `golden_check` captures fresh pixels and must
    // `pass`.
    store
        .write_candidate(staging_root, &key, image, Some(&env), &policy)
        .map_err(|e| ScenarioError::Driver(format!("write_candidate: {e}")))?;
    store
        .promote(staging_root, &key, &approval)
        .map_err(|e| ScenarioError::Driver(format!("promote: {e}")))?;
    gui.settle()?;
    let comparison = gui.golden_check(target, &key, Some(&env), &store)?;
    assert_eq!(
        comparison.report.verdict,
        GoldenVerdict::Pass,
        "second compare must pass against promoted baseline @ {theme_name} x scale {scale_factor}, got {:?}",
        comparison.report.stats
    );
    record_golden_step(&mut report, &comparison);

    // Story-keyed evidence: semantic + pixel layers both `pass`,
    // mechanism names the real-window lane honestly.
    assert_eq!(report.evidence.len(), 2);
    for evidence in &report.evidence {
        assert!(evidence.contains(fixture.story_id), "{evidence}");
        assert!(evidence.contains("real-os"), "{evidence}");
    }
    driver_lab::emit_evidence(
        fixture.story_id,
        "real-os",
        "pass",
        "pass",
        EVIDENCE_MECHANISM,
        &format!(
            "RealOsGuiTest: 7-state shared oracle + real OS-window pixel capture → golden_check (baseline_missing → promote → pass) @ {theme_name} x scale {scale_factor}"
        ),
    );
    Ok(())
}

/// The story matrix for one fixture: every {theme, scale} cell —
/// the residual scale-2.0 and second-theme cells included — runs
/// its own real-window session.
fn real_window_story(fixture: &StoryFixture) {
    for theme_name in THEMES {
        for scale_factor in SCALES {
            real_window_cell(fixture, theme_name, scale_factor);
        }
    }
}

#[test]
fn lab_story_sv_fader_dock_real_window() {
    real_window_story(&lab_story::sv_fader_dock_fixture());
}

#[test]
fn lab_story_al_waveform_seek_real_window() {
    real_window_story(&lab_story::al_waveform_seek_fixture());
}

// ---------------------------------------------------------------------
// Issue #313 — `RealOsPointerOrKeyboard` OS-injection scenario.
//
// This lane is the honest OS-input chain, distinct from every other
// layer: `RealOsGuiTest::os_*` ops drive `os_input` (Win32 `SendInput`
// — OS automation injection, *not* physical manual input and *not* the
// GPUI `dispatch_keystroke`/`simulate_*` or `dispatch_input` semantic
// paths). Every op is gated by the Issue #552 `os_input::preflight`
// effect probe; a host where `SendInput` accepts events that never
// take effect records `environment_blocked`, never a pass.
//
// Chain proven on Windows: open real window → `request_focus` →
// observed `Focused(true)` → `os_press_key` (Tab/Right/Tab/Left) →
// `os_pointer_move_to`/`os_mouse_down`/`os_mouse_up` (measured
// `component_bounds` × `window_geometry` aim + cursor read-back) →
// IPC intents minted by the real bindings → receipt posted back →
// `observe` `CommandPhase::Applied` → teardown. `run_scenario`
// supplies capability pre-flight + report assembly.
//
// On Linux/macOS the capability is not claimed, `require_capabilities`
// fails honestly, and the lane records `unsupported` — the same test
// runs under the Xvfb lane and must not pretend input happened.
// ---------------------------------------------------------------------

/// Env var gating the capability-matrix verdict JSON — same
/// relative-to-workspace anchor as `png_evidence_dir`.
const OS_INPUT_VERDICT_ENV: &str = "RUVIFORM_REAL_OS_INPUT_VERDICT";

/// Driver-lab mechanism tag for this lane.
#[cfg(windows)]
const OS_INPUT_MECHANISM: &str = "mise-task:test-ui-real-os-scenario";
#[cfg(target_os = "linux")]
const OS_INPUT_MECHANISM: &str = "mise-task:test-ui-offscreen-xvfb";

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn a11y_slider(label: &str, o: Orientation) -> A11ySpec {
    A11ySpec {
        role: Some(A11yRole::Slider),
        label: Some(label.to_string()),
        orientation: Some(o),
        value_min: Some(0.0),
        value_max: Some(1.0),
        value_text: None,
    }
}

fn os_input_control(name: &str, label: &str, o: Orientation) -> ControlSpec {
    ControlSpec {
        key: key(name),
        control_id: None,
        target: Some(format!("mixer.{name}")),
        descriptor: ValueDescriptor {
            default_value: Some(0.5),
            step: Some(0.05),
            ..ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: o,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: a11y_slider(label, o),
    }
}

/// The input spec — same two-control panel the `SendInput` smoke
/// examples drive: Tab traverses `gain` → `seek`, arrows step the
/// focused control (#479 convention), pointer drags/clicks aim at
/// painted bounds.
fn os_input_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF313 OS input scenario".to_string()),
            children: vec![
                ComponentSpec::Fader(os_input_control("gain", "Gain", Orientation::Vertical)),
                ComponentSpec::Seek(os_input_control("seek", "Seek", Orientation::Horizontal)),
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// Resolves `RUVIFORM_REAL_OS_INPUT_VERDICT` relative to the
/// workspace root — `None` disables the file (env-gate convention).
fn os_input_verdict_path() -> Option<PathBuf> {
    let raw = std::env::var(OS_INPUT_VERDICT_ENV).ok()?;
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    Some(if path.is_absolute() {
        path
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    })
}

/// Host OS/GPU metadata for the verdict record — PowerShell probe on
/// Windows (same approach as the smoke examples), unavailable tags
/// elsewhere.
#[cfg(windows)]
fn os_input_host_env() -> Value {
    let script = "$os = Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty Caption; $gpu = Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name; [pscustomobject]@{ os_version = $os; gpu = $gpu } | ConvertTo-Json -Compress";
    let parsed = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()
        .and_then(|out| serde_json::from_slice::<Value>(&out.stdout).ok());
    parsed.unwrap_or_else(|| json!({"os_version": "unavailable", "gpu": "unavailable"}))
}

#[cfg(target_os = "linux")]
fn os_input_host_env() -> Value {
    json!({"os_version": "unavailable", "gpu": "unavailable"})
}

/// Writes the `rf313-real-os-input-scenario/v1` verdict JSON when
/// the env gate names a path. `verdict` uses the task vocabulary
/// (`pass`/`fail`/`not_run`/`unsupported`/`environment_blocked`).
fn write_os_input_verdict(
    verdict: &str,
    reasons: &[String],
    environment: &Value,
    preflight: Option<&PreflightReport>,
    run_log: &[Value],
    report: Option<&ScenarioReport>,
) {
    let Some(path) = os_input_verdict_path() else {
        return;
    };
    let doc = json!({
        "schema": "rf313-real-os-input-scenario/v1",
        "issue": 313,
        "lane": "windows-sendinput-scenario",
        "driver": "real-os",
        "capability": "real-os-pointer-or-keyboard",
        "input_method": "win32-SendInput",
        "input_method_note": "OS automation injection through the real foreground path — not physical manual input; distinct from GPUI dispatch_keystroke/simulate_* and from ViewHost semantic dispatch",
        "verdict": verdict,
        "reasons": reasons,
        "environment": environment,
        "preflight": preflight.map(|p| json!({
            "effective": p.effective,
            "checks": p.checks.iter().map(|c| json!({
                "family": c.family,
                "effective": c.effective,
                "detail": c.detail,
            })).collect::<Vec<_>>(),
        })),
        "run": run_log,
        "scenario_report": report.map(|r| json!({
            "driver": format!("{:?}", r.driver),
            "verdict": r.verdict.as_str(),
            "detail": r.detail,
            "required": r.required.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>(),
            "capabilities": r.capabilities.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>(),
            "timeout": r.timeout.map(|t| json!({"budget_ms": t.budget_ms, "elapsed_ms": t.elapsed_ms, "timed_out": t.timed_out})),
        })),
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("verdict dir");
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).expect("verdict json"),
    )
    .unwrap_or_else(|e| panic!("write verdict {}: {e}", path.display()));
}

/// `(control_id, normalized, atomic, gesture_id)` projections of the
/// commit intents the run log analysis needs.
fn commit_rows(intents: &[ruviform_ui_test::MintedIntent]) -> Vec<(String, f64, bool, String)> {
    intents
        .iter()
        .filter_map(|m| match &m.intent {
            CommandIntent::Commit {
                control_id,
                value,
                atomic,
                gesture_id,
                ..
            } => Some((
                control_id.clone(),
                value.normalized,
                *atomic,
                format!("{gesture_id:?}"),
            )),
            _ => None,
        })
        .collect()
}

/// `(control_id, normalized, atomic, gesture_id)` for Begin intents —
/// a keyboard step pairs Begin+Commit `atomic: true`; a pointer
/// gesture pairs non-atomic Begin+Commit on one `gesture_id`.
fn begin_rows(intents: &[ruviform_ui_test::MintedIntent]) -> Vec<(String, f64, bool, String)> {
    intents
        .iter()
        .filter_map(|m| match &m.intent {
            CommandIntent::Begin {
                control_id,
                value,
                atomic,
                gesture_id,
                ..
            } => Some((
                control_id.clone(),
                value.normalized,
                *atomic,
                format!("{gesture_id:?}"),
            )),
            _ => None,
        })
        .collect()
}

/// The `RealOsPointerOrKeyboard` E2E scenario — Windows measures the
/// full OS-injection chain; other OSes verify the capability is
/// honestly unclaimed.
#[test]
fn real_os_input_scenario() {
    let _ = env_logger::builder().is_test(false).try_init();
    let view = ViewInstanceId::new(313).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(os_input_spec()).expect("mount input spec");
    host.drain_events();

    let outcome = RealOsGuiTest::run(
        vec![RealWindowMount {
            window: WindowSpec::default(),
            view: ViewMount {
                host,
                theme: Some(resolve_theme("synvelis-dark")),
            },
        }],
        RealWindowOptions {
            startup_settle_ticks: STARTUP_SETTLE_TICKS,
            ..RealWindowOptions::default()
        },
        |gui| os_input_session(gui, view),
    );
    if let Err(error) = outcome {
        let message = error.to_string();
        let environment = os_input_environment(None);
        write_os_input_verdict(
            "environment_blocked",
            &[format!("real-window session could not start: {message}")],
            &environment,
            None,
            &[],
            None,
        );
        driver_lab::emit_evidence(
            "rf313-real-os-input",
            "real-os",
            "pass",
            "environment_blocked",
            OS_INPUT_MECHANISM,
            &format!("real-window session could not start: {message}"),
        );
        panic!("environment_blocked: real-window session could not start: {message}");
    }
}

/// Environment record for the verdict JSON — measured values only;
/// `scale_factor` comes from the real capture meta when available.
fn os_input_environment(scale_factor: Option<f64>) -> Value {
    let host = os_input_host_env();
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "os_version": host["os_version"],
        "gpu": host["gpu"],
        "scale_factor": scale_factor,
        "revision": env!("CARGO_PKG_VERSION"),
        "timestamp_unix": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    })
}

/// One early-out lane record: verdict JSON + driver-lab evidence row.
fn os_input_lane_result(
    verdict: &str,
    reasons: &[String],
    environment: &Value,
    preflight: Option<&PreflightReport>,
) {
    write_os_input_verdict(verdict, reasons, environment, preflight, &[], None);
    driver_lab::emit_evidence(
        "rf313-real-os-input",
        "real-os",
        "pass",
        verdict,
        OS_INPUT_MECHANISM,
        &reasons.join("; "),
    );
}

/// Session body: capability gate → focus → preflight → `run_scenario`
/// → verdict JSON + driver-lab evidence + the test's own assertion.
fn os_input_session(gui: &mut RealOsGuiTest, view: ViewInstanceId) {
    let window = gui
        .windows()
        .into_iter()
        .next()
        .expect("one mounted window");
    let target = gui
        .resolve_target(&TargetQuery::new(window, view))
        .expect("resolve input target");
    let environment = os_input_environment(None);

    // Capability honesty: no backend → require fails → `unsupported`.
    if !gui
        .capabilities()
        .contains(&ScenarioCapability::RealOsPointerOrKeyboard)
    {
        os_input_lane_result(
            "unsupported",
            &["adapter does not claim real-os-pointer-or-keyboard on this OS".to_string()],
            &environment,
            None,
        );
        return;
    }

    // Foreground window is a hard precondition for keyboard/button
    // delivery — refuse to inject at an unfocused window.
    gui.request_focus(window, true)
        .expect("queue focus request");
    let focused = settle_until(gui, |g| g.window_focused(window), 40).expect("settle for focus");
    if !focused {
        let reasons = vec![format!("window {window} never reported Focused(true)")];
        os_input_lane_result("not_run", &reasons, &environment, None);
        panic!("not_run: {}", reasons.join("; "));
    }

    // Issue #552 gate: injection acceptance ≠ effect.
    let preflight = gui.os_input_preflight(PreflightScope::PointerAndKeyboard);
    if !preflight.effective {
        let reasons: Vec<String> = preflight
            .checks
            .iter()
            .filter(|c| !c.effective)
            .map(|c| format!("{}: {}", c.family, c.detail))
            .collect();
        os_input_lane_result(
            "environment_blocked",
            &reasons,
            &environment,
            Some(&preflight),
        );
        panic!("environment_blocked: {}", reasons.join("; "));
    }

    // Preserve the user's cursor — the session restores it on exit.
    let saved_cursor = os_input::cursor_position().ok();

    let spec = ScenarioSpec::new(
        RunId::new("run-rf313-os-input"),
        ScenarioId::new("rf313-real-os-input"),
    )
    .require(ScenarioCapability::RealOsPointerOrKeyboard)
    .require(ScenarioCapability::SemanticAction)
    .timeout_ms(60_000);
    let report = run_scenario(gui, &spec, |gui| os_input_scenario_body(gui, &target));

    // Best-effort cursor restore — evidence already records it.
    if let Some((x, y)) = saved_cursor {
        let _ = os_input::move_to(x, y);
    }

    let verdict = match report.verdict {
        ScenarioVerdict::Blocked => "environment_blocked",
        other => other.as_str(),
    };
    let reasons: Vec<String> = report.detail.iter().cloned().collect();
    let run_log: Vec<Value> = report
        .observed
        .as_ref()
        .and_then(|v| v.get("run_log").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();
    write_os_input_verdict(
        verdict,
        &reasons,
        &os_input_environment(gui.last_capture_meta().map(|m| m.scale_factor)),
        Some(&preflight),
        &run_log,
        Some(&report),
    );
    driver_lab::emit_evidence(
        "rf313-real-os-input",
        "real-os",
        "pass",
        verdict,
        OS_INPUT_MECHANISM,
        &format!(
            "RealOsPointerOrKeyboard OS-injection scenario: {}",
            report.detail.clone().unwrap_or_else(|| verdict.to_string())
        ),
    );
    assert_eq!(
        report.verdict,
        ScenarioVerdict::Pass,
        "OS-injection scenario must pass on an unblocked Windows host: {:?}",
        report.detail
    );
}

/// Records one `os_*` outcome into the run log. `Err` propagates —
/// contract failures are `fail`. `Ok(non-Injected)` records the
/// honest lane verdict the sequence must halt with.
fn record_os_step(
    tag: &str,
    result: Result<OsInputOutcome, ScenarioError>,
    log: &mut Vec<Value>,
    problems: &mut Vec<String>,
) -> Result<Option<ScenarioVerdict>, ScenarioError> {
    match result? {
        OsInputOutcome::Injected {
            action,
            commanded,
            cursor,
            detail,
        } => {
            log.push(json!({
                "channel": "os-injection", "op": tag, "action": action,
                "commanded": commanded, "cursor": cursor, "detail": detail,
            }));
            Ok(None)
        }
        OsInputOutcome::EnvironmentBlocked { detail } => {
            log.push(json!({"channel": "os-injection", "op": tag, "outcome": "environment_blocked", "detail": detail}));
            problems.push(format!("{tag}: environment_blocked — {detail}"));
            Ok(Some(ScenarioVerdict::Blocked))
        }
        OsInputOutcome::Unsupported { reason } => {
            log.push(json!({"channel": "os-injection", "op": tag, "outcome": "unsupported", "detail": reason}));
            problems.push(format!("{tag}: unsupported — {reason}"));
            Ok(Some(ScenarioVerdict::Unsupported))
        }
        OsInputOutcome::NotRun { reason } => {
            log.push(json!({"channel": "os-injection", "op": tag, "outcome": "not_run", "detail": reason}));
            problems.push(format!("{tag}: not_run — {reason}"));
            Ok(Some(ScenarioVerdict::NotRun))
        }
        other => {
            log.push(
                json!({"channel": "os-injection", "op": tag, "outcome": format!("{other:?}")}),
            );
            problems.push(format!("{tag}: unexpected outcome {other:?}"));
            Ok(Some(ScenarioVerdict::Fail))
        }
    }
}

/// The scenario body run by `run_scenario`: keyboard traversal +
/// step commits, a pointer drag on `gain`, a click on `seek`, then
/// the receipt/display leg. Returns the honest verdict —
/// `EnvironmentBlocked`/`NotRun`/`Unsupported` outcomes never reach
/// `Err` (they are evidence, not contract failures).
fn os_input_scenario_body(
    gui: &mut RealOsGuiTest,
    target: &ScenarioTarget,
) -> Result<ScenarioOutcome, ScenarioError> {
    let mut run_log: Vec<Value> = vec![];
    let mut problems: Vec<String> = vec![];
    let mut stop: Option<ScenarioVerdict> = None;

    // Run the op, record its outcome, settle two drain boundaries so
    // the OS-delivered event lands before the next injection.
    macro_rules! os_step {
        ($gui:expr, $tag:literal, $result:expr) => {{
            let result = $result;
            if stop.is_none()
                && let Some(v) = record_os_step($tag, result, &mut run_log, &mut problems)?
            {
                stop = Some(v);
            }
            $gui.settle()?;
            $gui.settle()?;
        }};
    }

    // Keyboard leg — Issue #451 traversal + #479 step commits.
    os_step!(gui, "tab-1", gui.os_press_key(target, OsKey::Tab));
    os_step!(gui, "right", gui.os_press_key(target, OsKey::Right));
    os_step!(gui, "tab-2", gui.os_press_key(target, OsKey::Tab));
    os_step!(gui, "left", gui.os_press_key(target, OsKey::Left));

    // Pointer leg — drag `gain` down-to-up, click `seek` near 0.8.
    os_step!(
        gui,
        "gain-move",
        gui.os_pointer_move_to(target, &key("gain"), (0.5, 0.75))
    );
    os_step!(gui, "gain-down", gui.os_mouse_down(target));
    os_step!(
        gui,
        "gain-drag",
        gui.os_pointer_move_to(target, &key("gain"), (0.5, 0.25))
    );
    os_step!(gui, "gain-up", gui.os_mouse_up(target));
    os_step!(
        gui,
        "seek-move",
        gui.os_pointer_move_to(target, &key("seek"), (0.8, 0.5))
    );
    os_step!(gui, "seek-down", gui.os_mouse_down(target));
    os_step!(gui, "seek-up", gui.os_mouse_up(target));

    if let Some(verdict) = stop {
        let mut outcome = ScenarioOutcome::new(verdict);
        outcome.observed = Some(json!({"run_log": run_log}));
        outcome.detail = Some(problems.join("; "));
        return Ok(outcome);
    }

    // IPC leg — the real bindings minted intents from OS input.
    let intents = gui.drain_intents();
    let commits = commit_rows(&intents);
    run_log.push(json!({
        "channel": "ipc",
        "intents": intents.iter().map(|m| json!({
            "command_id": m.command_id,
            "expect_receipt": m.expect_receipt,
            "intent": format!("{:?}", m.intent),
        })).collect::<Vec<_>>(),
    }));
    problems.extend(check_os_input_intents(&intents, &commits));

    let receipts = os_input_receipt_leg(gui, target, &intents, &mut run_log, &mut problems)?;

    // Display leg — a real OS capture proves the window is live
    // (capture is #314 evidence; here it documents the display end).
    match gui.capture_frame(target)? {
        CaptureOutcome::Captured(_) => {
            run_log.push(json!({"channel": "os-capture", "outcome": "captured"}));
        }
        other => {
            run_log.push(json!({"channel": "os-capture", "outcome": format!("{other:?}")}));
            problems.push(format!("capture_frame did not produce pixels: {other:?}"));
        }
    }

    let mut outcome = ScenarioOutcome::new(if problems.is_empty() {
        ScenarioVerdict::Pass
    } else {
        ScenarioVerdict::Fail
    });
    outcome.targets = vec![target.clone()];
    outcome.receipts = receipts;
    outcome.expected = Some(json!({
        "gain": {"keyboard_commit": 0.55, "pointer_commit": 0.75},
        "seek": {"keyboard_commit": 0.45, "pointer_commit": 0.80},
        "receipt_phase": "Applied",
        "channels": ["os-injection", "ipc", "os-capture"],
    }));
    outcome.observed = Some(json!({
        "run_log": run_log,
        "commits": commits.iter().map(|(c, v, a, g)| json!({
            "control_id": c, "normalized": v, "atomic": a, "gesture": g,
        })).collect::<Vec<_>>(),
    }));
    outcome
        .evidence
        .push("verdict:rf313-real-os-input-scenario".to_string());
    outcome.detail = if problems.is_empty() {
        Some(
            "keyboard + pointer OS-injection chain, intents, receipt and display legs verified"
                .to_string(),
        )
    } else {
        Some(problems.join("; "))
    };
    Ok(outcome)
}

/// Verifies the minted intents against the input oracle: keyboard
/// steps are atomic Begin+Commit pairs (gain 0.55, seek 0.45);
/// pointer gestures are non-atomic Begin+Commit on one `gesture_id`
/// (gain ≈0.75, seek ≈0.80). Returns the problems found.
fn check_os_input_intents(
    intents: &[ruviform_ui_test::MintedIntent],
    commits: &[(String, f64, bool, String)],
) -> Vec<String> {
    let begins = begin_rows(intents);
    let mut problems = Vec::new();
    let approx = |v: f64, want: f64| (v - want).abs() <= 0.02;
    for (control, want) in [("gain", 0.55), ("seek", 0.45)] {
        let Some((_, got, atomic, _)) = commits.iter().find(|(c, _, _, _)| c == control) else {
            problems.push(format!("no commit intent observed for {control}"));
            continue;
        };
        if !approx(*got, want) {
            problems.push(format!("{control} commit {got} != expected {want}"));
        }
        if !atomic {
            problems.push(format!("{control} keyboard commit not atomic"));
        }
        if !begins.iter().any(|(c, _, a, _)| c == control && *a) {
            problems.push(format!("{control} keyboard begin missing/not atomic"));
        }
    }
    for (control, want) in [("gain", 0.75), ("seek", 0.80)] {
        let commit = commits
            .iter()
            .rev()
            .find(|(c, _, atomic, _)| c == control && !atomic);
        let Some((_, got, _, gesture)) = commit else {
            problems.push(format!("no non-atomic commit observed for {control}"));
            continue;
        };
        if !approx(*got, want) {
            problems.push(format!("{control} pointer commit {got} != expected {want}"));
        }
        if !begins
            .iter()
            .any(|(c, _, a, g)| c == control && !a && g == gesture)
        {
            problems.push(format!(
                "{control} pointer begin missing for gesture {gesture}"
            ));
        }
    }
    problems
}

/// Receipt leg — answers the binding's `expect_receipt` commit for
/// `seek`, then the view must observe `CommandPhase::Applied`.
/// Returns the receipts recorded onto the outcome.
fn os_input_receipt_leg(
    gui: &mut RealOsGuiTest,
    target: &ScenarioTarget,
    intents: &[ruviform_ui_test::MintedIntent],
    run_log: &mut Vec<Value>,
    problems: &mut Vec<String>,
) -> Result<Vec<CommandReceipt>, ScenarioError> {
    let mut receipts = Vec::new();
    // The head record is the newest receipt-expecting command — the
    // receipt must answer it for the phase to read `Applied`.
    if let Some(minted) = intents.iter().rev().find(|m| {
        m.expect_receipt
            && matches!(&m.intent, CommandIntent::Commit { control_id, .. }
                if control_id == "seek")
    }) {
        let event = Event::result_ok(
            minted.command_id.clone(),
            json!({"status": "applied", "revision": 1}),
        );
        if let Some(receipt) = CommandReceipt::from_result(&event) {
            receipts.push(receipt);
        }
        gui.post_event(target, &event)?;
        gui.settle()?;
        gui.settle()?;
        let snapshot = gui.observe(target, &key("seek"))?;
        run_log.push(json!({
            "channel": "ipc",
            "receipt": "posted",
            "seek_phase": format!("{:?}", snapshot.phase),
        }));
        if snapshot.phase != CommandPhase::Applied {
            problems.push(format!(
                "seek phase after receipt is {:?}, expected Applied",
                snapshot.phase
            ));
        }
    } else {
        problems.push("no expect_receipt commit for seek — receipt leg unproven".to_string());
    }
    Ok(receipts)
}
