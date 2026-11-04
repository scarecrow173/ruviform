//! In-process contract tests for `rvf.uitest.v1` (Issue #312).
//!
//! Drives `HeadlessIo` + `TestAgent` directly: handshake, capability
//! gating, fixture, clock scope, fault allowlist, observation,
//! close-cancel, two-window ownership, restart-shaped cleanup, and
//! the `serve` frame loop over `std::io::pipe`.
//!
//! **RT contract:** non-RT only.

use std::io::{Write, pipe};

use ruviform_ui::{
    CloseDecision, HostEvent, WindowConfig, WindowEvent, WindowHandle, WindowId, codes,
};
use ruviform_ui_test::{
    AgentConfig, AppMeta, FixtureController, FixtureReceipt, FixtureSpec, HeadlessIo, Journal,
    ReadyState, ServeOutcome, TestAgent, TestApp, TestIo, serve,
};
use serde_json::{Value, json};

// ---------- test app + fixture ----------

/// Sink app: records events, answers `CloseRequested` per its
/// configured decision (None = leave unanswered → host default).
struct Sink {
    decision: Option<CloseDecision>,
    seen: Vec<String>,
    jobs_cancelled: Vec<u64>,
}

impl Sink {
    fn new(decision: Option<CloseDecision>) -> Self {
        Self {
            decision,
            seen: Vec::new(),
            jobs_cancelled: Vec::new(),
        }
    }
}

impl TestApp for Sink {
    fn on_event(&mut self, wev: WindowEvent, wh: &mut dyn WindowHandle) {
        if let WindowEvent::Windowed { window, event } = &wev {
            self.seen.push(format!("w{}:{event:?}", window.get()));
            if matches!(event, HostEvent::CloseRequested)
                && let Some(d) = self.decision
            {
                let _ = wh.respond_close(*window, d);
            }
            if matches!(event, HostEvent::Destroyed) {
                self.jobs_cancelled.push(window.get());
            }
        }
    }
}

/// `dual` fixture — creates two ready windows.
struct DualFixture;

impl FixtureController for DualFixture {
    fn name(&self) -> &'static str {
        "dual"
    }

    fn prepare(
        &mut self,
        spec: &FixtureSpec,
        io: &mut dyn TestIo,
    ) -> Result<FixtureReceipt, ruviform_ui::ErrorBody> {
        let mut ids = Vec::new();
        for _ in 0..2 {
            let id = io.create_window(&WindowConfig::default())?;
            io.emit(WindowEvent::Windowed {
                window: id,
                event: HostEvent::Ready {
                    url: "rvf://app/index.html".to_string(),
                },
            });
            ids.push(id.get());
        }
        Ok(FixtureReceipt {
            name: spec.name.clone(),
            applied: json!({ "windows": ids, "seed": spec.seed }),
        })
    }
}

fn config() -> AgentConfig {
    AgentConfig::new(AppMeta {
        name: "contract-test".to_string(),
        version: "0.0.0-test".to_string(),
        revision: None,
    })
}

fn intent(id: &str, op: &str, params: Value) -> String {
    let mut v = json!({
        "v": 1u32,
        "kind": "intent",
        "id": id,
        "op": op,
    });
    v["params"] = params;
    v.to_string()
}

fn hello(id: &str, run_id: &str, secret: Option<&str>) -> String {
    let mut p = json!({ "run_id": run_id });
    if let Some(s) = secret {
        p["secret"] = json!(s);
    }
    intent(id, "test.hello", p)
}

/// Parses agent reply events; returns `(kind, id, ok, data_or_error)`.
fn parse(events: &[ruviform_ui::Event]) -> Vec<(String, Option<String>, Value)> {
    events
        .iter()
        .map(|e| {
            let v: Value = serde_json::from_str(&e.to_json()).unwrap();
            (
                v["kind"].as_str().unwrap_or("?").to_string(),
                v["id"].as_str().map(str::to_string),
                v,
            )
        })
        .collect()
}

