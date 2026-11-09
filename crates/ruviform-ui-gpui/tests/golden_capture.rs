//! Issue #314 slice 3 — real-OS window pixel capture → golden compare.
//!
//! This target is empty unless built on Windows with the real `gpui`
//! backend (`cargo test -p ruviform-ui-gpui --features gpui --test
//! golden_capture`). The e2e test opens a real OS window through
//! `GpuiDriver::run`, captures its client-area pixels via
//! `GpuiDriver::capture_window` (`PrintWindow` primary, screen
//! `BitBlt` fallback), feeds the frame into `RgbaImage` →
//! `BaselineStore::compare`, and verifies the candidate → promote →
//! `pass` loop plus evidence emission.
//!
//! The capture measures real window pixels: the window must stay
//! visible while the test runs (it is created, captured, and closed
//! within the same event-loop turn sequence).

#![cfg(all(windows, feature = "gpui"))]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11ySpec, AttachEpoch, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec,
    Orientation, PanelSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{DriverEvent, UiError, WindowDriver, WindowId, WindowLifecycle, WindowSpec};
use ruviform_ui_gpui::{GpuiDriver, OsCapturedFrame, ViewMount};
use ruviform_ui_test::golden::{
    BaselineApproval, BaselineStore, CapturePath, ChannelTolerance, DiffPolicy, EnvironmentMeta,
    GoldenComparison, GoldenEvidence, GoldenKey, RgbaImage, append_evidence,
};
use ruviform_ui_test::lab_story;
use ruviform_ui_theme::{ResolvedTheme, ThemeResolver, TokenDocument};

/// Ticks to let the window finish opening animations before the
/// first capture. Measured on Windows: the first ~200 ms after
/// `Ready` still carries a settling transition (a small region near
/// the bottom edge repaints once, then frames are bit-identical),
/// so 30 ticks (~0.5 s) keeps the baseline deterministic.
const SETTLE_TICKS: u32 = 30;

/// Hard bound so a stuck event loop cannot hang the test forever —
/// recorded as a failure, not a hang.
const MAX_TICKS: u32 = SETTLE_TICKS + 30;

