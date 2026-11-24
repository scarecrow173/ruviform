//! Stateful-test-plugin L1 end-to-end tests — this crate's own
//! `test_stateful` CLAP cdylib served out-of-process by
//! `ruviform-plugin-test-clap-l1-runner` (Issue #630 `test_stateful`
//! slice). Structural mirror of `l1_real_test_plugin_e2e.rs`.
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
//!
//! ## Blob shape
//!
//! CLAP `clap.ext.state` is one plugin-defined byte stream with no
//! adapter framing: the fixture writes exactly
//! `[u32 LE "RFST"][u32 LE version=2][f64 LE plain gain][u32 LE
//! checksum]` — 20 bytes (the legacy 16-byte v1 layout still
//! restores — migration).

#![cfg(feature = "l1-runner")]

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::{
    DirtyNotice, Event, EventKind, EventList, FailureCause, HostError, HostInput, HostOutput,
    LifecycleState, ParamEvent, ParamId, ParamNotice, ParamValue, PluginDescriptor, ProcessConfig,
    ProcessResult, RescanAspects,
};
use ruviform_plugin_host::{
    DiagnosticQuery, HostCommand, HostReply, OutOfProcessSpec, PluginHandle, PluginHost,
    ReportDrain, SpawnPhase,
};

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}`. Local to the
/// test crate: the adapter's `fixture_lib_name()` is compile-time
/// fixed to the adapter fixture.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The descriptor the child synthesizes from a real
/// `clap_plugin_factory` scan — the `OutOfProcessSpec` oracle must
/// field-match it (`id`/`name`/`vendor`/`version`/`features`,
/// order-preserving — `Descriptor::matches`, wire.rs). The parent
/// pins the fixture's known values as a const while the child
/// computes them from the actual binary — the handshake is two
/// independent sources agreeing, not the same scan on both sides
/// (`stateful.rs` `TestStatefulClapPlugin::get_descriptor`;
/// `version` is the fixture's declared `"0.0.0"`).
const EXPECTED: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform-test.clap-test-stateful",
    name: "test_stateful",
    vendor: "Ruviform Test",
    version: "0.0.0",
    features: &["audio-effect", "stereo"],
};

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
/// `#![cfg]` guarantees). A missing binary is an `assert!` failure,
/// never a silent skip.
fn runner_program() -> OsString {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-clap-l1-runner"));
    assert!(
        path.is_file(),
        "L1 runner missing at {} — `mise run test-l1-real-plugin` builds it",
        path.display()
    );
    path.into_os_string()
}

/// The fixture cdylib the runner will dlopen — resolved the way the
/// runner resolves it with `RUVIFORM_L1_REAL_PLUGIN_DIR` unset: a
/// sibling of the runner executable (cargo uplifts both the bin and
/// the cdylib into `target/<profile>/`).
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ruviform-plugin-test-clap-l1-runner"))
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

/// Bound on `Instantiate` retries after a pre-mint
/// `SpawnFailed{Timeout}` — the rf#841 C1 signature. The failure is
/// an environment deadline event (a stalled spawn-path wait), not a
/// contract verdict: `SpawnFailed` is pre-mint only, so a fresh
/// attempt is the defined recovery — the same "pre-contract
/// environment retry" shape as `ARM_RESEND_CALLS` (rf#766). The
/// bound keeps a persistent stall an honest failure instead of an
/// absorbed flake (`docs/discovery/issue-841-spawn-deadline-flake.md`
/// Decision C — assertion/gate unchanged).
const SPAWN_TIMEOUT_RETRIES: u32 = 3;

