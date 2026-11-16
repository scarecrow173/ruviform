//! Issue #631 slice — `RealOsGuiTest` window-close / DPI
//! observation surface on real OS windows (`real-scenario`
//! feature).
//!
//! Two lanes, both driving the *real* GPUI platform loop through
//! `RealOsGuiTest::run` — never the `gpui-test` simulated backend:
//!
//! - `real_window_close_lifecycle` — the veto-able close protocol
//!   end to end: `arm_close_response` queues the decision the
//!   app-thread `CloseRequested` handler applies inside the
//!   synchronous event flush (the only point a `Veto`/`Defer` can
//!   still beat the unanswered-`Approve` default),
//!   `request_close`/`respond_close` forward the driver-neutral
//!   `WindowDriver` ops, and `window_events` observes the recorded
//!   `Created`/`Ready`/`CloseRequested`/`Destroyed` transitions —
//!   including terminal-id retirement and sibling-window isolation.
//! - `real_window_dpi_geometry_observation` — `window_geometry`
//!   reads the live OS-measured client rect (physical pixels +
//!   scale factor) and the lifecycle log's `Resized`/`Moved`/
//!   `ScaleFactorChanged` records are checked against it. DPI
//!   *changes* (mixed-monitor moves) stay out of scope — no window
//!   move/resize verb exists on this surface yet.
//!
//! Honesty contract — identical to `tests/real_window.rs`: a
//! session that cannot start on a headless host records
//! `environment_blocked` (driver-lab evidence row + panicking
//! `environment_blocked:` tag), never a pass; every asserted value
//! comes from the real window, never a fabricated state.
//!
//! Interactive display required (Windows interactive desktop; Linux
//! X display — the CI offscreen lane runs `tests/real_window.rs`
//! under Xvfb, this target is exercised through
//! `mise run test ruviform-ui-gpui --features real-scenario` or
//! `cargo test -p ruviform-ui-gpui --features real-scenario --test
//! real_window_lifecycle`).
//!
//! Durable evidence: `RUVIFORM_RF631_SURFACE_EVIDENCE` names a
//! JSONL file (relative paths anchor at the workspace root) that
//! receives `rf631-real-window-surface/v1` run/case rows — same
//! env-gate convention as the driver-lab/golden evidence channels.

#![cfg(all(any(windows, target_os = "linux"), feature = "real-scenario"))]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState, PanelSpec,
    ViewHost, ViewSpec,
};
use ruviform_ui::{CloseDecision, LogicalPosition, UiError, WindowId, WindowSpec};
use ruviform_ui_gpui::{
    OsWindowGeometry, RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount,
    WindowLifecycleEvent,
};
use ruviform_ui_test::{ScenarioError, SemanticGuiTest, TargetQuery, settle_until};
use serde_json::{Value, json};

/// `drain` boundaries waited before `body` — same measured budgets
/// as `tests/real_window.rs` (Linux's llvmpipe first frame needs far
/// more ticks than Windows' GPU path).
#[cfg(windows)]
const STARTUP_SETTLE_TICKS: u32 = 30;
#[cfg(target_os = "linux")]
const STARTUP_SETTLE_TICKS: u32 = 600;

/// Env var gating the surface evidence JSONL — relative paths anchor
/// at the workspace root (`CARGO_MANIFEST_DIR/../..`), the same rule
/// `driver_lab::evidence_target` applies.
const SURFACE_EVIDENCE_ENV: &str = "RUVIFORM_RF631_SURFACE_EVIDENCE";

/// Serializes evidence appends — the two tests may run on parallel
/// test threads and must not interleave a JSONL row.
static EVIDENCE_LOCK: Mutex<()> = Mutex::new(());

fn panel_spec(title: &str) -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("root".to_string()),
            title: Some(title.to_string()),
            children: vec![],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// One [`RealWindowMount`] — minimal panel view, no theme (the
/// lifecycle/DPI surface does not read pixels or tokens).
fn mount(
    view: ViewInstanceId,
    epochs: &mut AttachEpochAllocator,
    title: &str,
    position: Option<LogicalPosition>,
) -> RealWindowMount {
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(panel_spec(title)).expect("mount panel spec");
    host.drain_events();
    RealWindowMount {
        window: WindowSpec {
            title: title.to_string(),
            position,
            ..WindowSpec::default()
        },
        view: ViewMount { host, theme: None },
    }
}

