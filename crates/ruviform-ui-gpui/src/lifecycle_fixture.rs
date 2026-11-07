//! Shared Window/Dock lifecycle fixture on the GPUI driver (Issue #386).
//!
//! Runs the same scenario as the `FakeDriver` test in
//! `ruviform-ui-test/tests/lifecycle_fixture.rs`, inside a
//! `TestAppContext` (gpui test-support, `gpui-test` feature). The
//! driver's platform loop is not started; the harness turns it one
//! step at a time. Real OS window behaviour (DPI, IME, focus, real
//! close) is manual verification and is NOT covered here.

use ruviform_ui::{WindowDriver, WindowId};

use crate::GpuiDriver;
use ruviform_ui_test::lifecycle::{Handler, LifecycleHarness, expected_trace, run_lifecycle};

struct GpuiHarness<'a> {
    cx: &'a mut gpui::TestAppContext,
    driver: GpuiDriver,
}

impl LifecycleHarness for GpuiHarness<'_> {
    fn driver(&mut self) -> &mut dyn WindowDriver {
        &mut self.driver
    }

    fn settle(&mut self, on_event: Handler) {
        self.driver.settle_for_test(self.cx, on_event);
    }

    fn exited(&self) -> bool {
        self.driver.exited()
    }

    fn received(&self, window: WindowId) -> usize {
        self.driver.inbox_len_for_test(window)
    }
}

#[gpui::test]
fn gpui_driver_matches_shared_lifecycle_trace(cx: &mut gpui::TestAppContext) {
    let mut harness = GpuiHarness {
        cx,
        driver: GpuiDriver::new(),
    };
    let trace = run_lifecycle(&mut harness);
    assert_eq!(trace, expected_trace());
    assert!(harness.exited());
}
