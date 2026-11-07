//! Driver-neutral window/host contract (Issue #386).
//!
//! This module is the backend-neutral semantic contract every GUI
//! driver implements — GPUI (the standard driver), the legacy
//! `wry`/`tao` `WebView` adapter, and test fakes alike. It deliberately
//! carries **no** backend types: no `gpui::Window`/`Entity`, no DOM or
//! JS handles, no `wry`/`tao` types, and no `HWND`/`NSView`/X11
//! handles. Those live inside the driver adapter (for example
//! `ruviform-ui-gpui`) or the driver-specific extension that opts in.
//!
//! Relationship to the legacy contract ([`crate::event`],
//! [`crate::window`]): this module is the migration target, added
//! alongside the existing types without breaking them. The old
//! `WindowConfig`/`WindowHandle`/`HostEvent` surface stays available
//! for the `WebView` adapter until the migration table in
//! `docs/designs/issue-386-driver-neutral-gui-host.md` retires it.
//!
//! - [`WindowSpec`]: neutral per-window creation parameters — title,
//!   logical size, position, resizability. Unlike the legacy
//!   [`crate::window::WindowConfig`] it carries no JS `url` or
//!   `devtools` flag (those belong to the `WebView` adapter's own
//!   config).
//! - [`WindowLifecycle`]: what a driver emits for one window —
//!   `Created`, `Ready`, geometry/focus/scale changes,
//!   `CloseRequested`, `Destroyed`, and the typed action channel
//!   `Action(Intent)`. `Action` replaces the raw-string
//!   `HostEvent::Message`: drivers produce typed
//!   [`crate::ipc::Intent`]s directly (the `WebView` adapter decodes
//!   `postMessage` bodies into them) instead of pushing JS.
//! - [`DriverEvent`]: the multi-window envelope — `Windowed`
//!   carries the origin window plus a [`WindowLifecycle`] payload;
//!   `Tick` is the loop-global job-drain wakeup.
//! - [`WindowDriver`]: the port driving the host back — create,
//!   resize, focus, close flow, monitor info, typed event push, loop
//!   exit, capability snapshot. It is [`crate::window::WindowHandle`]
//!   minus `eval`, plus `post_event` and `capabilities`.
//!
//! Consumer routing of [`DriverEvent`] into [`crate::UiRuntime`]:
//! `Windowed { event: Action(intent) }` goes to
//! [`crate::UiRuntime::dispatch_window_intent`]; `Tick` goes to
//! [`crate::UiRuntime::drain_jobs_routed`]; `Windowed { event:
//! CloseRequested }` emits the `closing` lifecycle event (the same
//! mapping [`crate::UiRuntime::handle_window`] applies to the
//! legacy `HostEvent`); `Destroyed` lets the consumer apply its own
//! teardown policy (for example
//! [`crate::UiRuntime::cancel_window_jobs`]). Remaining lifecycle
//! variants carry no runtime semantics and are consumer UI state.
//!
//! **RT contract:** non-RT only — produced and consumed on the UI
//! thread.

use crate::error::UiError;
use crate::ipc::{Event, Intent};
use crate::window::{CloseDecision, MonitorInfo, WindowId};

/// Logical size in logical pixels (unscaled, before the window's
/// scale factor is applied).
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalSize {
    /// Logical width.
    pub width: u32,
    /// Logical height.
    pub height: u32,
}

impl LogicalSize {
    /// Creates a logical size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Logical position in virtual-desktop coordinates (unscaled).
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalPosition {
    /// Logical x coordinate.
    pub x: i32,
    /// Logical y coordinate.
    pub y: i32,
}

impl LogicalPosition {
    /// Creates a logical position.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Per-window creation parameters for [`WindowDriver::create_window`].
///
/// Unlike the legacy [`crate::window::WindowConfig`], this is
/// driver-neutral: it carries no JS `url` and no `devtools` flag.
/// Session-level settings (tick cadence, shared driver state) live on
/// the driver's session config, not here.
///
/// Consumers construct this — it is deliberately not
/// `#[non_exhaustive]`.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct WindowSpec {
    /// Window title.
    pub title: String,
    /// Initial logical size.
    pub logical_size: LogicalSize,
    /// Initial logical position in virtual-desktop coordinates, or
    /// `None` for the platform default placement — used by layout
    /// restore (Issue #182).
    pub position: Option<LogicalPosition>,
    /// Whether the user may resize the window.
    pub resizable: bool,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: "ruviform".to_string(),
            logical_size: LogicalSize::new(1280, 800),
            position: None,
            resizable: true,
        }
    }
}

