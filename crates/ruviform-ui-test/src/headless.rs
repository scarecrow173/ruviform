//! `HeadlessIo` — the [`TestIo`] implementation over [`FakeDriver`]
//! (Issue #312, rebased onto the neutral driver fake in Issue #313
//! slice S2).
//!
//! Composition: `FakeDriver` (window/event plumbing, `WindowDriver`
//! semantics) + a `TestApp` (the consumer's event wiring) + `Journal`
//! (observation) + named `FixtureController`s. `emit` injects driver
//! events through the fake queue and pumps them deterministically;
//! every pump records inbound receipts, app→host calls pass through
//! `JournalHandle`, and window lifecycle transitions produce notices.
//!
//! **RT contract:** non-RT only.

use ruviform_ui::ipc::{Event, parse_intent};
use ruviform_ui::{
    DriverEvent, ErrorBody, WindowDriver, WindowId, WindowLifecycle, WindowSpec, codes,
};
use serde_json::Value;

use crate::fake_driver::FakeDriver;
use crate::fixture::{FixtureController, TestApp};
use crate::io::TestIo;
use crate::journal::{Journal, JournalHandle};
use crate::proto::{
    BackendMeta, Capabilities, CleanupReport, ClockDomain, DeniedCapabilities, Direction,
    FaultKind, FixtureReceipt, FixtureSpec, Notice, ReadyState, ReceiptPage, Screenshot,
    ShutdownMode, WindowMeta,
};

/// `TestIo` backed by [`FakeDriver`] — the headless path every CI
/// lane can drive end to end.
pub struct HeadlessIo<A: TestApp> {
    host: FakeDriver,
    app: A,
    journal: Journal,
    fixtures: Vec<Box<dyn FixtureController>>,
    ready: ReadyState,
    exited: bool,
    notices: Vec<Notice>,
    notice_seq: u64,
}

impl<A: TestApp> HeadlessIo<A> {
    /// Creates headless wiring around `app` with an empty journal and
    /// no fixtures (register via [`add_fixture`](Self::add_fixture)).
    #[must_use]
    pub fn new(app: A) -> Self {
        Self {
            host: FakeDriver::new(),
            app,
            journal: Journal::default(),
            fixtures: Vec::new(),
            ready: ReadyState::Ready,
            exited: false,
            notices: Vec::new(),
            notice_seq: 0,
        }
    }

    /// Registers a fixture reachable via `test.fixture.prepare`.
    pub fn add_fixture(&mut self, fixture: Box<dyn FixtureController>) {
        self.fixtures.push(fixture);
    }

    /// Overrides the readiness `test.hello` observes — the way an app
    /// reports a failed initialization (the `Failed` handshake half).
    pub fn set_ready_state(&mut self, ready: ReadyState) {
        self.ready = ready;
    }

    /// The app-under-test, for observing what delivery produced.
    #[must_use]
    pub fn app(&self) -> &A {
        &self.app
    }

    /// The underlying fake driver (inspection beyond `windows()`).
    #[must_use]
    pub fn host(&self) -> &FakeDriver {
        &self.host
    }

    /// Mutable access to the fake driver.
    pub fn host_mut(&mut self) -> &mut FakeDriver {
        &mut self.host
    }

    /// The observation journal.
    #[must_use]
    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// Whether shutdown has been requested.
    #[must_use]
    pub fn exited(&self) -> bool {
        self.exited
    }

    fn push_notice(&mut self, kind: &str, window: Option<WindowId>) {
        let seq = self.notice_seq;
        self.notice_seq += 1;
        self.notices.push(Notice {
            seq,
            kind: kind.to_string(),
            window: window.map(WindowId::get),
            detail: Value::Null,
        });
    }

    /// Pumps the fake queue through journal + app — every emitted
    /// event records `In` receipts, lifecycle notices, and drives
    /// `TestApp::on_event` under a `JournalHandle`. The interceptor
    /// is the same delivery shape a real adapter uses: journal in,
    /// lifecycle bookkeeping, then app delivery.
    fn pump_events(&mut self) {
        let Self {
            host,
            app,
            journal,
            notices,
            notice_seq,
            ..
        } = self;
        let suppressed = host.pump(&mut |dev, drv| {
            journal.record_in(&dev);
            if let DriverEvent::Windowed { window, event } = &dev {
                let kind = match event {
                    WindowLifecycle::Created => Some("window.created"),
                    WindowLifecycle::CloseRequested => Some("window.close_requested"),
                    WindowLifecycle::Destroyed => Some("window.destroyed"),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let seq = *notice_seq;
                    *notice_seq += 1;
                    notices.push(Notice {
                        seq,
                        kind: kind.to_string(),
                        window: Some(window.get()),
                        detail: Value::Null,
                    });
                }
            }
            let mut jh = JournalHandle::new(drv, journal);
            app.on_event(dev, &mut jh);
        });
        for (window, intent) in suppressed {
            let bytes = serde_json::to_string(&intent).map_or(0, |s| s.len());
            self.journal.record_suppressed(window, bytes);
            self.push_notice("ipc.suppressed", Some(window));
        }
    }
}

