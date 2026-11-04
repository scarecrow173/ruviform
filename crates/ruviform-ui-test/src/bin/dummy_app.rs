//! `dummy_app` — the Issue #312 dummy consumer (RF-GT-03 acceptance).
//!
//! A headless app-under-test composing `UiRuntime` + `FakeHost`
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
//!
//! All diagnostics go to stderr — stdout is the control channel.

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use ruviform_ui::{
    CloseDecision, DataRegistry, ErrorBody, HostEvent, UiRuntime, WindowConfig, WindowEvent,
    WindowHandle, codes,
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

/// The dummy consumer's normal host wiring: normalize → dispatch →
/// deliver, exactly what a production `on_event` closure does.
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
    fn on_event(&mut self, wev: WindowEvent, wh: &mut dyn WindowHandle) {
        match wev {
            WindowEvent::Windowed { window, event } => {
                if matches!(event, HostEvent::CloseRequested) {
                    let decision = match self.state.lock().unwrap().policy {
                        ClosePolicy::Unanswered => None,
                        ClosePolicy::Approve => Some(CloseDecision::Approve),
                        ClosePolicy::Veto => Some(CloseDecision::Veto),
                        ClosePolicy::Defer => Some(CloseDecision::Defer),
                    };
                    if let Some(d) = decision {
                        let _ = wh.respond_close(window, d);
                    }
                }
                if matches!(event, HostEvent::Destroyed) {
                    // The app's own window-job policy: a dead window's
                    // jobs are cancelled by the consumer, never the host.
                    self.runtime.cancel_window_jobs(window);
                }
                for re in self.runtime.handle_window(window, event) {
                    deliver_routed(wh, &re);
                }
            }
            WindowEvent::Tick => {
                for re in self.runtime.drain_jobs_routed() {
                    deliver_routed(wh, &re);
                }
            }
            _ => {}
        }
    }

    /// Cleanup accounting reported verbatim as `CleanupReport::app`.
    fn shutdown(&mut self, _mode: ShutdownMode, wh: &mut dyn WindowHandle) -> Value {
        json!({
            "jobs_running": self.runtime.has_running_jobs(),
            "windows_open": wh.windows().len(),
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
            let id = io.create_window(&WindowConfig {
                title: "dummy".to_string(),
                width: 800,
                height: 600,
                position: None,
                url: "rvf://app/index.html".to_string(),
                devtools: false,
            })?;
            if ready {
                io.emit(WindowEvent::Windowed {
                    window: id,
                    event: HostEvent::Ready {
                        url: "rvf://app/index.html".to_string(),
                    },
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

/// The `ipc_emit` fixture: emits one `HostEvent::Message` into
/// `params.window` — the inbound path an `ipc_disconnect` fault
/// suppresses. The body goes through the runtime's normal intent
/// parse, so malformed bodies produce the normal error event.
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
        io.emit(WindowEvent::Windowed {
            window,
            event: HostEvent::Message(body),
        });
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
            other => {
                eprintln!("dummy_app: unknown arg {other:?}");
                return ExitCode::from(2);
            }
        }
    }
    for _ in 0..windows {
        if let Ok(id) = io.create_window(&WindowConfig::default()) {
            io.emit(WindowEvent::Windowed {
                window: id,
                event: HostEvent::Ready {
                    url: "rvf://app/index.html".to_string(),
                },
            });
        }
    }

    let config = AgentConfig::new(AppMeta {
        name: "dummy_app".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        revision: option_env!("RVF_APP_REVISION").map(str::to_string),
    });
    let outcome = serve_from_env(&mut io, config);
    eprintln!("dummy_app: {outcome:?}");
    match outcome {
        ServeOutcome::Shutdown { .. }
        | ServeOutcome::DriverGone { .. }
        | ServeOutcome::Disabled { .. } => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
