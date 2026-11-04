//! `TestAgent` — the in-app `rvf.uitest.v1` server (Issue #312).
//!
//! The agent is a pure dispatcher: [`TestAgent::handle`] consumes one
//! intent body, drives the app's [`TestIo`] port, and returns the
//! `Event`s to write back. [`serve`] runs the transport-agnostic
//! frame loop shared by stdio and loopback-TCP; `serve_stdio` /
//! `serve_tcp` / [`serve_from_env`] are the app-facing entry points.
//!
//! Lifecycle contract:
//!
//! - `test.hello` must arrive first — every earlier op answers
//!   `unauthenticated` (fail-closed); a second hello inside a live
//!   session is rejected the same way.
//! - A driver EOF runs graceful cleanup and ends serving
//!   (`ServeOutcome::DriverGone`); `test.shutdown` replies `{bye,
//!   cleanup}` then closes (`ServeOutcome::Shutdown`).
//! - `RVF_TEST_DISABLE` or [`TestAgent::set_enabled`] refuse every
//!   serve path (`ServeOutcome::Disabled`).
//!
//! **RT contract:** non-RT only.

use std::io::{Read, Write};
use std::net::TcpStream;

use ruviform_ui::ipc::{ErrorBody, Event, codes, parse_intent};
use ruviform_ui::{UiError, WindowEvent, WindowId};
use serde_json::{Value, json};

use crate::io::TestIo;
use crate::proto::{
    AppMeta, CleanupReport, ClockDomain, FaultKind, FixtureSpec, Notice, PROTOCOL_VERSION,
    ReadyState, ServeOutcome, ShutdownMode, Welcome, ops, test_codes,
};
use crate::transport::{
    AgentTransport, FrameRead, disabled_from_env, expected_secret_from_env, gen_token, read_frame,
    transport_from_env, write_frame,
};

/// Static agent configuration supplied by the embedding app.
pub struct AgentConfig {
    /// Application identity reported in [`Welcome::app`].
    pub app: AppMeta,
    /// `ruviform-ui` revision reported in [`Welcome::ruviform_revision`].
    pub ruviform_revision: String,
}

impl AgentConfig {
    /// Config for an app embedding `ruviform-ui-test` directly.
    #[must_use]
    pub fn new(app: AppMeta) -> Self {
        Self {
            app,
            ruviform_revision: ruviform_ui::version().to_string(),
        }
    }
}

/// Live session state (post-handshake).
struct Session {
    id: String,
    run_id: String,
}

/// The `rvf.uitest.v1` server state.
pub struct TestAgent {
    config: AgentConfig,
    expected_secret: Option<String>,
    enabled: bool,
    session: Option<Session>,
    ended: Option<CleanupReport>,
    notify_seq: u64,
}

impl TestAgent {
    /// Creates an agent. `expected_secret` is the run-bound token
    /// `test.hello` must present — `Some` for the TCP transport,
    /// `None` for stdio.
    #[must_use]
    pub fn new(config: AgentConfig, expected_secret: Option<String>) -> Self {
        Self {
            config,
            expected_secret,
            enabled: true,
            session: None,
            ended: None,
            notify_seq: 0,
        }
    }

