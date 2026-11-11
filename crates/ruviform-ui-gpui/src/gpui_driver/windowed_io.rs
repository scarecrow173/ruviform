//! `WindowedIo` — the [`TestIo`] implementation over a real
//! [`GpuiDriver`] event loop (Issue #313 slice S2, RF-GT-04).
//!
//! `GpuiDriver` is `Rc<RefCell<Inner>>`-backed and therefore not
//! `Send`: the `TestAgent` serve loop on the control thread cannot
//! touch it. This adapter bridges the two threads through the
//! driver's [`UiJob`] seam — control-side ops enqueue `Send` jobs
//! that `drain_ui_jobs` executes inside `App` context, and replies
//! travel back over per-call oneshot channels.
//!
//! Delivery parity with [`ruviform_ui_test::HeadlessIo`] is the
//! contract: every event the app observes — real OS lifecycle, real
//! `Action` ingress, or `emit`-injected — flows through the same
//! interceptor (`UiSide::deliver`): journal `In` receipt → lifecycle
//! notice → `ipc_disconnect` suppression → `TestApp::on_event` under
//! a [`JournalHandle`]. Mutating ops use an **action-job +
//! confirm-job** pair: the action queues driver work, and the
//! confirm — necessarily collected at a later `drain` because the
//! action's reply is sent only during job execution — runs after
//! that drain's op phase applied the queued [`Op`] and calls
//! `flush_events` so derived `DriverEvent` delivery, journal
//! recording, and notices are complete when the caller resumes.
//!
//! `emit` is honest per event: `Tick`/`Action`/`Focused` carry no
//! driver-side bookkeeping and are injected through `inner.events`
//! (the same channel `IntentSink` uses); `CloseRequested` maps to
//! the real `request_close` op so veto/defer/default-approve
//! bookkeeping is exercised; `Created`/`Ready`/`Destroyed`/
//! `Resized`/`Moved`/`ScaleFactorChanged` are dropped with an
//! `emit.unsupported` notice — fabricating them would desync slot
//! state (ready gate, retire ledger, size/scale tracking) from what
//! the app observed.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use base64::Engine as _;
use gpui::App;
use ruviform_ui::ipc::{MAX_INTENT_BYTES, parse_intent};
use ruviform_ui::{
    DriverEvent, ErrorBody, Event, Intent, LogicalSize, UiError, WindowDriver, WindowId,
    WindowLifecycle, WindowSpec, codes,
};
use ruviform_ui_test::proto::Direction;
use ruviform_ui_test::{
    BackendMeta, Capabilities, CleanupReport, ClockDomain, DeniedCapabilities, FaultKind,
    FixtureController, FixtureReceipt, FixtureSpec, Journal, JournalHandle, Notice, ReadyState,
    ReceiptPage, Screenshot, ShutdownMode, TestApp, TestIo, WindowMeta,
};
use serde_json::Value;

use super::{GpuiDriver, Inner, UiJob, flush_events};

/// State shared between the control thread and the UI thread —
/// observation (journal/notices), the `ipc_disconnect` fault set,
/// spec metadata recorded at `create_window`, and the pending
/// shutdown the interceptor consumes inside delivery.
#[derive(Default)]
struct Shared {
    journal: Journal,
    notices: Vec<Notice>,
    notice_seq: u64,
    /// Per-window `ipc_disconnect` fault — `Action` events (real or
    /// injected) for armed windows are suppressed in `deliver`, and
    /// `emit_ipc_body` drops pre-decode. Per-window only: the
    /// protocol's `window = None` global arm stays `invalid_params`.
    ipc_down: BTreeSet<WindowId>,
    /// `WindowSpec` metadata recorded by `create_window` — the
    /// driver's slots do not retain the spec.
    specs: BTreeMap<WindowId, (String, LogicalSize)>,
    /// Armed by `request_shutdown`; consumed by `deliver` — the app
    /// shutdown hook is non-`Send` app state and can only run on the
    /// UI thread.
    pending_shutdown: Option<ShutdownMode>,
    /// `CleanupReport::app` payload produced by the shutdown hook.
    shutdown_detail: Value,
}

impl Shared {
    fn push_notice(&mut self, kind: &str, window: Option<WindowId>) {
        self.push_notice_detail(kind, window, Value::Null);
    }

