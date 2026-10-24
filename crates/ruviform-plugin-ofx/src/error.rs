//! Adapter error type for the OFX host.

use ruviform_plugin_core::LifecycleState;

/// Errors returned by the OFX host adapter.
///
/// Opaque adapter-level representation — no `libloading`, SDK, or OS error
/// type leaks across the public boundary.
#[non_exhaustive]
#[derive(Debug)]
pub enum OfxError {
    /// The operation is not supported by this adapter (yet or per contract).
    Unsupported(&'static str),
    /// A lifecycle/state precondition was violated before any foreign call.
    InvalidState {
        /// Operation that was attempted.
        operation: &'static str,
        /// State the instance was in.
        state: LifecycleState,
    },
    /// A plain value or argument failed contract validation.
    InvalidValue(&'static str),
    /// A native handle did not name a live, correctly-typed object.
    BadHandle,
    /// A memory-suite allocation failed.
    Memory,
    /// A dynamic library could not be loaded or is malformed.
    Library(String),
    /// A native action returned a non-OK status.
    Native {
        /// Action name.
        action: &'static str,
        /// Raw status code.
        status: i32,
    },
}

impl std::fmt::Display for OfxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
            Self::InvalidState { operation, state } => {
                write!(f, "invalid state {state:?} for {operation}")
            }
            Self::InvalidValue(what) => write!(f, "invalid value: {what}"),
            Self::BadHandle => f.write_str("bad native handle"),
            Self::Memory => f.write_str("native memory allocation failed"),
            Self::Library(msg) => write!(f, "library error: {msg}"),
            Self::Native { action, status } => {
                write!(f, "native action {action} failed with status {status}")
            }
        }
    }
}

impl std::error::Error for OfxError {}