fn result(events: &[ruviform_ui::Event], id: &str) -> Value {
    let parsed = parse(events);
    let found = parsed
        .iter()
        .find(|(k, i, _)| k == "result" && i.as_deref() == Some(id));
    let Some((_, _, v)) = found else {
        panic!("no result for {id}: {parsed:?}");
    };
    v.clone()
}

fn result_ok(events: &[ruviform_ui::Event], id: &str) -> Value {
    let v = result(events, id);
    assert_eq!(v["ok"], true, "expected ok for {id}: {v}");
    v["data"].clone()
}

fn result_err(events: &[ruviform_ui::Event], id: &str) -> Value {
    let v = result(events, id);
    assert_eq!(v["ok"], false, "expected err for {id}: {v}");
    v["error"].clone()
}

fn agent(secret: Option<&str>) -> TestAgent {
    TestAgent::new(config(), secret.map(str::to_string))
}

/// The `Notice` payloads carried as `Notify` events after a command.
fn notices_of(events: &[ruviform_ui::Event]) -> Vec<Value> {
    parse(events)
        .into_iter()
        .filter(|(k, _, _)| k == "notify")
        .map(|(_, _, v)| v["data"]["notice"].clone())
        .collect()
}

fn do_hello(a: &mut TestAgent, io: &mut dyn TestIo, run: &str) -> Value {
    let evs = a.handle(io, &hello("h1", run, None));
    result_ok(&evs, "h1")
}

// ---------- handshake ----------

#[test]
fn handshake_welcome_contract() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let w = do_hello(&mut a, &mut io, "run-42");
    assert_eq!(w["v"], 1);
    assert!(w["session_id"].as_str().unwrap().starts_with("s-"));
    assert_eq!(w["run_id"], "run-42");
    assert_eq!(w["app"]["name"], "contract-test");
    assert_eq!(w["app"]["version"], "0.0.0-test");
    assert_eq!(w["ruviform_revision"], ruviform_ui::version());
    assert_eq!(w["backend"]["name"], "headless-fake");
    let caps = &w["capabilities"];
    assert_eq!(caps["clock_domains"], json!(["host_tick"]));
    assert_eq!(caps["faults"], json!(["window_crash", "ipc_disconnect"]));
    assert_eq!(caps["fixture"], false);
    assert_eq!(caps["screenshot"], false);
    // RF-GT-04 boundary flags are honest — never claimed here.
    assert_eq!(caps["input_injection"], false);
    assert_eq!(caps["dom_access"], false);
    assert_eq!(w["windows"], json!([]));
    assert_eq!(w["os"], std::env::consts::OS);
}

#[test]
fn hello_must_be_first_op() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let evs = a.handle(&mut io, &intent("x1", "test.capabilities", json!({})));
    let e = result_err(&evs, "x1");
    assert_eq!(e["code"], "unauthenticated");
}

#[test]
fn second_hello_rejected() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r1");
    let evs = a.handle(&mut io, &hello("h2", "r2", None));
    assert_eq!(result_err(&evs, "h2")["code"], "unauthenticated");
}

#[test]
fn hello_secret_enforced_on_tcp() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(Some("s3cret"));
    // Missing secret.
    let evs = a.handle(&mut io, &hello("h1", "r1", None));
    assert_eq!(result_err(&evs, "h1")["code"], "unauthenticated");
    // Wrong secret.
    let evs = a.handle(&mut io, &hello("h2", "r1", Some("wrong")));
    assert_eq!(result_err(&evs, "h2")["code"], "unauthenticated");
    // Right secret.
    let evs = a.handle(&mut io, &hello("h3", "r1", Some("s3cret")));
    assert_eq!(result_ok(&evs, "h3")["run_id"], "r1");
}

#[test]
fn hello_reports_failed_ready_state() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.set_ready_state(ReadyState::Failed {
        reason: "asset init failed".to_string(),
    });
    let mut a = agent(None);
    let evs = a.handle(&mut io, &hello("h1", "r1", None));
    let e = result_err(&evs, "h1");
    assert_eq!(e["code"], "unavailable");
    assert!(e["message"].as_str().unwrap().contains("asset init"));
}

