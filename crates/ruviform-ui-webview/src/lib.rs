//! `ruviform-ui-webview` — `wry` + `tao` adapter implementing the
//! `ruviform-ui` host boundary (Issue #156).
//!
//! The adapter owns everything backend-shaped so `ruviform-ui` stays
//! dependency-free:
//!
//! - [`host`]: windowed [`run`](host::run) — tao event loop, wry
//!   webview, `EventLoopProxy` wake-up, `set_bounds` resize follow,
//!   tick-driven job drain. Emits normalized [`HostEvent`]s, accepts a
//!   [`HostHandle`] for driving back.
//! - `scheme`: `rvf://app` + `rvf://data` custom-protocol handler —
//!   asset serving, bounded pulls, traversal rejection, CORS. Pure;
//!   headless-tested.
//! - [`embed`]: `rust-embed`-backed [`ruviform_ui::AssetSource`] for
//!   the production single-binary layout.
//! - [`detect`]: `wry::webview_version()` backend probe (no window).
//!
//! Rust→frontend pushes serialize [`ruviform_ui::Event`] via
//! [`ruviform_ui::dispatch_script`] + `evaluate_script`; the
//! `window.__rvf` bridge is injected by the host.
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
pub use embed::EmbedAssets;
#[cfg(feature = "webview")]
pub use host::{HostConfig, run};

pub use detect::{backend_name, detect_backend, webview_available};

// Re-exported for adapter users wiring an event push loop:
// `host.eval(&dispatch_script(&event))`.
pub use ruviform_ui::dispatch_script;

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
