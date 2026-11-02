//! Multi-window contract tests (Issue #181) — a fake host drives
//! [`UiRuntime::handle_window`] and [`WindowHandle`] end to end,
//! proving:
//!
//! - events from two windows stay separated by [`WindowId`];
//! - `CloseRequested` is veto-able and distinct from `Destroyed`;
//! - delivery to an already-destroyed window drops safely;
//! - colliding per-webview correlation ids route independently;
//! - a sub-window close never cancels jobs or exits the loop.
//!
//! **RT contract:** non-RT only.

use std::collections::HashMap;
use std::time::Duration;

use ruviform_ui::{
    CallOutcome, CloseDecision, DataRegistry, DemoService, ErrorBody, Event, HostEvent,
    LifecycleState, MonitorInfo, OpInfo, RoutedEvent, Service, ServiceContext, UiError, UiRuntime,
    WindowConfig, WindowHandle, WindowId, codes, dispatch_script,
};
use serde_json::{Value, json};

// ---------- fake host ----------

struct FakeWindow {
    ready: bool,
    close_pending: bool,
    backlog: Vec<String>,
}

/// Minimal multi-window host implementing [`WindowHandle`]: windows
/// are records in a map, `eval` deliveries are captured per window so
/// tests can inspect routing. Models the adapter delivery contract —
/// unknown/destroyed destinations drop silently, not-ready windows
/// buffer a bounded backlog.
struct FakeHost {
    next: u64,
    windows: HashMap<WindowId, FakeWindow>,
    /// `eval` scripts delivered to a ready window: `(window, script)`.
    delivered: Vec<(WindowId, String)>,
    exited: bool,
}

const MAX_BACKLOG: usize = 16;

impl FakeHost {
    fn new() -> Self {
        Self {
            next: 1,
            windows: HashMap::new(),
            delivered: Vec::new(),
            exited: false,
        }
    }

    fn spawn(&mut self, title: &str) -> WindowId {
        let cfg = WindowConfig {
            title: title.to_string(),
            ..WindowConfig::default()
        };
        self.create_window(&cfg).expect("fake create")
    }

    /// Marks the window's webview as loaded → `Ready` flow: the
    /// buffered backlog flushes.
    fn mark_ready(&mut self, w: WindowId) {
        let fw = self.windows.get_mut(&w).expect("window exists");
        fw.ready = true;
        let flush = std::mem::take(&mut fw.backlog);
        self.delivered.extend(flush.into_iter().map(|s| (w, s)));
    }

    /// Simulates the user clicking the close button: the host marks
    /// the close pending and the runtime sees `CloseRequested`.
    fn user_close(&mut self, w: WindowId) {
        self.windows
            .get_mut(&w)
            .expect("window exists")
            .close_pending = true;
    }

    /// Delivered scripts at a window.
    fn scripts(&self, w: WindowId) -> Vec<&str> {
        self.delivered
            .iter()
            .filter(|(dw, _)| *dw == w)
            .map(|(_, s)| s.as_str())
            .collect()
    }

    /// Delivers a routed event the way an adapter would.
    fn deliver(&mut self, re: &RoutedEvent) {
        if let Some(w) = re.window {
            self.eval(w, &dispatch_script(&re.event));
        }
    }
}

impl WindowHandle for FakeHost {
    fn eval(&mut self, window: WindowId, script: &str) {
        // Delivery contract: unknown/destroyed → silent drop, never an
        // error or panic; not-ready → bounded backlog (drop-oldest).
        let Some(fw) = self.windows.get_mut(&window) else {
            return;
        };
        if fw.ready {
            self.delivered.push((window, script.to_string()));
        } else {
            if fw.backlog.len() >= MAX_BACKLOG {
                fw.backlog.remove(0);
            }
            fw.backlog.push(script.to_string());
        }
    }

