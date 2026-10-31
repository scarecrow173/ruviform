//! Backend detection report types.
//!
//! Detection itself lives in the adapter (`ruviform-ui-webview` calls
//! `wry::webview_version()` without creating a window — the `poc/ui`
//! `check` pattern). These types carry the result so applications can
//! gate their GUI path and surface a useful error instead of a late
//! crash (for example `WebView2` Evergreen missing).

/// Whether a backend is usable.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    /// Backend runtime detected; GUI path can start.
    Available,
    /// Backend runtime missing — the app should fall back to CLI or
    /// surface install guidance.
    Unavailable,
    /// Detection could not run on this platform/build.
    Unknown,
}

/// Detection result for one UI backend.
#[derive(Debug, Clone)]
pub struct BackendReport {
    /// Stable backend name (`"webview2"`, `"wkwebview"`,
    /// `"webkit2gtk"`).
    pub name: &'static str,
    /// Runtime version string when detected.
    pub version: Option<String>,
    /// Availability verdict.
    pub state: BackendState,
    /// Human-readable notes (install hints, platform caveats).
    pub notes: Vec<String>,
}

impl BackendReport {
    /// Convenience constructor for an available backend.
    #[must_use]
    pub fn available(name: &'static str, version: String) -> Self {
        Self {
            name,
            version: Some(version),
            state: BackendState::Available,
            notes: Vec::new(),
        }
    }

    /// Convenience constructor for a missing backend.
    #[must_use]
    pub fn unavailable(name: &'static str, note: impl Into<String>) -> Self {
        Self {
            name,
            version: None,
            state: BackendState::Unavailable,
            notes: vec![note.into()],
        }
    }
}