/// A deliberately *static* view — golden baselines require
/// frame-to-frame pixel stability, and the parity fixture's
/// Spectrogram/TimeAxis produce small per-frame diffs. Two vertical
/// Faders at fixed values render non-uniform pixels with no
/// animation surface.
fn static_view() -> ViewSpec {
    fn fader(name: &str) -> ComponentSpec {
        ComponentSpec::Fader(ControlSpec {
            key: ComponentKey(name.to_string()),
            control_id: None,
            target: Some(format!("mixer.{name}")),
            descriptor: ValueDescriptor {
                min: 0.0,
                max: 1.0,
                default_value: Some(0.5),
                step: Some(0.05),
                ..ValueDescriptor::ranged(0.0, 1.0)
            },
            orientation: Orientation::Vertical,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    }
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("root".to_string()),
            title: None,
            children: vec![fader("ch1"), fader("ch2")],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn mount_window_with(
    driver: &mut GpuiDriver,
    view: ViewSpec,
    theme: Option<ResolvedTheme>,
) -> WindowId {
    let mut host = ViewHost::new(
        ViewInstanceId::new(5150).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(view).expect("mount view");
    host.drain_events();
    driver
        .create_view_window(&WindowSpec::default(), ViewMount { host, theme })
        .expect("create_view_window")
}

fn golden_key(scale_factor: f64) -> GoldenKey {
    GoldenKey {
        story_id: "golden-capture-probe".to_string(),
        theme: "default".to_string(),
        state: "idle".to_string(),
        driver_class: "gpui-real-window-os-capture".to_string(),
        os_class: "windows".to_string(),
        gpu_class: None,
        scale_factor,
        font_class: None,
        revision: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// The story `GoldenKey` for the real-window lane — the canonical
/// story/theme ids from the shared story fixture, the `idle`
/// state, and the *measured* window scale factor (built lazily once
/// the first frame reports it). `driver_class` names the capture
/// path, not a driver claim: OS input remains Issue #313.
fn story_golden_key(story_id: &str, theme: &str, scale_factor: f64) -> GoldenKey {
    GoldenKey {
        story_id: story_id.to_string(),
        theme: theme.to_string(),
        state: "idle".to_string(),
        driver_class: "gpui-real-window-os-capture".to_string(),
        os_class: "windows".to_string(),
        gpu_class: None,
        scale_factor,
        font_class: None,
        revision: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Resolves a theme fixture from the shared `ruviform-ui-test` theme
/// JSON — the same documents the Fake/GPUI-simulated matrices use.
fn resolve_theme(name: &str) -> ResolvedTheme {
    let path = format!(
        "{}/../ruviform-ui-test/fixtures/themes/{name}.dtcg.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let doc = TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    ThemeResolver::resolve(&[doc], "dark").unwrap_or_else(|e| panic!("resolve {name}: {e}"))
}

fn environment(scale_factor: f64, backend: &str) -> EnvironmentMeta {
    EnvironmentMeta {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os_version: None,
        backend: backend.to_string(),
        backend_detail: Some(format!(
            "OsCapturedFrame.backend = {backend} (PrintWindow preferred, screen BitBlt fallback)"
        )),
        gpu: None,
        scale_factor,
        font_set: None,
        capture: CapturePath::RealWindow,
        ruviform_revision: env!("CARGO_PKG_VERSION").to_string(),
        tool_revision: None,
    }
}

fn to_rgba(frame: &OsCapturedFrame) -> RgbaImage {
    RgbaImage::new(frame.width, frame.height, frame.pixels.clone())
        .expect("captured frame is well-formed RGBA8")
}

/// Captures `window` and compares the frame against `key` — the
/// `capture → RgbaImage → BaselineStore::compare` path under test.
fn capture_and_compare(
    driver: &GpuiDriver,
    window: WindowId,
    store: &BaselineStore,
    key: &GoldenKey,
    env: &EnvironmentMeta,
) -> Result<(OsCapturedFrame, GoldenComparison), String> {
    let frame = driver
        .capture_window(window)
        .map_err(|e| format!("capture_window: {e}"))?;
    ensure_nonuniform(&frame).map_err(|e| format!("capture_window: {e}"))?;
    let image = to_rgba(&frame);
    let comparison = store
        .compare(key, &image, Some(env))
        .map_err(|e| format!("compare: {e}"))?;
    Ok((frame, comparison))
}

/// Guards against a silently blank capture — `PrintWindow` can
/// return an all-black frame for windows it cannot render, which
/// would still compare `pass` against itself. The static fixture
/// view is non-uniform by construction, so a frame with fewer than
/// 3 distinct sampled colors is a capture failure, not a verdict.
fn ensure_nonuniform(frame: &OsCapturedFrame) -> Result<(), String> {
    let mut colors = std::collections::HashSet::new();
    for px in frame.pixels.as_chunks::<4>().0 {
        colors.insert((px[0], px[1], px[2]));
        if colors.len() >= 3 {
            return Ok(());
        }
    }
    Err(format!(
        "captured frame is uniform ({}x{}) — suspect blank capture",
        frame.width, frame.height
    ))
}

/// Builds the `(GoldenKey, EnvironmentMeta)` pair once the first
/// frame reports the *measured* `scale_factor`/`backend` — keys are
/// never assumed.
type KeyEnvBuilder = dyn Fn(f64, &str) -> (GoldenKey, EnvironmentMeta);

/// `Rc`-shared builder — moved into the event-loop closure.
type SharedKeyEnvBuilder = Rc<KeyEnvBuilder>;

/// Step 0 of the e2e flow: first capture → compare (baseline absent →
/// `blocked`) → write candidate → promote → record evidence. The
/// promoted baseline is what the second capture compares against.
/// Returns `Err(reason)` on any failure — the caller turns it into
/// `st.fatal` and exits the loop.
#[expect(clippy::too_many_arguments)]
fn run_first_step(
    st: &mut Outcome,
    capture_driver: &GpuiDriver,
    window: WindowId,
    key_env: &Rc<RefCell<Option<(GoldenKey, EnvironmentMeta)>>>,
    build_key_env: &KeyEnvBuilder,
    store: &BaselineStore,
    staging_root: &Path,
    evidence_path: &Path,
    policy: &DiffPolicy,
    approval: &BaselineApproval,
) -> Result<(), String> {
    let frame = capture_driver
        .capture_window(window)
        .map_err(|e| format!("first capture: {e}"))?;
    ensure_nonuniform(&frame).map_err(|e| format!("first capture: {e}"))?;
    st.first_frame = Some((frame.width, frame.height, frame.scale_factor, frame.backend));
    // `scale_factor`/`backend` are measured from the frame — the key
    // and environment are built lazily here, never assumed.
    {
        let mut ke = key_env.borrow_mut();
        if ke.is_none() {
            *ke = Some(build_key_env(frame.scale_factor, frame.backend));
        }
    }
    let (key, env) = key_env.borrow().as_ref().expect("key/env").clone();
    let image = to_rgba(&frame);
    let comparison = store
        .compare(&key, &image, Some(&env))
        .map_err(|e| format!("first compare: {e}"))?;
    st.first_verdict = Some(format!(
        "{}/{}",
        comparison.report.verdict.as_str(),
        comparison.report.reason.as_str()
    ));
    st.first_stats = comparison.report.stats.as_ref().map(|s| format!("{s:?}"));
    store
        .write_candidate(staging_root, &key, &image, Some(&env), policy)
        .map_err(|e| format!("write_candidate: {e}"))?;
    store
        .promote(staging_root, &key, approval)
        .map_err(|e| format!("promote: {e}"))?;
    let ev = GoldenEvidence::from(&comparison.report);
    append_evidence(evidence_path, &ev)
        .map(|_| ())
        .map_err(|_| "append_evidence (first)".to_string())
}

/// Step 1 of the e2e flow: second capture against the promoted
/// baseline → compare → record evidence.
fn run_second_step(
    st: &mut Outcome,
    capture_driver: &GpuiDriver,
    window: WindowId,
    key_env: &Rc<RefCell<Option<(GoldenKey, EnvironmentMeta)>>>,
    store: &BaselineStore,
    evidence_path: &Path,
) -> Result<(), String> {
    let (key, env) = key_env.borrow().as_ref().expect("key/env").clone();
    let (frame, comparison) = capture_and_compare(capture_driver, window, store, &key, &env)
        .map_err(|e| format!("second capture/compare: {e}"))?;
    st.second_frame = Some((frame.width, frame.height, frame.scale_factor, frame.backend));
    st.second_verdict = Some(format!(
        "{}/{}",
        comparison.report.verdict.as_str(),
        comparison.report.reason.as_str()
    ));
    st.second_stats = comparison.report.stats.as_ref().map(|s| format!("{s:?}"));
    let ev = GoldenEvidence::from(&comparison.report);
    append_evidence(evidence_path, &ev)
        .map(|_| ())
        .map_err(|_| "append_evidence (second)".to_string())
}

/// What the event loop recorded — asserted after `run` returns so a
/// panic inside the platform callback cannot corrupt the loop.
#[derive(Default)]
struct Outcome {
    ready: bool,
    ticks: u32,
    /// Capture steps completed: 0 = none yet, 1 = first compare +
    /// promote done, 2 = second compare + evidence done.
    step: u8,
    resized: Option<(u32, u32)>,
    first_frame: Option<(u32, u32, f64, &'static str)>,
    first_verdict: Option<String>,
    first_stats: Option<String>,
    second_frame: Option<(u32, u32, f64, &'static str)>,
    second_verdict: Option<String>,
    second_stats: Option<String>,
    fatal: Option<String>,
}

#[test]
fn capture_window_rejects_unknown_and_unopened_windows() {
    let mut driver = GpuiDriver::new();
    let unknown = WindowId::new(u64::MAX).expect("nonzero");
    assert!(
        matches!(driver.capture_window(unknown), Err(UiError::Host { .. })),
        "unknown window id must be a host error"
    );
    // Reserved but not yet opened by `run` — no OS window exists.
    let reserved =
        WindowDriver::create_window(&mut driver, &WindowSpec::default()).expect("reserve window");
    assert!(
        matches!(driver.capture_window(reserved), Err(UiError::Host { .. })),
        "unopened window must be a host error"
    );
}

/// The shared real-window capture e2e: open `view` (with `theme`),
/// settle, capture → compare (`blocked`/`baseline_missing`) →
/// candidate → promote → capture → compare (`pass`/`match`), and
/// append both verdicts to the evidence JSONL. `build_key_env` turns
/// the measured `(scale_factor, backend)` into the `GoldenKey` +
/// `EnvironmentMeta` pair — the probe and the story-keyed variants
/// differ only there.
fn run_capture_e2e(
    name: &str,
    view: ViewSpec,
    theme: Option<ResolvedTheme>,
    build_key_env: SharedKeyEnvBuilder,
) {
    let work = std::env::temp_dir().join(format!(
        "ruviform-golden-capture-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos())
    ));
    let baseline_root = work.join("baselines");
    let staging_root = work.join("staging");
    let evidence_path = work.join("evidence.jsonl");
    std::fs::create_dir_all(&baseline_root).expect("baseline dir");
    std::fs::create_dir_all(&staging_root).expect("staging dir");

    let store = BaselineStore::new(&baseline_root);
    let policy = DiffPolicy {
        channel_tolerance: ChannelTolerance::uniform(2),
        mask: Vec::new(),
    };
    let approval = BaselineApproval {
        approved_by: "golden-capture-test".to_string(),
        note: Some("self-approved e2e baseline for slice 3 verification".to_string()),
    };

    let mut driver = GpuiDriver::new();
    let window = mount_window_with(&mut driver, view, theme);
    let capture_driver = driver.clone();
    let outcome = Rc::new(RefCell::new(Outcome::default()));
    // Built lazily on the first captured frame — `scale_factor` is
    // measured, not assumed.
    let key_env = Rc::new(RefCell::new(None::<(GoldenKey, EnvironmentMeta)>));
    // The closure owns the loop-side path; the post-run assertion
    // reads through this clone.
    let evidence_path_out = evidence_path.clone();

    let run_result = driver.run(Some(Duration::from_millis(16)), {
        let outcome = outcome.clone();
        let key_env = key_env.clone();
        move |event, drv| {
            let mut st = outcome.borrow_mut();
            match event {
                DriverEvent::Windowed {
                    window: w,
                    event: WindowLifecycle::Ready,
                } if w == window => st.ready = true,
                DriverEvent::Windowed {
                    window: w,
                    event: WindowLifecycle::Resized { width, height },
                } if w == window => st.resized = Some((width, height)),
                // `step < 2` — once both capture steps are done the
                // queued `exit` is still in flight; further ticks are
                // ignored rather than treated as a fault. `fatal` —
                // a failed step already asked the loop to exit, but
                // queued ticks can still arrive; running the next
                // step then would panic on missing state inside the
                // window procedure (which cannot unwind).
                DriverEvent::Tick if st.ready && st.step < 2 && st.fatal.is_none() => {
                    st.ticks += 1;
                    if st.ticks > MAX_TICKS {
                        st.fatal = Some("event loop did not finish capture steps".to_string());
                        drv.exit();
                        return;
                    }
                    if st.ticks < SETTLE_TICKS {
                        return;
                    }
                    // First capture: baseline is absent → `blocked`;
                    // then candidate → promote so the second capture
                    // compares against a real baseline. Second
                    // capture: compare against the promoted baseline.
                    let result = match st.step {
                        0 => {
                            st.step = 1;
                            run_first_step(
                                &mut st,
                                &capture_driver,
                                window,
                                &key_env,
                                build_key_env.as_ref(),
                                &store,
                                &staging_root,
                                &evidence_path,
                                &policy,
                                &approval,
                            )
                        }
                        1 => {
                            st.step = 2;
                            run_second_step(
                                &mut st,
                                &capture_driver,
                                window,
                                &key_env,
                                &store,
                                &evidence_path,
                            )
                        }
                        _ => Err("unreachable step".to_string()),
                    };
                    match result {
                        Err(e) => {
                            st.fatal = Some(e);
                            drv.exit();
                        }
                        Ok(()) if st.step == 2 => drv.exit(),
                        Ok(()) => {}
                    }
                }
                _ => {}
            }
        }
    });

    let st = outcome.borrow();
    run_result.expect("driver.run");
    verify_outcome(&st, &evidence_path_out);

    let _ = std::fs::remove_dir_all(&work);
}

/// Opens a real OS window — requires an interactive desktop session.
/// Ignored by default so headless/`gpui-test` lanes stay green; run
/// explicitly with `cargo test -p ruviform-ui-gpui --features gpui
/// --test golden_capture -- --ignored` (or `--include-ignored`).
#[test]
#[ignore = "opens a real OS window — needs a desktop session"]
fn real_window_capture_feeds_golden_compare_to_pass() {
    run_capture_e2e(
        "probe",
        static_view(),
        None,
        Rc::new(|scale_factor, backend| {
            (golden_key(scale_factor), environment(scale_factor, backend))
        }),
    );
}

/// Issue #426 — story-keyed real-window capture: the canonical
/// `LAB-SV-FADER-DOCK` spec mounted with its declared
/// `synvelis-dark` theme feeds the #314 capture → compare → promote
/// → compare loop under the story `GoldenKey`. This is pixel
/// evidence for the story's `idle` state on the real-window lane —
/// separate from the semantic oracle (Fake/GPUI-simulated) and from
/// the OS *input* driver (#313). The window's scale factor is
/// measured from the captured frame; the story's declared
/// `scaleFactor: [1.0, 2.0]` matrix on this lane is bounded by what
/// the display actually reports.
#[test]
#[ignore = "opens a real OS window — needs a desktop session"]
fn real_window_story_capture_sv_fader_dock() {
    let fixture = lab_story::sv_fader_dock_fixture();
    let theme = resolve_theme(fixture.theme);
    let (story_id, theme_name) = (fixture.story_id, fixture.theme);
    run_capture_e2e(
        "lab-sv-fader-dock",
        fixture.spec,
        Some(theme),
        Rc::new(move |scale_factor, backend| {
            (
                story_golden_key(story_id, theme_name, scale_factor),
                environment(scale_factor, backend),
            )
        }),
    );
    ruviform_ui_test::driver_lab::emit_evidence(
        story_id,
        "gpui",
        "pass",
        "pass",
        "cargo-test:ruviform-ui-gpui/golden_capture (--features gpui, --ignored, desktop session)",
        "real-window OS pixel capture (Win32 PrintWindow/BitBlt) idle-state golden compare — pixel lane evidence, not the OS input driver (#313)",
    );
}

/// Issue #426 — story-keyed real-window capture for
/// `LAB-AL-WAVEFORM-SEEK` (`audiolens-dark`), same contract as the
/// Synvelis variant above.
#[test]
#[ignore = "opens a real OS window — needs a desktop session"]
fn real_window_story_capture_al_waveform_seek() {
    let fixture = lab_story::al_waveform_seek_fixture();
    let theme = resolve_theme(fixture.theme);
    let (story_id, theme_name) = (fixture.story_id, fixture.theme);
    run_capture_e2e(
        "lab-al-waveform-seek",
        fixture.spec,
        Some(theme),
        Rc::new(move |scale_factor, backend| {
            (
                story_golden_key(story_id, theme_name, scale_factor),
                environment(scale_factor, backend),
            )
        }),
    );
    ruviform_ui_test::driver_lab::emit_evidence(
        story_id,
        "gpui",
        "pass",
        "pass",
        "cargo-test:ruviform-ui-gpui/golden_capture (--features gpui, --ignored, desktop session)",
        "real-window OS pixel capture (Win32 PrintWindow/BitBlt) idle-state golden compare — pixel lane evidence, not the OS input driver (#313)",
    );
}

/// Post-run assertions on the recorded [`Outcome`] plus the emitted
/// evidence JSONL — kept out of the test body for `too_many_lines`.
fn verify_outcome(st: &Outcome, evidence_path: &Path) {
    assert!(st.ready, "window never reached Ready");
    assert!(st.fatal.is_none(), "fatal: {:?}", st.fatal);
    assert_eq!(st.step, 2, "capture steps did not complete");

    let (fw, fh, sf, backend) = st.first_frame.expect("first frame recorded");
    assert!(fw > 0 && fh > 0, "captured frame must be non-empty");
    assert!(sf > 0.0, "scale factor must be positive, got {sf}");
    assert!(
        matches!(backend, "win32-printwindow" | "win32-bitblt-screen"),
        "unexpected capture backend tag: {backend}"
    );
    if let Some((rw, rh)) = st.resized {
        assert_eq!((fw, fh), (rw, rh), "capture size must match Resized");
    }

    // First compare: no baseline yet → blocked/baseline_missing.
    assert_eq!(
        st.first_verdict.as_deref(),
        Some("blocked/baseline_missing"),
        "first compare must report missing baseline, got {:?}",
        st.first_verdict
    );
    // Second compare against the promoted baseline → pass/match.
    assert_eq!(
        st.second_verdict.as_deref(),
        Some("pass/match"),
        "second compare must pass against promoted baseline, got {:?} stats {:?}",
        st.second_verdict,
        st.second_stats
    );
    assert_eq!(
        st.second_frame.map(|(w, h, _, _)| (w, h)),
        Some((fw, fh)),
        "second capture must have identical dimensions"
    );
    assert_eq!(
        st.second_frame.map(|(_, _, _, b)| b),
        Some(backend),
        "both captures must use the same OS backend path"
    );

    // Evidence: two JSONL lines (blocked + pass).
    let evidence = std::fs::read_to_string(evidence_path).expect("evidence file");
    let lines: Vec<&str> = evidence.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 evidence lines, got {lines:?}");
    assert!(lines[0].contains("\"verdict\":\"blocked\""));
    assert!(lines[1].contains("\"verdict\":\"pass\""));
}