/// `Instantiate` with a bounded retry on the rf#841 C1 signature —
/// `SpawnFailed{cause: Timeout}` proven pre-mint by its retained
/// diagnostics record. The signature check is explicit per Decision
/// C: the error's `cause` is `Timeout` AND the record is `Found`
/// with `cause == Some(Timeout)` AND `phase` is one of the
/// `spawn_timeout`-bounded waits' freeze phases — `{PreSpawn,
/// Spawn, DrainInit, Handshake, Live}`, every variant the enum
/// currently declares. `Live` is eligible here — and only inside
/// this signature — because `SpawnFailed` is definitionally
/// pre-mint (the variant exists only on the spawn/create paths, so
/// a post-mint command timeout can never produce it): the phase
/// tracks *child* lifecycle and goes `Live` the instant `Hello`
/// passes, so the `Scan`/`CreateInstance` waits — themselves
/// `spawn_timeout`-bounded mint-window waits — freeze `Live`
/// records on both a fresh child and a pooled re-mint. A
/// non-`Timeout` cause or an `Evicted`/`NotFound` query result is
/// signature-missed and fails at once — never retried. Every
/// `SpawnFailed` attempt is traced to the real stderr — bypassing
/// the harness's per-test capture so the trace lands in lane logs
/// even when a later attempt recovers the test — carrying the
/// attempt index plus the record's `diagnostics_id`/`phase`/`cause`
/// (the deadline-phase attribution the `SpawnFailed` payload alone
/// cannot show — rf#841 Residual). On the terminal failure the
/// retained record and the child's captured stderr are printed in
/// full — the Issue #207 diagnosis aid. Bound exhaustion is an
/// honest failure — the flake is never absorbed silently.
fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    for attempt in 0..=SPAWN_TIMEOUT_RETRIES {
        match host.execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        }) {
            Ok(HostReply::Instantiated { handle }) => return handle,
            Ok(other) => panic!("unexpected reply: {other:?}"),
            Err(HostError::SpawnFailed {
                cause,
                diagnostics_id,
            }) => {
                let query = host.diagnostics_query(diagnostics_id);
                let (phase, record_cause) = match &query {
                    DiagnosticQuery::Found(record) => (Some(record.phase), record.cause),
                    _ => (None, None),
                };
                let _ = writeln!(
                    std::io::stderr(),
                    "instantiate SpawnFailed trace: plugin={plugin_id} \
                     attempt={attempt} cause={cause:?} \
                     diagnostics_id={diagnostics_id} phase={phase:?} \
                     record_cause={record_cause:?}"
                );
                let signature = cause == FailureCause::Timeout
                    && matches!(
                        &query,
                        DiagnosticQuery::Found(record)
                            if matches!(
                                record.phase,
                                SpawnPhase::PreSpawn
                                    | SpawnPhase::Spawn
                                    | SpawnPhase::DrainInit
                                    | SpawnPhase::Handshake
                                    | SpawnPhase::Live
                            ) && record.cause == Some(FailureCause::Timeout)
                    );
                // A pre-mint failure carries a retained diagnostics
                // record — print it so CI logs show the frozen
                // phase/cause/stderr at the failure point (Issue
                // #207 diagnosis aid; dumped only when this attempt
                // is terminal).
                if !signature || attempt == SPAWN_TIMEOUT_RETRIES {
                    match &query {
                        DiagnosticQuery::Found(record) => {
                            eprintln!("diagnostics record: {record:#?}");
                            eprintln!(
                                "child stderr (lossy): {}",
                                String::from_utf8_lossy(&record.stderr.bytes)
                            );
                        }
                        other => eprintln!("diagnostics query result: {other:?}"),
                    }
                }
                assert!(
                    signature,
                    "instantiate failed: SpawnFailed{{cause:{cause:?}, \
                     diagnostics_id:{diagnostics_id}}} is outside the \
                     pre-mint timeout signature (phase={phase:?} \
                     record_cause={record_cause:?}) — not retried"
                );
                assert!(
                    attempt < SPAWN_TIMEOUT_RETRIES,
                    "instantiate failed: the pre-mint \
                     SpawnFailed{{cause:Timeout}} signature persisted \
                     through the {SPAWN_TIMEOUT_RETRIES}-retry bound \
                     (phase={phase:?}) — an honest environment \
                     failure, not an absorbed flake"
                );
            }
            Err(error) => panic!("instantiate failed: {error:?}"),
        }
    }
    unreachable!("the loop exits only through `return` or `panic!`")
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

/// `SaveState` → the state blob (`HostReply::State`) — the raw
/// 20-byte `clap.ext.state` stream, no adapter framing.
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

