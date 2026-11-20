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
use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
use ruviform_plugin_clap::host::ClapLibrary;
use ruviform_plugin_core::{
    ErasedPlugin, Event, EventKind, EventList, GuiSize, HostError, HostInput, HostOutput,
    ParamEvent, ParamId, ParamValue, ParentLease, PluginFactory, ProcessConfig, ProcessContext,
    ProcessResult,
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
/// `1` = `test_fault`, `2` = `test_stateful`, `3` = `test_instrument`,
/// `4` = `test_editor`).
fn prepared_plugin_at(index: usize) -> Box<dyn ErasedPlugin> {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 6, "the entry exposes six plugins");
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

/// `test_instrument`'s prepared instance — the instrument oracle's
/// entry point.
fn prepared_instrument_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(3)
}

/// `test_editor`'s prepared instance — the gui/pass-through oracle's
/// entry point.
fn prepared_editor_plugin() -> Box<dyn ErasedPlugin> {
    prepared_plugin_at(4)
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
/// `test_fault`, `test_stateful`, `test_instrument` and
/// `test_editor` plugin descriptors, and reports each fixture's
/// vendor/name/features.
#[test]
fn discovers_suite_plugins_through_production_host() {
    let library = load_library();
    let descriptors = library.descriptors();
    assert_eq!(
        descriptors.len(),
        6,
        "the entry exposes exactly six plugin descriptors"
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
    let instrument = descriptors[3];
    assert_eq!(instrument.name, "test_instrument");
    assert_eq!(instrument.vendor, "Ruviform Test");
    assert_eq!(instrument.version, "0.0.0");
    assert!(instrument.features.contains(&"instrument"));
    let editor = descriptors[4];
    assert_eq!(editor.name, "test_editor");
    assert_eq!(editor.vendor, "Ruviform Test");
    assert_eq!(editor.version, "0.0.0");
    assert!(editor.features.contains(&"audio-effect"));
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
    assert_eq!(factories.len(), 6);
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
/// range `[0.0, 8.0]`, default `0.0` (`none`).
#[test]
fn reports_fault_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 6);
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

/// `test_fault` `error` mode through the real ABI: normalized `0.5`
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
/// `TAIL_INFINITE` — the plugin reports `CLAP_PROCESS_TAIL`, which
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
        "bad_time must claim TAIL_INFINITE via CLAP_PROCESS_TAIL"
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

/// `test_stateful`'s parameter surface through the production path:
/// the `params` extension reports the same gain contract as
/// `test_gain` — plain range `[0.0, 2.0]`, default `1.0`.
#[test]
fn reports_stateful_param_in_plain_domain() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 6);
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
/// neutral→CLAP→fixture event path end to end, no L1 pipeline.
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

/// `test_editor`'s parameter surface through the production path:
/// the plugin declares no `clap.ext.params` extension, so the host
/// reports an empty parameter table — an honest absence, not a
/// fabricated empty-but-declared surface.
#[test]
fn editor_reports_no_params() {
    let library = load_library();
    let factories = library.factories();
    assert_eq!(factories.len(), 6);
    let plugin = factories[4].create();
    assert!(
        plugin.params().is_empty(),
        "test_editor declares no parameters"
    );
}

/// `test_editor` DSP law through the production host bridge: pure
/// pass-through — `out == in` bitwise — and a `ParamValue` event for
/// a parameter the fixture does not declare is dropped at the
/// event-translation boundary and cannot perturb the output.
#[test]
fn editor_is_parameterless_pass_through() {
    let mut plugin = prepared_editor_plugin();

    let (result, [left, right]) = drive_block(&mut *plugin, &[]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
    assert_eq!(right, vec![-0.25_f32; FRAMES]);

    // A foreign ParamValue event (no such parameter exists) is inert.
    let foreign = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.5).unwrap(),
        }),
    )];
    let (result, [left, right]) = drive_block(&mut *plugin, &foreign);
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
/// `clap.ext.gui` `set_parent`, whose `IsWindow` gate rejects the
/// non-window parent → the host rolls back via `destroy` →
/// `PluginFailed`; on non-Windows `EDITOR_GUI_API` is `None` →
/// `Unsupported` before `create` is even reached. Both are defined
/// rejections — a **real** HWND attach is the `scarecrow173/ruviform#631`
/// real-attach lane's evidence, not claimed here. The failed open
/// leaves the instance usable: `close_gui` honestly reports no open
/// GUI and audio still flows afterwards.
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

    // The rollback ran `destroy` — no GUI is open, so `close_gui`
    // reports the honest `Unsupported`, and the instance still
    // processes audio (defined rejection, not instance corruption).
    assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));
    let (result, [left, _right]) = drive_block(&mut *plugin, &[]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
}

