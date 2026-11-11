//! Windowed `rvf.uitest.v1` lane (Issue #313 slice S2, RF-GT-04) —
//! spawns the real GPUI `windowed_test_app` AUT through
//! [`TestDriver::spawn_stdio`] and drives the closed op set end to
//! end over stdio: handshake → observation → fixture → clock →
//! fault → screenshot → close → graceful shutdown, recording a
//! per-op verdict JSON under
//! `evidence/issue-313-windowed-uitest/`.
//!
//! This is a **manual/interactive-desktop lane**: it opens real OS
//! windows, so it runs only when `RVF_WINDOWED_UITEST=1` is set
//! (the `test-ui-windowed-uitest` mise task sets it). Without the
//! gate the test body is a no-op — a skipped lane writes no verdict
//! and never pretends to have run.
//!
//! Verdict honesty: spawn/handshake/early-exit failures are
//! `environment_blocked` (no desktop session), an `unavailable`
//! screenshot on an OS without a capture backend is `unsupported`
//! — only observed protocol violations are `fail`.
//!
//! **RT contract:** non-RT only.

#![cfg(feature = "uitest")]

use std::process::Command;
use std::time::{Duration, Instant};

use base64::Engine as _;
use ruviform_ui_test::{
    ClockDomain, Direction, DriverError, FaultKind, ShutdownMode, TestDriver, WindowMeta,
};
use serde_json::{Value, json};

/// Runtime gate — the lane only runs on an interactive desktop.
const ENV_GATE: &str = "RVF_WINDOWED_UITEST";
/// Upper bound for a real window's `Ready` — a stalled loop is an
/// environment failure, not a hang.
const READY_WAIT: Duration = Duration::from_secs(15);
/// Poll cadence for readiness.
const POLL: Duration = Duration::from_millis(100);

fn aut(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_windowed_test_app"));
    cmd.args(args);
    cmd
}

fn spawn(args: &[&str]) -> Result<TestDriver, DriverError> {
    TestDriver::spawn_stdio(&mut aut(args))
}

/// One op's outcome in the verdict JSON.
#[derive(Clone)]
struct OpRec {
    verdict: &'static str,
    detail: Value,
}

/// The lane verdict — serialized to
/// `evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`
/// per the test-spec `rf313-windowed-uitest/v1` schema.
struct Verdict {
    ops: Vec<(String, OpRec)>,
}

impl Verdict {
    fn new() -> Self {
        Self { ops: Vec::new() }
    }

    fn record(&mut self, op: &str, verdict: &'static str, detail: Value) {
        self.ops.push((op.to_string(), OpRec { verdict, detail }));
    }

    /// Overall verdict: `fail` if any op failed, else `pass`
    /// (`unsupported`/`not_run`/`environment_blocked` are honest
    /// per-op verdicts, not failures).
    fn overall(&self) -> &'static str {
        if self.ops.iter().any(|(_, r)| r.verdict == "fail") {
            "fail"
        } else {
            "pass"
        }
    }

    /// Writes the verdict JSON to the evidence directory.
    fn write(&self, lane: &str) -> std::path::PathBuf {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("evidence/issue-313-windowed-uitest");
        std::fs::create_dir_all(&root).expect("evidence dir");
        let path = root.join("verdict-windowed-uitest.json");
        let ops: Value = self
            .ops
            .iter()
            .map(|(k, r)| {
                (
                    k.clone(),
                    json!({ "verdict": r.verdict, "detail": r.detail }),
                )
            })
            .collect::<serde_json::Map<String, Value>>()
            .into();
        let doc = json!({
            "schema": "rf313-windowed-uitest/v1",
            "issue": 313,
            "slice": "S2",
            "lane": lane,
            "driver": "gpui-windowed",
            "backend": "gpui",
            "verdict": self.overall(),
            "environment": {
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "capture_backend": cfg!(any(windows, target_os = "linux")),
                "desktop_session": true,
            },
            "ops": ops,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).expect("verdict write");
        path
    }
}