#[test]
fn disabled_agent_refuses_everything() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    a.set_enabled(false);
    let evs = a.handle(&mut io, &hello("h1", "r1", None));
    // Disabled is an uncorrelated `error` event — fail-closed.
    let parsed = parse(&evs);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].0, "error");
    assert_eq!(parsed[0].2["error"]["code"], "disabled");
    // And serve refuses outright.
    let (r, _w_in) = pipe().unwrap();
    let (_r_out, w) = pipe().unwrap();
    assert!(matches!(
        serve(&mut a, &mut io, r, w),
        ServeOutcome::Disabled { .. }
    ));
}

// ---------- capabilities / unknown ops ----------

#[test]
fn unknown_op_is_typed_error() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(&mut io, &intent("x", "test.eval", json!({"script": "1"})));
    assert_eq!(result_err(&evs, "x")["code"], "unknown_op");
    // Arbitrary eval/shell is unrepresentable: `exec`/`shell`/`file`
    // ops all hit the same closed-set rejection.
    let evs = a.handle(&mut io, &intent("y", "test.shell", json!({})));
    assert_eq!(result_err(&evs, "y")["code"], "unknown_op");
}

#[test]
fn malformed_intent_is_typed_error() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let evs = a.handle(&mut io, "not json");
    let parsed = parse(&evs);
    assert_eq!(parsed[0].0, "error");
    assert_eq!(parsed[0].2["error"]["code"], codes::MALFORMED_MESSAGE);
}

#[test]
fn fixture_op_gated_by_capability() {
    // No fixtures registered → capability absent → unavailable,
    // before any fixture-name lookup runs.
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    assert_eq!(result_err(&evs, "f")["code"], "unavailable");
}

#[test]
fn screenshot_op_gated_by_capability() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("s", "test.observe.screenshot", json!({"window": 1})),
    );
    assert_eq!(result_err(&evs, "s")["code"], "unavailable");
}

// ---------- fixture / windows ----------

#[test]
fn fixture_creates_two_windows_with_receipt() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let w = do_hello(&mut a, &mut io, "r");
    assert_eq!(w["capabilities"]["fixture"], true);
    let evs = a.handle(
        &mut io,
        &intent(
            "f",
            "test.fixture.prepare",
            json!({"name": "dual", "seed": 7, "params": {}}),
        ),
    );
    let data = result_ok(&evs, "f");
    assert_eq!(data["name"], "dual");
    let ws = data["applied"]["windows"].as_array().unwrap();
    assert_eq!(ws.len(), 2);
    assert_ne!(ws[0], ws[1]); // distinct nonzero ids
    assert_eq!(data["applied"]["seed"], 7);
    // window.created notices drain as Notify events after the result.
    let parsed = parse(&evs);
    let notify: Vec<&(String, Option<String>, Value)> =
        parsed.iter().filter(|(k, _, _)| k == "notify").collect();
    assert_eq!(notify.len(), 2);
    assert_eq!(notify[0].2["data"]["notice"]["kind"], "window.created");
    // Windows are now observable.
    let evs = a.handle(&mut io, &intent("w", "test.observe.windows", json!({})));
    let data = result_ok(&evs, "w");
    let wins = data["windows"].as_array().unwrap();
    assert_eq!(wins.len(), 2);
    assert_eq!(wins[0]["ready"], true);
    assert_eq!(wins[0]["close_pending"], false);
}

#[test]
fn unknown_fixture_is_typed_error() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "nope"})),
    );
    assert_eq!(result_err(&evs, "f")["code"], "invalid_params");
}

// ---------- clock ----------

#[test]
fn clock_advance_host_tick_scope() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent(
            "c",
            "test.clock.advance",
            json!({"domain": "host_tick", "ticks": 3}),
        ),
    );
    assert_eq!(result_ok(&evs, "c")["advanced"], 3);
    // Three Tick deliveries are journaled as `in` receipts.
    let page = io.receipts(0, 64);
    let ticks = page.entries.iter().filter(|e| e.kind == "tick").count();
    assert_eq!(ticks, 3);
}

