//! Backend detection — `wry::webview_version` probe without creating
//! a window (the `poc/ui` `check` pattern, minus the winreg dependency:
//! wry already reads the `WebView2` `pv` registry key on Windows).

use ruviform_ui::{BackendReport, BackendState};

/// Detected backend name for this platform.
#[must_use]
pub const fn backend_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "webview2"
    }
    #[cfg(target_os = "macos")]
    {
        "wkwebview"
    }
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "ios"))
    ))]
    {
        "webkit2gtk"
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        all(unix, not(any(target_os = "android", target_os = "ios")))
    )))]
    {
        "unknown"
    }
}

/// Install hint shown when the backend probe fails.
#[cfg(feature = "webview")]
fn install_hint() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "WebView2 Evergreen Runtime required (https://go.microsoft.com/fwlink/p/?LinkId=2124703 or `winget install Microsoft.EdgeWebView2Runtime`)"
    }
    #[cfg(target_os = "macos")]
    {
        "WKWebView ships with macOS — a missing report indicates a deeper platform issue"
    }
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "ios"))
    ))]
    {
        "libwebkit2gtk-4.1 required (e.g. `apt install libwebkit2gtk-4.1-0`); WebKitGTK >= 2.40 for request bodies"
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        all(unix, not(any(target_os = "android", target_os = "ios")))
    )))]
    {
        "no supported webview backend on this platform"
    }
}

/// Probes the platform `WebView` backend without creating a window.
#[cfg(feature = "webview")]
#[must_use]
pub fn detect_backend() -> BackendReport {
    match wry::webview_version() {
        Ok(version) => BackendReport::available(backend_name(), version),
        Err(e) => {
            let mut report = BackendReport::unavailable(backend_name(), install_hint());
            report.notes.push(format!("probe error: {e}"));
            report
        }
    }
}

/// Non-`webview`-feature stub: detection always reports `Unknown`
/// (no backend linked into this build).
#[cfg(not(feature = "webview"))]
#[must_use]
pub fn detect_backend() -> BackendReport {
    BackendReport {
        name: backend_name(),
        version: None,
        state: BackendState::Unknown,
        notes: vec!["built without the `webview` feature".to_string()],
    }
}

/// Whether a usable `WebView` backend is present (cheap form of
/// [`detect_backend`]).
#[must_use]
pub fn webview_available() -> bool {
    detect_backend().state == BackendState::Available
}