fn session_options() -> RealWindowOptions {
    RealWindowOptions {
        startup_settle_ticks: STARTUP_SETTLE_TICKS,
        ..RealWindowOptions::default()
    }
}

/// `count` of `event` in `window`'s recorded lifecycle log.
fn event_count(gui: &RealOsGuiTest, window: WindowId, event: WindowLifecycleEvent) -> usize {
    gui.window_events(window)
        .iter()
        .filter(|observed| **observed == event)
        .count()
}

/// Stable `&'static str` names for evidence rows — the enum is
/// `#[non_exhaustive]`, so future variants report `"unknown"`
/// rather than failing the evidence write.
fn event_name(event: WindowLifecycleEvent) -> &'static str {
    match event {
        WindowLifecycleEvent::Created => "created",
        WindowLifecycleEvent::Ready => "ready",
        WindowLifecycleEvent::Resized { .. } => "resized",
        WindowLifecycleEvent::Moved { .. } => "moved",
        WindowLifecycleEvent::Focused(_) => "focused",
        WindowLifecycleEvent::ScaleFactorChanged { .. } => "scale-factor-changed",
        WindowLifecycleEvent::CloseRequested => "close-requested",
        WindowLifecycleEvent::Destroyed => "destroyed",
        _ => "unknown",
    }
}

fn event_names(events: &[WindowLifecycleEvent]) -> Vec<&'static str> {
    events.iter().map(|event| event_name(*event)).collect()
}

/// Best-effort repo SHA for evidence rows — `"unknown"` when git is
/// unavailable; never blocks the lane.
fn repo_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn surface_evidence_path() -> Option<PathBuf> {
    let raw = std::env::var(SURFACE_EVIDENCE_ENV).ok()?;
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

fn append_evidence(row: &Value) {
    let Some(path) = surface_evidence_path() else {
        return;
    };
    let _guard = EVIDENCE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("surface evidence dir");
    }
    let line = format!(
        "{}\n",
        serde_json::to_string(row).expect("surface evidence row")
    );
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
        .unwrap_or_else(|e| panic!("write surface evidence {}: {e}", path.display()));
}

/// Emits the `run` row and returns the `run_id` the case rows share.
fn emit_run_row(test: &str) -> String {
    let run_id = format!("rf631-window-surface-{}-{}", std::process::id(), unix_now());
    append_evidence(&json!({
        "schema": "rf631-real-window-surface/v1",
        "type": "run",
        "run_id": run_id,
        "test": test,
        "lane": "cargo-test:real_window_lifecycle",
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "repo_sha": repo_sha(),
        "revision": env!("CARGO_PKG_VERSION"),
        "timestamp_unix": unix_now(),
    }));
    run_id
}

fn emit_case_row(
    run_id: &str,
    case: &str,
    verdict: &str,
    detail: &str,
    observed: &Value,
    environment: &Value,
) {
    append_evidence(&json!({
        "schema": "rf631-real-window-surface/v1",
        "type": "case",
        "run_id": run_id,
        "case": case,
        "verdict": verdict,
        "detail": detail,
        "observed": observed,
        "environment": environment,
    }));
}

/// Environment record shared by both cases — measured values only.
fn surface_environment(scale_factor: Option<f64>) -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "scale_factor": scale_factor,
        "repo_sha": repo_sha(),
        "revision": env!("CARGO_PKG_VERSION"),
        "timestamp_unix": unix_now(),
    })
}

/// Classifies a `RealOsGuiTest::run` session-level failure —
/// `Host`/`BackendUnavailable` mean no interactive desktop could be
/// established: recorded `environment_blocked`, panicked with the
/// same tag so it is never mistaken for a GUI/test failure.
fn session_failure(test: &str, run_id: &str, error: &UiError) -> ! {
    let message = error.to_string();
    if matches!(
        error,
        UiError::Host { .. } | UiError::BackendUnavailable { .. }
    ) {
        emit_case_row(
            run_id,
            test,
            "environment_blocked",
            &format!("real-window session could not start: {message}"),
            &json!({}),
            &surface_environment(None),
        );
        panic!("environment_blocked: {test} real-window session could not start: {message}");
    }
    panic!("{test} real-window session failed: {error}");
}