/// Whether a driver-level failure means the AUT could not run a GUI
/// session at all — an environment verdict, never a fake pass.
fn looks_environment_blocked(err: &DriverError) -> bool {
    match err {
        // Spawn/transport failure, EOF before/inside the loop, or the
        // AUT dying (exit 3 path) — no desktop session is the usual
        // cause; a timed-out reply is a stalled GUI loop.
        DriverError::Io(_) | DriverError::Disconnected | DriverError::Timeout => true,
        DriverError::Handshake(m) => {
            m.contains("control channel closed")
                || m.contains("app exited")
                || m.contains("bad welcome")
        }
        // A typed protocol error is a live agent answering — not an
        // environment failure; every other variant is treated alike.
        _ => false,
    }
}

/// Polls `observe_windows` until every window reports `ready` —
/// real `Ready` arrives on the first OS frame; nothing is
/// fabricated.
fn wait_all_ready(d: &mut TestDriver) -> Result<Vec<WindowMeta>, DriverError> {
    let deadline = Instant::now() + READY_WAIT;
    loop {
        let wins = d.observe_windows()?;
        if !wins.is_empty() && wins.iter().all(|w| w.ready) {
            return Ok(wins);
        }
        if Instant::now() > deadline {
            return Err(DriverError::Timeout);
        }
        std::thread::sleep(POLL);
    }
}

fn receipts_of(d: &mut TestDriver) -> Vec<(Direction, String, Option<u64>)> {
    d.observe_receipts(0, 4096)
        .map(|p| {
            p.entries
                .iter()
                .map(|e| (e.direction, e.kind.clone(), e.window))
                .collect()
        })
        .unwrap_or_default()
}

/// Drains buffered `Notify` payloads — async notices attach to the
/// reply of the op that produced them, so they arrive here rather
/// than via the `test.notices` op (which reports only the
/// still-undrained tail).
fn pending_notices(d: &mut TestDriver) -> Vec<Value> {
    d.notifications()
        .into_iter()
        .filter_map(|v| v.get("notice").cloned())
        .collect()
}

/// Extracts `ErrorBody` from a protocol-level failure.
fn proto_err(r: Result<Value, DriverError>) -> ruviform_ui::ErrorBody {
    match r {
        Err(DriverError::Protocol(e)) => e,
        other => panic!("expected protocol error, got {other:?}"),
    }
}

#[test]
fn windowed_uitest_lane() {
    if std::env::var_os(ENV_GATE).is_none() {
        eprintln!("windowed uitest lane skipped — set {ENV_GATE}=1 on an interactive desktop");
        return;
    }
    let mut v = Verdict::new();
    let lane = format!("{}-gpui-uitest", std::env::consts::OS);

    // ---- spawn + handshake ----
    let mut d = match spawn(&["--windows", "1"]) {
        Ok(d) => {
            let w = d.welcome();
            let ok = w.backend.name == "gpui-windowed"
                && w.capabilities.fixture
                && w.capabilities.clock_domains == ["host_tick"]
                && !w.capabilities.denied.input_injection
                && !w.capabilities.denied.dom_access
                && w.capabilities.screenshot == cfg!(any(windows, target_os = "linux"));
            v.record(
                "hello",
                if ok { "pass" } else { "fail" },
                json!({
                    "backend": w.backend.name,
                    "screenshot_claim": w.capabilities.screenshot,
                    "denied": { "input_injection": w.capabilities.denied.input_injection,
                                "dom_access": w.capabilities.denied.dom_access },
                }),
            );
            assert!(ok, "welcome contract violated: {w:?}");
            d
        }
        Err(e) => {
            let verdict = if looks_environment_blocked(&e) {
                "environment_blocked"
            } else {
                "fail"
            };
            v.record("hello", verdict, json!({ "error": format!("{e:?}") }));
            let path = v.write(&lane);
            eprintln!("verdict written: {}", path.display());
            panic!("windowed uitest lane: handshake {verdict}: {e:?}");
        }
    };

    // The lane body runs to its end even on op failure — every op
    // gets an honest verdict; a panic path still writes the file.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_ops(&mut d, &mut v);
    }));
    let path = v.write(&lane);
    eprintln!("verdict written: {}", path.display());
    match result {
        Ok(()) => {}
        Err(p) => std::panic::resume_unwind(p),
    }
    assert_eq!(
        v.overall(),
        "pass",
        "verdict: {:?}",
        v.ops
            .iter()
            .map(|(k, r)| (k, r.verdict))
            .collect::<Vec<_>>()
    );
}

