//! Driver Lab GPUI bindings (Issue #426): the GPUI-layer tests named by
//! `ruviform-ui-test/fixtures/driver-lab/cases.json` for UI-DVR-01, 07
//! and 08. Each test runs on the real GPUI test-support path
//! (`TestAppContext`, `gpui-test` feature). Real OS window behaviour is
//! manual verification (#313) and is NOT covered here.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ruviform_ui::data::DataRegistry;
use ruviform_ui::demo::DemoService;
use ruviform_ui::{
    Capability, DriverEvent, Event, HostEvent, JobState, LogicalSize, UiRuntime, WindowDriver,
    WindowLifecycle, WindowSpec,
};

use crate::GpuiDriver;

type Events = Rc<RefCell<Vec<DriverEvent>>>;

fn settle_collecting(driver: &mut GpuiDriver, cx: &mut gpui::TestAppContext) -> Events {
    let events: Events = Rc::default();
    let sink = events.clone();
    driver.settle_for_test(cx, move |ev, _| sink.borrow_mut().push(ev));
    events
}

/// UI-DVR-01 GPUI lane: a driverless-contract window opens on the GPUI
/// backend under test-support and reaches `Created` → `Ready`.
#[gpui::test]
fn ui_dvr_01_gpui_headless_smoke_window_opens(cx: &mut gpui::TestAppContext) {
    let mut driver = GpuiDriver::new();
    let window = driver
        .create_window(&WindowSpec::default())
        .expect("create window");
    let events = settle_collecting(&mut driver, cx);
    assert!(events.borrow().iter().any(|e| matches!(
        e,
        DriverEvent::Windowed { window: w, event: WindowLifecycle::Created } if *w == window
    )));
    assert!(events.borrow().iter().any(|e| matches!(
        e,
        DriverEvent::Windowed { window: w, event: WindowLifecycle::Ready } if *w == window
    )));
}

/// UI-DVR-07 GPUI lane: the declared capability map is honest. Supported
/// window operations really succeed, IME stays `Unsupported` (owned by
/// #389), and `native_surface_slot` reports exactly the backend the
/// build has.
#[gpui::test]
fn ui_dvr_07_gpui_capabilities_are_honest(cx: &mut gpui::TestAppContext) {
    let mut driver = GpuiDriver::new();
    let caps = driver.capabilities();
    assert_eq!(caps.resize, Capability::Supported);
    assert_eq!(caps.focus, Capability::Supported);
    assert_eq!(caps.multi_window, Capability::Supported);
    assert_eq!(caps.ime, Capability::Unsupported, "IME routing is #389");
    // Pinned to the per-OS container backend (`surface::BACKEND_AVAILABLE`,
    // `pub(super)` so not reachable from this module): Win32 only today.
    let slot = if cfg!(target_os = "windows") {
        Capability::Supported
    } else {
        Capability::Unsupported
    };
    assert_eq!(caps.native_surface_slot, slot);

    let first = driver
        .create_window(&WindowSpec::default())
        .expect("create first window");
    let second = driver
        .create_window(&WindowSpec::default())
        .expect("multi_window: second window");
    settle_collecting(&mut driver, cx);
    driver
        .resize(
            first,
            LogicalSize {
                width: 640,
                height: 480,
            },
        )
        .expect("declared-supported resize runs");
    driver
        .focus(second, true)
        .expect("declared-supported focus");
}

/// UI-DVR-08 GPUI lane: a window teardown reaches the terminal
/// `Destroyed` state, and the runtime's window-scoped job cancel (the
/// consumer close policy) ends the window's running job with a
/// `Cancelled` terminal event.
#[gpui::test]
fn ui_dvr_08_gpui_window_teardown_cancels_jobs(cx: &mut gpui::TestAppContext) {
    let mut driver = GpuiDriver::new();
    let window = driver
        .create_window(&WindowSpec::default())
        .expect("create window");
    settle_collecting(&mut driver, cx);

    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(DemoService::new())
        .expect("demo service");
    let msg = r#"{"v":1,"kind":"intent","id":"gpui-08","op":"demo.sleep","params":{"ms":60000}}"#
        .to_string();
    let _ = rt.handle_window(window, HostEvent::Message(msg));
    assert!(
        rt.has_running_jobs(),
        "window job is running before teardown"
    );

    driver.destroy_window(window).expect("destroy window");
    let events = settle_collecting(&mut driver, cx);
    assert!(events.borrow().iter().any(|e| matches!(
        e,
        DriverEvent::Windowed { window: w, event: WindowLifecycle::Destroyed } if *w == window
    )));

    assert!(
        rt.cancel_window_jobs(window) >= 1,
        "window job is cancelled"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut all = Vec::new();
    while rt.has_running_jobs() && Instant::now() < deadline {
        all.extend(rt.poll(Some(Duration::from_millis(50))));
    }
    all.extend(rt.drain_jobs());
    assert!(!rt.has_running_jobs(), "no job survives teardown");
    assert!(
        all.iter().any(|e| matches!(
            e,
            Event::Job {
                state: JobState::Cancelled,
                ..
            }
        )),
        "cancel terminal event is emitted: {all:?}"
    );
}