/// What the close-lifecycle body collected for the evidence row —
/// `[a, b, c]` ids and their recorded event logs, in mount order.
struct CloseObservation {
    ids: [WindowId; 3],
    events: [Vec<WindowLifecycleEvent>; 3],
}

fn close_observed_json(obs: &CloseObservation) -> Value {
    let [a, b, c] = obs.ids;
    json!({
        "window_a": a,
        "window_b": b,
        "window_c": c,
        "window_a_events": event_names(&obs.events[0]),
        "window_b_events": event_names(&obs.events[1]),
        "window_c_events": event_names(&obs.events[2]),
        "sequence": [
            "veto: arm_close_response(Veto) -> request_close -> CloseRequested observed, window stays open",
            "defer: arm_close_response(Defer) -> request_close -> CloseRequested observed, window stays open close-pending",
            "deferred-approve: respond_close(Approve) -> Destroyed, id retired terminally",
            "default-approve: unanswered request_close on sibling -> CloseRequested -> Destroyed",
            "sentinel: window C stays open — the GPUI loop exits when its last window closes, so a live sentinel keeps the UiJob pump answering through every observation",
        ],
    })
}

/// Waits until every window reaches `Ready`, then asserts the
/// created/ready/physical-resized baseline events exist.
fn assert_all_ready(gui: &mut RealOsGuiTest, windows: [WindowId; 3]) -> Result<(), ScenarioError> {
    let ready = settle_until(
        gui,
        |g| {
            windows.iter().all(|window| {
                g.window_events(*window)
                    .contains(&WindowLifecycleEvent::Ready)
            })
        },
        60,
    )?;
    assert!(ready, "all windows must reach Ready");
    for window in windows {
        let events = gui.window_events(window);
        assert!(events.contains(&WindowLifecycleEvent::Created));
        assert!(events.contains(&WindowLifecycleEvent::Ready));
        assert!(
            events.iter().any(|event| matches!(
                event,
                WindowLifecycleEvent::Resized { width, height }
                    if *width > 0 && *height > 0
            )),
            "physical Resized event observed for {window}"
        );
    }
    Ok(())
}

/// Veto path: the armed decision is consumed inside the synchronous
/// `CloseRequested` flush; the window survives and siblings observe
/// no close lifecycle. Also pins the honest error for answering a
/// close that was never requested.
fn exercise_veto(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    siblings: [WindowId; 2],
) -> Result<(), ScenarioError> {
    // A close response with nothing pending is an honest error,
    // not a silent no-op.
    assert!(
        gui.respond_close(window, CloseDecision::Approve).is_err(),
        "respond_close without a pending close must fail"
    );

    gui.arm_close_response(window, CloseDecision::Veto);
    gui.request_close(window)?;
    let seen = settle_until(
        gui,
        |g| event_count(g, window, WindowLifecycleEvent::CloseRequested) >= 1,
        40,
    )?;
    assert!(seen, "CloseRequested must be observed");
    gui.settle()?;
    assert!(
        gui.windows().contains(&window),
        "vetoed window must stay open"
    );
    assert_eq!(
        event_count(gui, window, WindowLifecycleEvent::Destroyed),
        0,
        "vetoed window must not emit Destroyed"
    );
    for sibling in siblings {
        assert!(
            !gui.window_events(sibling)
                .contains(&WindowLifecycleEvent::CloseRequested),
            "sibling window {sibling} must be untouched by the close"
        );
    }
    Ok(())
}