/// `test_gain`/`test_fault`/`test_stateful`/`test_instrument` ship no
/// `clap.ext.gui` extension: `get_extension::<PluginGui>()` finds
/// nothing, so `open_gui` converges to the honest `Unsupported` on
/// every platform — `test_editor` is the suite's only GUI surface.
#[test]
fn non_editor_classes_decline_open_gui() {
    for mut plugin in [
        prepared_plugin(),
        prepared_fault_plugin(),
        prepared_stateful_plugin(),
        prepared_instrument_plugin(),
    ] {
        let Err(err) = plugin.open_gui(ParentLease::new(0, 0)) else {
            panic!("non-editor classes ship no gui extension — open_gui must fail");
        };
        assert_eq!(err, HostError::Unsupported);
    }
}

/// `test_editor` before any `open_gui`: the session verbs have no
/// live GUI to act on — every one converges to the defined
/// `Err(Unsupported)` through the production bridge (`host.rs`'s
/// `self.gui.is_none()` gates), on every OS. The refusal leaves the
/// instance healthy: audio still flows afterwards.
#[test]
fn editor_gui_verbs_decline_without_open_gui() {
    let mut plugin = prepared_editor_plugin();
    assert_eq!(
        plugin.set_gui_size(GuiSize {
            width: 800.0,
            height: 600.0,
        }),
        Err(HostError::Unsupported)
    );
    assert_eq!(plugin.set_gui_visibility(true), Err(HostError::Unsupported));
    assert_eq!(
        plugin.set_gui_visibility(false),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        plugin.set_gui_parent(ParentLease::new(0, 0)),
        Err(HostError::Unsupported)
    );
    assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));

    let (result, [left, _right]) = drive_block(&mut *plugin, &[]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
}

/// `test_gain`/`test_fault`/`test_stateful`/`test_instrument` ship no
/// `clap.ext.gui` extension and hold no GUI state — the same four
/// session verbs converge to the adapter/trait default
/// `Err(Unsupported)`, the same defined refusal `open_gui` already
/// reports for these classes.
#[test]
fn non_editor_classes_decline_gui_verbs() {
    for mut plugin in [
        prepared_plugin(),
        prepared_fault_plugin(),
        prepared_stateful_plugin(),
        prepared_instrument_plugin(),
    ] {
        assert_eq!(
            plugin.set_gui_size(GuiSize {
                width: 800.0,
                height: 600.0,
            }),
            Err(HostError::Unsupported)
        );
        assert_eq!(plugin.set_gui_visibility(true), Err(HostError::Unsupported));
        assert_eq!(
            plugin.set_gui_visibility(false),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_parent(ParentLease::new(0, 0)),
            Err(HostError::Unsupported)
        );
        assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));
    }
}

