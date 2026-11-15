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
    ErasedPlugin, Event, EventKind, EventList, HostError, HostInput, HostOutput, ParamEvent,
    ParamId, ParamValue, ParentLease, PluginFactory, ProcessConfig, ProcessContext, ProcessResult,
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
/// `index` selects the suite plugin (`0` = `test_gain`,
/// `1` = `test_fault`, `2` = `test_editor`).
fn prepared_plugin_at(index: usize) -> Box<dyn ErasedPlugin> {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 3, "the module ships three plugins");
    let mut plugin = factories[index].create();
    plugin.init().expect("init");
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
    plugin
        .prepare(&config, &mut ProcessContext::new())
        .expect("prepare");
    plugin.start_processing().expect("start_processing");
    plugin
}

/// `test_gain`'s prepared instance — the gain oracle's entry point.
fn prepared_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(0)
}

/// `test_fault`'s prepared instance — the fault oracle's entry point.
fn prepared_fault_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(1)
}

/// `test_editor`'s prepared instance — the editor oracle's entry
/// point (its `IPlugView` is exercised through `open_gui` below and
/// the in-crate COM-level tests).
fn prepared_editor_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(2)
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
/// the real binary's factory, finds the `test_gain`, `test_fault`,
/// and `test_editor` component classes (the distributed controller
/// classes are filtered out, as with every VST3 module), and reports
/// each fixture's vendor/name.
#[test]
fn discovers_suite_plugins_through_production_host() {
    let library = load_library();
    let descriptors = library.descriptors();
    assert_eq!(
        descriptors.len(),
        3,
        "only the audio component classes are plugin descriptors"
    );
    let gain = descriptors[0];
    assert_eq!(gain.name, "test_gain");
    assert_eq!(gain.vendor, "Ruviform Test");
    assert_eq!(gain.version, "0.0.0");
    assert!(gain.features.contains(&"Fx"));
    let fault = descriptors[1];
    assert_eq!(fault.name, "test_fault");
    assert_eq!(fault.vendor, "Ruviform Test");
    assert_eq!(fault.version, "0.0.0");
    assert!(fault.features.contains(&"Fx"));
    let editor = descriptors[2];
    assert_eq!(editor.name, "test_editor");
    assert_eq!(editor.vendor, "Ruviform Test");
    assert_eq!(editor.version, "0.0.0");
    assert!(editor.features.contains(&"Fx"));
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
    assert_eq!(factories.len(), 3);
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

/// The fault parameter surface through the production path:
/// `test_fault`'s distributed controller declares the stepped `Fault`
/// parameter — plain range `[0.0, 4.0]`, default `0.0` (`none`).
#[test]
fn reports_fault_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 3);
    let plugin = factories[1].create();
    let params = plugin.params();
    assert_eq!(params.len(), 1, "test_fault exposes exactly one parameter");
    let fault = params[0];
    assert_eq!(fault.id.0, 0);
    assert_eq!(fault.name, "Fault");
    assert_eq!(fault.min_value, 0.0);
    assert_eq!(fault.max_value, 4.0);
    assert_eq!(fault.default_value, 0.0);
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

/// `test_fault` unarmed: an event-free block is pure pass-through —
/// `out == in` bitwise through the real ABI — and an explicit `none`
/// arming event (normalized `0.0` → plain `0.0`) stays inert.
/// (`abort`/`exit`/`hang` are never
/// driven in-process — they would kill this test binary; they are
/// L1-only faults.)
#[test]
fn fault_unarmed_and_none_mode_are_pass_through() {
    let mut plugin = prepared_fault_plugin();

    let (result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);

    // normalized 0.0 → plain 0.0 → mode `none` — still inert.
    let none_events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.0).unwrap(),
        }),
    )];
    let (result, [left, right]) = drive_block(&mut *plugin, &none_events);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// `test_fault` `error` mode through the real ABI: normalized `1.0`
/// → plain `4.0` → the plugin returns `kResultFalse`, which the
/// production bridge reports as `ProcessStatus::Error` — a defined
/// rejection, not a crash. The instance stays alive: a follow-up
/// `none` event (normalized `0.0`) restores pass-through.
#[test]
fn fault_error_mode_is_defined_rejection_not_crash() {
    let mut plugin = prepared_fault_plugin();

    let error_events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(1.0).unwrap(),
        }),
    )];
    let (result, _out) = drive_block(&mut *plugin, &error_events);
    assert_eq!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Error,
        "armed error mode must surface as a defined process error"
    );

    // Disarm: normalized 0.0 → plain 0.0 → mode `none`.
    let none_events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.0).unwrap(),
        }),
    )];
    let (result, [left, right]) = drive_block(&mut *plugin, &none_events);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// `test_editor`'s parameter surface through the production path:
/// its distributed controller declares **zero** parameters — the
/// surface the component's sentinel `param_id` mirrors by rejecting
/// every `IParamValueQueue` point.
#[test]
fn editor_reports_no_params() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 3);
    let plugin = factories[2].create();
    assert!(
        plugin.params().is_empty(),
        "test_editor exposes no parameters"
    );
}

/// `test_editor` audio through the production path: a parameterless
/// pass-through — `out == in` bitwise — and a foreign `ParamValue`
/// event (which the class cannot produce) is dropped by the
/// component's queue filter, changing nothing.
#[test]
fn editor_processes_audio_as_pass_through() {
    let mut plugin = prepared_editor_plugin();

    let events = [Event::new(
        0,
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
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// `open_gui` on `test_editor` through the production host bridge
/// with an opaque lease payload (`ParentLease::new(0, 0)` — payload
/// `0` is not a real HWND): on Windows the call chain reaches
/// `attached`, whose `IsWindow` gate rejects the non-window parent
/// with `kResultFalse` → `PluginFailed` (with the `setFrame(null)`
/// rollback); on non-Windows `EDITOR_PLATFORM_TYPE` is `None` →
/// `Unsupported` before `createView` is even reached. Both are
/// defined rejections — a **real** HWND attach is the #631
/// real-dock lane's evidence, not claimed here.
#[test]
fn editor_open_gui_rejects_opaque_parent_lease() {
    let mut plugin = prepared_editor_plugin();
    let Err(err) = plugin.open_gui(ParentLease::new(0, 0)) else {
        panic!("an opaque lease payload must not produce a GUI handle");
    };
    #[cfg(target_os = "windows")]
    assert_eq!(err, HostError::PluginFailed);
    #[cfg(not(target_os = "windows"))]
    assert_eq!(err, HostError::Unsupported);
}

/// `test_gain`/`test_fault` ship no editor: `createView` returns
/// null on every platform, so `open_gui` converges to the honest
/// `Unsupported` — the `test_editor` class is the suite's only GUI
/// surface.
#[test]
fn gain_and_fault_have_no_editor_view() {
    for mut plugin in [prepared_plugin(), prepared_fault_plugin()] {
        let Err(err) = plugin.open_gui(ParentLease::new(0, 0)) else {
            panic!("test_gain/test_fault ship no editor — open_gui must fail");
        };
        assert_eq!(err, HostError::Unsupported);
    }
}
