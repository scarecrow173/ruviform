//! Cross-process E2E for `rvf.uitest.v1` (Issue #312 acceptance).
//!
//! Spawns the `dummy_app` binary as the app-under-test through
//! [`TestDriver`] and exercises the acceptance flow end to end over
//! real pipes/sockets: start → handshake → capabilities → fixture →
//! observation → graceful/abnormal end → complete cleanup — plus
//! fail-closed checks (no control env, disabled env, unauthenticated,
//! external connect), close-cancel, ipc disconnect, restart, driver
//! failure, and orphan cleanup.
//!
//! These tests build `dummy_app` via `CARGO_BIN_EXE_dummy_app`.
//!
//! **RT contract:** non-RT only.

use std::io::BufReader;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ruviform_ui_test::transport::{
    ENV_ADDR, ENV_CONTROL, ENV_DISABLE, ENV_SECRET, FrameRead, read_frame, write_frame,
};
use ruviform_ui_test::{ClockDomain, DriverError, FaultKind, ShutdownMode, TestDriver, ops};
use serde_json::{Value, json};

fn dummy_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dummy_app"))
}

fn spawn_stdio(args: &[&str]) -> Result<TestDriver, DriverError> {
    let mut cmd = dummy_cmd();
    cmd.args(args);
    TestDriver::spawn_stdio(&mut cmd)
}

/// Extracts the `ErrorBody` from a protocol-level failure.
fn proto_err(r: Result<Value, DriverError>) -> ruviform_ui::ErrorBody {
    match r {
        Err(DriverError::Protocol(e)) => e,
        other => panic!("expected protocol error, got {other:?}"),
    }
}

/// Drains the driver's buffered `Notify` events — notices arrive
/// eagerly after every reply, this is the read-only notice channel.
fn pending_notices(d: &mut TestDriver) -> Vec<Value> {
    d.notifications()
        .into_iter()
        .map(|v| v["notice"].clone())
        .collect()
}

// ---------- acceptance: full lifecycle ----------

#[test]
fn e2e_stdio_lifecycle_acceptance() {
    // start → handshake
    let mut d = spawn_stdio(&[]).expect("handshake");
    let w = d.welcome().clone();
    assert_eq!(w.v, 1);
    assert_ne!(w.session_id, "");
    assert_eq!(w.run_id, d.run_id());
    assert_eq!(w.app.name, "dummy_app");
    assert_eq!(w.ruviform_revision, ruviform_ui::version());
    assert_eq!(w.backend.name, "headless-fake");
    assert!(w.capabilities.fixture);
    assert_eq!(w.capabilities.clock_domains, ["host_tick"]);
    assert!(!w.capabilities.screenshot);
    assert!(!w.capabilities.denied.input_injection);
    assert!(!w.capabilities.denied.dom_access);
    assert_eq!(w.windows.len(), 1); // default initial window

    // capabilities query
    let caps = d.capabilities().expect("capabilities");
    assert_eq!(caps["session_id"], json!(w.session_id));

    // fixture
    let receipt = d
        .prepare_fixture("windows", Some(7), json!({"count": 2}))
        .expect("fixture");
    let created = receipt.applied["windows"].as_array().unwrap();
    assert_eq!(created.len(), 2);

    // observation: windows
    let wins = d.observe_windows().expect("observe windows");
    assert_eq!(wins.len(), 3);
    assert!(wins.iter().all(|m| m.ready));

    // observation: receipts carry the fixture traffic
    let page = d.observe_receipts(0, 256).expect("receipts");
    assert!(page.entries.iter().any(|e| e.kind == "ready"));
    assert!(!page.truncated);

    // clock: declared domain advances; undeclared fails closed
    d.advance_clock(ClockDomain::HostTick, 4).expect("advance");
    let e = proto_err(d.call(ops::CLOCK_ADVANCE, json!({"domain": "gpu", "ticks": 1})));
    assert_eq!(e.code, "unavailable");

    // graceful end → complete cleanup → process exits
    let report = d.shutdown(ShutdownMode::Graceful).expect("shutdown");
    assert_eq!(report.windows_open, 3);
    assert_eq!(report.windows_closed, 3);
    assert!(report.exited);
    assert_eq!(report.app["jobs_running"], false);
    let status = d.try_exit_status().expect("exit status");
    assert!(status.is_some_and(|s| s.success()), "child must exit 0");
}

