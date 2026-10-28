//! `ruviform-plugin-clap` — the thin CLAP standard adapter for the
//! Ruviform plugin foundation, built on the `clack` binding crates
//! (`clack-plugin` / `clack-common` / `clack-extensions` 0.2.0).
//!
//! This crate is also the CLAP binding-method `PoC` (task
//! `TASK-034JgsGOtyuy2MkLRLZQI5`): a minimal gain plugin exported as a
//! real `clap_entry` cdylib entry point, whose `process` path feeds
//! host-supplied per-channel buffers into the neutral
//! [`PluginProcessor`][ruviform_plugin_core::PluginProcessor] contract
//! with **zero copies** by wrapping them in
//! `ruviform_audio_core::PlanarBufferView` / `PlanarBufferViewMut`.
//!
//! ## Responsibilities (thin adapter)
//!
//! - Export the standard entry point (`clack_export_entry!` →
//!   `clap_entry`), a plugin descriptor and the plugin factory.
//! - Translate CLAP ABI data into the neutral foundation types —
//!   audio buffers → planar views, `ParamValueEvent` →
//!   [`EventList`][ruviform_plugin_core::EventList] /
//!   [`ParamEvent`][ruviform_plugin_core::ParamEvent], `TransportEvent`
//!   → [`TransportInfo`][ruviform_plugin_core::TransportInfo],
//!   `ProcessResult` → CLAP process status.
//! - Drive the common [`Lifecycle`][ruviform_plugin_core::Lifecycle]
//!   machine from the CLAP call sequence and reject host protocol
//!   violations through the shared transition table instead of
//!   silently accepting them.
//! - Allocate all per-block scratch (channel-pointer bookkeeping,
//!   event scratch) at activation time so the RT path never
//!   allocates.
//!
//! The `PluginProcessor` contract is owned by `ruviform-plugin-core`;
//! this crate only implements and drives it. Binding-crate types are
//! an implementation detail: no `clack` / `clap-sys` type appears in
//! the public Rust API of this crate.
//!
//! ## Allowed dependency layers
//!
//! Extension layer — `ruviform-plugin-core` + Domain foundations
//! (`ruviform-audio-core`) + Primitives (`ruviform-media-core`), plus
//! the `clack` binding crates as adapter-internal implementation
//! dependencies.
//!
//! ## RT contract
//!
//! `process` is the RT callback — no alloc/dealloc, blocking, I/O,
//! unbounded waits/retries or unbounded work. All per-instance scratch
//! is allocated in `activate`; the per-block path performs bounded
//! scans only. `rt_`-prefixed tests select the RT contract lane
//! (`mise run test-rt`); the Issue #22 RT-verification lane arms
//! `assert_no_alloc` around the per-block path so any (de)allocation
//! aborts the test binary — machine-detected, not just a documented
//! contract. Passing CI is still not *proof* of RT safety: static
//! constraints (lock duration, priority, execution-time bounds) remain
//! documentation + review responsibilities per
//! `docs/adr/realtime-contract-and-verification-policy.md`. `PoC`
//! findings are recorded in
//! `docs/impl/ir/poc-clap-binding-adapter.md`.
//!
//! ## Host bridge (`feature = "host"`)
//!
//! The optional `host` feature compiles `host`, the mirror image of
//! this adapter: a host-side bridge that loads a real `.clap` shared
//! object in-process with `clack-host` and maps its descriptor,
//! parameters and lifecycle onto the neutral
//! [`PluginFactory`][ruviform_plugin_core::PluginFactory] /
//! [`Plugin`][ruviform_plugin_core::Plugin] contracts. It exists for
//! the L1 out-of-process runner (Issue #114) and stays out of the
//! default dependency tree — `PluginEntry::load` is the crate's one
//! `unsafe` boundary, scoped inside `host` with `#[expect(unsafe_code)]`
//! + SAFETY justifications.
//!
//! Contract source: `docs/specs/implement-plugin-foundation.md`,
//! `docs/plans/implement-plugin-foundation.md` (task T3).

mod adapt;
mod gain;
#[cfg(feature = "host")]
pub mod host;
mod plugin;

use clack_plugin::entry::SinglePluginEntry;
use clack_plugin::prelude::clack_export_entry;

pub use gain::{GAIN_PARAM, GainProcessor};

pub(crate) use plugin::GainClapPlugin;

// The CLAP standard entry point: `#[no_mangle] pub static clap_entry`.
// The macro emits its own `#[allow(unsafe_code)]` for the generated
// static; no unsafe code is written in this crate.
clack_export_entry!(SinglePluginEntry::<GainClapPlugin>);

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_plugin_clap::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// The adapter's public API must not expose binding-crate types:
    /// the only re-exports are the neutral `PluginProcessor` impl and
    /// its `ParamInfo`.
    #[test]
    fn public_surface_is_binding_neutral() {
        let _ = GAIN_PARAM;
        let _core_version = ruviform_plugin_core::version();
        // `host::ClapHostError` carries erased `Box<dyn Error>`
        // payloads — no named clack type in the public signature.
        #[cfg(feature = "host")]
        {
            fn assert_error_bounds<T: std::error::Error + Send + Sync>() {}
            assert_error_bounds::<host::ClapHostError>();
        }
    }
}