#[test]
fn clock_undeclared_domain_rejected() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    for domain in ["gpu", "wall_clock", "os_timer"] {
        let evs = a.handle(
            &mut io,
            &intent(
                "c",
                "test.clock.advance",
                json!({"domain": domain, "ticks": 1}),
            ),
        );
        assert_eq!(result_err(&evs, "c")["code"], "unavailable", "{domain}");
    }
}

// ---------- faults ----------

#[test]
fn fault_allowlist_rejects_unknown_kind() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent(
            "f",
            "test.fault.set",
            json!({"kind": "gpu_hang", "window": 1}),
        ),
    );
    assert_eq!(result_err(&evs, "f")["code"], "unavailable");
}

#[test]
fn fault_requires_known_window() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent(
            "f",
            "test.fault.set",
            json!({"kind": "ipc_disconnect", "window": 99}),
        ),
    );
    assert_eq!(result_err(&evs, "f")["code"], "invalid_params");
}

#[test]
fn ipc_disconnect_suppresses_messages_observably() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    // Arm suppression.
    let evs = a.handle(
        &mut io,
        &intent(
            "s",
            "test.fault.set",
            json!({"kind": "ipc_disconnect", "window": w1}),
        ),
    );
    assert_eq!(result_ok(&evs, "s")["applied"], true);
    // Inbound message → suppressed receipt + notice, not delivered.
    let before = io.app().seen.len();
    io.emit(WindowEvent::Windowed {
        window: WindowId::new(w1).unwrap(),
        event: HostEvent::Message("x".to_string()),
    });
    let page = io.receipts(0, 64);
    assert!(
        page.entries
            .iter()
            .any(|e| e.kind == "suppressed" && e.window == Some(w1))
    );
    assert!(io.notices().iter().any(|n| n.kind == "ipc.suppressed"));
    assert_eq!(
        io.app().seen.len(),
        before,
        "suppressed message must not reach the app"
    );
    // Clear → next message delivers normally.
    let evs = a.handle(
        &mut io,
        &intent(
            "c",
            "test.fault.clear",
            json!({"kind": "ipc_disconnect", "window": w1}),
        ),
    );
    assert_eq!(result_ok(&evs, "c")["applied"], false);
    io.emit(WindowEvent::Windowed {
        window: WindowId::new(w1).unwrap(),
        event: HostEvent::Message("y".to_string()),
    });
    let page = io.receipts(0, 64);
    assert!(
        page.entries
            .iter()
            .any(|e| e.kind == "message" && e.window == Some(w1))
    );
    assert!(io.app().seen.iter().any(|s| s.contains("Message")));
}

#[test]
fn window_crash_fault_emits_destroyed() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let ws = result_ok(&evs, "f")["applied"]["windows"].clone();
    let w1 = ws[0].as_u64().unwrap();
    let evs = a.handle(
        &mut io,
        &intent(
            "c",
            "test.fault.set",
            json!({"kind": "window_crash", "window": w1}),
        ),
    );
    assert_eq!(result_ok(&evs, "c")["applied"], true);
    // Destroyed receipt + window.destroyed notice; w1 gone from
    // observe.windows, w2 unaffected (ownership stays per-window).
    let notices = notices_of(&evs);
    assert!(
        notices
            .iter()
            .any(|n| n["kind"] == "window.destroyed" && n["window"] == w1)
    );
    let live: Vec<u64> = io.windows().iter().map(|w| w.id).collect();
    assert!(!live.contains(&w1));
    assert_eq!(live.len(), 1);
    // window_crash cannot be cleared.
    let evs = a.handle(
        &mut io,
        &intent(
            "x",
            "test.fault.clear",
            json!({"kind": "window_crash", "window": w1}),
        ),
    );
    assert_eq!(result_err(&evs, "x")["code"], "invalid_params");
}

