//! Deterministic fake implementing the driver-neutral
//! [`WindowDriver`] contract (Issue #386) — no backend, no event
//! loop, no real windows. It mirrors [`crate::FakeHost`]'s delivery
//! contract so the same multi-window lifecycle fixtures can run
//! against the neutral port:
//!
//! - `post_event` to unknown/destroyed windows drops silently;
//!   not-yet-`Ready` windows buffer a bounded backlog (drop-oldest)
//!   that flushes on `Ready`.
//! - `Destroyed` events always reach the consumer — even for ids
//!   retired by an abnormal teardown — so job subscriptions are
//!   always cleaned up.
//! - `CloseRequested` marks the window close-pending and defaults to
//!   `Approve` for that window only when the consumer leaves it
//!   unanswered — matching the host contract.
//! - `request_close` on an unknown window is ignored; fallible
//!   operations on unknown windows report
//!   [`UiError::Host`]; operations the fake is configured not to
//!   support report [`UiError::Unsupported`].
//! - `exit` records the request; it never exits the pump loop
//!   implicitly.
//!
//! **`pump` is deterministic, not real-time**: injected events are
//! delivered FIFO each `pump`, and `Destroyed` events queued by
//! approved closes are delivered in the same pump (mirroring a real
//! driver's synchronous teardown ordering).
//!
//! **RT contract:** non-RT only. Test-only — crate is a dev/test
//! dependency.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ruviform_ui::{
    CloseDecision, DriverEvent, Event, Intent, LogicalSize, MonitorInfo, UiError,
    WindowCapabilities, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};

/// Maximum buffered `post_event` backlog per not-yet-`Ready` window —
/// delivery beyond it is dropped, oldest first.
pub const MAX_EVENT_BACKLOG: usize = 16;

/// Close-request bookkeeping — kept as a sub-struct so the window
/// entry does not accumulate loose bools.
#[derive(Default)]
struct CloseFlags {
    pending: bool,
    awaiting_response: bool,
}

struct FakeWindow {
    spec: WindowSpec,
    ready: bool,
    focused: bool,
    close: CloseFlags,
    backlog: VecDeque<Event>,
    size: LogicalSize,
}

/// Deterministic fake [`WindowDriver`] — the driver-neutral
/// counterpart of [`crate::FakeHost`].
///
/// Construction starts empty; tests create windows via
/// [`WindowDriver::create_window`] and drive the lifecycle by
/// injecting [`DriverEvent`]s and pumping.
pub struct FakeDriver {
    windows: BTreeMap<WindowId, FakeWindow>,
    destroyed: BTreeSet<WindowId>,
    monitors: Vec<MonitorInfo>,
    capabilities: WindowCapabilities,
    queue: VecDeque<DriverEvent>,
    delivered: Vec<(WindowId, Event)>,
    ipc_down: BTreeSet<WindowId>,
    next: u64,
    exited: bool,
}