    /// Runtime enable/disable — a disabled agent answers every frame
    /// with `disabled` and refuses `serve`.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// The established session id, when hello has completed.
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.session.as_ref().map(|s| s.id.as_str())
    }

    /// Handles one intent body; returns the `Event`s to write back
    /// (including drained async notices after every command).
    pub fn handle(&mut self, io: &mut dyn TestIo, body: &str) -> Vec<Event> {
        if !self.enabled {
            return vec![Event::error(ErrorBody::new(
                test_codes::DISABLED,
                "test control is disabled",
            ))];
        }
        let intent = match parse_intent(body) {
            Ok(i) => i,
            Err(e) => return vec![Event::error(e)],
        };
        if intent.op != ops::HELLO && self.session.is_none() {
            return vec![Event::result_err(
                intent.id,
                ErrorBody::new(
                    test_codes::UNAUTHENTICATED,
                    "test.hello must be the first op",
                ),
            )];
        }
        let mut events = self.dispatch(io, intent);
        events.extend(self.drain_notices(io));
        events
    }

    fn drain_notices(&mut self, io: &mut dyn TestIo) -> Vec<Event> {
        io.notices()
            .into_iter()
            .map(|n: Notice| {
                self.notify_seq += 1;
                Event::Notify {
                    seq: self.notify_seq,
                    data: json!({ "notice": n }),
                }
            })
            .collect()
    }

    fn dispatch(&mut self, io: &mut dyn TestIo, intent: ruviform_ui::ipc::Intent) -> Vec<Event> {
        let ruviform_ui::ipc::Intent { id, op, params, .. } = intent;
        match op.as_str() {
            ops::HELLO => vec![self.hello(io, id, &params)],
            ops::CAPABILITIES => {
                let mut caps =
                    serde_json::to_value(io.capabilities()).unwrap_or_else(|_| json!({}));
                caps["session_id"] = json!(self.session.as_ref().map(|s| s.id.as_str()));
                caps["windows"] = serde_json::to_value(io.windows()).unwrap_or_else(|_| json!([]));
                vec![Event::result_ok(id, caps)]
            }
            ops::FIXTURE_PREPARE => Self::gated(io, id, "fixture", |io| {
                let spec: FixtureSpec =
                    serde_json::from_value(params.clone()).map_err(|e| invalid(&e))?;
                io.prepare_fixture(&spec)
                    .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
            }),
            ops::CLOCK_ADVANCE => Self::clock_advance(io, id, &params),
            ops::FAULT_SET => Self::fault(io, id, &params, true),
            ops::FAULT_CLEAR => Self::fault(io, id, &params, false),
            ops::WINDOW_REQUEST_CLOSE => Self::window_request_close(io, id, &params),
            ops::OBSERVE_WINDOWS => Self::gated(io, id, "windows", |io| {
                Ok(json!({ "windows": io.windows() }))
            }),
            ops::OBSERVE_RECEIPTS => Self::gated(io, id, "receipts", |io| {
                let since = params.get("since").and_then(Value::as_u64).unwrap_or(0);
                let limit = params
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map_or(256, |l| usize::try_from(l).unwrap_or(256).min(4096));
                Ok(serde_json::to_value(io.receipts(since, limit)).unwrap_or_else(|_| json!({})))
            }),
            ops::OBSERVE_SCREENSHOT => Self::screenshot(io, id, &params),
            ops::NOTICES => vec![Event::result_ok(id, json!({ "notices": io.notices() }))],
            ops::SHUTDOWN => vec![self.shutdown(io, id, &params)],
            _ => vec![Event::result_err(
                id,
                ErrorBody::new(codes::UNKNOWN_OP, format!("unregistered op {op:?}")),
            )],
        }
    }

    /// Runs `f` after the capability check for `cap` — observation
    /// capabilities are looked up in `observation`, others in their
    /// own field.
    fn gated(
        io: &mut dyn TestIo,
        id: String,
        cap: &str,
        f: impl FnOnce(&mut dyn TestIo) -> Result<Value, ErrorBody>,
    ) -> Vec<Event> {
        if let Err(e) = Self::check_cap(io, cap) {
            return vec![Event::result_err(id, e)];
        }
        vec![match f(io) {
            Ok(v) => Event::result_ok(id, v),
            Err(e) => Event::result_err(id, e),
        }]
    }

    fn check_cap(io: &dyn TestIo, cap: &str) -> Result<(), ErrorBody> {
        let c = io.capabilities();
        let granted = match cap {
            "fixture" => c.fixture,
            "screenshot" => c.screenshot,
            _ => c.observation.iter().any(|o| o == cap),
        };
        if granted {
            Ok(())
        } else {
            Err(ErrorBody::new(
                codes::UNAVAILABLE,
                format!("capability {cap:?} is not declared by this host"),
            ))
        }
    }

    fn hello(&mut self, io: &mut dyn TestIo, id: String, params: &Value) -> Event {
        if self.session.is_some() {
            return Event::result_err(
                id,
                ErrorBody::new(
                    test_codes::UNAUTHENTICATED,
                    "a session is already established",
                ),
            );
        }
        if let ReadyState::Failed { reason } = io.ready_state() {
            return Event::result_err(
                id,
                ErrorBody::new(
                    codes::UNAVAILABLE,
                    format!("app-under-test is not ready: {reason}"),
                ),
            );
        }
        if let Some(expected) = &self.expected_secret {
            let presented = params.get("secret").and_then(Value::as_str);
            if presented != Some(expected.as_str()) {
                return Event::result_err(
                    id,
                    ErrorBody::new(test_codes::UNAUTHENTICATED, "missing or invalid run secret"),
                );
            }
        }
        let Some(run_id) = params.get("run_id").and_then(Value::as_str) else {
            return Event::result_err(
                id,
                ErrorBody::new(codes::INVALID_PARAMS, "test.hello requires \"run_id\""),
            );
        };
        let session = Session {
            id: format!("s-{}", gen_token()),
            run_id: run_id.to_string(),
        };
        let welcome = Welcome {
            v: PROTOCOL_VERSION,
            session_id: session.id.clone(),
            run_id: session.run_id.clone(),
            app: self.config.app.clone(),
            ruviform_revision: self.config.ruviform_revision.clone(),
            backend: io.backend(),
            capabilities: io.capabilities(),
            windows: io.windows(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        };
        self.session = Some(session);
        Event::result_ok(
            id,
            serde_json::to_value(welcome).unwrap_or_else(|_| json!({})),
        )
    }

    fn clock_advance(io: &mut dyn TestIo, id: String, params: &Value) -> Vec<Event> {
        let Some(domain_s) = params.get("domain").and_then(Value::as_str) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(codes::INVALID_PARAMS, "clock.advance requires \"domain\""),
            )];
        };
        let Some(ticks) = params.get("ticks").and_then(Value::as_u64) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(
                    codes::INVALID_PARAMS,
                    "clock.advance requires a numeric \"ticks\"",
                ),
            )];
        };
        let ticks = ticks.min(4096);
        let supported = io
            .capabilities()
            .clock_domains
            .iter()
            .any(|d| d == domain_s);
        let Some(domain) = ClockDomain::parse(domain_s).filter(|_| supported) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(
                    codes::UNAVAILABLE,
                    format!("clock domain {domain_s:?} is not controllable here"),
                ),
            )];
        };
        for _ in 0..ticks {
            io.emit(WindowEvent::Tick);
        }
        vec![Event::result_ok(
            id,
            json!({ "domain": domain.as_str(), "advanced": ticks }),
        )]
    }

    fn fault(io: &mut dyn TestIo, id: String, params: &Value, on: bool) -> Vec<Event> {
        let Some(kind_s) = params.get("kind").and_then(Value::as_str) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(codes::INVALID_PARAMS, "fault op requires \"kind\""),
            )];
        };
        let allowed = io.capabilities().faults.iter().any(|f| f == kind_s);
        let Some(kind) = FaultKind::parse(kind_s).filter(|_| allowed) else {
            return vec![Event::result_err(
                id,
                ErrorBody::new(
                    codes::UNAVAILABLE,
                    format!("fault {kind_s:?} is not in this host's allowlist"),
                ),
            )];
        };
        let window = match window_param(params) {
            Ok(w) => w,
            Err(e) => return vec![Event::result_err(id, e)],
        };
        vec![match io.set_fault(kind, window, on) {
            Ok(()) => Event::result_ok(id, json!({ "kind": kind.as_str(), "applied": on })),
            Err(e) => Event::result_err(id, e),
        }]
    }

    fn window_request_close(io: &mut dyn TestIo, id: String, params: &Value) -> Vec<Event> {
        let window = match window_param(params) {
            Ok(Some(w)) => w,
            Ok(None) => {
                return vec![Event::result_err(
                    id,
                    ErrorBody::new(codes::INVALID_PARAMS, "request_close requires \"window\""),
                )];
            }
            Err(e) => return vec![Event::result_err(id, e)],
        };
        io.emit(WindowEvent::Windowed {
            window,
            event: ruviform_ui::HostEvent::CloseRequested,
        });
        vec![Event::result_ok(
            id,
            json!({ "window": window.get(), "requested": true }),
        )]
    }

    fn screenshot(io: &mut dyn TestIo, id: String, params: &Value) -> Vec<Event> {
        if let Err(e) = Self::check_cap(io, "screenshot") {
            return vec![Event::result_err(id, e)];
        }
        let window = match window_param(params) {
            Ok(Some(w)) => w,
            Ok(None) => {
                return vec![Event::result_err(
                    id,
                    ErrorBody::new(codes::INVALID_PARAMS, "screenshot requires \"window\""),
                )];
            }
            Err(e) => return vec![Event::result_err(id, e)],
        };
        vec![match io.screenshot(window) {
            Ok(shot) => {
                Event::result_ok(id, serde_json::to_value(shot).unwrap_or_else(|_| json!({})))
            }
            Err(e) => Event::result_err(id, e),
        }]
    }

    fn shutdown(&mut self, io: &mut dyn TestIo, id: String, params: &Value) -> Event {
        let mode = match params.get("mode").and_then(Value::as_str) {
            None | Some("graceful") => ShutdownMode::Graceful,
            Some("now") => ShutdownMode::Now,
            Some(other) => {
                return Event::result_err(
                    id,
                    ErrorBody::new(
                        codes::INVALID_PARAMS,
                        format!("unknown shutdown mode {other:?}"),
                    ),
                );
            }
        };
        let report = io.request_shutdown(mode);
        self.ended = Some(report.clone());
        Event::result_ok(id, json!({ "bye": true, "cleanup": report }))
    }
}