// ---------- close flow ----------

#[test]
fn close_veto_keeps_window_and_records_flow() {
    let mut io = HeadlessIo::new(Sink::new(Some(CloseDecision::Veto)));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    let evs = a.handle(
        &mut io,
        &intent("rc", "test.window.request_close", json!({"window": w1})),
    );
    assert_eq!(result_ok(&evs, "rc")["requested"], true);
    // Vetoed: still alive, close_pending cleared, close_requested
    // receipt + notice exist.
    let meta = io
        .windows()
        .into_iter()
        .find(|m| m.id == w1)
        .expect("vetoed window stays");
    assert!(!meta.close_pending);
    assert!(
        notices_of(&evs)
            .iter()
            .any(|n| n["kind"] == "window.close_requested")
    );
    let page = io.receipts(0, 64);
    assert!(page.entries.iter().any(|e| e.kind == "close_requested"));
    assert!(page.entries.iter().any(|e| e.kind == "respond_close"));
    assert!(!page.entries.iter().any(|e| e.kind == "destroyed"));
}

#[test]
fn close_approve_destroys_and_notifies() {
    let mut io = HeadlessIo::new(Sink::new(Some(CloseDecision::Approve)));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    let evs = a.handle(
        &mut io,
        &intent("rc", "test.window.request_close", json!({"window": w1})),
    );
    assert!(!io.windows().iter().any(|m| m.id == w1));
    assert!(
        notices_of(&evs)
            .iter()
            .any(|n| n["kind"] == "window.destroyed")
    );
}

#[test]
fn unanswered_close_defaults_to_approve() {
    // Sink with `decision: None` leaves the request unanswered —
    // the host's per-window default Approve applies.
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    let _ = a.handle(
        &mut io,
        &intent("rc", "test.window.request_close", json!({"window": w1})),
    );
    assert!(!io.windows().iter().any(|m| m.id == w1));
}

#[test]
fn close_defer_stays_pending_then_destroy() {
    let mut io = HeadlessIo::new(Sink::new(Some(CloseDecision::Defer)));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    let _ = a.handle(
        &mut io,
        &intent("rc", "test.window.request_close", json!({"window": w1})),
    );
    let meta = io
        .windows()
        .into_iter()
        .find(|m| m.id == w1)
        .expect("deferred window stays");
    assert!(meta.close_pending);
    // The app later destroys it explicitly — ownership intact.
    io.host_mut()
        .destroy_window(WindowId::new(w1).unwrap())
        .unwrap();
    io.emit(WindowEvent::Tick); // pump delivery
    assert!(!io.windows().iter().any(|m| m.id == w1));
}

// ---------- observation ----------

#[test]
fn receipts_paginate_and_report_truncation() {
    let mut j = Journal::new(4);
    for i in 0..10usize {
        j.record(ruviform_ui_test::Direction::In, None, format!("e{i}"), 0);
    }
    let page = j.entries_since(0, 3);
    assert_eq!(page.entries.len(), 3);
    assert_eq!(page.entries[0].seq, 7); // oldest retained (seqs start at 1)
    assert_eq!(page.next_since, 9);
    assert!(page.truncated);
    let page = j.entries_since(page.next_since, 64);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].seq, 10);
    assert_eq!(page.next_since, 10);
    // A fresh cursor is not truncated.
    let page = j.entries_since(10, 64);
    assert_eq!(page.entries.len(), 0);
    assert!(!page.truncated);
}

#[test]
fn observation_is_read_only_window_snapshot() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let _ = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let before = io.receipts(0, 64).entries.len();
    // Repeated reads change nothing.
    for _ in 0..3 {
        let _ = a.handle(&mut io, &intent("o", "test.observe.windows", json!({})));
        let _ = a.handle(&mut io, &intent("r", "test.observe.receipts", json!({})));
    }
    assert_eq!(io.windows().len(), 2);
    assert!(io.receipts(0, 4096).entries.len() >= before);
}