/// Defer path: the close stays pending after `CloseRequested`, then
/// the deferred `Approve` arrives through the public
/// `respond_close` path and destroys the window while the sibling
/// survives untouched.
fn exercise_defer_then_approve(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    sibling: WindowId,
) -> Result<(), ScenarioError> {
    gui.arm_close_response(window, CloseDecision::Defer);
    gui.request_close(window)?;
    let seen = settle_until(
        gui,
        |g| event_count(g, window, WindowLifecycleEvent::CloseRequested) >= 2,
        40,
    )?;
    assert!(seen, "second CloseRequested must be observed");
    gui.settle()?;
    assert!(
        gui.windows().contains(&window),
        "deferred close must keep the window open (close-pending)"
    );
    gui.respond_close(window, CloseDecision::Approve)?;
    let destroyed = settle_until(
        gui,
        |g| {
            g.window_events(window)
                .contains(&WindowLifecycleEvent::Destroyed)
        },
        40,
    )?;
    assert!(destroyed, "approved deferred close must destroy the window");
    assert!(!gui.windows().contains(&window));
    assert!(
        gui.windows().contains(&sibling),
        "sibling window must survive the destruction"
    );
    assert!(
        !gui.window_events(sibling)
            .contains(&WindowLifecycleEvent::Destroyed),
        "sibling window must not emit Destroyed"
    );
    Ok(())
}

/// Terminal semantics: the destroyed id is retired — never reused,
/// no `CloseRequested` may follow `Destroyed`, further close ops are
/// honest errors, and target resolution reports `NoSuchWindow`.
fn assert_terminal_retirement(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
) -> Result<(), ScenarioError> {
    gui.request_close(window)?;
    gui.settle()?;
    assert!(
        !gui.window_events(window)
            .iter()
            .skip_while(|event| **event != WindowLifecycleEvent::Destroyed)
            .any(|event| matches!(event, WindowLifecycleEvent::CloseRequested)),
        "no CloseRequested may follow Destroyed"
    );
    assert!(
        gui.respond_close(window, CloseDecision::Approve).is_err(),
        "destroyed window has no pending close"
    );
    assert!(matches!(
        gui.resolve_target(&TargetQuery::new(window, view)),
        Err(ScenarioError::NoSuchWindow { .. })
    ));
    Ok(())
}

/// Unanswered default: a `request_close` with no armed decision
/// takes the driver's `Approve` default; only the sentinel window
/// remains and it observed no close lifecycle.
fn exercise_default_approve(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    sentinel: WindowId,
) -> Result<(), ScenarioError> {
    gui.request_close(window)?;
    let destroyed = settle_until(
        gui,
        |g| {
            g.window_events(window)
                .contains(&WindowLifecycleEvent::Destroyed)
        },
        40,
    )?;
    assert!(destroyed, "unanswered close must default to Approve");
    assert_eq!(
        gui.windows(),
        vec![sentinel],
        "only the sentinel window remains open"
    );
    let sentinel_events = gui.window_events(sentinel);
    assert!(
        !sentinel_events.contains(&WindowLifecycleEvent::CloseRequested)
            && !sentinel_events.contains(&WindowLifecycleEvent::Destroyed),
        "sentinel window must observe no close lifecycle"
    );
    Ok(())
}

/// The close-lifecycle session body — orchestrates the phase helpers
/// and collects the observation record for the evidence row.
fn close_lifecycle_body(
    gui: &mut RealOsGuiTest,
    view_a: ViewInstanceId,
) -> Result<CloseObservation, ScenarioError> {
    let windows = gui.windows();
    assert_eq!(windows.len(), 3, "three mounted windows");
    let (window_a, window_b, window_c) = (windows[0], windows[1], windows[2]);

    // All windows must reach Ready before any close request —
    // `window_events` is the observation surface under test.
    assert_all_ready(gui, [window_a, window_b, window_c])?;
    exercise_veto(gui, window_a, [window_b, window_c])?;
    exercise_defer_then_approve(gui, window_a, window_b)?;
    assert_terminal_retirement(gui, window_a, view_a)?;
    exercise_default_approve(gui, window_b, window_c)?;

    Ok(CloseObservation {
        ids: [window_a, window_b, window_c],
        events: [
            gui.window_events(window_a),
            gui.window_events(window_b),
            gui.window_events(window_c),
        ],
    })
}

