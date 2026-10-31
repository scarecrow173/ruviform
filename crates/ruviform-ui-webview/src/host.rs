//! Windowed host — tao event loop + wry webview wiring that implements
//! the `ruviform-ui` host boundary.
//!
//! Structure (inherited from `poc/ui`, where every mechanism was
//! measured):
//!
//! - **Wake-up**: `WebView2` callbacks do not wake a `WaitUntil` event
//!   loop, so IPC/page-load handlers push onto a queue and nudge an
//!   `EventLoopProxy`. `Event::UserEvent` drains the queue.
//! - **Resize follow**: the webview is `build()`ed (not
//!   `build_as_child` — `WebView2` 155 did not propagate child-container
//!   resizes into the JS viewport) and `set_bounds` re-applied on every
//!   `Resized`/`ScaleFactorChanged`.
//! - **Tick**: an optional periodic `WaitUntil` deadline produces
//!   [`HostEvent::Tick`] — the drain point for job mailboxes.
//! - **Close**: `CloseRequested` (or [`HostHandle::request_close`])
//!   exits the loop; `run` returns afterwards.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};
use ruviform_ui::{AssetSource, DataRegistry, HostEvent, HostHandle, UiError};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

/// Host configuration for [`run`].
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Window title.
    pub title: String,
    /// Initial logical width.
    pub width: u32,
    /// Initial logical height.
    pub height: u32,
    /// Initial URL — normally inside the `rvf://app` namespace.
    pub url: String,
    /// `WebView` user-data folder. `None` selects a process-unique temp
    /// dir (`WebView2`'s UDF is a singleton lock — a stale browser child
    /// would otherwise block the next init; measured in `poc/ui`).
    pub data_dir: Option<PathBuf>,
    /// Periodic [`HostEvent::Tick`] cadence (job-drain wakeup).
    /// `None` = never.
    pub tick: Option<Duration>,
    /// Enable backend devtools (the app should bind this to
    /// `cfg!(debug_assertions)` or a flag).
    pub devtools: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            title: "ruviform".to_string(),
            width: 1280,
            height: 800,
            url: "rvf://app/index.html".to_string(),
            data_dir: None,
            tick: Some(Duration::from_millis(16)),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Runs the host loop until close.