    fn resize(&mut self, window: WindowId, _w: u32, _h: u32) -> Result<(), UiError> {
        if self.windows.contains_key(&window) {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn focus(&mut self, window: WindowId, _focused: bool) -> Result<(), UiError> {
        if self.windows.contains_key(&window) {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn create_window(&mut self, _config: &WindowConfig) -> Result<WindowId, UiError> {
        let id = WindowId::new(self.next).expect("nonzero");
        self.next += 1;
        self.windows.insert(
            id,
            FakeWindow {
                ready: false,
                close_pending: false,
                backlog: Vec::new(),
            },
        );
        Ok(id)
    }

    fn request_close(&mut self, window: WindowId) {
        if self.windows.contains_key(&window) {
            self.user_close(window);
        }
    }

    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError> {
        let Some(fw) = self.windows.get_mut(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        if !fw.close_pending {
            return Err(UiError::host(format!(
                "window {window} has no pending close"
            )));
        }
        match decision {
            CloseDecision::Approve => {
                self.windows.remove(&window);
            }
            CloseDecision::Veto => fw.close_pending = false,
            _ => {} // Defer (and future decisions): stay close-pending
        }
        Ok(())
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        if self.windows.remove(&window).is_some() {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        vec![MonitorInfo {
            name: Some("fake-monitor".to_string()),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale_factor: 1.0,
        }]
    }

    fn exit(&mut self) {
        self.exited = true;
    }
}

// ---------- helpers ----------

fn rt() -> UiRuntime {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(DemoService::new())
        .expect("demo service registration");
    rt
}

fn intent_body(id: &str, op: &str, params: &Value) -> String {
    format!(
        r#"{{"v":1,"kind":"intent","id":"{id}","op":"{op}","params":{}}}"#,
        serde_json::to_string(params).expect("params json")
    )
}

/// Waits for all running jobs and returns the routed drain — what a
/// fake-host tick loop would see (`drain_jobs_routed` is the
/// routed-path equivalent of `poll`).
fn drain_all(rt: &mut UiRuntime, timeout: Duration) -> Vec<RoutedEvent> {
    let deadline = std::time::Instant::now() + timeout;
    let mut all = Vec::new();
    while rt.has_running_jobs() && std::time::Instant::now() < deadline {
        all.extend(rt.drain_jobs_routed());
        std::thread::sleep(Duration::from_millis(10));
    }
    all.extend(rt.drain_jobs_routed());
    all
}

// ---------- tests ----------

/// Events from two windows stay separated: each window's `Message`
/// produces routed events tagged with its own [`WindowId`].
#[test]
fn multi_window_events_stay_separated() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w1 = host.spawn("main");
    let w2 = host.spawn("panel");

    let r1 = rt.handle_window(
        w1,
        HostEvent::Message(intent_body("i-1", "sys.ping", &json!({}))),
    );
    let r2 = rt.handle_window(
        w2,
        HostEvent::Message(intent_body("i-9", "sys.ping", &json!({}))),
    );

    assert_eq!(r1.len(), 1);
    assert_eq!(r2.len(), 1);
    assert_eq!(r1[0].window, Some(w1));
    assert_eq!(r2[0].window, Some(w2));
    let id_of = |e: &Event| match e {
        Event::Result { id, .. } => id.clone(),
        other => panic!("expected result, got {other:?}"),
    };
    assert_eq!(id_of(&r1[0].event), "i-1");
    assert_eq!(id_of(&r2[0].event), "i-9");
}

/// Duplicate per-webview correlation ids (`i-1` from both `WebViews`)
/// route independently — routing is by window, not by id.
#[test]
fn colliding_correlation_ids_route_by_window() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w1 = host.spawn("main");
    let w2 = host.spawn("panel");
    host.mark_ready(w1);
    host.mark_ready(w2);

    // Both webviews use the same local correlation id — the
    // WebView-local counter restarts per page.
    let r1 = rt.handle_window(
        w1,
        HostEvent::Message(intent_body("i-1", "sys.ping", &json!({}))),
    );
    let r2 = rt.handle_window(
        w2,
        HostEvent::Message(intent_body("i-1", "sys.ping", &json!({}))),
    );

    for re in r1.iter().chain(r2.iter()) {
        host.deliver(re);
    }

    assert_eq!(host.scripts(w1).len(), 1);
    assert_eq!(host.scripts(w2).len(), 1);
    // Both scripts carry "i-1" but were delivered to distinct windows.
    assert!(host.scripts(w1)[0].contains("i-1"));
    assert!(host.scripts(w2)[0].contains("i-1"));
}

/// `CloseRequested` is veto-able: veto keeps the window open and
/// other windows untouched.
#[test]
fn close_request_can_be_vetoed() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w1 = host.spawn("main");
    let w2 = host.spawn("panel");
    host.mark_ready(w1);

    host.user_close(w1);
    let evs = rt.handle_window(w1, HostEvent::CloseRequested);
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].window, Some(w1));
    assert!(matches!(
        evs[0].event,
        Event::Lifecycle {
            state: LifecycleState::Closing,
        }
    ));

    // Consumer vetoes: window stays open, close is cleared.
    host.respond_close(w1, CloseDecision::Veto)
        .expect("veto ok");
    assert!(host.windows.contains_key(&w1));
    assert!(host.windows.contains_key(&w2));

    // A vetoed close leaves a live window that still receives events.
    let r = rt.handle_window(
        w1,
        HostEvent::Message(intent_body("i-2", "sys.ping", &json!({}))),
    );
    host.deliver(&r[0]);
    assert_eq!(host.scripts(w1).len(), 1);
}

/// `Destroyed` is distinct from `CloseRequested`: the request alone
/// destroys nothing; approving ends the window.
#[test]
fn destroy_is_distinct_from_close_request() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w = host.spawn("doomed");

