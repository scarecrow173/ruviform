//! Production host-path verification (Issue #630 CLAP slice).
//!
//! These tests `dlopen` **this crate's own built cdylib** through
//! `ruviform_plugin_clap::host::ClapLibrary` — the same L0
//! in-process production host bridge the plugin host uses — and drive
//! the real CLAP ABI end to end: `clap_entry` → factory scan →
//! `clap_plugin_factory.create_plugin` → `plugin.init` →
//! `plugin.activate` → `start_processing` → `clap_process` →
//! `stop_processing`/`deactivate`/`destroy` on drop. No `Plugin` port
//! fake is involved anywhere in the path — the shared object is the
//! only artifact under test.
//!
//! The fixture binary resolves by the same `current_exe`-relative
//! `deps/` convention the adapter's own fixture tests use; a missing
//! binary is an assertion failure, never a silent skip.

use std::path::PathBuf;

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_clap::host::ClapLibrary;
use ruviform_plugin_core::{
    ErasedPlugin, Event, EventKind, EventList, HostError, HostInput, HostOutput, ParamEvent,
    ParamId, ParamValue, PluginFactory, ProcessConfig, ProcessContext, ProcessResult,
};

const FRAMES: usize = 64;
const FRAMES_U32: u32 = 64;

/// This crate's cdylib file name — `{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}`.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}`
/// — the test spec's fixture resolution.
fn fixture_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe is available");
    let deps = exe.parent().expect("test binary lives in deps/");
    deps.join(fixture_lib_name())
}

fn load_library() -> ClapLibrary {
    let path = fixture_path();
    assert!(
        path.is_file(),
        "test fixture cdylib missing: {}",
        path.display()
    );
    ClapLibrary::load(&path).expect("fixture dlopen succeeds")
}

/// The full legal handshake through the production host bridge:
/// `ClapLibrary::load` → `factories()` → `create()` → `init` →
/// `prepare` → `start_processing` — the same sequence the L1 runner
/// drives (`prepared_plugin()` precedent in the adapter's host tests).
/// `index` selects the suite plugin (`0` = `test_gain`,
/// `1` = `test_fault`, `2` = `test_stateful`).
fn prepared_plugin_at(index: usize) -> Box<dyn ErasedPlugin> {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 3, "the entry exposes three plugins");
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

