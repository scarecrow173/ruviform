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

use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpochAllocator, ViewHost};
use ruviform_ui::{UiError, WindowId, WindowSpec};
use ruviform_ui_gpui::{CaptureMeta, RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::driver_lab;
use ruviform_ui_test::golden::{
    BaselineApproval, BaselineStore, CapturePath, ChannelTolerance, DiffPolicy, EnvironmentMeta,
    GoldenKey, GoldenVerdict,
};
use ruviform_ui_test::lab_story::{self, StoryFixture};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    RunId, ScenarioError, ScenarioId, ScenarioReport, ScenarioTarget, SemanticGuiTest, TargetQuery,
    record_golden_step,
};
use ruviform_ui_theme::{ResolvedTheme, ScaleContext, ThemeResolver, TokenDocument};

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