/// Parses the optional `window` param into a [`WindowId`].
fn window_param(params: &Value) -> Result<Option<WindowId>, ErrorBody> {
    match params.get("window") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(WindowId::new)
            .map(Some)
            .ok_or_else(|| invalid("window must be a nonzero integer")),
    }
}

fn invalid(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(codes::INVALID_PARAMS, e.to_string())
}

/// The transport-agnostic frame loop shared by every serve path.
///
/// Reads frames, hands them to [`TestAgent::handle`], writes replies,
/// and ends on `test.shutdown`, clean EOF (graceful cleanup), or a
/// write failure (driver gone). A single oversized frame yields a
/// `message_too_large` error event — it never kills the session.
pub fn serve(
    agent: &mut TestAgent,
    io: &mut dyn TestIo,
    mut r: impl Read,
    mut w: impl Write,
) -> ServeOutcome {
    if !agent.enabled {
        return ServeOutcome::Disabled {
            reason: "test agent is disabled".to_string(),
        };
    }
    loop {
        match read_frame(&mut r) {
            Ok(FrameRead::Frame(body)) => {
                let events = agent.handle(io, &body);
                let mut write_failed = false;
                for ev in &events {
                    if write_frame(&mut w, &ev.to_json()).is_err() {
                        write_failed = true;
                        break;
                    }
                }
                if write_failed {
                    let report = io.request_shutdown(ShutdownMode::Graceful);
                    return ServeOutcome::DriverGone { report };
                }
                if let Some(report) = agent.ended.take() {
                    return ServeOutcome::Shutdown { report };
                }
            }
            Ok(FrameRead::Eof) => {
                let report = io.request_shutdown(ShutdownMode::Graceful);
                return ServeOutcome::DriverGone { report };
            }
            Ok(FrameRead::TooLarge { declared }) => {
                let ev = Event::error(ErrorBody::new(
                    codes::MESSAGE_TOO_LARGE,
                    format!("control frame {declared} exceeds cap"),
                ));
                if write_frame(&mut w, &ev.to_json()).is_err() {
                    let report = io.request_shutdown(ShutdownMode::Graceful);
                    return ServeOutcome::DriverGone { report };
                }
            }
            Err(e) => {
                let _ = io.request_shutdown(ShutdownMode::Graceful);
                return ServeOutcome::Failed {
                    reason: format!("control channel read failed: {e}"),
                };
            }
        }
    }
}

