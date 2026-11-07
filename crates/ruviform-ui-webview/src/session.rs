//! Session host — one tao event loop managing many windows/webviews
//! behind the driver-neutral [`WindowDriver`] port (Issues #181, #423).
//!
//! - **Registry**: every window/webview pair gets a stable [`WindowId`]
//!   issued at create; backend callbacks carry it into a shared queue so
//!   every [`DriverEvent::Windowed`] knows its origin.
//! - **Shared `WebContext`**: all webviews share one context/user-data
//!   dir (`WebView2`'s UDF is a process singleton lock — `poc/ui`
//!   measured stale browser children blocking the next init). The
//!   context lives for the whole session and drops after the loop.
//! - **Close flow**: `CloseRequested` marks the window close-pending and
//!   emits — it never destroys and never exits the loop. The consumer
//!   answers via [`WindowDriver::respond_close`]; an unanswered request
//!   defaults to `Approve` for that window only. `Destroyed` follows any
//!   teardown (approved close, programmatic destroy, backend loss).
//! - **Delivery safety**: `post_event` to an unknown or destroyed window
//!   is a silent drop; a not-yet-ready webview buffers a bounded backlog
//!   (drop-oldest) flushed on `Ready`.
//! - **App-exit policy**: the loop only exits via
//!   [`WindowDriver::exit`] — last-window semantics are the consumer's.
//! - **Adapter-private mapping**: wry callbacks are decoded in
//!   [`crate::host`]; the legacy `HostEvent`/`HostHandle`/`WindowConfig`
//!   surface is not used here (migration table H1–H5,
//!   docs/designs/issue-386-driver-neutral-gui-host.md).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ruviform_ui::{
    AssetSource, Capability, CloseDecision, DataRegistry, DriverEvent, Event, LogicalSize,
    MonitorInfo, UiError, WindowCapabilities, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
    dispatch_script,
};
use tao::event::{Event as TaoEvent, StartCause, WindowEvent as TaoWindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::{WebContext, WebView};

use crate::host::{Inbound, bounds_for, build_webview, full_bounds};

/// Bounded script backlog per not-yet-ready window (drop-oldest).
const MAX_PENDING_SCRIPTS: usize = 16;

/// Per-window creation parameters: the driver-neutral [`WindowSpec`]
/// plus the WebView-only page and devtools settings (table H5).
///
/// Consumers construct this — it is deliberately not
/// `#[non_exhaustive]`.
#[derive(Debug, Clone)]
pub struct WebViewWindowConfig {
    /// Driver-neutral window parameters (title, size, position, resize).
    pub spec: WindowSpec,
    /// Initial URL — normally inside the `rvf://app` namespace.
    pub url: String,
    /// Enable backend devtools for this window's webview.
    pub devtools: bool,
}

impl Default for WebViewWindowConfig {
    fn default() -> Self {
        Self {
            spec: WindowSpec::default(),
            url: "rvf://app/index.html".to_string(),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Session-level configuration for [`run_multi`].
///
/// Consumers construct this — it is deliberately not
/// `#[non_exhaustive]`.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// `WebView` user-data folder shared by every window in the
    /// session (`WebView2`'s UDF is a singleton lock — separate
    /// contexts on the same folder would collide). `None` selects a
    /// process-unique temp dir.
    pub data_dir: Option<PathBuf>,
    /// Periodic [`DriverEvent::Tick`] cadence (job-drain wakeup).
    /// `None` = never.
    pub tick: Option<Duration>,
    /// Windows to create before the loop starts. May be empty —
    /// consumers can create windows later via
    /// [`WindowDriver::create_window`]; the loop still runs until
    /// [`WindowDriver::exit`].
    pub windows: Vec<WebViewWindowConfig>,
    /// Page for windows created through [`WindowDriver::create_window`],
    /// which carries only a [`WindowSpec`] and therefore no URL.
    pub default_url: String,
    /// Devtools flag for windows created through
    /// [`WindowDriver::create_window`].
    pub devtools: bool,
}

/// One managed window/webview pair plus delivery/close state.
struct Entry {
    window: tao::window::Window,
    webview: WebView,
    ready: bool,
    backlog: VecDeque<String>,
    /// A `CloseRequested` is open for this window (veto-able until
    /// destroyed).
    close_pending: bool,
    /// The most recently emitted `CloseRequested` has not been
    /// answered yet — used to apply the default `Approve` once the
    /// emit returns.
    awaiting_response: bool,
}

/// Mutable session state moved into the event loop closure.
struct Session {
    windows: HashMap<WindowId, Entry>,
    /// tao window id → contract [`WindowId`].
    by_tao: HashMap<tao::window::WindowId, WindowId>,
    next_id: u64,
    /// Backend callbacks + host pushes share this queue:
    /// `(window, inbound)` pairs, drained on `Event::UserEvent`.
    queue: Rc<RefCell<VecDeque<(WindowId, Inbound)>>>,
    proxy: EventLoopProxy<()>,
    web_context: WebContext,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    default_url: String,
    devtools: bool,
    want_exit: Cell<bool>,
}

impl Session {
    /// Issues the next [`WindowId`] (nonzero, monotonic, never reused).
    fn issue_id(&mut self) -> WindowId {
        let id = WindowId::new(self.next_id).expect("counter nonzero");
        self.next_id += 1;
        id
    }

    /// Creates a window + webview on `target` and registers it.
    fn create_entry(
        &mut self,
        target: &EventLoopWindowTarget<()>,
        spec: &WindowSpec,
        url: &str,
        devtools: bool,
    ) -> Result<WindowId, UiError> {
        let mut attrs = WindowBuilder::new()
            .with_title(&spec.title)
            .with_inner_size(dpi::LogicalSize::new(
                spec.logical_size.width,
                spec.logical_size.height,
            ))
            .with_resizable(spec.resizable);
        if let Some(p) = spec.position {
            attrs = attrs.with_position(dpi::LogicalPosition::new(p.x, p.y));
        }
        let window = attrs
            .build(target)
            .map_err(|e| UiError::host(format!("window creation failed: {e}")))?;

        let id = self.issue_id();
        // The webview's backend callbacks stamp every queued item with
        // this window's id — IPC bodies stay opaque until decoded.
        let enqueue = {
            let queue = self.queue.clone();
            let proxy = self.proxy.clone();
            move |inbound: Inbound| {
                queue.borrow_mut().push_back((id, inbound));
                let _ = proxy.send_event(());
            }
        };
        // A failed webview build must not leave a live window behind:
        // `window` drops with the early return.
        let webview = build_webview(
            &window,
            &mut self.web_context,
            url,
            devtools,
            self.assets.clone(),
            self.data.clone(),
            enqueue,
        )?;

        self.by_tao.insert(window.id(), id);
        self.windows.insert(
            id,
            Entry {
                window,
                webview,
                ready: false,
                backlog: VecDeque::new(),
                close_pending: false,
                awaiting_response: false,
            },
        );
        self.queue
            .borrow_mut()
            .push_back((id, Ok(WindowLifecycle::Created)));
        let _ = self.proxy.send_event(());
        Ok(id)
    }

    /// Destroys a window: drops webview then window, retires the id,
    /// and queues the terminal `Destroyed` event.
    fn destroy_entry(&mut self, id: WindowId) -> Result<(), UiError> {
        let Some(entry) = self.windows.remove(&id) else {
            return Err(UiError::host(format!("no window {id}")));
        };
        self.by_tao.remove(&entry.window.id());
        // Drop order: webview before its window.
        let Entry {
            window, webview, ..
        } = entry;
        drop(webview);
        drop(window);
        self.queue
            .borrow_mut()
            .push_back((id, Ok(WindowLifecycle::Destroyed)));
        let _ = self.proxy.send_event(());
        Ok(())
    }

    /// Delivers a typed event to `window`'s view (table H1): the
    /// `rvf.ipc.v1` envelope becomes the page's dispatch script, held
    /// in a bounded backlog until the page reports `Ready`.
    fn post(&mut self, window: WindowId, event: &Event) {
        // Unknown/destroyed — silent drop.
        let Some(entry) = self.windows.get_mut(&window) else {
            return;
        };
        let script = dispatch_script(event);
        if entry.ready {
            // Fire-and-forget: a dead page must not wedge the loop.
            let _ = entry.webview.evaluate_script(&script);
        } else {
            if entry.backlog.len() >= MAX_PENDING_SCRIPTS {
                entry.backlog.pop_front();
            }
            entry.backlog.push_back(script);
        }
    }

    /// Marks `window`'s page ready and flushes its held scripts.
    fn mark_ready(&mut self, window: WindowId) {
        if let Some(entry) = self.windows.get_mut(&window) {
            entry.ready = true;
            for script in std::mem::take(&mut entry.backlog) {
                let _ = entry.webview.evaluate_script(&script);
            }
        }
    }

    /// Handles one tao window event for registry window `wid`:
    /// performs the native follow (bounds/focus) then emits the
    /// normalized lifecycle payload.
    fn on_tao_window_event(
        &mut self,
        on_event: &mut dyn FnMut(DriverEvent, &mut dyn WindowDriver),
        target: &EventLoopWindowTarget<()>,
        wid: WindowId,
        event: &TaoWindowEvent,
    ) {
        let lifecycle = match event {
            TaoWindowEvent::Resized(size) => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = e.webview.set_bounds(bounds_for(*size));
                }
                WindowLifecycle::Resized {
                    width: size.width,
                    height: size.height,
                }
            }
            TaoWindowEvent::Focused(focused) => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = if *focused {
                        e.webview.focus()
                    } else {
                        e.webview.focus_parent()
                    };
                }
                WindowLifecycle::Focused(*focused)
            }
            TaoWindowEvent::Moved(pos) => WindowLifecycle::Moved { x: pos.x, y: pos.y },
            TaoWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = e.webview.set_bounds(full_bounds(&e.window));
                }
                WindowLifecycle::ScaleFactorChanged {
                    factor: *scale_factor,
                }
            }
            // Veto-able request only — never destroys, never exits the
            // loop. The default Approve is applied after the emit.
            TaoWindowEvent::CloseRequested => WindowLifecycle::CloseRequested,
            _ => return,
        };
        self.emit(
            on_event,
            target,
            DriverEvent::Windowed {
                window: wid,
                event: lifecycle,
            },
        );
    }

    /// Emits one event to the consumer with a bound [`WindowDriver`],
    /// applying pre/post semantics (ready-flush, close default).
    fn emit(
        &mut self,
        on_event: &mut dyn FnMut(DriverEvent, &mut dyn WindowDriver),
        target: &EventLoopWindowTarget<()>,
        ev: DriverEvent,
    ) {
        // Pre-emit bookkeeping.
        let mut close_requested = None;
        if let DriverEvent::Windowed { window, event } = &ev {
            match event {
                WindowLifecycle::Ready => self.mark_ready(*window),
                WindowLifecycle::CloseRequested => {
                    if let Some(entry) = self.windows.get_mut(window) {
                        entry.close_pending = true;
                        entry.awaiting_response = true;
                    }
                    close_requested = Some(*window);
                }
                _ => {}
            }
        }

        {
            let mut handle = SessionHandle {
                session: self,
                target,
            };
            on_event(ev, &mut handle);
        }

        // Post-emit default: an unanswered CloseRequested approves for
        // that window only (never the loop).
        if let Some(wid) = close_requested
            && self.windows.get(&wid).is_some_and(|e| e.awaiting_response)
        {
            let _ = self.destroy_entry(wid);
        }
    }
}