/// Veto → Defer → deferred Approve → terminal retirement →
/// unanswered-default Approve, across real windows with sibling
/// isolation checked at every step and a sentinel window keeping
/// the session's job pump alive through the last destruction.
#[test]
fn real_window_close_lifecycle() {
    let _ = env_logger::builder().is_test(false).try_init();
    let run_id = emit_run_row("real_window_close_lifecycle");
    let view_a = ViewInstanceId::new(6311).expect("nonzero");
    let view_b = ViewInstanceId::new(6312).expect("nonzero");
    let view_c = ViewInstanceId::new(6313).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    // Three windows: A/B exercise every close decision, C is the
    // sentinel — the GPUI platform loop exits when its last window
    // closes, which would kill the UiJob pump before a post-destroy
    // observation could be answered. C stays open until session
    // teardown so every `settle`/`call` keeps a live pump.
    let mounts = vec![
        mount(
            view_a,
            &mut epochs,
            "rf631 surface window A",
            Some(LogicalPosition::new(40, 40)),
        ),
        mount(
            view_b,
            &mut epochs,
            "rf631 surface window B",
            Some(LogicalPosition::new(80, 80)),
        ),
        mount(
            view_c,
            &mut epochs,
            "rf631 surface window C",
            Some(LogicalPosition::new(120, 120)),
        ),
    ];

    let outcome = RealOsGuiTest::run(mounts, session_options(), |gui| {
        close_lifecycle_body(gui, view_a)
    });

    match outcome {
        Ok(Ok(obs)) => {
            emit_case_row(
                &run_id,
                "window-close-lifecycle",
                "pass",
                "veto/defer/deferred-approve/default-approve observed on real windows; sentinel kept the job pump alive; terminal id retirement and sibling isolation verified",
                &close_observed_json(&obs),
                &surface_environment(None),
            );
        }
        Ok(Err(error)) => {
            emit_case_row(
                &run_id,
                "window-close-lifecycle",
                "fail",
                &format!("scenario body failed: {error}"),
                &json!({}),
                &surface_environment(None),
            );
            panic!("window-close-lifecycle scenario failed: {error}");
        }
        Err(error) => session_failure("window-close-lifecycle", &run_id, &error),
    }
}

/// What the DPI/geometry body collected for the evidence row.
struct GeometryObservation {
    window: WindowId,
    events: Vec<WindowLifecycleEvent>,
    geometry: OsWindowGeometry,
    resized: Vec<(u32, u32)>,
}

/// Two reads of a settled window must agree — the OS client rect
/// does not drift between queries. Asserts positive physical size
/// and a finite positive scale factor.
fn assert_geometry_stable(
    gui: &mut RealOsGuiTest,
    window: WindowId,
) -> Result<OsWindowGeometry, ScenarioError> {
    let first = gui.window_geometry(window)?;
    let second = gui.window_geometry(window)?;
    assert_eq!(first, second, "geometry read must be stable");
    assert!(first.width > 0 && first.height > 0, "client rect measured");
    assert!(
        first.scale_factor.is_finite() && first.scale_factor > 0.0,
        "scale factor measured: {}",
        first.scale_factor
    );
    Ok(first)
}

/// Lifecycle geometry record: physical-pixel `Resized` transitions
/// consistent with the live OS client rect (same physical area
/// within two independent reads' rounding slack).
fn assert_lifecycle_geometry(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    geometry: &OsWindowGeometry,
) -> Vec<(u32, u32)> {
    let events = gui.window_events(window);
    assert!(events.contains(&WindowLifecycleEvent::Created));
    assert!(events.contains(&WindowLifecycleEvent::Ready));
    let resized: Vec<(u32, u32)> = events
        .iter()
        .filter_map(|event| match event {
            WindowLifecycleEvent::Resized { width, height } => Some((*width, *height)),
            _ => None,
        })
        .collect();
    assert!(
        !resized.is_empty(),
        "at least one physical Resized transition must be observed"
    );
    assert!(
        resized.iter().all(|(w, h)| *w > 0 && *h > 0),
        "Resized carries physical pixels"
    );
    let last = *resized.last().expect("nonempty");
    let dw = i64::from(last.0) - i64::from(geometry.width);
    let dh = i64::from(last.1) - i64::from(geometry.height);
    assert!(
        dw.abs() <= 2 && dh.abs() <= 2,
        "Resized {last:?} vs client geometry {}x{} disagree beyond rounding",
        geometry.width,
        geometry.height
    );
    resized
}