fn run_ops(d: &mut TestDriver, v: &mut Verdict) {
    let Some((w1, w2)) = op_observe_and_fixture(d, v) else {
        return;
    };
    op_clock(d, v);
    op_ipc(d, v, w1);
    op_faults(d, v, w1, w2);
    op_close_and_capture(d, v, w1);
    op_shutdown(d, v);
}

/// Observation + fixture + receipts — returns the initial and the
/// fixture-created window ids, or records the failure and stops.
fn op_observe_and_fixture(d: &mut TestDriver, v: &mut Verdict) -> Option<(u64, u64)> {
    // ---- observation: real windows reach Ready ----
    let wins = match wait_all_ready(d) {
        Ok(w) => {
            v.record(
                "windows",
                "pass",
                json!({ "windows": w.iter().map(|m| m.id).collect::<Vec<_>>() }),
            );
            w
        }
        Err(e) => {
            v.record(
                "windows",
                if looks_environment_blocked(&e) {
                    "environment_blocked"
                } else {
                    "fail"
                },
                json!({ "error": format!("{e:?}") }),
            );
            return None;
        }
    };
    let w1 = wins[0].id;

    // ---- fixture: real window creation + close-policy knob ----
    match d.prepare_fixture(
        "windows",
        Some(7),
        json!({"count": 1, "close_policy": "veto"}),
    ) {
        Ok(r) => {
            let created = r.applied["windows"].as_array().map_or(0, Vec::len);
            v.record("fixture.prepare", "pass", json!({ "created": created }));
        }
        Err(e) => {
            v.record(
                "fixture.prepare",
                "fail",
                json!({ "error": format!("{e:?}") }),
            );
        }
    }
    let wins = wait_all_ready(d).unwrap_or_default();
    let Some(w2) = wins.iter().map(|w| w.id).find(|id| *id != w1) else {
        v.record(
            "fixture.prepare",
            "fail",
            json!({ "error": "fixture window never became ready" }),
        );
        return None;
    };

    // ---- receipts: creation/lifecycle traffic journaled ----
    let receipts = receipts_of(d);
    let has_created = receipts
        .iter()
        .any(|(dir, k, _)| *dir == Direction::In && k == "created");
    let has_ready = receipts
        .iter()
        .any(|(dir, k, _)| *dir == Direction::In && k == "ready");
    v.record(
        "receipts",
        if has_created && has_ready {
            "pass"
        } else {
            "fail"
        },
        json!({ "created": has_created, "ready": has_ready }),
    );
    Some((w1, w2))
}

/// Clock: declared domain advances; undeclared fails closed.
fn op_clock(d: &mut TestDriver, v: &mut Verdict) {
    match d.advance_clock(ClockDomain::HostTick, 2) {
        Ok(_) => {
            let receipts = receipts_of(d);
            let has_tick = receipts
                .iter()
                .any(|(dir, k, _)| *dir == Direction::In && k == "tick");
            let e = proto_err(d.call("test.clock.advance", json!({"domain": "gpu", "ticks": 1})));
            let ok = has_tick && e.code == "unavailable";
            v.record(
                "clock.advance",
                if ok { "pass" } else { "fail" },
                json!({ "tick": has_tick, "undeclared_code": e.code }),
            );
        }
        Err(e) => {
            v.record(
                "clock.advance",
                "fail",
                json!({ "error": format!("{e:?}") }),
            );
        }
    }
}