///
/// `on_event` receives normalized [`HostEvent`]s plus a
/// [`HostHandle`] for driving back (eval pushes, resize, focus,
/// close). It is invoked on the UI thread — it must not block; heavy
/// work belongs to job workers via `UiRuntime`.
///
/// # Errors
/// [`UiError::BackendUnavailable`] when the `WebView` backend cannot
/// initialize (missing `WebView2`/`WebKitGTK` runtime); [`UiError::Host`]
/// for window/loop failures.
pub fn run<F>(
    config: &HostConfig,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    mut on_event: F,
) -> Result<(), UiError>
where
    F: FnMut(HostEvent, &mut dyn HostHandle),
{
    let mut event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .build(&event_loop)
        .map_err(|e| UiError::host(format!("window creation failed: {e}")))?;

    let data_dir = config
        .data_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("rvf-udf-{}", std::process::id())));
    let mut web_context = WebContext::new(Some(data_dir));

    // Backend callbacks run on WebView-owned threads: they queue
    // events and wake the loop via the proxy (see module docs).
    let queue: Rc<RefCell<VecDeque<HostEvent>>> = Rc::new(RefCell::new(VecDeque::new()));
    let enqueue = {
        let queue = queue.clone();
        let proxy = proxy.clone();
        move |ev: HostEvent| {
            queue.borrow_mut().push_back(ev);
            let _ = proxy.send_event(());
        }
    };

    let webview = build_webview(&window, &mut web_context, config, assets, data, enqueue)?;

    let want_exit = Cell::new(false);
    let mut next_tick = config.tick.map(|d| Instant::now() + d);
    let tick = config.tick;

    let code = event_loop.run_return(move |event, _elwt, control_flow| {
        *control_flow = match next_tick {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        let mut host = WebHost {
            webview: &webview,
            window: &window,
            want_exit: &want_exit,
        };
        let mut emit = |ev: HostEvent| on_event(ev, &mut host);
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                emit(HostEvent::Tick);
                next_tick = tick.map(|d| Instant::now() + d);
            }
            Event::UserEvent(()) => {
                while let Some(ev) = queue.borrow_mut().pop_front() {
                    emit(ev);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    let _ = webview.set_bounds(bounds_for(size));
                    emit(HostEvent::Resized {
                        width: size.width,
                        height: size.height,
                    });
                }
                WindowEvent::Focused(focused) => {
                    if focused {
                        let _ = webview.focus();
                    } else {
                        let _ = webview.focus_parent();
                    }
                    emit(HostEvent::Focused(focused));
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let _ = webview.set_bounds(bounds_for(window.inner_size()));
                    emit(HostEvent::ScaleFactorChanged {
                        factor: scale_factor,
                    });
                }
                WindowEvent::CloseRequested => {
                    emit(HostEvent::CloseRequested);
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            Event::MainEventsCleared if want_exit.get() => {
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

/// Wires the `rvf` scheme, IPC handler, page-load handler, navigation
/// policy, bridge script and initial bounds into a webview build.
fn build_webview(
    window: &tao::window::Window,
    web_context: &mut WebContext,
    config: &HostConfig,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    enqueue: impl Fn(HostEvent) + Clone + 'static,
) -> Result<WebView, UiError> {
    let ipc_enqueue = enqueue.clone();
    let load_enqueue = enqueue;
    WebViewBuilder::new_with_web_context(web_context)
        .with_url(config.url.clone())
        .with_initialization_script(ruviform_ui::BRIDGE_JS)
        .with_custom_protocol("rvf".to_string(), move |_id, request| {
            crate::scheme::serve(&assets, &data, &request)
        })
        .with_ipc_handler(move |request: http::Request<String>| {
            ipc_enqueue(HostEvent::Message(request.into_body()));
        })
        .with_on_page_load_handler(move |event, url| {
            if matches!(event, PageLoadEvent::Finished) {
                load_enqueue(HostEvent::Ready { url });
            }
        })
        .with_navigation_handler(|url| {
            // Only the app origin (and its Windows http://rvf.* rewrite)
            // may navigate; everything else is rejected.
            url.starts_with("rvf://app")
                || url.starts_with("http://rvf.app")
                || url == "about:blank"
        })
        .with_devtools(config.devtools)
        .with_bounds(full_bounds(window))
        .build(window)
        .map_err(|e| UiError::backend_unavailable(format!("webview init failed: {e}")))
}

/// Host handle bound to the live wry/tao objects.
struct WebHost<'a> {
    webview: &'a WebView,
    window: &'a tao::window::Window,
    want_exit: &'a Cell<bool>,
}

impl HostHandle for WebHost<'_> {
    fn eval(&mut self, script: &str) {
        // Fire-and-forget: a dead page must not wedge the host loop.
        let _ = self.webview.evaluate_script(script);
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), UiError> {
        self.window.set_inner_size(LogicalSize::new(width, height));
        Ok(())
    }

    fn focus(&mut self, focused: bool) -> Result<(), UiError> {
        let result = if focused {
            self.window.set_focusable(true);
            self.webview.focus()
        } else {
            self.webview.focus_parent()
        };
        result.map_err(|e| UiError::host(format!("focus request failed: {e}")))
    }

    fn request_close(&mut self) {
        self.want_exit.set(true);
    }
}

fn full_bounds(window: &tao::window::Window) -> Rect {
    bounds_for(window.inner_size())
}

fn bounds_for(size: PhysicalSize<u32>) -> Rect {
    Rect {
        position: Position::Physical(PhysicalPosition::new(0, 0)),
        size: Size::Physical(size),
    }
}