/// Windows-only real-HWND coverage of the `ruviform#754` CLAP editor
/// verb chain through the production host bridge — the CLAP
/// counterpart of the VST3 real lane. A hidden top-level parent is
/// created with `CreateWindowExW`, `open_gui` attaches the fixture's
/// real `WS_CHILD` HWND, and every verb is observed through Win32
/// directly: `GetWindow(GW_CHILD)`/`GetParent`/`IsWindow` for
/// attach/detach, `GetWindowRect` for the `set_gui_size` resize
/// (the fixture is resize-capable — `can_resize` → `true`), and
/// `GetWindowLongW(GWL_STYLE) & WS_VISIBLE` for hide/show (the
/// parent stays hidden, so `IsWindowVisible`'s ancestor-chain check
/// cannot observe the child's own visibility state). `set_gui_parent`
/// against a second parent hits the fixture's `gui already embedded`
/// rejection → `PluginFailed`. `#[ignore]` keeps it out of the
/// default lane — it needs an interactive window station; run with
/// `cargo test -p ruviform-plugin-test-clap --test host_path --
/// --ignored`. This is verb-level coverage only — real dock,
/// message pump, pixels, input/focus and re-open belong to the
/// `scarecrow173/ruviform#631` real-editor lane.
#[cfg(target_os = "windows")]
#[ignore = "requires an interactive Windows window station — real-HWND lane"]
#[expect(unsafe_code)]
#[test]
fn editor_gui_verbs_over_real_child_hwnd() {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GW_CHILD, GWL_STYLE, GetParent, GetWindow, GetWindowLongW,
        GetWindowRect, IsWindow, WS_OVERLAPPED, WS_VISIBLE,
    };

    // Hidden top-level parents (STATIC needs no class registration)
    // — they exist only to own the child; no message pump is needed.
    let class: Vec<u16> = "STATIC".encode_utf16().chain(std::iter::once(0)).collect();
    let make_parent = || unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            WS_OVERLAPPED,
            0,
            0,
            320,
            240,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    let parent = make_parent();
    let parent2 = make_parent();
    assert!(
        !parent.is_null() && !parent2.is_null(),
        "cannot create parents"
    );

    let mut plugin = prepared_editor_plugin();
    // `open_gui` → is_api_supported → create → set_parent → show.
    plugin
        .open_gui(ParentLease::new(0, parent as usize as u64))
        .expect("open_gui attaches the real child HWND");

    unsafe {
        let child: HWND = GetWindow(parent, GW_CHILD);
        assert!(!child.is_null(), "open_gui must create a child HWND");
        assert_eq!(GetParent(child), parent, "child must be parented");
        assert_ne!(IsWindow(child), 0, "child must be a live window");
        // `show` ran during open_gui — the child's own WS_VISIBLE bit
        // is set (the hidden parent makes IsWindowVisible unusable).
        let style = GetWindowLongW(child, GWL_STYLE).cast_unsigned();
        assert_ne!(style & WS_VISIBLE, 0, "child starts visible");

        // `set_gui_size` → can_resize gate → set_size → SetWindowPos:
        // the child HWND must really become 800x600.
        plugin
            .set_gui_size(GuiSize {
                width: 800.0,
                height: 600.0,
            })
            .expect("resize-capable fixture accepts set_gui_size");
        let mut rc = RECT::default();
        GetWindowRect(child, &raw mut rc);
        assert_eq!(rc.right - rc.left, 800, "child width after resize");
        assert_eq!(rc.bottom - rc.top, 600, "child height after resize");

        // `set_gui_visibility` → hide/show → the child's own
        // WS_VISIBLE bit flips while the HWND stays alive.
        plugin.set_gui_visibility(false).expect("hide succeeds");
        let style = GetWindowLongW(child, GWL_STYLE).cast_unsigned();
        assert_eq!(style & WS_VISIBLE, 0, "hide clears WS_VISIBLE");
        assert_ne!(IsWindow(child), 0, "hide keeps the child alive");
        plugin.set_gui_visibility(true).expect("show succeeds");
        let style = GetWindowLongW(child, GWL_STYLE).cast_unsigned();
        assert_ne!(style & WS_VISIBLE, 0, "show sets WS_VISIBLE");

        // `set_gui_parent` against a second parent hits the
        // fixture's `gui already embedded` rejection — the host
        // maps the plugin error to PluginFailed and the child stays
        // under the first parent.
        let Err(err) = plugin.set_gui_parent(ParentLease::new(0, parent2 as usize as u64)) else {
            panic!("re-embedding must be rejected by the fixture");
        };
        assert_eq!(err, HostError::PluginFailed);
        assert_eq!(GetParent(child), parent, "rejected reparent keeps parent");

        // `close_gui` → hide + destroy → the child HWND is gone.
        plugin.close_gui().expect("close_gui succeeds");
        assert_eq!(IsWindow(child), 0, "close_gui destroys the child");
        assert!(
            GetWindow(parent, GW_CHILD).is_null(),
            "parent has no child after close"
        );
        // A second close has no GUI to act on — defined Unsupported.
        assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));

        let _ = DestroyWindow(parent);
        let _ = DestroyWindow(parent2);
    }

    // The instance stays healthy after the GUI session ended.
    let (result, [left, _right]) = drive_block(&mut *plugin, &[]);
    assert!(matches!(
        result.status,
        ruviform_plugin_core::ProcessStatus::Continue
    ));
    assert_eq!(left, vec![0.5_f32; FRAMES]);
}