#[test]
fn notices_op_drains_queue() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    // Notices created by the fixture attach as Notify events on the
    // fixture result — `test.notices` also reports (and drains) any
    // remainder. Emit a destroy directly to create a fresh notice.
    let evs = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let w1 = result_ok(&evs, "f")["applied"]["windows"][0]
        .as_u64()
        .unwrap();
    io.host_mut()
        .destroy_window(WindowId::new(w1).unwrap())
        .unwrap();
    io.emit(WindowEvent::Tick);
    let evs = a.handle(&mut io, &intent("n", "test.notices", json!({})));
    let data = result_ok(&evs, "n");
    let notices = data["notices"].as_array().unwrap();
    assert!(notices.iter().any(|n| n["kind"] == "window.destroyed"));
    // Drained — second read is empty.
    let evs = a.handle(&mut io, &intent("n2", "test.notices", json!({})));
    assert_eq!(result_ok(&evs, "n2")["notices"], json!([]));
}

// ---------- shutdown / serve loop ----------

#[test]
fn shutdown_reports_cleanup_and_ends_serve() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let _ = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let evs = a.handle(
        &mut io,
        &intent("bye", "test.shutdown", json!({"mode": "graceful"})),
    );
    let data = result_ok(&evs, "bye");
    assert_eq!(data["bye"], true);
    assert_eq!(data["cleanup"]["windows_open"], 2);
    assert_eq!(data["cleanup"]["windows_closed"], 2);
    assert_eq!(data["cleanup"]["exited"], true);
    assert!(io.exited());
}

#[test]
fn shutdown_now_skips_teardown() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let _ = do_hello(&mut a, &mut io, "r");
    let _ = a.handle(
        &mut io,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    );
    let evs = a.handle(
        &mut io,
        &intent("bye", "test.shutdown", json!({"mode": "now"})),
    );
    let data = result_ok(&evs, "bye");
    assert_eq!(data["cleanup"]["windows_open"], 2);
    assert_eq!(data["cleanup"]["windows_closed"], 0);
}

#[test]
fn serve_loop_full_session_over_pipes() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let (cmd_r, mut cmd_w) = pipe().unwrap();
    let (mut ev_r, ev_w) = pipe().unwrap();
    let t = std::thread::spawn(move || serve(&mut a, &mut io, cmd_r, ev_w));
    // hello → capabilities → shutdown.
    ruviform_ui_test::transport::write_frame(&mut cmd_w, &hello("h", "run-pipe", None)).unwrap();
    let frame = read_one(&mut ev_r);
    assert_eq!(frame["ok"], true);
    assert_eq!(frame["data"]["run_id"], "run-pipe");
    ruviform_ui_test::transport::write_frame(&mut cmd_w, &intent("s", "test.shutdown", json!({})))
        .unwrap();
    let frame = read_one(&mut ev_r);
    assert_eq!(frame["data"]["bye"], true);
    let outcome = t.join().unwrap();
    assert!(matches!(outcome, ServeOutcome::Shutdown { .. }));
}

fn read_one(r: &mut impl std::io::Read) -> Value {
    match ruviform_ui_test::transport::read_frame(r).unwrap() {
        ruviform_ui_test::transport::FrameRead::Frame(body) => serde_json::from_str(&body).unwrap(),
        other => panic!("expected frame, got {other:?}"),
    }
}

#[test]
fn serve_driver_eof_runs_graceful_cleanup() {
    let mut io = HeadlessIo::new(Sink::new(None));
    io.add_fixture(Box::new(DualFixture));
    let mut a = agent(None);
    let (cmd_r, mut cmd_w) = pipe().unwrap();
    let (mut ev_r, ev_w) = pipe().unwrap();
    let t = std::thread::spawn(move || serve(&mut a, &mut io, cmd_r, ev_w));
    ruviform_ui_test::transport::write_frame(&mut cmd_w, &hello("h", "r", None)).unwrap();
    let _ = read_one(&mut ev_r);
    ruviform_ui_test::transport::write_frame(
        &mut cmd_w,
        &intent("f", "test.fixture.prepare", json!({"name": "dual"})),
    )
    .unwrap();
    let _ = read_one(&mut ev_r);
    // Driver drops its end without shutdown → graceful cleanup runs.
    drop(cmd_w);
    let outcome = t.join().unwrap();
    match outcome {
        ServeOutcome::DriverGone { report } => {
            assert_eq!(report.windows_open, 2);
            assert_eq!(report.windows_closed, 2);
            assert!(report.exited);
        }
        other => panic!("expected DriverGone, got {other:?}"),
    }
}

