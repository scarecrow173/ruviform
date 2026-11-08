//! Real-OS-window lifecycle probe (Issue #243, Windows partial scope).
//!
//! Opens two GPUI windows (`main`, `veto`) on the local session and
//! exercises the driver-neutral contract items the headless fixture
//! covers but Issue #431 left unmeasured on a real OS:
//!
//! - per-window `Created`/`Ready`/`Focused`/`Resized`/`CloseRequested`/
//!   `Destroyed` events, including `focus(w, true)` transitions and
//!   the `focus(w, false)` -> `Unsupported` / unknown-window -> `Host`
//!   error contract;
//! - `resize` -> real `Resized`, `monitors()` real values;
//! - OS close path (`WM_CLOSE`, driven externally — see
//!   `evidence/issue-431-real-verify/wm-close.ps1`, NOT physical input):
//!   `veto` gets Veto -> Defer -> Approve across three closes and must
//!   stay open after the first two; `main` gets an unanswered
//!   `CloseRequested` which must default to Approve;
//! - window id non-reuse: after `veto` is destroyed a `reborn` window
//!   is created and must get a fresh `WindowId`;
//! - `request_close(reborn)` -> `Veto`, then `destroy_window(reborn)`
//!   which the contract says bypasses `CloseRequested` entirely.
//!
//! Drive the closes from a second shell once the probe ops finish
//! (the ops take ~5s after both windows are Ready; start closes after
//! ~15s to stay clear of them):
//!
//! ```text
//! powershell -NoProfile -ExecutionPolicy Bypass -File \
//!   evidence/issue-431-real-verify/wm-close.ps1 \
//!   -Exe target/debug/examples/window_lifecycle_probe.exe \
//!   -Steps "veto,veto,veto,main" -StartWaitMs 15000
//! ```
//!
//! `--degenerate-create` runs a separate observation mode: a 0x0
//! `WindowSpec` window creation, classified by which lifecycle events
//! (if any) arrive — the window-creation-failure degrade path.
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example window_lifecycle_probe`
//!
//! Manual verification only (real OS session). Exits 0 only when every
//! recorded check passed; observations that are contract-legal but
//! environment-dependent are logged, not asserted.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    CloseDecision, DriverEvent, LogicalSize, UiError, WindowDriver, WindowId, WindowLifecycle,
    WindowSpec,
};
use ruviform_ui_gpui::GpuiDriver;

const TICK: Duration = Duration::from_millis(100);
const TIMEOUT: Duration = Duration::from_secs(180);
/// Ticks each probe op waits before the next — ~0.8s per step so
/// focus/geometry events settle between operations.
const STEP_TICKS: u64 = 8;
/// Probe phases 0..=5 issue the ops below; phase 6 evaluates the
/// checkpoint once, then waits for the externally driven closes.
const PHASE_CHECKPOINT: u8 = 6;
/// Ticks between `request_close(reborn)`'s Veto and `destroy_window`.
const DESTROY_DELAY_TICKS: u64 = 30;
/// Degenerate-mode observation window before classifying.
const DEGENERATE_TIMEOUT: Duration = Duration::from_secs(15);

/// Lifecycle kinds the verdict trace records (geometry/focus/scale are
/// timing-dependent and stay out of it — same convention as the shared
/// `ruviform-ui-test` fixture).
fn kind(event: &WindowLifecycle) -> Option<&'static str> {
    Some(match event {
        WindowLifecycle::Created => "Created",
        WindowLifecycle::Ready => "Ready",
        WindowLifecycle::CloseRequested => "CloseRequested",
        WindowLifecycle::Destroyed => "Destroyed",
        _ => return None,
    })
}

struct State {
    main: WindowId,
    veto: WindowId,
    reborn: Option<WindowId>,
    names: BTreeMap<WindowId, &'static str>,
    ready: BTreeSet<WindowId>,
    /// Lifecycle-kind trace per window, in delivery order.
    trace: BTreeMap<WindowId, Vec<&'static str>>,
    /// Last observed `Focused` payload per window.
    last_focus: BTreeMap<WindowId, bool>,
    /// `Focused(false)` deliveries seen across all windows.
    focus_lost_count: u32,
    veto_closes: u32,
    reborn_closes: u32,
    resize_issued: bool,
    resized_main_after_call: bool,
    /// Set once the post-op checkpoint has been evaluated.
    checkpoint_done: bool,
    /// `destroy_window(reborn)` scheduled at this tick count.
    destroy_at: Option<u64>,
    phase: u8,
    step_ticks: u64,
    ticks: u64,
    started: Instant,
    failures: Vec<String>,
    verdict: Option<Result<(), String>>,
}

fn main() -> ExitCode {
    if std::env::args().any(|a| a == "--degenerate-create") {
        return degenerate();
    }
    let mut driver = GpuiDriver::new();
    eprintln!("[probe] capabilities: {:?}", driver.capabilities());
    let main = create(&mut driver, "rf243 probe main");
    let veto = create(&mut driver, "rf243 probe veto");

    let state = Rc::new(RefCell::new(State {
        main,
        veto,
        reborn: None,
        names: BTreeMap::from([(main, "main"), (veto, "veto")]),
        ready: BTreeSet::new(),
        trace: BTreeMap::new(),
        last_focus: BTreeMap::new(),
        focus_lost_count: 0,
        veto_closes: 0,
        reborn_closes: 0,
        resize_issued: false,
        resized_main_after_call: false,
        checkpoint_done: false,
        destroy_at: None,
        phase: 0,
        step_ticks: 0,
        ticks: 0,
        started: Instant::now(),
        failures: Vec::new(),
        verdict: None,
    }));
    let handler = state.clone();
    driver
        .run(Some(TICK), move |ev, drv| {
            on_event(&mut handler.borrow_mut(), ev, drv);
        })
        .expect("run");

    match state.borrow_mut().verdict.take() {
        Some(Ok(())) => {
            eprintln!("[probe] PASS");
            ExitCode::SUCCESS
        }
        Some(Err(reason)) => {
            eprintln!("[probe] FAIL: {reason}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("[probe] FAIL: loop ended without a verdict");
            ExitCode::FAILURE
        }
    }
}

fn create(driver: &mut GpuiDriver, title: &str) -> WindowId {
    let spec = WindowSpec {
        title: title.to_string(),
        ..WindowSpec::default()
    };
    let id = driver.create_window(&spec).expect("create window");
    eprintln!("[probe] created {id} '{title}'");
    id
}

fn fail(st: &mut State, reason: impl Into<String>) {
    let reason = reason.into();
    eprintln!("[probe] check failed: {reason}");
    st.failures.push(reason);
}

fn on_event(st: &mut State, ev: DriverEvent, driver: &mut dyn WindowDriver) {
    if st.verdict.is_some() {
        return;
    }
    if matches!(ev, DriverEvent::Tick) {
        st.ticks += 1;
        if st.started.elapsed() > TIMEOUT {
            return finish(st, driver, Err(format!("timeout after {TIMEOUT:?}")));
        }
        on_tick(st, driver);
        return;
    }
    let DriverEvent::Windowed { window, event } = ev else {
        return;
    };
    let name = st.names.get(&window).copied().unwrap_or("?");
    match &event {
        WindowLifecycle::Focused(active) => {
            eprintln!("[probe] {name}: Focused({active})");
            st.last_focus.insert(window, *active);
            if !*active {
                st.focus_lost_count += 1;
            }
        }
        WindowLifecycle::Resized { width, height } => {
            eprintln!("[probe] {name}: Resized {width}x{height}");
            if window == st.main && st.resize_issued {
                st.resized_main_after_call = true;
            }
        }
        WindowLifecycle::Moved { x, y } => eprintln!("[probe] {name}: Moved {x},{y}"),
        WindowLifecycle::ScaleFactorChanged { factor } => {
            eprintln!("[probe] {name}: ScaleFactorChanged {factor}");
        }
        WindowLifecycle::Action(intent) => {
            eprintln!("[probe] {name}: Action({intent:?})");
        }
        _ => {}
    }
    if let Some(k) = kind(&event) {
        eprintln!("[probe] {name}: {k}");
        st.trace.entry(window).or_default().push(k);
    }
    match event {
        WindowLifecycle::Ready => {
            st.ready.insert(window);
        }
        WindowLifecycle::CloseRequested => on_close_requested(st, driver, window),
        WindowLifecycle::Destroyed => on_destroyed(st, driver, window),
        _ => {}
    }
}

/// One tick of the probe step machine — issues the next op once the
/// previous step's events have had `STEP_TICKS` to arrive.
fn on_tick(st: &mut State, driver: &mut dyn WindowDriver) {
    if st.phase < PHASE_CHECKPOINT {
        st.step_ticks += 1;
        if st.step_ticks < STEP_TICKS {
            return;
        }
        st.step_ticks = 0;
        probe_step(st, driver);
        return;
    }
    if !st.checkpoint_done {
        // Both windows have been Ready for ~5s — focus/geometry events
        // from the probe ops have settled; evaluate them once.
        if st.ready.len() == 2 {
            st.checkpoint_done = true;
            evaluate_checkpoint(st);
        }
        return;
    }
    // `destroy_window(reborn)` fires only after the Veto'd
    // `request_close` has been answered and settled.
    if let (Some(at), Some(reborn)) = (st.destroy_at, st.reborn)
        && st.ticks >= at
    {
        st.destroy_at = None;
        eprintln!("[probe] op: destroy_window({reborn})");
        if let Err(e) = driver.destroy_window(reborn) {
            fail(st, format!("destroy_window: {e:?}"));
        }
    }
}

/// Issues the op for the current probe phase, then advances.
fn probe_step(st: &mut State, driver: &mut dyn WindowDriver) {
    let phase = st.phase;
    st.phase += 1;
    match phase {
        // Ops before both windows are Ready are legal (they queue),
        // but gating keeps the observations clean — wait instead.
        0 if st.ready.len() < 2 => {
            st.phase = 0;
        }
        0 => {
            eprintln!("[probe] op: focus(veto, true)");
            report(st, driver.focus(st.veto, true), "focus veto");
        }
        1 => {
            eprintln!("[probe] op: focus(main, true)");
            report(st, driver.focus(st.main, true), "focus main");
        }
        2 => {
            eprintln!("[probe] op: focus(veto, false) — expect Unsupported");
            match driver.focus(st.veto, false) {
                Err(UiError::Unsupported { .. }) => {
                    eprintln!("[probe] focus release -> Unsupported (contract)");
                }
                other => fail(
                    st,
                    format!("focus release expected Unsupported, got {other:?}"),
                ),
            }
        }
        3 => {
            let unknown = WindowId::new(u64::MAX).expect("nonzero");
            eprintln!("[probe] op: focus({unknown}, true) — expect Host");
            match driver.focus(unknown, true) {
                Err(UiError::Host { .. }) => {
                    eprintln!("[probe] focus unknown -> Host (contract)");
                }
                other => fail(st, format!("focus unknown expected Host, got {other:?}")),
            }
        }
        4 => {
            eprintln!("[probe] op: resize(main, 960x600)");
            st.resize_issued = true;
            report(
                st,
                driver.resize(st.main, LogicalSize::new(960, 600)),
                "resize main",
            );
        }
        5 => {
            eprintln!("[probe] op: monitors()");
            let monitors = driver.monitors();
            if monitors.is_empty() {
                eprintln!("[probe] monitor: <none reported>");
            }
            for m in monitors {
                eprintln!(
                    "[probe] monitor: name={:?} {}x{} @{},{} scale={}",
                    m.name, m.width, m.height, m.x, m.y, m.scale_factor
                );
            }
        }
        _ => {}
    }
}

fn report(st: &mut State, result: Result<(), UiError>, what: &str) {
    match result {
        Ok(()) => eprintln!("[probe] {what}: ok"),
        Err(e) => fail(st, format!("{what}: {e:?}")),
    }
}

/// Post-op assertions: per-window focus independence, the driver-API
/// resize reaching a real `Resized`, and that a focus-lost transition
/// was observed (two windows cannot both stay focused).
fn evaluate_checkpoint(st: &mut State) {
    eprintln!("[probe] checkpoint: last_focus={:?}", st.last_focus);
    if st.last_focus.get(&st.main) != Some(&true) {
        fail(
            st,
            format!(
                "expected main Focused(true) last, got {:?}",
                st.last_focus.get(&st.main)
            ),
        );
    }
    if st.last_focus.get(&st.veto) != Some(&false) {
        fail(
            st,
            format!(
                "expected veto Focused(false) last, got {:?}",
                st.last_focus.get(&st.veto)
            ),
        );
    }
    if st.focus_lost_count == 0 {
        fail(st, "no Focused(false) transition observed across windows");
    }
    if !st.resized_main_after_call {
        fail(st, "no Resized event for main after resize() call");
    }
    eprintln!("[probe] checkpoint done: failures={}", st.failures.len());
}

/// Routes a `CloseRequested` to the per-window close choreography.
fn on_close_requested(st: &mut State, driver: &mut dyn WindowDriver, window: WindowId) {
    if window == st.veto {
        st.veto_closes += 1;
        // OS path (WM_CLOSE): Veto keeps it open, Defer keeps it
        // close-pending, Approve destroys.
        let decision = match st.veto_closes {
            1 => CloseDecision::Veto,
            2 => CloseDecision::Defer,
            _ => CloseDecision::Approve,
        };
        eprintln!(
            "[probe] veto: CloseRequested #{} -> respond {decision:?}",
            st.veto_closes
        );
        if let Err(e) = driver.respond_close(window, decision) {
            fail(st, format!("respond_close veto: {e:?}"));
        }
    } else if window == st.main {
        // Deliberately unanswered: the contract default is Approve
        // for this window only. This is the OS path's default-close
        // measurement — do not respond.
        eprintln!("[probe] main: CloseRequested — no response (default approve)");
    } else if st.reborn == Some(window) {
        st.reborn_closes += 1;
        // First close is the programmatic `request_close` — Veto it.
        // A second CloseRequested can only come from `destroy_window`
        // routing through the close flow (a contract deviation); answer
        // Approve so the run still terminates and the trace records it.
        let decision = if st.reborn_closes == 1 {
            CloseDecision::Veto
        } else {
            CloseDecision::Approve
        };
        eprintln!(
            "[probe] reborn: CloseRequested #{} -> respond {decision:?}",
            st.reborn_closes
        );
        if let Err(e) = driver.respond_close(window, decision) {
            fail(st, format!("respond_close reborn: {e:?}"));
        }
    }
}

fn on_destroyed(st: &mut State, driver: &mut dyn WindowDriver, window: WindowId) {
    if window == st.veto {
        // Ids are never reused: the reborn window must mint a fresh id.
        let spec = WindowSpec {
            title: "rf243 probe reborn".to_string(),
            ..WindowSpec::default()
        };
        match driver.create_window(&spec) {
            Ok(id) => {
                eprintln!("[probe] reborn created {id} (veto was {})", st.veto);
                if id == st.veto || id == st.main {
                    fail(st, format!("window id reused: {id}"));
                }
                st.names.insert(id, "reborn");
                st.reborn = Some(id);
            }
            Err(e) => fail(st, format!("reborn create_window: {e:?}")),
        }
    } else if window == st.main {
        // Programmatic close path on the surviving window, then the
        // scheduled `destroy_window` bypass check.
        if let Some(reborn) = st.reborn {
            eprintln!("[probe] op: request_close({reborn})");
            driver.request_close(reborn);
            st.destroy_at = Some(st.ticks + DESTROY_DELAY_TICKS);
        } else {
            fail(st, "main destroyed before reborn existed");
        }
    } else if st.reborn == Some(window) {
        finish(st, driver, final_verdict(st));
    }
}

/// Aggregates step failures with the lifecycle trace contract.
fn final_verdict(st: &State) -> Result<(), String> {
    let mut failures = st.failures.clone();
    let veto_trace = st.trace.get(&st.veto).cloned().unwrap_or_default();
    let expected_veto = [
        "Created",
        "Ready",
        "CloseRequested",
        "CloseRequested",
        "CloseRequested",
        "Destroyed",
    ];
    if veto_trace != expected_veto {
        failures.push(format!("veto trace {veto_trace:?} != {expected_veto:?}"));
    }
    let main_trace = st.trace.get(&st.main).cloned().unwrap_or_default();
    if main_trace.last() != Some(&"Destroyed") || !main_trace.contains(&"CloseRequested") {
        failures.push(format!("main trace unexpected: {main_trace:?}"));
    }
    if let Some(reborn) = st.reborn {
        let reborn_trace = st.trace.get(&reborn).cloned().unwrap_or_default();
        let expected_reborn = ["Created", "Ready", "CloseRequested", "Destroyed"];
        if reborn_trace != expected_reborn {
            failures.push(format!(
                "reborn trace {reborn_trace:?} != {expected_reborn:?} \
                 (a second CloseRequested means destroy_window did not bypass)"
            ));
        }
    } else {
        failures.push("reborn was never created".to_string());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn finish(st: &mut State, driver: &mut dyn WindowDriver, verdict: Result<(), String>) {
    for (window, kinds) in &st.trace {
        let name = st.names.get(window).copied().unwrap_or("?");
        eprintln!("[probe] trace {name} ({window}): {kinds:?}");
    }
    st.verdict = Some(verdict);
    driver.exit();
}

/// Observation state for `--degenerate-create`.
struct Deg {
    id: WindowId,
    saw_created: bool,
    saw_ready: bool,
    saw_destroyed: bool,
    started: Instant,
    verdict: Option<String>,
}

impl Deg {
    fn classify(&self) -> String {
        match (self.saw_created, self.saw_ready, self.saw_destroyed) {
            (_, true, _) => {
                "0x0 spec accepted: Created+Ready observed (driver does not reject zero size)"
                    .to_string()
            }
            (true, false, true) => {
                "Created then Destroyed without Ready (creation degraded)".to_string()
            }
            (false, false, true) => {
                "Destroyed without Created (creation failed before accept)".to_string()
            }
            (true, false, false) => {
                "Created but never Ready/Destroyed within timeout (stuck)".to_string()
            }
            (false, false, false) => "no lifecycle events within timeout".to_string(),
        }
    }
}

/// `--degenerate-create`: observe how the driver answers a 0x0
/// `WindowSpec`. Classification is printed, never asserted — the
/// contract only requires that a failed create leaves no observable
/// window and terminates as `Destroyed`.
fn degenerate() -> ExitCode {
    let mut driver = GpuiDriver::new();
    eprintln!(
        "[probe-degenerate] capabilities: {:?}",
        driver.capabilities()
    );
    let spec = WindowSpec {
        title: "rf243 probe degenerate".to_string(),
        logical_size: LogicalSize::new(0, 0),
        position: None,
        resizable: true,
    };
    let id = match driver.create_window(&spec) {
        Ok(id) => {
            eprintln!("[probe-degenerate] create_window ok {id}");
            id
        }
        Err(e) => {
            eprintln!("[probe-degenerate] create_window err {e:?}");
            eprintln!("[probe-degenerate] classification: create_window returned Err");
            return ExitCode::SUCCESS;
        }
    };

    let state = Rc::new(RefCell::new(Deg {
        id,
        saw_created: false,
        saw_ready: false,
        saw_destroyed: false,
        started: Instant::now(),
        verdict: None,
    }));
    let handler = state.clone();
    driver
        .run(Some(TICK), move |ev, drv| {
            let mut st = handler.borrow_mut();
            if st.verdict.is_some() {
                return;
            }
            match ev {
                DriverEvent::Tick => {
                    if st.started.elapsed() > DEGENERATE_TIMEOUT {
                        st.verdict = Some(st.classify());
                        drv.exit();
                    }
                }
                DriverEvent::Windowed { window, event } => {
                    if window != st.id {
                        return;
                    }
                    eprintln!("[probe-degenerate] {event:?}");
                    match event {
                        WindowLifecycle::Created => st.saw_created = true,
                        WindowLifecycle::Ready => st.saw_ready = true,
                        WindowLifecycle::Destroyed => {
                            st.saw_destroyed = true;
                            st.verdict = Some(st.classify());
                            drv.exit();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .expect("run");

    let class = state
        .borrow()
        .verdict
        .clone()
        .unwrap_or_else(|| "loop ended without classification".to_string());
    eprintln!("[probe-degenerate] classification: {class}");
    ExitCode::SUCCESS
}
