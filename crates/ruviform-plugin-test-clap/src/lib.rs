//! `ruviform-plugin-test-clap` — the Ruviform **test-only** CLAP test
//! plugin suite (Issue #630: `test_gain` + `test_fault` +
//! `test_stateful` + `test_instrument` + `test_editor`).
//!
//! ## Test-only declaration
//!
//! This crate builds a real CLAP shared object (`.clap` cdylib) that
//! exists **only as a test fixture** for verifying the Ruviform
//! production plugin host — it is never a product distribution
//! artifact, no production crate depends on this crate, and nothing
//! here ships to consumers. The binary is a deterministic,
//! license-free stand-in for commercial plugins so host discovery /
//! instantiation / processing ABI paths can be exercised without
//! third-party plugins.
//!
//! ## Independence boundary
//!
//! The plugin-side implementation (DSP law, lifecycle mapping, port
//! and parameter contracts) is written **directly** on the `clack`
//! binding crates plus the neutral `ruviform-plugin-core` /
//! `ruviform-audio-core` contracts — it deliberately does not reuse
//! `ruviform-plugin-clap` internals (which are `pub(crate)` anyway),
//! so host↔plugin round trips through this module are not a
//! self-verification loop of a single adapter implementation. The
//! only shared layer is the `clack-*` `=0.2.0` binding crates — the
//! SDK-equivalent binding layer recorded in
//! `docs/designs/issue-630-test-plugin-suite.md`.
//!
//! ## Module contents
//!
//! - `test_gain` — one stereo main audio port pair, one automatable
//!   gain parameter; `out[c][n] = in[c][n] * gain(n)` with
//!   sample-accurate parameter application at event frame offsets.
//! - `test_fault` — the same port shape with a stepped `Fault`
//!   parameter; unarmed (`none`) it is pure pass-through, and an
//!   in-band `ParamValue` event arms `abort`/`exit`/`hang`/`error`
//!   dispatch inside the arming block's `process()` — the L1
//!   out-of-process crash/hang/timeout/isolation oracle.
//! - `test_stateful` — the gain port/parameter/DSP shape plus the
//!   `clap.ext.state` extension: a real 16-byte plugin-defined state
//!   stream (`["RFST"][ver=1][f64 plain gain]`) whose `save`/`load`
//!   verbs the production host bridge's `save_state`/`restore_state`
//!   drive — activation preserves the persisted gain (unlike
//!   `test_gain`'s per-activation default reset).
//! - `test_instrument` — a generative polyphonic instrument (8
//!   triangle voices, velocity/CC7 volume/CC1 tremolo, gain
//!   parameter) plus a transport-clock click oracle: note on/off,
//!   MIDI CC and `Process::transport` snapshots drive deterministic
//!   PCM — the real-ABI counterpart of the VST3 `test_instrument`
//!   fixture.
//! - `test_editor` — a parameterless pass-through plus the plugin
//!   side of `clap.ext.gui`: on Windows `set_parent` creates a real
//!   `WS_CHILD` HWND under the host-supplied parent and
//!   `show`/`hide`/`destroy` manage it; opaque parent payloads are
//!   rejected by an `IsWindow` gate. The CLAP counterpart of the
//!   VST3 `test_editor` fixture.
//!
//! All `clack`/`clap-sys` binding types stay private — the crate
//! exports no public Rust API; hosts reach the plugins purely through
//! the ABI entry point below.
//!
//! ## Entry point
//!
//! | Symbol | Status |
//! | --- | --- |
//! | `clap_entry` | required by the CLAP module spec — a custom
//!   `Entry` + `PluginFactoryImpl` exposing `test_gain`,
//!   `test_fault`, `test_stateful`, `test_instrument` and
//!   `test_editor`/`test_spawn_fail` through one factory (`plugin_count() == 6`) |
//!
//! ## Process path (RT)
//!
//! `process` validates the `clap_audio_buffer` descriptors at the
//! boundary (frame bounds, channel counts, pointer null/alignment,
//! per-channel range-overlap alias detection) and drives each
//! processor through zero-copy
//! `ruviform_audio_core::PlanarBufferView` views over the host's
//! channel buffers — `ChannelPair::InPlace` blocks copy the shared
//! region in place with no aliased `&`/`&mut` pair ever constructed.
//! Channel-pointer, event and sort scratch is per-instance state
//! allocated at `activate`; the `none`-armed process path performs
//! only bounded work and never allocates, blocks or does I/O —
//! verified by the `rt_` `assert_no_alloc` tests. The armed fault
//! modes intentionally violate RT rules inside the out-of-process
//! child only.

mod block;
mod editor;
mod fault;
mod gain;
mod instrument;
mod instrument_core;
mod plugin;
mod preset;
mod stateful;

use std::ffi::CStr;

use clack_common::plugin::features;
use clack_extensions::preset_discovery::prelude::PresetDiscoveryFactoryWrapper;
use clack_plugin::entry::DefaultPluginFactory;
use clack_plugin::entry::prelude::{
    Entry, EntryFactories, EntryLoadError, HostInfo, PluginDescriptor, PluginFactoryImpl,
    PluginFactoryWrapper, PluginInstance,
};
use clack_plugin::prelude::clack_export_entry;