/// Driver-neutral per-window event payload — the migration
/// successor of [`crate::event::HostEvent`].
///
/// The `Ready`/`Resized`/`Moved`/`Focused`/`ScaleFactorChanged`/
/// `CloseRequested`/`Destroyed` variants map one-to-one onto the
/// legacy payload (minus `Ready`'s `WebView` `url`), and `Action`
/// replaces the raw `Message(String)` channel with the typed
/// `rvf.ipc.v1` [`Intent`].
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum WindowLifecycle {
    /// The host accepted the window and issued its [`WindowId`].
    /// The window may not be visible or interactive yet — `Ready`
    /// follows once it is.
    Created,
    /// The window became visible and interactive — its first frame
    /// was presented and it can take input. This is the
    /// driver-neutral meaning of the legacy
    /// `HostEvent::Ready { url }`; the navigation URL is a
    /// `WebView`-adapter concern and stays inside that adapter.
    Ready,
    /// The window was resized; physical pixels.
    Resized {
        /// Physical pixel width.
        width: u32,
        /// Physical pixel height.
        height: u32,
    },
    /// The window moved; physical position in virtual-desktop
    /// coordinates.
    Moved {
        /// Physical x position.
        x: i32,
        /// Physical y position.
        y: i32,
    },
    /// The window focus state changed.
    Focused(bool),
    /// The window moved to a monitor with a different scale factor.
    ScaleFactorChanged {
        /// New scale factor (for example `1.5` at 150%).
        factor: f64,
    },
    /// The user or OS requested the window to close.
    ///
    /// This is a veto-able request answered via
    /// [`WindowDriver::respond_close`] — it neither destroys the
    /// window by itself nor cancels jobs nor exits the loop.
    CloseRequested,
    /// The window was destroyed — after an approved close, a
    /// programmatic [`WindowDriver::destroy_window`], or abnormal
    /// driver teardown. This is the terminal event for a
    /// [`WindowId`]; the id is retired and never reused, and further
    /// deliveries to it are dropped.
    Destroyed,
    /// A typed `rvf.ipc.v1` action destined for
    /// [`crate::UiRuntime`]. Drivers produce [`Intent`]s directly
    /// (GPUI views construct them in Rust; the `WebView` adapter
    /// decodes `postMessage` bodies via
    /// [`crate::ipc::parse_intent`]). The runtime dispatches them
    /// through [`crate::UiRuntime::dispatch_window_intent`], which
    /// takes the same routing the legacy `Message` branch takes.
    Action(Intent),
}

/// Event envelope emitted by a multi-window driver.
///
/// Per-window events carry their origin [`WindowId`]; the periodic
/// wakeup is loop-global and carries none — the same structure the
/// legacy [`crate::window::WindowEvent`] uses.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum DriverEvent {
    /// A normalized event originating at one window.
    Windowed {
        /// Origin window.
        window: WindowId,
        /// Lifecycle payload.
        event: WindowLifecycle,
    },
    /// Periodic job-drain wakeup shared by the whole event loop.
    Tick,
}

/// Whether the driver supports a capability.
///
/// A driver that cannot honour a request reports [`Unsupported`]
/// from `capabilities()` and returns [`UiError::Unsupported`] from
/// the operation — it never silently degrades or fakes the result.
/// Capabilities not listed on [`WindowCapabilities`] are treated as
/// unsupported.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Capability {
    /// The driver implements the capability.
    Supported,
    /// The driver does not implement the capability; the operation
    /// fails with [`UiError::Unsupported`].
    #[default]
    Unsupported,
}

/// Snapshot of a driver's window-level capabilities.
///
/// `native_surface_slot` is the declared plug point for the
/// `NativeSurfaceSlot` semantic contract (Issue #388) — this
/// contract carries only the flag, not the slot type.
///
/// Drivers construct this — it is deliberately not
/// `#[non_exhaustive]`. [`Default`] reports everything unsupported,
/// which is also what `windows()`/`monitors()`-less drivers should
/// leave in place.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowCapabilities {
    /// Accepts [`WindowDriver::resize`] requests.
    pub resize: Capability,
    /// Accepts [`WindowDriver::focus`] requests with `focused = true`
    /// and emits [`WindowLifecycle::Focused`] transitions. Releasing
    /// focus (`focused = false`) is not implied by this flag: a driver
    /// whose platform cannot release focus keeps `Supported` here and
    /// answers the release with [`UiError::Unsupported`] per operation.
    pub focus: Capability,
    /// Emits [`WindowLifecycle::ScaleFactorChanged`] transitions.
    pub scale_factor: Capability,
    /// Manages more than one window in the same session.
    pub multi_window: Capability,
    /// Routes IME input to views (Input Method Editor).
    /// Forward-declared — the IME event surface belongs to the
    /// editor-session contract (Issue #389), not this module.
    pub ime: Capability,
    /// Hosts a `NativeSurfaceSlot` on managed windows (Issue #388).
    pub native_surface_slot: Capability,
}

