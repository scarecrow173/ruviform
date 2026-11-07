//! `TestDriver` — the test-side `rvf.uitest.v1` client (Issue #312).
//!
//! Spawns the app-under-test (stdio default, or loopback TCP with a
//! run-bound secret), performs the ready/failed handshake, exposes
//! typed wrappers for every `test.*` op, buffers asynchronous
//! `Notify` events, and owns cleanup: `shutdown` waits for the child
//! to exit, and `Drop` kills a still-running child so no orphan
//! process/window/port survives a failed test.
//!
//! **RT contract:** non-RT only.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use ruviform_ui::ipc::{ErrorBody, Intent, codes};
use serde_json::{Value, json};

use crate::proto::{
    CleanupReport, ClockDomain, FaultKind, FixtureReceipt, Notice, ReceiptPage, Screenshot,
    ShutdownMode, Welcome, WindowMeta, ops,
};
use crate::transport::{
    ENV_ADDR, ENV_CONTROL, ENV_SECRET, FrameRead, gen_token, is_loopback, read_frame, write_frame,
};

/// Default handshake/accept wait bound.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Default shutdown wait bound for the child to exit.
const EXIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Driver-side failure.
#[non_exhaustive]
#[derive(Debug)]
pub enum DriverError {
    /// Transport or process I/O failed.
    Io(std::io::Error),
    /// The agent returned an error result (`code`/`message`).
    Protocol(ErrorBody),
    /// The handshake failed or the app refused the session.
    Handshake(String),
    /// The control channel ended unexpectedly.
    Disconnected,
    /// A wait exceeded its bound.
    Timeout,
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Protocol(e) => write!(f, "protocol: {e}"),
            Self::Handshake(m) => write!(f, "handshake: {m}"),
            Self::Disconnected => write!(f, "control channel disconnected"),
            Self::Timeout => write!(f, "wait timed out"),
        }
    }
}

impl std::error::Error for DriverError {}

impl std::fmt::Debug for TestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDriver")
            .field("run_id", &self.run_id)
            .field("session_id", &self.welcome.session_id)
            .field("alive", &self.child.is_some())
            .finish_non_exhaustive()
    }
}

impl From<std::io::Error> for DriverError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ErrorBody> for DriverError {
    fn from(e: ErrorBody) -> Self {
        Self::Protocol(e)
    }
}

/// Test-side client: owns the child process and the control channel.
pub struct TestDriver {
    child: Option<Child>,
    reader: Box<dyn BufRead + Send>,
    writer: Box<dyn Write + Send>,
    welcome: Welcome,
    pending: VecDeque<Value>,
    next_id: u64,
    run_id: String,
}