use editor::TestEditorClapPlugin;
use fault::TestFaultClapPlugin;
use instrument::TestInstrumentClapPlugin;
use plugin::TestGainClapPlugin;
use preset::TestPresetDiscoveryFactory;
use stateful::TestStatefulClapPlugin;

/// The suite's CLAP entry — one factory exposing all test plugins.
///
/// `SinglePluginEntry` can only expose one plugin, so the suite uses
/// the custom-entry form the `clack` docs prescribe: an [`Entry`]
/// holding a [`PluginFactoryWrapper`] around a [`PluginFactoryImpl`]
/// that dispatches `create_plugin` on the requested plugin id.
struct TestSuiteEntry {
    plugin_factory: PluginFactoryWrapper<TestSuiteFactory>,
    preset_factory: PresetDiscoveryFactoryWrapper<TestPresetDiscoveryFactory>,
}

impl Entry for TestSuiteEntry {
    fn new(_bundle_path: Option<&CStr>) -> Result<Self, EntryLoadError> {
        Ok(Self {
            plugin_factory: PluginFactoryWrapper::new(TestSuiteFactory::new()),
            preset_factory: PresetDiscoveryFactoryWrapper::new(TestPresetDiscoveryFactory::new()),
        })
    }

    fn declare_factories<'a>(&'a self, builder: &mut EntryFactories<'a>) {
        builder.register_factory(&self.plugin_factory);
        builder.register_factory(&self.preset_factory);
    }
}

/// The suite's plugin factory — holds all descriptors and dispatches
/// instantiation by plugin id.
struct TestSuiteFactory {
    gain: PluginDescriptor,
    fault: PluginDescriptor,
    stateful: PluginDescriptor,
    instrument: PluginDescriptor,
    editor: PluginDescriptor,
    /// `test_spawn_fail` — listed in `plugin_descriptor` but
    /// deliberately unmatched in `create_plugin`, so instantiation
    /// returns `None` (the spawn-phase `CreateInstance`-rejection
    /// oracle).
    spawn_fail: PluginDescriptor,
}

impl TestSuiteFactory {
    fn new() -> Self {
        Self {
            gain: TestGainClapPlugin::get_descriptor(),
            fault: TestFaultClapPlugin::get_descriptor(),
            stateful: TestStatefulClapPlugin::get_descriptor(),
            instrument: TestInstrumentClapPlugin::get_descriptor(),
            editor: TestEditorClapPlugin::get_descriptor(),
            spawn_fail: PluginDescriptor::new(
                "org.ruviform-test.clap-test-spawn-fail",
                "test_spawn_fail",
            )
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — listed but never creatable (spawn-fault oracle)",
            )
            .with_features([features::AUDIO_EFFECT, features::STEREO]),
        }
    }
}

impl PluginFactoryImpl for TestSuiteFactory {
    fn plugin_count(&self) -> u32 {
        6
    }

    fn plugin_descriptor(&self, index: u32) -> Option<&PluginDescriptor> {
        match index {
            0 => Some(&self.gain),
            1 => Some(&self.fault),
            2 => Some(&self.stateful),
            3 => Some(&self.instrument),
            4 => Some(&self.editor),
            5 => Some(&self.spawn_fail),
            _ => None,
        }
    }

    fn create_plugin<'a>(
        &'a self,
        host_info: HostInfo<'a>,
        plugin_id: &CStr,
    ) -> Option<PluginInstance<'a>> {
        if plugin_id == self.gain.id().unwrap_or_default() {
            Some(PluginInstance::new::<TestGainClapPlugin>(
                host_info,
                &self.gain,
                TestGainClapPlugin::new_shared,
                TestGainClapPlugin::new_main_thread,
            ))
        } else if plugin_id == self.fault.id().unwrap_or_default() {
            Some(PluginInstance::new::<TestFaultClapPlugin>(
                host_info,
                &self.fault,
                TestFaultClapPlugin::new_shared,
                TestFaultClapPlugin::new_main_thread,
            ))
        } else if plugin_id == self.stateful.id().unwrap_or_default() {
            Some(PluginInstance::new::<TestStatefulClapPlugin>(
                host_info,
                &self.stateful,
                TestStatefulClapPlugin::new_shared,
                TestStatefulClapPlugin::new_main_thread,
            ))
        } else if plugin_id == self.instrument.id().unwrap_or_default() {
            Some(PluginInstance::new::<TestInstrumentClapPlugin>(
                host_info,
                &self.instrument,
                TestInstrumentClapPlugin::new_shared,
                TestInstrumentClapPlugin::new_main_thread,
            ))
        } else if plugin_id == self.editor.id().unwrap_or_default() {
            Some(PluginInstance::new::<TestEditorClapPlugin>(
                host_info,
                &self.editor,
                TestEditorClapPlugin::new_shared,
                TestEditorClapPlugin::new_main_thread,
            ))
        } else {
            None
        }
    }
}

// The CLAP standard entry point: `#[no_mangle] pub static clap_entry`.
// The macro emits its own `#[allow(unsafe_code)]` for the generated
// static. The only `unsafe` this crate writes is confined to
// `editor.rs`'s `#[cfg(target_os = "windows")]` `win32` module —
// `#[expect(unsafe_code)]` + SAFETY comments per call site (the
// VST3 `test_editor` precedent).
clack_export_entry!(TestSuiteEntry);