/// IPC: a valid raw body decodes at the inbound boundary and reaches
/// the app as `Action` through the normal delivery path.
fn op_ipc(d: &mut TestDriver, v: &mut Verdict, w1: u64) {
    let body = json!({"v":1,"kind":"intent","id":"i1","op":"demo.nop","params":{}}).to_string();
    match d.prepare_fixture("ipc_emit", None, json!({"window": w1, "body": body})) {
        Ok(_) => {
            let receipts = receipts_of(d);
            let ok = receipts
                .iter()
                .any(|(dir, k, win)| *dir == Direction::In && k == "message" && *win == Some(w1));
            v.record(
                "ipc_emit",
                if ok { "pass" } else { "fail" },
                json!({ "message_receipt": ok }),
            );
        }
        Err(e) => {
            v.record("ipc_emit", "fail", json!({ "error": format!("{e:?}") }));
        }
    }
}

/// Faults: `ipc_disconnect` suppresses pre-decode observably;
/// `window_crash` destroys the armed window with a notice.
fn op_faults(d: &mut TestDriver, v: &mut Verdict, w1: u64, w2: u64) {
    match d.set_fault(FaultKind::IpcDisconnect, w1) {
        Ok(_) => {
            let ipc_down = d
                .observe_windows()
                .unwrap_or_default()
                .iter()
                .any(|m| m.id == w1 && m.ipc_down);
            let _ = d.prepare_fixture("ipc_emit", None, json!({"window": w1, "body": "x"}));
            let receipts = receipts_of(d);
            let suppressed = receipts.iter().any(|(dir, k, win)| {
                *dir == Direction::In && k == "suppressed" && *win == Some(w1)
            });
            let noticed = pending_notices(d)
                .iter()
                .any(|n| n["kind"] == "ipc.suppressed");
            let _ = d.clear_fault(FaultKind::IpcDisconnect, w1);
            let ok = ipc_down && suppressed && noticed;
            v.record(
                "fault.set",
                if ok { "pass" } else { "fail" },
                json!({ "ipc_down": ipc_down, "suppressed": suppressed, "notice": noticed }),
            );
        }
        Err(e) => {
            v.record("fault.set", "fail", json!({ "error": format!("{e:?}") }));
        }
    }

    match d.set_fault(FaultKind::WindowCrash, w2) {
        Ok(_) => {
            let live = d.observe_windows().unwrap_or_default();
            let gone = !live.iter().any(|m| m.id == w2);
            let noticed = pending_notices(d)
                .iter()
                .any(|n| n["kind"] == "window.destroyed" && n["window"] == w2);
            v.record(
                "fault.window_crash",
                if gone && noticed { "pass" } else { "fail" },
                json!({ "destroyed": gone, "notice": noticed }),
            );
        }
        Err(e) => {
            v.record(
                "fault.window_crash",
                "fail",
                json!({ "error": format!("{e:?}") }),
            );
        }
    }
}