/// Extracts the plain gain from the state blob — the v2 layout
/// `[u32 LE "RFST"][u32 LE ver=2][f64 LE][u32 LE checksum]`; the f64
/// sits at offset 8.
fn blob_gain(blob: &[u8]) -> f64 {
    assert_eq!(blob.len(), 20, "the CLAP state stream is one 20-byte blob");
    assert_eq!(&blob[0..4], b"RFST");
    assert_eq!(&blob[4..8], &2_u32.to_le_bytes());
    f64::from_le_bytes(blob[8..16].try_into().unwrap())
}

/// The `test_stateful` L1 round-trip: real `clap.ext.state`
/// `save`/`load` through the production host bridge, out-of-process.
///
/// save(initial, gain 1.0) → mutate to 0.75 via a `ParamValue` event
/// → save(mutated) → mutate to 1.75 → save → restore(initial) →
/// save == initial byte-exact → audio proves the restored gain drives
/// the DSP. Every state verb is gated on delivered audio so the
/// child's data plane has provably applied each mutation first.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_save_mutate_restore_save() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);
    assert_eq!(
        host.lifecycle_state(handle),
        Some(LifecycleState::Processing)
    );

    // --- save(initial): default plain gain 1.0 ---
    let initial = save_state(&mut host, handle);
    assert_eq!(blob_gain(&initial), 1.0);
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
    assert_eq!(blob_gain(&mutated), 0.75);

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
    assert_eq!(blob_gain(&third), 1.75);
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

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);

    let valid = save_state(&mut host, handle);
    assert_eq!(blob_gain(&valid), 1.0);

    // Truncated blob → the codec's malformed check → `PluginFailed`.
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: valid[..8].to_vec(),
        }),
        Err(HostError::PluginFailed)
    );

    // A well-formed-length blob with a corrupted magic →
    // `PluginFailed` at codec validation.
    let mut corrupt = valid.clone();
    corrupt[0] ^= 0xFF;
    assert_eq!(
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: corrupt,
        }),
        Err(HostError::PluginFailed)
    );

    // Checksum mismatch → `PluginFailed` — the codec's integrity
    // check.
    let mut corrupt_sum = valid.clone();
    corrupt_sum[16] ^= 0xFF;
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
    future[4..8].copy_from_slice(&3_u32.to_le_bytes());
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

/// Migration and forward-compat degrade through the L1 wire: the
/// legacy 16-byte v1 blob restores (`Migrated`) and the next
/// `SaveState` re-serializes the 20-byte v2 layout; a v2 blob with
/// an unknown extension tail restores (`Degraded`) and re-saves
/// canonical. The restored gain drives the DSP afterwards — the same
/// state contract the in-process host-path test asserts,
/// out-of-process.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_migrates_and_degrades_state() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);

    let canonical = save_state(&mut host, handle);
    assert_eq!(blob_gain(&canonical), 1.0);

    // Craft the v1 payload: the v2 core (magic+version+f64) with the
    // version rewritten to 1 — 16 bytes, no checksum field.
    let mut v1 = canonical[..16].to_vec();
    v1[4..8].copy_from_slice(&1_u32.to_le_bytes());
    host.execute(HostCommand::RestoreState { handle, bytes: v1 })
        .unwrap();
    let migrated = save_state(&mut host, handle);
    assert_eq!(migrated, canonical, "v1 in → canonical v2 out");

    // v2 core + unknown extension tail → degraded accept, tail
    // dropped on re-save.
    let mut degraded = canonical.clone();
    degraded.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
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