/// `test_stateful`'s prepared instance — the state oracle's entry
/// point.
fn prepared_stateful_plugin() -> Box<dyn ErasedPlugin> {
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

/// Discovery through the production path: `ClapLibrary::load` scans
/// the real binary's `clap_entry`, finds the `test_gain`,
/// `test_fault` and `test_stateful` plugin descriptors, and reports
/// each fixture's vendor/name/features.
#[test]
fn discovers_suite_plugins_through_production_host() {
    let library = load_library();
    let descriptors = library.descriptors();
    assert_eq!(
        descriptors.len(),
        3,
        "the entry exposes exactly three plugin descriptors"
    );
    let gain = descriptors[0];
    assert_eq!(gain.name, "test_gain");
    assert_eq!(gain.vendor, "Ruviform Test");
    assert_eq!(gain.version, "0.0.0");
    assert!(gain.features.contains(&"audio-effect"));
    let fault = descriptors[1];
    assert_eq!(fault.name, "test_fault");
    assert_eq!(fault.vendor, "Ruviform Test");
    assert_eq!(fault.version, "0.0.0");
    assert!(fault.features.contains(&"audio-effect"));
    let stateful = descriptors[2];
    assert_eq!(stateful.name, "test_stateful");
    assert_eq!(stateful.vendor, "Ruviform Test");
    assert_eq!(stateful.version, "0.0.0");
    assert!(stateful.features.contains(&"audio-effect"));
}

/// Parameter surface through the production path: the host
/// instantiates a throwaway plugin at load time, reads the `params`
/// extension (`clap_plugin_params.get_info`), and maps the plain
/// domain directly — the reported plain range `[0.0, 2.0]` default
/// `1.0` proves the parameter contract end to end.
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

/// The fault parameter surface through the production path: the
/// `params` extension reports the stepped `Fault` parameter — plain
/// range `[0.0, 4.0]`, default `0.0` (`none`).
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
/// 64-frame block — the host bridge denormalizes it into a CLAP
/// `ParamValueEvent` carrying plain `1.5`. Frames 0–7 keep unity
/// gain; frames 8–63 use `1.5`. A second event-free block proves the
/// gain persists through the real ABI. Drop runs the host's
/// stop/deactivate/destroy teardown — the uninstantiate half of the
/// contract.
#[test]
fn processes_audio_with_sample_accurate_gain_events() {
    let mut plugin = prepared_plugin();

    // normalized 0.75 → host denormalize → plain 1.5, effective at
    // frame 8.
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
/// (`abort`/`exit`/`hang` are never driven in-process — they would
/// kill this test binary; they are L1-only faults.)
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
/// → plain `4.0` → the plugin returns `CLAP_PROCESS_ERROR`, which
/// the production bridge reports as `ProcessStatus::Error` — a
/// defined rejection, not a crash. The instance stays alive: a
/// follow-up `none` event (normalized `0.0`) restores pass-through.
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

/// `test_stateful`'s parameter surface through the production path:
/// the `params` extension reports the same gain contract as
/// `test_gain` — plain range `[0.0, 2.0]`, default `1.0`.
#[test]
fn reports_stateful_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 3);
    let plugin = factories[2].create();
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

/// Extracts the plain gain from the fixture's 16-byte state blob —
/// `[u32 LE "RFST"][u32 LE version=1][f64 LE]`; the f64 sits at
/// offset 8.
fn blob_gain(blob: &[u8]) -> f64 {
    assert_eq!(blob.len(), 16, "the CLAP state stream is one 16-byte blob");
    assert_eq!(&blob[0..4], b"RFST");
    assert_eq!(&blob[4..8], &1_u32.to_le_bytes());
    f64::from_le_bytes(blob[8..16].try_into().unwrap())
}

/// The `test_stateful` host-path round-trip: real `clap.ext.state`
/// `save`/`load` through the production `ClapLibrary` bridge —
/// save(initial, gain 1.0) → mutate to 0.75 via a `ParamValue` event
/// → save(mutated) → mutate to 1.75 → save → restore(initial) →
/// save == initial byte-exact → audio proves the restored gain drives
/// the DSP.
#[test]
fn stateful_save_mutate_restore_save_round_trips() {
    let mut plugin = prepared_stateful_plugin();

    // --- save(initial): default plain gain 1.0 ---
    let initial = plugin.save_state().expect("save_state");
    assert_eq!(blob_gain(&initial), 1.0);
    // Deterministic: an immediate second save is byte-identical.
    let again = plugin.save_state().expect("save_state");
    assert_eq!(again, initial, "save → save is byte-identical");

    // --- mutate → 0.75: normalized 0.375 → plain 0.75 @ frame 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    let (result, [left, right]) = drive_block(&mut *plugin, &events);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.375_f32; FRAMES]);
    assert_eq!(right, vec![-0.1875_f32; FRAMES]);
    let mutated = plugin.save_state().expect("save_state");
    assert_ne!(mutated, initial, "the mutation must change the blob");
    assert_eq!(blob_gain(&mutated), 0.75);

    // --- mutate → 1.75: normalized 0.875 → plain 1.75 @ frame 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.875).unwrap(),
        }),
    )];
    let (_result, [left, right]) = drive_block(&mut *plugin, &events);
    assert_eq!(left, vec![0.875_f32; FRAMES]);
    assert_eq!(right, vec![-0.4375_f32; FRAMES]);
    let third = plugin.save_state().expect("save_state");
    assert_eq!(blob_gain(&third), 1.75);
    assert_ne!(third, mutated);

    // --- restore(initial) → save == initial, byte-exact ---
    plugin.restore_state(&initial).expect("restore_state");
    let restored = plugin.save_state().expect("save_state");
    assert_eq!(
        restored, initial,
        "save → mutate → restore → save is byte-exact"
    );

    // Audio proves the restored gain drives the DSP: ×1.0 — `out`
    // equals the constant input on both channels.
    let (_result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);
}

/// Malformed state blobs are defined `PluginFailed` rejections —
/// never silent accepts — and the instance survives: `save_state`
/// keeps working afterwards (the defined-rejection half of the
/// state contract, in-process).
#[test]
fn stateful_restore_rejects_malformed_blob() {
    let mut plugin = prepared_stateful_plugin();

    let valid = plugin.save_state().expect("save_state");
    assert_eq!(blob_gain(&valid), 1.0);

    // Truncated blob → `PluginFailed`.
    assert_eq!(
        plugin.restore_state(&valid[..8]),
        Err(HostError::PluginFailed)
    );
    // Trailing bytes → `PluginFailed`.
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        plugin.restore_state(&trailing),
        Err(HostError::PluginFailed)
    );
    // Corrupted magic → `PluginFailed`.
    let mut corrupt = valid.clone();
    corrupt[0] ^= 0xFF;
    assert_eq!(plugin.restore_state(&corrupt), Err(HostError::PluginFailed));
    // Non-finite payload → `PluginFailed`.
    let mut nan = valid.clone();
    nan[8..16].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    assert_eq!(plugin.restore_state(&nan), Err(HostError::PluginFailed));

    // The rejected writes left the instance alive and its state
    // surface intact — `save_state` still returns the valid blob.
    let still = plugin.save_state().expect("save_state");
    assert_eq!(still, valid, "rejected restores must not corrupt state");
}

/// The stateless fixtures keep their honest capability decline: no
/// `clap.ext.state` extension → `save_state`/`restore_state` report
/// `HostError::Unsupported`, never a fabricated blob.
#[test]
fn stateless_classes_decline_state_verbs() {
    let mut gain = prepared_plugin();
    assert_eq!(gain.save_state(), Err(HostError::Unsupported));
    assert_eq!(gain.restore_state(&[1, 2, 3]), Err(HostError::Unsupported));

    let mut fault = prepared_fault_plugin();
    assert_eq!(fault.save_state(), Err(HostError::Unsupported));
    assert_eq!(fault.restore_state(&[1, 2, 3]), Err(HostError::Unsupported));
}
