//! Multi-window host — one tao event loop managing many
//! windows/webviews (Issue #181).
//!
//! Same mechanics as the single-window [`host`](crate::host) —
//! `EventLoopProxy` wake-up, `set_bounds` resize follow, tick-driven
//! job drain — plus the multi-window contract:
//!
//! - **Registry**: every window/webview pair gets a stable
//!   [`WindowId`] issued at create; backend callbacks carry it into a
//!   shared queue so every emitted [`WindowEvent::Windowed`] knows its
//!   origin.
//! - **Shared `WebContext`**: all webviews share one context/user-data
//!   dir (`WebView2`'s UDF is a process singleton lock — `poc/ui`
//!   measured stale browser children blocking the next init). The
//!   context lives for the whole session and drops after the loop.
//! - **Close flow**: `CloseRequested` marks the window close-pending
//!   and emits — it never destroys and never exits the loop. The
//!   consumer answers via [`WindowHandle::respond_close`]; an
//!   unanswered request defaults to `Approve` for that window only.
//!   `Destroyed` follows any teardown (approved close, programmatic
//!   destroy, backend loss).
//! - **Delivery safety**: `eval` to an unknown or destroyed window is
//!   a silent drop; a not-yet-ready webview buffers a bounded backlog
//!   (drop-oldest) flushed on `Ready`.
//! - **App-exit policy**: the loop only exits via
//!   [`WindowHandle::exit`] — last-window semantics are the
//!   consumer's.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dpi::LogicalSize;
use ruviform_ui::{
    AssetSource, CloseDecision, DataRegistry, HostEvent, MonitorInfo, UiError, WindowConfig,
    WindowEvent, WindowHandle, WindowId,
};
use tao::event::{Event, StartCause, WindowEvent as TaoEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::{WebContext, WebView};

use crate::host::{HostConfig, bounds_for, build_webview, full_bounds};

/// Bounded eval backlog per not-yet-ready window (drop-oldest).
const MAX_EVAL_BACKLOG: usize = 16;

/// Session-level configuration for [`run_multi`].
///
/// Session concerns (the shared `WebContext` user-data dir, the tick
/// cadence) live here; per-window parameters are [`WindowConfig`]s.
/// The single-window [`run`](crate::host::run) takes [`HostConfig`]
/// instead — [`SessionConfig::single`] converts one for parity.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// `WebView` user-data folder shared by every window in the
    /// session (`WebView2`'s UDF is a singleton lock — separate
    /// contexts on the same folder would collide). `None` selects a
    /// process-unique temp dir.
    pub data_dir: Option<PathBuf>,
    /// Periodic [`WindowEvent::Tick`] cadence (job-drain wakeup).
    /// `None` = never.
    pub tick: Option<Duration>,
    /// Windows to create before the loop starts. May be empty —
    /// consumers can create windows later via
    /// [`WindowHandle::create_window`]; the loop still runs until
    /// [`WindowHandle::exit`].
    pub windows: Vec<WindowConfig>,
}

impl SessionConfig {
    /// Builds a single-window session from a [`HostConfig`] — the
    /// multi-window equivalent of [`run`](crate::host::run)'s config.
    #[must_use]
    pub fn single(config: &HostConfig) -> Self {
        Self {
            data_dir: config.data_dir.clone(),
            tick: config.tick,
            windows: vec![WindowConfig {
                title: config.title.clone(),
                width: config.width,
                height: config.height,
                url: config.url.clone(),
                devtools: config.devtools,
            }],
        }
    }
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
    /// `(window, event)` pairs, drained on `Event::UserEvent`.
    queue: Rc<RefCell<VecDeque<(WindowId, HostEvent)>>>,
    proxy: EventLoopProxy<()>,
    web_context: WebContext,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
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
        config: &WindowConfig,
    ) -> Result<WindowId, UiError> {
        let window = WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .build(target)
            .map_err(|e| UiError::host(format!("window creation failed: {e}")))?;

        let id = self.issue_id();
        // The webview's backend callbacks stamp every queued event
        // with this window's id — IPC bodies stay opaque.
        let enqueue = {
            let queue = self.queue.clone();
            let proxy = self.proxy.clone();
            move |ev: HostEvent| {
                queue.borrow_mut().push_back((id, ev));
                let _ = proxy.send_event(());
            }
        };
        // A failed webview build must not leave a live window behind:
        // `window` drops with the early return.
        let webview = build_webview(
            &window,
            &mut self.web_context,
            &config.url,
            config.devtools,
            self.assets.clone(),
            self.data.clone(),
            enqueue,
        )?;

        let tao_id = window.id();
        self.by_tao.insert(tao_id, id);
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
            .push_back((id, HostEvent::Destroyed));
        let _ = self.proxy.send_event(());
        Ok(())
    }

    /// Handles one tao window event for registry window `wid`:
    /// performs the native follow (bounds/focus) then emits the
    /// normalized [`HostEvent`].
    fn on_tao_window_event(
        &mut self,
        on_event: &mut dyn FnMut(WindowEvent, &mut dyn WindowHandle),
        target: &EventLoopWindowTarget<()>,
        wid: WindowId,
        event: &TaoEvent,
    ) {
        let mut emit = |session: &mut Self, ev: HostEvent| {
            session.emit(
                on_event,
                target,
                WindowEvent::Windowed {
                    window: wid,
                    event: ev,
                },
            );
        };
        match event {
            TaoEvent::Resized(size) => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = e.webview.set_bounds(bounds_for(*size));
                }
                emit(
                    self,
                    HostEvent::Resized {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
            TaoEvent::Focused(focused) => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = if *focused {
                        e.webview.focus()
                    } else {
                        e.webview.focus_parent()
                    };
                }
                emit(self, HostEvent::Focused(*focused));
            }
            TaoEvent::Moved(pos) => {
                emit(self, HostEvent::Moved { x: pos.x, y: pos.y });
            }
            TaoEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(e) = self.windows.get(&wid) {
                    let _ = e.webview.set_bounds(full_bounds(&e.window));
                }
                emit(
                    self,
                    HostEvent::ScaleFactorChanged {
                        factor: *scale_factor,
                    },
                );
            }
            TaoEvent::CloseRequested => {
                // Veto-able request only — never destroys, never exits
                // the loop. The default Approve for an unanswered
                // request is applied post-emit.
                emit(self, HostEvent::CloseRequested);
            }
            _ => {}
        }
    }

    /// Emits one event to the consumer with a bound [`WindowHandle`],
    /// applying pre/post semantics (ready-flush, close default).
    fn emit(
        &mut self,
        on_event: &mut dyn FnMut(WindowEvent, &mut dyn WindowHandle),
        target: &EventLoopWindowTarget<()>,
        wev: WindowEvent,
    ) {
        // Pre-emit bookkeeping.
        if let WindowEvent::Windowed { window, event } = &wev {
            let window = *window;
            match event {
                HostEvent::Ready { .. } => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.ready = true;
                        let backlog = std::mem::take(&mut entry.backlog);
                        for script in backlog {
                            let _ = entry.webview.evaluate_script(&script);
                        }
                    }
                }
                HostEvent::CloseRequested => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.close_pending = true;
                        entry.awaiting_response = true;
                    }
                }
                _ => {}
            }
        }

        // Remember whether this emit was a CloseRequested — `wev` is
        // moved into `on_event` below.
        let close_check = if let WindowEvent::Windowed {
            window,
            event: HostEvent::CloseRequested,
        } = &wev
        {
            Some(*window)
        } else {
            None
        };

        {
            let mut host = SessionHandle {
                session: self,
                target,
            };
            on_event(wev, &mut host);
        }

        // Post-emit default: an unanswered CloseRequested approves for
        // that window only (never the loop).
        if let Some(wid) = close_check {
            let default_approve = self.windows.get(&wid).is_some_and(|e| e.awaiting_response);
            if default_approve {
                let _ = self.destroy_entry(wid);
            }
        }
    }
}

