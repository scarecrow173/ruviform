//! Shared Window/Dock lifecycle fixture on the `FakeDriver` (Issue #386).
//!
//! The same scenario runs on the GPUI driver in
//! `ruviform-ui-gpui/src/lifecycle_fixture.rs`; both must produce
//! `expected_trace()`.

use std::collections::BTreeSet;

use ruviform_ui::{DriverEvent, WindowDriver, WindowId, WindowLifecycle};
use ruviform_ui_test::FakeDriver;
use ruviform_ui_test::lifecycle::{Handler, LifecycleHarness, expected_trace, run_lifecycle};

#[derive(Default)]
struct FakeHarness {
    driver: FakeDriver,
    readied: BTreeSet<WindowId>,
}

impl LifecycleHarness for FakeHarness {
    fn driver(&mut self) -> &mut dyn WindowDriver {
        &mut self.driver
    }

    fn settle(&mut self, mut on_event: Handler) {
        // The fake has no surface to present: every open window turns
        // Ready on its first turn, as a driver does after its first frame.
        for window in self.driver.windows() {
            if self.readied.insert(window) {
                self.driver.inject(DriverEvent::Windowed {
                    window,
                    event: WindowLifecycle::Ready,
                });
            }
        }
        self.driver.pump(&mut *on_event);
    }

    fn exited(&self) -> bool {
        self.driver.exited()
    }

    fn received(&self, window: WindowId) -> usize {
        self.driver.events_for(window).len()
    }
}

#[test]
fn fake_driver_matches_shared_lifecycle_trace() {
    let mut harness = FakeHarness::default();
    let trace = run_lifecycle(&mut harness);
    assert_eq!(trace, expected_trace());
    assert!(harness.exited());
}