#[test]
fn e2e_shutdown_now_skips_teardown() {
    let mut d = spawn_stdio(&[]).expect("handshake");
    let report = d.shutdown(ShutdownMode::Now).expect("shutdown now");
    assert_eq!(report.windows_closed, 0);
    assert!(report.exited);
    let status = d.try_exit_status().expect("exit status");
    assert!(status.is_some_and(|s| s.success()));
}

#[test]
fn e2e_two_windows_and_close_cancel() {
    let mut d = spawn_stdio(&["--windows", "0"]).expect("handshake");
    let receipt = d
        .prepare_fixture("windows", None, json!({"count": 2, "close_policy": "veto"}))
        .expect("fixture");
    let ws: Vec<u64> = receipt.applied["windows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let (w1, w2) = (ws[0], ws[1]);

    // Veto cancels the close: w1 survives, w2 untouched — ownership
    // stays per-window.
    d.request_close(w1).expect("request close");
    let wins = d.observe_windows().expect("observe");
    assert_eq!(wins.len(), 2);
    let m1 = wins.iter().find(|m| m.id == w1).unwrap();
    assert!(!m1.close_pending);
    let notices = pending_notices(&mut d);
    assert!(
        notices
            .iter()
            .any(|n| n["kind"] == "window.close_requested")
    );

    // Flip policy → approve: w1 destroyed, w2 still alive.
    d.prepare_fixture(
        "windows",
        None,
        json!({"count": 0, "close_policy": "approve"}),
    )
    .expect("policy");
    d.request_close(w1).expect("request close");
    let wins = d.observe_windows().expect("observe");
    assert_eq!(wins.len(), 1);
    assert_eq!(wins[0].id, w2);
    let notices = pending_notices(&mut d);
    assert!(
        notices
            .iter()
            .any(|n| n["kind"] == "window.destroyed" && n["window"] == w1)
    );

    let report = d.shutdown(ShutdownMode::Graceful).expect("shutdown");
    assert_eq!(report.windows_closed, 1);
}

#[test]
fn e2e_ipc_disconnect_fault_suppression() {
    let mut d = spawn_stdio(&[]).expect("handshake");
    let w1 = d.observe_windows().unwrap()[0].id;

    d.set_fault(FaultKind::IpcDisconnect, w1).expect("arm");
    let wins = d.observe_windows().unwrap();
    assert!(wins[0].ipc_down);
    // Inbound message while armed → suppressed receipt.
    d.prepare_fixture("ipc_emit", None, json!({"window": w1, "body": "not-json"}))
        .expect("emit");
    let page = d.observe_receipts(0, 256).unwrap();
    assert!(
        page.entries
            .iter()
            .any(|e| e.kind == "suppressed" && e.window == Some(w1))
    );
    assert!(!page.entries.iter().any(|e| e.kind == "message"));

    d.clear_fault(FaultKind::IpcDisconnect, w1).expect("clear");
    d.prepare_fixture("ipc_emit", None, json!({"window": w1, "body": "not-json"}))
        .expect("emit");
    let page = d.observe_receipts(0, 256).unwrap();
    assert!(
        page.entries
            .iter()
            .any(|e| e.kind == "message" && e.window == Some(w1))
    );
    d.shutdown(ShutdownMode::Graceful).expect("shutdown");
}

#[test]
fn e2e_window_crash_fault_keeps_other_windows() {
    let mut d = spawn_stdio(&[]).expect("handshake");
    let receipt = d
        .prepare_fixture("windows", None, json!({"count": 1}))
        .expect("fixture");
    let w2 = receipt.applied["windows"][0].as_u64().unwrap();
    let w1 = d.observe_windows().unwrap()[0].id;

    d.set_fault(FaultKind::WindowCrash, w1).expect("crash");
    let wins = d.observe_windows().unwrap();
    assert_eq!(wins.len(), 1);
    assert_eq!(wins[0].id, w2);
    let notices = pending_notices(&mut d);
    assert!(
        notices
            .iter()
            .any(|n| n["kind"] == "window.destroyed" && n["window"] == w1)
    );
    d.shutdown(ShutdownMode::Graceful).expect("shutdown");
}

#[test]
fn e2e_restart_issues_fresh_session() {
    let mut d = spawn_stdio(&[]).expect("handshake");
    let s1 = d.welcome().session_id.clone();
    let mut cmd = dummy_cmd();
    let d2 = d.restart(&mut cmd).expect("restart");
    assert_ne!(d2.welcome().session_id, s1);
    assert_eq!(d2.welcome().app.name, "dummy_app");
    let mut d2 = d2;
    d2.shutdown(ShutdownMode::Graceful).expect("shutdown");
}

// ---------- abnormal paths ----------

#[test]
fn e2e_abnormal_termination_is_reaped() {
    let mut d = spawn_stdio(&[]).expect("handshake");
    d.kill().expect("kill");
    let status = d.try_exit_status().expect("exit status");
    assert!(status.is_some(), "killed child must be reaped");
    assert!(!status.unwrap().success());
    // Drop on a dead child is a no-op; no orphan process survives.
}

#[test]
fn e2e_driver_disconnect_runs_app_cleanup() {
    // Drive the child manually so we can drop the pipe mid-session.
    let mut child = dummy_cmd()
        .env(ENV_CONTROL, "stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    write_frame(
        &mut stdin,
        &json!({
            "v": 1u32, "kind": "intent", "id": "h",
            "op": "test.hello", "params": {"run_id": "r-eof"}
        })
        .to_string(),
    )
    .unwrap();
    match read_frame(&mut stdout).unwrap() {
        FrameRead::Frame(body) => {
            let v: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(v["ok"], true);
        }
        other => panic!("expected welcome, got {other:?}"),
    }
    // Driver failure: drop the write end mid-session.
    drop(stdin);
    let status = child.wait().expect("wait");
    assert!(
        status.success(),
        "driver EOF must run graceful cleanup then exit 0"
    );
}

// ---------- fail-closed ----------

#[test]
fn e2e_fail_init_reports_failed_handshake() {
    let err = spawn_stdio(&["--fail-init"]).expect_err("must refuse");
    match err {
        DriverError::Handshake(m) => assert!(m.contains("not ready"), "{m}"),
        other => panic!("expected Handshake error, got {other:?}"),
    }
}

#[test]
fn e2e_no_control_env_offers_no_channel() {
    // A release-shaped run: no RVF_TEST_* — the control port is
    // unreachable, the app refuses serving and exits cleanly.
    let mut child = dummy_cmd()
        .env_remove(ENV_CONTROL)
        .env_remove(ENV_ADDR)
        .env_remove(ENV_SECRET)
        .env_remove(ENV_DISABLE)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let status = child.wait().expect("wait");
    assert!(status.success(), "no-control run must still exit 0");
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    assert!(err.contains("Disabled"), "stderr: {err}");
}

#[test]
fn e2e_disable_env_kills_active_transport() {
    // RVF_TEST_DISABLE beats an explicit opt-in: the child refuses
    // serve even though a driver asked for stdio.
    let mut cmd = dummy_cmd();
    cmd.env(ENV_DISABLE, "1");
    let err = TestDriver::spawn_stdio(&mut cmd).expect_err("must refuse");
    match err {
        DriverError::Handshake(m) => {
            assert!(m.contains("closed before welcome"), "{m}");
        }
        other => panic!("expected handshake refusal, got {other:?}"),
    }
}

#[test]
fn e2e_unauthenticated_ops_rejected_before_hello() {
    let mut child = dummy_cmd()
        .env(ENV_CONTROL, "stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    write_frame(
        &mut stdin,
        &json!({
            "v": 1u32, "kind": "intent", "id": "x",
            "op": "test.observe.windows", "params": {}
        })
        .to_string(),
    )
    .unwrap();
    match read_frame(&mut stdout).unwrap() {
        FrameRead::Frame(body) => {
            let v: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(v["ok"], false);
            assert_eq!(v["error"]["code"], "unauthenticated");
        }
        other => panic!("expected result, got {other:?}"),
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
}

// ---------- loopback TCP (opt-in transport) ----------

#[test]
fn e2e_tcp_authenticated_lifecycle() {
    let mut cmd = dummy_cmd();
    let mut d = TestDriver::spawn_tcp(&mut cmd).expect("tcp handshake");
    assert_eq!(d.welcome().backend.name, "headless-fake");
    d.shutdown(ShutdownMode::Graceful).expect("shutdown");
}

#[test]
fn e2e_tcp_wrong_secret_is_unauthenticated() {
    // Manually stand up the listener + child so we control the
    // presented secret.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let mut child = dummy_cmd()
        .env(ENV_CONTROL, "tcp")
        .env(ENV_ADDR, addr.to_string())
        .env(ENV_SECRET, "right-secret")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let (mut stream, peer) = loop {
        match listener.accept() {
            Ok(pair) => break pair,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    panic!("child never dialled the listener");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("accept: {e}"),
        }
    };
    assert!(peer.ip().is_loopback());
    // Accepted sockets inherit nonblocking on Windows — restore.
    stream.set_nonblocking(false).unwrap();
    write_frame(
        &mut stream,
        &json!({
            "v": 1u32, "kind": "intent", "id": "h",
            "op": "test.hello",
            "params": {"run_id": "r", "secret": "wrong-secret"}
        })
        .to_string(),
    )
    .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    match read_frame(&mut reader).unwrap() {
        FrameRead::Frame(body) => {
            let v: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(v["ok"], false);
            assert_eq!(v["error"]["code"], "unauthenticated");
        }
        other => panic!("expected result, got {other:?}"),
    }
    // Send FIN for the write direction so the server reads a clean EOF;
    // keep the sockets alive until the child exits — dropping them with
    // data in flight can race into an RST on Windows and turn the
    // server's EOF into a hard read error.
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let status = child.wait().expect("wait");
    drop(stream);
    drop(reader);
    assert!(status.success(), "unauthenticated then EOF exits cleanly");
}

#[test]
fn e2e_tcp_without_secret_refuses_to_serve() {
    // A TCP opt-in without a run-bound secret must fail closed:
    // the agent refuses to dial and the app exits without serving.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let mut child = dummy_cmd()
        .env(ENV_CONTROL, "tcp")
        .env(ENV_ADDR, addr.to_string())
        .env_remove(ENV_SECRET)
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    // No connection ever arrives.
    listener.set_nonblocking(true).expect("nonblocking");
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut connected = false;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((s, _)) => {
                connected = true;
                drop(s);
            }
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
        if connected {
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(!connected, "app must not dial without a run secret");
}

// ---------- stall: DriverError::Timeout paths ----------

/// `--stall after-serve` completes the protocol session — the
/// shutdown reply (including `cleanup.exited`, the host-loop flag,
/// not the OS process) is delivered — but the process never exits.
/// A bounded [`TestDriver::wait_exit`] must report
/// [`DriverError::Timeout`], never hang or silently pass, and `kill`
/// must still reap the stalled child.
#[test]
fn e2e_stall_after_serve_wait_exit_times_out() {
    let mut d = spawn_stdio(&["--stall", "after-serve"]).expect("handshake");
    let reply = d
        .call(ops::SHUTDOWN, json!({ "mode": "graceful" }))
        .expect("shutdown reply");
    assert_eq!(reply["bye"], true);
    assert_eq!(reply["cleanup"]["exited"], true);
    let err = d
        .wait_exit(Duration::from_millis(500))
        .expect_err("stalled child must not be reported as exited");
    assert!(
        matches!(err, DriverError::Timeout),
        "expected DriverError::Timeout, got {err:?}"
    );
    // The cleanup guarantee still holds: kill reaps the stalled child.
    d.kill().expect("kill");
    let status = d.try_exit_status().expect("exit status");
    assert!(status.is_some_and(|s| !s.success()));
}

/// `--stall before-serve` never opens the control channel: the
/// driver's TCP accept loop must give up with
/// [`DriverError::Timeout`] at its bound instead of hanging, and the
/// never-dialling child is killed — no orphan survives.
#[test]
fn e2e_stall_before_serve_tcp_connect_times_out() {
    let mut cmd = dummy_cmd();
    cmd.arg("--stall").arg("before-serve");
    let err = TestDriver::spawn_tcp(&mut cmd).expect_err("must time out");
    assert!(
        matches!(err, DriverError::Timeout),
        "expected DriverError::Timeout, got {err:?}"
    );
}

/// Driver EOF ends the serve loop the same way `test.shutdown`
/// does — with `--stall after-serve` the abnormal-end path must also
/// leave the process alive (non-exit is not graceful-exit-only), and
/// an explicit kill still reaps it.
#[test]
fn e2e_stall_after_driver_eof_child_survives_until_killed() {
    let mut child = dummy_cmd()
        .arg("--stall")
        .arg("after-serve")
        .env(ENV_CONTROL, "stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    write_frame(
        &mut stdin,
        &json!({
            "v": 1u32, "kind": "intent", "id": "h",
            "op": "test.hello", "params": {"run_id": "r-stall-eof"}
        })
        .to_string(),
    )
    .unwrap();
    match read_frame(&mut stdout).unwrap() {
        FrameRead::Frame(body) => {
            let v: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(v["ok"], true);
        }
        other => panic!("expected welcome, got {other:?}"),
    }
    // Driver failure: drop the write end mid-session.
    drop(stdin);
    // The stalled AUT must still be running — EOF cleanup ran but
    // the process refuses to exit.
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        assert!(
            child.try_wait().unwrap().is_none(),
            "stalled child exited after driver EOF"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().expect("kill");
    let status = child.wait().expect("wait");
    assert!(!status.success());
}