/// Serves the agent over stdin/stdout — the default transport: an
/// OS-permission-inherited pipe to the spawning driver.
///
/// The app must keep stdout for this channel: log to stderr.
#[must_use = "the outcome reports how the run ended"]
pub fn serve_stdio(agent: &mut TestAgent, io: &mut dyn TestIo) -> ServeOutcome {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(agent, io, stdin.lock(), stdout.lock())
}

/// Serves the agent by dialling the driver's loopback listener.
///
/// Fail-closed: refuses when `expected_secret` is `None` or the peer
/// is not loopback. `expected_secret` is also bound into the agent so
/// `test.hello` must present it.
///
/// # Errors
/// [`UiError::Host`] on connect/peer-policy failure.
pub fn serve_tcp(
    agent: &mut TestAgent,
    io: &mut dyn TestIo,
    addr: &str,
    expected_secret: Option<String>,
) -> Result<ServeOutcome, UiError> {
    let Some(secret) = expected_secret else {
        return Err(UiError::host(
            "refusing TCP test control without a run secret",
        ));
    };
    agent.expected_secret = Some(secret);
    let stream = TcpStream::connect(addr)
        .map_err(|e| UiError::host(format!("test control dial {addr} failed: {e}")))?;
    let peer = stream
        .peer_addr()
        .map_err(|e| UiError::host(format!("peer addr failed: {e}")))?;
    if !crate::transport::is_loopback(peer.ip()) {
        return Err(UiError::host(format!(
            "refusing non-loopback test control peer {peer}"
        )));
    }
    let reader = stream
        .try_clone()
        .map_err(|e| UiError::host(format!("stream clone failed: {e}")))?;
    Ok(serve(agent, io, reader, stream))
}