impl Default for FakeDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeDriver {
    /// Creates an empty fake driver with every capability supported.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            destroyed: BTreeSet::new(),
            monitors: Vec::new(),
            capabilities: WindowCapabilities {
                resize: ruviform_ui::Capability::Supported,
                focus: ruviform_ui::Capability::Supported,
                scale_factor: ruviform_ui::Capability::Supported,
                multi_window: ruviform_ui::Capability::Supported,
                ime: ruviform_ui::Capability::Supported,
                native_surface_slot: ruviform_ui::Capability::Supported,
            },
            queue: VecDeque::new(),
            delivered: Vec::new(),
            ipc_down: BTreeSet::new(),
            next: 1,
            exited: false,
        }
    }

    /// Overrides the capability snapshot — tests set individual
    /// fields to `Capability::Unsupported` to exercise the
    /// `UiError::Unsupported` path.
    pub fn set_capabilities(&mut self, capabilities: WindowCapabilities) {
        self.capabilities = capabilities;
    }

    /// Overrides the monitor list `monitors()` reports.
    pub fn set_monitors(&mut self, monitors: Vec<MonitorInfo>) {
        self.monitors = monitors;
    }

    /// Queues a driver-side event for the next [`pump`](Self::pump).
    /// Use it for `Ready`, `Tick`, `CloseRequested`, `Action`,
    /// backend-loss `Destroyed`, resize/focus/scale notifications —
    /// every [`DriverEvent`] a real driver would emit.
    pub fn inject(&mut self, ev: DriverEvent) {
        self.queue.push_back(ev);
    }

    /// Whether `exit` was requested.
    #[must_use]
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Ids the fake recorded as destroyed — retire order preserved
    /// by `BTreeSet` ordering on id.
    #[must_use]
    pub fn destroyed(&self) -> BTreeSet<WindowId> {
        self.destroyed.clone()
    }

    /// Whether `window` has reported `Ready` per fake state.
    #[must_use]
    pub fn ready(&self, window: WindowId) -> bool {
        self.windows.get(&window).is_some_and(|w| w.ready)
    }

    /// Whether `window` has an unanswered `CloseRequested`.
    #[must_use]
    pub fn close_pending(&self, window: WindowId) -> bool {
        self.windows.get(&window).is_some_and(|w| w.close.pending)
    }

    /// Whether `window` is currently focused per fake state.
    #[must_use]
    pub fn focused(&self, window: WindowId) -> bool {
        self.windows.get(&window).is_some_and(|w| w.focused)
    }

    /// The `WindowSpec` `window` was created with — for spec-mapping
    /// assertions.
    #[must_use]
    pub fn spec_of(&self, window: WindowId) -> Option<&WindowSpec> {
        self.windows.get(&window).map(|w| &w.spec)
    }

    /// Events delivered to ready windows, oldest first.
    #[must_use]
    pub fn delivered(&self) -> &[(WindowId, Event)] {
        &self.delivered
    }

    /// Events delivered to one window.
    #[must_use]
    pub fn events_for(&self, window: WindowId) -> Vec<&Event> {
        self.delivered
            .iter()
            .filter(|(w, _)| *w == window)
            .map(|(_, e)| e)
            .collect()
    }

    /// Arms the `ipc_disconnect` fault for `window` — inbound
    /// `Action` intents are suppressed while armed.
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
    /// the deterministic equivalent of the driver's event loop.
    ///
    /// Pre/post bookkeeping mirrors [`crate::FakeHost::pump`]:
    /// `Ready` flushes the event backlog, `CloseRequested` marks the
    /// window and defaults to `Approve` when unanswered, `Destroyed`
    /// is always delivered even for retired ids.
    ///
    /// Returns the inbound `Action` intents suppressed by the
    /// `ipc_disconnect` fault — the caller records them; they never
    /// reach `on_event`.
    pub fn pump(
        &mut self,
        on_event: &mut dyn FnMut(DriverEvent, &mut dyn WindowDriver),
    ) -> Vec<(WindowId, Intent)> {
        let mut suppressed = Vec::new();
        while let Some(dev) = self.queue.pop_front() {
            let DriverEvent::Windowed { window, event } = dev else {
                on_event(DriverEvent::Tick, self);
                continue;
            };
            // Dead-window drop — except `Destroyed`, which must always
            // reach the consumer (mirrors a real driver's teardown).
            if !matches!(event, WindowLifecycle::Destroyed) && !self.windows.contains_key(&window) {
                continue;
            }
            if matches!(event, WindowLifecycle::Action(_)) && self.ipc_down.contains(&window) {
                let WindowLifecycle::Action(intent) = event else {
                    unreachable!()
                };
                suppressed.push((window, intent));
                continue;
            }
            // Pre-emit bookkeeping.
            match &event {
                WindowLifecycle::Ready => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.ready = true;
                        let flush = std::mem::take(&mut entry.backlog);
                        self.delivered
                            .extend(flush.into_iter().map(|e| (window, e)));
                    }
                }
                WindowLifecycle::Focused(focused) => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.focused = *focused;
                    }
                }
                WindowLifecycle::CloseRequested => {
                    if let Some(entry) = self.windows.get_mut(&window) {
                        entry.close.pending = true;
                        entry.close.awaiting_response = true;
                    }
                }
                _ => {}
            }

            on_event(DriverEvent::Windowed { window, event }, self);

            // Post-emit default: an unanswered CloseRequested approves
            // for that window only (never the loop).
            if self
                .windows
                .get(&window)
                .is_some_and(|e| e.close.awaiting_response)
            {
                let _ = self.destroy_window_inner(window);
            }
        }
        suppressed
    }

    /// Internal destroy: drops the entry, records the retire, and
    /// queues `Destroyed` through the same channel backend teardown
    /// uses.
    fn destroy_window_inner(&mut self, window: WindowId) -> Result<(), UiError> {
        if self.windows.remove(&window).is_none() {
            return Err(UiError::host(format!("no window {window}")));
        }
        self.destroyed.insert(window);
        self.queue.push_back(DriverEvent::Windowed {
            window,
            event: WindowLifecycle::Destroyed,
        });
        Ok(())
    }

    /// Guard for capability-gated fallible ops.
    fn require_known(&self, window: WindowId) -> Result<&FakeWindow, UiError> {
        self.windows
            .get(&window)
            .ok_or_else(|| UiError::host(format!("no window {window}")))
    }
}

