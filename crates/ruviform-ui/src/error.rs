//! Host-level error type for the UI foundation.
//!
//! `UiError` covers adapter/host operations (window creation, resize,
//! focus, eval). IPC-level failures — malformed intents, unknown ops,
//! job failures — are reported in-band as [`crate::ipc::ErrorBody`]
//! events, not through `UiError`.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors returned by host/adapter operations.
///
/// **RT contract:** non-RT only — surfaced on the UI thread.
#[non_exhaustive]
#[derive(Debug)]
pub enum UiError {
    /// The `WebView` backend is missing or failed to initialize (for
    /// example `WebView2` Evergreen not installed on Windows).
    BackendUnavailable {
        /// Human-readable reason.
        message: String,
    },
    /// A host operation (eval, resize, focus, close request) failed.
    Host {
        /// Human-readable reason.
        message: String,
    },
    /// An internal invariant was violated.
    Internal {
        /// Human-readable reason.
        message: String,
    },
    /// The driver does not implement the requested capability
    /// (Issue #386 — declared via
    /// [`crate::driver::WindowCapabilities`]). Drivers must surface
    /// this instead of silently degrading or faking the result.
    Unsupported {
        /// The capability that was requested (for example
        /// `"native_surface_slot"`).
        capability: String,
        /// Human-readable reason.
        message: String,
    },
}

impl UiError {
    /// Convenience constructor for [`UiError::BackendUnavailable`].
    #[must_use]
    pub fn backend_unavailable(message: impl Into<String>) -> Self {
        Self::BackendUnavailable {
            message: message.into(),
        }
    }

    /// Convenience constructor for [`UiError::Host`].
    #[must_use]
    pub fn host(message: impl Into<String>) -> Self {
        Self::Host {
            message: message.into(),
        }
    }

    /// Convenience constructor for [`UiError::Internal`].
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Convenience constructor for [`UiError::Unsupported`].
    #[must_use]
    pub fn unsupported(capability: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Unsupported {
            capability: capability.into(),
            message: message.into(),
        }
    }
}

impl Display for UiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackendUnavailable { message } => {
                write!(f, "webview backend unavailable: {message}")
            }
            Self::Host { message } => write!(f, "host operation failed: {message}"),
            Self::Internal { message } => write!(f, "internal ui error: {message}"),
            Self::Unsupported {
                capability,
                message,
            } => write!(f, "unsupported capability {capability}: {message}"),
        }
    }
}

impl Error for UiError {}