    fn push_notice_detail(&mut self, kind: &str, window: Option<WindowId>, detail: Value) {
        let seq = self.notice_seq;
        self.notice_seq += 1;
        self.notices.push(Notice {
            seq,
            kind: kind.to_string(),
            window: window.map(WindowId::get),
            detail,
        });
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The control-thread [`TestIo`] — `rvf.uitest.v1` over real GPUI
/// windows. All backend interaction crosses to the UI thread as
/// [`UiJob`]s; journal/notices/fault bookkeeping live in `shared`
/// and are readable without a round-trip.
pub struct WindowedIo {
    jobs: Sender<UiJob>,
    shared: Arc<Mutex<Shared>>,
    fixtures: Vec<Box<dyn FixtureController>>,
    ready: ReadyState,
    exited: bool,
}

/// The UI-thread half: owns the [`UiJob`] inbox and is consumed by
/// [`UiSide::run`] on the app thread — where the (non-`Send`) app is
/// attached. Carries only `Send` state so the AUT may move it across
/// `thread::spawn`.
pub struct UiSide {
    jobs_rx: Receiver<UiJob>,
    shared: Arc<Mutex<Shared>>,
}

/// The interceptor installed as `GpuiDriver::run`'s event handler —
/// `deliver` is the single delivery path every `DriverEvent` takes.
struct UiDeliver<A> {
    app: A,
    shared: Arc<Mutex<Shared>>,
}

/// What `emit_ipc_body` decided for one raw inbound body — resolved
/// under the `shared` lock before delivery work runs.
enum IpcOutcome {
    /// `ipc_disconnect` armed — suppressed pre-decode.
    Suppressed,
    /// Decoded intent — delivered as `Action` through the event path.
    Deliver(Intent),
    /// Undecodable — answered driver-side with `Event::error`.
    Malformed(ErrorBody),
}

impl WindowedIo {
    /// Upper bound for one UI-job reply — a live-but-stalled app
    /// loop surfaces a typed error, never a hang.
    const JOB_TIMEOUT: Duration = Duration::from_secs(15);

    /// Creates the control/UI pair. The returned [`UiSide`] moves to
    /// the app thread; this side serves `rvf.uitest.v1` wherever the
    /// agent runs.
    #[must_use]
    pub fn new() -> (Self, UiSide) {
        let (jobs, jobs_rx) = channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        (
            Self {
                jobs,
                shared: Arc::clone(&shared),
                fixtures: Vec::new(),
                ready: ReadyState::Ready,
                exited: false,
            },
            UiSide { jobs_rx, shared },
        )
    }

    /// Registers a fixture reachable via `test.fixture.prepare`.
    pub fn add_fixture(&mut self, fixture: Box<dyn FixtureController>) {
        self.fixtures.push(fixture);
    }

    /// Overrides the readiness `test.hello` observes — the way an app
    /// reports a failed initialization.
    pub fn set_ready_state(&mut self, ready: ReadyState) {
        self.ready = ready;
    }

    /// Whether shutdown has been requested.
    #[must_use]
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Sends `f` as a oneshot-reply [`UiJob`] and waits for it. The
    /// reply is sent from inside the job's execution — after the
    /// drain's collection phase — so any job enqueued *after* this
    /// call's return is collected no earlier than the next drain
    /// pass (the ordering the confirm pattern relies on).
    fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Rc<RefCell<Inner>>, &GpuiDriver, &mut App) -> T + Send + 'static,
    ) -> Result<T, ErrorBody> {
        let (tx, rx) = channel();
        self.jobs
            .send(Box::new(move |inner, driver, cx| {
                let _ = tx.send(f(inner, driver, cx));
            }))
            .map_err(|_| {
                ErrorBody::new(
                    codes::SERVICE_ERROR,
                    "ui job channel closed — the GPUI loop is not running",
                )
            })?;
        rx.recv_timeout(Self::JOB_TIMEOUT).map_err(|_| {
            ErrorBody::new(
                codes::SERVICE_ERROR,
                "ui job timed out — the GPUI loop is not draining jobs",
            )
        })
    }

