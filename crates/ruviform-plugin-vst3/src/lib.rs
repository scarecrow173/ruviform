//! `ruviform-plugin-vst3` — the VST3 standard adapter for the
//! ruviform-plugin foundation (Issue #29, T4 binding `PoC`).
//!
//! The crate builds a VST3 module (`.vst3` cdylib) around the neutral
//! `ruviform_plugin_core` contracts: a `PoC` gain plugin with one stereo
//! input bus, one stereo output bus and one normalized gain parameter.
//! All `vst3`-crate binding types stay private — the crate exports no
//! public Rust API; hosts reach the plugin purely through the ABI
//! entry points below.
//!
//! ## Entry points
//!
//! | Symbol | Platform | Status |
//! | --- | --- | --- |
//! | `GetPluginFactory` | all | required by the VST3 module spec |
//! | `InitDll` / `ExitDll` | Windows | optional in the spec — implemented as trivially-true hooks so the loader path is uniform |
//! | `bundleEntry` / `bundleExit` | macOS | required structurally; real bundle wiring is a packaging concern out of `PoC` scope |
//! | `ModuleEntry` / `ModuleExit` | Linux | required; trivially-true hooks (no module state) |
//!
//! Note: the upstream `vst3` crate's example names the macOS hooks
//! `BundleEntry`/`BundleExit`; the VST3 spec (and this task) uses the
//! lowercase `bundleEntry`/`bundleExit` — recorded in the `PoC`
//! findings.
//!
//! ## Process path (RT)
//!
//! `process` validates `ProcessData` at the boundary (`numSamples`,
//! `kSample32` only — `kSample64` is rejected, never reinterpreted,
//! channel counts, pointer null/alignment, per-channel-pair alias
//! detection) and drives `GainProcessor` through zero-copy
//! `ruviform_audio_core::PlanarBufferView` views over the host's
//! `channelBuffers32` — per the task's zero-copy condition. In-place
//! aliasing is handled by the T2-approved bounce-scratch strategy
//! (per-channel bounce + `finish_block` writeback). Channel-list and
//! event scratch is per-instance state allocated at
//! `setupProcessing`; the process path performs only bounded work and
//! never allocates, blocks or does I/O.
//!
//! ## Host bridge (`feature = "host"`)
//!
//! The optional `host` feature compiles `host`, the mirror image of
//! this adapter: a host-side bridge that dlopens a real VST3 module
//! in-process via `libloading`, scans its class table through the raw
//! `vst3`-crate COM bindings and maps descriptors, parameters and the
//! component/controller lifecycle onto the neutral
//! [`PluginFactory`][ruviform_plugin_core::PluginFactory] /
//! [`Plugin`][ruviform_plugin_core::Plugin] contracts. It exists for
//! the L1 out-of-process runner (Issue #114) and stays out of the
//! default dependency tree — the `libloading`/COM boundary is scoped
//! inside `host` with `#[expect(unsafe_code)]` + SAFETY
//! justifications.

mod block;
mod component;
mod controller;
mod factory;
mod gain;
#[cfg(feature = "host")]
pub mod host;
mod strings;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::ffi::c_void;

use crate::factory::GainFactory;
use vst3::ComWrapper;
use vst3::Steinberg::IPluginFactory;

/// `GetPluginFactory` — the universal VST3 module entry point.
/// Returns a new `IPluginFactory` COM pointer whose ownership is
/// transferred to the caller per the VST3 contract.
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    let wrapper = ComWrapper::new(GainFactory::new());
    match wrapper.to_com_ptr::<IPluginFactory>() {
        Some(ptr) => ptr.into_raw(),
        None => std::ptr::null_mut(),
    }
}

/// `InitDll` — Windows module hook (optional in the spec; implemented
/// as a trivially-true hook so the module shape is uniform across
/// platforms — decision recorded in the `PoC` findings).
#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn InitDll() -> bool {
    true
}

/// `ExitDll` — see [`InitDll`].
#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn ExitDll() -> bool {
    true
}

/// `bundleEntry` — macOS bundle hook, required structurally by the
/// spec (the upstream example calls it `BundleEntry` — the spec name
/// is `bundleEntry`). The `PoC` ships no bundle resources, so this is a
/// trivially-true hook.
#[cfg(target_os = "macos")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn bundleEntry(_bundle: *mut c_void) -> bool {
    true
}

/// `bundleExit` — see [`bundleEntry`].
#[cfg(target_os = "macos")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn bundleExit() -> bool {
    true
}

/// `ModuleEntry` — Linux module hook required by the spec;
/// trivially-true (no module-global state exists).
#[cfg(target_os = "linux")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn ModuleEntry(_module: *mut c_void) -> bool {
    true
}

/// `ModuleExit` — see [`ModuleEntry`].
#[cfg(target_os = "linux")]
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn ModuleExit() -> bool {
    true
}
