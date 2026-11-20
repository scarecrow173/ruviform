//! Stateful-test-plugin L1 end-to-end tests — this crate's own
//! `test_stateful` and `test_stateful_single` VST3 cdylib classes
//! served out-of-process by
//! `ruviform-plugin-test-vst3-l1-runner` (Issue #630 `test_stateful`
//! and single-component slices). Structural mirror of
//! `l1_real_test_plugin_e2e.rs`.
//!
//! Every test is `#[ignore]`-gated: they require the fixture cdylib
//! and the `l1-runner`-feature runner binary, which only
//! `mise run test-l1-real-plugin` builds deterministically before
//! selecting `--ignored`. The file-level `#![cfg(feature = "l1-runner")]`
//! keeps the file empty — and `env!("CARGO_BIN_EXE_*")` undefined —
//! without the feature, so `cargo test` default runs never see these
//! tests. Missing artifacts are `assert!` failures, never silent
//! skips.
//!
//! ## Ordering contract
//!
//! `SaveState`/`RestoreState` are synchronous wire commands — their
//! `Ok` reply proves the child already applied the verb. Audio blocks
//! are asynchronous (the L1 data plane's lookahead pipeline), so a
//! state verb issued right after `process` could overtake the block
//! carrying the mutation event. The tests therefore gate every state
//! verb on *delivered audio*: the fixture's three gains (`1.0`,
//! `0.75`, `1.75`) produce pairwise-distinct output on the constant
//! `0.5 / -0.25` input, so observing a given gain's output proves the
//! child processed every earlier block — including the event block —
//! before the verb is sent.

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::{
    Event, EventKind, EventList, HostError, HostInput, HostOutput, ParamEvent, ParamId, ParamValue,
    PluginDescriptor, ProcessConfig, ProcessResult,
};
use ruviform_plugin_host::{HostCommand, HostReply, OutOfProcessSpec, PluginHandle, PluginHost};

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The descriptor id the child synthesizes for `test_stateful` — the
/// component CID's raw 16 bytes as lowercase hex (`tuid_hex`),
/// derived at test time from `vst3::uid(...)` with the same constants
/// `stateful.rs` builds `TestStatefulComponent::CID` from (the `uid`
/// byte order is platform-dependent; deriving it here verifies the
/// mapping rather than restating a literal). `TestStatefulComponent`
/// itself is `pub(crate)` and unreachable from `tests/`.
fn expected_id() -> &'static str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let cid = vst3::uid(0x8D2F_6B94, 0xA1C5_3E07, 0xF4B8_9D26, 0xC7E3_5A81);
    let mut out = String::with_capacity(32);
    for &byte in &cid {
        let byte = byte.cast_unsigned();
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    String::leak(out)
}

/// The descriptor the child synthesizes from the class scan — same
/// field-match contract as the gain fixture's `expected_descriptor`.
fn expected_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_id(),
        name: "test_stateful",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

/// The descriptor id the child synthesizes for
/// `test_stateful_single` — the component CID's raw 16 bytes as
/// lowercase hex, derived at test time from `vst3::uid(...)` with the
/// same constants `single.rs` builds `TestStatefulSingle::CID` from
/// (see `expected_id` for the derivation contract).
fn expected_single_id() -> &'static str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let cid = vst3::uid(0x7A3E_5C91, 0x2D4F_8B36, 0xE9A1_6C42, 0xB5D8_3F07);
    let mut out = String::with_capacity(32);
    for &byte in &cid {
        let byte = byte.cast_unsigned();
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    String::leak(out)
}

/// The descriptor the child synthesizes for the single-component
/// class — same field-match contract, one `Audio Module Class` row
/// with no controller row.
fn expected_single_descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: expected_single_id(),
        name: "test_stateful_single",
        vendor: "Ruviform Test",
        version: "0.0.0",
        features: &["Fx"],
    }
}

/// Block shape — 512 max frames, so the defined L1 latency is
/// `lookahead_blocks` (2) × 512 = 1024 frames (the
/// `l1_real_test_plugin_e2e` precedent).
const FRAMES: usize = 512;
const FRAMES_U32: u32 = 512;

