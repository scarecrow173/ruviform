//! Issue #388/#389 real native-dock `PoC` harness — the consumer-side
//! composition the architecture baseline reserves for the "利用側"
//! layer: `GpuiSurfaceAdapter` (ui side) + `PluginEditorSession` +
//! `InProcessHost` (plugin side) wired into one real native-editor
//! attach on a real GPUI window.
//!
//! Run: `cargo run -p ruviform-poc-native-dock --features gpui`
//! (Windows only — the surface backend is Win32 today).
//!
//! What it exercises on a real OS session:
//! - `NativeSurfaceSlot::declare_slot`/`lease_parent` → real child-HWND
//!   container under the GPUI window.
//! - `LeaseRedeemer` redemption — opaque `ParentSlotLease` →
//!   `GuiParentToken` → `Plugin::open_gui` → real native editor HWND
//!   parented under the container.
//! - `set_geometry`/`set_visibility`/`request_focus` landing on the
//!   real editor (`GetWindowRect`/`IsWindowVisible`/`GetFocus`).
//! - `close` → `close_gui` → editor destroyed; `release_lease` →
//!   container destroyed.
//!
//! This is a real-execution probe of the embedding mechanism, **not**
//! a Dock PASS: the surface lives in a GPUI window region, not inside
//! an actual dock layout (no GPUI dock widget exists yet).

#[cfg(not(all(feature = "gpui", target_os = "windows")))]
fn main() {
    eprintln!(
        "ruviform-poc-native-dock: requires `--features gpui` on Windows \
         (real GPUI window + Win32 surface backend)"
    );
    std::process::exit(2);
}

#[cfg(all(feature = "gpui", target_os = "windows"))]
mod poc;

#[cfg(all(feature = "gpui", target_os = "windows"))]
fn main() {
    poc::run();
}
