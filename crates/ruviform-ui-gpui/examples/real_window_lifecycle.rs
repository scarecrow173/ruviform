//! Real-OS-window Window/Dock lifecycle check (Issue #431).
//!
//! Opens two GPUI windows (`main`, `inspector`) on the local session.
//! Once both are `Ready`, the Dock workspace runs a committed 2-phase
//! move (v2 → inspector), an aborted move (v1 stays), and a detach /
//! reattach (v1 → inspector). The OS close requests are driven
//! externally (see `evidence/issue-431-real-verify/wm-close.ps1`):
//! the first `inspector` close is deferred (defer path, the window must
//! stay open), the second is approved, then `main` is closed and
//! approved. The normalized trace must equal the shared Window/Dock
//! fixture's `inspector`/`main` entries (`ruviform_ui_test::lifecycle`).
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example real_window_lifecycle`
//!
//! Manual verification only (real OS session). Exits 0 only when the
//! trace and every Dock check passed.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ruviform_ui::dock::{
    DockNode, DockTarget, DockTree, DockWorkspace, ViewInstanceId, WindowCloseDisposition,
};
use ruviform_ui::{
    CloseDecision, DriverEvent, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::GpuiDriver;

const TICK: Duration = Duration::from_millis(100);
const TIMEOUT: Duration = Duration::from_secs(120);

type Trace = BTreeMap<&'static str, Vec<&'static str>>;

struct State {
    main: WindowId,
    inspector: WindowId,
    ws: DockWorkspace,
    ready: BTreeSet<WindowId>,
    docked: bool,
    inspector_closes: u32,
    trace: Trace,
    started: Instant,
    verdict: Option<Result<(), String>>,
}

fn main() -> ExitCode {
    let mut driver = GpuiDriver::new();
    eprintln!("[real] capabilities: {:?}", driver.capabilities());
    let main = create(&mut driver, "rf431 real window main");
    let inspector = create(&mut driver, "rf431 real window inspector");
    let mut ws = DockWorkspace::new();
    ws.register_window(main);
    ws.register_window(inspector);

    let state = Rc::new(RefCell::new(State {
        main,
        inspector,
        ws,
        ready: BTreeSet::new(),
        docked: false,
        inspector_closes: 0,
        trace: Trace::new(),
        started: Instant::now(),
        verdict: None,
    }));
    let handler = state.clone();
    driver
        .run(Some(TICK), move |ev, drv| {
            on_event(&mut handler.borrow_mut(), ev, drv);
        })
        .expect("run");

    let verdict = state.borrow_mut().verdict.take();
    match verdict {
        Some(Ok(())) => {
            eprintln!("[real] PASS");
            ExitCode::SUCCESS
        }
        Some(Err(reason)) => {
            eprintln!("[real] FAIL: {reason}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("[real] FAIL: loop ended without a verdict");
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
    eprintln!("[real] created {id} '{title}'");
    id
}

fn on_event(st: &mut State, ev: DriverEvent, driver: &mut dyn WindowDriver) {
    if st.verdict.is_some() {
        return;
    }
    if matches!(ev, DriverEvent::Tick) && st.started.elapsed() > TIMEOUT {
        return finish(st, driver, Err(format!("timeout after {TIMEOUT:?}")));
    }
    let DriverEvent::Windowed { window, event } = ev else {
        return;
    };
    let name = if window == st.main {
        "main"
    } else {
        "inspector"
    };
    if let Some(kind) = kind(&event) {
        eprintln!("[real] {name}: {kind}");
        st.trace.entry(name).or_default().push(kind);
    }
    match event {
        WindowLifecycle::Ready => {
            st.ready.insert(window);
            if st.ready.len() == 2 && !st.docked {
                st.docked = true;
                if let Err(reason) = dock_moves(st) {
                    finish(st, driver, Err(reason));
                }
            }
        }
        WindowLifecycle::CloseRequested if window == st.inspector => {
            st.inspector_closes += 1;
            // First close: defer-path check — the window must stay open.
            let decision = if st.inspector_closes == 1 {
                CloseDecision::Defer
            } else {
                CloseDecision::Approve
            };
            eprintln!("[real] inspector: respond {decision:?}");
            if let Err(e) = driver.respond_close(window, decision) {
                finish(st, driver, Err(format!("respond_close: {e}")));
            }
        }
        WindowLifecycle::CloseRequested => {
            eprintln!("[real] main: respond Approve");
            if let Err(e) = driver.respond_close(window, CloseDecision::Approve) {
                finish(st, driver, Err(format!("respond_close: {e}")));
            }
        }
        WindowLifecycle::Destroyed if window == st.inspector => {
            // Consumer policy: relocate the inspector's views to main.
            let relocated = st.ws.remove_window(
                st.inspector,
                WindowCloseDisposition::RelocateTo { target: st.main },
            );
            match relocated {
                Ok(moved) if moved.len() == 2 => {}
                other => finish(st, driver, Err(format!("relocate views: {other:?}"))),
            }
        }
        WindowLifecycle::Destroyed => {
            let expected = expected_trace();
            let verdict = if st.trace == expected {
                Ok(())
            } else {
                Err(format!("trace {:?} != expected {expected:?}", st.trace))
            };
            finish(st, driver, verdict);
        }
        _ => {}
    }
}

/// Committed 2-phase move, aborted move, detach/reattach — each step
/// is checked against the Dock ownership it must leave behind.
fn dock_moves(st: &mut State) -> Result<(), String> {
    let (main, inspector) = (st.main, st.inspector);
    let (v1, v2) = (view(1)?, view(2)?);
    let ws = &mut st.ws;
    ws.attach_tree(main, tree(&[v1, v2])?)
        .map_err(|e| format!("attach tree: {e:?}"))?;
    let token = ws
        .begin_move(v2, DockTarget::Window(inspector))
        .map_err(|e| format!("begin move: {e:?}"))?;
    ws.commit_move(token)
        .map_err(|e| format!("commit move: {e:?}"))?;
    check(
        ws.owner_of(v2) == Some(inspector),
        "v2 committed to inspector",
    )?;
    let token = ws
        .begin_move(v1, DockTarget::Window(inspector))
        .map_err(|e| format!("begin rollback move: {e:?}"))?;
    ws.abort_move(token)
        .map_err(|e| format!("abort move: {e:?}"))?;
    check(ws.owner_of(v1) == Some(main), "v1 abort keeps main")?;
    ws.detach_view(v1)
        .map_err(|e| format!("detach view: {e:?}"))?;
    check(ws.is_detached(v1), "v1 detached")?;
    ws.attach_view(v1, DockTarget::Window(inspector))
        .map_err(|e| format!("reattach view: {e:?}"))?;
    check(
        ws.owner_of(v1) == Some(inspector),
        "v1 reattached to inspector",
    )
}

fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("dock check failed: {what}"))
    }
}

fn finish(st: &mut State, driver: &mut dyn WindowDriver, verdict: Result<(), String>) {
    st.verdict = Some(verdict);
    driver.exit();
}

/// Lifecycle kinds the trace records (geometry/focus are timing-dependent).
fn kind(event: &WindowLifecycle) -> Option<&'static str> {
    Some(match event {
        WindowLifecycle::Created => "Created",
        WindowLifecycle::Ready => "Ready",
        WindowLifecycle::CloseRequested => "CloseRequested",
        WindowLifecycle::Destroyed => "Destroyed",
        _ => return None,
    })
}

/// Same `inspector`/`main` entries as the shared fixture's
/// `expected_trace()`; `late`/`reborn` belong to the headless scenario.
fn expected_trace() -> Trace {
    BTreeMap::from([
        (
            "main",
            vec!["Created", "Ready", "CloseRequested", "Destroyed"],
        ),
        (
            "inspector",
            vec![
                "Created",
                "Ready",
                "CloseRequested",
                "CloseRequested",
                "Destroyed",
            ],
        ),
    ])
}

fn view(raw: u64) -> Result<ViewInstanceId, String> {
    ViewInstanceId::new(raw).ok_or_else(|| format!("zero view id {raw}"))
}

fn tree(views: &[ViewInstanceId]) -> Result<DockTree, String> {
    let root = DockNode::tabs(views.to_vec(), 0).map_err(|e| format!("tabs: {e:?}"))?;
    DockTree::new(root).map_err(|e| format!("tree: {e:?}"))
}
