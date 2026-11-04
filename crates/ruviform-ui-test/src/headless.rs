//! `HeadlessIo` — the [`TestIo`] implementation over [`FakeHost`]
//! (Issue #312).
//!
//! Composition: `FakeHost` (window/event plumbing) + a `TestApp`
//! (the consumer's event wiring) + `Journal` (observation) + named
//! `FixtureController`s. `emit` injects host events through the fake
//! queue and pumps them deterministically; every pump records inbound
//! receipts, app→host calls pass through `JournalHandle`, and window
//! lifecycle transitions produce notices.
//!
//! **RT contract:** non-RT only.

use ruviform_ui::{ErrorBody, HostEvent, WindowConfig, WindowEvent, WindowHandle, WindowId, codes};
use serde_json::Value;

use crate::fake::{FakeHost, FakeWindow};
use crate::fixture::{FixtureController, TestApp};
use crate::io::TestIo;
use crate::journal::{Journal, JournalHandle};
use crate::proto::{
    BackendMeta, Capabilities, CleanupReport, ClockDomain, DeniedCapabilities, FaultKind,
    FixtureReceipt, FixtureSpec, Notice, ReadyState, ReceiptPage, Screenshot, ShutdownMode,
    WindowMeta,
};

/// `TestIo` backed by [`FakeHost`] — the headless path every CI lane
/// can drive end to end.
pub struct HeadlessIo<A: TestApp> {
    host: FakeHost,
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
            host: FakeHost::new(),
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

    /// The underlying fake host (inspection beyond `windows()`).
    #[must_use]
    pub fn host(&self) -> &FakeHost {
        &self.host
    }

    /// Mutable access to the fake host.
    pub fn host_mut(&mut self) -> &mut FakeHost {
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
    /// `TestApp::on_event` under a `JournalHandle`.
    fn pump_events(&mut self) {
        let Self {
            host,
            app,
            journal,
            notices,
            notice_seq,
            ..
        } = self;
        let suppressed = host.pump(&mut |wev, wh| {
            journal.record_in(&wev);
            if let WindowEvent::Windowed { window, event } = &wev {
                match event {
                    HostEvent::Destroyed => {
                        let seq = *notice_seq;
                        *notice_seq += 1;
                        notices.push(Notice {
                            seq,
                            kind: "window.destroyed".to_string(),
                            window: Some(window.get()),
                            detail: Value::Null,
                        });
                    }
                    HostEvent::CloseRequested => {
                        let seq = *notice_seq;
                        *notice_seq += 1;
                        notices.push(Notice {
                            seq,
                            kind: "window.close_requested".to_string(),
                            window: Some(window.get()),
                            detail: Value::Null,
                        });
                    }
                    _ => {}
                }
            }
            let mut jh = JournalHandle::new(wh, journal);
            app.on_event(wev, &mut jh);
        });
        for (window, event) in suppressed {
            self.journal.record_suppressed(window, &event);
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

    fn emit(&mut self, ev: WindowEvent) {
        self.host.inject(ev);
        self.pump_events();
    }

    fn windows(&self) -> Vec<WindowMeta> {
        self.host
            .windows()
            .into_iter()
            .filter_map(|id| {
                self.host
                    .windows
                    .get(&id)
                    .map(|fw: &FakeWindow| WindowMeta {
                        id: id.get(),
                        title: Some(fw.config.title.clone()),
                        url: Some(fw.config.url.clone()),
                        width: fw.config.width,
                        height: fw.config.height,
                        ready: fw.ready,
                        close_pending: fw.close_pending,
                        ipc_down: self.host.is_ipc_down(id),
                    })
            })
            .collect()
    }

    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, ErrorBody> {
        self.host
            .create_window(config)
            .inspect(|&id| self.push_notice("window.created", Some(id)))
            .map_err(|e| ErrorBody::new(codes::SERVICE_ERROR, e.to_string()))
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
        if !self.host.windows.contains_key(&id) {
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