/// Close + capture: veto keeps `w1` alive, real `capture_window`
/// pixels where the backend exists, then the unanswered request
/// defaults to approve and destroys it.
fn op_close_and_capture(d: &mut TestDriver, v: &mut Verdict, w1: u64) {
    // A dedicated small capture target: a full-size RGBA frame's
    // base64 reply would exceed the transport's 1 MiB frame cap —
    // the pixels are real either way.
    let shot_window = d
        .prepare_fixture(
            "windows",
            None,
            json!({"count": 1, "width": 320, "height": 240}),
        )
        .ok()
        .and_then(|_| {
            wait_all_ready(d)
                .ok()
                .and_then(|ws| ws.iter().map(|m| m.id).find(|id| *id != w1))
        });

    let mut close_err = String::new();
    let mut vetoed = false;
    match d.request_close(w1) {
        Ok(_) => {
            // Policy is "veto" (set by the fixture above) — the
            // window survives and the pending flag clears.
            match d.observe_windows() {
                Ok(ws) => vetoed = ws.iter().any(|m| m.id == w1 && !m.close_pending),
                Err(e) => close_err = format!("observe after veto: {e:?}"),
            }
        }
        Err(e) => {
            close_err = format!("{e:?}");
        }
    }

    match shot_window {
        Some(sw) => match d.screenshot(sw) {
            Ok(shot) => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&shot.data_base64)
                    .unwrap_or_default();
                let expected = shot.width as usize * shot.height as usize * 4;
                let ok = shot.format == "rgba-raw"
                    && shot.width > 0
                    && shot.height > 0
                    && decoded.len() == expected;
                v.record(
                    "screenshot",
                    if ok { "pass" } else { "fail" },
                    json!({
                        "format": shot.format,
                        "width": shot.width,
                        "height": shot.height,
                        "decoded_bytes": decoded.len(),
                    }),
                );
            }
            Err(DriverError::Protocol(e)) if e.code == "unavailable" => {
                v.record(
                    "screenshot",
                    "unsupported",
                    json!({ "error": e.to_string() }),
                );
            }
            Err(e) => {
                v.record("screenshot", "fail", json!({ "error": format!("{e:?}") }));
            }
        },
        None => {
            v.record(
                "screenshot",
                "fail",
                json!({ "error": "small capture window never became ready" }),
            );
        }
    }

    let _ = d.prepare_fixture(
        "windows",
        None,
        json!({"count": 0, "close_policy": "unanswered"}),
    );
    let _ = d.request_close(w1);
    let mut gone = false;
    match d.observe_windows() {
        Ok(ws) => gone = !ws.iter().any(|m| m.id == w1),
        Err(e) => close_err = format!("observe after default-approve: {e:?}"),
    }
    let ok = close_err.is_empty() && vetoed && gone;
    v.record(
        "request_close",
        if ok { "pass" } else { "fail" },
        json!({
            "veto_kept": vetoed,
            "default_approve_destroyed": gone,
            "error": close_err,
        }),
    );
}

/// Graceful shutdown: the app hook runs, every live window is
/// destroyed, and the AUT exits with code 0.
fn op_shutdown(d: &mut TestDriver, v: &mut Verdict) {
    // Leave one live window so cleanup has something real to close.
    let _ = d.prepare_fixture("windows", None, json!({"count": 1}));
    let open_before = match d.observe_windows() {
        Ok(ws) => ws.len(),
        Err(e) => {
            v.record(
                "shutdown",
                "fail",
                json!({ "error": format!("pre-shutdown observe: {e:?}") }),
            );
            return;
        }
    };
    match d.shutdown(ShutdownMode::Graceful) {
        Ok(report) => {
            let ok = report.exited && report.windows_closed == open_before;
            v.record(
                "shutdown",
                if ok { "pass" } else { "fail" },
                json!({
                    "windows_open": report.windows_open,
                    "windows_closed": report.windows_closed,
                    "open_before": open_before,
                    "exited": report.exited,
                }),
            );
        }
        Err(e) => {
            v.record("shutdown", "fail", json!({ "error": format!("{e:?}") }));
        }
    }
}

#[test]
fn windowed_fail_init_reports_failed_handshake() {
    if std::env::var_os(ENV_GATE).is_none() {
        eprintln!("windowed uitest lane skipped — set {ENV_GATE}=1 on an interactive desktop");
        return;
    }
    match spawn(&["--fail-init"]) {
        Err(DriverError::Handshake(m)) => assert!(m.contains("not ready"), "{m}"),
        // No desktop → the AUT dies before hello — an environment
        // verdict, never a fake pass.
        Err(e) if looks_environment_blocked(&e) => {
            eprintln!("fail-init: environment_blocked: {e:?}");
        }
        other => panic!("expected failed handshake, got {other:?}"),
    }
}