impl<A: TestApp> TestIo for HeadlessIo<A> {
    fn ready_state(&self) -> ReadyState {
        self.ready.clone()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            fixture: !self.fixtures.is_empty(),
            clock_domains: vec![ClockDomain::HostTick.as_str().to_string()],
            faults: vec![
                FaultKind::WindowCrash.as_str().to_string(),
                FaultKind::IpcDisconnect.as_str().to_string(),
            ],
            observation: vec![
                "windows".to_string(),
                "receipts".to_string(),
                "notices".to_string(),
            ],
            screenshot: false,
            denied: DeniedCapabilities::default(),
        }
    }

    fn backend(&self) -> BackendMeta {
        BackendMeta {
            name: "headless-fake".to_string(),
            detail: None,
        }
    }

    fn emit(&mut self, ev: DriverEvent) {
        self.host.inject(ev);
        self.pump_events();
    }

    fn emit_ipc_body(&mut self, window: WindowId, body: &str) {
        // Dead-window drop — the same silent-drop contract the driver
        // applies to windowed events for retired ids.
        if !self.host.windows().contains(&window) {
            return;
        }
        if self.host.is_ipc_down(window) {
            self.journal.record_suppressed(window, body.len());
            self.push_notice("ipc.suppressed", Some(window));
            return;
        }
        match parse_intent(body) {
            Ok(intent) => self.emit(DriverEvent::Windowed {
                window,
                event: WindowLifecycle::Action(intent),
            }),
            Err(e) => {
                // The raw body crossed the inbound boundary; the
                // driver-side answer is an error event posted to the
                // window — the same mapping real adapters apply to
                // malformed inbound bodies.
                self.journal
                    .record(Direction::In, Some(window), "message", body.len());
                self.journal
                    .record(Direction::Out, Some(window), "post_event", 0);
                self.host.post_event(window, &Event::error(e));
            }
        }
    }

    fn windows(&self) -> Vec<WindowMeta> {
        self.host
            .windows()
            .into_iter()
            .map(|id| {
                let spec = self.host.spec_of(id);
                WindowMeta {
                    id: id.get(),
                    title: spec.map(|s| s.title.clone()),
                    url: None,
                    width: spec.map_or(0, |s| s.logical_size.width),
                    height: spec.map_or(0, |s| s.logical_size.height),
                    ready: self.host.ready(id),
                    close_pending: self.host.close_pending(id),
                    ipc_down: self.host.is_ipc_down(id),
                }
            })
            .collect()
    }

    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, ErrorBody> {
        let id = self
            .host
            .create_window(spec)
            .map_err(|e| ErrorBody::new(codes::SERVICE_ERROR, e.to_string()))?;
        // Deliver the queued `Created` deterministically — the
        // `window.created` notice and `In "created"` receipt are
        // observable at this call's return.
        self.pump_events();
        Ok(id)
    }

    fn receipts(&self, since: u64, limit: usize) -> ReceiptPage {
        self.journal.entries_since(since, limit)
    }

    fn notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    fn set_fault(
        &mut self,
        kind: FaultKind,
        window: Option<WindowId>,
        on: bool,
    ) -> Result<(), ErrorBody> {
        let Some(id) = window else {
            return Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                "fault requires a \"window\" param",
            ));
        };
        if !self.host.windows().contains(&id) {
            return Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                format!("unknown window {id}"),
            ));
        }
        match kind {
            FaultKind::WindowCrash => {
                if !on {
                    return Err(ErrorBody::new(
                        codes::INVALID_PARAMS,
                        "window_crash cannot be cleared",
                    ));
                }
                self.host
                    .destroy_window(id)
                    .map_err(|e| ErrorBody::new(codes::SERVICE_ERROR, e.to_string()))?;
                self.pump_events();
                Ok(())
            }
            FaultKind::IpcDisconnect => {
                if on {
                    self.host.ipc_disconnect(id);
                } else {
                    self.host.ipc_reconnect(id);
                }
                Ok(())
            }
        }
    }

    fn prepare_fixture(&mut self, spec: &FixtureSpec) -> Result<FixtureReceipt, ErrorBody> {
        let Some(pos) = self.fixtures.iter().position(|f| f.name() == spec.name) else {
            return Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                format!("unknown fixture {:?}", spec.name),
            ));
        };
        // Take the controller out so `prepare` can call back into
        // `self` (`io.create_window`, `io.emit`, …), then reinsert.
        let mut ctl = self.fixtures.remove(pos);
        let result = ctl.prepare(spec, self);
        self.fixtures.insert(pos, ctl);
        result
    }

    fn screenshot(&mut self, _window: WindowId) -> Result<Screenshot, ErrorBody> {
        Err(ErrorBody::new(
            codes::UNAVAILABLE,
            "screenshot capture is not implemented on the headless backend",
        ))
    }

    fn request_shutdown(&mut self, mode: ShutdownMode) -> CleanupReport {
        let windows_open = self.host.windows().len();
        self.push_notice("app.closing", None);
        let mut windows_closed = 0usize;
        let mut app_detail = Value::Null;
        if matches!(mode, ShutdownMode::Graceful) {
            {
                let mut jh = JournalHandle::new(&mut self.host, &mut self.journal);
                app_detail = self.app.shutdown(mode, &mut jh);
            }
            for id in self.host.windows() {
                if self.host.destroy_window(id).is_ok() {
                    windows_closed += 1;
                }
            }
            self.pump_events();
        }
        self.host.exit();
        self.exited = true;
        CleanupReport {
            windows_open,
            windows_closed,
            exited: self.host.exited(),
            app: app_detail,
        }
    }
}
