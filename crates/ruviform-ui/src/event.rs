//! Normalized host events and the host-control port.
//!
//! **Legacy (Issue #465):** this module is the WebView-adapter
//! compatibility surface. Adapters (for example
//! `ruviform-ui-webview`) translate their
//! backend's native window/webview events into [`HostEvent`]. The
//! variants deliberately mirror the event surface the `poc/ui` harness
//! proved sufficient: page-load readiness, raw IPC payloads, resize,
//! focus, scale-factor changes, close requests and periodic ticks.
//!
//! The driver-neutral migration target is [`crate::driver`]:
//! `WindowLifecycle` carries the same lifecycle semantics without the
//! WebView-specific `Ready { url }` and `Message(String)` payloads
//! (`Action(Intent)` replaces the raw message string). This module
//! stays available for the `WebView` adapter and existing consumers
//! until Issue #465 P2 decides physical removal.
//!
//! [`HostHandle`] is the port through which application code drives the
//! host back (script eval, resize, focus, close). It contains no
//! backend types, so headless tests can substitute fakes. `eval` is
//! JS-execution specific to the `WebView` adapter; the driver-neutral
//! counterpart is [`crate::driver::WindowDriver`].

use crate::error::UiError;

/// Backend-normalized host event delivered to the UI runtime.
///
/// **Legacy (Issue #465):** WebView-adapter compatibility surface —
/// migration target is [`crate::driver::WindowLifecycle`]. Kept for
/// the `WebView` adapter and current consumers; not a parity target
/// for new drivers.
///
/// **RT contract:** non-RT only — produced and consumed on the UI
/// thread.
#[non_exhaustive]
#[derive(Debug)]
pub enum HostEvent {
    /// The frontend page finished loading and the bridge is live.
    Ready {
        /// URL the webview navigated to (the `rvf://app` origin).
        url: String,
    },
    /// Raw IPC message body sent by the frontend
    /// (`window.ipc.postMessage`). Expected to hold an `rvf.ipc.v1`
    /// intent JSON document; parsing happens in the runtime so the
    /// same contract is exercised headless.
    Message(String),
    /// The window was resized; physical pixels.
    Resized {
        /// Physical pixel width.
        width: u32,
        /// Physical pixel height.
        height: u32,
    },
    /// The window focus state changed.
    Focused(bool),
    /// The window moved to a monitor with a different scale factor.
    ScaleFactorChanged {
        /// New scale factor (for example `1.5` at 150%).
        factor: f64,
    },
    /// The window moved; physical position in virtual-desktop
    /// coordinates (Issue #181).
    Moved {
        /// Physical x position.
        x: i32,
        /// Physical y position.
        y: i32,
    },
    /// The user or OS requested the window to close.
    ///
    /// Single-window path (`host::run` + `UiRuntime::handle`): the
    /// host exits its run loop after delivering this event (v1: no
    /// veto). Multi-window path
    /// ([`crate::window`]): this is a veto-able request answered via
    /// `WindowHandle::respond_close` — it neither destroys the window
    /// by itself nor cancels jobs nor exits the loop.
    CloseRequested,
    /// The window/webview was destroyed (Issue #181) — after an
    /// approved close, a programmatic `destroy_window`, or abnormal
    /// backend teardown. This is the terminal event for a
    /// [`crate::window::WindowId`]; the id is retired and never
    /// reused, and further deliveries to it are dropped.
    Destroyed,
    /// Periodic wakeup requested via `HostConfig::tick` — the drain
    /// point for job progress mailboxes.
    Tick,
}

/// Port for driving the host back. Implemented by adapters; the UI
/// runtime and services interact only with this trait.
///
/// **Legacy (Issue #465):** WebView-adapter compatibility surface —
/// migration target is [`crate::driver::WindowDriver`]. `eval` is
/// JS-execution specific and has no driver-neutral counterpart.
///
/// **RT contract:** non-RT only — called on the UI thread.
pub trait HostHandle {
    /// Evaluates `script` in the webview's JS context. Delivery
    /// failures are logged by the adapter; eval is fire-and-forget
    /// because a dead page must not wedge the host loop.
    fn eval(&mut self, script: &str);

    /// Requests a new logical size for the window.
    ///
    /// # Errors
    /// Returns [`UiError::Host`] when the backend rejects the request.
    fn resize(&mut self, width: u32, height: u32) -> Result<(), UiError>;

    /// Requests or releases window focus.
    ///
    /// # Errors
    /// Returns [`UiError::Host`] when the backend rejects the request.
    fn focus(&mut self, focused: bool) -> Result<(), UiError>;

    /// Asks the host to terminate its run loop at the next turn.
    fn request_close(&mut self);
}
