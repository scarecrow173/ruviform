//! `ruviform-plugin-ofx` — `OpenFX` Image Effect host adapter (Issue #98).
//!
//! L0 same-process synchronous hosting of OFX image-effect plugins behind
//! the neutral [`ruviform_plugin_core::VideoPluginInstance`] contract.
//!
//! Rust owns the host/session/instance lifecycle; a private C bridge owns
//! SDK layouts, suite tables, variadic entry points, `OfxHost`, and typed
//! native-action wrappers. No OFX SDK type or foreign-ABI error appears in
//! the public API.
//!
//! This is a NONRT port — every method, including `render`, is a
//! control-plane synchronous call.
//!
//! Native OFX libraries are admitted through [`OfxHost::load_library`],
//! which runs the real `Load`/`Describe`/`DescribeInContext` sequence over a
//! private C bridge and records per-entry [`PluginRejection`]s. Instances
//! follow the neutral [`LifecycleState`] contract; `render` stays
//! [`OfxError::Unsupported`] until the video task lands.

mod error;
mod ffi;
mod host;
mod instance;

pub use error::OfxError;
pub use host::{OfxHost, OfxLibrary, PluginRejection};
pub use instance::OfxInstance;
