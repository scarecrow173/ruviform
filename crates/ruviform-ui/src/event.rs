//! Normalized host events and the host-control port.
//!
//! Adapters (for example `ruviform-ui-webview`) translate their
//! backend's native window/webview events into [`HostEvent`]. The
//! variants deliberately mirror the event surface the `poc/ui` harness
//! proved sufficient: page-load readiness, raw IPC payloads, resize,
//! focus, scale-factor changes, close requests and periodic ticks.
//!
//! [`HostHandle`] is the port through which application code drives the
//! host back (script eval, resize, focus, close). It contains no
//! backend types, so headless tests can substitute fakes.

use crate::error::UiError;

/// Backend-normalized host event delivered to the UI runtime.
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
    /// The user or OS requested the window to close. The host exits
    /// its run loop after delivering this event (v1: no veto).
    CloseRequested,
    /// Periodic wakeup requested via `HostConfig::tick` — the drain
    /// point for job progress mailboxes.
    Tick,
}

/// Port for driving the host back. Implemented by adapters; the UI
/// runtime and services interact only with this trait.
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