/// [`WindowDriver`] bound to the live session — handed to `on_event`.
struct SessionHandle<'a> {
    session: &'a mut Session,
    target: &'a EventLoopWindowTarget<()>,
}

impl WindowDriver for SessionHandle<'_> {
    fn capabilities(&self) -> WindowCapabilities {
        WindowCapabilities {
            resize: Capability::Supported,
            // `focus(false)` hands keyboard focus back to the parent
            // window (`focus_parent`), so the release direction is
            // implemented as well as acquisition.
            focus: Capability::Supported,
            scale_factor: Capability::Supported,
            multi_window: Capability::Supported,
            ime: Capability::Unsupported,
            native_surface_slot: Capability::Unsupported,
        }
    }

    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError> {
        let (url, devtools) = (self.session.default_url.clone(), self.session.devtools);
        self.session.create_entry(self.target, spec, &url, devtools)
    }

    fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError> {
        let Some(entry) = self.session.windows.get(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        entry
            .window
            .set_inner_size(dpi::LogicalSize::new(size.width, size.height));
        Ok(())
    }

    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError> {
        let Some(entry) = self.session.windows.get(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        let result = if focused {
            entry.window.set_focusable(true);
            entry.webview.focus()
        } else {
            entry.webview.focus_parent()
        };
        result.map_err(|e| UiError::host(format!("focus request failed: {e}")))
    }

    fn request_close(&mut self, window: WindowId) {
        if self.session.windows.contains_key(&window) {
            self.session
                .queue
                .borrow_mut()
                .push_back((window, Ok(WindowLifecycle::CloseRequested)));
            let _ = self.session.proxy.send_event(());
        }
    }

    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError> {
        let Some(entry) = self.session.windows.get_mut(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        if !entry.close_pending {
            return Err(UiError::host(format!(
                "window {window} has no pending close"
            )));
        }
        entry.awaiting_response = false;
        entry.close_pending = matches!(decision, CloseDecision::Defer);
        if matches!(decision, CloseDecision::Approve) {
            return self.session.destroy_entry(window);
        }
        Ok(())
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        self.session.destroy_entry(window)
    }

    fn post_event(&mut self, window: WindowId, event: &Event) {
        self.session.post(window, event);
    }

    fn windows(&self) -> Vec<WindowId> {
        let mut ids: Vec<WindowId> = self.session.windows.keys().copied().collect();
        ids.sort();
        ids
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        self.target
            .available_monitors()
            .map(|m| {
                let size = m.size();
                let pos = m.position();
                MonitorInfo {
                    name: m.name(),
                    width: size.width,
                    height: size.height,
                    x: pos.x,
                    y: pos.y,
                    scale_factor: m.scale_factor(),
                }
            })
            .collect()
    }

    fn exit(&mut self) {
        self.session.want_exit.set(true);
    }
}

/// Runs the multi-window host loop until [`WindowDriver::exit`].
///
/// `on_event` receives [`DriverEvent`]s plus a [`WindowDriver`] for
/// driving back. It is invoked on the UI thread — it must not block;
/// heavy work belongs to job workers via `UiRuntime`.
///
/// # Errors
/// [`UiError::BackendUnavailable`] when the `WebView` backend cannot
/// initialize; [`UiError::Host`] when an initial window fails or the
/// loop exits abnormally.
pub fn run_multi<F>(
    config: &SessionConfig,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    mut on_event: F,
) -> Result<(), UiError>
where
    F: FnMut(DriverEvent, &mut dyn WindowDriver),
{
    let mut event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let data_dir = config
        .data_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("rvf-udf-{}", std::process::id())));
    let web_context = WebContext::new(Some(data_dir));

    let mut session = Session {
        windows: HashMap::new(),
        by_tao: HashMap::new(),
        next_id: 1,
        queue: Rc::new(RefCell::new(VecDeque::new())),
        proxy,
        web_context,
        assets,
        data,
        default_url: config.default_url.clone(),
        devtools: config.devtools,
        want_exit: Cell::new(false),
    };

    // Initial windows — a failed create aborts the session before the
    // loop starts; nothing observable is left behind.
    for window in &config.windows {
        session.create_entry(&event_loop, &window.spec, &window.url, window.devtools)?;
    }

    let mut next_tick = config.tick.map(|d| Instant::now() + d);
    let tick = config.tick;

    let code = event_loop.run_return(move |event, elwt, control_flow| {
        *control_flow = match next_tick {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        match event {
            TaoEvent::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                session.emit(&mut on_event, elwt, DriverEvent::Tick);
                next_tick = tick.map(|d| Instant::now() + d);
            }
            TaoEvent::UserEvent(()) => loop {
                let next = session.queue.borrow_mut().pop_front();
                let Some((wid, inbound)) = next else { break };
                match inbound {
                    Ok(ev) => {
                        // Events for windows destroyed between queueing and
                        // draining are dropped — except `Destroyed` itself,
                        // which must always reach the consumer.
                        if !matches!(ev, WindowLifecycle::Destroyed)
                            && !session.windows.contains_key(&wid)
                        {
                            continue;
                        }
                        session.emit(
                            &mut on_event,
                            elwt,
                            DriverEvent::Windowed {
                                window: wid,
                                event: ev,
                            },
                        );
                    }
                    // A malformed IPC body is answered on its origin
                    // window; the consumer never sees it.
                    Err(error) => session.post(wid, &Event::error(error)),
                }
            },
            TaoEvent::WindowEvent {
                window_id, event, ..
            } => {
                let Some(&wid) = session.by_tao.get(&window_id) else {
                    // Event for a window not in the registry (already
                    // destroyed or foreign) — drop it.
                    return;
                };
                session.on_tao_window_event(&mut on_event, elwt, wid, &event);
            }
            TaoEvent::MainEventsCleared if session.want_exit.get() => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
    if code == 0 {
        Ok(())
    } else {
        Err(UiError::host(format!(
            "event loop exited abnormally (code {code})"
        )))
    }
}