/// [`WindowHandle`] bound to the live session — handed to `on_event`.
struct SessionHandle<'a> {
    session: &'a mut Session,
    target: &'a EventLoopWindowTarget<()>,
}

impl WindowHandle for SessionHandle<'_> {
    fn eval(&mut self, window: WindowId, script: &str) {
        let Some(entry) = self.session.windows.get_mut(&window) else {
            return; // unknown/destroyed — silent drop
        };
        if entry.ready {
            // Fire-and-forget: a dead page must not wedge the loop.
            let _ = entry.webview.evaluate_script(script);
        } else {
            if entry.backlog.len() >= MAX_EVAL_BACKLOG {
                entry.backlog.pop_front();
            }
            entry.backlog.push_back(script.to_string());
        }
    }

    fn resize(&mut self, window: WindowId, width: u32, height: u32) -> Result<(), UiError> {
        let Some(entry) = self.session.windows.get(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        entry.window.set_inner_size(LogicalSize::new(width, height));
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

    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, UiError> {
        self.session.create_entry(self.target, config)
    }

    fn request_close(&mut self, window: WindowId) {
        if self.session.windows.contains_key(&window) {
            self.session
                .queue
                .borrow_mut()
                .push_back((window, HostEvent::CloseRequested));
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

/// Runs the multi-window host loop until [`WindowHandle::exit`].
///
/// `on_event` receives [`WindowEvent`]s plus a [`WindowHandle`] for
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
    F: FnMut(WindowEvent, &mut dyn WindowHandle),
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
        want_exit: Cell::new(false),
    };

    // Initial windows — a failed create aborts the session before the
    // loop starts; nothing observable is left behind.
    for cfg in &config.windows {
        session.create_entry(&event_loop, cfg)?;
    }

    let mut next_tick = config.tick.map(|d| Instant::now() + d);
    let tick = config.tick;

    let code = event_loop.run_return(move |event, elwt, control_flow| {
        *control_flow = match next_tick {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                session.emit(&mut on_event, elwt, WindowEvent::Tick);
                next_tick = tick.map(|d| Instant::now() + d);
            }
            Event::UserEvent(()) => {
                loop {
                    let next = session.queue.borrow_mut().pop_front();
                    let Some((wid, ev)) = next else { break };
                    // Events for windows destroyed between queueing and
                    // draining are dropped — except `Destroyed` itself,
                    // which must always reach the consumer.
                    if !matches!(ev, HostEvent::Destroyed) && !session.windows.contains_key(&wid) {
                        continue;
                    }
                    session.emit(
                        &mut on_event,
                        elwt,
                        WindowEvent::Windowed {
                            window: wid,
                            event: ev,
                        },
                    );
                }
            }
            Event::WindowEvent {
                window_id, event, ..
            } => {
                let Some(&wid) = session.by_tao.get(&window_id) else {
                    // Event for a window not in the registry (already
                    // destroyed or foreign) — drop it.
                    return;
                };
                session.on_tao_window_event(&mut on_event, elwt, wid, &event);
            }
            Event::MainEventsCleared if session.want_exit.get() => {
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
