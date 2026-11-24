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
use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
use ruviform_plugin_core::{
    ErasedPlugin, Event, EventKind, EventList, GuiFocusTarget, GuiPopupKind, HostError, HostInput,
    HostOutput, ParamEvent, ParamId, ParamValue, ParentLease, PluginFactory, ProcessConfig,
    ProcessContext, ProcessResult,
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
/// `1` = `test_fault`, `2` = `test_editor`, `3` = `test_stateful`,
/// `4` = `test_instrument`, `5` = `test_spawn_fail`,
/// `6` = `test_stateful_single`).
fn prepared_plugin_at(index: usize) -> Box<dyn ErasedPlugin> {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 7, "the module ships seven plugins");
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

/// `test_stateful`'s prepared instance — the state oracle's entry
/// point (its `setState`/`getState` verbs are exercised through
/// `save_state`/`restore_state` below).
fn prepared_stateful_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(3)
}

/// `test_instrument`'s prepared instance — the instrument oracle's
/// entry point (its `IEventList` MIDI and `ProcessContext` transport
/// are exercised through `process_host` below).
fn prepared_instrument_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(4)
}

/// `test_stateful_single`'s prepared instance — the
/// single-component oracle's entry point. `init` is where the
/// bridge's `single_component` arm runs: own-CID detection → the
/// shared object's `cast::<IEditController>` → one `initialize` →
/// `setComponentHandler` on the shared object → no `IConnectionPoint`
/// pairing (the single arm's guard skips the pairing block; even a
/// bridge that tried would silently skip on the `None` cast).
fn prepared_single_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(6)
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
/// `test_editor`, `test_stateful`, `test_instrument`,
/// `test_spawn_fail`, and `test_stateful_single` component classes
/// (the distributed controller classes are filtered out, as
/// with every VST3 module), and reports each fixture's
/// vendor/name/features.
#[test]
fn discovers_suite_plugins_through_production_host() {
    let library = load_library();
    let descriptors = library.descriptors();
    assert_eq!(
        descriptors.len(),
        7,
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
    let stateful = descriptors[3];
    assert_eq!(stateful.name, "test_stateful");
    assert_eq!(stateful.vendor, "Ruviform Test");
    assert_eq!(stateful.version, "0.0.0");
    assert!(stateful.features.contains(&"Fx"));
    let instrument = descriptors[4];
    assert_eq!(instrument.name, "test_instrument");
    assert_eq!(instrument.vendor, "Ruviform Test");
    assert_eq!(instrument.version, "0.0.0");
    assert!(instrument.features.contains(&"Instrument"));
    assert!(instrument.features.contains(&"Fx"));
    let spawn_fail = descriptors[5];
    assert_eq!(spawn_fail.name, "test_spawn_fail");
    assert_eq!(spawn_fail.vendor, "Ruviform Test");
    assert_eq!(spawn_fail.version, "0.0.0");
    assert!(spawn_fail.features.contains(&"Fx"));
    let single = descriptors[6];
    assert_eq!(single.name, "test_stateful_single");
    assert_eq!(single.vendor, "Ruviform Test");
    assert_eq!(single.version, "0.0.0");
    assert!(single.features.contains(&"Fx"));
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
    assert_eq!(factories.len(), 7);
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
/// parameter — plain range `[0.0, 8.0]`, default `0.0` (`none`).
#[test]
fn reports_fault_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 7);
    let plugin = factories[1].create();
    let params = plugin.params();
    assert_eq!(params.len(), 1, "test_fault exposes exactly one parameter");
    let fault = params[0];
    assert_eq!(fault.id.0, 0);
    assert_eq!(fault.name, "Fault");
    assert_eq!(fault.min_value, 0.0);
    assert_eq!(fault.max_value, 8.0);
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

/// `test_fault` `error` mode through the real ABI: normalized `0.5`
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
            value: ParamValue::new(0.5).unwrap(),
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

/// One `Fault` arming event at normalized `normalized` — the shared
/// builder for the safe in-process fault modes below.
fn fault_arm(normalized: f64) -> Event {
    Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(normalized).unwrap(),
        }),
    )
}