/// Destroys `window` — the sentinel keeps the session's job pump
/// alive — then probes the honest dead-window errors: geometry is
/// never synthesized for a destroyed target.
fn probe_dead_window_geometry(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    sentinel: WindowId,
) -> Result<(), ScenarioError> {
    gui.request_close(window)?;
    let destroyed = settle_until(
        gui,
        |g| {
            g.window_events(window)
                .contains(&WindowLifecycleEvent::Destroyed)
        },
        40,
    )?;
    assert!(destroyed);
    assert_eq!(gui.windows(), vec![sentinel]);
    assert!(gui.window_geometry(window).is_err());
    assert!(
        !gui.window_events(sentinel)
            .contains(&WindowLifecycleEvent::CloseRequested),
        "sentinel window must observe no close lifecycle"
    );
    Ok(())
}

/// The DPI/geometry session body.
fn dpi_geometry_body(gui: &mut RealOsGuiTest) -> Result<GeometryObservation, ScenarioError> {
    let mut ids = gui.windows();
    assert_eq!(ids.len(), 2, "observed + sentinel windows");
    let sentinel = ids.pop().expect("two windows");
    let window = ids.pop().expect("two windows");
    let ready = settle_until(
        gui,
        |g| {
            g.window_events(window)
                .contains(&WindowLifecycleEvent::Ready)
        },
        60,
    )?;
    assert!(ready, "window must reach Ready");

    let first = assert_geometry_stable(gui, window)?;
    let resized = assert_lifecycle_geometry(gui, window, &first);
    probe_dead_window_geometry(gui, window, sentinel)?;

    Ok(GeometryObservation {
        window,
        events: gui.window_events(window),
        geometry: first,
        resized,
    })
}

/// Live OS-measured client geometry + the physical-pixel lifecycle
/// record — the DPI observation surface for Issue #631's
/// `real-window-dpi` prerequisite.
#[test]
fn real_window_dpi_geometry_observation() {
    let _ = env_logger::builder().is_test(false).try_init();
    let run_id = emit_run_row("real_window_dpi_geometry_observation");
    let view = ViewInstanceId::new(6321).expect("nonzero");
    let view_sentinel = ViewInstanceId::new(6322).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    // The observed window plus a sentinel: the GPUI platform loop
    // exits when its last window closes, so destroying the observed
    // window while the sentinel stays open keeps the UiJob pump
    // answering the post-destroy `window_geometry` probe.
    let mounts = vec![
        mount(
            view,
            &mut epochs,
            "rf631 surface dpi window",
            Some(LogicalPosition::new(120, 120)),
        ),
        mount(
            view_sentinel,
            &mut epochs,
            "rf631 surface dpi sentinel",
            Some(LogicalPosition::new(160, 160)),
        ),
    ];

    let outcome = RealOsGuiTest::run(mounts, session_options(), dpi_geometry_body);

    match outcome {
        Ok(Ok(obs)) => {
            emit_case_row(
                &run_id,
                "window-dpi-geometry",
                "pass",
                "live OS client geometry (physical px + scale factor) agrees with lifecycle Resized record; dead-window geometry errors honestly",
                &json!({
                    "window": obs.window,
                    "events": event_names(&obs.events),
                    "geometry": {
                        "origin_x": obs.geometry.origin_x,
                        "origin_y": obs.geometry.origin_y,
                        "width": obs.geometry.width,
                        "height": obs.geometry.height,
                        "scale_factor": obs.geometry.scale_factor,
                    },
                    "resized_events": obs.resized,
                }),
                &surface_environment(Some(obs.geometry.scale_factor)),
            );
        }
        Ok(Err(error)) => {
            emit_case_row(
                &run_id,
                "window-dpi-geometry",
                "fail",
                &format!("scenario body failed: {error}"),
                &json!({}),
                &surface_environment(None),
            );
            panic!("window-dpi-geometry scenario failed: {error}");
        }
        Err(error) => session_failure("window-dpi-geometry", &run_id, &error),
    }
}