#[test]
fn serve_oversized_frame_errors_but_session_survives() {
    let mut io = HeadlessIo::new(Sink::new(None));
    let mut a = agent(None);
    let (cmd_r, mut cmd_w) = pipe().unwrap();
    let (mut ev_r, ev_w) = pipe().unwrap();
    let t = std::thread::spawn(move || serve(&mut a, &mut io, cmd_r, ev_w));
    // A frame body larger than MAX_INTENT_BYTES.
    let declared = u32::try_from(ruviform_ui::ipc::MAX_INTENT_BYTES + 1).unwrap();
    cmd_w.write_all(&declared.to_le_bytes()).unwrap();
    cmd_w.write_all(&vec![b'x'; declared as usize]).unwrap();
    cmd_w.flush().unwrap();
    let frame = read_one(&mut ev_r);
    assert_eq!(frame["kind"], "error");
    assert_eq!(frame["error"]["code"], "message_too_large");
    // Stream stayed aligned — a normal hello still works.
    ruviform_ui_test::transport::write_frame(&mut cmd_w, &hello("h", "r", None)).unwrap();
    let frame = read_one(&mut ev_r);
    assert_eq!(frame["ok"], true);
    drop(cmd_w);
    let outcome = t.join().unwrap();
    assert!(matches!(outcome, ServeOutcome::DriverGone { .. }));
}

// ---------- fake host semantics ----------

#[test]
fn fake_host_eval_backlog_and_dead_drops() {
    use ruviform_ui_test::FakeHost;
    let mut h = FakeHost::new();
    let w = h.create_window(&WindowConfig::default()).unwrap();
    // Dead window eval — silent drop.
    h.eval(WindowId::new(99).unwrap(), "dead()");
    // Not-ready backlog holds 16, drop-oldest.
    for i in 0..20 {
        h.eval(w, &format!("s{i}()"));
    }
    h.inject(WindowEvent::Windowed {
        window: w,
        event: HostEvent::Ready {
            url: "u".to_string(),
        },
    });
    let mut seen = Vec::new();
    let _ = h.pump(&mut |wev, _| seen.push(format!("{wev:?}")));
    assert_eq!(seen.len(), 1);
    // Backlog flushed on Ready — the 4 oldest of 20 were dropped.
    assert_eq!(h.scripts_for(w).len(), 16);
    assert_eq!(h.scripts_for(w)[0], "s4()");
    // Ready now delivers directly.
    h.eval(w, "live()");
    assert_eq!(h.scripts_for(w).last().copied(), Some("live()"));
}

#[test]
fn destroyed_is_terminal_and_events_to_dead_windows_drop() {
    use ruviform_ui_test::FakeHost;
    let mut h = FakeHost::new();
    let w = h.create_window(&WindowConfig::default()).unwrap();
    h.destroy_window(w).unwrap();
    // A second destroy errors; an event queued for the dead id drops.
    assert!(h.destroy_window(w).is_err());
    h.inject(WindowEvent::Windowed {
        window: w,
        event: HostEvent::Focused(true),
    });
    let mut seen = Vec::new();
    let _ = h.pump(&mut |wev, _| {
        if let WindowEvent::Windowed { window, event } = &wev {
            seen.push((window.get(), format!("{event:?}")));
        }
    });
    // Only the terminal Destroyed reaches the consumer.
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, w.get());
    assert!(seen[0].1.contains("Destroyed"));
}
