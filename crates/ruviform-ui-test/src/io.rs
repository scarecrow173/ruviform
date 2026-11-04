//! `TestIo` — the port through which [`crate::TestAgent`] reaches the
//! app-under-test's host boundary (Issue #312).
//!
//! Implemented by headless wiring ([`crate::HeadlessIo`]) and by
//! windowed-host wiring (RF-GT-04 consumes it over `run_multi`). Every
//! method maps onto existing `ruviform-ui` port surface —
//! `WindowEvent`/`WindowHandle` — so a real adapter needs no new
//! host-facing API.
//!
//! **RT contract:** non-RT only.

use ruviform_ui::{ErrorBody, WindowConfig, WindowEvent, WindowId};

use crate::proto::{
    BackendMeta, Capabilities, CleanupReport, FaultKind, FixtureReceipt, FixtureSpec, Notice,
    ReadyState, ReceiptPage, Screenshot, ShutdownMode, WindowMeta,
};

/// The agent↔app bridge. All methods are called by `TestAgent` on
/// whichever thread the app drives the agent from.
pub trait TestIo {
    /// Readiness asked at `test.hello` — `Failed` answers the failed
    /// half of the handshake.
    fn ready_state(&self) -> ReadyState;

    /// The truthfully-declared capability set — enforced per op.
    fn capabilities(&self) -> Capabilities;

    /// Backend identity reported in the handshake.
    fn backend(&self) -> BackendMeta;

    /// Injects one host-side event into the app (the same delivery a
    /// real event loop performs). Used by `clock.advance`,
    /// `window.request_close`, and fixtures.
    fn emit(&mut self, ev: WindowEvent);

    /// Read-only observation: window metadata snapshot.
    fn windows(&self) -> Vec<WindowMeta>;

    /// Fixture-facing window creation.
    ///
    /// # Errors
    /// [`ErrorBody`] when creation fails or the port does not support it.
    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, ErrorBody>;

    /// Read-only observation: paged receipt journal.
    fn receipts(&self, since: u64, limit: usize) -> ReceiptPage;

    /// Drains pending asynchronous notices.
    fn notices(&mut self) -> Vec<Notice>;

    /// Arms (`on = true`) or disarms an allowlisted fault.
    ///
    /// # Errors
    /// [`ErrorBody`] (`unavailable`/`invalid_params`) for unsupported
    /// kinds, unknown windows, or invalid clear requests.
    fn set_fault(
        &mut self,
        kind: FaultKind,
        window: Option<WindowId>,
        on: bool,
    ) -> Result<(), ErrorBody>;

    /// Runs a registered fixture by name.
    ///
    /// # Errors
    /// [`ErrorBody`] (`invalid_params`/`unavailable`) for unknown
    /// names or failed preparation.
    fn prepare_fixture(&mut self, spec: &FixtureSpec) -> Result<FixtureReceipt, ErrorBody>;

    /// Captures a screenshot of `window`.
    ///
    /// # Errors
    /// [`ErrorBody`] (`unavailable`) when the backend cannot capture —
    /// the honest answer on every backend without a real capture path.
    fn screenshot(&mut self, window: WindowId) -> Result<Screenshot, ErrorBody>;

    /// Runs shutdown/cleanup and reports it. `Graceful` tears windows
    /// down and runs the app's cleanup hook; `Now` exits immediately.
    fn request_shutdown(&mut self, mode: ShutdownMode) -> CleanupReport;
}
