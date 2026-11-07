//! `ruviform-ui-webview` — `wry` + `tao` adapter implementing the
//! driver-neutral `ruviform-ui` [`WindowDriver`](ruviform_ui::WindowDriver)
//! port (Issue #156; contract mapping Issue #423).
//!
//! The adapter owns everything backend-shaped so `ruviform-ui` stays
//! dependency-free:
//!
//! - [`session`]: windowed [`run_multi`] — tao event loop, wry
//!   webviews, `EventLoopProxy` wake-up, `set_bounds` resize follow,
//!   tick-driven job drain. Emits [`DriverEvent`](ruviform_ui::DriverEvent)s
//!   and accepts a [`WindowDriver`](ruviform_ui::WindowDriver) for driving
//!   back (create, resize, focus, close flow, `post_event`, exit).
//! - [`host`] (crate-private): per-window wry wiring that decodes page
//!   load into `WindowLifecycle::Ready` and IPC bodies into typed
//!   `WindowLifecycle::Action(Intent)`.
//! - `scheme`: `rvf://app` + `rvf://data` custom-protocol handler —
//!   asset serving, bounded pulls, traversal rejection, CORS. Pure;
//!   headless-tested.
//! - [`embed`]: `rust-embed`-backed [`ruviform_ui::AssetSource`] for
//!   the production single-binary layout.
//! - [`detect`]: `wry::webview_version()` backend probe (no window).
//!
//! Rust→frontend pushes go through `post_event`, which the adapter
//! realises as `evaluate_script(dispatch_script(..))` — the
//! `window.__rvf` bridge is injected by the host. The legacy
//! `HostEvent`/`HostHandle`/`WindowConfig` types are not used by this
//! crate (migration table H1–H5 in
//! `docs/designs/issue-386-driver-neutral-gui-host.md`).
//!
//! ## Feature gate
//!
//! Everything backend-dependent lives behind the non-default
//! `webview` feature — `cargo check --workspace` stays a headless
//! verification path. OS runtime requirements: `WebView2` Evergreen
//! (Windows), `WKWebView` (macOS, system-provided),
//! `libwebkit2gtk-4.1` (Linux).
//!
//! ## Dependency layers
//!
//! Optional platform adapter per the baseline (A.2): depends on
//! `ruviform-ui` (the port-defining crate) plus wry/tao/http/dpi/
//! rust-embed internally — none of which escape into the public API.
//!
//! ## RT constraints
//!
//! **Non-RT only** — UI-thread code throughout.

#![allow(clippy::module_name_repetitions)]

pub mod detect;

#[cfg(feature = "webview")]
mod embed;
#[cfg(feature = "webview")]
mod host;
#[cfg(feature = "webview")]
mod scheme;
#[cfg(feature = "webview")]
mod session;

#[cfg(feature = "webview")]
pub use embed::EmbedAssets;
#[cfg(feature = "webview")]
pub use session::{SessionConfig, WebViewWindowConfig, run_multi};

pub use detect::{backend_name, detect_backend, webview_available};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