    /// Action-job + confirm-job for every mutating op: `f` queues
    /// driver work inside the job context; the confirm job lands in
    /// a later `drain` (see [`Self::call`]), runs after that drain's
    /// op phase applied the queued op, and calls `flush_events` so
    /// derived event delivery, journal recording, and notices are
    /// complete when this returns.
    fn act<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Rc<RefCell<Inner>>, &GpuiDriver, &mut App) -> T + Send + 'static,
    ) -> Result<T, ErrorBody> {
        let value = self.call(f)?;
        self.call(|inner, driver, _cx| flush_events(inner, driver))?;
        Ok(value)
    }

    /// Injects `ev` into the driver event queue and delivers it
    /// through the same interceptor path real events take, then
    /// confirms across one drain so app→driver replies (`post_event`
    /// ops) are applied too. Dead-window targets drop silently — the
    /// same contract the driver applies to windowed events for
    /// retired ids.
    fn inject_and_confirm(&mut self, ev: DriverEvent) -> Result<(), ErrorBody> {
        self.act(move |inner, driver, _cx| {
            let deliver = match &ev {
                DriverEvent::Windowed { window, .. } => inner.borrow().windows.contains_key(window),
                _ => true,
            };
            if deliver {
                inner.borrow_mut().events.push_back(ev);
                flush_events(inner, driver);
            }
        })
    }

    /// Observably drops an `emit` payload the driver cannot honestly
    /// fabricate — a notice, never a silent drop or a journaled
    /// receipt (nothing crossed the boundary).
    fn drop_emit(&mut self, window: Option<WindowId>, event: &WindowLifecycle) {
        lock(&self.shared).push_notice_detail(
            "emit.unsupported",
            window,
            serde_json::json!({ "event": format!("{event:?}") }),
        );
    }
}

impl UiSide {
    /// Installs the job inbox on `driver`, then runs the GPUI event
    /// loop delivering every [`DriverEvent`] through the interceptor
    /// to `app`. Call on the app thread; blocks until the driver
    /// exits.
    ///
    /// # Errors
    /// [`UiError::BackendUnavailable`] when the driver loop was
    /// already started (see [`GpuiDriver::run`]).
    pub fn run<A: TestApp + 'static>(
        self,
        driver: &mut GpuiDriver,
        tick: Option<Duration>,
        app: A,
    ) -> Result<(), UiError> {
        driver.set_ui_jobs(self.jobs_rx);
        let mut deliver = UiDeliver {
            app,
            shared: self.shared,
        };
        driver.run(tick, move |dev, drv| deliver.event(dev, drv))
    }
}

impl<A: TestApp> UiDeliver<A> {
    /// The single delivery path — the windowed counterpart of
    /// `HeadlessIo::pump_events`'s interceptor: `In` receipt →
    /// lifecycle notice → `ipc_disconnect` suppression → app
    /// delivery under [`JournalHandle`], then the pending-shutdown
    /// check that keeps the non-`Send` app hook on this thread.
    fn event(&mut self, dev: DriverEvent, drv: &mut dyn WindowDriver) {
        let mut shared = lock(&self.shared);
        shared.journal.record_in(&dev);
        if let DriverEvent::Windowed { window, event } = &dev {
            let kind = match event {
                WindowLifecycle::Created => Some("window.created"),
                WindowLifecycle::CloseRequested => Some("window.close_requested"),
                WindowLifecycle::Destroyed => Some("window.destroyed"),
                _ => None,
            };
            if let Some(kind) = kind {
                shared.push_notice(kind, Some(*window));
            }
            if let WindowLifecycle::Action(intent) = event
                && shared.ipc_down.contains(window)
            {
                let bytes = serde_json::to_string(intent).map_or(0, |s| s.len());
                shared.journal.record_suppressed(*window, bytes);
                shared.push_notice("ipc.suppressed", Some(*window));
                return;
            }
        }
        {
            let mut jh = JournalHandle::new(&mut *drv, &mut shared.journal);
            self.app.on_event(dev, &mut jh);
        }
        // A pending shutdown rides the same delivery path — only
        // here does the app hook exist.
        if let Some(mode) = shared.pending_shutdown.take() {
            {
                let mut jh = JournalHandle::new(&mut *drv, &mut shared.journal);
                if matches!(mode, ShutdownMode::Graceful) {
                    shared.shutdown_detail = self.app.shutdown(mode, &mut jh);
                }
            }
            if matches!(mode, ShutdownMode::Graceful) {
                for id in drv.windows() {
                    let _ = drv.destroy_window(id);
                }
            }
            drv.exit();
        }
    }
}

