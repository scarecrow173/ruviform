//! `test_stateful`'s preset surface — the CLAP preset-discovery
//! factory/provider plus the catalog the `clap.ext.preset-load`
//! extension dispatches on (Issue #630 preset slice).
//!
//! ## Surface ownership (CLAP preset-discovery extension)
//!
//! | piece | owner | role |
//! | ----- | ----- | ---- |
//! | `clap.factory.preset-discovery` | entry factory | exposes one [`ProviderDescriptor`] |
//! | `clap_preset_discovery_provider` | plugin instance | declares locations + emits preset metadata |
//! | `clap.ext.preset-load` | plugin main thread | `load_from_location` restores a preset's state |
//!
//! The suite entry registers a [`PresetDiscoveryFactoryWrapper`]
//! beside the plugin factory; a host that asks the entry for the
//! preset-discovery factory id gets a factory reporting exactly one
//! provider backed by [`PRESETS`]. The provider declares a single
//! [`Location::Plugin`] location (factory content baked into the
//! plugin — no file types, no soundpacks) and emits one preset per
//! catalog entry, each tagged with the `test_stateful` plugin id so
//! a host preset browser can bind it.
//!
//! ## Preset law
//!
//! [`PRESETS`] mirrors the VST3 fixture's catalog exactly — the same
//! four names and plain-domain gains — so the two ABIs present an
//! identical preset pool. A preset's `load_key` is its catalog name
//! (opaque to the host, unique within `Location::Plugin`); the
//! `clap.ext.preset-load` implementation in `stateful.rs` maps the
//! key back to its gain and restores it through the same blob law
//! `clap.ext.state` `load` applies, including the `VALUES` params
//! rescan a parameter-changing restore owes the host.
//!
//! `state-context` stays disabled: preset loads carry no extra
//! context channel — the restored value is just the state blob's
//! gain, applied through the existing `clap.ext.state` decode law.

use std::ffi::CStr;

use clack_extensions::preset_discovery::prelude::{
    Flags, IndexerInfo, Location, LocationInfo, MetadataReceiver, PresetDiscoveryFactoryImpl,
    ProviderDescriptor, ProviderImpl, ProviderInstance, UniversalPluginId,
};
use clack_plugin::prelude::PluginError;

// ---------------------------------------------------------------------------
// Preset catalog — the deterministic factory preset pool
// ---------------------------------------------------------------------------

/// One factory preset — a display `name` (which doubles as the
/// opaque `load_key`) and its plain-domain gain.
pub(crate) struct PresetDef {
    /// Program/preset display name and `load_key` — null-terminated
    /// for the CLAP metadata receiver and the preset-load dispatch.
    pub name: &'static CStr,
    /// The preset's plain gain — the `f64` field of the `RFST`
    /// state blob the preset-load path restores.
    pub gain: f64,
}

/// The built-in factory presets — the same four names and gains the
/// VST3 fixture's `"Factory"` program list carries, so both ABIs
/// present an identical preset pool.
pub(crate) static PRESETS: &[PresetDef] = &[
    PresetDef {
        name: c"Default",
        gain: 1.0,
    },
    PresetDef {
        name: c"Half",
        gain: 0.5,
    },
    PresetDef {
        name: c"Boost",
        gain: 1.5,
    },
    PresetDef {
        name: c"Mute",
        gain: 0.0,
    },
];

/// The `test_stateful` plugin's CLAP id — must match the id
/// `TestStatefulClapPlugin::get_descriptor` registers; preset
/// metadata tags each preset with it so hosts can bind presets to
/// the plugin that loads them.
pub(crate) const STATEFUL_PLUGIN_ID: &CStr = c"org.ruviform-test.clap-test-stateful";

/// The preset-discovery provider's descriptor id.
const PROVIDER_ID: &str = "org.ruviform-test.clap-presets";

/// The preset-discovery provider's display name.
const PROVIDER_NAME: &str = "Ruviform Test Presets";

/// The single declared preset location's display name.
const LOCATION_NAME: &CStr = c"Built-in";

/// Looks up the catalog preset a `load_key` names — `None` for any
/// key outside [`PRESETS`] (including the `None` case, which the
/// caller handles separately).
pub(crate) fn preset_by_load_key(load_key: &CStr) -> Option<&'static PresetDef> {
    PRESETS.iter().find(|preset| preset.name == load_key)
}