/// How many follow-up blocks `wait_for_gain_output` submits before
/// giving up — 24 × ~5 ms pacing is generous headroom for a loaded
/// CI host (the existing suite's delivery bounds are looser still).
const MAX_FOLLOW_UPS: u32 = 24;

/// Cap on mutation-event resends — each resend is one call carrying
/// the `ParamValue` event; if none lands within the window the
/// ordering gate fails honestly instead of looping (the same bound
/// the fault suite's `ARM_RESEND_CALLS` uses).
const MUTATION_RESEND_CALLS: u32 = 8;

/// The runner executable — `CARGO_BIN_EXE_*` names it directly
/// (defined only under `--features l1-runner`, which this file's
/// `#![cfg]` guarantees).
fn runner_program() -> OsString {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-vst3-l1-runner"));
    assert!(
        path.is_file(),
        "L1 runner missing at {} — `mise run test-l1-real-plugin` builds it",
        path.display()
    );
    path.into_os_string()
}

/// The fixture cdylib the runner will dlopen — a sibling of the
/// runner executable under `target/<profile>/`.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-vst3-l1-runner"))
        .parent()
        .expect("the runner lives under target/<profile>/")
        .join(fixture_lib_name())
}

/// Every test that spawns the real child asserts the fixture exists
/// first — a missing artifact is a failure, not a skip.
fn assert_fixture_present() {
    let path = fixture_path();
    assert!(
        path.is_file(),
        "fixture cdylib missing: {} — `mise run test-l1-real-plugin` builds it via \
         `cargo build --features l1-runner`",
        path.display()
    );
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap()
}

/// Bit-exact sample comparison — the fixture gains (`0.75`, `1.75`,
/// `1.0`) are exactly representable in `f32` and the constant input
/// (`0.5`, `-0.25`) scales exactly, so defined values compare
/// bit-for-bit.
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// Per-call counter deltas recorded around one `process` — the
/// oracle's provenance record for the delivered block.
/// `passthrough_submits` increments when the leg substitutes
/// hold-last/silence output instead of a matched child block;
/// `dropped_submits` increments when this call's input block —
/// audio plus its queued events — never reached the child at all.
/// A dropped submit also substitutes output, so `passthrough > 0`
/// marks every non-matched block and `dropped` distinguishes event
/// loss. The delivered-audio gate must classify per call: a
/// substituted block replays `last_output`, which can satisfy the
/// expected-gain equality without the child having rendered
/// anything new.
#[derive(Clone, Copy, Default)]
struct LegDeltas {
    /// `passthrough_submits` delta: `> 0` ⇒ substituted output.
    passthrough: u64,
    /// `dropped_submits` delta: `> 0` ⇒ this call's submit dropped.
    dropped: u64,
}

/// One stereo block through `PluginHost::process` — the fixture
/// declares a single stereo bus pair. Returns the `ProcessResult`
/// plus the leg's per-call counter deltas so callers can classify
/// the delivered block as matched (`passthrough == 0`) or
/// substituted (`passthrough > 0`).
fn host_block(
    host: &mut PluginHost,
    handle: PluginHandle,
    input: &[&[f32]; 2],
    out_left: &mut [f32],
    out_right: &mut [f32],
    events: EventList<'_>,
) -> (Result<ProcessResult, HostError>, LegDeltas) {
    let counters = |h: &PluginHost| {
        h.audio_counters(handle)
            .map_or((0, 0), |c| (c.passthrough_submits, c.dropped_submits))
    };
    let before = counters(host);
    let input_view = PlanarBufferView::new(&input[..]).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut *out_left, &mut *out_right];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    let result = host.process(handle, &inputs, &mut outputs, events, None);
    let after = counters(host);
    (
        result,
        LegDeltas {
            passthrough: after.0 - before.0,
            dropped: after.1 - before.1,
        },
    )
}