/// Port for driving a driver-neutral multi-window host. Implemented
/// by drivers (for example `ruviform-ui-gpui`) and by fake drivers in
/// tests; consumer code interacts only with this trait — no backend
/// types.
///
/// Delivery safety contract mirrors the legacy
/// [`crate::window::WindowHandle`]: [`post_event`](Self::post_event)
/// is fire-and-forget — events to unknown, destroyed, or not-yet-ready
/// destinations are never errors and never panic (not-yet-ready
/// windows buffer a bounded backlog, drop-oldest). The fallible
/// operations report unknown/dead windows as [`UiError::Host`] and
/// unsupported capabilities as [`UiError::Unsupported`]. Window loss
/// of any kind terminates as [`WindowLifecycle::Destroyed`].
///
/// **RT contract:** non-RT only — called on the UI thread.
pub trait WindowDriver {
    /// Snapshot of the driver's declared capabilities — returned by
    /// value so interior-mutable drivers (event loops owning their
    /// state behind a cell) can implement it.
    fn capabilities(&self) -> WindowCapabilities;

    /// Creates a new window from `spec` and returns its freshly
    /// issued [`WindowId`]. The driver emits
    /// [`WindowLifecycle::Created`] once it accepts the window and
    /// [`WindowLifecycle::Ready`] once it is visible and
    /// interactive.
    ///
    /// # Errors
    /// [`UiError::Host`]/[`UiError::BackendUnavailable`] when window
    /// creation fails; [`UiError::Unsupported`] when the driver
    /// declares `multi_window` unsupported and a further window is
    /// requested. A failed create leaves no observable window
    /// behind.
    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError>;

    /// Requests a new logical size for `window`.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed or the
    /// driver rejects the request; [`UiError::Unsupported`] when the
    /// driver declares `resize` unsupported.
    fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError>;

    /// Requests focus on `window` (`focused = true`), or releases it
    /// (`focused = false`) where the platform allows.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed or the
    /// driver rejects the request; [`UiError::Unsupported`] when the
    /// driver declares `focus` unsupported, or when it cannot honour
    /// the requested direction (for example a release on a platform
    /// without one — see [`WindowCapabilities::focus`]).
    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError>;

    /// Asks `window` to close through the normal `CloseRequested`
    /// flow — equivalent to the user clicking the close button: the
    /// request is veto-able and never destroys the window by itself.
    /// Unknown windows are ignored.
    fn request_close(&mut self, window: WindowId);

    /// Answers a pending `CloseRequested` for `window`.
    ///
    /// `Approve` destroys the window (`Destroyed` follows); `Veto`
    /// cancels the close; `Defer` leaves it close-pending. An
    /// unanswered `CloseRequested` defaults to `Approve` for that
    /// window only — never for the whole loop or app.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed or has no
    /// pending close request.
    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError>;

    /// Destroys `window` immediately, bypassing `CloseRequested`;
    /// a `Destroyed` event follows.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed.
    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError>;

    /// Delivers a typed `rvf.ipc.v1` [`Event`] to `window`'s view —
    /// the driver-neutral successor of script `eval`. Fire-and-forget:
    /// unknown or destroyed windows are silently dropped, and
    /// not-yet-ready windows hold a bounded backlog flushed on
    /// `Ready`. A dead destination must never wedge or crash the
    /// host. How the event reaches the view is the driver's business
    /// (eval script, entity update, IPC post) — never the caller's.
    ///
    /// Never fails on delivery grounds — implementations report
    /// delivery failures through logging or their own diagnostics
    /// channel, not [`UiError`].
    fn post_event(&mut self, window: WindowId, event: &Event);

    /// Ids of currently living windows — the input to the consumer's
    /// last-window / app-exit policy.
    fn windows(&self) -> Vec<WindowId>;

    /// Snapshot of attached monitors (backend-agnostic).
    fn monitors(&self) -> Vec<MonitorInfo>;

    /// Exits the host event loop — the application-level shutdown.
    /// The loop never exits on its own when the last window closes;
    /// that policy belongs to the consumer.
    fn exit(&mut self);
}