impl Default for WindowedIo {
    fn default() -> Self {
        Self::new().0
    }
}

impl TestIo for WindowedIo {
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
            // Real OS capture exists only where the capture backend
            // does — honest per-OS claim.
            screenshot: cfg!(any(windows, target_os = "linux")),
            denied: DeniedCapabilities::default(),
        }
    }

    fn backend(&self) -> BackendMeta {
        BackendMeta {
            name: "gpui-windowed".to_string(),
            detail: Some(format!("gpui/{}", std::env::consts::OS)),
        }
    }

    fn emit(&mut self, ev: DriverEvent) {
        match &ev {
            DriverEvent::Tick => {
                let _ = self.inject_and_confirm(ev);
            }
            DriverEvent::Windowed { window, event } => match event {
                WindowLifecycle::Action(_) | WindowLifecycle::Focused(_) => {
                    let _ = self.inject_and_confirm(ev);
                }
                WindowLifecycle::CloseRequested => {
                    let window = *window;
                    // The real op — the veto-able request, pending
                    // bookkeeping, and unanswered default-approve all
                    // live in `Op::RequestClose`.
                    let _ = self.act(move |_inner, driver, _cx| {
                        driver.clone().request_close(window);
                    });
                }
                _ => self.drop_emit(Some(*window), event),
            },
            _ => {
                // Unknown future variants — observable drop, never
                // fabricated delivery.
                lock(&self.shared).push_notice("emit.unsupported", None);
            }
        }
    }

    fn emit_ipc_body(&mut self, window: WindowId, body: &str) {
        let shared = Arc::clone(&self.shared);
        let body = body.to_string();
        let _ = self.act(move |inner, driver, _cx| {
            if !inner.borrow().windows.contains_key(&window) {
                // Dead-window drop — the same silent-drop contract the
                // driver applies to windowed events for retired ids.
                return;
            }
            let outcome = {
                // `shared` must be released before `flush_events` —
                // the handler locks it during delivery.
                let mut shared = lock(&shared);
                if shared.ipc_down.contains(&window) {
                    shared.journal.record_suppressed(window, body.len());
                    shared.push_notice("ipc.suppressed", Some(window));
                    IpcOutcome::Suppressed
                } else {
                    match parse_intent(&body) {
                        Ok(intent) => IpcOutcome::Deliver(intent),
                        Err(e) => {
                            shared.journal.record(
                                Direction::In,
                                Some(window),
                                "message",
                                body.len(),
                            );
                            shared
                                .journal
                                .record(Direction::Out, Some(window), "post_event", 0);
                            IpcOutcome::Malformed(e)
                        }
                    }
                }
            };
            match outcome {
                IpcOutcome::Suppressed => {}
                IpcOutcome::Deliver(intent) => {
                    inner.borrow_mut().events.push_back(DriverEvent::Windowed {
                        window,
                        event: WindowLifecycle::Action(intent),
                    });
                    flush_events(inner, driver);
                }
                IpcOutcome::Malformed(e) => {
                    // The raw body crossed the inbound boundary; the
                    // driver-side answer is an error event to the
                    // window — the same mapping real adapters apply.
                    driver.clone().post_event(window, &Event::error(e));
                }
            }
        });
    }

    fn windows(&self) -> Vec<WindowMeta> {
        let slots = self
            .call(|inner, _driver, _cx| {
                inner
                    .borrow()
                    .windows
                    .iter()
                    .map(|(id, slot)| (*id, slot.ready, slot.close.pending))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let shared = lock(&self.shared);
        slots
            .into_iter()
            .map(|(id, ready, close_pending)| {
                let spec = shared.specs.get(&id);
                WindowMeta {
                    id: id.get(),
                    title: spec.map(|(title, _)| title.clone()),
                    // No URL concept on this backend — honest `None`.
                    url: None,
                    width: spec.map_or(0, |(_, size)| size.width),
                    height: spec.map_or(0, |(_, size)| size.height),
                    ready,
                    close_pending,
                    ipc_down: shared.ipc_down.contains(&id),
                }
            })
            .collect()
    }

    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, ErrorBody> {
        let owned = spec.clone();
        let id = self
            .act(move |_inner, driver, _cx| driver.clone().create_window(&owned))
            .and_then(|r| r.map_err(|e| ErrorBody::new(codes::SERVICE_ERROR, e.to_string())))?;
        lock(&self.shared)
            .specs
            .insert(id, (spec.title.clone(), spec.logical_size));
        Ok(id)
    }

    fn receipts(&self, since: u64, limit: usize) -> ReceiptPage {
        lock(&self.shared).journal.entries_since(since, limit)
    }

    fn notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut lock(&self.shared).notices)
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
        let known = self
            .call(move |inner, _driver, _cx| inner.borrow().windows.contains_key(&id))
            .unwrap_or(false);
        if !known {
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
                self.act(move |_inner, driver, _cx| driver.clone().destroy_window(id))
                    .and_then(|r| {
                        r.map_err(|e| ErrorBody::new(codes::SERVICE_ERROR, e.to_string()))
                    })
            }
            FaultKind::IpcDisconnect => {
                let mut shared = lock(&self.shared);
                if on {
                    shared.ipc_down.insert(id);
                } else {
                    shared.ipc_down.remove(&id);
                }
                Ok(())
            }
            // `FaultKind` is non-exhaustive: unknown kinds fail closed.
            _ => Err(ErrorBody::new(
                codes::INVALID_PARAMS,
                format!("unsupported fault kind {kind:?}"),
            )),
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

    fn screenshot(&mut self, window: WindowId) -> Result<Screenshot, ErrorBody> {
        let frame = self
            .call(move |_inner, driver, _cx| driver.capture_window(window))
            .and_then(|r| {
                r.map_err(|e| match e {
                    UiError::Unsupported { .. } => {
                        ErrorBody::new(codes::UNAVAILABLE, e.to_string())
                    }
                    UiError::Host { ref message } if message.starts_with("no window") => {
                        ErrorBody::new(codes::INVALID_PARAMS, e.to_string())
                    }
                    UiError::Host { ref message } if message.contains("no OS window yet") => {
                        ErrorBody::new(codes::UNAVAILABLE, e.to_string())
                    }
                    other => ErrorBody::new(codes::SERVICE_ERROR, other.to_string()),
                })
            })?;
        let data_base64 = base64::engine::general_purpose::STANDARD.encode(&frame.pixels);
        // The framed transport caps replies at `MAX_INTENT_BYTES`;
        // a large window's RGBA payload would fail the reply write
        // and end the session (serve's write-fail → `DriverGone`).
        // Fail this op honestly instead of killing the channel.
        if data_base64.len() + 512 > MAX_INTENT_BYTES {
            return Err(ErrorBody::new(
                codes::SERVICE_ERROR,
                format!(
                    "screenshot of {}x{} window exceeds the {MAX_INTENT_BYTES}-byte reply cap",
                    frame.width, frame.height,
                ),
            ));
        }
        Ok(Screenshot {
            format: "rgba-raw".to_string(),
            data_base64,
            width: frame.width,
            height: frame.height,
        })
    }

    fn request_shutdown(&mut self, mode: ShutdownMode) -> CleanupReport {
        let windows_open = self
            .call(|inner, _driver, _cx| inner.borrow().windows.len())
            .unwrap_or(0);
        {
            let mut shared = lock(&self.shared);
            shared.push_notice("app.closing", None);
            shared.pending_shutdown = Some(mode);
        }
        // Wake the delivery path — `pending_shutdown` is consumed by
        // the interceptor on the app thread, where the non-`Send`
        // app hook can run.
        let _ = self.call(|inner, driver, _cx| {
            inner.borrow_mut().events.push_back(DriverEvent::Tick);
            flush_events(inner, driver);
        });
        // Confirm at a later drain: the queued destroy/exit ops were
        // applied in that pass's op phase and `sweep_destroyed`
        // already retired dead slots at the drain's top.
        let shared = Arc::clone(&self.shared);
        let report = self
            .call(move |inner, driver, _cx| {
                flush_events(inner, driver);
                let live = inner.borrow().windows.len();
                let shared = lock(&shared);
                CleanupReport {
                    windows_open,
                    windows_closed: if matches!(mode, ShutdownMode::Graceful) {
                        windows_open.saturating_sub(live)
                    } else {
                        0
                    },
                    exited: driver.exited(),
                    app: shared.shutdown_detail.clone(),
                }
            })
            .unwrap_or_else(|_| CleanupReport {
                windows_open,
                windows_closed: 0,
                exited: false,
                app: Value::Null,
            });
        self.exited = true;
        report
    }
}
