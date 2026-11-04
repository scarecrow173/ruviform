//! `FakeHost` — a headless [`WindowHandle`] mirroring the
//! `run_multi` delivery contract (Issue #312).
//!
//! The fake keeps every semantic the real session documents:
//!
//! - eval to an unknown or destroyed window is a **silent drop**;
//!   not-yet-ready windows buffer a bounded backlog (drop-oldest)
//!   flushed when their `Ready` event is emitted;
//! - `CloseRequested` is veto-able: it marks the window
//!   close-pending, and an unanswered request defaults to `Approve`
//!   for that window only — never the loop;
//! - `Destroyed` is the window-terminal event, delivered through the
//!   event queue like backend teardown;
//! - events queued for windows destroyed between queueing and
//!   pumping are dropped, except `Destroyed` itself;
//! - the `ipc_disconnect` fault suppresses inbound `Message`
//!   delivery — suppressed events are returned to the caller for
//!   honest recording, never silently dropped.
//!
//! Events are delivered through [`FakeHost::inject`] +
//! [`FakeHost::pump`] — the fake never calls the app except inside
//! `pump`, which makes delivery deterministic.
//!
//! **RT contract:** non-RT only.

use std::collections::{HashMap, HashSet, VecDeque};

use ruviform_ui::{
    CloseDecision, HostEvent, MonitorInfo, UiError, WindowConfig, WindowEvent, WindowHandle,
    WindowId,
};

/// Bounded eval backlog per not-yet-ready window (drop-oldest) —
/// same bound as `run_multi`.
const MAX_EVAL_BACKLOG: usize = 16;

/// One managed fake window.
pub struct FakeWindow {
    /// The [`WindowConfig`] it was created with.
    pub config: WindowConfig,
    /// Whether its `Ready` event has been emitted.
    pub ready: bool,
    /// Whether a `CloseRequested` is pending an answer.
    pub close_pending: bool,
    /// Whether the latest `CloseRequested` is still unanswered —
    /// controls the post-emit default `Approve`.
    awaiting_response: bool,
    /// Buffered evals while not ready.
    backlog: VecDeque<String>,
}