/// Resolves the environment (`RVF_TEST_*`) and serves accordingly —
/// the single entry point an app-under-test calls in test mode.
///
/// - disabled env → `Disabled`;
/// - `RVF_TEST_CONTROL=stdio` → stdio serve;
/// - `RVF_TEST_CONTROL=tcp` + `RVF_TEST_ADDR` + `RVF_TEST_SECRET` → TCP serve;
/// - anything else → `Disabled` (no control channel).
#[must_use = "the outcome reports how the run ended"]
pub fn serve_from_env(io: &mut dyn TestIo, config: AgentConfig) -> ServeOutcome {
    if disabled_from_env() {
        return ServeOutcome::Disabled {
            reason: format!("{} is set", crate::transport::ENV_DISABLE),
        };
    }
    match transport_from_env() {
        AgentTransport::Off => ServeOutcome::Disabled {
            reason: format!("{} unset or 'off'", crate::transport::ENV_CONTROL),
        },
        AgentTransport::Stdio => {
            let mut agent = TestAgent::new(config, None);
            serve_stdio(&mut agent, io)
        }
        AgentTransport::Tcp { addr } => {
            let secret = expected_secret_from_env();
            let mut agent = TestAgent::new(config, secret.clone());
            match serve_tcp(&mut agent, io, &addr, secret) {
                Ok(outcome) => outcome,
                Err(e) => ServeOutcome::Failed {
                    reason: e.to_string(),
                },
            }
        }
    }
}