/// `test_fault` `rt_alloc` mode through the real ABI: normalized
/// `0.625` → plain `5.0` → the armed block's `process()` runs a
/// bounded heap allocation and writes `f32::from(0xC8)` = `200.0` —
/// read *from* the allocated buffer — into `output[0][0]`. The
/// marker is the fixture's own report that the RT violation ran (a
/// host-side allocator cannot observe the plugin's heap); the rest
/// of the block stays pass-through. Disarm restores clean audio.
#[test]
fn fault_rt_alloc_mode_marks_armed_block() {
    let mut plugin = prepared_fault_plugin();

    let (result, [left, right]) = drive_block(&mut *plugin, &[fault_arm(0.625)]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left[0], 200.0, "the heap marker proves the alloc ran");
    assert_eq!(left[1..], vec![0.5_f32; FRAMES - 1]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);

    let (result, [left, right]) = drive_block(&mut *plugin, &[fault_arm(0.0)]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// `test_fault` `bad_time` mode through the real ABI: normalized
/// `0.75` → plain `6.0` → a zero-tail pass-through fixture claims
/// `TAIL_INFINITE` — `getTailSamples` reports `0xFFFFFFFF`, which
/// the production bridge surfaces as `tail_frames == u32::MAX` —
/// the bad-timing lie the host oracle detects. Disarm restores the
/// zero-tail truth.
#[test]
fn fault_bad_time_mode_claims_infinite_tail() {
    let mut plugin = prepared_fault_plugin();

    let (result, _out) = drive_block(&mut *plugin, &[fault_arm(0.75)]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(
        result.tail_frames,
        u32::MAX,
        "bad_time must claim TAIL_INFINITE on getTailSamples"
    );

    let (result, _out) = drive_block(&mut *plugin, &[fault_arm(0.0)]);
    assert_eq!(result.tail_frames, 0, "disarm restores a zero tail");
}

/// `test_fault` `invalid` mode through the real ABI: normalized
/// `0.875` → plain `7.0` → the armed block delivers a full-NaN
/// output — the invalid audio state a host must detect and isolate.
/// Disarm restores clean pass-through.
#[test]
fn fault_invalid_mode_fills_nan_outputs() {
    let mut plugin = prepared_fault_plugin();

    let (result, [left, right]) = drive_block(&mut *plugin, &[fault_arm(0.875)]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert!(left.iter().all(|s| s.is_nan()));
    assert!(right.iter().all(|s| s.is_nan()));

    let (result, [left, right]) = drive_block(&mut *plugin, &[fault_arm(0.0)]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// `test_editor`'s parameter surface through the production path
/// (Issue #885): its distributed controller declares the shared
/// stepped `Fault` parameter — plain range `[0.0, 8.0]`, default
/// `0.0` (`none`) — the same descriptor `test_fault` reports.
#[test]
fn editor_reports_fault_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 7);
    let plugin = factories[2].create();
    let params = plugin.params();
    assert_eq!(params.len(), 1, "test_editor exposes exactly one parameter");
    let fault = params[0];
    assert_eq!(fault.id.0, 0);
    assert_eq!(fault.name, "Fault");
    assert_eq!(fault.min_value, 0.0);
    assert_eq!(fault.max_value, 8.0);
    assert_eq!(fault.default_value, 0.0);
}

/// `test_editor` audio through the production path: unarmed the
/// class is pure pass-through — `out == in` bitwise — and a
/// `ParamValue` event on a foreign parameter id (anything but the
/// `Fault` id `0`) is dropped by the component's `wanted` queue
/// filter, arming nothing and changing nothing.
#[test]
fn editor_processes_audio_as_pass_through() {
    let mut plugin = prepared_editor_plugin();

    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(1),
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

/// `test_editor` `error` mode through the real ABI (Issue #885):
/// normalized `0.5` → plain `4.0` — the `Fault` event passes the
/// component's `wanted` filter, arms inside the block's `process()`,
/// and the plugin returns `kResultFalse`, which the production
/// bridge reports as `ProcessStatus::Error` — a defined rejection,
/// not a crash. The instance stays alive: a follow-up `none` event
/// (normalized `0.0`) restores pass-through.
#[test]
fn editor_fault_error_mode_is_defined_rejection() {
    let mut plugin = prepared_editor_plugin();

    let error_events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.5).unwrap(),
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

/// The seam-backed editor verbs (`set_gui_focus_target` /
/// `set_gui_popup`, Issue #630 slice 3) reach `Vst3Plugin` through
/// the production bridge: with no GUI open every arm answers the
/// defined `Unsupported` — on `test_editor` (a GUI class whose view
/// is simply not open) and on `test_gain` (a class with no view at
/// all) alike.
#[test]
fn editor_verbs_without_open_gui_are_unsupported() {
    for mut plugin in [prepared_editor_plugin(), prepared_plugin()] {
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::Editor),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 3 }),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::HostWindow),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::Dialog, true),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::ChildWindow, false),
            Err(HostError::Unsupported)
        );
    }
}

