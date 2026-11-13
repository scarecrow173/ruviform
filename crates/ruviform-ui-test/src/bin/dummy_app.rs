//! `dummy_app` — the Issue #312 dummy consumer (RF-GT-03 acceptance),
//! on the neutral driver vocabulary (Issue #313 slice S2).
//!
//! A headless app-under-test composing `UiRuntime` + `FakeDriver`
//! through [`HeadlessIo`], serving `rvf.uitest.v1` via
//! [`serve_from_env`]. It is the showcase stand-in for the acceptance
//! flow: test-mode start → capability query → fixture → observation →
//! graceful/abnormal end → complete cleanup.
//!
//! Args:
//!
//! - `--fail-init`: report `ReadyState::Failed` — the handshake then
//!   answers the failed half of the ready/failed contract.
//! - `--windows N`: create `N` initial windows before serving
//!   (default 1; each emits `Ready`).
//! - `--stall before-serve|after-serve`: park forever instead of
//!   exiting — `before-serve` never opens the control channel,
//!   `after-serve` ends the protocol session but never exits. The
//!   non-exit fixture for driver timeout tests; the driver must
//!   kill and reap the process.
//!
//! All diagnostics go to stderr — stdout is the control channel.

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use ruviform_ui::{
    CloseDecision, DataRegistry, DriverEvent, ErrorBody, Event, LifecycleState, RoutedEvent,
    UiRuntime, WindowDriver, WindowLifecycle, WindowSpec, codes,
};
use ruviform_ui_test::{
    AgentConfig, AppMeta, FixtureController, FixtureReceipt, FixtureSpec, HeadlessIo, ReadyState,
    ServeOutcome, ShutdownMode, TestApp, TestIo, deliver_routed, serve_from_env,
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
/// deliver, exactly what a production `DriverEvent` handler does.
struct DummyApp {
    runtime: UiRuntime,
    state: Arc<Mutex<DummyState>>,
}

impl DummyApp {
    fn new(state: Arc<Mutex<DummyState>>) -> Self {
        let mut runtime = UiRuntime::new(DataRegistry::new());
        runtime
            .register_service(ruviform_ui::DemoService::new())
            .unwrap_or_else(|e| eprintln!("dummy_app: service register failed: {e}"));
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

/// The `windows` fixture: creates `params.count` windows (default 2),
/// emits `Ready` for each unless `params.ready == false`, and applies
/// `params.close_policy` when present. No user-gesture simulation —
/// fixtures arrange host-side state only.
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
        let mut created = Vec::new();
        for _ in 0..count {
            let id = io.create_window(&WindowSpec {
                title: "dummy".to_string(),
                logical_size: ruviform_ui::LogicalSize::new(800, 600),
                position: None,
                resizable: true,
            })?;
            if ready {
                io.emit(DriverEvent::Windowed {
                    window: id,
                    event: WindowLifecycle::Ready,
                });
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

/// Where `--stall` parks the process — the non-exit fixture for
/// driver timeout tests.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StallPhase {
    /// Park before `serve_from_env` — the control channel never opens.
    BeforeServe,
    /// Park after `serve_from_env` returns — the protocol session
    /// ended (shutdown / driver EOF / disabled) but the process
    /// never exits.
    AfterServe,
}

/// The stall fixture: never return — the driver must kill and reap
/// the process. Models an AUT that hangs instead of exiting.
fn park_forever(phase: &str) -> ! {
    eprintln!("dummy_app: --stall {phase}: process will not exit");
    loop {
        std::thread::park();
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
    let mut io = HeadlessIo::new(DummyApp::new(Arc::clone(&state)));
    io.add_fixture(Box::new(WindowsFixture {
        state: Arc::clone(&state),
    }));
    io.add_fixture(Box::new(IpcEmitFixture));

    let mut windows = 1u32;
    let mut stall = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fail-init" => io.set_ready_state(ReadyState::Failed {
                reason: "init requested to fail (--fail-init)".to_string(),
            }),
            "--windows" => {
                let Some(n) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                    eprintln!("dummy_app: --windows requires an integer");
                    return ExitCode::from(2);
                };
                windows = n.min(8);
            }
            "--stall" => {
                let Some(phase) = args.next() else {
                    eprintln!("dummy_app: --stall requires before-serve|after-serve");
                    return ExitCode::from(2);
                };
                stall = Some(match phase.as_str() {
                    "before-serve" => StallPhase::BeforeServe,
                    "after-serve" => StallPhase::AfterServe,
                    other => {
                        eprintln!("dummy_app: unknown stall phase {other:?}");
                        return ExitCode::from(2);
                    }
                });
            }
            other => {
                eprintln!("dummy_app: unknown arg {other:?}");
                return ExitCode::from(2);
            }
        }
    }
    for _ in 0..windows {
        if let Ok(id) = io.create_window(&WindowSpec::default()) {
            io.emit(DriverEvent::Windowed {
                window: id,
                event: WindowLifecycle::Ready,
            });
        }
    }

    if stall == Some(StallPhase::BeforeServe) {
        park_forever("before-serve");
    }
    let config = AgentConfig::new(AppMeta {
        name: "dummy_app".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        revision: option_env!("RVF_APP_REVISION").map(str::to_string),
    });
    let outcome = serve_from_env(&mut io, config);
    eprintln!("dummy_app: {outcome:?}");
    if stall == Some(StallPhase::AfterServe) {
        park_forever("after-serve");
    }
    match outcome {
        ServeOutcome::Shutdown { .. }
        | ServeOutcome::DriverGone { .. }
        | ServeOutcome::Disabled { .. } => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
