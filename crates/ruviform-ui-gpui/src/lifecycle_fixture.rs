//! Shared Window/Dock lifecycle fixture on the GPUI driver (Issue #386).
//!
//! Runs the same scenario as the `FakeDriver` test in
//! `ruviform-ui-test/tests/lifecycle_fixture.rs`, through the public
//! [`GpuiObserver`] seam (Issue #429) on a gpui test context. The
//! driver's platform loop is not started; the harness turns it one
//! step at a time. Real OS window behaviour (DPI, IME, focus, real
//! close) is manual verification and is NOT covered here.

use ruviform_ui::{WindowDriver, WindowId};

use crate::GpuiObserver;
use ruviform_ui_test::lifecycle::{Handler, LifecycleHarness, expected_trace, run_lifecycle};

struct GpuiHarness {
    observer: GpuiObserver,
}

impl LifecycleHarness for GpuiHarness {
    fn driver(&mut self) -> &mut dyn WindowDriver {
        self.observer.driver_mut()
    }

    fn settle(&mut self, on_event: Handler) {
        self.observer.settle(on_event);
    }

    fn exited(&self) -> bool {
        self.observer.driver().exited()
    }

    fn received(&self, window: WindowId) -> usize {
        self.observer.inbox_len(window)
    }
}

#[test]
fn gpui_driver_matches_shared_lifecycle_trace() {
    let mut harness = GpuiHarness {
        observer: GpuiObserver::new(),
    };
    let trace = run_lifecycle(&mut harness);
    assert_eq!(trace, expected_trace());
    assert!(harness.observer.driver().exited());
}
