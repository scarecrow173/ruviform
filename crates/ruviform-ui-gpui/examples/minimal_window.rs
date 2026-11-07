//! Minimal GPUI window fixture (Issue #386) — manual verification
//! entry point for the driver-neutral `WindowDriver` contract on the
//! real GPUI backend.
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example
//! minimal_window`
//!
//! What it exercises on a real OS session:
//! - `WindowSpec` → `WindowOptions` mapping (title, logical size,
//!   resizable).
//! - `Created`/`Ready`/`Resized`/`Moved`/`Focused`/`ScaleFactorChanged`
//!   lifecycle delivery.
//! - Veto-able `CloseRequested` → `respond_close` → terminal
//!   `Destroyed` — close the window and watch the journal.
//! - `post_event` typed push + `IntentSink` typed action ingress.
//! - `exit` loop shutdown.
//!
//! This is the fixture RF388/RF389 extend for real native Dock and
//! plugin editor work; it is NOT evidence of native Dock success —
//! it verifies the window lifecycle contract only.

use ruviform_ui::{DriverEvent, WindowDriver, WindowLifecycle, WindowSpec};
use ruviform_ui_gpui::GpuiDriver;

fn main() {
    let mut driver = GpuiDriver::new();
    eprintln!("[fixture] capabilities: {:?}", driver.capabilities());
    let main = driver
        .create_window(&WindowSpec {
            title: "ruviform — minimal gpui window".to_string(),
            ..WindowSpec::default()
        })
        .expect("create_window");
    eprintln!("[fixture] window id: {main}");

    driver
        .run(None, move |ev, drv| {
            if let DriverEvent::Windowed { window, event } = &ev {
                match event {
                    WindowLifecycle::Action(intent) => {
                        eprintln!("[event] {window}: Action op={}", intent.op);
                    }
                    WindowLifecycle::CloseRequested => {
                        eprintln!("[event] {window}: CloseRequested — approve");
                        if let Err(e) =
                            drv.respond_close(*window, ruviform_ui::CloseDecision::Approve)
                        {
                            eprintln!("[event] respond_close failed: {e}");
                        }
                    }
                    WindowLifecycle::Destroyed => {
                        eprintln!("[event] {window}: Destroyed — exiting loop");
                        drv.exit();
                    }
                    other => eprintln!("[event] {window}: {other:?}"),
                }
            }
        })
        .expect("run");
    eprintln!("[fixture] exited cleanly");
}
