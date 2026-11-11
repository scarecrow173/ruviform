//! `windowed_test_app` — the windowed app-under-test for
//! `rvf.uitest.v1` (Issue #313 slice S2, RF-GT-04): real GPUI windows
//! driven through [`WindowedIo`], the windowed counterpart of
//! `ruviform-ui-test`'s `dummy_app`.
//!
//! Thread layout: the control thread (main) serves the protocol via
//! [`serve_from_env`]; a spawned app thread owns `GpuiDriver` +
//! `UiRuntime` — neither is `Send`, so the app is constructed inside
//! that thread and every control op crosses as a `UiJob`. A dead
//! driver loop is a dead AUT: `run` errors/panics exit the process
//! (fail-closed) instead of leaving the control channel hung.
//!
//! Args (`dummy_app` parity):
//!
//! - `--fail-init`: report `ReadyState::Failed` — the handshake then
//!   answers the failed half of the ready/failed contract.
//! - `--windows N`: create `N` initial windows before serving
//!   (default 1). Real `Ready` arrives from the OS frame — nothing
//!   is fabricated.
//!
//! Requires an interactive desktop session (real OS windows). All
//! diagnostics go to stderr — stdout is the control channel.

use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ruviform_ui::{
    CloseDecision, DataRegistry, DriverEvent, ErrorBody, Event, LifecycleState, RoutedEvent,
    UiRuntime, WindowDriver, WindowLifecycle, WindowSpec, codes,
};
use ruviform_ui_gpui::{GpuiDriver, WindowedIo};
use ruviform_ui_test::{
    AgentConfig, AppMeta, FixtureController, FixtureReceipt, FixtureSpec, ReadyState, ServeOutcome,
    ShutdownMode, TestApp, TestIo, deliver_routed, serve_from_env,
};
use serde_json::{Value, json};

/// How the app answers `CloseRequested` — the fixture-controlled
/// knob exercising approve / veto / defer / default-approve paths.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClosePolicy {
    /// Leave the request unanswered — the host applies its
    /// per-window default `Approve`.
    Unanswered,
    /// `respond_close(Approve)`.
    Approve,
    /// `respond_close(Veto)` — the close-cancel path.
    Veto,
    /// `respond_close(Defer)` — stays open marked close-pending.
    Defer,
}

impl ClosePolicy {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "unanswered" => Some(Self::Unanswered),
            "approve" => Some(Self::Approve),
            "veto" => Some(Self::Veto),
            "defer" => Some(Self::Defer),
            _ => None,
        }
    }
}

/// App state shared with fixtures (registration-time sharing — no
/// control surface beyond what the app itself exposes).
struct DummyState {
    policy: ClosePolicy,
}

/// The dummy consumer's normal driver wiring: normalize → dispatch →
/// deliver, exactly what a production `DriverEvent` handler does —
/// identical semantics to `dummy_app`'s `DummyApp`.
struct DummyApp {
    runtime: UiRuntime,
    state: Arc<Mutex<DummyState>>,
}

impl DummyApp {
    fn new(state: Arc<Mutex<DummyState>>) -> Self {
        let mut runtime = UiRuntime::new(DataRegistry::new());
        runtime
            .register_service(ruviform_ui::DemoService::new())
            .unwrap_or_else(|e| eprintln!("windowed_test_app: service register failed: {e}"));
        Self { runtime, state }
    }
}

impl TestApp for DummyApp {
    fn on_event(&mut self, dev: DriverEvent, drv: &mut dyn WindowDriver) {
        match dev {
            DriverEvent::Windowed { window, event } => match event {
                WindowLifecycle::Action(intent) => {
                    for re in self.runtime.dispatch_window_intent(window, intent) {
                        deliver_routed(drv, &re);
                    }
                }
                WindowLifecycle::CloseRequested => {
                    let decision = match self.state.lock().unwrap().policy {
                        ClosePolicy::Unanswered => None,
                        ClosePolicy::Approve => Some(CloseDecision::Approve),
                        ClosePolicy::Veto => Some(CloseDecision::Veto),
                        ClosePolicy::Defer => Some(CloseDecision::Defer),
                    };
                    if let Some(d) = decision {
                        let _ = drv.respond_close(window, d);
                    }
                    // The same `closing` lifecycle event the legacy
                    // `handle_window` path emitted for CloseRequested.
                    deliver_routed(
                        drv,
                        &RoutedEvent {
                            window: Some(window),
                            event: Event::lifecycle(LifecycleState::Closing),
                        },
                    );
                }
                WindowLifecycle::Destroyed => {
                    // The app's own window-job policy: a dead window's
                    // jobs are cancelled by the consumer, never the host.
                    self.runtime.cancel_window_jobs(window);
                }
                _ => {}
            },
            DriverEvent::Tick => {
                for re in self.runtime.drain_jobs_routed() {
                    deliver_routed(drv, &re);
                }
            }
            _ => {}
        }
    }

    /// Cleanup accounting reported verbatim as `CleanupReport::app`.
    fn shutdown(&mut self, _mode: ShutdownMode, drv: &mut dyn WindowDriver) -> Value {
        json!({
            "jobs_running": self.runtime.has_running_jobs(),
            "windows_open": drv.windows().len(),
        })
    }
}

/// Upper bound for a fixture's real-`Ready` wait — a live window
/// reports `Ready` on its first frame; beyond this the backend is
/// genuinely stalled, so the fixture fails `unavailable` honestly.
const READY_POLL: Duration = Duration::from_millis(25);
/// `READY_POLL` intervals ≈ 5 s.
const READY_POLLS: u32 = 200;