/// A declined `open_gui` rolls the view back (`PluginFailed` on
/// Windows where `IsWindow` rejects the opaque lease, `Unsupported`
/// off-Windows where the platform type is absent) — either way no
/// view is stored, so the editor verbs keep answering `Unsupported`
/// afterwards rather than reaching a stale `IPlugView`.
#[test]
fn editor_verbs_after_declined_open_stay_unsupported() {
    let mut plugin = prepared_editor_plugin();
    let _ = plugin.open_gui(ParentLease::new(0, 0));
    assert_eq!(
        plugin.set_gui_focus_target(GuiFocusTarget::Editor),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 3 }),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        plugin.set_gui_popup(GuiPopupKind::Dialog, true),
        Err(HostError::Unsupported)
    );
}

/// Window-text of `hwnd` as a `String` — empty when `hwnd` is dead
/// or carries no title (`GetWindowText*` decline foreign or stale
/// handles rather than faulting the caller).
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
fn window_text(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};

    // SAFETY: `GetWindowText*` accept any HWND value; `buf` is a
    // valid writable buffer for the reported length.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; usize::try_from(len).unwrap_or(0) + 1];
        let read = GetWindowTextW(
            hwnd,
            buf.as_mut_ptr(),
            i32::try_from(buf.len()).unwrap_or(i32::MAX),
        );
        String::from_utf16_lossy(&buf[..usize::try_from(read).unwrap_or(0)])
    }
}