    host.user_close(w);
    let _ = rt.handle_window(w, HostEvent::CloseRequested);

    // Window is still alive after the request alone.
    assert!(host.windows.contains_key(&w));

    host.respond_close(w, CloseDecision::Approve)
        .expect("approve");
    assert!(!host.windows.contains_key(&w));

    // `Destroyed` is a window-terminal event the runtime can consume
    // without producing IPC traffic.
    let evs = rt.handle_window(w, HostEvent::Destroyed);
    assert!(evs.is_empty());
}

/// `defer` keeps the window alive in a close-pending state; a later
/// `destroy_window` still ends it.
#[test]
fn deferred_close_stays_alive_then_destroyed() {
    let mut host = FakeHost::new();
    let w = host.spawn("deferred");

    host.user_close(w);
    host.respond_close(w, CloseDecision::Defer).expect("defer");
    assert!(host.windows.contains_key(&w));
    assert!(host.windows[&w].close_pending);

    host.destroy_window(w).expect("destroy");
    assert!(!host.windows.contains_key(&w));
}

/// Delivery to an already-destroyed window drops silently — no
/// panic, no error, no wedged host.
#[test]
fn delivery_to_dead_window_is_safe() {
    let mut host = FakeHost::new();
    let w = host.spawn("gone");
    host.destroy_window(w).expect("destroy");

    // eval to a dead window: silent drop.
    host.eval(w, "window.__rvf({});");
    // Fallible ops on dead windows report errors.
    assert!(host.resize(w, 100, 100).is_err());
    assert!(host.focus(w, true).is_err());
    assert!(host.respond_close(w, CloseDecision::Approve).is_err());
    assert!(host.destroy_window(w).is_err());
    assert!(!host.exited);
}

/// A not-yet-ready window buffers evals and flushes them on `Ready`.
#[test]
fn eval_before_ready_is_buffered_then_flushed() {
    let mut host = FakeHost::new();
    let w = host.spawn("booting");

    host.eval(w, "early();"); // buffered — no Ready yet
    assert_eq!(host.scripts(w), [] as [&str; 0]);

    host.mark_ready(w); // → Ready
    assert_eq!(host.scripts(w), vec!["early();"]);
}

/// A sub-window `CloseRequested` + `Destroyed` never cancels jobs
/// owned by other windows, and never exits the loop.
#[test]
fn subwindow_close_leaves_jobs_and_loop_running() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w1 = host.spawn("main");
    let w2 = host.spawn("panel");

    // Start a long job from w1.
    let spawn = rt.handle_window(
        w1,
        HostEvent::Message(intent_body("i-1", "demo.sleep", &json!({ "ms": 500 }))),
    );
    assert_eq!(spawn.len(), 1);
    assert!(rt.has_running_jobs());

    // w2 closes completely — runtime emits its lifecycle event but
    // must not cancel w1's job or signal loop exit.
    host.user_close(w2);
    let _ = rt.handle_window(w2, HostEvent::CloseRequested);
    host.respond_close(w2, CloseDecision::Approve)
        .expect("approve w2");
    let _ = rt.handle_window(w2, HostEvent::Destroyed);

    assert!(rt.has_running_jobs(), "w1 job must survive w2 close");
    assert!(!host.exited, "loop must not exit on sub-window close");

    // Cleanup: cancel the job so the test doesn't linger.
    assert_eq!(rt.cancel_window_jobs(w1), 1);
}