/// Waits for `id`'s real `Ready` — the windowed fixture never
/// fabricates the event the way the headless fixture does.
fn wait_ready(io: &mut dyn TestIo, id: ruviform_ui::WindowId) -> Result<(), ErrorBody> {
    for _ in 0..READY_POLLS {
        if io.windows().iter().any(|m| m.id == id.get() && m.ready) {
            return Ok(());
        }
        thread::sleep(READY_POLL);
    }
    Err(ErrorBody::new(
        codes::UNAVAILABLE,
        format!("window {id} did not report Ready"),
    ))
}

/// The `windows` fixture: creates `params.count` windows (default 2)
/// and — unless `params.ready == false` — waits for each window's
/// real `Ready` before replying. Applies `params.close_policy` when
/// present. No user-gesture simulation — fixtures arrange host-side
/// state only.
struct WindowsFixture {
    state: Arc<Mutex<DummyState>>,
}

impl FixtureController for WindowsFixture {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn prepare(
        &mut self,
        spec: &FixtureSpec,
        io: &mut dyn TestIo,
    ) -> Result<FixtureReceipt, ErrorBody> {
        let count = spec
            .params
            .get("count")
            .and_then(Value::as_u64)
            .unwrap_or(2)
            .min(8);
        let ready = spec
            .params
            .get("ready")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        // Small windows are the honest way to keep a screenshot
        // reply under the 1 MiB frame cap (`MAX_INTENT_BYTES`).
        let width = spec
            .params
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(800)
            .clamp(1, 16384) as u32;
        let height = spec
            .params
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(600)
            .clamp(1, 16384) as u32;
        let mut created = Vec::new();
        for _ in 0..count {
            let id = io.create_window(&WindowSpec {
                title: "dummy".to_string(),
                logical_size: ruviform_ui::LogicalSize::new(width, height),
                position: None,
                resizable: true,
            })?;
            if ready {
                wait_ready(io, id)?;
            }
            created.push(id.get());
        }
        if let Some(policy) = spec.params.get("close_policy").and_then(Value::as_str) {
            let Some(p) = ClosePolicy::parse(policy) else {
                return Err(ErrorBody::new(
                    codes::INVALID_PARAMS,
                    format!("unknown close_policy {policy:?}"),
                ));
            };
            self.state.lock().unwrap().policy = p;
        }
        Ok(FixtureReceipt {
            name: spec.name.clone(),
            applied: json!({
                "windows": created,
                "seed": spec.seed,
            }),
        })
    }
}

/// The `ipc_emit` fixture: injects one raw IPC body into
/// `params.window`'s inbound channel — the path an `ipc_disconnect`
/// fault suppresses pre-decode. Valid bodies become `Action` events;
/// malformed bodies are answered driver-side with an `Event::error`.
struct IpcEmitFixture;

impl FixtureController for IpcEmitFixture {
    fn name(&self) -> &'static str {
        "ipc_emit"
    }

    fn prepare(
        &mut self,
        spec: &FixtureSpec,
        io: &mut dyn TestIo,
    ) -> Result<FixtureReceipt, ErrorBody> {
        let Some(raw) = spec.params.get("window").and_then(Value::as_u64) else {
            return Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                "ipc_emit requires a \"window\" param",
            ));
        };
        let Some(window) = ruviform_ui::WindowId::new(raw) else {
            return Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                "window must be nonzero",
            ));
        };
        let body = spec
            .params
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or("{}")
            .to_string();
        io.emit_ipc_body(window, &body);
        Ok(FixtureReceipt {
            name: spec.name.clone(),
            applied: json!({ "window": raw }),
        })
    }
}

fn main() -> ExitCode {
    let state = Arc::new(Mutex::new(DummyState {
        policy: ClosePolicy::Approve,
    }));
    let (mut io, ui) = WindowedIo::new();
    io.add_fixture(Box::new(WindowsFixture {
        state: Arc::clone(&state),
    }));
    io.add_fixture(Box::new(IpcEmitFixture));

    let mut windows = 1u32;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fail-init" => io.set_ready_state(ReadyState::Failed {
                reason: "init requested to fail (--fail-init)".to_string(),
            }),
            "--windows" => {
                let Some(n) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                    eprintln!("windowed_test_app: --windows requires an integer");
                    return ExitCode::from(2);
                };
                windows = n.min(8);
            }
            other => {
                eprintln!("windowed_test_app: unknown arg {other:?}");
                return ExitCode::from(2);
            }
        }
    }

    // The app thread owns `GpuiDriver` + `UiRuntime` — both are
    // non-`Send`, so `DummyApp` is constructed inside the thread and
    // control ops cross as `UiJob`s. A dead loop ends the process:
    // the control channel must never outlive its backend.
    let app_state = Arc::clone(&state);
    let app_thread = thread::spawn(move || {
        let mut driver = GpuiDriver::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ui.run(&mut driver, None, DummyApp::new(app_state))
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                eprintln!("windowed_test_app: driver run failed: {e}");
                std::process::exit(3);
            }
            Err(_) => {
                eprintln!("windowed_test_app: driver loop panicked");
                std::process::exit(3);
            }
        }
    });

    for _ in 0..windows {
        let _ = io.create_window(&WindowSpec::default());
    }

    let config = AgentConfig::new(AppMeta {
        name: "windowed_test_app".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        revision: option_env!("RVF_APP_REVISION").map(str::to_string),
    });
    let outcome = serve_from_env(&mut io, config);
    eprintln!("windowed_test_app: {outcome:?}");
    let _ = app_thread.join();
    match outcome {
        ServeOutcome::Shutdown { .. }
        | ServeOutcome::DriverGone { .. }
        | ServeOutcome::Disabled { .. } => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
