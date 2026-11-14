//! Production host-path verification (Issue #630 first slice).
//!
//! These tests `dlopen` **this crate's own built cdylib** through
//! `ruviform_plugin_vst3::host::Vst3Library` — the same L0
//! in-process production host bridge the plugin host uses — and drive
//! the real VST3 ABI end to end: `GetPluginFactory` → class scan →
//! `createInstance` (component + distributed controller) →
//! initialize/activate/setupProcessing → `process` → deactivate /
//! terminate on drop. No `Plugin` port fake is involved anywhere in
//! the path — the module binary is the only artifact under test.
//!
//! The fixture binary resolves by the same `current_exe`-relative
//! `deps/` convention the adapter's own fixture tests use; a missing
//! binary is an assertion failure, never a silent skip.

use std::path::PathBuf;

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::{
    ErasedPlugin, Event, EventKind, EventList, HostInput, HostOutput, ParamEvent, ParamId,
    ParamValue, PluginFactory, ProcessConfig, ProcessContext, ProcessResult,
};
use ruviform_plugin_vst3::host::Vst3Library;

const FRAMES: usize = 64;
const FRAMES_U32: u32 = 64;

/// This crate's cdylib file name — `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`
/// — the test spec's fixture resolution.
fn fixture_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe is available");
    let deps = exe.parent().expect("test binary lives in deps/");
    deps.join(fixture_lib_name())
}

fn load_library() -> Vst3Library {
    let path = fixture_path();
    assert!(
        path.is_file(),
        "test fixture cdylib missing: {}",
        path.display()
    );
    Vst3Library::load(&path).expect("fixture dlopen succeeds")
}

/// The full legal handshake through the production host bridge:
/// `Vst3Library::load` → `factories()` → `create()` → `init` →
/// `prepare` → `start_processing` — the same sequence the L1 runner
/// drives (`prepared_plugin()` precedent in the adapter's host tests).
fn prepared_plugin() -> Box<dyn ErasedPlugin> {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 1, "the module ships one factory");
    let mut plugin = factories[0].create();
    plugin.init().expect("init");
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
    plugin
        .prepare(&config, &mut ProcessContext::new())
        .expect("prepare");
    plugin.start_processing().expect("start_processing");
    plugin
}

/// Drives one 64-frame stereo block of golden PCM through the
/// production path — deterministic input `0.5 / -0.25` — and returns
/// the neutral `ProcessResult` plus the output channels.
fn drive_block(plugin: &mut dyn ErasedPlugin, events: &[Event]) -> (ProcessResult, [Vec<f32>; 2]) {
    let in_left = vec![0.5_f32; FRAMES];
    let in_right = vec![-0.25_f32; FRAMES];
    let in_channels = [&in_left[..], &in_right[..]];
    let in_view = PlanarBufferView::new(&in_channels).unwrap();
    let mut out_left = vec![0.0_f32; FRAMES];
    let mut out_right = vec![0.0_f32; FRAMES];
    let result = {
        let mut out_channels = [&mut out_left[..], &mut out_right[..]];
        let mut out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();
        let inputs = [HostInput(&in_view)];
        let mut outputs = [HostOutput(&mut out_view)];
        let list = EventList::new(events, FRAMES_U32).unwrap();
        plugin.process_host(
            &inputs,
            &mut outputs,
            list,
            None,
            &mut ProcessContext::new(),
        )
    };
    (result, [out_left, out_right])
}

/// Discovery through the production path: `Vst3Library::load` scans
/// the real binary's factory, finds exactly the `test_gain` component
/// class (the distributed controller class is filtered out, as with
/// every VST3 module), and reports the fixture's vendor/name.
#[test]
fn discovers_test_gain_through_production_host() {
    let library = load_library();
    let descriptors = library.descriptors();
    assert_eq!(
        descriptors.len(),
        1,
        "only the audio component class is a plugin descriptor"
    );
    let descriptor = descriptors[0];
    assert_eq!(descriptor.name, "test_gain");
    assert_eq!(descriptor.vendor, "Ruviform Test");
    assert_eq!(descriptor.version, "0.0.0");
    assert!(descriptor.features.contains(&"Fx"));
}

/// Parameter surface through the production path: the host instantiates
/// the distributed controller via `getControllerClassId`, reads
/// `IParameterInfo`, and maps the normalized domain through
/// `normalizedParamToPlain` — the reported plain range `[0.0, 2.0]`
/// default `1.0` proves the non-identity mapping end to end.
#[test]
fn reports_gain_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 1);
    let plugin = factories[0].create();
    let params = plugin.params();
    assert_eq!(params.len(), 1, "test_gain exposes exactly one parameter");
    let gain = params[0];
    assert_eq!(gain.id.0, 0);
    assert_eq!(gain.name, "Gain");
    assert_eq!(gain.min_value, 0.0);
    assert_eq!(gain.max_value, 2.0);
    assert_eq!(gain.default_value, 1.0);
}

/// The core oracle: fixed PCM golden vectors plus a normalized
/// `0.75` automation point (→ plain `1.5`) at frame offset 8 of a
/// 64-frame block. Frames 0–7 keep unity gain; frames 8–63 use `1.5`.
/// A second event-free block proves the gain persists through the
/// real ABI. Drop runs the host's deactivate/terminate teardown —
/// the uninstantiate half of the contract.
#[test]
fn processes_audio_with_sample_accurate_gain_events() {
    let mut plugin = prepared_plugin();

    // normalized 0.75 → plain 1.5, effective at frame 8.
    let events = [Event::new(
        8,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.75).unwrap(),
        }),
    )];
    let (result, [left, right]) = drive_block(&mut *plugin, &events);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));

    let mut expected_left = vec![0.5_f32; 8];
    expected_left.extend(vec![0.75_f32; FRAMES - 8]);
    let mut expected_right = vec![-0.25_f32; 8];
    expected_right.extend(vec![-0.375_f32; FRAMES - 8]);
    assert_eq!(
        left, expected_left,
        "left channel: unity then ×1.5 at frame 8"
    );
    assert_eq!(
        right, expected_right,
        "right channel: unity then ×1.5 at frame 8"
    );

    // No new events: the adopted gain persists into the next block —
    // the whole block runs at ×1.5 through the real ABI.
    let (_result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert_eq!(left, vec![0.75_f32; FRAMES]);
    assert_eq!(right, vec![-0.375_f32; FRAMES]);
}
