//! `ruviform-plugin-test-vst3` — the Ruviform **test-only** VST3 test
//! plugin suite (Issue #630: `test_gain`, `test_fault`, `test_editor`,
//! `test_stateful`, `test_instrument`).
//!
//! ## Test-only declaration
//!
//! This crate builds real VST3 module binaries (`.vst3`/`.dll`
//! cdylibs) that exist **only as test fixtures** for verifying the
//! Ruviform production plugin host — they are never product
//! distribution artifacts, no production crate depends on this
//! crate, and nothing here ships to consumers. The binaries are
//! deterministic, license-free stand-ins for commercial plugins so
//! host discovery / instantiation / processing / state / editor ABI
//! paths can be exercised without third-party plugins.
//!
//! ## Independence boundary
//!
//! The plugin-side implementation (DSP law, lifecycle mapping, bus
//! and parameter contracts) is written **directly** on the `vst3`
//! binding crate plus the neutral `ruviform-plugin-core` /
//! `ruviform-audio-core` contracts — it deliberately does not reuse
//! `ruviform-plugin-vst3` internals (which are `pub(crate)` anyway),
//! so host↔plugin round trips through this module are not a
//! self-verification loop of a single adapter implementation. The
//! only shared layer is the `vst3` crate itself — the SDK-equivalent
//! binding layer recorded in
//! `docs/designs/issue-630-test-plugin-suite.md`.
//!
//! ## Module contents
//!
//! - `test_gain` — one stereo main bus pair, one automatable gain
//!   parameter; `out[c][n] = in[c][n] * gain(n)` with sample-accurate
//!   parameter application at event frame offsets.
//! - `test_fault` — the same bus shape with a stepped `Fault`
//!   parameter; unarmed (`none`) it is pure pass-through, and an
//!   in-band `ParamValue` event arms `abort`/`exit`/`hang`/`error`
//!   dispatch inside the arming block's `process()` — the L1
//!   out-of-process crash/hang/timeout/isolation oracle.
//! - `test_editor` — a parameterless pass-through processor whose
//!   controller answers `createView("editor")` with a real
//!   `IPlugView`: on Windows `attached` creates a `WS_CHILD` HWND
//!   under the host-provided parent (the AC-4 plugin-fixture half);
//!   every other platform/parent combination is an honest rejection.
//! - `test_stateful` — the `test_gain` law with a real serialized
//!   state surface: the gain lives in a shared atomic cell that
//!   `IComponent::setState`/`getState` and the controller's
//!   `setComponentState`/`setState`/`getState` round-trip through a
//!   deterministic 16-byte blob — the L1 save → mutate → restore →
//!   save oracle.
//! - `test_instrument` — a generative deterministic instrument: up to
//!   8 triangle-wave voices driven by `IEventList` note/CC events at
//!   frame offsets, plus a transport-clock click oracle (single-frame
//!   impulses at `ppqn` boundaries while playing with musical time)
//!   fed by `ProcessContext`. The suite's only `MidiTransport` input
//!   surface.
//!
//! All `vst3`-crate binding types stay private — the crate exports
//! no public Rust API; hosts reach the plugin purely through the ABI
//! entry points below.
//!
//! ## Entry points
//!
//! | Symbol | Platform | Status |
//! | --- | --- | --- |
//! | `GetPluginFactory` | all | required by the VST3 module spec |
//! | `InitDll` / `ExitDll` | Windows | optional in the spec — implemented as trivially-true hooks so the loader path is uniform |
//! | `bundleEntry` / `bundleExit` | macOS | required structurally; trivially-true hooks (no bundle resources ship) |
//! | `ModuleEntry` / `ModuleExit` | Linux | required; trivially-true hooks (no module state) |
//!
//! ## Process path (RT)
//!
//! `process` validates `ProcessData` at the boundary (sample size,
//! frame bounds, channel counts, pointer null/alignment, per-channel
//! alias detection with bounce writeback) and drives the gain
//! processor through zero-copy
//! `ruviform_audio_core::PlanarBufferView` views over the host's
//! `channelBuffers32`. Channel-list and event scratch is
//! per-instance state allocated at `setupProcessing`; the process
//! path performs only bounded work and never allocates, blocks or
//! does I/O — verified by the `rt_` `assert_no_alloc` tests.

mod block;
mod component;
mod controller;
mod editor;
mod editor_view;
mod events;
mod factory;
mod fault;
mod gain;
mod instrument;
mod stateful;
mod strings;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::ffi::c_void;

use crate::factory::TestGainFactory;
use vst3::ComWrapper;
use vst3::Steinberg::IPluginFactory;

/// Converts a VST3 SDK enum constant to the `int32` domain the
/// generated bindings use for struct fields and trait parameters.
/// The bindings declare enum constants as `DefaultEnumType` — `c_int`
/// on Windows, `c_uint` elsewhere — so routing through `i64` keeps the
/// range check meaningful on every platform and no `as` cast is
/// needed. Every constant the SDK ships fits `i32`, so `try_from`
/// cannot fail in practice; the `unwrap_or` arm exists only to keep
/// the path defined for a hypothetical out-of-range future constant.
fn sdk_enum_i32(value: impl Into<i64>) -> i32 {
    i32::try_from(value.into()).unwrap_or(i32::MAX)
}

/// `GetPluginFactory` — the universal VST3 module entry point.
/// Returns a new `IPluginFactory` COM pointer whose ownership is
/// transferred to the caller per the VST3 contract.
#[allow(non_snake_case)]
// Symbol export is the crate's only ABI surface — the names are the
// VST3 module contract; collisions are impossible inside a module.
#[expect(unsafe_code)]
#[unsafe(no_mangle)]
extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    let wrapper = ComWrapper::new(TestGainFactory::new());
    match wrapper.to_com_ptr::<IPluginFactory>() {
        Some(ptr) => ptr.into_raw(),
        None => std::ptr::null_mut(),
    }
}

/// `InitDll` — Windows module hook (optional in the spec; implemented
/// as a trivially-true hook so the module shape is uniform across
/// platforms).
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
/// spec. The suite ships no bundle resources, so this is a
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

#[cfg(test)]
mod tests {
    use super::sdk_enum_i32;

    /// `int32` values pass through unchanged; out-of-range
    /// `DefaultEnumType` values clamp to `i32::MAX` — never the
    /// sign-wrap an `as` cast would produce.
    #[test]
    fn sdk_enum_i32_boundaries() {
        assert_eq!(sdk_enum_i32(0u32), 0);
        assert_eq!(sdk_enum_i32(0x7FFF_FFFFu32), i32::MAX);
        assert_eq!(sdk_enum_i32(0x8000_0000u32), i32::MAX);
        assert_eq!(sdk_enum_i32(u32::MAX), i32::MAX);
        assert_eq!(sdk_enum_i32(-1i32), -1);
        assert_eq!(sdk_enum_i32(i32::MIN), i32::MIN);
    }
}