/// Headless multi-window host implementing [`WindowHandle`].
///
/// Besides the trait, it exposes `inject`/`pump` (event delivery),
/// `ipc_disconnect`/`ipc_reconnect` (the `ipc_disconnect` fault), and
/// `delivered`/`windows` inspection for observation.
pub struct FakeHost {
    next: u64,
    /// Living windows.
    pub windows: HashMap<WindowId, FakeWindow>,
    /// Eval scripts delivered to ready windows, in order.
    delivered: Vec<(WindowId, String)>,
    /// Queued host-side events (mirrors the adapter's shared queue).
    queue: VecDeque<WindowEvent>,
    /// Windows whose inbound `Message` events are suppressed by the
    /// `ipc_disconnect` fault.
    ipc_down: HashSet<WindowId>,
    /// Whether `exit` was requested.
    exited: bool,
    /// Monitor list reported by `monitors()`.
    monitors: Vec<MonitorInfo>,
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHost {
    /// Creates an empty fake host reporting one 1920×1080 monitor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next: 1,
            windows: HashMap::new(),
            delivered: Vec::new(),
            queue: VecDeque::new(),
            ipc_down: HashSet::new(),
            exited: false,
            monitors: vec![MonitorInfo {
                name: Some("fake-monitor".to_string()),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                scale_factor: 1.0,
            }],
        }
    }

    /// Overrides the monitor list `monitors()` reports.
    pub fn set_monitors(&mut self, monitors: Vec<MonitorInfo>) {
        self.monitors = monitors;
    }

    /// Queues a host-side event for the next [`pump`](Self::pump).
    /// Use it for `Ready`, `Tick`, `CloseRequested`, `Message`,
    /// backend-loss `Destroyed`, resize/focus notifications — every
    /// `WindowEvent` an adapter would emit.
    pub fn inject(&mut self, ev: WindowEvent) {
        self.queue.push_back(ev);
    }

    /// Whether `exit` was requested.
    #[must_use]
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Eval scripts delivered to ready windows, oldest first.
    #[must_use]
    pub fn delivered(&self) -> &[(WindowId, String)] {
        &self.delivered
    }

    /// Scripts delivered to one window.
    #[must_use]
    pub fn scripts_for(&self, window: WindowId) -> Vec<&str> {
        self.delivered
            .iter()
            .filter(|(w, _)| *w == window)
            .map(|(_, s)| s.as_str())
            .collect()
    }

    /// Arms the `ipc_disconnect` fault for `window`.
    pub fn ipc_disconnect(&mut self, window: WindowId) {
        self.ipc_down.insert(window);
    }

    /// Clears the `ipc_disconnect` fault for `window`.
    pub fn ipc_reconnect(&mut self, window: WindowId) {
        self.ipc_down.remove(&window);
    }

    /// Whether the `ipc_disconnect` fault is armed for `window`.
    #[must_use]
    pub fn is_ipc_down(&self, window: WindowId) -> bool {
        self.ipc_down.contains(&window)
    }

    /// Drains the injected queue: emits each event to `on_event` —
    /// the deterministic equivalent of the adapter's event loop.
    ///
    /// Pre/post bookkeeping mirrors `run_multi`: `Ready` flushes the
    /// eval backlog, `CloseRequested` marks the window and defaults
    /// to `Approve` when unanswered, `Destroyed` is always delivered
    /// even for retired ids.
    ///
    /// Returns the inbound `Message` events suppressed by the
    /// `ipc_disconnect` fault — the caller records them; they never
    /// reach `on_event`.
    pub fn pump(
        &mut self,
        on_event: &mut dyn FnMut(WindowEvent, &mut dyn WindowHandle),
    ) -> Vec<(WindowId, HostEvent)> {
        let mut suppressed = Vec::new();
        while let Some(wev) = self.queue.pop_front() {
            let WindowEvent::Windowed { window, event } = wev else {
                on_event(WindowEvent::Tick, self);
                continue;
            };
            // Dead-window drop — except `Destroyed`, which must always
            // reach the consumer (mirrors run_multi's queue drain).
            if !matches!(event, HostEvent::Destroyed) && !self.windows.contains_key(&window) {
                continue;
            }
            if matches!(event, HostEvent::Message(_)) && self.ipc_down.contains(&window) {
                suppressed.push((window, event));
                continue;
            }
            // Pre-emit bookkeeping.
            match &event {
                HostEvent::Ready { .. } => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.ready = true;
                        let flush = std::mem::take(&mut entry.backlog);
                        self.delivered
                            .extend(flush.into_iter().map(|s| (window, s)));
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

            on_event(
                WindowEvent::Windowed { window, event },
                self as &mut dyn WindowHandle,
            );

            // Post-emit default: an unanswered CloseRequested approves
            // for that window only (never the loop).
            if self
                .windows
                .get(&window)
                .is_some_and(|e| e.awaiting_response)
            {
                let _ = self.destroy_window_inner(window);
            }
        }
        suppressed
    }

    /// Internal destroy: drops the entry and queues `Destroyed`
    /// through the same channel backend teardown uses.
    fn destroy_window_inner(&mut self, window: WindowId) -> Result<(), UiError> {
        if self.windows.remove(&window).is_none() {
            return Err(UiError::host(format!("no window {window}")));
        }
        self.queue.push_back(WindowEvent::Windowed {
            window,
            event: HostEvent::Destroyed,
        });
        Ok(())
    }
}

impl WindowHandle for FakeHost {
    fn eval(&mut self, window: WindowId, script: &str) {
        let Some(entry) = self.windows.get_mut(&window) else {
            return; // unknown/destroyed — silent drop per contract
        };
        if entry.ready {
            self.delivered.push((window, script.to_string()));
        } else {
            if entry.backlog.len() >= MAX_EVAL_BACKLOG {
                entry.backlog.pop_front();
            }
            entry.backlog.push_back(script.to_string());
        }
    }

    fn resize(&mut self, window: WindowId, _width: u32, _height: u32) -> Result<(), UiError> {
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

    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, UiError> {
        let id = WindowId::new(self.next).expect("counter nonzero");
        self.next += 1;
        self.windows.insert(
            id,
            FakeWindow {
                config: config.clone(),
                ready: false,
                close_pending: false,
                awaiting_response: false,
                backlog: VecDeque::new(),
            },
        );
        Ok(id)
    }

    fn request_close(&mut self, window: WindowId) {
        if self.windows.contains_key(&window) {
            self.inject(WindowEvent::Windowed {
                window,
                event: HostEvent::CloseRequested,
            });
        }
    }

    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError> {
        let Some(entry) = self.windows.get_mut(&window) else {
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
            return self.destroy_window_inner(window);
        }
        Ok(())
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        self.destroy_window_inner(window)
    }

    fn windows(&self) -> Vec<WindowId> {
        let mut ids: Vec<WindowId> = self.windows.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.clone()
    }

    fn exit(&mut self) {
        self.exited = true;
    }
}
