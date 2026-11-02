//! Multi-window contract — per-window identity, event envelopes,
//! routed IPC events, close decisions, and the host-driving port
//! (Issue #181).
//!
//! The single-window path ([`crate::event::HostEvent`] +
//! [`crate::event::HostHandle`]) stays untouched; this module adds the
//! multi-window counterpart so one native event loop can manage many
//! windows/webviews behind the same backend-free boundary:
//!
//! - [`WindowId`]: stable per-session window identity issued by the
//!   host — nonzero, monotonically issued, never reused after destroy.
//!   It is a host-side routing token only; it never appears in
//!   `rvf.ipc.v1` payloads.
//! - [`WindowEvent`]: what a multi-window host emits — `Windowed`
//!   carries the origin window plus the normalized [`HostEvent`]
//!   payload; `Tick` is the loop-global wakeup.
//! - [`RoutedEvent`]: what [`UiRuntime`](crate::UiRuntime) returns in
//!   multi-window mode — an [`Event`] plus its destination window.
//! - [`CloseDecision`]: the answer to a `CloseRequested` — approve
//!   (destroy now), veto (cancel), or defer (keep pending).
//! - [`WindowHandle`]: the port driving the host back (create, eval,
//!   resize, focus, close flow, monitor info, loop exit). Adapters and
//!   fake hosts implement it; no backend types appear here.
//!
//! **RT contract:** non-RT only — produced and consumed on the UI
//! thread.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::error::UiError;
use crate::event::HostEvent;
use crate::ipc::Event;

/// Stable per-window identifier within one host session.
///
/// Issued monotonically by the host starting at `1`, unique for the
/// session's lifetime, and never reused after the window is
/// destroyed — a stale id can therefore never deliver to an unrelated
/// window. This is a host-side routing token: it is never serialized
/// into `rvf.ipc.v1` wire payloads (per-webview correlation `id`s
/// remain local to their webview, which is sufficient because
/// responses always return to the issuing webview).
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(NonZeroU64);

impl WindowId {
    /// Wraps a raw value as a [`WindowId`]; `0` is rejected.
    ///
    /// Hosts normally issue ids via an internal counter — this
    /// constructor exists for adapters restoring/restating ids and
    /// for tests.
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// The raw nonzero value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "w{}", self.0)
    }
}

/// Event envelope emitted by a multi-window host.
///
/// Per-window events carry their origin [`WindowId`]; the periodic
/// wakeup is loop-global and carries none. A [`HostEvent::Tick`]
/// payload must never appear inside `Windowed` — ticks arrive as
/// [`WindowEvent::Tick`].
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum WindowEvent {
    /// A normalized event originating at one window.
    Windowed {
        /// Origin window.
        window: WindowId,
        /// Normalized event payload.
        event: HostEvent,
    },
    /// Periodic job-drain wakeup shared by the whole event loop.
    Tick,
}

/// An `rvf.ipc.v1` [`Event`] plus its delivery destination.
///
/// `window` is `Some(w)` when the event belongs to window `w`
/// (intent replies, `lifecycle` notifications, job events whose owner
/// window is known) and `None` for unrouted events — those originate
/// without a window context (`UiRuntime::dispatch_op`, legacy
/// single-window path, ownerless jobs) and the consumer picks the
/// sink (for example the primary window).
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct RoutedEvent {
    /// Destination window, or `None` when unrouted.
    pub window: Option<WindowId>,
    /// The event payload (`rvf.ipc.v1` envelope — unchanged).
    pub event: Event,
}

/// The answer to a pending `CloseRequested` for a window.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// Destroy the window now; a `Destroyed` event follows.
    Approve,
    /// Cancel the close; the window stays fully open.
    Veto,
    /// Defer the close: the window stays open marked close-pending —
    /// the consumer must later call
    /// [`WindowHandle::destroy_window`] or approve a subsequent
    /// `CloseRequested`. Use for asynchronous cleanup flows (for
    /// example a "save changes?" prompt).
    Defer,
}

/// Per-window creation parameters for [`WindowHandle::create_window`].
///
/// Session-level settings (the shared `WebContext` user-data dir, the
/// tick cadence) live on the host's session config, not here.
///
/// Consumers construct this — it is deliberately not
/// `#[non_exhaustive]`.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Initial logical width.
    pub width: u32,
    /// Initial logical height.
    pub height: u32,
    /// Initial URL — normally inside the `rvf://app` namespace.
    pub url: String,
    /// Enable backend devtools for this window's webview.
    pub devtools: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "ruviform".to_string(),
            width: 1280,
            height: 800,
            url: "rvf://app/index.html".to_string(),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Backend-agnostic monitor description.
///
/// Positions and sizes are physical pixels in virtual-desktop
/// coordinates; `scale_factor` maps logical units to physical pixels
/// (`1.0` = 100% / 96 DPI).
///
/// Adapters construct this — it is deliberately not
/// `#[non_exhaustive]`.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor name when the backend exposes one.
    pub name: Option<String>,
    /// Physical pixel width.
    pub width: u32,
    /// Physical pixel height.
    pub height: u32,
    /// Physical x position in the virtual desktop.
    pub x: i32,
    /// Physical y position in the virtual desktop.
    pub y: i32,
    /// Scale factor (`1.0` = 100%).
    pub scale_factor: f64,
}

/// Port for driving a multi-window host. Implemented by adapters
/// (for example `ruviform-ui-webview`) and by fake hosts in tests;
/// consumer code interacts only with this trait — no backend types.
///
/// Delivery safety contract: [`eval`](Self::eval) is fire-and-forget —
/// messages to unknown, destroyed, or not-yet-ready destinations are
/// never errors and never panic (not-yet-ready windows buffer a
/// bounded backlog, drop-oldest). The fallible operations report
/// unknown/dead windows as [`UiError::Host`] instead. Window loss of
/// any kind terminates as [`HostEvent::Destroyed`].
///
/// **RT contract:** non-RT only — called on the UI thread.
pub trait WindowHandle {
    /// Evaluates `script` in `window`'s webview. Fire-and-forget:
    /// unknown or destroyed windows are silently dropped, and
    /// not-yet-ready windows hold a bounded backlog flushed on
    /// `Ready`. A dead destination must never wedge or crash the
    /// host.
    fn eval(&mut self, window: WindowId, script: &str);

    /// Requests a new logical size for `window`.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed or the
    /// backend rejects the request.
    fn resize(&mut self, window: WindowId, width: u32, height: u32) -> Result<(), UiError>;

    /// Requests or releases focus on `window`.
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` is unknown/destroyed or the
    /// backend rejects the request.
    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError>;

    /// Creates a new window + webview with `config` and returns its
    /// freshly issued [`WindowId`].
    ///
    /// # Errors
    /// [`UiError::Host`]/[`UiError::BackendUnavailable`] when window
    /// or webview creation fails; a failed create leaves no
    /// observable window behind.
    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, UiError>;

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