/// Submits one constant-input block (`0.5 / -0.25`) carrying
/// `events`, returning the delivered output channels plus the
/// leg's counter deltas.
fn submit_block(
    host: &mut PluginHost,
    handle: PluginHandle,
    events: EventList<'_>,
) -> ([f32; FRAMES], [f32; FRAMES], LegDeltas) {
    let left = [0.5_f32; FRAMES];
    let right = [-0.25_f32; FRAMES];
    let input = [&left[..], &right[..]];
    let mut out_left = [0.0_f32; FRAMES];
    let mut out_right = [0.0_f32; FRAMES];
    thread::sleep(Duration::from_millis(5));
    let (result, deltas) = host_block(host, handle, &input, &mut out_left, &mut out_right, events);
    assert_eq!(result, Ok(ProcessResult::CONTINUE));
    (out_left, out_right, deltas)
}

/// Submits blocks carrying `events` until one lands — a `dropped`
/// delta of 0 means this call's block *and* its queued events
/// reached the child-side queue. A single-shot mutation submit is
/// not a reliable mutation: a busy slot or full channel drops the
/// submit wholesale and the gain never changes, which the
/// delivered-audio gate then cannot observe — a flaky failure with
/// no oracle defect elsewhere. Re-sends are idempotent: the
/// fixture's gain setter is a plain overwrite of the last
/// `ParamValue` (`stateful.rs`), so a later copy carries the same
/// intent. Returns whether a submit landed within
/// `MUTATION_RESEND_CALLS`.
fn submit_mutation_until_landed(
    host: &mut PluginHost,
    handle: PluginHandle,
    events: EventList<'_>,
) -> bool {
    for _ in 0..MUTATION_RESEND_CALLS {
        let (_out_left, _out_right, deltas) = submit_block(host, handle, events);
        if deltas.dropped == 0 {
            return true;
        }
    }
    false
}

