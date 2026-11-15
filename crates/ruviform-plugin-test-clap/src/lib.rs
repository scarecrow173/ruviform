//! `ruviform-plugin-test-clap` — the Ruviform **test-only** CLAP test
//! plugin suite (Issue #630: `test_gain` + `test_fault`).
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
//!   `Entry` + `PluginFactoryImpl` exposing both `test_gain` and
//!   `test_fault` through one factory (`plugin_count() == 2`) |
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
mod fault;
mod gain;
mod plugin;

use std::ffi::CStr;

use clack_plugin::entry::DefaultPluginFactory;
use clack_plugin::entry::prelude::{
    Entry, EntryFactories, EntryLoadError, HostInfo, PluginDescriptor, PluginFactoryImpl,
    PluginFactoryWrapper, PluginInstance,
};
use clack_plugin::prelude::clack_export_entry;

use fault::TestFaultClapPlugin;
use plugin::TestGainClapPlugin;

/// The suite's CLAP entry — one factory exposing both test plugins.
///
/// `SinglePluginEntry` can only expose one plugin, so the suite uses
/// the custom-entry form the `clack` docs prescribe: an [`Entry`]
/// holding a [`PluginFactoryWrapper`] around a [`PluginFactoryImpl`]
/// that dispatches `create_plugin` on the requested plugin id.
struct TestSuiteEntry {
    plugin_factory: PluginFactoryWrapper<TestSuiteFactory>,
}

impl Entry for TestSuiteEntry {
    fn new(_bundle_path: Option<&CStr>) -> Result<Self, EntryLoadError> {
        Ok(Self {
            plugin_factory: PluginFactoryWrapper::new(TestSuiteFactory::new()),
        })
    }

    fn declare_factories<'a>(&'a self, builder: &mut EntryFactories<'a>) {
        builder.register_factory(&self.plugin_factory);
    }
}

/// The suite's plugin factory — holds both descriptors and dispatches
/// instantiation by plugin id.
struct TestSuiteFactory {
    gain: PluginDescriptor,
    fault: PluginDescriptor,
}

impl TestSuiteFactory {
    fn new() -> Self {
        Self {
            gain: TestGainClapPlugin::get_descriptor(),
            fault: TestFaultClapPlugin::get_descriptor(),
        }
    }
}

impl PluginFactoryImpl for TestSuiteFactory {
    fn plugin_count(&self) -> u32 {
        2
    }

    fn plugin_descriptor(&self, index: u32) -> Option<&PluginDescriptor> {
        match index {
            0 => Some(&self.gain),
            1 => Some(&self.fault),
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
        } else {
            None
        }
    }
}

// The CLAP standard entry point: `#[no_mangle] pub static clap_entry`.
// The macro emits its own `#[allow(unsafe_code)]` for the generated
// static; no unsafe code is written in this crate.
clack_export_entry!(TestSuiteEntry);
