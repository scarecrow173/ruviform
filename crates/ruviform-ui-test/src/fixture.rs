//! App-side traits the test port delegates to (Issue #312, neutral
//! re-expression in Issue #313 slice S2).
//!
//! - [`TestApp`]: the consumer's normal event wiring — exactly what a
//!   production `DriverEvent` handler does (`UiRuntime` dispatch,
//!   `post_event` delivery, close policy), plus a `shutdown` cleanup
//!   hook.
//! - [`FixtureController`]: named, app-registered fixture
//!   preparation reached through `test.fixture.prepare`. Fixtures
//!   arrange deterministic state via [`crate::TestIo`] — they never
//!   simulate user gestures (that belongs to the OS driver).
//!
//! **RT contract:** non-RT only.

use ruviform_ui::{DriverEvent, ErrorBody, WindowDriver};
use serde_json::Value;

use crate::TestIo;
use crate::proto::{FixtureReceipt, FixtureSpec, ShutdownMode};

/// The app-under-test's normal driver-event wiring.
///
/// A consumer's production `DriverEvent` handler does the same three
/// things: normalize → dispatch → deliver. Implemented once here so
/// the test host (fake or real) drives identical semantics.
pub trait TestApp {
    /// Handles one [`DriverEvent`] with the driver — the same call a
    /// real `run`/`run_multi` handler receives.
    fn on_event(&mut self, dev: DriverEvent, drv: &mut dyn WindowDriver);

    /// App-side cleanup on `test.shutdown`; the returned payload is
    /// reported verbatim as [`crate::CleanupReport::app`].
    ///
    /// Default: no app work (empty detail).
    fn shutdown(&mut self, _mode: ShutdownMode, _drv: &mut dyn WindowDriver) -> Value {
        Value::Null
    }
}

/// A named fixture the app registers with the test host.
///
/// `prepare` runs inside `test.fixture.prepare` dispatch; it arranges
/// deterministic state through `io` (create windows, emit events,
/// arm faults) and reports what it did — it must not fabricate user
/// input or mutate app state behind the port's back.
///
/// `Send` is required: a control thread may own the agent/serve loop
/// distinct from the UI thread.
pub trait FixtureController: Send {
    /// The name `FixtureSpec::name` selects.
    fn name(&self) -> &'static str;

    /// Applies the fixture.
    ///
    /// # Errors
    /// [`ErrorBody`] (`invalid_params`/`unavailable`) on bad specs or
    /// unmet preconditions.
    fn prepare(
        &mut self,
        spec: &FixtureSpec,
        io: &mut dyn TestIo,
    ) -> Result<FixtureReceipt, ErrorBody>;
}