/// `EnumChildWindows` callback — records into `lparam` the first
/// HWND whose title starts with the `rvf-test-editor-child-` token.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
unsafe extern "system" fn find_child_surface(
    hwnd: windows_sys::Win32::Foundation::HWND,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> i32 {
    if window_text(hwnd).starts_with("rvf-test-editor-child-") {
        // SAFETY: `lparam` is the caller's `&mut HWND` for the
        // callback's duration.
        unsafe { *(lparam as *mut windows_sys::Win32::Foundation::HWND) = hwnd };
        return 0;
    }
    1
}

/// `EnumWindows` callback — same shape for the top-level
/// `rvf-test-editor-popup-` dialog surface.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
unsafe extern "system" fn find_popup_surface(
    hwnd: windows_sys::Win32::Foundation::HWND,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> i32 {
    if window_text(hwnd).starts_with("rvf-test-editor-popup-") {
        // SAFETY: `lparam` is the caller's `&mut HWND` for the
        // callback's duration.
        unsafe { *(lparam as *mut windows_sys::Win32::Foundation::HWND) = hwnd };
        return 0;
    }
    1
}

/// Windows-only real-HWND lane: `open_gui` on a real container HWND,
/// then every editor verb through the production bridge — no test
/// fake anywhere in the path. Oracles: the fixture stamps verb
/// tokens into the editor child HWND's title
/// (`rvf-test-editor-attach-{n}|{ev},{ev},…`), `GetFocus` witnesses
/// the real focus moves (`Editor` → editor child, `HostWindow` →
/// container), `EnumChildWindows` finds the `WS_CHILD` popup
/// surface under the editor child and `EnumWindows` the `WS_POPUP`
/// dialog surface by title.
#[cfg(target_os = "windows")]
#[ignore = "requires a Windows window station — real HWND verification lane"]
#[expect(unsafe_code)]
#[test]
fn editor_verbs_drive_real_windows_through_bridge() {
    use std::ptr;

    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, EnumChildWindows, EnumWindows, GW_CHILD, GetWindow,
        IsWindow, WS_OVERLAPPED,
    };

    let class: Vec<u16> = "STATIC".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: valid class name, nulls where Win32 permits; the
    // returned HWND is this thread's — every subsequent call drives
    // it on the same thread.
    let parent = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            WS_OVERLAPPED,
            0,
            0,
            320,
            240,
            ptr::null_mut(),
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null(),
        )
    };
    assert!(!parent.is_null(), "cannot create a container HWND");

    let mut plugin = prepared_editor_plugin();
    plugin
        .open_gui(ParentLease::new(0, parent as u64))
        .expect("open_gui on a real parent HWND");

    // SAFETY: `parent` is a live HWND this thread owns; the editor
    // child lives under it.
    unsafe {
        let editor_child: HWND = GetWindow(parent, GW_CHILD);
        assert!(
            !editor_child.is_null(),
            "attached must create the editor child"
        );

        // `Editor` — `onFocus(1)`: the fixture SetFocus's the child.
        plugin
            .set_gui_focus_target(GuiFocusTarget::Editor)
            .expect("focus target Editor");
        assert_eq!(GetFocus(), editor_child, "f:1 must move real focus");

        // `EditorElement` — the seam's `apply_focus` delivery token.
        plugin
            .set_gui_focus_target(GuiFocusTarget::EditorElement { element: 7 })
            .expect("focus target EditorElement");

        // `ChildWindow` — a real `WS_CHILD` surface under the
        // editor child.
        plugin
            .set_gui_popup(GuiPopupKind::ChildWindow, true)
            .expect("popup ChildWindow open");
        let mut child_surface: HWND = ptr::null_mut();
        let _ = EnumChildWindows(
            editor_child,
            Some(find_child_surface),
            &raw mut child_surface as LPARAM,
        );
        assert!(
            !child_surface.is_null(),
            "EnumChildWindows must find rvf-test-editor-child-1"
        );

        // `Dialog` — the real `WS_POPUP` surface (top-level — found
        // by its unique title through `EnumWindows`).
        plugin
            .set_gui_popup(GuiPopupKind::Dialog, true)
            .expect("popup Dialog open");
        let mut popup_surface: HWND = ptr::null_mut();
        let _ = EnumWindows(Some(find_popup_surface), &raw mut popup_surface as LPARAM);
        assert!(
            !popup_surface.is_null(),
            "EnumWindows must find rvf-test-editor-popup-1"
        );

        // `HostWindow` — real SetFocus back to the container, and the
        // plugin gets its `onFocus(false)` notification (`f:0`).
        plugin
            .set_gui_focus_target(GuiFocusTarget::HostWindow)
            .expect("focus target HostWindow");
        assert_eq!(GetFocus(), parent, "focus must return to the container");

        // The whole verb chain rides the title-token channel.
        let title = window_text(editor_child);
        for token in ["f:1", "fe:7:1", "co:1", "po:1", "f:0"] {
            assert!(title.contains(token), "missing {token} in {title}");
        }

        // Close both surfaces, then `close_gui` tears everything down.
        plugin
            .set_gui_popup(GuiPopupKind::ChildWindow, false)
            .expect("popup ChildWindow close");
        plugin
            .set_gui_popup(GuiPopupKind::Dialog, false)
            .expect("popup Dialog close");
        plugin.close_gui().expect("close_gui");
        assert_eq!(IsWindow(editor_child), 0, "removed must destroy the child");

        let _ = DestroyWindow(parent);
    }
}

/// `test_stateful`'s parameter surface through the production path:
/// the same `Gain` parameter `test_gain` declares — plain `[0.0, 2.0]`,
/// default `1.0`, automatable — read back through the distributed
/// controller's `IParameterInfo`.
#[test]
fn stateful_reports_gain_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 7);
    let plugin = factories[3].create();
    let params = plugin.params();
    assert_eq!(
        params.len(),
        1,
        "test_stateful exposes exactly one parameter"
    );
    let gain = params[0];
    assert_eq!(gain.id.0, 0);
    assert_eq!(gain.name, "Gain");
    assert_eq!(gain.min_value, 0.0);
    assert_eq!(gain.max_value, 2.0);
    assert_eq!(gain.default_value, 1.0);
}