impl TestDriver {
    /// Spawns `cmd` with the stdio transport (`RVF_TEST_CONTROL` is
    /// set for the child) and performs the handshake.
    ///
    /// # Errors
    /// [`DriverError`] on spawn, channel, or handshake failure.
    pub fn spawn_stdio(cmd: &mut Command) -> Result<Self, DriverError> {
        cmd.env(ENV_CONTROL, "stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            DriverError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "child stdin unavailable",
            ))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            DriverError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "child stdout unavailable",
            ))
        })?;
        Self::handshake(
            Some(child),
            Box::new(BufReader::new(stdout)),
            Box::new(stdin),
            None,
        )
    }

    /// Spawns `cmd` with the loopback-TCP transport: binds an
    /// ephemeral loopback listener, passes `RVF_TEST_ADDR` +
    /// `RVF_TEST_SECRET` to the child, accepts exactly one
    /// connection (loopback peers only), then performs the
    /// authenticated handshake.
    ///
    /// # Errors
    /// [`DriverError`] on bind/spawn/accept/handshake failure.
    pub fn spawn_tcp(cmd: &mut Command) -> Result<Self, DriverError> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener
            .local_addr()
            .map_err(|e| DriverError::Handshake(format!("listener addr: {e}")))?;
        let secret = gen_token();
        cmd.env(ENV_CONTROL, "tcp")
            .env(ENV_ADDR, addr.to_string())
            .env(ENV_SECRET, &secret)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn()?;
        listener.set_nonblocking(true)?;
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let stream = loop {
            match listener.accept() {
                Ok((s, peer)) => {
                    if !is_loopback(peer.ip()) {
                        drop(s);
                        let _ = child.kill();
                        return Err(DriverError::Handshake(format!(
                            "refusing non-loopback peer {peer}"
                        )));
                    }
                    break s;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        return Err(DriverError::Timeout);
                    }
                    if child.try_wait()?.is_some() {
                        return Err(DriverError::Handshake(
                            "app exited before TCP handshake".to_string(),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(e.into());
                }
            }
        };
        // Accepted sockets inherit the listener's nonblocking flag
        // (Windows) — restore blocking I/O for the framed channel.
        stream.set_nonblocking(false)?;
        Self::handshake(
            Some(child),
            Box::new(BufReader::new(stream.try_clone()?)),
            Box::new(stream),
            Some(secret.as_str()),
        )
    }

    /// Handshake: `test.hello` → `Welcome` (or `Handshake` error).
    fn handshake(
        child: Option<Child>,
        mut reader: Box<dyn BufRead + Send>,
        mut writer: Box<dyn Write + Send>,
        secret: Option<&str>,
    ) -> Result<Self, DriverError> {
        let run_id = format!("run-{}", gen_token());
        let mut params = json!({ "run_id": run_id, "driver": "ruviform-ui-test" });
        if let Some(s) = &secret {
            params["secret"] = json!(s);
        }
        let intent = Intent::new("d1", ops::HELLO, params);
        let body = serde_json::to_string(&intent)
            .map_err(|e| DriverError::Handshake(format!("intent serialize: {e}")))?;
        write_frame(&mut writer, &body)?;
        let welcome: Welcome = loop {
            match read_frame(&mut reader)? {
                FrameRead::Frame(body) => {
                    let v: Value = serde_json::from_str(&body)
                        .map_err(|e| DriverError::Handshake(format!("bad welcome json: {e}")))?;
                    match v.get("kind").and_then(Value::as_str) {
                        Some("result") if v.get("id") == Some(&Value::String("d1".into())) => {
                            if v.get("ok") == Some(&Value::Bool(true)) {
                                break serde_json::from_value(v["data"].clone()).map_err(|e| {
                                    DriverError::Handshake(format!("bad welcome: {e}"))
                                })?;
                            }
                            let err: ErrorBody = serde_json::from_value(v["error"].clone())
                                .unwrap_or_else(|_| {
                                    ErrorBody::new(codes::SERVICE_ERROR, "handshake failed")
                                });
                            return Err(DriverError::Handshake(format!(
                                "test.hello refused: {err}"
                            )));
                        }
                        _ => {}
                    }
                }
                FrameRead::Eof => {
                    return Err(DriverError::Handshake(
                        "control channel closed before welcome".to_string(),
                    ));
                }
                FrameRead::TooLarge { declared } => {
                    return Err(DriverError::Handshake(format!(
                        "oversized welcome frame {declared}"
                    )));
                }
            }
        };
        Ok(Self {
            child,
            reader,
            writer,
            welcome,
            pending: VecDeque::new(),
            next_id: 1,
            run_id,
        })
    }

    /// The negotiated `Welcome` (session id, caps, metadata).
    #[must_use]
    pub fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    /// The driver-issued run id.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Sends one op and waits for its `result`; intermediate `Notify`
    /// events are buffered for [`notifications`](Self::notifications).
    ///
    /// # Errors
    /// [`DriverError::Protocol`] for an error result,
    /// [`DriverError::Disconnected`] on channel EOF, or
    /// [`DriverError::Io`] on transport failure.
    pub fn call(&mut self, op: &str, params: Value) -> Result<Value, DriverError> {
        self.next_id += 1;
        let id = format!("d{}", self.next_id);
        let intent = Intent::new(id.clone(), op, params);
        let body = serde_json::to_string(&intent)
            .map_err(|e| DriverError::Handshake(format!("intent serialize: {e}")))?;
        write_frame(&mut self.writer, &body)?;
        loop {
            match read_frame(&mut self.reader)? {
                FrameRead::Frame(body) => {
                    let v: Value = serde_json::from_str(&body)
                        .map_err(|e| DriverError::Handshake(format!("bad event json: {e}")))?;
                    match v.get("kind").and_then(Value::as_str) {
                        Some("result") if v.get("id") == Some(&Value::String(id.clone())) => {
                            if v.get("ok") == Some(&Value::Bool(true)) {
                                return Ok(v.get("data").cloned().unwrap_or(Value::Null));
                            }
                            let err: ErrorBody = serde_json::from_value(v["error"].clone())
                                .map_err(|e| {
                                    DriverError::Handshake(format!("bad error body: {e}"))
                                })?;
                            return Err(DriverError::Protocol(err));
                        }
                        Some("notify") => {
                            self.pending.push_back(v["data"].clone());
                        }
                        Some("error") => {
                            let err: ErrorBody = serde_json::from_value(v["error"].clone())
                                .map_err(|e| {
                                    DriverError::Handshake(format!("bad error body: {e}"))
                                })?;
                            return Err(DriverError::Protocol(err));
                        }
                        _ => {}
                    }
                }
                FrameRead::Eof => return Err(DriverError::Disconnected),
                FrameRead::TooLarge { declared } => {
                    return Err(DriverError::Handshake(format!(
                        "oversized reply frame {declared}"
                    )));
                }
            }
        }
    }

    /// Buffered `Notify` payloads plus any drained `test.notices`
    /// replies — call after each op to collect async events.
    pub fn notifications(&mut self) -> Vec<Value> {
        self.pending.drain(..).collect()
    }

    // ---- typed ops ----

    /// `test.capabilities`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn capabilities(&mut self) -> Result<Value, DriverError> {
        self.call(ops::CAPABILITIES, json!({}))
    }

    /// `test.fixture.prepare`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure or fixture rejection.
    pub fn prepare_fixture(
        &mut self,
        name: &str,
        seed: Option<u64>,
        params: Value,
    ) -> Result<FixtureReceipt, DriverError> {
        let mut p = json!({ "name": name, "seed": seed });
        p["params"] = params;
        let data = self.call(ops::FIXTURE_PREPARE, p)?;
        serde_json::from_value(data).map_err(|e| DriverError::Handshake(e.to_string()))
    }

    /// `test.clock.advance`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure or an uncontrollable domain.
    pub fn advance_clock(&mut self, domain: ClockDomain, ticks: u64) -> Result<Value, DriverError> {
        self.call(
            ops::CLOCK_ADVANCE,
            json!({ "domain": domain.as_str(), "ticks": ticks }),
        )
    }

    /// `test.fault.set`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn set_fault(&mut self, kind: FaultKind, window: u64) -> Result<Value, DriverError> {
        self.call(
            ops::FAULT_SET,
            json!({ "kind": kind.as_str(), "window": window }),
        )
    }

    /// `test.fault.clear`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn clear_fault(&mut self, kind: FaultKind, window: u64) -> Result<Value, DriverError> {
        self.call(
            ops::FAULT_CLEAR,
            json!({ "kind": kind.as_str(), "window": window }),
        )
    }

    /// `test.window.request_close`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn request_close(&mut self, window: u64) -> Result<Value, DriverError> {
        self.call(ops::WINDOW_REQUEST_CLOSE, json!({ "window": window }))
    }

    /// `test.observe.windows`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn observe_windows(&mut self) -> Result<Vec<WindowMeta>, DriverError> {
        let data = self.call(ops::OBSERVE_WINDOWS, json!({}))?;
        let windows = data
            .get("windows")
            .cloned()
            .ok_or_else(|| DriverError::Handshake("missing windows field".to_string()))?;
        serde_json::from_value(windows).map_err(|e| DriverError::Handshake(e.to_string()))
    }

    /// `test.observe.receipts`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn observe_receipts(&mut self, since: u64, limit: u64) -> Result<ReceiptPage, DriverError> {
        let data = self.call(
            ops::OBSERVE_RECEIPTS,
            json!({ "since": since, "limit": limit }),
        )?;
        serde_json::from_value(data).map_err(|e| DriverError::Handshake(e.to_string()))
    }

    /// `test.observe.screenshot` — returns the `unavailable` protocol
    /// error on backends without capture.
    ///
    /// # Errors
    /// [`DriverError`] on call failure (incl. `unavailable`).
    pub fn screenshot(&mut self, window: u64) -> Result<Screenshot, DriverError> {
        let data = self.call(ops::OBSERVE_SCREENSHOT, json!({ "window": window }))?;
        serde_json::from_value(data).map_err(|e| DriverError::Handshake(e.to_string()))
    }

    /// `test.notices`.
    ///
    /// # Errors
    /// [`DriverError`] on call failure.
    pub fn notices(&mut self) -> Result<Vec<Notice>, DriverError> {
        let data = self.call(ops::NOTICES, json!({}))?;
        serde_json::from_value(data["notices"].clone())
            .map_err(|e| DriverError::Handshake(e.to_string()))
    }

    /// `test.shutdown` — sends the op, reads `{bye, cleanup}`, waits
    /// for the child to exit, and returns the cleanup report.
    ///
    /// # Errors
    /// [`DriverError`] on call failure or exit timeout.
    pub fn shutdown(&mut self, mode: ShutdownMode) -> Result<CleanupReport, DriverError> {
        let mode_s = match mode {
            ShutdownMode::Graceful => "graceful",
            _ => "now",
        };
        let data = self.call(ops::SHUTDOWN, json!({ "mode": mode_s }));
        let report: CleanupReport = match data {
            Ok(v) => serde_json::from_value(v["cleanup"].clone())
                .map_err(|e| DriverError::Handshake(e.to_string()))?,
            Err(DriverError::Disconnected) => CleanupReport {
                windows_open: 0,
                windows_closed: 0,
                exited: false,
                app: Value::Null,
            },
            Err(e) => return Err(e),
        };
        let _ = self.wait_exit(EXIT_TIMEOUT)?;
        Ok(report)
    }

    /// Ends the run and spawns a fresh session — returns the new
    /// driver. The new `Welcome` carries a new `session_id`.
    ///
    /// # Errors
    /// [`DriverError`] on shutdown or respawn failure.
    pub fn restart(&mut self, cmd: &mut Command) -> Result<Self, DriverError> {
        let _ = self.shutdown(ShutdownMode::Graceful);
        Self::spawn_stdio(cmd)
    }

    /// Waits for the child to exit within `timeout`.
    ///
    /// # Errors
    /// [`DriverError::Timeout`] if it outlives the bound.
    pub fn wait_exit(&mut self, timeout: Duration) -> Result<ExitStatus, DriverError> {
        let Some(child) = self.child.as_mut() else {
            return Err(DriverError::Disconnected);
        };
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if Instant::now() > deadline {
                return Err(DriverError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Kills the child immediately (the abnormal-termination path —
    /// cleanup still reaps the process).
    ///
    /// # Errors
    /// [`DriverError::Io`] on kill failure.
    pub fn kill(&mut self) -> Result<(), DriverError> {
        if let Some(child) = self.child.as_mut() {
            child.kill()?;
            let _ = child.wait();
        }
        Ok(())
    }

    /// The child's exit status if it already exited.
    ///
    /// # Errors
    /// [`DriverError::Io`] on query failure.
    pub fn try_exit_status(&mut self) -> Result<Option<ExitStatus>, DriverError> {
        match self.child.as_mut() {
            Some(c) => Ok(c.try_wait()?),
            None => Ok(None),
        }
    }
}

impl Drop for TestDriver {
    /// Cleanup guarantee: a live child is killed and reaped — no
    /// orphan process, window, or port survives a dropped driver.
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}