/// The `test_stateful` plugin→host dirty-notification path (Issue
/// #891), out-of-process: a `RestoreState` that changes the gain
/// makes the fixture call `clap_host_state.mark_dirty` — the
/// bridge's `HostState` main-thread handler latches
/// `DirtyNotice::StateDirty`, pulled to the consumer through
/// `PluginHost::drain_dirty_notices` (`DrainDirtyNotices` →
/// `Reply::DirtyNotices`, wire v7).
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_state_restore_announces_dirty() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);

    // Baseline: the since-last-drain pull surface starts empty.
    let mut notices = Vec::new();
    assert_eq!(
        host.drain_dirty_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(notices, []);

    let initial = save_state(&mut host, handle); // plain gain 1.0

    // Mutate to ×0.75 so the restore below is a change edge — the
    // same delivered-audio gate every state verb rides.
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×0.75 mutation event never landed"
    );
    assert!(
        wait_for_gain_output(&mut host, handle, 0.375, -0.1875),
        "delivered audio never showed the ×0.75 mutation"
    );
    let mutated = save_state(&mut host, handle);
    assert_eq!(blob_gain(&mutated), 0.75);

    // restore(initial): `clap.ext.state` `load` →
    // `apply_restored_gain(1.0)` — changed from 0.75 → the fixture
    // calls `mark_dirty` → `DirtyNotice::StateDirty`.
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial.clone(),
    })
    .unwrap();

    notices.clear();
    assert_eq!(
        host.drain_dirty_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(notices, [DirtyNotice::StateDirty]);

    // Restoring the already-current value is no change edge — the
    // fixture fires no `mark_dirty` and the drain stays empty.
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial,
    })
    .unwrap();
    notices.clear();
    assert_eq!(
        host.drain_dirty_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(notices, []);

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}

/// The `test_stateful` plugin→host param-notification path (Issue
/// #912), out-of-process: a `RestoreState` that changes the gain
/// makes the fixture call `clap_host_params.rescan(VALUES)` — the
/// bridge's `HostParams` main-thread handler enqueues
/// `ParamNotice::RescanRequested`, pulled to the consumer through
/// `PluginHost::drain_param_notices` (`DrainParamNotices` →
/// `Reply::ParamNotices`, wire v9). The registered
/// `clap_host_preset_load` surface has no wire verb that reaches it
/// — its `loaded`/`on_error` coverage stays at the unit level.
#[test]
#[ignore = "requires the workspace-built test-fixture cdylib and the `l1-runner`-feature L1 runner — `mise run test-l1-real-plugin` builds both and selects this test"]
fn real_stateful_test_plugin_l1_state_restore_requests_param_rescan() {
    assert_fixture_present();

    let mut host = PluginHost::new();
    host.register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner_program()))
        .unwrap();
    let handle = instantiate(&mut host, EXPECTED.id);
    drive_to_processing(&mut host, handle);

    // Baseline: the since-last-drain pull surface starts empty.
    let mut notices = Vec::new();
    assert_eq!(
        host.drain_param_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(notices, []);

    let initial = save_state(&mut host, handle); // plain gain 1.0

    // Mutate to ×0.75 so the restore below is a change edge — the
    // same delivered-audio gate every state verb rides.
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.375).unwrap(),
        }),
    )];
    assert!(
        submit_mutation_until_landed(
            &mut host,
            handle,
            EventList::new(&events, FRAMES_U32).unwrap(),
        ),
        "the ×0.75 mutation event never landed"
    );
    assert!(
        wait_for_gain_output(&mut host, handle, 0.375, -0.1875),
        "delivered audio never showed the ×0.75 mutation"
    );
    let mutated = save_state(&mut host, handle);
    assert_eq!(blob_gain(&mutated), 0.75);

    // restore(initial): `clap.ext.state` `load` →
    // `apply_restored_gain(1.0)` — changed from 0.75 → the fixture
    // calls `rescan(VALUES)` (alongside `mark_dirty`, which lands
    // on the dirty surface).
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial.clone(),
    })
    .unwrap();

    notices.clear();
    assert_eq!(
        host.drain_param_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(
        notices,
        [ParamNotice::RescanRequested {
            aspects: RescanAspects::VALUES,
        }]
    );

    // Restoring the already-current value is no change edge — the
    // fixture fires no `rescan` and the drain stays empty.
    host.execute(HostCommand::RestoreState {
        handle,
        bytes: initial,
    })
    .unwrap();
    notices.clear();
    assert_eq!(
        host.drain_param_notices(handle, &mut notices).unwrap(),
        ReportDrain::Drained
    );
    assert_eq!(notices, []);

    // Clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert_eq!(host.lifecycle_state(handle), None);
}