/// Drive one instance through `Processing`.
fn drive_to_processing(host: &mut PluginHost, handle: PluginHandle) {
    host.execute(HostCommand::Initialize { handle }).unwrap();
    host.execute(HostCommand::Activate {
        handle,
        config: config(),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
}

/// `SaveState` → the framed state blob (`HostReply::State`).
fn save_state(host: &mut PluginHost, handle: PluginHandle) -> Vec<u8> {
    match host.execute(HostCommand::SaveState { handle }).unwrap() {
        HostReply::State { bytes } => bytes,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// Submits event-free constant-input blocks until a *matched*
/// delivered result shows `expected_left`/`expected_right` — the
/// gain-scaled golden values for the constant `0.5 / -0.25` input —
/// or `MAX_FOLLOW_UPS` elapse. Returns whether the expected output
/// was observed.
///
/// This is the ordering gate described in the module docs: a matching
/// output can only come from a *new* result the child produced after
/// the gain reached `expected`, which proves every earlier block —
/// including the mutation event block — was processed first. Only
/// matched calls count: a substituted block replays an older
/// `last_output` — defined, but not this call's child rendering — so
/// it can satisfy the equality while proving nothing.
fn wait_for_gain_output(
    host: &mut PluginHost,
    handle: PluginHandle,
    expected_left: f32,
    expected_right: f32,
) -> bool {
    let mut substituted = 0_u32;
    for _ in 0..MAX_FOLLOW_UPS {
        let (out_left, out_right, deltas) =
            submit_block(host, handle, EventList::empty(FRAMES_U32));
        if deltas.passthrough > 0 {
            substituted += 1;
            continue;
        }
        if out_left.iter().all(|&s| sample_eq(s, expected_left))
            && out_right.iter().all(|&s| sample_eq(s, expected_right))
        {
            return true;
        }
    }
    eprintln!(
        "wait_for_gain_output({expected_left}, {expected_right}): \
         {substituted}/{MAX_FOLLOW_UPS} calls substituted, none matched the expected gain"
    );
    false
}

/// Extracts the component payload's plain gain from the framed blob —
/// `[u32 comp_len][comp][u32 ctrl_len][ctrl]` where `comp` is the
/// fixture's 20-byte v2 `["RFST"][ver=2][f64 LE][cksum]` payload
/// (f64 at comp offset 8, i.e. blob offset 12).
fn component_gain(blob: &[u8]) -> f64 {
    assert_eq!(blob.len(), 48, "two 20-byte payloads + two u32 lengths");
    assert_eq!(&blob[0..4], &20_u32.to_le_bytes());
    assert_eq!(&blob[4..8], b"RFST");
    assert_eq!(&blob[24..28], &20_u32.to_le_bytes());
    assert_eq!(&blob[28..32], b"RFST");
    f64::from_le_bytes(blob[12..20].try_into().unwrap())
}

/// Extracts the component payload's plain gain from a
/// **single-component** framed blob — `[u32 comp_len][comp][u32 0]`
/// where `comp` is the same 20-byte v2 `["RFST"][ver=2][f64 LE][cksum]`
/// payload and the controller payload is *empty* (`ctrl_len = 0`: the
/// `single_component` save arm only a real single-component binary
/// produces — 28 bytes total).
fn single_component_gain(blob: &[u8]) -> f64 {
    assert_eq!(
        blob.len(),
        28,
        "one 20-byte component payload + two u32 lengths (ctrl_len = 0)"
    );
    assert_eq!(&blob[0..4], &20_u32.to_le_bytes());
    assert_eq!(&blob[4..8], b"RFST");
    assert_eq!(
        &blob[24..28],
        &0_u32.to_le_bytes(),
        "single-component save frames an empty controller payload"
    );
    f64::from_le_bytes(blob[12..20].try_into().unwrap())
}

/// The `test_stateful` L1 round-trip: real `IBStream` state verbs
/// through the production host bridge, out-of-process.
///
/// save(initial, gain 1.0) → mutate to 0.75 via a `ParamValue` event →
/// save(mutated) → mutate to 1.75 → save → restore(initial) →
/// save == initial byte-exact → audio proves the restored gain drives
/// the DSP. Every state verb is gated on delivered audio so the
/// child's data plane has provably applied each mutation first.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_save_mutate_restore_save() {
    assert_fixture_present();

    let expected = expected_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(ruviform_plugin_core::LifecycleState::Processing)
    );

    // --- save(initial): default plain gain 1.0, normalized 0.5 ---
    let initial = save_state(&mut host, handle);
    assert_eq!(component_gain(&initial), 1.0);
    // Deterministic: an immediate second save is byte-identical.
    let again = save_state(&mut host, handle);
    assert_eq!(again, initial, "save → save is byte-identical");

    // --- mutate → 0.75: normalized 0.375 → plain 0.75 @ offset 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    // Resend until a submit lands — a dropped block would lose the
    // event wholesale; re-sends overwrite the same gain, idempotent.
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×0.75 mutation event never landed"
    );
    // Gate: delivered ×0.75 audio proves the child processed the
    // event block before the next state verb.
    assert!(
        wait_for_gain_output(&mut host, handle, 0.375, -0.1875),
        "delivered audio never showed the ×0.75 mutation"
    );
    let mutated = save_state(&mut host, handle);
    assert_ne!(mutated, initial, "the mutation must change the blob");
    assert_eq!(component_gain(&mutated), 0.75);

    // --- mutate → 1.75: normalized 0.875 → plain 1.75 @ offset 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.875).unwrap(),
        }),
    )];
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×1.75 mutation event never landed"
    );
    assert!(
        wait_for_gain_output(&mut host, handle, 0.875, -0.4375),
        "delivered audio never showed the ×1.75 mutation"
    );
    let third = save_state(&mut host, handle);
    assert_eq!(component_gain(&third), 1.75);
    assert_ne!(third, mutated);

    // --- restore(initial) → save == initial, byte-exact ---
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial.clone(),
    })
    .unwrap();
    let restored = save_state(&mut host, handle);
    assert_eq!(
        restored, initial,
        "save → mutate → restore → save is byte-exact"
    );

    // Audio proves the restored gain drives the DSP: ×1.0 — `out`
    // equals the constant input on both channels.
    assert!(
        wait_for_gain_output(&mut host, handle, 0.5, -0.25),
        "delivered audio never showed the restored ×1.0 gain"
    );

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// Malformed state blobs are defined `PluginFailed` rejections —
/// never silent accepts — and the instance survives: `SaveState`
/// keeps working afterwards (the defined-rejection half of the
/// state contract, out-of-process).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_rejects_malformed_state() {
    assert_fixture_present();

    let expected = expected_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);

    let valid = save_state(&mut host, handle);
    assert_eq!(component_gain(&valid), 1.0);

    // Foreign short blob (< the 8-byte frame header) → passthrough →
    // the component payload fails the codec → `PluginFailed`.
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: vec![0xAA, 0xBB],
        }),
        Err(HostError::PluginFailed)
    );

    // A well-framed blob with a corrupted component magic →
    // `PluginFailed` at `component.setState`.
    let mut corrupt = valid.clone();
    corrupt[4] ^= 0xFF;
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: corrupt,
        }),
        Err(HostError::PluginFailed)
    );

    // A checksum-corrupt component payload → `PluginFailed` — the
    // codec's integrity check.
    let mut corrupt_sum = valid.clone();
    corrupt_sum[20] ^= 0xFF;
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: corrupt_sum,
        }),
        Err(HostError::PluginFailed)
    );

    // A version this build does not know → `PluginFailed` — the
    // codec's unsupported-version check.
    let mut future = valid.clone();
    future[8..12].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: future,
        }),
        Err(HostError::PluginFailed)
    );

    // The rejected writes left the instance alive and its state
    // surface intact — `SaveState` still returns the valid blob.
    let still = save_state(&mut host, handle);
    assert_eq!(still, valid, "rejected restores must not corrupt state");

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// Migration and forward-compat degrade through the L1 wire: a
/// framed blob whose payloads are the legacy 16-byte v1 layout
/// restores (`Migrated`) and the next `SaveState` re-serializes the
/// 48-byte v2 frame; a v2 component payload with an unknown
/// extension tail restores (`Degraded`) and re-saves canonical. The
/// restored gain drives the DSP afterwards — the same state contract
/// the in-process host-path test asserts, out-of-process.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_migrates_and_degrades_state() {
    assert_fixture_present();

    let expected = expected_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);

    let canonical = save_state(&mut host, handle);
    assert_eq!(component_gain(&canonical), 1.0);

    // Craft a v1 frame: [u32 16][v1 comp][u32 16][v1 ctrl] — each
    // payload is the v2 core (magic+version+f64) with the version
    // rewritten to 1.
    let mut comp_v1 = canonical[4..20].to_vec();
    comp_v1[4..8].copy_from_slice(&1_u32.to_le_bytes());
    let mut ctrl_v1 = canonical[28..44].to_vec();
    ctrl_v1[4..8].copy_from_slice(&1_u32.to_le_bytes());
    let mut v1_frame = Vec::new();
    v1_frame.extend_from_slice(&16_u32.to_le_bytes());
    v1_frame.extend_from_slice(&comp_v1);
    v1_frame.extend_from_slice(&16_u32.to_le_bytes());
    v1_frame.extend_from_slice(&ctrl_v1);

    host.execute(HostCommand::RestoreState {
        handle,
        bytes: v1_frame,
    })
    .unwrap();
    // The re-save re-serializes canonical v2 — migration grew the
    // checksum field back.
    let migrated = save_state(&mut host, handle);
    assert_eq!(migrated, canonical, "v1 in → canonical v2 out");

    // A v2 component payload + 4 unknown extension bytes (comp_len
    // 24, valid checksum over the core) → degraded accept.
    let mut degraded = Vec::new();
    degraded.extend_from_slice(&24_u32.to_le_bytes());
    degraded.extend_from_slice(&canonical[4..24]);
    degraded.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    degraded.extend_from_slice(&canonical[24..]);
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: degraded,
    })
    .unwrap();
    let resaved = save_state(&mut host, handle);
    assert_eq!(resaved, canonical, "degraded in → canonical v2 out");

    // Audio proves the restored ×1.0 gain still drives the DSP.
    assert!(
        wait_for_gain_output(&mut host, handle, 0.5, -0.25),
        "delivered audio never showed the restored ×1.0 gain"
    );

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// The `test_stateful_single` L1 round-trip — the single-component
/// class through the production host bridge's `single_component`
/// arm, out-of-process. `getControllerClassId` reports the
/// component's own CID → the bridge casts the shared object to
/// `IEditController`, initializes it once, skips `IConnectionPoint`
/// pairing, and frames saves with `ctrl_len = 0` (24 bytes).
///
/// Same wire-oracle shape as the distributed `test_stateful` case:
/// save(initial, gain 1.0) → mutate to 0.75 via a `ParamValue`
/// event → save(mutated) → mutate to 1.75 → save →
/// restore(initial) → save == initial byte-exact → audio proves the
/// restored gain drives the DSP — every state verb gated on
/// delivered audio.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_single_test_plugin_l1_save_mutate_restore_save() {
    assert_fixture_present();

    let expected = expected_single_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(ruviform_plugin_core::LifecycleState::Processing)
    );

    // --- save(initial): default plain gain 1.0 — the 24-byte
    // single-component frame (`ctrl_len = 0`) ---
    let initial = save_state(&mut host, handle);
    assert_eq!(single_component_gain(&initial), 1.0);
    // Deterministic: an immediate second save is byte-identical.
    let again = save_state(&mut host, handle);
    assert_eq!(again, initial, "save → save is byte-identical");

    // --- mutate → 0.75: normalized 0.375 → plain 0.75 @ offset 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    // Resend until a submit lands — a dropped block would lose the
    // event wholesale; re-sends overwrite the same gain, idempotent.
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×0.75 mutation event never landed"
    );
    // Gate: delivered ×0.75 audio proves the child processed the
    // event block before the next state verb.
    assert!(
        wait_for_gain_output(&mut host, handle, 0.375, -0.1875),
        "delivered audio never showed the ×0.75 mutation"
    );
    let mutated = save_state(&mut host, handle);
    assert_ne!(mutated, initial, "the mutation must change the blob");
    assert_eq!(single_component_gain(&mutated), 0.75);

    // --- mutate → 1.75: normalized 0.875 → plain 1.75 @ offset 0 ---
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.875).unwrap(),
        }),
    )];
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×1.75 mutation event never landed"
    );
    assert!(
        wait_for_gain_output(&mut host, handle, 0.875, -0.4375),
        "delivered audio never showed the ×1.75 mutation"
    );
    let third = save_state(&mut host, handle);
    assert_eq!(single_component_gain(&third), 1.75);
    assert_ne!(third, mutated);

    // --- restore(initial) → save == initial, byte-exact ---
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial.clone(),
    })
    .unwrap();
    let restored = save_state(&mut host, handle);
    assert_eq!(
        restored, initial,
        "save → mutate → restore → save is byte-exact"
    );

    // Audio proves the restored gain drives the DSP: ×1.0 — `out`
    // equals the constant input on both channels.
    assert!(
        wait_for_gain_output(&mut host, handle, 0.5, -0.25),
        "delivered audio never showed the restored ×1.0 gain"
    );

    // Clean teardown — the strict fixture `terminate` runs once
    // here; a bridge double-terminate on the shared object would
    // surface as a `PluginFailed` reply instead of `Ok`.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// Malformed state blobs on the single-component class are defined
/// `PluginFailed` rejections — never silent accepts — and the
/// instance survives: `SaveState` keeps working afterwards (the
/// defined-rejection half of the single-component state contract,
/// out-of-process).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_single_test_plugin_l1_rejects_malformed_state() {
    assert_fixture_present();

    let expected = expected_single_descriptor();
    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(expected, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, expected.id);
    drive_to_processing(&mut host, handle);

    let valid = save_state(&mut host, handle);
    assert_eq!(single_component_gain(&valid), 1.0);

    // Foreign short blob (< the 8-byte frame header) → passthrough →
    // the component payload fails the codec → `PluginFailed`.
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: vec![0xAA, 0xBB],
        }),
        Err(HostError::PluginFailed)
    );

    // A well-framed blob with a corrupted component magic →
    // `PluginFailed` at `component.setState`.
    let mut corrupt = valid.clone();
    corrupt[4] ^= 0xFF;
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: corrupt,
        }),
        Err(HostError::PluginFailed)
    );

    // The rejected writes left the instance alive and its state
    // surface intact — `SaveState` still returns the valid blob.
    let still = save_state(&mut host, handle);
    assert_eq!(still, valid, "rejected restores must not corrupt state");

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}
