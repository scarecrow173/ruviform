//! Shared Window/Dock lifecycle fixture on the GPUI driver (Issue #386).
//!
//! Runs the same scenario as the `FakeDriver` test in
//! `ruviform-ui-test/tests/lifecycle_fixture.rs`, through the public
//! [`GpuiObserver`] seam (Issue #429) on a gpui test context. The
//! driver's platform loop is not started; the harness turns it one
//! step at a time. Real OS window behaviour (DPI, IME, focus, real
//! close) is manual verification and is NOT covered here.

use ruviform_ui::{Capability, UiError, WindowDriver, WindowId, WindowSpec};

use crate::{GpuiDriver, GpuiObserver};
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

/// Issue #423: `focus` is declared `Supported`, which covers focus
/// acquisition. The release direction is the per-operation
/// `Unsupported` the contract allows — the pairing must not regress
/// to a declared-but-refused acquisition or a silent release no-op.
#[test]
fn gpui_focus_capability_covers_acquire_and_reports_release_unsupported() {
    let mut driver = GpuiDriver::new();
    assert_eq!(driver.capabilities().focus, Capability::Supported);
    let window = driver
        .create_window(&WindowSpec::default())
        .expect("create window");
    driver.focus(window, true).expect("focus acquire");
    assert!(matches!(
        driver.focus(window, false),
        Err(UiError::Unsupported { .. })
    ));
}