// ---------------------------------------------------------------------------
// Provider — emits the catalog as preset metadata
// ---------------------------------------------------------------------------

/// The suite's single preset provider — stateless; every
/// `get_metadata(Location::Plugin)` call re-emits the same catalog.
struct TestPresetProvider;

impl ProviderImpl<'_> for TestPresetProvider {
    /// Emits one preset per [`PRESETS`] entry for
    /// [`Location::Plugin`]. File locations are never declared by the
    /// indexer, so they produce no metadata — a defined empty answer,
    /// not an error.
    ///
    /// # Errors
    ///
    /// `PluginError` — the host's metadata receiver declined a
    /// `begin_preset` (propagated via `From`).
    fn get_metadata(
        &mut self,
        location: Location,
        receiver: &mut MetadataReceiver,
    ) -> Result<(), PluginError> {
        let Location::Plugin = location else {
            return Ok(());
        };
        for preset in PRESETS {
            receiver
                .begin_preset(Some(preset.name), Some(preset.name))?
                .set_flags(Flags::IS_FACTORY_CONTENT)
                .add_plugin_id(UniversalPluginId::clap(STATEFUL_PLUGIN_ID))
                .add_creator(c"Ruviform Test");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory — the entry's `clap.factory.preset-discovery` implementation
// ---------------------------------------------------------------------------

/// The suite's preset-discovery factory — reports exactly one
/// provider ([`TestPresetProvider`]'s descriptor) and instantiates it
/// for the matching provider id.
pub(crate) struct TestPresetDiscoveryFactory {
    descriptor: ProviderDescriptor,
}

impl TestPresetDiscoveryFactory {
    /// A factory holding the provider descriptor — constructed once
    /// by the entry and shared for the entry's lifetime.
    pub(crate) fn new() -> Self {
        Self {
            descriptor: ProviderDescriptor::new(PROVIDER_ID, PROVIDER_NAME)
                .with_vendor("Ruviform Test"),
        }
    }
}

impl PresetDiscoveryFactoryImpl for TestPresetDiscoveryFactory {
    fn provider_count(&self) -> u32 {
        1
    }

    fn provider_descriptor(&self, index: u32) -> Option<&ProviderDescriptor> {
        (index == 0).then_some(&self.descriptor)
    }

    /// Creates the provider for [`PROVIDER_ID`], declaring the single
    /// built-in preset location during initialization — the indexer
    /// handle is the host's, consumed exactly once here.
    ///
    /// Returns `None` for any other provider id — the defined
    /// "unknown provider" answer.
    fn create_provider<'a>(
        &'a self,
        indexer: IndexerInfo<'a>,
        provider_id: &CStr,
    ) -> Option<ProviderInstance<'a>> {
        if self.descriptor.id() != Some(provider_id) {
            return None;
        }
        Some(ProviderInstance::new(
            indexer,
            &self.descriptor,
            |mut indexer| {
                indexer.declare_location(LocationInfo {
                    name: LOCATION_NAME,
                    flags: Flags::IS_FACTORY_CONTENT,
                    location: Location::Plugin,
                })?;
                Ok(TestPresetProvider)
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalog is the cross-ABI contract — pin the exact pool so
    /// a drift between the VST3 and CLAP fixtures is a test failure,
    /// not a silent divergence.
    #[test]
    fn catalog_is_the_shared_four_preset_pool() {
        assert_eq!(PRESETS.len(), 4);
        let names: Vec<&CStr> = PRESETS.iter().map(|preset| preset.name).collect();
        assert_eq!(
            names,
            [c"Default", c"Half", c"Boost", c"Mute"],
            "preset names must match the VST3 fixture's catalog"
        );
        let gains: Vec<f64> = PRESETS.iter().map(|preset| preset.gain).collect();
        assert_eq!(gains, [1.0, 0.5, 1.5, 0.0]);
    }

    /// `load_key` dispatch resolves every catalog preset and rejects
    /// foreign keys — the `clap.ext.preset-load` lookup law.
    #[test]
    fn load_key_dispatch_is_catalog_scoped() {
        for preset in PRESETS {
            let found = preset_by_load_key(preset.name).expect("catalog preset resolves");
            assert!(std::ptr::eq(found, preset));
        }
        assert!(preset_by_load_key(c"Nope").is_none());
        assert!(preset_by_load_key(c"").is_none());
    }

    /// The factory reports exactly one provider whose descriptor is
    /// well-formed — id and name present, index bounds honored.
    #[test]
    fn factory_reports_one_wellformed_provider() {
        let factory = TestPresetDiscoveryFactory::new();
        assert_eq!(factory.provider_count(), 1);
        assert!(factory.provider_descriptor(0).is_some());
        assert!(factory.provider_descriptor(1).is_none());
        let descriptor = factory.provider_descriptor(0).expect("provider 0");
        assert_eq!(
            descriptor.id().and_then(|id| id.to_str().ok()),
            Some(PROVIDER_ID)
        );
        assert_eq!(
            descriptor.name().and_then(|name| name.to_str().ok()),
            Some(PROVIDER_NAME)
        );
    }

    // -- In-process ABI: preset discovery + preset load ------------------
    //
    // These tests drive the real CLAP ABI surface in-process through
    // `clack-host`: the entry's `clap.factory.preset-discovery` factory,
    // the provider's `get_metadata` callback into a recording
    // `MetadataReceiverImpl`, and `test_stateful`'s
    // `clap.ext.preset-load` `from_location` into a recording
    // `HostPresetLoadImpl` — the same verbs a real host issues, minus
    // the out-of-process boundary the L1 lane covers separately.

    use std::sync::{Arc, Mutex};

    use clack_common::utils::{ClapId, Timestamp};
    use clack_extensions::params::{
        HostParams, HostParamsImplMainThread, HostParamsImplShared, ParamClearFlags,
        ParamRescanFlags,
    };
    use clack_extensions::preset_discovery::HostPresetLoadImpl;
    use clack_extensions::preset_discovery::prelude::{
        FileType, HostPresetLoad, IndexerImpl, MetadataReceiverImpl, PluginPresetLoad,
        PresetDiscoveryFactory, Provider, Soundpack,
    };
    use clack_extensions::state::PluginState;
    use clack_host::entry::PluginEntry;
    use clack_host::host::{
        HostError, HostExtensions, HostHandlers, HostInfo, MainThreadHandler, SharedHandler,
    };
    use clack_host::plugin::PluginInstance;

    use crate::stateful::encode_state_blob;

    /// A `Location` reduced to owned data for assertions — the raw
    /// type borrows host strings we don't keep alive.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum RecordedLocation {
        Plugin,
        File(String),
    }

    fn record_location(location: Location) -> RecordedLocation {
        match location {
            Location::Plugin => RecordedLocation::Plugin,
            Location::File { path } => RecordedLocation::File(path.to_string_lossy().into_owned()),
        }
    }

    /// Recording `IndexerImpl` — captures every declaration the
    /// provider makes during `init`.
    #[derive(Default)]
    struct ProbeIndexer {
        /// `(name, flags, location)` per declared location — owned
        /// copies, since `LocationInfo` borrows host strings.
        locations: Vec<(String, Flags, RecordedLocation)>,
        filetypes: u32,
        soundpacks: u32,
    }

    impl IndexerImpl for ProbeIndexer {
        fn declare_filetype(&mut self, _file_type: FileType) -> Result<(), HostError> {
            self.filetypes += 1;
            Ok(())
        }

        fn declare_location(&mut self, location: LocationInfo) -> Result<(), HostError> {
            self.locations.push((
                location.name.to_string_lossy().into_owned(),
                location.flags,
                record_location(location.location),
            ));
            Ok(())
        }

        fn declare_soundpack(&mut self, _soundpack: Soundpack) -> Result<(), HostError> {
            self.soundpacks += 1;
            Ok(())
        }
    }

    /// One preset as the receiver saw it — name/key plus the metadata
    /// fields the provider sets after `begin_preset`.
    #[derive(Default)]
    struct RecordedPreset {
        name: Option<String>,
        load_key: Option<String>,
        plugin_ids: Vec<(String, String)>,
        flags: Flags,
        creators: Vec<String>,
    }

    /// Recording `MetadataReceiverImpl` — the observable half of the
    /// provider's `get_metadata` contract.
    #[derive(Default)]
    struct ProbeReceiver {
        presets: Vec<RecordedPreset>,
        errors: Vec<(i32, Option<String>)>,
    }

    impl MetadataReceiverImpl for ProbeReceiver {
        fn on_error(&mut self, error_code: i32, error_message: Option<&CStr>) {
            self.errors.push((
                error_code,
                error_message.map(|message| message.to_string_lossy().into_owned()),
            ));
        }

        fn begin_preset(
            &mut self,
            name: Option<&CStr>,
            load_key: Option<&CStr>,
        ) -> Result<(), HostError> {
            self.presets.push(RecordedPreset {
                name: name.map(|name| name.to_string_lossy().into_owned()),
                load_key: load_key.map(|key| key.to_string_lossy().into_owned()),
                plugin_ids: Vec::new(),
                flags: Flags::empty(),
                creators: Vec::new(),
            });
            Ok(())
        }

        fn add_plugin_id(&mut self, plugin_id: UniversalPluginId) {
            if let Some(current) = self.presets.last_mut() {
                current.plugin_ids.push((
                    plugin_id.abi.to_string_lossy().into_owned(),
                    plugin_id.id.to_string_lossy().into_owned(),
                ));
            }
        }

        fn set_soundpack_id(&mut self, _soundpack_id: &CStr) {}

        fn set_flags(&mut self, flags: Flags) {
            if let Some(current) = self.presets.last_mut() {
                current.flags = flags;
            }
        }

        fn add_creator(&mut self, creator: &CStr) {
            if let Some(current) = self.presets.last_mut() {
                current
                    .creators
                    .push(creator.to_string_lossy().into_owned());
            }
        }

        fn set_description(&mut self, _description: &CStr) {}

        fn set_timestamps(
            &mut self,
            _creation_time: Option<Timestamp>,
            _modification_time: Option<Timestamp>,
        ) {
        }

        fn add_feature(&mut self, _feature: &CStr) {}

        fn add_extra_info(&mut self, _key: &CStr, _value: &CStr) {}
    }

    /// One `loaded` host callback, recorded.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RecordedLoad {
        location: RecordedLocation,
        load_key: Option<String>,
    }

    /// One `on_error` host callback, recorded.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RecordedPresetError {
        location: RecordedLocation,
        load_key: Option<String>,
        os_error: i32,
        message: Option<String>,
    }

    /// Shared recorder for the preset-load host callbacks plus the
    /// params-rescan signal a gain-changing restore owes.
    #[derive(Clone, Default)]
    struct PresetProbeShared {
        rescans: Arc<Mutex<Vec<ParamRescanFlags>>>,
        loaded: Arc<Mutex<Vec<RecordedLoad>>>,
        errors: Arc<Mutex<Vec<RecordedPresetError>>>,
    }

    struct PresetProbeMainThread<'a> {
        shared: &'a PresetProbeShared,
    }

    struct PresetProbeHost;

    impl SharedHandler<'_> for PresetProbeShared {
        fn request_restart(&self) {}
        fn request_process(&self) {}
        fn request_callback(&self) {}
    }

    impl HostParamsImplShared for PresetProbeShared {
        fn request_flush(&self) {}
    }

    impl<'a> MainThreadHandler<'a> for PresetProbeMainThread<'a> {}

    impl HostParamsImplMainThread for PresetProbeMainThread<'_> {
        fn rescan(&self, flags: ParamRescanFlags) {
            self.shared
                .rescans
                .lock()
                .expect("rescan recorder poisoned")
                .push(flags);
        }

        fn clear(&self, _param_id: ClapId, _flags: ParamClearFlags) {}
    }

    impl HostPresetLoadImpl for PresetProbeMainThread<'_> {
        fn on_error(
            &self,
            location: Location,
            load_key: Option<&CStr>,
            os_error: i32,
            message: Option<&CStr>,
        ) {
            self.shared
                .errors
                .lock()
                .expect("preset-load error recorder poisoned")
                .push(RecordedPresetError {
                    location: record_location(location),
                    load_key: load_key.map(|key| key.to_string_lossy().into_owned()),
                    os_error,
                    message: message.map(|message| message.to_string_lossy().into_owned()),
                });
        }

        fn loaded(&self, location: Location, load_key: Option<&CStr>) {
            self.shared
                .loaded
                .lock()
                .expect("preset-load recorder poisoned")
                .push(RecordedLoad {
                    location: record_location(location),
                    load_key: load_key.map(|key| key.to_string_lossy().into_owned()),
                });
        }
    }

    impl HostHandlers for PresetProbeHost {
        type Shared<'a> = PresetProbeShared;
        type MainThread<'a> = PresetProbeMainThread<'a>;
        type AudioProcessor<'a> = ();

        fn declare_extensions(builder: &mut HostExtensions<Self>, _shared: &Self::Shared<'_>) {
            builder
                .register::<HostParams>()
                .register::<HostPresetLoad>();
        }
    }

    /// Instantiates `test_stateful` through the suite entry under the
    /// recording preset-load host — the real `clap_entry` → factory →
    /// `create_plugin` → extension path.
    fn instantiate_stateful(
        entry: &PluginEntry,
        host_info: &HostInfo,
        shared: PresetProbeShared,
    ) -> PluginInstance<PresetProbeHost> {
        PluginInstance::<PresetProbeHost>::new(
            move |()| shared,
            |shared| PresetProbeMainThread { shared },
            entry,
            STATEFUL_PLUGIN_ID,
            host_info,
        )
        .expect("test_stateful must instantiate")
    }

    /// `clap.factory.preset-discovery` on the real entry exposes
    /// exactly one provider; `init` declares only the built-in
    /// location and `get_metadata` emits the full catalog with
    /// plugin binding, factory flags and creator — all through the
    /// ABI, observed by the recording host-side impls.
    #[test]
    fn preset_discovery_exposes_catalog_through_real_abi() {
        let entry = PluginEntry::load_from_clack::<crate::TestSuiteEntry>(c"suite.clap").unwrap();
        let host_info = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();

        let factory = entry
            .get_factory::<PresetDiscoveryFactory>()
            .expect("entry must expose the preset-discovery factory");
        let descriptors: Vec<_> = factory.provider_descriptors().collect();
        assert_eq!(descriptors.len(), 1, "exactly one preset provider");
        let descriptor = descriptors[0];
        assert_eq!(
            descriptor.id().and_then(|id| id.to_str().ok()),
            Some("org.ruviform-test.clap-presets")
        );
        assert_eq!(
            descriptor.name().and_then(|name| name.to_str().ok()),
            Some("Ruviform Test Presets")
        );
        let provider_id = descriptor.id().expect("provider id present");

        let mut provider =
            Provider::instantiate(ProbeIndexer::default(), &entry, provider_id, &host_info)
                .expect("provider must instantiate and init");

        // `init` declared exactly the built-in location — factory
        // content, no file types, no soundpacks.
        assert_eq!(provider.indexer().locations.len(), 1);
        let (name, flags, location) = &provider.indexer().locations[0];
        assert_eq!(name, "Built-in");
        assert!(flags.contains(Flags::IS_FACTORY_CONTENT));
        assert_eq!(*location, RecordedLocation::Plugin);
        assert_eq!(provider.indexer().filetypes, 0);
        assert_eq!(provider.indexer().soundpacks, 0);

        let mut receiver = ProbeReceiver::default();
        provider.get_metadata(Location::Plugin, &mut receiver);
        assert_eq!(receiver.errors.len(), 0);
        assert_eq!(receiver.presets.len(), PRESETS.len());
        for (preset, recorded) in PRESETS.iter().zip(&receiver.presets) {
            let name = preset.name.to_str().expect("preset name is UTF-8");
            assert_eq!(recorded.name.as_deref(), Some(name));
            assert_eq!(recorded.load_key.as_deref(), Some(name));
            assert!(recorded.flags.contains(Flags::IS_FACTORY_CONTENT));
            assert_eq!(
                recorded.plugin_ids,
                vec![(
                    "clap".to_string(),
                    "org.ruviform-test.clap-test-stateful".to_string()
                )]
            );
            assert_eq!(recorded.creators, vec!["Ruviform Test".to_string()]);
        }
    }

    /// `clap.ext.preset-load` `from_location` on `test_stateful`
    /// restores the catalog preset's gain through the state law —
    /// observable via `clap.ext.state` `save` — fires the host's
    /// `loaded` callback and owes a `VALUES` params rescan.
    #[test]
    fn preset_load_restores_gain_and_reports_loaded() {
        let shared = PresetProbeShared::default();
        let rescans = Arc::clone(&shared.rescans);
        let loaded = Arc::clone(&shared.loaded);
        let errors = Arc::clone(&shared.errors);
        let entry = PluginEntry::load_from_clack::<crate::TestSuiteEntry>(c"suite.clap").unwrap();
        let host_info = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let mut instance = instantiate_stateful(&entry, &host_info, shared);

        let handle = instance.plugin_handle();
        let preset_load = handle
            .get_extension::<PluginPresetLoad>()
            .expect("test_stateful must expose clap.ext.preset-load");
        let state = handle
            .get_extension::<PluginState>()
            .expect("test_stateful must expose clap.ext.state");

        // "Boost" → gain 1.5, different from the 1.0 default.
        preset_load
            .load_from_location(&handle, Location::Plugin, Some(c"Boost"))
            .expect("catalog preset must load");

        // The restored gain is the state blob's payload — the same
        // cell `clap.ext.state` save emits.
        let mut bytes = Vec::new();
        state.save(&handle, &mut bytes).unwrap();
        assert_eq!(bytes, encode_state_blob(1.5));

        // Host saw `loaded` for the exact location/key and the gain
        // change owed a VALUES rescan.
        assert_eq!(
            loaded.lock().expect("loaded recorder poisoned").as_slice(),
            &[RecordedLoad {
                location: RecordedLocation::Plugin,
                load_key: Some("Boost".to_string()),
            }]
        );
        assert!(errors.lock().expect("error recorder poisoned").is_empty());
        assert_eq!(
            rescans.lock().expect("rescan recorder poisoned").as_slice(),
            &[ParamRescanFlags::VALUES]
        );

        // Re-loading the same preset is a no-op for the rescan (the
        // gain did not change) but still reports `loaded`.
        preset_load
            .load_from_location(&handle, Location::Plugin, Some(c"Boost"))
            .expect("catalog preset must load");
        assert_eq!(
            rescans.lock().expect("rescan recorder poisoned").len(),
            1,
            "identical restore must not rescan again"
        );
        assert_eq!(loaded.lock().expect("loaded recorder poisoned").len(), 2);
    }

    /// `clap.ext.preset-load` rejections are defined: file locations,
    /// missing keys and unknown keys all fail the call AND report
    /// `on_error` to the host — the mirror stays untouched.
    #[test]
    fn preset_load_rejects_undefined_inputs_and_reports_errors() {
        let shared = PresetProbeShared::default();
        let loaded = Arc::clone(&shared.loaded);
        let errors = Arc::clone(&shared.errors);
        let entry = PluginEntry::load_from_clack::<crate::TestSuiteEntry>(c"suite.clap").unwrap();
        let host_info = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let mut instance = instantiate_stateful(&entry, &host_info, shared);

        let handle = instance.plugin_handle();
        let preset_load = handle
            .get_extension::<PluginPresetLoad>()
            .expect("test_stateful must expose clap.ext.preset-load");
        let state = handle
            .get_extension::<PluginState>()
            .expect("test_stateful must expose clap.ext.state");

        // Establish a known gain first so rejections provably leave it.
        preset_load
            .load_from_location(&handle, Location::Plugin, Some(c"Half"))
            .expect("catalog preset must load");
        let mut bytes = Vec::new();
        state.save(&handle, &mut bytes).unwrap();
        assert_eq!(bytes, encode_state_blob(0.5));

        // File location — declared nowhere by the provider.
        let file = c"/tmp/factory.rvf";
        assert!(
            preset_load
                .load_from_location(&handle, Location::File { path: file }, Some(c"Half"))
                .is_err()
        );
        // Missing load key.
        assert!(
            preset_load
                .load_from_location(&handle, Location::Plugin, None)
                .is_err()
        );
        // Unknown load key.
        assert!(
            preset_load
                .load_from_location(&handle, Location::Plugin, Some(c"Nope"))
                .is_err()
        );

        // The gain is untouched — still the "Half" preset's 0.5.
        let mut bytes = Vec::new();
        state.save(&handle, &mut bytes).unwrap();
        assert_eq!(bytes, encode_state_blob(0.5));

        // `loaded` fired only for the one successful load; `on_error`
        // fired once per rejection with the location/key echoed.
        assert_eq!(loaded.lock().expect("loaded recorder poisoned").len(), 1);
        let errors = errors.lock().expect("error recorder poisoned");
        assert_eq!(errors.len(), 3);
        assert_eq!(
            errors[0].location,
            RecordedLocation::File("/tmp/factory.rvf".to_string())
        );
        assert_eq!(errors[0].load_key.as_deref(), Some("Half"));
        assert_eq!(errors[1].location, RecordedLocation::Plugin);
        assert_eq!(errors[1].load_key, None);
        assert_eq!(errors[2].location, RecordedLocation::Plugin);
        assert_eq!(errors[2].load_key.as_deref(), Some("Nope"));
    }
}