/// `test_stateful`'s audio law through the production path — the
/// `test_gain` DSP verbatim: a normalized `0.75` event (→ plain
/// `1.5`) at offset 8 switches frames 8–63, and the gain persists
/// into the next block.
#[test]
fn stateful_processes_audio_with_sample_accurate_gain_events() {
    let mut plugin = prepared_stateful_plugin();

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
    assert_eq!(left, expected_left);
    assert_eq!(right, expected_right);

    let (_result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert_eq!(left, vec![0.75_f32; FRAMES]);
    assert_eq!(right, vec![-0.375_f32; FRAMES]);
}

/// The state oracle through the production VST3 bridge: `save_state`
/// drives `IComponent::getState` + `IEditController::getState` and
/// frames the two payloads; `restore_state` drives the documented
/// `component.setState` → `component.getState` →
/// `controller.setComponentState` → `controller.setState` sequence.
/// The save → mutate → restore → save round-trip is byte-exact —
/// the determinism the L1 e2e asserts out-of-process.
#[test]
fn stateful_save_mutate_restore_save_round_trips() {
    let mut plugin = prepared_stateful_plugin();

    // Initial save — default plain gain 1.0 / normalized 0.5.
    // Framed blob: [u32 comp_len][comp][u32 ctrl_len][ctrl], each
    // payload the fixture's 20-byte v2 `["RFST"][ver=2][f64][cksum]`
    // blob.
    let initial = plugin.save_state().expect("save_state");
    assert_eq!(initial.len(), 48, "two 20-byte payloads + two u32 lengths");
    assert_eq!(&initial[0..4], &20_u32.to_le_bytes());
    assert_eq!(&initial[4..8], b"RFST");
    assert_eq!(
        f64::from_le_bytes(initial[12..20].try_into().unwrap()),
        1.0,
        "component payload: plain gain"
    );
    assert_eq!(&initial[24..28], &20_u32.to_le_bytes());
    assert_eq!(&initial[28..32], b"RFST");
    assert_eq!(
        f64::from_le_bytes(initial[36..44].try_into().unwrap()),
        0.5,
        "controller payload: normalized value"
    );

    // Deterministic: an immediate second save is byte-identical.
    let again = plugin.save_state().expect("save_state");
    assert_eq!(again, initial);

    // Mutate through the real ABI: normalized 0.375 → plain 0.75 at
    // offset 0 — the whole block runs at ×0.75 and the cell adopts it.
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    let (_result, [left, _right]) = drive_block(&mut *plugin, &events);
    assert_eq!(left, vec![0.375_f32; FRAMES]);

    // The mutated save differs — the component payload carries the
    // new plain gain (the controller payload keeps its last-set
    // normalized value: param events reach the component, not the
    // controller mirror).
    let mutated = plugin.save_state().expect("save_state");
    assert_ne!(mutated, initial);
    assert_eq!(
        f64::from_le_bytes(mutated[12..20].try_into().unwrap()),
        0.75
    );

    // Restore the initial blob — the documented verb sequence runs
    // and the re-saved blob is byte-identical to the initial save.
    plugin.restore_state(&initial).expect("restore_state");
    let restored = plugin.save_state().expect("save_state");
    assert_eq!(restored, initial, "save → restore → save is exact");

    // Audio proves the restored gain drives the DSP again: ×1.0 —
    // `out == in` through the real ABI.
    let (_result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// A malformed state blob is a defined `PluginFailed`, never a
/// silent accept — and the instance survives: `save_state` still
/// works afterwards (fault isolation through the real verbs).
#[test]
fn stateful_restore_rejects_malformed_blob() {
    let mut plugin = prepared_stateful_plugin();
    plugin.save_state().expect("save_state");

    // Too short to be the framed format → foreign passthrough → the
    // component payload fails the codec → `PluginFailed`.
    assert_eq!(
        plugin.restore_state(&[0xAA, 0xBB]),
        Err(HostError::PluginFailed)
    );
    // A well-framed blob with a corrupted component magic →
    // `PluginFailed` at `component.setState`.
    let mut corrupt = plugin.save_state().expect("save_state");
    corrupt[4] ^= 0xFF;
    assert_eq!(plugin.restore_state(&corrupt), Err(HostError::PluginFailed));
    // A well-framed blob with one trailing byte → the adapter's
    // `unframe_state` requires `ctrl_end == blob.len()` →
    // `PluginFailed` before any plugin verb runs.
    let mut trailing = plugin.save_state().expect("save_state");
    trailing.push(0x00);
    assert_eq!(
        plugin.restore_state(&trailing),
        Err(HostError::PluginFailed)
    );
    // A well-framed blob whose component payload carries a NaN
    // gain → the stale checksum fails integrity before the value
    // check → `kResultFalse` → `PluginFailed` at `component.setState`.
    let mut nan = plugin.save_state().expect("save_state");
    nan[12..20].copy_from_slice(&f64::NAN.to_le_bytes());
    assert_eq!(plugin.restore_state(&nan), Err(HostError::PluginFailed));
    // A checksum-corrupt component payload → `PluginFailed`.
    let mut corrupt_sum = plugin.save_state().expect("save_state");
    corrupt_sum[20] ^= 0xFF;
    assert_eq!(
        plugin.restore_state(&corrupt_sum),
        Err(HostError::PluginFailed)
    );
    // A version this build does not know → `PluginFailed`.
    let mut future = plugin.save_state().expect("save_state");
    future[8..12].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(plugin.restore_state(&future), Err(HostError::PluginFailed));
    // The instance is still alive and its state surface intact.
    plugin.save_state().expect("save_state still works");
}

/// Restore accepts a framed blob whose payloads are the legacy
/// 16-byte v1 layout and the next `save_state` re-serializes the
/// current v2 layout — migration observable through the production
/// bridge (the frame shrinks in, grows out). A valid-core extension
/// tail on the component payload degrades the same way.
#[test]
fn stateful_restore_migrates_v1_and_degrades_extension() {
    let mut plugin = prepared_stateful_plugin();
    let canonical = plugin.save_state().expect("save_state");

    // Craft a v1 frame: [u32 16][v1 comp][u32 16][v1 ctrl] — each
    // payload is the v2 core with the version field rewritten to 1.
    let comp_v1: Vec<u8> = canonical[4..20]
        .iter()
        .enumerate()
        .map(|(i, &b)| if i == 4 { 1 } else { b })
        .collect();
    let ctrl_v1: Vec<u8> = canonical[28..44]
        .iter()
        .enumerate()
        .map(|(i, &b)| if i == 4 { 1 } else { b })
        .collect();
    let mut v1_frame = Vec::new();
    v1_frame.extend_from_slice(&16_u32.to_le_bytes());
    v1_frame.extend_from_slice(&comp_v1);
    v1_frame.extend_from_slice(&16_u32.to_le_bytes());
    v1_frame.extend_from_slice(&ctrl_v1);

    plugin.restore_state(&v1_frame).expect("v1 restores");
    // The re-save re-serializes canonical v2 — migrated state grew
    // the checksum field back.
    let migrated = plugin.save_state().expect("save_state");
    assert_eq!(migrated, canonical, "v1 in → canonical v2 out");

    // A v2 component payload + unknown extension tail: comp_len 24,
    // valid checksum over the 16-byte core → degraded accept; the
    // re-save drops the tail.
    let mut degraded = Vec::new();
    degraded.extend_from_slice(&24_u32.to_le_bytes());
    degraded.extend_from_slice(&canonical[4..24]);
    degraded.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    degraded.extend_from_slice(&canonical[24..]);
    plugin.restore_state(&degraded).expect("degraded restores");
    let resaved = plugin.save_state().expect("save_state");
    assert_eq!(resaved, canonical, "degraded in → canonical v2 out");
}

/// `test_stateful_single`'s parameter surface through the production
/// path: the class reports its *own* CID from `getControllerClassId`,
/// so the bridge's `single_component` arm resolves `IEditController`
/// by casting the component — and the parameter scan proves the
/// shared object answers the controller surface (`Gain`, plain
/// `[0.0, 2.0]`, default `1.0`).
#[test]
fn single_component_reports_gain_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 7);
    let plugin = factories[6].create();
    let params = plugin.params();
    assert_eq!(
        params.len(),
        1,
        "test_stateful_single exposes exactly one parameter"
    );
    let gain = params[0];
    assert_eq!(gain.id.0, 0);
    assert_eq!(gain.name, "Gain");
    assert_eq!(gain.min_value, 0.0);
    assert_eq!(gain.max_value, 2.0);
    assert_eq!(gain.default_value, 1.0);
}

/// The single-component state oracle through the production VST3
/// bridge — the arm this class exists to reach. `single_component
/// == true` → `save_state` runs `component.getState` only and frames
/// `[u32 comp_len][comp][u32 0]` — 24 bytes total, `ctrl_len = 0`:
/// the empty-controller-payload shape only a real single-component
/// binary produces (the distributed classes always frame two
/// payloads). The save → mutate → restore → save round-trip is
/// byte-exact and the restored gain drives the DSP — all on the one
/// shared object, one `initialize`/`terminate` pair (the fixture's
/// strict contracts would fail `init`/teardown on a double call).
#[test]
fn single_component_save_mutate_restore_save_round_trips() {
    let mut plugin = prepared_single_plugin();

    // Initial save — default plain gain 1.0. Single-component
    // framing: [u32 comp_len][comp payload][u32 ctrl_len = 0] —
    // 4 + 20 + 4 = 28 bytes, the `ctrl_len = 0` arm.
    let initial = plugin.save_state().expect("save_state");
    assert_eq!(
        initial.len(),
        28,
        "one 20-byte component payload + two u32 lengths (ctrl_len = 0)"
    );
    assert_eq!(&initial[0..4], &20_u32.to_le_bytes());
    assert_eq!(&initial[4..8], b"RFST");
    assert_eq!(
        f64::from_le_bytes(initial[12..20].try_into().unwrap()),
        1.0,
        "component payload: plain gain"
    );
    assert_eq!(
        &initial[24..28],
        &0_u32.to_le_bytes(),
        "single-component save frames an empty controller payload"
    );

    // Deterministic: an immediate second save is byte-identical.
    let again = plugin.save_state().expect("save_state");
    assert_eq!(again, initial);

    // Mutate through the real ABI: normalized 0.375 → plain 0.75 at
    // offset 0 — the whole block runs at ×0.75 and the shared cell
    // adopts it.
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    let (_result, [left, _right]) = drive_block(&mut *plugin, &events);
    assert_eq!(left, vec![0.375_f32; FRAMES]);

    // The mutated save differs — same 28-byte single-component
    // frame, new plain gain in the component payload.
    let mutated = plugin.save_state().expect("save_state");
    assert_eq!(mutated.len(), 28);
    assert_eq!(
        f64::from_le_bytes(mutated[12..20].try_into().unwrap()),
        0.75
    );
    assert_eq!(&mutated[24..28], &0_u32.to_le_bytes());

    // Restore the initial blob — the single-component restore arm
    // (`component.setState` → `component.getState`, no controller
    // state verbs) runs and the re-saved blob is byte-identical to
    // the initial save.
    plugin.restore_state(&initial).expect("restore_state");
    let restored = plugin.save_state().expect("save_state");
    assert_eq!(restored, initial, "save → restore → save is exact");

    // Audio proves the restored gain drives the DSP again: ×1.0 —
    // `out == in` through the real ABI on the shared object.
    let (_result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// A malformed state blob on the single-component class is a
/// defined `PluginFailed`, never a silent accept — same contract as
/// the distributed class, same fault isolation: the instance
/// survives and `save_state` still works afterwards.
#[test]
fn single_component_restore_rejects_malformed_blob() {
    let mut plugin = prepared_single_plugin();
    plugin.save_state().expect("save_state");

    // Too short to be the framed format → foreign passthrough → the
    // component payload fails the codec → `PluginFailed`.
    assert_eq!(
        plugin.restore_state(&[0xAA, 0xBB]),
        Err(HostError::PluginFailed)
    );
    // A well-framed blob with a corrupted component magic →
    // `PluginFailed` at `component.setState`.
    let mut corrupt = plugin.save_state().expect("save_state");
    corrupt[4] ^= 0xFF;
    assert_eq!(plugin.restore_state(&corrupt), Err(HostError::PluginFailed));
    // A well-framed blob with one trailing byte → `unframe_state`
    // requires `ctrl_end == blob.len()` → `PluginFailed` before any
    // plugin verb runs.
    let mut trailing = plugin.save_state().expect("save_state");
    trailing.push(0x00);
    assert_eq!(
        plugin.restore_state(&trailing),
        Err(HostError::PluginFailed)
    );
    // A well-framed blob whose component payload carries a NaN
    // gain → the stale checksum fails integrity before the value
    // check → `kResultFalse` → `PluginFailed` at `component.setState`.
    let mut nan = plugin.save_state().expect("save_state");
    nan[12..20].copy_from_slice(&f64::NAN.to_le_bytes());
    assert_eq!(plugin.restore_state(&nan), Err(HostError::PluginFailed));
    // The instance is still alive and its state surface intact.
    plugin.save_state().expect("save_state still works");
}

/// The stateless classes keep the honest `Unsupported` on both state
/// verbs — `test_stateful`/`test_stateful_single` are the suite's
/// only state surfaces.
#[test]
fn stateless_classes_decline_state_verbs() {
    for mut plugin in [
        prepared_plugin(),
        prepared_fault_plugin(),
        prepared_editor_plugin(),
        prepared_instrument_plugin(),
    ] {
        assert_eq!(plugin.save_state(), Err(HostError::Unsupported));
        assert_eq!(
            plugin.restore_state(&[0xAA, 0xBB]),
            Err(HostError::Unsupported)
        );
    }
}

fn midi(offset: u32, kind: MidiKind) -> Event {
    Event::new(
        offset,
        EventKind::Midi(MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind,
        }),
    )
}

fn note_on(offset: u32) -> Event {
    midi(
        offset,
        MidiKind::NoteOn {
            key: NoteKey::new(69).unwrap(),
            velocity: u32::MAX,
        },
    )
}

fn note_off(offset: u32) -> Event {
    midi(
        offset,
        MidiKind::NoteOff {
            key: NoteKey::new(69).unwrap(),
            velocity: 0,
        },
    )
}

/// `test_instrument` note lifecycle through the production host
/// bridge in-process: a retriggered note-on makes every block's
/// frame 0 the triangle's `-1` extreme; switching to note-off lets
/// the fixed 2400-frame release decay to exact silence — the
/// neutral→VST3 `IEventList`→fixture event path end to end, no L1
/// pipeline.
#[test]
fn instrument_note_lifecycle_releases_to_silence() {
    let mut plugin = prepared_instrument_plugin();

    // Sounding phase: note-on retriggered at offset 0 every block.
    let on = [note_on(0)];
    for _ in 0..4 {
        let (_result, [left, _right]) = drive_block(&mut *plugin, &on);
        assert_eq!(left[0].to_bits(), (-1.0_f32).to_bits());
    }

    // Release phase: ONE note-off, then empty blocks — the fixture
    // law restarts the 2400-frame release on every NoteOff, so a
    // repeated note-off would hold the voice in release forever.
    // 2400 frames ≈ 38 blocks of 64 — drive 48 and require the tail
    // silent.
    let off = [note_off(0)];
    let empty: [Event; 0] = [];
    for block in 0..48 {
        let events: &[Event] = if block == 0 { &off } else { &empty };
        let (_result, [left, right]) = drive_block(&mut *plugin, events);
        if block >= 44 {
            assert!(
                left.iter().all(|&s| s == 0.0) && right.iter().all(|&s| s == 0.0),
                "block {block}: release tail must be exact silence"
            );
        }
    }
}