impl WindowDriver for FakeDriver {
    fn capabilities(&self) -> WindowCapabilities {
        self.capabilities.clone()
    }

    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError> {
        if self.capabilities.multi_window != ruviform_ui::Capability::Supported
            && !self.windows.is_empty()
        {
            return Err(UiError::unsupported(
                "multi_window",
                "driver does not manage more than one window",
            ));
        }
        let id = WindowId::new(self.next).expect("counter nonzero");
        self.next += 1;
        self.windows.insert(
            id,
            FakeWindow {
                spec: spec.clone(),
                ready: false,
                focused: false,
                close: CloseFlags::default(),
                backlog: VecDeque::new(),
                size: spec.logical_size,
            },
        );
        self.queue.push_back(DriverEvent::Windowed {
            window: id,
            event: WindowLifecycle::Created,
        });
        Ok(id)
    }

    fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError> {
        if self.capabilities.resize != ruviform_ui::Capability::Supported {
            return Err(UiError::unsupported(
                "resize",
                "driver cannot resize windows",
            ));
        }
        self.require_known(window)?;
        if let Some(entry) = self.windows.get_mut(&window) {
            entry.size = size;
        }
        // A real driver reports the OS-side resize back; the fake
        // echoes it deterministically (scale factor 1).
        self.inject(DriverEvent::Windowed {
            window,
            event: WindowLifecycle::Resized {
                width: size.width,
                height: size.height,
            },
        });
        Ok(())
    }

    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError> {
        if self.capabilities.focus != ruviform_ui::Capability::Supported {
            return Err(UiError::unsupported("focus", "driver cannot focus windows"));
        }
        self.require_known(window)?;
        // Like resize, the OS-side transition arrives as an event.
        self.inject(DriverEvent::Windowed {
            window,
            event: WindowLifecycle::Focused(focused),
        });
        Ok(())
    }

    fn request_close(&mut self, window: WindowId) {
        if self.windows.contains_key(&window) {
            self.inject(DriverEvent::Windowed {
                window,
                event: WindowLifecycle::CloseRequested,
            });
        }
    }

    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError> {
        let Some(entry) = self.windows.get_mut(&window) else {
            return Err(UiError::host(format!("no window {window}")));
        };
        if !entry.close.pending {
            return Err(UiError::host(format!(
                "window {window} has no pending close"
            )));
        }
        entry.close.awaiting_response = false;
        entry.close.pending = matches!(decision, CloseDecision::Defer);
        if matches!(decision, CloseDecision::Approve) {
            return self.destroy_window_inner(window);
        }
        Ok(())
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        self.destroy_window_inner(window)
    }

    fn post_event(&mut self, window: WindowId, event: &Event) {
        let Some(entry) = self.windows.get_mut(&window) else {
            return; // unknown/destroyed — silent drop per contract
        };
        if entry.ready {
            self.delivered.push((window, event.clone()));
        } else {
            if entry.backlog.len() >= MAX_EVENT_BACKLOG {
                entry.backlog.pop_front();
            }
            entry.backlog.push_back(event.clone());
        }
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