/// `CloseRequested` on the multi-window path never cancels jobs —
/// even the closing window's own jobs (explicit
/// `cancel_window_jobs` is the opt-in vocabulary).
#[test]
fn close_requested_does_not_cancel_jobs() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w = host.spawn("owner");

    let _ = rt.handle_window(
        w,
        HostEvent::Message(intent_body("i-1", "demo.sleep", &json!({ "ms": 500 }))),
    );
    assert!(rt.has_running_jobs());

    let _ = rt.handle_window(w, HostEvent::CloseRequested);
    assert!(rt.has_running_jobs(), "close request must not cancel jobs");

    // Opt-in cleanup: consumer cancels only this window's jobs.
    assert_eq!(rt.cancel_window_jobs(w), 1);
    // And only that window's: an unrelated job survives.
    let _ = rt.handle_window(
        w,
        HostEvent::Message(intent_body("i-2", "demo.sleep", &json!({ "ms": 500 }))),
    );
    let w2 = host.spawn("other");
    let _ = rt.handle_window(
        w2,
        HostEvent::Message(intent_body("i-3", "demo.sleep", &json!({ "ms": 500 }))),
    );
    assert_eq!(rt.cancel_window_jobs(w), 1);
    assert!(rt.has_running_jobs(), "w2 job must survive");
    rt.cancel_window_jobs(w2);
}

/// Jobs spawned from two windows route their terminal events back to
/// their owner windows via `drain_jobs_routed`.
#[test]
fn job_events_route_to_owner_window() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w1 = host.spawn("main");
    let w2 = host.spawn("panel");

    let _ = rt.handle_window(
        w1,
        HostEvent::Message(intent_body("i-1", "demo.sleep", &json!({ "ms": 30 }))),
    );
    let _ = rt.handle_window(
        w2,
        HostEvent::Message(intent_body("i-2", "demo.sleep", &json!({ "ms": 30 }))),
    );

    let drained = drain_all(&mut rt, Duration::from_secs(5));
    // Both terminal `job` events arrive, each routed to its owner.
    let mut owners = HashMap::new();
    for re in &drained {
        if let Event::Job { job, .. } = re.event {
            owners.insert(job, re.window);
        }
    }
    assert_eq!(owners.len(), 2);
    let mut seen: Vec<WindowId> = owners.values().flatten().copied().collect();
    seen.sort();
    assert_eq!(seen, vec![w1, w2]);
}

/// Headless/legacy dispatch produces unrouted (`window: None`)
/// events — the consumer picks the sink.
#[test]
fn unrouted_events_for_headless_dispatch() {
    let mut rt = rt();
    let _ = rt.dispatch_op("c1".to_string(), "demo.sleep", json!({ "ms": 30 }));
    let drained = drain_all(&mut rt, Duration::from_secs(5));
    assert!(drained.iter().all(|re| re.window.is_none()));
    assert!(
        drained
            .iter()
            .any(|re| matches!(re.event, Event::Job { .. }))
    );
}

/// The `service` context observes the calling window.
#[test]
fn service_context_sees_origin_window() {
    struct Echo;
    impl Service for Echo {
        fn ops(&self) -> &'static [OpInfo] {
            const OPS: &[OpInfo] = &[OpInfo {
                name: "echo.origin",
                summary: "echoes the origin window",
            }];
            OPS
        }
        fn call(
            &mut self,
            op: &str,
            _params: &Value,
            ctx: &mut ServiceContext<'_>,
        ) -> Result<CallOutcome, ErrorBody> {
            assert_eq!(op, "echo.origin");
            Ok(CallOutcome::Result(json!({
                "origin": ctx.origin.map(WindowId::get),
            })))
        }
    }

    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(Echo).expect("register echo");
    let w = WindowId::new(7).expect("nonzero");

    let evs = rt.handle_window(
        w,
        HostEvent::Message(intent_body("i-1", "echo.origin", &json!({}))),
    );
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].window, Some(w));
    match &evs[0].event {
        Event::Result { data, .. } => assert_eq!(data.as_ref().expect("data")["origin"], json!(7)),
        other => panic!("expected result, got {other:?}"),
    }

    // Legacy path: no window context.
    let evs = rt.dispatch_op("c1".to_string(), "echo.origin", json!({}));
    match &evs[0] {
        Event::Result { data, .. } => {
            assert_eq!(data.as_ref().expect("data")["origin"], Value::Null);
        }
        other => panic!("expected result, got {other:?}"),
    }
}

/// Malformed bodies route the uncorrelated error back to the window
/// it arrived on — the error isn't broadcast loop-wide.
#[test]
fn malformed_message_routes_error_to_origin_window() {
    let mut host = FakeHost::new();
    let mut rt = rt();
    let w = host.spawn("main");
    let evs = rt.handle_window(w, HostEvent::Message("garbage".to_string()));
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].window, Some(w));
    assert!(
        matches!(&evs[0].event, Event::Error { error } if error.code == codes::MALFORMED_MESSAGE)
    );
}
