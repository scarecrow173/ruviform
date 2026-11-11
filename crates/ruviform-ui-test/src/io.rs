//! `TestIo` — the port through which [`crate::TestAgent`] reaches the
//! app-under-test's host boundary (Issue #312, neutral re-expression
//! in Issue #313 slice S2).
//!
//! Implemented by headless wiring ([`crate::HeadlessIo`] over
//! `FakeDriver`) and by windowed-host wiring (RF-GT-04's GPUI
//! adapter). Every method maps onto the driver-neutral
//! `ruviform-ui` port surface — `DriverEvent`/`WindowDriver`/
//! `WindowSpec`/typed `Event` — so a real adapter needs no new
//! host-facing API and no legacy `WindowEvent`/`WindowHandle`
//! vocabulary crosses the boundary.
//!
//! **RT contract:** non-RT only.

use ruviform_ui::{DriverEvent, ErrorBody, WindowId, WindowSpec};

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
    /// `window.request_close`, and fixtures. Backends that cannot
    /// fabricate a given driver-fact event drop it observably (a
    /// notice), never silently.
    fn emit(&mut self, ev: DriverEvent);

    /// Injects one raw IPC body into a window's inbound channel —
    /// the malformed-body counterpart of `emit(Action)` used by the
    /// `ipc_emit` fixture. The backend decodes at its inbound
    /// boundary, mirroring what real adapters do: an armed
    /// `ipc_disconnect` suppresses the body pre-decode (suppressed
    /// receipt + notice), a valid body becomes `Action` on the
    /// normal delivery path, and a malformed body is answered
    /// driver-side with `post_event(Event::error)` — the consumer
    /// never sees it.
    fn emit_ipc_body(&mut self, window: WindowId, body: &str);

    /// Read-only observation: window metadata snapshot.
    fn windows(&self) -> Vec<WindowMeta>;

    /// Fixture-facing window creation.
    ///
    /// # Errors
    /// [`ErrorBody`] when creation fails or the port does not support it.
    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, ErrorBody>;

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
